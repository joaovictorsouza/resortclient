//! Dev preview of the inventory window with fake data (no server needed).
//!
//!   cargo run -p game --example inventory_preview            # interactive
//!   cargo run -p game --example inventory_preview -- shot.png [search text]  # screenshot and exit
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_hui_widgets::prelude::TextInput;
use game::UiFont;
use game::furniture::PlayerInventoryState;
use game::inventory::InventoryPlugin;
use game::navigator::ActiveRoomMetadata;
use game::state::AppState;
use protocol::InventoryItemDto;

fn item(
    catalog_id: &str,
    name: &str,
    category: &str,
    w: i32,
    d: i32,
    qty: i32,
) -> InventoryItemDto {
    InventoryItemDto {
        id: format!("inst_{catalog_id}"),
        catalog_id: catalog_id.to_string(),
        name: name.to_string(),
        category: category.to_string(),
        quantity: qty,
        width: w,
        depth: d,
        walkable: category == "floor_decoration",
        color: "#ffffff".to_string(),
        asset_id: String::new(),
    }
}

fn main() {
    let shot = std::env::args().nth(1);
    let search = std::env::args().nth(2);
    let mut app = App::new();
    app.add_plugins(preview_plugins())
        .add_plugins(game::preview::PhoneSimulation)
        .init_state::<AppState>()
        .init_resource::<UiFont>()
        .insert_resource(ActiveRoomMetadata {
            is_owner: true,
            ..default()
        })
        .add_plugins(InventoryPlugin)
        .insert_resource(PlayerInventoryState {
            is_open: true,
            is_loading: false,
            selected_catalog_id: Some("seat_armchair_brown_leather".to_string()),
            items: vec![
                item(
                    "seat_armchair_brown_leather",
                    "Poltrona de Couro Marrom",
                    "collider",
                    1,
                    1,
                    3,
                ),
                item("seat_bench_2x1", "Banco de Praça 2x1", "collider", 2, 1, 1),
                item("seat_stool_1x1", "Banqueta Alta", "collider", 1, 1, 12),
                item("bed_single_1x2", "Cama de Solteiro", "collider", 1, 2, 2),
                item(
                    "solid_red_block",
                    "Bloco Sólido Carmesim",
                    "collider",
                    1,
                    1,
                    8,
                ),
                item(
                    "solid_blue_block",
                    "Bloco Sólido Índigo",
                    "collider",
                    1,
                    1,
                    8,
                ),
                item(
                    "solid_barrier_2x1",
                    "Divisória de Carvalho 2x1",
                    "collider",
                    2,
                    1,
                    4,
                ),
                item(
                    "floor_yellow_tile",
                    "Piso Decorativo Dourado",
                    "floor_decoration",
                    1,
                    1,
                    31,
                ),
                item(
                    "carpet_green_tile",
                    "Tapete Verde Esmeralda 2x2",
                    "floor_decoration",
                    2,
                    2,
                    1,
                ),
            ],
        })
        .add_systems(
            Startup,
            |mut commands: Commands, mut next: ResMut<NextState<AppState>>| {
                commands.spawn(Camera2d);
                next.set(AppState::InRoom);
            },
        );

    // INV_MANY=1 adds filler items (no thumbnails) to exercise the scrollbar;
    // INV_SCROLL=1 then scrolls to the bottom.
    if std::env::var("INV_MANY").is_ok() {
        app.add_systems(Startup, |mut inventory: ResMut<PlayerInventoryState>| {
            for n in 1..=18 {
                let category = if n % 3 == 0 {
                    "floor_decoration"
                } else {
                    "collider"
                };
                inventory.items.push(item(
                    &format!("extra_{n}"),
                    &format!("Item extra {n}"),
                    category,
                    1,
                    1,
                    n,
                ));
            }
        });
    }
    if std::env::var("INV_SCROLL").is_ok() {
        app.add_systems(
            Update,
            |mut scrolls: Query<(&bevy_hui::prelude::UiId, &mut ScrollPosition)>,
             mut frame: Local<u32>| {
                *frame += 1;
                if *frame == 60 {
                    for (id, mut scroll) in &mut scrolls {
                        if id.id() == "inv_scroll" {
                            scroll.y = 150.0;
                        }
                    }
                }
            },
        );
    }

    if let Some(text) = search {
        // Simulates typing: writes the widget's value, exercising the on_change -> filter path.
        app.add_systems(
            Update,
            move |mut inputs: Query<&mut TextInput>, mut frame: Local<u32>| {
                *frame += 1;
                if *frame == 40 {
                    for mut input in &mut inputs {
                        input.value = text.clone();
                    }
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

fn preview_plugins() -> impl PluginGroup {
    game::preview::default_plugins()
}
