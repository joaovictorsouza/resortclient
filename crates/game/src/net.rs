use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use nakama_client::{NakamaClientHandle, NakamaCommand, NakamaEvent};
use protocol::{
    AvatarAction, AvatarAppearanceUpdated, AvatarExpressionChanged, AvatarHoldingChanged,
    AvatarJoined, AvatarLeft, AvatarPath, AvatarPostureChanged, BuildModeRejected, ClientMessage,
    GetShopCatalogResponse as ShopCatalogResponse, InventoryItemDto, ItemMoved, ItemPlaced,
    ItemRemoved, ItemRotated, MoveRejected, MoveRequest, PetAction, PetInteractResult, PetPath,
    PetPlaced, PetRemoved, PurchaseItemResponse, RoomSnapshot, ServerMessage, WalletDto,
};

use crate::config::GameConfig;
use crate::connection::ConnectionStatus;
use crate::identity::UserProfile;
use crate::navigator::{ActiveRoomMetadata, DoorbellState, RoomNavigatorState};
use crate::state::AppState;

#[derive(Resource)]
pub struct NakamaResource {
    pub handle: NakamaClientHandle,
    pub local_user_id: String,
}

#[derive(Message, Debug, Clone)]
pub struct NetRoomSnapshotEvent(pub RoomSnapshot);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarJoinedEvent(pub AvatarJoined);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarLeftEvent(pub AvatarLeft);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarPathEvent(pub AvatarPath);

#[derive(Message, Debug, Clone)]
pub struct NetMoveRejectedEvent(pub MoveRejected);

#[derive(Message, Debug, Clone)]
pub struct NetItemPlacedEvent(pub ItemPlaced);

#[derive(Message, Debug, Clone)]
pub struct NetItemMovedEvent(pub ItemMoved);

#[derive(Message, Debug, Clone)]
pub struct NetItemRotatedEvent(pub ItemRotated);

#[derive(Message, Debug, Clone)]
pub struct NetItemRemovedEvent(pub ItemRemoved);

#[derive(Message, Debug, Clone)]
pub struct NetBuildModeRejectedEvent(pub BuildModeRejected);

#[derive(Message, Debug, Clone)]
pub struct NetInventoryLoadedEvent(pub Vec<InventoryItemDto>);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarAppearanceUpdatedEvent(pub AvatarAppearanceUpdated);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarPostureChangedEvent(pub AvatarPostureChanged);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarExpressionChangedEvent(pub AvatarExpressionChanged);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarHoldingChangedEvent(pub AvatarHoldingChanged);

#[derive(Message, Debug, Clone)]
pub struct NetAvatarActionEvent(pub AvatarAction);

#[derive(Message, Debug, Clone)]
pub struct NetPetPlacedEvent(pub PetPlaced);

#[derive(Message, Debug, Clone)]
pub struct NetPetPathEvent(pub PetPath);

#[derive(Message, Debug, Clone)]
pub struct NetPetActionEvent(pub PetAction);

#[derive(Message, Debug, Clone)]
pub struct NetPetRemovedEvent(pub PetRemoved);

/// The server's answer to a pet interaction request (to the requester only).
#[derive(Message, Debug, Clone)]
pub struct NetPetInteractResultEvent(pub PetInteractResult);

/// The writers of the pet messages (bundled to stay under Bevy's parameter limit).
#[derive(SystemParam)]
pub struct PetMessageWriters<'w> {
    placed: MessageWriter<'w, NetPetPlacedEvent>,
    path: MessageWriter<'w, NetPetPathEvent>,
    action: MessageWriter<'w, NetPetActionEvent>,
    removed: MessageWriter<'w, NetPetRemovedEvent>,
    interaction: MessageWriter<'w, NetPetInteractResultEvent>,
}

/// The writers of the avatar messages other than movement: appearance, posture, expression, held
/// prop and avatar action (bundled to stay under Bevy's parameter limit).
#[derive(SystemParam)]
pub struct EmoteMessageWriters<'w> {
    appearance: MessageWriter<'w, NetAvatarAppearanceUpdatedEvent>,
    posture: MessageWriter<'w, NetAvatarPostureChangedEvent>,
    expression: MessageWriter<'w, NetAvatarExpressionChangedEvent>,
    holding: MessageWriter<'w, NetAvatarHoldingChangedEvent>,
    action: MessageWriter<'w, NetAvatarActionEvent>,
}

/// New balances. `delta_*` are zero unless the server reported the change (heartbeat rewards,
/// pushed grants), which lets the HUD play the gold sound only for incoming gold.
#[derive(Message, Debug, Clone)]
pub struct NetWalletUpdatedEvent {
    pub wallet: WalletDto,
    pub delta_silver: i64,
    pub delta_gold: i64,
    pub reason: String,
}

#[derive(Message, Debug, Clone)]
pub struct NetShopCatalogLoadedEvent(pub ShopCatalogResponse);

#[derive(Message, Debug, Clone)]
pub struct NetPurchaseSucceededEvent(pub PurchaseItemResponse);

#[derive(Message, Debug, Clone)]
pub struct NetPurchaseFailedEvent {
    pub catalog_id: String,
    pub message: String,
}

#[derive(SystemParam)]
pub struct EconomyMessageWriters<'w> {
    pub wallet: MessageWriter<'w, NetWalletUpdatedEvent>,
    pub catalog: MessageWriter<'w, NetShopCatalogLoadedEvent>,
    pub catalogs: MessageWriter<'w, crate::catalog_sync::NetCatalogsLoadedEvent>,
    pub purchased: MessageWriter<'w, NetPurchaseSucceededEvent>,
    pub purchase_failed: MessageWriter<'w, NetPurchaseFailedEvent>,
}

#[derive(SystemParam)]
pub struct FurnitureMessageWriters<'w> {
    pub placed: MessageWriter<'w, NetItemPlacedEvent>,
    pub moved: MessageWriter<'w, NetItemMovedEvent>,
    pub rotated: MessageWriter<'w, NetItemRotatedEvent>,
    pub removed: MessageWriter<'w, NetItemRemovedEvent>,
    pub rejected: MessageWriter<'w, NetBuildModeRejectedEvent>,
    pub inventory: MessageWriter<'w, NetInventoryLoadedEvent>,
}

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NetRoomSnapshotEvent>()
            .add_message::<NetAvatarJoinedEvent>()
            .add_message::<NetAvatarLeftEvent>()
            .add_message::<NetAvatarPathEvent>()
            .add_message::<NetMoveRejectedEvent>()
            .add_message::<NetItemPlacedEvent>()
            .add_message::<NetItemMovedEvent>()
            .add_message::<NetItemRotatedEvent>()
            .add_message::<NetItemRemovedEvent>()
            .add_message::<NetBuildModeRejectedEvent>()
            .add_message::<NetInventoryLoadedEvent>()
            .add_message::<NetAvatarAppearanceUpdatedEvent>()
            .add_message::<NetAvatarPostureChangedEvent>()
            .add_message::<NetAvatarExpressionChangedEvent>()
            .add_message::<NetAvatarHoldingChangedEvent>()
            .add_message::<NetAvatarActionEvent>()
            .add_message::<NetPetPlacedEvent>()
            .add_message::<NetPetPathEvent>()
            .add_message::<NetPetActionEvent>()
            .add_message::<NetPetRemovedEvent>()
            .add_message::<NetPetInteractResultEvent>()
            .add_message::<NetWalletUpdatedEvent>()
            .add_message::<NetShopCatalogLoadedEvent>()
            .add_message::<NetPurchaseSucceededEvent>()
            .add_message::<NetPurchaseFailedEvent>()
            .add_systems(OnEnter(AppState::Connecting), setup_network)
            .add_systems(
                Update,
                reconnect_after_background.run_if(resource_exists::<NakamaResource>),
            )
            .add_systems(
                Update,
                poll_nakama_events.run_if(resource_exists::<NakamaResource>),
            );
    }
}

/// Whether this lifecycle step ends a suspension: the app is running again after having been
/// suspended. `suspended` carries the state between calls.
pub fn resumed_from_background(suspended: &mut bool, event: bevy::window::AppLifecycle) -> bool {
    use bevy::window::AppLifecycle;
    match event {
        AppLifecycle::WillSuspend | AppLifecycle::Suspended => {
            *suspended = true;
            false
        }
        AppLifecycle::WillResume | AppLifecycle::Running => std::mem::take(suspended),
        AppLifecycle::Idle => false,
    }
}

/// The socket of a phone that slept is dead even when the OS has not noticed yet: log in again and
/// re-join the room as soon as the app is back, without showing anything to the player.
fn reconnect_after_background(
    mut lifecycle: MessageReader<bevy::window::AppLifecycle>,
    mut suspended: Local<bool>,
    net: Res<NakamaResource>,
    state: Res<State<AppState>>,
) {
    for event in lifecycle.read() {
        if resumed_from_background(&mut suspended, *event) && *state.get() == AppState::InRoom {
            info!("App resumed from the background: reconnecting");
            net.handle.send_command(NakamaCommand::ForceReconnect);
        }
    }
}

fn setup_network(mut commands: Commands, config: Res<GameConfig>) {
    info!(
        "Connecting to Nakama at {}:{} as {} (Device: {})",
        config.host, config.port, config.username, config.device_id
    );

    let handle = NakamaClientHandle::spawn();
    handle.send_command(NakamaCommand::ConnectAndJoinPublicRoom {
        host: config.host.clone(),
        port: config.port,
        server_key: config.server_key.clone(),
        device_id: config.device_id.clone(),
        username: config.username.clone(),
        room_id: config.initial_room.clone(),
        google_id_token: config.google_id_token.clone(),
        refresh_token: config.refresh_token.clone(),
    });

    commands.insert_resource(NakamaResource {
        handle,
        local_user_id: config.device_id.clone(),
    });
}

/// What the connect handler reads besides the Nakama state (bundled to stay under Bevy's parameter limit).
#[derive(bevy::ecs::system::SystemParam)]
struct ConnectContext<'w> {
    config: Res<'w, GameConfig>,
    catalog: Res<'w, crate::character_creator::CharacterCatalog>,
}

#[allow(clippy::too_many_arguments)]
fn poll_nakama_events(
    mut net: ResMut<NakamaResource>,
    ctx: ConnectContext,
    mut next_state: ResMut<NextState<AppState>>,
    mut snapshot_events: MessageWriter<NetRoomSnapshotEvent>,
    mut joined_events: MessageWriter<NetAvatarJoinedEvent>,
    mut left_events: MessageWriter<NetAvatarLeftEvent>,
    mut path_events: MessageWriter<NetAvatarPathEvent>,
    mut rejected_events: MessageWriter<NetMoveRejectedEvent>,
    mut nav_state: ResMut<RoomNavigatorState>,
    mut doorbell_state: ResMut<DoorbellState>,
    mut active_meta: ResMut<ActiveRoomMetadata>,
    mut furni_writers: FurnitureMessageWriters,
    mut emotes: EmoteMessageWriters,
    mut pets: PetMessageWriters,
    mut economy: EconomyMessageWriters,
    mut connection: ResMut<ConnectionStatus>,
) {
    let config = &ctx.config;
    let catalog = &ctx.catalog;
    while let Some(event) = net.handle.try_recv_event() {
        // Read before `record`, which ends the reconnection on `Connected`.
        let was_reconnecting = connection.reconnecting;
        connection.record(&event);
        match event {
            NakamaEvent::Connecting => {
                info!("Nakama: Connecting...");
            }
            NakamaEvent::Reconnecting => {
                info!("Nakama: connection dropped, reconnecting...");
            }
            NakamaEvent::Connected {
                match_id, user_id, ..
            } => {
                info!("Nakama: Connected to match {} as {}", match_id, user_id);
                // Room snapshots identify avatars by Nakama user id, not device id.
                net.local_user_id = user_id;
                // After a silent reconnection the player is already in the room: the fresh
                // snapshot replaces the avatars and nothing else is rebuilt.
                if was_reconnecting {
                    connection.reconnecting = false;
                } else {
                    next_state.set(AppState::InRoom);
                }
                net.handle.send_command(NakamaCommand::FetchMyRooms);
                net.handle.send_command(NakamaCommand::FetchPopularRooms);
                net.handle.send_command(NakamaCommand::FetchCatalogs {
                    if_version: crate::catalog_sync::cached_version(config),
                });

                // Sync saved local appearance so the server (and other clients) see
                // the correct avatar immediately on connect.
                // Without a catalog (first run, nothing cached yet) there is nothing to map the parts
                // onto, and sanitizing against an empty list would wipe the saved avatar.
                if let Some(profile_path) = &config.profile_path
                    && !catalog.items.is_empty()
                    && let Ok(profile) = UserProfile::load_from_file(profile_path)
                    && let Some(mut appearance) = profile.appearance
                {
                    use protocol::UpdateAppearanceRequest;
                    // A profile saved with a previous asset pack would be rejected
                    // by the server, so map its parts onto the current catalog.
                    crate::character_creator::sanitize_appearance(&catalog.items, &mut appearance);
                    let req = UpdateAppearanceRequest { appearance };
                    net.handle.send_command(NakamaCommand::SendMessage(
                        ClientMessage::UpdateAppearance(Box::new(req)),
                    ));
                    info!("Synced saved appearance to Nakama on connect");
                }
            }
            NakamaEvent::MessageReceived(msg) => match *msg {
                ServerMessage::RoomSnapshot(snap) => {
                    info!("Received RoomSnapshot for room: {}", snap.room_id);
                    active_meta.room_id = snap.room_id.clone();
                    active_meta.room_name = snap.room_name.clone();

                    // Always recomputed for the room being entered, never inherited from the previous one.
                    let (owner_id, is_owner) = crate::navigator::resolve_ownership(
                        &snap.room_id,
                        &snap.owner_id,
                        &net.local_user_id,
                        &nav_state.my_rooms,
                    );
                    active_meta.owner_id = owner_id;
                    active_meta.is_owner = is_owner;
                    if snap.room_id == "lobby" || snap.room_id == "cafe" {
                        active_meta.visibility = "public".to_string();
                    } else if let Some(listed) = nav_state
                        .my_rooms
                        .iter()
                        .chain(nav_state.popular_rooms.iter())
                        .chain(nav_state.search_results.iter())
                        .find(|r| r.room_id == snap.room_id)
                    {
                        active_meta.visibility = listed.visibility.clone();
                    }

                    snapshot_events.write(NetRoomSnapshotEvent(snap));
                }
                ServerMessage::AvatarJoined(joined) => {
                    info!("Avatar joined: {}", joined.display_name);
                    joined_events.write(NetAvatarJoinedEvent(joined));
                }
                ServerMessage::AvatarLeft(left) => {
                    info!("Avatar left: {}", left.user_id);
                    left_events.write(NetAvatarLeftEvent(left));
                }
                ServerMessage::AvatarPath(path) => {
                    path_events.write(NetAvatarPathEvent(path));
                }
                ServerMessage::MoveRejected(rej) => {
                    warn!("Movement rejected by server: {}", rej.reason);
                    rejected_events.write(NetMoveRejectedEvent(rej));
                }
                ServerMessage::DoorbellRequest(req) => {
                    info!(
                        "Received doorbell request from {}: {}",
                        req.requester_name, req.request_id
                    );
                    doorbell_state.incoming_request = Some(req);
                }
                ServerMessage::RoomClosed(closed) => {
                    info!(
                        "Current room closed by owner ({}), returning to lobby...",
                        closed.reason
                    );
                    active_meta.is_owner = false;
                    active_meta.owner_id = None;
                    active_meta.visibility = "public".to_string();
                    net.handle.send_command(NakamaCommand::RequestJoinRoom {
                        room_id: "lobby".to_string(),
                    });
                }
                ServerMessage::ItemPlaced(placed) => {
                    furni_writers.placed.write(NetItemPlacedEvent(placed));
                }
                ServerMessage::ItemMoved(moved) => {
                    furni_writers.moved.write(NetItemMovedEvent(moved));
                }
                ServerMessage::ItemRotated(rot) => {
                    furni_writers.rotated.write(NetItemRotatedEvent(rot));
                }
                ServerMessage::ItemRemoved(rem) => {
                    furni_writers.removed.write(NetItemRemovedEvent(rem));
                }
                ServerMessage::BuildModeRejected(rej) => {
                    furni_writers.rejected.write(NetBuildModeRejectedEvent(rej));
                }
                ServerMessage::AvatarAppearanceUpdated(upd) => {
                    emotes
                        .appearance
                        .write(NetAvatarAppearanceUpdatedEvent(upd));
                }
                ServerMessage::AvatarPostureChanged(change) => {
                    emotes.posture.write(NetAvatarPostureChangedEvent(change));
                }
                ServerMessage::AvatarExpressionChanged(change) => {
                    emotes
                        .expression
                        .write(NetAvatarExpressionChangedEvent(change));
                }
                ServerMessage::AvatarHoldingChanged(change) => {
                    emotes.holding.write(NetAvatarHoldingChangedEvent(change));
                }
                ServerMessage::AvatarAction(action) => {
                    emotes.action.write(NetAvatarActionEvent(action));
                }
                ServerMessage::PetPlaced(pet) => {
                    pets.placed.write(NetPetPlacedEvent(pet));
                }
                ServerMessage::PetPath(path) => {
                    pets.path.write(NetPetPathEvent(path));
                }
                ServerMessage::PetAction(action) => {
                    pets.action.write(NetPetActionEvent(action));
                }
                ServerMessage::PetRemoved(removed) => {
                    pets.removed.write(NetPetRemovedEvent(removed));
                }
                ServerMessage::PetInteractResult(result) => {
                    pets.interaction.write(NetPetInteractResultEvent(result));
                }
            },

            NakamaEvent::InventoryLoaded(items) => {
                furni_writers
                    .inventory
                    .write(NetInventoryLoadedEvent(items));
            }
            NakamaEvent::WalletUpdated {
                wallet,
                delta_silver,
                delta_gold,
                reason,
            } => {
                economy.wallet.write(NetWalletUpdatedEvent {
                    wallet,
                    delta_silver,
                    delta_gold,
                    reason,
                });
            }
            NakamaEvent::ShopCatalogLoaded(catalog) => {
                economy.catalog.write(NetShopCatalogLoadedEvent(catalog));
            }
            NakamaEvent::CatalogsLoaded(catalogs) => {
                economy
                    .catalogs
                    .write(crate::catalog_sync::NetCatalogsLoadedEvent(*catalogs));
            }
            NakamaEvent::PurchaseSucceeded(done) => {
                economy.purchased.write(NetPurchaseSucceededEvent(done));
            }
            NakamaEvent::PurchaseFailed {
                catalog_id,
                message,
            } => {
                economy.purchase_failed.write(NetPurchaseFailedEvent {
                    catalog_id,
                    message,
                });
            }
            NakamaEvent::RoomLayoutsLoaded(layouts) => {
                info!("Loaded {} room layouts from catalog", layouts.len());
                if nav_state.selected_layout_id.is_none()
                    && let Some(first) = layouts.first()
                {
                    nav_state.selected_layout_id = Some(first.id.clone());
                }
                nav_state.layouts = layouts;
            }
            NakamaEvent::PopularRoomsLoaded(rooms) => {
                info!("Loaded {} popular rooms", rooms.len());
                nav_state.is_loading_popular = false;
                nav_state.popular_rooms = rooms;
            }
            NakamaEvent::MyRoomsLoaded(rooms) => {
                info!("Loaded {} user-owned rooms", rooms.len());
                // Check if current room belongs to player
                for r in &rooms {
                    if r.room_id == active_meta.room_id {
                        active_meta.is_owner = true;
                        active_meta.owner_id = r
                            .owner_id
                            .clone()
                            .or_else(|| Some(net.local_user_id.clone()));
                        active_meta.visibility = r.visibility.clone();
                    }
                }
                nav_state.my_rooms = rooms;
            }
            NakamaEvent::SearchResultsLoaded(rooms) => {
                info!("Loaded {} search results", rooms.len());
                nav_state.is_searching = false;
                nav_state.search_results = rooms;
            }
            NakamaEvent::RoomCreated(created) => {
                info!(
                    "Room created successfully: {} ({})",
                    created.name, created.room_id
                );
                active_meta.is_owner = true;
                active_meta.owner_id = Some(net.local_user_id.clone());
                // Known as mine from now on, so the snapshot of the new room resolves to "mine" even
                // before `MyRoomsLoaded` arrives.
                nav_state.my_rooms.push(protocol::RoomSummaryDto {
                    room_id: created.room_id.clone(),
                    name: created.name.clone(),
                    owner_id: Some(net.local_user_id.clone()),
                    owner_name: None,
                    visibility: created.visibility.clone(),
                    user_count: 0,
                    max_users: created.max_users,
                    match_id: None,
                });
                active_meta.room_id = created.room_id.clone();
                active_meta.room_name = created.name.clone();
                active_meta.visibility = created.visibility.clone();
                // Join new room automatically
                net.handle.send_command(NakamaCommand::RequestJoinRoom {
                    room_id: created.room_id,
                });
            }
            NakamaEvent::RoomUpdated => {
                info!("Room updated successfully on server");
                net.handle.send_command(NakamaCommand::FetchMyRooms);
            }
            NakamaEvent::RoomDeleted => {
                info!("Room deleted successfully, redirecting to lobby");
                active_meta.is_owner = false;
                active_meta.owner_id = None;
                active_meta.visibility = "public".to_string();
                net.handle.send_command(NakamaCommand::RequestJoinRoom {
                    room_id: "lobby".to_string(),
                });
            }
            NakamaEvent::JoinRoomResult(join_res) => match join_res.status.as_str() {
                "allowed" => {
                    if let Some(match_id) = join_res.match_id {
                        info!("Join allowed, switching match to: {}", match_id);
                        doorbell_state.is_waiting_approval = false;
                        net.handle
                            .send_command(NakamaCommand::SwitchMatch { match_id });
                    }
                }
                "doorbell_pending" => {
                    info!("Doorbell pending for room approval");
                    doorbell_state.is_waiting_approval = true;
                    doorbell_state.waiting_message = Some(
                        "Campainha tocada! Aguardando o proprietário abrir a porta...".to_string(),
                    );
                }
                "denied" => {
                    let reason = join_res
                        .reason
                        .unwrap_or_else(|| "Acesso negado".to_string());
                    warn!("Join room denied: {}", reason);
                    doorbell_state.is_waiting_approval = false;
                    doorbell_state.waiting_message =
                        Some(format!("Entrada não autorizada: {}", reason));
                }
                _ => {}
            },
            NakamaEvent::Disconnected { reason } => {
                warn!("Nakama: Disconnected: {}", reason);
                connection.reconnecting = false;
                connection.fail(reason);
                next_state.set(AppState::ConnectionError);
            }
            NakamaEvent::Error { message } => {
                error!("Nakama error: {}", message);
                // Do not disconnect whole app on minor rpc errors
                if !message.contains("RPC error") {
                    connection.reconnecting = false;
                    connection.fail(message);
                    next_state.set(AppState::ConnectionError);
                }
            }
        }
    }
}

pub fn send_move_request(net: &NakamaResource, x: i32, z: i32) {
    net.handle
        .send_command(NakamaCommand::SendMessage(ClientMessage::Move(
            MoveRequest { x, z },
        )));
}

#[cfg(test)]
mod lifecycle_tests {
    use super::resumed_from_background;
    use bevy::window::AppLifecycle;

    #[test]
    fn resuming_after_a_suspension_triggers_one_reconnection() {
        let mut suspended = false;
        assert!(!resumed_from_background(
            &mut suspended,
            AppLifecycle::WillSuspend
        ));
        assert!(!resumed_from_background(
            &mut suspended,
            AppLifecycle::Suspended
        ));
        assert!(resumed_from_background(
            &mut suspended,
            AppLifecycle::WillResume
        ));
        assert!(!resumed_from_background(
            &mut suspended,
            AppLifecycle::Running
        ));
    }

    #[test]
    fn running_without_a_suspension_does_not_reconnect() {
        let mut suspended = false;
        assert!(!resumed_from_background(
            &mut suspended,
            AppLifecycle::Running
        ));
        assert!(!resumed_from_background(&mut suspended, AppLifecycle::Idle));
    }
}
