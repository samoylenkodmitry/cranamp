#![allow(clippy::missing_errors_doc)]

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub title: String,
    pub path: Option<String>,
    pub duration_seconds: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackProgress {
    pub elapsed_seconds: f32,
    pub duration_seconds: Option<f32>,
    pub finished: bool,
}

pub const VISUALIZER_BAND_COUNT: usize = 19;
pub type VisualizerBands = [f32; VISUALIZER_BAND_COUNT];

struct DemoTrack {
    title: &'static str,
    file_name: &'static str,
    duration_seconds: f32,
}

const DEMO_MUSIC_WEB_DIR: &str = "demo-music";
const DEMO_TRACKS: &[DemoTrack] = &[
    DemoTrack {
        title: "Cranamp Demo 01 - Retro Tracker",
        file_name: "cranamp-demo-01-retro-tracker.mp3",
        duration_seconds: 100.0,
    },
    DemoTrack {
        title: "Cranamp Demo 02 - Neon Ambient",
        file_name: "cranamp-demo-02-neon-ambient.mp3",
        duration_seconds: 90.0,
    },
    DemoTrack {
        title: "Cranamp Demo 03 - Lo-Fi Jungle",
        file_name: "cranamp-demo-03-lofi-jungle.mp3",
        duration_seconds: 120.0,
    },
    DemoTrack {
        title: "Cranamp Demo 04 - Minimal Synthwave",
        file_name: "cranamp-demo-04-minimal-synthwave.mp3",
        duration_seconds: 100.0,
    },
    DemoTrack {
        title: "Cranamp Demo 05 - Soft Chip Lounge",
        file_name: "cranamp-demo-05-soft-chip-lounge.mp3",
        duration_seconds: 90.0,
    },
];

impl Track {
    pub fn display_title(&self) -> &str {
        self.title.as_str()
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        not(target_os = "android"),
        not(target_os = "ios"),
        feature = "native-dialogs"
    ))]
    fn from_path(path: std::path::PathBuf) -> Self {
        let title = path
            .file_stem()
            .or_else(|| path.file_name())
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Untitled")
            .to_string();

        let path = path.to_string_lossy().to_string();
        let duration_seconds = known_demo_duration_seconds(&path).or_else(|| {
            probe_track_duration_seconds(std::path::Path::new(&path))
                .ok()
                .flatten()
        });
        Track {
            title,
            path: Some(path),
            duration_seconds,
        }
    }
}

pub(crate) fn track_from_title_path(title: impl Into<String>, path: impl Into<String>) -> Track {
    let path = path.into();
    let duration_seconds = known_demo_duration_seconds(&path);
    Track {
        title: title.into(),
        path: Some(path),
        duration_seconds,
    }
}

fn known_demo_duration_seconds(path: &str) -> Option<f32> {
    let file_name = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())?;
    DEMO_TRACKS
        .iter()
        .find(|track| track.file_name == file_name)
        .map(|track| track.duration_seconds)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn demo_playlist_tracks() -> Vec<Track> {
    let Some(directory) = find_demo_music_directory() else {
        return Vec::new();
    };

    DEMO_TRACKS
        .iter()
        .filter_map(|track| {
            let path = directory.join(track.file_name);
            path.is_file()
                .then(|| track_from_title_path(track.title, path.to_string_lossy()))
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub fn demo_playlist_tracks() -> Vec<Track> {
    DEMO_TRACKS
        .iter()
        .map(|track| {
            track_from_title_path(
                track.title,
                format!("{DEMO_MUSIC_WEB_DIR}/{}", track.file_name),
            )
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn find_demo_music_directory() -> Option<std::path::PathBuf> {
    demo_music_candidate_directories()
        .into_iter()
        .find(|directory| demo_music_directory_has_tracks(directory))
}

#[cfg(not(target_arch = "wasm32"))]
fn demo_music_candidate_directories() -> Vec<std::path::PathBuf> {
    let mut directories = Vec::new();

    if let Ok(executable) = std::env::current_exe() {
        if let Some(executable_dir) = executable.parent() {
            directories.push(executable_dir.join(DEMO_MUSIC_WEB_DIR));
            directories.push(
                executable_dir
                    .join("assets")
                    .join("demo-music")
                    .join("generated"),
            );
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        directories.push(current_dir.join(DEMO_MUSIC_WEB_DIR));
        directories.push(
            current_dir
                .join("assets")
                .join("demo-music")
                .join("generated"),
        );
    }

    directories
}

#[cfg(not(target_arch = "wasm32"))]
fn demo_music_directory_has_tracks(directory: &std::path::Path) -> bool {
    DEMO_TRACKS
        .iter()
        .any(|track| directory.join(track.file_name).is_file())
}

/// Builds playable tracks from an entry chosen through the Cranpose native file
/// picker. The entry may be a single file or a folder served by any system
/// document provider (cloud, WebDAV, …).
///
/// Nothing is copied for the common cases: a desktop selection is already a
/// filesystem path, and an Android selection is a `content://` URI the audio
/// engine streams straight from the provider. Only sources with no stable,
/// re-openable location (iOS security-scoped URLs) are materialized into a
/// temporary cache so the audio engine can re-open them on playback.
pub async fn tracks_from_picked_entry(entry: cranpose_services::PickedEntryRef) -> Vec<Track> {
    let cache = picker_cache_dir();
    let mut tracks = collect_picked_audio_tracks(entry, cache).await;
    tracks.sort_by(|a, b| a.display_title().cmp(b.display_title()));
    tracks
}

/// Builds a single playable track from one file entry yielded by a streaming
/// folder pick ([`cranpose_services::FolderStream`]), or `None` if the entry is
/// not a supported audio file. Used to append discovered tracks incrementally so
/// a huge folder on a slow provider starts playing before the walk finishes.
pub async fn track_from_picked_file(entry: cranpose_services::PickedEntryRef) -> Option<Track> {
    if entry.kind() != cranpose_services::PickedKind::File || !is_audio_name(&entry.name()) {
        return None;
    }
    let cache = picker_cache_dir();
    picked_audio_track(&entry, &cache).await
}

fn picker_cache_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("cranamp-picker");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn collect_picked_audio_tracks(
    entry: cranpose_services::PickedEntryRef,
    cache: std::path::PathBuf,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<Track>>>> {
    Box::pin(async move {
        let mut tracks = Vec::new();
        match entry.kind() {
            cranpose_services::PickedKind::Folder => {
                if let Ok(children) = entry.list().await {
                    for child in children {
                        tracks.extend(collect_picked_audio_tracks(child, cache.clone()).await);
                    }
                }
            }
            cranpose_services::PickedKind::File => {
                if is_audio_name(&entry.name()) {
                    if let Some(track) = picked_audio_track(&entry, &cache).await {
                        tracks.push(track);
                    }
                }
            }
        }
        tracks
    })
}

async fn picked_audio_track(
    entry: &cranpose_services::PickedEntryRef,
    cache: &std::path::Path,
) -> Option<Track> {
    let name = entry.name();
    let title = name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(&name)
        .to_string();
    let display = entry.display_path();
    // `content://` (Android) streams from the provider; a real filesystem path
    // (desktop) is used as-is. Neither is copied.
    let location = if display.starts_with("content://") || std::path::Path::new(&display).is_file()
    {
        display
    } else {
        // iOS security-scoped URLs are not re-openable later; cache the bytes.
        let bytes = entry.read_bytes().await.ok()?;
        let destination = cache.join(picker_safe_file_name(&name));
        std::fs::write(&destination, &bytes).ok()?;
        destination.to_string_lossy().into_owned()
    };
    Some(track_from_title_path(title, location))
}

fn is_audio_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    supported_audio_extensions()
        .iter()
        .any(|extension| lower.ends_with(&format!(".{extension}")))
}

fn picker_safe_file_name(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "track".to_string()
    } else {
        safe
    }
}

pub fn supported_audio_extensions() -> &'static [&'static str] {
    &[
        "aac", "aiff", "alac", "caf", "flac", "m4a", "m4b", "m4v", "mov", "mp1", "mp2", "mp3",
        "mp4", "oga", "ogg", "opus", "wav", "wave", "webm",
    ]
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        not(target_os = "android"),
        not(target_os = "ios"),
        feature = "native-dialogs"
    )
))]
fn dialog_audio_extensions() -> Vec<String> {
    let mut extensions = Vec::with_capacity(supported_audio_extensions().len() * 2);
    for extension in supported_audio_extensions() {
        extensions.push((*extension).to_string());
        extensions.push(extension.to_ascii_uppercase());
    }
    extensions
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
fn is_audio_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            supported_audio_extensions()
                .iter()
                .any(|candidate| *candidate == extension)
        })
        .unwrap_or(false)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
fn tracks_from_selected_paths(paths: impl IntoIterator<Item = std::path::PathBuf>) -> Vec<Track> {
    sort_tracks(paths.into_iter().map(Track::from_path).collect())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
fn sort_tracks(mut tracks: Vec<Track>) -> Vec<Track> {
    tracks.sort_by(|left, right| left.title.cmp(&right.title));
    tracks
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
pub fn pick_audio_files() -> Result<Option<Vec<Track>>, String> {
    let extensions = dialog_audio_extensions();
    let files = rfd::FileDialog::new()
        .set_title("Open audio files")
        .add_filter("Audio", &extensions)
        .pick_files();

    Ok(files.map(tracks_from_selected_paths))
}

#[cfg(not(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
)))]
pub fn pick_audio_files() -> Result<Option<Vec<Track>>, String> {
    Err("native file picker is not available on this target yet".to_string())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
pub fn pick_audio_folder() -> Result<Option<Vec<Track>>, String> {
    let Some(folder) = pick_audio_folder_path()? else {
        return Ok(None);
    };

    Ok(Some(tracks_from_audio_folder(folder)))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
pub fn pick_audio_folder_path() -> Result<Option<std::path::PathBuf>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("Open audio folder")
        .pick_folder())
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
))]
pub fn tracks_from_audio_folder(folder: std::path::PathBuf) -> Vec<Track> {
    let tracks = walkdir::WalkDir::new(folder)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| is_audio_path(path))
        .map(Track::from_path)
        .collect::<Vec<_>>();

    sort_tracks(tracks)
}

#[cfg(not(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
)))]
pub fn pick_audio_folder() -> Result<Option<Vec<Track>>, String> {
    Err("native folder picker is not available on this target yet".to_string())
}

#[cfg(not(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
)))]
pub fn pick_audio_folder_path() -> Result<Option<std::path::PathBuf>, String> {
    Err("native folder picker is not available on this target yet".to_string())
}

#[cfg(not(all(
    not(target_arch = "wasm32"),
    not(target_os = "android"),
    not(target_os = "ios"),
    feature = "native-dialogs"
)))]
pub fn tracks_from_audio_folder(_folder: std::path::PathBuf) -> Vec<Track> {
    Vec::new()
}

/// Picks audio files (or every audio file inside a folder) through the browser
/// and returns playable tracks. Each track's path is a blob object URL backed
/// by the picked `File`, so the browser keeps the data alive and `play_track`
/// can play any entry without re-reading it — exactly like an HTTP-backed track.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub async fn pick_web_audio_tracks(folder: bool) -> Result<Vec<Track>, String> {
    use wasm_bindgen::JsCast;

    let files = pick_web_files(folder).await?;
    let mut tracks = Vec::new();
    for file in files {
        let name = file.name();
        if !is_audio_name(&name) {
            continue;
        }
        let blob: &web_sys::Blob = file.unchecked_ref();
        let url = web_sys::Url::create_object_url_with_blob(blob).map_err(web_js_error)?;
        tracks.push(Track {
            title: web_track_title(&name),
            path: Some(url),
            duration_seconds: known_demo_duration_seconds(&name),
        });
    }
    tracks.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(tracks)
}

/// Drives a hidden `<input type="file">` so the picker works in every browser:
/// `webkitdirectory` selects a folder where supported and otherwise degrades to
/// a multi-file selection. Resolves with the chosen files, or empty on cancel.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
async fn pick_web_files(folder: bool) -> Result<Vec<web_sys::File>, String> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or("no browser window")?;
    let document = window.document().ok_or("no document")?;
    let input: web_sys::HtmlInputElement = document
        .create_element("input")
        .map_err(web_js_error)?
        .dyn_into()
        .map_err(|_| "failed to create file input".to_string())?;
    input.set_type("file");
    input.set_multiple(true);
    let accept = dialog_audio_extensions()
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>()
        .join(",");
    input.set_accept(&accept);
    if folder {
        let _ = input.set_attribute("webkitdirectory", "");
    }

    let (tx, rx) = futures_channel::oneshot::channel::<Vec<web_sys::File>>();
    let tx = Rc::new(RefCell::new(Some(tx)));

    let input_for_change = input.clone();
    let tx_change = Rc::clone(&tx);
    let on_change = Closure::<dyn FnMut()>::new(move || {
        let files = input_for_change
            .files()
            .map(|list| (0..list.length()).filter_map(|i| list.get(i)).collect())
            .unwrap_or_default();
        if let Some(tx) = tx_change.borrow_mut().take() {
            let _ = tx.send(files);
        }
    });
    let tx_cancel = Rc::clone(&tx);
    let on_cancel = Closure::<dyn FnMut()>::new(move || {
        if let Some(tx) = tx_cancel.borrow_mut().take() {
            let _ = tx.send(Vec::new());
        }
    });
    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    input
        .add_event_listener_with_callback("cancel", on_cancel.as_ref().unchecked_ref())
        .map_err(web_js_error)?;

    input.click();

    // The closures must outlive the dialog, so keep them alive until it resolves.
    let files = rx.await.unwrap_or_default();
    drop(on_change);
    drop(on_cancel);
    Ok(files)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn web_track_title(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(name)
        .to_string()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn web_js_error(value: wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "browser file picker failed".to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
mod native {
    use super::{Track, VisualizerBands, VISUALIZER_BAND_COUNT};
    use rodio::{
        ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, Sample, SampleRate, Source,
    };
    use std::f32::consts::PI;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
    use std::sync::{Arc, Condvar, Mutex, OnceLock};
    use std::time::Duration;
    use symphonia::core::audio::{AudioSpec, GenericAudioBufferRef};
    use symphonia::core::codecs::audio::{
        AudioDecoder as SymphoniaDecoder, AudioDecoderOptions, CODEC_ID_NULL_AUDIO,
    };
    use symphonia::core::errors::Error as SymphoniaError;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, FormatReader};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::units::{Time, Timestamp};

    struct NativeAudio {
        sink: MixerDeviceSink,
        player: Option<Player>,
        duration: Option<Duration>,
    }

    static AUDIO: OnceLock<Mutex<Option<NativeAudio>>> = OnceLock::new();
    static VISUALIZER_BANDS: OnceLock<[AtomicU32; VISUALIZER_BAND_COUNT]> = OnceLock::new();
    static EQUALIZER: OnceLock<EqualizerShared> = OnceLock::new();
    const ANALYZER_WINDOW_SAMPLES: usize = 2048;
    const EQUALIZER_VALUE_COUNT: usize = 11;
    const EQUALIZER_BAND_COUNT: usize = EQUALIZER_VALUE_COUNT - 1;
    const EQUALIZER_MAX_GAIN_DB: f32 = 12.0;
    const EQUALIZER_BAND_Q: f32 = 1.15;
    const EQUALIZER_CENTER_FREQUENCIES: [f32; EQUALIZER_BAND_COUNT] = [
        60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0,
    ];
    pub(super) type BoxedSource = Box<dyn Source + Send>;
    type SelectedAudioTrack = (u32, Box<dyn SymphoniaDecoder>, Option<Duration>);

    struct EqualizerShared {
        enabled: AtomicBool,
        values: [AtomicU32; EQUALIZER_VALUE_COUNT],
        revision: AtomicU32,
    }

    #[derive(Clone, Copy)]
    struct EqualizerSnapshot {
        enabled: bool,
        values: [f32; EQUALIZER_VALUE_COUNT],
        revision: u32,
    }

    #[derive(Clone, Copy, Debug)]
    struct Biquad {
        b0: f32,
        b1: f32,
        b2: f32,
        a1: f32,
        a2: f32,
        z1: f32,
        z2: f32,
    }

    impl Biquad {
        fn bypass() -> Self {
            Self {
                b0: 1.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
                z1: 0.0,
                z2: 0.0,
            }
        }

        fn peaking(sample_rate: u32, frequency: f32, gain_db: f32) -> Self {
            if gain_db.abs() < 0.01 || sample_rate == 0 {
                return Self::bypass();
            }

            let sample_rate = sample_rate as f32;
            let nyquist = sample_rate * 0.5;
            let frequency = frequency.clamp(20.0, (nyquist * 0.92).max(20.0));
            let omega = 2.0 * PI * frequency / sample_rate;
            let sin_omega = omega.sin();
            let cos_omega = omega.cos();
            let alpha = sin_omega / (2.0 * EQUALIZER_BAND_Q);
            let gain = 10.0_f32.powf(gain_db / 40.0);

            let b0 = 1.0 + alpha * gain;
            let b1 = -2.0 * cos_omega;
            let b2 = 1.0 - alpha * gain;
            let a0 = 1.0 + alpha / gain;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha / gain;

            Self {
                b0: b0 / a0,
                b1: b1 / a0,
                b2: b2 / a0,
                a1: a1 / a0,
                a2: a2 / a0,
                z1: 0.0,
                z2: 0.0,
            }
        }

        fn process(&mut self, sample: f32) -> f32 {
            let output = self.b0.mul_add(sample, self.z1);
            self.z1 = self.b1.mul_add(sample, self.z2 - self.a1 * output);
            self.z2 = self.b2 * sample - self.a2 * output;
            output
        }
    }

    struct EqualizerSource {
        inner: BoxedSource,
        snapshot: EqualizerSnapshot,
        sample_rate: u32,
        channel_count: usize,
        next_channel: usize,
        preamp_gain: f32,
        filters: Vec<[Biquad; EQUALIZER_BAND_COUNT]>,
    }

    impl EqualizerSource {
        fn new(inner: BoxedSource) -> Self {
            let snapshot = equalizer_snapshot();
            let sample_rate = inner.sample_rate().get();
            let channel_count = usize::from(inner.channels().get()).max(1);
            let preamp_gain = equalizer_preamp_gain(snapshot.values[0]);
            let filters = equalizer_filters(channel_count, sample_rate, snapshot.values);
            Self {
                inner,
                snapshot,
                sample_rate,
                channel_count,
                next_channel: 0,
                preamp_gain,
                filters,
            }
        }

        fn refresh_if_stale(&mut self) {
            let sample_rate = self.inner.sample_rate().get();
            let channel_count = usize::from(self.inner.channels().get()).max(1);
            let revision = equalizer_storage().revision.load(Ordering::Relaxed);
            if revision == self.snapshot.revision
                && sample_rate == self.sample_rate
                && channel_count == self.channel_count
            {
                return;
            }

            self.snapshot = equalizer_snapshot();
            self.sample_rate = sample_rate;
            self.channel_count = channel_count;
            self.next_channel = 0;
            self.preamp_gain = equalizer_preamp_gain(self.snapshot.values[0]);
            self.filters = equalizer_filters(channel_count, sample_rate, self.snapshot.values);
        }

        fn process_sample(&mut self, sample: Sample) -> Sample {
            self.refresh_if_stale();
            let channel = self.next_channel.min(self.filters.len().saturating_sub(1));
            self.next_channel = (self.next_channel + 1) % self.channel_count;

            if !self.snapshot.enabled {
                return sample;
            }

            let mut output = sample * self.preamp_gain;
            for filter in &mut self.filters[channel] {
                output = filter.process(output);
            }
            output.clamp(-1.0, 1.0)
        }
    }

    impl Iterator for EqualizerSource {
        type Item = Sample;

        fn next(&mut self) -> Option<Self::Item> {
            let sample = self.inner.next()?;
            Some(self.process_sample(sample))
        }
    }

    impl Source for EqualizerSource {
        fn current_span_len(&self) -> Option<usize> {
            self.inner.current_span_len()
        }

        fn channels(&self) -> ChannelCount {
            self.inner.channels()
        }

        fn sample_rate(&self) -> SampleRate {
            self.inner.sample_rate()
        }

        fn total_duration(&self) -> Option<Duration> {
            self.inner.total_duration()
        }

        fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
            self.inner.try_seek(pos)?;
            self.next_channel = 0;
            self.filters =
                equalizer_filters(self.channel_count, self.sample_rate, self.snapshot.values);
            Ok(())
        }
    }

    struct AnalyzerSource {
        inner: BoxedSource,
        window: Vec<Sample>,
    }

    impl AnalyzerSource {
        fn new(inner: BoxedSource) -> Self {
            Self {
                inner,
                window: Vec::with_capacity(ANALYZER_WINDOW_SAMPLES),
            }
        }

        fn analyze_sample(&mut self, sample: Sample) {
            self.window.push(sample);
            if self.window.len() >= ANALYZER_WINDOW_SAMPLES {
                let bands = compute_analyzer_bands(&self.window, self.sample_rate().get());
                publish_visualizer_bands(bands);
                self.window.clear();
            }
        }
    }

    impl Iterator for AnalyzerSource {
        type Item = Sample;

        fn next(&mut self) -> Option<Self::Item> {
            let sample = self.inner.next()?;
            self.analyze_sample(sample);
            Some(sample)
        }
    }

    impl Source for AnalyzerSource {
        fn current_span_len(&self) -> Option<usize> {
            self.inner.current_span_len()
        }

        fn channels(&self) -> ChannelCount {
            self.inner.channels()
        }

        fn sample_rate(&self) -> SampleRate {
            self.inner.sample_rate()
        }

        fn total_duration(&self) -> Option<Duration> {
            self.inner.total_duration()
        }

        fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
            self.inner.try_seek(pos)?;
            self.window.clear();
            reset_visualizer_bands();
            Ok(())
        }
    }

    struct SymphoniaAudioSource {
        decoder: Box<dyn SymphoniaDecoder>,
        format: Box<dyn FormatReader>,
        track_id: u32,
        samples: Vec<Sample>,
        sample_offset: usize,
        spec: AudioSpec,
        total_duration: Option<Duration>,
        exhausted: bool,
    }

    impl SymphoniaAudioSource {
        fn new(
            source: Box<dyn symphonia::core::io::MediaSource>,
            extension_hint: Option<&str>,
        ) -> Result<Self, String> {
            let mut hint = Hint::new();
            if let Some(extension_hint) = extension_hint {
                hint.with_extension(extension_hint);
            }

            let stream = MediaSourceStream::new(source, Default::default());
            let mut format = symphonia::default::get_probe()
                .probe(
                    &hint,
                    stream,
                    FormatOptions::default(),
                    MetadataOptions::default(),
                )
                .map_err(format_symphonia_error)?;

            let (track_id, mut decoder, total_duration) = select_audio_track(&*format)?;
            let (spec, samples) = read_next_audio_buffer(&mut *format, &mut *decoder, track_id)?
                .ok_or_else(|| "no decodable audio packets found".to_string())?;

            Ok(Self {
                decoder,
                format,
                track_id,
                samples,
                sample_offset: 0,
                spec,
                total_duration,
                exhausted: false,
            })
        }

        fn load_next_buffer(&mut self) -> bool {
            match read_next_audio_buffer(&mut *self.format, &mut *self.decoder, self.track_id) {
                Ok(Some((spec, samples))) => {
                    self.spec = spec;
                    self.samples = samples;
                    self.sample_offset = 0;
                    true
                }
                Ok(None) | Err(_) => {
                    self.exhausted = true;
                    false
                }
            }
        }
    }

    impl Iterator for SymphoniaAudioSource {
        type Item = Sample;

        fn next(&mut self) -> Option<Self::Item> {
            // A decoded packet can legitimately carry zero samples (e.g. the
            // first packet after a seek resets the decoder), so keep pulling
            // buffers until one yields audio.
            while self.sample_offset >= self.samples.len() {
                if !self.load_next_buffer() {
                    return None;
                }
            }

            let sample = self.samples[self.sample_offset];
            self.sample_offset += 1;
            Some(sample)
        }
    }

    impl Source for SymphoniaAudioSource {
        fn current_span_len(&self) -> Option<usize> {
            if self.exhausted {
                Some(0)
            } else {
                Some(self.samples.len())
            }
        }

        fn channels(&self) -> ChannelCount {
            ChannelCount::new(
                self.spec
                    .channels()
                    .count()
                    .try_into()
                    .expect("audio channel count exceeds u16::MAX"),
            )
            .expect("audio should have at least one channel")
        }

        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(self.spec.rate()).expect("audio should have a non-zero sample rate")
        }

        fn total_duration(&self) -> Option<Duration> {
            self.total_duration
        }

        fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
            use symphonia::core::formats::{SeekMode, SeekTo};

            let Some(time) = Time::try_from_nanos_u128(pos.as_nanos()) else {
                return Err(rodio::source::SeekError::Other(std::sync::Arc::new(
                    std::io::Error::other("seek target out of range"),
                )));
            };
            self.format
                .seek(
                    SeekMode::Coarse,
                    SeekTo::Time {
                        time,
                        track_id: Some(self.track_id),
                    },
                )
                .map_err(|error| {
                    rodio::source::SeekError::Other(std::sync::Arc::new(std::io::Error::other(
                        format!("symphonia seek failed: {error}"),
                    )))
                })?;
            self.decoder.reset();
            self.samples.clear();
            self.sample_offset = 0;
            self.exhausted = false;
            Ok(())
        }
    }

    fn select_audio_track(format: &dyn FormatReader) -> Result<SelectedAudioTrack, String> {
        for track in format.tracks() {
            let Some(params) = track
                .codec_params
                .as_ref()
                .and_then(|params| params.audio())
            else {
                continue;
            };
            if params.codec == CODEC_ID_NULL_AUDIO {
                continue;
            }

            let Ok(decoder) = symphonia::default::get_codecs()
                .make_audio_decoder(params, &AudioDecoderOptions::default())
            else {
                continue;
            };

            let total_duration = params
                .sample_rate
                .zip(track.num_frames)
                .map(|(sample_rate, frames)| {
                    Duration::from_secs_f64(frames as f64 / sample_rate as f64)
                })
                .or_else(|| {
                    track
                        .time_base
                        .zip(track.duration)
                        .and_then(|(base, duration)| {
                            Timestamp::try_from(duration.get())
                                .ok()
                                .and_then(|duration| base.calc_time(duration))
                                .and_then(time_to_std_duration)
                        })
                })
                .filter(|duration: &Duration| !duration.is_zero());

            return Ok((track.id, decoder, total_duration));
        }

        Err("no supported audio track found".to_string())
    }

    fn read_next_audio_buffer(
        format: &mut dyn FormatReader,
        decoder: &mut dyn SymphoniaDecoder,
        track_id: u32,
    ) -> Result<Option<(AudioSpec, Vec<Sample>)>, String> {
        loop {
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return Ok(None),
                Err(SymphoniaError::IoError(_)) => return Ok(None),
                Err(error) => return Err(format_symphonia_error(error)),
            };

            if packet.track_id != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => return Ok(Some(copy_audio_buffer(decoded))),
                Err(SymphoniaError::DecodeError(_)) | Err(SymphoniaError::IoError(_)) => continue,
                Err(error) => return Err(format_symphonia_error(error)),
            }
        }
    }

    fn copy_audio_buffer(decoded: GenericAudioBufferRef<'_>) -> (AudioSpec, Vec<Sample>) {
        let spec = decoded.spec().clone();
        let mut samples = Vec::<Sample>::with_capacity(decoded.samples_interleaved());
        decoded.copy_to_vec_interleaved(&mut samples);
        (spec, samples)
    }

    fn time_to_std_duration(time: Time) -> Option<Duration> {
        let (seconds, nanos) = time.parts();
        Some(Duration::new(seconds.try_into().ok()?, nanos))
    }

    fn equalizer_storage() -> &'static EqualizerShared {
        EQUALIZER.get_or_init(|| EqualizerShared {
            enabled: AtomicBool::new(true),
            values: std::array::from_fn(|_| AtomicU32::new(0.5_f32.to_bits())),
            revision: AtomicU32::new(1),
        })
    }

    fn equalizer_snapshot() -> EqualizerSnapshot {
        let storage = equalizer_storage();
        EqualizerSnapshot {
            enabled: storage.enabled.load(Ordering::Relaxed),
            values: std::array::from_fn(|index| {
                f32::from_bits(storage.values[index].load(Ordering::Relaxed)).clamp(0.0, 1.0)
            }),
            revision: storage.revision.load(Ordering::Relaxed),
        }
    }

    fn equalizer_value_gain_db(value: f32) -> f32 {
        (value.clamp(0.0, 1.0) - 0.5) * 2.0 * EQUALIZER_MAX_GAIN_DB
    }

    fn equalizer_preamp_gain(value: f32) -> f32 {
        10.0_f32.powf(equalizer_value_gain_db(value) / 20.0)
    }

    fn equalizer_filters(
        channel_count: usize,
        sample_rate: u32,
        values: [f32; EQUALIZER_VALUE_COUNT],
    ) -> Vec<[Biquad; EQUALIZER_BAND_COUNT]> {
        let band_gains: [f32; EQUALIZER_BAND_COUNT] =
            std::array::from_fn(|index| equalizer_value_gain_db(values[index + 1]));
        (0..channel_count.max(1))
            .map(|_| {
                std::array::from_fn(|index| {
                    Biquad::peaking(
                        sample_rate,
                        EQUALIZER_CENTER_FREQUENCIES[index],
                        band_gains[index],
                    )
                })
            })
            .collect()
    }

    pub fn set_equalizer(enabled: bool, values: [f32; EQUALIZER_VALUE_COUNT]) {
        let storage = equalizer_storage();
        storage.enabled.store(enabled, Ordering::Relaxed);
        for (cell, value) in storage.values.iter().zip(values) {
            cell.store(value.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        }
        storage.revision.fetch_add(1, Ordering::Relaxed);
    }

    pub fn probe_track_duration_seconds(path: &Path) -> Result<Option<f32>, String> {
        // Don't open a descriptor per `content://` track just to read its
        // duration during background hydration; it resolves on playback.
        if path.to_str().is_some_and(is_content_uri) {
            return Ok(None);
        }
        let (_, duration) = decode_track_source(path, false)?;
        Ok(duration.map(|duration| duration.as_secs_f32()))
    }

    fn analyzer_band_storage() -> &'static [AtomicU32; VISUALIZER_BAND_COUNT] {
        VISUALIZER_BANDS.get_or_init(|| std::array::from_fn(|_| AtomicU32::new(0)))
    }

    fn publish_visualizer_bands(bands: VisualizerBands) {
        let storage = analyzer_band_storage();
        for (band, value) in storage.iter().zip(bands) {
            band.store(
                (value.clamp(0.0, 1.0) * 1000.0).round() as u32,
                Ordering::Relaxed,
            );
        }
    }

    fn reset_visualizer_bands() {
        publish_visualizer_bands([0.0; VISUALIZER_BAND_COUNT]);
    }

    pub fn visualizer_bands() -> VisualizerBands {
        std::array::from_fn(|index| {
            analyzer_band_storage()[index].load(Ordering::Relaxed) as f32 / 1000.0
        })
    }

    pub(super) fn compute_analyzer_bands(samples: &[Sample], sample_rate: u32) -> VisualizerBands {
        if samples.is_empty() || sample_rate == 0 {
            return [0.0; VISUALIZER_BAND_COUNT];
        }

        let nyquist = sample_rate as f32 * 0.5;
        let min_frequency = 60.0_f32;
        let max_frequency = nyquist.min(12_000.0).max(min_frequency + 1.0);
        let frequency_ratio = max_frequency / min_frequency;
        let sample_count = samples.len() as f32;

        std::array::from_fn(|band| {
            let position = band as f32 / (VISUALIZER_BAND_COUNT - 1) as f32;
            let frequency = min_frequency * frequency_ratio.powf(position);
            let omega = 2.0 * PI * frequency / sample_rate as f32;
            let coeff = 2.0 * omega.cos();
            let mut previous = 0.0;
            let mut previous_2 = 0.0;
            for sample in samples.iter().copied() {
                let current = sample + coeff * previous - previous_2;
                previous_2 = previous;
                previous = current;
            }

            let power =
                previous.mul_add(previous, previous_2 * previous_2) - coeff * previous * previous_2;
            let magnitude = power.max(0.0).sqrt() / sample_count;
            analyzer_magnitude_to_level(magnitude, band)
        })
    }

    fn analyzer_magnitude_to_level(magnitude: f32, band: usize) -> f32 {
        let high_band_boost = 1.0 + (band as f32 / (VISUALIZER_BAND_COUNT - 1) as f32) * 0.85;
        (magnitude * 28.0 * high_band_boost).sqrt().clamp(0.0, 1.0)
    }

    pub(super) fn decode_track_source(
        path: &Path,
        repeat: bool,
    ) -> Result<(BoxedSource, Option<Duration>), String> {
        let location = path.to_str().unwrap_or_default();
        let content = is_content_uri(location);
        let mut file = open_audio_file(location)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
        // A real file (desktop, or a local document provider) is seekable, so the
        // decoder reads it in place. A network/cloud/WebDAV `content://` document
        // is usually a non-seekable pipe whose `len()` reads back as 0; assuming
        // it is a seekable, sized file makes the decoder treat it as empty, so the
        // track "loads" but never plays.
        let seekable = file.stream_position().is_ok();
        // A `content://` document carries no usable filesystem extension, so let
        // the decoder probe the format; a real path keeps its extension hint.
        let extension_hint = if content {
            None
        } else {
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
        };
        log::info!(
            "decode_track_source: {} (content={content}, seekable={seekable})",
            path.display()
        );

        let media: Box<dyn symphonia::core::io::MediaSource> = if seekable {
            Box::new(file)
        } else {
            // Spool the stream to a temporary on-disk cache: playback starts
            // immediately from the front while the rest downloads in the
            // background, and seeking works as soon as the target offset is
            // cached. The cache is bounded — it is deleted when the track stops.
            let cached = StreamCache::spool(Box::new(file))
                .map_err(|error| format!("failed to start streaming cache: {error}"))?;
            Box::new(cached)
        };
        let source = SymphoniaAudioSource::new(media, extension_hint.as_deref())?;
        let duration = source.total_duration();
        if repeat {
            Ok((Box::new(source.repeat_infinite()), duration))
        } else {
            Ok((Box::new(source), duration))
        }
    }

    /// Whether a track location is an Android `content://` document URI rather
    /// than a filesystem path.
    fn is_content_uri(location: &str) -> bool {
        location.starts_with("content://")
    }

    /// Opens a track for decoding. On Android a picked document is a
    /// `content://` URI streamed straight from the provider (no copy); every
    /// other location is an ordinary filesystem path.
    fn open_audio_file(location: &str) -> std::io::Result<File> {
        #[cfg(target_os = "android")]
        if is_content_uri(location) {
            return cranpose::open_content_uri(location);
        }
        File::open(location)
    }

    // ----- Streaming cache for non-seekable provider documents ----------------
    //
    // A cloud/WebDAV `content://` descriptor is typically a non-seekable pipe, so
    // the decoder can neither probe nor seek it. Spool it to a temporary file in
    // a background thread: the decoder reads from the file and only blocks until
    // the bytes it needs have arrived, so playback starts at the front right away
    // and seeking works once the target offset is cached. The spool file is
    // deleted as soon as the track stops, so the cache stays bounded even for a
    // terabyte-scale library — only the track(s) currently playing are on disk.

    struct StreamCacheState {
        downloaded: u64,
        total: Option<u64>,
        finished: bool,
        error: Option<String>,
    }

    struct StreamCacheShared {
        state: Mutex<StreamCacheState>,
        ready: Condvar,
        cancel: AtomicBool,
        path: PathBuf,
    }

    impl StreamCacheShared {
        /// Block until at least `wanted` bytes are cached, the download finishes,
        /// or it is cancelled/errored; returns how many bytes are available.
        fn wait_for(&self, wanted: u64) -> std::io::Result<u64> {
            let mut state = self.state.lock().unwrap();
            loop {
                if let Some(error) = &state.error {
                    return Err(std::io::Error::other(error.clone()));
                }
                if state.downloaded >= wanted
                    || state.finished
                    || self.cancel.load(Ordering::Relaxed)
                {
                    return Ok(state.downloaded);
                }
                state = self.ready.wait(state).unwrap();
            }
        }

        /// Block until the total length is known (the download finished).
        fn wait_for_total(&self) -> std::io::Result<u64> {
            let mut state = self.state.lock().unwrap();
            loop {
                if let Some(error) = &state.error {
                    return Err(std::io::Error::other(error.clone()));
                }
                if let Some(total) = state.total {
                    return Ok(total);
                }
                if state.finished || self.cancel.load(Ordering::Relaxed) {
                    return Ok(state.downloaded);
                }
                state = self.ready.wait(state).unwrap();
            }
        }
    }

    /// Stops the downloader and deletes the spool file once every reader is gone.
    struct StreamCacheGuard {
        shared: Arc<StreamCacheShared>,
    }

    impl Drop for StreamCacheGuard {
        fn drop(&mut self) {
            self.shared.cancel.store(true, Ordering::Relaxed);
            self.shared.ready.notify_all();
            let _ = std::fs::remove_file(&self.shared.path);
        }
    }

    /// A seekable `Read` view over a source being spooled to a temporary file.
    pub(super) struct StreamCache {
        shared: Arc<StreamCacheShared>,
        file: File,
        position: u64,
        _guard: Arc<StreamCacheGuard>,
    }

    impl StreamCache {
        pub(super) fn spool(source: Box<dyn Read + Send>) -> std::io::Result<Self> {
            let directory = stream_cache_directory().ok_or_else(|| {
                std::io::Error::other("no writable directory for the streaming cache")
            })?;
            sweep_stale_stream_cache(&directory);
            let path = directory.join(next_stream_cache_name());
            let writer = File::create(&path)?;
            let reader = File::open(&path)?;
            let shared = Arc::new(StreamCacheShared {
                state: Mutex::new(StreamCacheState {
                    downloaded: 0,
                    total: None,
                    finished: false,
                    error: None,
                }),
                ready: Condvar::new(),
                cancel: AtomicBool::new(false),
                path,
            });
            let guard = Arc::new(StreamCacheGuard {
                shared: Arc::clone(&shared),
            });
            let download_shared = Arc::clone(&shared);
            std::thread::spawn(move || run_stream_download(source, writer, download_shared));
            Ok(Self {
                shared,
                file: reader,
                position: 0,
                _guard: guard,
            })
        }
    }

    impl Read for StreamCache {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let available = self.shared.wait_for(self.position + 1)?;
            if self.position >= available {
                return Ok(0);
            }
            self.file.seek(SeekFrom::Start(self.position))?;
            let limit = (available - self.position).min(buf.len() as u64) as usize;
            let read = self.file.read(&mut buf[..limit])?;
            self.position += read as u64;
            Ok(read)
        }
    }

    impl symphonia::core::io::MediaSource for StreamCache {
        fn is_seekable(&self) -> bool {
            true
        }

        fn byte_len(&self) -> Option<u64> {
            None
        }
    }

    impl Seek for StreamCache {
        fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
            let target = match from {
                SeekFrom::Start(offset) => offset,
                SeekFrom::Current(delta) => offset_from(self.position, delta)?,
                SeekFrom::End(delta) => offset_from(self.shared.wait_for_total()?, delta)?,
            };
            self.position = target;
            Ok(target)
        }
    }

    fn offset_from(base: u64, delta: i64) -> std::io::Result<u64> {
        let target = base as i64 + delta;
        u64::try_from(target).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "attempted to seek before the start of the stream",
            )
        })
    }

    fn run_stream_download(
        mut source: Box<dyn Read + Send>,
        mut writer: File,
        shared: Arc<StreamCacheShared>,
    ) {
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            if shared.cancel.load(Ordering::Relaxed) {
                break;
            }
            match source.read(&mut buffer) {
                Ok(0) => {
                    let mut state = shared.state.lock().unwrap();
                    state.total = Some(state.downloaded);
                    state.finished = true;
                    shared.ready.notify_all();
                    break;
                }
                Ok(read) => {
                    if let Err(error) = writer.write_all(&buffer[..read]) {
                        fail_stream_download(&shared, format!("cache write failed: {error}"));
                        break;
                    }
                    let mut state = shared.state.lock().unwrap();
                    state.downloaded += read as u64;
                    shared.ready.notify_all();
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    fail_stream_download(&shared, format!("stream read failed: {error}"));
                    break;
                }
            }
        }
    }

    fn fail_stream_download(shared: &StreamCacheShared, message: String) {
        log::error!("streaming cache: {message}");
        let mut state = shared.state.lock().unwrap();
        state.error = Some(message);
        state.finished = true;
        shared.ready.notify_all();
    }

    fn next_stream_cache_name() -> String {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        format!("stream-{}-{sequence}.tmp", std::process::id())
    }

    fn stream_cache_directory() -> Option<PathBuf> {
        #[cfg(target_os = "android")]
        let directory = crate::android_bridge::stream_cache_dir();
        #[cfg(not(target_os = "android"))]
        let directory = Some(std::env::temp_dir().join("cranamp-stream-cache"));
        let directory = directory?;
        std::fs::create_dir_all(&directory).ok()?;
        Some(directory)
    }

    /// Remove spool files left behind by a previous run (e.g. after a crash).
    /// Runs once per process; live tracks delete their own files on drop.
    fn sweep_stale_stream_cache(directory: &Path) {
        static SWEPT: std::sync::Once = std::sync::Once::new();
        SWEPT.call_once(|| {
            if let Ok(entries) = std::fs::read_dir(directory) {
                for entry in entries.flatten() {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        });
    }

    fn format_symphonia_error(error: SymphoniaError) -> String {
        match error {
            SymphoniaError::Unsupported(reason) => reason.to_string(),
            other => other.to_string(),
        }
    }

    fn with_audio<T>(f: impl FnOnce(&mut NativeAudio) -> Result<T, String>) -> Result<T, String> {
        let audio = AUDIO.get_or_init(|| Mutex::new(None));
        // Recover from a poisoned lock instead of failing forever: a panic while
        // decoding one (e.g. malformed streamed) track must not permanently
        // disable audio for every later track.
        let mut audio = match audio.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::warn!("audio backend lock was poisoned by a previous panic; recovering");
                poisoned.into_inner()
            }
        };

        if audio.is_none() {
            log::info!("opening default audio output device");
            let mut sink = DeviceSinkBuilder::open_default_sink().map_err(|error| {
                log::error!("failed to open default audio device: {error}");
                format!("failed to open default audio device: {error}")
            })?;
            sink.log_on_drop(false);
            log::info!("default audio output device opened");
            *audio = Some(NativeAudio {
                sink,
                player: None,
                duration: None,
            });
        }

        let audio = audio
            .as_mut()
            .ok_or_else(|| "audio backend failed to initialize".to_string())?;
        f(audio)
    }

    pub fn play_track(track: &Track, volume: f32, repeat: bool) -> Result<(), String> {
        let path = track
            .path
            .as_deref()
            .ok_or_else(|| "selected track has no filesystem path".to_string())?;
        let (source, duration) = decode_track_source(Path::new(path), repeat).map_err(|error| {
            log::error!("play_track: failed to decode {}: {error}", track.title);
            format!("failed to decode {}: {error}", track.title)
        })?;
        log::info!(
            "play_track: decoded {} -> {} Hz, {} channel(s), duration {:?}",
            track.title,
            source.sample_rate().get(),
            source.channels().get(),
            duration
        );

        with_audio(|audio| {
            if let Some(player) = audio.player.take() {
                player.stop();
            }

            reset_visualizer_bands();
            let player = Player::connect_new(audio.sink.mixer());
            player.set_volume(volume.clamp(0.0, 1.0));
            player.append(AnalyzerSource::new(Box::new(EqualizerSource::new(source))));
            player.play();
            audio.player = Some(player);
            audio.duration = duration;
            log::info!("play_track: playback started for {}", track.title);
            Ok(())
        })
    }

    pub fn resume() -> Result<(), String> {
        with_audio(|audio| {
            if let Some(player) = &audio.player {
                player.play();
            }
            Ok(())
        })
    }

    pub fn pause() -> Result<(), String> {
        with_audio(|audio| {
            if let Some(player) = &audio.player {
                player.pause();
            }
            Ok(())
        })
    }

    pub fn stop() -> Result<(), String> {
        with_audio(|audio| {
            if let Some(player) = &audio.player {
                player.stop();
            }
            audio.player = None;
            audio.duration = None;
            reset_visualizer_bands();
            Ok(())
        })
    }

    pub fn set_volume(volume: f32) -> Result<(), String> {
        with_audio(|audio| {
            if let Some(player) = &audio.player {
                player.set_volume(volume.clamp(0.0, 1.0));
            }
            Ok(())
        })
    }

    pub fn seek_fraction(fraction: f32) -> Result<(), String> {
        with_audio(|audio| {
            if let Some(player) = &audio.player {
                let duration = audio.duration.unwrap_or_else(|| Duration::from_secs(300));
                let target = duration.mul_f32(fraction.clamp(0.0, 1.0));
                player
                    .try_seek(target)
                    .map_err(|error| format!("seek failed: {error}"))?;
            }
            Ok(())
        })
    }

    pub fn playback_progress() -> Result<Option<super::PlaybackProgress>, String> {
        with_audio(|audio| {
            let Some(player) = &audio.player else {
                return Ok(None);
            };

            Ok(Some(super::PlaybackProgress {
                elapsed_seconds: player.get_pos().as_secs_f32(),
                duration_seconds: audio.duration.map(|duration| duration.as_secs_f32()),
                finished: player.empty(),
            }))
        })
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod web_audio {
    use super::Track;
    use std::cell::RefCell;
    use wasm_bindgen::JsValue;

    struct WebAudio {
        element: web_sys::HtmlAudioElement,
    }

    thread_local! {
        static AUDIO: RefCell<Option<WebAudio>> = const { RefCell::new(None) };
    }

    fn js_error(value: JsValue) -> String {
        value
            .as_string()
            .unwrap_or_else(|| "browser audio operation failed".to_string())
    }

    pub fn play_track(track: &Track, volume: f32, repeat: bool) -> Result<(), String> {
        let url = track
            .path
            .as_deref()
            .ok_or_else(|| "selected track has no URL".to_string())?;
        let element = web_sys::HtmlAudioElement::new_with_src(url).map_err(js_error)?;
        play_element(element, volume, repeat)
    }

    fn play_element(
        element: web_sys::HtmlAudioElement,
        volume: f32,
        repeat: bool,
    ) -> Result<(), String> {
        element.set_volume(volume.clamp(0.0, 1.0) as f64);
        element.set_loop(repeat);
        let _ = element.play().map_err(js_error)?;

        AUDIO.with(|audio| {
            if let Some(previous) = audio.borrow_mut().take() {
                previous.element.pause().ok();
            }
            *audio.borrow_mut() = Some(WebAudio { element });
        });

        Ok(())
    }

    pub fn resume() -> Result<(), String> {
        AUDIO.with(|audio| {
            if let Some(audio) = audio.borrow().as_ref() {
                let _ = audio.element.play().map_err(js_error)?;
            }
            Ok(())
        })
    }

    pub fn pause() -> Result<(), String> {
        AUDIO.with(|audio| {
            if let Some(audio) = audio.borrow().as_ref() {
                audio.element.pause().map_err(js_error)?;
            }
            Ok(())
        })
    }

    pub fn stop() -> Result<(), String> {
        AUDIO.with(|audio| {
            if let Some(audio) = audio.borrow().as_ref() {
                audio.element.pause().map_err(js_error)?;
                let _ = audio.element.set_current_time(0.0);
            }
            Ok(())
        })
    }

    pub fn set_volume(volume: f32) -> Result<(), String> {
        AUDIO.with(|audio| {
            if let Some(audio) = audio.borrow().as_ref() {
                audio.element.set_volume(volume.clamp(0.0, 1.0) as f64);
            }
            Ok(())
        })
    }

    pub fn seek_fraction(fraction: f32) -> Result<(), String> {
        AUDIO.with(|audio| {
            if let Some(audio) = audio.borrow().as_ref() {
                let duration = audio.element.duration();
                let target = if duration.is_finite() {
                    duration * fraction.clamp(0.0, 1.0) as f64
                } else {
                    0.0
                };
                audio.element.set_current_time(target);
            }
            Ok(())
        })
    }

    pub fn playback_progress() -> Result<Option<super::PlaybackProgress>, String> {
        AUDIO.with(|audio| {
            let audio = audio.borrow();
            let Some(audio) = audio.as_ref() else {
                return Ok(None);
            };

            let duration = audio.element.duration();
            Ok(Some(super::PlaybackProgress {
                elapsed_seconds: audio.element.current_time() as f32,
                duration_seconds: duration.is_finite().then_some(duration as f32),
                finished: audio.element.ended(),
            }))
        })
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
pub fn play_track(track: &Track, volume: f32, repeat: bool) -> Result<(), String> {
    native::play_track(track, volume, repeat)
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn play_track(track: &Track, volume: f32, repeat: bool) -> Result<(), String> {
    web_audio::play_track(track, volume, repeat)
}

#[cfg(not(any(
    all(not(target_arch = "wasm32"), feature = "native-audio"),
    all(feature = "web", target_arch = "wasm32")
)))]
pub fn play_track(_track: &Track, _volume: f32, _repeat: bool) -> Result<(), String> {
    Err("audio playback is not enabled for this target".to_string())
}

pub fn resume() -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::resume()
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_audio::resume()
    }
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "native-audio"),
        all(feature = "web", target_arch = "wasm32")
    )))]
    {
        Err("audio playback is not enabled for this target".to_string())
    }
}

pub fn pause() -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::pause()
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_audio::pause()
    }
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "native-audio"),
        all(feature = "web", target_arch = "wasm32")
    )))]
    {
        Err("audio playback is not enabled for this target".to_string())
    }
}

pub fn stop() -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::stop()
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_audio::stop()
    }
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "native-audio"),
        all(feature = "web", target_arch = "wasm32")
    )))]
    {
        Err("audio playback is not enabled for this target".to_string())
    }
}

pub fn set_volume(volume: f32) -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::set_volume(volume)
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_audio::set_volume(volume)
    }
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "native-audio"),
        all(feature = "web", target_arch = "wasm32")
    )))]
    {
        let _ = volume;
        Ok(())
    }
}

pub fn set_equalizer(enabled: bool, values: [f32; 11]) -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::set_equalizer(enabled, values);
        Ok(())
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-audio")))]
    {
        let _ = (enabled, values);
        Ok(())
    }
}

pub fn seek_fraction(fraction: f32) -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::seek_fraction(fraction)
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_audio::seek_fraction(fraction)
    }
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "native-audio"),
        all(feature = "web", target_arch = "wasm32")
    )))]
    {
        let _ = fraction;
        Ok(())
    }
}

pub fn playback_progress() -> Result<Option<PlaybackProgress>, String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::playback_progress()
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_audio::playback_progress()
    }
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "native-audio"),
        all(feature = "web", target_arch = "wasm32")
    )))]
    {
        Ok(None)
    }
}

pub fn probe_track_duration_seconds(path: &std::path::Path) -> Result<Option<f32>, String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::probe_track_duration_seconds(path)
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-audio")))]
    {
        let _ = path;
        Ok(None)
    }
}

pub fn visualizer_bands() -> VisualizerBands {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::visualizer_bands()
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-audio")))]
    {
        [0.0; VISUALIZER_BAND_COUNT]
    }
}

#[cfg(test)]
mod tests {
    use super::{dialog_audio_extensions, supported_audio_extensions};

    #[test]
    fn extensions_include_common_winamp_formats() {
        let extensions = supported_audio_extensions();
        assert!(extensions.contains(&"mp3"));
        assert!(extensions.contains(&"flac"));
        assert!(extensions.contains(&"m4a"));
        assert!(extensions.contains(&"mp4"));
        assert!(extensions.contains(&"ogg"));
        assert!(extensions.contains(&"wav"));
    }

    #[test]
    fn dialog_extensions_include_uppercase_variants() {
        let extensions = dialog_audio_extensions();
        assert!(extensions.iter().any(|extension| extension == "MP4"));
        assert!(extensions.iter().any(|extension| extension == "M4A"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn audio_path_detection_is_case_insensitive() {
        assert!(super::is_audio_path(std::path::Path::new("SONG.MP4")));
        assert!(super::is_audio_path(std::path::Path::new(
            "Album/Track.FLAC"
        )));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn explicit_file_selection_is_preserved_for_decoder_feedback() {
        let tracks = super::tracks_from_selected_paths([
            std::path::PathBuf::from("/tmp/video.mp4"),
            std::path::PathBuf::from("/tmp/unknown-extension.custom"),
        ]);

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "unknown-extension");
        assert_eq!(tracks[1].title, "video");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn demo_playlist_uses_external_mp3_files() {
        let tracks = super::demo_playlist_tracks();

        assert_eq!(tracks.len(), 5);
        assert!(tracks.iter().all(|track| {
            track
                .path
                .as_deref()
                .map(|path| path.ends_with(".mp3"))
                .unwrap_or(false)
        }));
        assert_eq!(
            tracks
                .iter()
                .map(|track| track.duration_seconds)
                .collect::<Vec<_>>(),
            vec![
                Some(100.0),
                Some(90.0),
                Some(120.0),
                Some(100.0),
                Some(90.0)
            ]
        );
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    #[test]
    fn mp4_video_container_decodes_audio_track() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tone-video.mp4");
        let (mut source, duration) = super::native::decode_track_source(&path, false)
            .expect("video mp4 should decode its AAC audio track");
        assert!(duration.is_some());
        assert!(source.by_ref().take(4096).count() > 0);
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    #[test]
    fn stream_cache_streams_and_seeks_a_nonseekable_source() {
        use std::io::{Read, Seek, SeekFrom};

        // A reader that refuses to seek, mimicking a network/WebDAV pipe.
        struct PipeReader(std::io::Cursor<Vec<u8>>);
        impl Read for PipeReader {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                self.0.read(buf)
            }
        }

        let data: Vec<u8> = (0..20_000u32).map(|byte| byte as u8).collect();
        let source = Box::new(PipeReader(std::io::Cursor::new(data.clone())));
        let mut cache =
            super::native::StreamCache::spool(source).expect("spool should start the cache");

        // Sequential read blocks only until bytes arrive, then yields all of them.
        let mut streamed = vec![0_u8; data.len()];
        cache
            .read_exact(&mut streamed)
            .expect("the whole source should stream through");
        assert_eq!(streamed, data);

        // Backward seek into already-cached data is exact.
        cache.seek(SeekFrom::Start(1_000)).expect("seek back");
        let mut window = [0_u8; 256];
        cache.read_exact(&mut window).expect("re-read cached bytes");
        assert_eq!(&window[..], &data[1_000..1_256]);

        // Seeking to the end resolves to the full, now-known length.
        assert_eq!(
            cache.seek(SeekFrom::End(0)).expect("seek end"),
            data.len() as u64
        );
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    #[test]
    fn real_files_decode_through_the_seekable_rodio_path() {
        for relative in [
            "assets/demo-music/generated/cranamp-demo-01-retro-tracker.mp3",
            "assets/demo-music/generated/cranamp-demo-01-retro-tracker.ogg",
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
            let (mut source, duration) = super::native::decode_track_source(&path, false)
                .unwrap_or_else(|error| panic!("{relative} should decode: {error}"));
            assert!(duration.is_some(), "{relative} should report a duration");
            assert!(
                source.by_ref().take(4096).count() > 0,
                "{relative} should yield samples"
            );
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    #[test]
    fn analyzer_bands_follow_sample_energy() {
        let sample_rate = 44_100;
        let samples = (0..2048)
            .map(|sample| {
                let phase = (sample as f32 * 440.0 * std::f32::consts::TAU) / sample_rate as f32;
                phase.sin() * 0.8
            })
            .collect::<Vec<_>>();

        let bands = super::native::compute_analyzer_bands(&samples, sample_rate);

        assert!(bands.iter().any(|band| *band > 0.2));
    }
}

/// Regression net for the unified symphonia decode path: every format now
/// routes through [`native::SymphoniaAudioSource`] (rodio carries no decoder
/// features), so decoding and seeking must keep working for the formats the
/// bundled demo playlist ships.
#[cfg(all(test, not(target_arch = "wasm32"), feature = "native-audio"))]
mod symphonia_source_tests {
    use rodio::Source;
    use std::path::Path;
    use std::time::Duration;

    fn demo_track(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/demo-music/generated")
            .join(name)
    }

    fn decode(name: &str) -> (super::native::BoxedSource, Option<Duration>) {
        super::native::decode_track_source(&demo_track(name), false)
            .unwrap_or_else(|error| panic!("decode {name}: {error}"))
    }

    #[test]
    fn demo_tracks_decode_with_duration_and_samples() {
        for name in [
            "cranamp-demo-01-retro-tracker.mp3",
            "cranamp-demo-01-retro-tracker.ogg",
        ] {
            let (mut source, duration) = decode(name);
            let duration = duration.unwrap_or_else(|| panic!("{name} should report duration"));
            assert!(duration > Duration::from_secs(1), "{name}: {duration:?}");
            assert!(source.sample_rate().get() > 8_000, "{name}");
            assert!(source.channels().get() >= 1, "{name}");
            let decoded = source.by_ref().take(48_000).count();
            assert_eq!(decoded, 48_000, "{name} should yield a second of samples");
        }
    }

    #[test]
    fn seeking_moves_forward_and_keeps_decoding() {
        for name in [
            "cranamp-demo-01-retro-tracker.mp3",
            "cranamp-demo-01-retro-tracker.ogg",
        ] {
            let (mut source, _) = decode(name);
            source
                .try_seek(Duration::from_secs(2))
                .unwrap_or_else(|error| panic!("seek {name}: {error}"));
            let decoded = source.by_ref().take(4_800).count();
            assert_eq!(decoded, 4_800, "{name} should keep decoding after seek");
        }
    }
}
