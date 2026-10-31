//! Reusable Bevy plugin for animals: toon-shaded, skinned, animated, with a level of detail per animal.
//!
//! The crate only *draws and animates*; it decides nothing. A game (or a server, over the network) says what
//! each animal is and does by changing three components on an entity that has an [`Animal`]:
//!
//! * [`AnimalAction`]: what it is doing (`Idle`, `Sit`, `Lay`, `Walk`, `Run`, `Bark`, `Pant`), which picks the clip;
//! * [`AnimalRoute`]: the world positions to walk through, at a speed; the crate moves and turns the animal;
//! * [`AnimalFacing`]: where it looks while standing.
//!
//! **Species, breeds and coats.** An animal is `species / breed / coat` (`dog / shihtzu / default`). Breeds of
//! one species share a skeleton and its animations, so there is one animation file per species and meshes per
//! breed. Files are read through the app's `AssetServer` (a CDN, say) from this layout:
//!
//! ```text
//! animals/<species>/anims.glb               armature and clips (named like the keys of clips.json)
//! animals/<species>/clips.json              duration, loop, ground speed (m/s in `tiles_per_sec`), model height
//! animals/<species>/<breed>/lod{0,1,2}.glb  skinned meshes, high to low detail
//! ```
//!
//! The application loads each species' `clips.json` and describes it in [`AnimalConfig`] (the crate does not read
//! JSON assets itself, so it never clashes with the app's own loaders). Until a species is configured its animals
//! stay invisible. A coat is a texture: today `default` is the one baked into the model; other coats will swap
//! the base texture of the breed's material (the `coat` is kept on [`Animal`] for that).
//!
//! The look comes from `toon_avatar`'s scene style: the models carry standard materials and the style turns
//! them into the toon material. The level of detail follows the size on the screen and the device tier of
//! `toon_avatar`.

mod action;
mod animal;
mod animation;
mod lod;

pub use action::AnimalAction;
pub use animal::{
    Animal, AnimalConfig, AnimalFacing, AnimalRoute, SpeciesAnimations, lod_model, playback_speed,
};

use bevy::prelude::*;

pub struct AnimalsPlugin;

impl Plugin for AnimalsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimalConfig>()
            .init_resource::<animal::Libraries>()
            .add_systems(
                Update,
                (
                    animal::build_graphs,
                    animal::attach_visuals,
                    animal::follow_route,
                    animal::turn_standing,
                    lod::update_lod,
                    animation::bind_rig,
                    animation::drive_animation,
                )
                    .chain(),
            );
    }
}
