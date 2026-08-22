//! Process-global sync coordinator.
//!
//! Holds this device's own document (resume point, play counts, playlist),
//! debounces writes into the shared folder, and merges peer documents on demand.
//! Every write is best-effort: a read-only or unreachable folder flips
//! [`SyncStatus`] to `ReceiveOnly`/`Error` and the player keeps running.
//!
//! Folder I/O (which may hit a network mount) is always performed *outside* the
//! global lock: hot paths prepare a snapshot under the lock, do the read/write
//! unlocked, then briefly re-lock to record the outcome. The UI thread therefore
//! never blocks on a slow WebDAV write. Callers should still drive [`flush_if_due`]
//! and [`poll`] from a background worker (see the winamp integration).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cranpose_services::{open_writable_folder, FolderError, Signal, WritableFolderStoreRef};

use super::config::{self, SyncConfig};
use super::{
    merge, MergedSync, PlayCount, ResumePoint, SyncDocument, SyncTrack, TrackFingerprint,
    UnixSeconds,
};

/// Minimum gap between background flushes of this device's document. Keeps
/// network writes infrequent even while a resume point ticks every second.
const FLUSH_INTERVAL_SECS: u64 = 12;

/// Shared, cross-platform writable folder (desktop std::fs / Android SAF),
/// provided by cranpose. Stores one `<device-id>.cransync` document per device.
type SharedStore = WritableFolderStoreRef;

/// Writes a device document (text) into the folder.
fn store_write(store: &SharedStore, file_name: &str, contents: &str) -> Result<(), FolderError> {
    store.write(file_name, contents.as_bytes())
}

/// Reads every `*.cransync` device document as `(file_name, contents)`, skipping
/// any that cannot be read.
fn store_read_documents(store: &SharedStore) -> Result<Vec<(String, String)>, FolderError> {
    let suffix = format!(".{}", super::SYNC_FILE_EXT);
    let mut documents = Vec::new();
    // A listing carries what the provider knows about each file now — name,
    // length, modification time — rather than a bare name, so a caller that
    // wants to skip an unchanged or empty file never has to open it to find out.
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

/// User-facing sync state, surfaced in the Settings "Sync" section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncStatus {
    /// Sync is turned off.
    Disabled,
    /// Enabled but no folder picked yet.
    NotConfigured,
    /// Folder is writable and sync is live.
    Active,
    /// Folder is reachable but read-only — we still read peers, can't publish.
    ReceiveOnly,
    /// Last operation failed; carries a short message.
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

/// Latest merged view, refreshed by the background worker so the UI can read it
/// without doing folder I/O on the render thread.
static LATEST_MERGED: Mutex<Option<MergedSync>> = Mutex::new(None);
static WORKER_STARTED: AtomicBool = AtomicBool::new(false);

/// Carries the first merged view the worker produces to whoever is waiting for
/// it. The worker runs off the render thread and the resume point is wanted
/// exactly once per launch, which is the one-value hand-off [`Signal`] is.
static FIRST_MERGED: OnceLock<Signal<MergedSync>> = OnceLock::new();

/// How often the background worker flushes pending writes and re-reads peers.
const WORKER_INTERVAL: Duration = Duration::from_secs(5);

fn now() -> UnixSeconds {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Opens the cross-platform writable folder for an active config, or `None` when
/// sync is off / unconfigured / unsupported on this platform.
fn build_store(config: &SyncConfig) -> Option<SharedStore> {
    if !config.is_active() {
        return None;
    }
    let folder = config.folder.as_deref()?;
    open_writable_folder(folder)
}

// ---------------------------------------------------------------------------
// Inner runtime
// ---------------------------------------------------------------------------

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
            // Status is refined by the first probe/flush from the worker.
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

    /// Snapshot needed to perform a flush outside the lock, or `None` if there
    /// is nothing to do (no store, or not dirty/not yet due when `force` false).
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

    /// Snapshot for a poll outside the lock: the store and our own in-memory
    /// document (so the merge reflects unflushed local changes).
    fn prepare_poll(&self) -> Option<(SharedStore, SyncDocument)> {
        Some((self.store.clone()?, self.doc.clone()))
    }
}

/// A pending folder write, executed without holding the global lock.
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

// ---------------------------------------------------------------------------
// Config persistence
// ---------------------------------------------------------------------------

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
            // First run: mint a stable identity and persist it immediately so
            // the device keeps the same id across restarts.
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

// ---------------------------------------------------------------------------
// Global API — all folder I/O happens outside the lock
// ---------------------------------------------------------------------------

fn with_runtime<T>(f: impl FnOnce(&mut SyncRuntime) -> T) -> Option<T> {
    let mut guard = RUNTIME.lock().ok()?;
    if guard.is_none() {
        let config = load_config();
        let store = build_store(&config);
        *guard = Some(SyncRuntime::new(config, store));
    }
    guard.as_mut().map(f)
}

/// Loads config and builds the store on first call (idempotent). Prefer calling
/// this from a background worker so the (possibly slow) first folder touch does
/// not land on the render thread.
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

/// Flushes the device document if dirty and the debounce interval has elapsed.
pub fn flush_if_due() {
    run_flush(false);
}

/// Flushes immediately regardless of debounce (e.g. on app pause / config change).
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

/// Reads every peer document, merges, and returns the combined view. Substitutes
/// our own in-memory document for the on-disk copy so unflushed changes show.
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

/// The most recent merged view across devices (worker-maintained; cheap read).
pub fn latest_merged() -> Option<MergedSync> {
    LATEST_MERGED.lock().ok().and_then(|slot| slot.clone())
}

/// Re-reads peers and caches the merged result. Does folder I/O — callers on the
/// UI thread should reserve this for explicit user actions; the worker handles
/// the steady-state cadence.
fn refresh_merged() {
    if let Some(merged) = poll() {
        if let Ok(mut slot) = LATEST_MERGED.lock() {
            *slot = Some(merged.clone());
        }
        // A second `set` is ignored, so this delivers the launch-time view and
        // nothing after it.
        first_merged().set(merged);
    }
}

/// Resolves with the first merged view across devices. Awaiting this is how the
/// composition picks up the resume point without watching the frame clock.
pub fn first_merged() -> Signal<MergedSync> {
    FIRST_MERGED.get_or_init(Signal::new).clone()
}

/// Starts the background sync worker exactly once. It loads config, then on a
/// fixed interval flushes any pending document and refreshes the merged view.
/// All folder I/O lives here, off the render thread.
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

/// Applies a config change: persists it, rebuilds the store, re-seeds this
/// device's document from any existing file in the new folder, and flushes once.
/// Returns the new status. Called on explicit user action (toggle / pick folder).
fn reconfigure(mutate: impl FnOnce(&mut SyncConfig)) -> SyncStatus {
    // Phase 1 (locked): mutate + persist config, rebuild store, reset doc.
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

    // Phase 2 (unlocked): seed + flush against the (maybe network) folder. The
    // first `force_flush` is the authoritative writability check — its actual
    // write sets `Active` or `ReceiveOnly` via `finish_flush`. We deliberately do
    // not pre-probe with `is_writable()`: on a slow/flaky network provider that
    // extra round-trip both doubles the enable latency and can transiently report
    // a writable folder (a writable WebDAV mount) as read-only before the real
    // write proves otherwise.
    if let Some(store) = prepared {
        seed_from_folder(&store);
        force_flush();
        refresh_merged();
    } else if let Ok(mut slot) = LATEST_MERGED.lock() {
        // Sync turned off / unconfigured — drop the stale device list.
        *slot = None;
    }
    status()
}

/// Restores our own resume/counts/playlist from a document we previously wrote,
/// so history survives a restart and a re-enable picks up where we left off.
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
mod tests {
    use super::*;
    use cranpose_services::{FolderEntry, FolderReader, FolderWriter, WritableFolderStore};
    use std::sync::{Arc, Mutex as StdMutex};

    /// The mock folder's file table: name to bytes, shared between the store
    /// and whatever writer is currently committing into it.
    type MockFiles = Arc<StdMutex<Vec<(String, Vec<u8>)>>>;

    /// In-memory writable folder so the inner runtime can be exercised without a
    /// real filesystem or the process-global state.
    #[derive(Default)]
    struct MockStore {
        // Shared so a writer handed out by `open_write` can commit into it when
        // it finishes, the way a real provider's writer commits to its folder.
        files: MockFiles,
        writable: bool,
    }

    impl MockStore {
        fn new(writable: bool) -> Arc<Self> {
            Arc::new(Self {
                files: Arc::new(StdMutex::new(Vec::new())),
                writable,
            })
        }
    }

    struct OneChunkReader(Option<Vec<u8>>);

    impl FolderReader for OneChunkReader {
        fn read_chunk(&mut self) -> Result<Option<Vec<u8>>, FolderError> {
            Ok(self.0.take())
        }
    }

    struct CollectingWriter {
        store: MockFiles,
        name: String,
        buffer: Vec<u8>,
    }

    impl FolderWriter for CollectingWriter {
        fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FolderError> {
            self.buffer.extend_from_slice(bytes);
            Ok(())
        }

        fn finish(self: Box<Self>) -> Result<(), FolderError> {
            let mut files = self.store.lock().unwrap();
            files.retain(|(name, _)| name != &self.name);
            files.push((self.name, self.buffer));
            Ok(())
        }
    }

    impl WritableFolderStore for MockStore {
        fn write(&self, name: &str, contents: &[u8]) -> Result<(), FolderError> {
            if !self.writable {
                return Err(FolderError::ReadOnly);
            }
            let mut files = self.files.lock().unwrap();
            files.retain(|(existing, _)| existing != name);
            files.push((name.to_string(), contents.to_vec()));
            Ok(())
        }
        fn read(&self, name: &str) -> Result<Vec<u8>, FolderError> {
            self.files
                .lock()
                .unwrap()
                .iter()
                .find(|(existing, _)| existing == name)
                .map(|(_, bytes)| bytes.clone())
                .ok_or_else(|| FolderError::NotFound(name.to_string()))
        }
        fn list(&self) -> Result<Vec<FolderEntry>, FolderError> {
            Ok(self
                .files
                .lock()
                .unwrap()
                .iter()
                .map(|(name, contents)| FolderEntry {
                    name: name.clone(),
                    len: contents.len() as u64,
                    modified_millis: None,
                })
                .collect())
        }

        // The store reads and writes whole files in this test, so its streaming
        // operations are the chunked shape over exactly one chunk. A real
        // provider streams; a mock only has to keep the contract.
        fn open_read(&self, name: &str) -> Result<Box<dyn FolderReader>, FolderError> {
            let bytes = self.read(name)?;
            Ok(Box::new(OneChunkReader(Some(bytes))))
        }

        fn open_write(&self, name: &str) -> Result<Box<dyn FolderWriter>, FolderError> {
            if !self.writable {
                return Err(FolderError::ReadOnly);
            }
            Ok(Box::new(CollectingWriter {
                store: Arc::clone(&self.files),
                name: name.to_string(),
                buffer: Vec::new(),
            }))
        }
        fn remove(&self, name: &str) -> Result<(), FolderError> {
            self.files
                .lock()
                .unwrap()
                .retain(|(existing, _)| existing != name);
            Ok(())
        }
        fn is_writable(&self) -> bool {
            self.writable
        }
        fn handle(&self) -> String {
            "/mock".to_string()
        }
    }

    fn runtime_with(store: Arc<MockStore>) -> SyncRuntime {
        let config = SyncConfig {
            device_id: "self".to_string(),
            device_label: "Me".to_string(),
            enabled: true,
            folder: Some("/mock".to_string()),
        };
        SyncRuntime::new(config, Some(store))
    }

    /// Drives a flush against the inner runtime + store directly (mirrors the
    /// unlocked global `run_flush`, without touching the process-global state).
    fn drive_flush(rt: &mut SyncRuntime, force: bool) {
        if let Some(job) = rt.prepare_flush(force) {
            let result = store_write(&job.store, &job.file_name, &job.contents);
            rt.finish_flush(result);
        }
    }

    fn drive_poll(rt: &SyncRuntime) -> MergedSync {
        let (store, own) = rt.prepare_poll().expect("store");
        let raw = store_read_documents(&store).unwrap();
        let own_id = own.device_id.clone();
        let mut docs: Vec<SyncDocument> = super::super::parse_documents(&raw)
            .into_iter()
            .filter(|doc| doc.device_id != own_id)
            .collect();
        docs.push(own);
        merge(&docs)
    }

    #[test]
    fn flush_writes_one_file_and_merges_peers() {
        let store = MockStore::new(true);
        let mut peer = SyncDocument::new("peer".into(), "Phone".into(), "android".into());
        peer.updated_at = 5;
        peer.resume = Some(ResumePoint {
            fingerprint: TrackFingerprint::new("song.mp3", 200),
            title: "Song".into(),
            position_s: 30.0,
            updated_at: 5,
        });
        peer.counts = vec![PlayCount {
            fingerprint: TrackFingerprint::new("song.mp3", 200),
            title: "Song".into(),
            count: 2,
            last_played: 5,
        }];
        store
            .write(
                "peer.cransync",
                super::super::serialize_document(&peer).as_bytes(),
            )
            .unwrap();

        let mut rt = runtime_with(store.clone());
        rt.record_play(TrackFingerprint::new("song.mp3", 200), "Song".into());
        drive_flush(&mut rt, true);

        // One file per device: our own plus the seeded peer.
        assert_eq!(store.list().unwrap().len(), 2);

        let merged = drive_poll(&rt);
        let entry = merged
            .counts
            .get(&TrackFingerprint::new("song.mp3", 200))
            .expect("count");
        assert_eq!(entry.count, 3); // peer 2 + ours 1
        assert_eq!(merged.resume.unwrap().fingerprint.basename, "song.mp3");
    }

    #[test]
    fn read_only_store_degrades_to_receive_only() {
        let store = MockStore::new(false);
        let mut rt = runtime_with(store);
        rt.record_resume(TrackFingerprint::new("a.mp3", 10), "A".into(), 3.0);
        drive_flush(&mut rt, true);
        assert_eq!(rt.status, SyncStatus::ReceiveOnly);
    }

    #[test]
    fn debounce_blocks_until_forced() {
        let store = MockStore::new(true);
        let mut rt = runtime_with(store.clone());
        rt.record_play(TrackFingerprint::new("x.mp3", 100), "X".into());
        // Not yet due (last_flush just initialized to now-ish via 0 < interval).
        rt.last_flush = now();
        assert!(rt.prepare_flush(false).is_none());
        // Forced flush always proceeds.
        drive_flush(&mut rt, true);
        assert_eq!(store.list().unwrap().len(), 1);
    }
}
