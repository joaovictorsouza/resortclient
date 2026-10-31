//! Catalogs served by the server (`get_catalogs`) with an on-disk cache.
//!
//! Order of preference for the cosmetics catalog: the fresh server reply, then the cached copy of the
//! last reply from this server; nothing is embedded in the app (`CharacterCatalog::default` is empty).

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use protocol::GetCatalogsResponse;

use crate::cdn_assets::JsonText;
use crate::character_creator::CharacterCatalog;
use crate::config::GameConfig;

/// A `get_catalogs` reply received from the server.
#[derive(Message, Debug, Clone)]
pub struct NetCatalogsLoadedEvent(pub GetCatalogsResponse);

pub struct CatalogSyncPlugin;

impl Plugin for CatalogSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NetCatalogsLoadedEvent>()
            .add_systems(Startup, (load_cached_cosmetics, load_cdn_cosmetics_file))
            .add_systems(
                Update,
                (apply_cdn_cosmetics_file, apply_loaded_catalogs).chain(),
            );
    }
}

/// Cache file next to the player profile, one per server host so a dev cache never leaks into prod.
pub fn cache_path(config: &GameConfig) -> Option<PathBuf> {
    let dir = config.profile_path.as_ref()?.parent()?.to_path_buf();
    let host: String = config
        .host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Some(dir.join(format!("catalogs-{host}.json")))
}

pub fn read_cache(path: &Path) -> Option<GetCatalogsResponse> {
    let raw = std::fs::read_to_string(path).ok()?;
    let cached: GetCatalogsResponse = serde_json::from_str(&raw).ok()?;
    // An `unchanged` reply carries no content and is never cached; ignore such a file.
    (!cached.unchanged && !cached.catalog_version.is_empty()).then_some(cached)
}

pub fn write_cache(path: &Path, catalogs: &GetCatalogsResponse) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Write to a temporary file and rename, so a crash never leaves a half-written cache.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_vec(catalogs).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(tmp, path)
}

/// `catalog_version` of the cached copy, or an empty string when nothing is cached.
pub fn cached_version(config: &GameConfig) -> String {
    cache_path(config)
        .and_then(|p| read_cache(&p))
        .map(|c| c.catalog_version)
        .unwrap_or_default()
}

/// Cosmetics of a catalogs reply, when present and valid.
pub fn cosmetics_of(catalogs: &GetCatalogsResponse) -> Option<CharacterCatalog> {
    let value = catalogs.cosmetics.clone()?;
    let catalog: CharacterCatalog = serde_json::from_value(value).ok()?;
    // Never replace a good catalog with an empty one.
    (!catalog.items.is_empty()).then_some(catalog)
}

/// Catalog published with the assets: what a first launch uses before the server has ever answered
/// (the character creator opens before the connection), and the server's reply replaces it.
const CDN_COSMETICS_FILE: &str = "characters/cosmetics_v3.json";

#[derive(Resource)]
struct CdnCosmeticsFile(Handle<JsonText>);

fn load_cdn_cosmetics_file(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CdnCosmeticsFile(asset_server.load(CDN_COSMETICS_FILE)));
}

/// Parses the CDN catalog text; `None` when it is invalid or empty.
pub fn cosmetics_from_text(text: &str) -> Option<CharacterCatalog> {
    let catalog: CharacterCatalog = serde_json::from_str(text).ok()?;
    (!catalog.items.is_empty()).then_some(catalog)
}

fn apply_cdn_cosmetics_file(
    file: Option<Res<CdnCosmeticsFile>>,
    texts: Res<Assets<JsonText>>,
    mut catalog: ResMut<CharacterCatalog>,
    mut done: Local<bool>,
) {
    let Some(file) = file else { return };
    if *done {
        return;
    }
    // A cached or fresh server catalog always wins.
    if !catalog.items.is_empty() {
        *done = true;
        return;
    }
    let Some(text) = texts.get(&file.0) else {
        return;
    };
    *done = true;
    match cosmetics_from_text(&text.0) {
        Some(cosmetics) => {
            info!(
                "Using the CDN catalog ({} cosmetics)",
                cosmetics.items.len()
            );
            *catalog = cosmetics;
        }
        None => warn!("{CDN_COSMETICS_FILE} is invalid or empty"),
    }
}

fn load_cached_cosmetics(config: Res<GameConfig>, mut catalog: ResMut<CharacterCatalog>) {
    let Some(cached) = cache_path(&config).and_then(|p| read_cache(&p)) else {
        return;
    };
    if let Some(cosmetics) = cosmetics_of(&cached) {
        info!(
            "Using cached catalogs {} ({} cosmetics)",
            cached.catalog_version,
            cosmetics.items.len()
        );
        *catalog = cosmetics;
    }
}

fn apply_loaded_catalogs(
    mut events: MessageReader<NetCatalogsLoadedEvent>,
    config: Res<GameConfig>,
    mut catalog: ResMut<CharacterCatalog>,
) {
    for NetCatalogsLoadedEvent(catalogs) in events.read() {
        if catalogs.unchanged {
            debug!("Catalogs unchanged ({})", catalogs.catalog_version);
            continue;
        }
        if let Some(cosmetics) = cosmetics_of(catalogs) {
            info!(
                "Catalogs updated to {} ({} cosmetics)",
                catalogs.catalog_version,
                cosmetics.items.len()
            );
            *catalog = cosmetics;
        }
        if let Some(path) = cache_path(&config)
            && let Err(e) = write_cache(&path, catalogs)
        {
            warn!("Could not cache catalogs at {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdn_catalog_text_is_parsed_and_empty_or_broken_text_is_refused() {
        let text = include_str!("../tests/fixtures/cosmetics.json");
        assert!(!cosmetics_from_text(text).unwrap().items.is_empty());
        assert!(cosmetics_from_text(r#"{"version":1,"items":[]}"#).is_none());
        assert!(cosmetics_from_text("not json").is_none());
    }

    fn sample() -> GetCatalogsResponse {
        serde_json::from_str(include_str!("../../../contract/fixtures/get_catalogs.json")).unwrap()
    }

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("resort-catalogs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn cache_roundtrip_keeps_version_and_cosmetics() {
        let path = temp_file("roundtrip.json");
        write_cache(&path, &sample()).unwrap();
        let back = read_cache(&path).expect("cache should be readable");
        assert_eq!(back.catalog_version, sample().catalog_version);
        assert_eq!(cosmetics_of(&back).unwrap().items[0].id, "F_head.001");
    }

    #[test]
    fn missing_or_corrupt_cache_is_ignored() {
        assert!(read_cache(&temp_file("missing.json")).is_none());
        let bad = temp_file("bad.json");
        std::fs::write(&bad, "not json").unwrap();
        assert!(read_cache(&bad).is_none());
    }

    #[test]
    fn unchanged_reply_is_never_a_cache() {
        let path = temp_file("unchanged.json");
        let unchanged = GetCatalogsResponse {
            catalog_version: "abc".into(),
            unchanged: true,
            ..Default::default()
        };
        write_cache(&path, &unchanged).unwrap();
        assert!(read_cache(&path).is_none());
    }

    #[test]
    fn empty_cosmetics_do_not_replace_the_embedded_reserve() {
        let mut empty = sample();
        empty.cosmetics = Some(serde_json::json!({"version": 2, "items": []}));
        assert!(cosmetics_of(&empty).is_none());
        empty.cosmetics = None;
        assert!(cosmetics_of(&empty).is_none());
    }

    #[test]
    fn cache_file_depends_on_the_server_host() {
        let a = GameConfig {
            profile_path: Some(PathBuf::from("/data/profile.json")),
            host: "resort-svr-dev.joaovictor.eng.br".into(),
            ..GameConfig::default()
        };
        let mut b = a.clone();
        b.host = "assets.example.com:8".into();
        assert_ne!(cache_path(&a), cache_path(&b));
        assert!(
            cache_path(&b)
                .unwrap()
                .ends_with("catalogs-assets_example_com_8.json")
        );
    }
}
