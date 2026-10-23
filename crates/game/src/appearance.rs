//! Pure (engine-agnostic) helpers for avatar appearance: catalog selection with a fallback
//! for items that no longer exist, and which parts take the player's color.
//!
//! This module intentionally depends on `std` only so it can be unit tested in isolation.

/// Read-only view of a catalog entry.
pub trait CatalogEntry {
    fn id(&self) -> &str;
    fn category(&self) -> &str;
    /// `true` when the part is drawn flat in the color the player picks; `false` keeps the
    /// colors painted on the model (faces, accessories, multicolored outfits).
    fn tint(&self) -> bool;
}

/// Items the player can pick for `category`, in catalog order.
pub fn selectable<'a, T: CatalogEntry>(items: &'a [T], category: &str) -> Vec<&'a T> {
    items.iter().filter(|i| i.category() == category).collect()
}

/// Resolves the item that should actually be rendered for a slot.
///
/// - empty id -> `None` (slot intentionally empty)
/// - id of an item of `category` -> that item
/// - anything else (an item removed by a new asset pack, or one of another category) -> the
///   first item of the category, so an old saved avatar is never left without that part
///   or asks the asset loader for a file that is gone; `None` if the category is empty
pub fn resolve_part_id<'a, T: CatalogEntry>(
    items: &'a [T],
    category: &str,
    id: &str,
) -> Option<&'a T> {
    if id.is_empty() {
        return None;
    }
    items
        .iter()
        .find(|i| i.category() == category && i.id() == id)
        .or_else(|| items.iter().find(|i| i.category() == category))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        id: &'static str,
        category: &'static str,
        tint: bool,
    }

    impl CatalogEntry for Item {
        fn id(&self) -> &str {
            self.id
        }
        fn category(&self) -> &str {
            self.category
        }
        fn tint(&self) -> bool {
            self.tint
        }
    }

    fn it(id: &'static str, category: &'static str, tint: bool) -> Item {
        Item { id, category, tint }
    }

    fn catalog() -> Vec<Item> {
        vec![
            it("hair_a", "hair", true),
            it("hair_b", "hair", true),
            it("top_a", "top", true),
            it("costume", "top", false),
            it("hat", "accessory", false),
        ]
    }

    #[test]
    fn selectable_lists_one_category_in_catalog_order() {
        let c = catalog();
        let ids: Vec<&str> = selectable(&c, "top").iter().map(|i| i.id()).collect();
        assert_eq!(ids, vec!["top_a", "costume"]);
        assert!(selectable(&c, "shoes").is_empty());
    }

    fn resolved<'a>(c: &'a [Item], category: &str, id: &str) -> Option<&'a str> {
        resolve_part_id(c, category, id).map(|i| i.id())
    }

    #[test]
    fn resolve_keeps_a_known_id_and_an_empty_slot() {
        let c = catalog();
        assert_eq!(resolved(&c, "hair", ""), None);
        assert_eq!(resolved(&c, "hair", "hair_b"), Some("hair_b"));
    }

    #[test]
    fn resolve_replaces_unknown_ids_with_the_first_item_of_the_category() {
        let c = catalog();
        assert_eq!(resolved(&c, "hair", "hair_boy_hair_001"), Some("hair_a"));
    }

    #[test]
    fn resolve_does_not_accept_an_item_of_another_category() {
        let c = catalog();
        assert_eq!(resolved(&c, "hair", "hat"), Some("hair_a"));
    }

    #[test]
    fn resolve_returns_none_when_the_category_is_empty() {
        let c = catalog();
        assert_eq!(resolved(&c, "shoes", "shoe_old"), None);
    }

    #[test]
    fn resolved_item_carries_the_tint_flag() {
        let c = catalog();
        assert!(resolve_part_id(&c, "hair", "hair_a").unwrap().tint());
        assert!(!resolve_part_id(&c, "top", "costume").unwrap().tint());
        assert!(!resolve_part_id(&c, "accessory", "hat").unwrap().tint());
    }
}
