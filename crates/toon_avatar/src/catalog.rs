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
