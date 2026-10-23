use bevy::prelude::*;
use bevy_hui::prelude::*;
use bevy_hui_widgets::prelude::TextInput;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::config::GameConfig;
use crate::google_signin::{GoogleSignInError, GoogleSignInResult, request_google_id_token_for};
use crate::state::AppState;
use nakama_client::{AccountCheck, ProfileStatus, check_google_account, restore_session};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NicknameError {
    TooShort,
    TooLong,
    InvalidCharacters,
}

impl std::fmt::Display for NicknameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "Nickname must be at least 3 characters long"),
            Self::TooLong => write!(f, "Nickname must be at most 16 characters long"),
            Self::InvalidCharacters => {
                write!(
                    f,
                    "Nickname may only contain letters, numbers, and underscores"
                )
            }
        }
    }
}

impl std::error::Error for NicknameError {}

/// Validates whether a nickname conforms to platform requirements:
/// - 3 to 16 characters
/// - ASCII alphanumeric characters and underscores only
pub fn validate_nickname(nickname: &str) -> Result<(), NicknameError> {
    let trimmed = nickname.trim();
    if trimmed.len() < 3 {
        return Err(NicknameError::TooShort);
    }
    if trimmed.len() > 16 {
        return Err(NicknameError::TooLong);
    }
    for c in trimmed.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(NicknameError::InvalidCharacters);
        }
    }
    Ok(())
}

/// Persistent user profile stored locally on the client device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserProfile {
    pub device_id: String,
    pub nickname: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub appearance: Option<protocol::AvatarAppearance>,
    /// The profile belongs to a Google account, so the next launch signs in with Google again.
    #[serde(default)]
    pub google_account: bool,
    /// Nakama refresh token of the Google login, so the next launch does not ask to sign in again.
    #[serde(default)]
    pub refresh_token: Option<String>,
}

impl UserProfile {
    pub fn new(device_id: String, nickname: String) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            device_id,
            nickname,
            created_at,
            appearance: None,
            google_account: false,
            refresh_token: None,
        }
    }

    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let profile = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(profile)
    }
}

/// Resolves the storage path for a profile based on optional profile name and storage dir.
pub fn resolve_profile_path(profile_name: Option<&str>) -> PathBuf {
    let filename = match profile_name {
        Some(name) if !name.trim().is_empty() => format!("profile_{}.json", name.trim()),
        _ => "profile.json".to_string(),
    };

    if let Ok(custom_dir) = std::env::var("GAME_STORAGE_DIR") {
        PathBuf::from(custom_dir).join(filename)
    } else {
        PathBuf::from(filename)
    }
}

/// Bevy resource tracking the UI state of the nickname entry screen.
#[derive(Resource, Default, Debug)]
pub struct NicknameInputState {
    pub current_input: String,
    pub error_message: Option<String>,
}

/// UI state of the login screen: the in-flight Google request and the last failure message.
#[derive(Resource, Default)]
pub struct LoginState {
    pub status: Option<String>,
    pending: Option<Mutex<Receiver<GoogleSignInResult>>>,
    /// Account check (`get_profile_status`) running after the id_token arrived.
    checking: Option<crossbeam_channel::Receiver<Result<AccountCheck, String>>>,
}

/// The saved Google session being restored at launch (see [`check_auth_status`]).
#[derive(Resource, Default)]
struct SessionRestore {
    pending: Option<crossbeam_channel::Receiver<Result<AccountCheck, String>>>,
}

#[derive(Component)]
pub struct LoginScreenRoot;

#[derive(Component)]
pub struct NicknameScreenRoot;

#[derive(Resource)]
struct IdentityTemplates {
    login: Handle<HtmlTemplate>,
    nickname: Handle<HtmlTemplate>,
}

const NICKNAME_MAX_LEN: usize = 16;

pub struct IdentityPlugin;

impl Plugin for IdentityPlugin {
    fn build(&self, app: &mut App) {
        crate::ensure_hui_plugins(app);
        app.init_resource::<NicknameInputState>()
            .init_resource::<LoginState>()
            .init_resource::<SessionRestore>()
            .add_systems(Startup, setup_identity_ui)
            .add_systems(
                Update,
                check_auth_status
                    .run_if(in_state(AppState::InitAuth))
                    .run_if(crate::cdn_assets::cdn_settled),
            )
            .add_systems(OnEnter(AppState::Login), setup_login_screen)
            .add_systems(OnExit(AppState::Login), cleanup_login_screen)
            .add_systems(
                Update,
                (poll_google_sign_in, update_login_screen_ui).run_if(in_state(AppState::Login)),
            )
            .add_systems(OnEnter(AppState::ChooseNickname), setup_nickname_screen)
            .add_systems(OnExit(AppState::ChooseNickname), cleanup_nickname_screen)
            .add_systems(
                Update,
                (
                    nickname_enter_submit.after(crate::ime::return_key_bridge),
                    restore_nickname_input,
                    update_nickname_screen_ui,
                )
                    .run_if(in_state(AppState::ChooseNickname)),
            );
    }
}

/// State after the identity is settled: character creator when no appearance was saved yet.
fn state_after_identity(config: &GameConfig) -> AppState {
    let has_saved_appearance = if let Some(path) = &config.profile_path
        && let Ok(profile) = UserProfile::load_from_file(path)
    {
        profile.appearance.is_some()
    } else {
        false
    };

    if !has_saved_appearance && !config.is_dev_mode {
        AppState::CharacterCreator
    } else {
        AppState::Connecting
    }
}

/// Decides whether to show login, open character creator, or proceed to connecting. A saved
/// Google session is restored first, so a returning player does not sign in on every launch.
fn check_auth_status(
    mut config: ResMut<GameConfig>,
    mut restore: ResMut<SessionRestore>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if let Some(pending) = &restore.pending {
        match pending.try_recv() {
            Ok(Ok(check)) => {
                restore.pending = None;
                config.refresh_token = check.refresh_token.or(config.refresh_token.take());
                next_state.set(apply_profile_status(&check.status, &mut config));
            }
            Ok(Err(message)) => {
                warn!("Saved login could not be restored: {message}");
                restore.pending = None;
                config.refresh_token = None;
                next_state.set(AppState::Login);
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                restore.pending = None;
                config.refresh_token = None;
                next_state.set(AppState::Login);
            }
        }
        return;
    }

    if config.needs_nickname_setup
        && !config.is_dev_mode
        && let Some(refresh_token) = config.refresh_token.clone()
    {
        restore.pending = Some(restore_session(
            config.host.clone(),
            config.port,
            config.server_key.clone(),
            refresh_token,
        ));
    } else if config.needs_nickname_setup {
        next_state.set(AppState::Login);
    } else {
        next_state.set(state_after_identity(&config));
    }
}

/// Adopts the nickname stored on the server for a returning Google account and returns where to go
/// next; a new account is sent to the nickname screen.
pub fn apply_profile_status(status: &ProfileStatus, config: &mut GameConfig) -> AppState {
    if !status.nickname_set {
        return AppState::ChooseNickname;
    }
    config.username = status.username.clone();
    config.needs_nickname_setup = false;
    save_profile(config);
    state_after_identity(config)
}

/// Keeps the refresh token in the Keychain/Keystore and out of the profile file; on platforms
/// without a secure store (desktop, development) the profile file holds it.
pub fn persist_refresh_token(config: &GameConfig, profile: &mut UserProfile) {
    let key = crate::secure_store::refresh_token_key(config.profile_name.as_deref());
    match &config.refresh_token {
        Some(token) if crate::secure_store::save(&key, token) => profile.refresh_token = None,
        Some(token) => profile.refresh_token = Some(token.clone()),
        None => {
            crate::secure_store::delete(&key);
            profile.refresh_token = None;
        }
    }
}

/// Persists the nickname (keeping any saved appearance) unless running in dev mode.
fn save_profile(config: &GameConfig) {
    if config.is_dev_mode {
        return;
    }
    let Some(path) = &config.profile_path else {
        return;
    };
    let mut profile = UserProfile::load_from_file(path)
        .unwrap_or_else(|_| UserProfile::new(config.device_id.clone(), config.username.clone()));
    profile.nickname = config.username.clone();
    profile.google_account = config.is_google_account();
    persist_refresh_token(config, &mut profile);
    if let Err(e) = profile.save_to_file(path) {
        warn!("Failed to save profile to disk: {}", e);
    } else {
        info!("Profile saved successfully to {:?}", path);
    }
}

/// Forgets the Google login (in memory and on disk) so the next launch starts at the login
/// screen. The appearance and the device id stay on the device.
pub fn sign_out(config: &mut GameConfig) {
    let was_google = config.is_google_account();
    config.google_id_token = None;
    config.refresh_token = None;
    config.needs_nickname_setup = true;
    crate::secure_store::delete(&crate::secure_store::refresh_token_key(
        config.profile_name.as_deref(),
    ));
    if was_google {
        // The nickname belongs to the Google account; the server returns it at the next login.
        config.username = String::new();
    }
    if config.is_dev_mode {
        return;
    }
    let Some(path) = &config.profile_path else {
        return;
    };
    if let Ok(mut profile) = UserProfile::load_from_file(path) {
        profile.google_account = false;
        profile.refresh_token = None;
        if let Err(e) = profile.save_to_file(path) {
            warn!("Failed to save profile after sign-out: {e}");
        }
    }
}

/// What the login screen does after a Google sign-in attempt resolves.
#[derive(Debug, PartialEq, Eq)]
pub enum LoginOutcome {
    /// Continue to the nickname screen (with Google, or with the device login as fallback).
    Proceed,
    /// Stay on the login screen and show this message.
    Retry(String),
}

/// Friendly login failure text; outside production it carries the technical reason (error
/// domain/code or server answer, never a token) so a failing build can be diagnosed on the device.
pub fn google_failure_message(reason: &str, config: &GameConfig) -> String {
    const FRIENDLY: &str = "Não foi possível entrar com o Google.";
    if config.game_env.shows_status_panel() && !reason.trim().is_empty() {
        format!(
            "{FRIENDLY}\n{}",
            crate::game_log::truncate_line(reason.trim(), 200)
        )
    } else {
        FRIENDLY.to_string()
    }
}

/// Applies a Google sign-in result to the config. Without a native bridge (desktop/dev) the game
/// falls back to the device login, i.e. no id_token.
pub fn apply_google_result(result: GoogleSignInResult, config: &mut GameConfig) -> LoginOutcome {
    match result {
        Ok(id_token) => {
            config.google_id_token = Some(id_token);
            LoginOutcome::Proceed
        }
        Err(GoogleSignInError::Unavailable) => {
            config.google_id_token = None;
            LoginOutcome::Proceed
        }
        Err(GoogleSignInError::Cancelled) => {
            LoginOutcome::Retry("Login cancelado. Tente novamente.".to_string())
        }
        Err(GoogleSignInError::Failed(reason)) => {
            warn!("Google sign-in failed: {reason}");
            LoginOutcome::Retry(google_failure_message(&reason, config))
        }
    }
}

/// Attempts to validate and save the nickname, transitioning to AppState::Connecting on success.
pub fn submit_nickname(
    input: &str,
    config: &mut GameConfig,
    state: &mut NicknameInputState,
    next_state: &mut NextState<AppState>,
) -> bool {
    match validate_nickname(input) {
        Ok(()) => {
            config.username = input.trim().to_string();
            config.needs_nickname_setup = false;

            save_profile(config);

            next_state.set(AppState::CharacterCreator);
            true
        }
        Err(err) => {
            state.error_message = Some(err.to_string());
            false
        }
    }
}

/// Keeps only the characters a nickname may contain, up to the maximum length.
pub fn sanitize_nickname_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(NICKNAME_MAX_LEN)
        .collect()
}

fn setup_identity_ui(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut functions: HtmlFunctions,
) {
    commands.insert_resource(IdentityTemplates {
        login: server.load("ui/login.html"),
        nickname: server.load("ui/nickname.html"),
    });

    functions.register(
        "login_google",
        |_: In<Entity>, mut state: ResMut<LoginState>, config: Res<GameConfig>| {
            if state.pending.is_none() && state.checking.is_none() {
                state.status = None;
                state.pending = Some(Mutex::new(request_google_id_token_for(config.game_env)));
            }
        },
    );
    functions.register(
        "nickname_changed",
        |In(entity): In<Entity>,
         mut inputs: Query<&mut TextInput>,
         mut state: ResMut<NicknameInputState>| {
            let Ok(mut input) = inputs.get_mut(entity) else {
                return;
            };
            let clean = sanitize_nickname_input(&input.value);
            if clean != input.value {
                input.value = clean.clone();
            }
            if state.current_input != clean {
                state.current_input = clean;
                state.error_message = None;
            }
        },
    );
    functions.register(
        "nickname_submit",
        |_: In<Entity>,
         mut state: ResMut<NicknameInputState>,
         mut config: ResMut<GameConfig>,
         mut next_state: ResMut<NextState<AppState>>| {
            let input = state.current_input.clone();
            submit_nickname(&input, &mut config, &mut state, &mut next_state);
        },
    );
}

fn setup_login_screen(
    mut commands: Commands,
    templates: Res<IdentityTemplates>,
    mut state: ResMut<LoginState>,
) {
    state.status = None;
    state.pending = None;
    state.checking = None;
    commands.spawn((
        LoginScreenRoot,
        HtmlNode(templates.login.clone()),
        GlobalZIndex(50),
    ));
}

fn cleanup_login_screen(mut commands: Commands, query: Query<Entity, With<LoginScreenRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn poll_google_sign_in(
    mut state: ResMut<LoginState>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if let Some(checking) = &state.checking {
        match checking.try_recv() {
            Ok(Ok(check)) => {
                state.checking = None;
                config.refresh_token = check.refresh_token;
                next_state.set(apply_profile_status(&check.status, &mut config));
            }
            Ok(Err(message)) => {
                warn!("Google account check failed: {message}");
                state.checking = None;
                config.google_id_token = None;
                state.status = Some(google_failure_message(&message, &config));
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                state.checking = None;
                config.google_id_token = None;
                state.status = Some(google_failure_message("account check aborted", &config));
            }
        }
        return;
    }
    let Some(pending) = &state.pending else {
        return;
    };
    let result = match pending.lock().map(|rx| rx.try_recv()) {
        Ok(Ok(result)) => result,
        Ok(Err(TryRecvError::Empty)) => return,
        // The bridge dropped its sender without answering.
        Ok(Err(TryRecvError::Disconnected)) | Err(_) => Err(GoogleSignInError::Cancelled),
    };
    state.pending = None;
    match apply_google_result(result, &mut config) {
        LoginOutcome::Proceed => match config.google_id_token.clone() {
            // Find out whether this Google account already has a nickname.
            Some(id_token) => {
                state.checking = Some(check_google_account(
                    config.host.clone(),
                    config.port,
                    config.server_key.clone(),
                    id_token,
                ));
            }
            None => next_state.set(AppState::ChooseNickname),
        },
        LoginOutcome::Retry(message) => state.status = Some(message),
    }
}

fn update_login_screen_ui(state: Res<LoginState>, mut texts: Query<(&UiId, &mut Text)>) {
    if !state.is_changed() {
        return;
    }
    for (id, mut text) in &mut texts {
        if id.id() == "login_status" {
            // A lone space keeps the (otherwise empty) text node laid out.
            text.0 = state.status.clone().unwrap_or_else(|| " ".to_string());
        }
    }
}

fn setup_nickname_screen(mut commands: Commands, templates: Res<IdentityTemplates>) {
    commands.spawn((
        NicknameScreenRoot,
        HtmlNode(templates.nickname.clone()),
        GlobalZIndex(50),
    ));
}

fn cleanup_nickname_screen(mut commands: Commands, query: Query<Entity, With<NicknameScreenRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Enter inside the field (which the widget turns into "unfocus") also submits the nickname.
fn nickname_enter_submit(
    keys: Res<ButtonInput<KeyCode>>,
    inputs: Query<(), (With<TextInput>, With<UiActive>)>,
    mut was_active: Local<bool>,
    mut state: ResMut<NicknameInputState>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let active = !inputs.is_empty();
    if *was_active && !active && keys.just_pressed(KeyCode::Enter) {
        let input = state.current_input.clone();
        submit_nickname(&input, &mut config, &mut state, &mut next_state);
    }
    *was_active = active;
}

/// A freshly built input starts with the text already typed.
fn restore_nickname_input(
    state: Res<NicknameInputState>,
    mut inputs: Query<(&mut TextInput, &UiId), Added<TextInput>>,
) {
    for (mut input, id) in &mut inputs {
        if id.id() == "nick_input" {
            input.value = state.current_input.clone();
        }
    }
}

fn update_nickname_screen_ui(
    state: Res<NicknameInputState>,
    mut texts: Query<(&UiId, &mut Text)>,
    mut placeholders: Query<(&UiId, &mut Visibility)>,
) {
    let wanted = if state.current_input.is_empty() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (id, mut visibility) in &mut placeholders {
        if id.id() == "nick_input_ph" && *visibility != wanted {
            *visibility = wanted;
        }
    }
    if !state.is_changed() {
        return;
    }
    for (id, mut text) in &mut texts {
        if id.id() == "nick_error" {
            // A lone space keeps the (otherwise empty) text node laid out.
            text.0 = state
                .error_message
                .clone()
                .unwrap_or_else(|| " ".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_validate_nickname_valid() {
        assert!(validate_nickname("Alice").is_ok());
        assert!(validate_nickname("Player_1").is_ok());
        assert!(validate_nickname("abc").is_ok());
        assert!(validate_nickname("a_very_long_name").is_ok()); // 16 chars
    }

    #[test]
    fn test_validate_nickname_too_short() {
        assert_eq!(validate_nickname(""), Err(NicknameError::TooShort));
        assert_eq!(validate_nickname("ab"), Err(NicknameError::TooShort));
        assert_eq!(validate_nickname(" a "), Err(NicknameError::TooShort));
    }

    #[test]
    fn test_validate_nickname_too_long() {
        assert_eq!(
            validate_nickname("a_very_long_name_that_exceeds"),
            Err(NicknameError::TooLong)
        );
    }

    #[test]
    fn test_validate_nickname_invalid_characters() {
        assert_eq!(
            validate_nickname("Alice 12"),
            Err(NicknameError::InvalidCharacters)
        );
        assert_eq!(
            validate_nickname("Alice!"),
            Err(NicknameError::InvalidCharacters)
        );
        assert_eq!(
            validate_nickname("Bob@home"),
            Err(NicknameError::InvalidCharacters)
        );
        assert_eq!(
            validate_nickname("user-name"),
            Err(NicknameError::InvalidCharacters)
        );
    }

    #[test]
    fn test_user_profile_save_and_load() {
        let temp_dir = env::temp_dir().join(format!("test_profile_{}", uuid::Uuid::new_v4()));
        let profile_path = temp_dir.join("profile.json");

        let original = UserProfile::new("dev-12345".to_string(), "TestNick".to_string());
        assert!(original.save_to_file(&profile_path).is_ok());

        let loaded = UserProfile::load_from_file(&profile_path).expect("Failed to load profile");
        assert_eq!(loaded.device_id, original.device_id);
        assert_eq!(loaded.nickname, original.nickname);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_profile_path() {
        assert_eq!(resolve_profile_path(None), PathBuf::from("profile.json"));
        assert_eq!(
            resolve_profile_path(Some("session1")),
            PathBuf::from("profile_session1.json")
        );
    }

    #[test]
    fn test_submit_nickname_invalid() {
        let mut config = GameConfig {
            host: "127.0.0.1".into(),
            port: 7350,
            server_key: "defaultkey".into(),
            device_id: "test-dev".into(),
            username: "".into(),
            initial_room: "lobby".into(),
            profile_name: None,
            is_dev_mode: true,
            game_env: crate::config::GameEnv::Dev,
            profile_path: None,
            needs_nickname_setup: true,
            google_id_token: None,
            refresh_token: None,
        };
        let mut state = NicknameInputState::default();
        let mut next_state = NextState::<AppState>::Pending(AppState::InitAuth);

        let success = submit_nickname("ab", &mut config, &mut state, &mut next_state);
        assert!(!success);
        assert!(state.error_message.is_some());
        assert_eq!(config.username, "");
        assert!(config.needs_nickname_setup);
    }

    #[test]
    fn test_submit_nickname_valid() {
        let mut config = GameConfig {
            host: "127.0.0.1".into(),
            port: 7350,
            server_key: "defaultkey".into(),
            device_id: "test-dev".into(),
            username: "".into(),
            initial_room: "lobby".into(),
            profile_name: None,
            is_dev_mode: true,
            game_env: crate::config::GameEnv::Dev,
            profile_path: None,
            needs_nickname_setup: true,
            google_id_token: None,
            refresh_token: None,
        };
        let mut state = NicknameInputState::default();
        let mut next_state = NextState::<AppState>::Pending(AppState::InitAuth);

        let success = submit_nickname("ValidNick_12", &mut config, &mut state, &mut next_state);
        assert!(success);
        assert!(state.error_message.is_none());
        assert_eq!(config.username, "ValidNick_12");
        assert!(!config.needs_nickname_setup);
        assert!(matches!(
            next_state,
            NextState::Pending(AppState::CharacterCreator)
        ));
    }

    #[test]
    fn failure_message_shows_the_reason_outside_prod_only() {
        let mut config = test_config();
        let dev = google_failure_message("com.google.GIDSignIn -4: bad client", &config);
        assert!(dev.starts_with("Não foi possível entrar com o Google.\n"));
        assert!(dev.contains("bad client"));

        config.game_env = crate::config::GameEnv::Prod;
        assert_eq!(
            google_failure_message("com.google.GIDSignIn -4: bad client", &config),
            "Não foi possível entrar com o Google."
        );
        config.game_env = crate::config::GameEnv::Dev;
        assert_eq!(
            google_failure_message("  ", &config),
            "Não foi possível entrar com o Google."
        );
    }

    fn test_config() -> GameConfig {
        GameConfig {
            host: "127.0.0.1".into(),
            port: 7350,
            server_key: "defaultkey".into(),
            device_id: "test-dev".into(),
            username: "".into(),
            initial_room: "lobby".into(),
            profile_name: None,
            is_dev_mode: true,
            game_env: crate::config::GameEnv::Dev,
            profile_path: None,
            needs_nickname_setup: true,
            google_id_token: None,
            refresh_token: None,
        }
    }

    #[test]
    fn test_google_token_is_stored_and_proceeds() {
        let mut config = test_config();
        let outcome = apply_google_result(Ok("tok".into()), &mut config);
        assert_eq!(outcome, LoginOutcome::Proceed);
        assert_eq!(config.google_id_token.as_deref(), Some("tok"));
    }

    #[test]
    fn test_google_unavailable_falls_back_to_device_login() {
        let mut config = test_config();
        let outcome = apply_google_result(Err(GoogleSignInError::Unavailable), &mut config);
        assert_eq!(outcome, LoginOutcome::Proceed);
        assert!(config.google_id_token.is_none());
    }

    #[test]
    fn test_google_cancel_and_failure_stay_on_login() {
        let mut config = test_config();
        assert!(matches!(
            apply_google_result(Err(GoogleSignInError::Cancelled), &mut config),
            LoginOutcome::Retry(_)
        ));
        assert!(matches!(
            apply_google_result(Err(GoogleSignInError::Failed("x".into())), &mut config),
            LoginOutcome::Retry(_)
        ));
        assert!(config.google_id_token.is_none());
    }

    #[test]
    fn test_sanitize_nickname_input() {
        assert_eq!(sanitize_nickname_input("Al ice!_1"), "Alice_1");
        assert_eq!(
            sanitize_nickname_input("abcdefghijklmnopqrstuvwxyz").len(),
            16
        );
    }

    #[test]
    fn test_new_google_account_goes_to_nickname_screen() {
        let mut config = test_config();
        let next = apply_profile_status(&ProfileStatus::default(), &mut config);
        assert_eq!(next, AppState::ChooseNickname);
        assert!(config.needs_nickname_setup);
    }

    #[test]
    fn test_returning_google_account_skips_nickname_screen() {
        let mut config = test_config();
        let status = ProfileStatus {
            nickname_set: true,
            username: "Alice_1".into(),
        };
        let next = apply_profile_status(&status, &mut config);
        assert_eq!(config.username, "Alice_1");
        assert!(!config.needs_nickname_setup);
        // dev mode never opens the creator
        assert_eq!(next, AppState::Connecting);
    }

    #[test]
    fn test_google_profile_roundtrip_flag() {
        let temp_dir = env::temp_dir().join(format!("test_profile_{}", uuid::Uuid::new_v4()));
        let path = temp_dir.join("profile.json");
        let mut config = test_config();
        config.is_dev_mode = false;
        config.profile_path = Some(path.clone());
        config.google_id_token = Some("tok".into());
        config.username = "Alice_1".into();
        save_profile(&config);
        assert!(UserProfile::load_from_file(&path).unwrap().google_account);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_refresh_token_is_saved_with_the_profile() {
        let temp_dir = env::temp_dir().join(format!("test_profile_{}", uuid::Uuid::new_v4()));
        let path = temp_dir.join("profile.json");
        let mut config = test_config();
        config.is_dev_mode = false;
        config.profile_path = Some(path.clone());
        config.refresh_token = Some("refresh".into());
        config.username = "Alice_1".into();
        save_profile(&config);
        let profile = UserProfile::load_from_file(&path).unwrap();
        assert!(profile.google_account);
        assert_eq!(profile.refresh_token.as_deref(), Some("refresh"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_old_profiles_without_refresh_token_still_load() {
        let json = r#"{"device_id":"d","nickname":"n","google_account":true}"#;
        let profile: UserProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.refresh_token, None);
    }

    #[test]
    fn test_sign_out_forgets_the_google_login_but_keeps_the_appearance() {
        let temp_dir = env::temp_dir().join(format!("test_profile_{}", uuid::Uuid::new_v4()));
        let path = temp_dir.join("profile.json");
        let mut config = test_config();
        config.is_dev_mode = false;
        config.profile_path = Some(path.clone());
        config.refresh_token = Some("refresh".into());
        config.google_id_token = Some("tok".into());
        config.username = "Alice_1".into();
        config.needs_nickname_setup = false;
        let mut profile = UserProfile::new("test-dev".into(), "Alice_1".into());
        profile.google_account = true;
        profile.refresh_token = Some("refresh".into());
        profile.appearance = Some(protocol::AvatarAppearance::default());
        profile.save_to_file(&path).unwrap();

        sign_out(&mut config);

        assert!(!config.is_google_account());
        assert!(config.needs_nickname_setup);
        assert_eq!(config.username, "");
        let saved = UserProfile::load_from_file(&path).unwrap();
        assert!(!saved.google_account);
        assert_eq!(saved.refresh_token, None);
        assert!(saved.appearance.is_some());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
