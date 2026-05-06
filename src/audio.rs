#![allow(clippy::missing_errors_doc)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Track {
    pub title: String,
    pub path: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackProgress {
    pub elapsed_seconds: f32,
    pub duration_seconds: Option<f32>,
    pub finished: bool,
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
        "aac", "aiff", "alac", "caf", "flac", "m4a", "m4b", "m4v", "mov", "mp1", "mp2", "mp3",
        "mp4", "oga", "ogg", "opus", "wav", "wave", "webm",
    ]
}

fn dialog_audio_extensions() -> Vec<String> {
    let mut extensions = Vec::with_capacity(supported_audio_extensions().len() * 2);
    for extension in supported_audio_extensions() {
        extensions.push((*extension).to_string());
        extensions.push(extension.to_ascii_uppercase());
    }
    extensions
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

#[cfg(not(target_arch = "wasm32"))]
fn tracks_from_selected_paths(paths: impl IntoIterator<Item = std::path::PathBuf>) -> Vec<Track> {
    sort_tracks(paths.into_iter().map(Track::from_path).collect())
}

#[cfg(not(target_arch = "wasm32"))]
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
    let Some(folder) = rfd::FileDialog::new()
        .set_title("Open audio folder")
        .pick_folder()
    else {
        return Ok(None);
    };

    let tracks = walkdir::WalkDir::new(folder)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| is_audio_path(path))
        .map(Track::from_path)
        .collect::<Vec<_>>();

    Ok(Some(sort_tracks(tracks)))
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
    let extensions = dialog_audio_extensions();
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_title("Open audio file")
        .add_filter("Audio", &extensions)
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
    use rodio::{
        ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, Sample, SampleRate, Source,
    };
    use std::fs::File;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    use symphonia::core::audio::{AudioBufferRef, SampleBuffer, SignalSpec};
    use symphonia::core::codecs::{Decoder as SymphoniaDecoder, DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::errors::Error as SymphoniaError;
    use symphonia::core::formats::{FormatOptions, FormatReader};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::core::units;

    struct NativeAudio {
        sink: MixerDeviceSink,
        player: Option<Player>,
        duration: Option<Duration>,
    }

    static AUDIO: OnceLock<Mutex<Option<NativeAudio>>> = OnceLock::new();
    type BoxedSource = Box<dyn Source + Send>;
    type SelectedAudioTrack = (u32, Box<dyn SymphoniaDecoder>, Option<Duration>);

    struct IsoMp4AudioSource {
        decoder: Box<dyn SymphoniaDecoder>,
        format: Box<dyn FormatReader>,
        track_id: u32,
        samples: Vec<Sample>,
        sample_offset: usize,
        spec: SignalSpec,
        total_duration: Option<Duration>,
        exhausted: bool,
    }

    impl IsoMp4AudioSource {
        fn new(file: File, extension_hint: Option<&str>) -> Result<Self, String> {
            let mut hint = Hint::new();
            if let Some(extension_hint) = extension_hint {
                hint.with_extension(extension_hint);
            }

            let stream = MediaSourceStream::new(Box::new(file), Default::default());
            let mut probed = symphonia::default::get_probe()
                .format(
                    &hint,
                    stream,
                    &FormatOptions::default(),
                    &MetadataOptions::default(),
                )
                .map_err(format_symphonia_error)?;

            let (track_id, mut decoder, total_duration) = select_audio_track(&*probed.format)?;
            let (spec, samples) =
                read_next_audio_buffer(&mut *probed.format, &mut *decoder, track_id)?
                    .ok_or_else(|| "no decodable audio packets found".to_string())?;

            Ok(Self {
                decoder,
                format: probed.format,
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

    impl Iterator for IsoMp4AudioSource {
        type Item = Sample;

        fn next(&mut self) -> Option<Self::Item> {
            if self.sample_offset >= self.samples.len() && !self.load_next_buffer() {
                return None;
            }

            let sample = self.samples[self.sample_offset];
            self.sample_offset += 1;
            Some(sample)
        }
    }

    impl Source for IsoMp4AudioSource {
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
                    .channels
                    .count()
                    .try_into()
                    .expect("audio channel count exceeds u16::MAX"),
            )
            .expect("audio should have at least one channel")
        }

        fn sample_rate(&self) -> SampleRate {
            SampleRate::new(self.spec.rate).expect("audio should have a non-zero sample rate")
        }

        fn total_duration(&self) -> Option<Duration> {
            self.total_duration
        }
    }

    fn select_audio_track(format: &dyn FormatReader) -> Result<SelectedAudioTrack, String> {
        for track in format.tracks() {
            let params = &track.codec_params;
            if params.codec == CODEC_TYPE_NULL {
                continue;
            }

            let Ok(decoder) =
                symphonia::default::get_codecs().make(params, &DecoderOptions::default())
            else {
                continue;
            };

            let total_duration = params
                .time_base
                .zip(params.n_frames)
                .map(|(base, frames)| base.calc_time(frames).into())
                .filter(|duration: &Duration| !duration.is_zero());

            return Ok((track.id, decoder, total_duration));
        }

        Err("no supported audio track found".to_string())
    }

    fn read_next_audio_buffer(
        format: &mut dyn FormatReader,
        decoder: &mut dyn SymphoniaDecoder,
        track_id: u32,
    ) -> Result<Option<(SignalSpec, Vec<Sample>)>, String> {
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(_)) => return Ok(None),
                Err(error) => return Err(format_symphonia_error(error)),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => return Ok(Some(copy_audio_buffer(decoded))),
                Err(SymphoniaError::DecodeError(_)) | Err(SymphoniaError::IoError(_)) => continue,
                Err(error) => return Err(format_symphonia_error(error)),
            }
        }
    }

    fn copy_audio_buffer(decoded: AudioBufferRef<'_>) -> (SignalSpec, Vec<Sample>) {
        let spec = *decoded.spec();
        let mut samples =
            SampleBuffer::<Sample>::new(units::Duration::from(decoded.capacity() as u64), spec);
        samples.copy_interleaved_ref(decoded);
        (spec, samples.samples().to_vec())
    }

    pub(super) fn decode_track_source(
        path: &Path,
        repeat: bool,
    ) -> Result<(BoxedSource, Option<Duration>), String> {
        let file = File::open(path)
            .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
        let extension_hint = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);

        if extension_hint
            .as_deref()
            .map(is_iso_mp4_extension)
            .unwrap_or(false)
        {
            let source = IsoMp4AudioSource::new(file, extension_hint.as_deref())?;
            let duration = source.total_duration();
            return if repeat {
                Ok((Box::new(source.repeat_infinite()), duration))
            } else {
                Ok((Box::new(source), duration))
            };
        }

        let byte_len = file
            .metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
            .len();
        let mut builder = rodio::Decoder::builder()
            .with_data(file)
            .with_byte_len(byte_len)
            .with_seekable(true);
        if let Some(extension_hint) = extension_hint.as_deref() {
            builder = builder.with_hint(extension_hint);
        }

        let source = builder.build().map_err(|error| error.to_string())?;
        let duration = source.total_duration();
        if repeat {
            Ok((Box::new(source.repeat_infinite()), duration))
        } else {
            Ok((Box::new(source), duration))
        }
    }

    fn is_iso_mp4_extension(extension: &str) -> bool {
        matches!(extension, "m4a" | "m4b" | "m4v" | "mov" | "mp4")
    }

    fn format_symphonia_error(error: SymphoniaError) -> String {
        match error {
            SymphoniaError::Unsupported(reason) => reason.to_string(),
            other => other.to_string(),
        }
    }

    fn with_audio<T>(f: impl FnOnce(&mut NativeAudio) -> Result<T, String>) -> Result<T, String> {
        let audio = AUDIO.get_or_init(|| Mutex::new(None));
        let mut audio = audio
            .lock()
            .map_err(|_| "audio backend lock is poisoned".to_string())?;

        if audio.is_none() {
            let mut sink = DeviceSinkBuilder::open_default_sink()
                .map_err(|error| format!("failed to open default audio device: {error}"))?;
            sink.log_on_drop(false);
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
        let (source, duration) = decode_track_source(Path::new(path), repeat)
            .map_err(|error| format!("failed to decode {}: {error}", track.title))?;

        with_audio(|audio| {
            if let Some(player) = audio.player.take() {
                player.stop();
            }

            let player = Player::connect_new(audio.sink.mixer());
            player.set_volume(volume.clamp(0.0, 1.0));
            player.append(source);
            player.play();
            audio.player = Some(player);
            audio.duration = duration;
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
}
