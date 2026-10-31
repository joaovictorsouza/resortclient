//! The game's side of the `animals` crate: pets are NPCs the server owns (it decides what they do), and
//! this module only mirrors them in the room.
//!
//! * `ROOM_SNAPSHOT.pets` and `PET_PLACED` create or replace a pet; `PET_PATH` makes it walk a route,
//!   `PET_ACTION` sets what it does standing; `PET_REMOVED` takes it out.
//! * Each species' `animals/<species>/clips.json` is read from the asset source (the CDN) the first time a
//!   pet of that species shows up, and described to the crate in its `AnimalConfig`.
//! * Interactions are a request/response pair: write a [`PetInteractionRequest`] and listen to
//!   `NetPetInteractResultEvent`. No interaction ships yet; tools built later use these two.
//!
//! `GAME_PET=off` hides pets. Without the species files in the CDN the pet stays invisible (a warning).

use std::collections::{HashMap, HashSet, VecDeque};

use animals::{
    Animal, AnimalAction, AnimalConfig, AnimalFacing, AnimalRoute, AnimalsPlugin, SpeciesAnimations,
};
use bevy::asset::LoadState;
use bevy::prelude::*;
use nakama_client::NakamaCommand;
use protocol::{ClientMessage, PetInteractRequest as PetInteractMessage, PetSnapshot};
use toon_avatar::parse_clips;

use crate::cdn_assets::{JsonText, JsonTextLoader};
use crate::net::{
    NakamaResource, NetPetActionEvent, NetPetPathEvent, NetPetPlacedEvent, NetPetRemovedEvent,
    NetRoomSnapshotEvent,
};
use crate::room::tile_to_world;

/// Height of a pet in world units (the avatars are 1.8).
const PET_HEIGHT: f32 = 0.5;

/// The id the server gave a pet.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ServerPet {
    pub id: String,
}

/// An action received while the pet is still walking its route: it starts when the route ends.
#[derive(Component, Debug, Clone)]
struct PendingAction {
    action: AnimalAction,
    tile: (i32, i32),
    rotation: i32,
}

/// Write this to ask the server for an interaction with a pet (`kind` is open: the server only accepts the
/// ones it serves). The answer arrives as `NetPetInteractResultEvent`.
#[derive(Message, Debug, Clone)]
pub struct PetInteractionRequest {
    pub pet_id: String,
    pub kind: String,
}

/// Where each species' data is: asked for once, then kept.
#[derive(Resource, Default)]
struct SpeciesData {
    loading: HashMap<String, Handle<JsonText>>,
    failed: HashSet<String>,
}

fn clips_file(species: &str) -> String {
    format!("animals/{species}/clips.json")
}

fn animations_file(species: &str) -> String {
    format!("animals/{species}/anims.glb")
}

/// The rotation of a server facing (0..7 in 45° steps; 0 = +Z, 2 = +X) as a transform rotation.
pub fn facing_from_rotation(rotation: i32) -> Quat {
    let angle = rotation.rem_euclid(8) as f32 * std::f32::consts::FRAC_PI_4;
    let direction = Vec3::new(angle.sin(), 0.0, angle.cos());
    Transform::default().looking_to(direction, Vec3::Y).rotation
}

pub struct PetPlugin;

impl Plugin for PetPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<Assets<JsonText>>().is_none() {
            app.init_asset::<JsonText>()
                .register_asset_loader(JsonTextLoader);
        }
        app.add_message::<PetInteractionRequest>();
        if std::env::var("GAME_PET").is_ok_and(|v| v == "off") {
            return;
        }
        app.add_plugins(AnimalsPlugin)
            .init_resource::<SpeciesData>()
            .add_systems(
                Update,
                (
                    apply_snapshot,
                    apply_placed,
                    apply_path,
                    apply_action,
                    apply_removed,
                    start_pending_actions,
                    load_species,
                    send_interactions,
                )
                    .chain(),
            );
    }
}

fn spawn_pet(commands: &mut Commands, pet: &PetSnapshot) {
    commands.spawn((
        ServerPet { id: pet.id.clone() },
        Name::new(format!("pet {}", pet.name)),
        Animal::new(&pet.species, &pet.breed, &pet.coat, PET_HEIGHT),
        AnimalAction::from_name(&pet.action).unwrap_or_default(),
        AnimalFacing(facing_from_rotation(pet.rotation)),
        Transform::from_translation(tile_to_world(pet.x, pet.z))
            .with_rotation(facing_from_rotation(pet.rotation)),
    ));
}

/// A new room: its pets replace the old room's.
fn apply_snapshot(
    mut commands: Commands,
    mut snapshots: MessageReader<NetRoomSnapshotEvent>,
    existing: Query<Entity, With<ServerPet>>,
) {
    for event in snapshots.read() {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
        for pet in &event.0.pets {
            spawn_pet(&mut commands, pet);
        }
    }
}

/// `PET_PLACED` creates the pet or replaces the one with the same id (furniture pushed it to another tile).
fn apply_placed(
    mut commands: Commands,
    mut placed: MessageReader<NetPetPlacedEvent>,
    pets: Query<(Entity, &ServerPet)>,
) {
    for event in placed.read() {
        let pet = &event.0;
        for (entity, existing) in &pets {
            if existing.id == pet.id {
                commands.entity(entity).despawn();
            }
        }
        spawn_pet(&mut commands, pet);
    }
}

fn apply_path(
    mut commands: Commands,
    mut paths: MessageReader<NetPetPathEvent>,
    pets: Query<(Entity, &ServerPet)>,
) {
    for event in paths.read() {
        let path = &event.0;
        let Some((entity, _)) = pets.iter().find(|(_, p)| p.id == path.id) else {
            continue;
        };
        let waypoints: VecDeque<Vec3> = path
            .path
            .iter()
            .map(|t| tile_to_world(t[0], t[1]))
            .collect();
        commands.entity(entity).remove::<PendingAction>().insert((
            AnimalAction::from_name(&path.action).unwrap_or(AnimalAction::Walk),
            AnimalRoute {
                waypoints,
                tiles_per_sec: path.tiles_per_sec as f32,
            },
        ));
    }
}

fn apply_action(
    mut commands: Commands,
    mut actions: MessageReader<NetPetActionEvent>,
    pets: Query<(Entity, &ServerPet)>,
) {
    for event in actions.read() {
        let action = &event.0;
        let Some((entity, _)) = pets.iter().find(|(_, p)| p.id == action.id) else {
            continue;
        };
        commands.entity(entity).insert(PendingAction {
            action: AnimalAction::from_name(&action.action).unwrap_or_default(),
            tile: (action.x, action.z),
            rotation: action.rotation,
        });
    }
}

fn apply_removed(
    mut commands: Commands,
    mut removed: MessageReader<NetPetRemovedEvent>,
    pets: Query<(Entity, &ServerPet)>,
) {
    for event in removed.read() {
        for (entity, pet) in &pets {
            if pet.id == event.0.id {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// A pending action starts once the pet has finished its walk (so it never slides while barking).
#[allow(clippy::type_complexity)]
fn start_pending_actions(
    mut commands: Commands,
    mut pets: Query<
        (Entity, &PendingAction, &mut Transform),
        (With<ServerPet>, Without<AnimalRoute>),
    >,
) {
    for (entity, pending, mut transform) in &mut pets {
        let spot = tile_to_world(pending.tile.0, pending.tile.1);
        // The walk ended near the server's tile: settle exactly on it.
        if transform.translation.distance(spot) < 1.5 {
            transform.translation = spot;
        }
        commands.entity(entity).remove::<PendingAction>().insert((
            pending.action,
            AnimalFacing(facing_from_rotation(pending.rotation)),
        ));
    }
}

/// Reads the `clips.json` of every species in the room and tells the crate about it.
fn load_species(
    mut data: ResMut<SpeciesData>,
    mut config: ResMut<AnimalConfig>,
    assets: Res<AssetServer>,
    texts: Res<Assets<JsonText>>,
    pets: Query<&Animal>,
) {
    for animal in &pets {
        let species = &animal.species;
        if config.species.contains_key(species)
            || data.loading.contains_key(species)
            || data.failed.contains(species)
        {
            continue;
        }
        data.loading
            .insert(species.clone(), assets.load(clips_file(species)));
    }
    let mut done = Vec::new();
    for (species, handle) in &data.loading {
        if matches!(assets.load_state(handle), LoadState::Failed(_)) {
            warn!(
                "the files of the '{species}' animals are not available: they stay out of the room"
            );
            done.push((species.clone(), None));
            continue;
        }
        let Some(text) = texts.get(handle) else {
            continue;
        };
        done.push((species.clone(), Some(species_animations(species, &text.0))));
    }
    for (species, loaded) in done {
        data.loading.remove(&species);
        match loaded {
            Some(Ok(animations)) => {
                config.species.insert(species, animations);
            }
            Some(Err(error)) => {
                warn!("invalid clips of the '{species}' animals: {error}");
                data.failed.insert(species);
            }
            None => {
                data.failed.insert(species);
            }
        }
    }
}

/// The crate's description of a species from its `clips.json`.
fn species_animations(species: &str, json: &str) -> Result<SpeciesAnimations, String> {
    let clips = parse_clips(json).map_err(|e| e.to_string())?;
    let height = serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v["model_height_m"].as_f64())
        .filter(|h| *h > 0.0)
        .ok_or("model_height_m is missing")?;
    Ok(SpeciesAnimations {
        gltf_path: animations_file(species),
        clips,
        model_height_m: height as f32,
    })
}

fn send_interactions(
    mut requests: MessageReader<PetInteractionRequest>,
    net: Option<Res<NakamaResource>>,
) {
    for request in requests.read() {
        let Some(net) = &net else {
            continue;
        };
        net.handle
            .send_command(NakamaCommand::SendMessage(ClientMessage::PetInteract(
                PetInteractMessage {
                    pet_id: request.pet_id.clone(),
                    kind: request.kind.clone(),
                },
            )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_rotations_face_the_documented_directions() {
        // 0 = +Z, 2 = +X, 4 = -Z, 6 = -X.
        for (rotation, direction) in [
            (0, Vec3::Z),
            (2, Vec3::X),
            (4, Vec3::NEG_Z),
            (6, Vec3::NEG_X),
        ] {
            // A transform faces -Z.
            let facing = facing_from_rotation(rotation) * Vec3::NEG_Z;
            assert!(
                facing.abs_diff_eq(direction, 1e-5),
                "{rotation}: {facing:?}"
            );
        }
        assert!(facing_from_rotation(8).abs_diff_eq(facing_from_rotation(0), 1e-5));
        assert!(facing_from_rotation(-2).abs_diff_eq(facing_from_rotation(6), 1e-5));
    }

    #[test]
    fn clips_json_becomes_the_species_description() {
        let json = r#"{"version":1,"model_height_m":0.4,"clips":{
            "walk":{"loop":true,"duration_s":1.0,"contact_height":0.0,"tiles_per_sec":0.125},
            "sit":{"loop":true,"duration_s":3.0,"contact_height":0.0}}}"#;
        let species = species_animations("dog", json).unwrap();
        assert_eq!(species.gltf_path, "animals/dog/anims.glb");
        assert_eq!(species.model_height_m, 0.4);
        assert_eq!(species.clips["walk"].tiles_per_sec, Some(0.125));
        assert!(species_animations("dog", r#"{"clips":{}}"#).is_err());
        assert!(species_animations("dog", "nonsense").is_err());
    }

    fn snapshot(id: &str, action: &str) -> PetSnapshot {
        PetSnapshot {
            id: id.into(),
            catalog_id: "pet_dog_shihtzu".into(),
            owner_id: "u".into(),
            name: "Shih-tzu".into(),
            species: "dog".into(),
            breed: "shihtzu".into(),
            coat: "default".into(),
            x: 4,
            z: 3,
            rotation: 2,
            action: action.into(),
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<NetRoomSnapshotEvent>()
            .add_message::<NetPetPlacedEvent>()
            .add_message::<NetPetPathEvent>()
            .add_message::<NetPetActionEvent>()
            .add_message::<NetPetRemovedEvent>()
            .add_systems(
                Update,
                (
                    apply_snapshot,
                    apply_placed,
                    apply_path,
                    apply_action,
                    apply_removed,
                    start_pending_actions,
                )
                    .chain(),
            );
        app
    }

    fn find(app: &mut App, id: &str) -> Option<Entity> {
        app.world_mut()
            .query::<(Entity, &ServerPet)>()
            .iter(app.world())
            .find(|(_, p)| p.id == id)
            .map(|(e, _)| e)
    }

    #[test]
    fn a_snapshot_creates_its_pets_on_their_tiles_and_the_next_room_replaces_them() {
        let mut app = app();
        let mut snap = protocol_snapshot();
        snap.pets = vec![snapshot("a", "sit")];
        app.world_mut().write_message(NetRoomSnapshotEvent(snap));
        app.update();
        let pet = find(&mut app, "a").expect("pet spawned");
        assert_eq!(
            app.world().get::<AnimalAction>(pet),
            Some(&AnimalAction::Sit)
        );
        let at = app.world().get::<Transform>(pet).unwrap().translation;
        assert!(at.abs_diff_eq(tile_to_world(4, 3), 1e-5));

        app.world_mut()
            .write_message(NetRoomSnapshotEvent(protocol_snapshot()));
        app.update();
        assert!(find(&mut app, "a").is_none(), "an empty room has no pets");
    }

    #[test]
    fn placed_replaces_a_pet_with_the_same_id_and_removed_takes_it_out() {
        let mut app = app();
        app.world_mut()
            .write_message(NetPetPlacedEvent(snapshot("a", "idle")));
        app.update();
        let mut moved = snapshot("a", "idle");
        moved.x = 1;
        app.world_mut().write_message(NetPetPlacedEvent(moved));
        app.update();
        let pets: Vec<_> = app
            .world_mut()
            .query::<&ServerPet>()
            .iter(app.world())
            .collect();
        assert_eq!(pets.len(), 1);
        let pet = find(&mut app, "a").unwrap();
        let at = app.world().get::<Transform>(pet).unwrap().translation;
        assert!(at.abs_diff_eq(tile_to_world(1, 3), 1e-5));

        app.world_mut()
            .write_message(NetPetRemovedEvent(protocol::PetRemoved { id: "a".into() }));
        app.update();
        assert!(find(&mut app, "a").is_none());
    }

    #[test]
    fn a_path_starts_a_walk_and_an_action_waits_for_it_to_end() {
        let mut app = app();
        app.world_mut()
            .write_message(NetPetPlacedEvent(snapshot("a", "idle")));
        app.update();
        app.world_mut()
            .write_message(NetPetPathEvent(protocol::PetPath {
                id: "a".into(),
                path: vec![[5, 3], [6, 3]],
                tiles_per_sec: 0.35,
                action: "walk".into(),
            }));
        app.update();
        let pet = find(&mut app, "a").unwrap();
        let route = app.world().get::<AnimalRoute>(pet).expect("walking");
        assert_eq!(route.waypoints.len(), 2);
        assert_eq!(
            app.world().get::<AnimalAction>(pet),
            Some(&AnimalAction::Walk)
        );

        app.world_mut()
            .write_message(NetPetActionEvent(protocol::PetAction {
                id: "a".into(),
                action: "bark".into(),
                x: 6,
                z: 3,
                rotation: 4,
            }));
        app.update();
        assert_eq!(
            app.world().get::<AnimalAction>(pet),
            Some(&AnimalAction::Walk),
            "still walking: the bark waits"
        );
        // The walk ends a little short of the last tile (the crate removes the route).
        app.world_mut()
            .entity_mut(pet)
            .get_mut::<Transform>()
            .unwrap()
            .translation = tile_to_world(6, 3) + Vec3::new(-0.2, 0.0, 0.1);
        app.world_mut().entity_mut(pet).remove::<AnimalRoute>();
        app.update();
        assert_eq!(
            app.world().get::<AnimalAction>(pet),
            Some(&AnimalAction::Bark)
        );
        let at = app.world().get::<Transform>(pet).unwrap().translation;
        assert!(
            at.abs_diff_eq(tile_to_world(6, 3), 1e-5),
            "settles on the server's tile"
        );
    }

    fn protocol_snapshot() -> protocol::RoomSnapshot {
        protocol::RoomSnapshot {
            room_id: "r".into(),
            room_name: "R".into(),
            owner_id: String::new(),
            layout: protocol::LayoutSnapshot {
                id: "l".into(),
                width: 8,
                height: 8,
                door: [0, 0],
                heightmap: vec!["00000000".into(); 8],
            },
            avatars: vec![],
            items: vec![],
            pets: vec![],
        }
    }
}
