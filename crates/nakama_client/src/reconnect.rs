//! Transparent reconnection: when the game returns from the background (or the network blinks),
//! the worker re-authenticates and re-joins the same match instead of reporting a disconnection.

use std::time::Duration;

use crate::rest::{AuthResponse, NakamaRestClient, RestError};
use crate::socket::{WsStream, connect_ws, send_match_join};

/// Wait before each reconnection attempt; when the last one fails the game shows the error screen.
pub const BACKOFF: [Duration; 6] = [
    Duration::from_millis(0),
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(12),
];

/// A silent connection is dropped after this long: a suspended phone leaves the socket half-open
/// and the OS only notices after minutes.
pub const SILENCE_LIMIT: Duration = Duration::from_secs(30);
/// How often the worker pings the server to keep the silence timer meaningful.
pub const PING_EVERY: Duration = Duration::from_secs(10);

/// What is needed to log in again without the player.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub host: String,
    pub port: u16,
    pub server_key: String,
    pub device_id: String,
    pub username: String,
    /// Google `id_token` of the first login; usually expired by the time it is needed.
    pub google_id_token: Option<String>,
    /// Latest refresh token (Google accounts).
    pub refresh_token: Option<String>,
}

/// Whether an authentication error will not go away by retrying (revoked or expired login).
pub fn is_permanent(error: &RestError) -> bool {
    matches!(error, RestError::Api { status, .. } if (400..500).contains(status) && *status != 408 && *status != 429)
}

async fn authenticate(
    rest: &NakamaRestClient,
    creds: &Credentials,
) -> Result<AuthResponse, RestError> {
    if let Some(refresh) = &creds.refresh_token {
        return rest.refresh_session(refresh).await;
    }
    if let Some(id_token) = &creds.google_id_token {
        return rest.authenticate_google(id_token).await;
    }
    rest.authenticate_device(&creds.device_id, &creds.username)
        .await
}

/// Result of a successful reconnection.
pub struct Reconnected {
    pub rest: NakamaRestClient,
    pub auth: AuthResponse,
    pub ws: WsStream,
}

/// One attempt: new session, new socket, same match.
pub async fn attempt(creds: &Credentials, match_id: &str) -> Result<Reconnected, AttemptError> {
    let rest = NakamaRestClient::new(&creds.host, creds.port, &creds.server_key);
    let auth = authenticate(&rest, creds).await.map_err(|e| {
        if is_permanent(&e) {
            AttemptError::Permanent(e.to_string())
        } else {
            AttemptError::Transient(e.to_string())
        }
    })?;
    let mut ws = connect_ws(&creds.host, creds.port, &auth.token)
        .await
        .map_err(|e| AttemptError::Transient(e.to_string()))?;
    send_match_join(&mut ws, match_id)
        .await
        .map_err(|e| AttemptError::Transient(e.to_string()))?;
    Ok(Reconnected { rest, auth, ws })
}

#[derive(Debug)]
pub enum AttemptError {
    /// Network trouble: try again after the next backoff.
    Transient(String),
    /// The server rejected the login: retrying cannot help.
    Permanent(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn api(status: u16) -> RestError {
        RestError::Api {
            status,
            body: String::new(),
        }
    }

    #[test]
    fn rejected_logins_are_permanent_but_outages_are_not() {
        assert!(is_permanent(&api(401)));
        assert!(is_permanent(&api(400)));
        assert!(!is_permanent(&api(408)));
        assert!(!is_permanent(&api(429)));
        assert!(!is_permanent(&api(502)));
        assert!(!is_permanent(&api(503)));
    }

    #[test]
    fn first_attempt_is_immediate_and_the_wait_grows() {
        assert_eq!(BACKOFF[0], Duration::ZERO);
        assert!(BACKOFF.windows(2).all(|w| w[0] < w[1]));
        let total: Duration = BACKOFF.iter().sum();
        assert!(total < Duration::from_secs(60));
    }
}
