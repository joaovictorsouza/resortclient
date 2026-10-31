//! Props in the hand: a cup or a can held in the right hand, for the drinking gesture.
//!
//! A prop is a glTF authored in metres with its origin at its centre and +Y up (see
//! `tools/props/make_demo_props.py`). It hangs on the right hand bone of the avatar's current body
//! with the grip the animation pipeline wrote (`clips.json`), so it follows the hand during the
//! `drink` clips with no per-prop adjustment. A prop can add its own `offset` and `rotation` when
//! its shape needs another grip.

use bevy::{gltf::GltfAssetLabel, prelude::*};

use crate::{
    animation::AvatarAnimationConfig,
    avatar::{Applied, Avatar, Body, BodyReady, OwnedBy},
};

const HAND_BONE: &str = "CC_BaseHand.R";

/// A prop held by an avatar.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldProp {
    /// Asset path of the prop's glTF.
    pub scene: String,
    /// Extra offset from the standard grip, in metres, in the prop's own frame.
    pub offset: Vec3,
    /// Extra rotation from the standard grip, in the prop's own frame.
    pub rotation: Quat,
}

impl HeldProp {
    pub fn new(scene: impl Into<String>) -> Self {
        Self {
            scene: scene.into(),
            offset: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

/// What the avatar holds in its right hand.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct AvatarHolding(pub Option<HeldProp>);

/// A prop scene in a hand.
#[derive(Component)]
pub struct HeldPropEntity {
    pub body: Entity,
}

/// The body and prop already built for an avatar.
#[derive(Component, Default)]
pub struct PropApplied(Option<(Entity, Option<HeldProp>)>);

pub struct AvatarPropsPlugin;

impl Plugin for AvatarPropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_props);
    }
}

/// Product of the local scales from `entity` up to, but not including, `root` (the body): the scale
/// the skeleton carries. What the game puts above the body (it scales the whole avatar) stays.
fn world_scale(
    mut entity: Entity,
    root: Entity,
    transforms: &Query<(&Transform, Option<&ChildOf>)>,
) -> f32 {
    let mut scale = 1.0;
    while entity != root
        && let Ok((t, parent)) = transforms.get(entity)
    {
        scale *= t.scale.x;
        match parent {
            Some(p) => entity = p.parent(),
            None => break,
        }
    }
    scale
}

/// The transform of a prop under the hand bone: the grip, with the prop's own adjustment, in the
/// bone's units (the skeleton is in centimetres, the prop in metres).
pub fn prop_transform(
    grip_translation_cm: Vec3,
    grip_rotation: Quat,
    prop: &HeldProp,
    bone_world_scale: f32,
) -> Transform {
    let rotation = grip_rotation * prop.rotation;
    // The offset is in metres in the prop's frame; the bone's frame is in 1/`bone_world_scale` metres.
    let translation = grip_translation_cm + (grip_rotation * prop.offset) / bone_world_scale;
    Transform {
        translation,
        rotation,
        scale: Vec3::splat(1.0 / bone_world_scale),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn sync_props(
    mut commands: Commands,
    assets: Res<AssetServer>,
    config: Option<Res<AvatarAnimationConfig>>,
    mut avatars: Query<(&Avatar, &AvatarHolding, &Applied, &mut PropApplied)>,
    bodies: Query<&Body, With<BodyReady>>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<(&Transform, Option<&ChildOf>)>,
    existing: Query<(Entity, &HeldPropEntity)>,
) {
    let Some(config) = config else {
        return;
    };
    for (avatar, holding, applied, mut done) in &mut avatars {
        let Some(body) = applied.current_body() else {
            continue;
        };
        let want = (body, holding.0.clone());
        if done.0.as_ref() == Some(&want) {
            continue;
        }
        let Ok(body_info) = bodies.get(body) else {
            continue;
        };
        let Some(hand) = children
            .iter_descendants(body)
            .find(|e| names.get(*e).is_ok_and(|n| n.as_str() == HAND_BONE))
        else {
            continue;
        };
        for (entity, held) in &existing {
            if held.body == body {
                commands.entity(entity).try_despawn();
            }
        }
        if let Some(prop) = &holding.0 {
            let Some(grip) = config.of(body_info.body_type).grip else {
                warn!(
                    "the animation library has no grip: {} is not shown",
                    prop.scene
                );
                done.0 = Some(want);
                continue;
            };
            let transform = prop_transform(
                grip.translation_cm,
                grip.rotation,
                prop,
                world_scale(hand, body, &transforms),
            );
            commands.spawn((
                HeldPropEntity { body },
                OwnedBy(body),
                WorldAssetRoot(
                    assets.load(GltfAssetLabel::Scene(0).from_asset(prop.scene.clone())),
                ),
                transform,
                ChildOf(hand),
            ));
        }
        let _ = avatar;
        done.0 = Some(want);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prop_without_adjustment_takes_the_grip_and_the_bone_scale() {
        let t = prop_transform(
            Vec3::new(-3.5, 3.0, 0.0),
            Quat::IDENTITY,
            &HeldProp::new("p.glb"),
            0.01,
        );
        assert_eq!(t.translation, Vec3::new(-3.5, 3.0, 0.0));
        assert_eq!(t.scale, Vec3::splat(100.0));
        assert_eq!(t.rotation, Quat::IDENTITY);
    }

    #[test]
    fn a_prop_offset_is_in_metres_and_follows_the_grip_rotation() {
        let mut prop = HeldProp::new("p.glb");
        prop.offset = Vec3::new(0.01, 0.0, 0.0);
        let grip = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let t = prop_transform(Vec3::ZERO, grip, &prop, 0.01);
        // 1 cm along the prop's X, rotated a quarter turn about Z, is 1 cm along Y: 1 unit in the bone's centimetres.
        assert!(
            (t.translation - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-4,
            "{:?}",
            t.translation
        );
    }
}
