//! Reusable Bevy plugins for modular, toon-shaded characters.
//!
//! * [`ToonAvatarPlugins`] adds everything: the toon material, the wardrobe (body, garments,
//!   head attachments), the skin-and-face atlas, mesh merging and the quality tier.
//! * Insert an [`AvatarCatalog`] and a [`MaterialDefs`] resource, then spawn
//!   [`Avatar::new`]`(look)` on an entity. Change `Avatar::look` to change what it wears.
//!
//! The crate knows nothing about any game: no networking, no UI, no asset source. Assets are
//! read through the app's `AssetServer` using the paths in the catalog and material
//! definitions, so a game can serve them from a CDN. Shaders are embedded in the binary.

mod atlas;
mod avatar;
mod catalog;
mod material;
mod merge;
mod quality;

pub use atlas::{AtlasBuilt, AtlasMaterial};
pub use avatar::{Attachment, Avatar, Body, BodyPart, BodyReady, Equipped, OwnedBy};
pub use catalog::{
    AvatarCatalog, AvatarLook, BodyCatalog, BodyType, CatalogEntry, ColorChoice, Slot,
};
pub use material::{
    AvatarMaterials, COLOR_SLOTS, MaterialDef, MaterialDefs, ToonMaterial, ToonMaterialPlugin,
    ToonParams, ToonSettings,
};
pub use merge::{MergeEnabled, MergePlugin};
pub use quality::{Quality, QualityPlugin, QualityRenderSettingsPlugin};

use bevy::{app::PluginGroupBuilder, prelude::*};

/// Every plugin of the crate.
pub struct ToonAvatarPlugins;

impl PluginGroup for ToonAvatarPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(QualityPlugin)
            .add(ToonMaterialPlugin)
            .add(atlas::AtlasPlugin)
            .add(avatar::AvatarPlugin)
            .add(MergePlugin)
    }
}
