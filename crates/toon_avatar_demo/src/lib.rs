//! Demonstrator for `toon_avatar`: the character assembly screen. It is not used by the game;
//! it exists to try the crate (and to take regression screenshots) with a real asset folder.
//!
//! Assets are read from `$TOON_AVATAR_ASSETS` (default `assets`), which must hold
//! `characters/{models,original,lod1,lod2,textures,textures_basis,items,materials}` as written
//! by the asset pipeline. The assets are never part of this repository.
//!
//! Controls: Space = top, B = bottom, S = shoes, U = outfit, H = face, X = body type,
//! N = hairstyle, Y = hat, G = glasses, O = outline, M = merge, Q = quality. Each also has an
//! on-screen button. Debug: `TOON_DEMO_SNAPSHOT=<file.png>` renders offscreen, saves and exits;
//! `TOON_DEMO_CAM=x,y,z,tx,ty,tz` moves the camera; `TOON_AVATAR_QUALITY=low|mid|high`.

mod ui;

use std::path::{Path, PathBuf};

use bevy::{asset::AssetPlugin, prelude::*};
use toon_avatar::{
    Avatar, AvatarCatalog, AvatarLook, BodyType, DeviceTier, LodFocus, MaterialDefs, MergeEnabled,
    QualityRenderSettingsPlugin, Slot, TierPreference, ToonAvatarPlugins, ToonSettings,
};

fn assets_dir() -> PathBuf {
    std::env::var("TOON_AVATAR_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("assets"))
}

fn read(dir: &Path, rel: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(rel)).ok()
}

/// The catalog and material definitions the asset pipeline wrote.
fn load_data(dir: &Path) -> (AvatarCatalog, MaterialDefs) {
    let outfits =
        read(dir, "characters/items/outfits.json").expect("characters/items/outfits.json");
    let attachments =
        read(dir, "characters/items/attachments.json").expect("characters/items/attachments.json");
    let custom = read(dir, "characters/items/custom_items.json");
    let catalog = AvatarCatalog::from_pipeline_json(&outfits, &attachments, custom.as_deref())
        .expect("the catalog files are valid");
    let materials = read(dir, "characters/materials/materials.json")
        .expect("characters/materials/materials.json");
    let mut documents = vec![materials];
    documents.extend(read(dir, "characters/materials/custom_materials.json"));
    let documents: Vec<&str> = documents.iter().map(String::as_str).collect();
    (
        catalog,
        MaterialDefs::from_json(&documents).expect("the material files are valid"),
    )
}

/// The avatar the demo starts with.
fn starting_look(catalog: &AvatarCatalog) -> AvatarLook {
    let body = BodyType::Female;
    let c = catalog.of(body);
    let first = |slot: Slot| c.slot(slot).first().map(|e| e.name.clone());
    AvatarLook {
        body,
        face: first(Slot::Face),
        top: first(Slot::Top),
        bottom: first(Slot::Bottom),
        hair: c
            .hair
            .iter()
            .find(|e| e.name == "Hairstyle.06")
            .map(|e| e.name.clone())
            .or_else(|| first(Slot::Hair)),
        ..default()
    }
}

/// Default camera; `TOON_DEMO_CAM=x,y,z,tx,ty,tz` (eye and target) overrides it for debug captures.
fn camera_transform() -> Transform {
    let values: Vec<f32> = std::env::var("TOON_DEMO_CAM")
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_default();
    match values[..] {
        [x, y, z, tx, ty, tz] => {
            Transform::from_xyz(x, y, z).looking_at(Vec3::new(tx, ty, tz), Vec3::Y)
        }
        _ => Transform::from_xyz(0.0, 1.1, 3.2).looking_at(Vec3::new(0.0, 0.9, 0.0), Vec3::Y),
    }
}

/// Offscreen image used by the snapshot debug mode.
#[derive(Resource)]
struct SnapshotTarget(Handle<Image>);

/// Saves the offscreen render once the scene has had time to load, then quits.
fn snapshot(
    mut commands: Commands,
    time: Res<Time>,
    target: Option<Res<SnapshotTarget>>,
    mut taken: Local<Option<f32>>,
    mut exit: MessageWriter<AppExit>,
) {
    let (Some(target), Ok(path)) = (target, std::env::var("TOON_DEMO_SNAPSHOT")) else {
        return;
    };
    let delay: f32 = std::env::var("TOON_DEMO_SNAPSHOT_DELAY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(14.0);
    if taken.is_none() && time.elapsed_secs() > delay {
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                target.0.clone(),
            ))
            .observe(bevy::render::view::screenshot::save_to_disk(path));
        *taken = Some(time.elapsed_secs());
    }
    if taken.is_some_and(|t| time.elapsed_secs() > t + 3.0) {
        exit.write(AppExit::Success);
    }
}

#[bevy_main]
pub fn main() {
    let dir = assets_dir();
    let (catalog, materials) = load_data(&dir);
    let look = starting_look(&catalog);
    App::new()
        // Entities can be despawned (body swap, quality change) while commands for them are
        // still queued; that must not take the demo down.
        .set_error_handler(bevy::ecs::error::warn)
        .add_plugins((
            DefaultPlugins.set(AssetPlugin {
                file_path: dir.to_string_lossy().into_owned(),
                ..default()
            }),
            ToonAvatarPlugins,
            QualityRenderSettingsPlugin,
            ui::UiPlugin,
        ))
        .insert_resource(catalog)
        .insert_resource(materials)
        .insert_resource(ClearColor(Color::srgb(0.25, 0.3, 0.45)))
        .add_systems(
            Startup,
            move |commands: Commands, images: ResMut<Assets<Image>>| {
                setup(commands, images, look.clone())
            },
        )
        .add_systems(
            Update,
            (wardrobe_input, toggles, log_render_stats, snapshot),
        )
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, look: AvatarLook) {
    // `TOON_DEMO_SECOND=1` adds a second avatar with another body, skin and outfit colour, to
    // check that colours do not leak between avatars that wear the same items.
    if std::env::var("TOON_DEMO_SECOND").is_ok() {
        let mut other = look.clone();
        other.body = BodyType::Male;
        other.set_color("mat_body_M", 1, [0.95, 0.80, 0.65]);
        other.set_color("mat_face_M", 1, [0.95, 0.80, 0.65]);
        commands.spawn((Avatar::new(other), Transform::from_xyz(0.9, 0.0, 0.0)));
    }
    let x = if std::env::var("TOON_DEMO_SECOND").is_ok() {
        -0.9
    } else {
        0.0
    };
    commands.spawn((
        Avatar::new(look),
        LodFocus,
        Transform::from_xyz(x, 0.0, 0.0),
    ));
    let mut camera = commands.spawn((
        Camera3d::default(),
        // Toon colours are authored for display directly; skip tone mapping.
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        camera_transform(),
    ));
    if std::env::var("TOON_DEMO_SNAPSHOT").is_ok() {
        let target = images.add(Image::new_target_texture(
            900,
            1300,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        camera.insert(bevy::camera::RenderTarget::Image(
            bevy::camera::ImageRenderTarget {
                handle: target.clone(),
                scale_factor: 1.0,
            },
        ));
        commands.insert_resource(SnapshotTarget(target));
    }
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
    commands.insert_resource(GlobalAmbientLight {
        brightness: 400.0,
        ..default()
    });
}

/// Cycles `current` through `names` and "none" (`None` only when `allow_none`).
fn next_item(names: &[&str], current: Option<&str>, allow_none: bool) -> Option<String> {
    let index = current.and_then(|c| names.iter().position(|n| *n == c));
    match index {
        None => names.first().map(|n| n.to_string()),
        Some(i) if i + 1 < names.len() => Some(names[i + 1].to_string()),
        Some(_) if allow_none => None,
        Some(_) => names.first().map(|n| n.to_string()),
    }
}

/// Keeps only the items the new body type also has.
fn reconcile(look: &mut AvatarLook, catalog: &AvatarCatalog) {
    let body = catalog.of(look.body);
    for slot in Slot::ALL {
        if let Some(name) = look.item(slot)
            && body.find(slot, name).is_none()
        {
            look.set_item(slot, None);
        }
    }
}

fn wardrobe_input(
    keys: Res<ButtonInput<KeyCode>>,
    catalog: Res<AvatarCatalog>,
    mut avatars: Query<&mut Avatar>,
) {
    let Ok(mut avatar) = avatars.single_mut() else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyX) {
        avatar.look.body = avatar.look.body.other();
        reconcile(&mut avatar.look, &catalog);
        return;
    }
    let slot = [
        (KeyCode::Space, Slot::Top),
        (KeyCode::KeyB, Slot::Bottom),
        (KeyCode::KeyS, Slot::Shoes),
        (KeyCode::KeyU, Slot::Outfit),
        (KeyCode::KeyH, Slot::Face),
        (KeyCode::KeyN, Slot::Hair),
        (KeyCode::KeyY, Slot::Hat),
        (KeyCode::KeyG, Slot::Glasses),
    ]
    .into_iter()
    .find(|(key, _)| keys.just_pressed(*key))
    .map(|(_, slot)| slot);
    let Some(slot) = slot else {
        return;
    };
    let names: Vec<&str> = catalog
        .of(avatar.look.body)
        .slot(slot)
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    // A body always needs a face.
    let next = next_item(&names, avatar.look.item(slot), slot != Slot::Face);
    avatar.look.set_item(slot, next);
}

/// O = outline, M = mesh merge, Q = quality tier.
fn toggles(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ToonSettings>,
    mut merge: ResMut<MergeEnabled>,
    tier: Option<Res<DeviceTier>>,
    mut preference: ResMut<TierPreference>,
) {
    if keys.just_pressed(KeyCode::KeyO) {
        settings.outline = !settings.outline;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        merge.0 = !merge.0;
    }
    if keys.just_pressed(KeyCode::KeyQ)
        && let Some(tier) = tier
    {
        *preference = TierPreference::Fixed(tier.next());
    }
}

/// Logs the number of visible meshes (~draw calls per pass) and their triangles when they change.
#[allow(clippy::type_complexity)]
fn log_render_stats(
    meshes: Res<Assets<Mesh>>,
    visible: Query<
        (&Mesh3d, &ViewVisibility),
        Or<(
            With<MeshMaterial3d<toon_avatar::ToonMaterial>>,
            With<MeshMaterial3d<toon_avatar::AtlasMaterial>>,
        )>,
    >,
    mut last: Local<(usize, usize)>,
) {
    let (mut draws, mut tris) = (0, 0);
    for (mesh, vis) in &visible {
        if vis.get() {
            draws += 1;
            tris += meshes
                .get(&mesh.0)
                .and_then(|m| m.indices().map(|i| i.len() / 3))
                .unwrap_or(0);
        }
    }
    if (draws, tris) != *last {
        *last = (draws, tris);
        info!("render stats: {draws} visible meshes, {tris} triangles");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycling_goes_through_every_name_and_then_none() {
        let names = ["a", "b"];
        assert_eq!(next_item(&names, None, true).as_deref(), Some("a"));
        assert_eq!(next_item(&names, Some("a"), true).as_deref(), Some("b"));
        assert_eq!(next_item(&names, Some("b"), true), None);
        assert_eq!(next_item(&names, Some("b"), false).as_deref(), Some("a"));
    }

    #[test]
    fn switching_body_drops_items_the_other_body_does_not_have() {
        let catalog = AvatarCatalog::from_pipeline_json(
            r#"{"F":{"body":"f","top":[{"name":"T","glbs":[]}]},"M":{"body":"m","top":[]}}"#,
            r#"{"hair":[],"hat":[],"glasses":[]}"#,
            None,
        )
        .unwrap();
        let mut look = AvatarLook {
            top: Some("T".into()),
            body: BodyType::Male,
            ..default()
        };
        reconcile(&mut look, &catalog);
        assert_eq!(look.top, None);
    }
}
