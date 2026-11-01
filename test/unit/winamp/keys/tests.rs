use super::*;
use cranpose_ui::Modifiers;

fn press(key: Key) -> KeyEvent {
    KeyEvent::key_down(key, "")
}

#[test]
fn the_winamp_letters_drive_the_transport() {
    assert_eq!(winamp_key(&press(Key::Z)), Some(WinampKey::Previous));
    assert_eq!(winamp_key(&press(Key::X)), Some(WinampKey::Play));
    assert_eq!(winamp_key(&press(Key::C)), Some(WinampKey::Pause));
    assert_eq!(winamp_key(&press(Key::V)), Some(WinampKey::Stop));
    assert_eq!(winamp_key(&press(Key::B)), Some(WinampKey::Next));
    assert_eq!(winamp_key(&press(Key::L)), Some(WinampKey::OpenFiles));
}

#[test]
fn the_arrows_seek_and_change_the_volume() {
    assert_eq!(
        winamp_key(&press(Key::DirectionLeft)),
        Some(WinampKey::SeekBack)
    );
    assert_eq!(
        winamp_key(&press(Key::DirectionRight)),
        Some(WinampKey::SeekForward)
    );
    assert_eq!(
        winamp_key(&press(Key::DirectionUp)),
        Some(WinampKey::VolumeUp)
    );
    assert_eq!(
        winamp_key(&press(Key::DirectionDown)),
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
        let copy = KeyEvent::key_down_with_modifiers(Key::C, "c", modifiers);
        assert_eq!(winamp_key(&copy), None, "{modifiers:?}+C is the platform's");
    }
    let shifted = KeyEvent::key_down_with_modifiers(
        Key::B,
        "B",
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(winamp_key(&shifted), Some(WinampKey::Next));
}

#[test]
fn alt_with_w_e_and_g_opens_and_closes_winamps_windows() {
    let alt = |key| {
        KeyEvent::key_down_with_modifiers(
            key,
            "",
            Modifiers {
                alt: true,
                ..Modifiers::NONE
            },
        )
    };
    assert_eq!(window_key(&alt(Key::W)), Some(WinampWindowKey::Main));
    assert_eq!(window_key(&alt(Key::E)), Some(WinampWindowKey::Playlist));
    assert_eq!(window_key(&alt(Key::G)), Some(WinampWindowKey::Equalizer));
    assert_eq!(window_key(&alt(Key::Q)), None);
    assert_eq!(window_key(&press(Key::W)), None, "W alone is not Alt+W");
    let ctrl_alt = KeyEvent::key_down_with_modifiers(
        Key::W,
        "",
        Modifiers {
            alt: true,
            ctrl: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(window_key(&ctrl_alt), None, "Ctrl+Alt+W is the platform's");
}

#[test]
fn keys_that_are_not_winamp_keys_pass_through() {
    assert_eq!(winamp_key(&press(Key::Q)), None);
    assert_eq!(winamp_key(&press(Key::Spacebar)), None);
}

#[test]
fn a_seek_stays_inside_the_song() {
    assert_eq!(seek_target(10.0, Some(100.0), 5.0), Some((15.0, 0.15)));
    assert_eq!(seek_target(2.0, Some(100.0), -5.0), Some((0.0, 0.0)));
    assert_eq!(seek_target(98.0, Some(100.0), 5.0), Some((100.0, 1.0)));
    assert_eq!(seek_target(10.0, None, 5.0), None);
    assert_eq!(seek_target(10.0, Some(0.0), 5.0), None);
}
