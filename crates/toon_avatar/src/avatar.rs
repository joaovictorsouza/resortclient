//! Avatars: one entity per character, a body scene underneath, garments re-pointed at the
//! body skeleton and head attachments (hair, hats, glasses).
//!
//! Every glTF of a body type carries the same 71-joint armature (identical bone names). A
//! garment is spawned as its own scene, then its skinned meshes are re-pointed at the body's
//! joint entities (matched by `Name`) and re-parented under the body; the garment's own
//! armature is despawned. Body parts covered by a garment are hidden (Unity's `BodyPartTag`
//! logic). Hair, hats and glasses carry their own small skeletons, so the whole scene is
//! parented to the body's head bone instead.
//!
//! Spawn an avatar with [`Avatar::new`]; change `Avatar::look` to change what it wears. Colours
//! live in the look, so they never leak between avatars.
//!
//! A *body* is one complete set of meshes of an avatar at one level of detail. When the level of
//! detail (or the body type) changes, the new body is built hidden next to the old one, and the
//! old one is removed only when the new one is complete (`settle_bodies`), so the avatar never
//! flickers or shows up half dressed.

use std::collections::HashMap;

use bevy::{
    gltf::GltfAssetLabel,
    mesh::{
        morph::{MeshMorphWeights, MorphWeights},
        skinning::SkinnedMesh,
    },
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    animation::AvatarPose,
    atlas::{AtlasBuilt, AtlasMaterial},
    catalog::{AvatarCatalog, AvatarLook, BodyType, CatalogEntry, Slot},
    face::AvatarFace,
    lod::AvatarLod,
    material::{AvatarMaterials, ToonMaterial},
    props::{AvatarHolding, PropApplied},
    quality::Quality,
};

const HEAD_BONE: &str = "CC_Base_Head";
/// Frames a complete body waits before it replaces the old one, so materials and merged meshes
/// built in the frames after the scene loads are in place.
const SETTLE_FRAMES: u8 = 3;

/// Body mesh node name (without the `F_`/`M_` prefix) -> Unity `BodyPartType` value.
const BODY_PARTS: &[(&str, u32)] = &[
    ("body_hips", 0),
    ("body_torso", 1),
    ("body_torso.001", 2),
    ("body_shoulders", 3),
    ("body_torso.002", 5),
    ("body_arms_lower", 20),
    ("body_arms_upper", 21),
    ("body_hands", 22),
    ("body_legs_upper", 30),
    ("body_legs_knee", 31),
    ("body_legs_lower", 32),
    ("body_feet", 33),
];

/// A character. Insert it on an entity (with a `Transform`) and the plugin builds the body,
/// the garments and the attachments underneath.
#[derive(Component, Clone, Debug)]
#[require(
    Transform,
    Visibility,
    Applied,
    AvatarLod,
    AvatarPose,
    AvatarFace,
    AvatarHolding,
    PropApplied
)]
pub struct Avatar {
    pub look: AvatarLook,
}

impl Avatar {
    pub fn new(look: AvatarLook) -> Self {
        Self { look }
    }
}

/// The body being built for an avatar, and the one it currently shows.
#[derive(Component, Default, Debug)]
pub struct Applied {
    /// The body the avatar wants (the newest one; may still be loading).
    current: Option<(Entity, BodyType, Quality)>,
    /// The complete body that is visible. It is `current` once that has settled.
    shown: Option<Entity>,
    items: HashMap<Slot, Option<String>>,
    /// Body, hair-or-hat name and glasses name of the attachments in place on `current`.
    attachments: Option<(Entity, Option<String>, Option<String>)>,
}

impl Applied {
    /// The newest body (the one the avatar is building or showing); props and attachments go on it.
    pub fn current_body(&self) -> Option<Entity> {
        self.current.map(|(body, _, _)| body)
    }

    /// The body currently shown to the player, if there is one yet.
    pub fn shown_body(&self) -> Option<Entity> {
        self.shown
    }
}

/// One complete set of meshes of an avatar at one level of detail.
#[derive(Component)]
#[require(AvatarMaterials, AtlasBuilt)]
pub struct Body {
    pub avatar: Entity,
    pub body_type: BodyType,
    pub lod: Quality,
}

/// Marks the body root once its scene has finished spawning.
#[derive(Component)]
pub struct BodyReady;

/// Counts the frames a complete body has been waiting to be shown.
#[derive(Component)]
struct Settling(u8);

/// A garment scene root that is waiting to be attached to the body.
#[derive(Component)]
struct PendingGarment {
    body: Entity,
    slot: Slot,
    hides: Vec<u32>,
}

/// A skinned mesh that was moved onto the body.
#[derive(Component)]
pub struct Equipped {
    pub body: Entity,
    pub slot: Slot,
    pub hides: Vec<u32>,
}

/// A body mesh that can be hidden when a garment covers its part.
#[derive(Component)]
pub struct BodyPart(pub u32);

/// A head attachment scene (hair, hat or glasses).
#[derive(Component)]
pub struct Attachment {
    pub body: Entity,
    /// Hair, hat or glasses.
    pub slot: Slot,
}

/// Marks the roots (and moved meshes) that belong to a body, so any descendant can find its
/// body by walking up the hierarchy.
#[derive(Component)]
pub struct OwnedBy(pub Entity);

/// The body an entity belongs to: the entity itself or the nearest ancestor with `OwnedBy`.
pub(crate) fn owner_of(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    owners: &Query<&OwnedBy>,
) -> Option<Entity> {
    loop {
        if let Ok(owner) = owners.get(entity) {
            return Some(owner.0);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}

pub struct AvatarPlugin;

impl Plugin for AvatarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AvatarCatalog>()
            .add_systems(
                Update,
                (
                    sync_avatars,
                    attach_garments,
                    tag_body_parts,
                    hide_covered_parts,
                    sync_attachments,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, settle_bodies);
    }
}

fn spawn_body(
    commands: &mut Commands,
    assets: &AssetServer,
    avatar: Entity,
    body_type: BodyType,
    lod: Quality,
    path: &str,
) -> Entity {
    let id = commands
        .spawn((
            Body {
                avatar,
                body_type,
                lod,
            },
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(lod.model_path(path)))),
            ChildOf(avatar),
            // Hidden until it is complete; `settle_bodies` shows it.
            Visibility::Hidden,
        ))
        .observe(|ev: On<WorldInstanceReady>, mut commands: Commands| {
            commands.entity(ev.entity).try_insert(BodyReady);
        })
        .id();
    commands.entity(id).try_insert(OwnedBy(id));
    id
}

fn spawn_garment(
    commands: &mut Commands,
    assets: &AssetServer,
    body: Entity,
    lod: Quality,
    slot: Slot,
    entry: &CatalogEntry,
) {
    for glb in &entry.glbs {
        commands.spawn((
            PendingGarment {
                body,
                slot,
                hides: entry.hides.clone(),
            },
            OwnedBy(body),
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(lod.model_path(glb)))),
            // Hidden until its meshes are moved so the garment armature never shows.
            Visibility::Hidden,
        ));
    }
}

/// Brings the bodies and the worn garments of every avatar in line with its look and level of
/// detail.
#[allow(clippy::too_many_arguments)]
fn sync_avatars(
    mut commands: Commands,
    assets: Res<AssetServer>,
    catalog: Res<AvatarCatalog>,
    mut avatars: Query<(Entity, Ref<Avatar>, Ref<AvatarLod>, &mut Applied)>,
    equipped: Query<(Entity, &Equipped)>,
    pending: Query<(Entity, &PendingGarment)>,
) {
    for (avatar_entity, avatar, lod, mut applied) in &mut avatars {
        if !avatar.is_changed()
            && !lod.is_changed()
            && !catalog.is_changed()
            && applied.current.is_some()
        {
            continue;
        }
        let look = &avatar.look;
        let body_catalog = catalog.of(look.body);
        if body_catalog.body.is_empty() {
            continue;
        }
        let wanted = (look.body, lod.level);

        if applied.current.map(|(_, b, q)| (b, q)) != Some(wanted) {
            // A body that was never shown is dropped: only the newest request matters. The shown
            // body stays until the new one is complete.
            if let Some((old, _, _)) = applied.current
                && applied.shown != Some(old)
            {
                commands.entity(old).try_despawn();
                for (entity, p) in &pending {
                    if p.body == old {
                        commands.entity(entity).try_despawn();
                    }
                }
            }
            let body = spawn_body(
                &mut commands,
                &assets,
                avatar_entity,
                wanted.0,
                wanted.1,
                &body_catalog.body,
            );
            applied.current = Some((body, wanted.0, wanted.1));
            applied.items.clear();
            applied.attachments = None;
        }
        let Some((body, _, level)) = applied.current else {
            continue;
        };

        for slot in Slot::ALL.into_iter().filter(|s| !s.is_attachment()) {
            let mut want = look.effective(slot).map(str::to_string);
            // A body always needs a face: an empty or unknown one falls back to the first.
            if slot == Slot::Face
                && want
                    .as_deref()
                    .is_none_or(|n| body_catalog.find(Slot::Face, n).is_none())
            {
                want = body_catalog.face.first().map(|e| e.name.clone());
            }
            if applied.items.get(&slot) == Some(&want) {
                continue;
            }
            for (entity, eq) in &equipped {
                if eq.body == body && eq.slot == slot {
                    commands.entity(entity).try_despawn();
                }
            }
            for (entity, p) in &pending {
                if p.body == body && p.slot == slot {
                    commands.entity(entity).try_despawn();
                }
            }
            if let Some(entry) = want.as_deref().and_then(|n| body_catalog.find(slot, n)) {
                spawn_garment(&mut commands, &assets, body, level, slot, entry);
            }
            applied.items.insert(slot, want);
        }
    }
}

/// Moves the skinned meshes of every ready garment onto its body skeleton.
#[allow(clippy::too_many_arguments)]
fn attach_garments(
    mut commands: Commands,
    bodies: Query<(), (With<Body>, With<BodyReady>)>,
    pending: Query<(Entity, &PendingGarment)>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut skins: Query<&mut SkinnedMesh>,
    mut mesh_morphs: Query<&mut MeshMorphWeights>,
    morphs: Query<&MorphWeights>,
) {
    if pending.is_empty() {
        return;
    }
    // Body entity -> (joint name -> joint entity), built lazily.
    let mut joint_maps: HashMap<Entity, HashMap<String, Entity>> = HashMap::new();

    for (root, garment) in &pending {
        let body = garment.body;
        if !bodies.contains(body) {
            continue;
        }
        // The scene is spawned once the root has descendants carrying a skin.
        let skinned: Vec<Entity> = children
            .iter_descendants(root)
            .filter(|e| skins.contains(*e))
            .collect();
        if skinned.is_empty() {
            continue;
        }
        let body_joints = joint_maps.entry(body).or_insert_with(|| {
            children
                .iter_descendants(body)
                .filter_map(|e| names.get(e).ok().map(|n| (n.as_str().to_string(), e)))
                .collect()
        });

        for mesh in skinned {
            let mut skin = skins.get_mut(mesh).unwrap();
            let mut missing = 0;
            for joint in skin.joints.iter_mut() {
                let name = names.get(*joint).map(|n| n.as_str()).unwrap_or("");
                match body_joints.get(name) {
                    Some(target) => *joint = *target,
                    None => missing += 1,
                }
            }
            if missing > 0 {
                warn!("garment mesh {mesh:?}: {missing} joints not found on body");
            }
            // glTF meshes reference the `MorphWeights` of their parent node, which is
            // despawned with the garment scene: copy the weights onto the mesh instead.
            if let Ok(mut mm) = mesh_morphs.get_mut(mesh)
                && let MeshMorphWeights::Reference(node) = *mm
                && let Ok(mw) = morphs.get(node)
            {
                *mm = MeshMorphWeights::Value {
                    weights: mw.weights().to_vec(),
                };
            }
            commands.entity(mesh).try_remove::<ChildOf>().try_insert((
                ChildOf(body),
                OwnedBy(body),
                Equipped {
                    body,
                    slot: garment.slot,
                    hides: garment.hides.clone(),
                },
            ));
        }
        // Drops the now empty garment scene together with its armature.
        commands.entity(root).try_despawn();
    }
}

/// Tags the body's part nodes (matched by node name) as they spawn.
fn tag_body_parts(mut commands: Commands, nodes: Query<(Entity, &Name), Added<Name>>) {
    // Visibility::Hidden on the glTF node also hides its primitive children.
    for (entity, name) in &nodes {
        let name = name.as_str();
        let name = name
            .strip_prefix("F_")
            .or_else(|| name.strip_prefix("M_"))
            .unwrap_or(name);
        if let Some((_, ty)) = BODY_PARTS.iter().find(|(n, _)| *n == name) {
            commands.entity(entity).try_insert(BodyPart(*ty));
        }
    }
}

/// Hides every body part covered by a garment of the same body (Unity's `BodyPartTag` logic).
fn hide_covered_parts(
    equipped: Query<&Equipped>,
    parents: Query<&ChildOf>,
    owners: Query<&OwnedBy>,
    mut parts: Query<(Entity, &BodyPart, &mut Visibility)>,
) {
    for (entity, part, mut vis) in &mut parts {
        let Some(body) = owner_of(entity, &parents, &owners) else {
            continue;
        };
        let covered = equipped
            .iter()
            .any(|e| e.body == body && e.hides.contains(&part.0));
        let target = if covered {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != target {
            *vis = target;
        }
    }
}

/// Product of the local scales from `entity` up to the root.
fn world_scale(mut entity: Entity, transforms: &Query<(&Transform, Option<&ChildOf>)>) -> f32 {
    let mut scale = 1.0;
    while let Ok((t, parent)) = transforms.get(entity) {
        scale *= t.scale.x;
        match parent {
            Some(p) => entity = p.parent(),
            None => break,
        }
    }
    scale
}

/// Hair, hat and glasses: scenes parented to the head bone of the body.
#[allow(clippy::too_many_arguments)]
fn sync_attachments(
    mut commands: Commands,
    assets: Res<AssetServer>,
    catalog: Res<AvatarCatalog>,
    mut avatars: Query<(&Avatar, &mut Applied)>,
    bodies: Query<(), (With<Body>, With<BodyReady>)>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<(&Transform, Option<&ChildOf>)>,
    existing: Query<(Entity, &Attachment)>,
) {
    for (avatar, mut applied) in &mut avatars {
        let Some((body, body_type, level)) = applied.current else {
            continue;
        };
        if !bodies.contains(body) {
            continue;
        }
        let look = &avatar.look;
        // A hat replaces the hair (its model already includes the hair that fits under it).
        let hair_or_hat = look
            .item(Slot::Hat)
            .or(look.item(Slot::Hair))
            .map(str::to_string);
        let glasses = look.item(Slot::Glasses).map(str::to_string);
        let want = (body, hair_or_hat.clone(), glasses.clone());
        if applied.attachments.as_ref() == Some(&want) {
            continue;
        }
        let Some(head) = children
            .iter_descendants(body)
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == HEAD_BONE))
        else {
            continue;
        };
        for (entity, a) in &existing {
            if a.body == body {
                commands.entity(entity).try_despawn();
            }
        }

        // Authored in metres; cancel the scale the skeleton carries down to the head bone.
        let local = Transform::from_scale(Vec3::splat(1.0 / world_scale(head, &transforms)));
        let body_catalog = catalog.of(body_type);
        let hat_name = look.item(Slot::Hat);
        let hair_entry = hair_or_hat.as_deref().and_then(|n| {
            if hat_name == Some(n) {
                body_catalog.find(Slot::Hat, n)
            } else {
                body_catalog.find(Slot::Hair, n)
            }
        });
        let glasses_entry = glasses
            .as_deref()
            .and_then(|n| body_catalog.find(Slot::Glasses, n));
        let hair_slot = if hat_name.is_some() {
            Slot::Hat
        } else {
            Slot::Hair
        };
        for (slot, entry) in [(hair_slot, hair_entry), (Slot::Glasses, glasses_entry)] {
            let Some(entry) = entry else {
                continue;
            };
            for glb in &entry.glbs {
                commands.spawn((
                    Attachment { body, slot },
                    OwnedBy(body),
                    WorldAssetRoot(
                        assets.load(GltfAssetLabel::Scene(0).from_asset(level.model_path(glb))),
                    ),
                    local,
                    ChildOf(head),
                ));
            }
        }
        applied.attachments = Some(want);
    }
}

/// Shows a complete new body and removes the one it replaces.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn settle_bodies(
    mut commands: Commands,
    assets: Res<AssetServer>,
    toon_materials: Res<Assets<ToonMaterial>>,
    atlas_materials: Res<Assets<AtlasMaterial>>,
    mut avatars: Query<&mut Applied>,
    mut waiting: Query<
        (&AvatarMaterials, &AtlasBuilt, Option<&mut Settling>),
        (With<Body>, With<BodyReady>),
    >,
    pending: Query<&PendingGarment>,
) {
    for mut applied in &mut avatars {
        let Some((current, _, _)) = applied.current else {
            continue;
        };
        if applied.shown == Some(current) {
            continue;
        }
        let Ok((materials, atlas, settling)) = waiting.get_mut(current) else {
            continue;
        };
        let complete = !pending.iter().any(|p| p.body == current)
            && applied.attachments.as_ref().is_some_and(|a| a.0 == current)
            && atlas.is_built()
            && materials.textures_loaded(&assets, &toon_materials)
            && atlas.textures_loaded(&assets, &atlas_materials);
        if !complete {
            if let Some(mut s) = settling {
                s.0 = 0;
            }
            continue;
        }
        match settling {
            None => {
                commands.entity(current).try_insert(Settling(1));
            }
            Some(mut s) if s.0 < SETTLE_FRAMES => s.0 += 1,
            Some(_) => {
                commands
                    .entity(current)
                    .try_insert(Visibility::Inherited)
                    .try_remove::<Settling>();
                if let Some(old) = applied.shown.take() {
                    commands.entity(old).try_despawn();
                }
                debug!("body {current:?} settled");
                applied.shown = Some(current);
            }
        }
    }
}
