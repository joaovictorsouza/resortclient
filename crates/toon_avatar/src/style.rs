//! Optional toon look for the scene: floors, walls and furniture drawn with the same ramp and light
//! model as the characters. It is a setting ([`SceneStyle`], off by default): turning it on swaps the
//! `StandardMaterial` of every scene mesh for a [`SceneToonMaterial`], turning it off puts the originals
//! back, and nothing else changes (nothing is added when it is off).
//!
//! Game code that swaps the material of a mesh at run time (walls turning see-through) must change
//! [`SceneToon::original`] instead while the mesh is toon-styled; the style follows the change.

use std::collections::HashMap;

use bevy::{
    asset::embedded_asset,
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use crate::avatar::{OwnedBy, owner_of};

const SHADER: &str = "embedded://toon_avatar/shaders/toon_scene.wesl";
const RAMP: &str = "characters/textures/ColorRamp.png";
const FLAG_HAS_TEXTURE: u32 = 1;
const FLAG_MASK: u32 = 2;

/// How the scene is drawn.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SceneStyle {
    /// Cartoon look for rooms and objects (off by default).
    pub cartoon: bool,
}

/// Marks a scene mesh that is drawn with the toon material; keeps the material it had.
#[derive(Component, Clone, Debug)]
pub struct SceneToon {
    pub original: Handle<StandardMaterial>,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SceneToonParams {
    pub color: Vec4,
    pub alpha_cutoff: f32,
    /// Width of the dark edge (0 turns it off).
    pub edge: f32,
    pub flags: u32,
    pub pad: u32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SceneToonMaterial {
    #[uniform(0)]
    pub params: SceneToonParams,
    #[texture(1)]
    #[sampler(2)]
    pub base: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub ramp: Option<Handle<Image>>,
    pub alpha_mode: AlphaMode,
}

impl Material for SceneToonMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}

/// The toon material made for each original material, so meshes that share one share the other.
#[derive(Resource, Default)]
struct SceneToonCache(HashMap<AssetId<StandardMaterial>, Handle<SceneToonMaterial>>);

#[derive(Resource, Default)]
struct RampHandle(Option<Handle<Image>>);

pub struct SceneStylePlugin;

impl Plugin for SceneStylePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/toon_scene.wesl");
        app.add_plugins(MaterialPlugin::<SceneToonMaterial>::default())
            .init_resource::<SceneStyle>()
            .init_resource::<SceneToonCache>()
            .init_resource::<RampHandle>()
            .add_systems(Update, (apply_scene_style, follow_original_changes).chain());
    }
}

/// The toon version of a standard material: its colour, texture and transparency.
pub fn toon_from_standard(
    material: &StandardMaterial,
    ramp: Option<Handle<Image>>,
) -> SceneToonMaterial {
    let linear = material.base_color.to_linear();
    let mut flags = 0;
    if material.base_color_texture.is_some() {
        flags |= FLAG_HAS_TEXTURE;
    }
    let alpha_cutoff = match material.alpha_mode {
        AlphaMode::Mask(cutoff) => {
            flags |= FLAG_MASK;
            cutoff
        }
        _ => 0.0,
    };
    SceneToonMaterial {
        params: SceneToonParams {
            color: Vec4::new(linear.red, linear.green, linear.blue, linear.alpha),
            alpha_cutoff,
            edge: 0.3,
            flags,
            pad: 0,
        },
        base: material.base_color_texture.clone(),
        ramp,
        alpha_mode: material.alpha_mode,
    }
}

fn ramp_handle(assets: &AssetServer, ramp: &mut RampHandle) -> Handle<Image> {
    ramp.0
        .get_or_insert_with(|| {
            assets
                .load_builder()
                .with_settings(|s: &mut ImageLoaderSettings| {
                    s.is_srgb = true;
                    s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::ClampToEdge,
                        address_mode_v: ImageAddressMode::ClampToEdge,
                        ..ImageSamplerDescriptor::linear()
                    });
                })
                .load(RAMP.to_string())
        })
        .clone()
}

/// Swaps the materials of the scene meshes to match the style, in both directions.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply_scene_style(
    mut commands: Commands,
    style: Res<SceneStyle>,
    assets: Res<AssetServer>,
    standard: Res<Assets<StandardMaterial>>,
    mut toon_materials: ResMut<Assets<SceneToonMaterial>>,
    mut cache: ResMut<SceneToonCache>,
    mut ramp: ResMut<RampHandle>,
    plain: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<SceneToon>>,
    styled: Query<(Entity, &SceneToon)>,
    parents: Query<&ChildOf>,
    owners: Query<&OwnedBy>,
) {
    if style.cartoon {
        let ramp_image = ramp_handle(&assets, &mut ramp);
        for (entity, material) in &plain {
            // Characters and the props they hold have their own materials.
            if owner_of(entity, &parents, &owners).is_some() {
                continue;
            }
            let Some(original) = standard.get(&material.0) else {
                continue;
            };
            let handle = cache
                .0
                .entry(material.0.id())
                .or_insert_with(|| {
                    toon_materials.add(toon_from_standard(original, Some(ramp_image.clone())))
                })
                .clone();
            commands
                .entity(entity)
                .try_remove::<MeshMaterial3d<StandardMaterial>>()
                .try_insert((
                    MeshMaterial3d(handle),
                    SceneToon {
                        original: material.0.clone(),
                    },
                ));
        }
    } else if !styled.is_empty() {
        for (entity, toon) in &styled {
            commands
                .entity(entity)
                .try_remove::<(MeshMaterial3d<SceneToonMaterial>, SceneToon)>()
                .try_insert(MeshMaterial3d(toon.original.clone()));
        }
    }
}

/// A styled mesh whose original material was swapped (a wall turning see-through) gets the toon
/// version of the new one.
#[allow(clippy::type_complexity)]
fn follow_original_changes(
    style: Res<SceneStyle>,
    assets: Res<AssetServer>,
    standard: Res<Assets<StandardMaterial>>,
    mut toon_materials: ResMut<Assets<SceneToonMaterial>>,
    mut cache: ResMut<SceneToonCache>,
    mut ramp: ResMut<RampHandle>,
    mut styled: Query<(&SceneToon, &mut MeshMaterial3d<SceneToonMaterial>), Changed<SceneToon>>,
) {
    if !style.cartoon {
        return;
    }
    let ramp_image = ramp_handle(&assets, &mut ramp);
    for (toon, mut material) in &mut styled {
        let Some(original) = standard.get(&toon.original) else {
            continue;
        };
        let handle = cache
            .0
            .entry(toon.original.id())
            .or_insert_with(|| {
                toon_materials.add(toon_from_standard(original, Some(ramp_image.clone())))
            })
            .clone();
        if material.0 != handle {
            material.0 = handle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scene_is_realistic_by_default() {
        assert!(!SceneStyle::default().cartoon);
    }

    #[test]
    fn a_toon_material_keeps_the_colour_the_texture_and_the_transparency() {
        let texture: Handle<Image> = Handle::default();
        let standard = StandardMaterial {
            base_color: Color::srgba(0.5, 0.25, 0.75, 0.4),
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Blend,
            ..default()
        };
        let toon = toon_from_standard(&standard, None);
        assert_eq!(toon.alpha_mode, AlphaMode::Blend);
        assert_eq!(toon.params.flags & FLAG_HAS_TEXTURE, FLAG_HAS_TEXTURE);
        assert!((toon.params.color.w - 0.4).abs() < 1e-5);
        assert!(toon.base.is_some());
        // sRGB 0.5 is 0.2140 in linear space.
        assert!((toon.params.color.x - 0.2140).abs() < 1e-3);
    }

    #[test]
    fn a_mask_material_discards_below_its_cutoff() {
        let standard = StandardMaterial {
            alpha_mode: AlphaMode::Mask(0.3),
            ..default()
        };
        let toon = toon_from_standard(&standard, None);
        assert_eq!(toon.params.flags & FLAG_MASK, FLAG_MASK);
        assert_eq!(toon.params.alpha_cutoff, 0.3);
    }

    fn style_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ImagePlugin::default(),
        ))
        .init_asset::<StandardMaterial>()
        .init_asset::<SceneToonMaterial>()
        .init_resource::<SceneStyle>()
        .init_resource::<SceneToonCache>()
        .init_resource::<RampHandle>()
        .add_systems(Update, (apply_scene_style, follow_original_changes).chain());
        app
    }

    fn spawn_scene_mesh(app: &mut App, color: Color) -> (Entity, Handle<StandardMaterial>) {
        let handle = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: color,
                ..default()
            });
        let entity = app.world_mut().spawn(MeshMaterial3d(handle.clone())).id();
        (entity, handle)
    }

    #[test]
    fn turning_the_style_on_and_off_swaps_and_restores_the_materials() {
        let mut app = style_app();
        let (entity, original) = spawn_scene_mesh(&mut app, Color::WHITE);
        app.update();
        // Off: nothing changes.
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .is_some()
        );
        assert!(app.world().get::<SceneToon>(entity).is_none());

        app.world_mut().resource_mut::<SceneStyle>().cartoon = true;
        app.update();
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .is_none()
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<SceneToonMaterial>>(entity)
                .is_some()
        );
        assert_eq!(
            app.world().get::<SceneToon>(entity).unwrap().original,
            original
        );

        // Off again: the original comes back, and nothing of the style is left.
        app.world_mut().resource_mut::<SceneStyle>().cartoon = false;
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .unwrap()
                .0,
            original
        );
        assert!(
            app.world()
                .get::<MeshMaterial3d<SceneToonMaterial>>(entity)
                .is_none()
        );
        assert!(app.world().get::<SceneToon>(entity).is_none());
    }

    #[test]
    fn meshes_sharing_a_material_share_the_toon_material() {
        let mut app = style_app();
        let (a, handle) = spawn_scene_mesh(&mut app, Color::WHITE);
        let b = app.world_mut().spawn(MeshMaterial3d(handle)).id();
        app.world_mut().resource_mut::<SceneStyle>().cartoon = true;
        app.update();
        let ha = app
            .world()
            .get::<MeshMaterial3d<SceneToonMaterial>>(a)
            .unwrap()
            .0
            .clone();
        let hb = app
            .world()
            .get::<MeshMaterial3d<SceneToonMaterial>>(b)
            .unwrap()
            .0
            .clone();
        assert_eq!(ha, hb);
    }

    #[test]
    fn a_mesh_spawned_while_the_style_is_on_is_styled_too() {
        let mut app = style_app();
        app.world_mut().resource_mut::<SceneStyle>().cartoon = true;
        app.update();
        let (entity, _) = spawn_scene_mesh(&mut app, Color::BLACK);
        app.update();
        assert!(
            app.world()
                .get::<MeshMaterial3d<SceneToonMaterial>>(entity)
                .is_some()
        );
    }

    #[test]
    fn the_characters_materials_are_left_alone() {
        let mut app = style_app();
        let (entity, _) = spawn_scene_mesh(&mut app, Color::WHITE);
        app.world_mut()
            .entity_mut(entity)
            .insert(OwnedBy(Entity::PLACEHOLDER));
        app.world_mut().resource_mut::<SceneStyle>().cartoon = true;
        app.update();
        assert!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(entity)
                .is_some()
        );
        assert!(app.world().get::<SceneToon>(entity).is_none());
    }

    #[test]
    fn swapping_the_original_of_a_styled_mesh_swaps_its_toon_material() {
        let mut app = style_app();
        let (entity, _) = spawn_scene_mesh(&mut app, Color::WHITE);
        app.world_mut().resource_mut::<SceneStyle>().cartoon = true;
        app.update();
        let before = app
            .world()
            .get::<MeshMaterial3d<SceneToonMaterial>>(entity)
            .unwrap()
            .0
            .clone();
        let ghost = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, 0.2),
                alpha_mode: AlphaMode::Blend,
                ..default()
            });
        app.world_mut()
            .get_mut::<SceneToon>(entity)
            .unwrap()
            .original = ghost;
        app.update();
        let after = app
            .world()
            .get::<MeshMaterial3d<SceneToonMaterial>>(entity)
            .unwrap()
            .0
            .clone();
        assert_ne!(before, after);
        let toon = app
            .world()
            .resource::<Assets<SceneToonMaterial>>()
            .get(&after)
            .unwrap();
        assert_eq!(toon.alpha_mode, AlphaMode::Blend);
    }
}
