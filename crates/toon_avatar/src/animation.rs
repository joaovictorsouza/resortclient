//! Skeletal animation of avatars.
//!
//! Each body skeleton becomes the single animated skeleton of its avatar: it gets the
//! [`AnimationPlayer`] and the animation targets, and the garments and attachments follow
//! because their skins already point at the body's joints (see `avatar.rs`).
//!
//! Clips come from a rig-only glTF built by `tools/animations/build_cc_animations.py`
//! (`idle`, `walk`, `sit`, `lay`) and their metadata from the matching `clips.json`; the
//! application loads both however it likes (a CDN, say) and describes them in an
//! [`AvatarAnimationConfig`] resource.
//!
//! **Animation level of detail.** An avatar at the lowest level of detail animates at a lower
//! rate ([`AvatarLod::anim_interval_secs`]): its player is stopped, and every interval one frame
//! of the clip is evaluated and left standing, so the skeleton costs nothing in between. An
//! avatar outside the view is not evaluated at all.

use std::{collections::HashMap, time::Duration};

use bevy::{
    animation::{AnimatedBy, AnimationTargetId, graph::AnimationNodeIndex},
    gltf::Gltf,
    prelude::*,
};
use serde::Deserialize;

use crate::{
    avatar::{Avatar, Body, BodyReady},
    catalog::BodyType,
    lod::AvatarLod,
};

/// Name the animation library gives to the root of its skeleton; the body's own armature node
/// takes this name in the animation target paths, whatever it is called in its model.
pub const RIG_ROOT_NAME: &str = "Skeleton";
const TRANSITION: Duration = Duration::from_millis(200);
const MIN_WALK_SPEED: f32 = 0.25;
const MAX_WALK_SPEED: f32 = 3.0;

/// What an avatar is doing.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AvatarPose {
    #[default]
    Idle,
    Walk,
    Sit,
    Lay,
    /// Bringing a held cup to the mouth while standing; ends by itself back in `Idle`.
    Drink,
    /// The same, seated; ends back in `Sit`.
    DrinkSit,
}

impl AvatarPose {
    pub fn clip_name(self) -> &'static str {
        match self {
            AvatarPose::Idle => "idle",
            AvatarPose::Walk => "walk",
            AvatarPose::Sit => "sit",
            AvatarPose::Lay => "lay",
            AvatarPose::Drink => "drink",
            AvatarPose::DrinkSit => "drink_sit",
        }
    }

    /// The pose to drink from, if the avatar can drink from this one (standing still or seated).
    pub fn drinking(self) -> Option<Self> {
        match self {
            AvatarPose::Idle => Some(AvatarPose::Drink),
            AvatarPose::Sit => Some(AvatarPose::DrinkSit),
            _ => None,
        }
    }

    /// The pose a one-shot gesture returns to when it ends.
    pub fn after_gesture(self) -> Option<Self> {
        match self {
            AvatarPose::Drink => Some(AvatarPose::Idle),
            AvatarPose::DrinkSit => Some(AvatarPose::Sit),
            _ => None,
        }
    }
}

/// Ground speed (tiles per second) the avatar is moving at; the walk clip is played faster or
/// slower to keep the feet from sliding.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AvatarWalkSpeed(pub f32);

/// Metadata of one clip, from `clips.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClipMeta {
    #[serde(rename = "loop")]
    pub looping: bool,
    pub duration_s: f32,
    /// How high above the avatar origin the body rests on its support (world units).
    pub contact_height: f32,
    #[serde(default)]
    pub tiles_per_sec: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct ClipsFile {
    clips: HashMap<String, ClipMeta>,
    #[serde(default)]
    grip: Option<GripsFile>,
}

#[derive(Debug, Deserialize)]
struct GripsFile {
    right: GripFile,
}

#[derive(Debug, Deserialize)]
struct GripFile {
    translation_cm: [f32; 3],
    rotation: [f32; 4],
}

/// Where a held prop sits in the right hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grip {
    /// The centre of the prop in the hand bone's frame, in the skeleton's centimetres.
    pub translation_cm: Vec3,
    /// From the prop's frame (+Y up) to the hand bone's frame.
    pub rotation: Quat,
}

/// Parses the `grip` section of a `clips.json`, if it has one.
pub fn parse_grip(json: &str) -> Option<Grip> {
    let file = serde_json::from_str::<ClipsFile>(json).ok()?;
    let g = file.grip?.right;
    Some(Grip {
        translation_cm: Vec3::from_array(g.translation_cm),
        rotation: Quat::from_xyzw(g.rotation[0], g.rotation[1], g.rotation[2], g.rotation[3])
            .normalize(),
    })
}

/// Parses a `clips.json` written by the animation pipeline.
pub fn parse_clips(json: &str) -> Result<HashMap<String, ClipMeta>, serde_json::Error> {
    serde_json::from_str::<ClipsFile>(json).map(|f| f.clips)
}

/// The animation library of one body type.
#[derive(Clone, Debug)]
pub struct BodyAnimations {
    /// Asset path of the rig-only glTF holding the clips.
    pub gltf_path: String,
    pub clips: HashMap<String, ClipMeta>,
    /// Where a held prop sits in the right hand (from the `grip` of `clips.json`).
    pub grip: Option<Grip>,
}

/// Where the animations are. Insert it when the clip metadata is known; the plugin loads the
/// glTF and builds the animation graph.
#[derive(Resource, Clone, Debug)]
pub struct AvatarAnimationConfig {
    pub female: BodyAnimations,
    pub male: BodyAnimations,
}

impl AvatarAnimationConfig {
    pub fn of(&self, body: BodyType) -> &BodyAnimations {
        match body {
            BodyType::Female => &self.female,
            BodyType::Male => &self.male,
        }
    }
}

struct BuiltLibrary {
    gltf: Handle<Gltf>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: HashMap<String, AnimationNodeIndex>,
}

/// The loaded animation graphs, one per body type.
#[derive(Resource, Default)]
pub struct AvatarAnimationLibrary {
    female: Option<BuiltLibrary>,
    male: Option<BuiltLibrary>,
}

impl AvatarAnimationLibrary {
    fn of(&self, body: BodyType) -> Option<&BuiltLibrary> {
        match body {
            BodyType::Female => self.female.as_ref(),
            BodyType::Male => self.male.as_ref(),
        }
    }
}

/// Lives next to the [`AnimationPlayer`] on the armature node of a body.
#[derive(Component, Debug)]
pub struct AvatarAnimator {
    avatar: Entity,
    current: Option<AvatarPose>,
    /// Own clock of the clip, advanced even while the player is stopped (throttled avatars).
    clock: f32,
    since_tick: f32,
    /// A frame was evaluated on purpose and the player must be stopped before the next one.
    evaluating: bool,
    throttled: bool,
}

/// The body skeleton already has its animation targets.
#[derive(Component)]
struct RigBound;

pub struct AvatarAnimationPlugin;

impl Plugin for AvatarAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AvatarAnimationLibrary>().add_systems(
            Update,
            (
                load_libraries,
                build_graphs,
                bind_skeletons,
                drive_animators,
                finish_gestures,
            )
                .chain(),
        );
    }
}

/// Starts loading the glTF of every body type once the configuration is known.
fn load_libraries(
    config: Option<Res<AvatarAnimationConfig>>,
    assets: Res<AssetServer>,
    mut library: ResMut<AvatarAnimationLibrary>,
) {
    let Some(config) = config else {
        return;
    };
    if library.female.is_none() || config.is_changed() {
        for (body, slot) in [(BodyType::Female, 0), (BodyType::Male, 1)] {
            let built = BuiltLibrary {
                gltf: assets.load(&config.of(body).gltf_path),
                graph: None,
                nodes: HashMap::new(),
            };
            if slot == 0 {
                library.female = Some(built);
            } else {
                library.male = Some(built);
            }
        }
    }
}

fn build_graphs(
    config: Option<Res<AvatarAnimationConfig>>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut library: ResMut<AvatarAnimationLibrary>,
) {
    let Some(config) = config else {
        return;
    };
    for body in [BodyType::Female, BodyType::Male] {
        let slot = match body {
            BodyType::Female => &mut library.female,
            BodyType::Male => &mut library.male,
        };
        let Some(built) = slot else {
            continue;
        };
        if built.graph.is_some() {
            continue;
        }
        let Some(gltf) = gltfs.get(&built.gltf) else {
            continue;
        };
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        for name in config.of(body).clips.keys() {
            match gltf.named_animations.get(name.as_str()) {
                Some(clip) => {
                    built
                        .nodes
                        .insert(name.clone(), graph.add_clip(clip.clone(), 1.0, root));
                }
                None => warn!(
                    "animation clip '{name}' missing from {}",
                    config.of(body).gltf_path
                ),
            }
        }
        built.graph = Some(graphs.add(graph));
    }
}

/// Depth-first collection of `(entity, name path from the rig root)` for the rig subtree; the
/// root's own name is replaced by [`RIG_ROOT_NAME`].
fn collect_rig_paths(
    rig: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Vec<(Entity, Vec<Name>)> {
    let mut out = Vec::new();
    let mut stack = vec![(rig, vec![Name::new(RIG_ROOT_NAME)])];
    while let Some((entity, path)) = stack.pop() {
        if let Ok(kids) = children.get(entity) {
            for child in kids.iter() {
                if let Ok(name) = names.get(child) {
                    let mut child_path = path.clone();
                    child_path.push(name.clone());
                    stack.push((child, child_path));
                }
            }
        }
        out.push((entity, path));
    }
    out
}

/// The armature node of a body: the first descendant named `Armature*`.
fn find_armature(
    body: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    children.iter_descendants(body).find(|e| {
        names
            .get(*e)
            .is_ok_and(|n| n.as_str().starts_with("Armature"))
    })
}

#[allow(clippy::type_complexity)]
fn bind_skeletons(
    mut commands: Commands,
    library: Res<AvatarAnimationLibrary>,
    bodies: Query<(Entity, &Body), (With<BodyReady>, Without<RigBound>)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for (body_entity, body) in &bodies {
        let Some(graph) = library.of(body.body_type).and_then(|b| b.graph.clone()) else {
            continue;
        };
        let Some(rig) = find_armature(body_entity, &children, &names) else {
            continue;
        };
        for (entity, path) in collect_rig_paths(rig, &children, &names) {
            commands
                .entity(entity)
                .try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(rig)));
        }
        commands.entity(rig).try_insert((
            AnimationPlayer::default(),
            AnimationGraphHandle(graph),
            AnimationTransitions::new(),
            AvatarAnimator {
                avatar: body.avatar,
                current: None,
                clock: 0.0,
                since_tick: 0.0,
                evaluating: false,
                throttled: false,
            },
        ));
        commands.entity(body_entity).try_insert(RigBound);
    }
}

/// Playback speed that keeps the feet from sliding: path speed / clip's native speed.
pub fn walk_playback_speed(path_tiles_per_sec: f32, clip_tiles_per_sec: Option<f32>) -> f32 {
    match clip_tiles_per_sec {
        Some(native) if native > 0.0 && path_tiles_per_sec > 0.0 => {
            (path_tiles_per_sec / native).clamp(MIN_WALK_SPEED, MAX_WALK_SPEED)
        }
        _ => 1.0,
    }
}

/// Whether a throttled animator should evaluate a frame now.
pub fn tick_due(since_tick: f32, interval: f32) -> bool {
    since_tick >= interval
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn drive_animators(
    time: Res<Time>,
    library: Res<AvatarAnimationLibrary>,
    config: Option<Res<AvatarAnimationConfig>>,
    bodies: Query<&Body>,
    parents: Query<&ChildOf>,
    avatars: Query<(&Avatar, &AvatarPose, &AvatarLod, Option<&AvatarWalkSpeed>)>,
    mut animators: Query<(
        Entity,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut AvatarAnimator,
    )>,
) {
    let Some(config) = config else {
        return;
    };
    let dt = time.delta_secs();
    for (rig, mut player, mut transitions, mut animator) in &mut animators {
        let Ok((avatar, pose, lod, walk)) = avatars.get(animator.avatar) else {
            continue;
        };
        // Only the body the avatar shows animates; a body being built is posed on its first frame.
        let Some(body) = parents
            .iter_ancestors(rig)
            .find_map(|e| bodies.get(e).ok().map(|b| (e, b)))
        else {
            continue;
        };
        let body_type = body.1.body_type;
        let Some(built) = library.of(body_type) else {
            continue;
        };
        let clip = pose.clip_name();
        let Some(&node) = built.nodes.get(clip) else {
            continue;
        };
        let meta = config.of(body_type).clips.get(clip);
        let looping = meta.is_none_or(|m| m.looping);
        let duration = meta.map_or(1.0, |m| m.duration_s.max(0.01));
        let speed = match (pose, walk) {
            (AvatarPose::Walk, Some(w)) => {
                walk_playback_speed(w.0, meta.and_then(|m| m.tiles_per_sec))
            }
            _ => 1.0,
        };
        let _ = avatar;

        let throttle = lod.anim_interval_secs > 0.0 || !lod.on_screen;
        if animator.current != Some(*pose) {
            animator.clock = 0.0;
        }
        animator.clock += dt * speed;
        if looping {
            animator.clock %= duration;
        } else {
            animator.clock = animator.clock.min(duration);
        }

        if !throttle {
            // Full rate: blend between clips and let the engine advance the player.
            if animator.throttled {
                // Back from throttling: resume the clip where the own clock is.
                animator.throttled = false;
                animator.evaluating = false;
                animator.current = None;
            }
            if animator.current != Some(*pose) {
                let clock = animator.clock;
                let active = transitions.play(&mut player, node, TRANSITION);
                if looping {
                    active.repeat();
                }
                active.seek_to(clock);
                animator.current = Some(*pose);
            }
            if *pose == AvatarPose::Walk
                && let Some(active) = player.animation_mut(node)
            {
                active.set_speed(speed);
            }
            continue;
        }

        // Throttled: the player rests, a frame is evaluated every interval and then left standing.
        animator.throttled = true;
        if animator.evaluating {
            player.stop_all();
            animator.evaluating = false;
        }
        animator.since_tick += dt;
        let pose_changed = animator.current != Some(*pose);
        if !lod.on_screen && !pose_changed {
            continue;
        }
        if pose_changed || tick_due(animator.since_tick, lod.anim_interval_secs) {
            animator.since_tick = 0.0;
            player.stop_all();
            player.start(node).seek_to(animator.clock);
            animator.evaluating = true;
            animator.current = Some(*pose);
        }
    }
}

/// A one-shot gesture (drinking) hands the avatar back to its base pose when its clip ends.
fn finish_gestures(
    config: Option<Res<AvatarAnimationConfig>>,
    animators: Query<&AvatarAnimator>,
    mut avatars: Query<(&Avatar, &mut AvatarPose)>,
) {
    let Some(config) = config else {
        return;
    };
    for animator in &animators {
        let Ok((avatar, mut pose)) = avatars.get_mut(animator.avatar) else {
            continue;
        };
        let (Some(back), true) = (pose.after_gesture(), animator.current == Some(*pose)) else {
            continue;
        };
        let duration = config
            .of(avatar.look.body)
            .clips
            .get(pose.clip_name())
            .map_or(0.0, |m| m.duration_s);
        if animator.clock >= duration - 1e-3 {
            *pose = back;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_walk_clip_is_sped_up_to_match_the_ground_speed() {
        assert_eq!(walk_playback_speed(2.0, Some(1.0)), 2.0);
        assert_eq!(walk_playback_speed(0.1, Some(1.0)), MIN_WALK_SPEED);
        assert_eq!(walk_playback_speed(9.0, Some(1.0)), MAX_WALK_SPEED);
        assert_eq!(walk_playback_speed(1.0, None), 1.0);
        assert_eq!(walk_playback_speed(0.0, Some(1.0)), 1.0);
    }

    #[test]
    fn clips_json_is_parsed() {
        let clips = parse_clips(
            r#"{"version":3,"clips":{"walk":{"loop":true,"duration_s":0.97,"contact_height":0.0,"tiles_per_sec":1.3},
               "sit":{"loop":true,"duration_s":0.5,"contact_height":0.45}}}"#,
        )
        .unwrap();
        assert_eq!(clips["walk"].tiles_per_sec, Some(1.3));
        assert_eq!(clips["sit"].tiles_per_sec, None);
        assert!(clips["sit"].looping);
    }

    #[test]
    fn a_throttled_avatar_evaluates_only_when_the_interval_has_passed() {
        assert!(!tick_due(0.05, 0.1));
        assert!(tick_due(0.1, 0.1));
        assert!(tick_due(0.3, 0.1));
    }

    #[test]
    fn only_a_standing_or_seated_avatar_can_drink_and_the_gesture_returns_to_its_base() {
        assert_eq!(AvatarPose::Idle.drinking(), Some(AvatarPose::Drink));
        assert_eq!(AvatarPose::Sit.drinking(), Some(AvatarPose::DrinkSit));
        assert_eq!(AvatarPose::Walk.drinking(), None);
        assert_eq!(AvatarPose::Lay.drinking(), None);
        assert_eq!(AvatarPose::Drink.after_gesture(), Some(AvatarPose::Idle));
        assert_eq!(AvatarPose::DrinkSit.after_gesture(), Some(AvatarPose::Sit));
        assert_eq!(AvatarPose::Idle.after_gesture(), None);
    }

    #[test]
    fn the_grip_is_read_from_clips_json() {
        let grip = parse_grip(r#"{"clips":{},"grip":{"right":{"translation_cm":[-3.5,3.0,0.0],"rotation":[0,0,0,1]}}}"#).unwrap();
        assert_eq!(grip.translation_cm, Vec3::new(-3.5, 3.0, 0.0));
        assert_eq!(grip.rotation, Quat::IDENTITY);
        assert_eq!(parse_grip(r#"{"clips":{}}"#), None);
    }

    #[test]
    fn every_pose_has_a_clip_name() {
        for pose in [
            AvatarPose::Drink,
            AvatarPose::DrinkSit,
            AvatarPose::Idle,
            AvatarPose::Walk,
            AvatarPose::Sit,
            AvatarPose::Lay,
        ] {
            assert!(!pose.clip_name().is_empty());
        }
    }
}
