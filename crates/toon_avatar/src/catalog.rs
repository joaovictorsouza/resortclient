//! Plain data describing what an avatar can wear (`AvatarCatalog`) and what one avatar
//! wears (`AvatarLook`). Everything here is engine-light and serde-friendly so a game can
//! store the look in its own protocol and load the catalog from any source.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Which of the two bodies an avatar uses.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    #[default]
    Female,
    Male,
}

impl BodyType {
    /// `'F'` or `'M'`, the prefix used by the asset names.
    pub fn letter(self) -> char {
        match self {
            BodyType::Female => 'F',
            BodyType::Male => 'M',
        }
    }

    pub fn other(self) -> Self {
        match self {
            BodyType::Female => BodyType::Male,
            BodyType::Male => BodyType::Female,
        }
    }
}

/// A place on the avatar that holds one catalog item.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Face,
    Top,
    Bottom,
    Shoes,
    Outfit,
    Hair,
    Hat,
    Glasses,
}

impl Slot {
    pub const ALL: [Slot; 8] = [
        Slot::Face,
        Slot::Top,
        Slot::Bottom,
        Slot::Shoes,
        Slot::Outfit,
        Slot::Hair,
        Slot::Hat,
        Slot::Glasses,
    ];

    /// Garments are skinned onto the body skeleton; attachments hang on the head bone.
    pub fn is_attachment(self) -> bool {
        matches!(self, Slot::Hair | Slot::Hat | Slot::Glasses)
    }
}

/// One wearable: the models to spawn and the body parts it covers.
#[derive(Clone, Debug, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub glbs: Vec<String>,
    /// `BodyPartType` ids of the body meshes hidden while this item is worn.
    #[serde(default)]
    pub hides: Vec<u32>,
}

/// Everything available to one body type.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct BodyCatalog {
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub face: Vec<CatalogEntry>,
    #[serde(default)]
    pub top: Vec<CatalogEntry>,
    #[serde(default)]
    pub bottom: Vec<CatalogEntry>,
    #[serde(default)]
    pub shoes: Vec<CatalogEntry>,
    #[serde(default)]
    pub outfit: Vec<CatalogEntry>,
    #[serde(default)]
    pub hair: Vec<CatalogEntry>,
    #[serde(default)]
    pub hat: Vec<CatalogEntry>,
    #[serde(default)]
    pub glasses: Vec<CatalogEntry>,
}

impl BodyCatalog {
    pub fn slot(&self, slot: Slot) -> &[CatalogEntry] {
        match slot {
            Slot::Face => &self.face,
            Slot::Top => &self.top,
            Slot::Bottom => &self.bottom,
            Slot::Shoes => &self.shoes,
            Slot::Outfit => &self.outfit,
            Slot::Hair => &self.hair,
            Slot::Hat => &self.hat,
            Slot::Glasses => &self.glasses,
        }
    }

    pub fn find(&self, slot: Slot, name: &str) -> Option<&CatalogEntry> {
        self.slot(slot).iter().find(|e| e.name == name)
    }

    fn slot_mut(&mut self, slot: Slot) -> &mut Vec<CatalogEntry> {
        match slot {
            Slot::Face => &mut self.face,
            Slot::Top => &mut self.top,
            Slot::Bottom => &mut self.bottom,
            Slot::Shoes => &mut self.shoes,
            Slot::Outfit => &mut self.outfit,
            Slot::Hair => &mut self.hair,
            Slot::Hat => &mut self.hat,
            Slot::Glasses => &mut self.glasses,
        }
    }
}

/// The wardrobe of both bodies. Insert it as a resource before spawning avatars.
#[derive(Resource, Clone, Debug, Default)]
pub struct AvatarCatalog {
    pub female: BodyCatalog,
    pub male: BodyCatalog,
}

#[derive(Deserialize)]
struct BodiesFile {
    #[serde(rename = "F")]
    female: BodyCatalog,
    #[serde(rename = "M")]
    male: BodyCatalog,
}

#[derive(Deserialize)]
struct AttachmentsFile {
    #[serde(default)]
    hair: Vec<CatalogEntry>,
    #[serde(default)]
    hat: Vec<CatalogEntry>,
    #[serde(default)]
    glasses: Vec<CatalogEntry>,
}

impl AvatarCatalog {
    pub fn of(&self, body: BodyType) -> &BodyCatalog {
        match body {
            BodyType::Female => &self.female,
            BodyType::Male => &self.male,
        }
    }

    /// Parses a catalog written as one file: `{"F": {...}, "M": {...}}`, each body with its
    /// garments and head attachments (`BodyCatalog`).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let bodies: BodiesFile = serde_json::from_str(json)?;
        Ok(AvatarCatalog {
            female: bodies.female,
            male: bodies.male,
        })
    }

    /// Builds the catalog from the files the asset pipeline writes: per-body garments
    /// (`outfits.json`), head attachments shared by both bodies (`attachments.json`) and
    /// optional extra garments (`custom_items.json`, same shape as `outfits.json`, entries are
    /// put first).
    pub fn from_pipeline_json(
        outfits: &str,
        attachments: &str,
        custom: Option<&str>,
    ) -> Result<Self, serde_json::Error> {
        let bodies: BodiesFile = serde_json::from_str(outfits)?;
        let mut catalog = AvatarCatalog {
            female: bodies.female,
            male: bodies.male,
        };
        let attach: AttachmentsFile = serde_json::from_str(attachments)?;
        for body in [&mut catalog.female, &mut catalog.male] {
            body.hair = attach.hair.clone();
            body.hat = attach.hat.clone();
            body.glasses = attach.glasses.clone();
        }
        if let Some(custom) = custom {
            let custom: HashMap<String, BodyCatalog> = serde_json::from_str(custom)?;
            for (key, extra) in custom {
                let body = match key.as_str() {
                    "F" => &mut catalog.female,
                    "M" => &mut catalog.male,
                    _ => continue,
                };
                for slot in Slot::ALL {
                    let mut merged = extra.slot(slot).to_vec();
                    merged.append(body.slot_mut(slot));
                    *body.slot_mut(slot) = merged;
                }
            }
        }
        Ok(catalog)
    }
}

/// A part of the avatar whose colours the player sets as a group. Each group covers the materials
/// of that part (see [`ColorGroup::of_material`]), and its colours are the six colour slots
/// A1, A2, B1, B2, C1, C2 of those materials.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorGroup {
    /// The body and the face (slots A1 = shade, A2 = main tone).
    Skin,
    Eyes,
    /// The clothes drawn on the body itself (slots B1, B2 of the body material).
    Underwear,
    Hair,
    Hat,
    Glasses,
    Top,
    Bottom,
    Outfit,
    Shoes,
}

impl ColorGroup {
    pub const ALL: [ColorGroup; 10] = [
        ColorGroup::Skin,
        ColorGroup::Eyes,
        ColorGroup::Underwear,
        ColorGroup::Hair,
        ColorGroup::Hat,
        ColorGroup::Glasses,
        ColorGroup::Top,
        ColorGroup::Bottom,
        ColorGroup::Outfit,
        ColorGroup::Shoes,
    ];

    /// Name used by the network protocol and the files.
    pub fn name(self) -> &'static str {
        match self {
            ColorGroup::Skin => "skin",
            ColorGroup::Eyes => "eyes",
            ColorGroup::Underwear => "underwear",
            ColorGroup::Hair => "hair",
            ColorGroup::Hat => "hat",
            ColorGroup::Glasses => "glasses",
            ColorGroup::Top => "top",
            ColorGroup::Bottom => "bottom",
            ColorGroup::Outfit => "outfit",
            ColorGroup::Shoes => "shoes",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|g| g.name() == name)
    }

    /// The group that colours colour slot `slot` (0 = A1 ... 5 = C2) of a material, given the slot
    /// of the item it belongs to (`None` for the body and face scenes). The body material uses
    /// A1/A2 for the skin and B1/B2 for the underwear; the face skin A1/A2; the eye material all of
    /// its slots for the eyes; any other material takes the group of its item's slot.
    pub fn of_material(material: &str, item_slot: Option<Slot>, slot: usize) -> Option<Self> {
        if material.starts_with("mat_body") {
            return match slot {
                0 | 1 => Some(ColorGroup::Skin),
                2 | 3 => Some(ColorGroup::Underwear),
                _ => None,
            };
        }
        if material.starts_with("mat_face") {
            return (slot < 2).then_some(ColorGroup::Skin);
        }
        if material == "mat_eye" {
            return Some(ColorGroup::Eyes);
        }
        match item_slot? {
            Slot::Hair => Some(ColorGroup::Hair),
            Slot::Hat => Some(ColorGroup::Hat),
            Slot::Glasses => Some(ColorGroup::Glasses),
            Slot::Top => Some(ColorGroup::Top),
            Slot::Bottom => Some(ColorGroup::Bottom),
            Slot::Outfit => Some(ColorGroup::Outfit),
            Slot::Shoes => Some(ColorGroup::Shoes),
            Slot::Face => None,
        }
    }
}

/// One colour chosen for one slot (A1, A2, B1, B2, C1, C2) of one glTF material.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorChoice {
    pub material: String,
    pub slot: u8,
    /// sRGB, like a colour picker.
    pub rgb: [f32; 3],
}

/// What one avatar wears. Pure data: change it on the `Avatar` component and the avatar
/// reconciles its parts and colours without being recreated.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AvatarLook {
    pub body: BodyType,
    pub face: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub shoes: Option<String>,
    pub outfit: Option<String>,
    pub hair: Option<String>,
    pub hat: Option<String>,
    pub glasses: Option<String>,
    pub colors: Vec<ColorChoice>,
    /// Colours set by group: up to six values per group (A1..C2); `None` keeps the material's own.
    pub group_colors: std::collections::BTreeMap<ColorGroup, Vec<Option<[f32; 3]>>>,
}

impl AvatarLook {
    pub fn item(&self, slot: Slot) -> Option<&str> {
        match slot {
            Slot::Face => &self.face,
            Slot::Top => &self.top,
            Slot::Bottom => &self.bottom,
            Slot::Shoes => &self.shoes,
            Slot::Outfit => &self.outfit,
            Slot::Hair => &self.hair,
            Slot::Hat => &self.hat,
            Slot::Glasses => &self.glasses,
        }
        .as_deref()
    }

    pub fn set_item(&mut self, slot: Slot, name: Option<String>) {
        *match slot {
            Slot::Face => &mut self.face,
            Slot::Top => &mut self.top,
            Slot::Bottom => &mut self.bottom,
            Slot::Shoes => &mut self.shoes,
            Slot::Outfit => &mut self.outfit,
            Slot::Hair => &mut self.hair,
            Slot::Hat => &mut self.hat,
            Slot::Glasses => &mut self.glasses,
        } = name;
    }

    /// The item actually worn: an outfit replaces the top and the bottom, and a hat replaces
    /// the hair (its model already includes the hair that fits under it).
    pub fn effective(&self, slot: Slot) -> Option<&str> {
        match slot {
            Slot::Top | Slot::Bottom if self.outfit.is_some() => None,
            Slot::Hair if self.hat.is_some() => None,
            _ => self.item(slot),
        }
    }

    /// The colour of colour slot `slot` of `material`: the per-material choice if there is one,
    /// else the choice of the group the slot belongs to.
    pub fn color_of(
        &self,
        material: &str,
        item_slot: Option<Slot>,
        slot: usize,
    ) -> Option<[f32; 3]> {
        self.color(material, slot).or_else(|| {
            let group = ColorGroup::of_material(material, item_slot, slot)?;
            self.group_colors
                .get(&group)?
                .get(slot_in_group(group, slot))?
                .as_ref()
                .copied()
        })
    }

    pub fn color(&self, material: &str, slot: usize) -> Option<[f32; 3]> {
        self.colors
            .iter()
            .find(|c| c.material == material && c.slot as usize == slot)
            .map(|c| c.rgb)
    }

    pub fn set_color(&mut self, material: &str, slot: usize, rgb: [f32; 3]) {
        match self
            .colors
            .iter_mut()
            .find(|c| c.material == material && c.slot as usize == slot)
        {
            Some(c) => c.rgb = rgb,
            None => self.colors.push(ColorChoice {
                material: material.to_string(),
                slot: slot as u8,
                rgb,
            }),
        }
    }

    pub fn clear_colors(&mut self, material: &str) {
        self.colors.retain(|c| c.material != material);
    }
}

/// Position of a material colour slot in its group's list: the underwear group is B1, B2 of the
/// body, so slots 2 and 3 are its entries 0 and 1; every other group is indexed by slot.
fn slot_in_group(group: ColorGroup, slot: usize) -> usize {
    match group {
        ColorGroup::Underwear => slot - 2,
        _ => slot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> CatalogEntry {
        CatalogEntry {
            name: name.into(),
            glbs: vec![format!("{name}.glb")],
            hides: vec![],
        }
    }

    #[test]
    fn an_outfit_replaces_top_and_bottom() {
        let look = AvatarLook {
            top: Some("t".into()),
            bottom: Some("b".into()),
            outfit: Some("o".into()),
            ..default()
        };
        assert_eq!(look.effective(Slot::Top), None);
        assert_eq!(look.effective(Slot::Bottom), None);
        assert_eq!(look.effective(Slot::Outfit), Some("o"));
        assert_eq!(look.item(Slot::Top), Some("t"));
    }

    #[test]
    fn a_hat_replaces_the_hair() {
        let look = AvatarLook {
            hair: Some("h".into()),
            hat: Some("cap".into()),
            ..default()
        };
        assert_eq!(look.effective(Slot::Hair), None);
        assert_eq!(look.effective(Slot::Hat), Some("cap"));
    }

    #[test]
    fn colors_are_set_replaced_and_cleared_per_material() {
        let mut look = AvatarLook::default();
        look.set_color("mat_a", 0, [1.0, 0.0, 0.0]);
        look.set_color("mat_a", 0, [0.0, 1.0, 0.0]);
        look.set_color("mat_b", 0, [0.0, 0.0, 1.0]);
        assert_eq!(look.colors.len(), 2);
        assert_eq!(look.color("mat_a", 0), Some([0.0, 1.0, 0.0]));
        look.clear_colors("mat_a");
        assert_eq!(look.color("mat_a", 0), None);
        assert_eq!(look.color("mat_b", 0), Some([0.0, 0.0, 1.0]));
    }

    #[test]
    fn group_colors_reach_the_slots_of_their_materials() {
        let mut look = AvatarLook::default();
        look.group_colors.insert(
            ColorGroup::Skin,
            vec![Some([0.1, 0.0, 0.0]), Some([0.9, 0.8, 0.7])],
        );
        look.group_colors
            .insert(ColorGroup::Underwear, vec![None, Some([0.0, 0.0, 1.0])]);
        look.group_colors.insert(
            ColorGroup::Top,
            vec![Some([1.0, 0.0, 0.0]), None, Some([0.0, 1.0, 0.0])],
        );
        // Skin: the body and the face take A1 and A2.
        assert_eq!(look.color_of("mat_body_F", None, 1), Some([0.9, 0.8, 0.7]));
        assert_eq!(look.color_of("mat_face_M", None, 0), Some([0.1, 0.0, 0.0]));
        // The underwear is the body's B2.
        assert_eq!(look.color_of("mat_body_F", None, 3), Some([0.0, 0.0, 1.0]));
        assert_eq!(look.color_of("mat_body_F", None, 2), None);
        // An item's materials take its slot's group, slot by slot.
        assert_eq!(
            look.color_of("mat_top.001_shirt", Some(Slot::Top), 0),
            Some([1.0, 0.0, 0.0])
        );
        assert_eq!(look.color_of("mat_top.001_shirt", Some(Slot::Top), 1), None);
        assert_eq!(
            look.color_of("mat_top.001_shirt", Some(Slot::Top), 2),
            Some([0.0, 1.0, 0.0])
        );
        // A bottom is not painted by the top's colours.
        assert_eq!(
            look.color_of("mat_bot.001_pants", Some(Slot::Bottom), 0),
            None
        );
        // A per-material choice wins over the group.
        look.set_color("mat_top.001_shirt", 0, [0.5, 0.5, 0.5]);
        assert_eq!(
            look.color_of("mat_top.001_shirt", Some(Slot::Top), 0),
            Some([0.5, 0.5, 0.5])
        );
    }

    #[test]
    fn color_groups_round_trip_through_their_names() {
        for g in ColorGroup::ALL {
            assert_eq!(ColorGroup::from_name(g.name()), Some(g));
        }
        assert_eq!(ColorGroup::from_name("tail"), None);
    }

    #[test]
    fn the_pipeline_files_become_a_catalog_with_shared_attachments_and_custom_items_first() {
        let outfits = r#"{"F":{"body":"f.glb","top":[{"name":"T1","glbs":["t1.glb"],"hides":[1]}]},
                          "M":{"body":"m.glb","top":[{"name":"T1","glbs":["mt1.glb"]}]}}"#;
        let attachments = r#"{"hair":[{"name":"H1","glbs":["h1.glb"]}],"hat":[],"glasses":[]}"#;
        let custom = r#"{"F":{"top":[{"name":"Custom","glbs":["c.glb"]}]}}"#;
        let catalog =
            AvatarCatalog::from_pipeline_json(outfits, attachments, Some(custom)).unwrap();
        assert_eq!(catalog.female.top[0].name, "Custom");
        assert_eq!(catalog.female.top[1].name, "T1");
        assert_eq!(catalog.male.top.len(), 1);
        assert_eq!(catalog.female.hair[0].name, "H1");
        assert_eq!(catalog.male.hair[0].name, "H1");
        assert_eq!(catalog.female.find(Slot::Top, "T1").unwrap().hides, vec![1]);
        let _ = entry("unused");
    }
}
