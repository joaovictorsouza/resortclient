//! Popup of the HUD avatar button: "Visuais" opens the avatar editor and "Sair" signs out.

use bevy::prelude::*;

use crate::UiFont;
use crate::character_creator::CharacterCreatorState;
use crate::config::GameConfig;
use crate::identity::sign_out;
use crate::navigator::HUD_BTN_SIZE;
use crate::net::NakamaResource;
use crate::safe_area::{HUD_EDGE_MARGIN, HudEdge, SafeArea};
use crate::state::AppState;
use crate::ui_scale::HUD_SIZE_FACTOR;

/// Whether the popup under the HUD avatar button is shown.
#[derive(Resource, Default, Debug)]
pub struct AvatarMenuState {
    pub is_open: bool,
}

#[derive(Component)]
struct AvatarMenuRoot;

#[derive(Component)]
struct AvatarMenuBackdrop;

#[derive(Component)]
struct VisualsBtn;

#[derive(Component)]
struct SignOutBtn;

const ROW_BG: Color = Color::srgb(0.12, 0.16, 0.26);
const ROW_BG_HOVER: Color = Color::srgb(0.2, 0.26, 0.4);
const SIGN_OUT_BG_HOVER: Color = Color::srgb(0.55, 0.18, 0.2);

pub struct AvatarMenuPlugin;

impl Plugin for AvatarMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AvatarMenuState>()
            .add_systems(OnExit(AppState::InRoom), close_menu)
            .add_systems(
                Update,
                (
                    render_menu,
                    // After the room click handler, which still sees the menu open on the press
                    // that closes it.
                    handle_menu_buttons.after(crate::input::handle_pointer_click_or_tap),
                    menu_hover,
                )
                    .run_if(in_state(AppState::InRoom)),
            );
    }
}

fn close_menu(
    mut commands: Commands,
    mut state: ResMut<AvatarMenuState>,
    roots: Query<Entity, With<AvatarMenuRoot>>,
) {
    state.is_open = false;
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn render_menu(
    mut commands: Commands,
    state: Res<AvatarMenuState>,
    existing: Query<Entity, With<AvatarMenuRoot>>,
    ui_font: Res<UiFont>,
    safe_area: Res<SafeArea>,
    ui_scale: Res<UiScale>,
) {
    if !state.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if !state.is_open {
        return;
    }

    let insets = safe_area.layout_insets(ui_scale.0, HUD_EDGE_MARGIN);
    // A transparent full-screen backdrop: a press outside the menu closes it and never reaches
    // the HUD buttons or the room below.
    commands
        .spawn((
            AvatarMenuRoot,
            AvatarMenuBackdrop,
            crate::LegacyButton,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(40),
        ))
        .with_children(|backdrop| {
            backdrop
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(insets.top + HUD_BTN_SIZE + 8.0),
                        right: Val::Px(insets.right),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0 * HUD_SIZE_FACTOR),
                        padding: UiRect::all(Val::Px(8.0 * HUD_SIZE_FACTOR)),
                        border_radius: BorderRadius::all(Val::Px(14.0)),
                        ..default()
                    },
                    HudEdge {
                        right: true,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.059, 0.09, 0.165)),
                ))
                .with_children(|menu| {
                    spawn_row(menu, &ui_font, VisualsBtn, "Visuais", ROW_BG_HOVER);
                    spawn_row(menu, &ui_font, SignOutBtn, "Sair", SIGN_OUT_BG_HOVER);
                });
        });
}

fn spawn_row(
    parent: &mut ChildSpawnerCommands,
    ui_font: &UiFont,
    marker: impl Component,
    label: &str,
    hover: Color,
) {
    parent
        .spawn((
            marker,
            crate::LegacyButton,
            RowColors { hover },
            Node {
                min_width: Val::Px(150.0 * HUD_SIZE_FACTOR),
                height: Val::Px(44.0 * HUD_SIZE_FACTOR),
                padding: UiRect::horizontal(Val::Px(16.0)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(ROW_BG),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                ui_font.text_font(18.0 * HUD_SIZE_FACTOR),
                TextColor(Color::WHITE),
            ));
        });
}

#[derive(Component)]
struct RowColors {
    hover: Color,
}

fn menu_hover(
    mut rows: Query<(&Interaction, &RowColors, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, colors, mut bg) in &mut rows {
        *bg = BackgroundColor(match interaction {
            Interaction::None => ROW_BG,
            _ => colors.hover,
        });
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn handle_menu_buttons(
    mut commands: Commands,
    mut state: ResMut<AvatarMenuState>,
    backdrop: Query<&Interaction, (With<AvatarMenuBackdrop>, Changed<Interaction>)>,
    visuals: Query<&Interaction, (With<VisualsBtn>, Changed<Interaction>)>,
    sign_out_btn: Query<&Interaction, (With<SignOutBtn>, Changed<Interaction>)>,
    mut creator_state: ResMut<CharacterCreatorState>,
    mut config: ResMut<GameConfig>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if visuals.iter().any(|i| *i == Interaction::Pressed) {
        state.is_open = false;
        creator_state.return_to_room = true;
        next_state.set(AppState::CharacterCreator);
    } else if sign_out_btn.iter().any(|i| *i == Interaction::Pressed) {
        state.is_open = false;
        if let Some(refresh_token) = config.refresh_token.clone() {
            nakama_client::revoke_session(
                config.host.clone(),
                config.port,
                config.server_key.clone(),
                refresh_token,
            );
        }
        sign_out(&mut config);
        // Dropping the handle stops the network worker and its connection.
        commands.remove_resource::<NakamaResource>();
        next_state.set(AppState::Login);
    } else if backdrop.iter().any(|i| *i == Interaction::Pressed) {
        state.is_open = false;
    }
}
