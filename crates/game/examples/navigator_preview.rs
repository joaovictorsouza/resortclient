//! Dev preview of the room navigator window with fake data (no server needed).
//!
//!   cargo run -p game --example navigator_preview                      # interactive
//!   cargo run -p game --example navigator_preview -- shot.png [tab]    # screenshot and exit
//!
//! The optional tab is one of: public, popular, mine, search, create.
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game::UiFont;
use game::character_creator::CharacterCreatorState;
use game::navigator::{ActiveRoomMetadata, NavigatorTab, RoomNavigatorPlugin, RoomNavigatorState};
use game::net::{NetPurchaseSucceededEvent, NetWalletUpdatedEvent};
use game::room::ActiveRoom;
use game::state::AppState;
use game::wallet::{WalletPlugin, WalletState};
use protocol::{RoomLayoutDto, RoomSummaryDto};

fn room(id: &str, name: &str, owner: &str, users: i32, vis: &str) -> RoomSummaryDto {
    RoomSummaryDto {
        room_id: id.into(),
        name: name.into(),
        owner_id: None,
        owner_name: Some(owner.into()),
        visibility: vis.into(),
        user_count: users,
        max_users: 20,
        match_id: None,
    }
}

fn layout(id: &str, name: &str, w: i32, h: i32) -> RoomLayoutDto {
    RoomLayoutDto {
        id: id.into(),
        name: name.into(),
        width: w,
        height: h,
        door: [0, 0],
        max_users: 20,
        image_asset: String::new(),
    }
}

fn main() {
    let shot = std::env::args().nth(1);
    let tab = std::env::args()
        .nth(2)
        .and_then(|k| NavigatorTab::from_key(&k))
        .unwrap_or_default();

    let mut app = App::new();
    app.add_plugins(preview_plugins())
        .add_plugins(game::preview::PhoneSimulation)
        .init_state::<AppState>()
        .init_resource::<UiFont>()
        .init_resource::<ActiveRoom>()
        .init_resource::<CharacterCreatorState>()
        .init_resource::<WalletState>()
        .init_resource::<ActiveRoomMetadata>()
        .add_message::<NetWalletUpdatedEvent>()
        .add_message::<NetPurchaseSucceededEvent>()
        .add_plugins((RoomNavigatorPlugin, WalletPlugin))
        .insert_resource(RoomNavigatorState {
            is_open: true,
            current_tab: tab,
            popular_rooms: vec![
                room("a", "Sala da Alice", "Alice", 7, "public"),
                room("b", "Cantinho do Bob", "Bob", 2, "private"),
            ],
            my_rooms: vec![room("c", "Meu Espaço", "Eu", 0, "hidden")],
            layouts: vec![
                layout("small_6x6", "Pequena", 6, 6),
                layout("medium_10x10", "Média", 10, 10),
                layout("l_shape", "Formato L", 12, 12),
            ],
            selected_layout_id: Some("medium_10x10".into()),
            search_query: "alice".into(),
            create_name: "Minha sala nova".into(),
            has_searched: true,
            search_results: vec![room("a", "Sala da Alice", "Alice", 7, "public")],
            ..default()
        })
        .add_systems(
            Startup,
            |mut commands: Commands, mut next: ResMut<NextState<AppState>>| {
                commands.spawn(Camera2d);
                next.set(AppState::InRoom);
            },
        );

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

fn preview_plugins() -> impl PluginGroup {
    game::preview::default_plugins()
}
