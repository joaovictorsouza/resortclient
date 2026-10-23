//! Bridge between the game and the platform's native Google sign-in.
//!
//! Android (Credential Manager) and iOS (GoogleSignIn) register a [`GoogleSignInBridge`] at startup;
//! the login screen then asks for an id_token through [`request_google_id_token`]. Without a bridge
//! (desktop/dev) the request resolves to [`GoogleSignInError::Unavailable`] and the game falls back
//! to the device login.

use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Web client ID used as the `serverClientId`, so the id_token's audience is the Nakama server.
pub const GOOGLE_SERVER_CLIENT_ID: &str =
    "785803086063-jun3ecqn84ogjm34sf6727nnblacg0sj.apps.googleusercontent.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleSignInError {
    /// No native bridge is registered on this platform.
    Unavailable,
    /// The player dismissed the Google account picker.
    Cancelled,
    Failed(String),
}

pub type GoogleSignInResult = Result<String, GoogleSignInError>;

/// Implemented by each native bridge. Must not block: send the outcome on `reply` when it is known.
pub trait GoogleSignInBridge: Send + Sync {
    fn request_id_token(&self, server_client_id: &str, reply: Sender<GoogleSignInResult>);
}

static BRIDGE: OnceLock<Box<dyn GoogleSignInBridge>> = OnceLock::new();

/// Registers the platform bridge; returns it back if one is already registered.
pub fn register_google_sign_in_bridge(
    bridge: Box<dyn GoogleSignInBridge>,
) -> Result<(), Box<dyn GoogleSignInBridge>> {
    BRIDGE.set(bridge)
}

/// Starts a native Google sign-in and returns the channel its outcome arrives on.
pub fn request_google_id_token() -> Receiver<GoogleSignInResult> {
    request_with(BRIDGE.get().map(|b| b.as_ref()))
}

/// Debug aid for simulators/CI: `GAME_SKIP_GOOGLE=1` answers "unavailable" without opening the
/// Google sheet, so the game continues with the device login. Never honoured in production.
pub fn skip_google_requested(env: crate::config::GameEnv, flag: Option<&str>) -> bool {
    env.shows_status_panel() && matches!(flag, Some("1") | Some("true"))
}

/// Like [`request_google_id_token`], but honours [`skip_google_requested`].
pub fn request_google_id_token_for(env: crate::config::GameEnv) -> Receiver<GoogleSignInResult> {
    let flag = std::env::var("GAME_SKIP_GOOGLE").ok();
    if skip_google_requested(env, flag.as_deref()) {
        return request_with(None);
    }
    request_google_id_token()
}

fn request_with(bridge: Option<&dyn GoogleSignInBridge>) -> Receiver<GoogleSignInResult> {
    let (tx, rx) = channel();
    match bridge {
        Some(bridge) => bridge.request_id_token(GOOGLE_SERVER_CLIENT_ID, tx),
        None => {
            let _ = tx.send(Err(GoogleSignInError::Unavailable));
        }
    }
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_flag_is_ignored_in_prod() {
        use crate::config::GameEnv;
        assert!(skip_google_requested(GameEnv::Dev, Some("1")));
        assert!(skip_google_requested(GameEnv::Dev, Some("true")));
        assert!(!skip_google_requested(GameEnv::Dev, Some("0")));
        assert!(!skip_google_requested(GameEnv::Dev, None));
        assert!(!skip_google_requested(GameEnv::Prod, Some("1")));
    }

    struct Fixed(GoogleSignInResult);

    impl GoogleSignInBridge for Fixed {
        fn request_id_token(&self, server_client_id: &str, reply: Sender<GoogleSignInResult>) {
            assert_eq!(server_client_id, GOOGLE_SERVER_CLIENT_ID);
            let _ = reply.send(self.0.clone());
        }
    }

    #[test]
    fn test_without_bridge_is_unavailable() {
        let rx = request_with(None);
        assert_eq!(rx.try_recv(), Ok(Err(GoogleSignInError::Unavailable)));
    }

    #[test]
    fn test_bridge_token_is_delivered() {
        let bridge = Fixed(Ok("id-token".into()));
        let rx = request_with(Some(&bridge));
        assert_eq!(rx.try_recv(), Ok(Ok("id-token".to_string())));
    }

    #[test]
    fn test_bridge_cancel_is_delivered() {
        let bridge = Fixed(Err(GoogleSignInError::Cancelled));
        let rx = request_with(Some(&bridge));
        assert_eq!(rx.try_recv(), Ok(Err(GoogleSignInError::Cancelled)));
    }
}
