use bevy::prelude::*;
use std::collections::HashMap;
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::identity::{UserProfile, resolve_profile_path};

/// Deployment the build talks to. `Prod` hides the developer status panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GameEnv {
    #[default]
    Dev,
    Prod,
}

impl GameEnv {
    /// Parses `dev`/`prod` (also `production`), case-insensitively; anything else is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dev" | "development" => Some(Self::Dev),
            "prod" | "production" => Some(Self::Prod),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Prod => "prod",
        }
    }

    /// The developer status panel exists in every environment except production.
    pub fn shows_status_panel(self) -> bool {
        self != Self::Prod
    }
}

#[derive(Debug, Clone, Resource)]
pub struct GameConfig {
    pub host: String,
    pub port: u16,
    pub server_key: String,
    pub device_id: String,
    pub username: String,
    pub initial_room: String,
    pub profile_name: Option<String>,
    pub is_dev_mode: bool,
    /// Which deployment this build targets (`GAME_ENV`); defaults to `dev`.
    pub game_env: GameEnv,
    pub profile_path: Option<PathBuf>,
    pub needs_nickname_setup: bool,
    /// Set by the native Google sign-in; makes the connection authenticate with Google.
    pub google_id_token: Option<String>,
    /// Nakama refresh token of a saved Google login; lets the next launch skip the login screen.
    pub refresh_token: Option<String>,
}

impl GameConfig {
    /// Whether the player signed in with Google (now or in a previous launch).
    pub fn is_google_account(&self) -> bool {
        self.google_id_token.is_some() || self.refresh_token.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawConfigInput {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub server_key: Option<String>,
    pub device_id: Option<String>,
    pub username: Option<String>,
    pub initial_room: Option<String>,
    pub profile_name: Option<String>,
    pub is_dev_mode: bool,
    pub game_env: Option<GameEnv>,
}

/// Parses CLI arguments into structured configuration input.
pub fn parse_cli_args(args: &[String]) -> RawConfigInput {
    let mut input = RawConfigInput::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--host" if i + 1 < args.len() => {
                input.host = Some(args[i + 1].clone());
                i += 1;
            }
            "--port" if i + 1 < args.len() => {
                input.port = args[i + 1].parse().ok();
                i += 1;
            }
            "--server-key" if i + 1 < args.len() => {
                input.server_key = Some(args[i + 1].clone());
                i += 1;
            }
            "--device-id" if i + 1 < args.len() => {
                input.device_id = Some(args[i + 1].clone());
                i += 1;
            }
            "--nick" | "--nickname" | "-n" if i + 1 < args.len() => {
                input.username = Some(args[i + 1].clone());
                i += 1;
            }
            "--profile" | "-p" if i + 1 < args.len() => {
                input.profile_name = Some(args[i + 1].clone());
                i += 1;
            }
            "--room" | "-r" if i + 1 < args.len() => {
                input.initial_room = Some(args[i + 1].clone());
                i += 1;
            }
            "--game-env" if i + 1 < args.len() => {
                input.game_env = GameEnv::parse(&args[i + 1]);
                i += 1;
            }
            "--dev" | "-d" => {
                input.is_dev_mode = true;
            }
            _ => {}
        }
        i += 1;
    }

    input
}

/// Parses environment variables into structured configuration input.
pub fn parse_env_vars(env_map: &HashMap<String, String>) -> RawConfigInput {
    let mut input = RawConfigInput::default();

    if let Some(host) = env_map.get("NAKAMA_HOST") {
        input.host = Some(host.clone());
    }
    if let Some(port) = env_map.get("NAKAMA_PORT").and_then(|p| p.parse().ok()) {
        input.port = Some(port);
    }
    if let Some(server_key) = env_map.get("NAKAMA_SERVER_KEY") {
        input.server_key = Some(server_key.clone());
    }
    if let Some(device_id) = env_map.get("GAME_DEVICE_ID") {
        input.device_id = Some(device_id.clone());
    }
    if let Some(username) = env_map.get("GAME_USERNAME") {
        input.username = Some(username.clone());
    }
    if let Some(room) = env_map.get("GAME_ROOM_ID") {
        input.initial_room = Some(room.clone());
    }
    if let Some(profile) = env_map.get("GAME_PROFILE") {
        input.profile_name = Some(profile.clone());
    }
    if let Some(game_env) = env_map.get("GAME_ENV") {
        input.game_env = GameEnv::parse(game_env);
    }
    if let Some(dev) = env_map.get("GAME_DEV_MODE") {
        input.is_dev_mode = dev == "1" || dev.eq_ignore_ascii_case("true");
    }

    input
}

/// Resolves complete GameConfig following the priority: CLI > ENV > Storage > Default.
pub fn resolve_game_config<F>(
    cli: RawConfigInput,
    env: RawConfigInput,
    profile_loader: F,
) -> GameConfig
where
    F: Fn(&Path) -> io::Result<UserProfile>,
{
    let host = cli
        .host
        .or(env.host)
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = cli.port.or(env.port).unwrap_or(7350);
    let server_key = cli
        .server_key
        .or(env.server_key)
        .unwrap_or_else(|| "defaultkey".to_string());
    let initial_room = cli
        .initial_room
        .or(env.initial_room)
        .unwrap_or_else(|| "lobby".to_string());
    let profile_name = cli.profile_name.or(env.profile_name);
    let is_dev_mode = cli.is_dev_mode || env.is_dev_mode;
    let game_env = cli.game_env.or(env.game_env).unwrap_or_default();

    // 1. Dev Mode: Ephemeral identity, never persistent
    if is_dev_mode {
        let device_id = cli
            .device_id
            .or(env.device_id)
            .unwrap_or_else(|| format!("dev-{}", &Uuid::new_v4().to_string()[..8]));

        let username = cli.username.or(env.username).unwrap_or_else(|| {
            let suffix = if device_id.len() >= 8 {
                &device_id[4..8]
            } else {
                "user"
            };
            format!("Dev_{suffix}")
        });

        return GameConfig {
            host,
            port,
            server_key,
            device_id,
            username,
            initial_room,
            profile_name,
            is_dev_mode: true,
            game_env,
            profile_path: None,
            needs_nickname_setup: false,
            google_id_token: None,
            refresh_token: None,
        };
    }

    let resolved_path = resolve_profile_path(profile_name.as_deref());

    // 2. Explicit username provided via CLI or ENV
    if let Some(explicit_user) = cli.username.or(env.username) {
        let existing_profile = profile_loader(&resolved_path).ok();
        let device_id = cli
            .device_id
            .or(env.device_id)
            .or_else(|| existing_profile.as_ref().map(|p| p.device_id.clone()))
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if existing_profile.is_none() {
            let new_prof = UserProfile::new(device_id.clone(), explicit_user.clone());
            let _ = new_prof.save_to_file(&resolved_path);
        }

        return GameConfig {
            host,
            port,
            server_key,
            device_id,
            username: explicit_user,
            initial_room,
            profile_name,
            is_dev_mode: false,
            game_env,
            profile_path: Some(resolved_path),
            needs_nickname_setup: false,
            google_id_token: None,
            refresh_token: None,
        };
    }

    // 3. Check for existing saved profile
    let secret_key = crate::secure_store::refresh_token_key(profile_name.as_deref());
    if let Ok(saved_profile) = profile_loader(&resolved_path) {
        let device_id = cli
            .device_id
            .or(env.device_id)
            .unwrap_or(saved_profile.device_id);

        // Google accounts restore the saved session (or sign in again); the server returns the
        // stored nickname.
        let google_account = saved_profile.google_account;

        return GameConfig {
            host,
            port,
            server_key,
            device_id,
            username: if google_account {
                String::new()
            } else {
                saved_profile.nickname
            },
            initial_room,
            profile_name,
            is_dev_mode: false,
            game_env,
            profile_path: Some(resolved_path),
            needs_nickname_setup: google_account,
            google_id_token: None,
            // The Keychain/Keystore wins; the profile file only holds it on platforms without one.
            refresh_token: crate::secure_store::load(&secret_key).or(saved_profile.refresh_token),
        };
    }

    // 4. First-run: no saved profile and no nickname supplied
    let device_id = cli
        .device_id
        .or(env.device_id)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    GameConfig {
        host,
        port,
        server_key,
        device_id,
        username: String::new(),
        initial_room,
        profile_name,
        is_dev_mode: false,
        game_env,
        profile_path: Some(resolved_path),
        needs_nickname_setup: true,
        google_id_token: None,
        refresh_token: None,
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let env_map: HashMap<String, String> = env::vars().collect();

        let cli_input = parse_cli_args(&args);
        let env_input = parse_env_vars(&env_map);

        resolve_game_config(cli_input, env_input, |path| {
            UserProfile::load_from_file(path)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cli_args_all_flags() {
        let args = vec![
            "game".to_string(),
            "--host".to_string(),
            "192.168.1.10".to_string(),
            "--port".to_string(),
            "8000".to_string(),
            "--nick".to_string(),
            "Alice".to_string(),
            "--profile".to_string(),
            "p1".to_string(),
            "--room".to_string(),
            "park".to_string(),
            "--dev".to_string(),
        ];

        let parsed = parse_cli_args(&args);
        assert_eq!(parsed.host, Some("192.168.1.10".to_string()));
        assert_eq!(parsed.port, Some(8000));
        assert_eq!(parsed.username, Some("Alice".to_string()));
        assert_eq!(parsed.profile_name, Some("p1".to_string()));
        assert_eq!(parsed.initial_room, Some("park".to_string()));
        assert!(parsed.is_dev_mode);
    }

    #[test]
    fn game_env_parses_known_values_only() {
        assert_eq!(GameEnv::parse("dev"), Some(GameEnv::Dev));
        assert_eq!(GameEnv::parse(" PROD "), Some(GameEnv::Prod));
        assert_eq!(GameEnv::parse("production"), Some(GameEnv::Prod));
        assert_eq!(GameEnv::parse("staging"), None);
    }

    #[test]
    fn status_panel_is_hidden_only_in_prod() {
        assert!(GameEnv::Dev.shows_status_panel());
        assert!(!GameEnv::Prod.shows_status_panel());
    }

    #[test]
    fn game_env_defaults_to_dev_and_comes_from_env_or_cli() {
        let resolve = |cli: RawConfigInput, env: RawConfigInput| {
            resolve_game_config(cli, env, |_| {
                Err(io::Error::new(io::ErrorKind::NotFound, "no profile"))
            })
            .game_env
        };
        assert_eq!(
            resolve(RawConfigInput::default(), RawConfigInput::default()),
            GameEnv::Dev
        );

        let mut env_map = HashMap::new();
        env_map.insert("GAME_ENV".to_string(), "prod".to_string());
        let env = parse_env_vars(&env_map);
        assert_eq!(env.game_env, Some(GameEnv::Prod));
        assert_eq!(
            resolve(RawConfigInput::default(), env.clone()),
            GameEnv::Prod
        );

        let cli = parse_cli_args(&["game".into(), "--game-env".into(), "dev".into()]);
        assert_eq!(resolve(cli, env), GameEnv::Dev);
    }

    #[test]
    fn test_cli_overrides_env() {
        let mut env_map = HashMap::new();
        env_map.insert("GAME_USERNAME".to_string(), "EnvUser".to_string());
        env_map.insert("NAKAMA_HOST".to_string(), "10.0.0.1".to_string());

        let env_input = parse_env_vars(&env_map);
        let cli_input = RawConfigInput {
            username: Some("CliUser".to_string()),
            ..Default::default()
        };

        let config = resolve_game_config(cli_input, env_input, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "no profile"))
        });

        assert_eq!(config.username, "CliUser");
        assert_eq!(config.host, "10.0.0.1");
        assert!(!config.needs_nickname_setup);
    }

    #[test]
    fn test_dev_mode_ephemeral_never_needs_nickname_setup() {
        let cli = RawConfigInput {
            is_dev_mode: true,
            ..Default::default()
        };
        let env = RawConfigInput::default();

        let config = resolve_game_config(cli, env, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "no profile"))
        });

        assert!(config.is_dev_mode);
        assert!(!config.needs_nickname_setup);
        assert!(config.username.starts_with("Dev_"));
        assert!(config.device_id.starts_with("dev-"));
        assert_eq!(config.profile_path, None);
    }

    #[test]
    fn test_first_run_without_profile_needs_nickname_setup() {
        let cli = RawConfigInput::default();
        let env = RawConfigInput::default();

        let config = resolve_game_config(cli, env, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "no profile"))
        });

        assert!(!config.is_dev_mode);
        assert!(config.needs_nickname_setup);
        assert_eq!(config.username, "");
        assert!(config.profile_path.is_some());
    }

    #[test]
    fn test_existing_profile_loaded_cleanly() {
        let cli = RawConfigInput::default();
        let env = RawConfigInput::default();

        let config = resolve_game_config(cli, env, |_| {
            Ok(UserProfile {
                device_id: "persisted-device-123".to_string(),
                nickname: "PersistedPlayer".to_string(),
                created_at: 1000,
                appearance: None,
                google_account: false,
                refresh_token: None,
            })
        });

        assert!(!config.needs_nickname_setup);
        assert_eq!(config.username, "PersistedPlayer");
        assert_eq!(config.device_id, "persisted-device-123");
    }

    #[test]
    fn test_google_profile_requires_sign_in_again() {
        let config =
            resolve_game_config(RawConfigInput::default(), RawConfigInput::default(), |_| {
                Ok(UserProfile {
                    device_id: "dev".to_string(),
                    nickname: "Alice_1".to_string(),
                    created_at: 1000,
                    appearance: None,
                    google_account: true,
                    refresh_token: Some("refresh".to_string()),
                })
            });
        // The nickname comes from the server once the saved session is restored.
        assert!(config.needs_nickname_setup);
        assert_eq!(config.username, "");
        assert_eq!(config.refresh_token.as_deref(), Some("refresh"));
        assert!(config.is_google_account());
    }
}
