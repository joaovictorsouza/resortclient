//! The face: blinking and expressions, driven through the morph targets (blend shapes) of the
//! head. Only avatars whose level of detail keeps the morph targets show it; the lowest level
//! has none and costs nothing.
//!
//! Which blend shape does what was checked by rendering every one of the 46 shape keys of the
//! head: `eyes_closed.03` shuts both eyes, `happy_01..03` / `sad_01..03` / `angry_01..03` are
//! three strengths of each emotion (03 opens the mouth), `surprised_01` raises the brows and
//! opens the mouth.

use bevy::{mesh::morph::MeshMorphWeights, prelude::*};

use crate::{
    avatar::{Body, Equipped},
    catalog::Slot,
    lod::AvatarLod,
};

/// How the avatar looks right now, apart from blinking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Expression {
    #[default]
    Neutral,
    Happy,
    Laugh,
    Sad,
    Angry,
    Surprised,
}

impl Expression {
    pub const ALL: [Expression; 6] = [
        Expression::Neutral,
        Expression::Happy,
        Expression::Laugh,
        Expression::Sad,
        Expression::Angry,
        Expression::Surprised,
    ];

    /// The blend shapes of the expression and how far each is pushed (0..1).
    pub fn shapes(self) -> &'static [(&'static str, f32)] {
        match self {
            Expression::Neutral => &[],
            Expression::Happy => &[("happy_02", 0.9)],
            Expression::Laugh => &[("happy_03", 1.0), ("eyebrows_up", 0.2)],
            Expression::Sad => &[("sad_02", 1.0)],
            Expression::Angry => &[("angry_02", 1.0)],
            Expression::Surprised => &[("surprised_01", 1.0), ("eyes_wide", 0.6)],
        }
    }

    /// Stable name for storage and the network.
    pub fn name(self) -> &'static str {
        match self {
            Expression::Neutral => "neutral",
            Expression::Happy => "happy",
            Expression::Laugh => "laugh",
            Expression::Sad => "sad",
            Expression::Angry => "angry",
            Expression::Surprised => "surprised",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }
}

/// What the face does. Set `expression` to change it; turn `blink` off for a fixed stare.
#[derive(Component, Clone, Copy, Debug)]
#[require(FaceAnimator)]
pub struct AvatarFace {
    pub expression: Expression,
    pub blink: bool,
}

impl Default for AvatarFace {
    fn default() -> Self {
        Self {
            expression: Expression::Neutral,
            blink: true,
        }
    }
}

/// The blend shapes the face drives, in the order of [`FaceAnimator::current`].
const DRIVEN: [&str; 11] = [
    "eyes_closed.03",
    "happy_02",
    "happy_03",
    "sad_02",
    "angry_02",
    "surprised_01",
    "eyes_wide",
    "eyebrows_up",
    "eyebrows_down",
    "jaw_open",
    "eyes_closed.01",
];

/// How fast the expression follows its target: the fraction left after one second.
const EXPRESSION_RESIDUE_PER_SEC: f32 = 1e-4;
const BLINK_CLOSE: f32 = 0.07;
const BLINK_HOLD: f32 = 0.04;
const BLINK_OPEN: f32 = 0.11;

/// Per-avatar state of the face.
#[derive(Component, Default, Debug)]
pub struct FaceAnimator {
    current: [f32; DRIVEN.len()],
    /// Seconds since the last blink started; `None` while no blink is running.
    blink_elapsed: Option<f32>,
    /// Seconds until the next blink.
    next_blink: f32,
    rng: u32,
}

/// How closed the eyes are `elapsed` seconds into a blink: closing, a short hold, opening.
pub fn blink_amount(elapsed: f32) -> f32 {
    if elapsed < 0.0 {
        0.0
    } else if elapsed < BLINK_CLOSE {
        elapsed / BLINK_CLOSE
    } else if elapsed < BLINK_CLOSE + BLINK_HOLD {
        1.0
    } else if elapsed < BLINK_CLOSE + BLINK_HOLD + BLINK_OPEN {
        1.0 - (elapsed - BLINK_CLOSE - BLINK_HOLD) / BLINK_OPEN
    } else {
        0.0
    }
}

pub const BLINK_DURATION: f32 = BLINK_CLOSE + BLINK_HOLD + BLINK_OPEN;

/// xorshift32: a small deterministic generator, enough to vary the blinks of each avatar.
fn next_random(state: &mut u32) -> f32 {
    let mut x = if *state == 0 { 0x9E37_79B9 } else { *state };
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Seconds until the next blink: usually 2 to 6, and now and then a quick double blink.
pub fn next_blink_interval(state: &mut u32) -> f32 {
    if next_random(state) < 0.15 {
        0.25
    } else {
        2.0 + next_random(state) * 4.0
    }
}

/// Target weight of each driven shape for an expression and a blink amount.
pub fn target_weights(expression: Expression, blink: f32) -> [f32; DRIVEN.len()] {
    let mut out = [0.0; DRIVEN.len()];
    for (name, value) in expression.shapes() {
        if let Some(i) = DRIVEN.iter().position(|d| d == name) {
            out[i] = *value;
        }
    }
    // Blinking closes the eyes over whatever the expression does with them.
    out[0] = out[0].max(blink);
    out
}

pub struct AvatarFacePlugin;

impl Plugin for AvatarFacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, drive_faces);
    }
}

/// Moves the state of one avatar's face forward by `dt` and returns the weights to write.
fn advance_face(face: &AvatarFace, animator: &mut FaceAnimator, avatar_index: u32, dt: f32) {
    if animator.rng == 0 {
        animator.rng = avatar_index.wrapping_mul(2_654_435_761) | 1;
        animator.next_blink = next_blink_interval(&mut animator.rng);
    }
    let mut blink = 0.0;
    if face.blink {
        match animator.blink_elapsed {
            Some(elapsed) => {
                let elapsed = elapsed + dt;
                if elapsed >= BLINK_DURATION {
                    animator.blink_elapsed = None;
                    animator.next_blink = next_blink_interval(&mut animator.rng);
                } else {
                    animator.blink_elapsed = Some(elapsed);
                    blink = blink_amount(elapsed);
                }
            }
            None => {
                animator.next_blink -= dt;
                if animator.next_blink <= 0.0 {
                    animator.blink_elapsed = Some(0.0);
                }
            }
        }
    }
    let target = target_weights(face.expression, blink);
    let keep = EXPRESSION_RESIDUE_PER_SEC.powf(dt);
    for (i, t) in target.iter().enumerate() {
        // The blink is fast and follows its curve exactly; the rest eases.
        animator.current[i] = if i == 0 {
            *t
        } else {
            *t + (animator.current[i] - *t) * keep
        };
    }
}

/// Writes the face state into the blend-shape weights of the head meshes. The head is one of the
/// garments of the body (slot `Face`); its meshes keep their own morph weights.
#[allow(clippy::too_many_arguments)]
fn drive_faces(
    time: Res<Time>,
    meshes: Res<Assets<Mesh>>,
    bodies: Query<&Body>,
    mut avatars: Query<(&AvatarFace, &AvatarLod, &mut FaceAnimator)>,
    mut heads: Query<(&Equipped, &Mesh3d, &mut MeshMorphWeights)>,
) {
    let dt = time.delta_secs();
    let mut advanced: Vec<Entity> = Vec::new();
    for (equipped, mesh, mut weights) in &mut heads {
        if equipped.slot != Slot::Face {
            continue;
        }
        let Ok(body) = bodies.get(equipped.body) else {
            continue;
        };
        let Ok((face, lod, mut animator)) = avatars.get_mut(body.avatar) else {
            continue;
        };
        if !lod.on_screen {
            continue;
        }
        // The state advances once per avatar, however many head meshes and bodies it has.
        if !advanced.contains(&body.avatar) {
            advanced.push(body.avatar);
            advance_face(face, &mut animator, body.avatar.index_u32(), dt);
        }
        let (Some(names), MeshMorphWeights::Value { weights }) = (
            meshes.get(&mesh.0).and_then(|m| m.morph_target_names()),
            &mut *weights,
        ) else {
            continue;
        };
        for (i, driven) in DRIVEN.iter().enumerate() {
            if let Some(slot) = names.iter().position(|n| n == driven)
                && let Some(w) = weights.get_mut(slot)
            {
                *w = animator.current[i];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blink_closes_holds_and_opens() {
        assert_eq!(blink_amount(-1.0), 0.0);
        assert_eq!(blink_amount(0.0), 0.0);
        assert!((blink_amount(BLINK_CLOSE / 2.0) - 0.5).abs() < 1e-5);
        assert_eq!(blink_amount(BLINK_CLOSE + BLINK_HOLD / 2.0), 1.0);
        let opening = blink_amount(BLINK_CLOSE + BLINK_HOLD + BLINK_OPEN / 2.0);
        assert!((opening - 0.5).abs() < 1e-5);
        assert_eq!(blink_amount(BLINK_DURATION + 0.01), 0.0);
    }

    #[test]
    fn blinks_come_every_two_to_six_seconds_with_the_odd_double_blink() {
        let mut state = 12345u32;
        let mut quick = 0;
        for _ in 0..2000 {
            let t = next_blink_interval(&mut state);
            if t == 0.25 {
                quick += 1;
            } else {
                assert!((2.0..=6.0).contains(&t), "{t}");
            }
        }
        assert!((150..450).contains(&quick), "{quick} quick blinks of 2000");
    }

    #[test]
    fn avatars_with_different_seeds_do_not_blink_together() {
        let (mut a, mut b) = (0x1234_5678u32, 0x9abc_def1u32);
        let first: Vec<f32> = (0..5).map(|_| next_blink_interval(&mut a)).collect();
        let second: Vec<f32> = (0..5).map(|_| next_blink_interval(&mut b)).collect();
        assert_ne!(first, second);
    }

    #[test]
    fn the_blink_closes_the_eyes_over_any_expression() {
        let w = target_weights(Expression::Surprised, 1.0);
        assert_eq!(w[0], 1.0);
        let neutral = target_weights(Expression::Neutral, 0.0);
        assert!(neutral.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn every_expression_drives_only_known_shapes() {
        for e in Expression::ALL {
            for (name, value) in e.shapes() {
                assert!(DRIVEN.contains(name), "{name} is not driven");
                assert!((0.0..=1.0).contains(value));
            }
        }
    }

    #[test]
    fn expressions_round_trip_through_their_names() {
        for e in Expression::ALL {
            assert_eq!(Expression::from_name(e.name()), Some(e));
        }
        assert_eq!(Expression::from_name("nope"), None);
    }
}
