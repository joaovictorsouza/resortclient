"""
drink.py
--------
Pure-Python authoring of the "drink" gesture for the CC skeleton: the right hand brings a cup
(or a can) to the mouth, tilts it, and lowers it again. The arm is solved with a two-bone IK
(law of cosines) for the hand position of each key, so the hand lands on the mouth whatever the
base pose (standing or sitting) is. Built on `rig_math` (no Blender, no numpy).

Conventions: the rig's own world space (the skeleton space of the GLB, centimetres, Y up, +Z
forward). The character's right side is -X.
"""
import math

import rig_math as rm

UPPERARM = "CC_BaseUpperarm.R"
FOREARM = "CC_BaseForearm.R"
HAND = "CC_BaseHand.R"
HEAD = "CC_Base_Head"

# Mouth relative to the head joint, in the head's rest frame (cm): measured on the face mesh.
MOUTH_FROM_HEAD = (0.0, 0.5, 7.8)
# Where the cup is held: its centre relative to the hand joint, in the hand's local frame (cm).
# The hand's local +Y points along the fingers; the palm faces local -X.
CUP_CENTRE_IN_HAND = (-3.5, 3.0, 0.0)
CUP_HALF_HEIGHT = 4.5
HOLD_TILT_DEG = 35.0
# Elbow direction hint: out to the side and down.
POLE = (-0.5, -0.7, -0.3)

# (time in seconds, tilt of the cup in degrees, rim position relative to the mouth in cm); the
# first and last keys are the plain base pose.
KEYS = [
    (0.0, None, None),
    (0.9, 0.0, (0.0, -5.0, 8.0)),
    (1.3, 15.0, (0.0, 0.0, 2.5)),
    (1.8, HOLD_TILT_DEG, (0.0, 0.6, 1.2)),
    (2.4, HOLD_TILT_DEG, (0.0, 0.6, 1.2)),
    (2.9, 10.0, (0.0, -5.0, 8.0)),
    (3.6, None, None),
]


def _norm(v):
    return rm.vnorm(v)


def _scale(v, s):
    return (v[0] * s, v[1] * s, v[2] * s)


def _matrix_to_quat(cols):
    """Quaternion of a rotation whose columns (local x, y, z in world space) are `cols`."""
    (m00, m10, m20), (m01, m11, m21), (m02, m12, m22) = cols
    trace = m00 + m11 + m22
    if trace > 0:
        s = math.sqrt(trace + 1.0) * 2
        q = ((m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s)
    elif m00 > m11 and m00 > m22:
        s = math.sqrt(1.0 + m00 - m11 - m22) * 2
        q = (0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s)
    elif m11 > m22:
        s = math.sqrt(1.0 + m11 - m00 - m22) * 2
        q = ((m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s)
    else:
        s = math.sqrt(1.0 + m22 - m00 - m11) * 2
        q = ((m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s)
    return rm.qnorm(q)


def hand_hold_rotation():
    """World rotation of the right hand while it holds the cup at the mouth (cup tilted by `HOLD_TILT_DEG`)."""
    # Fingers point mostly up and a little forward and across; the palm faces the cup (+X).
    y = _norm((0.35, 0.9, 0.2))
    n = (1.0, 0.0, 0.0)
    n = _norm(rm.vsub(n, _scale(y, rm.vdot(n, y))))
    x = _scale(n, -1.0)  # local +X points away from the palm
    z = rm.vcross(x, y)
    return _matrix_to_quat((x, y, z))


def rot_about_x(deg):
    half = math.radians(deg) / 2
    return (math.sin(half), 0.0, 0.0, math.cos(half))


def cup_up(tilt_deg):
    t = math.radians(tilt_deg)
    return (0.0, math.cos(t), math.sin(t))


def grip_rotation():
    """Rotation from the prop's frame (+Y up) to the right hand's frame, so the prop is `HOLD_TILT_DEG` tilted at the hold."""
    hand = hand_hold_rotation()
    cup = rm.q_between((0.0, 1.0, 0.0), cup_up(HOLD_TILT_DEG))
    return rm.qnorm(rm.qmul(rm.qconj(hand), cup))


def hand_rotation(tilt_deg):
    """World rotation of the hand when the cup is tilted by `tilt_deg` (the cup is rigid with the hand)."""
    return rm.qnorm(rm.qmul(rot_about_x(tilt_deg - HOLD_TILT_DEG), hand_hold_rotation()))


def two_bone_ik(shoulder, elbow, wrist, target, pole):
    """New elbow position for a wrist at `target`, keeping the bone lengths; the elbow bends towards `pole`."""
    l1 = rm.vlen(rm.vsub(elbow, shoulder))
    l2 = rm.vlen(rm.vsub(wrist, elbow))
    to_target = rm.vsub(target, shoulder)
    d = min(max(rm.vlen(to_target), abs(l1 - l2) + 1e-3), l1 + l2 - 1e-3)
    u = _norm(to_target)
    a = (l1 * l1 - l2 * l2 + d * d) / (2 * d)
    h = math.sqrt(max(l1 * l1 - a * a, 0.0))
    side = rm.vsub(pole, _scale(u, rm.vdot(pole, u)))
    side = _norm(side)
    new_elbow = rm.vadd(shoulder, rm.vadd(_scale(u, a), _scale(side, h)))
    new_wrist = rm.vadd(shoulder, _scale(u, d))
    return new_elbow, new_wrist


def locals_from_world(rig, base_locals, world_overrides):
    """Local rotations that give the joints in `world_overrides` those world rotations, the others keep their local rotation."""
    world, locals_ = [], []
    for i, parent in enumerate(rig.parents):
        parent_world = world[parent] if parent is not None else rm.IDENTITY
        name = rig.names[i]
        if name in world_overrides:
            w = world_overrides[name]
            local = rm.qnorm(rm.qmul(rm.qconj(parent_world), w))
        else:
            local = tuple(base_locals[i])
            w = rm.qnorm(rm.qmul(parent_world, local))
        locals_.append(local)
        world.append(w)
    return locals_


def drink_pose(rig, base_locals, hips_override, rest_head_world, tilt_deg, rim_offset):
    """Local rotations with the right arm holding the cup at `rim_offset` (cm) from the mouth."""
    world_r, world_p = rig.forward(base_locals, hips_override)
    i_up, i_fo, i_ha, i_he = (rig.index[n] for n in (UPPERARM, FOREARM, HAND, HEAD))
    # The mouth follows the head.
    mouth_rel = rm.qrot(rm.qconj(rest_head_world), MOUTH_FROM_HEAD)
    mouth = rm.vadd(world_p[i_he], rm.qrot(world_r[i_he], mouth_rel))
    rim = rm.vadd(mouth, rim_offset)
    up = cup_up(tilt_deg)
    centre = rm.vsub(rim, _scale(up, CUP_HALF_HEIGHT))
    hand_rot = hand_rotation(tilt_deg)
    wrist = rm.vsub(centre, rm.qrot(hand_rot, CUP_CENTRE_IN_HAND))

    shoulder, elbow, hand_now = world_p[i_up], world_p[i_fo], world_p[i_ha]
    new_elbow, new_wrist = two_bone_ik(shoulder, elbow, hand_now, wrist, POLE)

    # Swing the upper arm, then the forearm, then set the hand.
    q_upper = rm.q_between(rm.vsub(elbow, shoulder), rm.vsub(new_elbow, shoulder))
    upper_world = rm.qnorm(rm.qmul(q_upper, world_r[i_up]))
    fore_dir_after_upper = rm.qrot(q_upper, rm.vsub(hand_now, elbow))
    q_fore = rm.q_between(fore_dir_after_upper, rm.vsub(new_wrist, new_elbow))
    fore_world = rm.qnorm(rm.qmul(q_fore, rm.qmul(q_upper, world_r[i_fo])))
    return locals_from_world(rig, base_locals, {UPPERARM: upper_world, FOREARM: fore_world, HAND: hand_rot}), (new_wrist, mouth)


def drink_keys(rig, base_locals, hips_override, rest_head_world):
    """`[(time, locals)]` for the whole gesture; the first and last keys are the base pose."""
    out = []
    for time, tilt, rim in KEYS:
        if tilt is None:
            out.append((time, list(base_locals)))
        else:
            locals_, _ = drink_pose(rig, base_locals, hips_override, rest_head_world, tilt, rim)
            out.append((time, locals_))
    return out
