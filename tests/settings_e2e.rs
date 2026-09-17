#![cfg(not(target_arch = "wasm32"))]
mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
use cranpose_foundation::Modifiers;
use std::cell::RefCell;
use std::rc::Rc;
fn contains(texts: &[String], needle: &str) -> bool {
    texts.iter().any(|text| text.contains(needle))
}
#[test]
fn clicking_the_logo_opens_and_closes_the_settings_window() {
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        root_key,
        cranamp::winamp::WinampSurfaceApp,
    );
    shell.set_buffer_size(500, 700);
    shell.set_viewport(500.0, 700.0);
    pump(&mut shell);
    let before = visible_texts(&mut shell);
    assert!(
        !contains(&before, "SYNC"),
        "settings panel should be closed on launch; visible={before:?}"
    );
    let logo_x = 26.0 + 249.0 + 13.0;
    let logo_y = 22.0 + 79.0 + 16.0;
    shell.set_cursor(logo_x, logo_y);
    let pressed = shell.pointer_pressed();
    pump(&mut shell);
    let released = shell.pointer_released();
    pump(&mut shell);
    assert!(
        pressed && released,
        "pointer down/up should hit the logo target at ({logo_x},{logo_y})"
    );
    let after = visible_texts(&mut shell);
    assert!(
        contains(&after, "Settings"),
        "settings panel header should appear after logo click; visible={after:?}"
    );
    assert!(
        contains(&after, "SKINS"),
        "skins section should appear; visible={after:?}"
    );
    assert!(
        contains(&after, "Catamp Silverplay (Bundled)"),
        "bundled skin row should be listed; visible={after:?}"
    );
    assert!(
        contains(&after, "Catamp Feral Night (Bundled)"),
        "the second bundled skin should be listed too; visible={after:?}"
    );
    assert!(
        contains(&after, "Catamp Cat Scan (Bundled)"),
        "the third bundled skin should be listed too; visible={after:?}"
    );
    assert!(
        contains(&after, "Catamp Seance (Bundled)"),
        "the fourth bundled skin should be listed too; visible={after:?}"
    );
    assert!(
        contains(&after, "Catamp Salvage (Bundled)"),
        "the fifth bundled skin should be listed too; visible={after:?}"
    );
    assert!(
        contains(&after, "Catamp Freefall (Bundled)"),
        "the sixth bundled skin should be listed too; visible={after:?}"
    );
    assert!(
        contains(&after, "SYNC"),
        "sync section should appear; visible={after:?}"
    );
    assert!(
        contains(&after, "UPDATES"),
        "updates section should appear; visible={after:?}"
    );
    assert!(
        contains(&after, "Open Skin Studio"),
        "desktop settings should expose Skin Studio; visible={after:?}"
    );
    shell.set_cursor(20.0, 20.0);
    shell.pointer_pressed();
    pump(&mut shell);
    shell.pointer_released();
    pump(&mut shell);
    let after_close = visible_texts(&mut shell);
    assert!(
        !contains(&after_close, "SYNC"),
        "settings panel should close after tapping the backdrop; visible={after_close:?}"
    );
}
#[test]
fn playlist_row_click_reports_shift_and_ctrl_from_the_pointer_event() {
    let captured: Rc<RefCell<Vec<Modifiers>>> = Rc::new(RefCell::new(Vec::new()));
    let captured_for_app = Rc::clone(&captured);
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(HitGraphRenderer::default(), root_key, move || {
        let captured = Rc::clone(&captured_for_app);
        cranamp::winamp::PlaylistRowClickTarget(0.0, 0.0, 100.0, 20.0, 1.0, 0, move |modifiers| {
            captured.borrow_mut().push(modifiers);
        });
    });
    shell.set_buffer_size(100, 100);
    shell.set_viewport(100.0, 100.0);
    pump(&mut shell);
    shell.set_cursor(10.0, 10.0);
    assert!(shell.pointer_pressed(), "press should hit the row target");
    pump(&mut shell);
    assert!(
        shell.pointer_released(),
        "release should complete the click"
    );
    pump(&mut shell);
    shell.set_modifiers(Modifiers {
        shift: true,
        ..Modifiers::NONE
    });
    shell.set_cursor(10.0, 10.0);
    assert!(shell.pointer_pressed());
    pump(&mut shell);
    assert!(shell.pointer_released());
    pump(&mut shell);
    shell.set_modifiers(Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    });
    shell.set_cursor(10.0, 10.0);
    assert!(shell.pointer_pressed());
    pump(&mut shell);
    assert!(shell.pointer_released());
    pump(&mut shell);
    let events = captured.borrow();
    assert_eq!(events.len(), 3, "expected exactly three clicks: {events:?}");
    assert_eq!(
        events[0],
        Modifiers::NONE,
        "click before set_modifiers was ever called must report no modifiers"
    );
    assert!(
        events[1].shift && !events[1].ctrl,
        "shift-held click: {:?}",
        events[1]
    );
    assert!(
        events[2].ctrl && !events[2].shift,
        "ctrl-held click: {:?}",
        events[2]
    );
}
#[test]
fn audio_handed_over_by_another_application_plays() {
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        root_key,
        cranamp::winamp::WinampSurfaceApp,
    );
    shell.set_buffer_size(500, 700);
    shell.set_viewport(500.0, 700.0);
    pump(&mut shell);
    let name = "handed-over-by-another-app.mp3";
    let before = visible_texts(&mut shell);
    assert!(
        !contains(&before, "handed-over-by-another-app"),
        "the handed-over track must not already be present; visible={before:?}"
    );
    let bytes =
        std::fs::read(demo_track_for_handover()).expect("a bundled demo track to hand over");
    cranpose_services::publish_incoming_content(
        cranpose_services::IncomingContent::from_bytes(bytes)
            .with_name(name)
            .with_mime_type("audio/mpeg"),
    );
    for _ in 0..80 {
        shell.update();
    }
    let after = visible_texts(&mut shell);
    assert!(
        contains(&after, "handed-over-by-another-app"),
        "handed-over audio should appear in the player; visible={after:?}"
    );
    assert!(
        contains(&after, "Cranamp Demo 01 - Retro Tracker"),
        "the existing playlist should be kept; visible={after:?}"
    );
    assert!(
        after
            .first()
            .is_some_and(|current| current.contains("handed-over-by-another-app")),
        "an idle player should start the handed-over track; visible={after:?}"
    );
}
fn demo_track_for_handover() -> std::path::PathBuf {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/demo-music/generated");
    std::fs::read_dir(&directory)
        .expect("demo music directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "mp3"))
        .expect("at least one bundled demo mp3")
}
