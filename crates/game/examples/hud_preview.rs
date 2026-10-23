#![allow(deprecated)]
//! Dev preview of the HUD and the doorbell popup (no server needed).
//!
//!   cargo run -p game --example hud_preview -- shot.png
//!
//! HUD_SETTINGS=1 opens the room settings window over the HUD. HUD_PROBE=1 puts the cursor on
//! the shop button and prints whether the button or the window above it receives the pointer.
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game::UiFont;
use game::furniture::PlayerInventoryState;
use game::navigator::{ActiveRoomMetadata, DoorbellState, RoomNavigatorPlugin, RoomSettingsState};
use game::net::{NetPurchaseSucceededEvent, NetWalletUpdatedEvent};
use game::room::ActiveRoom;
use game::state::AppState;
use game::wallet::{WalletPlugin, WalletState};
use protocol::{DoorbellRequest, WalletDto};

fn main() {
    let shot = std::env::args().nth(1);
    let mut app = App::new();
    app.add_plugins(game::preview::default_plugins())
        .add_plugins(game::preview::PhoneSimulation)
        .init_state::<AppState>()
        .init_resource::<UiFont>()
        .init_resource::<PlayerInventoryState>()
        .init_resource::<ActiveRoom>()
        .init_resource::<game::character_creator::CharacterCreatorState>()
        .add_message::<NetWalletUpdatedEvent>()
        .add_message::<NetPurchaseSucceededEvent>()
        .insert_resource(WalletState {
            balance: WalletDto {
                silver: 1250,
                gold: 3,
            },
            loaded: true,
        })
        .add_plugins((RoomNavigatorPlugin, WalletPlugin))
        .insert_resource(ActiveRoomMetadata {
            is_owner: true,
            visibility: "private".into(),
            ..default()
        })
        .insert_resource(DoorbellState {
            incoming_request: Some(DoorbellRequest {
                request_id: "r1".into(),
                requester_id: "u2".into(),
                requester_name: "Bob".into(),
            }),
            ..default()
        })
        .add_systems(
            Startup,
            |mut commands: Commands, mut next: ResMut<NextState<AppState>>| {
                commands.spawn(Camera2d);
                next.set(AppState::InRoom);
            },
        );

    if std::env::var("HUD_SETTINGS").is_ok() {
        app.insert_resource(RoomSettingsState {
            is_open: true,
            edited_name: "Meu espaço".into(),
            edited_visibility: "private".into(),
            ..default()
        });
    }
    if std::env::var("HUD_PROBE").is_ok() {
        app.add_systems(
            Update,
            |mut windows: Query<&mut Window>,
             shop: Query<
                (&Interaction, &ComputedNode, &UiGlobalTransform),
                With<game::shop::HudShopBtn>,
            >,
             mut frame: Local<u32>| {
                *frame += 1;
                let Ok((interaction, node, transform)) = shop.single() else {
                    return;
                };
                if *frame == 30 {
                    let center = transform.translation;
                    if let Ok(mut window) = windows.single_mut() {
                        window.set_physical_cursor_position(Some(center.as_dvec2()));
                    }
                    let _ = node;
                }
                if *frame == 60 {
                    println!("PROBE shop button interaction: {interaction:?}");
                }
            },
        );
    }

    if let Some(path) = shot {
        app.add_systems(
            Update,
            move |mut commands: Commands,
                  mut frame: Local<u32>,
                  mut exit: MessageWriter<AppExit>| {
                *frame += 1;
                if *frame == 90 {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(path.clone()));
                }
                if *frame == 120 {
                    exit.write(AppExit::Success);
                }
            },
        );
    }
    app.run();
}
