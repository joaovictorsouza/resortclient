#!/usr/bin/env python3
"""
build_cc_animations.py
----------------------
Builds the animation library for the Character Creator ("CC_Base") skeleton of the modular
anime character from Mixamo sources, and exports it as a rig-only glTF with one named animation
per clip, plus a `clips.json` with per-clip metadata.

Clips:
  * `walk`: the whole Mixamo walk cycle, retargeted frame by frame. Horizontal root motion is
    dropped (the game moves the avatar along its path); `tiles_per_sec` comes from the stride.
  * `idle`: the standing frame of the walk cycle where the feet are closest together, with a slow
    procedural breathing motion on top.
  * `sit`, `lay`: a held frame of a Mixamo FBX.

Blender is only used to sample the FBX files. Usage:
  python3 build_cc_animations.py --body F_body.glb --head F_head.001.glb --walk Walking.fbx --sit Sitting.fbx \
      --lay "Lying Down.fbx" --out DIR [--blender /Applications/Blender.app/Contents/MacOS/Blender]

Outputs: DIR/character_anims.glb and DIR/clips.json. The skeleton root node is renamed `Skeleton`
in the output; the game renames the armature of its body models the same way (see
`rename_rig_root.py`), so animation target paths match.
"""
import argparse
import json
import math
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cc_config as cc  # noqa: E402
import drink as dk  # noqa: E402
import glb_tools as gt  # noqa: E402
import rig_math as rm  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
SAMPLER = os.path.join(HERE, "sample_mixamo.py")
DEFAULT_BLENDER = "/Applications/Blender.app/Contents/MacOS/Blender"
RIG_ROOT_OUT = "Skeleton"


def sample_fbx(blender, fbx, frame, out_json):
    subprocess.run([blender, "-b", "--python", SAMPLER, "--", fbx, str(frame), out_json], check=True, stdout=subprocess.DEVNULL)
    with open(out_json, encoding="utf-8") as f:
        raw = json.load(f)
    tup = lambda d: {k: tuple(v) for k, v in d.items()}  # noqa: E731
    return {
        "fps": raw["fps"], "first": raw["first"], "last": raw["last"], "hips": raw["hips"],
        "rest_rot": tup(raw["rest_rot"]), "rest_pos": tup(raw["rest_pos"]),
        "frames": [
            {"frame": fr["frame"], "pose_rot": tup(fr["pose_rot"]), "pose_pos": tup(fr["pose_pos"])}
            for fr in raw["frames"]
        ],
    }


def source_for(sampled, frame):
    return {
        "hips": sampled["hips"], "rest_rot": sampled["rest_rot"], "rest_pos": sampled["rest_pos"],
        "pose_rot": frame["pose_rot"], "pose_pos": frame["pose_pos"],
    }


class BodyMesh:
    """The skinned body (every primitive), used to measure where a pose touches its support."""

    def __init__(self, gltf, binary, rig):
        self.positions, self.joints, self.weights = [], [], []
        for mesh in gltf["meshes"]:
            for prim in mesh["primitives"]:
                attrs = prim["attributes"]
                self.positions += gt.read_accessor(gltf, binary, attrs["POSITION"])
                self.joints += [tuple(int(j) for j in js) for js in gt.read_accessor(gltf, binary, attrs["JOINTS_0"])]
                self.weights += gt.read_accessor(gltf, binary, attrs["WEIGHTS_0"])
        skin = gltf["skins"][0]
        self.inverse_bind = [rm.mat_from_gltf(m) for m in gt.read_accessor(gltf, binary, skin["inverseBindMatrices"])]
        self.slot_to_rig = [rig.index[gltf["nodes"][n]["name"]] for n in skin["joints"]]

    def posed(self, rig, rotations=None, overrides=None):
        world_r, world_p = rig.forward(rotations, overrides)
        matrices = [rm.mmul(rm.qmatrix(world_r[i], world_p[i]), self.inverse_bind[slot]) for slot, i in enumerate(self.slot_to_rig)]
        return rm.skin_vertices(self.positions, self.joints, self.weights, matrices)


def hips_local_translation(rig, world_y_delta, base_world_pos, parent_world):
    """Local translation of the hips joint that puts it `world_y_delta` above its rest world position."""
    parent_r, parent_p = parent_world
    target = (base_world_pos[0], base_world_pos[1] + world_y_delta, base_world_pos[2])
    return rm.qrot(rm.qconj(parent_r), rm.vsub(target, parent_p))


class Retargeter:
    def __init__(self, rig):
        self.rig = rig
        self.world_r, self.world_p = rig.forward()
        self.hips = rig.index[cc.CC_HIPS]
        self.parent = rig.parents[self.hips]
        self.mapping = cc.prefixed_mapping()

    def pose(self, src):
        """Local rotations and the hips local translation for one source frame."""
        rotations, hips_y = rm.retarget_pose(self.rig, self.mapping, src)
        rest_y = self.world_p[self.hips][1]
        local_t = hips_local_translation(
            self.rig, hips_y - rest_y, self.world_p[self.hips], (self.world_r[self.parent], self.world_p[self.parent])
        )
        return rotations, local_t


def tracks_for(rig, gltf, times, frames):
    """
    `frames` is a list of `(rotations, hips_translation)`. Every joint is keyed in every clip
    (rotation, translation, scale) so blending between clips never inherits a stale value.
    """
    tracks = []
    for i, node in enumerate(rig.node_indices):
        scale = gltf["nodes"][node].get("scale", (1.0, 1.0, 1.0))
        translations = [tuple(f[1]) if rig.names[i] == cc.CC_HIPS else tuple(rig.rest_t[i]) for f in frames]
        tracks.append((node, "translation", times, translations, "LINEAR"))
        tracks.append((node, "rotation", times, [tuple(f[0][i]) for f in frames], "LINEAR"))
        tracks.append((node, "scale", times, [tuple(scale)] * len(times), "LINEAR"))
    return tracks


def feet_gap(frame, src):
    """Distance between the feet in the source frame (smaller = a standing pose)."""
    a = frame["pose_pos"][cc.mixamo_name("LeftFoot")]
    b = frame["pose_pos"][cc.mixamo_name("RightFoot")]
    return math.dist((a[0], a[1]), (b[0], b[1]))


def walk_speed(sampled, scale):
    """Forward speed (source units per second) of the stance feet relative to the hips, times `scale`."""
    frames, fps = sampled["frames"], sampled["fps"]
    hips = sampled["hips"]
    speeds = []
    for foot in ("LeftFoot", "RightFoot"):
        name = cc.mixamo_name(foot)
        low = min(f["pose_pos"][name][2] for f in frames)
        for a, b in zip(frames, frames[1:]):
            if a["pose_pos"][name][2] < low + 0.04 and b["pose_pos"][name][2] < low + 0.04:
                # Mixamo faces -Y in Blender world; a stance foot moves backward (+Y) relative to the hips.
                ra = a["pose_pos"][name][1] - a["pose_pos"][hips][1]
                rb = b["pose_pos"][name][1] - b["pose_pos"][hips][1]
                speeds.append((rb - ra) * fps)
    if not speeds:
        raise SystemExit("could not measure the walk speed: no stance frames")
    return sum(speeds) / len(speeds) * scale


def breathing_frame(rig, rest_rotations, phase, hips_t, amplitude_deg=0.9):
    """Rest rotations with a small pitch added to the spine and head (world-space X axis)."""
    world_r, _ = rig.forward(rest_rotations)
    out = list(rest_rotations)
    for name, factor, lag in (("CC_Base_Waist", 0.6, 0.0), ("CC_Base_Spine01", 1.0, 0.15), ("CC_Base_Spine02", 0.8, 0.3), ("CC_Base_Head", -0.5, 0.5)):
        i = rig.index[name]
        angle = math.radians(amplitude_deg * factor) * math.sin(2 * math.pi * (phase - lag))
        half = angle / 2
        delta = (math.sin(half), 0.0, 0.0, math.cos(half))
        parent = rig.parents[i]
        # Recompute the world rotation of the parent with the rotations edited so far.
        parent_world, _ = rig.forward(out)
        new_world = rm.qnorm(rm.qmul(delta, world_r[i]))
        out[i] = rm.qnorm(rm.qmul(rm.qconj(parent_world[parent]), new_world))
    return out, hips_t


def model_height_cm(head_glb):
    """Full height of the model: the top of a head mesh (metres in the file)."""
    gltf, binary = gt.read_glb(head_glb)
    top = max(
        v[1]
        for mesh in gltf["meshes"]
        for prim in mesh["primitives"]
        for v in gt.read_accessor(gltf, binary, prim["attributes"]["POSITION"])
    )
    return top * 100.0


def build(body_glb, head_glb, walk_fbx, sit_fbx, lay_fbx, blender, out_dir):
    gltf, binary = gt.read_glb(body_glb)
    rig = rm.Rig.from_gltf(gltf)
    body = BodyMesh(gltf, binary, rig)
    rest = body.posed(rig)
    # The mesh is in metres and the skeleton (and the posed vertices) in centimetres.
    drift = max(max(abs(a - b * 100.0) for a, b in zip(p, q)) for p, q in zip(rest, body.positions))
    if drift > 1e-1:
        raise SystemExit(f"skinning self-check failed: rest pose drifts by {drift:.5f} cm")
    height_cm = model_height_cm(head_glb)
    model_to_world = cc.WORLD_AVATAR_HEIGHT / (height_cm / 100.0)
    print(f"Body height {height_cm:.2f} cm; model->world {model_to_world:.4f}")

    retarget = Retargeter(rig)
    work = tempfile.mkdtemp(prefix="cc_samples_")
    clips, meta = {}, {}

    # --- walk
    walk = sample_fbx(blender, walk_fbx, "all", os.path.join(work, "walk.json"))
    first = walk["frames"][0]["frame"]
    posed = [retarget.pose(source_for(walk, f)) for f in walk["frames"]]
    times = [(f["frame"] - first) / walk["fps"] for f in walk["frames"]]
    duration = times[-1] + 1.0 / walk["fps"]
    # Close the loop: one more key equal to the first at the end of the cycle.
    times.append(duration)
    posed.append(posed[0])
    clips["walk"] = tracks_for(rig, gltf, times, posed)
    hips_rest_cm = retarget.world_p[retarget.hips][1]
    src_hips_rest = rm.to_gltf_vec(walk["rest_pos"][walk["hips"]])[1]
    stride_scale = (hips_rest_cm / 100.0) / src_hips_rest  # CC legs against Mixamo legs
    speed_m = walk_speed(walk, stride_scale)
    meta["walk"] = cc.clip_metadata(True, duration, 0.0, model_to_world, speed_m * model_to_world)
    print(f"[walk] {len(posed) - 1} frames, {duration:.2f}s, {speed_m:.3f} m/s -> {meta['walk']['tiles_per_sec']} tiles/s")

    # --- idle: standing frame of the walk, breathing on top
    standing = min(walk["frames"], key=lambda f: feet_gap(f, walk))
    base_rot, base_t = retarget.pose(source_for(walk, standing))
    n = 16
    idle_times = [cc.IDLE_BREATH_SECONDS * i / n for i in range(n + 1)]
    idle_frames = [breathing_frame(rig, base_rot, (i % n) / n, base_t) for i in range(n + 1)]
    clips["idle"] = tracks_for(rig, gltf, idle_times, idle_frames)
    meta["idle"] = cc.clip_metadata(True, idle_times[-1], 0.0, model_to_world)
    print(f"[idle] frame {standing['frame']} of the walk, {idle_times[-1]:.1f}s breathing")

    # --- sit / lay: held frames
    held = {}
    for name, fbx, frame, ground in (("sit", sit_fbx, 185, False), ("lay", lay_fbx, "last", True)):
        sampled = sample_fbx(blender, fbx, frame, os.path.join(work, f"{name}.json"))
        rotations, local_t = retarget.pose(source_for(sampled, sampled["frames"][0]))
        verts = body.posed(rig, rotations, {cc.CC_HIPS: local_t})
        if ground:
            lowest = min(v[1] for v in verts)
            local_t = hips_local_translation(rig, -lowest + (local_t_world_delta(retarget, local_t)), retarget.world_p[retarget.hips], (retarget.world_r[retarget.parent], retarget.world_p[retarget.parent]))
            verts = body.posed(rig, rotations, {cc.CC_HIPS: local_t})
            contact = 0.0
        else:
            seat = [v[1] for v in verts if v[2] <= cc.SEAT_CONTACT_MAX_Z_CM]
            contact = min(seat)
        zs = [v[2] for v in verts]
        held[name] = (rotations, local_t, contact)
        clips[name] = tracks_for(rig, gltf, [0.0, cc.STATIC_CLIP_SECONDS], [(rotations, local_t)] * 2)
        meta[name] = cc.clip_metadata(True, cc.STATIC_CLIP_SECONDS, contact / 100.0, model_to_world)
        print(f"[{name}] contact={contact:.2f} cm, body middle along Z {(min(zs) + max(zs)) / 2:.2f} cm "
              f"= {(min(zs) + max(zs)) / 200.0 * model_to_world:.4f} world units, y range {min(v[1] for v in verts):.1f}..{max(v[1] for v in verts):.1f}")

    # --- drink: the right hand brings a cup to the mouth, from a standing and from a seated base pose
    head_rest = retarget.world_r[rig.index[dk.HEAD]]
    for name, (base_rot, base_t, contact_m) in (
        ("drink", (base_rot, base_t, 0.0)),
        ("drink_sit", (held["sit"][0], held["sit"][1], held["sit"][2] / 100.0)),
    ):
        keys = dk.drink_keys(rig, base_rot, {cc.CC_HIPS: base_t}, head_rest)
        clips[name] = tracks_for(rig, gltf, [k[0] for k in keys], [(k[1], base_t) for k in keys])
        meta[name] = cc.clip_metadata(False, keys[-1][0], contact_m, model_to_world)
        print(f"[{name}] {len(keys)} keys, {keys[-1][0]:.1f}s")

    os.makedirs(out_dir, exist_ok=True)
    glb_path = os.path.join(out_dir, "character_anims.glb")
    write_animation_glb(glb_path, (gltf, binary), clips)
    print(f"Exported {glb_path} ({os.path.getsize(glb_path)} bytes)")
    with open(os.path.join(out_dir, "clips.json"), "w", encoding="utf-8") as f:
        json.dump(
            {
                "version": 4,
                "model_height_m": round(height_cm / 100.0, 4),
                "clips": meta,
                # Where a held prop sits in the right hand: its centre (cm, hand frame) and the rotation
                # from the prop's frame (+Y up) to the hand frame, so it is upright-ish at the drinking pose.
                "grip": {"right": {"translation_cm": list(dk.CUP_CENTRE_IN_HAND), "rotation": [round(c, 5) for c in dk.grip_rotation()]}},
            },
            f,
            indent=2,
        )
        f.write("\n")


def local_t_world_delta(retarget, local_t):
    """World-space Y offset of the hips (relative to rest) that `local_t` produces."""
    parent_r, parent_p = retarget.world_r[retarget.parent], retarget.world_p[retarget.parent]
    world = rm.vadd(parent_p, rm.qrot(parent_r, local_t))
    return world[1] - retarget.world_p[retarget.hips][1]


def write_animation_glb(path, source, clips):
    """Writes a glTF holding only the skeleton nodes of `source` (mesh nodes dropped) and the animations."""
    gltf_src, _ = source
    nodes_src = gltf_src["nodes"]
    keep, order = {}, []
    scene_roots = gltf_src["scenes"][gltf_src.get("scene", 0)]["nodes"]
    stack = list(reversed(scene_roots))
    while stack:
        index = stack.pop()
        if "mesh" in nodes_src[index]:
            continue
        keep[index] = len(order)
        order.append(index)
        stack.extend(reversed(nodes_src[index].get("children", [])))
    roots = [keep[r] for r in scene_roots if r in keep]
    nodes = []
    for old in order:
        node = {k: v for k, v in nodes_src[old].items() if k in ("name", "translation", "rotation", "scale")}
        children = [keep[c] for c in nodes_src[old].get("children", []) if c in keep]
        if children:
            node["children"] = children
        nodes.append(node)
    for r in roots:
        nodes[r]["name"] = RIG_ROOT_OUT
    gltf = {"asset": {"version": "2.0", "generator": "toon_avatar build_cc_animations.py"}, "scene": 0,
            "scenes": [{"nodes": roots}], "nodes": nodes, "animations": []}
    builder = gt.BufferBuilder(gltf)
    for name, tracks in clips.items():
        samplers, channels = [], []
        for node, path_name, times, values, interpolation in tracks:
            samplers.append({
                "input": builder.add_floats(times, "SCALAR"),
                "output": builder.add_floats(values, "VEC4" if path_name == "rotation" else "VEC3"),
                "interpolation": interpolation,
            })
            channels.append({"sampler": len(samplers) - 1, "target": {"node": keep[node], "path": path_name}})
        gltf["animations"].append({"name": name, "samplers": samplers, "channels": channels})
    gt.write_glb(path, gltf, bytes(builder.data))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--body", required=True, help="body GLB with the CC skeleton and skin (e.g. F_body.glb)")
    parser.add_argument("--head", required=True, help="a head GLB, to measure the full height of the model")
    parser.add_argument("--walk", required=True)
    parser.add_argument("--sit", required=True)
    parser.add_argument("--lay", required=True)
    parser.add_argument("--blender", default=DEFAULT_BLENDER)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    build(args.body, args.head, args.walk, args.sit, args.lay, args.blender, args.out)


if __name__ == "__main__":
    main()
