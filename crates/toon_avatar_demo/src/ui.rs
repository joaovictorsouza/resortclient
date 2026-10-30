//! Touch/mouse UI for the prototype.
//!
//! * A bar of buttons that press the same keys as the keyboard shortcuts, so the
//!   existing input systems work unchanged on phones.
//! * A colour panel (like the Unity demo's colour pickers): choose a category, then tap a
//!   swatch to step through a palette. Colours are per glTF material and apply to every
//!   mesh sharing it.

use std::collections::HashMap;

use bevy::{gltf::GltfMaterialName, prelude::*};

use toon_avatar::{Avatar, COLOR_SLOTS, DeviceTier, Equipped, MaterialDefs, Slot};

const BUTTONS: &[(&str, KeyCode)] = &[
    ("Top", KeyCode::Space),
    ("Bottom", KeyCode::KeyB),
    ("Face", KeyCode::KeyH),
    ("Shoes", KeyCode::KeyS),
    ("Outfit", KeyCode::KeyU),
    ("Body", KeyCode::KeyX),
    ("Hair", KeyCode::KeyN),
    ("Hat", KeyCode::KeyY),
    ("Glasses", KeyCode::KeyG),
    ("Outline", KeyCode::KeyO),
    ("Merge", KeyCode::KeyM),
    ("Quality", KeyCode::KeyQ),
    ("Pose", KeyCode::KeyP),
    ("Emote", KeyCode::KeyF),
];

/// sRGB picker values the swatches step through.
const PALETTE: &[[f32; 3]] = &[
    [0.93, 0.74, 0.64],
    [0.80, 0.58, 0.46],
    [0.62, 0.46, 0.41],
    [0.45, 0.30, 0.22],
    [0.30, 0.18, 0.12],
    [0.08, 0.08, 0.10],
    [0.55, 0.55, 0.58],
    [0.96, 0.96, 0.96],
    [0.85, 0.15, 0.15],
    [0.95, 0.55, 0.15],
    [0.95, 0.85, 0.20],
    [0.30, 0.70, 0.30],
    [0.20, 0.65, 0.85],
    [0.25, 0.30, 0.80],
    [0.60, 0.30, 0.80],
    [0.95, 0.50, 0.70],
];

const IDLE: Color = Color::srgba(0.1, 0.1, 0.15, 0.75);
const SELECTED: Color = Color::srgba(0.35, 0.4, 0.7, 0.9);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Target {
    Skin,
    Eyes,
    Hair,
    Top,
    Bottom,
    Shoes,
    Outfit,
}

const TARGETS: [(&str, Target); 7] = [
    ("Skin", Target::Skin),
    ("Eyes", Target::Eyes),
    ("Hair", Target::Hair),
    ("Top", Target::Top),
    ("Bottom", Target::Bottom),
    ("Shoes", Target::Shoes),
    ("Outfit", Target::Outfit),
];

const SLOT_LABELS: [&str; COLOR_SLOTS] = ["A1", "A2", "B1", "B2", "C1", "C2"];

#[derive(Resource)]
struct ColorPanel {
    open: bool,
    target: Target,
    /// Palette position per (category, slot), so each tap moves to the next colour.
    steps: HashMap<(Target, usize), usize>,
}

#[derive(Component)]
struct PanelRoot;

#[derive(Component)]
struct QualityLabel;

#[derive(Component)]
struct Swatch(usize);

#[derive(Component)]
struct TargetChip(Target);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ColorPanel {
            open: false,
            target: Target::Skin,
            steps: HashMap::new(),
        })
        .add_systems(Startup, (spawn_buttons, spawn_color_panel))
        .add_systems(Update, (refresh_panel, update_quality_label));
    }
}

fn button_node() -> Node {
    Node {
        padding: UiRect::axes(px(18), px(14)),
        border_radius: BorderRadius::all(px(10)),
        ..default()
    }
}

fn spawn_buttons(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: px(48),
            left: px(12),
            right: px(12),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            column_gap: px(8),
            row_gap: px(8),
            ..default()
        })
        .with_children(|bar| {
            for (label, key) in BUTTONS {
                let key = *key;
                bar.spawn((button_node(), BackgroundColor(IDLE)))
                    // Picking works for mouse and touch alike. The action only needs
                    // `just_pressed`, so the key is released right away.
                    .observe(
                        move |_: On<PointerPress>, mut keys: ResMut<ButtonInput<KeyCode>>| {
                            keys.press(key);
                            keys.release(key);
                        },
                    )
                    .with_children(|button| {
                        let mut text =
                            button.spawn((Text::new(*label), TextFont::from_font_size(22.0)));
                        if *label == "Quality" {
                            text.insert(QualityLabel);
                        }
                    });
            }
            bar.spawn((button_node(), BackgroundColor(SELECTED)))
                .observe(|_: On<PointerPress>, mut panel: ResMut<ColorPanel>| {
                    panel.open = !panel.open;
                })
                .with_child((Text::new("Colors"), TextFont::from_font_size(22.0)));
        });
}

fn spawn_color_panel(mut commands: Commands) {
    commands
        .spawn((
            PanelRoot,
            Node {
                position_type: PositionType::Absolute,
                top: px(72),
                left: px(12),
                right: px(12),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(8),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    justify_content: JustifyContent::Center,
                    column_gap: px(6),
                    row_gap: px(6),
                    ..default()
                })
                .with_children(|chips| {
                    for (label, target) in TARGETS {
                        chips
                            .spawn((
                                TargetChip(target),
                                Node {
                                    padding: UiRect::axes(px(12), px(8)),
                                    border_radius: BorderRadius::all(px(8)),
                                    ..default()
                                },
                                BackgroundColor(IDLE),
                            ))
                            .observe(move |_: On<PointerPress>, mut panel: ResMut<ColorPanel>| {
                                panel.target = target;
                            })
                            .with_child((Text::new(label), TextFont::from_font_size(18.0)));
                    }
                });
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|row| {
                    #[allow(clippy::needless_range_loop)]
                    for slot in 0..COLOR_SLOTS {
                        row.spawn((
                            Swatch(slot),
                            Node {
                                width: px(48),
                                height: px(48),
                                border_radius: BorderRadius::all(px(24)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                        ))
                        .observe(move |_: On<PointerPress>,
                                       mut panel: ResMut<ColorPanel>,
                                       mut avatars: Query<&mut Avatar>,
                                       defs: Res<MaterialDefs>,
                                       names: Query<&GltfMaterialName>,
                                       equipped: Query<(&Equipped, &GltfMaterialName)>| {
                            let materials = target_materials(panel.target, &defs, &names, &equipped);
                            let target = panel.target;
                            let step = panel.steps.entry((target, slot)).or_insert(PALETTE.len() - 1);
                            *step = (*step + 1) % PALETTE.len();
                            let color = PALETTE[*step];
                            for mut avatar in &mut avatars {
                                for name in &materials {
                                    avatar.look.set_color(name, slot, color);
                                }
                            }
                        })
                        .with_child((Text::new(SLOT_LABELS[slot]), TextFont::from_font_size(14.0)));
                    }
                    row.spawn((button_node(), BackgroundColor(IDLE)))
                        .observe(|_: On<PointerPress>,
                                  mut panel: ResMut<ColorPanel>,
                                  mut avatars: Query<&mut Avatar>,
                                  defs: Res<MaterialDefs>,
                                  names: Query<&GltfMaterialName>,
                                  equipped: Query<(&Equipped, &GltfMaterialName)>| {
                            let target = panel.target;
                            for mut avatar in &mut avatars {
                                for name in target_materials(target, &defs, &names, &equipped) {
                                    avatar.look.clear_colors(&name);
                                }
                            }
                            panel.steps.retain(|(t, _), _| *t != target);
                        })
                        .with_child((Text::new("Reset"), TextFont::from_font_size(18.0)));
                });
        });
}

/// Distinct toon-material names of the meshes that belong to a category.
fn target_materials(
    target: Target,
    defs: &MaterialDefs,
    names: &Query<&GltfMaterialName>,
    equipped: &Query<(&Equipped, &GltfMaterialName)>,
) -> Vec<String> {
    let mut out: Vec<String> = match target {
        Target::Skin => names
            .iter()
            .map(|n| n.0.clone())
            .filter(|n| n.starts_with("mat_body") || n.starts_with("mat_face"))
            .collect(),
        Target::Eyes => names
            .iter()
            .map(|n| n.0.clone())
            .filter(|n| n == "mat_eye")
            .collect(),
        Target::Hair => names
            .iter()
            .map(|n| n.0.clone())
            .filter(|n| n.starts_with("hair_"))
            .collect(),
        Target::Top | Target::Bottom | Target::Shoes | Target::Outfit => {
            let slot = match target {
                Target::Top => Slot::Top,
                Target::Bottom => Slot::Bottom,
                Target::Shoes => Slot::Shoes,
                _ => Slot::Outfit,
            };
            equipped
                .iter()
                .filter(|(e, _)| e.slot == slot)
                .map(|(_, n)| n.0.clone())
                .collect()
        }
    };
    out.retain(|n| defs.contains(n));
    out.sort();
    out.dedup();
    out
}

/// Shows/hides the panel and keeps swatch colours and chip highlights up to date.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh_panel(
    panel: Res<ColorPanel>,
    defs: Res<MaterialDefs>,
    avatars: Query<&Avatar>,
    names: Query<&GltfMaterialName>,
    equipped: Query<(&Equipped, &GltfMaterialName)>,
    mut roots: Query<&mut Node, (With<PanelRoot>, Without<Swatch>)>,
    mut swatches: Query<(&Swatch, &mut BackgroundColor, &mut Node), Without<PanelRoot>>,
    mut chips: Query<(&TargetChip, &mut BackgroundColor), Without<Swatch>>,
) {
    for mut node in &mut roots {
        node.display = if panel.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !panel.open {
        return;
    }
    for (chip, mut bg) in &mut chips {
        *bg = BackgroundColor(if chip.0 == panel.target {
            SELECTED
        } else {
            IDLE
        });
    }
    let materials = target_materials(panel.target, &defs, &names, &equipped);
    for (swatch, mut bg, mut node) in &mut swatches {
        let color = materials.first().and_then(|name| {
            avatars
                .iter()
                .find_map(|a| a.look.color(name, swatch.0))
                .or_else(|| defs.color(name, swatch.0))
        });
        // Slots that are black by default are unused by the material: hide them.
        let used = color.is_some_and(|c| c.iter().sum::<f32>() > 0.02);
        node.display = if used { Display::Flex } else { Display::None };
        if let Some(c) = color {
            *bg = BackgroundColor(Color::srgb(c[0], c[1], c[2]));
        }
    }
}

fn update_quality_label(
    tier: Option<Res<DeviceTier>>,
    mut labels: Query<&mut Text, With<QualityLabel>>,
) {
    let Some(tier) = tier.filter(|t| t.is_changed()) else {
        return;
    };
    for mut text in &mut labels {
        text.0 = format!("Tier: {}", tier.label());
    }
}
