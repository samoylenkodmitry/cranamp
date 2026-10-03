#![allow(clippy::missing_errors_doc)]
use cranpose_services::{MediaItem, MediaMetadata};
use std::f32::consts::PI;
use std::time::Duration;
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
    if cfg!(feature = "store") {
        return Vec::new();
    }
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
    if cfg!(feature = "store") {
        return Vec::new();
    }
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
pub async fn tracks_from_picked_entry(
    entry: cranpose_services::ContentHandle,
) -> Result<Vec<Track>, String> {
    Ok(track_from_picked_file(entry).await?.into_iter().collect())
}
pub async fn track_from_picked_file(
    entry: cranpose_services::ContentHandle,
) -> Result<Option<Track>, String> {
    if !is_audio_name(&entry.metadata().name) {
        return Ok(None);
    }
    let name = entry.metadata().name;
    let title = name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(&name)
        .to_string();
    let location = imported_audio_location(&entry)
        .await
        .map_err(|error| format!("Cannot import {name}: {error:#}"))?;
    Ok(Some(track_from_title_path(title, location)))
}
#[cfg(not(target_arch = "wasm32"))]
async fn imported_audio_location(
    entry: &cranpose_services::ContentHandle,
) -> anyhow::Result<String> {
    let display = entry.metadata().identifier;
    if !cfg!(any(
        target_os = "ios",
        target_os = "android",
        all(target_os = "macos", feature = "store")
    )) && std::path::Path::new(&display).is_file()
    {
        return Ok(display);
    }
    let directory = cranpose::application_directories()?
        .data
        .join("imported-audio");
    copy_audio_into(entry, &directory).await
}
#[cfg(not(target_arch = "wasm32"))]
async fn copy_audio_into(
    entry: &cranpose_services::ContentHandle,
    directory: &std::path::Path,
) -> anyhow::Result<String> {
    use std::io::Write;
    const LIMIT: u64 = 2 * 1024 * 1024 * 1024;
    anyhow::ensure!(
        entry.metadata().len.unwrap_or(0) <= LIMIT,
        "Imported audio is limited to 2 GiB per file"
    );
    std::fs::create_dir_all(directory)?;
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce)
        .map_err(|error| anyhow::anyhow!("Cannot create an import name: {error}"))?;
    let unique: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    let destination = directory.join(format!(
        "{unique}-{}",
        picker_safe_file_name(&entry.metadata().name)
    ));
    let partial = destination.with_extension("partial");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&partial)?;
    let mut pending = PendingAudioImport {
        path: partial,
        committed: false,
    };
    let mut file = std::io::BufWriter::new(file);
    let reader = entry.open().await?;
    let mut written = 0u64;
    while let Some(chunk) = reader.read_chunk().await? {
        written += chunk.len() as u64;
        anyhow::ensure!(
            written <= LIMIT,
            "Imported audio is limited to 2 GiB per file"
        );
        file.write_all(&chunk)?;
    }
    file.flush()?;
    drop(file);
    std::fs::rename(&pending.path, &destination)?;
    pending.committed = true;
    Ok(destination.to_string_lossy().into_owned())
}
#[cfg(not(target_arch = "wasm32"))]
struct PendingAudioImport {
    path: std::path::PathBuf,
    committed: bool,
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for PendingAudioImport {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
#[cfg(target_arch = "wasm32")]
async fn imported_audio_location(
    entry: &cranpose_services::ContentHandle,
) -> anyhow::Result<String> {
    let bytes = crate::content_io::read_content(entry, 64 * 1024 * 1024).await?;
    Ok(browser_audio_url(&bytes))
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function browser_audio_url(bytes) { return URL.createObjectURL(new Blob([bytes])); }"
)]
extern "C" {
    fn browser_audio_url(bytes: &[u8]) -> String;
}
fn is_audio_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    supported_audio_extensions()
        .iter()
        .any(|extension| lower.ends_with(&format!(".{extension}")))
}
#[cfg(not(target_arch = "wasm32"))]
fn picker_safe_file_name(name: &str) -> String {
    let safe: String = name
        .chars()
        .take(128)
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
        "aac", "aif", "aiff", "caf", "flac", "m4a", "m4b", "mka", "mkv", "mp1", "mp2", "mp3",
        "mp4", "oga", "ogg", "wav", "wave", "webm",
    ]
}
const EQUALIZER_MAX_GAIN_DB: f32 = 12.0;
fn equalizer_value_gain_db(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) - 0.5) * 2.0 * EQUALIZER_MAX_GAIN_DB
}
fn media_item(track: &Track) -> Option<MediaItem> {
    let path = track.path.as_deref()?;
    let uri = if cfg!(target_arch = "wasm32") || has_uri_scheme(path) {
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
pub(crate) fn has_uri_scheme(value: &str) -> bool {
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
#[derive(Clone, Copy)]
pub struct PlaybackSettings {
    pub volume: f32,
    pub balance: f32,
    pub equalizer_enabled: bool,
    pub equalizer_values: [f32; 11],
}
pub fn playing_status(track: &Track) -> String {
    if cranpose_services::media_capabilities().equalizer {
        format!("Playing {}", track.display_title())
    } else {
        format!(
            "Playing {} · EQ, balance and visualizer unavailable for this stream",
            track.display_title()
        )
    }
}
pub fn play_track(track: &Track, settings: PlaybackSettings) -> Result<(), String> {
    let item = media_item(track).ok_or_else(|| "track has no source".to_string())?;
    cranpose_services::set_media_volume(settings.volume);
    cranpose_services::set_media_looping(false);
    cranpose_services::open_media(item).map_err(|error| error.to_string())?;
    let _ = set_balance(settings.balance);
    let _ = set_equalizer(settings.equalizer_enabled, settings.equalizer_values);
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
pub fn set_balance(fraction: f32) -> Result<(), String> {
    if cranpose_services::set_media_balance(fraction.clamp(0.0, 1.0) * 2.0 - 1.0) {
        Ok(())
    } else {
        Err("Balance is unavailable for this stream".to_string())
    }
}
pub fn set_equalizer(enabled: bool, values: [f32; 11]) -> Result<(), String> {
    let bands = cranpose_services::media_equalizer_bands();
    let settings = cranpose_services::EqualizerSettings {
        enabled,
        preamp_db: equalizer_value_gain_db(values[0]),
        gains_db: values[1..]
            .iter()
            .copied()
            .map(equalizer_value_gain_db)
            .collect(),
    }
    .clamped_to(&bands);
    if cranpose_services::set_media_equalizer(settings) {
        Ok(())
    } else {
        Err("Equalizer is unavailable for this stream".to_string())
    }
}
pub fn seek_fraction(fraction: f32) -> Result<(), String> {
    cranpose_services::seek_media_fraction(fraction).map_err(|error| error.to_string())
}
pub fn probe_track_duration_seconds(path: &std::path::Path) -> Result<Option<f32>, String> {
    let uri = match path.to_str() {
        Some(text) if has_uri_scheme(text) => text.to_string(),
        _ => cranpose_services::uri_for_path(path),
    };
    Ok(
        cranpose_services::probe_media_duration(&MediaItem::new(uri))
            .map(|duration| duration.as_secs_f32()),
    )
}
pub fn visualizer_bands() -> VisualizerBands {
    if !cranpose_services::media_capabilities().analysis {
        return [0.0; VISUALIZER_BAND_COUNT];
    }
    let Some(samples) = cranpose_services::latest_media_samples() else {
        return [0.0; VISUALIZER_BAND_COUNT];
    };
    compute_analyzer_bands(&samples.samples, samples.sample_rate, samples.channels)
}
/// How many samples of the wave the oscilloscope spreads across itself, the
/// size of the chunks Winamp handed its visualizers.
const VISUALIZER_WAVE_LEN: usize = 576;
/// The latest stretch of the wave, mixed down to one channel, from -1 to 1;
/// nothing while no song is sounding.
pub fn visualizer_wave() -> Vec<f32> {
    if !cranpose_services::media_capabilities().analysis {
        return Vec::new();
    }
    let Some(samples) = cranpose_services::latest_media_samples() else {
        return Vec::new();
    };
    mixed_down(&samples.samples, samples.channels, VISUALIZER_WAVE_LEN)
}
fn mixed_down(samples: &[f32], channels: u16, len: usize) -> Vec<f32> {
    let channels = usize::from(channels.max(1));
    samples
        .chunks_exact(channels)
        .take(len)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
fn compute_analyzer_bands(samples: &[f32], sample_rate: u32, channels: u16) -> VisualizerBands {
    let channels = usize::from(channels.max(1));
    let frames = samples.chunks_exact(channels);
    if frames.len() == 0 || sample_rate == 0 {
        return [0.0; VISUALIZER_BAND_COUNT];
    }
    let nyquist = sample_rate as f32 * 0.5;
    let min_frequency = 60.0_f32;
    let max_frequency = nyquist.min(12_000.0).max(min_frequency + 1.0);
    let frequency_ratio = max_frequency / min_frequency;
    let sample_count = frames.len() as f32;
    std::array::from_fn(|band| {
        let position = band as f32 / (VISUALIZER_BAND_COUNT - 1) as f32;
        let frequency = min_frequency * frequency_ratio.powf(position);
        let omega = 2.0 * PI * frequency / sample_rate as f32;
        let coeff = 2.0 * omega.cos();
        let mut previous = 0.0;
        let mut previous_2 = 0.0;
        for frame in frames.clone() {
            let sample = frame.iter().sum::<f32>() / channels as f32;
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
#[path = "../test/unit/audio/tests.rs"]
mod tests;
