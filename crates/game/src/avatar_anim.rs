//! Avatar poses and skeletal animation.
//!
//! Every avatar is assembled from several glTF scenes (body + cosmetic parts) that each
//! carry their own copy of the 44-joint `Skeleton` of the character pack. The body skeleton
//! becomes the single animated skeleton of the avatar: it gets the [`AnimationPlayer`] and
//! the animation targets, and the skinned meshes of every cosmetic part are re-pointed at
//! the body's joints so clothes, hair and accessories follow the body.
//!
//! Clips come from `characters/animations/character_anims.glb` (built by
//! `resort-game-assets`: `tools/asset_pipeline/build_animations.py`) and their metadata from `clips.json`.

use std::collections::HashMap;
use std::time::Duration;

use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::gltf::Gltf;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use serde::Deserialize;

use crate::avatar::{AvatarVisualRoot, PathFollower};
use crate::cdn_assets::JsonText;

const ANIMATIONS_FILE: &str = "characters/animations/character_anims.glb";
/// Clip metadata; delivered by the CDN together with the animation library.
const CLIPS_FILE: &str = "characters/animations/clips.json";
/// Name of the scene root node (the skeleton's parent, holding the meshes) in every pack glTF.
/// Bevy derives animation targets from the name path starting here.
const RIG_ROOT_NAME: &str = "Skeleton";
const TRANSITION: Duration = Duration::from_millis(200);
/// Walk playback speed is clamped so very slow/fast paths don't look broken.
const MIN_WALK_SPEED: f32 = 0.25;
const MAX_WALK_SPEED: f32 = 3.0;

pub struct AvatarAnimationPlugin;

impl Plugin for AvatarAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_animation_library)
            .add_systems(
                Update,
                (
                    build_animation_graph,
                    bind_body_skeletons,
                    rebind_part_joints,
                    update_avatar_pose,
                    drive_animators,
                    apply_posture_offset,
                    reveal_posed_avatars,
                )
                    .chain(),
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

/// Marks the body scene of an avatar; its skeleton drives the whole avatar.
#[derive(Component, Debug, Clone, Copy)]
pub struct AvatarBodyPart;

/// Marks a cosmetic part scene whose skin must follow the body skeleton.
#[derive(Component, Debug, Clone, Copy)]
pub struct AvatarClothingPart;

/// Joints of a bound body skeleton, by node name.
#[derive(Component, Debug)]
pub struct AvatarSkeleton {
    joints: HashMap<String, Entity>,
}

/// Keeps a freshly spawned avatar hidden until it is ready to be seen: the glTF scenes load
/// asynchronously and show in their rest pose (arms spread, a T-pose) until the skeleton is
/// bound and the first clip plays. Put it on the root of the avatar visuals together with
/// `Visibility::Hidden`; [`reveal_posed_avatars`] shows it and removes the marker.
#[derive(Component, Debug, Default)]
pub struct HiddenUntilPosed {
    elapsed: f32,
}

/// An avatar that never becomes ready (e.g. the animation library failed to load) is shown
/// anyway after this long, instead of staying invisible.
const REVEAL_TIMEOUT_SECS: f32 = 5.0;

/// Marks a cosmetic part whose skinned meshes already point at the body joints.
#[derive(Component)]
struct JointsRebound;

/// Lives next to the [`AnimationPlayer`] on the body's `Skeleton` node.
#[derive(Component, Debug)]
struct AvatarAnimator {
    /// Entity carrying [`AvatarPose`] (the avatar root or the creator preview root).
    pose_source: Entity,
    current: Option<&'static str>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ClipMeta {
    #[serde(rename = "loop")]
    pub looping: bool,
    pub duration_s: f32,
    pub contact_height: f32,
    #[serde(default)]
    pub tiles_per_sec: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClipsFile {
    clips: HashMap<String, ClipMeta>,
}

pub fn parse_clips(json: &str) -> Result<HashMap<String, ClipMeta>, serde_json::Error> {
    serde_json::from_str::<ClipsFile>(json).map(|f| f.clips)
}

#[derive(Resource, Debug)]
pub struct AvatarAnimationLibrary {
    clips_file: Handle<JsonText>,
    clips_parsed: bool,
    gltf: Handle<Gltf>,
    graph: Option<Handle<AnimationGraph>>,
    nodes: HashMap<String, AnimationNodeIndex>,
    pub clips: HashMap<String, ClipMeta>,
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

/// glTF animation name (and `clips.json` key) of a pose.
pub fn clip_name(pose: AvatarPose) -> &'static str {
    match pose {
        AvatarPose::Idle => "idle",
        AvatarPose::Walk => "walk",
        AvatarPose::Sit => "sit",
        AvatarPose::Lay => "lay",
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

/// Vertical offset (world units) of the avatar visuals so the clip's contact point
/// (pelvis for seats, back for beds) lands on the supporting surface.
pub fn posture_offset(support_height: Option<f32>, contact_height: f32) -> f32 {
    support_height.map_or(0.0, |support| support - contact_height)
}

/// Maps each joint of a part skin (by name) to the body joint with the same name.
/// Returns `None` if any joint has no counterpart, leaving the part skin untouched.
pub fn remap_joints(
    part_joint_names: &[Option<&str>],
    body_joints: &HashMap<String, Entity>,
) -> Option<Vec<Entity>> {
    part_joint_names
        .iter()
        .map(|name| name.and_then(|n| body_joints.get(n).copied()))
        .collect()
}

/// Animation target id of a node, from the names on the path starting at the rig root
/// (inclusive) — the same convention as Bevy's glTF loader.
pub fn target_id(path: &[Name]) -> AnimationTargetId {
    AnimationTargetId::from_names(path.iter())
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn load_animation_library(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AvatarAnimationLibrary {
        clips_file: asset_server.load(CLIPS_FILE),
        clips_parsed: false,
        gltf: asset_server.load(ANIMATIONS_FILE),
        graph: None,
        nodes: HashMap::new(),
        clips: HashMap::new(),
    });
}

fn build_animation_graph(
    library: Option<ResMut<AvatarAnimationLibrary>>,
    gltfs: Res<Assets<Gltf>>,
    texts: Res<Assets<JsonText>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let Some(mut library) = library else { return };
    if library.graph.is_some() {
        return;
    }
    if !library.clips_parsed {
        let Some(text) = texts.get(&library.clips_file) else {
            return;
        };
        library.clips = parse_clips(&text.0).unwrap_or_else(|err| {
            error!("Invalid {CLIPS_FILE}: {err}");
            HashMap::new()
        });
        library.clips_parsed = true;
    }
    let Some(gltf) = gltfs.get(&library.gltf) else {
        return;
    };

    let mut graph = AnimationGraph::new();
    let root = graph.root;
    let mut nodes = HashMap::new();
    for name in library.clips.keys() {
        match gltf.named_animations.get(name.as_str()) {
            Some(clip) => {
                nodes.insert(name.clone(), graph.add_clip(clip.clone(), 1.0, root));
            }
            None => warn!("Animation clip '{name}' missing from {ANIMATIONS_FILE}"),
        }
    }
    library.nodes = nodes;
    library.graph = Some(graphs.add(graph));
}

/// Depth-first collection of `(entity, name path from rig root)` for the rig subtree.
fn collect_rig_paths(
    rig: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Vec<(Entity, Vec<Name>)> {
    let mut out = Vec::new();
    let Ok(root_name) = names.get(rig) else {
        return out;
    };
    let mut stack = vec![(rig, vec![root_name.clone()])];
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

fn find_named_descendant(
    root: Entity,
    target: &str,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if names.get(entity).is_ok_and(|n| n.as_str() == target) {
            return Some(entity);
        }
        if let Ok(kids) = children.get(entity) {
            stack.extend(kids.iter());
        }
    }
    None
}

fn find_pose_source(
    start: Entity,
    parents: &Query<&ChildOf>,
    poses: &Query<&AvatarPose>,
) -> Option<Entity> {
    let mut current = start;
    loop {
        if poses.contains(current) {
            return Some(current);
        }
        current = parents.get(current).ok()?.parent();
    }
}

#[allow(clippy::too_many_arguments)]
fn bind_body_skeletons(
    mut commands: Commands,
    library: Option<Res<AvatarAnimationLibrary>>,
    bodies: Query<Entity, (With<AvatarBodyPart>, Without<AvatarSkeleton>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    poses: Query<&AvatarPose>,
) {
    let Some(library) = library else { return };
    let Some(graph) = library.graph.clone() else {
        return;
    };

    for body in bodies.iter() {
        // The scene instance is spawned in one go; no rig yet means it is still loading.
        let Some(rig) = find_named_descendant(body, RIG_ROOT_NAME, &children, &names) else {
            continue;
        };
        let Some(pose_source) = find_pose_source(body, &parents, &poses) else {
            continue;
        };

        let mut joints = HashMap::new();
        for (entity, path) in collect_rig_paths(rig, &children, &names) {
            if let Some(last) = path.last() {
                joints.insert(last.as_str().to_string(), entity);
            }
            commands
                .entity(entity)
                .try_insert((target_id(&path), AnimatedBy(rig)));
        }

        commands.entity(rig).try_insert((
            AnimationPlayer::default(),
            AnimationGraphHandle(graph.clone()),
            AnimationTransitions::new(),
            AvatarAnimator {
                pose_source,
                current: None,
            },
        ));
        commands.entity(body).try_insert(AvatarSkeleton { joints });
    }
}

/// What [`reveal_posed_avatars`] has to see in an avatar's descendants before showing it.
#[derive(Debug, Default, PartialEq, Eq)]
struct AvatarReadiness {
    animating: bool,
    parts: usize,
    parts_ready: usize,
}

impl AvatarReadiness {
    /// The skeleton plays a clip, and every part has its joints rebound and its color applied.
    fn is_ready(&self) -> bool {
        self.animating && self.parts_ready == self.parts
    }
}

#[allow(clippy::type_complexity)]
fn reveal_posed_avatars(
    time: Res<Time>,
    mut commands: Commands,
    mut pending: Query<(Entity, &mut HiddenUntilPosed)>,
    children: Query<&Children>,
    animators: Query<&AvatarAnimator>,
    parts: Query<
        (
            Has<JointsRebound>,
            Has<crate::avatar::AvatarPartTint>,
            Has<crate::avatar::PartTintApplied>,
        ),
        With<AvatarClothingPart>,
    >,
    tinted_bodies: Query<
        (
            Has<crate::avatar::AvatarPartTint>,
            Has<crate::avatar::PartTintApplied>,
        ),
        With<AvatarBodyPart>,
    >,
) {
    for (root, mut hidden) in pending.iter_mut() {
        hidden.elapsed += time.delta_secs();
        let mut readiness = AvatarReadiness::default();
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            if animators.get(entity).is_ok_and(|a| a.current.is_some()) {
                readiness.animating = true;
            }
            if let Ok((rebound, tinted, applied)) = parts.get(entity) {
                readiness.parts += 1;
                readiness.parts_ready += usize::from(rebound && (!tinted || applied));
            }
            if let Ok((tinted, applied)) = tinted_bodies.get(entity) {
                readiness.parts += 1;
                readiness.parts_ready += usize::from(!tinted || applied);
            }
            if let Ok(kids) = children.get(entity) {
                stack.extend(kids.iter());
            }
        }
        if readiness.is_ready() || hidden.elapsed >= REVEAL_TIMEOUT_SECS {
            commands
                .entity(root)
                .try_insert(Visibility::Inherited)
                .try_remove::<HiddenUntilPosed>();
        }
    }
}

/// Cosmetic parts whose skins still point at their own copy of the skeleton.
type UnboundParts = (With<AvatarClothingPart>, Without<JointsRebound>);

fn rebind_part_joints(
    mut commands: Commands,
    parts: Query<(Entity, &ChildOf), UnboundParts>,
    skeletons: Query<&AvatarSkeleton>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut skins: Query<&mut SkinnedMesh>,
) {
    for (part, child_of) in parts.iter() {
        if children.get(part).is_err() {
            continue; // part scene still loading
        }
        let Some(skeleton) = children
            .get(child_of.parent())
            .ok()
            .and_then(|siblings| siblings.iter().find_map(|s| skeletons.get(s).ok()))
        else {
            continue; // body skeleton not bound yet
        };

        let mut stack = vec![part];
        while let Some(entity) = stack.pop() {
            if let Ok(mut skin) = skins.get_mut(entity) {
                let joint_names: Vec<Option<&str>> = skin
                    .joints
                    .iter()
                    .map(|j| names.get(*j).ok().map(Name::as_str))
                    .collect();
                match remap_joints(&joint_names, &skeleton.joints) {
                    Some(joints) => skin.joints = joints,
                    None => warn!("Part {part:?} has joints missing from the body skeleton"),
                }
            }
            if let Ok(kids) = children.get(entity) {
                stack.extend(kids.iter());
            }
        }
        commands.entity(part).try_insert(JointsRebound);
    }
}

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

fn drive_animators(
    library: Option<Res<AvatarAnimationLibrary>>,
    mut animators: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut AvatarAnimator,
    )>,
    sources: Query<(&AvatarPose, Option<&PathFollower>)>,
) {
    let Some(library) = library else { return };

    for (mut player, mut transitions, mut animator) in animators.iter_mut() {
        let Ok((pose, follower)) = sources.get(animator.pose_source) else {
            continue;
        };
        let clip = clip_name(*pose);
        let Some(&node) = library.nodes.get(clip) else {
            continue;
        };

        if animator.current != Some(clip) {
            let active = transitions.play(&mut player, node, TRANSITION);
            if library.clips.get(clip).is_none_or(|m| m.looping) {
                active.repeat();
            }
            animator.current = Some(clip);
        }

        if *pose == AvatarPose::Walk
            && let Some(follower) = follower
            && let Some(active) = player.animation_mut(node)
        {
            let native = library.clips.get(clip).and_then(|m| m.tiles_per_sec);
            active.set_speed(walk_playback_speed(follower.speed, native));
        }
    }
}

fn apply_posture_offset(
    library: Option<Res<AvatarAnimationLibrary>>,
    avatars: Query<(&AvatarPosture, &AvatarPose, &Children)>,
    mut visuals: Query<&mut Transform, With<AvatarVisualRoot>>,
) {
    let Some(library) = library else { return };
    for (posture, pose, children) in avatars.iter() {
        let contact = library
            .clips
            .get(clip_name(*pose))
            .map_or(0.0, |m| m.contact_height);
        let offset = if *pose == AvatarPose::Walk {
            0.0
        } else {
            posture_offset(posture.support_height, contact)
        };
        for child in children.iter() {
            if let Ok(mut transform) = visuals.get_mut(child)
                && transform.translation.y != offset
            {
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
    use bevy::ecs::system::RunSystemOnce;
    use std::collections::HashSet;

    /// Clip table with the shape of the one the CDN delivers (the real one lives in resort-game-assets,
    /// where `test_assets_consistency.py` checks it against the animation library).
    const CLIPS_FIXTURE: &str = include_str!("../tests/fixtures/clips.json");

    fn reveal_app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, reveal_posed_avatars);
        app
    }

    fn spawn_hidden_avatar(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((Visibility::Hidden, HiddenUntilPosed::default()))
            .id()
    }

    fn visibility(app: &App, entity: Entity) -> Visibility {
        *app.world().get::<Visibility>(entity).unwrap()
    }

    fn animator(current: Option<&'static str>) -> AvatarAnimator {
        AvatarAnimator {
            pose_source: Entity::PLACEHOLDER,
            current,
        }
    }

    #[test]
    fn avatar_stays_hidden_until_the_skeleton_animates() {
        let mut app = reveal_app();
        let root = spawn_hidden_avatar(&mut app);
        let body = app.world_mut().spawn((AvatarBodyPart, ChildOf(root))).id();
        let rig = app.world_mut().spawn((animator(None), ChildOf(body))).id();

        app.update();
        assert_eq!(visibility(&app, root), Visibility::Hidden);

        app.world_mut()
            .entity_mut(rig)
            .insert(animator(Some("idle")));
        app.update();
        assert_eq!(visibility(&app, root), Visibility::Inherited);
        assert!(app.world().get::<HiddenUntilPosed>(root).is_none());
    }

    #[test]
    fn avatar_waits_for_every_part_to_be_rebound() {
        let mut app = reveal_app();
        let root = spawn_hidden_avatar(&mut app);
        app.world_mut()
            .spawn((animator(Some("idle")), ChildOf(root)));
        let part = app
            .world_mut()
            .spawn((AvatarClothingPart, ChildOf(root)))
            .id();

        app.update();
        assert_eq!(visibility(&app, root), Visibility::Hidden);

        app.world_mut().entity_mut(part).insert(JointsRebound);
        app.update();
        assert_eq!(visibility(&app, root), Visibility::Inherited);
    }

    #[test]
    fn avatar_waits_for_the_color_to_be_applied() {
        let mut app = reveal_app();
        let root = spawn_hidden_avatar(&mut app);
        app.world_mut()
            .spawn((animator(Some("idle")), ChildOf(root)));
        let part = app
            .world_mut()
            .spawn((
                AvatarClothingPart,
                JointsRebound,
                crate::avatar::AvatarPartTint {
                    color: Color::WHITE,
                },
                ChildOf(root),
            ))
            .id();

        app.update();
        assert_eq!(visibility(&app, root), Visibility::Hidden);

        app.world_mut()
            .entity_mut(part)
            .insert(crate::avatar::PartTintApplied);
        app.update();
        assert_eq!(visibility(&app, root), Visibility::Inherited);
    }

    #[test]
    fn avatar_is_shown_anyway_after_the_timeout() {
        let mut app = reveal_app();
        let root = spawn_hidden_avatar(&mut app);
        app.update();
        assert_eq!(visibility(&app, root), Visibility::Hidden);

        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(
                REVEAL_TIMEOUT_SECS + 0.1,
            ));
        app.update();
        assert_eq!(visibility(&app, root), Visibility::Inherited);
    }

    #[test]
    fn debug_cycle_visits_every_posture_and_returns_to_stand() {
        let mut kind = PostureKind::Stand;
        let mut seen = Vec::new();
        for _ in 0..3 {
            kind = next_debug_posture(kind);
            seen.push(kind);
        }
        assert_eq!(kind, PostureKind::Stand);
        assert!(seen.contains(&PostureKind::Sit));
        assert!(seen.contains(&PostureKind::Lay));
    }

    #[test]
    fn server_posture_maps_to_client_posture() {
        use protocol::Posture;
        assert_eq!(
            AvatarPosture::from_server(Posture::Stand, 0.8),
            AvatarPosture::default()
        );
        assert_eq!(
            AvatarPosture::from_server(Posture::Sit, 0.8),
            AvatarPosture {
                kind: PostureKind::Sit,
                support_height: Some(0.8)
            }
        );
        assert_eq!(
            AvatarPosture::from_server(Posture::Lay, 0.5).kind,
            PostureKind::Lay
        );
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
    fn each_pose_has_its_own_clip_name() {
        let names: HashSet<&str> = [
            AvatarPose::Idle,
            AvatarPose::Walk,
            AvatarPose::Sit,
            AvatarPose::Lay,
        ]
        .into_iter()
        .map(clip_name)
        .collect();
        assert_eq!(names.len(), 4);
        assert_eq!(clip_name(AvatarPose::Walk), "walk");
    }

    #[test]
    fn every_pose_has_a_clip_in_clips_json() {
        let clips = parse_clips(CLIPS_FIXTURE).expect("clips fixture parses");
        for pose in [
            AvatarPose::Idle,
            AvatarPose::Walk,
            AvatarPose::Sit,
            AvatarPose::Lay,
        ] {
            assert!(clips.contains_key(clip_name(pose)), "{pose:?}");
        }
        assert!(clips["walk"].tiles_per_sec.is_some());
        assert!(clips["walk"].looping);
        // A seated avatar rests on the seat; a lying one on the bed surface.
        assert!(clips["sit"].contact_height > 0.1);
        assert_eq!(clips["lay"].contact_height, 0.0);
    }

    #[test]
    fn walk_speed_matches_path_speed() {
        assert!((walk_playback_speed(2.0, Some(1.0)) - 2.0).abs() < 1e-6);
        assert_eq!(walk_playback_speed(100.0, Some(1.0)), MAX_WALK_SPEED);
        assert_eq!(walk_playback_speed(0.01, Some(1.0)), MIN_WALK_SPEED);
        assert_eq!(walk_playback_speed(2.0, None), 1.0);
        assert_eq!(walk_playback_speed(0.0, Some(1.0)), 1.0);
    }

    #[test]
    fn posture_offset_puts_contact_on_support() {
        assert_eq!(posture_offset(None, 0.39), 0.0);
        assert!((posture_offset(Some(0.45), 0.39) - 0.06).abs() < 1e-6);
        assert!((posture_offset(Some(0.5), 0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn remap_joints_by_name() {
        let a = Entity::from_raw_u32(1).unwrap();
        let b = Entity::from_raw_u32(2).unwrap();
        let body: HashMap<String, Entity> =
            [("Spine".to_string(), a), ("Head".to_string(), b)].into();

        assert_eq!(
            remap_joints(&[Some("Head"), Some("Spine")], &body),
            Some(vec![b, a])
        );
        assert_eq!(remap_joints(&[Some("Missing")], &body), None);
        assert_eq!(remap_joints(&[None], &body), None);
        assert_eq!(remap_joints(&[], &body), Some(vec![]));
    }

    #[test]
    fn rig_paths_start_at_rig_root() {
        let mut world = World::new();
        let rig = world.spawn(Name::new(RIG_ROOT_NAME)).id();
        let arm = world.spawn((Name::new("LeftArm"), ChildOf(rig))).id();
        let forearm = world.spawn((Name::new("LeftForeArm"), ChildOf(arm))).id();

        let paths = world
            .run_system_once(move |children: Query<&Children>, names: Query<&Name>| {
                collect_rig_paths(rig, &children, &names)
            })
            .unwrap();
        let by_entity: HashMap<Entity, Vec<Name>> = paths.into_iter().collect();

        assert_eq!(
            by_entity[&forearm],
            vec![
                Name::new(RIG_ROOT_NAME),
                Name::new("LeftArm"),
                Name::new("LeftForeArm"),
            ]
        );
        assert_eq!(
            target_id(&by_entity[&forearm]),
            AnimationTargetId::from_names(
                [
                    Name::new(RIG_ROOT_NAME),
                    Name::new("LeftArm"),
                    Name::new("LeftForeArm")
                ]
                .iter()
            )
        );
        assert_eq!(by_entity[&rig], vec![Name::new(RIG_ROOT_NAME)]);
    }

    #[test]
    fn pose_source_is_nearest_ancestor_with_pose() {
        let mut world = World::new();
        let avatar = world.spawn(AvatarPose::Idle).id();
        let visual = world.spawn(ChildOf(avatar)).id();
        let body = world.spawn(ChildOf(visual)).id();
        let orphan = world.spawn_empty().id();

        let (found, missing) = world
            .run_system_once(move |parents: Query<&ChildOf>, poses: Query<&AvatarPose>| {
                (
                    find_pose_source(body, &parents, &poses),
                    find_pose_source(orphan, &parents, &poses),
                )
            })
            .unwrap();
        assert_eq!(found, Some(avatar));
        assert_eq!(missing, None);
    }
}
