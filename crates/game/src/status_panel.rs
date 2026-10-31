//! Developer status panel (non-production only): environment, server, app state, last Nakama
//! event, periodic `/healthcheck` result, build version and the tail of the game log.
//! Tapping the panel collapses/expands it. The server key is never read here.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crossbeam_channel::Receiver;
use nakama_client::health::{HealthResult, spawn_healthcheck};
use std::time::Duration;

use crate::UiFont;
use crate::config::{GameConfig, GameEnv};
use crate::connection::{ConnectionStatus, UiOverlayCamera};
use crate::game_log;
use crate::safe_area::{HUD_EDGE_MARGIN, HudEdge};
use crate::state::AppState;
use crate::ui_scale::HUD_SIZE_FACTOR;

const PROBE_INTERVAL: Duration = Duration::from_secs(10);
const LOG_LINES_SHOWN: usize = 6;
const LOG_LINE_CHARS: usize = 96;

/// `version` plus the short commit when the build knows it.
pub fn build_label(version: &str, commit: Option<&str>) -> String {
    match commit {
        Some(c) if !c.is_empty() => format!("v{version} ({c})"),
        _ => format!("v{version}"),
    }
}

fn current_build_label() -> String {
    build_label(env!("CARGO_PKG_VERSION"), option_env!("GAME_GIT_COMMIT"))
}

/// Latest `/healthcheck` outcome.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HealthState {
    #[default]
    Pending,
    Result(HealthResult),
}

impl HealthState {
    pub fn label(&self) -> String {
        match self {
            Self::Pending => "aguardando…".to_string(),
            Self::Result(HealthResult::Responded {
                ok,
                status,
                latency_ms,
            }) => format!(
                "{} {status} · {latency_ms} ms",
                if *ok { "ok" } else { "falhou" }
            ),
            Self::Result(HealthResult::Failed { error, latency_ms }) => {
                format!("falhou: {error} · {latency_ms} ms")
            }
        }
    }

    fn short_label(&self) -> String {
        match self {
            Self::Pending => "…".to_string(),
            Self::Result(HealthResult::Responded { ok, latency_ms, .. }) => {
                format!("{} {latency_ms}ms", if *ok { "ok" } else { "falhou" })
            }
            Self::Result(HealthResult::Failed { .. }) => "falhou".to_string(),
        }
    }
}

/// Decides when to probe: right away, then every `interval`, never while one is still running.
#[derive(Debug, Clone)]
pub struct ProbeSchedule {
    interval: Duration,
    since_last: Duration,
    started: bool,
}

impl ProbeSchedule {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            since_last: Duration::ZERO,
            started: false,
        }
    }

    /// Advances time; returns true when a new probe should start now.
    pub fn tick(&mut self, delta: Duration, in_flight: bool) -> bool {
        self.since_last += delta;
        if in_flight {
            return false;
        }
        if !self.started || self.since_last >= self.interval {
            self.started = true;
            self.since_last = Duration::ZERO;
            return true;
        }
        false
    }
}

/// Everything the panel prints. It deliberately has no field for the server key.
pub struct PanelData<'a> {
    pub env: GameEnv,
    pub host: &'a str,
    pub port: u16,
    pub app_state: &'a str,
    pub last_event: &'a str,
    pub health: &'a HealthState,
    pub build: &'a str,
    /// e.g. `844x390 @3x`, from the Bevy window.
    pub window: &'a str,
    pub log_tail: &'a [String],
}

/// One-line summary shown while the panel is collapsed.
pub fn collapsed_text(data: &PanelData) -> String {
    format!(
        "{} · {} · {} · health {}",
        data.env.as_str().to_uppercase(),
        data.app_state,
        data.build,
        data.health.short_label()
    )
}

/// `844x390 @3x` for the window's logical size and scale factor.
pub fn window_label(width: f32, height: f32, scale_factor: f32) -> String {
    format!("{width:.0}x{height:.0} @{scale_factor}x")
}

/// Full text shown while the panel is expanded.
pub fn expanded_text(data: &PanelData) -> String {
    let mut text = format!(
        "{} · {}:{}\nEstado: {}\nNakama: {}\nHealth: {}\nJanela: {}\n{}",
        data.env.as_str().to_uppercase(),
        data.host,
        data.port,
        data.app_state,
        game_log::truncate_line(data.last_event, LOG_LINE_CHARS),
        data.health.label(),
        data.window,
        data.build
    );
    if !data.log_tail.is_empty() {
        text.push_str("\n--- log");
        for line in data.log_tail {
            text.push('\n');
            text.push_str(&game_log::truncate_line(line, LOG_LINE_CHARS));
        }
    }
    text
}

#[derive(Resource)]
struct ServerHealth {
    state: HealthState,
    schedule: ProbeSchedule,
    in_flight: Option<Receiver<HealthResult>>,
}

#[derive(Resource, Default)]
struct PanelExpanded(bool);

#[derive(Component)]
struct StatusPanelButton;

#[derive(Component)]
struct StatusPanelText;

pub struct StatusPanelPlugin;

impl Plugin for StatusPanelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerHealth {
            state: HealthState::Pending,
            schedule: ProbeSchedule::new(PROBE_INTERVAL),
            in_flight: None,
        })
        .init_resource::<PanelExpanded>()
        .add_systems(PostStartup, spawn_panel.run_if(panel_enabled))
        .add_systems(
            Update,
            (poll_health, toggle_panel, refresh_panel_text)
                .chain()
                .run_if(panel_enabled),
        );
    }
}

fn panel_enabled(config: Res<GameConfig>) -> bool {
    config.game_env.shows_status_panel()
}

fn spawn_panel(
    mut commands: Commands,
    font: Res<UiFont>,
    camera: Single<Entity, With<UiOverlayCamera>>,
) {
    commands
        .spawn((
            StatusPanelButton,
            crate::LegacyButton,
            HudEdge {
                bottom: true,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                // Bottom, right of the corner buttons (character-creator rotate/random) and left of
                // the confirm button, so the dev panel never sits on top of a control.
                left: Val::Percent(15.0),
                bottom: Val::Px(HUD_EDGE_MARGIN),
                max_width: Val::Percent(60.0),
                padding: UiRect::all(Val::Px(6.0 * HUD_SIZE_FACTOR)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(1000),
            UiTargetCamera(*camera),
        ))
        .with_children(|panel| {
            panel.spawn((
                StatusPanelText,
                Text::new(""),
                font.text_font(11.0 * HUD_SIZE_FACTOR),
                TextColor(Color::srgb(0.75, 1.0, 0.8)),
            ));
        });
}

fn poll_health(time: Res<Time<Real>>, config: Res<GameConfig>, mut health: ResMut<ServerHealth>) {
    if let Some(rx) = &health.in_flight
        && let Ok(result) = rx.try_recv()
    {
        health.state = HealthState::Result(result);
        health.in_flight = None;
    }
    let in_flight = health.in_flight.is_some();
    if health.schedule.tick(time.delta(), in_flight) {
        health.in_flight = Some(spawn_healthcheck(config.host.clone(), config.port));
    }
}

fn toggle_panel(
    buttons: Query<&Interaction, (Changed<Interaction>, With<StatusPanelButton>)>,
    mut expanded: ResMut<PanelExpanded>,
) {
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        expanded.0 = !expanded.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_panel_text(
    config: Res<GameConfig>,
    app_state: Res<State<AppState>>,
    connection: Res<ConnectionStatus>,
    health: Res<ServerHealth>,
    expanded: Res<PanelExpanded>,
    window: Single<&Window, With<PrimaryWindow>>,
    tier: Option<Res<toon_avatar::DeviceTier>>,
    mut text: Single<&mut Text, With<StatusPanelText>>,
) {
    let log_tail = if expanded.0 {
        game_log::recent_lines(LOG_LINES_SHOWN)
    } else {
        Vec::new()
    };
    let state_name = format!("{:?}", app_state.get());
    let last_event = connection.last_event.label();
    let build = match tier {
        Some(tier) => format!("{} · gráficos {}", current_build_label(), tier.label()),
        None => current_build_label(),
    };
    let window_text = window_label(window.width(), window.height(), window.scale_factor());
    let data = PanelData {
        env: config.game_env,
        host: &config.host,
        port: config.port,
        app_state: &state_name,
        last_event: &last_event,
        health: &health.state,
        build: &build,
        window: &window_text,
        log_tail: &log_tail,
    };
    let new_text = if expanded.0 {
        expanded_text(&data)
    } else {
        collapsed_text(&data)
    };
    if text.0 != new_text {
        text.0 = new_text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::NetEventSummary;

    fn sample<'a>(health: &'a HealthState, tail: &'a [String]) -> PanelData<'a> {
        PanelData {
            env: GameEnv::Dev,
            host: "resort-svr-dev.example.com",
            port: 443,
            app_state: "Connecting",
            last_event: "Error: Auth error: 401",
            health,
            build: "v0.1.0 (abc12345)",
            window: "844x390 @3x",
            log_tail: tail,
        }
    }

    #[test]
    fn build_label_includes_commit_only_when_known() {
        assert_eq!(build_label("0.1.0", Some("abc123")), "v0.1.0 (abc123)");
        assert_eq!(build_label("0.1.0", None), "v0.1.0");
        assert_eq!(build_label("0.1.0", Some("")), "v0.1.0");
    }

    #[test]
    fn window_label_rounds_the_size() {
        assert_eq!(window_label(843.6, 390.2, 3.0), "844x390 @3x");
        assert_eq!(window_label(1280.0, 720.0, 1.5), "1280x720 @1.5x");
    }

    #[test]
    fn health_labels() {
        assert_eq!(HealthState::Pending.label(), "aguardando…");
        let ok = HealthState::Result(HealthResult::Responded {
            ok: true,
            status: 200,
            latency_ms: 85,
        });
        assert_eq!(ok.label(), "ok 200 · 85 ms");
        let bad = HealthState::Result(HealthResult::Responded {
            ok: false,
            status: 502,
            latency_ms: 12,
        });
        assert_eq!(bad.label(), "falhou 502 · 12 ms");
        let down = HealthState::Result(HealthResult::Failed {
            error: "timeout".into(),
            latency_ms: 5000,
        });
        assert_eq!(down.label(), "falhou: timeout · 5000 ms");
    }

    #[test]
    fn expanded_text_lists_environment_server_state_event_and_health() {
        let health = HealthState::Result(HealthResult::Responded {
            ok: true,
            status: 200,
            latency_ms: 85,
        });
        let text = expanded_text(&sample(&health, &[]));
        assert!(text.contains("DEV · resort-svr-dev.example.com:443"));
        assert!(text.contains("Estado: Connecting"));
        assert!(text.contains("Nakama: Error: Auth error: 401"));
        assert!(text.contains("Health: ok 200 · 85 ms"));
        assert!(text.contains("Janela: 844x390 @3x"));
        assert!(text.contains("v0.1.0 (abc12345)"));
        assert!(!text.contains("--- log"));
    }

    #[test]
    fn expanded_text_appends_the_log_tail() {
        let tail = vec!["one".to_string(), "two".to_string()];
        let text = expanded_text(&sample(&HealthState::Pending, &tail));
        assert!(text.ends_with("--- log\none\ntwo"));
    }

    #[test]
    fn collapsed_text_is_a_single_line() {
        let text = collapsed_text(&sample(&HealthState::Pending, &[]));
        assert_eq!(text, "DEV · Connecting · v0.1.0 (abc12345) · health …");
        assert!(!text.contains('\n'));
    }

    #[test]
    fn panel_never_prints_the_server_key() {
        // The key is not part of `PanelData`; a config holding one must not leak through the text.
        let config = GameConfig {
            server_key: "super-secret-key".into(),
            ..GameConfig::default()
        };
        let health = HealthState::Pending;
        let data = PanelData {
            env: config.game_env,
            host: &config.host,
            port: config.port,
            app_state: "InRoom",
            last_event: &NetEventSummary::Connected.label(),
            health: &health,
            build: "v0.1.0",
            window: "800x600 @1x",
            log_tail: &[],
        };
        assert!(!expanded_text(&data).contains("super-secret-key"));
        assert!(!collapsed_text(&data).contains("super-secret-key"));
    }

    #[test]
    fn panel_is_hidden_in_prod() {
        assert!(GameEnv::Dev.shows_status_panel());
        assert!(!GameEnv::Prod.shows_status_panel());
    }

    #[test]
    fn schedule_probes_immediately_then_every_interval() {
        let mut s = ProbeSchedule::new(Duration::from_secs(10));
        assert!(s.tick(Duration::from_millis(16), false));
        assert!(!s.tick(Duration::from_secs(5), false));
        assert!(s.tick(Duration::from_secs(5), false));
    }

    #[test]
    fn schedule_waits_for_the_running_probe() {
        let mut s = ProbeSchedule::new(Duration::from_secs(10));
        assert!(s.tick(Duration::ZERO, false));
        assert!(!s.tick(Duration::from_secs(30), true));
        // The interval has long passed, so the next free tick probes again.
        assert!(s.tick(Duration::ZERO, false));
    }
}
