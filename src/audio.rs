#![allow(clippy::missing_errors_doc)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Track {
    pub title: String,
    pub path: Option<String>,
}

impl Track {
    pub fn display_title(&self) -> &str {
        self.title.as_str()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_path(path: std::path::PathBuf) -> Self {
        let title = path
            .file_stem()
            .or_else(|| path.file_name())
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Untitled")
            .to_string();

        Self {
            title,
            path: Some(path.to_string_lossy().to_string()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
pub struct PickedWebTrack {
    pub track: Track,
    pub bytes: Vec<u8>,
}

pub fn supported_audio_extensions() -> &'static [&'static str] {
    &[
        "aac", "aiff", "alac", "caf", "flac", "m4a", "m4b", "mp1", "mp2", "mp3", "mp4", "oga",
        "ogg", "opus", "wav", "wave", "webm",
    ]
}

#[cfg(not(target_arch = "wasm32"))]
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
pub fn pick_audio_files() -> Result<Option<Vec<Track>>, String> {
    let files = rfd::FileDialog::new()
        .set_title("Open audio files")
        .add_filter("Audio", supported_audio_extensions())
        .pick_files();

    Ok(files.map(|paths| {
        let mut tracks = paths
            .into_iter()
            .filter(|path| is_audio_path(path))
            .map(Track::from_path)
            .collect::<Vec<_>>();
        tracks.sort_by(|left, right| left.title.cmp(&right.title));
        tracks
    }))
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
    let Some(folder) = rfd::FileDialog::new()
        .set_title("Open audio folder")
        .pick_folder()
    else {
        return Ok(None);
    };

    let mut tracks = walkdir::WalkDir::new(folder)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| is_audio_path(path))
        .map(Track::from_path)
        .collect::<Vec<_>>();

    tracks.sort_by(|left, right| left.title.cmp(&right.title));
    Ok(Some(tracks))
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

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub async fn pick_web_audio_file() -> Result<Option<PickedWebTrack>, String> {
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_title("Open audio file")
        .add_filter("Audio", supported_audio_extensions())
        .pick_file()
        .await
    else {
        return Ok(None);
    };

    let title = handle.file_name();
    let bytes = handle.read().await;
    Ok(Some(PickedWebTrack {
        track: Track { title, path: None },
        bytes,
    }))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
mod native {
    use super::Track;
    use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source};
    use std::fs::File;
    use std::io::BufReader;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    struct NativeAudio {
        sink: MixerDeviceSink,
        player: Option<Player>,
    }

    static AUDIO: OnceLock<Mutex<Option<NativeAudio>>> = OnceLock::new();

    fn with_audio<T>(f: impl FnOnce(&mut NativeAudio) -> Result<T, String>) -> Result<T, String> {
        let audio = AUDIO.get_or_init(|| Mutex::new(None));
        let mut audio = audio
            .lock()
            .map_err(|_| "audio backend lock is poisoned".to_string())?;

        if audio.is_none() {
            let mut sink = DeviceSinkBuilder::open_default_sink()
                .map_err(|error| format!("failed to open default audio device: {error}"))?;
            sink.log_on_drop(false);
            *audio = Some(NativeAudio { sink, player: None });
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
        let file = File::open(path).map_err(|error| format!("failed to open {path}: {error}"))?;

        with_audio(|audio| {
            if let Some(player) = audio.player.take() {
                player.stop();
            }

            let player = Player::connect_new(audio.sink.mixer());
            player.set_volume(volume.clamp(0.0, 1.0));

            if repeat {
                let source = rodio::Decoder::new_looped(BufReader::new(file))
                    .map_err(|error| format!("failed to decode {}: {error}", track.title))?
                    .amplify(1.0);
                player.append(source);
            } else {
                let source = rodio::Decoder::try_from(file)
                    .map_err(|error| format!("failed to decode {}: {error}", track.title))?
                    .amplify(1.0);
                player.append(source);
            }

            player.play();
            audio.player = Some(player);
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
                let seconds = (fraction.clamp(0.0, 1.0) * 300.0).round();
                let _ = player.try_seek(Duration::from_secs(seconds as u64));
            }
            Ok(())
        })
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod web_audio {
    use std::cell::RefCell;
    use wasm_bindgen::JsValue;

    struct WebAudio {
        element: web_sys::HtmlAudioElement,
        url: String,
    }

    thread_local! {
        static AUDIO: RefCell<Option<WebAudio>> = const { RefCell::new(None) };
    }

    fn js_error(value: JsValue) -> String {
        value
            .as_string()
            .unwrap_or_else(|| "browser audio operation failed".to_string())
    }

    pub fn play_bytes(bytes: Vec<u8>, volume: f32, repeat: bool) -> Result<(), String> {
        let parts = js_sys::Array::new();
        parts.push(&js_sys::Uint8Array::from(bytes.as_slice()));
        let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(js_error)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
        let element = web_sys::HtmlAudioElement::new_with_src(&url).map_err(js_error)?;
        element.set_volume(volume.clamp(0.0, 1.0) as f64);
        element.set_loop(repeat);
        let _ = element.play().map_err(js_error)?;

        AUDIO.with(|audio| {
            if let Some(previous) = audio.borrow_mut().take() {
                previous.element.pause().ok();
                let _ = web_sys::Url::revoke_object_url(&previous.url);
            }
            *audio.borrow_mut() = Some(WebAudio { element, url });
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
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn play_web_bytes(bytes: Vec<u8>, volume: f32, repeat: bool) -> Result<(), String> {
    web_audio::play_bytes(bytes, volume, repeat)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
pub fn play_track(track: &Track, volume: f32, repeat: bool) -> Result<(), String> {
    native::play_track(track, volume, repeat)
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "native-audio")))]
pub fn play_track(_track: &Track, _volume: f32, _repeat: bool) -> Result<(), String> {
    Err("native audio playback is not enabled for this target".to_string())
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

pub fn seek_fraction(fraction: f32) -> Result<(), String> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
    {
        native::seek_fraction(fraction)
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "native-audio")))]
    {
        let _ = fraction;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::supported_audio_extensions;

    #[test]
    fn extensions_include_common_winamp_formats() {
        let extensions = supported_audio_extensions();
        assert!(extensions.contains(&"mp3"));
        assert!(extensions.contains(&"flac"));
        assert!(extensions.contains(&"ogg"));
        assert!(extensions.contains(&"wav"));
    }
}
