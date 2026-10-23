use bevy::prelude::*;

use crate::net::{NakamaResource, send_move_request};
use crate::room::{Tile, world_to_tile};
use crate::state::AppState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchGesture>()
            .add_systems(Update, track_multi_touch)
            .add_systems(
                Update,
                handle_pointer_click_or_tap
                    .after(track_multi_touch)
                    .run_if(in_state(AppState::InRoom))
                    .run_if(resource_exists::<NakamaResource>),
            );
    }
}

/// A finger that moves more than this (logical px) between press and release is a drag, not a tap.
const TAP_SLOP_PX: f32 = 12.0;

/// Whether the current touch gesture ever had two fingers down (a pinch), so its fingers
/// lifting must not be read as taps on the floor.
#[derive(Resource, Default)]
pub(crate) struct TouchGesture {
    multi_touch: bool,
}

/// A released finger is a tap (move command) only if it barely moved and was not part of a pinch.
fn is_tap(drag_distance: f32, multi_touch: bool) -> bool {
    !multi_touch && drag_distance < TAP_SLOP_PX
}

fn track_multi_touch(touches: Res<Touches>, mut gesture: ResMut<TouchGesture>) {
    let down = touches.iter().count();
    if down >= 2 {
        gesture.multi_touch = true;
    } else if down == 1 && touches.any_just_pressed() {
        // A lone new finger starts a fresh gesture.
        gesture.multi_touch = false;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_pointer_click_or_tap(
    mouse_button: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::camera::MainRoomCamera>>,
    tiles_query: Query<&Tile>,
    net: Option<Res<NakamaResource>>,
    build_mode: Option<Res<crate::furniture::BuildModeState>>,
    inv_state: Option<Res<crate::furniture::PlayerInventoryState>>,
    gesture: Res<TouchGesture>,
    avatar_menu: Res<crate::avatar_menu::AvatarMenuState>,
) {
    // A press that closes the avatar menu must not also send the avatar walking.
    if avatar_menu.is_open {
        return;
    }

    // If in build mode or inventory is open, do not trigger avatar movement
    if build_mode.as_ref().is_some_and(|b| b.is_active)
        || inv_state.as_ref().is_some_and(|i| i.is_open)
    {
        return;
    }

    let Some(net) = net else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    // Unified pointer position: mouse press, or a finger lifted after a tap. Touches act on
    // release so a pinch or a drag is never mistaken for a move command.
    let pointer_pos = if mouse_button.just_pressed(MouseButton::Left) {
        window.cursor_position()
    } else {
        touches
            .iter_just_released()
            .find(|touch| is_tap(touch.distance().length(), gesture.multi_touch))
            .map(|touch| touch.position())
    };

    let Some(screen_pos) = pointer_pos else {
        return;
    };

    // Project screen position to 3D world ray
    let Ok(ray) = camera.viewport_to_world(camera_transform, screen_pos) else {
        return;
    };

    // Calculate intersection with the ground plane Y = 0.1
    let plane_y = 0.1;
    let normal = Vec3::Y;
    let denom = normal.dot(*ray.direction);

    if denom.abs() > 1e-6 {
        let t = (plane_y - ray.origin.y) / ray.direction.y;
        if t >= 0.0 {
            let hit_point = ray.origin + *ray.direction * t;
            let (tile_x, tile_z) = world_to_tile(hit_point);

            // Verify if (tile_x, tile_z) corresponds to an active walkable floor tile
            let tile_exists = tiles_query.iter().any(|t| t.x == tile_x && t.z == tile_z);
            if tile_exists {
                info!(
                    "Clicked tile: ({}, {}) -> Sending MOVE_REQUEST",
                    tile_x, tile_z
                );
                send_move_request(&net, tile_x, tile_z);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_single_finger_touch_is_a_tap() {
        assert!(is_tap(3.0, false));
    }

    #[test]
    fn drags_and_pinches_are_not_taps() {
        assert!(!is_tap(40.0, false));
        assert!(!is_tap(0.0, true));
    }
}
