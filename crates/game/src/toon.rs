//! The game's side of the `toon_avatar` crate: loading its data from the asset server (the CDN),
//! mapping the network appearance to a look, and the props an avatar can hold.
//!
//! The data files come from the asset pipeline in `resort-game-assets`:
//! `characters/avatar_catalog.json` (what each body can wear), `characters/materials.json`
//! (material definitions) and, per body, `characters/animations/{F,M}/clips.json` next to
//! `character_anims.glb`. Until all of them have loaded the crate keeps avatars out of the scene.

use std::collections::BTreeMap;

use bevy::prelude::*;
use protocol::AvatarAppearance;
use toon_avatar::{
    AvatarAnimationConfig, AvatarCatalog, AvatarLook, BodyAnimations, BodyType, ColorGroup,
    HeldProp, MaterialDefs, parse_clips, parse_grip,
};

use crate::appearance::parse_hex_rgb;
use crate::cdn_assets::JsonText;

pub const AVATAR_CATALOG_FILE: &str = "characters/avatar_catalog.json";
pub const MATERIALS_FILE: &str = "characters/materials.json";
const CLIPS_FILE_F: &str = "characters/animations/F/clips.json";
const CLIPS_FILE_M: &str = "characters/animations/M/clips.json";
const ANIMATIONS_FILE_F: &str = "characters/animations/F/character_anims.glb";
const ANIMATIONS_FILE_M: &str = "characters/animations/M/character_anims.glb";

/// Maps the network appearance onto what the crate draws.
pub fn look_from_appearance(appearance: &AvatarAppearance) -> AvatarLook {
    let item = |id: &str| (!id.is_empty()).then(|| id.to_string());
    let mut group_colors = BTreeMap::new();
    for (name, values) in &appearance.colors {
        let Some(group) = ColorGroup::from_name(name) else {
            continue;
        };
        group_colors.insert(group, values.iter().map(|hex| parse_hex_rgb(hex)).collect());
    }
    AvatarLook {
        body: if appearance.body_type == protocol::BODY_MALE {
            BodyType::Male
        } else {
            BodyType::Female
        },
        face: item(&appearance.face_id),
        top: item(&appearance.top_id),
        bottom: item(&appearance.bottom_id),
        shoes: item(&appearance.shoes_id),
        outfit: item(&appearance.outfit_id),
        hair: item(&appearance.hair_id),
        hat: item(&appearance.hat_id),
        glasses: item(&appearance.glasses_id),
        colors: Vec::new(),
        group_colors,
    }
}

/// The prop a catalog id stands for: its model lives in the CDN under `characters/props/`.
pub fn held_prop(id: &str) -> Option<HeldProp> {
    (!id.is_empty()).then(|| HeldProp::new(format!("characters/props/{id}.glb")))
}

/// The expression after `current` in the cycle of [`protocol::EXPRESSIONS`].
pub fn next_expression(current: &str) -> &'static str {
    let at = protocol::EXPRESSIONS
        .iter()
        .position(|e| *e == current)
        .unwrap_or(0);
    protocol::EXPRESSIONS[(at + 1) % protocol::EXPRESSIONS.len()]
}

/// The prop to hold after `current`: none, then each catalog prop, then none again.
pub fn next_prop<'a>(current: &str, props: &'a [&'a str]) -> &'a str {
    match props.iter().position(|p| *p == current) {
        None => props.first().copied().unwrap_or(""),
        Some(i) => props.get(i + 1).copied().unwrap_or(""),
    }
}

/// Debug shortcuts until the room HUD has emote buttons: F7 cycles the expression, F8 the prop in
/// the hand and F9 drinks. They go through the server like any other client would.
#[cfg(debug_assertions)]
fn debug_emote_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Option<Res<crate::navigator::ActiveInputFocus>>,
    net: Option<Res<crate::net::NakamaResource>>,
    avatars: Query<(&crate::avatar::Avatar, &Children)>,
    visuals: Query<
        (&toon_avatar::AvatarFace, &toon_avatar::AvatarHolding),
        With<crate::avatar::AvatarVisualRoot>,
    >,
) {
    use nakama_client::NakamaCommand;
    use protocol::{
        ACTION_DRINK, AvatarActionRequest, ClientMessage, SetExpressionRequest, SetHoldingRequest,
    };
    let Some(net) = net else { return };
    if focus.is_some_and(|f| f.0 != crate::navigator::FocusedInput::None) {
        return;
    }
    let Some((_, children)) = avatars.iter().find(|(a, _)| a.is_local) else {
        return;
    };
    let Some((face, hand)) = children.iter().find_map(|c| visuals.get(c).ok()) else {
        return;
    };
    let send =
        |message: ClientMessage| net.handle.send_command(NakamaCommand::SendMessage(message));
    if keys.just_pressed(KeyCode::F7) {
        send(ClientMessage::SetExpression(SetExpressionRequest {
            expression: next_expression(face.expression.name()).to_string(),
        }));
    }
    if keys.just_pressed(KeyCode::F8) {
        let current = hand
            .0
            .as_ref()
            .and_then(|p| p.scene.rsplit('/').next())
            .and_then(|f| f.strip_suffix(".glb"))
            .unwrap_or("");
        send(ClientMessage::SetHolding(SetHoldingRequest {
            prop_id: next_prop(current, &["cup", "can"]).to_string(),
        }));
    }
    if keys.just_pressed(KeyCode::F9) {
        send(ClientMessage::AvatarAction(AvatarActionRequest {
            action: ACTION_DRINK.to_string(),
        }));
    }
}

/// Debug shortcut: F10 turns the cartoon look of rooms and objects on and off.
#[cfg(debug_assertions)]
fn debug_toggle_scene_style(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Option<Res<crate::navigator::ActiveInputFocus>>,
    mut style: ResMut<toon_avatar::SceneStyle>,
) {
    if keys.just_pressed(KeyCode::F10)
        && focus.is_none_or(|f| f.0 == crate::navigator::FocusedInput::None)
    {
        style.cartoon = !style.cartoon;
        info!(
            "scene style: {}",
            if style.cartoon {
                "cartoon"
            } else {
                "realistic"
            }
        );
    }
}

/// The files being loaded; once all arrive the crate's resources are built.
#[derive(Resource)]
struct ToonDataFiles {
    catalog: Handle<JsonText>,
    materials: Handle<JsonText>,
    clips_f: Handle<JsonText>,
    clips_m: Handle<JsonText>,
}

pub struct ToonAvatarGamePlugin;

impl Plugin for ToonAvatarGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(toon_avatar::ToonAvatarPlugins)
            .add_systems(Startup, request_toon_data)
            .add_systems(
                Update,
                build_toon_data.run_if(resource_exists::<ToonDataFiles>),
            );
        // The cartoon look of rooms and objects is off unless asked for: `GAME_SCENE_STYLE=cartoon`
        // (F10 flips it in debug builds until the settings screen has the option).
        app.insert_resource(toon_avatar::SceneStyle {
            cartoon: std::env::var("GAME_SCENE_STYLE").is_ok_and(|v| v == "cartoon"),
        });
        #[cfg(debug_assertions)]
        app.add_systems(Update, (debug_emote_shortcuts, debug_toggle_scene_style));
    }
}

fn request_toon_data(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ToonDataFiles {
        catalog: assets.load(AVATAR_CATALOG_FILE),
        materials: assets.load(MATERIALS_FILE),
        clips_f: assets.load(CLIPS_FILE_F),
        clips_m: assets.load(CLIPS_FILE_M),
    });
}

/// Builds the crate's catalog, material definitions and animation library when every file is in.
fn build_toon_data(
    mut commands: Commands,
    files: Res<ToonDataFiles>,
    texts: Res<Assets<JsonText>>,
) {
    let (Some(catalog_text), Some(materials_text), Some(clips_f), Some(clips_m)) = (
        texts.get(&files.catalog),
        texts.get(&files.materials),
        texts.get(&files.clips_f),
        texts.get(&files.clips_m),
    ) else {
        return;
    };
    match (
        AvatarCatalog::from_json(&catalog_text.0),
        MaterialDefs::from_json(&[&materials_text.0]),
        parse_clips(&clips_f.0),
        parse_clips(&clips_m.0),
    ) {
        (Ok(catalog), Ok(materials), Ok(clips_f_meta), Ok(clips_m_meta)) => {
            commands.insert_resource(catalog);
            commands.insert_resource(materials);
            commands.insert_resource(AvatarAnimationConfig {
                female: BodyAnimations {
                    gltf_path: ANIMATIONS_FILE_F.to_string(),
                    clips: clips_f_meta,
                    grip: parse_grip(&clips_f.0),
                },
                male: BodyAnimations {
                    gltf_path: ANIMATIONS_FILE_M.to_string(),
                    clips: clips_m_meta,
                    grip: parse_grip(&clips_m.0),
                },
            });
            commands.remove_resource::<ToonDataFiles>();
            info!("character data loaded");
        }
        (catalog, materials, f, m) => {
            error!(
                "invalid character data: catalog {:?}, materials {:?}, clips F {:?}, clips M {:?}",
                catalog.err(),
                materials.err(),
                f.err(),
                m.err()
            );
            commands.remove_resource::<ToonDataFiles>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_appearance_becomes_a_dressed_female_look() {
        let look = look_from_appearance(&AvatarAppearance::default());
        assert_eq!(look.body, BodyType::Female);
        assert_eq!(look.face.as_deref(), Some("F_head.001"));
        assert_eq!(look.top.as_deref(), Some("Top.01"));
        assert_eq!(look.hat, None);
        assert_eq!(look.outfit, None);
    }

    #[test]
    fn colors_become_group_colors_with_empty_and_bad_values_left_to_the_material() {
        let mut appearance = AvatarAppearance {
            body_type: "male".into(),
            ..AvatarAppearance::default()
        };
        appearance.colors.insert(
            "top".into(),
            vec![
                "#FF0000".into(),
                "".into(),
                "nonsense".into(),
                "#0000FF".into(),
            ],
        );
        appearance
            .colors
            .insert("tail".into(), vec!["#FFFFFF".into()]);
        let look = look_from_appearance(&appearance);
        assert_eq!(look.body, BodyType::Male);
        assert_eq!(
            look.group_colors[&ColorGroup::Top],
            vec![Some([1.0, 0.0, 0.0]), None, None, Some([0.0, 0.0, 1.0])]
        );
        assert_eq!(look.group_colors.len(), 1, "an unknown group is ignored");
    }

    #[test]
    fn the_expression_cycle_visits_every_expression_and_wraps() {
        let mut seen = vec!["neutral"];
        let mut current = "neutral";
        for _ in 0..protocol::EXPRESSIONS.len() {
            current = next_expression(current);
            seen.push(current);
        }
        assert_eq!(seen.first(), seen.last());
        assert_eq!(seen[..seen.len() - 1], *protocol::EXPRESSIONS);
        // An unknown expression starts the cycle over.
        assert_eq!(next_expression("nope"), protocol::EXPRESSIONS[1]);
    }

    #[test]
    fn the_prop_cycle_goes_empty_then_each_prop_then_empty() {
        let props = ["cup", "can"];
        assert_eq!(next_prop("", &props), "cup");
        assert_eq!(next_prop("cup", &props), "can");
        assert_eq!(next_prop("can", &props), "");
        assert_eq!(next_prop("", &[]), "");
    }

    #[test]
    fn a_prop_id_points_at_its_model_and_an_empty_id_at_nothing() {
        assert_eq!(held_prop("cup").unwrap().scene, "characters/props/cup.glb");
        assert!(held_prop("").is_none());
    }
}
