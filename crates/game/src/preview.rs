//! Helpers shared by the dev preview examples (`examples/*_preview.rs`).
//!
//! `PREVIEW_PHONE=1` makes the window look like an iPhone held sideways: 852x393 points, the
//! touch UI scale and the safe-area insets of a notched iPhone, so layouts can be checked on the
//! desktop without a simulator.

use bevy::prelude::*;
use bevy::window::WindowResolution;

use crate::safe_area::{SafeArea, SafeAreaPlugin};
use crate::ui_scale::TOUCH_UI_SCALE;

fn phone_mode() -> bool {
    std::env::var("PREVIEW_PHONE").is_ok()
}

/// `DefaultPlugins` with the window sized for the chosen preview mode.
pub fn default_plugins() -> impl PluginGroup {
    let resolution = if phone_mode() {
        WindowResolution::new(852, 393)
    } else {
        WindowResolution::new(1280, 720)
    };
    DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resolution,
            ..default()
        }),
        ..default()
    })
}

/// Applies the touch UI scale and the safe area of a notched iPhone in `PREVIEW_PHONE` mode.
pub struct PhoneSimulation;

impl Plugin for PhoneSimulation {
    fn build(&self, app: &mut App) {
        if phone_mode() {
            app.insert_resource(UiScale(TOUCH_UI_SCALE))
                .insert_resource(SafeArea {
                    top: 0.0,
                    right: 59.0,
                    bottom: 21.0,
                    left: 59.0,
                });
        }
        app.add_plugins(SafeAreaPlugin);
    }
}
