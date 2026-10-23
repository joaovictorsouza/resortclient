//! Room navigator window built with `bevy_hui` templates (see `assets/ui/navigator*.html`).
//!
//! Text inputs use the `bevy_hui_widgets` `TextInput`; the input bar is rebuilt only when the tab
//! changes so typing never loses focus. Pure helpers are free of the ECS for unit testing.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_hui::prelude::*;
use bevy_hui_widgets::prelude::TextInput;
use nakama_client::NakamaCommand;
use protocol::{RoomLayoutDto, RoomSummaryDto};

use crate::inventory::scrollbar_metrics;
use crate::navigator::{
    ActiveInputFocus, ActiveRoomMetadata, NavigatorOverlayRoot, NavigatorTab, RoomNavigatorState,
    submit_create_room,
};
use crate::net::NakamaResource;
use crate::safe_area::SafeArea;
use crate::state::AppState;

const SCROLL_LINE_PX: f32 = 32.0;
pub const MAX_NAME_CHARS: usize = 50;
const TAB_BG: &str = "#1e293b"; // slate-800
const TAB_BG_SELECTED: &str = "#4f46e5"; // indigo-600
const EDGE: &str = "#334155"; // slate-700
const EDGE_SELECTED: &str = "#818cf8"; // indigo-400

/// Built-in public spaces: (room id, name, description, capacity).
pub const PUBLIC_SPACES: [(&str, &str, &str, u32); 2] = [
    (
        "lobby",
        "The Grand Lobby",
        "Espaço público central do hotel para interagir e conversar.",
        100,
    ),
    (
        "cafe",
        "Coffee House",
        "Cafeteria aconchegante para relaxar com amigos.",
        40,
    ),
];

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

impl NavigatorTab {
    pub const ALL: [NavigatorTab; 5] = [
        NavigatorTab::PublicSpaces,
        NavigatorTab::Popular,
        NavigatorTab::MyRooms,
        NavigatorTab::Search,
        NavigatorTab::CreateRoom,
    ];

    /// Stable identifier used in template tags.
    pub fn key(self) -> &'static str {
        match self {
            NavigatorTab::PublicSpaces => "public",
            NavigatorTab::Popular => "popular",
            NavigatorTab::MyRooms => "mine",
            NavigatorTab::Search => "search",
            NavigatorTab::CreateRoom => "create",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            NavigatorTab::PublicSpaces => "Espaços públicos",
            NavigatorTab::Popular => "Salas populares",
            NavigatorTab::MyRooms => "Minhas salas",
            NavigatorTab::Search => "Buscar",
            NavigatorTab::CreateRoom => "Criar espaço",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            NavigatorTab::PublicSpaces => "ui/icons/buildings.png",
            NavigatorTab::Popular => "ui/icons/fire.png",
            NavigatorTab::MyRooms => "ui/icons/house.png",
            NavigatorTab::Search => "ui/icons/magnifying-glass.png",
            NavigatorTab::CreateRoom => "ui/icons/plus-circle.png",
        }
    }

    /// Tabs whose input bar is a text field.
    pub fn has_input(self) -> bool {
        matches!(self, NavigatorTab::Search | NavigatorTab::CreateRoom)
    }
}

pub fn visibility_label(visibility: &str) -> &'static str {
    match visibility {
        "public" => "Pública",
        "hidden" => "Oculta",
        _ => "Privada",
    }
}

pub fn room_meta(room: &RoomSummaryDto) -> String {
    format!(
        "Criado por {}  •  Ocupação {}/{}  •  {}",
        room.owner_name.as_deref().unwrap_or("Player"),
        room.user_count,
        room.max_users,
        visibility_label(&room.visibility)
    )
}

pub fn layout_meta(layout: &RoomLayoutDto) -> String {
    format!(
        "{}x{} (máx. {})",
        layout.width, layout.height, layout.max_users
    )
}

/// Cuts text to at most `MAX_NAME_CHARS` characters.
pub fn clamp_name(text: &str) -> String {
    text.chars().take(MAX_NAME_CHARS).collect()
}

/// The text a tab's input bar should start with.
pub fn input_value(nav: &RoomNavigatorState) -> &str {
    match nav.current_tab {
        NavigatorTab::Search => &nav.search_query,
        NavigatorTab::CreateRoom => &nav.create_name,
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Actions shared by buttons and the Enter key
// ---------------------------------------------------------------------------

fn select_tab(tab: NavigatorTab, nav: &mut RoomNavigatorState, net: Option<&NakamaResource>) {
    nav.current_tab = tab;
    let Some(net) = net else { return };
    match tab {
        NavigatorTab::Popular => {
            nav.is_loading_popular = true;
            net.handle.send_command(NakamaCommand::FetchPopularRooms);
        }
        NavigatorTab::MyRooms => {
            net.handle.send_command(NakamaCommand::FetchMyRooms);
        }
        NavigatorTab::CreateRoom => {
            net.handle.send_command(NakamaCommand::FetchRoomLayouts);
        }
        _ => {}
    }
}

fn join_room(
    room_id: &str,
    nav: &mut RoomNavigatorState,
    meta: &mut ActiveRoomMetadata,
    focus: &mut ActiveInputFocus,
    net: Option<&NakamaResource>,
) {
    let Some(net) = net else { return };
    info!("Requesting join to room: {room_id}");
    if let Some(mine) = nav.my_rooms.iter().find(|r| r.room_id == room_id) {
        meta.is_owner = true;
        meta.visibility = mine.visibility.clone();
        meta.room_name = mine.name.clone();
    } else if room_id == "lobby" || room_id == "cafe" {
        meta.is_owner = false;
        meta.visibility = "public".to_string();
    } else {
        meta.is_owner = false;
    }
    net.handle.send_command(NakamaCommand::RequestJoinRoom {
        room_id: room_id.to_string(),
    });
    nav.is_open = false;
    focus.0 = crate::navigator::FocusedInput::None;
}

fn start_search(nav: &mut RoomNavigatorState, net: Option<&NakamaResource>) {
    let query = nav.search_query.trim().to_string();
    if let Some(net) = net {
        nav.is_searching = true;
        nav.has_searched = true;
        net.handle
            .send_command(NakamaCommand::SearchRooms { query });
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct NavigatorTemplates {
    window: Handle<HtmlTemplate>,
    tab: Handle<HtmlTemplate>,
    room: Handle<HtmlTemplate>,
    message: Handle<HtmlTemplate>,
    title: Handle<HtmlTemplate>,
    layout: Handle<HtmlTemplate>,
    create_actions: Handle<HtmlTemplate>,
    search_bar: Handle<HtmlTemplate>,
    create_bar: Handle<HtmlTemplate>,
}

pub struct NavigatorUiPlugin;

impl Plugin for NavigatorUiPlugin {
    fn build(&self, app: &mut App) {
        crate::ensure_hui_plugins(app);
        app.add_systems(Startup, setup_navigator_ui).add_systems(
            Update,
            (
                navigator_enter_submit,
                restore_input_value,
                sync_input_placeholder,
                sync_navigator_window,
                scroll_navigator_content,
                update_navigator_scrollbar,
            )
                .chain()
                .run_if(in_state(AppState::InRoom)),
        );
    }
}

fn setup_navigator_ui(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut functions: HtmlFunctions,
) {
    commands.insert_resource(NavigatorTemplates {
        window: server.load("ui/navigator.html"),
        tab: server.load("ui/navigator_tab.html"),
        room: server.load("ui/navigator_room.html"),
        message: server.load("ui/navigator_message.html"),
        title: server.load("ui/navigator_title.html"),
        layout: server.load("ui/navigator_layout.html"),
        create_actions: server.load("ui/navigator_create_actions.html"),
        search_bar: server.load("ui/navigator_search_bar.html"),
        create_bar: server.load("ui/navigator_create_bar.html"),
    });

    functions.register(
        "nav_close",
        |_: In<Entity>, mut nav: ResMut<RoomNavigatorState>| nav.is_open = false,
    );
    functions.register(
        "nav_tab",
        |In(entity): In<Entity>,
         tags: Query<&Tags>,
         mut nav: ResMut<RoomNavigatorState>,
         net: Option<Res<NakamaResource>>| {
            if let Some(tab) = tags
                .get(entity)
                .ok()
                .and_then(|t| t.get("key"))
                .and_then(|k| NavigatorTab::from_key(k))
            {
                select_tab(tab, &mut nav, net.as_deref());
            }
        },
    );
    functions.register(
        "nav_join",
        |In(entity): In<Entity>,
         tags: Query<&Tags>,
         mut nav: ResMut<RoomNavigatorState>,
         mut meta: ResMut<ActiveRoomMetadata>,
         mut focus: ResMut<ActiveInputFocus>,
         net: Option<Res<NakamaResource>>| {
            if let Some(room_id) = tags.get(entity).ok().and_then(|t| t.get("room_id")) {
                join_room(room_id, &mut nav, &mut meta, &mut focus, net.as_deref());
            }
        },
    );
    functions.register(
        "nav_layout",
        |In(entity): In<Entity>, tags: Query<&Tags>, mut nav: ResMut<RoomNavigatorState>| {
            if let Some(id) = tags.get(entity).ok().and_then(|t| t.get("layout_id")) {
                nav.selected_layout_id = Some(id.clone());
            }
        },
    );
    functions.register(
        "nav_create_submit",
        |_: In<Entity>,
         mut nav: ResMut<RoomNavigatorState>,
         mut focus: ResMut<ActiveInputFocus>,
         net: Option<Res<NakamaResource>>| {
            submit_create_room(&mut nav, &mut focus, net.as_deref());
        },
    );
    functions.register(
        "nav_search_submit",
        |_: In<Entity>, mut nav: ResMut<RoomNavigatorState>, net: Option<Res<NakamaResource>>| {
            start_search(&mut nav, net.as_deref());
        },
    );
    functions.register(
        "nav_input_changed",
        |In(entity): In<Entity>,
         mut inputs: Query<&mut TextInput>,
         mut nav: ResMut<RoomNavigatorState>| {
            let Ok(mut input) = inputs.get_mut(entity) else {
                return;
            };
            if input.value.chars().count() > MAX_NAME_CHARS {
                input.value = clamp_name(&input.value);
            }
            let value = input.value.clone();
            let target = match nav.current_tab {
                NavigatorTab::Search => &mut nav.search_query,
                NavigatorTab::CreateRoom => &mut nav.create_name,
                _ => return,
            };
            if *target != value {
                *target = value;
            }
        },
    );
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Enter inside a text field (which the widget turns into "unfocus") also submits it.
fn navigator_enter_submit(
    keys: Res<ButtonInput<KeyCode>>,
    inputs: Query<(), (With<TextInput>, With<UiActive>)>,
    mut was_active: Local<bool>,
    mut nav: ResMut<RoomNavigatorState>,
    mut focus: ResMut<ActiveInputFocus>,
    net: Option<Res<NakamaResource>>,
) {
    let active = !inputs.is_empty();
    if *was_active && !active && nav.is_open && keys.just_pressed(KeyCode::Enter) {
        match nav.current_tab {
            NavigatorTab::Search => start_search(&mut nav, net.as_deref()),
            NavigatorTab::CreateRoom if nav.selected_layout_id.is_some() => {
                submit_create_room(&mut nav, &mut focus, net.as_deref());
            }
            _ => {}
        }
    }
    *was_active = active;
}

/// A freshly built input bar starts with the text kept in the navigator state.
fn restore_input_value(
    nav: Res<RoomNavigatorState>,
    mut inputs: Query<(&mut TextInput, &UiId), Added<TextInput>>,
) {
    for (mut input, id) in &mut inputs {
        if id.id() == "nav_input" {
            input.value = input_value(&nav).to_string();
        }
    }
}

/// Shows the hint only while the input is empty.
fn sync_input_placeholder(
    inputs: Query<(&TextInput, &UiId)>,
    mut placeholders: Query<(&UiId, &mut Visibility)>,
) {
    let Some((input, _)) = inputs.iter().find(|(_, id)| id.id() == "nav_input") else {
        return;
    };
    let wanted = if input.value.is_empty() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (id, mut visibility) in &mut placeholders {
        if id.id() == "nav_input_ph" && *visibility != wanted {
            *visibility = wanted;
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Keeps the window in sync with `RoomNavigatorState`: spawns/despawns it, rebuilds the tabs and
/// content on every change, and the input bar only when the tab changes.
fn sync_navigator_window(
    mut commands: Commands,
    templates: Option<Res<NavigatorTemplates>>,
    nav: Res<RoomNavigatorState>,
    safe_area: Res<SafeArea>,
    ui_scale: Res<UiScale>,
    roots: Query<Entity, With<NavigatorOverlayRoot>>,
    ids: Query<(Entity, &UiId)>,
    mut bar_tab: Local<Option<NavigatorTab>>,
    mut needs_fill: Local<bool>,
) {
    let Some(templates) = templates else { return };

    if !nav.is_open {
        for root in &roots {
            commands.entity(root).despawn();
        }
        *bar_tab = None;
        *needs_fill = false;
        return;
    }
    if safe_area.is_changed() || ui_scale.is_changed() {
        // The spacers that keep the content inside the safe area are fixed when the window is
        // built: rebuild it (e.g. after a rotation).
        for root in &roots {
            commands.entity(root).despawn();
        }
        *bar_tab = None;
        *needs_fill = true;
        return;
    }
    if roots.is_empty() {
        commands.spawn((
            NavigatorOverlayRoot,
            // A clickable root blocks the HUD buttons below the window (see `LegacyButton`).
            crate::LegacyButton,
            HtmlNode(templates.window.clone()),
            safe_area.modal_properties(ui_scale.0),
            GlobalZIndex(50),
        ));
        *bar_tab = None;
        *needs_fill = true;
    }
    if nav.is_changed() {
        *needs_fill = true;
    }
    if !*needs_fill {
        return;
    }

    let find = |name: &str| ids.iter().find(|(_, id)| id.id() == name).map(|(e, _)| e);
    let (Some(tabs), Some(bar), Some(content), Some(scroll)) = (
        find("nav_tabs"),
        find("nav_bar"),
        find("nav_content"),
        find("nav_scroll"),
    ) else {
        return; // template not built yet; retry next frame
    };
    *needs_fill = false;

    // Tabs
    commands.entity(tabs).despawn_children();
    commands.entity(tabs).with_children(|parent| {
        for tab in NavigatorTab::ALL {
            parent.spawn((
                HtmlNode(templates.tab.clone()),
                TemplateProperties::default()
                    .with("key", tab.key())
                    .with("label", tab.label())
                    .with("icon", tab.icon())
                    .with(
                        "bg",
                        if tab == nav.current_tab {
                            TAB_BG_SELECTED
                        } else {
                            TAB_BG
                        },
                    ),
            ));
        }
    });

    // Input bar (only when the tab changed)
    if *bar_tab != Some(nav.current_tab) {
        *bar_tab = Some(nav.current_tab);
        commands.entity(bar).despawn_children();
        commands.entity(scroll).insert(ScrollPosition::default());
        let bar_template = match nav.current_tab {
            NavigatorTab::Search => Some((&templates.search_bar, "Digite o nome da sala...")),
            NavigatorTab::CreateRoom => Some((&templates.create_bar, "Ex.: Meu espaço virtual")),
            _ => None,
        };
        if let Some((template, placeholder)) = bar_template {
            commands.entity(bar).with_children(|parent| {
                parent.spawn((
                    HtmlNode(template.clone()),
                    TemplateProperties::default().with("placeholder", placeholder),
                ));
            });
        }
    }

    // Content
    commands.entity(content).despawn_children();
    commands
        .entity(content)
        .with_children(|parent| match nav.current_tab {
            NavigatorTab::PublicSpaces => {
                for (id, name, desc, max) in PUBLIC_SPACES {
                    spawn_room(
                        parent,
                        &templates,
                        id,
                        name,
                        &format!("{desc} (capacidade {max})"),
                    );
                }
            }
            NavigatorTab::Popular => {
                let empty = if nav.is_loading_popular {
                    "Carregando salas populares..."
                } else {
                    "Nenhuma sala encontrada no momento."
                };
                spawn_room_list(parent, &templates, &nav.popular_rooms, empty);
            }
            NavigatorTab::MyRooms => spawn_room_list(
                parent,
                &templates,
                &nav.my_rooms,
                "Você ainda não criou nenhum espaço virtual.",
            ),
            NavigatorTab::Search => {
                if nav.is_searching {
                    spawn_message(parent, &templates, "Buscando salas disponíveis...");
                } else if !nav.has_searched {
                    spawn_message(
                        parent,
                        &templates,
                        "Digite um termo e clique em Buscar (ou deixe vazio para listar todas).",
                    );
                } else {
                    spawn_room_list(
                        parent,
                        &templates,
                        &nav.search_results,
                        "Nenhum resultado encontrado para a busca.",
                    );
                }
            }
            NavigatorTab::CreateRoom => {
                parent.spawn((
                    HtmlNode(templates.title.clone()),
                    TemplateProperties::default().with("text", "Escolha o layout de tiles"),
                ));
                if nav.layouts.is_empty() {
                    spawn_message(parent, &templates, "Carregando layouts...");
                }
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(10.0),
                        row_gap: Val::Px(10.0),
                        width: Val::Percent(100.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for layout in &nav.layouts {
                            let selected = nav.selected_layout_id.as_deref() == Some(&layout.id);
                            row.spawn((
                                HtmlNode(templates.layout.clone()),
                                TemplateProperties::default()
                                    .with("layout_id", &layout.id)
                                    .with("name", &layout.name)
                                    .with("meta", &layout_meta(layout))
                                    .with("bg", if selected { TAB_BG_SELECTED } else { TAB_BG })
                                    .with("edge", if selected { EDGE_SELECTED } else { EDGE }),
                            ));
                        }
                    });
                parent.spawn(HtmlNode(templates.create_actions.clone()));
            }
        });
}

fn spawn_room(
    parent: &mut ChildSpawnerCommands,
    templates: &NavigatorTemplates,
    room_id: &str,
    name: &str,
    meta: &str,
) {
    parent.spawn((
        HtmlNode(templates.room.clone()),
        TemplateProperties::default()
            .with("room_id", room_id)
            .with("name", name)
            .with("meta", meta),
    ));
}

fn spawn_message(parent: &mut ChildSpawnerCommands, templates: &NavigatorTemplates, text: &str) {
    parent.spawn((
        HtmlNode(templates.message.clone()),
        TemplateProperties::default().with("text", text),
    ));
}

fn spawn_room_list(
    parent: &mut ChildSpawnerCommands,
    templates: &NavigatorTemplates,
    rooms: &[RoomSummaryDto],
    empty_message: &str,
) {
    if rooms.is_empty() {
        spawn_message(parent, templates, empty_message);
        return;
    }
    for room in rooms {
        spawn_room(
            parent,
            templates,
            &room.room_id,
            &room.name,
            &room_meta(room),
        );
    }
}

fn scroll_navigator_content(
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
        if id.id() == "nav_scroll" {
            scroll.y = (scroll.y - delta).max(0.0);
        }
    }
}

/// Drives the scrollbar thumb with `UiTransform` (same technique as the inventory window).
fn update_navigator_scrollbar(
    scrolls: Query<(&UiId, &ComputedNode, &ScrollPosition)>,
    mut parts: Query<(&UiId, &mut UiTransform, &mut Visibility)>,
) {
    let Some((_, scroll_node, position)) = scrolls.iter().find(|(id, ..)| id.id() == "nav_scroll")
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
            ("nav_track", metrics) => {
                let wanted = if metrics.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != wanted {
                    *visibility = wanted;
                }
            }
            ("nav_thumb", Some((ratio, progress))) => {
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

    fn room(visibility: &str, owner: Option<&str>) -> RoomSummaryDto {
        RoomSummaryDto {
            room_id: "r1".into(),
            name: "Sala".into(),
            owner_name: owner.map(str::to_string),
            visibility: visibility.into(),
            user_count: 3,
            max_users: 20,
            owner_id: None,
            match_id: None,
        }
    }

    #[test]
    fn tab_keys_round_trip() {
        for tab in NavigatorTab::ALL {
            assert_eq!(NavigatorTab::from_key(tab.key()), Some(tab));
        }
        assert_eq!(NavigatorTab::from_key("nope"), None);
    }

    #[test]
    fn only_search_and_create_have_inputs() {
        let with_input: Vec<_> = NavigatorTab::ALL
            .into_iter()
            .filter(|t| t.has_input())
            .collect();
        assert_eq!(
            with_input,
            vec![NavigatorTab::Search, NavigatorTab::CreateRoom]
        );
    }

    #[test]
    fn room_meta_describes_owner_occupancy_and_visibility() {
        assert_eq!(
            room_meta(&room("public", Some("Alice"))),
            "Criado por Alice  •  Ocupação 3/20  •  Pública"
        );
        assert!(room_meta(&room("hidden", None)).contains("Player"));
        assert!(room_meta(&room("other", None)).ends_with("Privada"));
    }

    #[test]
    fn name_is_clamped_by_characters_not_bytes() {
        let long = "ç".repeat(80);
        assert_eq!(clamp_name(&long).chars().count(), MAX_NAME_CHARS);
        assert_eq!(clamp_name("curto"), "curto");
    }

    #[test]
    fn input_value_follows_the_current_tab() {
        let mut nav = RoomNavigatorState {
            search_query: "abc".into(),
            create_name: "xyz".into(),
            ..Default::default()
        };
        nav.current_tab = NavigatorTab::Search;
        assert_eq!(input_value(&nav), "abc");
        nav.current_tab = NavigatorTab::CreateRoom;
        assert_eq!(input_value(&nav), "xyz");
        nav.current_tab = NavigatorTab::Popular;
        assert_eq!(input_value(&nav), "");
    }
}
