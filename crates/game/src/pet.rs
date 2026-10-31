//! A decorative pet (Mochi, a shih-tzu) that wanders the lobby on its own.
//!
//! The pet lives only in the client: there is no server message, no collision with players and no
//! persistence, so it never changes the room's state. It picks a random action (stand, sit, lay,
//! stroll, trot, bark, pant), walks between free tiles by a breadth-first path and plays the clips of
//! `animals/mochi/anims.glb`. Assets come from the CDN like any other model (lazy tier):
//!
//! * `animals/mochi/clips.json`: duration, loop and ground speed (m/s, in `tiles_per_sec`) of each clip;
//! * `animals/mochi/anims.glb`: the armature and the actions only, bound to whichever LOD is shown;
//! * `animals/mochi/lod{0,1,2}.glb`: the skinned meshes (high, mid and low detail, see `toon_avatar::Quality`).
//!
//! The model is a plain glTF with standard materials, so the scene style of `toon_avatar` turns it
//! into the same toon look as the room and the furniture. `GAME_PET=off` hides the pet.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

use bevy::{
    animation::{AnimatedBy, AnimationTargetId, graph::AnimationNodeIndex},
    asset::LoadState,
    gltf::{Gltf, GltfAssetLabel},
    prelude::*,
};
use toon_avatar::{
    ClipMeta, DeviceTier, LodConfig, Quality, SizeClass, desired_level, parse_clips, size_class,
};

use crate::cdn_assets::{JsonText, JsonTextLoader};
use crate::furniture::calculate_occupied_tiles;
use crate::net::NetRoomSnapshotEvent;
use crate::room::{FLOOR_TILE_TOP, tile_to_world};

const PET_DIR: &str = "animals/mochi";
const CLIPS_FILE: &str = "animals/mochi/clips.json";
const ANIMS_FILE: &str = "animals/mochi/anims.glb";
/// The room the pet lives in.
const PET_ROOM: &str = "lobby";
/// Height of the pet in world units (the avatars are 1.8).
const PET_HEIGHT: f32 = 0.5;
/// Name of the skeleton root in the animation file (the paths of its targets start with it).
const RIG_ROOT_NAME: &str = "Armature";
const TRANSITION: Duration = Duration::from_millis(300);
/// Ground speed of the two gaits, in tiles per second.
const STROLL_TPS: f32 = 0.35;
const TROT_TPS: f32 = 0.85;
const MIN_PLAYBACK: f32 = 0.5;
const MAX_PLAYBACK: f32 = 3.0;
/// How far (in tiles, manhattan) the pet goes on one walk.
const MAX_WALK_TILES: i32 = 6;
const TURN_RATE: f32 = 10.0;
const LOD_MIN_SWITCH_SECS: f32 = 1.0;

/// What the pet is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetAction {
    Idle,
    Sit,
    Lay,
    Stroll,
    Trot,
    Bark,
    Pant,
}

impl PetAction {
    pub fn clip(self) -> &'static str {
        match self {
            PetAction::Idle => "idle",
            PetAction::Sit => "sit",
            PetAction::Lay => "lay",
            PetAction::Stroll => "walk",
            PetAction::Trot => "run",
            PetAction::Bark => "bark",
            PetAction::Pant => "pant",
        }
    }

    pub fn moves(self) -> bool {
        matches!(self, PetAction::Stroll | PetAction::Trot)
    }

    /// Ground speed in tiles per second for the actions that move.
    fn speed_tps(self) -> f32 {
        match self {
            PetAction::Stroll => STROLL_TPS,
            PetAction::Trot => TROT_TPS,
            _ => 0.0,
        }
    }
}

/// Small deterministic generator (xorshift64*), enough for choosing what to do next.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// A value in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let bits = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
        bits as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// The weights of what the pet does after `current`; a standing pet moves around more, a seated or lying one rests.
fn weights(current: PetAction) -> &'static [(PetAction, f32)] {
    use PetAction::*;
    match current {
        Idle => &[
            (Stroll, 30.0),
            (Trot, 15.0),
            (Sit, 20.0),
            (Lay, 10.0),
            (Bark, 10.0),
            (Pant, 15.0),
        ],
        Sit => &[(Idle, 45.0), (Lay, 20.0), (Pant, 20.0), (Bark, 15.0)],
        Lay => &[(Sit, 50.0), (Idle, 50.0)],
        Stroll | Trot => &[(Idle, 45.0), (Sit, 25.0), (Pant, 20.0), (Bark, 10.0)],
        Bark | Pant => &[(Idle, 45.0), (Sit, 30.0), (Stroll, 25.0)],
    }
}

/// The action after `current`, from a roll in `[0, 1)`.
pub fn choose_action(current: PetAction, roll: f32) -> PetAction {
    let table = weights(current);
    let total: f32 = table.iter().map(|(_, w)| w).sum();
    let mut left = roll.clamp(0.0, 0.999_999) * total;
    for (action, weight) in table {
        if left < *weight {
            return *action;
        }
        left -= weight;
    }
    table[table.len() - 1].0
}

/// Seconds the pet stays in `action` (the moving actions last until the walk ends).
pub fn action_duration(action: PetAction, rng: &mut Rng) -> f32 {
    match action {
        PetAction::Idle => rng.range(2.0, 5.0),
        PetAction::Sit => rng.range(4.0, 9.0),
        PetAction::Lay => rng.range(5.0, 12.0),
        PetAction::Bark => rng.range(1.6, 2.6),
        PetAction::Pant => rng.range(3.0, 5.0),
        PetAction::Stroll | PetAction::Trot => f32::INFINITY,
    }
}

/// Shortest path between two free tiles (4-neighbour), `from` excluded and `to` included.
pub fn find_path(
    free: &HashSet<(i32, i32)>,
    from: (i32, i32),
    to: (i32, i32),
) -> Option<Vec<(i32, i32)>> {
    if from == to || !free.contains(&to) {
        return None;
    }
    let mut came: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut queue = VecDeque::from([from]);
    while let Some(tile) = queue.pop_front() {
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let next = (tile.0 + dx, tile.1 + dz);
            if next == from || came.contains_key(&next) || !free.contains(&next) {
                continue;
            }
            came.insert(next, tile);
            if next == to {
                let mut path = vec![to];
                let mut at = to;
                while let Some(&prev) = came.get(&at) {
                    if prev == from {
                        break;
                    }
                    path.push(prev);
                    at = prev;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(next);
        }
    }
    None
}

/// A random tile near `from` that can be reached over free tiles.
pub fn pick_walk(
    free: &HashSet<(i32, i32)>,
    from: (i32, i32),
    rng: &mut Rng,
) -> Option<Vec<(i32, i32)>> {
    for _ in 0..12 {
        let dx = (rng.range(-1.0, 1.0) * MAX_WALK_TILES as f32).round() as i32;
        let dz = (rng.range(-1.0, 1.0) * MAX_WALK_TILES as f32).round() as i32;
        if let Some(path) = find_path(free, from, (from.0 + dx, from.1 + dz)) {
            return Some(path);
        }
    }
    None
}

/// Playback speed of a gait clip that keeps the feet on the ground: wanted ground speed over the
/// clip's own (`native_mps` in model metres per second, `scale` model to world).
pub fn playback_speed(wanted_tps: f32, native_mps: Option<f32>, scale: f32) -> f32 {
    match native_mps {
        Some(native) if native > 0.0 && scale > 0.0 => {
            (wanted_tps / (native * scale)).clamp(MIN_PLAYBACK, MAX_PLAYBACK)
        }
        _ => 1.0,
    }
}

/// The model file of a level of detail.
pub fn lod_model(level: Quality) -> String {
    let n = match level {
        Quality::High => 0,
        Quality::Mid => 1,
        Quality::Low => 2,
    };
    format!("{PET_DIR}/lod{n}.glb")
}

/// The tiles the pet may use: floor tiles without a solid item on them.
pub fn free_tiles(snapshot: &protocol::RoomSnapshot) -> HashSet<(i32, i32)> {
    let mut free = HashSet::new();
    for (z, row) in snapshot.layout.heightmap.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch != 'x' && ch != 'X' {
                free.insert((x as i32, z as i32));
            }
        }
    }
    for item in snapshot.items.iter().filter(|i| !i.walkable) {
        for tile in calculate_occupied_tiles(item.x, item.z, item.width, item.depth) {
            free.remove(&tile);
        }
    }
    free
}

// ---------------------------------------------------------------------------
// Bevy side
// ---------------------------------------------------------------------------

/// The pet: its mind, and where it is going.
#[derive(Component)]
pub struct Pet {
    pub action: PetAction,
    time_left: f32,
    rng: Rng,
    tile: (i32, i32),
    route: VecDeque<Vec3>,
    scene: Entity,
    level: Quality,
    class: SizeClass,
    since_switch: f32,
}

/// The glTF scene of one level of detail.
#[derive(Component)]
struct PetScene;

/// The skeleton of a scene already has its animation targets.
#[derive(Component)]
struct RigBound;

/// Lives next to the [`AnimationPlayer`] on the armature node.
#[derive(Component)]
struct PetRig {
    pet: Entity,
    current: Option<PetAction>,
}

enum PetAssets {
    /// Nothing asked for yet (the pet's room has not been entered).
    Idle,
    Loading {
        clips: Handle<JsonText>,
        anims: Handle<Gltf>,
    },
    Ready,
    Disabled,
}

#[derive(Resource)]
struct PetState {
    assets: PetAssets,
    free: HashSet<(i32, i32)>,
    in_room: bool,
    clips: HashMap<String, ClipMeta>,
    /// Model to world scale (the clips file carries the model height).
    scale: f32,
    graph: Option<Handle<AnimationGraph>>,
    nodes: HashMap<String, AnimationNodeIndex>,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            assets: PetAssets::Idle,
            free: HashSet::new(),
            in_room: false,
            clips: HashMap::new(),
            scale: 1.0,
            graph: None,
            nodes: HashMap::new(),
        }
    }
}

pub struct PetPlugin;

impl Plugin for PetPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<Assets<JsonText>>().is_none() {
            app.init_asset::<JsonText>()
                .register_asset_loader(JsonTextLoader);
        }
        if std::env::var("GAME_PET").is_ok_and(|v| v == "off") {
            return;
        }
        app.init_resource::<PetState>().add_systems(
            Update,
            (
                track_room,
                load_assets,
                spawn_pet,
                think,
                walk,
                update_lod,
                bind_rig,
                drive_animation,
            )
                .chain(),
        );
    }
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0x9E37_79B9, |d| d.as_nanos() as u64)
}

/// Follows the room: the pet belongs to the lobby only, and is rebuilt on each entry.
fn track_room(
    mut commands: Commands,
    mut snapshots: MessageReader<NetRoomSnapshotEvent>,
    mut state: ResMut<PetState>,
    pets: Query<Entity, With<Pet>>,
) {
    for event in snapshots.read() {
        for pet in &pets {
            commands.entity(pet).despawn();
        }
        let lobby = event.0.room_id == PET_ROOM;
        state.in_room = lobby;
        state.free = if lobby {
            free_tiles(&event.0)
        } else {
            HashSet::new()
        };
    }
}

/// Starts loading the pet's files the first time the lobby is entered, and builds the animation graph.
fn load_assets(
    mut state: ResMut<PetState>,
    assets: Res<AssetServer>,
    texts: Res<Assets<JsonText>>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    match &state.assets {
        PetAssets::Idle if state.in_room => {
            state.assets = PetAssets::Loading {
                clips: assets.load(CLIPS_FILE),
                anims: assets.load(ANIMS_FILE),
            };
        }
        PetAssets::Loading { clips, anims } => {
            if matches!(assets.load_state(clips), LoadState::Failed(_))
                || matches!(assets.load_state(anims), LoadState::Failed(_))
            {
                warn!("the pet's files are not available, the lobby stays without it");
                state.assets = PetAssets::Disabled;
                return;
            }
            let (Some(text), Some(gltf)) = (texts.get(clips), gltfs.get(anims)) else {
                return;
            };
            let parsed = match parse_clips(&text.0) {
                Ok(parsed) => parsed,
                Err(error) => {
                    warn!("invalid pet clips: {error}");
                    state.assets = PetAssets::Disabled;
                    return;
                }
            };
            let height = serde_json::from_str::<serde_json::Value>(&text.0)
                .ok()
                .and_then(|v| v["model_height_m"].as_f64())
                .filter(|h| *h > 0.0)
                .unwrap_or(0.4) as f32;
            let mut graph = AnimationGraph::new();
            let root = graph.root;
            let mut nodes = HashMap::new();
            for name in parsed.keys() {
                match gltf.named_animations.get(name.as_str()) {
                    Some(clip) => {
                        nodes.insert(name.clone(), graph.add_clip(clip.clone(), 1.0, root));
                    }
                    None => warn!("pet clip '{name}' missing from {ANIMS_FILE}"),
                }
            }
            state.clips = parsed;
            state.scale = PET_HEIGHT / height;
            state.graph = Some(graphs.add(graph));
            state.nodes = nodes;
            state.assets = PetAssets::Ready;
            info!("pet data loaded");
        }
        _ => {}
    }
}

/// Puts the pet on a free tile of the lobby once its files are in.
fn spawn_pet(
    mut commands: Commands,
    state: Res<PetState>,
    assets: Res<AssetServer>,
    tier: Option<Res<DeviceTier>>,
    pets: Query<(), With<Pet>>,
) {
    if !state.in_room
        || !matches!(state.assets, PetAssets::Ready)
        || !pets.is_empty()
        || state.free.is_empty()
    {
        return;
    }
    let mut rng = Rng::new(seed());
    let mut tiles: Vec<_> = state.free.iter().copied().collect();
    tiles.sort_unstable();
    let tile = tiles[(rng.next_f32() * tiles.len() as f32) as usize % tiles.len()];
    let level = tier.map_or(Quality::Mid, |t| match *t {
        DeviceTier::High => Quality::Mid,
        DeviceTier::Mid => Quality::Mid,
        DeviceTier::Low => Quality::Low,
    });
    let yaw = rng.range(0.0, std::f32::consts::TAU);
    let root = commands
        .spawn((
            Transform::from_translation(tile_to_world(tile.0, tile.1))
                .with_rotation(Quat::from_rotation_y(yaw)),
            Visibility::default(),
        ))
        .id();
    // The model faces +Z while a transform faces -Z, like the avatars' scenes.
    let visual = commands
        .spawn((
            Transform::from_scale(Vec3::splat(state.scale))
                .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
            Visibility::default(),
            ChildOf(root),
        ))
        .id();
    let scene = commands
        .spawn((
            PetScene,
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(lod_model(level)))),
            Transform::default(),
            Visibility::default(),
            ChildOf(visual),
        ))
        .id();
    commands.entity(root).insert(Pet {
        action: PetAction::Idle,
        time_left: action_duration(PetAction::Idle, &mut rng),
        rng,
        tile,
        route: VecDeque::new(),
        scene,
        level,
        class: SizeClass::Medium,
        since_switch: 0.0,
    });
    info!(
        "pet spawned at tile {tile:?}, level {}",
        match level {
            Quality::High => "high",
            Quality::Mid => "mid",
            Quality::Low => "low",
        }
    );
}

/// Decides what to do next when the current action is over.
fn think(time: Res<Time>, state: Res<PetState>, mut pets: Query<&mut Pet>) {
    let dt = time.delta_secs();
    for mut pet in &mut pets {
        let ended = if pet.action.moves() {
            pet.route.is_empty()
        } else {
            pet.time_left -= dt;
            pet.time_left <= 0.0
        };
        if !ended {
            continue;
        }
        let roll = pet.rng.next_f32();
        let mut next = choose_action(pet.action, roll);
        if next.moves() {
            let from = pet.tile;
            match pick_walk(&state.free, from, &mut pet.rng) {
                Some(path) => {
                    pet.route = path.into_iter().map(|(x, z)| tile_to_world(x, z)).collect();
                }
                None => next = PetAction::Idle,
            }
        }
        pet.time_left = action_duration(next, &mut pet.rng);
        pet.action = next;
    }
}

/// Moves a walking pet along its route and turns it towards where it goes.
fn walk(time: Res<Time>, mut pets: Query<(&mut Pet, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut pet, mut transform) in &mut pets {
        let speed = pet.action.speed_tps();
        if speed <= 0.0 {
            continue;
        }
        let mut budget = speed * dt;
        while budget > 0.0 {
            let Some(&target) = pet.route.front() else {
                break;
            };
            let to = Vec3::new(
                target.x - transform.translation.x,
                0.0,
                target.z - transform.translation.z,
            );
            let distance = to.length();
            if distance > 1e-4 {
                let facing = Transform::default()
                    .looking_to(to / distance, Vec3::Y)
                    .rotation;
                transform.rotation = transform
                    .rotation
                    .slerp(facing, 1.0 - (-TURN_RATE * dt).exp());
            }
            if distance <= budget {
                transform.translation = Vec3::new(target.x, FLOOR_TILE_TOP, target.z);
                budget -= distance;
                pet.route.pop_front();
                pet.tile = (target.x.floor() as i32, target.z.floor() as i32);
            } else {
                transform.translation += to / distance * budget;
                budget = 0.0;
            }
        }
    }
}

/// Swaps the model for a lighter or heavier one by the size of the pet on the screen and the device tier.
fn update_lod(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AssetServer>,
    tier: Option<Res<DeviceTier>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut pets: Query<(Entity, &GlobalTransform, &mut Pet)>,
    visuals: Query<&ChildOf, With<PetScene>>,
) {
    let (Some(tier), Some((camera, camera_transform))) = (
        tier,
        cameras
            .iter()
            .find(|(c, _)| c.is_active && c.logical_viewport_size().is_some()),
    ) else {
        return;
    };
    // The pet is small: a dog of 60 px is as sharp as it needs to be.
    let cfg = LodConfig {
        near_px: 140.0,
        far_px: 55.0,
        ..LodConfig::default()
    };
    for (_, global, mut pet) in &mut pets {
        pet.since_switch += time.delta_secs();
        let feet = global.translation();
        let (Ok(a), Ok(b)) = (
            camera.world_to_viewport(camera_transform, feet),
            camera.world_to_viewport(camera_transform, feet + Vec3::Y * PET_HEIGHT),
        ) else {
            continue;
        };
        let class = size_class((a - b).length(), pet.class, &cfg);
        pet.class = class;
        let level = desired_level(*tier, false, class, true);
        if level == pet.level || pet.since_switch < LOD_MIN_SWITCH_SECS {
            continue;
        }
        let Ok(parent) = visuals.get(pet.scene) else {
            continue;
        };
        let visual = parent.parent();
        commands.entity(pet.scene).despawn();
        pet.scene = commands
            .spawn((
                PetScene,
                WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(lod_model(level)))),
                Transform::default(),
                Visibility::default(),
                ChildOf(visual),
            ))
            .id();
        pet.level = level;
        pet.since_switch = 0.0;
    }
}

/// Depth-first `(entity, name path from the rig root)` of the armature's subtree; the root is called
/// [`RIG_ROOT_NAME`] in the paths, as in the animation file.
fn rig_paths(
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

/// Gives a freshly spawned scene its animation player and targets.
#[allow(clippy::type_complexity)]
fn bind_rig(
    mut commands: Commands,
    state: Res<PetState>,
    scenes: Query<(Entity, &ChildOf), (With<PetScene>, Without<RigBound>)>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Some(graph) = state.graph.clone() else {
        return;
    };
    for (scene, visual) in &scenes {
        let Some(rig) = children
            .iter_descendants(scene)
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "Armature"))
        else {
            continue;
        };
        let Ok(pet) = parents.get(visual.parent()) else {
            continue;
        };
        for (entity, path) in rig_paths(rig, &children, &names) {
            commands
                .entity(entity)
                .try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(rig)));
        }
        commands.entity(rig).try_insert((
            AnimationPlayer::default(),
            AnimationGraphHandle(graph.clone()),
            AnimationTransitions::new(),
            PetRig {
                pet: pet.parent(),
                current: None,
            },
        ));
        commands.entity(scene).try_insert(RigBound);
    }
}

/// Plays the clip of what the pet is doing.
fn drive_animation(
    state: Res<PetState>,
    pets: Query<&Pet>,
    mut rigs: Query<(&mut AnimationPlayer, &mut AnimationTransitions, &mut PetRig)>,
) {
    for (mut player, mut transitions, mut rig) in &mut rigs {
        let Ok(pet) = pets.get(rig.pet) else {
            continue;
        };
        if rig.current == Some(pet.action) {
            continue;
        }
        let clip = pet.action.clip();
        let Some(&node) = state.nodes.get(clip) else {
            continue;
        };
        let meta = state.clips.get(clip);
        let speed = if pet.action.moves() {
            playback_speed(
                pet.action.speed_tps(),
                meta.and_then(|m| m.tiles_per_sec),
                state.scale,
            )
        } else {
            1.0
        };
        let active = transitions.play(&mut player, node, TRANSITION);
        if meta.is_none_or(|m| m.looping) {
            active.repeat();
        }
        active.set_speed(speed);
        rig.current = Some(pet.action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_floor(w: i32, h: i32) -> HashSet<(i32, i32)> {
        (0..w)
            .flat_map(|x| (0..h).map(move |z| (x, z)))
            .collect::<HashSet<_>>()
    }

    #[test]
    fn the_generator_stays_in_the_unit_interval_and_moves() {
        let mut rng = Rng::new(42);
        let values: Vec<f32> = (0..200).map(|_| rng.next_f32()).collect();
        assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
        assert!(values.windows(2).any(|w| w[0] != w[1]));
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        assert!((0.35..0.65).contains(&mean), "mean {mean}");
    }

    #[test]
    fn a_roll_picks_the_weighted_action_and_the_ends_are_reachable() {
        assert_eq!(choose_action(PetAction::Idle, 0.0), PetAction::Stroll);
        assert_eq!(choose_action(PetAction::Idle, 0.999), PetAction::Pant);
        assert_eq!(choose_action(PetAction::Lay, 0.0), PetAction::Sit);
        assert_eq!(choose_action(PetAction::Lay, 0.9), PetAction::Idle);
    }

    #[test]
    fn every_action_leads_somewhere_and_a_resting_pet_stands_before_walking() {
        for action in [
            PetAction::Idle,
            PetAction::Sit,
            PetAction::Lay,
            PetAction::Stroll,
            PetAction::Trot,
            PetAction::Bark,
            PetAction::Pant,
        ] {
            assert!(!weights(action).is_empty());
        }
        for rest in [PetAction::Sit, PetAction::Lay] {
            assert!(weights(rest).iter().all(|(next, _)| !next.moves()));
        }
    }

    #[test]
    fn walking_actions_last_until_the_route_ends_and_the_others_a_few_seconds() {
        let mut rng = Rng::new(7);
        assert!(action_duration(PetAction::Stroll, &mut rng).is_infinite());
        for _ in 0..50 {
            let d = action_duration(PetAction::Sit, &mut rng);
            assert!((4.0..9.0).contains(&d));
        }
    }

    #[test]
    fn the_path_goes_around_a_wall_and_ends_on_the_target() {
        let mut free = open_floor(5, 3);
        // A wall at x = 2 with a gap at z = 2.
        free.remove(&(2, 0));
        free.remove(&(2, 1));
        let path = find_path(&free, (0, 0), (4, 0)).expect("reachable");
        assert_eq!(path.last(), Some(&(4, 0)));
        assert!(path.contains(&(2, 2)));
        assert!(!path.contains(&(0, 0)));
        // Each step is one tile.
        let mut at = (0, 0);
        for step in &path {
            assert_eq!((step.0 - at.0).abs() + (step.1 - at.1).abs(), 1);
            at = *step;
        }
    }

    #[test]
    fn no_path_to_a_blocked_or_cut_off_tile() {
        let mut free = open_floor(5, 1);
        free.remove(&(2, 0));
        assert_eq!(find_path(&free, (0, 0), (4, 0)), None);
        assert_eq!(find_path(&free, (0, 0), (2, 0)), None);
        assert_eq!(find_path(&free, (0, 0), (0, 0)), None);
    }

    #[test]
    fn a_walk_is_picked_over_reachable_tiles_only() {
        let free = open_floor(8, 8);
        let mut rng = Rng::new(3);
        for _ in 0..30 {
            if let Some(path) = pick_walk(&free, (4, 4), &mut rng) {
                assert!(path.iter().all(|t| free.contains(t)));
            }
        }
        let alone: HashSet<_> = [(0, 0)].into();
        assert_eq!(pick_walk(&alone, (0, 0), &mut rng), None);
    }

    #[test]
    fn the_gait_is_sped_up_to_the_ground_speed_within_limits() {
        // 0.2 m/s in the model, scaled x1.25: 0.25 tiles/s native.
        assert!((playback_speed(0.5, Some(0.2), 1.25) - 2.0).abs() < 1e-5);
        assert_eq!(playback_speed(0.01, Some(0.2), 1.25), MIN_PLAYBACK);
        assert_eq!(playback_speed(9.0, Some(0.2), 1.25), MAX_PLAYBACK);
        assert_eq!(playback_speed(0.5, None, 1.25), 1.0);
    }

    #[test]
    fn each_level_of_detail_has_its_own_model() {
        assert_eq!(lod_model(Quality::High), "animals/mochi/lod0.glb");
        assert_eq!(lod_model(Quality::Mid), "animals/mochi/lod1.glb");
        assert_eq!(lod_model(Quality::Low), "animals/mochi/lod2.glb");
    }

    #[test]
    fn free_tiles_leave_out_the_void_and_solid_items_but_not_rugs() {
        let snapshot = protocol::RoomSnapshot {
            room_id: "lobby".into(),
            room_name: "Lobby".into(),
            owner_id: String::new(),
            layout: protocol::LayoutSnapshot {
                id: "l".into(),
                width: 3,
                height: 2,
                door: [0, 0],
                heightmap: vec!["000".into(), "x00".into()],
            },
            avatars: vec![],
            items: vec![item(1, 0, 1, 1, false), item(2, 1, 1, 1, true)],
        };
        let free = free_tiles(&snapshot);
        assert!(!free.contains(&(0, 1)), "void");
        assert!(!free.contains(&(1, 0)), "sofa");
        assert!(free.contains(&(2, 1)), "rug");
        assert_eq!(free.len(), 4);
    }

    fn item(x: i32, z: i32, w: i32, d: i32, walkable: bool) -> protocol::PlacedItemSnapshot {
        protocol::PlacedItemSnapshot {
            id: format!("i{x}{z}"),
            catalog_id: String::new(),
            owner_id: String::new(),
            x,
            z,
            y: 0.0,
            rotation: 0,
            width: w,
            depth: d,
            walkable,
            color: String::new(),
            asset_id: String::new(),
        }
    }
}
