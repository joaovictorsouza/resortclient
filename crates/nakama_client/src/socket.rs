use base64::Engine;
use futures_util::SinkExt;
use protocol::{ClientMessage, decode_server_message, encode_client_message};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, client_async_tls,
    tungstenite::{Error as WsError, client::IntoClientRequest, protocol::Message},
};

use crate::events::NakamaEvent;

#[derive(Error, Debug)]
pub enum SocketError {
    #[error("WebSocket connection error: {0}")]
    Connection(Box<WsError>),
    #[error("TCP connect error: {0}")]
    Tcp(String),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

impl From<WsError> for SocketError {
    fn from(err: WsError) -> Self {
        Self::Connection(Box::new(err))
    }
}

#[derive(Serialize)]
struct MatchJoinEnvelope<'a> {
    match_join: MatchJoinPayload<'a>,
}

#[derive(Serialize)]
struct MatchJoinPayload<'a> {
    match_id: &'a str,
}

#[derive(Serialize)]
struct MatchLeaveEnvelope<'a> {
    match_leave: MatchLeavePayload<'a>,
}

#[derive(Serialize)]
struct MatchLeavePayload<'a> {
    match_id: &'a str,
}

#[derive(Serialize)]
struct MatchDataSendEnvelope<'a> {
    match_data_send: MatchDataSendPayload<'a>,
}

#[derive(Serialize)]
struct MatchDataSendPayload<'a> {
    match_id: &'a str,
    op_code: i64,
    data: String,
}

#[derive(Deserialize, Debug)]
struct NakamaIncomingEnvelope {
    match_data: Option<IncomingMatchData>,
    error: Option<IncomingError>,
    notifications: Option<IncomingNotifications>,
}

#[derive(Deserialize, Debug)]
struct IncomingNotifications {
    #[serde(default)]
    notifications: Vec<IncomingNotification>,
}

#[derive(Deserialize, Debug)]
struct IncomingNotification {
    code: i64,
    /// JSON document encoded as a string.
    content: String,
}

#[derive(Deserialize, Debug)]
struct IncomingMatchData {
    op_code: String,
    data: Option<String>,
}

#[derive(Deserialize, Debug)]
struct IncomingError {
    message: String,
}

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn connect_ws(host: &str, port: u16, token: &str) -> Result<WsStream, SocketError> {
    let url = format!(
        "{}/ws?token={token}&format=json",
        crate::endpoint("ws", host, port)
    );
    let request = url.as_str().into_client_request()?;
    let stream = connect_tcp(host, port).await?;
    let (ws_stream, _) = client_async_tls(request, stream).await?;
    Ok(ws_stream)
}

/// Connects to the first reachable address of `host`, giving each one [`crate::CONNECT_TIMEOUT`].
async fn connect_tcp(host: &str, port: u16) -> Result<TcpStream, SocketError> {
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| SocketError::Tcp(format!("lookup {host}: {e}")))?;
    let mut last = format!("no address for {host}");
    for addr in addrs {
        match tokio::time::timeout(crate::CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => return Ok(stream),
            Ok(Err(e)) => last = format!("{addr}: {e}"),
            Err(_) => last = format!("{addr}: timed out"),
        }
    }
    Err(SocketError::Tcp(last))
}

pub async fn send_match_join(ws: &mut WsStream, match_id: &str) -> Result<(), SocketError> {
    let envelope = MatchJoinEnvelope {
        match_join: MatchJoinPayload { match_id },
    };
    let json_str = serde_json::to_string(&envelope)?;
    ws.send(Message::Text(json_str.into())).await?;
    Ok(())
}

pub async fn send_match_leave(ws: &mut WsStream, match_id: &str) -> Result<(), SocketError> {
    let envelope = MatchLeaveEnvelope {
        match_leave: MatchLeavePayload { match_id },
    };
    let json_str = serde_json::to_string(&envelope)?;
    ws.send(Message::Text(json_str.into())).await?;
    Ok(())
}

pub async fn send_match_data(
    ws: &mut WsStream,
    match_id: &str,
    msg: &ClientMessage,
) -> Result<(), SocketError> {
    let (op_code, bytes) = encode_client_message(msg).map_err(|e| {
        SocketError::Json(serde_json::Error::io(std::io::Error::other(e.to_string())))
    })?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let envelope = MatchDataSendEnvelope {
        match_data_send: MatchDataSendPayload {
            match_id,
            op_code,
            data: b64,
        },
    };
    let json_str = serde_json::to_string(&envelope)?;
    ws.send(Message::Text(json_str.into())).await?;
    Ok(())
}

fn wallet_notification_event(n: &IncomingNotification) -> Option<NakamaEvent> {
    if n.code != protocol::NOTIFICATION_WALLET_CHANGED {
        return None;
    }
    let change: protocol::WalletChangedNotification = serde_json::from_str(&n.content).ok()?;
    Some(NakamaEvent::WalletUpdated {
        wallet: protocol::WalletDto {
            silver: change.silver,
            gold: change.gold,
        },
        delta_silver: change.delta_silver,
        delta_gold: change.delta_gold,
        reason: change.reason,
    })
}

pub fn parse_incoming_message(text: &str) -> Option<NakamaEvent> {
    let env: NakamaIncomingEnvelope = serde_json::from_str(text).ok()?;

    if let Some(err) = env.error {
        return Some(NakamaEvent::Error {
            message: err.message,
        });
    }

    if let Some(batch) = env.notifications {
        return batch
            .notifications
            .iter()
            .find_map(wallet_notification_event);
    }

    if let Some(md) = env.match_data {
        let op_code: i64 = md.op_code.parse().ok()?;
        let raw_bytes = if let Some(b64) = md.data {
            base64::engine::general_purpose::STANDARD.decode(b64).ok()?
        } else {
            Vec::new()
        };

        if let Ok(server_msg) = decode_server_message(op_code, &raw_bytes) {
            return Some(NakamaEvent::MessageReceived(Box::new(server_msg)));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_notification_becomes_wallet_updated() {
        let content = r#"{\"silver\":0,\"gold\":50,\"delta_silver\":0,\"delta_gold\":50,\"reason\":\"iap_emulation\"}"#;
        let text = format!(
            r#"{{"notifications":{{"notifications":[{{"id":"n1","subject":"wallet_changed","content":"{content}","code":1001}}]}}}}"#
        );
        match parse_incoming_message(&text) {
            Some(NakamaEvent::WalletUpdated {
                wallet,
                delta_gold,
                reason,
                ..
            }) => {
                assert_eq!(wallet.gold, 50);
                assert_eq!(delta_gold, 50);
                assert_eq!(reason, "iap_emulation");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn unrelated_notifications_are_ignored() {
        let text = r#"{"notifications":{"notifications":[{"id":"n1","subject":"x","content":"{}","code":7}]}}"#;
        assert!(parse_incoming_message(text).is_none());
    }
}

#[cfg(test)]
mod connect_tests {
    use super::*;
    use std::net::TcpListener;

    #[tokio::test]
    async fn connect_tcp_reaches_a_listening_address() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(connect_tcp("127.0.0.1", port).await.is_ok());
    }

    #[tokio::test]
    async fn connect_tcp_reports_the_last_failure() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let err = connect_tcp("127.0.0.1", port).await.unwrap_err();
        assert!(matches!(err, SocketError::Tcp(msg) if msg.contains("127.0.0.1")));
    }
}
