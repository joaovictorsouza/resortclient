//! Avatar poses: how the game's movement and furniture interactions become the animation of the
//! `toon_avatar` character.
//!
//! The skeleton, the clips and their blending live in the `toon_avatar` crate (see
//! `crates/toon_avatar/src/animation.rs`); this module only decides **which pose** an avatar is in
//! (walking, standing, sitting or lying), tells the crate, and lifts the character so the clip's
//! contact point (the pelvis on a seat, the back on a bed) lands on the supporting surface.

use bevy::prelude::*;
use toon_avatar::{AvatarAnimationConfig, AvatarWalkSpeed};

use crate::avatar::{AvatarVisualRoot, PathFollower};

pub struct AvatarAnimationPlugin;

impl Plugin for AvatarAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_avatar_pose, sync_toon_poses, apply_posture_offset).chain(),
        );

        #[cfg(debug_assertions)]
        app.add_systems(Update, debug_cycle_local_posture);
    }
}

/// What the avatar is currently doing; picks the clip that plays.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum AvatarPose {
    #[default]
    Idle,
    Walk,
    Sit,
    Lay,
}

/// How the avatar rests when it is not walking (set by furniture interactions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PostureKind {
    #[default]
    Stand,
    Sit,
    Lay,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct AvatarPosture {
    pub kind: PostureKind,
    /// Height (world units, above the avatar's tile) of the surface the avatar rests on,
    /// e.g. the seat of a chair. `None` keeps the clip at its authored height.
    pub support_height: Option<f32>,
}

impl AvatarPosture {
    /// Maps the server-authoritative posture (spec 0004 do servidor) onto the client posture.
    pub fn from_server(posture: protocol::Posture, y: f64) -> Self {
        let kind = match posture {
            protocol::Posture::Stand => return Self::default(),
            protocol::Posture::Sit => PostureKind::Sit,
            protocol::Posture::Lay => PostureKind::Lay,
        };
        Self {
            kind,
            support_height: Some(y as f32),
        }
    }
}

// ---------------------------------------------------------------------------
// Pure logic
// ---------------------------------------------------------------------------

/// Walking always wins; otherwise the posture decides.
pub fn desired_pose(is_moving: bool, posture: PostureKind) -> AvatarPose {
    if is_moving {
        return AvatarPose::Walk;
    }
    match posture {
        PostureKind::Stand => AvatarPose::Idle,
        PostureKind::Sit => AvatarPose::Sit,
        PostureKind::Lay => AvatarPose::Lay,
    }
}

/// The pose of the `toon_avatar` character for a game pose.
pub fn toon_pose(pose: AvatarPose) -> toon_avatar::AvatarPose {
    match pose {
        AvatarPose::Idle => toon_avatar::AvatarPose::Idle,
        AvatarPose::Walk => toon_avatar::AvatarPose::Walk,
        AvatarPose::Sit => toon_avatar::AvatarPose::Sit,
        AvatarPose::Lay => toon_avatar::AvatarPose::Lay,
    }
}

/// What to tell the character, if anything: a drinking gesture in progress is left to finish while
/// the avatar stays in the pose it is drinking from; any other change replaces it.
pub fn pose_to_apply(
    wanted: AvatarPose,
    current: toon_avatar::AvatarPose,
) -> Option<toon_avatar::AvatarPose> {
    let wanted = toon_pose(wanted);
    if current.after_gesture() == Some(wanted) {
        return None;
    }
    (current != wanted).then_some(wanted)
}

/// Vertical offset (world units) of the avatar visuals so the clip's contact point
/// (pelvis for seats, back for beds) lands on the supporting surface.
pub fn posture_offset(support_height: Option<f32>, contact_height: f32) -> f32 {
    support_height.map_or(0.0, |support| support - contact_height)
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn update_avatar_pose(mut avatars: Query<(&PathFollower, &mut AvatarPosture, &mut AvatarPose)>) {
    for (follower, mut posture, mut pose) in avatars.iter_mut() {
        let moving = !follower.remaining.is_empty();
        if moving && posture.kind != PostureKind::Stand {
            *posture = AvatarPosture::default();
        }
        let wanted = desired_pose(moving, posture.kind);
        if *pose != wanted {
            *pose = wanted;
        }
    }
}

/// Hands the pose (and the walking speed) to the character child of every avatar.
fn sync_toon_poses(
    avatars: Query<(&AvatarPose, &PathFollower, &Children)>,
    mut characters: Query<
        (&mut toon_avatar::AvatarPose, &mut AvatarWalkSpeed),
        With<AvatarVisualRoot>,
    >,
) {
    for (pose, follower, children) in avatars.iter() {
        for child in children.iter() {
            let Ok((mut character_pose, mut speed)) = characters.get_mut(child) else {
                continue;
            };
            if let Some(next) = pose_to_apply(*pose, *character_pose) {
                *character_pose = next;
            }
            if speed.0 != follower.speed {
                speed.0 = follower.speed;
            }
        }
    }
}

/// Lifts the character so the contact point of its clip rests on the surface it is on.
fn apply_posture_offset(
    config: Option<Res<AvatarAnimationConfig>>,
    avatars: Query<(&crate::avatar::Avatar, &AvatarPosture, &Children)>,
    mut visuals: Query<(&mut Transform, &toon_avatar::AvatarPose), With<AvatarVisualRoot>>,
) {
    let Some(config) = config else { return };
    for (avatar, posture, children) in avatars.iter() {
        let clips = &config
            .of(if avatar.appearance.body_type == protocol::BODY_MALE {
                toon_avatar::BodyType::Male
            } else {
                toon_avatar::BodyType::Female
            })
            .clips;
        for child in children.iter() {
            let Ok((mut transform, pose)) = visuals.get_mut(child) else {
                continue;
            };
            let contact = clips
                .get(pose.clip_name())
                .map_or(0.0, |m| m.contact_height);
            let offset = if *pose == toon_avatar::AvatarPose::Walk {
                0.0
            } else {
                posture_offset(posture.support_height, contact)
            };
            if transform.translation.y != offset {
                transform.translation.y = offset;
            }
        }
    }
}

/// Debug helper until furniture exists: `P` (or F6, which macOS maps to a media key
/// unless `fn` is held) cycles the local avatar's resting posture.
#[cfg(debug_assertions)]
fn debug_cycle_local_posture(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Option<Res<crate::navigator::ActiveInputFocus>>,
    mut avatars: Query<(&crate::avatar::Avatar, &mut AvatarPosture)>,
) {
    if !(keys.just_pressed(KeyCode::KeyP) || keys.just_pressed(KeyCode::F6)) {
        return;
    }
    // Don't steal the key while the player is typing in a text field.
    if focus.is_some_and(|f| f.0 != crate::navigator::FocusedInput::None) {
        return;
    }
    for (avatar, mut posture) in avatars.iter_mut() {
        if !avatar.is_local {
            continue;
        }
        posture.kind = next_debug_posture(posture.kind);
        info!("Debug posture: {:?}", posture.kind);
    }
}

/// Order in which the debug shortcut cycles through the resting postures.
pub fn next_debug_posture(kind: PostureKind) -> PostureKind {
    match kind {
        PostureKind::Stand => PostureKind::Sit,
        PostureKind::Sit => PostureKind::Lay,
        PostureKind::Lay => PostureKind::Stand,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_cycle_visits_every_posture_and_returns_to_stand() {
        let mut kind = PostureKind::Stand;
        let mut seen = vec![kind];
        for _ in 0..3 {
            kind = next_debug_posture(kind);
            seen.push(kind);
        }
        assert_eq!(
            seen,
            vec![
                PostureKind::Stand,
                PostureKind::Sit,
                PostureKind::Lay,
                PostureKind::Stand
            ]
        );
    }

    #[test]
    fn server_posture_maps_to_client_posture() {
        assert_eq!(
            AvatarPosture::from_server(protocol::Posture::Stand, 0.0),
            AvatarPosture::default()
        );
        let sit = AvatarPosture::from_server(protocol::Posture::Sit, 0.8);
        assert_eq!(sit.kind, PostureKind::Sit);
        assert_eq!(sit.support_height, Some(0.8));
        let lay = AvatarPosture::from_server(protocol::Posture::Lay, 0.5);
        assert_eq!(lay.kind, PostureKind::Lay);
    }

    #[test]
    fn walking_overrides_posture() {
        assert_eq!(desired_pose(true, PostureKind::Sit), AvatarPose::Walk);
        assert_eq!(desired_pose(true, PostureKind::Stand), AvatarPose::Walk);
    }

    #[test]
    fn resting_pose_follows_posture() {
        assert_eq!(desired_pose(false, PostureKind::Stand), AvatarPose::Idle);
        assert_eq!(desired_pose(false, PostureKind::Sit), AvatarPose::Sit);
        assert_eq!(desired_pose(false, PostureKind::Lay), AvatarPose::Lay);
    }

    #[test]
    fn a_game_pose_maps_to_the_character_pose_of_the_same_name() {
        for (game, toon) in [
            (AvatarPose::Idle, "idle"),
            (AvatarPose::Walk, "walk"),
            (AvatarPose::Sit, "sit"),
            (AvatarPose::Lay, "lay"),
        ] {
            assert_eq!(toon_pose(game).clip_name(), toon);
        }
    }

    #[test]
    fn a_drinking_gesture_is_left_to_finish_but_any_other_change_replaces_it() {
        use toon_avatar::AvatarPose as Toon;
        // Standing still while drinking: nothing to do.
        assert_eq!(pose_to_apply(AvatarPose::Idle, Toon::Drink), None);
        assert_eq!(pose_to_apply(AvatarPose::Sit, Toon::DrinkSit), None);
        // Started walking, sat down from a standing drink, or lay down: the gesture is dropped.
        assert_eq!(
            pose_to_apply(AvatarPose::Walk, Toon::Drink),
            Some(Toon::Walk)
        );
        assert_eq!(pose_to_apply(AvatarPose::Sit, Toon::Drink), Some(Toon::Sit));
        assert_eq!(
            pose_to_apply(AvatarPose::Lay, Toon::DrinkSit),
            Some(Toon::Lay)
        );
        // A plain pose change, and no change.
        assert_eq!(
            pose_to_apply(AvatarPose::Walk, Toon::Idle),
            Some(Toon::Walk)
        );
        assert_eq!(pose_to_apply(AvatarPose::Idle, Toon::Idle), None);
    }

    #[test]
    fn posture_offset_puts_contact_on_support() {
        assert_eq!(posture_offset(None, 0.39), 0.0);
        assert!((posture_offset(Some(0.45), 0.39) - 0.06).abs() < 1e-6);
        assert!((posture_offset(Some(0.5), 0.0) - 0.5).abs() < 1e-6);
    }
}
