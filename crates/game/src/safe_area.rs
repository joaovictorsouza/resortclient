//! Screen areas the player cannot see or tap (notch, rounded corners, home indicator).
//!
//! On iOS the insets come from UIKit through `bevy_ios_safearea`; other platforms report none. Modals fill the whole screen, but the content inside them (and the HUD) must stay out of
//! these areas, so every consumer reads [`SafeArea`] instead of using a fixed margin.

use bevy::prelude::*;
use bevy_hui::prelude::TemplateProperties;

/// Distance (layout px) kept between content and the screen edge when there is no inset to
/// respect; the base margin of modal content and of the HUD.
pub const BASE_MARGIN: f32 = 12.0;
/// HUD elements sit a little further from the edge than modal content.
pub const HUD_EDGE_MARGIN: f32 = 16.0;

/// Safe area insets in window points (the same unit as `Window::width`), before `UiScale`.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct SafeArea {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Insets converted to layout px (what `Val::Px` means once `UiScale` is applied) with `margin`
/// added on every side. `ui_scale <= 0` is treated as 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl SafeArea {
    pub fn layout_insets(&self, ui_scale: f32, margin: f32) -> LayoutInsets {
        let scale = if ui_scale > 0.0 { ui_scale } else { 1.0 };
        let conv = |points: f32| points.max(0.0) / scale + margin;
        LayoutInsets {
            top: conv(self.top),
            right: conv(self.right),
            bottom: conv(self.bottom),
            left: conv(self.left),
        }
    }

    /// Template properties `safe_t/r/b/l` that the modal window templates use for the spacers
    /// that keep their content out of the unusable screen areas.
    pub fn modal_properties(&self, ui_scale: f32) -> TemplateProperties {
        let insets = self.layout_insets(ui_scale, BASE_MARGIN);
        TemplateProperties::default()
            .with("safe_t", &px(insets.top))
            .with("safe_r", &px(insets.right))
            .with("safe_b", &px(insets.bottom))
            .with("safe_l", &px(insets.left))
    }
}

fn px(value: f32) -> String {
    format!("{value:.1}px")
}

/// Marks an absolutely positioned HUD node whose listed screen edges are kept out of the unusable
/// screen areas: the offset on those sides is the safe-area inset plus [`HUD_EDGE_MARGIN`].
#[derive(Component, Clone, Copy, Default, Debug)]
pub struct HudEdge {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

fn apply_hud_edges(
    safe: Res<SafeArea>,
    ui_scale: Res<UiScale>,
    mut nodes: Query<(&HudEdge, &mut Node)>,
) {
    let insets = safe.layout_insets(ui_scale.0, HUD_EDGE_MARGIN);
    for (edge, mut node) in &mut nodes {
        let set = |flag: bool, current: Val, value: f32| {
            if flag { Val::Px(value) } else { current }
        };
        let (top, right, bottom, left) = (
            set(edge.top, node.top, insets.top),
            set(edge.right, node.right, insets.right),
            set(edge.bottom, node.bottom, insets.bottom),
            set(edge.left, node.left, insets.left),
        );
        if (node.top, node.right, node.bottom, node.left) != (top, right, bottom, left) {
            (node.top, node.right, node.bottom, node.left) = (top, right, bottom, left);
        }
    }
}

pub struct SafeAreaPlugin;

impl Plugin for SafeAreaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SafeArea>()
            .add_systems(Update, apply_hud_edges);
        #[cfg(target_os = "ios")]
        app.add_systems(Update, ios::read_safe_area);
    }
}

#[cfg(target_os = "ios")]
mod ios {
    use super::{SafeArea, landscape_insets};
    use bevy::prelude::*;
    use bevy::window::PrimaryWindow;
    use bevy_ios_safearea::IosSafeArea;

    /// Copies the UIKit insets (read by `bevy_ios_safearea`) into [`SafeArea`], fixed up for the
    /// current window orientation.
    pub fn read_safe_area(
        ios: IosSafeArea,
        windows: Query<&Window, With<PrimaryWindow>>,
        mut safe: ResMut<SafeArea>,
        mut logged: Local<Option<SafeArea>>,
    ) {
        let Ok(window) = windows.single() else {
            return;
        };
        let raw = SafeArea {
            top: ios.top(),
            right: ios.right(),
            bottom: ios.bottom(),
            left: ios.left(),
        };
        let fixed = landscape_insets(raw, window.width(), window.height());
        if *logged != Some(raw) {
            info!(
                "iOS safe area: raw {raw:?} -> {fixed:?} for a {}x{} window",
                window.width(),
                window.height()
            );
            *logged = Some(raw);
        }
        if *safe != fixed {
            *safe = fixed;
        }
    }
}

/// Bottom inset of an iPhone held sideways (home indicator).
const LANDSCAPE_BOTTOM_INSET: f32 = 21.0;

/// UIKit reports the insets of the orientation the view was first laid out in, which on a phone
/// that starts in portrait (or a window that is not rotated yet) means "notch at the top, nothing
/// on the sides" even though the window is now landscape. In a landscape window the notch/Dynamic
/// Island and the rounded corners are on the sides: such insets are rotated (the top inset becomes
/// both side insets, like UIKit reports it) and nothing is reserved at the top.
pub fn landscape_insets(raw: SafeArea, width: f32, height: f32) -> SafeArea {
    let landscape = width > height;
    if landscape && raw.top > 0.0 && raw.left == 0.0 && raw.right == 0.0 {
        return SafeArea {
            top: 0.0,
            right: raw.top,
            left: raw.top,
            bottom: raw.bottom.min(LANDSCAPE_BOTTOM_INSET),
        };
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insets_are_converted_to_layout_px_and_get_the_margin() {
        let safe = SafeArea {
            top: 0.0,
            right: 59.0,
            bottom: 21.0,
            left: 59.0,
        };
        let insets = safe.layout_insets(0.8, 12.0);
        assert!((insets.left - (59.0 / 0.8 + 12.0)).abs() < 1e-4);
        assert!((insets.bottom - (21.0 / 0.8 + 12.0)).abs() < 1e-4);
        assert_eq!(insets.top, 12.0);
    }

    #[test]
    fn portrait_insets_in_a_landscape_window_move_to_the_sides() {
        let stale = SafeArea {
            top: 59.0,
            right: 0.0,
            bottom: 34.0,
            left: 0.0,
        };
        let fixed = landscape_insets(stale, 852.0, 393.0);
        assert_eq!(
            fixed,
            SafeArea {
                top: 0.0,
                right: 59.0,
                bottom: 21.0,
                left: 59.0
            }
        );
        // A portrait window and already-correct landscape insets are left alone.
        assert_eq!(landscape_insets(stale, 393.0, 852.0), stale);
        let ok = SafeArea {
            top: 0.0,
            right: 62.0,
            bottom: 21.0,
            left: 62.0,
        };
        assert_eq!(landscape_insets(ok, 852.0, 393.0), ok);
    }

    #[test]
    fn no_insets_leaves_only_the_margin() {
        let insets = SafeArea::default().layout_insets(1.0, 12.0);
        assert_eq!(
            insets,
            LayoutInsets {
                top: 12.0,
                right: 12.0,
                bottom: 12.0,
                left: 12.0
            }
        );
    }

    #[test]
    fn bad_scale_and_negative_insets_are_ignored() {
        let safe = SafeArea {
            top: -5.0,
            right: 10.0,
            bottom: 0.0,
            left: 0.0,
        };
        let insets = safe.layout_insets(0.0, 0.0);
        assert_eq!(insets.top, 0.0);
        assert_eq!(insets.right, 10.0);
    }
}
