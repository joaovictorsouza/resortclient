use crate::safe_area::{BASE_MARGIN, HUD_EDGE_MARGIN, HudEdge, SafeArea};
use crate::ui_scale::HUD_SIZE_FACTOR;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_hui::prelude::{
    HtmlFunctions, HtmlNode, HtmlTemplate, Tags, TemplateProperties, UiActive,
};
use bevy_hui_widgets::prelude::TextInput;
use protocol::{ClientMessage, DoorbellRequest, DoorbellResponse, RoomLayoutDto, RoomSummaryDto};

use crate::UiFont;
use crate::net::NakamaResource;
use crate::room::ActiveRoom;
use crate::state::AppState;
use nakama_client::NakamaCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigatorTab {
    #[default]
    PublicSpaces,
    Popular,
    MyRooms,
    Search,
    CreateRoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedInput {
    #[default]
    None,
    SettingsRoomName,
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveInputFocus(pub FocusedInput);

#[derive(Resource, Default, Debug)]
pub struct RoomNavigatorState {
    pub is_open: bool,
    pub current_tab: NavigatorTab,
    pub layouts: Vec<RoomLayoutDto>,
    pub popular_rooms: Vec<RoomSummaryDto>,
    pub my_rooms: Vec<RoomSummaryDto>,
    pub search_results: Vec<RoomSummaryDto>,
    pub search_query: String,
    pub is_searching: bool,
    pub is_loading_popular: bool,
    pub has_searched: bool,
    pub create_name: String,
    pub selected_layout_id: Option<String>,
    pub status_message: Option<String>,
}

#[derive(Resource, Default, Debug)]
pub struct RoomSettingsState {
    pub is_open: bool,
    pub edited_name: String,
    pub edited_visibility: String,
    pub is_confirming_delete: bool,
    pub error_message: Option<String>,
}

#[derive(Resource, Default, Debug)]
pub struct DoorbellState {
    pub incoming_request: Option<DoorbellRequest>,
    pub is_waiting_approval: bool,
    pub waiting_message: Option<String>,
}

/// Who owns the room being entered, as seen by `local_user_id` (the Nakama user id). Returns
/// `(owner_id, is_owner)`; it never depends on the previously active room.
///
/// The server sends the owner in the room snapshot (`snapshot_owner_id`, `""` = no owner, protocol 2.0),
/// and that is authoritative. The room lists are only a fallback for a snapshot that carries no
/// owner id (an empty one on a room that is listed as mine), and the public rooms never have an owner.
pub fn resolve_ownership(
    room_id: &str,
    snapshot_owner_id: &str,
    local_user_id: &str,
    my_rooms: &[RoomSummaryDto],
) -> (Option<String>, bool) {
    if room_id == "lobby" || room_id == "cafe" {
        return (None, false);
    }
    if !snapshot_owner_id.is_empty() {
        let mine = !local_user_id.is_empty() && snapshot_owner_id == local_user_id;
        return (Some(snapshot_owner_id.to_string()), mine);
    }
    (None, my_rooms.iter().any(|r| r.room_id == room_id))
}

#[derive(Resource, Default, Debug)]
pub struct ActiveRoomMetadata {
    pub room_id: String,
    pub room_name: String,
    pub owner_id: Option<String>,
    pub is_owner: bool,
    pub visibility: String,
}

// UI Marker Components
#[derive(Component)]
pub struct HudRootNode;

/// Present when the Compose shell draws the HUD: the bevy_hui HUD is not spawned then.
#[derive(Resource)]
pub struct ComposeHud;

#[derive(Component)]
pub struct HudRoomTitleText;

#[derive(Component)]
pub struct HudNavigatorBtn;

#[derive(Component)]
pub struct HudEditRoomBtn;

#[derive(Component)]
pub struct HudAppearanceBtn;

#[derive(Component)]
pub struct NavigatorOverlayRoot;

#[derive(Component)]
pub struct SettingsOverlayRoot;

#[derive(Component)]
pub struct DoorbellPopupRoot;

#[derive(Component)]
pub struct TabBtn(pub NavigatorTab);

#[derive(Component)]
pub struct JoinRoomBtn(pub String);

#[derive(Component)]
pub struct SelectLayoutBtn(pub String);

#[derive(Component)]
pub struct SettingsRoomNameInputBtn;

#[derive(Component)]
pub struct SaveSettingsBtn;

#[derive(Component)]
pub struct DeleteRoomBtn;

#[derive(Component)]
pub struct ConfirmDeleteBtn;

#[derive(Component)]
pub struct CancelDeleteBtn;

#[derive(Component)]
pub struct CloseSettingsBtn;

#[derive(Component)]
pub struct VisibilitySelectBtn(pub String);

pub struct RoomNavigatorPlugin;

impl Plugin for RoomNavigatorPlugin {
    fn build(&self, app: &mut App) {
        crate::ensure_hui_plugins(app);
        app.add_plugins(crate::navigator_ui::NavigatorUiPlugin)
            .init_resource::<RoomNavigatorState>()
            .init_resource::<RoomSettingsState>()
            .init_resource::<DoorbellState>()
            .init_resource::<ActiveRoomMetadata>()
            .init_resource::<ActiveInputFocus>()
            .add_systems(Startup, setup_doorbell_ui)
            .add_systems(OnEnter(AppState::InRoom), setup_hud)
            .add_systems(OnExit(AppState::InRoom), cleanup_all_ui)
            .add_systems(
                Update,
                (
                    handle_hud_buttons,
                    handle_hud_appearance_btn,
                    hud_button_hover,
                    handle_settings_interactions,
                    update_hud_display,
                    render_settings_overlay,
                    render_doorbell_popup,
                    handle_keyboard_and_text_input,
                )
                    .run_if(in_state(AppState::InRoom)),
            );
    }
}

// ---------------------------------------------------------------------------
// HUD Setup & Systems
// ---------------------------------------------------------------------------

pub(crate) const HUD_BTN_SIZE: f32 = 52.0 * HUD_SIZE_FACTOR;
const HUD_BTN_ICON: f32 = 30.0 * HUD_SIZE_FACTOR;

/// Colours of an icon-only HUD button; `hud_button_hover` swaps between them.
#[derive(Component, Clone, Copy)]
pub struct HudButtonColors {
    base: Color,
    hover: Color,
}

fn spawn_hud_icon_button(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    marker: impl Component,
    icon: &str,
    base: Color,
    hover: Color,
    display: Display,
) {
    parent
        .spawn((
            marker,
            crate::LegacyButton,
            HudButtonColors { base, hover },
            Node {
                width: Val::Px(HUD_BTN_SIZE),
                height: Val::Px(HUD_BTN_SIZE),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(12.0)),
                display,
                ..default()
            },
            BackgroundColor(base),
        ))
        .with_children(|btn| {
            btn.spawn((
                ImageNode::new(asset_server.load(icon.to_string())).with_color(Color::WHITE),
                Node {
                    width: Val::Px(HUD_BTN_ICON),
                    height: Val::Px(HUD_BTN_ICON),
                    ..default()
                },
            ));
        });
}

fn hud_corner(top: bool) -> (Node, HudEdge) {
    (
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(HUD_EDGE_MARGIN),
            top: if top {
                Val::Px(HUD_EDGE_MARGIN)
            } else {
                Val::Auto
            },
            bottom: if top {
                Val::Auto
            } else {
                Val::Px(HUD_EDGE_MARGIN)
            },
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        },
        HudEdge {
            top,
            right: true,
            bottom: !top,
            left: false,
        },
    )
}

/// HUD: transparent, content-sized groups in the right corners (no full-width bar).
/// Top right: balances, shop and appearance. Bottom right: build tools, inventory and navigator.
fn setup_hud(
    mut commands: Commands,
    active_meta: Res<ActiveRoomMetadata>,
    ui_font: Res<UiFont>,
    asset_server: Res<AssetServer>,
    compose_hud: Option<Res<ComposeHud>>,
) {
    if compose_hud.is_some() {
        return; // The Compose shell draws the HUD.
    }
    let owner_display = if active_meta.is_owner {
        Display::Flex
    } else {
        Display::None
    };

    // Top right
    commands
        .spawn((HudRootNode, hud_corner(true)))
        .with_children(|group| {
            crate::wallet::spawn_wallet_chips(group, &asset_server, &ui_font);
            spawn_hud_icon_button(
                group,
                &asset_server,
                crate::shop::HudShopBtn,
                "ui/icons/storefront.png",
                Color::srgb(0.72, 0.52, 0.08),
                Color::srgb(0.85, 0.63, 0.14),
                Display::Flex,
            );
            spawn_hud_icon_button(
                group,
                &asset_server,
                HudAppearanceBtn,
                "ui/icons/user.png",
                Color::srgb(0.35, 0.25, 0.65),
                Color::srgb(0.45, 0.35, 0.75),
                Display::Flex,
            );
        });

    // Bottom right
    commands
        .spawn((HudRootNode, hud_corner(false)))
        .with_children(|group| {
            spawn_hud_icon_button(
                group,
                &asset_server,
                crate::furniture::HudBuildModeBtn,
                "ui/icons/hammer.png",
                Color::srgb(0.70, 0.45, 0.20),
                Color::srgb(0.82, 0.56, 0.28),
                owner_display,
            );
            spawn_hud_icon_button(
                group,
                &asset_server,
                HudEditRoomBtn,
                "ui/icons/gear.png",
                Color::srgb(0.30, 0.35, 0.45),
                Color::srgb(0.40, 0.45, 0.58),
                owner_display,
            );
            spawn_hud_icon_button(
                group,
                &asset_server,
                crate::furniture::HudInventoryBtn,
                "ui/icons/backpack.png",
                Color::srgb(0.20, 0.55, 0.45),
                Color::srgb(0.28, 0.66, 0.55),
                Display::Flex,
            );
            spawn_hud_icon_button(
                group,
                &asset_server,
                HudNavigatorBtn,
                "ui/icons/compass.png",
                Color::srgb(0.20, 0.35, 0.65),
                Color::srgb(0.28, 0.45, 0.80),
                Display::Flex,
            );
        });

    // Top left: current room title (text only, no background)
    commands.spawn((
        HudRootNode,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(HUD_EDGE_MARGIN),
            left: Val::Px(HUD_EDGE_MARGIN),
            ..default()
        },
        HudEdge {
            top: true,
            left: true,
            ..default()
        },
        children![(
            HudRoomTitleText,
            Text::new("Sala Atual"),
            ui_font.text_font(16.0),
            TextColor(Color::srgb(0.90, 0.94, 1.0)),
        )],
    ));
}

fn hud_button_hover(
    mut buttons: Query<
        (&Interaction, &HudButtonColors, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    for (interaction, colors, mut bg) in &mut buttons {
        *bg = BackgroundColor(match interaction {
            Interaction::None => colors.base,
            _ => colors.hover,
        });
    }
}

fn update_hud_display(
    active_meta: Res<ActiveRoomMetadata>,
    active_room: Res<ActiveRoom>,
    mut title_query: Query<&mut Text, With<HudRoomTitleText>>,
    mut edit_btn_query: Query<&mut Node, With<HudEditRoomBtn>>,
) {
    if active_meta.is_changed() || active_room.is_changed() {
        for mut text in title_query.iter_mut() {
            let vis_label = match active_meta.visibility.as_str() {
                "public" => "Pública",
                "hidden" => "Oculta",
                _ => "Privada",
            };
            let title = if active_room.name.is_empty() {
                "The Grand Lobby"
            } else {
                &active_room.name
            };
            text.0 = format!("{}  •  [{}]", title, vis_label);
        }

        for mut node in edit_btn_query.iter_mut() {
            node.display = if active_meta.is_owner {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn handle_hud_buttons(
    nav_btn_q: Query<&Interaction, (With<HudNavigatorBtn>, Changed<Interaction>)>,
    edit_btn_q: Query<&Interaction, (With<HudEditRoomBtn>, Changed<Interaction>)>,
    mut nav_state: ResMut<RoomNavigatorState>,
    mut settings_state: ResMut<RoomSettingsState>,
    mut focus: ResMut<ActiveInputFocus>,
    active_room: Res<ActiveRoom>,
    active_meta: Res<ActiveRoomMetadata>,
    net: Option<Res<NakamaResource>>,
) {
    for interaction in nav_btn_q.iter() {
        if *interaction == Interaction::Pressed {
            let open = !nav_state.is_open;
            set_navigator_open(
                open,
                &mut nav_state,
                &mut settings_state,
                &mut focus,
                net.as_deref(),
            );
        }
    }

    for interaction in edit_btn_q.iter() {
        if *interaction == Interaction::Pressed {
            toggle_room_settings(
                &mut settings_state,
                &mut nav_state,
                &mut focus,
                &active_room,
                &active_meta,
            );
        }
    }
}

/// Opens or closes the room navigator; opening closes the room settings and fetches the lists.
/// Shared by the bevy_hui HUD button and the Compose shell's `navigator_open`.
pub fn set_navigator_open(
    open: bool,
    nav_state: &mut RoomNavigatorState,
    settings_state: &mut RoomSettingsState,
    focus: &mut ActiveInputFocus,
    net: Option<&NakamaResource>,
) {
    nav_state.is_open = open;
    focus.0 = FocusedInput::None;
    if open {
        settings_state.is_open = false;
        nav_state.is_loading_popular = true;
        if let Some(net) = net {
            net.handle.send_command(NakamaCommand::FetchPopularRooms);
            net.handle.send_command(NakamaCommand::FetchMyRooms);
            net.handle.send_command(NakamaCommand::FetchRoomLayouts);
        }
    }
}

/// Toggles the room settings window (owner only for the Compose shell, which checks `is_owner`).
pub fn toggle_room_settings(
    settings_state: &mut RoomSettingsState,
    nav_state: &mut RoomNavigatorState,
    focus: &mut ActiveInputFocus,
    active_room: &ActiveRoom,
    active_meta: &ActiveRoomMetadata,
) {
    settings_state.is_open = !settings_state.is_open;
    if settings_state.is_open {
        nav_state.is_open = false;
        settings_state.edited_name = active_room.name.clone();
        settings_state.edited_visibility = active_meta.visibility.clone();
        settings_state.is_confirming_delete = false;
        focus.0 = FocusedInput::SettingsRoomName;
    } else {
        focus.0 = FocusedInput::None;
    }
}

fn handle_hud_appearance_btn(
    interaction_q: Query<&Interaction, (With<HudAppearanceBtn>, Changed<Interaction>)>,
    mut menu: ResMut<crate::avatar_menu::AvatarMenuState>,
) {
    for interaction in interaction_q.iter() {
        if *interaction == Interaction::Pressed {
            menu.is_open = !menu.is_open;
        }
    }
}

// ---------------------------------------------------------------------------
// Settings Overlay UI
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn render_settings_overlay(
    mut commands: Commands,
    settings_state: Res<RoomSettingsState>,
    existing: Query<Entity, With<SettingsOverlayRoot>>,
    ui_font: Res<UiFont>,
    focus: Res<ActiveInputFocus>,
    safe_area: Res<SafeArea>,
    ui_scale: Res<UiScale>,
    asset_server: Res<AssetServer>,
) {
    if !settings_state.is_changed()
        && !focus.is_changed()
        && !safe_area.is_changed()
        && !ui_scale.is_changed()
    {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    if !settings_state.is_open {
        return;
    }

    let insets = safe_area.layout_insets(ui_scale.0, BASE_MARGIN);
    // Same shell as the other modals: a backdrop that blocks the HUD below it, and a window as
    // wide as the screen (not as tall) whose content stays inside the safe area.
    commands
        .spawn((
            SettingsOverlayRoot,
            crate::LegacyButton,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.008, 0.024, 0.09, 0.7)),
            GlobalZIndex(50),
        ))
        .with_children(|backdrop| {
            backdrop
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        max_height: Val::Percent(92.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect {
                            left: Val::Px(insets.left),
                            right: Val::Px(insets.right),
                            top: Val::Px(BASE_MARGIN),
                            bottom: Val::Px(BASE_MARGIN),
                        },
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.059, 0.09, 0.165)),
                ))
        .with_children(|root| {
            // Header
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("CONFIGURAÇÕES DO ESPAÇO"),
                    ui_font.text_font(18.0),
                    TextColor(Color::srgb(0.95, 0.95, 1.0)),
                ));

                header
                    .spawn((
                        CloseSettingsBtn,
                        crate::LegacyButton,
                        Node {
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.35, 0.20, 0.25)),
                    ))
                    .with_children(|c| {
                        c.spawn((
                            ImageNode::new(asset_server.load("ui/icons/x.png"))
                                .with_color(Color::WHITE),
                            Node {
                                width: Val::Px(16.0),
                                height: Val::Px(16.0),
                                ..default()
                            },
                        ));
                    });
            });

            // Two columns so everything fits on a phone held sideways.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(24.0),
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                })
                .with_children(|root| {
            // Room Name Editing Input
            root.spawn((
                Text::new("Nome do Espaço (até 50 caracteres):"),
                ui_font.text_font(14.0),
                TextColor(Color::srgb(0.85, 0.88, 0.95)),
                Node {
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
            ));

            let is_name_focused = focus.0 == FocusedInput::SettingsRoomName;
            let border_color = if is_name_focused {
                Color::srgb(0.35, 0.65, 1.0)
            } else {
                Color::srgb(0.25, 0.28, 0.38)
            };
            let bg_color = if is_name_focused {
                Color::srgb(0.18, 0.22, 0.32)
            } else {
                Color::srgb(0.14, 0.16, 0.23)
            };

            root.spawn((
                SettingsRoomNameInputBtn,
                crate::LegacyButton,
                Node {
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
                BorderColor::all(border_color),
                BackgroundColor(bg_color),
            ))
            .with_children(|b| {
                let label = if settings_state.edited_name.is_empty() {
                    if is_name_focused {
                        "|"
                    } else {
                        "Digite o nome da sala..."
                    }
                } else if is_name_focused {
                    &format!("{}|", settings_state.edited_name)
                } else {
                    &settings_state.edited_name
                };
                b.spawn((
                    Text::new(label),
                    ui_font.text_font(14.0),
                    TextColor(if settings_state.edited_name.is_empty() && !is_name_focused {
                        Color::srgb(0.50, 0.55, 0.65)
                    } else {
                        Color::srgb(0.95, 0.98, 1.0)
                    }),
                ));
            });

            // Visibility Selector
            root.spawn((
                Text::new("Visibilidade de Acesso:"),
                ui_font.text_font(14.0),
                TextColor(Color::srgb(0.85, 0.88, 0.95)),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                flex_wrap: FlexWrap::Wrap,
                margin: UiRect::bottom(Val::Px(12.0)),
                ..default()
            })
            .with_children(|vis_row| {
                let options = [
                    ("public", "Pública (Qualquer um entra)"),
                    ("private", "Privada (Requer aprovação)"),
                    ("hidden", "Oculta (Invisível na busca)"),
                ];

                for (code, label) in options {
                    let is_sel = settings_state.edited_visibility == code;
                    let bg = if is_sel {
                        Color::srgb(0.20, 0.45, 0.75)
                    } else {
                        Color::srgb(0.16, 0.18, 0.25)
                    };

                    vis_row.spawn((
                        VisibilitySelectBtn(code.to_string()),
                        crate::LegacyButton,
                        Node {
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(bg),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(label),
                            ui_font.text_font(12.0),
                            TextColor(Color::srgb(0.95, 0.98, 1.0)),
                        ));
                    });
                }
            });

                });
                body.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                })
                .with_children(|root| {
            // Save Button
            root.spawn((
                SaveSettingsBtn,
                crate::LegacyButton,
                Node {
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    align_self: AlignSelf::FlexStart,
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.20, 0.55, 0.35)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new("Salvar Alterações"),
                    ui_font.text_font(14.0),
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
            });

            // Danger Zone: Delete Room
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            })
            .with_children(|danger| {
                if settings_state.is_confirming_delete {
                    danger.spawn((
                        Text::new("Tem certeza que deseja excluir esta sala? Você e todos os avatares retornarão ao Lobby."),
                        ui_font.text_font(13.0),
                        TextColor(Color::srgb(1.0, 0.70, 0.70)),
                        Node {
                            margin: UiRect::bottom(Val::Px(12.0)),
                            ..default()
                        },
                    ));

                    danger.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|actions| {
                        actions.spawn((
                            ConfirmDeleteBtn,
                            crate::LegacyButton,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.80, 0.20, 0.20)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Sim, Excluir"),
                                ui_font.text_font(13.0),
                                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            ));
                        });

                        actions.spawn((
                            CancelDeleteBtn,
                            crate::LegacyButton,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.30, 0.35, 0.45)),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("Cancelar"),
                                ui_font.text_font(13.0),
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                        });
                    });
                } else {
                    danger.spawn((
                        DeleteRoomBtn,
                        crate::LegacyButton,
                        Node {
                            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            align_self: AlignSelf::FlexStart,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.65, 0.20, 0.20)),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Excluir Espaço"),
                            ui_font.text_font(13.0),
                            TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        ));
                    });
                }
            });
                });
            });
        });
        });
} // ---------------------------------------------------------------------------
// Doorbell Popup UI
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct DoorbellTemplate(Handle<HtmlTemplate>);

fn setup_doorbell_ui(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut functions: HtmlFunctions,
) {
    commands.insert_resource(DoorbellTemplate(server.load("ui/doorbell.html")));
    functions.register(
        "doorbell_allow",
        |In(entity): In<Entity>,
         tags: Query<&Tags>,
         state: ResMut<DoorbellState>,
         net: Option<Res<NakamaResource>>| {
            respond_to_doorbell(entity, &tags, state, net, true)
        },
    );
    functions.register(
        "doorbell_deny",
        |In(entity): In<Entity>,
         tags: Query<&Tags>,
         state: ResMut<DoorbellState>,
         net: Option<Res<NakamaResource>>| {
            respond_to_doorbell(entity, &tags, state, net, false)
        },
    );
}

fn respond_to_doorbell(
    entity: Entity,
    tags: &Query<&Tags>,
    mut doorbell_state: ResMut<DoorbellState>,
    net: Option<Res<NakamaResource>>,
    allowed: bool,
) {
    let Some(request_id) = tags.get(entity).ok().and_then(|t| t.get("request_id")) else {
        return;
    };
    if let Some(net) = &net {
        net.handle
            .send_command(NakamaCommand::SendMessage(ClientMessage::DoorbellResponse(
                DoorbellResponse {
                    request_id: request_id.clone(),
                    allowed,
                },
            )));
    }
    doorbell_state.incoming_request = None;
}

fn render_doorbell_popup(
    mut commands: Commands,
    doorbell_state: Res<DoorbellState>,
    template: Option<Res<DoorbellTemplate>>,
    existing: Query<Entity, With<DoorbellPopupRoot>>,
) {
    let Some(template) = template else { return };
    if !doorbell_state.is_changed() && !template.is_added() {
        return;
    }

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    let Some(req) = &doorbell_state.incoming_request else {
        return;
    };

    commands.spawn((
        DoorbellPopupRoot,
        HtmlNode(template.0.clone()),
        TemplateProperties::default()
            .with("request_id", &req.request_id)
            .with("name", &req.requester_name),
        GlobalZIndex(60),
    ));
}

// ---------------------------------------------------------------------------
// Interaction Handlers
// ---------------------------------------------------------------------------

pub(crate) fn submit_create_room(
    nav_state: &mut RoomNavigatorState,
    focus: &mut ActiveInputFocus,
    net: Option<&NakamaResource>,
) {
    let name = if nav_state.create_name.trim().is_empty() {
        "Meu Espaço Virtual".to_string()
    } else {
        nav_state.create_name.trim().to_string()
    };

    let layout_id = nav_state
        .selected_layout_id
        .clone()
        .or_else(|| nav_state.layouts.first().map(|l| l.id.clone()))
        .unwrap_or_else(|| "small_6x6".to_string());

    if let Some(net) = net {
        info!(
            "Submitting create room: name={:?}, layout={:?}",
            name, layout_id
        );
        net.handle
            .send_command(NakamaCommand::CreateRoom { name, layout_id });
        nav_state.is_open = false;
        focus.0 = FocusedInput::None;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_settings_interactions(
    mut name_input_q: Query<&Interaction, (With<SettingsRoomNameInputBtn>, Changed<Interaction>)>,
    mut vis_q: Query<(&Interaction, &VisibilitySelectBtn), Changed<Interaction>>,
    mut save_q: Query<&Interaction, (With<SaveSettingsBtn>, Changed<Interaction>)>,
    mut del_q: Query<&Interaction, (With<DeleteRoomBtn>, Changed<Interaction>)>,
    mut confirm_q: Query<&Interaction, (With<ConfirmDeleteBtn>, Changed<Interaction>)>,
    mut cancel_q: Query<&Interaction, (With<CancelDeleteBtn>, Changed<Interaction>)>,
    mut close_q: Query<&Interaction, (With<CloseSettingsBtn>, Changed<Interaction>)>,
    mut settings_state: ResMut<RoomSettingsState>,
    mut focus: ResMut<ActiveInputFocus>,
    active_room: Res<ActiveRoom>,
    mut active_meta: ResMut<ActiveRoomMetadata>,
    mut nav_state: ResMut<RoomNavigatorState>,
    net: Option<Res<NakamaResource>>,
) {
    for interaction in name_input_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            focus.0 = FocusedInput::SettingsRoomName;
        }
    }

    for (interaction, btn) in vis_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            settings_state.edited_visibility = btn.0.clone();
        }
    }

    for interaction in close_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            settings_state.is_open = false;
            focus.0 = FocusedInput::None;
        }
    }

    for interaction in save_q.iter_mut() {
        if *interaction == Interaction::Pressed
            && let Some(net) = &net
        {
            let name = if settings_state.edited_name.trim().is_empty() {
                active_room.name.clone()
            } else {
                settings_state.edited_name.trim().to_string()
            };
            active_meta.visibility = settings_state.edited_visibility.clone();
            active_meta.room_name = name.clone();
            for r in &mut nav_state.my_rooms {
                if r.room_id == active_room.id {
                    r.name = name.clone();
                    r.visibility = settings_state.edited_visibility.clone();
                }
            }
            net.handle.send_command(NakamaCommand::UpdateRoom {
                room_id: active_room.id.clone(),
                name,
                visibility: settings_state.edited_visibility.clone(),
            });
            settings_state.is_open = false;
            focus.0 = FocusedInput::None;
        }
    }

    for interaction in del_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            settings_state.is_confirming_delete = true;
        }
    }

    for interaction in cancel_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            settings_state.is_confirming_delete = false;
        }
    }

    for interaction in confirm_q.iter_mut() {
        if *interaction == Interaction::Pressed
            && let Some(net) = &net
        {
            info!("Confirming room deletion for: {}", active_room.id);
            net.handle.send_command(NakamaCommand::DeleteRoom {
                room_id: active_room.id.clone(),
            });
            settings_state.is_open = false;
            settings_state.is_confirming_delete = false;
            focus.0 = FocusedInput::None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_keyboard_and_text_input(
    mut commands: Commands,
    mut keyboard_events: MessageReader<KeyboardInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<ActiveInputFocus>,
    mut nav_state: ResMut<RoomNavigatorState>,
    mut settings_state: ResMut<RoomSettingsState>,
    html_inputs: Query<Entity, (With<TextInput>, With<UiActive>)>,
    net: Option<Res<NakamaResource>>,
) {
    // The room name field of the settings window is edited here
    if focus.0 == FocusedInput::SettingsRoomName {
        for event in keyboard_events.read() {
            if !event.state.is_pressed() {
                continue;
            }
            match &event.logical_key {
                Key::Backspace => {
                    settings_state.edited_name.pop();
                }
                Key::Enter | Key::Escape => focus.0 = FocusedInput::None,
                Key::Space => {
                    if settings_state.edited_name.chars().count() < 50 {
                        settings_state.edited_name.push(' ');
                    }
                }
                Key::Character(text) => {
                    for c in text.chars().filter(|c| !c.is_control()) {
                        if settings_state.edited_name.chars().count() < 50 {
                            settings_state.edited_name.push(c);
                        }
                    }
                }
                _ => {}
            }
        }
        return;
    }

    // Drain keyboard events if not focused
    for _ in keyboard_events.read() {}

    // A focused HTML text field owns the keyboard: Esc only leaves it, other keys are typing.
    if !html_inputs.is_empty() {
        if keyboard.just_pressed(KeyCode::Escape) {
            for entity in &html_inputs {
                commands.entity(entity).remove::<UiActive>();
            }
        }
        return;
    }

    // Global navigation shortcuts (only active when no input field is focused)
    if keyboard.just_pressed(KeyCode::KeyN) {
        nav_state.is_open = !nav_state.is_open;
        if nav_state.is_open {
            settings_state.is_open = false;
            focus.0 = FocusedInput::None;
            if let Some(net) = &net {
                net.handle.send_command(NakamaCommand::FetchPopularRooms);
                net.handle.send_command(NakamaCommand::FetchMyRooms);
                net.handle.send_command(NakamaCommand::FetchRoomLayouts);
            }
        }
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        nav_state.is_open = false;
        settings_state.is_open = false;
        focus.0 = FocusedInput::None;
    }
}

fn cleanup_all_ui(
    mut commands: Commands,
    hud: Query<Entity, With<HudRootNode>>,
    nav: Query<Entity, With<NavigatorOverlayRoot>>,
    settings: Query<Entity, With<SettingsOverlayRoot>>,
    doorbell: Query<Entity, With<DoorbellPopupRoot>>,
) {
    for entity in hud.iter() {
        commands.entity(entity).despawn();
    }
    for entity in nav.iter() {
        commands.entity(entity).despawn();
    }
    for entity in settings.iter() {
        commands.entity(entity).despawn();
    }
    for entity in doorbell.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigator_tab_default() {
        let state = RoomNavigatorState::default();
        assert_eq!(state.current_tab, NavigatorTab::PublicSpaces);
        assert!(!state.is_open);
        assert!(state.selected_layout_id.is_none());
    }

    #[test]
    fn test_active_room_metadata_owner() {
        let mut meta = ActiveRoomMetadata::default();
        assert!(!meta.is_owner);

        meta.owner_id = Some("user-alice".to_string());
        meta.is_owner = true;
        meta.visibility = "private".to_string();

        assert!(meta.is_owner);
        assert_eq!(meta.visibility, "private");
    }

    #[test]
    fn test_room_settings_state_defaults() {
        let settings = RoomSettingsState::default();
        assert!(!settings.is_open);
        assert!(!settings.is_confirming_delete);
        assert_eq!(settings.edited_visibility, "");
    }

    #[test]
    fn test_doorbell_state_defaults() {
        let db = DoorbellState::default();
        assert!(db.incoming_request.is_none());
        assert!(!db.is_waiting_approval);
    }

    #[test]
    fn test_active_input_focus_defaults() {
        let focus = ActiveInputFocus::default();
        assert_eq!(focus.0, FocusedInput::None);

        let mut focus_edit = ActiveInputFocus(FocusedInput::SettingsRoomName);
        assert_eq!(focus_edit.0, FocusedInput::SettingsRoomName);
        focus_edit.0 = FocusedInput::None;
        assert_eq!(focus_edit.0, FocusedInput::None);
    }

    fn summary(id: &str) -> RoomSummaryDto {
        RoomSummaryDto {
            room_id: id.into(),
            name: id.into(),
            owner_id: None,
            owner_name: None,
            visibility: "public".into(),
            user_count: 0,
            max_users: 10,
            match_id: None,
        }
    }

    #[test]
    fn public_rooms_have_no_owner() {
        assert_eq!(resolve_ownership("lobby", "", "me", &[]), (None, false));
        assert_eq!(resolve_ownership("cafe", "", "me", &[]), (None, false));
    }

    #[test]
    fn the_snapshot_owner_decides_when_it_matches_my_user_id() {
        assert_eq!(
            resolve_ownership("r1", "me", "me", &[]),
            (Some("me".to_string()), true)
        );
    }

    #[test]
    fn someone_elses_room_is_not_mine_even_if_it_is_in_my_rooms() {
        // The snapshot wins over the lists.
        let mine = [summary("r2")];
        assert_eq!(
            resolve_ownership("r2", "alice", "me", &mine),
            (Some("alice".to_string()), false)
        );
    }

    #[test]
    fn an_empty_snapshot_owner_falls_back_to_my_rooms() {
        let mine = [summary("r3")];
        assert_eq!(resolve_ownership("r3", "", "me", &mine), (None, true));
        assert_eq!(resolve_ownership("r4", "", "me", &mine), (None, false));
    }

    #[test]
    fn an_empty_local_user_id_never_matches() {
        assert_eq!(
            resolve_ownership("r1", "someone", "", &[]),
            (Some("someone".to_string()), false)
        );
    }
}
