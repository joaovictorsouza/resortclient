//! Connection feedback: remembers the last Nakama event, and draws the "Conectando…" and
//! connection-error screens. Also owns the always-active UI overlay camera, because before
//! `InRoom` no other camera renders (the room camera is inactive until then), which left the
//! window empty.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use nakama_client::NakamaEvent;

use crate::UiFont;
use crate::config::GameConfig;
use crate::state::AppState;
use crate::ui_scale::HUD_SIZE_FACTOR;

/// Render layer of the overlay camera. Nothing in the world lives here, so the 2D overlay camera
/// draws only UI: on the default layer it would also draw world-space `Text2d` (avatar nametags).
pub const OVERLAY_RENDER_LAYER: usize = 31;

/// Order of the overlay camera: above the room (0) and the character-creator studio (10).
const OVERLAY_CAMERA_ORDER: isize = 100;

/// What the Nakama worker last reported, for the status panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetEventSummary {
    #[default]
    None,
    Connecting,
    Connected,
    Disconnected(String),
    Error(String),
}

impl NetEventSummary {
    /// Single-line label, e.g. `Error: Auth error: ...`.
    pub fn label(&self) -> String {
        match self {
            Self::None => "-".to_string(),
            Self::Connecting => "Connecting".to_string(),
            Self::Connected => "Connected".to_string(),
            Self::Disconnected(reason) => format!("Disconnected: {reason}"),
            Self::Error(message) => format!("Error: {message}"),
        }
    }
}

/// Last connection event plus the reason the connection-error screen shows.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct ConnectionStatus {
    pub last_event: NetEventSummary,
    /// Why the game entered `ConnectionError`; cleared when the user retries.
    pub failure: Option<String>,
    /// The worker is restoring a dropped connection; the player is not told.
    pub reconnecting: bool,
}

impl ConnectionStatus {
    /// Records the events the panel cares about; other events are ignored.
    pub fn record(&mut self, event: &NakamaEvent) {
        match event {
            NakamaEvent::Connecting => self.last_event = NetEventSummary::Connecting,
            NakamaEvent::Reconnecting => self.reconnecting = true,
            NakamaEvent::Connected { .. } => {
                self.last_event = NetEventSummary::Connected;
                self.failure = None;
            }
            NakamaEvent::Disconnected { reason } => {
                self.last_event = NetEventSummary::Disconnected(reason.clone());
            }
            NakamaEvent::Error { message } => {
                self.last_event = NetEventSummary::Error(message.clone());
            }
            _ => {}
        }
    }

    /// Stores the reason shown on the error screen.
    pub fn fail(&mut self, reason: impl Into<String>) {
        self.failure = Some(reason.into());
    }
}

/// Turns a raw network error into a short message for players (pt-BR).
pub fn friendly_error_message(reason: &str) -> &'static str {
    let r = reason.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| r.contains(n));
    if has(&[
        "401",
        "unauthorized",
        "unauthenticated",
        "server key",
        "invalid key",
    ]) {
        "O servidor recusou este app (chave de acesso inválida). Instale uma versão atualizada."
    } else if has(&["timed out", "timeout"]) {
        "O servidor demorou demais para responder."
    } else if has(&["tls", "certificate", "handshake"]) {
        "Falha na conexão segura com o servidor."
    } else if has(&[
        "dns",
        "lookup",
        "resolve",
        "no such host",
        "name or service",
    ]) {
        "Não foi possível encontrar o servidor. Verifique sua internet."
    } else if has(&["closed", "disconnect"]) {
        "A conexão com o servidor foi encerrada."
    } else if has(&["refused", "unreachable", "error sending request"]) {
        "Não foi possível conectar ao servidor. Verifique sua internet e tente de novo."
    } else {
        "Não foi possível conectar ao servidor."
    }
}

/// Where the "Tentar de novo" button leads.
pub const RETRY_STATE: AppState = AppState::Connecting;

#[derive(Component)]
pub struct UiOverlayCamera;

#[derive(Component)]
struct ConnectingScreenRoot;

/// The "Conectando…" label of the start screen (also shows the content download progress).
#[derive(Component)]
pub(crate) struct ConnectingText;

#[derive(Component)]
struct ErrorScreenRoot;

#[derive(Component)]
struct RetryButton;

pub struct ConnectionPlugin;

impl Plugin for ConnectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionStatus>()
            .add_systems(Startup, spawn_overlay_camera)
            .add_systems(OnEnter(AppState::InitAuth), spawn_connecting_screen)
            .add_systems(
                OnExit(AppState::InitAuth),
                despawn_all::<ConnectingScreenRoot>,
            )
            .add_systems(OnEnter(AppState::Connecting), spawn_connecting_screen)
            .add_systems(
                OnExit(AppState::Connecting),
                despawn_all::<ConnectingScreenRoot>,
            )
            .add_systems(OnEnter(AppState::ConnectionError), spawn_error_screen)
            .add_systems(
                OnExit(AppState::ConnectionError),
                despawn_all::<ErrorScreenRoot>,
            )
            .add_systems(
                Update,
                retry_on_press.run_if(in_state(AppState::ConnectionError)),
            );
    }
}

fn spawn_overlay_camera(mut commands: Commands) {
    commands.spawn((
        UiOverlayCamera,
        Camera2d,
        RenderLayers::layer(OVERLAY_RENDER_LAYER),
        Camera {
            order: OVERLAY_CAMERA_ORDER,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

fn despawn_all<T: Component>(mut commands: Commands, roots: Query<Entity, With<T>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn fullscreen_backdrop() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(16.0 * HUD_SIZE_FACTOR),
        padding: UiRect::all(Val::Px(24.0 * HUD_SIZE_FACTOR)),
        ..default()
    }
}

const BACKDROP: Color = Color::srgb(0.01, 0.02, 0.09);
const TEXT_MAIN: Color = Color::srgb(0.95, 0.97, 1.0);
const TEXT_DIM: Color = Color::srgb(0.58, 0.64, 0.72);

fn spawn_connecting_screen(
    mut commands: Commands,
    font: Res<UiFont>,
    camera: Single<Entity, With<UiOverlayCamera>>,
) {
    commands
        .spawn((
            ConnectingScreenRoot,
            fullscreen_backdrop(),
            BackgroundColor(BACKDROP),
            GlobalZIndex(40),
            UiTargetCamera(*camera),
        ))
        .with_children(|root| {
            root.spawn((
                ConnectingText,
                Text::new("Conectando…"),
                font.text_font(28.0 * HUD_SIZE_FACTOR),
                TextColor(TEXT_MAIN),
            ));
        });
}

fn spawn_error_screen(
    mut commands: Commands,
    font: Res<UiFont>,
    status: Res<ConnectionStatus>,
    config: Res<GameConfig>,
    camera: Single<Entity, With<UiOverlayCamera>>,
) {
    let reason = status.failure.clone().unwrap_or_default();
    commands
        .spawn((
            ErrorScreenRoot,
            fullscreen_backdrop(),
            BackgroundColor(BACKDROP),
            GlobalZIndex(40),
            UiTargetCamera(*camera),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Sem conexão"),
                font.text_font(28.0 * HUD_SIZE_FACTOR),
                TextColor(TEXT_MAIN),
            ));
            root.spawn((
                Text::new(friendly_error_message(&reason)),
                font.text_font(16.0 * HUD_SIZE_FACTOR),
                TextColor(TEXT_MAIN),
                TextLayout::justify(Justify::Center),
                Node {
                    max_width: Val::Percent(80.0),
                    ..default()
                },
            ));
            // The technical reason is for developers; players only see the friendly message.
            if config.game_env.shows_status_panel() && !reason.is_empty() {
                root.spawn((
                    Text::new(crate::game_log::truncate_line(&reason, 160)),
                    font.text_font(12.0 * HUD_SIZE_FACTOR),
                    TextColor(TEXT_DIM),
                    TextLayout::justify(Justify::Center),
                    Node {
                        max_width: Val::Percent(80.0),
                        ..default()
                    },
                ));
            }
            root.spawn((
                RetryButton,
                crate::LegacyButton,
                Node {
                    min_width: Val::Px(200.0 * HUD_SIZE_FACTOR),
                    height: Val::Px(48.0 * HUD_SIZE_FACTOR),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.31, 0.27, 0.9)),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Tentar de novo"),
                    font.text_font(18.0 * HUD_SIZE_FACTOR),
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn retry_on_press(
    buttons: Query<&Interaction, (Changed<Interaction>, With<RetryButton>)>,
    mut status: ResMut<ConnectionStatus>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if buttons.iter().any(|i| *i == Interaction::Pressed) {
        status.failure = None;
        next_state.set(RETRY_STATE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn event_error(message: &str) -> NakamaEvent {
        NakamaEvent::Error {
            message: message.to_string(),
        }
    }

    #[test]
    fn records_connection_lifecycle_events() {
        let mut status = ConnectionStatus::default();
        assert_eq!(status.last_event.label(), "-");

        status.record(&NakamaEvent::Connecting);
        assert_eq!(status.last_event, NetEventSummary::Connecting);

        status.fail("boom");
        status.record(&NakamaEvent::Connected {
            user_id: "u".into(),
            username: "n".into(),
            match_id: "m".into(),
        });
        assert_eq!(status.last_event, NetEventSummary::Connected);
        assert_eq!(
            status.failure, None,
            "a successful connection clears the failure"
        );

        status.record(&NakamaEvent::Disconnected {
            reason: "Server closed connection".into(),
        });
        assert_eq!(
            status.last_event.label(),
            "Disconnected: Server closed connection"
        );

        status.record(&event_error("Auth error: 401"));
        assert_eq!(status.last_event.label(), "Error: Auth error: 401");
    }

    #[test]
    fn unrelated_events_are_ignored() {
        let mut status = ConnectionStatus::default();
        status.record(&NakamaEvent::RoomUpdated);
        assert_eq!(status, ConnectionStatus::default());
    }

    #[test]
    fn friendly_messages_cover_common_failures() {
        let rejected = "Auth error: API error (401): {\"message\":\"Server key invalid\"}";
        assert!(friendly_error_message(rejected).contains("chave de acesso"));
        assert!(friendly_error_message("operation timed out").contains("demorou"));
        assert!(friendly_error_message("invalid peer certificate").contains("segura"));
        assert!(
            friendly_error_message("dns error: failed to lookup address").contains("encontrar")
        );
        assert!(
            friendly_error_message("tcp connect error: Connection refused").contains("conectar")
        );
        assert!(friendly_error_message("Server closed connection").contains("encerrada"));
        assert_eq!(
            friendly_error_message("???"),
            "Não foi possível conectar ao servidor."
        );
        assert_eq!(
            friendly_error_message(""),
            "Não foi possível conectar ao servidor."
        );
    }

    #[test]
    fn overlay_camera_does_not_render_world_entities() {
        let mut app = App::new();
        app.add_systems(Update, spawn_overlay_camera);
        app.update();
        let layers = app
            .world_mut()
            .query_filtered::<&RenderLayers, With<UiOverlayCamera>>()
            .single(app.world())
            .unwrap();
        assert!(!layers.intersects(&RenderLayers::default()));
    }

    #[test]
    fn room_systems_can_still_find_a_single_main_camera() {
        use crate::camera::MainRoomCamera;
        let mut world = World::new();
        world.spawn((MainRoomCamera, Camera::default()));
        world.spawn((UiOverlayCamera, Camera::default()));
        let mut main = world.query_filtered::<&Camera, With<MainRoomCamera>>();
        assert!(main.single(&world).is_ok());
        let mut any = world.query::<&Camera>();
        assert!(
            any.single(&world).is_err(),
            "unfiltered queries see two cameras"
        );
    }

    #[test]
    fn retry_leads_back_to_connecting() {
        assert_eq!(RETRY_STATE, AppState::Connecting);
    }

    fn retry_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .init_resource::<ConnectionStatus>()
            .add_systems(
                Update,
                retry_on_press.run_if(in_state(AppState::ConnectionError)),
            );
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::ConnectionError);
        app.update();
        app
    }

    #[test]
    fn pressing_retry_returns_to_connecting_and_clears_the_failure() {
        let mut app = retry_app();
        app.world_mut().resource_mut::<ConnectionStatus>().fail("x");
        app.world_mut().spawn((RetryButton, Interaction::Pressed));
        app.update(); // system queues the transition
        app.update(); // state applied
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::Connecting
        );
        assert_eq!(app.world().resource::<ConnectionStatus>().failure, None);
    }

    #[test]
    fn hovering_retry_does_not_leave_the_error_state() {
        let mut app = retry_app();
        app.world_mut().spawn((RetryButton, Interaction::Hovered));
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::ConnectionError
        );
    }
}
