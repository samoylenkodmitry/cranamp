#![allow(clippy::missing_errors_doc)]

//! Cranamp's audio layer.
//!
//! Track discovery, titles, the demo playlist and the visualiser's look are
//! Cranamp's own. Everything that makes sound - the decoder, the output device,
//! audio focus, the media session, seeking and the equalizer - is
//! `cranpose_services::media`, which has a backend on every target Cranamp
//! ships to.

use std::f32::consts::PI;
use std::time::Duration;

use cranpose_services::{MediaItem, MediaMetadata};

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub title: String,
    pub path: Option<String>,
    pub duration_seconds: Option<f32>,
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
pub async fn tracks_from_picked_entry(entry: cranpose_services::ContentHandle) -> Vec<Track> {
    let cache = picker_cache_dir();
    // A picked file is a file; the folder case comes through the entry enum.
    let mut tracks =
        collect_picked_audio_tracks(cranpose_services::ContentEntry::File(entry), cache).await;
    tracks.sort_by(|a, b| a.display_title().cmp(b.display_title()));
    tracks
}

/// Builds a single playable track from one file entry yielded by a streaming
/// folder pick ([`cranpose_services::FolderStream`]), or `None` if the entry is
/// not a supported audio file. Callers append discovered tracks incrementally,
/// so a huge folder on a slow provider starts playing before the walk finishes.
pub async fn track_from_picked_file(entry: cranpose_services::ContentHandle) -> Option<Track> {
    // A file stream yields files, and the handle's type says so, so the only
    // question left is whether this one is audio.
    if !is_audio_name(&entry.metadata().name) {
        return None;
    }
    let cache = picker_cache_dir();
    picked_audio_track(&entry, &cache).await
}

fn picker_cache_dir() -> std::path::PathBuf {
    let dir = cranpose::application_directories()
        .map(|directories| directories.temporary.join("picker"))
        .unwrap_or_else(|_| std::env::temp_dir().join("cranamp-picker"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn collect_picked_audio_tracks(
    entry: cranpose_services::ContentEntry,
    cache: std::path::PathBuf,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<Track>>>> {
    Box::pin(async move {
        let mut tracks = Vec::new();
        // A file and a folder are different types now rather than one handle
        // answering `kind()`, so a branch that forgets to check cannot compile.
        match entry {
            cranpose_services::ContentEntry::Folder(folder) => {
                if let Ok(children) = folder.entries().await {
                    for child in children {
                        tracks.extend(collect_picked_audio_tracks(child, cache.clone()).await);
                    }
                }
            }
            cranpose_services::ContentEntry::File(file) => {
                if is_audio_name(&file.metadata().name) {
                    if let Some(track) = picked_audio_track(&file, &cache).await {
                        tracks.push(track);
                    }
                }
            }
        }
        tracks
    })
}

async fn picked_audio_track(
    entry: &cranpose_services::ContentHandle,
    cache: &std::path::Path,
) -> Option<Track> {
    let name = entry.metadata().name;
    let title = name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(&name)
        .to_string();
    let display = entry.metadata().identifier;
    // `content://` (Android) streams from the provider; a real filesystem path
    // (desktop) is used as-is. Neither is copied.
    let location = if display.starts_with("content://") || std::path::Path::new(&display).is_file()
    {
        display
    } else {
        // iOS security-scoped URLs are not re-openable later; cache the bytes.
        let bytes = entry.read_all().await.ok()?;
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

/// How far a slider at either extreme moves a band, in decibels.
const EQUALIZER_MAX_GAIN_DB: f32 = 12.0;

/// A slider position in `0..=1` as the decibel gain it means, centre being flat.
fn equalizer_value_gain_db(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) - 0.5) * 2.0 * EQUALIZER_MAX_GAIN_DB
}

/// The media item a track addresses.
///
/// A track's `path` is a filesystem path on desktop, Android and iOS, and a
/// blob object URL in the browser; anything already carrying a scheme is a URI
/// and the rest is a path.
fn media_item(track: &Track) -> Option<MediaItem> {
    let path = track.path.as_deref()?;
    let uri = if has_uri_scheme(path) {
        path.to_string()
    } else {
        cranpose_services::uri_for_path(std::path::Path::new(path))
    };
    let mut metadata = MediaMetadata::titled(track.display_title());
    if let Some(duration) = track.duration_seconds {
        metadata = metadata.duration(Duration::from_secs_f32(duration.max(0.0)));
    }
    Some(MediaItem::new(uri).with_metadata(metadata))
}

/// Whether `value` already names a scheme, as `file:`, `content:` and `blob:`
/// do. A bare Windows drive letter is a path, not a one-letter scheme.
fn has_uri_scheme(value: &str) -> bool {
    match value.split_once(':') {
        Some((scheme, _)) => {
            scheme.len() > 1
                && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        }
        None => false,
    }
}

pub fn play_track(track: &Track, volume: f32, repeat: bool) -> Result<(), String> {
    let item = media_item(track).ok_or_else(|| "track has no source".to_string())?;
    cranpose_services::set_media_volume(volume);
    cranpose_services::set_media_looping(repeat);
    cranpose_services::open_media(item).map_err(|error| error.to_string())?;
    // The analyser is off until something asks for it, and the visualiser is
    // the only thing that does.
    cranpose_services::set_media_analysis_enabled(true);
    cranpose_services::play_media().map_err(|error| error.to_string())
}

pub fn resume() -> Result<(), String> {
    cranpose_services::play_media().map_err(|error| error.to_string())
}

pub fn pause() -> Result<(), String> {
    cranpose_services::pause_media();
    Ok(())
}

pub fn stop() -> Result<(), String> {
    cranpose_services::stop_media();
    Ok(())
}

pub fn set_volume(volume: f32) -> Result<(), String> {
    cranpose_services::set_media_volume(volume);
    Ok(())
}

/// Applies the equalizer screen's eleven sliders: a preamp and ten bands.
///
/// The backend states the bands it actually has, so the gains are mapped onto
/// however many that is rather than assuming the screen's ten.
pub fn set_equalizer(enabled: bool, values: [f32; 11]) -> Result<(), String> {
    let bands = cranpose_services::media_equalizer_bands();
    let settings = cranpose_services::EqualizerSettings {
        enabled,
        preamp_db: equalizer_value_gain_db(values[0]),
        gains_db: values[1..].iter().copied().map(equalizer_value_gain_db).collect(),
    }
    .clamped_to(&bands);
    cranpose_services::set_media_equalizer(settings);
    Ok(())
}

pub fn seek_fraction(fraction: f32) -> Result<(), String> {
    cranpose_services::seek_media_fraction(fraction).map_err(|error| error.to_string())
}

/// Reads a track's length without playing it, for the playlist's duration
/// column. `None` where the backend has no probe — see
/// [`cranpose_services::MediaCapabilities::probing`].
pub fn probe_track_duration_seconds(path: &std::path::Path) -> Result<Option<f32>, String> {
    let uri = match path.to_str() {
        Some(text) if has_uri_scheme(text) => text.to_string(),
        _ => cranpose_services::uri_for_path(path),
    };
    Ok(cranpose_services::probe_media_duration(&MediaItem::new(uri))
        .map(|duration| duration.as_secs_f32()))
}

/// The visualiser's bands, taken from the samples the backend is playing.
///
/// The band layout and the response curve are Cranamp's own look; what they
/// read is the framework's analysis tap.
pub fn visualizer_bands() -> VisualizerBands {
    let Some(samples) = cranpose_services::latest_media_samples() else {
        return [0.0; VISUALIZER_BAND_COUNT];
    };
    compute_analyzer_bands(&samples.samples, samples.sample_rate)
}

fn compute_analyzer_bands(samples: &[f32], sample_rate: u32) -> VisualizerBands {
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

#[cfg(test)]
mod tests {
    use super::{compute_analyzer_bands, equalizer_value_gain_db, has_uri_scheme, media_item, Track};

    #[test]
    fn extensions_include_common_winamp_formats() {
        let extensions = super::supported_audio_extensions();
        assert!(extensions.contains(&"mp3"));
        assert!(extensions.contains(&"flac"));
        assert!(extensions.contains(&"m4a"));
        assert!(extensions.contains(&"mp4"));
        assert!(extensions.contains(&"ogg"));
        assert!(extensions.contains(&"wav"));
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

    #[test]
    fn analyzer_bands_follow_sample_energy() {
        let sample_rate = 44_100;
        let samples = (0..2048)
            .map(|sample| {
                let phase = (sample as f32 * 440.0 * std::f32::consts::TAU) / sample_rate as f32;
                phase.sin() * 0.8
            })
            .collect::<Vec<_>>();

        let bands = compute_analyzer_bands(&samples, sample_rate);

        assert!(bands.iter().any(|band| *band > 0.2));
    }

    /// A track addresses its source differently per platform, and the media
    /// contract takes a URI. Anything already carrying a scheme is one; a bare
    /// path - including a Windows drive letter, whose single letter is not a
    /// scheme - is not.
    #[test]
    fn a_source_that_names_a_scheme_is_a_uri_and_the_rest_is_a_path() {
        assert!(has_uri_scheme("blob:https://example.test/9f2c"));
        assert!(has_uri_scheme("content://media/external/audio/media/42"));
        assert!(has_uri_scheme("file:///music/track.mp3"));
        assert!(!has_uri_scheme("/music/track.mp3"));
        assert!(!has_uri_scheme("C:\\Music\\track.mp3"));
        assert!(!has_uri_scheme("relative/track.mp3"));
    }

    #[test]
    fn a_blob_backed_track_keeps_its_url_and_a_path_becomes_a_file_uri() {
        let blob = Track {
            title: "Web".to_string(),
            path: Some("blob:https://example.test/9f2c".to_string()),
            duration_seconds: None,
        };
        assert_eq!(
            media_item(&blob).expect("a blob-backed track has a source").uri,
            "blob:https://example.test/9f2c"
        );

        let local = Track {
            title: "Local".to_string(),
            path: Some("/music/track.mp3".to_string()),
            duration_seconds: None,
        };
        assert_eq!(
            media_item(&local).expect("a local track has a source").uri,
            "file:///music/track.mp3"
        );

        let missing = Track {
            title: "Nothing".to_string(),
            path: None,
            duration_seconds: None,
        };
        assert!(media_item(&missing).is_none());
    }

    /// The equalizer screen's sliders run 0..1 with the centre flat, and the
    /// media contract takes decibels.
    #[test]
    fn a_centred_equalizer_slider_is_flat_and_the_ends_are_symmetric() {
        assert_eq!(equalizer_value_gain_db(0.5), 0.0);
        assert_eq!(equalizer_value_gain_db(1.0), 12.0);
        assert_eq!(equalizer_value_gain_db(0.0), -12.0);
        // Out-of-range slider positions clamp rather than exceeding the range
        // the bands accept.
        assert_eq!(equalizer_value_gain_db(2.0), 12.0);
        assert_eq!(equalizer_value_gain_db(-1.0), -12.0);
    }
}
