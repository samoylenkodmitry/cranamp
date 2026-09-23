use super::*;
use cranpose_ui::Modifiers;

fn press(key: KeyCode) -> KeyEvent {
    KeyEvent::key_down(key, "")
}

#[test]
fn the_winamp_letters_drive_the_transport() {
    assert_eq!(winamp_key(&press(KeyCode::Z)), Some(WinampKey::Previous));
    assert_eq!(winamp_key(&press(KeyCode::X)), Some(WinampKey::Play));
    assert_eq!(winamp_key(&press(KeyCode::C)), Some(WinampKey::Pause));
    assert_eq!(winamp_key(&press(KeyCode::V)), Some(WinampKey::Stop));
    assert_eq!(winamp_key(&press(KeyCode::B)), Some(WinampKey::Next));
    assert_eq!(winamp_key(&press(KeyCode::L)), Some(WinampKey::OpenFiles));
}

#[test]
fn the_arrows_seek_and_change_the_volume() {
    assert_eq!(
        winamp_key(&press(KeyCode::ArrowLeft)),
        Some(WinampKey::SeekBack)
    );
    assert_eq!(
        winamp_key(&press(KeyCode::ArrowRight)),
        Some(WinampKey::SeekForward)
    );
    assert_eq!(
        winamp_key(&press(KeyCode::ArrowUp)),
        Some(WinampKey::VolumeUp)
    );
    assert_eq!(
        winamp_key(&press(KeyCode::ArrowDown)),
        Some(WinampKey::VolumeDown)
    );
}

#[test]
fn a_key_held_with_a_system_modifier_is_left_to_the_platform() {
    for modifiers in [
        Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        },
        Modifiers {
            alt: true,
            ..Modifiers::NONE
        },
        Modifiers {
            meta: true,
            ..Modifiers::NONE
        },
    ] {
        let copy = KeyEvent::key_down_with_modifiers(KeyCode::C, "c", modifiers);
        assert_eq!(winamp_key(&copy), None, "{modifiers:?}+C is the platform's");
    }
    let shifted = KeyEvent::key_down_with_modifiers(
        KeyCode::B,
        "B",
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(winamp_key(&shifted), Some(WinampKey::Next));
}

#[test]
fn keys_that_are_not_winamp_keys_pass_through() {
    assert_eq!(winamp_key(&press(KeyCode::Q)), None);
    assert_eq!(winamp_key(&press(KeyCode::Space)), None);
}

#[test]
fn a_seek_stays_inside_the_song() {
    assert_eq!(seek_target(10.0, Some(100.0), 5.0), Some((15.0, 0.15)));
    assert_eq!(seek_target(2.0, Some(100.0), -5.0), Some((0.0, 0.0)));
    assert_eq!(seek_target(98.0, Some(100.0), 5.0), Some((100.0, 1.0)));
    assert_eq!(seek_target(10.0, None, 5.0), None);
    assert_eq!(seek_target(10.0, Some(0.0), 5.0), None);
}
