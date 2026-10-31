//! Mirrors the player's wallet to the Compose shell (the `WalletRepository` on the Kotlin side).
//! It is its own message, separate from the HUD snapshot (`hud_bridge`).
//!
//! Game -> shell (`HostEvent::App`): `wallet_state {"silver":i64,"gold":i64,"loaded":bool}`, sent when
//! the balance changes (a server reply, a heartbeat reward, a purchase) and again whenever the host
//! resumes from the background, so a shell that missed a message catches up. `loaded` is false until
//! the first server response (the same flag the bevy_hui HUD uses to show "-").
//! The balance is server-authoritative; nothing flows back (the shell has no wallet command).

use crate::wallet::WalletState;
use bevy::prelude::*;
use bevy_kmp_ffi::bevy_kmp::{HostLifecycle, Outbox};
use serde::Serialize;

pub const EVT_WALLET: &str = "wallet_state";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WalletSnapshot {
    pub silver: i64,
    pub gold: i64,
    pub loaded: bool,
}

impl From<&WalletState> for WalletSnapshot {
    fn from(state: &WalletState) -> Self {
        Self {
            silver: state.balance.silver,
            gold: state.balance.gold,
            loaded: state.loaded,
        }
    }
}

pub struct WalletBridgePlugin;

impl Plugin for WalletBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, publish_wallet);
    }
}

fn publish_wallet(
    wallet: Res<WalletState>,
    mut lifecycle: MessageReader<HostLifecycle>,
    outbox: Res<Outbox>,
    mut last: Local<Option<WalletSnapshot>>,
) {
    let resumed = lifecycle.read().any(|m| *m == HostLifecycle::Resumed);
    let snapshot = WalletSnapshot::from(&*wallet);
    if resumed || last.as_ref() != Some(&snapshot) {
        if let Ok(payload) = serde_json::to_vec(&snapshot) {
            outbox.send_app(EVT_WALLET, payload);
        }
        *last = Some(snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::WalletDto;

    #[test]
    fn snapshot_serialises_to_the_documented_keys() {
        let json = serde_json::to_string(&WalletSnapshot {
            silver: 12,
            gold: 3,
            loaded: true,
        })
        .unwrap();
        assert_eq!(json, r#"{"silver":12,"gold":3,"loaded":true}"#);
    }

    #[test]
    fn an_unloaded_wallet_is_reported_as_not_loaded() {
        let state = WalletState {
            balance: WalletDto { silver: 0, gold: 0 },
            loaded: false,
        };
        assert!(!WalletSnapshot::from(&state).loaded);
    }
}
