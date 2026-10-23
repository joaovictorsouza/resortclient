use protocol::{
    ClientMessage, CreateRoomResponse, JoinRoomResponse, RoomLayoutDto, RoomSummaryDto,
    ServerMessage,
};

#[derive(Debug, Clone)]
pub enum NakamaCommand {
    ConnectAndJoinPublicRoom {
        host: String,
        port: u16,
        server_key: String,
        device_id: String,
        username: String,
        room_id: String,
        /// Google `id_token` from the native sign-in; when set it replaces the device login.
        google_id_token: Option<String>,
        /// Saved session of a previous Google login; used when there is no fresh `id_token`.
        refresh_token: Option<String>,
    },
    SendMessage(ClientMessage),
    Disconnect,
    /// The app came back from the background: drop the (probably dead) socket and log in again.
    ForceReconnect,
    FetchRoomLayouts,
    CreateRoom {
        name: String,
        layout_id: String,
    },
    FetchPopularRooms,
    FetchMyRooms,
    SearchRooms {
        query: String,
    },
    UpdateRoom {
        room_id: String,
        name: String,
        visibility: String,
    },
    DeleteRoom {
        room_id: String,
    },
    RequestJoinRoom {
        room_id: String,
    },
    SwitchMatch {
        match_id: String,
    },
    FetchInventory,
    FetchWallet,
    WalletHeartbeat,
    FetchShopCatalog,
    /// Downloads the catalogs; `if_version` is the cached `catalog_version` (empty = none).
    FetchCatalogs {
        if_version: String,
    },
    PurchaseItem {
        catalog_id: String,
        quantity: i32,
    },
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum NakamaEvent {
    Connecting,
    /// The connection dropped and the worker is trying to restore it.
    Reconnecting,
    Connected {
        user_id: String,
        username: String,
        match_id: String,
    },
    MessageReceived(Box<ServerMessage>),
    Disconnected {
        reason: String,
    },
    Error {
        message: String,
    },
    RoomLayoutsLoaded(Vec<RoomLayoutDto>),
    RoomCreated(CreateRoomResponse),
    PopularRoomsLoaded(Vec<RoomSummaryDto>),
    MyRoomsLoaded(Vec<RoomSummaryDto>),
    SearchResultsLoaded(Vec<RoomSummaryDto>),
    RoomUpdated,
    RoomDeleted,
    JoinRoomResult(JoinRoomResponse),
    InventoryLoaded(Vec<protocol::InventoryItemDto>),
    /// Fresh balances. `delta_*` is only known for pushed notifications; `reason` says why
    /// (`"daily_login"`, `"play_time"`, `"iap_emulation"`, ...).
    WalletUpdated {
        wallet: protocol::WalletDto,
        delta_silver: i64,
        delta_gold: i64,
        reason: String,
    },
    ShopCatalogLoaded(protocol::GetShopCatalogResponse),
    CatalogsLoaded(Box<protocol::GetCatalogsResponse>),
    PurchaseSucceeded(protocol::PurchaseItemResponse),
    PurchaseFailed {
        catalog_id: String,
        message: String,
    },
}
