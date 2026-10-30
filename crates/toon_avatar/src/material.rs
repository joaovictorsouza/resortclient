//! Toon character material (port of Unity's `ShaderGraph_CharacterToon`) and the
//! per-glTF-material data exported by the asset pipeline.
//!
//! Colours chosen by the player live in each avatar's [`AvatarLook`], and every avatar owns
//! its materials ([`AvatarMaterials`]), so a colour never leaks to another avatar that wears
//! the same item.

use std::collections::HashMap;

use bevy::{
    asset::embedded_asset,
    gltf::GltfMaterialName,
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use serde::Deserialize;

use crate::{
    avatar::{Attachment, Avatar, Body, Equipped, OwnedBy, owner_of},
    catalog::{AvatarLook, Slot},
    quality::Quality,
};

pub(crate) const SHADER: &str = "embedded://toon_avatar/shaders/toon.wesl";
/// Mobile builds (and the `basis_textures` feature) use the GPU-compressed copies, one folder
/// per quality tier.
#[cfg(any(target_os = "ios", target_os = "android", feature = "basis_textures"))]
pub(crate) const COMPRESSED: bool = true;
#[cfg(not(any(target_os = "ios", target_os = "android", feature = "basis_textures")))]
pub(crate) const COMPRESSED: bool = false;

pub(crate) const RAMP: &str = "characters/textures/ColorRamp.png";

pub(crate) const FLAG_FLAT: u32 = 1;
pub(crate) const FLAG_HAS_NORMAL: u32 = 2;
pub(crate) const FLAG_HAS_MASK: u32 = 4;

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ToonParams {
    pub a1: Vec4,
    pub a2: Vec4,
    pub b1: Vec4,
    pub b2: Vec4,
    pub c1: Vec4,
    pub c2: Vec4,
    pub flat_color: Vec4,
    pub rim_color: Vec4,
    pub outline_color: Vec4,
    pub mask_remap: Vec2,
    pub mask_factor: f32,
    pub thickness: f32,
    pub normal_adjust: f32,
    pub alpha_cutoff: f32,
    pub flags: u32,
    /// Active outline width; 0 disables the outline.
    pub outline_thickness: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ToonMaterial {
    #[uniform(0)]
    pub params: ToonParams,
    #[texture(1)]
    #[sampler(2)]
    pub mask: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub normal: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    pub ramp: Option<Handle<Image>>,
}

impl Material for ToonMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// One entry of `materials.json`.
#[derive(Deserialize)]
pub struct MaterialDef {
    pub(crate) mode: String,
    pub(crate) mask: Option<String>,
    pub(crate) normal: Option<String>,
    /// The mask texture is a colour image (sRGB), not data. Used by custom flat-colour items.
    #[serde(default)]
    pub(crate) mask_srgb: bool,
    a1: [f32; 4],
    a2: [f32; 4],
    b1: [f32; 4],
    b2: [f32; 4],
    c1: [f32; 4],
    c2: [f32; 4],
    flat_color: [f32; 4],
    rim_color: [f32; 4],
    mask_remap: [f32; 2],
    mask_factor: f32,
    thickness: f32,
    normal_adjust: f32,
    alpha_cutoff: f32,
    outline_color: [f32; 4],
    outline_thickness: f32,
}

/// The material definitions by glTF material name. Insert it before spawning avatars.
#[derive(Resource, Default)]
pub struct MaterialDefs(HashMap<String, MaterialDef>);

/// Number of customizable colour slots: A1, A2, B1, B2, C1, C2.
pub const COLOR_SLOTS: usize = 6;

impl MaterialDefs {
    /// Parses one or more `materials.json` documents; later ones override earlier ones.
    pub fn from_json(documents: &[&str]) -> Result<Self, serde_json::Error> {
        let mut defs = HashMap::new();
        for document in documents {
            defs.extend(serde_json::from_str::<HashMap<String, MaterialDef>>(
                document,
            )?);
        }
        Ok(Self(defs))
    }

    /// Default sRGB colour of a customization slot, if the material exists.
    pub fn color(&self, name: &str, slot: usize) -> Option<[f32; 3]> {
        let d = self.0.get(name)?;
        let c = [d.a1, d.a2, d.b1, d.b2, d.c1, d.c2][slot];
        Some([c[0], c[1], c[2]])
    }

    pub(crate) fn get(&self, name: &str) -> Option<&MaterialDef> {
        self.0.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }
}

/// Settings shared by every avatar.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ToonSettings {
    /// Whether the fake outline is drawn (it also needs a quality tier that allows it).
    pub outline: bool,
}

impl Default for ToonSettings {
    fn default() -> Self {
        Self { outline: true }
    }
}

/// The toon materials of one body (one avatar at one level of detail), by glTF material name.
#[derive(Component, Default)]
pub struct AvatarMaterials {
    /// glTF material name -> (material, slot of the item it belongs to).
    map: HashMap<String, (Handle<ToonMaterial>, Option<Slot>)>,
}

impl AvatarMaterials {
    /// Whether every texture of the materials has finished loading.
    pub(crate) fn textures_loaded(
        &self,
        assets: &AssetServer,
        materials: &Assets<ToonMaterial>,
    ) -> bool {
        self.map.values().all(|(handle, _)| {
            materials.get(handle).is_none_or(|m| {
                [&m.mask, &m.normal, &m.ramp]
                    .into_iter()
                    .flatten()
                    .all(|image| assets.is_loaded_with_dependencies(image.id()))
            })
        })
    }
}

fn set_slot(params: &mut ToonParams, slot: usize, c: [f32; 3]) {
    let v = v4([c[0], c[1], c[2], 0.0]);
    match slot {
        0 => params.a1 = v,
        1 => params.a2 = v,
        2 => params.b1 = v,
        3 => params.b2 = v,
        4 => params.c1 = v,
        _ => params.c2 = v,
    }
}

pub struct ToonMaterialPlugin;

impl Plugin for ToonMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/toon.wesl");
        app.add_plugins(MaterialPlugin::<ToonMaterial>::default())
            .init_resource::<MaterialDefs>()
            .init_resource::<ToonSettings>()
            .add_systems(
                Update,
                (assign_toon_materials, apply_look_colors, apply_settings).chain(),
            );
    }
}

/// Path of a texture listed in `materials.json` (relative to `characters/textures/`).
fn texture_path(rel: &str, quality: Quality) -> String {
    if COMPRESSED {
        format!(
            "characters/textures_basis/{}/{}",
            quality.texture_size(),
            rel.replace(".png", ".basis")
        )
    } else {
        format!("characters/textures/{rel}")
    }
}

pub(crate) fn load_texture(assets: &AssetServer, path: &str, srgb: bool) -> Handle<Image> {
    assets
        .load_builder()
        .with_settings(move |s: &mut ImageLoaderSettings| {
            s.is_srgb = srgb;
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(path.to_string())
}

/// Material colours are stored as sRGB picker values. Unity (linear colour space) converts
/// them to linear before they reach the shader, so do the same.
fn v4(c: [f32; 4]) -> Vec4 {
    let linear = Color::srgb(c[0], c[1], c[2]).to_linear();
    Vec4::new(linear.red, linear.green, linear.blue, c[3])
}

/// Shader parameters of a material, with the avatar's colour choices applied.
pub(crate) fn toon_params(
    def: &MaterialDef,
    name: &str,
    flags: u32,
    outline_on: bool,
    look: &AvatarLook,
    item_slot: Option<Slot>,
) -> ToonParams {
    let mut params = ToonParams {
        a1: v4(def.a1),
        a2: v4(def.a2),
        b1: v4(def.b1),
        b2: v4(def.b2),
        c1: v4(def.c1),
        c2: v4(def.c2),
        flat_color: v4(def.flat_color),
        rim_color: v4(def.rim_color),
        outline_color: v4(def.outline_color),
        mask_remap: Vec2::from_array(def.mask_remap),
        mask_factor: def.mask_factor,
        thickness: def.thickness,
        normal_adjust: def.normal_adjust,
        alpha_cutoff: def.alpha_cutoff,
        flags,
        outline_thickness: if outline_on {
            def.outline_thickness
        } else {
            0.0
        },
    };
    for slot in 0..COLOR_SLOTS {
        if let Some(c) = look.color_of(name, item_slot, slot) {
            set_slot(&mut params, slot, c);
        }
    }
    params
}

/// Replaces the glTF `StandardMaterial` of every mesh with the toon material of its body that
/// matches its glTF material name.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn assign_toon_materials(
    mut commands: Commands,
    meshes: Query<
        (Entity, &GltfMaterialName),
        (With<Mesh3d>, With<MeshMaterial3d<StandardMaterial>>),
    >,
    parents: Query<&ChildOf>,
    owners: Query<&OwnedBy>,
    mut bodies: Query<(&Body, &mut AvatarMaterials)>,
    avatars: Query<&Avatar>,
    equipped: Query<&Equipped>,
    attachments: Query<&Attachment>,
    defs: Res<MaterialDefs>,
    assets: Res<AssetServer>,
    mut toon_materials: ResMut<Assets<ToonMaterial>>,
    settings: Res<ToonSettings>,
) {
    for (entity, name) in &meshes {
        let Some(owner) = owner_of(entity, &parents, &owners) else {
            continue;
        };
        let Some(def) = defs.get(name.0.as_str()) else {
            continue;
        };
        let Ok((body, mut materials)) = bodies.get_mut(owner) else {
            continue;
        };
        let Ok(avatar) = avatars.get(body.avatar) else {
            continue;
        };
        let quality = body.lod;
        // Skin and face pieces are drawn through the atlas material instead (see atlas.rs).
        if crate::atlas::piece(&name.0, quality).is_some() {
            commands
                .entity(entity)
                .try_remove::<MeshMaterial3d<StandardMaterial>>()
                .try_insert(crate::atlas::AtlasSource);
            continue;
        }
        let item_slot = item_slot_of(entity, &parents, &equipped, &attachments);
        let handle = materials
            .map
            .entry(name.0.clone())
            .or_insert_with(|| {
                let mut flags = 0;
                if def.mode == "flat" {
                    flags |= FLAG_FLAT;
                }
                if def.mask.is_some() {
                    flags |= FLAG_HAS_MASK;
                }
                let normal = def.normal.as_ref().filter(|_| quality.normal_maps());
                if normal.is_some() {
                    flags |= FLAG_HAS_NORMAL;
                }
                let handle = toon_materials.add(ToonMaterial {
                    params: toon_params(
                        def,
                        &name.0,
                        flags,
                        settings.outline && quality.outline(),
                        &avatar.look,
                        item_slot,
                    ),
                    // Mask and normal maps hold data, not colour: sample them linearly.
                    mask: def
                        .mask
                        .as_ref()
                        .map(|p| load_texture(&assets, &texture_path(p, quality), def.mask_srgb)),
                    normal: normal.map(|p| load_texture(&assets, &texture_path(p, quality), false)),
                    ramp: Some(load_texture(&assets, RAMP, true)),
                });
                (handle, item_slot)
            })
            .0
            .clone();
        commands
            .entity(entity)
            .try_remove::<MeshMaterial3d<StandardMaterial>>()
            .try_insert(MeshMaterial3d(handle));
    }
}

/// Pushes an avatar's colour choices into the materials of its bodies (and restores defaults on
/// reset).
fn apply_look_colors(
    defs: Res<MaterialDefs>,
    bodies: Query<(&Body, &AvatarMaterials)>,
    avatars: Query<Ref<Avatar>>,
    mut toon_materials: ResMut<Assets<ToonMaterial>>,
) {
    for (body, materials) in &bodies {
        let Ok(avatar) = avatars.get(body.avatar) else {
            continue;
        };
        if !avatar.is_changed() {
            continue;
        }
        for (name, (handle, item_slot)) in &materials.map {
            let Some(mut mat) = toon_materials.get_mut(handle) else {
                continue;
            };
            for slot in 0..COLOR_SLOTS {
                let color = avatar
                    .look
                    .color_of(name, *item_slot, slot)
                    .or_else(|| defs.color(name, slot));
                if let Some(c) = color {
                    set_slot(&mut mat.params, slot, c);
                }
            }
        }
    }
}

/// Applies the outline setting (and the level of detail's permission) to every body material.
fn apply_settings(
    settings: Res<ToonSettings>,
    defs: Res<MaterialDefs>,
    bodies: Query<(&Body, &AvatarMaterials)>,
    mut toon_materials: ResMut<Assets<ToonMaterial>>,
) {
    if !settings.is_changed() {
        return;
    }
    for (body, materials) in &bodies {
        for (name, (handle, _)) in &materials.map {
            if let (Some(def), Some(mut mat)) = (defs.get(name), toon_materials.get_mut(handle)) {
                mat.params.outline_thickness = if settings.outline && body.lod.outline() {
                    def.outline_thickness
                } else {
                    0.0
                };
            }
        }
    }
}

/// The slot of the item a mesh belongs to: a garment (`Equipped`) or a head attachment.
fn item_slot_of(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    equipped: &Query<&Equipped>,
    attachments: &Query<&Attachment>,
) -> Option<Slot> {
    loop {
        if let Ok(e) = equipped.get(entity) {
            return Some(e.slot);
        }
        if let Ok(a) = attachments.get(entity) {
            return Some(a.slot);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}
