//! Winamp skin renderer and Cranamp player surface.
//!
//! UI is intentionally split into per-control composables instead of a
//! monolithic draw pass so interactions and sprite mapping stay explicit.

#![allow(non_snake_case)]

mod skin;
mod sprites;

use std::rc::Rc;
use std::sync::OnceLock;

use cranpose::{
    rememberWindowState, WindowAttachPolicy, WindowConfig, WindowGroup, WindowId,
    WindowModifierExt, WindowMoveMode, WindowNode, WindowResizeDirection, WindowState,
};
use cranpose_core::{self, MutableState};
use cranpose_foundation::PointerButton;
use cranpose_ui::{
    composable, current_density, Box, BoxSpec, Button, Canvas, Color, Column, ColumnSpec, Modifier,
    Point, PointerEventKind, PointerInputScope, Size, Text, TextStyle,
};
use cranpose_ui_graphics::{Brush, ImageBitmap, Rect};

use crate::audio::{self, Track};
use skin::{load_skin, WinampSkin};
use sprites::*;

fn winamp_press_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("WINAMP_PRESS_DEBUG").is_some())
}

fn winamp_native_trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("CRANPOSE_NATIVE_TRACE").is_some())
}

fn trace_winamp_state(action: &str, state: &WinampState) {
    if winamp_native_trace_enabled() {
        println!(
            "winamp trace: action={action} closed={} playback={:?} eq_visible={} playlist_visible={} volume={:.3} current={:?} playlist_len={} status={:?}",
            state.closed,
            state.playback,
            state.eq_visible,
            state.playlist_visible,
            state.volume,
            state.current_index,
            state.playlist.len(),
            state.status
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn trace_tracks(action: &str, tracks: &[Track]) {
    if winamp_native_trace_enabled() {
        let paths = tracks
            .iter()
            .map(|track| track.path.as_deref().unwrap_or("<memory>"))
            .collect::<Vec<_>>();
        println!(
            "winamp trace: action={action} picked_count={} picked_paths={paths:?}",
            tracks.len()
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq)]
struct WinampState {
    closed: bool,
    playback: PlaybackState,
    shuffle: bool,
    repeat: bool,
    eq_visible: bool,
    playlist_visible: bool,
    eq_enabled: bool,
    eq_auto: bool,
    eq_values: [f32; 11],
    playlist_scroll: f32,
    volume: f32,
    balance: f32,
    position: f32,
    elapsed_seconds: f32,
    duration_seconds: Option<f32>,
    visualizer_phase: f32,
    status: String,
    playlist: Vec<Track>,
    current_index: Option<usize>,
}

impl Default for WinampState {
    fn default() -> Self {
        Self {
            closed: false,
            playback: PlaybackState::Stopped,
            shuffle: false,
            repeat: false,
            eq_visible: true,
            playlist_visible: true,
            eq_enabled: true,
            eq_auto: false,
            eq_values: [0.5; 11],
            playlist_scroll: 0.0,
            volume: 0.72,
            balance: 0.5,
            position: 0.0,
            elapsed_seconds: 0.0,
            duration_seconds: None,
            visualizer_phase: 0.0,
            status: "Stopped".to_string(),
            playlist: Vec::new(),
            current_index: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum WinampDragTarget {
    Inline(MutableState<Point>),
    NativeGroup,
}

#[derive(Clone, Copy, PartialEq)]
enum WinampCloseAction {
    SetStatus,
    CloseApp,
}

#[derive(Clone, Copy, PartialEq)]
enum WinampWindowSize {
    Fixed(Size),
    State(WindowState),
}

const MAIN_TITLE_DRAG_HIT_AREA: SpriteRect = (16.0, 0.0, 228.0, 14.0);
const EQ_TITLE_DRAG_HIT_AREA: SpriteRect = (0.0, 0.0, 264.0, 14.0);
const CRANAMP_WINAMP_MAIN_TITLE: &str = "Cranamp Winamp";
const CRANAMP_WINAMP_EQUALIZER_TITLE: &str = "Cranamp Winamp Equalizer";
const CRANAMP_WINAMP_PLAYLIST_TITLE: &str = "Cranamp Winamp Playlist";
const WINAMP_DEFAULT_SCREEN_POSITION: Point = Point { x: 140.0, y: 120.0 };

impl WinampWindowSize {
    fn get(self) -> Size {
        match self {
            Self::Fixed(size) => size,
            Self::State(state) => state.size(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct WinampTabState {
    player: MutableState<WinampState>,
    detached: MutableState<bool>,
    inline_windows: WinampInlineWindowStates,
    peer_windows: WinampPeerWindowStates,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct WinampInlineWindowStates {
    main: MutableState<Point>,
    equalizer: MutableState<Point>,
    playlist: MutableState<Point>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct WinampPeerWindowStates {
    main: WindowState,
    equalizer: WindowState,
    playlist: WindowState,
}

#[derive(Clone, Copy)]
struct WinampWindowPlacement {
    title: &'static str,
    initial_position: WinampInitialWindowPosition,
    state: WindowState,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum WinampInitialWindowPosition {
    Host(Point),
    Screen(Point),
}

#[composable]
pub(crate) fn remember_winamp_tab_state() -> WinampTabState {
    let peer_windows = WinampPeerWindowStates {
        main: rememberWindowState(MAIN_WIDTH, MAIN_HEIGHT),
        equalizer: rememberWindowState(EQ_WIDTH, EQ_HEIGHT),
        playlist: rememberWindowState(PLAYLIST_WIDTH, PLAYLIST_HEIGHT),
    };
    remember_saved_window_config(peer_windows);

    WinampTabState {
        player: cranpose_core::useState(WinampState::default),
        detached: cranpose_core::useState(native_winamp_windows_available),
        inline_windows: WinampInlineWindowStates {
            main: cranpose_core::useState(|| Point::new(26.0, 22.0)),
            equalizer: cranpose_core::useState(|| Point::new(26.0, 142.0)),
            playlist: cranpose_core::useState(|| Point::new(26.0, 262.0)),
        },
        peer_windows,
    }
}

#[composable]
pub(crate) fn WinampTab(tab_state: WinampTabState) {
    let scale = ui_scale();
    let state = tab_state.player;
    let native_available = native_winamp_windows_available();
    let detached = native_available && tab_state.detached.get();
    let snapshot = state.get();
    WinampRuntimeEffects(state, tab_state.peer_windows);
    let skin = match remember_winamp_skin() {
        Ok(skin) => skin,
        Err(error) => {
            WinampSkinError(error);
            return;
        }
    };

    Column(
        Modifier::empty()
            .fill_max_size()
            .padding(10.0)
            .background(Color(0.05, 0.06, 0.08, 1.0))
            .rounded_corners(12.0),
        ColumnSpec::default(),
        move || {
            Text(
                format!(
                    "{} | pos {:>3.0}% vol {:>3.0}% bal {:>3.0}%",
                    snapshot.status,
                    snapshot.position * 100.0,
                    snapshot.volume * 100.0,
                    snapshot.balance * 100.0,
                ),
                Modifier::empty().padding(8.0),
                TextStyle::default(),
            );

            if native_available {
                DockToggleButton(tab_state.detached, detached);
            }

            if !detached {
                WinampInlineStage(skin.clone(), state, tab_state.inline_windows, scale);
            } else {
                WinampNativeWindows(
                    skin.clone(),
                    state,
                    tab_state.inline_windows,
                    tab_state.peer_windows,
                    scale,
                    snapshot.clone(),
                );
            }
        },
    );
}

fn remember_winamp_skin() -> Result<WinampSkin, String> {
    cranpose_core::remember(|| {
        let wsz = include_bytes!("../../assets/winamp.wsz");
        load_skin(wsz).map_err(|err| format!("{err:#}"))
    })
    .with(|result| result.clone())
}

#[composable]
fn WinampRuntimeEffects(state: MutableState<WinampState>, peer_windows: WinampPeerWindowStates) {
    PlaybackProgressEffect(state);
    NativeWindowPersistence(peer_windows);
}

#[composable]
fn PlaybackProgressEffect(state: MutableState<WinampState>) {
    cranpose_core::LaunchedEffectAsync!((), move |scope| {
        Box::pin(async move {
            let clock = scope.runtime().frame_clock();
            loop {
                if !scope.is_active() {
                    break;
                }
                let _ = clock.next_frame().await;
                if !scope.is_active() {
                    break;
                }
                sync_playback_progress(state);
            }
        })
    });
}

fn sync_playback_progress(state: MutableState<WinampState>) {
    let snapshot = state.get_non_reactive();
    if snapshot.playback == PlaybackState::Stopped {
        return;
    }

    match audio::playback_progress() {
        Ok(Some(progress)) => {
            state.update(|s| {
                let duration = progress.duration_seconds.or(s.duration_seconds);
                let elapsed =
                    normalized_elapsed_seconds(progress.elapsed_seconds, duration, s.repeat);
                s.elapsed_seconds = elapsed;
                s.duration_seconds = duration;
                s.position = progress_fraction(elapsed, duration);
                if s.playback == PlaybackState::Playing {
                    s.visualizer_phase = (s.visualizer_phase + 1.0) % 10_000.0;
                }
                if progress.finished && !s.repeat {
                    s.playback = PlaybackState::Stopped;
                    s.status = "Stopped".to_string();
                }
            });
        }
        Ok(None) => {}
        Err(error) => state.update(|s| s.status = error),
    }
}

fn normalized_elapsed_seconds(elapsed: f32, duration: Option<f32>, repeat: bool) -> f32 {
    let elapsed = elapsed.max(0.0);
    let Some(duration) = duration.filter(|duration| *duration > 0.0) else {
        return elapsed;
    };
    if repeat {
        elapsed % duration
    } else {
        elapsed.min(duration)
    }
}

fn progress_fraction(elapsed: f32, duration: Option<f32>) -> f32 {
    duration
        .filter(|duration| *duration > 0.0)
        .map(|duration| (elapsed / duration).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

#[composable]
pub fn WinampFullscreenApp() {
    WinampSurfaceApp();
}

#[composable]
pub fn WinampWidgetApp() {
    WinampSurfaceApp();
}

#[composable]
fn WinampSurfaceApp() {
    let tab_state = remember_winamp_tab_state();
    WinampRuntimeEffects(tab_state.player, tab_state.peer_windows);
    let skin = match remember_winamp_skin() {
        Ok(skin) => skin,
        Err(error) => {
            WinampSkinError(error);
            return;
        }
    };

    Box(
        Modifier::empty()
            .fill_max_size()
            .clip_to_bounds()
            .background(Color(0.02, 0.02, 0.03, 1.0)),
        BoxSpec::default(),
        move || {
            WinampInlineStage(
                skin.clone(),
                tab_state.player,
                tab_state.inline_windows,
                ui_scale(),
            );
        },
    );
}

#[composable]
fn WinampSkinError(error: String) {
    Column(
        Modifier::empty().padding(16.0),
        ColumnSpec::default(),
        move || {
            Text(
                "Failed to load Winamp skin",
                Modifier::empty(),
                TextStyle::default(),
            );
            Text(error.clone(), Modifier::empty(), TextStyle::default());
        },
    );
}

#[composable]
fn DockToggleButton(detached_state: MutableState<bool>, detached: bool) {
    Button(
        Modifier::empty()
            .padding(8.0)
            .background(Color(0.18, 0.34, 0.58, 1.0))
            .rounded_corners(8.0)
            .padding(8.0),
        move || {
            detached_state.set(!detached_state.get_non_reactive());
        },
        move || {
            Text(
                if detached { "Dock" } else { "Undock" },
                Modifier::empty(),
                TextStyle::default(),
            );
        },
    );
}

#[composable]
fn WinampInlineStage(
    skin: WinampSkin,
    state: MutableState<WinampState>,
    windows: WinampInlineWindowStates,
    scale: f32,
) {
    Box(
        Modifier::empty()
            .fill_max_size()
            .clip_to_bounds()
            .background(Color(0.02, 0.02, 0.03, 1.0))
            .rounded_corners(8.0),
        BoxSpec::default(),
        move || {
            MainWindow(
                skin.clone(),
                state,
                WinampDragTarget::Inline(windows.main),
                WinampCloseAction::SetStatus,
                scale,
            );

            if state.get().eq_visible {
                EqualizerWindow(
                    skin.clone(),
                    state,
                    WinampDragTarget::Inline(windows.equalizer),
                    scale,
                );
            }

            if state.get().playlist_visible {
                PlaylistWindow(
                    skin.pledit.clone(),
                    skin.text.clone(),
                    state,
                    WinampDragTarget::Inline(windows.playlist),
                    WinampWindowSize::Fixed(Size::new(PLAYLIST_WIDTH, PLAYLIST_HEIGHT)),
                    scale,
                );
            }
        },
    );
}

#[composable]
fn WinampNativeWindows(
    skin: WinampSkin,
    state: MutableState<WinampState>,
    inline_windows: WinampInlineWindowStates,
    peer_windows: WinampPeerWindowStates,
    scale: f32,
    snapshot: WinampState,
) {
    WindowGroup("cranamp-winamp", winamp_attach_policy(), move || {
        WindowNode(
            winamp_main_window_id(),
            winamp_window_config(WinampWindowPlacement {
                title: CRANAMP_WINAMP_MAIN_TITLE,
                initial_position: WinampInitialWindowPosition::Host(inline_windows.main.get()),
                state: peer_windows.main,
            }),
            {
                let skin = skin.clone();
                move || {
                    MainWindow(
                        skin.clone(),
                        state,
                        WinampDragTarget::NativeGroup,
                        WinampCloseAction::SetStatus,
                        scale,
                    );
                }
            },
        );

        if snapshot.eq_visible {
            WindowNode(
                winamp_equalizer_window_id(),
                winamp_window_config(WinampWindowPlacement {
                    title: CRANAMP_WINAMP_EQUALIZER_TITLE,
                    initial_position: WinampInitialWindowPosition::Host(
                        inline_windows.equalizer.get(),
                    ),
                    state: peer_windows.equalizer,
                }),
                {
                    let skin = skin.clone();
                    move || {
                        EqualizerWindow(skin.clone(), state, WinampDragTarget::NativeGroup, scale);
                    }
                },
            );
        }

        if snapshot.playlist_visible {
            WindowNode(
                winamp_playlist_window_id(),
                winamp_window_config(WinampWindowPlacement {
                    title: CRANAMP_WINAMP_PLAYLIST_TITLE,
                    initial_position: WinampInitialWindowPosition::Host(
                        inline_windows.playlist.get(),
                    ),
                    state: peer_windows.playlist,
                })
                .with_resizable(true)
                .with_min_size(
                    scaled(PLAYLIST_WIDTH, scale),
                    scaled(PLAYLIST_HEIGHT, scale),
                ),
                {
                    let pledit = skin.pledit.clone();
                    let text = skin.text.clone();
                    move || {
                        PlaylistWindow(
                            pledit.clone(),
                            text.clone(),
                            state,
                            WinampDragTarget::NativeGroup,
                            WinampWindowSize::State(peer_windows.playlist),
                            scale,
                        );
                    }
                },
            );
        }
    });
}

#[composable]
pub fn WinampStandaloneApp() {
    let state = cranpose_core::useState(WinampState::default);
    let peer_windows = WinampPeerWindowStates {
        main: rememberWindowState(MAIN_WIDTH, MAIN_HEIGHT),
        equalizer: rememberWindowState(EQ_WIDTH, EQ_HEIGHT),
        playlist: rememberWindowState(PLAYLIST_WIDTH, PLAYLIST_HEIGHT),
    };
    remember_saved_window_config(peer_windows);
    WinampRuntimeEffects(state, peer_windows);
    let snapshot = state.get();
    if snapshot.closed {
        return;
    }
    let skin = match remember_winamp_skin() {
        Ok(skin) => skin,
        Err(error) => {
            WinampSkinError(error);
            return;
        }
    };

    WindowGroup("cranamp-winamp", winamp_attach_policy(), move || {
        WindowNode(
            winamp_main_window_id(),
            winamp_window_config(WinampWindowPlacement {
                title: CRANAMP_WINAMP_MAIN_TITLE,
                initial_position: WinampInitialWindowPosition::Screen(default_main_position()),
                state: peer_windows.main,
            }),
            {
                let skin = skin.clone();
                move || {
                    MainWindow(
                        skin.clone(),
                        state,
                        WinampDragTarget::NativeGroup,
                        WinampCloseAction::CloseApp,
                        ui_scale(),
                    );
                }
            },
        );

        if snapshot.eq_visible {
            WindowNode(
                winamp_equalizer_window_id(),
                winamp_window_config(WinampWindowPlacement {
                    title: CRANAMP_WINAMP_EQUALIZER_TITLE,
                    initial_position: WinampInitialWindowPosition::Screen(
                        default_equalizer_position(),
                    ),
                    state: peer_windows.equalizer,
                }),
                {
                    let skin = skin.clone();
                    move || {
                        EqualizerWindow(
                            skin.clone(),
                            state,
                            WinampDragTarget::NativeGroup,
                            ui_scale(),
                        );
                    }
                },
            );
        }

        if snapshot.playlist_visible {
            WindowNode(
                winamp_playlist_window_id(),
                winamp_window_config(WinampWindowPlacement {
                    title: CRANAMP_WINAMP_PLAYLIST_TITLE,
                    initial_position: WinampInitialWindowPosition::Screen(
                        default_playlist_position(),
                    ),
                    state: peer_windows.playlist,
                })
                .with_resizable(true)
                .with_min_size(PLAYLIST_WIDTH, PLAYLIST_HEIGHT),
                {
                    let pledit = skin.pledit.clone();
                    let text = skin.text.clone();
                    move || {
                        PlaylistWindow(
                            pledit.clone(),
                            text.clone(),
                            state,
                            WinampDragTarget::NativeGroup,
                            WinampWindowSize::State(peer_windows.playlist),
                            ui_scale(),
                        );
                    }
                },
            );
        }
    });
}

#[composable]
fn MainWindow(
    skin: WinampSkin,
    state: MutableState<WinampState>,
    drag_target: WinampDragTarget,
    close_action: WinampCloseAction,
    scale: f32,
) {
    let snapshot = state.get();

    Box(
        winamp_window_modifier(MAIN_WIDTH, MAIN_HEIGHT, scale, drag_target),
        BoxSpec::default(),
        move || {
            Sprite(skin.main.clone(), MAIN_WINDOW, 0.0, 0.0, scale);
            Sprite(
                skin.titlebar.clone(),
                MAIN_TITLE_BAR_SELECTED,
                0.0,
                0.0,
                scale,
            );

            WindowDragHandle(drag_target, MAIN_TITLE_DRAG_HIT_AREA, scale);

            {
                let state_click = state;
                PressableSprite(
                    skin.titlebar.clone(),
                    MAIN_OPTIONS_BUTTON,
                    MAIN_OPTIONS_BUTTON_SELECTED,
                    POS_OPTIONS_BUTTON.0,
                    POS_OPTIONS_BUTTON.1,
                    scale,
                    move || {
                        open_audio_folder(state_click);
                    },
                );
            }
            {
                let state_click = state;
                PressableSprite(
                    skin.titlebar.clone(),
                    MAIN_MINIMIZE_BUTTON,
                    MAIN_MINIMIZE_BUTTON_SELECTED,
                    POS_MINIMIZE_BUTTON.0,
                    POS_MINIMIZE_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| s.status = "Minimize".to_string());
                    },
                );
            }
            {
                let state_click = state;
                PressableSprite(
                    skin.titlebar.clone(),
                    MAIN_SHADE_BUTTON,
                    MAIN_SHADE_BUTTON_SELECTED,
                    POS_SHADE_BUTTON.0,
                    POS_SHADE_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| s.status = "Shade".to_string());
                    },
                );
            }
            {
                let state_click = state;
                PressableSprite(
                    skin.titlebar.clone(),
                    MAIN_CLOSE_BUTTON,
                    MAIN_CLOSE_BUTTON_SELECTED,
                    POS_CLOSE_BUTTON.0,
                    POS_CLOSE_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| match close_action {
                            WinampCloseAction::SetStatus => {
                                s.status = "Close".to_string();
                                trace_winamp_state("main-close-status", s);
                            }
                            WinampCloseAction::CloseApp => {
                                s.closed = true;
                                s.status = "Closed".to_string();
                                trace_winamp_state("main-close-app", s);
                            }
                        });
                    },
                );
            }

            let status_sprite = match snapshot.playback {
                PlaybackState::Stopped => STATUS_STOPPED,
                PlaybackState::Playing => STATUS_PLAYING,
                PlaybackState::Paused => STATUS_PAUSED,
            };
            Sprite(
                skin.playpaus.clone(),
                status_sprite,
                POS_STATUS.0,
                POS_STATUS.1,
                scale,
            );

            Visualizer(
                snapshot.playback == PlaybackState::Playing,
                snapshot.visualizer_phase,
                scale,
            );

            let digits = time_digits(snapshot.elapsed_seconds);
            for (i, digit) in digits.iter().enumerate() {
                let pos = POS_TIME_DIGITS[i];
                Sprite(
                    skin.numbers.clone(),
                    digit_rect(*digit),
                    pos.0,
                    pos.1,
                    scale,
                );
            }

            BitmapWinampText(
                skin.text.clone(),
                ellipsize_bitmap(main_display_title(&snapshot), MAIN_TRACK_TEXT_WIDTH),
                POS_MAIN_TRACK_TEXT.0,
                POS_MAIN_TRACK_TEXT.1,
                scale,
                [82, 242, 122, 255],
            );
            BitmapWinampText(
                skin.text.clone(),
                ellipsize_bitmap(main_display_meta(&snapshot), MAIN_META_TEXT_WIDTH),
                POS_MAIN_META_TEXT.0,
                POS_MAIN_META_TEXT.1,
                scale,
                [82, 242, 122, 255],
            );

            Sprite(
                skin.monoster.clone(),
                MONO_OFF,
                POS_MONO.0,
                POS_MONO.1,
                scale,
            );
            Sprite(
                skin.monoster.clone(),
                STEREO_OFF,
                POS_STEREO.0,
                POS_STEREO.1,
                scale,
            );

            Sprite(
                skin.posbar.clone(),
                POSBAR_BG,
                POS_POSBAR.0,
                POS_POSBAR.1,
                scale,
            );
            let position_thumb_x = slider_thumb_x(snapshot.position, POSBAR_BG.2, POSBAR_THUMB.2);
            Sprite(
                skin.posbar.clone(),
                POSBAR_THUMB,
                POS_POSBAR.0 + position_thumb_x,
                POS_POSBAR.1,
                scale,
            );
            {
                let state_drag = state;
                DragSlider(
                    POS_POSBAR.0,
                    POS_POSBAR.1,
                    POSBAR_BG.2,
                    POSBAR_BG.3,
                    scale,
                    move |fraction| {
                        state_drag.update(|s| {
                            s.position = fraction;
                            if let Some(duration) = s.duration_seconds {
                                s.elapsed_seconds = duration * fraction.clamp(0.0, 1.0);
                            }
                        });
                        if let Err(error) = audio::seek_fraction(fraction) {
                            state_drag.update(|s| s.status = error);
                        }
                    },
                );
            }

            TransportButtons(skin.cbuttons.clone(), state, scale);

            let vol_frame = slider_frame(snapshot.volume, VOLUME_FRAMES);
            Sprite(
                skin.volume.clone(),
                (
                    0.0,
                    vol_frame as f32 * VOLUME_BG_STRIDE,
                    VOLUME_BG_WIDTH,
                    VOLUME_BG_HEIGHT,
                ),
                POS_VOLUME.0,
                POS_VOLUME.1,
                scale,
            );
            let volume_thumb_x = slider_thumb_x(snapshot.volume, VOLUME_BG_WIDTH, VOLUME_THUMB.2);
            Sprite(
                skin.volume.clone(),
                VOLUME_THUMB,
                POS_VOLUME.0 + volume_thumb_x,
                POS_VOLUME.1 + 1.0,
                scale,
            );
            {
                let state_drag = state;
                DragSlider(
                    POS_VOLUME.0,
                    POS_VOLUME.1,
                    VOLUME_BG_WIDTH,
                    VOLUME_BG_HEIGHT,
                    scale,
                    move |fraction| {
                        state_drag.update(|s| {
                            s.volume = fraction;
                            trace_winamp_state("volume", s);
                        });
                        if let Err(error) = audio::set_volume(fraction) {
                            state_drag.update(|s| s.status = error);
                        }
                    },
                );
            }

            let bal_frame = slider_frame(snapshot.balance, BALANCE_FRAMES);
            Sprite(
                skin.balance.clone(),
                (
                    BALANCE_BG_X,
                    bal_frame as f32 * BALANCE_BG_STRIDE,
                    BALANCE_BG_WIDTH,
                    BALANCE_BG_HEIGHT,
                ),
                POS_BALANCE.0,
                POS_BALANCE.1,
                scale,
            );
            let balance_thumb_x =
                slider_thumb_x(snapshot.balance, BALANCE_BG_WIDTH, BALANCE_THUMB.2);
            Sprite(
                skin.balance.clone(),
                BALANCE_THUMB,
                POS_BALANCE.0 + balance_thumb_x,
                POS_BALANCE.1 + 1.0,
                scale,
            );
            {
                let state_drag = state;
                DragSlider(
                    POS_BALANCE.0,
                    POS_BALANCE.1,
                    BALANCE_BG_WIDTH,
                    BALANCE_BG_HEIGHT,
                    scale,
                    move |fraction| {
                        state_drag.update(|s| s.balance = fraction);
                    },
                );
            }

            let shuffle_normal = if snapshot.shuffle {
                SHUFFLE_ON
            } else {
                SHUFFLE_OFF
            };
            let shuffle_pressed = if snapshot.shuffle {
                SHUFFLE_ON_ACTIVE
            } else {
                SHUFFLE_OFF_ACTIVE
            };
            {
                let state_click = state;
                PressableSprite(
                    skin.shufrep.clone(),
                    shuffle_normal,
                    shuffle_pressed,
                    POS_SHUFFLE.0,
                    POS_SHUFFLE.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.shuffle = !s.shuffle;
                            s.status = if s.shuffle {
                                "Shuffle On".to_string()
                            } else {
                                "Shuffle Off".to_string()
                            };
                        });
                    },
                );
            }

            let repeat_normal = if snapshot.repeat {
                REPEAT_ON
            } else {
                REPEAT_OFF
            };
            let repeat_pressed = if snapshot.repeat {
                REPEAT_ON_ACTIVE
            } else {
                REPEAT_OFF_ACTIVE
            };
            {
                let state_click = state;
                PressableSprite(
                    skin.shufrep.clone(),
                    repeat_normal,
                    repeat_pressed,
                    POS_REPEAT.0,
                    POS_REPEAT.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.repeat = !s.repeat;
                            s.status = if s.repeat {
                                "Repeat On".to_string()
                            } else {
                                "Repeat Off".to_string()
                            };
                        });
                    },
                );
            }

            let eq_normal = if snapshot.eq_visible {
                EQ_BUTTON_ON
            } else {
                EQ_BUTTON_OFF
            };
            let eq_pressed = if snapshot.eq_visible {
                EQ_BUTTON_ON_ACTIVE
            } else {
                EQ_BUTTON_OFF_ACTIVE
            };
            {
                let state_click = state;
                PressableSprite(
                    skin.shufrep.clone(),
                    eq_normal,
                    eq_pressed,
                    POS_EQ_BUTTON.0,
                    POS_EQ_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.eq_visible = !s.eq_visible;
                            s.status = if s.eq_visible {
                                "Equalizer Shown".to_string()
                            } else {
                                "Equalizer Hidden".to_string()
                            };
                            trace_winamp_state("main-eq-toggle", s);
                        });
                    },
                );
            }

            let pl_normal = if snapshot.playlist_visible {
                PL_BUTTON_ON
            } else {
                PL_BUTTON_OFF
            };
            let pl_pressed = if snapshot.playlist_visible {
                PL_BUTTON_ON_ACTIVE
            } else {
                PL_BUTTON_OFF_ACTIVE
            };
            {
                let state_click = state;
                PressableSprite(
                    skin.shufrep.clone(),
                    pl_normal,
                    pl_pressed,
                    POS_PL_BUTTON.0,
                    POS_PL_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.playlist_visible = !s.playlist_visible;
                            s.status = if s.playlist_visible {
                                "Playlist Shown".to_string()
                            } else {
                                "Playlist Hidden".to_string()
                            };
                            trace_winamp_state("main-playlist-toggle", s);
                        });
                    },
                );
            }
        },
    );
}

#[composable]
fn EqualizerWindow(
    skin: WinampSkin,
    state: MutableState<WinampState>,
    drag_target: WinampDragTarget,
    scale: f32,
) {
    let snapshot = state.get();

    Box(
        winamp_window_modifier(EQ_WIDTH, EQ_HEIGHT, scale, drag_target),
        BoxSpec::default(),
        move || {
            Sprite(skin.eqmain.clone(), EQ_WINDOW, 0.0, 0.0, scale);
            Sprite(skin.eqmain.clone(), EQ_TITLE_BAR_SELECTED, 0.0, 0.0, scale);
            Sprite(
                skin.eqmain.clone(),
                EQ_GRAPH_BG,
                POS_EQ_GRAPH_BG.0,
                POS_EQ_GRAPH_BG.1,
                scale,
            );
            Sprite(
                skin.eqmain.clone(),
                EQ_PREAMP_LINE,
                POS_EQ_PREAMP_LINE.0,
                POS_EQ_PREAMP_LINE.1,
                scale,
            );

            WindowDragHandle(drag_target, EQ_TITLE_DRAG_HIT_AREA, scale);

            {
                let state_click = state;
                PressableSprite(
                    skin.eqmain.clone(),
                    EQ_CLOSE_BUTTON,
                    EQ_CLOSE_BUTTON_SELECTED,
                    POS_EQ_CLOSE_BUTTON.0,
                    POS_EQ_CLOSE_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.eq_visible = false;
                            s.status = "Equalizer Hidden".to_string();
                            trace_winamp_state("eq-close", s);
                        });
                    },
                );
            }

            let eq_on_normal = if snapshot.eq_enabled {
                EQ_ON_BUTTON_ON
            } else {
                EQ_ON_BUTTON_OFF
            };
            let eq_on_pressed = if snapshot.eq_enabled {
                EQ_ON_BUTTON_ON_SELECTED
            } else {
                EQ_ON_BUTTON_OFF_SELECTED
            };
            {
                let state_click = state;
                PressableSprite(
                    skin.eqmain.clone(),
                    eq_on_normal,
                    eq_on_pressed,
                    POS_EQ_ON_BUTTON.0,
                    POS_EQ_ON_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.eq_enabled = !s.eq_enabled;
                            s.status = if s.eq_enabled {
                                "EQ On".to_string()
                            } else {
                                "EQ Off".to_string()
                            };
                        });
                    },
                );
            }

            let eq_auto_normal = if snapshot.eq_auto {
                EQ_AUTO_BUTTON_ON
            } else {
                EQ_AUTO_BUTTON_OFF
            };
            let eq_auto_pressed = if snapshot.eq_auto {
                EQ_AUTO_BUTTON_ON_SELECTED
            } else {
                EQ_AUTO_BUTTON_OFF_SELECTED
            };
            {
                let state_click = state;
                PressableSprite(
                    skin.eqmain.clone(),
                    eq_auto_normal,
                    eq_auto_pressed,
                    POS_EQ_AUTO_BUTTON.0,
                    POS_EQ_AUTO_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.eq_auto = !s.eq_auto;
                            s.status = if s.eq_auto {
                                "EQ Auto On".to_string()
                            } else {
                                "EQ Auto Off".to_string()
                            };
                        });
                    },
                );
            }

            {
                let state_click = state;
                PressableSprite(
                    skin.eqmain.clone(),
                    EQ_PRESETS_BUTTON,
                    EQ_PRESETS_BUTTON_SELECTED,
                    POS_EQ_PRESETS_BUTTON.0,
                    POS_EQ_PRESETS_BUTTON.1,
                    scale,
                    move || {
                        state_click.update(|s| {
                            s.eq_values = [0.5; 11];
                            s.status = "EQ Reset".to_string();
                        });
                    },
                );
            }

            for (index, slider_x) in EQ_SLIDER_XS.iter().copied().enumerate() {
                let thumb_x = EQ_THUMB_XS[index];
                let value = snapshot.eq_values[index];
                let thumb_y = EQ_SLIDER_BG_Y
                    + vertical_slider_thumb_y(value, EQ_SLIDER_TRACK_HEIGHT, EQ_SLIDER_THUMB.3);

                Sprite(
                    skin.eqmain.clone(),
                    EQ_SLIDER_BG,
                    slider_x,
                    EQ_SLIDER_BG_Y,
                    scale,
                );
                Sprite(
                    skin.eqmain.clone(),
                    EQ_SLIDER_THUMB,
                    thumb_x,
                    thumb_y + EQ_SLIDER_THUMB_Y_OFFSET,
                    scale,
                );

                let state_drag = state;
                VerticalDragSlider(
                    slider_x,
                    EQ_SLIDER_BG_Y,
                    EQ_SLIDER_BG.2,
                    EQ_SLIDER_TRACK_HEIGHT,
                    scale,
                    true,
                    move |fraction| {
                        state_drag.update(|s| {
                            s.eq_values[index] = fraction;
                        });
                    },
                );
            }
        },
    );
}

#[composable]
fn PlaylistWindow(
    pledit: ImageBitmap,
    text: ImageBitmap,
    state: MutableState<WinampState>,
    drag_target: WinampDragTarget,
    window_size: WinampWindowSize,
    scale: f32,
) {
    let snapshot = state.get();
    let add_menu_open = cranpose_core::useState(|| false);
    let window_size = window_size.get();
    let skin_scale = scale.max(f32::EPSILON);
    let width = (window_size.width / skin_scale).max(PLAYLIST_WIDTH);
    let height = (window_size.height / skin_scale).max(PLAYLIST_HEIGHT);
    let right_x = width - PLAYLIST_RIGHT_TILE.2;
    let bottom_y = height - PLAYLIST_BOTTOM_LEFT_CORNER.3;
    let list_width = (right_x - PLAYLIST_LIST_BG.0).max(1.0);
    let list_height = (bottom_y - PLAYLIST_LIST_BG.1).max(1.0);
    let title_min_x = PLAYLIST_TOP_LEFT_CORNER.2;
    let title_max_x = (width - PLAYLIST_TOP_RIGHT_CORNER.2 - PLAYLIST_TITLE_BAR.2).max(title_min_x);
    let title_x = ((width - PLAYLIST_TITLE_BAR.2) * 0.5).clamp(title_min_x, title_max_x);
    let scroll_track_x = width - 15.0;

    Box(
        winamp_window_modifier(width, height, scale, drag_target),
        BoxSpec::default(),
        move || {
            Box(
                Modifier::empty()
                    .size_points(scaled(list_width, scale), scaled(list_height, scale))
                    .absolute_offset(
                        scaled(PLAYLIST_LIST_BG.0, scale),
                        scaled(PLAYLIST_LIST_BG.1, scale),
                    )
                    .background(Color(0.0, 0.0, 0.0, 1.0)),
                BoxSpec::default(),
                || {},
            );

            Sprite(pledit.clone(), PLAYLIST_TOP_LEFT_CORNER, 0.0, 0.0, scale);
            StretchSprite(
                pledit.clone(),
                PLAYLIST_TOP_TILE,
                PLAYLIST_TOP_LEFT_CORNER.2,
                0.0,
                width - PLAYLIST_TOP_LEFT_CORNER.2 - PLAYLIST_TOP_RIGHT_CORNER.2,
                PLAYLIST_TOP_TILE.3,
                scale,
            );
            Sprite(pledit.clone(), PLAYLIST_TITLE_BAR, title_x, 0.0, scale);
            Sprite(
                pledit.clone(),
                PLAYLIST_TOP_RIGHT_CORNER,
                width - PLAYLIST_TOP_RIGHT_CORNER.2,
                0.0,
                scale,
            );

            StretchSprite(
                pledit.clone(),
                PLAYLIST_LEFT_TILE,
                0.0,
                PLAYLIST_TOP_LEFT_CORNER.3,
                PLAYLIST_LEFT_TILE.2,
                bottom_y - PLAYLIST_TOP_LEFT_CORNER.3,
                scale,
            );
            StretchSprite(
                pledit.clone(),
                PLAYLIST_RIGHT_TILE,
                right_x,
                PLAYLIST_TOP_RIGHT_CORNER.3,
                PLAYLIST_RIGHT_TILE.2,
                bottom_y - PLAYLIST_TOP_RIGHT_CORNER.3,
                scale,
            );

            StretchSprite(
                pledit.clone(),
                PLAYLIST_BOTTOM_LEFT_CORNER,
                0.0,
                bottom_y,
                width - PLAYLIST_BOTTOM_RIGHT_CORNER.2,
                PLAYLIST_BOTTOM_LEFT_CORNER.3,
                scale,
            );
            Sprite(
                pledit.clone(),
                PLAYLIST_BOTTOM_RIGHT_CORNER,
                width - PLAYLIST_BOTTOM_RIGHT_CORNER.2,
                bottom_y,
                scale,
            );
            let scroll_y = PLAYLIST_LIST_BG.1
                + vertical_slider_thumb_y_down(
                    snapshot.playlist_scroll,
                    list_height,
                    PLAYLIST_SCROLL_HANDLE.3,
                );
            Sprite(
                pledit.clone(),
                PLAYLIST_SCROLL_HANDLE,
                scroll_track_x,
                scroll_y,
                scale,
            );

            PlaylistEntries(
                text.clone(),
                state,
                snapshot.clone(),
                list_width,
                list_height,
                scale,
            );

            {
                let state_drag = state;
                VerticalDragSlider(
                    scroll_track_x,
                    PLAYLIST_LIST_BG.1,
                    PLAYLIST_SCROLL_TRACK.2,
                    list_height,
                    scale,
                    false,
                    move |fraction| {
                        state_drag.update(|s| s.playlist_scroll = fraction);
                    },
                );
            }

            {
                let menu_state = add_menu_open;
                ClickTarget(
                    PLAYLIST_ADD_BUTTON_HIT_AREA.0,
                    bottom_y + PLAYLIST_ADD_BUTTON_HIT_AREA.1,
                    PLAYLIST_ADD_BUTTON_HIT_AREA.2,
                    PLAYLIST_ADD_BUTTON_HIT_AREA.3,
                    scale,
                    move || {
                        menu_state.update(|open| *open = !*open);
                    },
                );
            }

            if add_menu_open.get() {
                AddMenu(
                    text.clone(),
                    state,
                    add_menu_open,
                    PLAYLIST_ADD_BUTTON_HIT_AREA.0,
                    bottom_y - 31.0,
                    scale,
                );
            }

            WindowDragHandle(drag_target, (0.0, 0.0, width, PLAYLIST_DRAG_AREA.3), scale);
            WindowResizeHandle(
                drag_target,
                WindowResizeDirection::SouthEast,
                width - 16.0,
                height - 16.0,
                16.0,
                16.0,
                scale,
            );
        },
    );
}

#[composable]
fn PlaylistEntries(
    text_bitmap: ImageBitmap,
    state: MutableState<WinampState>,
    snapshot: WinampState,
    list_width: f32,
    list_height: f32,
    scale: f32,
) {
    let row_height = 12.0;
    let max_rows = ((list_height - 8.0) / row_height).floor().max(1.0) as usize;
    let x = PLAYLIST_LIST_BG.0 + 4.0;
    let y = PLAYLIST_LIST_BG.1 + 4.0;
    let text_width = (list_width - 12.0).max(1.0);

    if snapshot.playlist.is_empty() {
        BitmapWinampText(
            text_bitmap,
            snapshot.status.clone(),
            x,
            y + 2.0,
            scale,
            [82, 242, 122, 255],
        );
        return;
    }

    let max_start = snapshot.playlist.len().saturating_sub(max_rows);
    let start = ((snapshot.playlist_scroll * max_start as f32).round() as usize).min(max_start);
    for (row, track) in snapshot
        .playlist
        .iter()
        .enumerate()
        .skip(start)
        .take(max_rows)
    {
        let active = snapshot.current_index == Some(row);
        let title = ellipsize(
            format!("{:02}. {}", row + 1, track.display_title()),
            text_width,
        );
        BitmapWinampText(
            text_bitmap.clone(),
            title,
            x,
            y + ((row - start) as f32 * row_height) + 2.0,
            scale,
            if active {
                [242, 255, 148, 255]
            } else {
                [82, 242, 122, 255]
            },
        );

        {
            let state_click = state;
            ClickTarget(
                PLAYLIST_LIST_BG.0,
                PLAYLIST_LIST_BG.1 + ((row - start) as f32 * row_height),
                list_width,
                row_height,
                scale,
                move || {
                    activate_playlist_track(state_click, row);
                },
            );
        }
    }
}

#[composable]
fn AddMenu(
    text_bitmap: ImageBitmap,
    state: MutableState<WinampState>,
    menu_open: MutableState<bool>,
    x: f32,
    y: f32,
    scale: f32,
) {
    let width = 72.0;
    let row_height = 14.0;
    let height = row_height * 2.0;

    FilledRect(x, y, width, height, scale, Color(0.01, 0.015, 0.012, 1.0));
    FilledRect(x, y, width, 1.0, scale, Color(0.30, 0.42, 0.50, 1.0));
    FilledRect(
        x,
        y + row_height,
        width,
        1.0,
        scale,
        Color(0.12, 0.20, 0.24, 1.0),
    );

    BitmapWinampText(
        text_bitmap.clone(),
        "ADD FILE".to_string(),
        x + 5.0,
        y + 4.0,
        scale,
        [82, 242, 122, 255],
    );
    BitmapWinampText(
        text_bitmap,
        "ADD FOLDER".to_string(),
        x + 5.0,
        y + row_height + 4.0,
        scale,
        [82, 242, 122, 255],
    );

    {
        let state_click = state;
        let menu_state = menu_open;
        ClickTarget(x, y, width, row_height, scale, move || {
            menu_state.set(false);
            add_audio_files(state_click);
        });
    }
    {
        let state_click = state;
        let menu_state = menu_open;
        ClickTarget(x, y + row_height, width, row_height, scale, move || {
            menu_state.set(false);
            add_audio_folder(state_click);
        });
    }
}

#[composable]
fn FilledRect(x: f32, y: f32, width: f32, height: f32, scale: f32, color: Color) {
    let width = scaled(width, scale);
    let height = scaled(height, scale);
    Canvas(
        Modifier::empty()
            .size_points(width, height)
            .absolute_offset(scaled(x, scale), scaled(y, scale)),
        move |scope| {
            scope.draw_rect(Brush::solid(color));
        },
    );
}

#[composable]
fn BitmapWinampText(
    text_sheet: ImageBitmap,
    text: String,
    x: f32,
    y: f32,
    scale: f32,
    color: [u8; 4],
) {
    let bitmap = render_winamp_text(&text_sheet, &text, color);
    let width = bitmap.width() as f32;
    let height = bitmap.height() as f32;

    Canvas(
        Modifier::empty()
            .size_points(scaled(width, scale), scaled(height, scale))
            .absolute_offset(scaled(x, scale), scaled(y, scale)),
        move |scope| {
            let dst = Rect {
                x: 0.0,
                y: 0.0,
                width: scaled(width, scale),
                height: scaled(height, scale),
            };
            scope.draw_image_at(dst, bitmap.clone(), 1.0, None);
        },
    );
}

fn render_winamp_text(text_sheet: &ImageBitmap, text: &str, color: [u8; 4]) -> ImageBitmap {
    let glyph_width = 5usize;
    let glyph_height = 6usize;
    let output_width = (text.chars().count().max(1) * glyph_width) as u32;
    let output_height = glyph_height as u32;
    let mut pixels = vec![0u8; output_width as usize * output_height as usize * 4];

    for (char_index, ch) in text.chars().enumerate() {
        let Some((glyph_x, glyph_y)) = winamp_text_glyph(ch) else {
            continue;
        };
        for y in 0..glyph_height {
            for x in 0..glyph_width {
                let source_x = glyph_x + x;
                let source_y = glyph_y + y;
                let source_index = ((source_y * text_sheet.width() as usize) + source_x) * 4;
                let source = &text_sheet.pixels()[source_index..source_index + 4];
                if source[3] == 0 || (source[0] == 0 && source[1] == 0 && source[2] == 0) {
                    continue;
                }

                let target_x = char_index * glyph_width + x;
                let target_index = ((y * output_width as usize) + target_x) * 4;
                pixels[target_index..target_index + 4].copy_from_slice(&color);
            }
        }
    }

    ImageBitmap::from_rgba8(output_width, output_height, pixels)
        .expect("rendered Winamp text bitmap should be valid")
}

fn winamp_text_glyph(ch: char) -> Option<(usize, usize)> {
    let ch = ch.to_ascii_uppercase();
    if ch.is_ascii_uppercase() {
        return Some((((ch as u8 - b'A') as usize) * 5, 0));
    }
    if ch.is_ascii_digit() {
        return Some((((ch as u8 - b'0') as usize) * 5, 6));
    }

    let index = match ch {
        '.' => 10,
        ':' => 12,
        ')' => 13,
        '(' => 14,
        '-' => 15,
        '"' => 16,
        '!' => 17,
        '_' => 18,
        '+' => 19,
        '\\' => 20,
        '/' => 21,
        '[' => 22,
        ']' => 23,
        '^' => 24,
        '&' => 25,
        '%' => 26,
        ',' => 27,
        '=' => 28,
        '$' => 29,
        '#' => 30,
        _ => return None,
    };
    Some((index * 5, 6))
}

fn ellipsize(text: String, width: f32) -> String {
    let max_chars = (width / 6.0).floor().max(1.0) as usize;
    if text.chars().count() <= max_chars {
        return text;
    }

    let keep = max_chars.saturating_sub(1);
    let mut result = text.chars().take(keep).collect::<String>();
    result.push('~');
    result
}

fn ellipsize_bitmap(text: String, width: f32) -> String {
    let max_chars = (width / 5.0).floor().max(1.0) as usize;
    if text.chars().count() <= max_chars {
        return text;
    }

    let keep = max_chars.saturating_sub(1);
    let mut result = text.chars().take(keep).collect::<String>();
    result.push('~');
    result
}

fn main_display_title(state: &WinampState) -> String {
    state
        .current_index
        .and_then(|index| state.playlist.get(index))
        .map(|track| track.display_title().to_string())
        .unwrap_or_else(|| state.status.clone())
}

fn main_display_meta(state: &WinampState) -> String {
    let prefix = match state.playback {
        PlaybackState::Playing => "PLAY",
        PlaybackState::Paused => "PAUSE",
        PlaybackState::Stopped => "STOP",
    };
    let count = state.playlist.len();
    if count == 0 {
        return prefix.to_string();
    }

    let index = state.current_index.map(|index| index + 1).unwrap_or(1);
    format!("{prefix} {index:02}/{count:02}")
}

#[composable]
fn Visualizer(playing: bool, phase: f32, scale: f32) {
    let width = scaled(VISUALIZER_WIDTH, scale);
    let height = scaled(VISUALIZER_HEIGHT, scale);
    Canvas(
        Modifier::empty()
            .size_points(width, height)
            .absolute_offset(
                scaled(POS_VISUALIZER.0, scale),
                scaled(POS_VISUALIZER.1, scale),
            ),
        move |scope| {
            let bar_width = scaled(2.0, scale).max(1.0);
            let gap = scaled(2.0, scale).max(1.0);
            for bar in 0..VISUALIZER_BARS {
                let value = if playing {
                    visualizer_bar_height(phase, bar)
                } else {
                    1.0
                };
                let bar_height = scaled(value, scale).clamp(1.0, height);
                let x = bar as f32 * (bar_width + gap);
                let y = height - bar_height;
                let color = if value > 12.0 {
                    Color(1.0, 0.34, 0.16, 1.0)
                } else if value > 8.0 {
                    Color(0.95, 0.82, 0.22, 1.0)
                } else {
                    Color(0.22, 0.95, 0.38, 1.0)
                };
                scope.draw_rect_at(
                    Rect {
                        x,
                        y,
                        width: bar_width,
                        height: bar_height,
                    },
                    Brush::solid(color),
                );
            }
        },
    );
}

fn visualizer_bar_height(phase: f32, bar: usize) -> f32 {
    let wave = (phase * 0.17 + bar as f32 * 0.77).sin() * 0.5 + 0.5;
    let pulse = (phase * 0.07 + bar as f32 * 1.91).cos().abs();
    2.0 + ((wave * 0.72) + (pulse * 0.28)) * 14.0
}

#[composable]
fn Sprite(image: ImageBitmap, source: SpriteRect, x: f32, y: f32, scale: f32) {
    let w = scaled(source.2, scale);
    let h = scaled(source.3, scale);
    Canvas(
        Modifier::empty()
            .size_points(w, h)
            .absolute_offset(scaled(x, scale), scaled(y, scale)),
        move |scope| {
            let dst = Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            };
            scope.draw_image_src(image.clone(), to_rect(source), dst, 1.0, None);
        },
    );
}

#[composable]
fn StretchSprite(
    image: ImageBitmap,
    source: SpriteRect,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
) {
    let w = scaled(width.max(1.0), scale);
    let h = scaled(height.max(1.0), scale);
    Canvas(
        Modifier::empty()
            .size_points(w, h)
            .absolute_offset(scaled(x, scale), scaled(y, scale)),
        move |scope| {
            let dst = Rect {
                x: 0.0,
                y: 0.0,
                width: w,
                height: h,
            };
            scope.draw_image_src(image.clone(), to_rect(source), dst, 1.0, None);
        },
    );
}

#[composable]
fn PressableSprite(
    image: ImageBitmap,
    normal: SpriteRect,
    pressed: SpriteRect,
    x: f32,
    y: f32,
    scale: f32,
    on_click: impl Fn() + 'static,
) {
    let is_pressed = cranpose_core::useState(|| false);
    let on_click = Rc::new(on_click);

    let current = if is_pressed.get() { pressed } else { normal };
    if winamp_press_debug_enabled() {
        eprintln!(
            "[WINAMP_PRESS_DEBUG] compose button at ({:.1},{:.1}) pressed={} sprite=({:.1},{:.1},{:.1},{:.1})",
            x,
            y,
            is_pressed.get(),
            current.0,
            current.1,
            current.2,
            current.3
        );
    }
    let w = scaled(normal.2, scale);
    let h = scaled(normal.3, scale);

    Canvas(
        Modifier::empty()
            .size_points(w, h)
            .absolute_offset(scaled(x, scale), scaled(y, scale))
            .pointer_input((), {
                move |scope: PointerInputScope| {
                    let on_click = on_click.clone();
                    async move {
                        scope
                            .await_pointer_event_scope(|await_scope| async move {
                                loop {
                                    let event = await_scope.await_pointer_event().await;
                                    match event.kind {
                                        PointerEventKind::Down => {
                                            if winamp_press_debug_enabled() {
                                                eprintln!(
                                                    "[WINAMP_PRESS_DEBUG] down button ({:.1},{:.1}) local=({:.2},{:.2})",
                                                    x, y, event.position.x, event.position.y
                                                );
                                            }
                                            is_pressed.set(true);
                                            event.consume();
                                        }
                                        PointerEventKind::Move => {
                                            if is_pressed.get()
                                                && !event.buttons.contains(PointerButton::Primary)
                                            {
                                                if winamp_press_debug_enabled() {
                                                    eprintln!(
                                                        "[WINAMP_PRESS_DEBUG] move-clears button ({:.1},{:.1})",
                                                        x, y
                                                    );
                                                }
                                                is_pressed.set(false);
                                            }
                                        }
                                        PointerEventKind::Up => {
                                            let was_pressed = is_pressed.get();
                                            is_pressed.set(false);
                                            let inside = event.position.x >= 0.0
                                                && event.position.x <= w
                                                && event.position.y >= 0.0
                                                && event.position.y <= h;
                                            if winamp_press_debug_enabled() {
                                                eprintln!(
                                                    "[WINAMP_PRESS_DEBUG] up button ({:.1},{:.1}) was_pressed={} inside={} local=({:.2},{:.2})",
                                                    x, y, was_pressed, inside, event.position.x, event.position.y
                                                );
                                            }
                                            if was_pressed && inside {
                                                if winamp_press_debug_enabled() {
                                                    eprintln!(
                                                        "[WINAMP_PRESS_DEBUG] click fired button ({:.1},{:.1})",
                                                        x, y
                                                    );
                                                }
                                                on_click();
                                            }
                                            event.consume();
                                        }
                                        PointerEventKind::Cancel => {
                                            if winamp_press_debug_enabled() {
                                                eprintln!(
                                                    "[WINAMP_PRESS_DEBUG] cancel button ({:.1},{:.1})",
                                                    x, y
                                                );
                                            }
                                            is_pressed.set(false);
                                        }
                                        PointerEventKind::Scroll
                                        | PointerEventKind::Enter
                                        | PointerEventKind::Exit => {}
                                    }
                                }
                            })
                            .await;
                    }
                }
            }),
        move |scope| {
            let dst = Rect {
                x: 0.0,
                y: 0.0,
                width: scaled(current.2, scale),
                height: scaled(current.3, scale),
            };
            scope.draw_image_src(image.clone(), to_rect(current), dst, 1.0, None);
        },
    );
}

#[composable]
fn ClickTarget(x: f32, y: f32, width: f32, height: f32, scale: f32, on_click: impl Fn() + 'static) {
    let is_pressed = cranpose_core::useState(|| false);
    let on_click = Rc::new(on_click);
    let w = scaled(width, scale);
    let h = scaled(height, scale);

    Box(
        Modifier::empty()
            .size_points(w, h)
            .absolute_offset(scaled(x, scale), scaled(y, scale))
            .pointer_input((), {
                move |scope: PointerInputScope| {
                    let on_click = on_click.clone();
                    async move {
                        scope
                            .await_pointer_event_scope(|await_scope| async move {
                                loop {
                                    let event = await_scope.await_pointer_event().await;
                                    match event.kind {
                                        PointerEventKind::Down => {
                                            is_pressed.set(true);
                                            event.consume();
                                        }
                                        PointerEventKind::Move => {
                                            if is_pressed.get()
                                                && !event.buttons.contains(PointerButton::Primary)
                                            {
                                                is_pressed.set(false);
                                            }
                                        }
                                        PointerEventKind::Up => {
                                            let was_pressed = is_pressed.get();
                                            is_pressed.set(false);
                                            let inside = event.position.x >= 0.0
                                                && event.position.x <= w
                                                && event.position.y >= 0.0
                                                && event.position.y <= h;
                                            if was_pressed && inside {
                                                on_click();
                                            }
                                            event.consume();
                                        }
                                        PointerEventKind::Cancel => {
                                            is_pressed.set(false);
                                        }
                                        PointerEventKind::Scroll
                                        | PointerEventKind::Enter
                                        | PointerEventKind::Exit => {}
                                    }
                                }
                            })
                            .await;
                    }
                }
            }),
        BoxSpec::default(),
        || {},
    );
}

#[composable]
fn DragSlider(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
    on_change: impl Fn(f32) + 'static,
) {
    let on_change = Rc::new(on_change);

    Box(
        Modifier::empty()
            .size_points(scaled(width, scale), scaled(height, scale))
            .absolute_offset(scaled(x, scale), scaled(y, scale))
            .pointer_input((), {
                move |scope: PointerInputScope| {
                    let on_change = on_change.clone();
                    async move {
                        scope
                            .await_pointer_event_scope(|await_scope| async move {
                                let mut dragging = false;
                                loop {
                                    let event = await_scope.await_pointer_event().await;
                                    match event.kind {
                                        PointerEventKind::Down => {
                                            dragging = true;
                                            let value = (event.position.x / scaled(width, scale))
                                                .clamp(0.0, 1.0);
                                            on_change(value);
                                            event.consume();
                                        }
                                        PointerEventKind::Move if dragging => {
                                            let value = (event.position.x / scaled(width, scale))
                                                .clamp(0.0, 1.0);
                                            on_change(value);
                                            event.consume();
                                        }
                                        PointerEventKind::Up | PointerEventKind::Cancel => {
                                            dragging = false;
                                        }
                                        _ => {}
                                    }
                                }
                            })
                            .await;
                    }
                }
            }),
        BoxSpec::default(),
        || {},
    );
}

#[composable]
fn VerticalDragSlider(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
    invert: bool,
    on_change: impl Fn(f32) + 'static,
) {
    let on_change = Rc::new(on_change);

    Box(
        Modifier::empty()
            .size_points(scaled(width, scale), scaled(height, scale))
            .absolute_offset(scaled(x, scale), scaled(y, scale))
            .pointer_input((), {
                move |scope: PointerInputScope| {
                    let on_change = on_change.clone();
                    async move {
                        scope
                            .await_pointer_event_scope(|await_scope| async move {
                                let mut dragging = false;
                                loop {
                                    let event = await_scope.await_pointer_event().await;
                                    match event.kind {
                                        PointerEventKind::Down => {
                                            dragging = true;
                                            let raw = (event.position.y / scaled(height, scale))
                                                .clamp(0.0, 1.0);
                                            on_change(if invert { 1.0 - raw } else { raw });
                                            event.consume();
                                        }
                                        PointerEventKind::Move if dragging => {
                                            let raw = (event.position.y / scaled(height, scale))
                                                .clamp(0.0, 1.0);
                                            on_change(if invert { 1.0 - raw } else { raw });
                                            event.consume();
                                        }
                                        PointerEventKind::Up | PointerEventKind::Cancel => {
                                            dragging = false;
                                        }
                                        _ => {}
                                    }
                                }
                            })
                            .await;
                    }
                }
            }),
        BoxSpec::default(),
        || {},
    );
}

#[composable]
fn WindowDragHandle(drag_target: WinampDragTarget, area: SpriteRect, scale: f32) {
    let modifier = Modifier::empty()
        .size_points(scaled(area.2, scale), scaled(area.3, scale))
        .absolute_offset(scaled(area.0, scale), scaled(area.1, scale));

    match drag_target {
        WinampDragTarget::NativeGroup => {
            Box(modifier.window_drag_area(), BoxSpec::default(), || {});
        }
        WinampDragTarget::Inline(window_position) => {
            let drag_offset = cranpose_core::useState(|| None::<Point>);

            Box(
                modifier.pointer_input((), {
                    move |scope: PointerInputScope| async move {
                        scope
                            .await_pointer_event_scope(|await_scope| async move {
                                loop {
                                    let event = await_scope.await_pointer_event().await;
                                    match event.kind {
                                        PointerEventKind::Down => {
                                            let current = window_position.get();
                                            drag_offset.set(Some(Point::new(
                                                event.global_position.x - current.x,
                                                event.global_position.y - current.y,
                                            )));
                                            event.consume();
                                        }
                                        PointerEventKind::Move => {
                                            if !event.buttons.contains(PointerButton::Primary) {
                                                drag_offset.set(None);
                                                continue;
                                            }
                                            if let Some(offset) = drag_offset.get() {
                                                window_position.set(Point::new(
                                                    snap_to_pixel(
                                                        event.global_position.x - offset.x,
                                                    ),
                                                    snap_to_pixel(
                                                        event.global_position.y - offset.y,
                                                    ),
                                                ));
                                                event.consume();
                                            }
                                        }
                                        PointerEventKind::Up | PointerEventKind::Cancel => {
                                            drag_offset.set(None);
                                        }
                                        PointerEventKind::Scroll
                                        | PointerEventKind::Enter
                                        | PointerEventKind::Exit => {}
                                    }
                                }
                            })
                            .await;
                    }
                }),
                BoxSpec::default(),
                || {},
            );
        }
    }
}

#[composable]
fn WindowResizeHandle(
    drag_target: WinampDragTarget,
    direction: WindowResizeDirection,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    scale: f32,
) {
    if !matches!(drag_target, WinampDragTarget::NativeGroup) {
        return;
    }

    Box(
        Modifier::empty()
            .size_points(scaled(width, scale), scaled(height, scale))
            .absolute_offset(scaled(x, scale), scaled(y, scale))
            .window_resize_area(direction),
        BoxSpec::default(),
        || {},
    );
}

#[composable]
fn TransportButtons(cbuttons: ImageBitmap, state: MutableState<WinampState>, scale: f32) {
    {
        let state_click = state;
        PressableSprite(
            cbuttons.clone(),
            PREV_BUTTON,
            PREV_BUTTON_ACTIVE,
            POS_CBUTTONS.0,
            POS_CBUTTONS.1,
            scale,
            move || {
                previous_track(state_click);
            },
        );
    }

    {
        let state_click = state;
        PressableSprite(
            cbuttons.clone(),
            PLAY_BUTTON,
            PLAY_BUTTON_ACTIVE,
            POS_CBUTTONS.0 + 23.0,
            POS_CBUTTONS.1,
            scale,
            move || {
                play_or_resume(state_click);
            },
        );
    }

    {
        let state_click = state;
        PressableSprite(
            cbuttons.clone(),
            PAUSE_BUTTON,
            PAUSE_BUTTON_ACTIVE,
            POS_CBUTTONS.0 + 46.0,
            POS_CBUTTONS.1,
            scale,
            move || {
                pause_playback(state_click);
            },
        );
    }

    {
        let state_click = state;
        PressableSprite(
            cbuttons.clone(),
            STOP_BUTTON,
            STOP_BUTTON_ACTIVE,
            POS_CBUTTONS.0 + 69.0,
            POS_CBUTTONS.1,
            scale,
            move || {
                stop_playback(state_click);
            },
        );
    }

    {
        let state_click = state;
        PressableSprite(
            cbuttons.clone(),
            NEXT_BUTTON,
            NEXT_BUTTON_ACTIVE,
            POS_CBUTTONS.0 + 92.0,
            POS_CBUTTONS.1,
            scale,
            move || {
                next_track(state_click);
            },
        );
    }

    {
        let state_click = state;
        PressableSprite(
            cbuttons,
            EJECT_BUTTON,
            EJECT_BUTTON_ACTIVE,
            POS_EJECT.0,
            POS_EJECT.1,
            scale,
            move || {
                open_audio_files(state_click);
            },
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn open_audio_files(state: MutableState<WinampState>) {
    state.update(|s| s.status = "Opening File".to_string());
    match audio::pick_audio_files() {
        Ok(Some(tracks)) => {
            trace_tracks("open-files-picked", &tracks);
            replace_playlist_and_play(state, tracks);
        }
        Ok(None) => state.update(|s| s.status = "Open Cancelled".to_string()),
        Err(error) => state.update(|s| s.status = error),
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn open_audio_files(state: MutableState<WinampState>) {
    state.update(|s| s.status = "Opening File".to_string());
    wasm_bindgen_futures::spawn_local(async move {
        match audio::pick_web_audio_file().await {
            Ok(Some(picked)) => {
                let track = picked.track.clone();
                let snapshot = state.get_non_reactive();
                match audio::play_web_bytes(picked.bytes, snapshot.volume, snapshot.repeat) {
                    Ok(()) => {
                        state.update(|s| {
                            s.playlist = vec![track.clone()];
                            s.current_index = Some(0);
                            s.position = 0.0;
                            s.elapsed_seconds = 0.0;
                            s.duration_seconds = None;
                            s.playback = PlaybackState::Playing;
                            s.status = format!("Playing {}", track.display_title());
                            trace_winamp_state("web-open-file", s);
                        });
                    }
                    Err(error) => state.update(|s| s.status = error),
                }
            }
            Ok(None) => state.update(|s| s.status = "Open Cancelled".to_string()),
            Err(error) => state.update(|s| s.status = error),
        }
    });
}

#[cfg(all(not(feature = "web"), target_arch = "wasm32"))]
fn open_audio_files(state: MutableState<WinampState>) {
    state.update(|s| s.status = "Web picker is not enabled".to_string());
}

#[cfg(not(target_arch = "wasm32"))]
fn add_audio_files(state: MutableState<WinampState>) {
    state.update(|s| s.status = "Adding File".to_string());
    match audio::pick_audio_files() {
        Ok(Some(tracks)) => {
            trace_tracks("add-files-picked", &tracks);
            append_playlist_and_play(state, tracks);
        }
        Ok(None) => state.update(|s| s.status = "Add Cancelled".to_string()),
        Err(error) => state.update(|s| s.status = error),
    }
}

#[cfg(target_arch = "wasm32")]
fn add_audio_files(state: MutableState<WinampState>) {
    state.update(|s| {
        s.status = "Playlist add unavailable in the web widget".to_string();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn add_audio_folder(state: MutableState<WinampState>) {
    state.update(|s| s.status = "Adding Folder".to_string());
    match audio::pick_audio_folder() {
        Ok(Some(tracks)) => {
            trace_tracks("add-folder-picked", &tracks);
            append_playlist_and_play(state, tracks);
        }
        Ok(None) => state.update(|s| s.status = "Add Cancelled".to_string()),
        Err(error) => state.update(|s| s.status = error),
    }
}

#[cfg(target_arch = "wasm32")]
fn add_audio_folder(state: MutableState<WinampState>) {
    state.update(|s| {
        s.status = "Folder picker unavailable in the web widget".to_string();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn open_audio_folder(state: MutableState<WinampState>) {
    state.update(|s| s.status = "Opening Folder".to_string());
    match audio::pick_audio_folder() {
        Ok(Some(tracks)) => {
            trace_tracks("open-folder-picked", &tracks);
            replace_playlist_and_play(state, tracks);
        }
        Ok(None) => state.update(|s| s.status = "Open Cancelled".to_string()),
        Err(error) => state.update(|s| s.status = error),
    }
}

#[cfg(target_arch = "wasm32")]
fn open_audio_folder(state: MutableState<WinampState>) {
    state.update(|s| {
        s.status = "Folder picker unavailable in the web widget".to_string();
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn replace_playlist_and_play(state: MutableState<WinampState>, tracks: Vec<Track>) {
    if tracks.is_empty() {
        state.update(|s| {
            s.status = "No Supported Audio".to_string();
            trace_winamp_state("replace-empty", s);
        });
        return;
    }

    state.update(|s| {
        replace_playlist_tracks(s, tracks);
        trace_winamp_state("replace-playlist", s);
    });
    start_track(state, 0);
}

#[cfg(not(target_arch = "wasm32"))]
fn append_playlist_and_play(state: MutableState<WinampState>, tracks: Vec<Track>) {
    if tracks.is_empty() {
        state.update(|s| {
            s.status = "No Supported Audio".to_string();
            trace_winamp_state("append-empty", s);
        });
        return;
    }

    let should_start = state.get_non_reactive().playlist.is_empty();
    state.update(|s| {
        append_playlist_tracks(s, tracks);
        trace_winamp_state("append-playlist", s);
    });
    if should_start {
        start_track(state, 0);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn replace_playlist_tracks(state: &mut WinampState, tracks: Vec<Track>) {
    state.playlist = tracks;
    state.current_index = Some(0);
    state.playlist_scroll = 0.0;
    state.position = 0.0;
    state.elapsed_seconds = 0.0;
    state.duration_seconds = None;
    state.status = format!("Loaded {} Track(s)", state.playlist.len());
}

#[cfg(not(target_arch = "wasm32"))]
fn append_playlist_tracks(state: &mut WinampState, tracks: Vec<Track>) -> bool {
    let was_empty = state.playlist.is_empty();
    let added_count = tracks.len();

    state.playlist.extend(tracks);
    if was_empty {
        state.current_index = Some(0);
        state.playlist_scroll = 0.0;
        state.position = 0.0;
        state.elapsed_seconds = 0.0;
        state.duration_seconds = None;
        state.status = format!("Loaded {} Track(s)", state.playlist.len());
    } else {
        state.playlist_scroll = 1.0;
        state.status = format!("Added {added_count} Track(s)");
    }

    was_empty
}

fn play_or_resume(state: MutableState<WinampState>) {
    let snapshot = state.get_non_reactive();
    if snapshot.playback == PlaybackState::Paused {
        match audio::resume() {
            Ok(()) => {
                state.update(|s| {
                    s.playback = PlaybackState::Playing;
                    s.status = current_track_status(s, "Playing");
                    trace_winamp_state("resume", s);
                });
            }
            Err(error) => state.update(|s| s.status = error),
        }
        return;
    }

    let Some(index) = snapshot
        .current_index
        .or_else(|| (!snapshot.playlist.is_empty()).then_some(0))
    else {
        state.update(|s| s.status = "Open File".to_string());
        return;
    };

    #[cfg(target_arch = "wasm32")]
    if snapshot
        .playlist
        .get(index)
        .map(|track| track.path.is_none())
        .unwrap_or(false)
    {
        match audio::resume() {
            Ok(()) => {
                state.update(|s| {
                    s.playback = PlaybackState::Playing;
                    s.status = current_track_status(s, "Playing");
                    trace_winamp_state("web-resume", s);
                });
            }
            Err(error) => state.update(|s| s.status = error),
        }
        return;
    }

    start_track(state, index);
}

fn pause_playback(state: MutableState<WinampState>) {
    match audio::pause() {
        Ok(()) => {
            state.update(|s| {
                s.playback = PlaybackState::Paused;
                s.status = current_track_status(s, "Paused");
                trace_winamp_state("pause", s);
            });
        }
        Err(error) => state.update(|s| s.status = error),
    }
}

fn stop_playback(state: MutableState<WinampState>) {
    match audio::stop() {
        Ok(()) => {
            state.update(|s| {
                s.playback = PlaybackState::Stopped;
                s.position = 0.0;
                s.elapsed_seconds = 0.0;
                s.status = "Stopped".to_string();
                trace_winamp_state("stop", s);
            });
        }
        Err(error) => state.update(|s| s.status = error),
    }
}

fn next_track(state: MutableState<WinampState>) {
    let snapshot = state.get_non_reactive();
    if snapshot.playlist.is_empty() {
        state.update(|s| s.status = "Open File".to_string());
        return;
    }

    let current = snapshot.current_index.unwrap_or(0);
    let next = if current + 1 >= snapshot.playlist.len() {
        0
    } else {
        current + 1
    };
    start_track(state, next);
}

fn previous_track(state: MutableState<WinampState>) {
    let snapshot = state.get_non_reactive();
    if snapshot.playlist.is_empty() {
        state.update(|s| s.status = "Open File".to_string());
        return;
    }

    let current = snapshot.current_index.unwrap_or(0);
    let previous = if current == 0 {
        snapshot.playlist.len() - 1
    } else {
        current - 1
    };
    start_track(state, previous);
}

fn activate_playlist_track(state: MutableState<WinampState>, index: usize) {
    let snapshot = state.get_non_reactive();
    if index >= snapshot.playlist.len() {
        state.update(|s| s.status = "Track Missing".to_string());
        return;
    }

    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    if snapshot
        .playlist
        .get(index)
        .map(|track| track.path.is_none())
        .unwrap_or(false)
    {
        state.update(|s| {
            s.current_index = Some(index);
            s.status = current_track_status(s, "Selected");
        });
        play_or_resume(state);
        return;
    }

    start_track(state, index);
}

fn start_track(state: MutableState<WinampState>, index: usize) {
    let snapshot = state.get_non_reactive();
    let Some(track) = snapshot.playlist.get(index).cloned() else {
        state.update(|s| s.status = "Track Missing".to_string());
        return;
    };

    match audio::play_track(&track, snapshot.volume, snapshot.repeat) {
        Ok(()) => {
            state.update(|s| {
                s.current_index = Some(index);
                s.playback = PlaybackState::Playing;
                s.position = 0.0;
                s.elapsed_seconds = 0.0;
                s.duration_seconds = None;
                s.status = format!("Playing {}", track.display_title());
                trace_winamp_state("play", s);
            });
        }
        Err(error) => state.update(|s| {
            s.current_index = Some(index);
            s.playback = PlaybackState::Stopped;
            s.status = error;
            trace_winamp_state("play-error", s);
        }),
    }
}

fn current_track_status(state: &WinampState, prefix: &str) -> String {
    state
        .current_index
        .and_then(|index| state.playlist.get(index))
        .map(|track| format!("{prefix} {}", track.display_title()))
        .unwrap_or_else(|| prefix.to_string())
}

const WINAMP_NATIVE_HOST_OFFSET_X: f32 = 640.0;
const WINAMP_NATIVE_HOST_OFFSET_Y: f32 = 118.0;
const WINAMP_ATTACH_EPSILON: f32 = 3.0;
const WINAMP_SNAP_DISTANCE: f32 = 8.0;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SavedWindowConfig {
    position: Option<Point>,
    size: Option<Size>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SavedWinampWindowConfig {
    main: SavedWindowConfig,
    equalizer: SavedWindowConfig,
    playlist: SavedWindowConfig,
}

#[cfg(not(target_arch = "wasm32"))]
impl SavedWinampWindowConfig {
    fn from_states(peer_windows: WinampPeerWindowStates) -> Self {
        Self {
            main: SavedWindowConfig {
                position: peer_windows.main.position_non_reactive(),
                size: Some(peer_windows.main.size_non_reactive()),
            },
            equalizer: SavedWindowConfig {
                position: peer_windows.equalizer.position_non_reactive(),
                size: Some(peer_windows.equalizer.size_non_reactive()),
            },
            playlist: SavedWindowConfig {
                position: peer_windows.playlist.position_non_reactive(),
                size: Some(peer_windows.playlist.size_non_reactive()),
            },
        }
    }
}

#[composable]
fn remember_saved_window_config(peer_windows: WinampPeerWindowStates) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        cranpose_core::remember(move || {
            if let Some(config) = load_saved_window_config() {
                apply_saved_window_config(peer_windows, config);
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = peer_windows;
    }
}

#[composable]
fn NativeWindowPersistence(peer_windows: WinampPeerWindowStates) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let last_saved = cranpose_core::remember(|| None::<SavedWinampWindowConfig>);
        cranpose_core::SideEffect(move || {
            let config = SavedWinampWindowConfig::from_states(peer_windows);
            last_saved.update(|last| {
                if last.as_ref() != Some(&config) {
                    if let Err(error) = save_window_config(config) {
                        eprintln!("failed to save Cranamp window config: {error}");
                    }
                    *last = Some(config);
                }
            });
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let _ = peer_windows;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_saved_window_config(
    peer_windows: WinampPeerWindowStates,
    config: SavedWinampWindowConfig,
) {
    if let Some(position) = config.main.position {
        peer_windows.main.set_position(Some(position));
    }
    if let Some(position) = config.equalizer.position {
        peer_windows.equalizer.set_position(Some(position));
    }
    if let Some(position) = config.playlist.position {
        peer_windows.playlist.set_position(Some(position));
    }
    if let Some(size) = config.playlist.size {
        peer_windows
            .playlist
            .set_size(clamp_playlist_window_size(size));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn clamp_playlist_window_size(size: Size) -> Size {
    Size::new(
        size.width.max(PLAYLIST_WIDTH),
        size.height.max(PLAYLIST_HEIGHT),
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn serialize_window_config(config: SavedWinampWindowConfig) -> String {
    let mut lines = Vec::new();
    push_window_config_lines(&mut lines, "main", config.main);
    push_window_config_lines(&mut lines, "equalizer", config.equalizer);
    push_window_config_lines(&mut lines, "playlist", config.playlist);
    lines.join("\n") + "\n"
}

#[cfg(not(target_arch = "wasm32"))]
fn push_window_config_lines(lines: &mut Vec<String>, name: &str, config: SavedWindowConfig) {
    if let Some(position) = config.position {
        lines.push(format!("{name}.x={:.3}", position.x));
        lines.push(format!("{name}.y={:.3}", position.y));
    }
    if let Some(size) = config.size {
        lines.push(format!("{name}.width={:.3}", size.width));
        lines.push(format!("{name}.height={:.3}", size.height));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_window_config(input: &str) -> SavedWinampWindowConfig {
    let mut config = SavedWinampWindowConfig::default();
    for line in input.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Ok(value) = value.trim().parse::<f32>() else {
            continue;
        };
        apply_window_config_value(&mut config, key.trim(), value);
    }
    config
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_window_config_value(config: &mut SavedWinampWindowConfig, key: &str, value: f32) {
    let Some((window, field)) = key.split_once('.') else {
        return;
    };
    let Some(target) = saved_window_mut(config, window) else {
        return;
    };

    match field {
        "x" => target.position.get_or_insert(Point::ZERO).x = value,
        "y" => target.position.get_or_insert(Point::ZERO).y = value,
        "width" => target.size.get_or_insert(Size::ZERO).width = value,
        "height" => target.size.get_or_insert(Size::ZERO).height = value,
        _ => {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn saved_window_mut<'a>(
    config: &'a mut SavedWinampWindowConfig,
    window: &str,
) -> Option<&'a mut SavedWindowConfig> {
    match window {
        "main" => Some(&mut config.main),
        "equalizer" => Some(&mut config.equalizer),
        "playlist" => Some(&mut config.playlist),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_saved_window_config() -> Option<SavedWinampWindowConfig> {
    std::fs::read_to_string(window_config_path())
        .ok()
        .map(|text| parse_window_config(&text))
}

#[cfg(not(target_arch = "wasm32"))]
fn save_window_config(config: SavedWinampWindowConfig) -> Result<(), String> {
    let path = window_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    std::fs::write(&path, serialize_window_config(config))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

#[cfg(not(target_arch = "wasm32"))]
fn window_config_path() -> std::path::PathBuf {
    config_home_dir().join("cranamp").join("windows.conf")
}

#[cfg(not(target_arch = "wasm32"))]
fn config_home_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .map(|home| home.join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn default_main_position() -> Point {
    WINAMP_DEFAULT_SCREEN_POSITION
}

fn default_equalizer_position() -> Point {
    Point::new(
        WINAMP_DEFAULT_SCREEN_POSITION.x,
        WINAMP_DEFAULT_SCREEN_POSITION.y + MAIN_HEIGHT,
    )
}

fn default_playlist_position() -> Point {
    Point::new(
        WINAMP_DEFAULT_SCREEN_POSITION.x,
        WINAMP_DEFAULT_SCREEN_POSITION.y + MAIN_HEIGHT + EQ_HEIGHT,
    )
}

fn native_winamp_windows_available() -> bool {
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(target_os = "android"),
        not(target_os = "ios")
    ))]
    {
        std::env::var_os("CRANPOSE_WINAMP_INLINE").is_none()
    }

    #[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
    {
        false
    }
}

fn base_winamp_window_config(placement: WinampWindowPlacement) -> WindowConfig {
    let state_size = placement.state.size();
    let config = WindowConfig::borderless(placement.title, state_size.width, state_size.height);
    let config = match placement.initial_position {
        WinampInitialWindowPosition::Host(position) => config.with_host_window_position(
            snap_to_pixel(position.x + WINAMP_NATIVE_HOST_OFFSET_X),
            snap_to_pixel(position.y + WINAMP_NATIVE_HOST_OFFSET_Y),
        ),
        WinampInitialWindowPosition::Screen(position) => {
            config.with_position(snap_to_pixel(position.x), snap_to_pixel(position.y))
        }
    };
    config
        .with_transparent(false)
        .with_resizable(false)
        .with_visible(true)
}

fn winamp_window_config(placement: WinampWindowPlacement) -> WindowConfig {
    let state = placement.state;
    base_winamp_window_config(placement).with_state(state)
}

fn winamp_attach_policy() -> WindowAttachPolicy {
    WindowAttachPolicy::new(
        WINAMP_SNAP_DISTANCE,
        WINAMP_ATTACH_EPSILON,
        WindowMoveMode::DragLeaderOnly(vec![winamp_main_window_id()]),
    )
}

fn winamp_main_window_id() -> WindowId {
    WindowId::from_static("cranamp-winamp-main")
}

fn winamp_equalizer_window_id() -> WindowId {
    WindowId::from_static("cranamp-winamp-equalizer")
}

fn winamp_playlist_window_id() -> WindowId {
    WindowId::from_static("cranamp-winamp-playlist")
}

fn winamp_window_modifier(
    width: f32,
    height: f32,
    scale: f32,
    drag_target: WinampDragTarget,
) -> Modifier {
    let modifier = Modifier::empty().size_points(scaled(width, scale), scaled(height, scale));
    match drag_target {
        WinampDragTarget::Inline(position) => {
            let position = position.get();
            modifier.offset(snap_to_pixel(position.x), snap_to_pixel(position.y))
        }
        WinampDragTarget::NativeGroup => modifier,
    }
}

fn ui_scale() -> f32 {
    // Skin pixel coordinates map directly to dp.  On high-density screens the
    // renderer upscales automatically, keeping the skin at the same visual
    // size as on a 1× desktop display.
    1.0
}

fn snap_to_pixel(value: f32) -> f32 {
    let density = current_density();
    if density > 0.0 {
        (value * density).round() / density
    } else {
        value.round()
    }
}

fn scaled(value: f32, scale: f32) -> f32 {
    snap_to_pixel(value * scale)
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn slider_thumb_x(value: f32, bar_width: f32, knob_width: f32) -> f32 {
    clamp01(value) * (bar_width - knob_width)
}

fn slider_frame(value: f32, frames: u32) -> u32 {
    if frames <= 1 {
        return 0;
    }
    let max_index = frames - 1;
    (clamp01(value) * max_index as f32).round() as u32
}

fn vertical_slider_thumb_y(value: f32, track_height: f32, knob_height: f32) -> f32 {
    (1.0 - clamp01(value)) * (track_height - knob_height)
}

fn vertical_slider_thumb_y_down(value: f32, track_height: f32, knob_height: f32) -> f32 {
    clamp01(value) * (track_height - knob_height)
}

fn time_digits(elapsed_seconds: f32) -> [u8; 4] {
    let seconds = elapsed_seconds.max(0.0).round() as u32;
    let minutes = (seconds / 60) % 100;
    let remainder = seconds % 60;
    [
        ((minutes / 10) % 10) as u8,
        (minutes % 10) as u8,
        (remainder / 10) as u8,
        (remainder % 10) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_digits_are_mapped_correctly() {
        assert_eq!(time_digits(0.0), [0, 0, 0, 0]);
        assert_eq!(time_digits(65.0), [0, 1, 0, 5]);
        assert_eq!(time_digits(-1.0), [0, 0, 0, 0]);
    }

    #[test]
    fn slider_helpers_clamp_values() {
        assert_eq!(slider_frame(-1.0, 28), 0);
        assert_eq!(slider_frame(2.0, 28), 27);
        assert_eq!(slider_thumb_x(-1.0, 248.0, 29.0), 0.0);
        assert_eq!(slider_thumb_x(2.0, 248.0, 29.0), 219.0);
    }

    #[test]
    fn vertical_slider_helpers_clamp_values() {
        assert_eq!(vertical_slider_thumb_y(-1.0, 63.0, 11.0), 52.0);
        assert_eq!(vertical_slider_thumb_y(2.0, 63.0, 11.0), 0.0);
        assert_eq!(vertical_slider_thumb_y_down(-1.0, 145.0, 18.0), 0.0);
        assert_eq!(vertical_slider_thumb_y_down(2.0, 145.0, 18.0), 127.0);
    }

    #[test]
    fn progress_fraction_uses_duration_when_known() {
        assert_eq!(progress_fraction(30.0, Some(120.0)), 0.25);
        assert_eq!(progress_fraction(130.0, Some(120.0)), 1.0);
        assert_eq!(progress_fraction(30.0, None), 0.0);
    }

    #[test]
    fn repeated_elapsed_wraps_to_track_duration() {
        assert_eq!(normalized_elapsed_seconds(130.0, Some(120.0), true), 10.0);
        assert_eq!(normalized_elapsed_seconds(130.0, Some(120.0), false), 120.0);
    }

    #[test]
    fn saved_window_config_round_trips() {
        let config = SavedWinampWindowConfig {
            main: SavedWindowConfig {
                position: Some(Point::new(10.0, 20.0)),
                size: Some(Size::new(MAIN_WIDTH, MAIN_HEIGHT)),
            },
            equalizer: SavedWindowConfig {
                position: Some(Point::new(10.0, 136.0)),
                size: Some(Size::new(EQ_WIDTH, EQ_HEIGHT)),
            },
            playlist: SavedWindowConfig {
                position: Some(Point::new(10.0, 252.0)),
                size: Some(Size::new(320.0, 240.0)),
            },
        };

        assert_eq!(
            parse_window_config(&serialize_window_config(config)),
            config
        );
    }

    #[test]
    fn default_native_windows_stack_vertically() {
        assert_eq!(default_main_position(), Point::new(140.0, 120.0));
        assert_eq!(default_equalizer_position(), Point::new(140.0, 236.0));
        assert_eq!(default_playlist_position(), Point::new(140.0, 352.0));
    }

    #[test]
    fn append_playlist_tracks_starts_empty_playlist_at_first_added_track() {
        let mut state = WinampState::default();

        let should_start = append_playlist_tracks(&mut state, vec![test_track("First")]);

        assert!(should_start);
        assert_eq!(state.playlist, vec![test_track("First")]);
        assert_eq!(state.current_index, Some(0));
        assert_eq!(state.playlist_scroll, 0.0);
        assert_eq!(state.position, 0.0);
        assert_eq!(state.elapsed_seconds, 0.0);
        assert_eq!(state.status, "Loaded 1 Track(s)");
    }

    #[test]
    fn append_playlist_tracks_preserves_current_track_when_playlist_exists() {
        let mut state = WinampState {
            playback: PlaybackState::Playing,
            playlist: vec![test_track("First")],
            current_index: Some(0),
            playlist_scroll: 0.25,
            status: "Playing First".to_string(),
            ..WinampState::default()
        };

        let should_start = append_playlist_tracks(&mut state, vec![test_track("Second")]);

        assert!(!should_start);
        assert_eq!(
            state.playlist,
            vec![test_track("First"), test_track("Second")]
        );
        assert_eq!(state.current_index, Some(0));
        assert_eq!(state.playback, PlaybackState::Playing);
        assert_eq!(state.playlist_scroll, 1.0);
        assert_eq!(state.status, "Added 1 Track(s)");
    }

    fn test_track(title: &str) -> Track {
        Track {
            title: title.to_string(),
            path: Some(format!("/tmp/{title}.mp3")),
        }
    }
}
