//! Dev preview of the lobby pet (Mochi) with the toon scene style, no server needed.
//!
//!   cargo run -p game --example pet_preview -- out_dir   # a screenshot every 2 s following the pet, then exit
//!
//! `ROOM_ASSETS` points at the asset folder with `animals/mochi` and `characters/textures/ColorRamp.png`
//! (default `../resort-assets/assets`); `PET_CAM=side` films the pet from its side; `PET_SHOTS` sets the number of screenshots (default 12);
//! `TOON_AVATAR_TIER=low|mid|high` forces the device tier (and so the model's level of detail).
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game::net::NetRoomSnapshotEvent;
use game::pet::{Pet, PetPlugin};
use game::room::RoomPlugin;
use protocol::{LayoutSnapshot, RoomSnapshot};

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
        .insert_resource(toon_avatar::SceneStyle { cartoon: true })
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
        .insert_resource(Out(out))
        .add_systems(Startup, setup)
        .add_systems(Update, follow_and_shoot)
        .run();
}

#[derive(Resource)]
struct Out(String);

#[derive(Resource)]
struct Target(Handle<Image>);

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
        room_id: "lobby".into(),
        room_name: "Lobby".into(),
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

#[allow(clippy::too_many_arguments)]
fn follow_and_shoot(
    mut commands: Commands,
    time: Res<Time>,
    out: Res<Out>,
    target: Res<Target>,
    pet: Query<(&Pet, &Transform), Without<Camera3d>>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut exit: MessageWriter<AppExit>,
    mut shot: Local<usize>,
) {
    let Ok(mut cam) = camera.single_mut() else {
        return;
    };
    let Ok((pet, at)) = pet.single() else {
        // Until the files are in the pet is not there; look at the room's middle.
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
    let wanted: usize = std::env::var("PET_SHOTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    if *shot >= wanted {
        exit.write(AppExit::Success);
        return;
    }
    // The first shot waits for the textures and the toon materials.
    let due = 4.0 + *shot as f32 * 2.0;
    if time.elapsed_secs() > due {
        let _ = std::fs::create_dir_all(&out.0);
        info!("shot {} action {:?}", *shot, pet.action);
        commands
            .spawn(Screenshot::image(target.0.clone()))
            .observe(save_to_disk(format!(
                "{}/shot_{:02}_{:?}.png",
                out.0, *shot, pet.action
            )));
        *shot += 1;
    }
}
