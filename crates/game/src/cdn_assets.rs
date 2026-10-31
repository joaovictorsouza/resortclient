//! Loads downloadable assets (characters, furniture, audio) from the CDN cache.
//!
//! The default Bevy asset source is wrapped by [`CdnReader`]: a logical path such as
//! `characters/base/body_010.glb` is served from the downloaded, hash-verified cache file when the
//! [`CdnIndex`] knows it, and from the files bundled in the app (UI, icons, font) otherwise. When
//! the path is in neither place yet but the download is still running, the read waits for it, so
//! load sites keep using plain logical paths and never race the CDN.
//!
//! A background thread keeps the cache in sync (`nakama_client::asset_cache::sync_blocking`). The
//! `core` tier (avatar body, animations, catalogs) is downloaded on the start screen (see
//! [`cdn_settled`]). Everything else is `lazy`: the first read of a logical path that is in the manifest
//! but not on disk asks the thread to download it (clothing when the player picks it, furniture when a
//! room needs it), and later reads hit the cache. Files unused for their `ttl_days` are dropped.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetSource, AssetSourceBuilder, AssetSourceId,
    ErasedAssetReader, PathStream, Reader, VecReader,
};
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use nakama_client::asset_cache::{self, SyncEvent};

use crate::catalog_sync;
use crate::config::GameConfig;

/// Logical asset path -> downloaded file.
pub type CdnIndex = Arc<RwLock<HashMap<String, PathBuf>>>;

/// Reads from the CDN cache when it has the file, otherwise from the embedded assets.
pub struct CdnReader {
    inner: Box<dyn ErasedAssetReader>,
    index: CdnIndex,
    /// When set, a path found nowhere waits while the sync can still deliver it.
    cdn: Option<CdnAssets>,
}

/// How often a waiting read re-checks the cache.
const WAIT_STEP: Duration = Duration::from_millis(100);

/// A lazy file that was requested and has not arrived is requested again after this long (a failed
/// download, or a sync thread that was restarted).
const REQUEST_RETRY: Duration = Duration::from_secs(30);

impl CdnReader {
    pub fn new(inner: Box<dyn ErasedAssetReader>, index: CdnIndex) -> Self {
        Self {
            inner,
            index,
            cdn: None,
        }
    }

    /// Same, but reads of files that are still downloading wait for them.
    pub fn waiting(inner: Box<dyn ErasedAssetReader>, cdn: CdnAssets) -> Self {
        Self {
            inner,
            index: cdn.index.clone(),
            cdn: Some(cdn),
        }
    }

    fn cached_file(&self, path: &Path) -> Option<PathBuf> {
        let key = path.to_str()?.replace('\\', "/");
        self.index.read().ok()?.get(&key).cloned()
    }
}

impl AssetReader for CdnReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        loop {
            if let Some(file) = self.cached_file(path)
                && let Ok(bytes) = std::fs::read(&file)
            {
                asset_cache::touch_if_stale(&file);
                let reader: Box<dyn Reader + 'a> = Box::new(VecReader::new(bytes));
                return Ok(reader);
            }
            match self.inner.read(path).await {
                Err(AssetReaderError::NotFound(_))
                    if self.cdn.as_ref().is_some_and(|cdn| {
                        path.to_str()
                            .is_some_and(|p| cdn.wait_for(&p.replace('\\', "/")))
                    }) =>
                {
                    async_io::Timer::after(WAIT_STEP).await;
                }
                other => return other,
            }
        }
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        self.inner.read_meta(path).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.inner.read_directory(path).await
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.inner.is_directory(path).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CdnStatus {
    /// Not started yet.
    #[default]
    Idle,
    /// No manifest URL configured: embedded assets only.
    Disabled,
    Syncing {
        done: usize,
        total: usize,
    },
    /// Core assets are available; lazy ones are downloaded when asked for.
    Ready,
    /// The core tier could not be completed (offline, bad manifest...). The sync is retried with a
    /// backoff and the start screen keeps waiting: there is nothing embedded to fall back to.
    Failed(String),
}

impl CdnStatus {
    /// True once the game can start: the core tier is on disk, or the CDN is not in use.
    pub fn is_settled(&self) -> bool {
        matches!(self, Self::Disabled | Self::Ready)
    }
}

#[derive(Resource, Clone, Default)]
pub struct CdnAssets {
    pub index: CdnIndex,
    pub status: Arc<Mutex<CdnStatus>>,
    /// Queue of lazy files for the sync thread (absent while no thread runs).
    requests: Arc<Mutex<Option<crossbeam_channel::Sender<String>>>>,
    /// Logical names the manifest lists; set once the manifest is known.
    known: Arc<RwLock<HashSet<String>>>,
    listed: Arc<AtomicBool>,
    /// Lazy files already requested, and when.
    pending: Arc<Mutex<HashMap<String, Instant>>>,
}

impl CdnAssets {
    pub fn status(&self) -> CdnStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Whether a read of `name` that was found nowhere should keep waiting. It does while the manifest
    /// is not known yet, and for a manifest file that is not on disk yet (which it requests). A name
    /// the manifest does not list, or a CDN that is not in use, ends the wait.
    pub fn wait_for(&self, name: &str) -> bool {
        if matches!(self.status(), CdnStatus::Disabled) {
            return false;
        }
        if !self.listed.load(Ordering::Acquire) {
            return true;
        }
        let known = self.known.read().is_ok_and(|k| k.contains(name));
        if known {
            self.request(name);
        }
        known
    }

    /// Asks the sync thread to download `name` unless that was asked recently.
    pub fn request(&self, name: &str) {
        let now = Instant::now();
        let Ok(mut pending) = self.pending.lock() else {
            return;
        };
        if pending
            .get(name)
            .is_some_and(|at| now.duration_since(*at) < REQUEST_RETRY)
        {
            return;
        }
        if let Ok(sender) = self.requests.lock()
            && let Some(sender) = sender.as_ref()
            && sender.send(name.to_string()).is_ok()
        {
            pending.insert(name.to_string(), now);
        }
    }

    fn set_status(&self, status: CdnStatus) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }

    /// Applies one worker event; separated from the thread so it can be tested.
    pub fn apply(&self, event: SyncEvent) {
        match event {
            SyncEvent::Listed(names) => {
                if let Ok(mut known) = self.known.write() {
                    *known = names.into_iter().collect();
                }
                self.listed.store(true, Ordering::Release);
            }
            SyncEvent::FileReady { name, path } => {
                if let Ok(mut pending) = self.pending.lock() {
                    pending.remove(&name);
                }
                if let Ok(mut index) = self.index.write() {
                    index.insert(name, path);
                }
            }
            SyncEvent::Model { name, size, bottom } => {
                crate::furniture::register_cdn_model(&name, size, bottom)
            }
            SyncEvent::Progress { done, total } => {
                self.set_status(CdnStatus::Syncing { done, total })
            }
            SyncEvent::CoreReady => self.set_status(CdnStatus::Ready),
            SyncEvent::LazyFailed { name, reason } => {
                warn!("CDN asset {name} could not be downloaded, will retry: {reason}")
            }
            SyncEvent::Failed(reason) => self.set_status(CdnStatus::Failed(reason)),
        }
    }
}

/// Registers the CDN-backed default asset source. Call before adding `DefaultPlugins`
/// (Bevy builds asset sources when `AssetPlugin` is added).
pub fn install(app: &mut App) -> CdnAssets {
    install_with(app, AssetSource::get_default_reader("assets".to_string()))
}

/// Like [`install`], with the reader for the files bundled with the app. The KMP host passes a
/// file reader over the directory where the shell copied the bundled assets (Android keeps them
/// inside the APK, where Bevy's own reader cannot reach them without an Activity).
pub fn install_with(
    app: &mut App,
    mut make_inner: impl FnMut() -> Box<dyn bevy::asset::io::ErasedAssetReader> + Send + Sync + 'static,
) -> CdnAssets {
    let cdn = CdnAssets::default();
    let reader_cdn = cdn.clone();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::platform_default("assets", None)
            .with_reader(move || Box::new(CdnReader::waiting(make_inner(), reader_cdn.clone()))),
    );
    app.insert_resource(cdn.clone());
    cdn
}

/// Run condition: the CDN finished its first pass (or is not in use).
pub fn cdn_settled(cdn: Option<Res<CdnAssets>>) -> bool {
    cdn.is_none_or(|c| c.status().is_settled())
}

/// A `.json` file read as text, loaded through the asset server (and so through the CDN):
/// `clips.json` and `characters/cosmetics_v3.json`.
#[derive(Asset, TypePath, Debug)]
pub struct JsonText(pub String);

#[derive(Default, TypePath)]
struct JsonTextLoader;

impl AssetLoader for JsonTextLoader {
    type Asset = JsonText;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<JsonText, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        String::from_utf8(bytes)
            .map(JsonText)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

pub struct CdnAssetsPlugin;

impl Plugin for CdnAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<JsonText>()
            .register_asset_loader(JsonTextLoader)
            .add_systems(Startup, start_sync.run_if(resource_exists::<CdnAssets>))
            .add_systems(
                Update,
                (retry_failed_sync, show_progress)
                    .chain()
                    .run_if(resource_exists::<CdnAssets>),
            );
    }
}

/// Manifest URL: the one the server reported last time wins over the build-time value.
pub fn manifest_url(config: &GameConfig) -> Option<String> {
    let from_server = catalog_sync::cache_path(config)
        .and_then(|p| catalog_sync::read_cache(&p))
        .map(|c| c.assets_manifest_url)
        .filter(|u| !u.is_empty());
    from_server
        .or_else(|| std::env::var("ASSETS_MANIFEST_URL").ok())
        .map(|u| u.trim().to_string())
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
}

/// Downloads live next to the player profile, one directory per CDN host.
pub fn cache_dir(config: &GameConfig, manifest_url: &str) -> Option<PathBuf> {
    let dir = config.profile_path.as_ref()?.parent()?.to_path_buf();
    let host: String = manifest_url
        .split("//")
        .nth(1)
        .unwrap_or(manifest_url)
        .split('/')
        .next()
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Some(dir.join("assets-cache").join(host))
}

fn start_sync(cdn: Res<CdnAssets>, config: Res<GameConfig>) {
    spawn_sync(&cdn, &config);
}

/// Starts (or restarts) the background sync thread.
fn spawn_sync(cdn: &CdnAssets, config: &GameConfig) {
    let (Some(url), true) = (manifest_url(config), config.profile_path.is_some()) else {
        info!("CDN assets disabled (no manifest URL or no storage directory)");
        cdn.set_status(CdnStatus::Disabled);
        return;
    };
    let Some(dir) = cache_dir(config, &url) else {
        cdn.set_status(CdnStatus::Disabled);
        return;
    };
    info!("CDN assets: syncing {url} into {}", dir.display());
    cdn.set_status(CdnStatus::Syncing { done: 0, total: 0 });
    // A new thread gets a new queue; what was asked of the old one is asked again.
    let (tx, rx) = crossbeam_channel::unbounded();
    if let Ok(mut requests) = cdn.requests.lock() {
        *requests = Some(tx);
    }
    if let Ok(mut pending) = cdn.pending.lock() {
        pending.clear();
    }
    let spawned = std::thread::Builder::new()
        .name("cdn-assets".into())
        .spawn({
            let cdn = cdn.clone();
            move || asset_cache::sync_blocking(&url, &dir, rx, |event| cdn.apply(event))
        });
    if let Err(e) = spawned {
        cdn.set_status(CdnStatus::Failed(format!("thread: {e}")));
    }
}

/// Seconds to wait before the `attempt`-th retry (0-based): 5, 10, 20, then 30.
pub fn retry_delay_secs(attempt: u32) -> f32 {
    (5.0 * 2f32.powi(attempt.min(3) as i32)).min(30.0)
}

#[derive(Default)]
struct RetryState {
    attempt: u32,
    next_at: Option<f32>,
}

/// While the core tier is missing, tries again with a backoff; there is no embedded copy to use.
fn retry_failed_sync(
    cdn: Res<CdnAssets>,
    config: Res<GameConfig>,
    time: Res<Time>,
    mut state: Local<RetryState>,
) {
    if !matches!(cdn.status(), CdnStatus::Failed(_)) {
        *state = RetryState::default();
        return;
    }
    let now = time.elapsed_secs();
    let first_delay = retry_delay_secs(state.attempt);
    let due = *state.next_at.get_or_insert(now + first_delay);
    if now < due {
        return;
    }
    state.attempt += 1;
    state.next_at = Some(now + retry_delay_secs(state.attempt));
    spawn_sync(&cdn, &config);
}

/// Shows "Baixando conteúdo… n/m" on the start screen while the core tier downloads.
fn show_progress(
    cdn: Res<CdnAssets>,
    mut texts: Query<&mut Text, With<crate::connection::ConnectingText>>,
    mut last: Local<Option<CdnStatus>>,
) {
    let status = cdn.status();
    if last.as_ref() == Some(&status) {
        return;
    }
    let label = match &status {
        CdnStatus::Syncing { done, total } if *total > 0 => {
            format!("Baixando conteúdo… {done}/{total}")
        }
        CdnStatus::Syncing { .. } => "Verificando conteúdo…".to_string(),
        CdnStatus::Failed(reason) => {
            warn!("CDN assets unavailable, retrying: {reason}");
            "Sem conexão com o servidor de conteúdo. Tentando de novo…".to_string()
        }
        _ => "Conectando…".to_string(),
    };
    for mut text in &mut texts {
        **text = label.clone();
    }
    *last = Some(status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::io::file::FileAssetReader;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("resort-cdn-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn read_all(reader: &CdnReader, path: &str) -> Result<Vec<u8>, AssetReaderError> {
        bevy::tasks::block_on(async {
            let mut r = AssetReader::read(reader, Path::new(path)).await?;
            let mut out = Vec::new();
            r.read_to_end(&mut out).await.unwrap();
            Ok(out)
        })
    }

    fn reader_with(dir: &Path, index: &CdnIndex) -> CdnReader {
        CdnReader::new(Box::new(FileAssetReader::new(dir)), index.clone())
    }

    #[test]
    fn indexed_file_is_served_from_the_cache() {
        let embedded = temp_dir("embedded");
        std::fs::write(embedded.join("model.glb"), b"embedded").unwrap();
        let cache = temp_dir("cache");
        std::fs::write(cache.join("model.abcd1234.glb"), b"cdn").unwrap();

        let index = CdnIndex::default();
        let reader = reader_with(&embedded, &index);
        assert_eq!(read_all(&reader, "model.glb").unwrap(), b"embedded");

        index
            .write()
            .unwrap()
            .insert("model.glb".into(), cache.join("model.abcd1234.glb"));
        assert_eq!(read_all(&reader, "model.glb").unwrap(), b"cdn");
    }

    #[test]
    fn missing_cache_file_or_unindexed_path_falls_back_to_embedded() {
        let embedded = temp_dir("fallback");
        std::fs::write(embedded.join("a.glb"), b"embedded-a").unwrap();
        let index = CdnIndex::default();
        index
            .write()
            .unwrap()
            .insert("a.glb".into(), embedded.join("gone.glb"));
        let reader = reader_with(&embedded, &index);
        assert_eq!(read_all(&reader, "a.glb").unwrap(), b"embedded-a");
        assert!(matches!(
            read_all(&reader, "nope.glb"),
            Err(AssetReaderError::NotFound(_))
        ));
    }

    #[test]
    fn events_drive_index_and_status() {
        let cdn = CdnAssets::default();
        assert!(!cdn.status().is_settled());
        cdn.apply(SyncEvent::FileReady {
            name: "x.glb".into(),
            path: PathBuf::from("/c/x.1.glb"),
        });
        cdn.apply(SyncEvent::Progress { done: 1, total: 2 });
        assert_eq!(cdn.status(), CdnStatus::Syncing { done: 1, total: 2 });
        assert!(!cdn.status().is_settled());
        cdn.apply(SyncEvent::CoreReady);
        assert!(cdn.status().is_settled());
        assert_eq!(
            cdn.index.read().unwrap()["x.glb"],
            PathBuf::from("/c/x.1.glb")
        );
    }

    #[test]
    fn failed_core_keeps_the_game_waiting_and_is_retried() {
        let cdn = CdnAssets::default();
        cdn.apply(SyncEvent::Failed("offline".into()));
        assert!(!cdn.status().is_settled());
        // Reads keep waiting while the retries run: the manifest is not known yet.
        assert!(cdn.wait_for("anything.glb"));
        assert_eq!(retry_delay_secs(0), 5.0);
        assert_eq!(retry_delay_secs(1), 10.0);
        assert_eq!(retry_delay_secs(2), 20.0);
        assert_eq!(retry_delay_secs(3), 30.0);
        assert_eq!(retry_delay_secs(40), 30.0);
    }

    fn waiting_reader(dir: &Path, cdn: &CdnAssets) -> CdnReader {
        CdnReader::waiting(Box::new(FileAssetReader::new(dir)), cdn.clone())
    }

    #[test]
    fn a_read_waits_for_a_file_that_is_still_downloading() {
        let bundle = temp_dir("wait-bundle");
        let cache = temp_dir("wait-cache");
        std::fs::write(cache.join("m.1.glb"), b"late").unwrap();
        let cdn = CdnAssets::default();
        cdn.set_status(CdnStatus::Syncing { done: 0, total: 1 });
        let reader = waiting_reader(&bundle, &cdn);

        let delivering = cdn.clone();
        let cached = cache.join("m.1.glb");
        let worker = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(250));
            delivering.apply(SyncEvent::FileReady {
                name: "m.glb".into(),
                path: cached,
            });
        });
        assert_eq!(read_all(&reader, "m.glb").unwrap(), b"late");
        worker.join().unwrap();
    }

    #[test]
    fn a_read_gives_up_on_names_the_manifest_does_not_list() {
        let bundle = temp_dir("giveup-bundle");
        let cdn = CdnAssets::default();
        cdn.apply(SyncEvent::Listed(vec!["known.glb".into()]));
        let reader = waiting_reader(&bundle, &cdn);
        assert!(matches!(
            read_all(&reader, "never.glb"),
            Err(AssetReaderError::NotFound(_))
        ));
        // A disabled CDN never waits either, listed or not.
        let off = CdnAssets::default();
        off.set_status(CdnStatus::Disabled);
        assert!(!off.wait_for("never.glb"));
    }

    #[test]
    fn reading_a_listed_lazy_file_requests_it_once_and_waits_for_it() {
        let bundle = temp_dir("lazy-bundle");
        let cache = temp_dir("lazy-cache");
        std::fs::write(cache.join("hat.1.glb"), b"hat").unwrap();
        let cdn = CdnAssets::default();
        let (tx, rx) = crossbeam_channel::unbounded();
        *cdn.requests.lock().unwrap() = Some(tx);
        cdn.apply(SyncEvent::Listed(vec!["parts/hat.glb".into()]));
        cdn.apply(SyncEvent::CoreReady);
        let reader = waiting_reader(&bundle, &cdn);

        // The "sync thread": answers the request a little later.
        let worker = {
            let cdn = cdn.clone();
            let cached = cache.join("hat.1.glb");
            std::thread::spawn(move || {
                let asked = rx.recv_timeout(Duration::from_secs(5)).unwrap();
                assert_eq!(asked, "parts/hat.glb");
                std::thread::sleep(Duration::from_millis(250));
                cdn.apply(SyncEvent::FileReady {
                    name: asked,
                    path: cached,
                });
                // The reader polls every 100 ms but must not have asked a second time.
                assert!(rx.try_recv().is_err());
            })
        };
        assert_eq!(read_all(&reader, "parts/hat.glb").unwrap(), b"hat");
        worker.join().unwrap();
    }

    #[test]
    fn a_request_is_repeated_only_after_the_retry_window() {
        let cdn = CdnAssets::default();
        let (tx, rx) = crossbeam_channel::unbounded();
        *cdn.requests.lock().unwrap() = Some(tx);
        cdn.request("a.glb");
        cdn.request("a.glb");
        assert_eq!(rx.try_iter().count(), 1);
        // Backdate the first request: it is considered lost and sent again.
        *cdn.pending.lock().unwrap().get_mut("a.glb").unwrap() =
            Instant::now() - REQUEST_RETRY - Duration::from_secs(1);
        cdn.request("a.glb");
        assert_eq!(rx.try_iter().count(), 1);
        // Without a sync thread nothing is sent and nothing is remembered.
        let idle = CdnAssets::default();
        idle.request("b.glb");
        assert!(idle.pending.lock().unwrap().is_empty());
    }

    #[test]
    fn a_waiting_read_survives_a_failed_attempt_and_a_retry() {
        let bundle = temp_dir("retry-bundle");
        let cache = temp_dir("retry-cache");
        std::fs::write(cache.join("x.1.glb"), b"after-retry").unwrap();
        let cdn = CdnAssets::default();
        cdn.set_status(CdnStatus::Syncing { done: 0, total: 1 });
        let reader = waiting_reader(&bundle, &cdn);
        let worker = {
            let cdn = cdn.clone();
            let cached = cache.join("x.1.glb");
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(150));
                cdn.apply(SyncEvent::Failed("offline".into()));
                std::thread::sleep(std::time::Duration::from_millis(250));
                cdn.apply(SyncEvent::FileReady {
                    name: "x.glb".into(),
                    path: cached,
                });
            })
        };
        assert_eq!(read_all(&reader, "x.glb").unwrap(), b"after-retry");
        worker.join().unwrap();
    }

    #[test]
    fn cache_dir_is_per_cdn_host() {
        let config = GameConfig {
            profile_path: Some(PathBuf::from("/data/profile.json")),
            ..GameConfig::default()
        };
        let dev = cache_dir(
            &config,
            "https://assets-dev.joaovictor.eng.br/manifest.json",
        )
        .unwrap();
        let prod = cache_dir(&config, "https://assets.joaovictor.eng.br/manifest.json").unwrap();
        assert_ne!(dev, prod);
        assert!(dev.ends_with("assets-cache/assets-dev_joaovictor_eng_br"));
    }

    #[test]
    fn manifest_url_requires_http_scheme() {
        // SAFETY: tests in this module that touch the variable run on one thread via this single test.
        unsafe { std::env::set_var("ASSETS_MANIFEST_URL", "ftp://nope") };
        let config = GameConfig::default();
        assert_eq!(manifest_url(&config), None);
        unsafe { std::env::remove_var("ASSETS_MANIFEST_URL") };
    }

    /// The pre-registered default source survives `AssetPlugin` and serves indexed files.
    #[test]
    fn registered_default_source_serves_cdn_files_through_the_asset_server() {
        let cache = temp_dir("engine");
        std::fs::write(cache.join("probe.1234.bin"), b"from-cdn").unwrap();

        let mut app = App::new();
        let cdn = install(&mut app);
        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()));
        cdn.apply(SyncEvent::FileReady {
            name: "probe.bin".into(),
            path: cache.join("probe.1234.bin"),
        });

        let server = app.world().resource::<AssetServer>().clone();
        let source = server.get_source(AssetSourceId::Default).unwrap();
        let bytes = bevy::tasks::block_on(async {
            let mut reader = source.reader().read(Path::new("probe.bin")).await.unwrap();
            let mut out = Vec::new();
            reader.read_to_end(&mut out).await.unwrap();
            out
        });
        assert_eq!(bytes, b"from-cdn");
    }

    /// `clips.json` and the catalog reach the game as text assets served by the CDN source, even if
    /// they were requested before the download finished.
    #[test]
    fn json_text_loads_through_the_cdn_source_after_a_late_download() {
        let cache = temp_dir("json");
        std::fs::write(cache.join("clips.1234.json"), br#"{"clips":{}}"#).unwrap();

        let mut app = App::new();
        let cdn = install(&mut app);
        app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
            .init_asset::<JsonText>()
            .register_asset_loader(JsonTextLoader);
        cdn.set_status(CdnStatus::Syncing { done: 0, total: 1 });
        let handle: Handle<JsonText> = app.world().resource::<AssetServer>().load("clips.json");

        app.update();
        assert!(
            app.world()
                .resource::<Assets<JsonText>>()
                .get(&handle)
                .is_none()
        );

        cdn.apply(SyncEvent::FileReady {
            name: "clips.json".into(),
            path: cache.join("clips.1234.json"),
        });
        for _ in 0..200 {
            app.update();
            if app
                .world()
                .resource::<Assets<JsonText>>()
                .get(&handle)
                .is_some()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let text = app
            .world()
            .resource::<Assets<JsonText>>()
            .get(&handle)
            .expect("loaded");
        assert_eq!(text.0, r#"{"clips":{}}"#);
    }
}
