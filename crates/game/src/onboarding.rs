//! Bridge to the native onboarding (Compose): the shell draws the login and nickname screens and
//! this plugin keeps the game's own state machine (see `identity`) in charge.
//!
//! Shell -> game (`HostMessage`, JSON): `onboarding_reset` (development only: forget the session
//! and go back to the login screen), `onboarding_google`, `onboarding_device_login` (outside
//! production only), `onboarding_nickname {"nickname": ".."}`.
//! Game -> shell (`HostEvent::App`): `onboarding_stage {"stage","busy","error","dev"}` whenever it
//! changes; `stage` is `loading`, `login`, `nickname` or `hidden`.
//! Tokens never travel here: the Google id token arrives through the native `google_sign_in`
//! service and the refresh token is stored through `secure_*`.

use crate::config::GameConfig;
use crate::identity::{LoginState, NativeOnboarding, NicknameInputState, submit_nickname};
use crate::state::AppState;
use bevy::prelude::*;
use bevy_kmp_ffi::bevy_kmp::{HostMessage, Outbox};
use serde::{Deserialize, Serialize};

pub const MSG_GOOGLE: &str = "onboarding_google";
pub const MSG_DEVICE_LOGIN: &str = "onboarding_device_login";
pub const MSG_NICKNAME: &str = "onboarding_nickname";
pub const MSG_RESET: &str = "onboarding_reset";
pub const EVT_STAGE: &str = "onboarding_stage";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    pub stage: String,
    pub busy: bool,
    pub error: Option<String>,
    /// The device login (no Google) is allowed: development environments only.
    pub dev: bool,
}

#[derive(Debug, Deserialize)]
struct NicknameMessage {
    nickname: String,
}

pub struct OnboardingPlugin;

impl Plugin for OnboardingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeOnboarding>()
            .add_systems(Update, (handle_messages, publish_stage));
    }
}

fn stage_name(state: AppState) -> &'static str {
    match state {
        AppState::InitAuth => "loading",
        AppState::Login => "login",
        AppState::ChooseNickname => "nickname",
        _ => "hidden",
    }
}

fn current_stage(
    state: AppState,
    login: &LoginState,
    nickname: &NicknameInputState,
    config: &GameConfig,
) -> Stage {
    let error = match state {
        AppState::Login => login.status.clone(),
        AppState::ChooseNickname => nickname.error_message.clone(),
        _ => None,
    };
    Stage {
        stage: stage_name(state).to_string(),
        busy: login.is_busy(),
        error,
        dev: config.game_env.shows_status_panel(),
    }
}

fn publish_stage(
    state: Res<State<AppState>>,
    login: Res<LoginState>,
    nickname: Res<NicknameInputState>,
    config: Res<GameConfig>,
    outbox: Res<Outbox>,
    mut last: Local<Option<Stage>>,
) {
    let stage = current_stage(*state.get(), &login, &nickname, &config);
    if last.as_ref() != Some(&stage) {
        if let Ok(payload) = serde_json::to_vec(&stage) {
            outbox.send_app(EVT_STAGE, payload);
        }
        *last = Some(stage);
    }
}

/// Development tool: forgets the saved login (Keychain/Keystore token) and the profile file.
fn reset_session(config: &mut GameConfig) {
    crate::identity::sign_out(config);
    if let Some(path) = &config.profile_path
        && let Err(e) = std::fs::remove_file(path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!("Could not remove the profile file: {e}");
    }
    config.username = String::new();
    config.needs_nickname_setup = true;
    info!("Session reset (development tool)");
}

fn handle_messages(
    mut messages: MessageReader<HostMessage>,
    state: Res<State<AppState>>,
    mut login: ResMut<LoginState>,
    mut nickname: ResMut<NicknameInputState>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for msg in messages.read() {
        if msg.name == MSG_RESET && config.game_env.shows_status_panel() {
            reset_session(&mut config);
            login.status = None;
            nickname.current_input.clear();
            nickname.error_message = None;
            next_state.set(AppState::Login);
            continue;
        }
        match (msg.name.as_str(), *state.get()) {
            (MSG_GOOGLE, AppState::Login) => {
                login.start_google(config.game_env);
            }
            (MSG_DEVICE_LOGIN, AppState::Login) if config.game_env.shows_status_panel() => {
                config.google_id_token = None;
                next_state.set(AppState::ChooseNickname);
            }
            (MSG_NICKNAME, AppState::ChooseNickname) => {
                if let Ok(m) = serde_json::from_slice::<NicknameMessage>(&msg.payload) {
                    let input = crate::identity::sanitize_nickname_input(&m.nickname);
                    nickname.current_input = input.clone();
                    nickname.error_message = None;
                    submit_nickname(&input, &mut config, &mut nickname, &mut next_state);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_map_from_the_app_state() {
        assert_eq!(stage_name(AppState::InitAuth), "loading");
        assert_eq!(stage_name(AppState::Login), "login");
        assert_eq!(stage_name(AppState::ChooseNickname), "nickname");
        assert_eq!(stage_name(AppState::InRoom), "hidden");
    }

    #[test]
    fn nickname_error_is_published_on_the_nickname_stage() {
        let login = LoginState::default();
        let nick = NicknameInputState {
            current_input: String::new(),
            error_message: Some("too short".into()),
        };
        let config = GameConfig::default();
        let s = current_stage(AppState::ChooseNickname, &login, &nick, &config);
        assert_eq!(s.error.as_deref(), Some("too short"));
        assert!(!s.busy);
        let s = current_stage(AppState::Login, &login, &nick, &config);
        assert_eq!(s.error, None);
    }
}
