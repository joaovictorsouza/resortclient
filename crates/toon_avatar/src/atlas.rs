//! Skin + face atlas: the body parts that show skin and every piece of the head (face, eyes,
//! mouth, lashes, brows, highlight) are drawn in ONE draw call.
//!
//! Offline, `tools/texconv/src/bin/atlas.rs` packs their textures into one atlas per body type
//! and tier. At runtime the pieces are merged into a single mesh whose UVs are remapped into
//! the atlas cells, with the piece (material) id stored per vertex in the vertex colour. The
//! shader (`toon_atlas.wesl`) uses that id to pick the piece's parameters from an array, so
//! colours and the colour customization keep working per piece. Morph targets are merged too
//! (pieces without them get zeros) and the weights are forwarded from the original meshes.
//!
//! The cell layout must match the packer.

use bevy::{
    asset::embedded_asset,
    gltf::GltfMaterialName,
    mesh::{VertexAttributeValues, morph::MeshMorphWeights, skinning::SkinnedMesh},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use crate::{
    avatar::{Avatar, Body, BodyPart, Equipped, OwnedBy, owner_of},
    catalog::AvatarLook,
    material::{
        COMPRESSED, FLAG_FLAT, FLAG_HAS_MASK, FLAG_HAS_NORMAL, MaterialDefs, RAMP, ToonParams,
        ToonSettings, load_texture, toon_params,
    },
    quality::Quality,
};

const SHADER: &str = "embedded://toon_avatar/shaders/toon_atlas.wesl";
const ITEMS: usize = 8;
/// Number of distinct pieces; the rest of the uniform array repeats piece 0.
const PIECES: usize = 7;

/// Where a glTF material lives in the atlas.
pub struct Piece {
    /// Index into the parameter array (stored in the vertex colour).
    pub index: usize,
    /// Atlas cell as fractions: x, y, width, height.
    cell: [f32; 4],
    /// UV of the cell's origin in the source mesh (the body uses the second UDIM tile, u in 1..2).
    origin: [f32; 2],
    /// Flat-colour pieces sample one point of the white cell.
    collapse: bool,
    /// Whether the atlas holds a normal map for this piece.
    has_normal: bool,
}

/// Whether a glTF material belongs to the head (face skin, eyes, brows, lashes, mouth).
fn is_head_piece(name: &str) -> bool {
    matches!(
        name,
        "mat_face_F"
            | "mat_face_M"
            | "mat_eye"
            | "mat_mouth"
            | "mat_eye_highlight"
            | "mat_eyelashes"
            | "mat_eyebrows"
    )
}

/// The atlas cell of a material when it is drawn through the atlas at `quality`. The head is
/// left out where the face has blend shapes (blink and expressions): a merged mesh would not
/// show them, so those pieces keep their own meshes.
pub fn piece(name: &str, quality: Quality) -> Option<Piece> {
    if is_head_piece(name) && quality.keeps_morphs() {
        return None;
    }
    piece_info(name)
}

/// The atlas cell of a material, whatever the quality.
fn piece_info(name: &str) -> Option<Piece> {
    let p = |index, cell, origin, collapse, has_normal| Piece {
        index,
        cell,
        origin,
        collapse,
        has_normal,
    };
    match name {
        "mat_face_F" | "mat_face_M" => Some(p(0, [0.0, 0.0, 0.5, 0.5], [0.0, 0.0], false, true)),
        "mat_body_F" | "mat_body_M" => Some(p(1, [0.5, 0.0, 0.5, 0.5], [1.0, 0.0], false, true)),
        "mat_eye" => Some(p(2, [0.0, 0.5, 0.25, 0.25], [0.0, 0.0], false, false)),
        "mat_mouth" => Some(p(3, [0.25, 0.5, 0.25, 0.25], [0.0, 0.0], false, true)),
        "mat_eye_highlight" => Some(p(4, [0.0, 0.75, 0.25, 0.25], [0.0, 0.0], true, false)),
        "mat_eyelashes" => Some(p(5, [0.0, 0.75, 0.25, 0.25], [0.0, 0.0], true, false)),
        "mat_eyebrows" => Some(p(6, [0.0, 0.75, 0.25, 0.25], [0.0, 0.0], true, false)),
        _ => None,
    }
}

/// glTF material name of the piece with the given index.
fn piece_name(index: usize, gender: char) -> String {
    match index {
        0 => format!("mat_face_{gender}"),
        1 => format!("mat_body_{gender}"),
        2 => "mat_eye".into(),
        3 => "mat_mouth".into(),
        4 => "mat_eye_highlight".into(),
        5 => "mat_eyelashes".into(),
        _ => "mat_eyebrows".into(),
    }
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct AtlasParams {
    pub items: [ToonParams; ITEMS],
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct AtlasMaterial {
    #[uniform(0)]
    pub params: AtlasParams,
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

impl Material for AtlasMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// A skin/face piece (from the body or head scene) that is drawn through the atlas.
#[derive(Component)]
pub struct AtlasSource;

/// The merged mesh that replaces the atlas pieces.
#[derive(Component)]
pub(crate) struct AtlasMerged {
    pub(crate) body: Entity,
    // Keeps the merged mesh asset alive.
    _mesh: Handle<Mesh>,
}

/// State of the atlas material and merged mesh of one body.
#[derive(Component, Default)]
pub struct AtlasBuilt {
    signature: Vec<Entity>,
    quality: Option<Quality>,
    gender: Option<char>,
    material: Option<Handle<AtlasMaterial>>,
    /// Quality the current material (and its textures) was built for.
    material_quality: Option<Quality>,
}

impl AtlasBuilt {
    /// Whether the merged mesh and its material exist.
    pub(crate) fn is_built(&self) -> bool {
        self.material.is_some() && !self.signature.is_empty()
    }

    /// Whether every texture of the atlas material has finished loading.
    pub(crate) fn textures_loaded(
        &self,
        assets: &AssetServer,
        materials: &Assets<AtlasMaterial>,
    ) -> bool {
        self.material
            .as_ref()
            .and_then(|handle| materials.get(handle))
            .is_none_or(|m| {
                [&m.mask, &m.normal, &m.ramp]
                    .into_iter()
                    .flatten()
                    .all(|image| assets.is_loaded_with_dependencies(image.id()))
            })
    }
}

pub struct AtlasPlugin;

impl Plugin for AtlasPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/toon_atlas.wesl");
        app.add_plugins(MaterialPlugin::<AtlasMaterial>::default())
            .add_systems(Update, (update_atlas_params, merge_atlas).chain());
    }
}

fn atlas_params(
    gender: char,
    defs: &MaterialDefs,
    quality: Quality,
    outline_on: bool,
    look: &AvatarLook,
) -> Option<AtlasParams> {
    let mut items = Vec::with_capacity(ITEMS);
    for index in 0..PIECES {
        let name = piece_name(index, gender);
        let def = defs.get(&name)?;
        let info = piece_info(&name)?;
        // The atlas always supplies a mask (the white cell for flat pieces).
        let mut flags = FLAG_HAS_MASK;
        if def.mode == "flat" || def.mask.is_none() {
            flags |= FLAG_FLAT;
        }
        if info.has_normal && def.normal.is_some() && quality.normal_maps() {
            flags |= FLAG_HAS_NORMAL;
        }
        items.push(toon_params(
            def,
            &name,
            flags,
            outline_on && quality.outline(),
            look,
        ));
    }
    while items.len() < ITEMS {
        items.push(items[0]);
    }
    Some(AtlasParams {
        items: items.try_into().ok()?,
    })
}

fn atlas_texture(gender: char, quality: Quality, kind: &str) -> String {
    let tier = quality.texture_size();
    if COMPRESSED {
        format!("characters/textures_basis/{tier}/atlas/atlas_{gender}_{kind}.basis")
    } else {
        format!("characters/textures_atlas_png/{tier}/atlas_{gender}_{kind}.png")
    }
}

/// Keeps the atlas material in line with the avatar's colour choices and the outline setting.
fn update_atlas_params(
    defs: Res<MaterialDefs>,
    settings: Res<ToonSettings>,
    bodies: Query<(&Body, &AtlasBuilt)>,
    avatars: Query<Ref<Avatar>>,
    mut materials: ResMut<Assets<AtlasMaterial>>,
) {
    for (body, built) in &bodies {
        let Ok(avatar) = avatars.get(body.avatar) else {
            continue;
        };
        if !(avatar.is_changed() || settings.is_changed()) {
            continue;
        }
        let Some(handle) = &built.material else {
            continue;
        };
        if let (Some(mut material), Some(params)) = (
            materials.get_mut(handle),
            atlas_params(
                body.body_type.letter(),
                &defs,
                body.lod,
                settings.outline,
                &avatar.look,
            ),
        ) {
            material.params = params;
        }
    }
}

struct SourcePiece {
    mesh: Mesh,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn merge_atlas(
    mut commands: Commands,
    assets: Res<AssetServer>,
    defs: Res<MaterialDefs>,
    settings: Res<ToonSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<AtlasMaterial>>,
    equipped: Query<&Equipped>,
    sources: Query<
        (
            Entity,
            &Mesh3d,
            &SkinnedMesh,
            &GltfMaterialName,
            Option<&BodyPart>,
            Option<&MeshMorphWeights>,
        ),
        With<AtlasSource>,
    >,
    parents: Query<&ChildOf>,
    owners: Query<&OwnedBy>,
    parts: Query<&BodyPart>,
    merged: Query<(Entity, &AtlasMerged)>,
    mut bodies: Query<(Entity, &Body, &mut AtlasBuilt)>,
    avatars: Query<&Avatar>,
    body_lods: Query<&Body>,
) {
    // Visible pieces whose mesh data is loaded, by body.
    let mut visible_by_avatar: std::collections::HashMap<Entity, Vec<Entity>> =
        std::collections::HashMap::new();
    for (entity, mesh, _, name, own_part, _) in &sources {
        let Some(owning_body) = owner_of(entity, &parents, &owners) else {
            continue;
        };
        let part = own_part.or_else(|| {
            parents
                .get(entity)
                .ok()
                .and_then(|p| parts.get(p.parent()).ok())
        });
        let covered = part.is_some_and(|p| {
            equipped
                .iter()
                .any(|e| e.body == owning_body && e.hides.contains(&p.0))
        });
        let Some(quality) = body_lods.get(owning_body).ok().map(|b| b.lod) else {
            continue;
        };
        if covered || meshes.get(&mesh.0).is_none() || piece(&name.0, quality).is_none() {
            continue;
        }
        visible_by_avatar
            .entry(owning_body)
            .or_default()
            .push(entity);
    }

    for (body, body_info, mut built) in &mut bodies {
        let Ok(avatar) = avatars.get(body_info.avatar) else {
            continue;
        };
        let quality = body_info.lod;
        let mut visible = visible_by_avatar.remove(&body).unwrap_or_default();
        visible.sort();
        if visible == built.signature && built.quality == Some(quality) {
            continue;
        }
        built.signature = visible.clone();
        built.quality = Some(quality);

        for (entity, info) in &merged {
            if info.body == body {
                commands.entity(entity).try_despawn();
            }
        }
        let Some(first) = visible.first().and_then(|e| sources.get(*e).ok()) else {
            continue;
        };
        let gender = body_info.body_type.letter();
        let (skin, joints) = (first.2, first.2.joints.clone());

        // Remap UVs into the atlas, tag each vertex with its piece id.
        let mut pieces: Vec<SourcePiece> = Vec::new();
        for entity in &visible {
            let Ok((_, mesh, skin_i, name, _, _)) = sources.get(*entity) else {
                continue;
            };
            if skin_i.joints != joints {
                continue;
            }
            let (Some(info), Some(source)) = (piece(&name.0, quality), meshes.get(&mesh.0)) else {
                continue;
            };
            let mut m = source.clone();
            let vertices = m.count_vertices();
            if let Some(VertexAttributeValues::Float32x2(uvs)) =
                m.attribute_mut(Mesh::ATTRIBUTE_UV_0)
            {
                for uv in uvs.iter_mut() {
                    *uv = if info.collapse {
                        [
                            info.cell[0] + info.cell[2] * 0.5,
                            info.cell[1] + info.cell[3] * 0.5,
                        ]
                    } else {
                        [
                            info.cell[0] + (uv[0] - info.origin[0]) * info.cell[2],
                            info.cell[1] + (uv[1] - info.origin[1]) * info.cell[3],
                        ]
                    };
                }
            }
            m.insert_attribute(
                Mesh::ATTRIBUTE_COLOR,
                vec![[info.index as f32, 0.0, 0.0, 1.0]; vertices],
            );
            pieces.push(SourcePiece { mesh: m });
        }
        let Some((first_piece, rest)) = pieces.split_first() else {
            continue;
        };
        let mut merged_mesh = first_piece.mesh.clone();
        let mut failed = false;
        for p in rest {
            if let Err(err) = merged_mesh.merge(&p.mesh) {
                warn!("atlas merge failed: {err}");
                failed = true;
                break;
            }
        }
        if failed {
            continue;
        }

        // The material belongs to the avatar; rebuild it when the body type or the quality changes.
        if built.gender != Some(gender)
            || built.material.is_none()
            || built.material_quality != Some(quality)
        {
            let Some(params) = atlas_params(gender, &defs, quality, settings.outline, &avatar.look)
            else {
                continue;
            };
            built.material = Some(materials.add(AtlasMaterial {
                params,
                mask: Some(load_texture(
                    &assets,
                    &atlas_texture(gender, quality, "RGBMap"),
                    false,
                )),
                normal: quality.normal_maps().then(|| {
                    load_texture(&assets, &atlas_texture(gender, quality, "Normal"), false)
                }),
                ramp: Some(load_texture(&assets, RAMP, true)),
            }));
            built.gender = Some(gender);
            built.material_quality = Some(quality);
        }
        let Some(material) = built.material.clone() else {
            continue;
        };

        let handle = meshes.add(merged_mesh);
        let mut entity = commands.spawn((
            Mesh3d(handle.clone()),
            MeshMaterial3d(material),
            SkinnedMesh {
                inverse_bindposes: skin.inverse_bindposes.clone(),
                joints,
            },
            Transform::IDENTITY,
            ChildOf(body),
        ));
        entity.insert(AtlasMerged {
            body,
            _mesh: handle,
        });
        info!("atlas merge: {} pieces in one draw call", pieces.len());
    }
}
