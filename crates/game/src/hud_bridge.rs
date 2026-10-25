//! Feasibility test of a Compose HUD above the 3D scene (see ADR 0013 and bevy_kmp `docs/hud-compose.md`).
//! The bevy_hui HUD stays; this only mirrors a typed snapshot to the shell and takes one command back.
//!
//! Game -> shell (`HostEvent::App`): `hud_state {"name","silver","gold","in_room","reconnecting"}` whenever it changes.
//! Shell -> game (`HostMessage`): `hud_toggle_navigator`.

use crate::config::GameConfig;
use crate::connection::ConnectionStatus;
use crate::navigator::RoomNavigatorState;
use crate::state::AppState;
use crate::wallet::WalletState;
use bevy::prelude::*;
use bevy_kmp_ffi::bevy_kmp::{HostMessage, Outbox};
use serde::Serialize;

pub const EVT_STATE: &str = "hud_state";
pub const MSG_TOGGLE_NAVIGATOR: &str = "hud_toggle_navigator";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HudState {
    pub name: String,
    pub silver: i64,
    pub gold: i64,
    pub in_room: bool,
    pub reconnecting: bool,
}

/// The Compose HUD test is off unless the build sets `COMPOSE_HUD=1` (see `assets/dev.env`).
pub fn enabled(env: &[(String, String)]) -> bool {
    env.iter().any(|(k, v)| k == "COMPOSE_HUD" && v == "1")
}

pub struct HudBridgePlugin;

impl Plugin for HudBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (publish_state, handle_messages));
    }
}

fn publish_state(
    state: Res<State<AppState>>,
    config: Res<GameConfig>,
    wallet: Res<WalletState>,
    connection: Res<ConnectionStatus>,
    outbox: Res<Outbox>,
    mut last: Local<Option<HudState>>,
) {
    let hud = HudState {
        name: config.username.clone(),
        silver: wallet.balance.silver,
        gold: wallet.balance.gold,
        in_room: *state.get() == AppState::InRoom,
        reconnecting: connection.reconnecting,
    };
    if last.as_ref() != Some(&hud) {
        if let Ok(payload) = serde_json::to_vec(&hud) {
            outbox.send_app(EVT_STATE, payload);
        }
        *last = Some(hud);
    }
}

fn handle_messages(mut messages: MessageReader<HostMessage>, mut nav: ResMut<RoomNavigatorState>) {
    for msg in messages.read() {
        if msg.name == MSG_TOGGLE_NAVIGATOR {
            nav.is_open = !nav.is_open;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hud_test_is_off_by_default() {
        assert!(!enabled(&[]));
        assert!(!enabled(&[("COMPOSE_HUD".into(), "0".into())]));
        assert!(!enabled(&[("OTHER".into(), "1".into())]));
        assert!(enabled(&[("COMPOSE_HUD".into(), "1".into())]));
    }

    #[test]
    fn state_serialises_to_the_documented_keys() {
        let json = serde_json::to_string(&HudState {
            name: "a".into(),
            silver: 3,
            gold: 0,
            in_room: true,
            reconnecting: false,
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"name":"a","silver":3,"gold":0,"in_room":true,"reconnecting":false}"#
        );
    }
}
