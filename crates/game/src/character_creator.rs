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
use crate::avatar::AVATAR_MODEL_SCALE;
use crate::config::GameConfig;
use crate::identity::UserProfile;
use crate::inventory::scrollbar_metrics;
use crate::net::NakamaResource;
use crate::state::AppState;
use crate::toon::look_from_appearance;

const SCROLL_LINE_PX: f32 = 32.0;
const TAB_BG: &str = "#1e293b"; // slate-800
const TAB_BG_SELECTED: &str = "#4f46e5"; // indigo-600
const OPTION_EDGE: &str = "#334155"; // slate-700
const OPTION_EDGE_SELECTED: &str = "#818cf8"; // indigo-400

/// Asset path of a character part's thumbnail (`ui/thumbs/characters/<id>.png`), rendered by the
/// `toon_avatar_demo` thumbnail mode (`TOON_DEMO_THUMBS`) and published to the CDN.
pub fn creator_thumbnail_path(id: &str) -> String {
    format!("ui/thumbs/characters/{id}.png")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorCategory {
    Body,
    Face,
    Skin,
    Hair,
    Top,
    Bottom,
    Outfit,
    Shoes,
    Accessories,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogItem {
    pub id: String,
    pub name: String,
    pub category: String,
    /// Bodies (`female`, `male`) that can wear the item; empty means both.
    #[serde(default)]
    pub body_types: Vec<String>,
}

impl CatalogEntry for CatalogItem {
    fn id(&self) -> &str {
        &self.id
    }
    fn category(&self) -> &str {
        &self.category
    }
    fn body_types(&self) -> &[String] {
        &self.body_types
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

/// Present when the Compose shell draws the creator panel: the bevy_hui window is not spawned (the 3D
/// preview, camera and lights stay).
#[derive(Resource)]
pub struct ComposeCreator;

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
    pub const ALL: [CreatorCategory; 9] = [
        CreatorCategory::Body,
        CreatorCategory::Face,
        CreatorCategory::Skin,
        CreatorCategory::Hair,
        CreatorCategory::Top,
        CreatorCategory::Bottom,
        CreatorCategory::Outfit,
        CreatorCategory::Shoes,
        CreatorCategory::Accessories,
    ];

    /// Stable identifier used in template tags.
    pub fn key(self) -> &'static str {
        match self {
            CreatorCategory::Body => "body",
            CreatorCategory::Face => "face",
            CreatorCategory::Skin => "skin",
            CreatorCategory::Hair => "hair",
            CreatorCategory::Top => "top",
            CreatorCategory::Bottom => "bottom",
            CreatorCategory::Outfit => "outfit",
            CreatorCategory::Shoes => "shoes",
            CreatorCategory::Accessories => "accessories",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            CreatorCategory::Body => "Corpo",
            CreatorCategory::Face => "Rosto",
            CreatorCategory::Skin => "Pele",
            CreatorCategory::Hair => "Cabelo",
            CreatorCategory::Top => "Superior",
            CreatorCategory::Bottom => "Calças",
            CreatorCategory::Outfit => "Conjunto",
            CreatorCategory::Shoes => "Calçados",
            CreatorCategory::Accessories => "Acessórios",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            CreatorCategory::Body => "ui/icons/user.png",
            CreatorCategory::Face => "ui/icons/user.png",
            CreatorCategory::Skin => "ui/icons/drop.png",
            CreatorCategory::Hair => "ui/icons/scissors.png",
            CreatorCategory::Top => "ui/icons/t-shirt.png",
            CreatorCategory::Bottom => "ui/icons/pants.png",
            CreatorCategory::Outfit => "ui/icons/t-shirt.png",
            CreatorCategory::Shoes => "ui/icons/sneaker.png",
            CreatorCategory::Accessories => "ui/icons/sunglasses.png",
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// The colour group a category paints.
pub fn color_group(category: CreatorCategory) -> Option<&'static str> {
    match category {
        CreatorCategory::Skin => Some("skin"),
        CreatorCategory::Hair => Some("hair"),
        CreatorCategory::Top => Some("top"),
        CreatorCategory::Bottom => Some("bottom"),
        CreatorCategory::Outfit => Some("outfit"),
        CreatorCategory::Shoes => Some("shoes"),
        CreatorCategory::Body | CreatorCategory::Face | CreatorCategory::Accessories => None,
    }
}

/// The swatches offered for a category's colour group (empty when it has none).
pub fn palette_for(category: CreatorCategory) -> &'static [&'static str] {
    match category {
        CreatorCategory::Skin => SKIN_PALETTE,
        CreatorCategory::Hair => HAIR_PALETTE,
        CreatorCategory::Top
        | CreatorCategory::Bottom
        | CreatorCategory::Outfit
        | CreatorCategory::Shoes => CLOTHES_PALETTE,
        CreatorCategory::Body | CreatorCategory::Face | CreatorCategory::Accessories => &[],
    }
}

/// Applies a colour swatch to the given category (categories without colours are ignored): the
/// swatch becomes the group's main colour and the shade is derived from it.
pub fn apply_color_choice(appearance: &mut AvatarAppearance, category: CreatorCategory, hex: &str) {
    if let Some(group) = color_group(category) {
        appearance::set_main_color(appearance, group, hex);
    }
}

/// Applies a catalog item choice. Picking the item that is already worn takes it off, for the
/// slots that can be empty; the face always stays. An outfit replaces the top and the bottom, and
/// choosing a top or a bottom takes the outfit off. Accessories are hats and glasses: `id` is the
/// item, and `items` tells which of the two it is.
pub fn apply_item_choice(
    appearance: &mut AvatarAppearance,
    category: CreatorCategory,
    id: &str,
    items: &[CatalogItem],
) {
    let toggle = |slot: &mut String| {
        *slot = if slot == id {
            String::new()
        } else {
            id.to_string()
        };
    };
    match category {
        CreatorCategory::Face => appearance.face_id = id.to_string(),
        CreatorCategory::Hair => toggle(&mut appearance.hair_id),
        CreatorCategory::Top => {
            toggle(&mut appearance.top_id);
            appearance.outfit_id.clear();
        }
        CreatorCategory::Bottom => {
            toggle(&mut appearance.bottom_id);
            appearance.outfit_id.clear();
        }
        CreatorCategory::Outfit => {
            toggle(&mut appearance.outfit_id);
            if !appearance.outfit_id.is_empty() {
                appearance.top_id.clear();
                appearance.bottom_id.clear();
            }
        }
        CreatorCategory::Shoes => toggle(&mut appearance.shoes_id),
        CreatorCategory::Accessories => {
            match items
                .iter()
                .find(|i| i.id == id)
                .map(|i| i.category.as_str())
            {
                Some("hat") => toggle(&mut appearance.hat_id),
                Some("glasses") => toggle(&mut appearance.glasses_id),
                _ => {}
            }
        }
        CreatorCategory::Body | CreatorCategory::Skin => {}
    }
}

/// Switches the body (`female` or `male`): the face becomes one of the new body and the items the
/// other body cannot wear are taken off.
pub fn apply_body_choice(
    appearance: &mut AvatarAppearance,
    items: &[CatalogItem],
    body_type: &str,
) {
    if appearance.body_type == body_type {
        return;
    }
    let first_face = appearance::selectable(items, "face", body_type)
        .first()
        .map(|i| i.id.clone());
    appearance::switch_body_type(
        appearance,
        body_type,
        first_face.as_deref(),
        |category, id| {
            items
                .iter()
                .any(|i| i.category == category && i.id == id && i.fits(body_type))
        },
    );
}

/// Picks random colours and models; `seed` makes the result deterministic for tests.
pub fn randomize_appearance(appearance: &mut AvatarAppearance, items: &[CatalogItem], seed: usize) {
    appearance::set_main_color(appearance, "skin", SKIN_PALETTE[seed % SKIN_PALETTE.len()]);
    appearance::set_main_color(
        appearance,
        "hair",
        HAIR_PALETTE[(seed / 7) % HAIR_PALETTE.len()],
    );
    appearance::set_main_color(
        appearance,
        "top",
        CLOTHES_PALETTE[(seed / 13) % CLOTHES_PALETTE.len()],
    );
    appearance::set_main_color(
        appearance,
        "bottom",
        CLOTHES_PALETTE[(seed / 19) % CLOTHES_PALETTE.len()],
    );
    appearance::set_main_color(
        appearance,
        "shoes",
        CLOTHES_PALETTE[(seed / 23) % CLOTHES_PALETTE.len()],
    );

    let body = appearance.body_type.clone();
    let pick = |category: &str, divisor: usize| {
        let list = appearance::selectable(items, category, &body);
        (!list.is_empty()).then(|| list[(seed / divisor) % list.len()].id.clone())
    };
    if let Some(id) = pick("face", 29) {
        appearance.face_id = id;
    }
    if let Some(id) = pick("hair", 3) {
        appearance.hair_id = id;
    }
    appearance.outfit_id.clear();
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
    compose: Option<Res<ComposeCreator>>,
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
    spawn_preview_avatar(&mut commands, &state.appearance, state.rotation_y);

    // 3. UI (templates are loaded here: OnEnter of the initial state runs before Startup)
    let templates = CreatorTemplates {
        window: asset_server.load("ui/creator.html"),
        tab: asset_server.load("ui/creator_tab.html"),
        title: asset_server.load("ui/creator_title.html"),
        swatch: asset_server.load("ui/creator_swatch.html"),
        option: asset_server.load("ui/creator_option.html"),
    };
    if compose.is_none() {
        commands.spawn((
            CharacterCreatorRoot,
            HtmlNode(templates.window.clone()),
            bevy::ui::UiTargetCamera(studio_camera),
        ));
    }
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
        |In(entity): In<Entity>,
         tags: Query<&Tags>,
         catalog: Res<CharacterCatalog>,
         mut state: ResMut<CharacterCreatorState>| {
            let Ok(tags) = tags.get(entity) else { return };
            if let (Some(category), Some(id)) = (
                tags.get("category")
                    .and_then(|c| CreatorCategory::from_key(c)),
                tags.get("id"),
            ) {
                if category == CreatorCategory::Body {
                    apply_body_choice(&mut state.appearance, &catalog.items, id);
                    return;
                }
                let items = catalog.items.clone();
                apply_item_choice(&mut state.appearance, category, id, &items);
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

/// Replaces the parts the catalog no longer has (an avatar saved with a previous asset pack) by
/// the first item of their slot for the body, and takes off hats, glasses and outfits that are gone.
pub fn sanitize_appearance(items: &[CatalogItem], appearance: &mut AvatarAppearance) {
    if appearance.body_type != protocol::BODY_MALE {
        appearance.body_type = protocol::BODY_FEMALE.to_string();
    }
    let body = appearance.body_type.clone();
    // `fallback`: an unknown id becomes the first item of the category (the face, hair, top, bottom
    // and shoes of an avatar saved with an older pack); otherwise it is taken off (hat, glasses,
    // outfit).
    let fix = |category: &str, id: &mut String, fallback: bool| {
        if id.is_empty() {
            return;
        }
        let exact = appearance::selectable(items, category, &body)
            .into_iter()
            .any(|i| i.id == *id);
        if exact {
            return;
        }
        match appearance::resolve_part_id(items, category, id, &body) {
            Some(first) if fallback => *id = first.id.clone(),
            _ => id.clear(),
        }
    };
    // The face is required: an empty one becomes the first face of the body.
    if appearance.face_id.is_empty()
        && let Some(first) = appearance::selectable(items, "face", &body).first()
    {
        appearance.face_id = first.id.clone();
    }
    fix("face", &mut appearance.face_id, true);
    for (category, id, fallback) in [
        ("hair", &mut appearance.hair_id, true),
        ("hat", &mut appearance.hat_id, false),
        ("glasses", &mut appearance.glasses_id, false),
        ("top", &mut appearance.top_id, true),
        ("bottom", &mut appearance.bottom_id, true),
        ("outfit", &mut appearance.outfit_id, false),
        ("shoes", &mut appearance.shoes_id, true),
    ] {
        fix(category, id, fallback);
    }
}

fn spawn_preview_avatar(commands: &mut Commands, appearance: &AvatarAppearance, rotation_y: f32) {
    commands.spawn((
        AvatarPreviewRoot,
        CharacterCreatorRoot,
        // The character (built by `toon_avatar`, which keeps it hidden until it is complete, so the
        // new model never flashes in a T-pose) is the preview root itself.
        toon_avatar::Avatar::new(look_from_appearance(appearance)),
        toon_avatar::LodFocus,
        avian3d::prelude::RigidBody::Kinematic,
        avian3d::prelude::Collider::capsule(0.3, 1.2),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(rotation_y))
            .with_scale(Vec3::splat(AVATAR_MODEL_SCALE)),
    ));
}

/// Keeps the preview in line with the appearance being edited: the crate swaps only the parts and
/// colours that changed, so nothing is rebuilt (and nothing flashes) when the player picks again.
fn sync_avatar_preview_model(
    state: Res<CharacterCreatorState>,
    mut previews: Query<&mut toon_avatar::Avatar, With<AvatarPreviewRoot>>,
) {
    if !state.is_changed() {
        return;
    }
    let look = look_from_appearance(&state.appearance);
    for mut avatar in &mut previews {
        if avatar.look != look {
            avatar.look = look.clone();
        }
    }
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
    let body = appearance.body_type.as_str();
    commands
        .entity(content)
        .with_children(|parent| match state.active_category {
            CreatorCategory::Body => {
                let bodies = [
                    CatalogItem {
                        id: protocol::BODY_FEMALE.to_string(),
                        name: "Feminino".to_string(),
                        category: "body".to_string(),
                        body_types: Vec::new(),
                    },
                    CatalogItem {
                        id: protocol::BODY_MALE.to_string(),
                        name: "Masculino".to_string(),
                        category: "body".to_string(),
                        body_types: Vec::new(),
                    },
                ];
                let refs: Vec<&CatalogItem> = bodies.iter().collect();
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Body,
                    "Tipo de corpo",
                    &refs,
                    std::slice::from_ref(&appearance.body_type),
                );
            }
            CreatorCategory::Face => {
                let items = appearance::selectable(&catalog.items, "face", body);
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Face,
                    "Rosto",
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
                    appearance::main_color(appearance, "skin").unwrap_or(""),
                );
            }
            CreatorCategory::Hair => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Hair,
                    "Cor do cabelo",
                    HAIR_PALETTE,
                    appearance::main_color(appearance, "hair").unwrap_or(""),
                );
                let items = appearance::selectable(&catalog.items, "hair", body);
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
                    appearance::main_color(appearance, "top").unwrap_or(""),
                );
                let items = appearance::selectable(&catalog.items, "top", body);
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
                    appearance::main_color(appearance, "bottom").unwrap_or(""),
                );
                let items = appearance::selectable(&catalog.items, "bottom", body);
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Bottom,
                    "Modelo de calça / bermuda / saia",
                    &items,
                    std::slice::from_ref(&appearance.bottom_id),
                );
            }
            CreatorCategory::Outfit => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Outfit,
                    "Cor do conjunto",
                    CLOTHES_PALETTE,
                    appearance::main_color(appearance, "outfit").unwrap_or(""),
                );
                let items = appearance::selectable(&catalog.items, "outfit", body);
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Outfit,
                    "Conjunto (troca a roupa de cima e de baixo)",
                    &items,
                    std::slice::from_ref(&appearance.outfit_id),
                );
            }
            CreatorCategory::Shoes => {
                spawn_palette(
                    parent,
                    &templates,
                    CreatorCategory::Shoes,
                    "Cor dos calçados",
                    CLOTHES_PALETTE,
                    appearance::main_color(appearance, "shoes").unwrap_or(""),
                );
                let items = appearance::selectable(&catalog.items, "shoes", body);
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
                let hats = appearance::selectable(&catalog.items, "hat", body);
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Accessories,
                    "Bonés e chapéus",
                    &hats,
                    std::slice::from_ref(&appearance.hat_id),
                );
                let glasses = appearance::selectable(&catalog.items, "glasses", body);
                spawn_items(
                    parent,
                    &templates,
                    CreatorCategory::Accessories,
                    "Óculos",
                    &glasses,
                    std::slice::from_ref(&appearance.glasses_id),
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
                    .with("thumb", &creator_thumbnail_path(&item.id))
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
    state: ResMut<CharacterCreatorState>,
    config: Res<GameConfig>,
    net: Option<Res<NakamaResource>>,
    next_state: ResMut<NextState<AppState>>,
) {
    save_appearance(state, config, net, next_state);
}

/// Saves the appearance (profile file + server) and leaves the editor. Shared by the HTML button
/// and the Compose shell's `creator_save` command.
pub(crate) fn save_appearance(
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

    fn item(id: &str, category: &str, bodies: &[&str]) -> CatalogItem {
        CatalogItem {
            id: id.to_string(),
            name: id.to_string(),
            category: category.to_string(),
            body_types: bodies.iter().map(|b| b.to_string()).collect(),
        }
    }

    fn catalog() -> Vec<CatalogItem> {
        vec![
            item("F_head.001", "face", &["female"]),
            item("F_head.002", "face", &["female"]),
            item("M_head.001", "face", &["male"]),
            item("hair_a", "hair", &["female", "male"]),
            item("hair_b", "hair", &["female", "male"]),
            item("top_a", "top", &["female", "male"]),
            item("top_f", "top", &["female"]),
            item("pants_a", "bottom", &["female", "male"]),
            item("outfit_a", "outfit", &["female", "male"]),
            item("shoes_a", "shoes", &["female", "male"]),
            item("cap", "hat", &["female", "male"]),
            item("round", "glasses", &["female", "male"]),
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
    fn color_choice_sets_the_group_main_color_and_its_shade() {
        let mut a = AvatarAppearance::default();
        apply_color_choice(&mut a, CreatorCategory::Hair, "#123456");
        apply_color_choice(&mut a, CreatorCategory::Shoes, "#abcdef");
        assert_eq!(appearance::main_color(&a, "hair"), Some("#123456"));
        assert_eq!(appearance::main_color(&a, "shoes"), Some("#abcdef"));
        assert_eq!(a.colors["hair"][0], appearance::shade_of("#123456"));
        let before = a.clone();
        apply_color_choice(&mut a, CreatorCategory::Face, "#000000");
        apply_color_choice(&mut a, CreatorCategory::Accessories, "#000000");
        assert_eq!(a, before);
    }

    #[test]
    fn item_choice_targets_the_right_slot() {
        let items = catalog();
        let mut a = AvatarAppearance::default();
        apply_item_choice(&mut a, CreatorCategory::Face, "F_head.002", &items);
        apply_item_choice(&mut a, CreatorCategory::Hair, "hair_b", &items);
        apply_item_choice(&mut a, CreatorCategory::Top, "top_a", &items);
        apply_item_choice(&mut a, CreatorCategory::Bottom, "pants_a", &items);
        apply_item_choice(&mut a, CreatorCategory::Shoes, "shoes_a", &items);
        assert_eq!(
            (&a.face_id[..], &a.hair_id[..], &a.top_id[..]),
            ("F_head.002", "hair_b", "top_a")
        );
        assert_eq!((&a.bottom_id[..], &a.shoes_id[..]), ("pants_a", "shoes_a"));
        let before = a.clone();
        apply_item_choice(&mut a, CreatorCategory::Skin, "anything", &items);
        apply_item_choice(&mut a, CreatorCategory::Body, "male", &items);
        assert_eq!(a, before);
    }

    #[test]
    fn picking_the_worn_item_again_takes_it_off_but_the_face_stays() {
        let items = catalog();
        let mut a = AvatarAppearance::default();
        apply_item_choice(&mut a, CreatorCategory::Hair, "hair_a", &items);
        apply_item_choice(&mut a, CreatorCategory::Hair, "hair_a", &items);
        assert_eq!(a.hair_id, "");
        apply_item_choice(&mut a, CreatorCategory::Face, "F_head.001", &items);
        apply_item_choice(&mut a, CreatorCategory::Face, "F_head.001", &items);
        assert_eq!(a.face_id, "F_head.001");
    }

    #[test]
    fn an_outfit_replaces_the_top_and_the_bottom_and_they_take_it_off() {
        let items = catalog();
        let mut a = AvatarAppearance::default();
        assert!(!a.top_id.is_empty() && !a.bottom_id.is_empty());
        apply_item_choice(&mut a, CreatorCategory::Outfit, "outfit_a", &items);
        assert_eq!(a.outfit_id, "outfit_a");
        assert_eq!((a.top_id.as_str(), a.bottom_id.as_str()), ("", ""));
        apply_item_choice(&mut a, CreatorCategory::Top, "top_a", &items);
        assert_eq!((a.outfit_id.as_str(), a.top_id.as_str()), ("", "top_a"));
    }

    #[test]
    fn hats_and_glasses_are_the_accessories_and_each_toggles() {
        let items = catalog();
        let mut a = AvatarAppearance::default();
        apply_item_choice(&mut a, CreatorCategory::Accessories, "cap", &items);
        apply_item_choice(&mut a, CreatorCategory::Accessories, "round", &items);
        assert_eq!((a.hat_id.as_str(), a.glasses_id.as_str()), ("cap", "round"));
        apply_item_choice(&mut a, CreatorCategory::Accessories, "cap", &items);
        assert_eq!(a.hat_id, "");
        // An unknown id changes nothing.
        let before = a.clone();
        apply_item_choice(&mut a, CreatorCategory::Accessories, "nope", &items);
        assert_eq!(a, before);
    }

    #[test]
    fn switching_the_body_takes_a_face_of_that_body_and_drops_what_does_not_fit() {
        let items = catalog();
        let mut a = AvatarAppearance {
            top_id: "top_f".into(),
            hair_id: "hair_a".into(),
            ..AvatarAppearance::default()
        };
        apply_body_choice(&mut a, &items, "male");
        assert_eq!(a.body_type, "male");
        assert_eq!(a.face_id, "M_head.001");
        assert_eq!(a.top_id, "", "a top only the female body has is taken off");
        assert_eq!(a.hair_id, "hair_a");
        // Choosing the body that is already worn changes nothing.
        let before = a.clone();
        apply_body_choice(&mut a, &items, "male");
        assert_eq!(a, before);
    }

    #[test]
    fn randomize_is_deterministic_uses_palettes_and_respects_the_body() {
        let items = catalog();
        let mut a = AvatarAppearance::default();
        let mut b = AvatarAppearance::default();
        randomize_appearance(&mut a, &items, 12345);
        randomize_appearance(&mut b, &items, 12345);
        assert_eq!(a, b);
        assert!(SKIN_PALETTE.contains(&appearance::main_color(&a, "skin").unwrap()));
        assert!(CLOTHES_PALETTE.contains(&appearance::main_color(&a, "top").unwrap()));
        assert!(["F_head.001", "F_head.002"].contains(&a.face_id.as_str()));
        let mut m = AvatarAppearance {
            body_type: "male".into(),
            ..AvatarAppearance::default()
        };
        randomize_appearance(&mut m, &items, 7);
        assert_eq!(m.face_id, "M_head.001");
    }

    #[test]
    fn sanitize_replaces_items_from_a_previous_asset_pack() {
        let items = catalog();
        let mut a = AvatarAppearance {
            body_type: "robot".into(),
            hair_id: "hair_girl_cabelo_afro".to_string(),
            top_id: "top_girl_geo_girl_sweater".to_string(),
            bottom_id: "pants_a".to_string(),
            face_id: "hair_a".to_string(), // another category
            hat_id: "removed_hat".to_string(),
            glasses_id: "round".to_string(),
            ..AvatarAppearance::default()
        };
        sanitize_appearance(&items, &mut a);
        assert_eq!(a.body_type, "female");
        assert_eq!(a.hair_id, "hair_a");
        assert_eq!(a.top_id, "top_a");
        assert_eq!(a.face_id, "F_head.001");
        // valid choices are kept, unknown optional items are taken off
        assert_eq!(a.bottom_id, "pants_a");
        assert_eq!(a.glasses_id, "round");
        assert_eq!(a.hat_id, "");
    }

    #[test]
    fn an_empty_face_becomes_the_first_face_of_the_body() {
        let items = catalog();
        let mut a = AvatarAppearance {
            body_type: "male".into(),
            face_id: String::new(),
            ..AvatarAppearance::default()
        };
        sanitize_appearance(&items, &mut a);
        assert_eq!(a.face_id, "M_head.001");
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
            let found = appearance::resolve_part_id(&catalog.items, category, id, &d.body_type);
            assert_eq!(
                found.map(|i| i.id.as_str()),
                Some(id.as_str()),
                "{category}"
            );
        }
        for category in ["hat", "glasses", "outfit"] {
            assert!(
                !appearance::selectable(&catalog.items, category, &d.body_type).is_empty(),
                "no {category} in the catalog"
            );
        }
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
