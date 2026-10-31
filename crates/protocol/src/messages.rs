use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRequest {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub door: [i32; 2],
    pub heightmap: Vec<String>,
}

/// The two bodies of the character pack.
pub const BODY_FEMALE: &str = "female";
pub const BODY_MALE: &str = "male";

/// Colour groups of an appearance (see the contract, section 17).
pub const COLOR_GROUPS: &[&str] = &[
    "skin",
    "eyes",
    "underwear",
    "hair",
    "hat",
    "glasses",
    "top",
    "bottom",
    "outfit",
    "shoes",
];

/// Face sliders (blend shapes) a player can set, each 0..255.
pub const FACE_MORPHS: &[&str] = &["eye_size", "nose_length", "tuck_ears", "shorten"];

/// Expressions the face can show (section 18 of the contract).
pub const EXPRESSIONS: &[&str] = &["neutral", "happy", "laugh", "sad", "angry", "surprised"];

/// The only avatar action for now: bring the held prop to the mouth.
pub const ACTION_DRINK: &str = "drink";

/// Modular cosmetic configuration of an avatar (version 2 of the contract).
///
/// Item ids are the names of the cosmetics catalog; an empty id means "nothing" for the optional
/// slots. `colors` maps a colour group to up to six `#RRGGBB` values for the colour slots
/// A1, A2, B1, B2, C1, C2 of its materials (an empty string keeps the material's own colour).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarAppearance {
    #[serde(default = "default_body_type")]
    pub body_type: String,
    #[serde(default = "default_face_id")]
    pub face_id: String,
    #[serde(default = "default_hair_id")]
    pub hair_id: String,
    #[serde(default)]
    pub hat_id: String,
    #[serde(default)]
    pub glasses_id: String,
    #[serde(default = "default_top_id")]
    pub top_id: String,
    #[serde(default = "default_bottom_id")]
    pub bottom_id: String,
    #[serde(default)]
    pub outfit_id: String,
    #[serde(default = "default_shoes_id")]
    pub shoes_id: String,
    #[serde(default)]
    pub colors: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub face_morphs: BTreeMap<String, i32>,
}

fn default_body_type() -> String {
    BODY_FEMALE.to_string()
}
fn default_face_id() -> String {
    "F_head.001".to_string()
}
fn default_hair_id() -> String {
    "Hairstyle.06".to_string()
}
fn default_top_id() -> String {
    "Top.01".to_string()
}
fn default_bottom_id() -> String {
    "Bot.01".to_string()
}
fn default_shoes_id() -> String {
    "Shoes.01".to_string()
}

impl Default for AvatarAppearance {
    fn default() -> Self {
        Self {
            body_type: default_body_type(),
            face_id: default_face_id(),
            hair_id: default_hair_id(),
            hat_id: String::new(),
            glasses_id: String::new(),
            top_id: default_top_id(),
            bottom_id: default_bottom_id(),
            outfit_id: String::new(),
            shoes_id: default_shoes_id(),
            colors: BTreeMap::new(),
            face_morphs: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvatarSnapshot {
    pub user_id: String,
    pub display_name: String,
    pub x: i32,
    pub z: i32,
    #[serde(default)]
    pub appearance: Option<AvatarAppearance>,
    /// One of [`EXPRESSIONS`]; empty is neutral.
    #[serde(default)]
    pub expression: String,
    /// Id of the prop in the right hand; empty is nothing.
    #[serde(default)]
    pub holding: String,
    #[serde(default)]
    pub posture: Posture,
    /// Facing, 0..7 in 45° steps (0 = +Z, 2 = +X, 4 = -Z, 6 = -X).
    #[serde(default)]
    pub rotation: i32,
    /// Resting surface height above the floor (seat/bed), 0 when standing.
    #[serde(default)]
    pub y: f64,
    /// Point (tile units) the avatar rests on: tile center, or the whole bed's center.
    #[serde(default)]
    pub rest_x: f64,
    #[serde(default)]
    pub rest_z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedItemSnapshot {
    pub id: String,
    pub catalog_id: String,
    pub owner_id: String,
    pub x: i32,
    pub z: i32,
    pub y: f64,
    pub rotation: i32,
    pub width: i32,
    pub depth: i32,
    pub walkable: bool,
    pub color: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomSnapshot {
    pub room_id: String,
    pub room_name: String,
    /// The owner's user id, or `""` for rooms without an owner (lobby, cafe). Required since protocol 2.0.
    pub owner_id: String,
    pub layout: LayoutSnapshot,
    pub avatars: Vec<AvatarSnapshot>,
    #[serde(default)]
    pub items: Vec<PlacedItemSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvatarJoined {
    pub user_id: String,
    pub display_name: String,
    pub x: i32,
    pub z: i32,
    #[serde(default)]
    pub appearance: Option<AvatarAppearance>,
    /// One of [`EXPRESSIONS`]; empty is neutral.
    #[serde(default)]
    pub expression: String,
    /// Id of the prop in the right hand; empty is nothing.
    #[serde(default)]
    pub holding: String,
    #[serde(default)]
    pub posture: Posture,
    /// Facing, 0..7 in 45° steps (0 = +Z, 2 = +X, 4 = -Z, 6 = -X).
    #[serde(default)]
    pub rotation: i32,
    /// Resting surface height above the floor (seat/bed), 0 when standing.
    #[serde(default)]
    pub y: f64,
    /// Point (tile units) the avatar rests on: tile center, or the whole bed's center.
    #[serde(default)]
    pub rest_x: f64,
    #[serde(default)]
    pub rest_z: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateAppearanceRequest {
    pub appearance: AvatarAppearance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarAppearanceUpdated {
    pub user_id: String,
    pub appearance: AvatarAppearance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetExpressionRequest {
    pub expression: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarExpressionChanged {
    pub user_id: String,
    pub expression: String,
}

/// Holds a prop (a catalog item of category `prop`) or, with an empty id, puts it down.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetHoldingRequest {
    pub prop_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarHoldingChanged {
    pub user_id: String,
    pub prop_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarActionRequest {
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarAction {
    pub user_id: String,
    pub action: String,
}

/// Server-authoritative resting posture of an avatar (spec 0004 do servidor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Posture {
    #[default]
    Stand,
    Sit,
    Lay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvatarPostureChanged {
    pub user_id: String,
    pub posture: Posture,
    pub rotation: i32,
    pub y: f64,
    pub rest_x: f64,
    pub rest_z: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvatarLeft {
    pub user_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvatarPath {
    pub user_id: String,
    pub path: Vec<[i32; 2]>,
    pub tiles_per_sec: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRejected {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorbellRequest {
    pub request_id: String,
    pub requester_id: String,
    pub requester_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorbellResponse {
    pub request_id: String,
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomClosed {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceItemRequest {
    pub catalog_id: String,
    pub x: i32,
    pub z: i32,
    pub rotation: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveItemRequest {
    pub item_id: String,
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotateItemRequest {
    pub item_id: String,
    pub rotation: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickupItemRequest {
    pub item_id: String,
}

pub type ItemPlaced = PlacedItemSnapshot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemMoved {
    pub id: String,
    pub x: i32,
    pub z: i32,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRotated {
    pub id: String,
    pub rotation: i32,
    pub width: i32,
    pub depth: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRemoved {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildModeRejected {
    pub reason: String,
}

/// Enumeration of all messages sent by the client to the server
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ClientMessage {
    Move(MoveRequest),
    DoorbellResponse(DoorbellResponse),
    PlaceItem(PlaceItemRequest),
    MoveItem(MoveItemRequest),
    RotateItem(RotateItemRequest),
    PickupItem(PickupItemRequest),
    UpdateAppearance(Box<UpdateAppearanceRequest>),
    SetExpression(SetExpressionRequest),
    SetHolding(SetHoldingRequest),
    AvatarAction(AvatarActionRequest),
}

/// Enumeration of all messages received by the client from the server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    RoomSnapshot(RoomSnapshot),
    AvatarJoined(AvatarJoined),
    AvatarLeft(AvatarLeft),
    AvatarPath(AvatarPath),
    MoveRejected(MoveRejected),
    DoorbellRequest(DoorbellRequest),
    RoomClosed(RoomClosed),
    ItemPlaced(ItemPlaced),
    ItemMoved(ItemMoved),
    ItemRotated(ItemRotated),
    ItemRemoved(ItemRemoved),
    BuildModeRejected(BuildModeRejected),
    AvatarAppearanceUpdated(AvatarAppearanceUpdated),
    AvatarPostureChanged(AvatarPostureChanged),
    AvatarExpressionChanged(AvatarExpressionChanged),
    AvatarHoldingChanged(AvatarHoldingChanged),
    AvatarAction(AvatarAction),
}

// ---------------------------------------------------------------------------
// RPC DTOs (Avatar Appearance)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAvatarAppearanceRequest {
    pub appearance: AvatarAppearance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveAvatarAppearanceResponse {
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetAvatarAppearanceRequest {
    #[serde(default)]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetAvatarAppearanceResponse {
    pub user_id: String,
    pub appearance: AvatarAppearance,
}

// ---------------------------------------------------------------------------
// RPC DTOs (Room Management)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomLayoutDto {
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub door: [i32; 2],
    pub max_users: i32,
    pub image_asset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRoomLayoutsResponse {
    pub layouts: Vec<RoomLayoutDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub layout_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
    pub name: String,
    pub visibility: String,
    pub max_users: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomSummaryDto {
    pub room_id: String,
    pub name: String,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
    pub visibility: String,
    pub user_count: i32,
    pub max_users: i32,
    pub match_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRoomsResponse {
    pub rooms: Vec<RoomSummaryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateRoomRequest {
    pub room_id: String,
    pub name: String,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRoomRequest {
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinRoomResponse {
    pub status: String,
    pub match_id: Option<String>,
    pub reason: Option<String>,
    pub request_id: Option<String>,
}

// ---------------------------------------------------------------------------
// RPC DTOs (Inventory)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryItemDto {
    pub id: String,
    pub catalog_id: String,
    pub name: String,
    pub category: String,
    pub quantity: i32,
    pub width: i32,
    pub depth: i32,
    pub walkable: bool,
    pub color: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetUserInventoryResponse {
    pub items: Vec<InventoryItemDto>,
}

// ---------------------------------------------------------------------------
// Currency & shop RPCs (spec 0005 / client spec 0008)
// ---------------------------------------------------------------------------

/// Notification code the server uses for `wallet_changed` (Nakama notification).
pub const NOTIFICATION_WALLET_CHANGED: i64 = 1001;

/// Player balances; `get_wallet` response.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletDto {
    pub silver: i64,
    pub gold: i64,
}

impl WalletDto {
    /// Whether the balance can pay the given price (0 = not charged in that currency).
    pub fn covers(&self, price_silver: i64, price_gold: i64) -> bool {
        self.silver >= price_silver && self.gold >= price_gold
    }
}

/// `wallet_heartbeat` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub silver: i64,
    pub gold: i64,
    #[serde(default)]
    pub granted_silver: i64,
    #[serde(default)]
    pub daily_bonus: bool,
}

/// Content of the `wallet_changed` notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletChangedNotification {
    pub silver: i64,
    pub gold: i64,
    #[serde(default)]
    pub delta_silver: i64,
    #[serde(default)]
    pub delta_gold: i64,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopCategoryDto {
    pub id: String,
    pub name: String,
    pub icon: String,
    /// Empty for top-level menu entries.
    #[serde(default)]
    pub parent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopOfferDto {
    pub catalog_id: String,
    pub category_id: String,
    pub name: String,
    pub width: i32,
    pub depth: i32,
    pub walkable: bool,
    pub price_silver: i64,
    pub price_gold: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetShopCatalogResponse {
    pub categories: Vec<ShopCategoryDto>,
    pub offers: Vec<ShopOfferDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseItemRequest {
    pub catalog_id: String,
    pub quantity: i32,
}

/// `purchase_item` response, carrying the new balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseItemResponse {
    pub catalog_id: String,
    pub quantity: i32,
    pub silver: i64,
    pub gold: i64,
}

/// Furniture entry of `get_catalogs` (same shape as `contract/furniture-catalog.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogFurnitureDto {
    pub catalog_id: String,
    pub name: String,
    pub category: String,
    pub width: i32,
    pub depth: i32,
    pub walkable: bool,
    pub color: String,
    pub asset_id: String,
    /// `draft` or `live`; absent means live.
    #[serde(default)]
    pub status: Option<String>,
}

/// Request body of `get_catalogs`: the `catalog_version` of the cached copy, if any.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetCatalogsRequest {
    pub if_version: String,
}

/// `get_catalogs` response. With `unchanged` set only the version and manifest URL are present.
/// Cosmetics stay as raw JSON because the game crate owns their schema.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GetCatalogsResponse {
    pub catalog_version: String,
    #[serde(default)]
    pub unchanged: bool,
    #[serde(default)]
    pub assets_manifest_url: String,
    #[serde(default)]
    pub furniture: Vec<CatalogFurnitureDto>,
    #[serde(default)]
    pub cosmetics: Option<serde_json::Value>,
    #[serde(default)]
    pub shop: Option<GetShopCatalogResponse>,
}
