//! Dev preview of a server-driven pet (Mochi, the shih-tzu) with the toon scene style, no server needed:
//! a small script plays the part of the server (it sends the same messages) and a screenshot is taken
//! of each action.
//!
//!   cargo run -p game --example pet_preview -- out_dir
//!
//! `ROOM_ASSETS` points at the asset folder with `animals/dog` and `characters/textures/ColorRamp.png`
//! (default `../resort-assets/assets`); `PET_CAM=side` films the pet from its side;
//! `TOON_AVATAR_TIER=low|mid|high` forces the device tier (and so the model's level of detail).
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game::net::{NetPetActionEvent, NetPetPathEvent, NetPetPlacedEvent, NetRoomSnapshotEvent};
use game::pet::{PetPlugin, ServerPet};
use game::room::RoomPlugin;
use protocol::{LayoutSnapshot, PetAction, PetPath, PetSnapshot, RoomSnapshot};

const PET_ID: &str = "placed_preview";
/// What the fake server does, one step every `STEP_SECS`: (action, optional route to walk).
const SCRIPT: [(&str, &[[i32; 2]]); 8] = [
    ("sit", &[]),
    ("lay", &[]),
    ("idle", &[]),
    ("pant", &[]),
    ("bark", &[]),
    ("walk", &[[3, 2], [4, 2], [5, 2], [5, 3]]),
    ("run", &[[5, 4], [4, 4], [3, 4], [3, 3]]),
    ("sit", &[]),
];
const STEP_SECS: f32 = 3.0;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "pet_preview".into());
    let assets = std::env::var("ROOM_ASSETS").unwrap_or_else(|_| "../resort-assets/assets".into());
    let assets = std::fs::canonicalize(&assets).unwrap_or_else(|_| assets.into());
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                file_path: assets.to_string_lossy().into_owned(),
                ..default()
            }),
            toon_avatar::SceneStylePlugin,
            toon_avatar::LodPlugin,
            RoomPlugin,
            PetPlugin,
        ))
        .add_message::<NetRoomSnapshotEvent>()
        .add_message::<NetPetPlacedEvent>()
        .add_message::<NetPetPathEvent>()
        .add_message::<NetPetActionEvent>()
        .add_message::<game::net::NetPetRemovedEvent>()
        .insert_resource(toon_avatar::SceneStyle { cartoon: true })
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
        .insert_resource(Out(out))
        .add_systems(Startup, setup)
        .add_systems(Update, (fake_server, follow_and_shoot))
        .run();
}

#[derive(Resource)]
struct Out(String);

#[derive(Resource)]
struct Target(Handle<Image>);

fn pet(action: &str) -> PetSnapshot {
    PetSnapshot {
        id: PET_ID.into(),
        catalog_id: "pet_dog_shihtzu".into(),
        owner_id: "u".into(),
        name: "Cachorro Shih-tzu".into(),
        species: "dog".into(),
        breed: "shihtzu".into(),
        coat: "default".into(),
        x: 3,
        z: 3,
        rotation: 2,
        action: action.into(),
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut snapshots: MessageWriter<NetRoomSnapshotEvent>,
) {
    let target = images.add(Image::new_target_texture(
        960,
        540,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    commands.insert_resource(Target(target.clone()));
    snapshots.write(NetRoomSnapshotEvent(RoomSnapshot {
        room_id: "preview".into(),
        room_name: "Preview".into(),
        owner_id: String::new(),
        layout: LayoutSnapshot {
            id: "preview".into(),
            width: 8,
            height: 6,
            door: [0, 0],
            heightmap: (0..6).map(|_| "00000000".to_string()).collect(),
        },
        avatars: vec![],
        items: vec![],
        pets: vec![pet("idle")],
    }));
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 6.0, 3.0).looking_at(Vec3::new(3.0, 0.0, 2.0), Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget {
            handle: target,
            scale_factor: 1.0,
        }),
        Transform::default(),
    ));
}

/// Plays the server: the same messages, one script step every few seconds, after the files have loaded.
fn fake_server(
    time: Res<Time>,
    mut paths: MessageWriter<NetPetPathEvent>,
    mut actions: MessageWriter<NetPetActionEvent>,
    mut placed: MessageWriter<NetPetPlacedEvent>,
    mut step: Local<usize>,
) {
    let due = 6.0 + *step as f32 * STEP_SECS;
    if time.elapsed_secs() < due || *step >= SCRIPT.len() {
        return;
    }
    let (action, route) = SCRIPT[*step];
    if *step == 0 {
        // The server resends the pet like after a furniture push: the client replaces it.
        placed.write(NetPetPlacedEvent(pet("idle")));
    }
    if route.is_empty() {
        actions.write(NetPetActionEvent(PetAction {
            id: PET_ID.into(),
            action: action.into(),
            x: 3,
            z: 3,
            rotation: 2,
        }));
    } else {
        paths.write(NetPetPathEvent(PetPath {
            id: PET_ID.into(),
            path: route.to_vec(),
            tiles_per_sec: if action == "run" { 0.85 } else { 0.35 },
            action: action.into(),
        }));
    }
    *step += 1;
}

#[allow(clippy::too_many_arguments)]
fn follow_and_shoot(
    mut commands: Commands,
    time: Res<Time>,
    out: Res<Out>,
    target: Res<Target>,
    pet: Query<&Transform, (With<ServerPet>, Without<Camera3d>)>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut exit: MessageWriter<AppExit>,
    mut shot: Local<usize>,
) {
    let Ok(mut cam) = camera.single_mut() else {
        return;
    };
    let Ok(at) = pet.single() else {
        *cam = Transform::from_xyz(4.0, 6.0, 9.0).looking_at(Vec3::new(4.0, 0.0, 3.0), Vec3::Y);
        return;
    };
    let focus = at.translation + Vec3::Y * 0.25;
    let offset = if std::env::var("PET_CAM").is_ok_and(|v| v == "side") {
        at.rotation * Vec3::new(1.4, 0.3, 0.0)
    } else {
        Vec3::new(1.2, 1.4, 1.8)
    };
    *cam = Transform::from_translation(focus + offset).looking_at(focus, Vec3::Y);
    if *shot >= SCRIPT.len() {
        exit.write(AppExit::Success);
        return;
    }
    // A shot 2 s into each script step, once the transition has settled.
    let due = 6.0 + *shot as f32 * STEP_SECS + 2.0;
    if time.elapsed_secs() > due {
        let _ = std::fs::create_dir_all(&out.0);
        let name = SCRIPT[*shot].0;
        commands
            .spawn(Screenshot::image(target.0.clone()))
            .observe(save_to_disk(format!(
                "{}/shot_{:02}_{name}.png",
                out.0, *shot
            )));
        *shot += 1;
    }
}
