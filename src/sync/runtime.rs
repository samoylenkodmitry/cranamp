use super::config::{self, SyncConfig};
use super::{
    merge, MergedSync, PlayCount, ResumePoint, SyncDocument, SyncTrack, TrackFingerprint,
    UnixSeconds,
};
use cranpose_services::{open_writable_folder, FolderError, Signal, WritableFolderStoreRef};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
const FLUSH_INTERVAL_SECS: u64 = 12;
type SharedStore = WritableFolderStoreRef;
fn store_write(store: &SharedStore, file_name: &str, contents: &str) -> Result<(), FolderError> {
    store.write(file_name, contents.as_bytes())
}
fn store_read_documents(store: &SharedStore) -> Result<Vec<(String, String)>, FolderError> {
    let suffix = format!(".{}", super::SYNC_FILE_EXT);
    let mut documents = Vec::new();
    for entry in store.list()? {
        if !entry.name.ends_with(&suffix) {
            continue;
        }
        if let Ok(bytes) = store.read(&entry.name) {
            documents.push((entry.name, String::from_utf8_lossy(&bytes).into_owned()));
        }
    }
    Ok(documents)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncStatus {
    Disabled,
    NotConfigured,
    Active,
    ReceiveOnly,
    Error(String),
}
impl SyncStatus {
    pub fn summary(&self) -> String {
        match self {
            SyncStatus::Disabled => "Sync is off".to_string(),
            SyncStatus::NotConfigured => "Pick a sync folder to start".to_string(),
            SyncStatus::Active => "Syncing".to_string(),
            SyncStatus::ReceiveOnly => "Receive-only (folder is read-only)".to_string(),
            SyncStatus::Error(message) => format!("Sync error: {message}"),
        }
    }
}
static RUNTIME: Mutex<Option<SyncRuntime>> = Mutex::new(None);
static LATEST_MERGED: Mutex<Option<MergedSync>> = Mutex::new(None);
static WORKER_STARTED: AtomicBool = AtomicBool::new(false);
static FIRST_MERGED: OnceLock<Signal<MergedSync>> = OnceLock::new();
const WORKER_INTERVAL: Duration = Duration::from_secs(5);
fn now() -> UnixSeconds {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
fn build_store(config: &SyncConfig) -> Option<SharedStore> {
    if !config.is_active() {
        return None;
    }
    let folder = config.folder.as_deref()?;
    open_writable_folder(folder)
}
struct SyncRuntime {
    config: SyncConfig,
    store: Option<SharedStore>,
    status: SyncStatus,
    doc: SyncDocument,
    dirty: bool,
    last_flush: UnixSeconds,
}
impl SyncRuntime {
    fn new(config: SyncConfig, store: Option<SharedStore>) -> Self {
        let doc = SyncDocument::new(
            config.device_id.clone(),
            config.device_label.clone(),
            config::current_platform().to_string(),
        );
        let status = if !config.enabled {
            SyncStatus::Disabled
        } else if store.is_none() {
            SyncStatus::NotConfigured
        } else {
            SyncStatus::Active
        };
        Self {
            config,
            store,
            status,
            doc,
            dirty: false,
            last_flush: 0,
        }
    }
    fn record_resume(&mut self, fingerprint: TrackFingerprint, title: String, position_s: f32) {
        if fingerprint.is_empty() {
            return;
        }
        self.doc.resume = Some(ResumePoint {
            fingerprint,
            title,
            position_s: position_s.max(0.0),
            updated_at: now(),
        });
        self.dirty = true;
    }
    fn record_play(&mut self, fingerprint: TrackFingerprint, title: String) {
        if fingerprint.is_empty() {
            return;
        }
        let stamp = now();
        if let Some(existing) = self
            .doc
            .counts
            .iter_mut()
            .find(|play| play.fingerprint == fingerprint)
        {
            existing.count = existing.count.saturating_add(1);
            existing.last_played = stamp;
            if !title.is_empty() {
                existing.title = title;
            }
        } else {
            self.doc.counts.push(PlayCount {
                fingerprint,
                title,
                count: 1,
                last_played: stamp,
            });
        }
        self.dirty = true;
    }
    fn set_playlist(&mut self, tracks: Vec<SyncTrack>) {
        if tracks == self.doc.playlist {
            return;
        }
        self.doc.playlist = tracks;
        self.doc.playlist_updated_at = now();
        self.dirty = true;
    }
    fn prepare_flush(&mut self, force: bool) -> Option<FlushJob> {
        let store = self.store.clone()?;
        if !force && (!self.dirty || now().saturating_sub(self.last_flush) < FLUSH_INTERVAL_SECS) {
            return None;
        }
        self.doc.updated_at = now();
        Some(FlushJob {
            store,
            file_name: self.doc.file_name(),
            contents: super::serialize_document(&self.doc),
        })
    }
    fn finish_flush(&mut self, result: Result<(), FolderError>) {
        match result {
            Ok(()) => {
                self.dirty = false;
                self.last_flush = now();
                self.status = SyncStatus::Active;
            }
            Err(FolderError::ReadOnly) => self.status = SyncStatus::ReceiveOnly,
            Err(other) => self.status = SyncStatus::Error(other.to_string()),
        }
    }
    fn prepare_poll(&self) -> Option<(SharedStore, SyncDocument)> {
        Some((self.store.clone()?, self.doc.clone()))
    }
}
struct FlushJob {
    store: SharedStore,
    file_name: String,
    contents: String,
}
fn sanitized(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
fn config_path() -> PathBuf {
    crate::winamp::sync_config_path()
}
fn load_config() -> SyncConfig {
    let parsed = std::fs::read_to_string(config_path())
        .ok()
        .and_then(|text| config::parse_config(&text));
    match parsed {
        Some(config) => config,
        None => {
            let fresh = SyncConfig::fresh();
            save_config(&fresh);
            fresh
        }
    }
}
fn save_config(config: &SyncConfig) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, config::serialize_config(config));
}
fn with_runtime<T>(f: impl FnOnce(&mut SyncRuntime) -> T) -> Option<T> {
    let mut guard = RUNTIME.lock().ok()?;
    if guard.is_none() {
        let config = load_config();
        let store = build_store(&config);
        *guard = Some(SyncRuntime::new(config, store));
    }
    guard.as_mut().map(f)
}
pub fn ensure_loaded() {
    with_runtime(|_| {});
}
pub fn status() -> SyncStatus {
    with_runtime(|rt| rt.status.clone()).unwrap_or(SyncStatus::Disabled)
}
pub fn config_snapshot() -> Option<SyncConfig> {
    with_runtime(|rt| rt.config.clone())
}
pub fn record_resume(path: &str, title: &str, duration_s: Option<f32>, position_s: f32) {
    let fingerprint = TrackFingerprint::from_path(path, duration_s);
    with_runtime(|rt| rt.record_resume(fingerprint, title.to_string(), position_s));
}
pub fn record_play(path: &str, title: &str, duration_s: Option<f32>) {
    let fingerprint = TrackFingerprint::from_path(path, duration_s);
    with_runtime(|rt| rt.record_play(fingerprint, title.to_string()));
}
pub fn set_playlist(tracks: Vec<SyncTrack>) {
    with_runtime(|rt| rt.set_playlist(tracks));
}
pub fn flush_if_due() {
    run_flush(false);
}
pub fn force_flush() {
    run_flush(true);
}
fn run_flush(force: bool) {
    let Some(Some(job)) = with_runtime(|rt| rt.prepare_flush(force)) else {
        return;
    };
    let result = store_write(&job.store, &job.file_name, &job.contents);
    with_runtime(|rt| rt.finish_flush(result));
}
pub fn poll() -> Option<MergedSync> {
    let (store, own) = with_runtime(|rt| rt.prepare_poll()).flatten()?;
    match store_read_documents(&store) {
        Ok(raw) => {
            let own_id = own.device_id.clone();
            let mut docs: Vec<SyncDocument> = super::parse_documents(&raw)
                .into_iter()
                .filter(|doc| doc.device_id != own_id)
                .collect();
            docs.push(own);
            with_runtime(|rt| {
                if matches!(rt.status, SyncStatus::Error(_)) {
                    rt.status = SyncStatus::Active;
                }
            });
            Some(merge(&docs))
        }
        Err(error) => {
            with_runtime(|rt| rt.status = SyncStatus::Error(error.to_string()));
            None
        }
    }
}
pub fn forget_device(device_id: &str) {
    let store = with_runtime(|rt| rt.store.clone()).flatten();
    if let Some(store) = store {
        let file_name = format!("{}.{}", sanitized(device_id), super::SYNC_FILE_EXT);
        let _ = store.remove(&file_name);
    }
    refresh_merged();
}
pub fn latest_merged() -> Option<MergedSync> {
    LATEST_MERGED.lock().ok().and_then(|slot| slot.clone())
}
fn refresh_merged() {
    if let Some(merged) = poll() {
        if let Ok(mut slot) = LATEST_MERGED.lock() {
            *slot = Some(merged.clone());
        }
        first_merged().set(merged);
    }
}
pub fn first_merged() -> Signal<MergedSync> {
    FIRST_MERGED.get_or_init(Signal::new).clone()
}
pub fn start_worker() {
    if WORKER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("cranamp-sync".to_string())
        .spawn(|| {
            ensure_loaded();
            loop {
                flush_if_due();
                refresh_merged();
                std::thread::sleep(WORKER_INTERVAL);
            }
        })
        .ok();
}
fn reconfigure(mutate: impl FnOnce(&mut SyncConfig)) -> SyncStatus {
    let prepared = with_runtime(|rt| {
        mutate(&mut rt.config);
        save_config(&rt.config);
        rt.store = build_store(&rt.config);
        rt.doc.device_label = rt.config.device_label.clone();
        rt.last_flush = 0;
        rt.status = if !rt.config.enabled {
            SyncStatus::Disabled
        } else if rt.store.is_none() {
            SyncStatus::NotConfigured
        } else {
            SyncStatus::Active
        };
        rt.store.clone()
    })
    .flatten();
    if let Some(store) = prepared {
        seed_from_folder(&store);
        force_flush();
        refresh_merged();
    } else if let Ok(mut slot) = LATEST_MERGED.lock() {
        *slot = None;
    }
    status()
}
fn seed_from_folder(store: &SharedStore) {
    let file_name = with_runtime(|rt| rt.doc.file_name());
    let Some(file_name) = file_name else { return };
    let Ok(all) = store_read_documents(store) else {
        return;
    };
    let Some((_, contents)) = all.iter().find(|(name, _)| name == &file_name) else {
        return;
    };
    let Some(existing) = super::parse_document(contents) else {
        return;
    };
    with_runtime(|rt| {
        if rt.doc.resume.is_none() {
            rt.doc.resume = existing.resume;
        }
        if rt.doc.counts.is_empty() {
            rt.doc.counts = existing.counts;
        }
        if rt.doc.playlist.is_empty() {
            rt.doc.playlist = existing.playlist;
            rt.doc.playlist_updated_at = existing.playlist_updated_at;
        }
    });
}
pub fn set_enabled(enabled: bool) -> SyncStatus {
    reconfigure(|config| config.enabled = enabled)
}
pub fn set_folder(folder: Option<String>) -> SyncStatus {
    reconfigure(|config| config.folder = folder.filter(|f| !f.is_empty()))
}
pub fn set_label(label: String) -> SyncStatus {
    reconfigure(|config| config.device_label = label)
}
#[cfg(test)]
#[path = "../../test/unit/sync/runtime/tests.rs"]
mod tests;
