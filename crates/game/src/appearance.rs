//! Pure (engine-agnostic) helpers for avatar appearance (version 2 of the contract): catalog
//! selection per body type with a fallback for items that no longer exist, and the player's colour
//! choices.
//!
//! This module intentionally depends on `std` and `protocol` only so it can be unit tested in
//! isolation.

use protocol::{AvatarAppearance, BODY_MALE};

/// Read-only view of a catalog entry.
pub trait CatalogEntry {
    fn id(&self) -> &str;
    fn category(&self) -> &str;
    /// Body types (`female`, `male`) that can wear the item. Empty means both.
    fn body_types(&self) -> &[String];

    fn fits(&self, body_type: &str) -> bool {
        self.body_types().is_empty() || self.body_types().iter().any(|b| b == body_type)
    }
}

/// Items the player can pick for `category` with the given body, in catalog order.
pub fn selectable<'a, T: CatalogEntry>(
    items: &'a [T],
    category: &str,
    body_type: &str,
) -> Vec<&'a T> {
    items
        .iter()
        .filter(|i| i.category() == category && i.fits(body_type))
        .collect()
}

/// Resolves the item that should actually be shown for a slot.
///
/// - empty id -> `None` (slot intentionally empty)
/// - id of an item of `category` that fits the body -> that item
/// - anything else (an item removed by a new asset pack, one of another category, or one of the
///   other body) -> the first item of the category for this body, so an old saved avatar is never
///   left without that part; `None` if the category is empty
pub fn resolve_part_id<'a, T: CatalogEntry>(
    items: &'a [T],
    category: &str,
    id: &str,
    body_type: &str,
) -> Option<&'a T> {
    if id.is_empty() {
        return None;
    }
    let candidates = selectable(items, category, body_type);
    candidates
        .iter()
        .find(|i| i.id() == id)
        .or(candidates.first())
        .copied()
}

/// Parses `#RRGGBB` into sRGB components (0..1).
pub fn parse_hex_rgb(hex: &str) -> Option<[f32; 3]> {
    let clean = hex.strip_prefix('#')?;
    if clean.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&clean[i..i + 2], 16).ok();
    Some([
        channel(0)? as f32 / 255.0,
        channel(2)? as f32 / 255.0,
        channel(4)? as f32 / 255.0,
    ])
}

fn to_hex(rgb: [f32; 3]) -> String {
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        byte(rgb[0]),
        byte(rgb[1]),
        byte(rgb[2])
    )
}

/// The shade colour (slot A1) derived from a main colour (slot A2): darker and a little more
/// saturated, which is what the authored materials do (their A1 is the shadow tone of A2).
pub fn shade_of(hex: &str) -> String {
    let Some([r, g, b]) = parse_hex_rgb(hex) else {
        return hex.to_string();
    };
    let mean = (r + g + b) / 3.0;
    let saturate = |c: f32| (mean + (c - mean) * 1.15) * 0.72;
    to_hex([saturate(r), saturate(g), saturate(b)])
}

/// The main colour (slot A2) of a colour group, if the player set one.
pub fn main_color<'a>(appearance: &'a AvatarAppearance, group: &str) -> Option<&'a str> {
    appearance
        .colors
        .get(group)?
        .get(1)
        .map(String::as_str)
        .filter(|c| !c.is_empty())
}

/// Sets the main colour (A2) of a colour group and derives its shade (A1); the other slots keep
/// what they had.
pub fn set_main_color(appearance: &mut AvatarAppearance, group: &str, hex: &str) {
    let slots = appearance.colors.entry(group.to_string()).or_default();
    while slots.len() < 2 {
        slots.push(String::new());
    }
    slots[0] = shade_of(hex);
    slots[1] = hex.to_string();
}

/// Switches the body type, keeping what the other body can also wear: the face becomes the first
/// face of the new body (`first_face`), the rest stays when `keep` says the item fits.
pub fn switch_body_type(
    appearance: &mut AvatarAppearance,
    body_type: &str,
    first_face: Option<&str>,
    keep: impl Fn(&str, &str) -> bool,
) {
    appearance.body_type = body_type.to_string();
    if let Some(face) = first_face {
        appearance.face_id = face.to_string();
    }
    for (category, id) in [
        ("hair", &mut appearance.hair_id),
        ("hat", &mut appearance.hat_id),
        ("glasses", &mut appearance.glasses_id),
        ("top", &mut appearance.top_id),
        ("bottom", &mut appearance.bottom_id),
        ("outfit", &mut appearance.outfit_id),
        ("shoes", &mut appearance.shoes_id),
    ] {
        if !id.is_empty() && !keep(category, id) {
            id.clear();
        }
    }
}

/// Whether the avatar is the male body.
pub fn is_male(appearance: &AvatarAppearance) -> bool {
    appearance.body_type == BODY_MALE
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        id: &'static str,
        category: &'static str,
        bodies: Vec<String>,
    }

    impl CatalogEntry for Item {
        fn id(&self) -> &str {
            self.id
        }
        fn category(&self) -> &str {
            self.category
        }
        fn body_types(&self) -> &[String] {
            &self.bodies
        }
    }

    fn it(id: &'static str, category: &'static str, bodies: &[&str]) -> Item {
        Item {
            id,
            category,
            bodies: bodies.iter().map(|b| b.to_string()).collect(),
        }
    }

    fn catalog() -> Vec<Item> {
        vec![
            it("hair_a", "hair", &["female", "male"]),
            it("hair_b", "hair", &["female", "male"]),
            it("top_a", "top", &["female", "male"]),
            it("top_f", "top", &["female"]),
            it("cap", "hat", &[]),
        ]
    }

    #[test]
    fn selectable_lists_one_category_for_one_body_in_catalog_order() {
        let c = catalog();
        let ids =
            |cat, body| -> Vec<&str> { selectable(&c, cat, body).iter().map(|i| i.id()).collect() };
        assert_eq!(ids("top", "female"), vec!["top_a", "top_f"]);
        assert_eq!(ids("top", "male"), vec!["top_a"]);
        assert!(selectable(&c, "shoes", "female").is_empty());
        // An item without body types fits both.
        assert_eq!(ids("hat", "male"), vec!["cap"]);
    }

    fn resolved<'a>(c: &'a [Item], category: &str, id: &str, body: &str) -> Option<&'a str> {
        resolve_part_id(c, category, id, body).map(|i| i.id())
    }

    #[test]
    fn resolve_keeps_a_known_id_and_an_empty_slot() {
        let c = catalog();
        assert_eq!(resolved(&c, "hair", "", "female"), None);
        assert_eq!(resolved(&c, "hair", "hair_b", "female"), Some("hair_b"));
    }

    #[test]
    fn resolve_replaces_unknown_ids_with_the_first_item_of_the_category() {
        let c = catalog();
        assert_eq!(resolved(&c, "hair", "removed", "female"), Some("hair_a"));
        // An item of another category is not accepted.
        assert_eq!(resolved(&c, "hair", "cap", "female"), Some("hair_a"));
        // Neither is an item the other body cannot wear.
        assert_eq!(resolved(&c, "top", "top_f", "male"), Some("top_a"));
        assert_eq!(resolved(&c, "shoes", "old", "female"), None);
    }

    #[test]
    fn hex_colors_parse_and_reject_bad_input() {
        assert_eq!(parse_hex_rgb("#FF0000"), Some([1.0, 0.0, 0.0]));
        assert_eq!(parse_hex_rgb("FF0000"), None);
        assert_eq!(parse_hex_rgb("#F00"), None);
        assert_eq!(parse_hex_rgb("#GG0000"), None);
    }

    #[test]
    fn the_shade_is_darker_than_the_main_colour() {
        let shade = parse_hex_rgb(&shade_of("#E0AC8C")).unwrap();
        let main = parse_hex_rgb("#E0AC8C").unwrap();
        assert!(shade.iter().sum::<f32>() < main.iter().sum::<f32>());
        assert_eq!(shade_of("nonsense"), "nonsense");
    }

    #[test]
    fn the_main_colour_is_slot_a2_and_the_shade_slot_a1() {
        let mut app = AvatarAppearance::default();
        assert_eq!(main_color(&app, "skin"), None);
        set_main_color(&mut app, "skin", "#E0AC8C");
        assert_eq!(main_color(&app, "skin"), Some("#E0AC8C"));
        assert_eq!(app.colors["skin"][0], shade_of("#E0AC8C"));
        // Other slots of the group are kept when the main colour changes again.
        app.colors
            .get_mut("skin")
            .unwrap()
            .extend(["#111111".to_string()]);
        set_main_color(&mut app, "skin", "#FFFFFF");
        assert_eq!(app.colors["skin"][2], "#111111");
        assert_eq!(main_color(&app, "skin"), Some("#FFFFFF"));
    }

    #[test]
    fn switching_body_drops_what_the_other_body_cannot_wear() {
        let mut app = AvatarAppearance {
            top_id: "top_f".into(),
            hair_id: "hair_a".into(),
            ..AvatarAppearance::default()
        };
        switch_body_type(&mut app, "male", Some("M_head.001"), |_, id| id != "top_f");
        assert_eq!(app.body_type, "male");
        assert_eq!(app.face_id, "M_head.001");
        assert_eq!(app.top_id, "");
        assert_eq!(app.hair_id, "hair_a");
        assert!(is_male(&app));
    }
}
