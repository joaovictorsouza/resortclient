"""
rig_math.py
-----------
Pure-Python (no numpy, no Blender) math for the character rig: quaternions
`(x, y, z, w)`, forward kinematics over a glTF skeleton, linear blend skinning and the
world-space retarget of a Mixamo pose onto the pack skeleton.

Conventions: glTF world space (Y up, +Z forward). Blender world space (Z up, -Y forward)
is converted with `BLENDER_TO_GLTF`.
"""
import math

IDENTITY = (0.0, 0.0, 0.0, 1.0)
# Rotation of -90 degrees about X: Blender world (x, y, z) -> glTF world (x, z, -y).
BLENDER_TO_GLTF = (-math.sqrt(0.5), 0.0, 0.0, math.sqrt(0.5))


# --- vectors -----------------------------------------------------------------------

def vsub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def vadd(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def vdot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def vcross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def vlen(a):
    return math.sqrt(vdot(a, a))


def vnorm(a):
    length = vlen(a)
    if length == 0.0:
        raise ValueError("cannot normalize a zero vector")
    return (a[0] / length, a[1] / length, a[2] / length)


# --- quaternions -------------------------------------------------------------------

def qmul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return (
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    )


def qconj(q):
    return (-q[0], -q[1], -q[2], q[3])


def qnorm(q):
    length = math.sqrt(sum(c * c for c in q))
    return tuple(c / length for c in q)


def qrot(q, v):
    """Rotates vector `v` by unit quaternion `q`."""
    x, y, z = qmul(qmul(q, (v[0], v[1], v[2], 0.0)), qconj(q))[:3]
    return (x, y, z)


def q_between(a, b):
    """Shortest-arc rotation taking direction `a` onto direction `b`."""
    a, b = vnorm(a), vnorm(b)
    d = vdot(a, b)
    if d > 0.999999:
        return IDENTITY
    if d < -0.999999:
        # Opposite directions: any axis perpendicular to `a` works.
        axis = vcross((1.0, 0.0, 0.0), a)
        if vlen(axis) < 1e-6:
            axis = vcross((0.0, 1.0, 0.0), a)
        axis = vnorm(axis)
        return (axis[0], axis[1], axis[2], 0.0)
    c = vcross(a, b)
    return qnorm((c[0], c[1], c[2], 1.0 + d))


def qmatrix(q, t=(0.0, 0.0, 0.0)):
    """4x4 row-major matrix (as nested lists) of rotation `q` followed by translation `t`."""
    x, y, z, w = q
    return [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w), t[0]],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w), t[1]],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y), t[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]


def mmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def mat_from_gltf(flat):
    """glTF matrices are column-major; returns the row-major nested-list form."""
    return [[flat[col * 4 + row] for col in range(4)] for row in range(4)]


def mtransform(m, v):
    return (
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2] + m[0][3],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2] + m[1][3],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2] + m[2][3],
    )


# --- skeleton ----------------------------------------------------------------------

class Rig:
    """A skeleton: joint names, parent indices (parents always come first) and rest TRS."""

    def __init__(self, names, parents, translations, rotations):
        self.names = list(names)
        self.parents = list(parents)
        self.rest_t = [tuple(t) for t in translations]
        self.rest_r = [tuple(r) for r in rotations]
        self.index = {name: i for i, name in enumerate(self.names)}
        for i, p in enumerate(self.parents):
            if p is not None and p >= i:
                raise ValueError("joints must be ordered parent-first")

    @classmethod
    def from_gltf(cls, gltf, skin_index=0):
        """
        Builds a Rig from the joints of a skin. Joints are reordered parent-first; a joint
        whose parent is not a joint becomes a root (the non-joint ancestors are assumed to
        be identity, which holds for the pack's `Skeleton` node).
        """
        nodes = gltf["nodes"]
        joint_nodes = set(gltf["skins"][skin_index]["joints"])
        parent_of = {}
        for i, node in enumerate(nodes):
            for child in node.get("children", []):
                parent_of[child] = i
        order, seen = [], set()

        def visit(node_index):
            if node_index in seen:
                return
            parent = parent_of.get(node_index)
            if parent in joint_nodes:
                visit(parent)
            seen.add(node_index)
            order.append(node_index)

        for node_index in gltf["skins"][skin_index]["joints"]:
            visit(node_index)
        position = {n: i for i, n in enumerate(order)}
        rig = cls(
            [nodes[n]["name"] for n in order],
            [position[parent_of[n]] if parent_of.get(n) in joint_nodes else None for n in order],
            [nodes[n].get("translation", (0.0, 0.0, 0.0)) for n in order],
            [nodes[n].get("rotation", IDENTITY) for n in order],
        )
        rig.node_indices = order  # glTF node index of each joint
        return rig

    def children(self, name):
        i = self.index[name]
        return [self.names[j] for j, p in enumerate(self.parents) if p == i]

    def forward(self, rotations=None, hips_override=None):
        """
        Forward kinematics. `rotations` overrides the local rotations (list parallel to the
        joints); `hips_override` maps joint name -> local translation. Returns
        `(world_rotations, world_positions)`.
        """
        rotations = rotations or self.rest_r
        hips_override = hips_override or {}
        world_r, world_p = [], []
        for i, parent in enumerate(self.parents):
            local_t = hips_override.get(self.names[i], self.rest_t[i])
            if parent is None:
                world_r.append(tuple(rotations[i]))
                world_p.append(tuple(local_t))
            else:
                world_r.append(qnorm(qmul(world_r[parent], rotations[i])))
                world_p.append(vadd(world_p[parent], qrot(world_r[parent], local_t)))
        return world_r, world_p


def skin_vertices(positions, joints, weights, joint_matrices):
    """
    Linear blend skinning. `joint_matrices[j]` is `world_j * inverse_bind_j` for skin joint
    slot `j`. Returns the posed vertex positions.
    """
    out = []
    for p, js, ws in zip(positions, joints, weights):
        x = y = z = 0.0
        for j, w in zip(js, ws):
            if w <= 0.0:
                continue
            tx, ty, tz = mtransform(joint_matrices[j], p)
            x += w * tx
            y += w * ty
            z += w * tz
        out.append((x, y, z))
    return out


# --- retarget ----------------------------------------------------------------------

def to_gltf_vec(v):
    """Blender world position -> glTF world position."""
    return (v[0], v[2], -v[1])


def delta_to_gltf(rest, pose):
    """
    World-space rotation delta of a source bone (rest -> pose), expressed in glTF world
    space. `rest` and `pose` are the bone's Blender world rotations.
    """
    delta = qmul(pose, qconj(rest))
    return qnorm(qmul(qmul(BLENDER_TO_GLTF, delta), qconj(BLENDER_TO_GLTF)))


def retarget_pose(rig, mapping, src):
    """
    Poses `rig` like the Mixamo skeleton in `src`.

    `mapping` is a list of `(dst, src, dst_child, src_child)` bone names, parent-first. The
    rotation of each mapped joint is the source world delta (rest -> pose) applied on top of
    a rest-direction correction that aligns the rig's rest bone with the source's rest bone
    (the pack rests in an A-pose, Mixamo in a T-pose). Joints without a child direction
    reuse the correction of their mapped ancestor; unmapped joints keep their rest local
    rotation.

    `src` is a dict with Blender-world data: `rest_rot`, `rest_pos`, `pose_rot`, `pose_pos`
    (bone -> value) and `hips` (the source hips bone name). The first mapping entry must be
    the hips. Returns `(local_rotations, hips_y)` where `hips_y` is the new height of the
    rig's hips (horizontal motion is dropped, the vertical offset is scaled by hip height).
    """
    world_rest, pos_rest = rig.forward()
    corrections, deltas, targets = {}, {}, {}
    by_dst = {m[0]: m for m in mapping}

    for dst, src_name, dst_child, src_child in mapping:
        if dst_child and src_child:
            dir_dst = vsub(pos_rest[rig.index[dst_child]], pos_rest[rig.index[dst]])
            dir_src = to_gltf_vec(vsub(src["rest_pos"][src_child], src["rest_pos"][src_name]))
            corrections[dst] = q_between(dir_dst, dir_src)
        else:
            parent = rig.parents[rig.index[dst]]
            while parent is not None and rig.names[parent] not in by_dst:
                parent = rig.parents[parent]
            corrections[dst] = corrections[rig.names[parent]] if parent is not None else IDENTITY
        deltas[dst] = delta_to_gltf(src["rest_rot"][src_name], src["pose_rot"][src_name])
        targets[dst] = qnorm(qmul(qmul(deltas[dst], corrections[dst]), world_rest[rig.index[dst]]))

    locals_, world_new = [], []
    for i, parent in enumerate(rig.parents):
        parent_world = world_new[parent] if parent is not None else IDENTITY
        name = rig.names[i]
        if name in targets:
            world = targets[name]
            local = qnorm(qmul(qconj(parent_world), world))
        else:
            local = tuple(rig.rest_r[i])
            world = qnorm(qmul(parent_world, local))
        locals_.append(local)
        world_new.append(world)

    hips = src["hips"]
    src_rest_y = to_gltf_vec(src["rest_pos"][hips])[1]
    src_pose_y = to_gltf_vec(src["pose_pos"][hips])[1]
    rig_hips_y = pos_rest[rig.index[mapping[0][0]]][1]
    hips_y = rig_hips_y + (src_pose_y - src_rest_y) * (rig_hips_y / src_rest_y)
    return locals_, hips_y
