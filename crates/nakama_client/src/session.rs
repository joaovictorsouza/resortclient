//! Helpers for Nakama session tokens.

use base64::Engine;

/// Extracts the Nakama user id (`uid` claim) from a session token (a JWT).
///
/// The match protocol identifies avatars by this id, which differs from the device id
/// used to authenticate. The signature is not verified: the token came straight from
/// the server over the authenticated REST call.
pub fn user_id_from_session_token(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    claims.get("uid")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_with_payload(payload: &str) -> String {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        format!("eyJhbGciOiJIUzI1NiJ9.{encoded}.signature")
    }

    #[test]
    fn reads_uid_claim() {
        let token = token_with_payload(r#"{"uid":"3f1c-uuid","usn":"bob","exp":1}"#);
        assert_eq!(
            user_id_from_session_token(&token).as_deref(),
            Some("3f1c-uuid")
        );
    }

    #[test]
    fn rejects_malformed_tokens() {
        assert_eq!(user_id_from_session_token("not-a-jwt"), None);
        assert_eq!(user_id_from_session_token("a.%%%.c"), None);
        assert_eq!(
            user_id_from_session_token(&token_with_payload(r#"{"usn":"bob"}"#)),
            None
        );
    }
}
