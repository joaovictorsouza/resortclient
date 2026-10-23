// Bevy 0.20 deprecated `Button` and `Interaction` in favour of `Hovered`/`Pressed` + ui_widgets.
// The HUD still reads `Interaction`; migrating it is tracked separately, so the warnings are muted.
#![allow(deprecated)]

#[cfg(target_os = "android")]
pub mod android_google;
pub mod appearance;
pub mod avatar;
pub mod avatar_anim;
pub mod avatar_menu;
pub mod camera;
pub mod catalog_sync;
pub mod cdn_assets;
pub mod character_creator;
pub mod config;
pub mod connection;
pub mod furniture;
pub mod game_log;
pub mod google_signin;
pub mod gpu_compat;
pub mod identity;
pub mod ime;
pub mod input;
pub mod inventory;
#[cfg(target_os = "ios")]
pub mod ios_google;
#[cfg(target_os = "ios")]
pub mod ios_window;
pub mod navigator;
pub mod navigator_ui;
pub mod net;
pub mod preview;
pub mod room;
pub mod safe_area;
pub mod secure_store;
pub mod shop;
pub mod state;
pub mod status_panel;
pub mod touch_scroll;
pub mod ui_scale;
pub mod wallet;

use avian3d::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::RenderCreation;
use bevy::window::{MonitorSelection, WindowMode, WindowResolution};

use avatar::AvatarPlugin;
use avatar_anim::AvatarAnimationPlugin;
use avatar_menu::AvatarMenuPlugin;
use camera::CameraPlugin;
use catalog_sync::CatalogSyncPlugin;
use cdn_assets::CdnAssetsPlugin;
use character_creator::CharacterCreatorPlugin;
use config::GameConfig;
use connection::ConnectionPlugin;
use furniture::FurniturePlugin;
use identity::IdentityPlugin;
use input::InputPlugin;
use inventory::InventoryPlugin;
use navigator::RoomNavigatorPlugin;
use net::NetPlugin;
use room::RoomPlugin;
use safe_area::SafeAreaPlugin;
use shop::ShopPlugin;
use state::AppState;
use status_panel::StatusPanelPlugin;
use touch_scroll::TouchScrollPlugin;
use ui_scale::UiScalePlugin;
use wallet::WalletPlugin;

const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.otf");

#[derive(Resource, Clone, Debug)]
pub struct UiFont(pub Handle<Font>);

impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        if let Some(mut fonts) = world.get_resource_mut::<Assets<Font>>() {
            let font = Font::from_bytes(FONT_BYTES.to_vec());
            return UiFont(fonts.add(font));
        }

        if let Some(asset_server) = world.get_resource::<AssetServer>() {
            UiFont(asset_server.load("fonts/Inter-Regular.otf"))
        } else {
            UiFont(Handle::default())
        }
    }
}

impl UiFont {
    pub fn text_font(&self, size: f32) -> TextFont {
        TextFont {
            font: FontSource::Handle(self.0.clone()),
            font_size: FontSize::Px(size),
            ..default()
        }
    }
}

/// Replacement for the removed `Button` marker: a clickable node that blocks picks behind it and
/// carries the (deprecated, still working) `Interaction` component the HUD systems read.
#[derive(Component, Default, Clone, Copy, Debug)]
#[require(Node, bevy::ui::FocusPolicy::Block, Interaction)]
pub struct LegacyButton;

/// Adds the `bevy_hui` plugins once; every plugin with HTML templates calls this.
pub(crate) fn ensure_hui_plugins(app: &mut App) {
    if !app.is_plugin_added::<bevy_hui::prelude::HuiPlugin>() {
        app.add_plugins(bevy_hui::prelude::HuiPlugin);
    }
    if !app.is_plugin_added::<bevy_hui_widgets::prelude::HuiInputWidgetPlugin>() {
        app.add_plugins(bevy_hui_widgets::prelude::HuiInputWidgetPlugin);
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .init_resource::<GameConfig>()
            .init_resource::<UiFont>()
            .add_plugins(PhysicsPlugins::default())
            .add_plugins(UiScalePlugin)
            .add_plugins(SafeAreaPlugin)
            .add_plugins(ConnectionPlugin)
            .add_plugins(StatusPanelPlugin)
            .add_plugins(TouchScrollPlugin)
            .add_plugins(IdentityPlugin)
            .add_plugins(ime::ImePlugin)
            .add_plugins(CharacterCreatorPlugin)
            .add_plugins(NetPlugin)
            .add_plugins(CatalogSyncPlugin)
            .add_plugins(CdnAssetsPlugin)
            .add_plugins(RoomNavigatorPlugin)
            .add_plugins(CameraPlugin)
            .add_plugins(RoomPlugin)
            .add_plugins(FurniturePlugin)
            .add_plugins(InventoryPlugin)
            .add_plugins(WalletPlugin)
            .add_plugins(ShopPlugin)
            .add_plugins(AvatarPlugin)
            .add_plugins(AvatarAnimationPlugin)
            .add_plugins(AvatarMenuPlugin)
            .add_plugins(InputPlugin);

        #[cfg(target_os = "ios")]
        app.add_plugins((
            bevy_ios_app_delegate::IosAppDelegatePlugin,
            bevy_ios_safearea::IosSafeAreaPlugin,
            ios_window::IosWindowPlugin,
        ))
        .add_observer(ios_google::forward_open_url);
    }
}

/// Desktop gets a fixed 1280x720 window. On mobile the window must be fullscreen so Bevy takes
/// the real screen size; otherwise it keeps the default 1280x720 logical size, which is larger
/// than the device screen and makes the camera framing and the UI overflow.
fn primary_window() -> Window {
    let window = Window {
        title: "new_game - 3D Simulation".into(),
        ..default()
    };
    if cfg!(any(target_os = "ios", target_os = "android")) {
        Window {
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            resizable: false,
            ..window
        }
    } else {
        Window {
            resolution: WindowResolution::new(1280, 720),
            ..window
        }
    }
}

/// Main entrypoint capable of running on Desktop, Android, and iOS
pub fn run() -> AppExit {
    let mut app = App::new();
    // Must come before DefaultPlugins: Bevy builds the asset sources when AssetPlugin is added.
    cdn_assets::install(&mut app);
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(primary_window()),
                ..default()
            })
            .set(LogPlugin {
                custom_layer: game_log::custom_layer,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(gpu_compat::render_settings())),
                ..default()
            }),
    )
    .add_plugins(GamePlugin)
    .run()
}

/// Parses `KEY=VALUE` lines (`#` comments and blank lines ignored) from a dev config file.
pub fn parse_env_file(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

/// Android has no shell environment or command line: the profile goes to the app's private data
/// dir, and `assets/dev.env` (written by `scripts/run-android.sh`) provides `NAKAMA_HOST` etc.
#[cfg(target_os = "android")]
fn configure_android_env(app: &bevy::android::android_activity::AndroidApp) {
    use std::io::Read;

    if let Some(dir) = app.internal_data_path() {
        // SAFETY: called at startup, before any other thread reads the environment.
        unsafe { std::env::set_var("GAME_STORAGE_DIR", dir) };
    }
    let Ok(path) = std::ffi::CString::new("dev.env") else {
        return;
    };
    let Some(mut asset) = app.asset_manager().open(&path) else {
        return;
    };
    let mut content = String::new();
    if asset.read_to_string(&mut content).is_ok() {
        for (key, value) in parse_env_file(&content) {
            // SAFETY: same as above, still single-threaded startup.
            unsafe { std::env::set_var(key, value) };
        }
    }
}

/// Android entry point (NativeActivity loads `libgame.so` and calls this).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: bevy::android::android_activity::AndroidApp) {
    configure_android_env(&android_app);
    let _ = google_signin::register_google_sign_in_bridge(Box::new(
        android_google::AndroidGoogleSignIn,
    ));
    let _ = bevy::android::ANDROID_APP.set(android_app);
    let _ = run();
}

/// C entry point called from the iOS launcher (`ios/main.m`), which owns `main`.
#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn main_rs() {
    let _ = google_signin::register_google_sign_in_bridge(Box::new(ios_google::IosGoogleSignIn));
    let _ = run();
}

#[cfg(test)]
mod env_file_tests {
    use super::parse_env_file;

    #[test]
    fn parses_key_value_lines_ignoring_comments_and_blanks() {
        let parsed = parse_env_file("# dev\n\nNAKAMA_HOST = 192.168.0.5\nNAKAMA_PORT=7350\n");
        assert_eq!(
            parsed,
            vec![
                ("NAKAMA_HOST".to_string(), "192.168.0.5".to_string()),
                ("NAKAMA_PORT".to_string(), "7350".to_string()),
            ]
        );
    }

    #[test]
    fn skips_malformed_lines_and_keeps_equals_inside_values() {
        let parsed = parse_env_file("garbage\n=novalue\nTOKEN=a=b\n");
        assert_eq!(parsed, vec![("TOKEN".to_string(), "a=b".to_string())]);
    }
}

#[cfg(test)]
mod info_plist_tests {
    /// Xcode rewrites the plist with other indentation, so compare without whitespace.
    fn compact(plist: &str) -> String {
        plist.split_whitespace().collect()
    }

    fn info_plist() -> String {
        compact(include_str!("../../../ios/Info.plist"))
    }

    #[test]
    fn status_bar_is_controlled_by_the_view_controller() {
        assert!(
            info_plist().contains("<key>UIViewControllerBasedStatusBarAppearance</key><true/>")
        );
    }

    #[test]
    fn app_is_landscape_only_and_uses_scenes() {
        let plist = info_plist();
        assert!(plist.contains("UIInterfaceOrientationLandscapeLeft"));
        assert!(plist.contains("UIInterfaceOrientationLandscapeRight"));
        assert!(!plist.contains("UIInterfaceOrientationPortrait"));
        assert!(plist.contains("UIApplicationSceneManifest"));
    }

    #[test]
    fn compact_ignores_indentation() {
        assert_eq!(compact("<key>A</key>\n\t<true/>"), "<key>A</key><true/>");
    }
}
