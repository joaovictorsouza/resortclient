pub mod asset_cache;
pub mod events;
pub mod health;
pub mod reconnect;
pub mod rest;
pub mod session;
pub mod socket;

/// Port that Nakama is reached on through a TLS-terminating proxy (Caddy/Cloudflare).
/// Any other port keeps plain `http`/`ws` (local development).
pub const TLS_PORT: u16 = 443;

/// Per-address TCP connect timeout. Hosts behind Cloudflare publish several IPv4/IPv6 addresses
/// and some networks cannot reach one of them: without a limit per address the client hangs on
/// the dead one instead of trying the next.
pub const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Returns `true` when connections to `port` must use TLS (`https`/`wss`).
pub fn uses_tls(port: u16) -> bool {
    port == TLS_PORT
}

/// Builds `scheme://host[:port]` for the given protocol family (`http` or `ws`).
/// The default port is omitted for TLS endpoints.
pub fn endpoint(base_scheme: &str, host: &str, port: u16) -> String {
    if uses_tls(port) {
        format!("{base_scheme}s://{host}")
    } else {
        format!("{base_scheme}://{host}:{port}")
    }
}

use crossbeam_channel::{Receiver, Sender};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::thread;
use tokio_tungstenite::tungstenite::protocol::Message;

pub use events::{NakamaCommand, NakamaEvent};
use rest::NakamaRestClient;
pub use rest::ProfileStatus;
use socket::{
    connect_ws, parse_incoming_message, send_match_data, send_match_join, send_match_leave,
};

#[derive(Clone)]
pub struct NakamaClientHandle {
    cmd_tx: Sender<NakamaCommand>,
    event_rx: Receiver<NakamaEvent>,
}

impl NakamaClientHandle {
    /// Spawns a dedicated background thread running a Tokio runtime that manages network connections.
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<NakamaCommand>();
        let (event_tx, event_rx) = crossbeam_channel::unbounded::<NakamaEvent>();

        thread::Builder::new()
            .name("nakama-net-worker".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to initialize Tokio runtime for Nakama");

                rt.block_on(run_network_loop(cmd_rx, event_tx));
            })
            .expect("Failed to spawn nakama-net-worker thread");

        Self { cmd_tx, event_rx }
    }

    pub fn send_command(&self, cmd: NakamaCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn try_recv_event(&self) -> Option<NakamaEvent> {
        self.event_rx.try_recv().ok()
    }
}

/// Outcome of the account check: the nickname status and the refresh token to save for the next
/// launch (absent if the server did not return one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountCheck {
    pub status: ProfileStatus,
    pub refresh_token: Option<String>,
}

/// Runs `auth` against the REST client on its own thread, then reads the profile status with the
/// resulting session. The outcome arrives on the channel.
fn spawn_account_check<F>(
    thread_name: &str,
    host: String,
    port: u16,
    server_key: String,
    auth: F,
) -> Receiver<Result<AccountCheck, String>>
where
    F: for<'a> FnOnce(
            &'a NakamaRestClient,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<rest::AuthResponse, String>> + Send + 'a>,
        > + Send
        + 'static,
{
    let (tx, rx) = crossbeam_channel::bounded(1);
    let spawned = thread::Builder::new().name(thread_name.to_string()).spawn({
        let tx = tx.clone();
        move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
                .and_then(|rt| {
                    rt.block_on(async {
                        let rest = NakamaRestClient::new(&host, port, &server_key);
                        let auth = auth(&rest).await?;
                        let status = rest
                            .get_profile_status(&auth.token)
                            .await
                            .map_err(|e| format!("Profile error: {e}"))?;
                        Ok(AccountCheck {
                            status,
                            refresh_token: auth.refresh_token,
                        })
                    })
                });
            let _ = tx.send(result);
        }
    });
    if let Err(e) = spawned {
        let _ = tx.send(Err(e.to_string()));
    }
    rx
}

/// Authenticates with a Google `id_token` (creating the account on first login) and reports whether
/// the account already chose a nickname. Runs on its own thread; the outcome arrives on the channel.
pub fn check_google_account(
    host: String,
    port: u16,
    server_key: String,
    id_token: String,
) -> Receiver<Result<AccountCheck, String>> {
    spawn_account_check("nakama-google-check", host, port, server_key, move |rest| {
        Box::pin(async move {
            rest.authenticate_google(&id_token)
                .await
                .map_err(|e| format!("Auth error: {e}"))
        })
    })
}

/// Restores a saved Google login from its refresh token, skipping the login screen. Same outcome
/// as [`check_google_account`]; an expired or revoked token comes back as an error.
pub fn restore_session(
    host: String,
    port: u16,
    server_key: String,
    refresh_token: String,
) -> Receiver<Result<AccountCheck, String>> {
    spawn_account_check(
        "nakama-session-restore",
        host,
        port,
        server_key,
        move |rest| {
            Box::pin(async move {
                rest.refresh_session(&refresh_token)
                    .await
                    .map_err(|e| format!("Session error: {e}"))
            })
        },
    )
}

/// Revokes a saved login on the server so a copy of its refresh token stops working. Best effort
/// and fire-and-forget: the player is already signed out locally, so a failure (e.g. offline)
/// only means the token lives until it expires.
pub fn revoke_session(host: String, port: u16, server_key: String, refresh_token: String) {
    let _ = thread::Builder::new()
        .name("nakama-session-revoke".to_string())
        .spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            rt.block_on(async {
                let rest = NakamaRestClient::new(&host, port, &server_key);
                let result = match rest.refresh_session(&refresh_token).await {
                    Ok(auth) => {
                        let refresh = auth.refresh_token.as_deref().unwrap_or(&refresh_token);
                        rest.logout(&auth.token, refresh).await
                    }
                    Err(e) => Err(e),
                };
                if let Err(e) = result {
                    eprintln!("session revoke failed: {e}");
                }
            });
        });
}

async fn run_network_loop(cmd_rx: Receiver<NakamaCommand>, event_tx: Sender<NakamaEvent>) {
    let mut current_match_id: Option<String> = None;
    let mut ws_stream: Option<socket::WsStream> = None;
    let mut auth_token: Option<String> = None;
    let mut rest_client: Option<Arc<NakamaRestClient>> = None;
    let mut current_user_id = String::new();
    let mut current_username = String::new();
    // Kept to log in again without the player after a dropped connection.
    let mut creds: Option<reconnect::Credentials> = None;
    let mut last_rx = std::time::Instant::now();
    let mut last_ping = std::time::Instant::now();
    let mut reconnect_now = false;

    loop {
        // 1. Process pending commands from Bevy main thread
        loop {
            let cmd = match cmd_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                // Every handle was dropped (e.g. the game retried the connection): stop the worker.
                Err(crossbeam_channel::TryRecvError::Disconnected) => return,
            };
            match cmd {
                NakamaCommand::ConnectAndJoinPublicRoom {
                    host,
                    port,
                    server_key,
                    device_id,
                    username,
                    room_id,
                    google_id_token,
                    refresh_token,
                } => {
                    let _ = event_tx.send(NakamaEvent::Connecting);

                    let rest = Arc::new(NakamaRestClient::new(&host, port, &server_key));
                    rest_client = Some(rest.clone());

                    // 1. Authenticate with Google when a token is present, else via device ID.
                    // A Google account that already chose its nickname keeps the stored one.
                    let mut username = username;
                    let auth_result = match &google_id_token {
                        Some(id_token) => match rest.authenticate_google(id_token).await {
                            Ok(a) => match rest.get_profile_status(&a.token).await {
                                Ok(status) if status.nickname_set => {
                                    username = status.username;
                                    Ok(a)
                                }
                                Ok(_) => rest.set_nickname(&a.token, &username).await.map(|()| a),
                                Err(e) => Err(e),
                            },
                            Err(e) => Err(e),
                        },
                        None => match &refresh_token {
                            // Restored login: the account already has its nickname.
                            Some(refresh) => match rest.refresh_session(refresh).await {
                                Ok(a) => match rest.get_profile_status(&a.token).await {
                                    Ok(status) => {
                                        username = status.username;
                                        Ok(a)
                                    }
                                    Err(e) => Err(e),
                                },
                                Err(e) => Err(e),
                            },
                            None => rest.authenticate_device(&device_id, &username).await,
                        },
                    };
                    let auth = match auth_result {
                        Ok(a) => a,
                        Err(e) => {
                            let _ = event_tx.send(NakamaEvent::Error {
                                message: format!("Auth error: {e}"),
                            });
                            continue;
                        }
                    };

                    creds = Some(reconnect::Credentials {
                        host: host.clone(),
                        port,
                        server_key: server_key.clone(),
                        device_id: device_id.clone(),
                        username: username.clone(),
                        google_id_token: google_id_token.clone(),
                        refresh_token: auth.refresh_token.clone().or(refresh_token.clone()),
                    });
                    auth_token = Some(auth.token.clone());
                    // Avatars are keyed by the Nakama user id, not the device id.
                    current_user_id =
                        session::user_id_from_session_token(&auth.token).unwrap_or(device_id);
                    current_username = username.clone();

                    // 2. Query match ID for public room via RPC
                    let join_resp = match rest.join_public_room(&auth.token, &room_id).await {
                        Ok(r) => r,
                        Err(e) => {
                            let _ = event_tx.send(NakamaEvent::Error {
                                message: format!("Join room RPC error: {e}"),
                            });
                            continue;
                        }
                    };

                    // 3. Connect realtime WebSocket
                    let mut ws = match connect_ws(&host, port, &auth.token).await {
                        Ok(w) => w,
                        Err(e) => {
                            let _ = event_tx.send(NakamaEvent::Error {
                                message: format!("WebSocket error: {e}"),
                            });
                            continue;
                        }
                    };

                    // 4. Send match_join envelope
                    if let Err(e) = send_match_join(&mut ws, &join_resp.match_id).await {
                        let _ = event_tx.send(NakamaEvent::Error {
                            message: format!("Match join failed: {e}"),
                        });
                        continue;
                    }

                    current_match_id = Some(join_resp.match_id.clone());
                    ws_stream = Some(ws);
                    last_rx = std::time::Instant::now();

                    let _ = event_tx.send(NakamaEvent::Connected {
                        user_id: current_user_id.clone(),
                        username,
                        match_id: join_resp.match_id,
                    });
                }

                NakamaCommand::SendMessage(msg) => {
                    let Some(ws) = ws_stream.as_mut() else {
                        continue;
                    };
                    let Some(match_id) = current_match_id.as_ref() else {
                        continue;
                    };
                    if let Err(e) = send_match_data(ws, match_id, &msg).await {
                        let _ = event_tx.send(NakamaEvent::Error {
                            message: format!("Failed to send match data: {e}"),
                        });
                    }
                }

                NakamaCommand::SwitchMatch { match_id } => {
                    let Some(ws) = ws_stream.as_mut() else {
                        continue;
                    };
                    if let Some(prev) = current_match_id.take() {
                        let _ = send_match_leave(ws, &prev).await;
                    }
                    if let Err(e) = send_match_join(ws, &match_id).await {
                        let _ = event_tx.send(NakamaEvent::Error {
                            message: format!("Switch match failed: {e}"),
                        });
                        continue;
                    }
                    current_match_id = Some(match_id.clone());
                    let _ = event_tx.send(NakamaEvent::Connected {
                        user_id: current_user_id.clone(),
                        username: current_username.clone(),
                        match_id,
                    });
                }

                NakamaCommand::FetchRoomLayouts => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.list_room_layouts(&token).await {
                                Ok(res) => {
                                    let _ = tx.send(NakamaEvent::RoomLayoutsLoaded(res.layouts));
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to load layouts: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::CreateRoom { name, layout_id } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.create_room(&token, &name, &layout_id).await {
                                Ok(res) => {
                                    let _ = tx.send(NakamaEvent::RoomCreated(res));
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to create room: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::FetchPopularRooms => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.list_popular_rooms(&token).await {
                                Ok(res) => {
                                    let _ = tx.send(NakamaEvent::PopularRoomsLoaded(res.rooms));
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to fetch popular rooms: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::FetchMyRooms => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.list_my_rooms(&token).await {
                                Ok(res) => {
                                    let _ = tx.send(NakamaEvent::MyRoomsLoaded(res.rooms));
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to fetch my rooms: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::SearchRooms { query } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.search_rooms(&token, &query).await {
                                Ok(res) => {
                                    let _ = tx.send(NakamaEvent::SearchResultsLoaded(res.rooms));
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to search rooms: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::UpdateRoom {
                    room_id,
                    name,
                    visibility,
                } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.update_room(&token, &room_id, &name, &visibility).await {
                                Ok(_) => {
                                    let _ = tx.send(NakamaEvent::RoomUpdated);
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to update room: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::DeleteRoom { room_id } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.delete_room(&token, &room_id).await {
                                Ok(_) => {
                                    let _ = tx.send(NakamaEvent::RoomDeleted);
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to delete room: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::RequestJoinRoom { room_id } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.join_room(&token, &room_id).await {
                                Ok(res) => {
                                    let is_pending = res.status == "doorbell_pending";
                                    let req_id_opt = res.request_id.clone();
                                    let match_id_opt = res.match_id.clone();
                                    let _ = tx.send(NakamaEvent::JoinRoomResult(res));

                                    if is_pending
                                        && let (Some(req_id), Some(m_id)) =
                                            (req_id_opt, match_id_opt)
                                    {
                                        let tx_poll = tx.clone();
                                        let rest_poll = rest.clone();
                                        let token_poll = token.clone();
                                        tokio::spawn(async move {
                                            for _ in 0..60 {
                                                tokio::time::sleep(
                                                    std::time::Duration::from_millis(500),
                                                )
                                                .await;
                                                match rest_poll
                                                    .check_doorbell(&token_poll, &m_id, &req_id)
                                                    .await
                                                {
                                                    Ok(check_res) => {
                                                        if check_res.status == "allowed" {
                                                            let _ = tx_poll.send(
                                                                NakamaEvent::JoinRoomResult(
                                                                    protocol::JoinRoomResponse {
                                                                        status: "allowed"
                                                                            .to_string(),
                                                                        match_id: check_res
                                                                            .match_id,
                                                                        request_id: None,
                                                                        reason: None,
                                                                    },
                                                                ),
                                                            );
                                                            return;
                                                        } else if check_res.status == "denied" {
                                                            let _ = tx_poll.send(NakamaEvent::JoinRoomResult(protocol::JoinRoomResponse {
                                                                status: "denied".to_string(),
                                                                match_id: None,
                                                                request_id: None,
                                                                reason: Some("O proprietário recusou sua entrada.".to_string()),
                                                            }));
                                                            return;
                                                        }
                                                    }
                                                    Err(_) => {
                                                        // Continue polling until timeout
                                                    }
                                                }
                                            }
                                            let _ = tx_poll.send(NakamaEvent::JoinRoomResult(protocol::JoinRoomResponse {
                                                status: "denied".to_string(),
                                                match_id: None,
                                                request_id: None,
                                                reason: Some("Tempo de espera esgotado sem resposta do proprietário.".to_string()),
                                            }));
                                        });
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Join room request failed: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::FetchInventory => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let rest = rest.clone();
                        let token = token.clone();
                        let tx = event_tx.clone();
                        tokio::spawn(async move {
                            match rest.get_user_inventory(&token).await {
                                Ok(res) => {
                                    let _ = tx.send(NakamaEvent::InventoryLoaded(res.items));
                                }
                                Err(e) => {
                                    let _ = tx.send(NakamaEvent::Error {
                                        message: format!("Failed to fetch inventory: {e}"),
                                    });
                                }
                            }
                        });
                    }
                }

                NakamaCommand::FetchWallet => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let (rest, token, tx) = (rest.clone(), token.clone(), event_tx.clone());
                        tokio::spawn(async move {
                            let _ = tx.send(match rest.get_wallet(&token).await {
                                Ok(wallet) => wallet_event(wallet, 0, 0, "sync"),
                                Err(e) => rpc_error("fetch wallet", &e),
                            });
                        });
                    }
                }

                NakamaCommand::WalletHeartbeat => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let (rest, token, tx) = (rest.clone(), token.clone(), event_tx.clone());
                        tokio::spawn(async move {
                            let _ = tx.send(match rest.wallet_heartbeat(&token).await {
                                Ok(hb) => {
                                    let reason = if hb.daily_bonus {
                                        "daily_login"
                                    } else if hb.granted_silver > 0 {
                                        "play_time"
                                    } else {
                                        "sync"
                                    };
                                    wallet_event(
                                        protocol::WalletDto {
                                            silver: hb.silver,
                                            gold: hb.gold,
                                        },
                                        hb.granted_silver,
                                        0,
                                        reason,
                                    )
                                }
                                Err(e) => rpc_error("wallet heartbeat", &e),
                            });
                        });
                    }
                }

                NakamaCommand::FetchShopCatalog => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let (rest, token, tx) = (rest.clone(), token.clone(), event_tx.clone());
                        tokio::spawn(async move {
                            let _ = tx.send(match rest.get_shop_catalog(&token).await {
                                Ok(catalog) => NakamaEvent::ShopCatalogLoaded(catalog),
                                Err(e) => rpc_error("fetch shop catalog", &e),
                            });
                        });
                    }
                }

                NakamaCommand::FetchCatalogs { if_version } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let (rest, token, tx) = (rest.clone(), token.clone(), event_tx.clone());
                        tokio::spawn(async move {
                            let _ = tx.send(match rest.get_catalogs(&token, &if_version).await {
                                Ok(catalogs) => NakamaEvent::CatalogsLoaded(Box::new(catalogs)),
                                Err(e) => rpc_error("fetch catalogs", &e),
                            });
                        });
                    }
                }

                NakamaCommand::PurchaseItem {
                    catalog_id,
                    quantity,
                } => {
                    if let (Some(rest), Some(token)) = (&rest_client, &auth_token) {
                        let (rest, token, tx) = (rest.clone(), token.clone(), event_tx.clone());
                        tokio::spawn(async move {
                            let _ = tx.send(
                                match rest.purchase_item(&token, &catalog_id, quantity).await {
                                    Ok(done) => NakamaEvent::PurchaseSucceeded(done),
                                    Err(e) => NakamaEvent::PurchaseFailed {
                                        catalog_id,
                                        message: rest::api_error_message(&e),
                                    },
                                },
                            );
                        });
                    }
                }

                NakamaCommand::ForceReconnect => {
                    // The game came back from the background: the old socket is likely dead.
                    if ws_stream.take().is_some() {
                        reconnect_now = true;
                    }
                }

                NakamaCommand::Disconnect => {
                    creds = None;
                    ws_stream = None;
                    current_match_id = None;
                    let _ = event_tx.send(NakamaEvent::Disconnected {
                        reason: "User requested disconnect".to_string(),
                    });
                }
            }
        }

        // 2. Keep the socket honest and poll incoming messages if connected
        let mut dropped: Option<String> =
            std::mem::take(&mut reconnect_now).then(|| "app resumed".to_string());
        if dropped.is_none()
            && let Some(ws) = ws_stream.as_mut()
        {
            if last_ping.elapsed() >= reconnect::PING_EVERY {
                last_ping = std::time::Instant::now();
                if ws.send(Message::Ping(Vec::new().into())).await.is_err() {
                    dropped = Some("ping failed".to_string());
                }
            }
            if dropped.is_none() && last_rx.elapsed() > reconnect::SILENCE_LIMIT {
                dropped = Some("connection went silent".to_string());
            }
            if dropped.is_none() {
                tokio::select! {
                    biased;
                    Some(msg_result) = ws.next() => {
                        last_rx = std::time::Instant::now();
                        match msg_result {
                            Ok(Message::Text(text)) => {
                                if let Some(evt) = parse_incoming_message(&text) {
                                    let _ = event_tx.send(evt);
                                }
                            }
                            Ok(Message::Close(_)) => {
                                dropped = Some("Server closed connection".to_string());
                            }
                            Err(e) => {
                                dropped = Some(format!("WebSocket disconnected with error: {e}"));
                            }
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(5)) => {
                        // Loop tick to process further Bevy commands
                    }
                }
            }
        } else if dropped.is_none() {
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }

        // 3. A dropped connection is restored silently; only a failure that outlasts every retry
        // reaches the game as a disconnection.
        if let Some(reason) = dropped {
            ws_stream = None;
            let Some(match_id) = current_match_id.clone() else {
                let _ = event_tx.send(NakamaEvent::Disconnected { reason });
                continue;
            };
            let Some(credentials) = creds.as_mut() else {
                current_match_id = None;
                let _ = event_tx.send(NakamaEvent::Disconnected { reason });
                continue;
            };
            let mut last_error = reason;
            let mut restored = None;
            for delay in reconnect::BACKOFF {
                tokio::time::sleep(delay).await;
                // The game dropped its handle (e.g. signed out): nobody is listening any more.
                if event_tx.send(NakamaEvent::Reconnecting).is_err() {
                    return;
                }
                match reconnect::attempt(credentials, &match_id).await {
                    Ok(r) => {
                        restored = Some(r);
                        break;
                    }
                    Err(reconnect::AttemptError::Transient(e)) => last_error = e,
                    Err(reconnect::AttemptError::Permanent(e)) => {
                        last_error = e;
                        break;
                    }
                }
            }
            match restored {
                Some(r) => {
                    if let Some(new_refresh) = r.auth.refresh_token.clone() {
                        credentials.refresh_token = Some(new_refresh);
                    }
                    current_user_id = session::user_id_from_session_token(&r.auth.token)
                        .unwrap_or(current_user_id);
                    auth_token = Some(r.auth.token);
                    rest_client = Some(Arc::new(r.rest));
                    ws_stream = Some(r.ws);
                    last_rx = std::time::Instant::now();
                    let _ = event_tx.send(NakamaEvent::Connected {
                        user_id: current_user_id.clone(),
                        username: current_username.clone(),
                        match_id,
                    });
                }
                None => {
                    current_match_id = None;
                    let _ = event_tx.send(NakamaEvent::Disconnected {
                        reason: format!("Connection lost: {last_error}"),
                    });
                }
            }
        }
    }
}

fn wallet_event(
    wallet: protocol::WalletDto,
    delta_silver: i64,
    delta_gold: i64,
    reason: &str,
) -> NakamaEvent {
    NakamaEvent::WalletUpdated {
        wallet,
        delta_silver,
        delta_gold,
        reason: reason.to_string(),
    }
}

/// Minor RPC failures must not drop the connection: the game ignores errors tagged "RPC error".
fn rpc_error(what: &str, error: &rest::RestError) -> NakamaEvent {
    NakamaEvent::Error {
        message: format!(
            "RPC error: failed to {what}: {}",
            rest::api_error_message(error)
        ),
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    #[test]
    fn plain_endpoints_for_local_ports() {
        assert_eq!(endpoint("http", "127.0.0.1", 7350), "http://127.0.0.1:7350");
        assert_eq!(endpoint("ws", "localhost", 7350), "ws://localhost:7350");
    }

    #[test]
    fn tls_endpoints_for_port_443() {
        assert_eq!(
            endpoint("http", "dev.example.com", 443),
            "https://dev.example.com"
        );
        assert_eq!(
            endpoint("ws", "dev.example.com", 443),
            "wss://dev.example.com"
        );
    }
}
