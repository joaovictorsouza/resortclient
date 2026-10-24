//! Downloader and on-disk cache for the CDN assets described by `manifest.json`
//! (built by `tools/asset_pipeline/build_manifest.py`).
//!
//! Files are immutable and content-addressed, so the cache is simple: a file is valid when it exists
//! under its hashed name and its sha256 matches. Downloads go to a temporary file, are verified, and
//! are then renamed into place, so a crash or a corrupt download never leaves a usable bad file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Largest single asset the client accepts (guards against a hostile or broken CDN).
pub const MAX_ASSET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Manifest {
    pub manifest_version: u32,
    pub assets_version: String,
    #[serde(default)]
    pub min_app_version: String,
    pub files: BTreeMap<String, ManifestFile>,
}

/// Bounding box of a furniture model (model units), written by `build_manifest.py`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ModelMeta {
    pub size: [f32; 3],
    /// Distance from the model origin down to its lowest point.
    pub bottom: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ManifestFile {
    /// Hashed path relative to the CDN base URL.
    pub url: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub tier: Tier,
    /// Present for furniture models.
    #[serde(default)]
    pub model: Option<ModelMeta>,
    /// Days a lazy file may stay in the cache without being used (absent: kept while the manifest lists it).
    #[serde(default)]
    pub ttl_days: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Downloaded before the player enters the game.
    Core,
    /// Downloaded when something asks for it, and dropped after `ttl_days` without use.
    #[default]
    Lazy,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("manifest: {0}")]
    Manifest(String),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("hash mismatch for {0}")]
    HashMismatch(String),
    #[error("asset {0} is larger than allowed")]
    TooLarge(String),
}

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self, AssetError> {
        let m: Manifest =
            serde_json::from_str(raw).map_err(|e| AssetError::Manifest(e.to_string()))?;
        if m.manifest_version != 1 {
            return Err(AssetError::Manifest(format!(
                "unsupported manifest_version {}",
                m.manifest_version
            )));
        }
        for (name, f) in &m.files {
            // Names come from the network: never allow paths that escape the cache directory.
            if !safe_relative(&f.url) || !safe_relative(name) {
                return Err(AssetError::Manifest(format!("unsafe path {name}")));
            }
        }
        Ok(m)
    }
}

fn safe_relative(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\\')
        && p.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

/// Where a manifest file lives in the cache.
pub fn cache_file(cache_dir: &Path, file: &ManifestFile) -> PathBuf {
    cache_dir.join(&file.url)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// True when the cached file exists with the expected size and hash.
pub fn is_cached(cache_dir: &Path, file: &ManifestFile) -> bool {
    let path = cache_file(cache_dir, file);
    match std::fs::metadata(&path) {
        Ok(meta) if meta.len() == file.size => std::fs::read(&path)
            .map(|b| sha256_hex(&b) == file.sha256)
            .unwrap_or(false),
        _ => false,
    }
}

/// Names of the files of `tier` (or every tier when `None`) that still need to be downloaded.
pub fn missing<'a>(
    manifest: &'a Manifest,
    cache_dir: &Path,
    tier: Option<Tier>,
) -> Vec<(&'a str, &'a ManifestFile)> {
    manifest
        .files
        .iter()
        .filter(|(_, f)| tier.is_none_or(|t| f.tier == t))
        .filter(|(_, f)| !is_cached(cache_dir, f))
        .map(|(name, f)| (name.as_str(), f))
        .collect()
}

const DAY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// Marks a cached file as used now. Cheap enough to call on every read: the modification time is only
/// rewritten when it is more than a day old, which is all the resolution the expiry needs.
pub fn touch_if_stale(path: &Path) {
    let stale = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age >= DAY);
    if stale && let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) {
        let _ = file.set_modified(std::time::SystemTime::now());
    }
}

/// Deletes lazy files that were not used for their `ttl_days`. The modification time is the last use
/// (a download sets it, [`touch_if_stale`] refreshes it). Returns how many files were removed.
pub fn prune_expired(manifest: &Manifest, cache_dir: &Path, now: std::time::SystemTime) -> usize {
    let mut removed = 0;
    for file in manifest.files.values() {
        let Some(days) = file.ttl_days.filter(|_| file.tier == Tier::Lazy) else {
            continue;
        };
        let path = cache_file(cache_dir, file);
        let expired = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|used| now.duration_since(used).ok())
            .is_some_and(|idle| idle >= DAY * days);
        if expired && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Deletes cached files the manifest no longer references (old versions). Returns how many were removed.
pub fn prune(manifest: &Manifest, cache_dir: &Path) -> std::io::Result<usize> {
    let keep: std::collections::HashSet<PathBuf> = manifest
        .files
        .values()
        .map(|f| cache_file(cache_dir, f))
        .collect();
    let mut removed = 0;
    let mut stack = vec![cache_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if !keep.contains(&path) {
                std::fs::remove_file(&path)?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

/// Downloads one file, verifies size and hash, and moves it into the cache.
pub async fn download_file(
    http: &reqwest::Client,
    base_url: &str,
    cache_dir: &Path,
    name: &str,
    file: &ManifestFile,
) -> Result<(), AssetError> {
    if file.size > MAX_ASSET_BYTES {
        return Err(AssetError::TooLarge(name.to_string()));
    }
    let url = format!("{}/{}", base_url.trim_end_matches('/'), file.url);
    let bytes = http
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if bytes.len() as u64 != file.size || bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(AssetError::HashMismatch(name.to_string()));
    }
    if sha256_hex(&bytes) != file.sha256 {
        return Err(AssetError::HashMismatch(name.to_string()));
    }
    let dest = cache_file(cache_dir, file);
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(tmp, dest)?;
    Ok(())
}

/// Fetches and parses the manifest.
pub async fn fetch_manifest(
    http: &reqwest::Client,
    manifest_url: &str,
) -> Result<Manifest, AssetError> {
    let raw = http
        .get(manifest_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Manifest::parse(&raw)
}

/// Downloads every missing file of `tier`, calling `progress(done, total)` after each one.
pub async fn sync_tier(
    http: &reqwest::Client,
    base_url: &str,
    manifest: &Manifest,
    cache_dir: &Path,
    tier: Option<Tier>,
    mut progress: impl FnMut(usize, usize),
) -> Result<(), AssetError> {
    let todo = missing(manifest, cache_dir, tier);
    let total = todo.len();
    for (i, (name, file)) in todo.into_iter().enumerate() {
        download_file(http, base_url, cache_dir, name, file).await?;
        progress(i + 1, total);
    }
    Ok(())
}

/// Progress of [`sync_blocking`], reported from the worker thread.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncEvent {
    /// A cached file is valid and can be served: logical name and absolute path.
    FileReady { name: String, path: PathBuf },
    /// Every logical name the manifest lists (cached or not): what can still be requested.
    Listed(Vec<String>),
    /// A furniture model announced by the manifest (logical path and bounds).
    Model {
        name: String,
        size: [f32; 3],
        bottom: f32,
    },
    /// `done` of `total` core files are present.
    Progress { done: usize, total: usize },
    /// Every core file is present; the game may start.
    CoreReady,
    /// A requested lazy file could not be downloaded; asking again retries it.
    LazyFailed { name: String, reason: String },
    /// The core tier could not be completed (offline, bad manifest...).
    Failed(String),
}

const MANIFEST_FILE: &str = "manifest.json";
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Synchronizes the cache with the CDN on a throwaway runtime; call it from a dedicated thread.
///
/// Order: fetch the manifest (or fall back to the last saved copy), drop expired and unreferenced files,
/// announce every valid cached file, download the missing `core` files and announce
/// [`SyncEvent::CoreReady`]. Lazy files are then downloaded one at a time as their logical names
/// arrive on `requests`; the function returns when the sender is dropped.
pub fn sync_blocking(
    manifest_url: &str,
    cache_dir: &Path,
    requests: crossbeam_channel::Receiver<String>,
    mut emit: impl FnMut(SyncEvent),
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return emit(SyncEvent::Failed(format!("runtime: {e}"))),
    };
    runtime.block_on(async {
        let http = match reqwest::Client::builder().timeout(HTTP_TIMEOUT).build() {
            Ok(c) => c,
            Err(e) => return emit(SyncEvent::Failed(format!("http client: {e}"))),
        };
        let saved = cache_dir.join(MANIFEST_FILE);
        let manifest = match fetch_manifest_text(&http, manifest_url).await {
            Ok(text) => match Manifest::parse(&text) {
                Ok(m) => {
                    let _ = std::fs::create_dir_all(cache_dir);
                    let _ = std::fs::write(&saved, &text);
                    m
                }
                Err(e) => return emit(SyncEvent::Failed(e.to_string())),
            },
            Err(e) => match std::fs::read_to_string(&saved)
                .ok()
                .and_then(|t| Manifest::parse(&t).ok())
            {
                Some(m) => m, // offline: keep serving what is cached
                None => return emit(SyncEvent::Failed(format!("manifest: {e}"))),
            },
        };
        let base_url = base_of(manifest_url);

        let _ = prune_except(&manifest, cache_dir);
        prune_expired(&manifest, cache_dir, std::time::SystemTime::now());

        emit(SyncEvent::Listed(manifest.files.keys().cloned().collect()));
        for (name, file) in &manifest.files {
            if let Some(model) = file.model {
                emit(SyncEvent::Model {
                    name: name.clone(),
                    size: model.size,
                    bottom: model.bottom,
                });
            }
            // Lazy files are checked when they are asked for; only core ones are needed up front.
            if file.tier == Tier::Core && is_cached(cache_dir, file) {
                emit(SyncEvent::FileReady {
                    name: name.clone(),
                    path: cache_file(cache_dir, file),
                });
            }
        }

        let core = missing(&manifest, cache_dir, Some(Tier::Core));
        let total = core.len();
        for (i, (name, file)) in core.into_iter().enumerate() {
            if let Err(e) = download_file(&http, &base_url, cache_dir, name, file).await {
                return emit(SyncEvent::Failed(format!("{name}: {e}")));
            }
            emit(SyncEvent::FileReady {
                name: name.to_string(),
                path: cache_file(cache_dir, file),
            });
            emit(SyncEvent::Progress { done: i + 1, total });
        }
        emit(SyncEvent::CoreReady);

        // Lazy files on demand. Blocking here is fine: this runtime exists only for this loop.
        while let Ok(name) = requests.recv() {
            let Some(file) = manifest.files.get(&name) else {
                continue;
            };
            if !is_cached(cache_dir, file)
                && let Err(e) = download_file(&http, &base_url, cache_dir, &name, file).await
            {
                emit(SyncEvent::LazyFailed {
                    name,
                    reason: e.to_string(),
                });
                continue;
            }
            emit(SyncEvent::FileReady {
                path: cache_file(cache_dir, file),
                name,
            });
        }
    });
}

/// `https://cdn/x/manifest.json` -> `https://cdn/x`.
pub fn base_of(manifest_url: &str) -> String {
    manifest_url
        .rsplit_once('/')
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| manifest_url.to_string())
}

async fn fetch_manifest_text(
    http: &reqwest::Client,
    manifest_url: &str,
) -> Result<String, AssetError> {
    Ok(http
        .get(manifest_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

/// Like [`prune`] but never deletes the saved `manifest.json`.
fn prune_except(manifest: &Manifest, cache_dir: &Path) -> std::io::Result<usize> {
    let saved = cache_dir.join(MANIFEST_FILE);
    let keep_saved = std::fs::read(&saved).ok();
    let removed = prune(manifest, cache_dir)?;
    if let Some(bytes) = keep_saved {
        std::fs::write(saved, bytes)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW: &str = r#"{
      "manifest_version": 1, "assets_version": "v1", "min_app_version": "0.1.0",
      "files": {
        "characters/base/base.glb": {"url": "characters/base/base.3fa9c1d2.glb",
          "sha256": "2f26c0b0b4f9f6b48b1b0a2e55a4d7f3f0f1c6a8f6d6f6c1d1a0c5b0a8e9a111", "size": 4, "tier": "core"},
        "furniture/chair.glb": {"url": "furniture/chair.aaaa1111.glb",
          "sha256": "00", "size": 5}
      }
    }"#;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("resort-assets-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parses_manifest_and_defaults_tier_to_lazy() {
        let m = Manifest::parse(RAW).unwrap();
        assert_eq!(m.files["characters/base/base.glb"].tier, Tier::Core);
        assert_eq!(m.files["furniture/chair.glb"].tier, Tier::Lazy);
    }

    #[test]
    fn rejects_unsafe_paths_and_unknown_versions() {
        let evil = RAW.replace("furniture/chair.aaaa1111.glb", "../../etc/passwd");
        assert!(Manifest::parse(&evil).is_err());
        assert!(
            Manifest::parse(&RAW.replace("\"manifest_version\": 1", "\"manifest_version\": 9"))
                .is_err()
        );
        assert!(!safe_relative("/abs"));
        assert!(!safe_relative("a//b"));
        assert!(safe_relative("a/b.c"));
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_ignores_valid_files_and_detects_corruption() {
        let dir = temp_dir("missing");
        let file = ManifestFile {
            url: "furniture/chair.aaaa1111.glb".into(),
            sha256: sha256_hex(b"chair"),
            size: 5,
            tier: Tier::Lazy,
            model: None,
            ttl_days: None,
        };
        let manifest = Manifest {
            manifest_version: 1,
            assets_version: "v1".into(),
            min_app_version: String::new(),
            files: BTreeMap::from([("furniture/chair.glb".to_string(), file.clone())]),
        };
        assert_eq!(missing(&manifest, &dir, None).len(), 1);

        std::fs::create_dir_all(dir.join("furniture")).unwrap();
        std::fs::write(cache_file(&dir, &file), b"chair").unwrap();
        assert!(missing(&manifest, &dir, None).is_empty());

        std::fs::write(cache_file(&dir, &file), b"chaix").unwrap(); // same size, wrong content
        assert_eq!(missing(&manifest, &dir, None).len(), 1);
        assert!(missing(&manifest, &dir, Some(Tier::Core)).is_empty());
    }

    #[test]
    fn prune_removes_files_no_longer_in_the_manifest() {
        let dir = temp_dir("prune");
        let file = ManifestFile {
            url: "a/keep.1111.glb".into(),
            sha256: sha256_hex(b"k"),
            size: 1,
            tier: Tier::Lazy,
            model: None,
            ttl_days: None,
        };
        let manifest = Manifest {
            manifest_version: 1,
            assets_version: "v2".into(),
            min_app_version: String::new(),
            files: BTreeMap::from([("a/keep.glb".to_string(), file.clone())]),
        };
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::write(cache_file(&dir, &file), b"k").unwrap();
        std::fs::write(dir.join("a/old.0000.glb"), b"o").unwrap();
        assert_eq!(prune(&manifest, &dir).unwrap(), 1);
        assert!(cache_file(&dir, &file).exists());
        assert!(!dir.join("a/old.0000.glb").exists());
    }

    // --- end to end against a tiny local HTTP server ---------------------------------------

    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// Serves `files` (path -> body) until the process ends; unknown paths get 404.
    fn serve(files: HashMap<String, Vec<u8>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let files = Arc::new(files);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let files = files.clone();
                std::thread::spawn(move || {
                    let mut buf = [0u8; 2048];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let (status, body) = match files.get(&path) {
                        Some(b) => ("200 OK", b.clone()),
                        None => ("404 Not Found", Vec::new()),
                    };
                    let head = format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(&body);
                });
            }
        });
        format!("http://{addr}")
    }

    fn manifest_json(entries: &[(&str, &str, &[u8], &str)]) -> String {
        let files: Vec<String> = entries
            .iter()
            .map(|(name, url, body, tier)| {
                format!(
                    r#""{name}": {{"url": "{url}", "sha256": "{}", "size": {}, "tier": "{tier}"}}"#,
                    sha256_hex(body),
                    body.len()
                )
            })
            .collect();
        format!(
            r#"{{"manifest_version": 1, "assets_version": "t", "files": {{{}}}}}"#,
            files.join(",")
        )
    }

    /// Runs a whole sync; `requests` are the lazy files asked for after the core tier.
    fn run_sync(url: &str, dir: &Path, requests: &[&str]) -> Vec<SyncEvent> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let (tx, rx) = crossbeam_channel::unbounded();
        for name in requests {
            tx.send(name.to_string()).unwrap();
        }
        drop(tx); // the worker returns once the queue is drained
        sync_blocking(url, dir, rx, move |e| sink.lock().unwrap().push(e));
        events.lock().unwrap().clone()
    }

    #[test]
    fn sync_downloads_core_up_front_and_lazy_files_only_when_asked() {
        let (base, chair) = (b"base".as_slice(), b"chair".as_slice());
        let manifest = manifest_json(&[
            (
                "characters/base/base.glb",
                "characters/base/base.1111.glb",
                base,
                "core",
            ),
            (
                "furniture/chair.glb",
                "furniture/chair.2222.glb",
                chair,
                "lazy",
            ),
        ]);
        let mut files = HashMap::new();
        files.insert("/manifest.json".to_string(), manifest.into_bytes());
        files.insert("/characters/base/base.1111.glb".to_string(), base.to_vec());
        files.insert("/furniture/chair.2222.glb".to_string(), chair.to_vec());
        let url = serve(files);
        let dir = temp_dir("e2e");

        // Nothing asked for: the lazy file stays on the CDN.
        let events = run_sync(&format!("{url}/manifest.json"), &dir, &[]);
        assert!(events.contains(&SyncEvent::CoreReady));
        assert!(events.contains(&SyncEvent::Progress { done: 1, total: 1 }));
        assert!(events.contains(&SyncEvent::Listed(vec![
            "characters/base/base.glb".into(),
            "furniture/chair.glb".into()
        ])));
        assert!(!dir.join("furniture/chair.2222.glb").exists());

        // Asked for: downloaded, verified and announced after the core files.
        let events = run_sync(
            &format!("{url}/manifest.json"),
            &dir,
            &["furniture/chair.glb", "not/in/the/manifest.glb"],
        );
        let ready: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                SyncEvent::FileReady { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(ready, ["characters/base/base.glb", "furniture/chair.glb"]);
        assert_eq!(
            std::fs::read(dir.join("furniture/chair.2222.glb")).unwrap(),
            chair
        );
        // The core file was already on disk the second time: no download progress.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SyncEvent::Progress { .. }))
        );
    }

    #[test]
    fn a_failed_lazy_download_is_reported() {
        let base = b"base".as_slice();
        let manifest = manifest_json(&[
            (
                "characters/base/base.glb",
                "characters/base/base.1111.glb",
                base,
                "core",
            ),
            (
                "furniture/chair.glb",
                "furniture/chair.2222.glb",
                b"chair",
                "lazy",
            ),
        ]);
        let mut files = HashMap::new();
        files.insert("/manifest.json".to_string(), manifest.into_bytes());
        files.insert("/characters/base/base.1111.glb".to_string(), base.to_vec());
        // The chair is 404 on the CDN.
        let url = serve(files);
        let dir = temp_dir("lazy-fail");
        let events = run_sync(
            &format!("{url}/manifest.json"),
            &dir,
            &["furniture/chair.glb"],
        );
        assert!(events.contains(&SyncEvent::CoreReady));
        assert!(events.iter().any(
            |e| matches!(e, SyncEvent::LazyFailed { name, .. } if name == "furniture/chair.glb")
        ));
    }

    #[test]
    fn unused_lazy_files_expire_but_core_and_recent_ones_stay() {
        use std::time::{Duration, SystemTime};
        let dir = temp_dir("expire");
        std::fs::create_dir_all(dir.join("a")).unwrap();
        let mk = |name: &str, tier: Tier, ttl: Option<u32>| {
            let file = ManifestFile {
                url: format!("a/{name}.1111.glb"),
                sha256: sha256_hex(b"x"),
                size: 1,
                tier,
                model: None,
                ttl_days: ttl,
            };
            std::fs::write(cache_file(&dir, &file), b"x").unwrap();
            (format!("a/{name}.glb"), file)
        };
        let manifest = Manifest {
            manifest_version: 1,
            assets_version: "v".into(),
            min_app_version: String::new(),
            files: BTreeMap::from([
                mk("old_chair", Tier::Lazy, Some(7)),
                mk("fresh_chair", Tier::Lazy, Some(7)),
                mk("hat", Tier::Lazy, Some(30)),
                mk("body", Tier::Core, None),
            ]),
        };
        let now = SystemTime::now();
        // Eight days idle: past the 7 day furniture limit, inside the 30 day clothing limit.
        let in_eight_days = now + Duration::from_secs(8 * 24 * 3600);
        assert_eq!(prune_expired(&manifest, &dir, in_eight_days), 2);
        assert!(!dir.join("a/old_chair.1111.glb").exists());
        assert!(!dir.join("a/fresh_chair.1111.glb").exists());
        assert!(dir.join("a/hat.1111.glb").exists());
        assert!(dir.join("a/body.1111.glb").exists());
        // Thirty one days: the hat expires too, the core file never does.
        let in_a_month = now + Duration::from_secs(31 * 24 * 3600);
        assert_eq!(prune_expired(&manifest, &dir, in_a_month), 1);
        assert!(dir.join("a/body.1111.glb").exists());
    }

    #[test]
    fn touching_a_stale_file_renews_it() {
        let dir = temp_dir("touch");
        let path = dir.join("f.glb");
        std::fs::write(&path, b"x").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3 * 24 * 3600);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();
        touch_if_stale(&path);
        let age = std::fs::metadata(&path)
            .unwrap()
            .modified()
            .unwrap()
            .elapsed()
            .unwrap();
        assert!(age < std::time::Duration::from_secs(60), "{age:?}");
    }

    #[test]
    fn sync_rejects_a_corrupted_core_file_and_keeps_nothing() {
        let good = b"base".as_slice();
        let manifest = manifest_json(&[(
            "characters/base/base.glb",
            "characters/base/base.1111.glb",
            good,
            "core",
        )]);
        let mut files = HashMap::new();
        files.insert("/manifest.json".to_string(), manifest.into_bytes());
        files.insert(
            "/characters/base/base.1111.glb".to_string(),
            b"evil".to_vec(),
        ); // same size, wrong hash
        let url = serve(files);
        let dir = temp_dir("corrupt");

        let events = run_sync(&format!("{url}/manifest.json"), &dir, &[]);
        assert!(matches!(events.last(), Some(SyncEvent::Failed(m)) if m.contains("hash mismatch")));
        assert!(!events.contains(&SyncEvent::CoreReady));
        assert!(!dir.join("characters/base/base.1111.glb").exists());
    }

    #[test]
    fn sync_fails_on_missing_file_but_works_offline_from_the_saved_manifest() {
        let body = b"base".as_slice();
        let manifest = manifest_json(&[(
            "characters/base/base.glb",
            "characters/base/base.1111.glb",
            body,
            "core",
        )]);
        let dir = temp_dir("offline");

        // Manifest published but the file is 404: core cannot complete.
        let mut files = HashMap::new();
        files.insert("/manifest.json".to_string(), manifest.clone().into_bytes());
        let url = serve(files);
        let events = run_sync(&format!("{url}/manifest.json"), &dir, &[]);
        assert!(matches!(events.last(), Some(SyncEvent::Failed(_))));

        // Now the file exists on disk and the server is unreachable: the saved manifest is used.
        std::fs::create_dir_all(dir.join("characters/base")).unwrap();
        std::fs::write(dir.join("characters/base/base.1111.glb"), body).unwrap();
        let events = run_sync("http://127.0.0.1:9/manifest.json", &dir, &[]);
        assert!(events.contains(&SyncEvent::CoreReady), "{events:?}");
    }
}
