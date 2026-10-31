"""
sample_mixamo.py
----------------
Blender-only: dumps the rest pose and the poses of a Mixamo FBX as JSON, in Blender world space
(Z up). `build_cc_animations.py` converts it to glTF space and retargets it onto the CC skeleton;
keeping Blender out of the math keeps that part unit-testable.

Usage:
  Blender -b --python sample_mixamo.py -- <input.fbx> <frame|last|all> <output.json>

Output: {"fps", "first", "last", "hips", "rest_rot", "rest_pos", "frames": [{"frame", "pose_rot", "pose_pos"}]}
where rotations are quaternions [x, y, z, w] and positions are [x, y, z], both per bone name
(including the `mixamorig:` prefix).
"""
import json
import sys

import bpy


def quat_xyzw(matrix):
    q = matrix.to_3x3().normalized().to_quaternion()
    return [q.x, q.y, q.z, q.w]


def main():
    fbx, frame_arg, out_path = sys.argv[sys.argv.index("--") + 1:][:3]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=fbx)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    first, last = (int(round(v)) for v in arm.animation_data.action.frame_range)
    if frame_arg == "all":
        frames = list(range(first, last + 1))
    else:
        frame = last if frame_arg == "last" else int(frame_arg)
        if not first <= frame <= last:
            raise SystemExit(f"frame {frame} outside the animation range {first}..{last}")
        frames = [frame]

    world = arm.matrix_world
    rest_rot = {b.name: quat_xyzw(world @ b.matrix_local) for b in arm.data.bones}
    rest_pos = {b.name: list(world @ b.head_local) for b in arm.data.bones}
    out_frames = []
    for frame in frames:
        bpy.context.scene.frame_set(frame)
        out_frames.append({
            "frame": frame,
            "pose_rot": {pb.name: quat_xyzw(world @ pb.matrix) for pb in arm.pose.bones},
            "pose_pos": {pb.name: list(world @ pb.head) for pb in arm.pose.bones},
        })
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump({
            "fps": bpy.context.scene.render.fps, "first": first, "last": last, "hips": "mixamorig:Hips",
            "rest_rot": rest_rot, "rest_pos": rest_pos, "frames": out_frames,
        }, f)
    print(f"Sampled {fbx}: {len(frames)} frame(s) of {first}..{last} -> {out_path}")


if __name__ == "__main__":
    main()
