use std::collections::{HashMap, VecDeque};

use bevy::{
    animation::graph::AnimationNodeIndex,
    gltf::{Gltf, GltfAssetLabel},
    prelude::*,
};
use toon_avatar::{ClipMeta, DeviceTier, Quality, SizeClass};

use crate::action::AnimalAction;

/// Folder of the animals' files in the asset source.
pub const ASSET_ROOT: &str = "animals";
const TURN_RATE: f32 = 10.0;

/// An animal: add this to an entity (with a `Transform`) and the plugin draws it once its species is
/// configured. Its action, route and facing are set from outside.
#[derive(Component, Clone, Debug, PartialEq)]
#[require(AnimalAction, AnimalFacing, Transform, Visibility)]
pub struct Animal {
    pub species: String,
    pub breed: String,
    /// The skin of the breed; only `default` (the texture baked into the model) exists so far.
    pub coat: String,
    /// Height of the animal in world units.
    pub height: f32,
}

impl Animal {
    pub fn new(species: &str, breed: &str, coat: &str, height: f32) -> Self {
        Self {
            species: species.into(),
            breed: breed.into(),
            coat: coat.into(),
            height,
        }
    }
}

/// Where the animal looks while it stands still (a walking animal turns towards where it goes).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AnimalFacing(pub Quat);

/// A walk: the world positions to go through, in order, at `tiles_per_sec`. The plugin moves and turns the
/// animal along it and removes the component on arrival.
#[derive(Component, Clone, Debug)]
pub struct AnimalRoute {
    pub waypoints: VecDeque<Vec3>,
    pub tiles_per_sec: f32,
}

/// The animations of one species (the app reads them from `clips.json`).
#[derive(Clone, Debug)]
pub struct SpeciesAnimations {
    /// Asset path of the rig-only glTF holding the clips.
    pub gltf_path: String,
    pub clips: HashMap<String, ClipMeta>,
    /// Height of the model in metres, to scale it to the animal's `height`.
    pub model_height_m: f32,
}

/// The species the app has described. Animals of a species not in here yet are not drawn.
#[derive(Resource, Default, Clone, Debug)]
pub struct AnimalConfig {
    pub species: HashMap<String, SpeciesAnimations>,
}

/// The model file of a breed at a level of detail: `animals/<species>/<breed>/lod<n>.glb`.
pub fn lod_model(species: &str, breed: &str, level: Quality) -> String {
    let n = match level {
        Quality::High => 0,
        Quality::Mid => 1,
        Quality::Low => 2,
    };
    format!("{ASSET_ROOT}/{species}/{breed}/lod{n}.glb")
}

/// Playback speed of a gait clip that keeps the feet on the ground: the wanted ground speed over the clip's own
/// (`native_mps` in model metres per second, `scale` model to world).
pub fn playback_speed(wanted_tps: f32, native_mps: Option<f32>, scale: f32) -> f32 {
    match native_mps {
        Some(native) if native > 0.0 && scale > 0.0 => {
            (wanted_tps / (native * scale)).clamp(MIN_PLAYBACK, MAX_PLAYBACK)
        }
        _ => 1.0,
    }
}

const MIN_PLAYBACK: f32 = 0.5;
const MAX_PLAYBACK: f32 = 3.0;

/// The loaded animation graph of a species.
pub(crate) struct Library {
    pub gltf: Handle<Gltf>,
    pub graph: Option<Handle<AnimationGraph>>,
    pub nodes: HashMap<String, AnimationNodeIndex>,
}

#[derive(Resource, Default)]
pub(crate) struct Libraries(pub HashMap<String, Library>);

/// What an animal shows now: its model entity and level of detail.
#[derive(Component)]
pub(crate) struct AnimalVisual {
    pub scene: Entity,
    pub level: Quality,
    pub class: SizeClass,
    pub since_switch: f32,
}

/// The glTF scene of one level of detail.
#[derive(Component)]
pub(crate) struct AnimalScene;

pub(crate) fn build_graphs(
    config: Res<AnimalConfig>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut libraries: ResMut<Libraries>,
) {
    for (species, data) in &config.species {
        let library = libraries
            .0
            .entry(species.clone())
            .or_insert_with(|| Library {
                gltf: assets.load(&data.gltf_path),
                graph: None,
                nodes: HashMap::new(),
            });
        if library.graph.is_some() {
            continue;
        }
        let Some(gltf) = gltfs.get(&library.gltf) else {
            continue;
        };
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        for name in data.clips.keys() {
            match gltf.named_animations.get(name.as_str()) {
                Some(clip) => {
                    library
                        .nodes
                        .insert(name.clone(), graph.add_clip(clip.clone(), 1.0, root));
                }
                None => warn!("animation clip '{name}' missing from {}", data.gltf_path),
            }
        }
        library.graph = Some(graphs.add(graph));
    }
}

/// Gives an animal its model once its species' animations are ready.
pub(crate) fn attach_visuals(
    mut commands: Commands,
    assets: Res<AssetServer>,
    config: Res<AnimalConfig>,
    libraries: Res<Libraries>,
    tier: Option<Res<DeviceTier>>,
    animals: Query<(Entity, &Animal), Without<AnimalVisual>>,
) {
    for (entity, animal) in &animals {
        let (Some(data), Some(library)) = (
            config.species.get(&animal.species),
            libraries.0.get(&animal.species),
        ) else {
            continue;
        };
        if library.graph.is_none() {
            continue;
        }
        let level = match tier.as_deref() {
            Some(DeviceTier::Low) => Quality::Low,
            _ => Quality::Mid,
        };
        let scale = animal.height / data.model_height_m.max(0.01);
        // The model faces +Z while a transform faces -Z, like the avatars' scenes.
        let model = commands
            .spawn((
                Transform::from_scale(Vec3::splat(scale))
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                Visibility::default(),
                ChildOf(entity),
            ))
            .id();
        let scene = spawn_scene(&mut commands, &assets, animal, level, model);
        commands.entity(entity).insert(AnimalVisual {
            scene,
            level,
            class: SizeClass::Medium,
            since_switch: 0.0,
        });
    }
}

pub(crate) fn spawn_scene(
    commands: &mut Commands,
    assets: &AssetServer,
    animal: &Animal,
    level: Quality,
    model: Entity,
) -> Entity {
    commands
        .spawn((
            AnimalScene,
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(lod_model(
                &animal.species,
                &animal.breed,
                level,
            )))),
            Transform::default(),
            Visibility::default(),
            ChildOf(model),
        ))
        .id()
}

/// Moves a walking animal along its route and turns it towards where it goes.
pub(crate) fn follow_route(
    mut commands: Commands,
    time: Res<Time>,
    mut animals: Query<(Entity, &mut Transform, &mut AnimalRoute)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut route) in &mut animals {
        let mut budget = route.tiles_per_sec * dt;
        while budget > 0.0 {
            let Some(&target) = route.waypoints.front() else {
                break;
            };
            let flat = Vec3::new(
                target.x - transform.translation.x,
                0.0,
                target.z - transform.translation.z,
            );
            let distance = flat.length();
            if distance > 1e-4 {
                let facing = Transform::default()
                    .looking_to(flat / distance, Vec3::Y)
                    .rotation;
                transform.rotation = transform
                    .rotation
                    .slerp(facing, 1.0 - (-TURN_RATE * dt).exp());
            }
            if distance <= budget {
                transform.translation = target;
                budget -= distance;
                route.waypoints.pop_front();
            } else {
                transform.translation += flat / distance * budget;
                budget = 0.0;
            }
        }
        if route.waypoints.is_empty() {
            commands.entity(entity).remove::<AnimalRoute>();
        }
    }
}

/// Turns a standing animal towards its [`AnimalFacing`].
#[allow(clippy::type_complexity)]
pub(crate) fn turn_standing(
    time: Res<Time>,
    mut animals: Query<(&mut Transform, &AnimalFacing), (With<Animal>, Without<AnimalRoute>)>,
) {
    let t = 1.0 - (-TURN_RATE * time.delta_secs()).exp();
    for (mut transform, facing) in &mut animals {
        if transform.rotation.angle_between(facing.0) > 1e-3 {
            transform.rotation = transform.rotation.slerp(facing.0, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_level_of_detail_has_its_own_model_per_breed() {
        assert_eq!(
            lod_model("dog", "shihtzu", Quality::High),
            "animals/dog/shihtzu/lod0.glb"
        );
        assert_eq!(
            lod_model("dog", "shihtzu", Quality::Mid),
            "animals/dog/shihtzu/lod1.glb"
        );
        assert_eq!(
            lod_model("dog", "poodle", Quality::Low),
            "animals/dog/poodle/lod2.glb"
        );
    }

    #[test]
    fn the_gait_is_sped_up_to_the_ground_speed_within_limits() {
        // 0.2 m/s in the model, scaled x1.25: 0.25 tiles/s native.
        assert!((playback_speed(0.5, Some(0.2), 1.25) - 2.0).abs() < 1e-5);
        assert_eq!(playback_speed(0.01, Some(0.2), 1.25), MIN_PLAYBACK);
        assert_eq!(playback_speed(9.0, Some(0.2), 1.25), MAX_PLAYBACK);
        assert_eq!(playback_speed(0.5, None, 1.25), 1.0);
    }

    fn movement_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, (follow_route, turn_standing));
        app
    }

    #[test]
    fn a_route_is_walked_in_order_and_removed_on_arrival() {
        let mut app = movement_app();
        let animal = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.2, 0.0),
                AnimalRoute {
                    waypoints: VecDeque::from([Vec3::new(1.0, 0.2, 0.0), Vec3::new(1.0, 0.2, 1.0)]),
                    // 1000 tiles/s: the first frame is long enough to arrive.
                    tiles_per_sec: 1000.0,
                },
            ))
            .id();
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.update();
        let world = app.world();
        assert!(world.get::<AnimalRoute>(animal).is_none(), "the route ends");
        let at = world.get::<Transform>(animal).unwrap().translation;
        assert!(at.abs_diff_eq(Vec3::new(1.0, 0.2, 1.0), 1e-4), "{at:?}");
    }

    #[test]
    fn a_slow_walk_keeps_its_route_and_moves_part_of_the_way() {
        let mut app = movement_app();
        let animal = app
            .world_mut()
            .spawn((
                Transform::from_xyz(0.0, 0.2, 0.0),
                AnimalRoute {
                    waypoints: VecDeque::from([Vec3::new(10.0, 0.2, 0.0)]),
                    tiles_per_sec: 100.0,
                },
            ))
            .id();
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.update();
        let world = app.world();
        assert!(world.get::<AnimalRoute>(animal).is_some());
        let x = world.get::<Transform>(animal).unwrap().translation.x;
        assert!(x > 0.0 && x < 10.0, "x {x}");
    }

    #[test]
    fn a_standing_animal_turns_towards_its_facing() {
        let mut app = movement_app();
        let target = Quat::from_rotation_y(1.0);
        let animal = app
            .world_mut()
            .spawn((
                Animal::new("dog", "shihtzu", "default", 0.5),
                Transform::default(),
                AnimalFacing(target),
            ))
            .id();
        let before = Quat::IDENTITY.angle_between(target);
        for _ in 0..3 {
            std::thread::sleep(std::time::Duration::from_millis(16));
            app.update();
        }
        let after = app
            .world()
            .get::<Transform>(animal)
            .unwrap()
            .rotation
            .angle_between(target);
        assert!(after < before, "{after} < {before}");
    }
}
