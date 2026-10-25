//! Entry points used when a Kotlin Multiplatform shell hosts the game (`kmp-host` feature): the
//! shell owns the window and the Bevy app runs inside its surface through `bevy-kmp`.

use crate::{GamePlugin, cdn_assets, google_signin, gpu_compat};
use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use bevy::render::settings::WgpuSettings;
use bevy_kmp_ffi::bevy_kmp::{GpuInfo, HostConfig, request_service_blocking};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Where the bundled assets live (the shell copies them from the APK/bundle on first run).
fn asset_dir(config: &HostConfig) -> Option<PathBuf> {
    config
        .asset_dir
        .clone()
        .or_else(|| config.data_dir.as_ref().map(|d| d.join("assets")))
}

/// Runs before `DefaultPlugins`: the asset sources are built when `AssetPlugin` is added.
pub fn pre_plugins(app: &mut App, config: &HostConfig) {
    if let Some(dir) = &config.data_dir {
        // SAFETY: runs on the render thread right after it starts, before the game spawns threads.
        unsafe { std::env::set_var("GAME_STORAGE_DIR", dir) };
    }
    match asset_dir(config) {
        Some(dir) => {
            let _ = std::fs::create_dir_all(&dir);
            cdn_assets::install_with(app, move || Box::new(FileAssetReader::new(dir.clone())));
        }
        None => {
            // No bundled assets: everything comes from the CDN.
            cdn_assets::install_with(app, || Box::new(FileAssetReader::new("assets")));
        }
    }
    let _ = google_signin::register_google_sign_in_bridge(Box::new(HostGoogleSignIn));
}

/// Adds the game on top of the app `bevy-kmp` prepared (window, input, lifecycle).
pub fn build_app(app: &mut App, config: &HostConfig) {
    // The native shell draws the login and nickname screens (see `onboarding`).
    app.add_plugins((GamePlugin, crate::onboarding::OnboardingPlugin));
    if crate::hud_bridge::enabled(&config.env) {
        app.add_plugins(crate::hud_bridge::HudBridgePlugin);
    }
}

/// Per-GPU renderer workarounds, applied before the renderer exists (see `gpu_compat`).
pub fn gpu_hook(info: &GpuInfo, settings: &mut WgpuSettings) {
    if gpu_compat::is_affected_adreno(&info.name) {
        settings.constrained_limits = Some(gpu_compat::uniform_skinning_limits());
    }
}

/// Google sign-in is done by the shell (Credential Manager / GoogleSignIn-iOS) as a native
/// service: `google_sign_in` with the server client id, answered with the id token.
struct HostGoogleSignIn;

/// The account picker waits for the player, so the wait is long.
const GOOGLE_TIMEOUT: Duration = Duration::from_secs(300);

impl google_signin::GoogleSignInBridge for HostGoogleSignIn {
    fn request_id_token(
        &self,
        server_client_id: &str,
        reply: Sender<google_signin::GoogleSignInResult>,
    ) {
        let payload = server_client_id.as_bytes().to_vec();
        std::thread::spawn(move || {
            let result = match request_service_blocking("google_sign_in", payload, GOOGLE_TIMEOUT) {
                Ok(r) if r.ok => Ok(String::from_utf8_lossy(&r.payload).to_string()),
                Ok(r) => Err(map_failure(&String::from_utf8_lossy(&r.payload))),
                Err(_) => Err(google_signin::GoogleSignInError::Unavailable),
            };
            let _ = reply.send(result);
        });
    }
}

fn map_failure(reason: &str) -> google_signin::GoogleSignInError {
    match reason {
        "cancelled" => google_signin::GoogleSignInError::Cancelled,
        "unavailable" => google_signin::GoogleSignInError::Unavailable,
        other => google_signin::GoogleSignInError::Failed(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_failures_map_to_the_game_errors() {
        assert_eq!(
            map_failure("cancelled"),
            google_signin::GoogleSignInError::Cancelled
        );
        assert_eq!(
            map_failure("boom"),
            google_signin::GoogleSignInError::Failed("boom".into())
        );
    }
}
