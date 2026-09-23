//! Winamp's keyboard: the keys that drive the player from anywhere in it, as
//! Winamp 2 had them. A key held with Ctrl, Alt or the command key is left to
//! the platform, so the browser's and the system's own shortcuts keep working.

use cranpose_ui::{KeyCode, KeyEvent, KeyEventType};

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
    let held = event.modifiers;
    if event.event_type != KeyEventType::KeyDown || held.ctrl || held.alt || held.meta {
        return None;
    }
    Some(match event.key_code {
        KeyCode::Z => WinampKey::Previous,
        KeyCode::X => WinampKey::Play,
        KeyCode::C => WinampKey::Pause,
        KeyCode::V => WinampKey::Stop,
        KeyCode::B => WinampKey::Next,
        KeyCode::ArrowLeft => WinampKey::SeekBack,
        KeyCode::ArrowRight => WinampKey::SeekForward,
        KeyCode::ArrowUp => WinampKey::VolumeUp,
        KeyCode::ArrowDown => WinampKey::VolumeDown,
        KeyCode::L => WinampKey::OpenFiles,
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
