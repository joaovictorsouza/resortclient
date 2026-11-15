//! In-game shop window built with `bevy_hui` templates (spec 0008).
//!
//! Two columns: a side menu (categories with optional sub-categories + search) and a grid of
//! offers. The server owns prices and balances; the client only renders the catalog and sends
//! `purchase_item` requests. Pure helpers (menu, filtering, price labels) are free of the ECS so
//! they can be unit tested.

#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_hui::prelude::*;
use bevy_hui_widgets::prelude::TextInput;
use nakama_client::NakamaCommand;
use protocol::{ShopCategoryDto, ShopOfferDto, WalletDto};

use crate::UiFont;
use crate::furniture::PlayerInventoryState;
use crate::inventory::{normalize_search, scrollbar_metrics, thumbnail_path};
use crate::net::{
    NakamaResource, NetPurchaseFailedEvent, NetPurchaseSucceededEvent, NetShopCatalogLoadedEvent,
};
use crate::safe_area::SafeArea;
use crate::state::AppState;
use crate::wallet::{WalletState, format_amount};

const SCROLL_LINE_PX: f32 = 32.0;

const MENU_BG: &str = "#1e293b"; // slate-800
const MENU_BG_SELECTED: &str = "#4f46e5"; // indigo-600
const BUY_BG: &str = "#059669"; // emerald-600
const BUY_BG_HOVER: &str = "#10b981"; // emerald-500
const BUY_BG_DISABLED: &str = "#475569"; // slate-600
const STATUS_OK: Color = Color::srgb(0.20, 0.83, 0.60); // emerald-400
const STATUS_ERR: Color = Color::srgb(0.98, 0.44, 0.52); // rose-400
const STATUS_NEUTRAL: Color = Color::srgb(0.80, 0.84, 0.89); // slate-300

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// One entry of the side menu, already flattened for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuRow {
    pub key: String,
    pub label: String,
    pub icon: String,
    /// 0 = top-level, 1 = sub-category.
    pub depth: u8,
    pub selected: bool,
    /// `Some(open)` for a section with sub-categories (accordion header), `None` otherwise.
    pub accordion: Option<bool>,
}

pub fn category_icon_path(icon: &str) -> String {
    format!("ui/icons/{icon}.png")
}

fn top_level(categories: &[ShopCategoryDto]) -> impl Iterator<Item = &ShopCategoryDto> {
    categories.iter().filter(|c| c.parent.is_empty())
}

fn children<'a>(
    categories: &'a [ShopCategoryDto],
    parent: &'a str,
) -> impl Iterator<Item = &'a ShopCategoryDto> {
    categories.iter().filter(move |c| c.parent == parent)
}

/// The category selected when the shop opens: the first top-level entry.
pub fn default_category(categories: &[ShopCategoryDto]) -> Option<String> {
    top_level(categories).next().map(|c| c.id.clone())
}

/// Keeps `selected` if it still exists in the catalog, otherwise falls back to the default.
pub fn resolve_category(categories: &[ShopCategoryDto], selected: Option<&str>) -> Option<String> {
    selected
        .filter(|id| categories.iter().any(|c| c.id == *id))
        .map(str::to_string)
        .or_else(|| default_category(categories))
}

/// Whether `id` is a top-level section that has sub-categories.
pub fn has_children(categories: &[ShopCategoryDto], id: &str) -> bool {
    children(categories, id).next().is_some()
}

/// Whether `id` is a top-level section that is selected itself or holds the selected child.
fn holds_selection(categories: &[ShopCategoryDto], id: &str, selected: Option<&str>) -> bool {
    selected == Some(id)
        || selected.is_some_and(|sel| children(categories, id).any(|c| c.id == sel))
}

/// Accordion behaviour of the side menu: pressing a section selects (and opens) it; pressing the
/// section that is already open folds it. Returns the new selection.
pub fn press_category(
    categories: &[ShopCategoryDto],
    key: &str,
    selected: Option<&str>,
    collapsed: &mut HashSet<String>,
) -> String {
    if has_children(categories, key) {
        if holds_selection(categories, key, selected) && selected == Some(key) {
            if !collapsed.remove(key) {
                collapsed.insert(key.to_string());
            }
        } else {
            collapsed.remove(key);
        }
    }
    key.to_string()
}

/// Flattens the category tree as an accordion. A top-level entry expands (shows its
/// sub-categories) while it or one of its children is selected and it was not folded by the
/// player; `selected` is highlighted only when `highlight` is true (a text search suspends the
/// highlight).
pub fn menu_rows(
    categories: &[ShopCategoryDto],
    selected: Option<&str>,
    highlight: bool,
    collapsed: &HashSet<String>,
) -> Vec<MenuRow> {
    let mut rows = Vec::new();
    for top in top_level(categories) {
        let expanded =
            holds_selection(categories, &top.id, selected) && !collapsed.contains(&top.id);
        rows.push(MenuRow {
            key: top.id.clone(),
            label: top.name.clone(),
            icon: category_icon_path(&top.icon),
            depth: 0,
            selected: highlight && selected == Some(top.id.as_str()),
            accordion: has_children(categories, &top.id).then_some(expanded),
        });
        if expanded {
            for child in children(categories, &top.id) {
                rows.push(MenuRow {
                    key: child.id.clone(),
                    label: child.name.clone(),
                    icon: category_icon_path(&child.icon),
                    depth: 1,
                    selected: highlight && selected == Some(child.id.as_str()),
                    accordion: None,
                });
            }
        }
    }
    rows
}

/// Offers to show. A non-blank search looks at every category (name, accent/case insensitive);
/// otherwise the selected category is shown, including all of its sub-categories.
pub fn filter_offers<'a>(
    categories: &[ShopCategoryDto],
    offers: &'a [ShopOfferDto],
    selected: Option<&str>,
    query: &str,
) -> Vec<&'a ShopOfferDto> {
    let query = normalize_search(query.trim());
    if !query.is_empty() {
        return offers
            .iter()
            .filter(|o| normalize_search(&o.name).contains(&query))
            .collect();
    }
    let Some(selected) = selected else {
        return offers.iter().collect();
    };
    offers
        .iter()
        .filter(|o| {
            o.category_id == selected
                || categories
                    .iter()
                    .any(|c| c.id == o.category_id && c.parent == selected)
        })
        .collect()
}

/// Whether the wallet can pay for one unit of the offer.
pub fn can_afford(wallet: &WalletDto, offer: &ShopOfferDto) -> bool {
    wallet.covers(offer.price_silver, offer.price_gold)
}

/// Feedback line after a purchase attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShopStatus {
    Success(String),
    Error(String),
}

impl ShopStatus {
    pub fn text(&self) -> &str {
        match self {
            ShopStatus::Success(t) | ShopStatus::Error(t) => t,
        }
    }
}

pub fn purchase_success_text(name: &str) -> String {
    format!("{name} adicionado ao inventário!")
}

// ---------------------------------------------------------------------------
// State, components, handles
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Default)]
pub struct ShopState {
    pub is_open: bool,
    pub is_loading: bool,
    pub categories: Vec<ShopCategoryDto>,
    pub offers: Vec<ShopOfferDto>,
    pub selected_category: Option<String>,
    /// Sections folded by the player (accordion); a section is open when it holds the selection.
    pub collapsed: HashSet<String>,
    pub query: String,
    /// Offer being purchased (waiting for the server); blocks double clicks.
    pub pending: Option<String>,
    pub status: Option<ShopStatus>,
}

/// Present when the Compose shell draws the shop: the bevy_hui window is not spawned then.
#[derive(Resource)]
pub struct ComposeShop;

/// HUD button that opens the shop.
#[derive(Component)]
pub struct HudShopBtn;

#[derive(Component)]
struct ShopUiRoot;

#[derive(Resource)]
struct ShopTemplates {
    window: Handle<HtmlTemplate>,
    menu_item: Handle<HtmlTemplate>,
    offer: Handle<HtmlTemplate>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct ShopPlugin;

impl Plugin for ShopPlugin {
    fn build(&self, app: &mut App) {
        crate::ensure_hui_plugins(app);
        app.init_resource::<ShopState>()
            .add_systems(Startup, setup_shop_ui)
            .add_systems(OnExit(AppState::InRoom), close_shop_on_leaving_room)
            .add_systems(
                Update,
                (
                    toggle_shop_from_hud,
                    close_shop_when_inventory_opens,
                    close_shop_on_escape,
                    apply_catalog_loaded,
                    apply_purchase_results,
                    sync_search_placeholder,
                    sync_shop_window,
                    scroll_shop_grid,
                    update_shop_scrollbar,
                )
                    .chain()
                    .run_if(in_state(AppState::InRoom)),
            );
    }
}

fn setup_shop_ui(mut commands: Commands, server: Res<AssetServer>, mut functions: HtmlFunctions) {
    commands.insert_resource(ShopTemplates {
        window: server.load("ui/shop.html"),
        menu_item: server.load("ui/shop_menu_item.html"),
        offer: server.load("ui/shop_offer.html"),
    });

    functions.register(
        "shop_close",
        |_: In<Entity>, mut state: ResMut<ShopState>| {
            state.is_open = false;
        },
    );
    functions.register(
        "shop_select_category",
        |In(entity): In<Entity>, tags: Query<&Tags>, mut state: ResMut<ShopState>| {
            if let Some(key) = tags.get(entity).ok().and_then(|t| t.get("key")) {
                let state = &mut *state;
                state.selected_category = Some(press_category(
                    &state.categories,
                    key,
                    state.selected_category.as_deref(),
                    &mut state.collapsed,
                ));
                state.status = None;
            }
        },
    );
    functions.register(
        "shop_search_changed",
        |In(entity): In<Entity>, inputs: Query<&TextInput>, mut state: ResMut<ShopState>| {
            if let Ok(input) = inputs.get(entity)
                && state.query != input.value
            {
                state.query = input.value.clone();
            }
        },
    );
    functions.register("shop_buy", buy_offer);
}

fn buy_offer(
    In(entity): In<Entity>,
    tags: Query<&Tags>,
    mut state: ResMut<ShopState>,
    wallet: Res<WalletState>,
    net: Option<Res<NakamaResource>>,
) {
    let Some(catalog_id) = tags.get(entity).ok().and_then(|t| t.get("catalog_id")) else {
        return;
    };
    if state.pending.is_some() {
        return;
    }
    let Some(offer) = state.offers.iter().find(|o| &o.catalog_id == catalog_id) else {
        return;
    };
    // Client-side check is only a shortcut; the server re-validates everything.
    if !can_afford(&wallet.balance, offer) {
        state.status = Some(ShopStatus::Error("Saldo insuficiente.".to_string()));
        return;
    }
    let catalog_id = offer.catalog_id.clone();
    if let Some(net) = &net {
        net.handle.send_command(NakamaCommand::PurchaseItem {
            catalog_id: catalog_id.clone(),
            quantity: 1,
        });
        state.pending = Some(catalog_id);
        state.status = None;
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn toggle_shop_from_hud(
    button: Query<&Interaction, (Changed<Interaction>, With<HudShopBtn>)>,
    mut state: ResMut<ShopState>,
    mut inventory: ResMut<PlayerInventoryState>,
    net: Option<Res<NakamaResource>>,
) {
    if !button.iter().any(|i| *i == Interaction::Pressed) {
        return;
    }
    let open = !state.is_open;
    set_shop_open(open, &mut state, &mut inventory, net.as_deref());
}

/// Opens or closes the shop; opening closes the inventory and fetches the catalog. Shared by the
/// bevy_hui HUD button and the Compose shell's `shop_open`.
pub fn set_shop_open(
    open: bool,
    state: &mut ShopState,
    inventory: &mut PlayerInventoryState,
    net: Option<&NakamaResource>,
) {
    state.is_open = open;
    if open {
        inventory.is_open = false;
        state.is_loading = true;
        state.status = None;
        state.pending = None;
        if let Some(net) = net {
            net.handle.send_command(NakamaCommand::FetchShopCatalog);
        }
    }
}

/// The shop systems only run in a room, so leaving it (e.g. to the character creator) must close
/// the window here, or its overlay would stay on screen with nothing handling its buttons.
fn close_shop_on_leaving_room(
    mut commands: Commands,
    mut state: ResMut<ShopState>,
    roots: Query<Entity, With<ShopUiRoot>>,
) {
    state.is_open = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// Only one big window at a time: opening the inventory closes the shop.
fn close_shop_when_inventory_opens(
    inventory: Res<PlayerInventoryState>,
    mut state: ResMut<ShopState>,
) {
    if inventory.is_changed() && inventory.is_open && state.is_open {
        state.is_open = false;
    }
}

/// Esc first leaves the search field (if focused), and only then closes the window.
fn close_shop_on_escape(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ShopState>,
    focused_inputs: Query<Entity, (With<TextInput>, With<UiActive>)>,
) {
    if !state.is_open || !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if focused_inputs.is_empty() {
        state.is_open = false;
    } else {
        for entity in &focused_inputs {
            commands.entity(entity).remove::<UiActive>();
        }
    }
}

fn apply_catalog_loaded(
    mut events: MessageReader<NetShopCatalogLoadedEvent>,
    mut state: ResMut<ShopState>,
) {
    for event in events.read() {
        info!(
            "Shop catalog loaded: {} categories, {} offers",
            event.0.categories.len(),
            event.0.offers.len()
        );
        state.is_loading = false;
        state.categories = event.0.categories.clone();
        state.offers = event.0.offers.clone();
        state.selected_category =
            resolve_category(&state.categories, state.selected_category.as_deref());
    }
}

fn apply_purchase_results(
    mut succeeded: MessageReader<NetPurchaseSucceededEvent>,
    mut failed: MessageReader<NetPurchaseFailedEvent>,
    mut state: ResMut<ShopState>,
    mut inventory: ResMut<PlayerInventoryState>,
    net: Option<Res<NakamaResource>>,
) {
    for event in succeeded.read() {
        let name = state
            .offers
            .iter()
            .find(|o| o.catalog_id == event.0.catalog_id)
            .map_or(event.0.catalog_id.as_str(), |o| o.name.as_str())
            .to_string();
        state.pending = None;
        state.status = Some(ShopStatus::Success(purchase_success_text(&name)));
        // Bought items land in the inventory: refresh it so it is current when opened.
        inventory.is_loading = true;
        if let Some(net) = &net {
            net.handle.send_command(NakamaCommand::FetchInventory);
        }
    }
    for event in failed.read() {
        warn!("Purchase of {} failed: {}", event.catalog_id, event.message);
        state.pending = None;
        state.status = Some(ShopStatus::Error(capitalize(&event.message)));
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Shows the "Buscar..." hint only while the search field is empty.
fn sync_search_placeholder(
    inputs: Query<(&TextInput, &UiId)>,
    mut placeholders: Query<(&UiId, &mut Visibility)>,
) {
    let Some((input, _)) = inputs.iter().find(|(_, id)| id.id() == "shop_search") else {
        return;
    };
    let wanted = if input.value.is_empty() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (id, mut visibility) in &mut placeholders {
        if id.id() == "shop_search_ph" && *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// Keeps the window entity in sync with `ShopState.is_open` and refills the menu, grid, balances
/// and feedback line whenever the shop state or the wallet changes.
fn sync_shop_window(
    mut commands: Commands,
    templates: Option<Res<ShopTemplates>>,
    mut state: ResMut<ShopState>,
    wallet: Res<WalletState>,
    safe_area: Res<SafeArea>,
    ui_scale: Res<UiScale>,
    ui_font: Res<UiFont>,
    asset_server: Res<AssetServer>,
    roots: Query<Entity, With<ShopUiRoot>>,
    ids: Query<(Entity, &UiId)>,
    scrolls: Query<(), With<ScrollPosition>>,
    mut texts: Query<(&UiId, &mut Text, Option<&mut TextColor>)>,
    mut icons: Query<(&UiId, &mut ImageNode, &mut Visibility)>,
    mut needs_fill: Local<bool>,
    mut last_view: Local<(Option<String>, String)>,
    compose: Option<Res<ComposeShop>>,
) {
    let Some(templates) = templates else { return };

    if !state.is_open {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *needs_fill = false;
        if state.is_changed() && (!state.query.is_empty() || state.pending.is_some()) {
            // The search field is rebuilt empty next time.
            state.query.clear();
            state.pending = None;
        }
        return;
    }
    if compose.is_some() {
        return; // The Compose shell draws the shop window.
    }

    if safe_area.is_changed() || ui_scale.is_changed() {
        // The spacers that keep the content inside the safe area are fixed when the window is
        // built: rebuild it (e.g. after a rotation).
        for root in &roots {
            commands.entity(root).despawn();
        }
        *needs_fill = true;
        return;
    }
    if roots.is_empty() {
        commands.spawn((
            ShopUiRoot,
            // A clickable root blocks the HUD buttons below the window (see `LegacyButton`).
            crate::LegacyButton,
            HtmlNode(templates.window.clone()),
            safe_area.modal_properties(ui_scale.0),
            GlobalZIndex(50),
        ));
        *needs_fill = true;
    }
    if state.is_changed() || wallet.is_changed() {
        *needs_fill = true;
    }
    if !*needs_fill {
        return;
    }

    let find = |name: &str| ids.iter().find(|(_, id)| id.id() == name).map(|(e, _)| e);
    let (Some(menu), Some(grid), Some(scroll)) =
        (find("shop_menu"), find("shop_grid"), find("shop_scroll"))
    else {
        return; // template not built yet; retry next frame
    };
    *needs_fill = false;

    let searching = !state.query.trim().is_empty();
    let selected = state.selected_category.clone();

    // Side menu
    commands.entity(menu).despawn_children();
    commands.entity(menu).with_children(|parent| {
        for row in menu_rows(
            &state.categories,
            selected.as_deref(),
            !searching,
            &state.collapsed,
        ) {
            let (text_color, icon_color) = if row.selected {
                ("#ffffff", "#ffffff")
            } else if row.depth > 0 {
                ("#cbd5e1", "#94a3b8")
            } else {
                ("#f1f5f9", "#cbd5e1")
            };
            parent.spawn((
                HtmlNode(templates.menu_item.clone()),
                TemplateProperties::default()
                    .with("key", &row.key)
                    .with("label", &row.label)
                    .with("icon", &row.icon)
                    .with("indent", if row.depth > 0 { "12px" } else { "0px" })
                    .with(
                        "chevron",
                        if row.accordion == Some(true) {
                            "ui/icons/caret-down.png"
                        } else {
                            "ui/icons/caret-right.png"
                        },
                    )
                    .with(
                        "chevron_display",
                        if row.accordion.is_some() {
                            "flex"
                        } else {
                            "none"
                        },
                    )
                    .with("text_color", text_color)
                    .with("icon_color", icon_color)
                    .with(
                        "bg",
                        if row.selected {
                            MENU_BG_SELECTED
                        } else {
                            MENU_BG
                        },
                    ),
            ));
        }
    });

    // Offer grid. Keep the scroll offset across refills (balance/pending changes); jump to the
    // top only when the visible set changes because of the category or the search.
    let view = (selected.clone(), state.query.clone());
    let view_changed = *last_view != view;
    *last_view = view;
    commands.entity(grid).despawn_children();
    if scrolls.get(scroll).is_err() || view_changed {
        commands.entity(scroll).insert(ScrollPosition::default());
    }
    let visible = filter_offers(
        &state.categories,
        &state.offers,
        selected.as_deref(),
        &state.query,
    );
    commands.entity(grid).with_children(|parent| {
        if visible.is_empty() {
            let message = if state.is_loading {
                "Carregando..."
            } else if searching {
                "Nenhum item encontrado."
            } else {
                "Nenhum item aqui."
            };
            parent.spawn((
                Text::new(message),
                ui_font.text_font(14.0),
                TextColor(STATUS_NEUTRAL),
            ));
        }
        for offer in &visible {
            let affordable = can_afford(&wallet.balance, offer);
            let buying = state.pending.as_deref() == Some(offer.catalog_id.as_str());
            let (buy_bg, buy_hover) = if affordable && state.pending.is_none() {
                (BUY_BG, BUY_BG_HOVER)
            } else {
                (BUY_BG_DISABLED, BUY_BG_DISABLED)
            };
            parent.spawn((
                HtmlNode(templates.offer.clone()),
                TemplateProperties::default()
                    .with("catalog_id", &offer.catalog_id)
                    .with("name", &offer.name)
                    .with("thumb", &thumbnail_path(&offer.catalog_id))
                    .with("silver", &format_amount(offer.price_silver))
                    .with("gold", &format_amount(offer.price_gold))
                    .with(
                        "silver_display",
                        if offer.price_silver > 0 {
                            "flex"
                        } else {
                            "none"
                        },
                    )
                    .with(
                        "gold_display",
                        if offer.price_gold > 0 { "flex" } else { "none" },
                    )
                    .with("buy_bg", buy_bg)
                    .with("buy_hover", buy_hover)
                    .with("buy_label", if buying { "Comprando..." } else { "Comprar" }),
            ));
        }
    });

    // Header balances and feedback line
    let balance_label = |amount: i64| {
        if wallet.loaded {
            format_amount(amount)
        } else {
            "-".to_string()
        }
    };
    let status = state.status.clone();
    for (id, mut text, color) in &mut texts {
        match id.id().as_str() {
            "shop_silver" => text.0 = balance_label(wallet.balance.silver),
            "shop_gold" => text.0 = balance_label(wallet.balance.gold),
            "shop_status" => {
                // A lone space keeps the (otherwise empty) text node laid out.
                text.0 = status.as_ref().map_or(" ", ShopStatus::text).to_string();
                if let Some(mut color) = color {
                    color.0 = match &status {
                        Some(ShopStatus::Success(_)) => STATUS_OK,
                        Some(ShopStatus::Error(_)) => STATUS_ERR,
                        None => STATUS_NEUTRAL,
                    };
                }
            }
            _ => {}
        }
    }
    for (id, mut image, mut visibility) in &mut icons {
        if id.id() != "shop_status_icon" {
            continue;
        }
        let (path, tint, wanted) = match &status {
            Some(ShopStatus::Success(_)) => (
                "ui/icons/check-circle.png",
                STATUS_OK,
                Visibility::Inherited,
            ),
            Some(ShopStatus::Error(_)) => (
                "ui/icons/warning-circle.png",
                STATUS_ERR,
                Visibility::Inherited,
            ),
            None => ("ui/icons/check-circle.png", STATUS_OK, Visibility::Hidden),
        };
        image.image = asset_server.load(path);
        image.color = tint;
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

fn scroll_shop_grid(
    mut wheel: MessageReader<MouseWheel>,
    ids: Query<(&UiId, &mut ScrollPosition)>,
) {
    let delta: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y * SCROLL_LINE_PX,
            MouseScrollUnit::Pixel => ev.y,
        })
        .sum();
    if delta == 0.0 {
        return;
    }
    for (id, mut scroll) in ids {
        if id.id() == "shop_scroll" {
            scroll.y = (scroll.y - delta).max(0.0);
        }
    }
}

/// Drives the scrollbar thumb with `UiTransform` (same technique as the inventory window).
fn update_shop_scrollbar(
    scrolls: Query<(&UiId, &ComputedNode, &ScrollPosition)>,
    mut parts: Query<(&UiId, &mut UiTransform, &mut Visibility)>,
) {
    let Some((_, scroll_node, position)) = scrolls.iter().find(|(id, ..)| id.id() == "shop_scroll")
    else {
        return;
    };
    let metrics = scrollbar_metrics(
        scroll_node.size().y,
        scroll_node.content_size().y,
        position.y / scroll_node.inverse_scale_factor,
    );
    for (id, mut transform, mut visibility) in &mut parts {
        match (id.id().as_str(), metrics) {
            ("shop_track", metrics) => {
                let wanted = if metrics.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != wanted {
                    *visibility = wanted;
                }
            }
            ("shop_thumb", Some((ratio, progress))) => {
                let center = ratio / 2.0 + progress * (1.0 - ratio);
                transform.scale = Vec2::new(1.0, ratio);
                transform.translation = Val2::percent(0.0, (center - 0.5) * 100.0);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(id: &str, parent: &str) -> ShopCategoryDto {
        ShopCategoryDto {
            id: id.to_string(),
            name: id.to_uppercase(),
            icon: "package".to_string(),
            parent: parent.to_string(),
        }
    }

    fn offer(id: &str, name: &str, category: &str, silver: i64, gold: i64) -> ShopOfferDto {
        ShopOfferDto {
            catalog_id: id.to_string(),
            category_id: category.to_string(),
            name: name.to_string(),
            width: 1,
            depth: 1,
            walkable: false,
            price_silver: silver,
            price_gold: gold,
        }
    }

    fn categories() -> Vec<ShopCategoryDto> {
        vec![
            cat("furniture", ""),
            cat("seats", "furniture"),
            cat("beds", "furniture"),
            cat("floors", ""),
            cat("tiles", "floors"),
        ]
    }

    fn offers() -> Vec<ShopOfferDto> {
        vec![
            offer("stool", "Banqueta Alta", "seats", 15, 0),
            offer("armchair", "Poltrona de Couro", "seats", 0, 5),
            offer("bed", "Cama de Solteiro", "beds", 100, 2),
            offer("tile", "Piso Dourado", "tiles", 3, 0),
        ]
    }

    fn ids(list: Vec<&ShopOfferDto>) -> Vec<&str> {
        list.iter().map(|o| o.catalog_id.as_str()).collect()
    }

    #[test]
    fn default_category_is_first_top_level() {
        assert_eq!(
            default_category(&categories()).as_deref(),
            Some("furniture")
        );
        assert_eq!(default_category(&[]), None);
    }

    #[test]
    fn resolve_category_falls_back_when_missing() {
        let cats = categories();
        assert_eq!(
            resolve_category(&cats, Some("tiles")).as_deref(),
            Some("tiles")
        );
        assert_eq!(
            resolve_category(&cats, Some("gone")).as_deref(),
            Some("furniture")
        );
        assert_eq!(resolve_category(&cats, None).as_deref(), Some("furniture"));
    }

    #[test]
    fn menu_expands_only_the_selected_branch() {
        let rows = menu_rows(&categories(), Some("furniture"), true, &HashSet::new());
        let keys: Vec<_> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["furniture", "seats", "beds", "floors"]);
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[1].depth, 1);
        assert!(rows[0].selected && !rows[1].selected);

        // Selecting a child keeps its branch open and highlights only the child.
        let rows = menu_rows(&categories(), Some("tiles"), true, &HashSet::new());
        let keys: Vec<_> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["furniture", "floors", "tiles"]);
        assert!(rows.iter().filter(|r| r.selected).count() == 1 && rows[2].selected);
    }

    #[test]
    fn accordion_headers_report_their_state() {
        let rows = menu_rows(&categories(), Some("furniture"), true, &HashSet::new());
        assert_eq!(rows[0].accordion, Some(true));
        assert_eq!(rows[1].accordion, None);
        let floors = rows.iter().find(|r| r.key == "floors").unwrap();
        assert_eq!(floors.accordion, Some(false));
    }

    #[test]
    fn pressing_the_open_section_folds_it_and_pressing_again_opens_it() {
        let cats = categories();
        let mut collapsed = HashSet::new();
        // Opening another section selects it and keeps it open.
        let selected = press_category(&cats, "floors", Some("furniture"), &mut collapsed);
        assert_eq!(selected, "floors");
        assert!(collapsed.is_empty());
        // Pressing the selected section folds it ...
        press_category(&cats, "floors", Some("floors"), &mut collapsed);
        let rows = menu_rows(&cats, Some("floors"), true, &collapsed);
        assert!(rows.iter().all(|r| r.key != "tiles"));
        assert_eq!(
            rows.iter().find(|r| r.key == "floors").unwrap().accordion,
            Some(false)
        );
        // ... and pressing it once more opens it.
        press_category(&cats, "floors", Some("floors"), &mut collapsed);
        assert!(collapsed.is_empty());
    }

    #[test]
    fn menu_highlight_is_suspended_while_searching() {
        let rows = menu_rows(&categories(), Some("furniture"), false, &HashSet::new());
        assert!(rows.iter().all(|r| !r.selected));
    }

    #[test]
    fn parent_category_shows_children_offers() {
        let (cats, offs) = (categories(), offers());
        assert_eq!(
            ids(filter_offers(&cats, &offs, Some("furniture"), "")),
            vec!["stool", "armchair", "bed"]
        );
        assert_eq!(
            ids(filter_offers(&cats, &offs, Some("seats"), "")),
            vec!["stool", "armchair"]
        );
        assert_eq!(
            ids(filter_offers(&cats, &offs, Some("floors"), "")),
            vec!["tile"]
        );
        assert_eq!(filter_offers(&cats, &offs, None, "").len(), 4);
    }

    #[test]
    fn search_spans_all_categories_ignoring_case_and_accents() {
        let (cats, offs) = (categories(), offers());
        assert_eq!(
            ids(filter_offers(&cats, &offs, Some("floors"), "POLTRONA")),
            vec!["armchair"]
        );
        assert_eq!(
            ids(filter_offers(&cats, &offs, Some("seats"), "  piso ")),
            vec!["tile"]
        );
        assert_eq!(
            filter_offers(&cats, &offs, Some("floors"), "couro").len(),
            1
        );
        assert!(filter_offers(&cats, &offs, None, "xyz").is_empty());
    }

    #[test]
    fn affordability_checks_both_currencies() {
        let bed = offer("bed", "Cama", "beds", 100, 2);
        assert!(can_afford(
            &WalletDto {
                silver: 100,
                gold: 2
            },
            &bed
        ));
        assert!(!can_afford(
            &WalletDto {
                silver: 99,
                gold: 2
            },
            &bed
        ));
        assert!(!can_afford(
            &WalletDto {
                silver: 500,
                gold: 1
            },
            &bed
        ));
        let free_silver = offer("x", "X", "c", 0, 3);
        assert!(can_afford(&WalletDto { silver: 0, gold: 3 }, &free_silver));
    }

    #[test]
    fn texts() {
        assert_eq!(category_icon_path("rug"), "ui/icons/rug.png");
        assert_eq!(
            purchase_success_text("Cama"),
            "Cama adicionado ao inventário!"
        );
        assert_eq!(capitalize("saldo insuficiente"), "Saldo insuficiente");
        assert_eq!(capitalize(""), "");
    }
}
