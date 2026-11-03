//! Mirrors the game's state to the Compose shell (the `GameStateRepository` on the Kotlin side): which
//! screen the game is on, the connection, and the current room. Useful to any shell feature, not only the HUD.
//!
//! Game -> shell (`HostEvent::App`):
//! `game_state {"app","connection","error","room","build_mode"}` when it changes and when the host resumes.
//! - `app`: `loading|login|nickname|creator|connecting|in_room|error` (from `AppState`).
//! - `connection`: `idle|connecting|connected|reconnecting|disconnected|error` (from `ConnectionStatus`;
//!   `reconnecting` wins while the worker restores a dropped socket, and the app stays `in_room`).
//! - `error`: a short player-facing message while the connection failed, otherwise `null`.
//! - `room`: `null` outside a room, otherwise `{"id","name","visibility","is_owner"}` (kept while reconnecting).
//! - `build_mode`: `true` while the player is in a room with the build mode on (the shell shows its build controls).

use crate::connection::{ConnectionStatus, NetEventSummary, friendly_error_message};
use crate::furniture::BuildModeState;
use crate::navigator::ActiveRoomMetadata;
use crate::room::ActiveRoom;
use crate::state::AppState;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_kmp_ffi::bevy_kmp::{HostLifecycle, Outbox};
use serde::Serialize;

pub const EVT_GAME_STATE: &str = "game_state";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub visibility: String,
    pub is_owner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameStateSnapshot {
    pub app: &'static str,
    pub connection: &'static str,
    pub error: Option<String>,
    pub room: Option<RoomInfo>,
    pub build_mode: bool,
}

pub fn app_name(state: AppState) -> &'static str {
    match state {
        AppState::InitAuth => "loading",
        AppState::Login => "login",
        AppState::ChooseNickname => "nickname",
        AppState::CharacterCreator => "creator",
        AppState::Connecting => "connecting",
        AppState::InRoom => "in_room",
        AppState::ConnectionError => "error",
    }
}

pub fn connection_name(status: &ConnectionStatus) -> &'static str {
    if status.reconnecting {
        return "reconnecting";
    }
    match status.last_event {
        NetEventSummary::None => "idle",
        NetEventSummary::Connecting => "connecting",
        NetEventSummary::Connected => "connected",
        NetEventSummary::Disconnected(_) => "disconnected",
        NetEventSummary::Error(_) => "error",
    }
}

pub fn snapshot(
    state: AppState,
    status: &ConnectionStatus,
    room: &ActiveRoom,
    meta: &ActiveRoomMetadata,
    build: &BuildModeState,
) -> GameStateSnapshot {
    let in_room = state == AppState::InRoom;
    GameStateSnapshot {
        app: app_name(state),
        connection: connection_name(status),
        error: status
            .failure
            .as_deref()
            .map(|reason| friendly_error_message(reason).to_string()),
        room: in_room.then(|| RoomInfo {
            id: meta.room_id.clone(),
            name: if room.name.is_empty() {
                meta.room_name.clone()
            } else {
                room.name.clone()
            },
            visibility: meta.visibility.clone(),
            is_owner: meta.is_owner,
        }),
        build_mode: in_room && build.is_active,
    }
}

pub struct GameStateBridgePlugin;

impl Plugin for GameStateBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, publish_game_state);
    }
}

#[derive(SystemParam)]
struct SnapshotSources<'w> {
    state: Res<'w, State<AppState>>,
    status: Res<'w, ConnectionStatus>,
    room: Res<'w, ActiveRoom>,
    meta: Res<'w, ActiveRoomMetadata>,
    build: Res<'w, BuildModeState>,
}

fn publish_game_state(
    src: SnapshotSources,
    mut lifecycle: MessageReader<HostLifecycle>,
    outbox: Res<Outbox>,
    mut last: Local<Option<GameStateSnapshot>>,
) {
    let resumed = lifecycle.read().any(|m| *m == HostLifecycle::Resumed);
    let snap = snapshot(
        *src.state.get(),
        &src.status,
        &src.room,
        &src.meta,
        &src.build,
    );
    if resumed || last.as_ref() != Some(&snap) {
        if let Ok(payload) = serde_json::to_vec(&snap) {
            outbox.send_app(EVT_GAME_STATE, payload);
        }
        *last = Some(snap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(is_owner: bool) -> ActiveRoomMetadata {
        ActiveRoomMetadata {
            room_id: "r1".into(),
            room_name: "Minha sala".into(),
            owner_id: None,
            is_owner,
            visibility: "private".into(),
        }
    }

    #[test]
    fn outside_a_room_there_is_no_room() {
        let snap = snapshot(
            AppState::Login,
            &ConnectionStatus::default(),
            &ActiveRoom::default(),
            &meta(true),
            &BuildModeState::default(),
        );
        assert_eq!(snap.app, "login");
        assert_eq!(snap.connection, "idle");
        assert!(snap.room.is_none());
    }

    #[test]
    fn in_a_room_the_room_and_ownership_are_reported() {
        let status = ConnectionStatus {
            last_event: NetEventSummary::Connected,
            ..Default::default()
        };
        let snap = snapshot(
            AppState::InRoom,
            &status,
            &ActiveRoom::default(),
            &meta(true),
            &BuildModeState::default(),
        );
        assert_eq!(snap.connection, "connected");
        let room = snap.room.expect("room");
        assert_eq!((room.id.as_str(), room.name.as_str()), ("r1", "Minha sala"));
        assert!(room.is_owner);
    }

    #[test]
    fn build_mode_only_counts_inside_a_room() {
        let on = BuildModeState {
            is_active: true,
            ..Default::default()
        };
        let status = ConnectionStatus::default();
        let room = ActiveRoom::default();
        let inside = snapshot(AppState::InRoom, &status, &room, &meta(true), &on);
        assert!(inside.build_mode);
        let outside = snapshot(AppState::Login, &status, &room, &meta(true), &on);
        assert!(!outside.build_mode);
    }

    #[test]
    fn reconnecting_keeps_the_room() {
        let status = ConnectionStatus {
            last_event: NetEventSummary::Connected,
            reconnecting: true,
            ..Default::default()
        };
        let snap = snapshot(
            AppState::InRoom,
            &status,
            &ActiveRoom::default(),
            &meta(false),
            &BuildModeState::default(),
        );
        assert_eq!(snap.connection, "reconnecting");
        assert!(snap.room.is_some());
    }

    #[test]
    fn a_failed_connection_reports_a_friendly_error() {
        let mut status = ConnectionStatus {
            last_event: NetEventSummary::Error("boom".into()),
            ..Default::default()
        };
        status.fail("connection refused");
        let snap = snapshot(
            AppState::ConnectionError,
            &status,
            &ActiveRoom::default(),
            &meta(false),
            &BuildModeState::default(),
        );
        assert_eq!((snap.app, snap.connection), ("error", "error"));
        assert!(snap.error.is_some());
        assert!(snap.room.is_none());
    }

    #[test]
    fn payload_has_the_documented_keys() {
        let json = serde_json::to_string(&snapshot(
            AppState::Connecting,
            &ConnectionStatus::default(),
            &ActiveRoom::default(),
            &meta(false),
            &BuildModeState::default(),
        ))
        .unwrap();
        assert_eq!(
            json,
            r#"{"app":"connecting","connection":"idle","error":null,"room":null,"build_mode":false}"#
        );
    }
}
