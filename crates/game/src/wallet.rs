//! Player wallet: balances shown in the HUD, the heartbeat that earns silver and the gold sound
//! (spec 0008). Balances are server-authoritative; this module only mirrors them.

use std::time::Duration;

use bevy::prelude::*;
use nakama_client::NakamaCommand;
use protocol::WalletDto;

use crate::UiFont;
use crate::net::{NakamaResource, NetPurchaseSucceededEvent, NetWalletUpdatedEvent};
use crate::state::AppState;
use crate::ui_scale::HUD_SIZE_FACTOR;

/// How often the client reports presence; the server credits at most 120 s per beat.
pub const HEARTBEAT_SECONDS: u64 = 60;
pub const GOLD_SOUND_PATH: &str = "audio/gold_received.wav";
pub const COIN_ICON_PATH: &str = "ui/icons/coins.png";

pub const SILVER_TINT: Color = Color::srgb(0.80, 0.84, 0.89); // slate-300
pub const GOLD_TINT: Color = Color::srgb(0.98, 0.80, 0.08); // yellow-400

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Formats an amount with pt-BR thousands separators: 1234567 -> "1.234.567".
pub fn format_amount(amount: i64) -> String {
    let digits = amount.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    if amount < 0 {
        out.insert(0, '-');
    }
    out
}

/// The coin jingle plays only when gold arrives (never when it is spent).
pub fn should_play_gold_sound(delta_gold: i64) -> bool {
    delta_gold > 0
}

// ---------------------------------------------------------------------------
// State & components
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Default)]
pub struct WalletState {
    pub balance: WalletDto,
    /// False until the first server response, so the HUD does not flash a fake "0".
    pub loaded: bool,
}

#[derive(Component)]
pub struct HudSilverText;

#[derive(Component)]
pub struct HudGoldText;

#[derive(Resource)]
struct HeartbeatTimer(Timer);

// ---------------------------------------------------------------------------
// HUD builder (called from the navigator's HUD setup)
// ---------------------------------------------------------------------------

/// Spawns the two balance chips (silver, gold) as children of the HUD.
pub fn spawn_wallet_chips(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    font: &UiFont,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|chips| {
            spawn_chip(chips, asset_server, font, SILVER_TINT, HudSilverText);
            spawn_chip(chips, asset_server, font, GOLD_TINT, HudGoldText);
        });
}

fn spawn_chip(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    font: &UiFont,
    tint: Color,
    marker: impl Component,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                min_height: Val::Px(44.0 * HUD_SIZE_FACTOR),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(9999.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.09, 0.55)),
        ))
        .with_children(|chip| {
            chip.spawn((
                ImageNode::new(asset_server.load(COIN_ICON_PATH)).with_color(tint),
                Node {
                    width: Val::Px(26.0 * HUD_SIZE_FACTOR),
                    height: Val::Px(26.0 * HUD_SIZE_FACTOR),
                    ..default()
                },
            ));
            chip.spawn((
                marker,
                Text::new("-"),
                font.text_font(20.0 * HUD_SIZE_FACTOR),
                TextColor(Color::srgb(0.95, 0.97, 1.0)),
            ));
        });
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct WalletPlugin;

impl Plugin for WalletPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WalletState>()
            .insert_resource(HeartbeatTimer(Timer::new(
                Duration::from_secs(HEARTBEAT_SECONDS),
                TimerMode::Repeating,
            )))
            .add_systems(OnEnter(AppState::InRoom), request_wallet_on_enter)
            .add_systems(
                Update,
                (
                    send_heartbeat,
                    apply_wallet_updates,
                    apply_purchase_balance,
                    refresh_hud_wallet,
                )
                    .chain()
                    .run_if(in_state(AppState::InRoom)),
            );
    }
}

fn request_wallet_on_enter(net: Option<Res<NakamaResource>>, mut timer: ResMut<HeartbeatTimer>) {
    let Some(net) = net else { return };
    // The first heartbeat grants the daily bonus and returns the balance in one round trip.
    net.handle.send_command(NakamaCommand::WalletHeartbeat);
    timer.0.reset();
}

fn send_heartbeat(
    time: Res<Time>,
    mut timer: ResMut<HeartbeatTimer>,
    net: Option<Res<NakamaResource>>,
) {
    if timer.0.tick(time.delta()).just_finished()
        && let Some(net) = net
    {
        net.handle.send_command(NakamaCommand::WalletHeartbeat);
    }
}

fn apply_wallet_updates(
    mut commands: Commands,
    mut events: MessageReader<NetWalletUpdatedEvent>,
    mut state: ResMut<WalletState>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        state.balance = event.wallet;
        state.loaded = true;
        if event.delta_silver > 0 || event.delta_gold > 0 {
            info!(
                "Wallet +{} silver, +{} gold ({})",
                event.delta_silver, event.delta_gold, event.reason
            );
        }
        if should_play_gold_sound(event.delta_gold) {
            commands.spawn((
                AudioPlayer::new(asset_server.load(GOLD_SOUND_PATH)),
                PlaybackSettings::DESPAWN,
            ));
        }
    }
}

/// A purchase response carries the new balance, so the HUD updates without another request.
fn apply_purchase_balance(
    mut events: MessageReader<NetPurchaseSucceededEvent>,
    mut state: ResMut<WalletState>,
) {
    for event in events.read() {
        state.balance = WalletDto {
            silver: event.0.silver,
            gold: event.0.gold,
        };
        state.loaded = true;
    }
}

/// Either HUD balance label was just spawned.
type HudTextAdded = Or<(Added<HudSilverText>, Added<HudGoldText>)>;

fn refresh_hud_wallet(
    state: Res<WalletState>,
    mut silver: Query<&mut Text, (With<HudSilverText>, Without<HudGoldText>)>,
    mut gold: Query<&mut Text, (With<HudGoldText>, Without<HudSilverText>)>,
    added: Query<(), HudTextAdded>,
) {
    if !state.is_changed() && added.is_empty() {
        return;
    }
    let label = |amount: i64| {
        if state.loaded {
            format_amount(amount)
        } else {
            "-".to_string()
        }
    };
    for mut text in &mut silver {
        text.0 = label(state.balance.silver);
    }
    for mut text in &mut gold {
        text.0 = label(state.balance.gold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_amount_groups_thousands() {
        assert_eq!(format_amount(0), "0");
        assert_eq!(format_amount(999), "999");
        assert_eq!(format_amount(1000), "1.000");
        assert_eq!(format_amount(1_234_567), "1.234.567");
        assert_eq!(format_amount(-4500), "-4.500");
    }

    #[test]
    fn gold_sound_only_for_incoming_gold() {
        assert!(should_play_gold_sound(1));
        assert!(!should_play_gold_sound(0));
        assert!(!should_play_gold_sound(-5));
    }
}
