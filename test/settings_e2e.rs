#![cfg(not(target_arch = "wasm32"))]
mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
use cranpose_foundation::Modifiers;
use std::cell::RefCell;
use std::rc::Rc;
static APP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct TestHost(std::path::PathBuf);
impl cranpose_services::HostController for TestHost {
    fn set_keep_screen_on(&self, _: bool) {}
    fn exit(&self) {}
    fn background(&self) {}
    fn platform_directories(&self) -> Option<cranpose_services::PlatformDirectories> {
        Some(cranpose_services::PlatformDirectories {
            data: self.0.join("data"),
            config: self.0.join("config"),
            cache: self.0.join("cache"),
            documents: None,
            temporary: self.0.join("temporary"),
            shared: None,
        })
    }
}
struct TestApplication {
    directory: std::path::PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
}
impl TestApplication {
    fn new(name: &str) -> Self {
        let lock = APP_TEST_LOCK.lock().expect("application test lock");
        let directory = std::env::temp_dir().join(format!(
            "cranamp-settings-{}-{name}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos()
        ));
        std::fs::create_dir(&directory).expect("test storage");
        cranpose_services::set_application_id("cranamp-test").expect("test application id");
        cranpose_services::set_host_controller(std::sync::Arc::new(TestHost(directory.clone())));
        Self {
            directory,
            _lock: lock,
        }
    }
}
impl Drop for TestApplication {
    fn drop(&mut self) {
        cranpose_services::clear_host_controller();
        cranpose_services::clear_application_id();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
fn contains(texts: &[String], needle: &str) -> bool {
    texts.iter().any(|text| text.contains(needle))
}
#[test]
fn clicking_the_logo_opens_and_closes_the_settings_window() {
    let _application = TestApplication::new("settings");
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
        contains(&after, "Anyone with folder access can read them."),
        "sync must explain folder access before the user enables it; visible={after:?}"
    );
    assert!(
        contains(&after, "UPDATES") != cfg!(feature = "store"),
        "updates must appear only in direct builds; visible={after:?}"
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
        cranamp::winamp::PlaylistRowClickTarget(
            "Test track".into(),
            0.0,
            0.0,
            100.0,
            20.0,
            1.0,
            0,
            move |modifiers| {
                captured.borrow_mut().push(modifiers);
            },
        );
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
    let _application = TestApplication::new("handover");
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
        !contains(&before, "tone"),
        "the handed-over track must not already be present; visible={before:?}"
    );
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/audio/tone.wav");
    cranpose_services::publish_incoming_content(
        cranpose_services::IncomingContent::from_file(&source).with_mime_type("audio/wav"),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let after = loop {
        shell.update();
        let texts = visible_texts(&mut shell);
        if texts.iter().any(|text| text == "tone") || std::time::Instant::now() >= deadline {
            break texts;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert!(
        after.iter().any(|text| text == "tone"),
        "handed-over audio should appear in the player; visible={after:?}"
    );
    assert!(
        contains(&after, "Cranamp Demo 01 - Retro Tracker") != cfg!(feature = "store"),
        "direct builds keep the demo playlist; store builds start empty; visible={after:?}"
    );
    assert!(
        {
            fn current_title(node: &cranpose_ui::SemanticsNode) -> bool {
                if node.hidden {
                    return false;
                }
                (node.description.as_deref() == Some("tone") && node.bounds.y < 138.0)
                    || node.children.iter().any(current_title)
            }
            current_title(shell.semantics_tree().unwrap().root())
        },
        "an idle player should start the handed-over track; visible={after:?}"
    );
}

#[test]
fn picked_audio_keeps_the_original_file_without_an_import_copy() {
    let _application = TestApplication::new("audio-references");
    for name in ["tone.opus", "tone.flac", "tone.wav"] {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test/fixtures/audio")
            .join(name);
        let content = cranpose_services::file_content(&source);
        let track = cranamp::audio::track_from_picked_file(content)
            .unwrap()
            .unwrap();
        assert_eq!(
            track.path.as_deref(),
            Some(cranpose_services::uri_for_path(&source).as_str())
        );
    }
    let directory = cranpose_services::application_directories()
        .unwrap()
        .data
        .join("imported-audio");
    assert!(
        !directory.exists(),
        "selecting audio must not create an import cache"
    );
    let virtual_file = cranpose_services::BytesContent::named("memory.opus", vec![0; 128]).handle();
    assert!(cranamp::audio::track_from_picked_file(virtual_file).is_err());
    assert!(!directory.exists());
}

fn mobile_control_bounds(shell: &mut AppShell<HitGraphRenderer>, label: &str) -> cranpose_ui::Rect {
    fn find(node: &cranpose_ui::SemanticsNode, label: &str) -> Option<cranpose_ui::Rect> {
        if node.hidden {
            return None;
        }
        if node.description.as_deref() == Some(label)
            && node.enabled
            && !node.actions.is_empty()
            && (!matches!(label, "Player" | "Skin Studio" | "Settings")
                || node.widget_role == Some(cranpose_foundation::SemanticsWidgetRole::Tab))
        {
            return Some(node.bounds);
        }
        node.children.iter().find_map(|child| find(child, label))
    }
    shell.set_semantics_enabled(true);
    find(shell.semantics_tree().unwrap().root(), label)
        .unwrap_or_else(|| panic!("Missing control {label}"))
}

fn click_mobile(shell: &mut AppShell<HitGraphRenderer>, label: &str) {
    let bounds = mobile_control_bounds(shell, label);
    assert!(
        bounds.width >= 44.0 && bounds.height >= 44.0,
        "{label}: {bounds:?}"
    );
    fn action(
        node: &cranpose_ui::SemanticsNode,
        bounds: cranpose_ui::Rect,
    ) -> Option<cranpose_foundation::SemanticsCustomAction> {
        if node.bounds == bounds {
            if let Some(action) = &node.on_click {
                return Some(action.clone());
            }
        }
        node.children.iter().find_map(|child| action(child, bounds))
    }
    if let Some(action) = action(shell.semantics_tree().unwrap().root(), bounds) {
        action.invoke();
        pump(shell);
        return;
    }
    shell.set_cursor(bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.);
    shell.pointer_pressed();
    pump(shell);
    shell.pointer_released();
    pump(shell);
}

#[test]
fn mobile_tabs_keep_navigation_visible_and_studio_drafts_alive() {
    let _application = TestApplication::new("mobile");
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_buffer_size(393, 780);
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    click_mobile(&mut shell, "Settings");
    assert!(contains(&visible_texts(&mut shell), "Appearance"));
    click_mobile(&mut shell, "Choose skin");
    assert!(contains(&visible_texts(&mut shell), "Choose a skin"));
    click_mobile(&mut shell, "Back to Settings");
    click_mobile(&mut shell, "Skin Studio");
    assert!(contains(&visible_texts(&mut shell), "New project"));
    click_mobile(&mut shell, "New project");
    let studio = visible_texts(&mut shell);
    assert!(!contains(&studio, "Open working copy"), "{studio:?}");
    click_mobile(&mut shell, "Settings");
    click_mobile(&mut shell, "Skin Studio");
    let returned = visible_texts(&mut shell);
    assert!(
        !contains(&returned, "Open working copy"),
        "Draft lost: {returned:?}"
    );
    click_mobile(&mut shell, "Player");
    assert!(contains(&visible_texts(&mut shell), "Play"));
    for (width, height) in [(320., 568.), (844., 390.), (1024., 1366.)] {
        shell.set_buffer_size(width as u32, height as u32);
        shell.set_viewport(width, height);
        pump(&mut shell);
        for label in ["Player", "Skin Studio", "Settings"] {
            let bounds = mobile_control_bounds(&mut shell, label);
            assert!(
                bounds.y >= 0. && bounds.y + bounds.height <= height,
                "{label} outside {width}x{height}: {bounds:?}"
            );
        }
        click_mobile(&mut shell, "Settings");
        click_mobile(&mut shell, "Player");
    }
}
