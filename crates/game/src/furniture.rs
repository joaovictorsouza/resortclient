#![allow(clippy::too_many_arguments, clippy::collapsible_if)]

use avian3d::prelude::Collider;
use bevy::prelude::*;
use nakama_client::NakamaCommand;
use protocol::{
    ClientMessage, InventoryItemDto, MoveItemRequest, PickupItemRequest, PlaceItemRequest,
    PlacedItemSnapshot, RotateItemRequest,
};

use crate::UiFont;
use crate::navigator::ActiveRoomMetadata;
use crate::net::{
    NakamaResource, NetBuildModeRejectedEvent, NetInventoryLoadedEvent, NetItemMovedEvent,
    NetItemPlacedEvent, NetItemRemovedEvent, NetItemRotatedEvent, NetRoomSnapshotEvent,
};
use crate::room::{FLOOR_TILE_TOP, Tile};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Pure Functions & Geometry Logic
// ---------------------------------------------------------------------------

/// Computes the rotated dimensions (width, depth) for an item.
pub fn rotated_dimensions(width: i32, depth: i32, rotation: i32) -> (i32, i32) {
    let rot = rotation.rem_euclid(4);
    if rot == 1 || rot == 3 {
        (depth, width)
    } else {
        (width, depth)
    }
}

/// Computes the list of occupied tiles (x, z) given an anchor and rotated dimensions.
pub fn calculate_occupied_tiles(
    anchor_x: i32,
    anchor_z: i32,
    width: i32,
    depth: i32,
) -> Vec<(i32, i32)> {
    let mut tiles = Vec::with_capacity((width * depth) as usize);
    for dx in 0..width {
        for dz in 0..depth {
            tiles.push((anchor_x + dx, anchor_z + dz));
        }
    }
    tiles
}

/// Computes the world center position and mesh dimensions for a furniture piece.
pub fn calculate_furniture_transform(
    x: i32,
    z: i32,
    width: i32,
    depth: i32,
    walkable: bool,
) -> (Vec3, Vec3) {
    let height = if walkable { 0.04 } else { 0.8 };
    let center_x = x as f32 + (width as f32) / 2.0;
    let center_z = z as f32 + (depth as f32) / 2.0;
    // Everything rests on top of the floor tiles, which are only a subfloor.
    let center_y = FLOOR_TILE_TOP + (height / 2.0);

    let position = Vec3::new(center_x, center_y, center_z);
    let size = Vec3::new(width as f32 * 0.94, height, depth as f32 * 0.94);
    (position, size)
}

/// A 3D model that replaces the placeholder cube of a catalog item (`asset_id`).
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureModel {
    /// Logical asset path (served from the CDN cache or the embedded assets).
    pub path: String,
    /// Uniform scale from model units to world units.
    pub scale: f32,
    /// Distance from the model origin down to its lowest point, in model units.
    pub bottom: f32,
}

/// Bounds of a furniture model announced by the CDN manifest.
#[derive(Debug, Clone, PartialEq)]
pub struct CdnModel {
    pub path: String,
    /// `[x, y, z]` extent in model units; the front faces +Z like the built-in models.
    pub size: [f32; 3],
    pub bottom: f32,
}

static CDN_MODELS: std::sync::RwLock<std::collections::BTreeMap<String, CdnModel>> =
    std::sync::RwLock::new(std::collections::BTreeMap::new());

/// Registers a model by its logical path (`furniture/<asset_id>.glb`); other paths are ignored.
pub fn register_cdn_model(path: &str, size: [f32; 3], bottom: f32) {
    let Some(asset_id) = path
        .strip_prefix("furniture/")
        .and_then(|p| p.strip_suffix(".glb"))
    else {
        return;
    };
    if size.iter().any(|v| !v.is_finite() || *v <= 0.0) || !bottom.is_finite() {
        return;
    }
    if let Ok(mut models) = CDN_MODELS.write() {
        models.insert(
            asset_id.to_string(),
            CdnModel {
                path: path.to_string(),
                size,
                bottom,
            },
        );
    }
}

/// Fraction of a tile a model may fill, same margin as the placeholder cube.
const MODEL_TILE_FILL: f32 = 0.94;

/// Model for a catalog `asset_id` placed on a `width` x `depth` footprint (tiles, at rotation 0).
/// Built-in models win; otherwise a model announced by the CDN manifest is fitted to the footprint.
/// `None` keeps the placeholder cube.
pub fn furniture_model(asset_id: &str, width: i32, depth: i32) -> Option<FurnitureModel> {
    if let Some(model) = builtin_model(asset_id) {
        return Some(model);
    }
    let models = CDN_MODELS.read().ok()?;
    let cdn = models.get(asset_id)?;
    let fit_x = width.max(1) as f32 * MODEL_TILE_FILL / cdn.size[0];
    let fit_z = depth.max(1) as f32 * MODEL_TILE_FILL / cdn.size[2];
    Some(FurnitureModel {
        path: cdn.path.clone(),
        scale: fit_x.min(fit_z),
        bottom: cdn.bottom,
    })
}

/// Footprint at rotation 0 from the (rotated) width and depth of a placed item.
pub fn base_footprint(width: i32, depth: i32, rotation: i32) -> (i32, i32) {
    if rotation.rem_euclid(2) == 1 {
        (depth, width)
    } else {
        (width, depth)
    }
}

fn builtin_model(asset_id: &str) -> Option<FurnitureModel> {
    match asset_id {
        // ~1.9 x 1.6 x 1.7 model units, origin at its center, front facing +Z.
        "armchair_brown_leather" => Some(FurnitureModel {
            path: "furniture/armchair_brown_leather.glb".to_string(),
            scale: 0.5,
            bottom: 0.798,
        }),
        _ => None,
    }
}

/// Local transform of a model relative to the furniture cube entity (centered on the
/// cube, whose height is `cube_height`): sits on the floor and turns by the item rotation
/// (0..3 quarter turns, same convention as avatar facing: rotation 0 faces +Z).
pub fn furniture_model_transform(
    model: &FurnitureModel,
    cube_height: f32,
    rotation: i32,
) -> Transform {
    let yaw = rotation.rem_euclid(4) as f32 * std::f32::consts::FRAC_PI_2;
    Transform::from_xyz(0.0, model.bottom * model.scale - cube_height / 2.0, 0.0)
        .with_rotation(Quat::from_rotation_y(yaw))
        .with_scale(Vec3::splat(model.scale))
}

/// Marks the child entity holding the 3D model of a placed furniture.
#[derive(Component)]
pub struct FurnitureModelRoot;

/// Parses a hex color string like "#eb4d4b" into a Bevy Color.
pub fn parse_hex_color(hex: &str) -> Color {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return Color::srgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        }
    }
    Color::srgb(0.9, 0.4, 0.4)
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum BuildTool {
    None,
    Placing {
        catalog_id: String,
        base_width: i32,
        base_depth: i32,
        walkable: bool,
        color: Color,
        rotation: i32,
    },
    Moving {
        item_id: String,
        catalog_id: String,
        base_width: i32,
        base_depth: i32,
        walkable: bool,
        color: Color,
        rotation: i32,
    },
}

#[derive(Resource, Debug)]
pub struct BuildModeState {
    pub is_active: bool,
    pub active_tool: BuildTool,
    pub hovered_tile: Option<(i32, i32)>,
    pub is_valid_tile: bool,
    pub selected_furniture_id: Option<String>,
    pub skip_first_click: bool,
}

impl Default for BuildModeState {
    fn default() -> Self {
        Self {
            is_active: false,
            active_tool: BuildTool::None,
            hovered_tile: None,
            is_valid_tile: false,
            selected_furniture_id: None,
            skip_first_click: false,
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct PlayerInventoryState {
    pub is_open: bool,
    pub is_loading: bool,
    pub items: Vec<InventoryItemDto>,
    pub selected_catalog_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component, Debug, Clone)]
pub struct PlacedFurniture {
    pub item_id: String,
    pub catalog_id: String,
    pub owner_id: String,
    pub x: i32,
    pub z: i32,
    pub rotation: i32,
    pub width: i32,
    pub depth: i32,
    pub walkable: bool,
    pub color_hex: String,
    pub asset_id: String,
}

#[derive(Component)]
pub struct GhostPreviewEntity;

// UI Components
#[derive(Component)]
pub struct HudInventoryBtn;

#[derive(Component)]
pub struct HudBuildModeBtn;

#[derive(Component)]
pub struct HudBuildModeText;

#[derive(Component)]
pub struct FurnitureContextMenuRoot;

#[derive(Component)]
pub struct ContextRotateBtn(pub String); // item_id

#[derive(Component)]
pub struct ContextMoveBtn(pub String); // item_id

#[derive(Component)]
pub struct ContextPickupBtn(pub String); // item_id

#[derive(Component)]
pub struct ContextCloseBtn;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct FurniturePlugin;

impl Plugin for FurniturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildModeState>()
            .init_resource::<PlayerInventoryState>()
            .add_systems(
                Update,
                (
                    handle_room_snapshot_furniture,
                    poll_network_furniture_events,
                    update_hud_build_controls,
                    handle_hud_mode_buttons,
                    handle_furniture_context_menu,
                    handle_ghost_preview,
                    handle_build_mode_keyboard_shortcuts,
                    handle_furniture_picking_in_build_mode,
                )
                    .run_if(in_state(AppState::InRoom)),
            );
    }
}

// ---------------------------------------------------------------------------
// Systems: Network & Lifecycle
// ---------------------------------------------------------------------------

fn handle_room_snapshot_furniture(
    mut commands: Commands,
    mut snapshot_events: MessageReader<NetRoomSnapshotEvent>,
    existing_furniture: Query<Entity, With<PlacedFurniture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for event in snapshot_events.read() {
        let snap = &event.0;
        info!(
            "Spawning {} initial room furniture items for room {}",
            snap.items.len(),
            snap.room_id
        );

        // Despawn any existing furniture from previous room
        for entity in existing_furniture.iter() {
            commands.entity(entity).despawn();
        }

        for item in &snap.items {
            spawn_furniture_entity(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                item,
            );
        }
    }
}

fn poll_network_furniture_events(
    mut commands: Commands,
    net: Option<Res<NakamaResource>>,
    mut inventory_state: ResMut<PlayerInventoryState>,
    mut build_state: ResMut<BuildModeState>,
    mut furniture_query: Query<(Entity, &mut PlacedFurniture, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    model_roots: Query<(), With<FurnitureModelRoot>>,
    children_query: Query<&Children>,
    mut placed_events: MessageReader<NetItemPlacedEvent>,
    mut moved_events: MessageReader<NetItemMovedEvent>,
    mut rotated_events: MessageReader<NetItemRotatedEvent>,
    mut removed_events: MessageReader<NetItemRemovedEvent>,
    mut rejected_events: MessageReader<NetBuildModeRejectedEvent>,
    mut inv_events: MessageReader<NetInventoryLoadedEvent>,
) {
    for event in inv_events.read() {
        info!("Player inventory loaded with {} item types", event.0.len());
        inventory_state.is_loading = false;
        inventory_state.items = event.0.clone();
        if inventory_state.selected_catalog_id.is_none() {
            if let Some(first) = inventory_state.items.first() {
                inventory_state.selected_catalog_id = Some(first.catalog_id.clone());
            }
        }
    }

    for event in placed_events.read() {
        let placed = &event.0;
        info!("Item placed broadcast received: {}", placed.id);
        // Decrement local count if owned
        if let Some(inv_item) = inventory_state
            .items
            .iter_mut()
            .find(|i| i.catalog_id == placed.catalog_id)
        {
            inv_item.quantity -= 1;
            if inv_item.quantity <= 0 {
                inventory_state
                    .items
                    .retain(|i| i.catalog_id != placed.catalog_id);
                if inventory_state.selected_catalog_id.as_deref() == Some(&placed.catalog_id) {
                    inventory_state.selected_catalog_id =
                        inventory_state.items.first().map(|i| i.catalog_id.clone());
                }
            }
        }
        spawn_furniture_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            &asset_server,
            placed,
        );
    }

    for event in moved_events.read() {
        let moved = &event.0;
        info!("Item moved broadcast received: {}", moved.id);
        for (_, mut furni, mut transform) in furniture_query.iter_mut() {
            if furni.item_id == moved.id {
                furni.x = moved.x;
                furni.z = moved.z;
                let (pos, _) = calculate_furniture_transform(
                    furni.x,
                    furni.z,
                    furni.width,
                    furni.depth,
                    furni.walkable,
                );
                transform.translation = pos;
                break;
            }
        }
    }

    for event in rotated_events.read() {
        let rot = &event.0;
        info!("Item rotated broadcast received: {}", rot.id);
        for (entity, mut furni, mut transform) in furniture_query.iter_mut() {
            if furni.item_id == rot.id {
                furni.rotation = rot.rotation;
                furni.width = rot.width;
                furni.depth = rot.depth;

                let (pos, size) = calculate_furniture_transform(
                    furni.x,
                    furni.z,
                    furni.width,
                    furni.depth,
                    furni.walkable,
                );
                transform.translation = pos;
                let mut has_model = false;
                if let Ok(children) = children_query.get(entity) {
                    for child in children.iter().filter(|c| model_roots.contains(*c)) {
                        has_model = true;
                        let (bw, bd) = base_footprint(furni.width, furni.depth, furni.rotation);
                        if let Some(model) = furniture_model(&furni.asset_id, bw, bd) {
                            commands.entity(child).insert(furniture_model_transform(
                                &model,
                                size.y,
                                furni.rotation,
                            ));
                        }
                    }
                }
                if !has_model {
                    commands
                        .entity(entity)
                        .insert(Mesh3d(meshes.add(Cuboid::from_size(size))));
                }
                break;
            }
        }
    }

    for event in removed_events.read() {
        let removed = &event.0;
        info!("Item removed broadcast received: {}", removed.id);
        for (entity, furni, _) in furniture_query.iter() {
            if furni.item_id == removed.id {
                commands.entity(entity).despawn();
                if build_state.selected_furniture_id.as_deref() == Some(&removed.id) {
                    build_state.selected_furniture_id = None;
                }
                if let Some(net) = &net {
                    net.handle.send_command(NakamaCommand::FetchInventory);
                }
                break;
            }
        }
    }

    for event in rejected_events.read() {
        warn!("Build mode action rejected by server: {}", event.0.reason);
    }
}

fn spawn_furniture_entity(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
    item: &PlacedItemSnapshot,
) {
    let (pos, size) =
        calculate_furniture_transform(item.x, item.z, item.width, item.depth, item.walkable);

    let color = parse_hex_color(&item.color);
    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.5,
        reflectance: 0.3,
        ..default()
    });

    let mut entity_cmds = commands.spawn((
        PlacedFurniture {
            item_id: item.id.clone(),
            catalog_id: item.catalog_id.clone(),
            owner_id: item.owner_id.clone(),
            x: item.x,
            z: item.z,
            rotation: item.rotation,
            width: item.width,
            depth: item.depth,
            walkable: item.walkable,
            color_hex: item.color.clone(),
            asset_id: item.asset_id.clone(),
        },
        Transform::from_translation(pos),
        Visibility::default(),
    ));

    let (base_w, base_d) = base_footprint(item.width, item.depth, item.rotation);
    if let Some(model) = furniture_model(&item.asset_id, base_w, base_d) {
        // The cube stays invisible (it only provides the picking collider).
        let model_transform = furniture_model_transform(&model, size.y, item.rotation);
        entity_cmds.with_children(|parent| {
            parent.spawn((
                FurnitureModelRoot,
                WorldAssetRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(model.path.clone())),
                ),
                model_transform,
                Visibility::default(),
            ));
        });
    } else {
        entity_cmds.insert((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material),
        ));
    }

    // Solid colliders receive physics bounding box for picking and spatial queries
    if !item.walkable {
        entity_cmds.insert(Collider::cuboid(size.x, size.y, size.z));
    }
}

// ---------------------------------------------------------------------------
// Systems: Ghost Preview & Snapping
// ---------------------------------------------------------------------------

/// Pointer for build mode: the mouse cursor, or the finger on a touch screen.
/// Returns where it points and whether this frame confirms (mouse press, finger lifted).
fn build_pointer(
    window: &Window,
    mouse_button: &ButtonInput<MouseButton>,
    touches: &Touches,
) -> (Option<Vec2>, bool) {
    if let Some(touch) = touches.iter().next() {
        return (Some(touch.position()), false);
    }
    if let Some(touch) = touches.iter_just_released().next() {
        return (Some(touch.position()), true);
    }
    (
        window.cursor_position(),
        mouse_button.just_pressed(MouseButton::Left),
    )
}

fn handle_ghost_preview(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::camera::MainRoomCamera>>,
    tiles_query: Query<&Tile>,
    furniture_query: Query<&PlacedFurniture>,
    mut build_state: ResMut<BuildModeState>,
    mut ghost_query: Query<
        (Entity, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        With<GhostPreviewEntity>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ui_interaction: Query<&Interaction, With<Node>>,
    net: Option<Res<NakamaResource>>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };

    let (active_tool, rotation, width, depth, walkable, _base_color) =
        match &build_state.active_tool {
            BuildTool::Placing {
                base_width,
                base_depth,
                walkable,
                color,
                rotation,
                ..
            } => {
                let (w, d) = rotated_dimensions(*base_width, *base_depth, *rotation);
                (true, *rotation, w, d, *walkable, *color)
            }
            BuildTool::Moving {
                base_width,
                base_depth,
                walkable,
                color,
                rotation,
                ..
            } => {
                let (w, d) = rotated_dimensions(*base_width, *base_depth, *rotation);
                (true, *rotation, w, d, *walkable, *color)
            }
            BuildTool::None => (false, 0, 1, 1, false, Color::WHITE),
        };

    if !active_tool || !build_state.is_active {
        // Despawn ghost if not active
        for (entity, _, _) in ghost_query.iter() {
            commands.entity(entity).despawn();
        }
        build_state.hovered_tile = None;
        return;
    }

    // Release latch from UI button click once the pointer is up. This runs before the pointer is read:
    // on a touch screen there is no pointer while no finger is down, and the latch would never clear.
    if build_state.skip_first_click
        && !mouse_button.pressed(MouseButton::Left)
        && touches.iter().next().is_none()
        && touches.iter_just_released().next().is_none()
    {
        build_state.skip_first_click = false;
    }

    let (pointer, confirmed) = build_pointer(window, &mouse_button, &touches);
    let Some(cursor_pos) = pointer else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    // Intersect the walking surface plane
    let plane_y = FLOOR_TILE_TOP;
    let normal = Vec3::Y;
    let denom = normal.dot(*ray.direction);

    if denom.abs() < 1e-6 {
        return;
    }

    let t = (plane_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return;
    }

    let hit = ray.origin + *ray.direction * t;
    let anchor_x = hit.x.floor() as i32;
    let anchor_z = hit.z.floor() as i32;

    build_state.hovered_tile = Some((anchor_x, anchor_z));

    // Validate placement
    let occupied = calculate_occupied_tiles(anchor_x, anchor_z, width, depth);
    let mut is_valid = true;

    let moving_item_id = if let BuildTool::Moving { item_id, .. } = &build_state.active_tool {
        Some(item_id.as_str())
    } else {
        None
    };

    for &(tx, tz) in &occupied {
        // Must be on a walkable floor tile
        let tile_exists = tiles_query.iter().any(|tile| tile.x == tx && tile.z == tz);
        if !tile_exists {
            is_valid = false;
            break;
        }

        // Solid items cannot overlap another solid item
        if !walkable {
            let overlaps_solid = furniture_query.iter().any(|f| {
                if Some(f.item_id.as_str()) == moving_item_id || f.walkable {
                    return false;
                }
                tx >= f.x && tx < f.x + f.width && tz >= f.z && tz < f.z + f.depth
            });

            if overlaps_solid {
                is_valid = false;
                break;
            }
        }
    }

    build_state.is_valid_tile = is_valid;

    let (world_pos, size) =
        calculate_furniture_transform(anchor_x, anchor_z, width, depth, walkable);

    let ghost_color = if is_valid {
        Color::srgba(0.2, 0.8, 0.4, 0.5) // Translucent green/teal
    } else {
        Color::srgba(0.9, 0.2, 0.2, 0.5) // Translucent warning red
    };

    let mut ghost_iter = ghost_query.iter_mut();
    if let Some((entity, mut transform, mat_handle)) = ghost_iter.next() {
        transform.translation = world_pos;
        commands
            .entity(entity)
            .insert(Mesh3d(meshes.add(Cuboid::from_size(size))));
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = ghost_color;
        }
    } else {
        let mat = materials.add(StandardMaterial {
            base_color: ghost_color,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        commands.spawn((
            GhostPreviewEntity,
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(mat),
            Transform::from_translation(world_pos),
        ));
    }

    let is_over_ui = ui_interaction
        .iter()
        .any(|i| *i == Interaction::Hovered || *i == Interaction::Pressed);

    // Place or Move on click (only after release latch and when not clicking on UI)
    if !build_state.skip_first_click && !is_over_ui && confirmed && is_valid {
        if let Some(net) = net {
            match &build_state.active_tool {
                BuildTool::Placing { catalog_id, .. } => {
                    info!(
                        "Sending PLACE_ITEM_REQUEST for {} at ({}, {})",
                        catalog_id, anchor_x, anchor_z
                    );
                    net.handle
                        .send_command(NakamaCommand::SendMessage(ClientMessage::PlaceItem(
                            PlaceItemRequest {
                                catalog_id: catalog_id.clone(),
                                x: anchor_x,
                                z: anchor_z,
                                rotation,
                            },
                        )));
                }
                BuildTool::Moving { item_id, .. } => {
                    info!(
                        "Sending MOVE_ITEM_REQUEST for {} to ({}, {})",
                        item_id, anchor_x, anchor_z
                    );
                    net.handle
                        .send_command(NakamaCommand::SendMessage(ClientMessage::MoveItem(
                            MoveItemRequest {
                                item_id: item_id.clone(),
                                x: anchor_x,
                                z: anchor_z,
                            },
                        )));
                }
                BuildTool::None => {}
            }
        }

        // Reset tool but stay in build mode
        build_state.active_tool = BuildTool::None;
        for (entity, _, _) in ghost_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Systems: Keyboard & Picking
// ---------------------------------------------------------------------------

fn handle_build_mode_keyboard_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut build_state: ResMut<BuildModeState>,
) {
    if !build_state.is_active {
        return;
    }

    // Key R rotates the active ghost preview
    if keys.just_pressed(KeyCode::KeyR) {
        match &mut build_state.active_tool {
            BuildTool::Placing { rotation, .. } => {
                *rotation = (*rotation + 1) % 4;
            }
            BuildTool::Moving { rotation, .. } => {
                *rotation = (*rotation + 1) % 4;
            }
            BuildTool::None => {}
        }
    }

    // Escape cancels current placement/moving tool or closes context menu
    if keys.just_pressed(KeyCode::Escape) {
        if build_state.active_tool != BuildTool::None {
            build_state.active_tool = BuildTool::None;
        } else if build_state.selected_furniture_id.is_some() {
            build_state.selected_furniture_id = None;
        }
    }
}

fn handle_furniture_picking_in_build_mode(
    mouse_button: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::camera::MainRoomCamera>>,
    furniture_query: Query<&PlacedFurniture>,
    mut build_state: ResMut<BuildModeState>,
) {
    if !build_state.is_active || build_state.active_tool != BuildTool::None {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let (pointer, confirmed) = build_pointer(window, &mouse_button, &touches);
    if !confirmed {
        return;
    }
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    let Some(cursor_pos) = pointer else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    let plane_y = FLOOR_TILE_TOP;
    let normal = Vec3::Y;
    let denom = normal.dot(*ray.direction);
    if denom.abs() < 1e-6 {
        return;
    }

    let t = (plane_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return;
    }

    let hit = ray.origin + *ray.direction * t;
    let (click_x, click_z) = (hit.x.floor() as i32, hit.z.floor() as i32);

    // Check if clicked on any placed furniture
    let clicked_furniture = furniture_query.iter().find(|f| {
        click_x >= f.x && click_x < f.x + f.width && click_z >= f.z && click_z < f.z + f.depth
    });

    if let Some(furni) = clicked_furniture {
        info!(
            "Clicked furniture {} in build mode, opening context menu",
            furni.item_id
        );
        build_state.selected_furniture_id = Some(furni.item_id.clone());
    } else {
        // Clicked outside, dismiss context menu
        build_state.selected_furniture_id = None;
    }
}

// ---------------------------------------------------------------------------
// Systems: HUD & UI (Buttons, Modals, Menus)
// ---------------------------------------------------------------------------

fn update_hud_build_controls(
    mut build_btn_text: Query<&mut Text, With<HudBuildModeText>>,
    active_meta: Res<ActiveRoomMetadata>,
    build_state: Res<BuildModeState>,
    mut build_btn_node: Query<&mut Node, With<HudBuildModeBtn>>,
) {
    // Show/hide build mode button based on ownership
    for mut node in build_btn_node.iter_mut() {
        node.display = if active_meta.is_owner {
            Display::Flex
        } else {
            Display::None
        };
    }

    // Update text
    for mut text in build_btn_text.iter_mut() {
        if build_state.is_active {
            text.0 = "🔨 Sair da Construção".to_string();
        } else {
            text.0 = "🔨 Modo Construção".to_string();
        }
    }
}

fn handle_hud_mode_buttons(
    build_btn_query: Query<&Interaction, (Changed<Interaction>, With<HudBuildModeBtn>)>,
    mut build_state: ResMut<BuildModeState>,
) {
    // 2. Toggle Build Mode HUD Button
    for interaction in build_btn_query.iter() {
        if *interaction == Interaction::Pressed {
            build_state.is_active = !build_state.is_active;
            info!("Build mode toggled: {}", build_state.is_active);
            if !build_state.is_active {
                build_state.active_tool = BuildTool::None;
                build_state.selected_furniture_id = None;
            }
        }
    }
}

fn handle_furniture_context_menu(
    mut commands: Commands,
    rotate_btn_query: Query<(&Interaction, &ContextRotateBtn), Changed<Interaction>>,
    move_btn_query: Query<(&Interaction, &ContextMoveBtn), Changed<Interaction>>,
    pickup_btn_query: Query<(&Interaction, &ContextPickupBtn), Changed<Interaction>>,
    context_close_query: Query<&Interaction, (Changed<Interaction>, With<ContextCloseBtn>)>,
    furniture_query: Query<&PlacedFurniture>,
    mut build_state: ResMut<BuildModeState>,
    net: Option<Res<NakamaResource>>,
    ui_font: Res<UiFont>,
    context_query: Query<Entity, With<FurnitureContextMenuRoot>>,
) {
    // 4. Context Menu: Rotate
    for (interaction, btn) in rotate_btn_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(furni) = furniture_query.iter().find(|f| f.item_id == btn.0) {
                let next_rot = (furni.rotation + 1) % 4;
                info!("Rotating item {} to {}", btn.0, next_rot);
                if let Some(net) = &net {
                    net.handle
                        .send_command(NakamaCommand::SendMessage(ClientMessage::RotateItem(
                            RotateItemRequest {
                                item_id: btn.0.clone(),
                                rotation: next_rot,
                            },
                        )));
                }
            }
        }
    }

    // 5. Context Menu: Move
    for (interaction, btn) in move_btn_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(furni) = furniture_query.iter().find(|f| f.item_id == btn.0) {
                info!("Initiating move for item {}", btn.0);
                build_state.skip_first_click = true;
                build_state.active_tool = BuildTool::Moving {
                    item_id: furni.item_id.clone(),
                    catalog_id: furni.catalog_id.clone(),
                    base_width: if furni.rotation % 2 == 1 {
                        furni.depth
                    } else {
                        furni.width
                    },
                    base_depth: if furni.rotation % 2 == 1 {
                        furni.width
                    } else {
                        furni.depth
                    },
                    walkable: furni.walkable,
                    color: parse_hex_color(&furni.color_hex),
                    rotation: furni.rotation,
                };
                build_state.selected_furniture_id = None;
            }
        }
    }

    // 6. Context Menu: Pickup
    for (interaction, btn) in pickup_btn_query.iter() {
        if *interaction == Interaction::Pressed {
            info!("Sending pickup request for item {}", btn.0);
            if let Some(net) = &net {
                net.handle
                    .send_command(NakamaCommand::SendMessage(ClientMessage::PickupItem(
                        PickupItemRequest {
                            item_id: btn.0.clone(),
                        },
                    )));
            }
            build_state.selected_furniture_id = None;
        }
    }

    // 7. Context Menu: Dismiss
    for interaction in context_close_query.iter() {
        if *interaction == Interaction::Pressed {
            build_state.selected_furniture_id = None;
        }
    }

    // Synchronize Context Menu Visibility
    if build_state.is_changed() {
        for entity in context_query.iter() {
            commands.entity(entity).despawn();
        }
        if let Some(ref sel_id) = build_state.selected_furniture_id {
            if let Some(furni) = furniture_query.iter().find(|f| &f.item_id == sel_id) {
                spawn_furniture_context_menu(&mut commands, furni, &ui_font);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// UI Rendering: Inventory Modal & Context Menu
// ---------------------------------------------------------------------------

fn spawn_furniture_context_menu(
    commands: &mut Commands,
    furni: &PlacedFurniture,
    ui_font: &Res<UiFont>,
) {
    commands
        .spawn((
            FurnitureContextMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-180.0)),
                width: Val::Px(360.0),
                padding: UiRect::all(Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
            BackgroundColor(Color::srgb(0.12, 0.15, 0.22)),
        ))
        .with_children(|menu| {
            menu.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|top| {
                top.spawn((
                    Text::new("Opções do Móvel"),
                    ui_font.text_font(14.0),
                    TextColor(Color::WHITE),
                ));

                top.spawn((
                    ContextCloseBtn,
                    crate::LegacyButton,
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.28, 0.38)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("✖"),
                        ui_font.text_font(12.0),
                        TextColor(Color::WHITE),
                    ));
                });
            });

            menu.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|btns| {
                // Rotate Button
                btns.spawn((
                    ContextRotateBtn(furni.item_id.clone()),
                    crate::LegacyButton,
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.45, 0.75)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Girar 90°"),
                        ui_font.text_font(12.0),
                        TextColor(Color::WHITE),
                    ));
                });

                // Move Button
                btns.spawn((
                    ContextMoveBtn(furni.item_id.clone()),
                    crate::LegacyButton,
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.7, 0.5, 0.2)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Mover"),
                        ui_font.text_font(12.0),
                        TextColor(Color::WHITE),
                    ));
                });

                // Pickup Button
                btns.spawn((
                    ContextPickupBtn(furni.item_id.clone()),
                    crate::LegacyButton,
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.75, 0.25, 0.25)),
                ))
                .with_children(|b| {
                    b.spawn((
                        Text::new("Recolher"),
                        ui_font.text_font(12.0),
                        TextColor(Color::WHITE),
                    ));
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Unit Tests (Pure Logic)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armchair_model_sits_on_the_floor_and_turns_with_the_item() {
        let model = furniture_model("armchair_brown_leather", 1, 1).expect("armchair model");
        assert!(furniture_model("solid_red_block", 1, 1).is_none());
        assert!(furniture_model("", 1, 1).is_none());

        // Cube of height 0.8 is centered at 0.4 above the floor; the model's lowest
        // point (0.798 * 0.5 below its origin) must touch the floor.
        let t = furniture_model_transform(&model, 0.8, 0);
        assert!((t.translation.y + 0.4 - 0.798 * 0.5).abs() < 1e-5);
        assert!((t.translation.y + 0.4 - model.bottom * model.scale).abs() < 1e-5);

        // Model front is +Z at rotation 0 and follows the item rotation like avatars do.
        for (rotation, expected) in [
            (0, Vec3::Z),
            (1, Vec3::X),
            (2, Vec3::NEG_Z),
            (3, Vec3::NEG_X),
            (5, Vec3::X),
        ] {
            let front = furniture_model_transform(&model, 0.8, rotation).rotation * Vec3::Z;
            assert!(
                front.abs_diff_eq(expected, 1e-5),
                "rotation {rotation}: {front:?}"
            );
        }
    }

    #[test]
    fn test_rotated_dimensions() {
        assert_eq!(rotated_dimensions(2, 1, 0), (2, 1));
        assert_eq!(rotated_dimensions(2, 1, 1), (1, 2));
        assert_eq!(rotated_dimensions(2, 1, 2), (2, 1));
        assert_eq!(rotated_dimensions(2, 1, 3), (1, 2));
    }

    #[test]
    fn test_calculate_occupied_tiles() {
        let tiles = calculate_occupied_tiles(4, 5, 2, 2);
        assert_eq!(tiles.len(), 4);
        assert_eq!(tiles[0], (4, 5));
        assert_eq!(tiles[1], (4, 6));
        assert_eq!(tiles[2], (5, 5));
        assert_eq!(tiles[3], (5, 6));
    }

    #[test]
    fn test_calculate_furniture_transform() {
        let (pos, size) = calculate_furniture_transform(3, 4, 1, 1, false);
        assert_eq!(pos.x, 3.5);
        assert_eq!(pos.z, 4.5);
        assert_eq!(size.y, 0.8);

        // Floor decoration is flatter
        let (_, size_floor) = calculate_furniture_transform(3, 4, 1, 1, true);
        assert_eq!(size_floor.y, 0.04);
    }

    #[test]
    fn floor_decorations_sit_on_top_of_the_floor_tiles() {
        let (pos, size) = calculate_furniture_transform(3, 4, 2, 2, true);
        assert!((pos.y - size.y / 2.0 - FLOOR_TILE_TOP).abs() < 1e-6);
        // Solid furniture stands on the floor tiles too, not inside them.
        let (pos, size) = calculate_furniture_transform(3, 4, 1, 1, false);
        assert!((pos.y - size.y / 2.0 - FLOOR_TILE_TOP).abs() < 1e-6);
    }

    #[test]
    fn test_parse_hex_color() {
        let red = parse_hex_color("#eb4d4b");
        match red {
            Color::LinearRgba(_) | Color::Srgba(_) => {}
            _ => panic!("expected sRGB or LinearRgba"),
        }
    }

    #[test]
    fn cdn_models_are_fitted_to_the_footprint() {
        // 2 x 1 x 1 model units: a 1x1 item is limited by x, a 2x1 item fills both tiles.
        register_cdn_model("furniture/test_sofa.glb", [2.0, 1.0, 1.0], 0.5);
        let one = furniture_model("test_sofa", 1, 1).expect("cdn model");
        assert_eq!(one.path, "furniture/test_sofa.glb");
        assert!((one.scale - 0.47).abs() < 1e-4);
        assert_eq!(one.bottom, 0.5);
        let two = furniture_model("test_sofa", 2, 1).expect("cdn model");
        assert!((two.scale - 0.94).abs() < 1e-4);
    }

    #[test]
    fn invalid_cdn_models_are_ignored() {
        register_cdn_model("furniture/test_flat.glb", [1.0, 0.0, 1.0], 0.0);
        register_cdn_model("characters/test_part.glb", [1.0, 1.0, 1.0], 0.0);
        register_cdn_model("furniture/test_nan.glb", [f32::NAN, 1.0, 1.0], 0.0);
        assert!(furniture_model("test_flat", 1, 1).is_none());
        assert!(furniture_model("test_part", 1, 1).is_none());
        assert!(furniture_model("test_nan", 1, 1).is_none());
    }

    #[test]
    fn footprint_swaps_for_quarter_turns() {
        assert_eq!(base_footprint(2, 1, 0), (2, 1));
        assert_eq!(base_footprint(2, 1, 1), (1, 2));
        assert_eq!(base_footprint(2, 1, 2), (2, 1));
        assert_eq!(base_footprint(2, 1, 3), (1, 2));
    }
}
