use cranpose::{
    DesktopIntegration, DesktopIntegrationContext, DesktopWaker, DesktopWindow, DesktopWindowId,
};
use cranpose_services::{MediaCommand, MediaMetadata, MediaObserver};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaPlayback, MediaPosition, PlatformConfig, SeekDirection,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
type Cover = ::windows::Storage::Streams::RandomAccessStreamReference;
#[cfg(not(target_os = "windows"))]
type Cover = String;

#[cfg(target_os = "windows")]
pub(crate) async fn prepare_cover(artwork: cranpose_services::MediaArtwork) -> Option<Cover> {
    cranpose_core::withBlocking(move || windows::prepare_artwork(&artwork.bytes))
        .await
        .ok()?
        .map_err(|error| log::warn!("Prepare system media artwork: {error}"))
        .ok()
}

static DIRTY: AtomicBool = AtomicBool::new(true);

#[derive(Default)]
struct Updates {
    waker: Option<DesktopWaker>,
    cover: Option<(String, Option<Cover>)>,
}
static UPDATES: Mutex<Updates> = Mutex::new(Updates {
    waker: None,
    cover: None,
});

pub(crate) fn cover_changed(uri: String, cover: Option<Cover>) {
    DIRTY.store(true, Ordering::Release);
    let waker = {
        let mut updates = UPDATES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        updates.cover = Some((uri, cover));
        updates.waker.clone()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

#[derive(Default)]
pub(crate) struct Session {
    controls: Option<MediaControls>,
    window: Option<DesktopWindowId>,
    observers: Vec<MediaObserver>,
    metadata: Option<(MediaMetadata, Option<Cover>)>,
    playback: Option<MediaPlayback>,
    #[cfg(target_os = "windows")]
    hwnd: Option<isize>,
}

impl DesktopIntegration for Session {
    fn window_opened(
        &mut self,
        window: &DesktopWindow<'_>,
        context: &mut DesktopIntegrationContext<'_>,
    ) {
        if self.controls.is_some() {
            return;
        }
        #[cfg(target_os = "windows")]
        let hwnd = {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let Ok(handle) = window.window_handle() else {
                return;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return;
            };
            Some(handle.hwnd.get() as *mut std::ffi::c_void)
        };
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;
        let result = MediaControls::new(PlatformConfig {
            dbus_name: "cranamp",
            display_name: "Cranamp",
            hwnd,
        })
        .and_then(|mut controls| {
            controls.attach(command)?;
            Ok(controls)
        });
        let controls = match result {
            Ok(controls) => controls,
            Err(error) => {
                log::warn!("System media controls: {error:?}");
                return;
            }
        };
        let waker = context.waker();
        UPDATES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .waker = Some(waker.clone());
        let progress_waker = waker.clone();
        self.observers = vec![
            cranpose_services::observe_playback_state(move |_| {
                DIRTY.store(true, Ordering::Release);
                waker.wake();
            }),
            cranpose_services::observe_playback_progress(move |_| {
                DIRTY.store(true, Ordering::Release);
                progress_waker.wake();
            }),
        ];
        self.window = Some(window.id());
        #[cfg(target_os = "windows")]
        {
            self.hwnd = hwnd.map(|handle| handle as isize);
        }
        self.controls = Some(controls);
    }

    fn window_closed(&mut self, id: DesktopWindowId, _: &mut DesktopIntegrationContext<'_>) {
        if self.window == Some(id) {
            self.controls = None;
            self.observers.clear();
            self.metadata = None;
            self.playback = None;
            UPDATES
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .waker = None;
        }
    }

    fn about_to_wait(&mut self, _: &mut DesktopIntegrationContext<'_>) {
        if !DIRTY.swap(false, Ordering::AcqRel) {
            return;
        }
        let Some(controls) = self.controls.as_mut() else {
            return;
        };
        let item = cranpose_services::current_media_item();
        let progress = cranpose_services::playback_progress();
        let cover = {
            let updates = UPDATES
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            updates
                .cover
                .as_ref()
                .filter(|(uri, _)| item.as_ref().is_some_and(|item| item.uri == *uri))
                .and_then(|(_, cover)| cover.clone())
        };
        let mut metadata = item.map(|item| item.metadata).unwrap_or_default();
        metadata.duration = progress.duration.or(metadata.duration);
        if self.metadata.as_ref() != Some(&(metadata.clone(), cover.clone())) {
            #[cfg(target_os = "windows")]
            if let Some(hwnd) = self.hwnd {
                if let Err(error) = windows::set_artwork(hwnd, cover.as_ref()) {
                    log::warn!("Set system media artwork: {error}");
                }
            }
            let result = controls.set_metadata(souvlaki::MediaMetadata {
                title: Some(&metadata.title),
                artist: Some(&metadata.artist),
                album: Some(&metadata.album),
                duration: metadata.duration,
                #[cfg(not(target_os = "windows"))]
                cover_url: cover.as_deref(),
                // The prepared stream avoids Souvlaki's synchronous file read.
                #[cfg(target_os = "windows")]
                cover_url: None,
            });
            if let Err(error) = result {
                log::warn!("System media metadata: {error:?}");
            }
            self.metadata = Some((metadata, cover));
        }
        let playback = match cranpose_services::playback_state() {
            cranpose_services::PlaybackState::Playing => MediaPlayback::Playing {
                progress: Some(MediaPosition(progress.position)),
            },
            cranpose_services::PlaybackState::Paused
            | cranpose_services::PlaybackState::Loading => MediaPlayback::Paused {
                progress: Some(MediaPosition(progress.position)),
            },
            _ => MediaPlayback::Stopped,
        };
        if self.playback.as_ref() != Some(&playback) {
            if let Err(error) = controls.set_playback(playback.clone()) {
                log::warn!("System media playback: {error:?}");
            }
            self.playback = Some(playback);
        }
    }
}

fn command(event: MediaControlEvent) {
    let command = match event {
        MediaControlEvent::Play => MediaCommand::Play,
        MediaControlEvent::Pause => MediaCommand::Pause,
        MediaControlEvent::Toggle => MediaCommand::TogglePlayPause,
        MediaControlEvent::Stop => MediaCommand::Stop,
        MediaControlEvent::Next => MediaCommand::Next,
        MediaControlEvent::Previous => MediaCommand::Previous,
        MediaControlEvent::SetPosition(position) => MediaCommand::SeekTo(position.0),
        MediaControlEvent::Seek(direction) => seek(direction, Duration::from_secs(10)),
        MediaControlEvent::SeekBy(direction, distance) => seek(direction, distance),
        _ => return,
    };
    cranpose_services::publish_media_command(command);
}

fn seek(direction: SeekDirection, distance: Duration) -> MediaCommand {
    let position = cranpose_services::playback_progress().position;
    MediaCommand::SeekTo(match direction {
        SeekDirection::Forward => position.saturating_add(distance),
        SeekDirection::Backward => position.saturating_sub(distance),
    })
}
