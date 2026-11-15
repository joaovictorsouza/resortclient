"""
cc_config.py
------------
Pure (Blender-free) configuration of the animation pipeline for the Character Creator ("CC_Base")
skeleton of the Rukha93 modular character, and for Mixamo sources.
"""

MIXAMO = "mixamorig:"
# Rotation joints of the CC skeleton that follow a Mixamo bone.
# (CC joint, Mixamo bone, CC child, Mixamo child), parents first, hips first. The child pair
# gives the rest direction of the bone, used to correct the A-pose (CC) against the T-pose (Mixamo).
CC_ROOT = "RL_BoneRoot"
CC_HIPS = "CC_Base_Hip"


def _arm(side, letter):
    clavicle = f"CC_BaseClavicle.{letter}"
    upper = f"CC_BaseUpperarm.{letter}"
    fore = f"CC_BaseForearm.{letter}"
    hand = f"CC_BaseHand.{letter}"
    entries = [
        (clavicle, f"{side}Shoulder", upper, f"{side}Arm"),
        (upper, f"{side}Arm", fore, f"{side}ForeArm"),
        (fore, f"{side}ForeArm", hand, f"{side}Hand"),
        (hand, f"{side}Hand", f"CC_BaseMid1.{letter}", f"{side}HandMiddle1"),
    ]
    for cc_finger, mx_finger in (("Thumb", "Thumb"), ("Index", "Index"), ("Mid", "Middle"), ("Ring", "Ring"), ("Pinky", "Pinky")):
        for i in (1, 2, 3):
            child = (f"CC_Base{cc_finger}{i + 1}.{letter}", f"{side}Hand{mx_finger}{i + 1}") if i < 3 else (None, None)
            entries.append((f"CC_Base{cc_finger}{i}.{letter}", f"{side}Hand{mx_finger}{i}", child[0], child[1]))
    return entries


def _leg(side, letter):
    return [
        (f"CC_BaseThigh.{letter}", f"{side}UpLeg", f"CC_BaseCalf.{letter}", f"{side}Leg"),
        (f"CC_BaseCalf.{letter}", f"{side}Leg", f"CC_BaseFoot.{letter}", f"{side}Foot"),
        (f"CC_BaseFoot.{letter}", f"{side}Foot", f"CC_BaseToeBase.{letter}", f"{side}ToeBase"),
        (f"CC_BaseToeBase.{letter}", f"{side}ToeBase", None, None),
    ]


BONE_MAP = (
    [
        (CC_HIPS, "Hips", "CC_Base_Waist", "Spine"),
        ("CC_Base_Waist", "Spine", "CC_Base_Spine01", "Spine1"),
        ("CC_Base_Spine01", "Spine1", "CC_Base_Spine02", "Spine2"),
        ("CC_Base_Spine02", "Spine2", "CC_Base_NeckTwist01", "Neck"),
        ("CC_Base_NeckTwist01", "Neck", "CC_Base_Head", "Head"),
        ("CC_Base_Head", "Head", None, None),
    ]
    + _arm("Left", "L")
    + _arm("Right", "R")
    + _leg("Left", "L")
    + _leg("Right", "R")
)


def mixamo_name(bone):
    return MIXAMO + bone


def prefixed_mapping():
    return [
        (dst, mixamo_name(src), dst_child, mixamo_name(src_child) if src_child else None)
        for dst, src, dst_child, src_child in BONE_MAP
    ]


# Height of an avatar in world units (see `AVATAR_TARGET_HEIGHT` in the game).
WORLD_AVATAR_HEIGHT = 1.8
# Duration (s) of a held static pose clip; it loops, so any value works.
STATIC_CLIP_SECONDS = 0.5
# While seated, only the buttocks and the back of the thighs touch the seat: vertices in front of
# the pelvis (towards the knees) are ignored when measuring the contact height (centimetres).
SEAT_CONTACT_MAX_Z_CM = 5.0
# A standing frame picked from the walk cycle: the one where the feet are closest together.
IDLE_BREATH_SECONDS = 4.0


def clip_metadata(loop, duration_s, contact_height_m, model_to_world, speed=None):
    """
    Entry of `clips.json` for one clip. `contact_height_m` (metres) is how high above the avatar
    origin the body rests on its support: the bottom of the buttocks for seated clips, 0 for
    clips laid on a surface. The client lifts the avatar by `support_height - contact_height`.
    """
    meta = {
        "loop": loop,
        "duration_s": round(duration_s, 4),
        "contact_height": round(contact_height_m * model_to_world, 4),
    }
    if speed is not None:
        meta["tiles_per_sec"] = round(speed, 4)
    return meta
