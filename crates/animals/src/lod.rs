use bevy::prelude::*;
use toon_avatar::{DeviceTier, LodConfig, desired_level, size_class};

use crate::animal::{Animal, AnimalVisual, spawn_scene};

/// Shortest time between two model swaps of one animal.
const MIN_SWITCH_SECS: f32 = 1.0;

/// Swaps an animal's model for a lighter or heavier one by its size on the screen and the device tier.
pub(crate) fn update_lod(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<AssetServer>,
    tier: Option<Res<DeviceTier>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut animals: Query<(&GlobalTransform, &Animal, &mut AnimalVisual)>,
    scenes: Query<&ChildOf>,
) {
    let (Some(tier), Some((camera, camera_transform))) = (
        tier,
        cameras
            .iter()
            .find(|(c, _)| c.is_active && c.logical_viewport_size().is_some()),
    ) else {
        return;
    };
    // An animal is small: a dog of 60 px is as sharp as it needs to be.
    let cfg = LodConfig {
        near_px: 140.0,
        far_px: 55.0,
        ..LodConfig::default()
    };
    for (global, animal, mut visual) in &mut animals {
        visual.since_switch += time.delta_secs();
        let feet = global.translation();
        let (Ok(a), Ok(b)) = (
            camera.world_to_viewport(camera_transform, feet),
            camera.world_to_viewport(camera_transform, feet + Vec3::Y * animal.height),
        ) else {
            continue;
        };
        let class = size_class((a - b).length(), visual.class, &cfg);
        visual.class = class;
        let level = desired_level(*tier, false, class, true);
        if level == visual.level || visual.since_switch < MIN_SWITCH_SECS {
            continue;
        }
        let Ok(parent) = scenes.get(visual.scene) else {
            continue;
        };
        let model = parent.parent();
        commands.entity(visual.scene).despawn();
        visual.scene = spawn_scene(&mut commands, &assets, animal, level, model);
        visual.level = level;
        visual.since_switch = 0.0;
    }
}
