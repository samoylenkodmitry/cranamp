//! Winamp's keyboard: the keys that drive the player from anywhere in it, as
//! Winamp 2 had them. A key held with Ctrl, Alt or the command key is left to
//! the platform, so the browser's and the system's own shortcuts keep working,
//! except Winamp's own Alt keys for its windows.

use cranpose_ui::{Key, KeyEvent, KeyEventType};

/// How far the arrow keys move the song, in seconds.
pub(super) const SEEK_STEP_SECONDS: f32 = 5.0;
/// How far the up and down arrows move the volume, out of one.
pub(super) const VOLUME_STEP: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WinampKey {
    Previous,
    Play,
    Pause,
    Stop,
    Next,
    SeekBack,
    SeekForward,
    VolumeUp,
    VolumeDown,
    OpenFiles,
}

/// The player action a key press stands for, if any.
pub(super) fn winamp_key(event: &KeyEvent) -> Option<WinampKey> {
    if event.type_() != KeyEventType::KeyDown
        || event.is_ctrl_pressed()
        || event.is_alt_pressed()
        || event.is_meta_pressed()
    {
        return None;
    }
    Some(match event.key() {
        Key::Z => WinampKey::Previous,
        Key::X => WinampKey::Play,
        Key::C => WinampKey::Pause,
        Key::V => WinampKey::Stop,
        Key::B => WinampKey::Next,
        Key::DirectionLeft => WinampKey::SeekBack,
        Key::DirectionRight => WinampKey::SeekForward,
        Key::DirectionUp => WinampKey::VolumeUp,
        Key::DirectionDown => WinampKey::VolumeDown,
        Key::L => WinampKey::OpenFiles,
        _ => return None,
    })
}

/// A window Winamp opens and closes from the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WinampWindowKey {
    Main,
    Playlist,
    Equalizer,
}

/// The window Alt with a key opens or closes, if any: W the main window, E
/// the playlist and G the equalizer.
pub(super) fn window_key(event: &KeyEvent) -> Option<WinampWindowKey> {
    if event.type_() != KeyEventType::KeyDown
        || !event.is_alt_pressed()
        || event.is_ctrl_pressed()
        || event.is_meta_pressed()
    {
        return None;
    }
    Some(match event.key() {
        Key::W => WinampWindowKey::Main,
        Key::E => WinampWindowKey::Playlist,
        Key::G => WinampWindowKey::Equalizer,
        _ => return None,
    })
}

/// Where a seek of `step` seconds from `elapsed` lands, as a fraction of the
/// song, or nothing while the song's length is unknown.
pub(super) fn seek_target(elapsed: f32, duration: Option<f32>, step: f32) -> Option<(f32, f32)> {
    let duration = duration.filter(|duration| *duration > 0.0)?;
    let target = (elapsed + step).clamp(0.0, duration);
    Some((target, target / duration))
}

#[cfg(test)]
#[path = "../../test/unit/winamp/keys/tests.rs"]
mod tests;
