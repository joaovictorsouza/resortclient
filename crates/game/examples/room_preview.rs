//! Dev preview of the room geometry (floor and walls) with the toon scene style, no server needed.
//!
//!   cargo run -p game --example room_preview -- out_dir   # screenshots from several angles, then exit
//!
//! `ROOM_ASSETS` points at the asset folder that holds `characters/textures/ColorRamp.png`
//! (default `../resort-game-assets/assets`); `ROOM_STYLE=standard` turns the cartoon style off.
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game::net::NetRoomSnapshotEvent;
use game::room::RoomPlugin;
use protocol::{LayoutSnapshot, RoomSnapshot};

const ANGLES_DEG: [f32; 6] = [45.0, 135.0, 225.0, 315.0, 0.0, 90.0];

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "room_preview".into());
    let assets =
        std::env::var("ROOM_ASSETS").unwrap_or_else(|_| "../resort-game-assets/assets".into());
    let assets = std::fs::canonicalize(&assets).unwrap_or_else(|_| assets.into());
    let cartoon = std::env::var("ROOM_STYLE").is_ok_and(|v| v != "standard");
    App::new()
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                file_path: assets.to_string_lossy().into_owned(),
                ..default()
            }),
            toon_avatar::SceneStylePlugin,
            RoomPlugin,
        ))
        .add_message::<NetRoomSnapshotEvent>()
        .insert_resource(toon_avatar::SceneStyle { cartoon })
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.2)))
        .insert_resource(Out(out))
        .add_systems(Startup, setup)
        .add_systems(Update, orbit)
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
    // Offscreen target: a window capture can come out black when the window is not in front.
    let target = images.add(Image::new_target_texture(
        1280,
        720,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    commands.insert_resource(Target(target.clone()));
    // An L-shaped room with a hole, to get inner and outer corners.
    let heightmap: Vec<String> = ["000000", "000000", "0000xx", "0000xx"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    snapshots.write(NetRoomSnapshotEvent(RoomSnapshot {
        room_id: "preview".into(),
        room_name: "Preview".into(),
        owner_id: String::new(),
        layout: LayoutSnapshot {
            id: "preview".into(),
            width: 6,
            height: 4,
            door: [0, 0],
            heightmap,
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

fn orbit(
    mut commands: Commands,
    time: Res<Time>,
    out: Res<Out>,
    target: Res<Target>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut exit: MessageWriter<AppExit>,
    mut shot: Local<usize>,
) {
    // Give the first frames time to load the ramp texture and swap the materials.
    let step = ((time.elapsed_secs() - 6.0) / 2.0).floor();
    if step < 0.0 {
        return;
    }
    let step = step as usize;
    if step >= ANGLES_DEG.len() {
        exit.write(AppExit::Success);
        return;
    }
    let angle = ANGLES_DEG[step].to_radians();
    let center = Vec3::new(3.0, 0.3, 2.0);
    // Two heights per angle would double the shots; one steep 40° view shows floor and wall bases.
    let eye = center + Vec3::new(angle.cos() * 7.0, 6.0, angle.sin() * 7.0);
    if let Ok(mut t) = camera.single_mut() {
        *t = Transform::from_translation(eye).looking_at(center, Vec3::Y);
    }
    if *shot == step && (time.elapsed_secs() - 6.0) % 2.0 > 1.0 {
        let _ = std::fs::create_dir_all(&out.0);
        commands
            .spawn(Screenshot::image(target.0.clone()))
            .observe(save_to_disk(format!(
                "{}/angle_{:03}.png",
                out.0, ANGLES_DEG[step] as i32
            )));
        *shot += 1;
    }
}
