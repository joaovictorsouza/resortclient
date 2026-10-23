use bevy::prelude::*;

/// The UI templates are authored in `px` for a screen of at least this logical size.
const REFERENCE_WIDTH: f32 = 960.0;
const REFERENCE_HEIGHT: f32 = 600.0;
/// Below this the UI would be unreadable; such screens simply get a denser layout.
const MIN_UI_SCALE: f32 = 0.5;

/// Touch devices shrink the HTML screens less than the fit-to-reference scale would (which made
/// text unreadable), but still enough that the modals are not oversized.
pub const TOUCH_UI_SCALE: f32 = 0.8;

/// Multiplier for hand-built HUD sizes (in `px`) so they keep the same on-screen size
/// regardless of the UI scale: 1.0 on desktop, `1 / TOUCH_UI_SCALE` on touch devices.
pub const HUD_SIZE_FACTOR: f32 = if cfg!(any(target_os = "android", target_os = "ios")) {
    1.0 / TOUCH_UI_SCALE
} else {
    1.0
};

/// UI scale for a window of `width` x `height` logical pixels. On desktop it shrinks the
/// `px`-based UI so the reference layout fits small windows and never enlarges it; on touch
/// devices (`touch == true`) it is a fixed [`TOUCH_UI_SCALE`].
pub fn ui_scale_for(width: f32, height: f32, touch: bool) -> f32 {
    if touch {
        return TOUCH_UI_SCALE;
    }
    if width <= 0.0 || height <= 0.0 {
        return 1.0;
    }
    (width / REFERENCE_WIDTH)
        .min(height / REFERENCE_HEIGHT)
        .clamp(MIN_UI_SCALE, 1.0)
}

pub struct UiScalePlugin;

impl Plugin for UiScalePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScale>()
            .add_systems(Update, update_ui_scale);
    }
}

/// Keeps [`UiScale`] matched to the window (also on rotation and window resize).
fn update_ui_scale(windows: Query<&Window>, mut ui_scale: ResMut<UiScale>) {
    let Ok(window) = windows.single() else {
        return;
    };
    // `Window::width/height` are already divided by the OS scale factor but not by `UiScale`.
    let target = ui_scale_for(
        window.width(),
        window.height(),
        cfg!(any(target_os = "android", target_os = "ios")),
    );
    if (ui_scale.0 - target).abs() > 1e-3 {
        ui_scale.0 = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_and_tablet_screens_keep_the_authored_size() {
        assert_eq!(ui_scale_for(1280.0, 720.0, false), 1.0);
        assert_eq!(ui_scale_for(1024.0, 768.0, false), 1.0);
        assert_eq!(ui_scale_for(2560.0, 1440.0, false), 1.0);
    }

    #[test]
    fn touch_devices_use_a_fixed_scale() {
        // iPhone 16 landscape and portrait, small Android.
        assert_eq!(ui_scale_for(852.0, 393.0, true), TOUCH_UI_SCALE);
        assert_eq!(ui_scale_for(390.0, 844.0, true), TOUCH_UI_SCALE);
        assert_eq!(ui_scale_for(200.0, 100.0, true), TOUCH_UI_SCALE);
    }

    #[test]
    fn small_desktop_window_shrinks_to_fit_the_reference_layout() {
        let scale = ui_scale_for(852.0, 393.0, false);
        assert!((scale - 393.0 / 600.0).abs() < 1e-6, "{scale}");
    }

    #[test]
    fn narrow_portrait_screens_are_limited_by_width() {
        let scale = ui_scale_for(390.0, 844.0, false);
        assert!((scale - 390.0 / 960.0).abs() < 1e-6 || scale == MIN_UI_SCALE);
    }

    #[test]
    fn scale_never_drops_below_the_minimum() {
        assert_eq!(ui_scale_for(200.0, 100.0, false), MIN_UI_SCALE);
    }

    #[test]
    fn degenerate_sizes_fall_back_to_one() {
        assert_eq!(ui_scale_for(0.0, 0.0, false), 1.0);
    }
}
