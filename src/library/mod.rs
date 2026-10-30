//! A local metadata index. Audio stays at its original URI; only thumbnails are cached.
use crate::audio::Track;
use coroflow::MutableStateFlow;
#[cfg(not(target_arch = "wasm32"))]
use coroflow::StateFlow;
use cranpose_services::{ContentHandle, ContentMetadata};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    rc::Rc,
};

#[cfg(not(target_arch = "wasm32"))]
mod source;
mod tags;

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const INDEX_LIMIT: usize = 32 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u64>,
    pub disc_number: Option<u64>,
    pub duration: Option<f32>,
    pub artwork: Option<String>,
    folder: String,
    fingerprint: Fingerprint,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Fingerprint {
    len: Option<u64>,
    modified: Option<u64>,
}
impl From<&ContentMetadata> for Fingerprint {
    fn from(m: &ContentMetadata) -> Self {
        Self {
            len: m.len,
            modified: m.modified_millis,
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Cover {
    uri: String,
    rank: (u8, String),
    fingerprint: Fingerprint,
    thumbnail: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Index {
    version: u32,
    tracks: HashMap<String, Record>,
    covers: HashMap<String, Cover>,
    // Derived from embedded artwork; never persist fallback artwork as track metadata.
    #[serde(skip)]
    album_artwork: HashMap<String, BTreeMap<String, String>>,
}
impl Index {
    fn insert_track(&mut self, uri: String, record: Record) {
        if let Some(previous) = self.tracks.get(&uri) {
            let key = album_key(previous);
            if let Some(artwork) = self.album_artwork.get_mut(&key) {
                artwork.remove(&uri);
                if artwork.is_empty() {
                    self.album_artwork.remove(&key);
                }
            }
        }
        self.index_artwork(&uri, &record);
        self.tracks.insert(uri, record);
    }

    fn index_artwork(&mut self, uri: &str, record: &Record) {
        if let Some(artwork) = &record.artwork {
            if !record.folder.is_empty() {
                // URI order keeps the album cover stable across discovery order and restarts.
                self.album_artwork
                    .entry(album_key(record))
                    .or_default()
                    .insert(uri.to_owned(), artwork.clone());
            }
        }
    }
}
#[derive(Clone)]
struct Job {
    uri: String,
    name: String,
    folder: String,
    fingerprint: Fingerprint,
    image: bool,
    #[cfg(target_arch = "wasm32")]
    content: Option<ContentHandle>,
}
struct Runtime {
    root: Option<PathBuf>,
    index: Index,
    loaded: bool,
    running: bool,
    scope: Option<cranpose_core::CoroutineScope>,
    queue: VecDeque<Job>,
    seen: HashMap<String, Fingerprint>,
    folders: std::collections::HashSet<String>,
    revision: MutableStateFlow<u64>,
    generation: u64,
    #[cfg(not(target_arch = "wasm32"))]
    images: VecDeque<(String, cranpose_ui::ImageBitmap)>,
}
impl Runtime {
    fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            index: Index::default(),
            loaded: false,
            running: false,
            scope: None,
            queue: VecDeque::new(),
            seen: HashMap::new(),
            folders: Default::default(),
            revision: MutableStateFlow::new(0),
            generation: 0,
            #[cfg(not(target_arch = "wasm32"))]
            images: VecDeque::new(),
        }
    }
    fn changed(&mut self) {
        self.generation += 1;
        self.revision.set(self.generation);
    }
}
thread_local! { static RUNTIME: RefCell<Option<Rc<RefCell<Runtime>>>> = const { RefCell::new(None) }; }
fn runtime() -> Rc<RefCell<Runtime>> {
    RUNTIME.with(|slot| {
        slot.borrow_mut()
            .get_or_insert_with(|| {
                let root = cranpose_services::application_directories()
                    .ok()
                    .map(|d| d.cache.join("library-v1"));
                Rc::new(RefCell::new(Runtime::new(root)))
            })
            .clone()
    })
}
pub(crate) fn begin_scan() {
    let rt = runtime();
    let mut data = rt.borrow_mut();
    data.folders.clear();
    data.seen = data
        .queue
        .iter()
        .map(|job| (job.uri.clone(), job.fingerprint.clone()))
        .collect();
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn indexing() -> bool {
    runtime().borrow().running
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn revision() -> StateFlow<u64> {
    runtime().borrow().revision.as_state_flow()
}
pub(crate) fn record(track: &Track) -> Record {
    let rt = runtime();
    let rt = rt.borrow();
    let uri = track.path.as_deref().unwrap_or("");
    let mut result = rt.index.tracks.get(uri).cloned().unwrap_or_else(|| Record {
        folder: parent(uri),
        ..Default::default()
    });
    if result.disc_number.is_none() {
        result.disc_number = folder_disc_number(&result.folder);
    }
    if result.album.is_none() {
        result.album = folder_name(&album_folder(&result.folder));
    }
    if result.artwork.is_none() {
        result.artwork = rt
            .index
            .covers
            .get(&result.folder)
            .or_else(|| rt.index.covers.get(&album_folder(&result.folder)))
            .map(|cover| cover.thumbnail.clone())
            .or_else(|| {
                rt.index
                    .album_artwork
                    .get(&album_key(&result))?
                    .first_key_value()
                    .map(|(_, artwork)| artwork.clone())
            });
    }
    result
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn title(track: &Track) -> String {
    record(track)
        .title
        .unwrap_or_else(|| filename_title(&track.title))
}
#[cfg(not(target_arch = "wasm32"))]
fn filename_title(name: &str) -> String {
    let original = name;
    let name =
        name.trim_start_matches(|c: char| c.is_ascii_digit() || matches!(c, '.' | '-' | '_' | ' '));
    let title = name
        .split_once(" - ")
        .map_or(name, |(_, title)| title)
        .trim();
    if title.is_empty() {
        original.to_owned()
    } else {
        title.to_owned()
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn artist(track: &Track) -> String {
    let meta = record(track);
    meta.artist
        .or_else(|| {
            let title = track.title.trim_start_matches(|c: char| {
                c.is_ascii_digit() || matches!(c, '.' | '-' | '_' | ' ')
            });
            title
                .split_once(" - ")
                .map(|(artist, _)| artist.trim().to_owned())
        })
        .or(meta.album)
        .unwrap_or_else(|| "Audio track".into())
}
pub(crate) fn duration(track: &Track) -> Option<f32> {
    record(track).duration.or(track.duration_seconds)
}
fn parent(uri: &str) -> String {
    if uri.starts_with("blob:") || uri.starts_with("data:") {
        return String::new();
    }
    if let Some(path) = cranpose_services::path_from_uri(uri) {
        return path
            .parent()
            .map_or(String::new(), |p| p.to_string_lossy().replace('\\', "/"));
    }
    let uri = if uri.starts_with("http:") || uri.starts_with("https:") {
        uri.split(['?', '#']).next().unwrap_or(uri)
    } else {
        uri
    };
    cranpose_services::percent_decode_lossy(uri)
        .replace('\\', "/")
        .rsplit_once('/')
        .map_or(String::new(), |(parent, _)| parent.to_owned())
}
fn folder_name(folder: &str) -> Option<String> {
    folder
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}
fn folder_disc_number(folder: &str) -> Option<u64> {
    let last = folder_name(folder)?.to_ascii_lowercase();
    last.strip_prefix("disc")
        .or_else(|| last.strip_prefix("disk"))
        .or_else(|| last.strip_prefix("cd"))?
        .trim()
        .parse()
        .ok()
}
fn album_folder(folder: &str) -> String {
    if folder_disc_number(folder).is_some() {
        parent(folder)
    } else {
        folder.to_owned()
    }
}
fn album_key(record: &Record) -> String {
    let folder = album_folder(&record.folder);
    let title = record
        .album
        .clone()
        .or_else(|| folder_name(&folder))
        .unwrap_or_else(|| "Unknown album".into());
    // Use the same identity for grouping and artwork, including multi-disc folders.
    // Folder identity keeps separate releases with the same title apart.
    format!("{}\n{}", folder, title.to_lowercase())
}

fn image_rank(name: &str) -> Option<(u8, String)> {
    let lower = name.to_lowercase();
    let (stem, extension) = lower.rsplit_once('.')?;
    if !matches!(extension, "jpg" | "jpeg" | "png" | "bmp" | "webp") {
        return None;
    }
    let rank = match stem {
        "cover" => 0,
        "folder" => 1,
        "front" | "album" | "albumart" | "albumartsmall" => 2,
        _ if stem.contains("front") || stem.contains("cover") => 3,
        _ if stem.contains("back") || stem.contains("disc") || stem.contains("booklet") => 5,
        _ => 4,
    };
    Some((rank, lower))
}

/// Called from folder discovery before dropping provider handles. Never reads audio here.
pub(crate) fn observe(entry: ContentHandle, track: Option<&Track>) {
    let metadata = entry.metadata();
    let image = image_rank(&metadata.name).is_some();
    if track.is_none() && !image {
        return;
    }
    let uri = match track
        .and_then(|track| track.path.clone())
        .or_else(|| entry.media_uri().ok())
    {
        Some(uri) => uri,
        None => return,
    };
    let identifier = if metadata.identifier.contains('/') || metadata.identifier.contains('\\') {
        &metadata.identifier
    } else {
        &uri
    };
    let folder = parent(identifier);
    if metadata.modified_millis.is_none() {
        let rt = runtime();
        let mut data = rt.borrow_mut();
        if !data.queue.iter().any(|job| job.uri == uri) {
            data.seen.remove(&uri);
        }
    }
    enqueue(Job {
        uri,
        name: metadata.name.clone(),
        folder,
        fingerprint: Fingerprint::from(&metadata),
        image,
        #[cfg(target_arch = "wasm32")]
        content: Some(entry),
    });
}
pub(crate) fn enqueue_tracks(tracks: &[Track], scope: cranpose_core::CoroutineScope) {
    let root = cranpose_services::application_directories()
        .ok()
        .map(|d| d.cache.join("library-v1"));
    if runtime().borrow().root != root {
        RUNTIME.with(|slot| *slot.borrow_mut() = Some(Rc::new(RefCell::new(Runtime::new(root)))));
    }
    runtime().borrow_mut().scope = Some(scope);
    for track in tracks {
        let Some(uri) = &track.path else {
            continue;
        };
        enqueue(Job {
            uri: uri.clone(),
            name: uri.split(['?', '#']).next().unwrap_or(uri).to_owned(),
            folder: parent(uri),
            fingerprint: Fingerprint::default(),
            image: false,
            #[cfg(target_arch = "wasm32")]
            content: cranpose_services::resolve_content(uri),
        });
    }
    kick(runtime());
}
fn enqueue(job: Job) {
    let rt = runtime();
    let mut data = rt.borrow_mut();
    // A provider's version wins over the unversioned restored-playlist request.
    if data
        .seen
        .get(&job.uri)
        .is_some_and(|old| *old == job.fingerprint || job.fingerprint == Fingerprint::default())
    {
        return;
    }
    data.seen.insert(job.uri.clone(), job.fingerprint.clone());
    if job.image {
        data.queue.push_front(job);
    } else {
        data.queue.push_back(job);
    }
    drop(data);
    kick(rt);
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn prioritize(track: &Track) {
    let rt = runtime();
    let mut data = rt.borrow_mut();
    if let Some(at) = data
        .queue
        .iter()
        .position(|job| Some(&job.uri) == track.path.as_ref())
    {
        if let Some(job) = data.queue.remove(at) {
            data.queue.push_front(job);
        }
    }
}
fn kick(rt: Rc<RefCell<Runtime>>) {
    let mut data = rt.borrow_mut();
    if data.running || data.queue.is_empty() {
        return;
    }
    let Some(scope) = data.scope.clone() else {
        return;
    };
    data.running = true;
    drop(data);
    scope.launch(async move {
        run(rt).await;
    });
}
struct Worker(Rc<RefCell<Runtime>>);
impl Drop for Worker {
    fn drop(&mut self) {
        self.0.borrow_mut().running = false;
    }
}
async fn run(rt: Rc<RefCell<Runtime>>) {
    let worker = Worker(rt.clone());
    let root = rt.borrow().root.clone();
    if !rt.borrow().loaded {
        let load_root = root.clone();
        let index = background(move || load_index(load_root.as_deref()))
            .await
            .unwrap_or_default();
        let mut data = rt.borrow_mut();
        data.index = index;
        data.loaded = true;
        data.changed();
    }
    let mut since_save = 0;
    loop {
        #[allow(
            unused_mut,
            reason = "Native file inspection refreshes the fingerprint"
        )]
        let Some(mut job) = rt.borrow_mut().queue.pop_front() else {
            break;
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !job.image
                && (job.uri.starts_with("http://") || job.uri.starts_with("https://"))
                && rt.borrow_mut().folders.insert(job.folder.clone())
            {
                discover_network_covers(&job.uri).await;
            }
            let path = cranpose_services::path_from_uri(&job.uri);
            if let Some(path) = path {
                let inspect_folder = rt.borrow_mut().folders.insert(job.folder.clone());
                let found = background(move || {
                    let fingerprint = std::fs::metadata(&path).ok().map(|m| Fingerprint {
                        len: Some(m.len()),
                        modified: m
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_millis() as u64),
                    });
                    let mut images = Vec::new();
                    let mut scanned = Vec::new();
                    if inspect_folder {
                        if let Some(folder) = path.parent() {
                            // Include the album's root for Disc 1 / Disc 2 layouts.
                            let mut folders = vec![folder.to_path_buf()];
                            let folder_key = folder.to_string_lossy().replace('\\', "/");
                            if album_folder(&folder_key) != folder_key {
                                if let Some(parent) = folder.parent() {
                                    folders.push(parent.to_path_buf());
                                }
                            }
                            for folder in folders {
                                if let Ok(entries) = std::fs::read_dir(&folder) {
                                    scanned.push(folder.to_string_lossy().replace('\\', "/"));
                                    for entry in entries.flatten() {
                                        let name = entry.file_name().to_string_lossy().into_owned();
                                        if image_rank(&name).is_some() {
                                            let uri =
                                                cranpose_services::uri_for_path(&entry.path());
                                            images.push(Job {
                                                folder: parent(&uri),
                                                uri,
                                                name,
                                                fingerprint: Fingerprint::default(),
                                                image: true,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    (fingerprint, images, scanned)
                })
                .await;
                if let Some((fingerprint, images, scanned)) = found {
                    let mut data = rt.borrow_mut();
                    let before = data.index.covers.len();
                    data.index.covers.retain(|folder, cover| {
                        !scanned.contains(folder)
                            || cranpose_services::path_from_uri(&cover.uri).is_none()
                            || images.iter().any(|image| image.uri == cover.uri)
                    });
                    if before != data.index.covers.len() {
                        data.changed();
                        since_save += 1;
                    }
                    drop(data);
                    if let Some(fingerprint) = fingerprint {
                        job.fingerprint = fingerprint;
                    }
                    for image in images {
                        enqueue(image);
                    }
                }
            }
        }
        let (cached, cached_artwork) = {
            let data = rt.borrow();
            let cached = if job.image {
                data.index.covers.get(&job.folder).is_some_and(|cover| {
                    image_rank(&job.name).is_some_and(|rank| rank > cover.rank)
                        || (cover.uri == job.uri && cover.fingerprint == job.fingerprint)
                })
            } else {
                data.index.tracks.get(&job.uri).is_some_and(|record| {
                    job.fingerprint.modified.is_some() && record.fingerprint == job.fingerprint
                })
            };
            let key = if job.image {
                data.index
                    .covers
                    .get(&job.folder)
                    .map(|c| c.thumbnail.clone())
            } else {
                data.index
                    .tracks
                    .get(&job.uri)
                    .and_then(|r| r.artwork.clone())
            };
            (cached, key)
        };
        if cached {
            let cache_root = root.clone();
            if background(move || {
                cached_artwork.is_none_or(|key| thumbnail_exists(cache_root.as_deref(), &key))
            })
            .await
            .unwrap_or(false)
            {
                continue;
            }
        }
        let result = inspect(&job, root.clone()).await;
        if let Some((mut record, thumbnail)) = result {
            let mut data = rt.borrow_mut();
            if job.image {
                if let (Some(thumbnail), Some(rank)) = (thumbnail, image_rank(&job.name)) {
                    if data
                        .index
                        .covers
                        .get(&job.folder)
                        .is_none_or(|cover| rank <= cover.rank)
                    {
                        data.index.covers.insert(
                            job.folder,
                            Cover {
                                uri: job.uri,
                                rank,
                                fingerprint: job.fingerprint,
                                thumbnail,
                            },
                        );
                    }
                }
            } else {
                record.folder = job.folder;
                record.fingerprint = job.fingerprint;
                record.artwork = thumbnail;
                if data.index.tracks.len() < MAX_RECORDS || data.index.tracks.contains_key(&job.uri)
                {
                    data.index.insert_track(job.uri, record);
                }
            }
            since_save += 1;
            if since_save == 1 || since_save % 16 == 0 {
                data.changed();
            }
        }
        if since_save >= 64 {
            persist(&rt, false).await;
            since_save = 0;
        }
        coroflow::yield_now().await;
    }
    if since_save > 0 {
        rt.borrow_mut().changed();
        persist(&rt, true).await;
    }
    drop(worker);
    rt.borrow_mut().changed();
    // Entries can arrive during the final disk write.
    kick(rt);
}
#[cfg(not(target_arch = "wasm32"))]
async fn discover_network_covers(uri: &str) {
    let uri = uri.split(['?', '#']).next().unwrap_or(uri);
    let Some((parent, _)) = uri.rsplit_once('/') else {
        return;
    };
    let Ok(folder) = cranpose_services::WebDavFolder::new(
        &format!("{parent}/"),
        cranpose_services::default_http_client(),
    ) else {
        return;
    };
    let folder = folder.handle();
    let entries = futures_lite::future::race(async { folder.entries().await.ok() }, async {
        cranpose_core::delay(std::time::Duration::from_secs(8)).await;
        None
    })
    .await;
    if let Some(entries) = entries {
        for entry in entries {
            if let cranpose_services::ContentEntry::File(content) = entry {
                if image_rank(&content.metadata().name).is_some() {
                    observe(content, None);
                }
            }
        }
    }
}
async fn inspect(job: &Job, root: Option<PathBuf>) -> Option<(Record, Option<String>)> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let job = job.clone();
        background(move || {
            let source = source::open(&job.uri).ok()?;
            let (record, thumb) = if job.image {
                use std::io::Read;
                let mut bytes = Vec::new();
                source
                    .take(MAX_IMAGE_BYTES as u64 + 1)
                    .read_to_end(&mut bytes)
                    .ok()?;
                (Record::default(), Some(thumbnail(&bytes).ok()?))
            } else {
                tags::read(Box::new(source), &job.name).ok()?
            };
            let key = thumb.and_then(|bytes| store_thumbnail(root.as_deref(), &bytes).ok());
            Some((record, key))
        })
        .await
        .flatten()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let content = job
            .content
            .clone()
            .or_else(|| cranpose_services::resolve_content(&job.uri))?;
        let reader = content.open().await.ok()?;
        let mut bytes = Vec::new();
        while bytes.len() < MAX_IMAGE_BYTES {
            let Some(chunk) = reader.read_chunk().await.ok()? else {
                break;
            };
            let take = chunk.len().min(MAX_IMAGE_BYTES - bytes.len());
            bytes.extend_from_slice(&chunk[..take]);
        }
        let (record, thumb) = if job.image {
            (Record::default(), Some(thumbnail(&bytes).ok()?))
        } else {
            tags::read(Box::new(std::io::Cursor::new(bytes)), &job.name).ok()?
        };
        let key = thumb.and_then(|bytes| store_thumbnail(root.as_deref(), &bytes).ok());
        Some((record, key))
    }
}
async fn background<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        cranpose_core::withBlocking(work).await.ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        Some(work())
    }
}
fn thumbnail(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(bytes.len() <= MAX_IMAGE_BYTES, "Artwork exceeds limit");
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let (width, height) = reader.into_dimensions()?;
    anyhow::ensure!(
        width <= 4096
            && height <= 4096
            && u64::from(width) * u64::from(height) * 4 <= 64 * 1024 * 1024,
        "Artwork dimensions exceed limit"
    );
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let thumb = reader.decode()?.thumbnail(320, 320);
    let mut bytes = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(bytes.into_inner())
}
fn thumbnail_exists(root: Option<&std::path::Path>, key: &str) -> bool {
    if !valid_key(key) {
        return false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        root.is_some_and(|root| root.join(format!("{key}.png")).is_file())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = root;
        cranpose_services::preferences()
            .get(&format!("cranamp.thumb.{key}"))
            .is_some()
    }
}
fn valid_key(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit())
}
fn store_thumbnail(root: Option<&std::path::Path>, bytes: &[u8]) -> anyhow::Result<String> {
    let key: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let root = root.ok_or_else(|| anyhow::anyhow!("No application cache"))?;
        std::fs::create_dir_all(root)?;
        atomic_write(&root.join(format!("{key}.png")), bytes)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use base64::Engine;
        let _ = root;
        cranpose_services::preferences().set(
            &format!("cranamp.thumb.{key}"),
            &base64::engine::general_purpose::STANDARD.encode(bytes),
        )?;
    }
    Ok(key)
}
#[cfg(not(target_arch = "wasm32"))]
fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path)
}
fn load_index(root: Option<&std::path::Path>) -> Index {
    let load = || -> Option<Index> {
        #[cfg(not(target_arch = "wasm32"))]
        let bytes = {
            use std::io::Read;
            let file = std::fs::File::open(root?.join("index.json")).ok()?;
            if file.metadata().ok()?.len() > INDEX_LIMIT as u64 {
                return None;
            }
            let mut bytes = Vec::new();
            file.take(INDEX_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            bytes
        };
        #[cfg(target_arch = "wasm32")]
        let bytes = {
            let _ = root;
            cranpose_services::preferences()
                .get("cranamp.library.v1")?
                .into_bytes()
        };
        if bytes.len() > INDEX_LIMIT {
            return None;
        }
        let index: Index = serde_json::from_slice(&bytes).ok()?;
        (index.version == 1 && index.tracks.len() <= MAX_RECORDS).then_some(index)
    };
    let mut index = load().unwrap_or_default();
    let tracks = std::mem::take(&mut index.tracks);
    for (uri, record) in &tracks {
        index.index_artwork(uri, record);
    }
    index.tracks = tracks;
    index
}
async fn persist(rt: &Rc<RefCell<Runtime>>, prune: bool) {
    let (root, index) = {
        let mut data = rt.borrow_mut();
        data.index.version = 1;
        (data.root.clone(), data.index.clone())
    };
    background(move || {
        let Ok(bytes) = serde_json::to_vec(&index) else {
            return;
        };
        if bytes.len() > INDEX_LIMIT {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(root) = root {
                let _ = std::fs::create_dir_all(&root);
                if atomic_write(&root.join("index.json"), &bytes).is_ok() && prune {
                    prune_thumbnails(&root, &index);
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (root, prune);
            if let Ok(text) = String::from_utf8(bytes) {
                let _ = cranpose_services::preferences().set("cranamp.library.v1", &text);
            }
        }
    })
    .await;
}
#[cfg(not(target_arch = "wasm32"))]
fn prune_thumbnails(root: &std::path::Path, index: &Index) {
    let live: std::collections::HashSet<&str> = index
        .tracks
        .values()
        .filter_map(|r| r.artwork.as_deref())
        .chain(index.covers.values().map(|c| c.thumbnail.as_str()))
        .collect();
    let Ok(files) = std::fs::read_dir(root) else {
        return;
    };
    let mut kept = Vec::new();
    let mut size = 0u64;
    for entry in files.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "png") {
            continue;
        }
        let Some(key) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !valid_key(key) {
            continue;
        }
        if !live.contains(key) {
            let _ = std::fs::remove_file(path);
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            size += meta.len();
            kept.push((meta.modified().ok(), meta.len(), path));
        }
    }
    kept.sort_by_key(|(modified, _, _)| *modified);
    for (_, length, path) in kept {
        if size <= 256 * 1024 * 1024 {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            size = size.saturating_sub(length);
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn artwork(key: String) -> Option<cranpose_ui::ImageBitmap> {
    let rt = runtime();
    if let Some((_, image)) = rt.borrow().images.iter().find(|(id, _)| *id == key) {
        return Some(image.clone());
    }
    if !valid_key(&key) {
        return None;
    }
    let root = rt.borrow().root.clone();
    let read_key = key.clone();
    let image = background(move || {
        #[cfg(not(target_arch = "wasm32"))]
        let bytes = {
            use std::io::Read;
            let file = std::fs::File::open(root?.join(format!("{read_key}.png"))).ok()?;
            let mut bytes = Vec::new();
            file.take(1024 * 1024 + 1).read_to_end(&mut bytes).ok()?;
            if bytes.len() > 1024 * 1024 {
                return None;
            }
            bytes
        };
        #[cfg(target_arch = "wasm32")]
        let bytes = {
            use base64::Engine;
            let _ = root;
            base64::engine::general_purpose::STANDARD
                .decode(cranpose_services::preferences().get(&format!("cranamp.thumb.{read_key}"))?)
                .ok()?
        };
        let rgba = crate::content_io::decode_image(&bytes).ok()?;
        cranpose_ui::ImageBitmap::from_rgba8(rgba.width(), rgba.height(), rgba.into_raw()).ok()
    })
    .await
    .flatten()?;
    let mut data = rt.borrow_mut();
    if data.images.len() >= 96 {
        data.images.pop_front();
    }
    data.images.push_back((key, image.clone()));
    Some(image)
}

#[cfg(test)]
#[path = "../../test/unit/library.rs"]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Album {
    pub key: String,
    pub title: String,
    pub artist: String,
    pub tracks: Vec<usize>,
    pub cover_track: usize,
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn albums(tracks: &[Track], rows: &[usize]) -> Vec<Album> {
    let mut albums: Vec<Album> = Vec::new();
    let mut positions = HashMap::new();
    for &index in rows {
        let track = &tracks[index];
        let meta = record(track);
        let title = meta.album.clone().unwrap_or_else(|| "Unknown album".into());
        let key = album_key(&meta);
        let artist = meta
            .album_artist
            .clone()
            .or(meta.artist.clone())
            .unwrap_or_else(|| {
                let title = track.title.trim_start_matches(|c: char| {
                    c.is_ascii_digit() || matches!(c, '.' | '-' | '_' | ' ')
                });
                title
                    .split_once(" - ")
                    .map_or("Unknown artist", |(artist, _)| artist)
                    .to_owned()
            });
        let at = *positions.entry(key.clone()).or_insert_with(|| {
            albums.push(Album {
                key,
                title,
                artist: artist.clone(),
                tracks: Vec::new(),
                cover_track: index,
            });
            albums.len() - 1
        });
        let album = &mut albums[at];
        if album.artist != artist {
            album.artist = "Various artists".into();
        }
        if record(&tracks[album.cover_track]).artwork.is_none() && meta.artwork.is_some() {
            album.cover_track = index;
        }
        album.tracks.push(index);
    }
    for album in &mut albums {
        album.tracks.sort_by_cached_key(|&index| {
            let track = &tracks[index];
            let meta = record(track);
            let number = meta
                .track_number
                .or_else(|| {
                    track
                        .title
                        .split(|c: char| !c.is_ascii_digit())
                        .next()?
                        .parse()
                        .ok()
                })
                .unwrap_or(u64::MAX);
            (
                meta.disc_number.unwrap_or(1),
                number,
                title(track).to_lowercase(),
            )
        });
    }
    albums.sort_by_cached_key(|a| (a.title.to_lowercase(), a.artist.to_lowercase()));
    albums
}
