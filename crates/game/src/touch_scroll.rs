use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::prelude::*;

/// A finger must move this far (logical px) before a touch counts as a scroll drag, so taps on
/// buttons are not turned into scrolls.
const DRAG_THRESHOLD_PX: f32 = 8.0;

/// Turns one-finger drags into [`MouseWheel`] messages so every scrollable window (shop,
/// inventory, navigator, creator) scrolls on touch screens without its own touch code.
pub struct TouchScrollPlugin;

impl Plugin for TouchScrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, touch_drag_to_wheel);
    }
}

/// Wheel `y` for a finger movement of `delta_y` screen px. The scroll systems do
/// `scroll.y -= y` in UI layout units, so the content follows the finger: `y = delta_y / ui_scale`.
pub fn wheel_y_for_drag(delta_y: f32, drag_distance: f32, ui_scale: f32) -> Option<f32> {
    if drag_distance < DRAG_THRESHOLD_PX || ui_scale <= 0.0 {
        return None;
    }
    Some(delta_y / ui_scale)
}

fn touch_drag_to_wheel(
    touches: Res<Touches>,
    ui_scale: Res<UiScale>,
    windows: Query<Entity, With<Window>>,
    mut wheel: MessageWriter<MouseWheel>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    for touch in touches.iter() {
        let distance = touch.distance().length();
        if let Some(y) = wheel_y_for_drag(touch.delta().y, distance, ui_scale.0)
            && y != 0.0
        {
            wheel.write(MouseWheel {
                unit: MouseScrollUnit::Pixel,
                x: 0.0,
                y,
                phase: TouchPhase::Moved,
                window,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taps_and_tiny_movements_do_not_scroll() {
        assert_eq!(wheel_y_for_drag(3.0, 4.0, 1.0), None);
    }

    #[test]
    fn dragging_past_the_threshold_scrolls_with_the_finger() {
        // Finger moves up (negative delta): wheel y negative, so `scroll.y -= y` grows.
        assert_eq!(wheel_y_for_drag(-10.0, 30.0, 1.0), Some(-10.0));
    }

    #[test]
    fn delta_is_converted_to_ui_layout_units() {
        let y = wheel_y_for_drag(-8.0, 30.0, 0.8).unwrap();
        assert!((y + 10.0).abs() < 1e-6, "{y}");
    }

    #[test]
    fn invalid_scale_is_ignored() {
        assert_eq!(wheel_y_for_drag(5.0, 30.0, 0.0), None);
    }
}
