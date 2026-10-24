//! iOS window tweaks applied to the winit window once Bevy has created it.
//!
//! With the UIScene life cycle the status bar visibility is decided by the root view controller
//! of the scene's window (`Info.plist` `UIStatusBarHidden` is ignored while
//! `UIViewControllerBasedStatusBarAppearance` is on), and winit's view controller shows the bar
//! unless asked not to. UIKit must be called from the main thread, hence `NonSendMarker`.

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use objc2_ui_kit::UIViewController;
use winit::platform::ios::{ValidOrientations, WindowExtIOS};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub struct IosWindowPlugin;

impl Plugin for IosWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, configure_window);
    }
}

/// Frames during which the preference is re-asserted: the window moves onto the UIScene after it
/// was created, and a preference set before that is not always picked up by the scene's status bar.
const REASSERT_FRAMES: u32 = 600;
const REASSERT_EVERY: u32 = 20;
/// Frames (counted from window creation) at which the real UIKit state is written to the log.
const DIAGNOSTIC_FRAMES: [u32; 3] = [30, 300, 900];

/// Whether frame `frame` (counted from window creation) should (re)apply the preference.
pub(crate) fn should_apply(frame: u32) -> bool {
    frame < REASSERT_FRAMES && frame % REASSERT_EVERY == 0
}

fn configure_window(
    _main_thread: NonSendMarker,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut frame: Local<Option<u32>>,
) {
    let Ok(entity) = windows.single() else {
        return;
    };
    let current = frame.unwrap_or(0);
    if current > *DIAGNOSTIC_FRAMES.last().unwrap_or(&0) {
        return;
    }
    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(window) = winit_windows.get_window(entity) else {
            return;
        };
        if current == 0 {
            // winit's view controller allows every orientation on an iPad; the keyboard follows it.
            window.set_valid_orientations(ValidOrientations::Landscape);
        }
        if should_apply(current) {
            window.set_prefers_status_bar_hidden(true);
            window.set_prefers_home_indicator_hidden(true);
        }
        if DIAGNOSTIC_FRAMES.contains(&current) {
            info!(
                "iOS window state at frame {current}: {}",
                describe_window(window)
            );
        }
        *frame = Some(current + 1);
    });
}

/// What UIKit really thinks about the status bar, for the game log (diagnosing devices).
fn describe_window(window: &winit::window::Window) -> String {
    let Ok(handle) = window.window_handle() else {
        return "no window handle".to_string();
    };
    let RawWindowHandle::UiKit(handle) = handle.as_raw() else {
        return "not a UIKit handle".to_string();
    };
    let Some(vc_ptr) = handle.ui_view_controller else {
        return "winit window has no view controller".to_string();
    };
    // SAFETY: winit keeps its view controller alive as long as the window, and the handle was just
    // read on the main thread (this system is pinned there by `NonSendMarker`).
    let vc: &UIViewController = unsafe { &*(vc_ptr.as_ptr() as *const UIViewController) };
    let prefers_hidden = vc.prefersStatusBarHidden();
    let Some(ui_window) = vc.view().and_then(|v| v.window()) else {
        return format!("vc.prefersStatusBarHidden={prefers_hidden}, view not in a window");
    };
    let scene = ui_window.windowScene();
    let bar_hidden = scene
        .as_ref()
        .and_then(|s| s.statusBarManager())
        .map(|m| m.isStatusBarHidden());
    let root_is_winit_vc = ui_window
        .rootViewController()
        .is_some_and(|root| std::ptr::eq(&*root, vc));
    let frame = ui_window.frame();
    let orientation = scene.as_ref().map(|s| s.interfaceOrientation());
    format!(
        "vc.prefersStatusBarHidden={prefers_hidden}, statusBarManager.hidden={bar_hidden:?}, \
         key_window={}, root_is_winit_vc={root_is_winit_vc}, window_frame={}x{}, \
         scene_orientation={orientation:?}",
        ui_window.isKeyWindow(),
        frame.size.width,
        frame.size.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_is_applied_at_once_and_then_periodically_for_a_while() {
        assert!(should_apply(0));
        assert!(!should_apply(1));
        assert!(should_apply(20));
        assert!(should_apply(580));
        assert!(!should_apply(600));
        assert!(!should_apply(620));
    }
}
