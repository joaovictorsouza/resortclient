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
    Avatar, AvatarAnimationConfig, AvatarCatalog, AvatarFace, AvatarHolding, AvatarLook,
    AvatarPose, AvatarWalkSpeed, BodyAnimations, BodyType, ColorGroup, DeviceTier, Expression,
    HeldProp, LodFocus, MaterialDefs, MergeEnabled, QualityRenderSettingsPlugin, SceneStyle, Slot,
    TierPreference, ToonAvatarPlugins, ToonSettings, parse_clips, parse_grip,
};

/// `TOON_AVATAR_ASSETS`, else the first of these folders that holds `characters/`: `assets`, and the
/// asset repository next to this one.
///
/// The path is made absolute: Bevy resolves a relative asset folder against the executable, not the
/// working directory, which left the window empty.
fn assets_dir() -> PathBuf {
    let absolute = |dir: PathBuf| std::fs::canonicalize(&dir).unwrap_or(dir);
    if let Ok(dir) = std::env::var("TOON_AVATAR_ASSETS") {
        return absolute(PathBuf::from(dir));
    }
    let candidates = [
        "assets",
        "../resort-game-assets/assets",
        "../anime-character-bevy/assets",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|dir| dir.join("characters/items/outfits.json").is_file())
        .map(absolute)
        .unwrap_or_else(|| {
            panic!(
                "no character assets found: set TOON_AVATAR_ASSETS to a folder holding characters/ \
                 (for example ../resort-game-assets/assets); looked in {candidates:?}"
            )
        })
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

/// The animation library the pipeline wrote (`characters/animations/{F,M}`), if there is one.
fn load_animations(dir: &Path) -> Option<AvatarAnimationConfig> {
    let body = |letter: &str| -> Option<BodyAnimations> {
        let clips = read(dir, &format!("characters/animations/{letter}/clips.json"))?;
        Some(BodyAnimations {
            gltf_path: format!("characters/animations/{letter}/character_anims.glb"),
            grip: parse_grip(&clips),
            clips: parse_clips(&clips).ok()?,
        })
    };
    Some(AvatarAnimationConfig {
        female: body("F")?,
        male: body("M")?,
    })
}

/// The avatar the demo starts with.
/// `TOON_DEMO_BODY=M` starts with the male body and `TOON_DEMO_HAIR=<name>` with that hair (debug captures).
fn starting_look(catalog: &AvatarCatalog) -> AvatarLook {
    let body = if std::env::var("TOON_DEMO_BODY").is_ok_and(|v| v == "M") {
        BodyType::Male
    } else {
        BodyType::Female
    };
    let hair_name = std::env::var("TOON_DEMO_HAIR").unwrap_or_else(|_| "Hairstyle.06".into());
    let c = catalog.of(body);
    let first = |slot: Slot| c.slot(slot).first().map(|e| e.name.clone());
    let mut group_colors = std::collections::BTreeMap::new();
    // `TOON_DEMO_DEBUG_COLORS=1` paints the skin blue and the underwear green: shows which region of
    // the body mask each part of the body uses.
    if std::env::var("TOON_DEMO_DEBUG_COLORS").is_ok() {
        group_colors.insert(
            ColorGroup::Skin,
            vec![Some([0.0, 0.2, 1.0]), Some([0.0, 0.2, 1.0])],
        );
        group_colors.insert(
            ColorGroup::Underwear,
            vec![Some([0.0, 1.0, 0.0]), Some([0.0, 1.0, 0.0])],
        );
    }
    AvatarLook {
        group_colors,
        body,
        face: first(Slot::Face),
        top: std::env::var("TOON_DEMO_TOP")
            .ok()
            .filter(|n| c.top.iter().any(|e| &e.name == n))
            .or_else(|| first(Slot::Top)),
        bottom: first(Slot::Bottom),
        hair: c
            .hair
            .iter()
            .find(|e| e.name == hair_name)
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

/// Side of the square thumbnails (`TOON_DEMO_THUMBS`).
const THUMB_SIZE: u32 = 256;

/// `TOON_DEMO_THUMBS=<dir>` renders one transparent PNG per wardrobe item (`<dir>/<item name>.png`)
/// and quits: the pictures the creator shows instead of the item names. Items that already have a
/// picture are skipped, unless `TOON_DEMO_THUMBS_FORCE` is set.
fn thumbs_dir() -> Option<PathBuf> {
    std::env::var("TOON_DEMO_THUMBS").ok().map(PathBuf::from)
}

/// Where the camera looks for a slot: heads close, clothes at the body part, shoes at the feet.
fn thumb_camera(slot: Slot) -> Transform {
    let (eye, target) = match slot {
        Slot::Face | Slot::Hair | Slot::Hat | Slot::Glasses => {
            (Vec3::new(0.0, 1.52, 0.85), Vec3::new(0.0, 1.48, 0.0))
        }
        Slot::Top => (Vec3::new(0.0, 1.2, 1.25), Vec3::new(0.0, 1.15, 0.0)),
        Slot::Outfit => (Vec3::new(0.0, 0.95, 2.3), Vec3::new(0.0, 0.9, 0.0)),
        Slot::Bottom => (Vec3::new(0.0, 0.55, 1.45), Vec3::new(0.0, 0.5, 0.0)),
        Slot::Shoes => (Vec3::new(0.0, 0.25, 0.9), Vec3::new(0.0, 0.1, 0.0)),
    };
    Transform::from_translation(eye).looking_at(target, Vec3::Y)
}

struct ThumbJob {
    body: BodyType,
    slot: Slot,
    name: String,
}

#[derive(Default)]
struct ThumbRun {
    jobs: Option<Vec<ThumbJob>>,
    next: usize,
    /// When the current job was applied, and whether its picture was already requested.
    applied_at: Option<f32>,
    shot: bool,
}

#[allow(clippy::too_many_arguments)]
fn thumbnails(
    mut commands: Commands,
    time: Res<Time>,
    catalog: Res<AvatarCatalog>,
    target: Option<Res<SnapshotTarget>>,
    mut avatars: Query<&mut Avatar>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
    mut run: Local<ThumbRun>,
    mut exit: MessageWriter<AppExit>,
) {
    let (Some(dir), Some(target)) = (thumbs_dir(), target) else {
        return;
    };
    let Ok(mut avatar) = avatars.single_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    let force = std::env::var("TOON_DEMO_THUMBS_FORCE").is_ok();
    if run.jobs.is_none() {
        let _ = std::fs::create_dir_all(&dir);
        let mut seen = std::collections::HashSet::new();
        let mut jobs = Vec::new();
        for body in [BodyType::Female, BodyType::Male] {
            for slot in Slot::ALL {
                for entry in catalog.of(body).slot(slot) {
                    let only = std::env::var("TOON_DEMO_THUMBS_ONLY")
                        .map_or(true, |v| v.split(',').any(|n| n == entry.name));
                    let done = !only || dir.join(format!("{}.png", entry.name)).exists() && !force;
                    if !done && seen.insert(entry.name.clone()) {
                        jobs.push(ThumbJob {
                            body,
                            slot,
                            name: entry.name.clone(),
                        });
                    }
                }
            }
        }
        info!(
            "thumbnails: {} to render into {}",
            jobs.len(),
            dir.display()
        );
        run.jobs = Some(jobs);
    }
    let total = run.jobs.as_ref().map_or(0, Vec::len);
    if run.next >= total {
        exit.write(AppExit::Success);
        return;
    }
    // Let the first avatar and its textures load before the first picture.
    if run.applied_at.is_none() && now < 10.0 {
        return;
    }
    let (body, slot, name) = {
        let job = &run.jobs.as_ref().unwrap()[run.next];
        (job.body, job.slot, job.name.clone())
    };
    match run.applied_at {
        None => {
            let mut look = starting_look(&catalog);
            look.body = body;
            reconcile(&mut look, &catalog);
            if slot == Slot::Outfit {
                look.set_item(Slot::Top, None);
                look.set_item(Slot::Bottom, None);
            }
            look.set_item(slot, Some(name));
            avatar.look = look;
            if let Ok(mut camera) = cameras.single_mut() {
                *camera = thumb_camera(slot);
            }
            run.applied_at = Some(now);
            run.shot = false;
        }
        Some(at) if !run.shot && now > at + 3.0 => {
            let path = dir.join(format!("{name}.png"));
            commands
                .spawn(bevy::render::view::screenshot::Screenshot::image(
                    target.0.clone(),
                ))
                .observe(bevy::render::view::screenshot::save_to_disk(path));
            run.shot = true;
        }
        Some(at) if run.shot && now > at + 4.5 => {
            run.next += 1;
            run.applied_at = None;
        }
        _ => {}
    }
}

#[bevy_main]
pub fn main() {
    let dir = assets_dir();
    let (catalog, materials) = load_data(&dir);
    let look = starting_look(&catalog);
    let animations = load_animations(&dir);
    let mut app = App::new();
    app
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
        .insert_resource(if thumbs_dir().is_some() {
            // The PNG is saved without alpha, so the picture gets the creator's own slate backdrop.
            ClearColor(Color::srgb(0.20, 0.25, 0.33))
        } else {
            ClearColor(Color::srgb(0.25, 0.3, 0.45))
        })
        .add_systems(
            Startup,
            move |commands: Commands,
                  images: ResMut<Assets<Image>>,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>| {
                setup(commands, images, meshes, materials, look.clone())
            },
        )
        .add_systems(
            Update,
            (
                wardrobe_input,
                toggles,
                pose_input,
                expression_input,
                prop_input,
                scripted_drink,
                scripted_hair_swap,
                log_render_stats,
                snapshot,
                thumbnails,
            ),
        );
    if let Some(animations) = animations {
        app.insert_resource(animations);
    }
    app.run();
}

/// A floor and a few boxes with ordinary `StandardMaterial`s, to try the scene style on
/// (`TOON_DEMO_SCENE=1` adds them; `TOON_DEMO_SCENE_STYLE=cartoon` turns the cartoon look on, K flips it).
fn spawn_demo_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let floor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.62, 0.45),
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 0.1, 6.0))),
        MeshMaterial3d(floor),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));
    let sofa = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.45, 0.8),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 0.5, 0.7))),
        MeshMaterial3d(sofa.clone()),
        Transform::from_xyz(1.6, 0.25, -1.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.6, 0.5, 0.2))),
        MeshMaterial3d(sofa),
        Transform::from_xyz(1.6, 0.75, -1.25),
    ));
    let wall = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.9, 0.95, 0.35),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 1.8, 0.1))),
        MeshMaterial3d(wall),
        Transform::from_xyz(-1.8, 0.9, -1.5),
    ));
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    look: AvatarLook,
) {
    if std::env::var("TOON_DEMO_SCENE").is_ok() {
        spawn_demo_scene(&mut commands, &mut meshes, &mut materials);
        commands.insert_resource(SceneStyle {
            cartoon: std::env::var("TOON_DEMO_SCENE_STYLE").is_ok_and(|v| v == "cartoon"),
        });
    }
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
    // `TOON_DEMO_NO_FOCUS=1` makes the avatar a plain one, so `TOON_AVATAR_TIER=low` shows the Low level.
    let mut avatar = commands.spawn((
        Avatar::new(look),
        start_pose(),
        AvatarHolding(start_prop()),
        AvatarFace {
            expression: start_expression(),
            blink: std::env::var("TOON_DEMO_NO_BLINK").is_err(),
        },
        AvatarWalkSpeed(1.3),
        Transform::from_xyz(x, 0.0, 0.0),
    ));
    if std::env::var("TOON_DEMO_NO_FOCUS").is_err() {
        avatar.insert(LodFocus);
    }
    let mut camera = commands.spawn((
        Camera3d::default(),
        // Toon colours are authored for display directly; skip tone mapping.
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        camera_transform(),
    ));
    if std::env::var("TOON_DEMO_SNAPSHOT").is_ok() || thumbs_dir().is_some() {
        let (width, height) = if thumbs_dir().is_some() {
            (THUMB_SIZE, THUMB_SIZE)
        } else {
            (900, 1300)
        };
        let target = images.add(Image::new_target_texture(
            width,
            height,
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

/// `TOON_DEMO_POSE=idle|walk|sit|lay` picks the starting pose.
fn start_pose() -> AvatarPose {
    match std::env::var("TOON_DEMO_POSE").as_deref() {
        Ok("walk") => AvatarPose::Walk,
        Ok("sit") => AvatarPose::Sit,
        Ok("lay") => AvatarPose::Lay,
        Ok("drink") => AvatarPose::Drink,
        Ok("drink_sit") => AvatarPose::DrinkSit,
        _ => AvatarPose::Idle,
    }
}

/// `TOON_DEMO_EXPR=happy|laugh|sad|angry|surprised` picks the starting expression.
fn start_expression() -> Expression {
    std::env::var("TOON_DEMO_EXPR")
        .ok()
        .and_then(|name| Expression::from_name(&name))
        .unwrap_or_default()
}

/// F cycles the expression.
fn expression_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut faces: Query<&mut AvatarFace, With<LodFocus>>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }
    for mut face in &mut faces {
        let at = Expression::ALL
            .iter()
            .position(|e| *e == face.expression)
            .unwrap_or(0);
        face.expression = Expression::ALL[(at + 1) % Expression::ALL.len()];
    }
}

/// `TOON_DEMO_HOLD=cup|can` makes the avatar start with a prop in the right hand.
fn start_prop() -> Option<HeldProp> {
    prop_by_name(std::env::var("TOON_DEMO_HOLD").ok()?.as_str())
}

fn prop_by_name(name: &str) -> Option<HeldProp> {
    match name {
        "cup" | "can" => Some(HeldProp::new(format!("props/{name}.glb"))),
        _ => None,
    }
}

/// V cycles the held prop (none, cup, can); C drinks from it.
fn prop_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut avatars: Query<(&mut AvatarHolding, &mut AvatarPose), With<LodFocus>>,
) {
    for (mut holding, mut pose) in &mut avatars {
        if keys.just_pressed(KeyCode::KeyV) {
            let next = match holding.0.as_ref().map(|p| p.scene.as_str()) {
                None => "cup",
                Some("props/cup.glb") => "can",
                _ => "",
            };
            holding.0 = prop_by_name(next);
        }
        if keys.just_pressed(KeyCode::KeyC)
            && holding.0.is_some()
            && let Some(drinking) = pose.drinking()
        {
            *pose = drinking;
        }
    }
}

/// `TOON_DEMO_DRINK_AT=<seconds>` starts the drinking gesture at that time (for screenshots).
fn scripted_drink(
    time: Res<Time>,
    mut poses: Query<&mut AvatarPose, With<LodFocus>>,
    mut done: Local<bool>,
) {
    let Some(at) = std::env::var("TOON_DEMO_DRINK_AT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    else {
        return;
    };
    if !*done && time.elapsed_secs() >= at {
        *done = true;
        for mut pose in &mut poses {
            if let Some(drinking) = pose.drinking() {
                *pose = drinking;
            }
        }
    }
}

/// `TOON_DEMO_HAIR_SWAP=<name>@<seconds>` changes the hair after the avatar is already on screen,
/// the way the character creator does (for captures of the swap).
fn scripted_hair_swap(
    time: Res<Time>,
    mut avatars: Query<&mut Avatar, With<LodFocus>>,
    mut done: Local<bool>,
) {
    let Some((name, at)) = std::env::var("TOON_DEMO_HAIR_SWAP").ok().and_then(|v| {
        v.split_once('@')
            .map(|(n, s)| (n.to_string(), s.parse::<f32>().ok()))
    }) else {
        return;
    };
    if !*done && at.is_some_and(|at| time.elapsed_secs() >= at) {
        *done = true;
        for mut avatar in &mut avatars {
            avatar.look.hair = Some(name.clone());
        }
    }
}

/// P cycles the pose: idle, walk, sit, lay.
fn pose_input(keys: Res<ButtonInput<KeyCode>>, mut poses: Query<&mut AvatarPose, With<LodFocus>>) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }
    for mut pose in &mut poses {
        *pose = match *pose {
            AvatarPose::Idle => AvatarPose::Walk,
            AvatarPose::Walk => AvatarPose::Sit,
            AvatarPose::Sit => AvatarPose::Lay,
            AvatarPose::Lay => AvatarPose::Idle,
            AvatarPose::Drink | AvatarPose::DrinkSit => AvatarPose::Idle,
        };
    }
}

/// O = outline, M = mesh merge, Q = quality tier.
fn toggles(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<ToonSettings>,
    mut scene: ResMut<SceneStyle>,
    mut merge: ResMut<MergeEnabled>,
    tier: Option<Res<DeviceTier>>,
    mut preference: ResMut<TierPreference>,
) {
    if keys.just_pressed(KeyCode::KeyK) {
        scene.cartoon = !scene.cartoon;
    }
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
