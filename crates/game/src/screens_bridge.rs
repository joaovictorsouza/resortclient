//! Mirrors the game's screens (shop, inventory, room navigator, character creator) to the Compose
//! shell, and takes their commands back. The Kotlin side is `ShopRepository`, `InventoryRepository`,
//! `NavigatorRepository` and `CreatorRepository`. The bevy_hui screens keep working; nothing here
//! draws anything.
//!
//! Game -> shell (`HostEvent::App`, a JSON snapshot sent when it changes and when the host resumes):
//! - `shop_state {"open","loading","categories","offers","selected_category","query","pending","status"}`
//! - `inventory_state {"open","loading","items","selected_catalog_id"}`
//! - `navigator_state {"open","tab","public","popular","mine","search","layouts","query","searching",
//!   "has_searched","loading_popular","status"}`
//! - `creator_state {"active","category","appearance","categories"}`; every creator item carries a
//!   `thumbnail` asset path (`ui/thumbs/characters/<id>.png`, resolved against the CDN by the shell).
//!
//! Shell -> game (`HostMessage`, JSON payload):
//! - `shop_open {"open"}`, `shop_category {"id"}`, `shop_search {"query"}`, `shop_purchase {"catalog_id"}`
//! - `inventory_open {"open"}`, `inventory_select {"catalog_id"}`
//! - `navigator_open {"open"}`, `navigator_tab {"tab"}`, `navigator_query {"query"}`, `navigator_search`,
//!   `navigator_join {"room_id"}`, `navigator_create {"name","layout_id"}`
//! - `build_mode_set {"active"}`: turns the room build mode on or off (the state comes back in `game_state.build_mode`)
//! - `creator_category {"category"}`, `creator_pick {"id"}`, `creator_body {"body_type"}`, `creator_save`,
//!   `creator_cancel`
//!
//! The state stays server-authoritative: a command only does what the matching bevy_hui button does.

use crate::character_creator::{
    CatalogItem, CharacterCatalog, CharacterCreatorState, CreatorCategory, apply_body_choice,
    apply_item_choice, creator_thumbnail_path, save_appearance,
};
use crate::config::GameConfig;
use crate::furniture::{BuildModeState, BuildTool, PlayerInventoryState};
use crate::navigator::{
    ActiveInputFocus, ActiveRoomMetadata, FocusedInput, NavigatorTab, RoomNavigatorState,
};
use crate::navigator_ui::{join_room, select_tab, start_search};
use crate::net::NakamaResource;
use crate::shop::{ShopState, ShopStatus, can_afford};
use crate::state::AppState;
use crate::wallet::WalletState;
use bevy::prelude::*;
use bevy_kmp_ffi::bevy_kmp::{HostLifecycle, HostMessage, Outbox};
use nakama_client::NakamaCommand;
use protocol::{
    AvatarAppearance, InventoryItemDto, RoomLayoutDto, RoomSummaryDto, ShopCategoryDto,
    ShopOfferDto,
};
use serde::{Deserialize, Serialize};

pub const EVT_SHOP: &str = "shop_state";
pub const EVT_INVENTORY: &str = "inventory_state";
pub const EVT_NAVIGATOR: &str = "navigator_state";
pub const EVT_CREATOR: &str = "creator_state";

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusLine {
    pub kind: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShopSnapshot {
    pub open: bool,
    pub loading: bool,
    pub categories: Vec<ShopCategoryDto>,
    pub offers: Vec<ShopOfferDto>,
    pub selected_category: Option<String>,
    pub query: String,
    pub pending: Option<String>,
    pub status: Option<StatusLine>,
}

impl From<&ShopState> for ShopSnapshot {
    fn from(s: &ShopState) -> Self {
        Self {
            open: s.is_open,
            loading: s.is_loading,
            categories: s.categories.clone(),
            offers: s.offers.clone(),
            selected_category: s.selected_category.clone(),
            query: s.query.clone(),
            pending: s.pending.clone(),
            status: s.status.as_ref().map(|st| StatusLine {
                kind: match st {
                    ShopStatus::Success(_) => "success",
                    ShopStatus::Error(_) => "error",
                },
                text: st.text().to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InventorySnapshot {
    pub open: bool,
    pub loading: bool,
    pub items: Vec<InventoryItemDto>,
    pub selected_catalog_id: Option<String>,
}

impl From<&PlayerInventoryState> for InventorySnapshot {
    fn from(s: &PlayerInventoryState) -> Self {
        Self {
            open: s.is_open,
            loading: s.is_loading,
            items: s.items.clone(),
            selected_catalog_id: s.selected_catalog_id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NavigatorSnapshot {
    pub open: bool,
    pub tab: &'static str,
    pub popular: Vec<RoomSummaryDto>,
    pub mine: Vec<RoomSummaryDto>,
    pub search: Vec<RoomSummaryDto>,
    pub layouts: Vec<RoomLayoutDto>,
    pub query: String,
    pub searching: bool,
    pub has_searched: bool,
    pub loading_popular: bool,
    pub status: Option<String>,
}

impl From<&RoomNavigatorState> for NavigatorSnapshot {
    fn from(s: &RoomNavigatorState) -> Self {
        Self {
            open: s.is_open,
            tab: s.current_tab.key(),
            popular: s.popular_rooms.clone(),
            mine: s.my_rooms.clone(),
            search: s.search_results.clone(),
            layouts: s.layouts.clone(),
            query: s.search_query.clone(),
            searching: s.is_searching,
            has_searched: s.has_searched,
            loading_popular: s.is_loading_popular,
            status: s.status_message.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreatorItem {
    pub id: String,
    pub name: String,
    pub thumbnail: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreatorSection {
    pub key: &'static str,
    pub items: Vec<CreatorItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreatorSnapshot {
    pub active: bool,
    pub category: &'static str,
    pub appearance: AvatarAppearance,
    pub categories: Vec<CreatorSection>,
}

fn selected_id(category: CreatorCategory, a: &AvatarAppearance) -> Vec<&str> {
    match category {
        CreatorCategory::Face => vec![a.face_id.as_str()],
        CreatorCategory::Hair => vec![a.hair_id.as_str()],
        CreatorCategory::Top => vec![a.top_id.as_str()],
        CreatorCategory::Bottom => vec![a.bottom_id.as_str()],
        CreatorCategory::Outfit => vec![a.outfit_id.as_str()],
        CreatorCategory::Shoes => vec![a.shoes_id.as_str()],
        CreatorCategory::Accessories => vec![a.hat_id.as_str(), a.glasses_id.as_str()],
        CreatorCategory::Body | CreatorCategory::Skin => vec![],
    }
}

/// Catalog categories that make up each creator tab (accessories hold hats and glasses).
fn catalog_categories(category: CreatorCategory) -> &'static [&'static str] {
    match category {
        CreatorCategory::Face => &["face"],
        CreatorCategory::Hair => &["hair"],
        CreatorCategory::Top => &["top"],
        CreatorCategory::Bottom => &["bottom"],
        CreatorCategory::Outfit => &["outfit"],
        CreatorCategory::Shoes => &["shoes"],
        CreatorCategory::Accessories => &["hat", "glasses"],
        CreatorCategory::Body | CreatorCategory::Skin => &[],
    }
}

pub fn creator_snapshot(
    active: bool,
    state: &CharacterCreatorState,
    catalog: &CharacterCatalog,
) -> CreatorSnapshot {
    let body = state.appearance.body_type.as_str();
    let categories = CreatorCategory::ALL
        .iter()
        .map(|&cat| {
            let picked = selected_id(cat, &state.appearance);
            let items = catalog_categories(cat)
                .iter()
                .flat_map(|c| crate::appearance::selectable(&catalog.items, c, body))
                .map(|i: &CatalogItem| CreatorItem {
                    id: i.id.clone(),
                    name: i.name.clone(),
                    thumbnail: creator_thumbnail_path(&i.id),
                    selected: picked.contains(&i.id.as_str()),
                })
                .collect();
            CreatorSection {
                key: cat.key(),
                items,
            }
        })
        .collect();
    CreatorSnapshot {
        active,
        category: state.active_category.key(),
        appearance: state.appearance.clone(),
        categories,
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OpenCmd {
    open: bool,
}
#[derive(Deserialize)]
struct ActiveCmd {
    active: bool,
}
#[derive(Deserialize)]
struct IdCmd {
    id: String,
}
#[derive(Deserialize)]
struct CatalogIdCmd {
    catalog_id: String,
}
#[derive(Deserialize)]
struct QueryCmd {
    query: String,
}
#[derive(Deserialize)]
struct TabCmd {
    tab: String,
}
#[derive(Deserialize)]
struct RoomIdCmd {
    room_id: String,
}
#[derive(Deserialize)]
struct CreateRoomCmd {
    name: String,
    layout_id: Option<String>,
}
#[derive(Deserialize)]
struct CategoryCmd {
    category: String,
}
#[derive(Deserialize)]
struct BodyCmd {
    body_type: String,
}

fn parse<'a, T: Deserialize<'a>>(msg: &'a HostMessage) -> Option<T> {
    serde_json::from_slice(&msg.payload).ok()
}

fn creator_category_from_key(key: &str) -> Option<CreatorCategory> {
    CreatorCategory::ALL.into_iter().find(|c| c.key() == key)
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ScreensBridgePlugin;

impl Plugin for ScreensBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                publish_shop,
                publish_inventory,
                publish_navigator,
                publish_creator,
                shop_commands,
                inventory_commands,
                build_mode_commands,
                navigator_commands,
                creator_commands,
            ),
        );
    }
}

fn publish<T: Serialize + PartialEq + Clone>(
    outbox: &Outbox,
    event: &str,
    value: T,
    resumed: bool,
    last: &mut Option<T>,
) {
    if resumed || last.as_ref() != Some(&value) {
        if let Ok(payload) = serde_json::to_vec(&value) {
            outbox.send_app(event, payload);
        }
        *last = Some(value);
    }
}

fn resumed(lifecycle: &mut MessageReader<HostLifecycle>) -> bool {
    lifecycle.read().any(|m| *m == HostLifecycle::Resumed)
}

fn publish_shop(
    shop: Res<ShopState>,
    mut lifecycle: MessageReader<HostLifecycle>,
    outbox: Res<Outbox>,
    mut last: Local<Option<ShopSnapshot>>,
) {
    publish(
        &outbox,
        EVT_SHOP,
        ShopSnapshot::from(&*shop),
        resumed(&mut lifecycle),
        &mut last,
    );
}

fn publish_inventory(
    inventory: Res<PlayerInventoryState>,
    mut lifecycle: MessageReader<HostLifecycle>,
    outbox: Res<Outbox>,
    mut last: Local<Option<InventorySnapshot>>,
) {
    publish(
        &outbox,
        EVT_INVENTORY,
        InventorySnapshot::from(&*inventory),
        resumed(&mut lifecycle),
        &mut last,
    );
}

fn publish_navigator(
    nav: Res<RoomNavigatorState>,
    mut lifecycle: MessageReader<HostLifecycle>,
    outbox: Res<Outbox>,
    mut last: Local<Option<NavigatorSnapshot>>,
) {
    publish(
        &outbox,
        EVT_NAVIGATOR,
        NavigatorSnapshot::from(&*nav),
        resumed(&mut lifecycle),
        &mut last,
    );
}

fn publish_creator(
    app_state: Res<State<AppState>>,
    creator: Res<CharacterCreatorState>,
    catalog: Res<CharacterCatalog>,
    mut lifecycle: MessageReader<HostLifecycle>,
    outbox: Res<Outbox>,
    mut last: Local<Option<CreatorSnapshot>>,
) {
    let active = *app_state.get() == AppState::CharacterCreator;
    publish(
        &outbox,
        EVT_CREATOR,
        creator_snapshot(active, &creator, &catalog),
        resumed(&mut lifecycle),
        &mut last,
    );
}

fn shop_commands(
    mut messages: MessageReader<HostMessage>,
    mut shop: ResMut<ShopState>,
    wallet: Res<WalletState>,
    net: Option<Res<NakamaResource>>,
) {
    for msg in messages.read() {
        match msg.name.as_str() {
            "shop_open" => {
                if let Some(c) = parse::<OpenCmd>(msg) {
                    shop.is_open = c.open;
                }
            }
            "shop_category" => {
                if let Some(c) = parse::<IdCmd>(msg) {
                    shop.selected_category = Some(c.id);
                }
            }
            "shop_search" => {
                if let Some(c) = parse::<QueryCmd>(msg) {
                    shop.query = c.query;
                }
            }
            "shop_purchase" => {
                let Some(c) = parse::<CatalogIdCmd>(msg) else {
                    continue;
                };
                if shop.pending.is_some() {
                    continue;
                }
                let Some(offer) = shop.offers.iter().find(|o| o.catalog_id == c.catalog_id) else {
                    continue;
                };
                if !can_afford(&wallet.balance, offer) {
                    shop.status = Some(ShopStatus::Error("Saldo insuficiente.".to_string()));
                    continue;
                }
                if let Some(net) = &net {
                    net.handle.send_command(NakamaCommand::PurchaseItem {
                        catalog_id: c.catalog_id.clone(),
                        quantity: 1,
                    });
                    shop.pending = Some(c.catalog_id);
                    shop.status = None;
                }
            }
            _ => {}
        }
    }
}

fn inventory_commands(
    mut messages: MessageReader<HostMessage>,
    mut inventory: ResMut<PlayerInventoryState>,
    net: Option<Res<NakamaResource>>,
) {
    for msg in messages.read() {
        match msg.name.as_str() {
            "inventory_open" => {
                if let Some(c) = parse::<OpenCmd>(msg) {
                    inventory.is_open = c.open;
                    if c.open
                        && let Some(net) = &net
                    {
                        inventory.is_loading = true;
                        net.handle.send_command(NakamaCommand::FetchInventory);
                    }
                }
            }
            "inventory_select" => {
                if let Some(c) = parse::<CatalogIdCmd>(msg) {
                    inventory.selected_catalog_id = Some(c.catalog_id);
                }
            }
            _ => {}
        }
    }
}

/// Same effect as the bevy_hui build button: toggling off drops the tool and the selection.
fn build_mode_commands(
    mut messages: MessageReader<HostMessage>,
    mut build: ResMut<BuildModeState>,
) {
    for msg in messages.read() {
        if msg.name == "build_mode_set"
            && let Some(c) = parse::<ActiveCmd>(msg)
        {
            build.is_active = c.active;
            if !c.active {
                build.active_tool = BuildTool::None;
                build.selected_furniture_id = None;
            }
        }
    }
}

fn navigator_commands(
    mut messages: MessageReader<HostMessage>,
    mut nav: ResMut<RoomNavigatorState>,
    mut meta: ResMut<ActiveRoomMetadata>,
    mut focus: ResMut<ActiveInputFocus>,
    net: Option<Res<NakamaResource>>,
) {
    for msg in messages.read() {
        match msg.name.as_str() {
            "navigator_open" => {
                if let Some(c) = parse::<OpenCmd>(msg) {
                    nav.is_open = c.open;
                    if !c.open {
                        focus.0 = FocusedInput::None;
                    }
                }
            }
            "navigator_tab" => {
                if let Some(tab) = parse::<TabCmd>(msg).and_then(|c| NavigatorTab::from_key(&c.tab))
                {
                    select_tab(tab, &mut nav, net.as_deref());
                }
            }
            "navigator_query" => {
                if let Some(c) = parse::<QueryCmd>(msg) {
                    nav.search_query = c.query;
                }
            }
            "navigator_search" => start_search(&mut nav, net.as_deref()),
            "navigator_join" => {
                if let Some(c) = parse::<RoomIdCmd>(msg) {
                    join_room(&c.room_id, &mut nav, &mut meta, &mut focus, net.as_deref());
                }
            }
            "navigator_create" => {
                if let Some(c) = parse::<CreateRoomCmd>(msg) {
                    nav.create_name = crate::navigator_ui::clamp_name(&c.name);
                    nav.selected_layout_id = c.layout_id;
                    crate::navigator::submit_create_room(&mut nav, &mut focus, net.as_deref());
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn creator_commands(
    mut messages: MessageReader<HostMessage>,
    app_state: Res<State<AppState>>,
    mut creator: ResMut<CharacterCreatorState>,
    catalog: Res<CharacterCatalog>,
    mut next_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    if *app_state.get() != AppState::CharacterCreator {
        messages.clear();
        return;
    }
    let mut save = false;
    for msg in messages.read() {
        match msg.name.as_str() {
            "creator_category" => {
                if let Some(cat) =
                    parse::<CategoryCmd>(msg).and_then(|c| creator_category_from_key(&c.category))
                {
                    creator.active_category = cat;
                }
            }
            "creator_pick" => {
                if let Some(c) = parse::<IdCmd>(msg) {
                    let cat = creator.active_category;
                    apply_item_choice(&mut creator.appearance, cat, &c.id, &catalog.items);
                }
            }
            "creator_body" => {
                if let Some(c) = parse::<BodyCmd>(msg) {
                    apply_body_choice(&mut creator.appearance, &catalog.items, &c.body_type);
                }
            }
            "creator_save" => save = true,
            "creator_cancel" => {
                creator.return_to_room = false;
                next_state.set(AppState::InRoom);
            }
            _ => {}
        }
    }
    if save {
        commands.run_system_cached_with(save_current, ());
    }
}

fn save_current(
    _: In<()>,
    state: ResMut<CharacterCreatorState>,
    config: Res<GameConfig>,
    net: Option<Res<NakamaResource>>,
    next_state: ResMut<NextState<AppState>>,
) {
    save_appearance(state, config, net, next_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creator_thumbnails_live_under_the_characters_folder() {
        assert_eq!(
            creator_thumbnail_path("Bot.01"),
            "ui/thumbs/characters/Bot.01.png"
        );
    }

    #[test]
    fn navigator_snapshot_uses_the_tab_key() {
        let snap = NavigatorSnapshot::from(&RoomNavigatorState::default());
        assert_eq!(snap.tab, "public");
        assert!(!snap.open);
    }

    #[test]
    fn the_creator_snapshot_marks_the_worn_item_and_lists_every_tab() {
        let catalog = CharacterCatalog::fixture();
        let state = CharacterCreatorState::default();
        let snap = creator_snapshot(true, &state, &catalog);
        assert_eq!(snap.categories.len(), CreatorCategory::ALL.len());
        let face = snap.categories.iter().find(|c| c.key == "face").unwrap();
        assert!(
            face.items
                .iter()
                .any(|i| i.selected && i.id == state.appearance.face_id)
        );
        assert!(face.items.iter().all(|i| i.thumbnail.ends_with(".png")));
    }

    #[test]
    fn a_shop_status_serialises_with_its_kind() {
        let shop = ShopState {
            status: Some(ShopStatus::Error("x".into())),
            ..Default::default()
        };
        let json = serde_json::to_string(&ShopSnapshot::from(&shop).status).unwrap();
        assert_eq!(json, r#"{"kind":"error","text":"x"}"#);
    }
}
