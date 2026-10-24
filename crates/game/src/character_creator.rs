//! Character creator / appearance editor, built with `bevy_hui` templates (see `assets/ui/creator*.html`).
//!
//! Templates are generated from Tailwind-style sources by `tools/ui/build_ui.py`. The pure
//! helpers (category keys, colour/item choices, randomizer) are free of the ECS so they can be
//! unit tested.

use bevy::input::mouse::{MouseButton, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_hui::prelude::*;
use nakama_client::NakamaCommand;
use protocol::{AvatarAppearance, ClientMessage, UpdateAppearanceRequest};
use serde::Deserialize;

use crate::appearance::{self, CatalogEntry};
use crate::avatar::{AVATAR_MODEL_SCALE, spawn_appearance_parts};
use crate::config::GameConfig;
use crate::identity::UserProfile;
use crate::inventory::scrollbar_metrics;
use crate::net::NakamaResource;
use crate::state::AppState;

const SCROLL_LINE_PX: f32 = 32.0;
const TAB_BG: &str = "#1e293b"; // slate-800
const TAB_BG_SELECTED: &str = "#4f46e5"; // indigo-600
const OPTION_EDGE: &str = "#334155"; // slate-700
const OPTION_EDGE_SELECTED: &str = "#818cf8"; // indigo-400

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorCategory {
    Face,
    Skin,
    Hair,
    Top,
    Bottom,
    Shoes,
    Accessories,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub asset_path: String,
    /// `true` when the part is drawn flat in the color the player picks.
    #[serde(default)]
    pub tint: bool,
}

impl CatalogEntry for CatalogItem {
    fn id(&self) -> &str {
        &self.id
    }
    fn category(&self) -> &str {
        &self.category
    }
    fn tint(&self) -> bool {
        self.tint
    }
}

#[derive(Debug, Clone, Deserialize, Resource)]
pub struct CharacterCatalog {
    pub version: u32,
    pub items: Vec<CatalogItem>,
}

/// Empty until the server's cosmetics arrive (or the cached copy loads): nothing is embedded.
impl Default for CharacterCatalog {
    fn default() -> Self {
        Self {
            version: 0,
            items: Vec::new(),
        }
    }
}

#[cfg(test)]
impl CharacterCatalog {
    /// Synthetic catalog with the default appearance's parts, for tests.
    pub fn fixture() -> Self {
        serde_json::from_str(include_str!("../tests/fixtures/cosmetics.json"))
            .expect("cosmetics fixture parses")
    }
}

#[derive(Resource)]
pub struct CharacterCreatorState {
    pub appearance: AvatarAppearance,
    pub active_category: CreatorCategory,
    pub rotation_y: f32,
    pub is_dragging: bool,
    pub return_to_room: bool,
}

impl Default for CharacterCreatorState {
    fn default() -> Self {
        Self {
            appearance: AvatarAppearance::default(),
            active_category: CreatorCategory::Face,
            rotation_y: 0.0,
            is_dragging: false,
            return_to_room: false,
        }
    }
}

impl CreatorCategory {
    pub const ALL: [CreatorCategory; 7] = [
        CreatorCategory::Face,
        CreatorCategory::Skin,
        CreatorCategory::Hair,
        CreatorCategory::Top,
        CreatorCategory::Bottom,
        CreatorCategory::Shoes,
        CreatorCategory::Accessories,
    ];

    /// Stable identifier used in template tags.
    pub fn key(self) -> &'static str {
        match self {
            CreatorCategory::Face => "face",
            CreatorCategory::Skin => "skin",
            CreatorCategory::Hair => "hair",
            CreatorCategory::Top => "top",
            CreatorCategory::Bottom => "bottom",
            CreatorCategory::Shoes => "shoes",
            CreatorCategory::Accessories => "accessories",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            CreatorCategory::Face => "Rosto",
            CreatorCategory::Skin => "Pele",
            CreatorCategory::Hair => "Cabelo",
            CreatorCategory::Top => "Superior",
            CreatorCategory::Bottom => "Calças",
            CreatorCategory::Shoes => "Calçados",
            CreatorCategory::Accessories => "Acessórios",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            CreatorCategory::Face => "ui/icons/user.png",
            CreatorCategory::Skin => "ui/icons/drop.png",
            CreatorCategory::Hair => "ui/icons/scissors.png",
            CreatorCategory::Top => "ui/icons/t-shirt.png",
            CreatorCategory::Bottom => "ui/icons/pants.png",
            CreatorCategory::Shoes => "ui/icons/sneaker.png",
            CreatorCategory::Accessories => "ui/icons/sunglasses.png",
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Applies a colour swatch to the given category (categories without colours are ignored).
pub fn apply_color_choice(appearance: &mut AvatarAppearance, category: CreatorCategory, hex: &str) {
    let hex = hex.to_string();
    match category {
        CreatorCategory::Skin => appearance.skin_color = hex,
        CreatorCategory::Hair => appearance.hair_color = hex,
        CreatorCategory::Top => appearance.top_color = hex,
        CreatorCategory::Bottom => appearance.bottom_color = hex,
        CreatorCategory::Shoes => appearance.shoes_color = hex,
        CreatorCategory::Face | CreatorCategory::Accessories => {}
    }
}

/// Applies a catalog item choice; accessories toggle.
pub fn apply_item_choice(appearance: &mut AvatarAppearance, category: CreatorCategory, id: &str) {
    match category {
        CreatorCategory::Face => appearance.face_id = id.to_string(),
        CreatorCategory::Hair => appearance.hair_id = id.to_string(),
        CreatorCategory::Top => appearance.top_id = id.to_string(),
        CreatorCategory::Bottom => appearance.bottom_id = id.to_string(),
        CreatorCategory::Shoes => appearance.shoes_id = id.to_string(),
        CreatorCategory::Accessories => {
            if appearance.accessories.iter().any(|a| a == id) {
                appearance.accessories.retain(|a| a != id);
            } else {
                appearance.accessories.push(id.to_string());
            }
        }
        CreatorCategory::Skin => {}
    }
}

/// Picks random colours and models; `seed` makes the result deterministic for tests.
pub fn randomize_appearance(appearance: &mut AvatarAppearance, items: &[CatalogItem], seed: usize) {
    appearance.skin_color = SKIN_PALETTE[seed % SKIN_PALETTE.len()].to_string();
    appearance.hair_color = HAIR_PALETTE[(seed / 7) % HAIR_PALETTE.len()].to_string();
    appearance.top_color = CLOTHES_PALETTE[(seed / 13) % CLOTHES_PALETTE.len()].to_string();
    appearance.bottom_color = CLOTHES_PALETTE[(seed / 19) % CLOTHES_PALETTE.len()].to_string();
    appearance.shoes_color = CLOTHES_PALETTE[(seed / 23) % CLOTHES_PALETTE.len()].to_string();

    let pick = |category: &str, divisor: usize| {
        let list = appearance::selectable(items, category);
        (!list.is_empty()).then(|| list[(seed / divisor) % list.len()].id.clone())
    };
    if let Some(id) = pick("face", 29) {
        appearance.face_id = id;
    }
    if let Some(id) = pick("hair", 3) {
        appearance.hair_id = id;
    }
    if let Some(id) = pick("top", 5) {
        appearance.top_id = id;
    }
    if let Some(id) = pick("bottom", 11) {
        appearance.bottom_id = id;
    }
    if let Some(id) = pick("shoes", 17) {
        appearance.shoes_id = id;
    }
}

// ---------------------------------------------------------------------------
// Components, resources, plugin
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct CharacterCreatorRoot;

#[derive(Component)]
pub struct AvatarPreviewRoot;

#[derive(Resource)]
struct CreatorTemplates {
    window: Handle<HtmlTemplate>,
    tab: Handle<HtmlTemplate>,
    title: Handle<HtmlTemplate>,
    swatch: Handle<HtmlTemplate>,
    option: Handle<HtmlTemplate>,
}

/// What the option panel was last filled with, so it is only rebuilt on change.
#[derive(Resource, Default)]
struct CreatorFillCache {
    key: Option<(CreatorCategory, AvatarAppearance)>,
}

pub struct CharacterCreatorPlugin;

impl Plugin for CharacterCreatorPlugin {
    fn build(&self, app: &mut App) {
        crate::ensure_hui_plugins(app);
        crate::avatar::ensure_tint_plugin(app);
        app.init_resource::<CharacterCreatorState>()
            .init_resource::<CharacterCatalog>()
            .init_resource::<CreatorFillCache>()
            .add_systems(Startup, register_creator_functions)
            .add_systems(OnEnter(AppState::CharacterCreator), setup_creator_screen)
            .add_systems(OnExit(AppState::CharacterCreator), cleanup_creator_screen)
            .add_systems(
                Update,
                (
                    handle_mouse_drag_rotation,
                    sync_avatar_preview_model,
                    update_avatar_preview_transform,
                    sync_creator_ui,
                    sync_creator_mode,
                    scroll_creator_content,
                    update_creator_scrollbar,
                )
                    .run_if(in_state(AppState::CharacterCreator)),
            );
    }
}

// Curated stylish color palettes for character creation
pub const SKIN_PALETTE: &[&str] = &[
    "#FFE0BD", "#F5D0B5", "#E8BEAC", "#D6A374", "#C68642", "#8D5524", "#613B16", "#4A2C0F",
    "#FFE6D5", "#F0C8A0",
];

pub const HAIR_PALETTE: &[&str] = &[
    "#1C1C1C", "#3D2314", "#5A3825", "#8D5B4C", "#B55239", "#D6B370", "#E6CEA8", "#808080",
    "#FFFFFF", "#E91E63", "#2196F3", "#4CAF50",
];

pub const CLOTHES_PALETTE: &[&str] = &[
    "#FFFFFF", "#1E1E1E", "#F44336", "#E91E63", "#9C27B0", "#673AB7", "#3F51B5", "#2196F3",
    "#00BCD4", "#009688", "#4CAF50", "#8BC34A", "#FFEB3B", "#FF9800", "#795548",
];

fn setup_creator_screen(
    mut commands: Commands,
    mut state: ResMut<CharacterCreatorState>,
    mut cache: ResMut<CreatorFillCache>,
    config: Res<GameConfig>,
    asset_server: Res<AssetServer>,
    catalog: Res<CharacterCatalog>,
) {
    // Reset per-session mutable state (keep appearance)
    state.rotation_y = 0.0;
    state.is_dragging = false;
    state.active_category = CreatorCategory::Face;
    cache.key = None;

    // If local profile has saved appearance, initialize with it
    if let Some(profile_path) = &config.profile_path
        && let Ok(profile) = UserProfile::load_from_file(profile_path)
    {
        if let Some(saved_app) = &profile.appearance {
            state.appearance = saved_app.clone();
        }
        info!("Loaded profile appearance for {}", profile.nickname);
    }

    // Make sure the selected parts exist in the catalog (highlighted in the UI).
    sanitize_appearance(&catalog.items, &mut state.appearance);

    // 1. Studio Camera & Lighting
    // The UI must target this camera explicitly: `MainRoomCamera` (inactive here) would
    // otherwise be picked as the default UI camera and the whole panel would not render.
    let studio_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 10,
                ..default()
            },
            IsDefaultUiCamera,
            Transform::from_xyz(0.0, 1.2, 3.2).looking_at(Vec3::new(0.0, 0.9, 0.0), Vec3::Y),
            CharacterCreatorRoot,
        ))
        .id();

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        CharacterCreatorRoot,
    ));

    commands.spawn((
        PointLight {
            intensity: 400_000.0,
            color: Color::srgb(0.7, 0.85, 1.0),
            ..default()
        },
        Transform::from_xyz(-2.5, 2.0, 1.0),
        CharacterCreatorRoot,
    ));

    // 2. Spawn preview avatar immediately (not waiting for sync on next frame)
    spawn_preview_avatar(
        &mut commands,
        &asset_server,
        &catalog,
        &state.appearance,
        state.rotation_y,
    );

    // 3. UI (templates are loaded here: OnEnter of the initial state runs before Startup)
    let templates = CreatorTemplates {
        window: asset_server.load("ui/creator.html"),
        tab: asset_server.load("ui/creator_tab.html"),
        title: asset_server.load("ui/creator_title.html"),
        swatch: asset_server.load("ui/creator_swatch.html"),
        option: asset_server.load("ui/creator_option.html"),
    };
    commands.spawn((
        CharacterCreatorRoot,
        HtmlNode(templates.window.clone()),
        bevy::ui::UiTargetCamera(studio_camera),
    ));
    commands.insert_resource(templates);
}

fn register_creator_functions(mut functions: HtmlFunctions) {
    functions.register(
        "cc_tab",
        |In(entity): In<Entity>, tags: Query<&Tags>, mut state: ResMut<CharacterCreatorState>| {
            if let Some(category) = tags
                .get(entity)
                .ok()
                .and_then(|t| t.get("key"))
                .and_then(|k| CreatorCategory::from_key(k))
            {
                state.active_category = category;
            }
        },
    );
    functions.register(
        "cc_color",
        |In(entity): In<Entity>, tags: Query<&Tags>, mut state: ResMut<CharacterCreatorState>| {
            let Ok(tags) = tags.get(entity) else { return };
            if let (Some(category), Some(hex)) = (
                tags.get("category")
                    .and_then(|c| CreatorCategory::from_key(c)),
                tags.get("hex"),
            ) {
                apply_color_choice(&mut state.appearance, category, hex);
            }
        },
    );
    functions.register(
        "cc_item",
        |In(entity): In<Entity>, tags: Query<&Tags>, mut state: ResMut<CharacterCreatorState>| {
            let Ok(tags) = tags.get(entity) else { return };
            if let (Some(category), Some(id)) = (
                tags.get("category")
                    .and_then(|c| CreatorCategory::from_key(c)),
                tags.get("id"),
            ) {
                apply_item_choice(&mut state.appearance, category, id);
            }
        },
    );
    functions.register(
        "cc_randomize",
        |_: In<Entity>,
         catalog: Res<CharacterCatalog>,
         mut state: ResMut<CharacterCreatorState>| {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(42) as usize;
            randomize_appearance(&mut state.appearance, &catalog.items, nanos);
        },
    );
    functions.register(
        "cc_rotate_left",
        |_: In<Entity>, mut state: ResMut<CharacterCreatorState>| state.rotation_y -= 0.45,
    );
    functions.register(
        "cc_rotate_right",
        |_: In<Entity>, mut state: ResMut<CharacterCreatorState>| state.rotation_y += 0.45,
    );
    functions.register("cc_save", save_and_play);
    functions.register("cc_close", close_editor);
}

/// Replaces the parts the catalog no longer has (an avatar saved with a previous asset pack)
/// by the first item of their slot, and drops unknown accessories.
pub fn sanitize_appearance(items: &[CatalogItem], appearance: &mut AvatarAppearance) {
    let slots: [(&str, &mut String); 5] = [
        ("face", &mut appearance.face_id),
        ("hair", &mut appearance.hair_id),
        ("top", &mut appearance.top_id),
        ("bottom", &mut appearance.bottom_id),
        ("shoes", &mut appearance.shoes_id),
    ];
    for (category, id) in slots {
        if let Some(item) = appearance::resolve_part_id(items, category, id) {
            *id = item.id.clone();
        }
    }
    appearance.accessories.retain(|id| {
        items
            .iter()
            .any(|i| i.category == "accessory" && i.id == *id)
    });
}

fn spawn_preview_avatar(
    commands: &mut Commands,
    asset_server: &AssetServer,
    catalog: &CharacterCatalog,
    appearance: &AvatarAppearance,
    rotation_y: f32,
) {
    commands
        .spawn((
            AvatarPreviewRoot,
            CharacterCreatorRoot,
            crate::avatar_anim::AvatarPose::Idle,
            avian3d::prelude::RigidBody::Kinematic,
            avian3d::prelude::Collider::capsule(0.3, 1.2),
            Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(rotation_y))
                .with_scale(Vec3::splat(AVATAR_MODEL_SCALE)),
            // Hidden until the skeleton is animated, so the new model never flashes in T-pose.
            Visibility::Hidden,
            crate::avatar_anim::HiddenUntilPosed::default(),
        ))
        .with_children(|parent| {
            spawn_appearance_parts(parent, asset_server, catalog, appearance);
        });
}

#[allow(clippy::too_many_arguments)]
fn sync_avatar_preview_model(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Res<CharacterCatalog>,
    state: Res<CharacterCreatorState>,
    mut last_appearance: Local<Option<AvatarAppearance>>,
    preview_query: Query<Entity, With<AvatarPreviewRoot>>,
    children_query: Query<&Children>,
    mut parts: Query<(
        Entity,
        &crate::avatar::AvatarPartSlot,
        &mut crate::avatar::AvatarPartTint,
    )>,
) {
    // Only re-spawn when appearance actually changes AFTER setup
    // (setup already spawned the first avatar)
    if last_appearance.as_ref() == Some(&state.appearance) {
        return;
    }
    // Skip very first run: setup_creator_screen already spawned the avatar
    let Some(previous) = last_appearance.replace(state.appearance.clone()) else {
        return;
    };

    // Only colors changed: repaint the parts that are already loaded instead of reloading the avatar.
    if crate::avatar::same_parts(&previous, &state.appearance) {
        for root in preview_query.iter() {
            let mut stack = vec![root];
            while let Some(entity) = stack.pop() {
                if let Ok((_, slot, mut tint)) = parts.get_mut(entity)
                    && let Some(color) = crate::avatar::slot_color(&state.appearance, slot.0)
                {
                    tint.color = color;
                    commands
                        .entity(entity)
                        .remove::<crate::avatar::PartTintApplied>();
                }
                if let Ok(children) = children_query.get(entity) {
                    stack.extend(children.iter());
                }
            }
        }
        return;
    }

    for entity in preview_query.iter() {
        commands.entity(entity).despawn();
    }

    spawn_preview_avatar(
        &mut commands,
        &asset_server,
        &catalog,
        &state.appearance,
        state.rotation_y,
    );
}
// ---------------------------------------------------------------------------
// Option panel
// ---------------------------------------------------------------------------

/// Rebuilds the tab bar and the option list whenever the category or the appearance changes.
fn sync_creator_ui(
    mut commands: Commands,
    templates: Option<Res<CreatorTemplates>>,
    state: Res<CharacterCreatorState>,
    catalog: Res<CharacterCatalog>,
    mut cache: ResMut<CreatorFillCache>,
    ids: Query<(Entity, &UiId)>,
) {
    let Some(templates) = templates else { return };
    let key = (state.active_category, state.appearance.clone());
    if cache.key.as_ref() == Some(&key) {
        return;
    }
    let find = |name: &str| ids.iter().find(|(_, id)| id.id() == name).map(|(e, _)| e);
    let (Some(tabs), Some(content), Some(scroll)) =
        (find("cc_tabs"), find("cc_content"), find("cc_scroll"))
    else {
        return; // template not built yet; retry next frame
    };
    let category_changed = cache
        .key
        .as_ref()
        .is_none_or(|(category, _)| *category != state.active_category);
    cache.key = Some(key);

    // Tabs
    commands.entity(tabs).despawn_children();
    commands.entity(tabs).with_children(|parent| {
        for category in CreatorCategory::ALL {
            let active = category == state.active_category;
            parent.spawn((
                HtmlNode(templates.tab.clone()),
                TemplateProperties::default()
                    .with("key", category.key())
                    .with("label", category.label())
                    .with("icon", category.icon())
                    .with("bg", if active { TAB_BG_SELECTED } else { TAB_BG }),
            ));
        }
    });

    // Options
    commands.entity(content).despawn_children();
    if category_changed {
        commands.entity(scroll).insert(ScrollPosition::default());
    }
    let appearance = &state.appearance;
    commands
        .entity(content)
        .with_children(|parent| match state.active_category {
            CreatorCategory::Face => {
                let items = appearance::selectable(&catalog.items, "face");
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Face,
                    "Expressão do rosto",
                    &items,
                    std::slice::from_ref(&appearance.face_id),
                );
            }
            CreatorCategory::Skin => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Skin,
                    "Tom de pele",
                    SKIN_PALETTE,
                    &appearance.skin_color,
                );
            }
            CreatorCategory::Hair => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Hair,
                    "Cor do cabelo",
                    HAIR_PALETTE,
                    &appearance.hair_color,
                );
                let items = appearance::selectable(&catalog.items, "hair");
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Hair,
                    "Estilo de penteado",
                    &items,
                    std::slice::from_ref(&appearance.hair_id),
                );
            }
            CreatorCategory::Top => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Top,
                    "Cor da parte superior",
                    CLOTHES_PALETTE,
                    &appearance.top_color,
                );
                let items = appearance::selectable(&catalog.items, "top");
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Top,
                    "Modelo de roupa",
                    &items,
                    std::slice::from_ref(&appearance.top_id),
                );
            }
            CreatorCategory::Bottom => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Bottom,
                    "Cor das calças",
                    CLOTHES_PALETTE,
                    &appearance.bottom_color,
                );
                let items = appearance::selectable(&catalog.items, "bottom");
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Bottom,
                    "Modelo de calça / bermuda",
                    &items,
                    std::slice::from_ref(&appearance.bottom_id),
                );
            }
            CreatorCategory::Shoes => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Shoes,
                    "Cor dos calçados",
                    CLOTHES_PALETTE,
                    &appearance.shoes_color,
                );
                let items = appearance::selectable(&catalog.items, "shoes");
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Shoes,
                    "Modelo de tênis / sapato",
                    &items,
                    std::slice::from_ref(&appearance.shoes_id),
                );
            }
            CreatorCategory::Accessories => {
                let items = appearance::selectable(&catalog.items, "accessory");
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Accessories,
                    "Óculos, bonés, fones e mochilas (vários)",
                    &items,
                    &appearance.accessories,
                );
            }
        });
}

fn spawn_title(parent: &mut ChildSpawnerCommands, templates: &CreatorTemplates, text: &str) {
    parent.spawn((
        HtmlNode(templates.title.clone()),
        TemplateProperties::default().with("text", text),
    ));
}

fn wrap_row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(8.0),
        row_gap: Val::Px(8.0),
        width: Val::Percent(100.0),
        ..default()
    }
}

fn spawn_palette(
    parent: &mut ChildSpawnerCommands,
    templates: &CreatorTemplates,
    category: CreatorCategory,
    title: &str,
    palette: &[&str],
    selected_hex: &str,
) {
    spawn_title(parent, templates, title);
    parent.spawn(wrap_row()).with_children(|row| {
        for &hex in palette {
            let selected = selected_hex.eq_ignore_ascii_case(hex);
            row.spawn((
                HtmlNode(templates.swatch.clone()),
                TemplateProperties::default()
                    .with("category", category.key())
                    .with("hex", hex)
                    .with("edge", if selected { "#ffffff" } else { OPTION_EDGE }),
            ));
        }
    });
}

fn spawn_items(
    parent: &mut ChildSpawnerCommands,
    templates: &CreatorTemplates,
    category: CreatorCategory,
    title: &str,
    items: &[&CatalogItem],
    selected_ids: &[String],
) {
    spawn_title(parent, templates, title);
    parent.spawn(wrap_row()).with_children(|row| {
        for item in items {
            let selected = selected_ids.contains(&item.id);
            row.spawn((
                HtmlNode(templates.option.clone()),
                TemplateProperties::default()
                    .with("category", category.key())
                    .with("id", &item.id)
                    .with("label", &item.name)
                    .with("bg", if selected { TAB_BG_SELECTED } else { TAB_BG })
                    .with(
                        "edge",
                        if selected {
                            OPTION_EDGE_SELECTED
                        } else {
                            OPTION_EDGE
                        },
                    ),
            ));
        }
    });
}

fn scroll_creator_content(
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
        if id.id() == "cc_scroll" {
            scroll.y = (scroll.y - delta).max(0.0);
        }
    }
}

/// Drives the scrollbar thumb with `UiTransform` (same technique as the inventory window).
fn update_creator_scrollbar(
    scrolls: Query<(&UiId, &ComputedNode, &ScrollPosition)>,
    mut parts: Query<(&UiId, &mut UiTransform, &mut Visibility)>,
) {
    let Some((_, scroll_node, position)) = scrolls.iter().find(|(id, ..)| id.id() == "cc_scroll")
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
            ("cc_track", metrics) => {
                let wanted = if metrics.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != wanted {
                    *visibility = wanted;
                }
            }
            ("cc_thumb", Some((ratio, progress))) => {
                let center = ratio / 2.0 + progress * (1.0 - ratio);
                transform.scale = Vec2::new(1.0, ratio);
                transform.translation = Val2::percent(0.0, (center - 0.5) * 100.0);
            }
            _ => {}
        }
    }
}

fn handle_mouse_drag_rotation(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut motion_events: MessageReader<MouseMotion>,
    mut state: ResMut<CharacterCreatorState>,
) {
    if mouse_button.pressed(MouseButton::Left) {
        for motion in motion_events.read() {
            state.rotation_y += motion.delta.x * 0.015;
        }
    }
}

fn update_avatar_preview_transform(
    state: Res<CharacterCreatorState>,
    mut query: Query<&mut Transform, With<AvatarPreviewRoot>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(state.rotation_y);
    }
}
fn save_and_play(
    _: In<Entity>,
    mut state: ResMut<CharacterCreatorState>,
    config: Res<GameConfig>,
    net: Option<Res<NakamaResource>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    info!("Saving avatar appearance: {:?}", state.appearance);

    // 1. Save locally to device profile
    if let Some(profile_path) = &config.profile_path {
        let mut profile = UserProfile::load_from_file(profile_path).unwrap_or_else(|_| {
            UserProfile::new(config.device_id.clone(), config.username.clone())
        });
        profile.appearance = Some(state.appearance.clone());
        profile.google_account = config.is_google_account();
        crate::identity::persist_refresh_token(&config, &mut profile);
        if let Err(e) = profile.save_to_file(profile_path) {
            warn!("Failed to save profile with appearance: {e}");
        } else {
            info!(
                "Saved profile with custom appearance successfully to {:?}",
                profile_path
            );
        }
    }

    // 2. Send via realtime opcode to Nakama
    if let Some(net) = &net {
        let req = UpdateAppearanceRequest {
            appearance: state.appearance.clone(),
        };
        net.handle
            .send_command(NakamaCommand::SendMessage(ClientMessage::UpdateAppearance(
                Box::new(req),
            )));
    }

    // 3. Transition to Connecting or InRoom
    if state.return_to_room {
        state.return_to_room = false;
        next_state.set(AppState::InRoom);
    } else {
        next_state.set(AppState::Connecting);
    }
}

/// Leaves the editor without saving; the room still shows the avatar as it was.
fn close_editor(
    _: In<Entity>,
    mut state: ResMut<CharacterCreatorState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if state.return_to_room {
        state.return_to_room = false;
        next_state.set(AppState::InRoom);
    }
}

/// Text of the confirm button: the editor returns to the game, the first creation enters it.
pub fn confirm_label(editing: bool) -> &'static str {
    if editing {
        "Confirmar e voltar ao jogo"
    } else {
        "Confirmar e entrar no jogo"
    }
}

/// The close button and the confirm label depend on the mode, which the static template cannot
/// express: the template is built a few frames after entering the state.
fn sync_creator_mode(
    state: Res<CharacterCreatorState>,
    mut close: Query<(&UiId, &mut Node)>,
    mut labels: Query<(&UiId, &mut Text)>,
) {
    let editing = state.return_to_room;
    for (id, mut node) in &mut close {
        if id.id() == "cc_close" {
            let wanted = if editing {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != wanted {
                node.display = wanted;
            }
        }
    }
    for (id, mut text) in &mut labels {
        if id.id() == "cc_save_label" && text.0 != confirm_label(editing) {
            text.0 = confirm_label(editing).to_string();
        }
    }
}

fn cleanup_creator_screen(
    mut commands: Commands,
    query: Query<Entity, With<CharacterCreatorRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, category: &str) -> CatalogItem {
        CatalogItem {
            id: id.to_string(),
            name: id.to_string(),
            category: category.to_string(),
            asset_path: String::new(),
            tint: false,
        }
    }

    fn catalog() -> Vec<CatalogItem> {
        vec![
            item("face_a", "face"),
            item("face_b", "face"),
            item("hair_a", "hair"),
            item("hair_b", "hair"),
            item("top_a", "top"),
            item("pants_a", "bottom"),
            item("shoes_a", "shoes"),
            item("acc_cap", "accessory"),
        ]
    }

    #[test]
    fn category_keys_round_trip() {
        for category in CreatorCategory::ALL {
            assert_eq!(CreatorCategory::from_key(category.key()), Some(category));
        }
        assert_eq!(CreatorCategory::from_key("nope"), None);
        assert_eq!(CreatorCategory::from_key("gender"), None);
    }

    #[test]
    fn color_choice_targets_the_right_slot() {
        let mut a = AvatarAppearance::default();
        apply_color_choice(&mut a, CreatorCategory::Hair, "#123456");
        apply_color_choice(&mut a, CreatorCategory::Shoes, "#abcdef");
        assert_eq!(a.hair_color, "#123456");
        assert_eq!(a.shoes_color, "#abcdef");
        let before = a.clone();
        apply_color_choice(&mut a, CreatorCategory::Face, "#000000");
        assert_eq!(a, before);
    }

    #[test]
    fn item_choice_targets_the_right_slot() {
        let mut a = AvatarAppearance::default();
        apply_item_choice(&mut a, CreatorCategory::Face, "face_b");
        apply_item_choice(&mut a, CreatorCategory::Hair, "hair_b");
        apply_item_choice(&mut a, CreatorCategory::Top, "top_a");
        apply_item_choice(&mut a, CreatorCategory::Bottom, "pants_a");
        apply_item_choice(&mut a, CreatorCategory::Shoes, "shoes_a");
        assert_eq!(
            (&a.face_id[..], &a.hair_id[..], &a.top_id[..]),
            ("face_b", "hair_b", "top_a")
        );
        assert_eq!((&a.bottom_id[..], &a.shoes_id[..]), ("pants_a", "shoes_a"));
        let before = a.clone();
        apply_item_choice(&mut a, CreatorCategory::Skin, "anything");
        assert_eq!(a, before);
    }

    #[test]
    fn accessories_toggle() {
        let mut a = AvatarAppearance::default();
        apply_item_choice(&mut a, CreatorCategory::Accessories, "acc_cap");
        assert_eq!(a.accessories, vec!["acc_cap"]);
        apply_item_choice(&mut a, CreatorCategory::Accessories, "acc_cap");
        assert!(a.accessories.is_empty());
    }

    #[test]
    fn randomize_is_deterministic_and_uses_palettes() {
        let items = catalog();
        let mut a = AvatarAppearance::default();
        let mut b = AvatarAppearance::default();
        randomize_appearance(&mut a, &items, 12345);
        randomize_appearance(&mut b, &items, 12345);
        assert_eq!(a, b);
        assert!(SKIN_PALETTE.contains(&a.skin_color.as_str()));
        assert!(CLOTHES_PALETTE.contains(&a.top_color.as_str()));
        assert!(["face_a", "face_b"].contains(&a.face_id.as_str()));
    }

    #[test]
    fn sanitize_replaces_items_from_a_previous_asset_pack() {
        let items = catalog();
        let mut a = AvatarAppearance {
            hair_id: "hair_girl_cabelo_afro".to_string(),
            top_id: "top_girl_geo_girl_sweater".to_string(),
            bottom_id: "pants_a".to_string(),
            face_id: "hair_a".to_string(), // another category
            accessories: vec!["acc_cap".to_string(), "acc_removed".to_string()],
            ..AvatarAppearance::default()
        };
        sanitize_appearance(&items, &mut a);
        assert_eq!(a.hair_id, "hair_a");
        assert_eq!(a.top_id, "top_a");
        assert_eq!(a.face_id, "face_a");
        // valid choices are kept, unknown accessories are dropped
        assert_eq!(a.bottom_id, "pants_a");
        assert_eq!(a.accessories, vec!["acc_cap"]);
    }

    #[test]
    fn a_catalog_with_the_defaults_resolves_every_slot() {
        let catalog = CharacterCatalog::fixture();
        assert!(!catalog.items.is_empty(), "fixture must parse");
        let d = AvatarAppearance::default();
        for (category, id) in [
            ("face", &d.face_id),
            ("hair", &d.hair_id),
            ("top", &d.top_id),
            ("bottom", &d.bottom_id),
            ("shoes", &d.shoes_id),
        ] {
            let found = appearance::resolve_part_id(&catalog.items, category, id);
            assert_eq!(
                found.map(|i| i.id.as_str()),
                Some(id.as_str()),
                "{category}"
            );
        }
        assert!(
            !appearance::selectable(&catalog.items, "accessory").is_empty(),
            "no accessories in the catalog"
        );
    }

    #[test]
    fn nothing_is_embedded_so_the_default_catalog_is_empty() {
        assert!(CharacterCatalog::default().items.is_empty());
    }

    #[test]
    fn confirm_button_differs_between_the_editor_and_the_first_creation() {
        assert_eq!(confirm_label(false), "Confirmar e entrar no jogo");
        assert_eq!(confirm_label(true), "Confirmar e voltar ao jogo");
    }
}
