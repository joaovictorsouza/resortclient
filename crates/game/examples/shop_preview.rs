//! Dev preview of the shop window with fake data (no server needed).
//!
//!   cargo run -p game --example shop_preview                       # interactive
//!   cargo run -p game --example shop_preview -- shot.png [busca]   # screenshot and exit
//!
//! SHOP_CATEGORY=tiles selects a sub-category; SHOP_STATUS=ok|err shows a feedback line.
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_hui_widgets::prelude::TextInput;
use game::UiFont;
use game::furniture::PlayerInventoryState;
use game::net::{NetPurchaseFailedEvent, NetPurchaseSucceededEvent, NetShopCatalogLoadedEvent};
use game::shop::{ShopPlugin, ShopState, ShopStatus};
use game::state::AppState;
use game::wallet::WalletState;
use protocol::{ShopCategoryDto, ShopOfferDto, WalletDto};

fn category(id: &str, name: &str, icon: &str, parent: &str) -> ShopCategoryDto {
    ShopCategoryDto {
        id: id.to_string(),
        name: name.to_string(),
        icon: icon.to_string(),
        parent: parent.to_string(),
    }
}

fn offer(id: &str, name: &str, cat: &str, silver: i64, gold: i64) -> ShopOfferDto {
    ShopOfferDto {
        catalog_id: id.to_string(),
        category_id: cat.to_string(),
        name: name.to_string(),
        width: 1,
        depth: 1,
        walkable: false,
        price_silver: silver,
        price_gold: gold,
    }
}

fn main() {
    let shot = std::env::args().nth(1);
    let search = std::env::args().nth(2);
    let selected = std::env::var("SHOP_CATEGORY").ok();
    let status = std::env::var("SHOP_STATUS").ok();

    let mut app = App::new();
    app.add_plugins(preview_plugins())
        .add_plugins(game::preview::PhoneSimulation)
        .init_state::<AppState>()
        .init_resource::<UiFont>()
        .init_resource::<PlayerInventoryState>()
        .add_message::<NetShopCatalogLoadedEvent>()
        .add_message::<NetPurchaseSucceededEvent>()
        .add_message::<NetPurchaseFailedEvent>()
        .insert_resource(WalletState {
            balance: WalletDto {
                silver: 120,
                gold: 3,
            },
            loaded: true,
        })
        .add_plugins(game::inventory::InventoryPlugin)
        .add_plugins(ShopPlugin)
        .insert_resource(ShopState {
            is_open: true,
            categories: vec![
                category("furniture", "Móveis", "armchair", ""),
                category("seats", "Assentos", "armchair", "furniture"),
                category("beds", "Camas", "bed", "furniture"),
                category("blocks", "Blocos e divisórias", "cube", "furniture"),
                category("floors", "Pisos", "rug", ""),
                category("tiles", "Pisos decorativos", "rug", "floors"),
                category("carpets", "Tapetes", "rug", "floors"),
            ],
            offers: vec![
                offer("seat_stool_1x1", "Banqueta Alta", "seats", 15, 0),
                offer("seat_bench_2x1", "Banco de Praça 2x1", "seats", 30, 0),
                offer(
                    "seat_armchair_brown_leather",
                    "Poltrona de Couro Marrom",
                    "seats",
                    0,
                    5,
                ),
                offer("bed_single_1x2", "Cama de Solteiro", "beds", 100, 2),
                offer("solid_red_block", "Bloco Sólido Carmesim", "blocks", 5, 0),
                offer("solid_blue_block", "Bloco Sólido Índigo", "blocks", 5, 0),
                offer(
                    "solid_barrier_2x1",
                    "Divisória de Carvalho 2x1",
                    "blocks",
                    10,
                    0,
                ),
                offer(
                    "floor_yellow_tile",
                    "Piso Decorativo Dourado",
                    "tiles",
                    3,
                    0,
                ),
                offer(
                    "carpet_green_tile",
                    "Tapete Verde Esmeralda 2x2",
                    "carpets",
                    0,
                    1,
                ),
            ],
            selected_category: Some(selected.unwrap_or_else(|| "furniture".to_string())),
            status: match status.as_deref() {
                Some("ok") => Some(ShopStatus::Success(
                    "Poltrona de Couro Marrom adicionado ao inventário!".to_string(),
                )),
                Some("err") => Some(ShopStatus::Error("Saldo insuficiente.".to_string())),
                _ => None,
            },
            ..default()
        })
        .add_systems(
            Startup,
            |mut commands: Commands, mut next: ResMut<NextState<AppState>>| {
                commands.spawn(Camera2d);
                next.set(AppState::InRoom);
            },
        );

    // SHOP_FROM_INVENTORY=1 starts with the inventory open and switches to the shop at frame 40,
    // like pressing "Loja" while the inventory is showing.
    if std::env::var("SHOP_FROM_INVENTORY").is_ok() {
        app.add_systems(
            Startup,
            |mut shop: ResMut<ShopState>, mut inv: ResMut<PlayerInventoryState>| {
                shop.is_open = false;
                inv.is_open = true;
            },
        )
        .add_systems(
            Update,
            |mut shop: ResMut<ShopState>,
             mut inv: ResMut<PlayerInventoryState>,
             mut frame: Local<u32>| {
                *frame += 1;
                if *frame == 40 {
                    shop.is_open = true;
                    inv.is_open = false;
                }
            },
        );
    }

    if let Some(text) = search {
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

    // SHOP_CLICKS=beds,seats,furniture presses those menu entries one after another (press at
    // frame 60+20n, release 3 frames later) to exercise repeated clicks inside the window.
    if let Ok(keys) = std::env::var("SHOP_CLICKS") {
        let keys: Vec<String> = keys.split(',').map(str::to_string).collect();
        app.add_systems(
            Update,
            move |tags: Query<(&bevy_hui::prelude::Tags, &UiGlobalTransform)>,
                  mut windows: Query<(Entity, &mut Window)>,
                  mut input: MessageWriter<bevy::input::mouse::MouseButtonInput>,
                  mut frame: Local<u32>| {
                *frame += 1;
                if *frame < 60 {
                    return;
                }
                let step = ((*frame - 60) / 20) as usize;
                let phase = (*frame - 60) % 20;
                let Some(key) = keys.get(step) else { return };
                let Ok((entity, mut window)) = windows.single_mut() else {
                    return;
                };
                let Some((_, transform)) = tags.iter().find(|(t, _)| t.get("key") == Some(key))
                else {
                    return;
                };
                if phase == 0 {
                    window.set_physical_cursor_position(Some(transform.translation.as_dvec2()));
                    println!("CLICK {key}");
                }
                let state = match phase {
                    2 => Some(bevy::input::ButtonState::Pressed),
                    5 => Some(bevy::input::ButtonState::Released),
                    _ => None,
                };
                if let Some(state) = state {
                    input.write(bevy::input::mouse::MouseButtonInput {
                        button: MouseButton::Left,
                        state,
                        window: entity,
                    });
                }
            },
        );
    }

    // SHOP_TOGGLE=1 closes and reopens the window a few times.
    if std::env::var("SHOP_TOGGLE").is_ok() {
        app.add_systems(
            Update,
            |mut shop: ResMut<ShopState>, mut frame: Local<u32>| {
                *frame += 1;
                if *frame > 50 && (*frame - 50).is_multiple_of(20) {
                    shop.is_open = !shop.is_open;
                    println!("TOGGLE -> {}", shop.is_open);
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
                if *frame == 200 {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(path.clone()));
                }
                if *frame == 230 {
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
