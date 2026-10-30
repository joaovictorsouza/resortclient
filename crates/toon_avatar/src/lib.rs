//! Reusable Bevy plugins for modular, toon-shaded characters.
//!
//! * [`ToonAvatarPlugins`] adds everything: the toon material, the wardrobe (body, garments,
//!   head attachments), the skin-and-face atlas, mesh merging and the device tier and the level of detail of each avatar.
//! * Insert an [`AvatarCatalog`] and a [`MaterialDefs`] resource, then spawn
//!   [`Avatar::new`]`(look)` on an entity. Change `Avatar::look` to change what it wears.
//!
//! The crate knows nothing about any game: no networking, no UI, no asset source. Assets are
//! read through the app's `AssetServer` using the paths in the catalog and material
//! definitions, so a game can serve them from a CDN. Shaders are embedded in the binary.

mod animation;
mod atlas;
mod avatar;
mod catalog;
mod face;
mod lod;
mod material;
mod merge;
mod props;
mod quality;

pub use animation::{
    AvatarAnimationConfig, AvatarAnimationLibrary, AvatarAnimationPlugin, AvatarAnimator,
    AvatarPose, AvatarWalkSpeed, BodyAnimations, ClipMeta, Grip, RIG_ROOT_NAME, parse_clips,
    parse_grip, tick_due, walk_playback_speed,
};
pub use atlas::{AtlasBuilt, AtlasMaterial};
pub use avatar::{Applied, Attachment, Avatar, Body, BodyPart, BodyReady, Equipped, OwnedBy};
pub use catalog::{
    AvatarCatalog, AvatarLook, BodyCatalog, BodyType, CatalogEntry, ColorChoice, ColorGroup, Slot,
};
pub use face::{
    AvatarFace, AvatarFacePlugin, BLINK_DURATION, Expression, FaceAnimator, blink_amount,
    next_blink_interval, target_weights,
};
pub use lod::{
    AvatarLod, DeviceInfo, DeviceInfoOverride, DeviceTier, FrameGovernor, LodConfig, LodFocus,
    LodPlugin, SizeClass, TierPreference, animation_interval, classify, desired_level, size_class,
    too_slow,
};
pub use material::{
    AvatarMaterials, COLOR_SLOTS, MaterialDef, MaterialDefs, ToonMaterial, ToonMaterialPlugin,
    ToonParams, ToonSettings,
};
pub use merge::{MergeEnabled, MergePlugin};
pub use props::{AvatarHolding, AvatarPropsPlugin, HeldProp, HeldPropEntity, prop_transform};
pub use quality::{Quality, QualityRenderSettingsPlugin};

use bevy::{app::PluginGroupBuilder, prelude::*};

/// Every plugin of the crate.
pub struct ToonAvatarPlugins;

impl PluginGroup for ToonAvatarPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(LodPlugin)
            .add(ToonMaterialPlugin)
            .add(atlas::AtlasPlugin)
            .add(avatar::AvatarPlugin)
            .add(MergePlugin)
            .add(AvatarAnimationPlugin)
            .add(AvatarFacePlugin)
            .add(AvatarPropsPlugin)
    }
}
