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
        cranpose_services::set_platform_preferences(std::sync::Arc::new(
            cranpose_services::FilePreferences::new(),
        ));
        Self {
            directory,
            _lock: lock,
        }
    }
}
impl Drop for TestApplication {
    fn drop(&mut self) {
        cranpose_services::clear_platform_uri_handler();
        cranpose_services::clear_platform_preferences();
        cranpose_services::clear_host_controller();
        cranpose_services::clear_application_id();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct UnavailableLinkHandler(Rc<RefCell<Vec<String>>>);
impl cranpose_services::UriHandler for UnavailableLinkHandler {
    fn open_uri(&self, uri: &str) -> Result<(), cranpose_services::UriHandlerError> {
        self.0.borrow_mut().push(uri.into());
        Err(cranpose_services::UriHandlerError::NoWindow)
    }
}

#[test]
fn mobile_about_has_offline_licenses_and_recovers_from_unavailable_link_apps() {
    let _application = TestApplication::new("about");
    let opened = Rc::new(RefCell::new(Vec::new()));
    cranpose_services::set_platform_uri_handler(Rc::new(UnavailableLinkHandler(opened.clone())));
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_buffer_size(320, 568);
    shell.set_viewport(320., 568.);
    pump(&mut shell);
    click_mobile(&mut shell, "Settings");
    click_mobile(&mut shell, "About Cranamp");
    let about = visible_texts(&mut shell);
    assert!(contains(
        &about,
        &format!("Version {}", env!("CARGO_PKG_VERSION"))
    ));
    assert!(contains(&about, "cranamp@dmitrysamoylenko.in"));
    click_mobile(&mut shell, "Contact support");
    assert_eq!(&*opened.borrow(), &["mailto:cranamp@dmitrysamoylenko.in"]);
    assert!(contains(
        &visible_texts(&mut shell),
        "Could not open this link"
    ));
    click_mobile(&mut shell, "Open source licenses");
    click_mobile(&mut shell, "Cranamp — Apache License 2.0");
    assert!(contains(
        &visible_texts(&mut shell),
        "Grant of Copyright License"
    ));
    // Legal screens stay above the navigation controls and remain usable offline.
    let back = mobile_control_bounds(&mut shell, "Back to Licenses");
    assert!(back.y >= 0. && back.y + back.height < 568. - 86.);
    click_mobile(&mut shell, "Back to Licenses");
    click_mobile(&mut shell, "Liberation Sans — SIL Open Font License");
    assert!(contains(
        &visible_texts(&mut shell),
        "SIL OPEN FONT LICENSE"
    ));
    click_mobile(&mut shell, "Back to Licenses");
    click_mobile(&mut shell, "Back to About");
    click_mobile(&mut shell, "Back to Settings");
    assert!(contains(&visible_texts(&mut shell), "APPEARANCE"));
    click_mobile(&mut shell, "Player");
    assert!(contains(&visible_texts(&mut shell), "Play"));
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
            && (!matches!(label, "Player" | "Library" | "Skin Studio" | "Settings")
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
        shell.debug_enter_app_context(|| action.invoke());
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
fn mobile_player_keeps_the_skin_edge_at_the_screen_edge() {
    let _application = TestApplication::new("mobile-width");
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    for (width, height) in [(440., 860.), (393., 760.), (320., 548.)] {
        shell.set_buffer_size(width as u32, height as u32);
        shell.set_viewport(width, height);
        pump(&mut shell);
        let close = mobile_control_bounds(&mut shell, "Close player");
        let right_border = 2.0 * width / 275.0;
        assert!(
            (close.x + close.width + right_border - width).abs() <= 1.0,
            "Skin inset at {width}x{height}: close button {close:?}"
        );
    }
}

#[test]
fn mobile_controls_avoid_the_notch_and_home_indicator() {
    let _application = TestApplication::new("mobile-safe-area");
    for (width, height, insets) in [
        (
            440.,
            956.,
            cranpose_ui::EdgeInsets::from_components(0., 62., 0., 34.),
        ),
        (
            844.,
            390.,
            cranpose_ui::EdgeInsets::from_components(47., 0., 47., 21.),
        ),
    ] {
        let mut shell = AppShell::new(
            HitGraphRenderer::default(),
            location_key(file!(), line!(), column!()),
            move || {
                cranpose_core::CompositionLocalProvider(
                    vec![cranpose_ui::local_safe_area_insets().provides(insets)],
                    cranamp::winamp::WinampMobileApp,
                )
            },
        );
        shell.set_buffer_size(width as u32, height as u32);
        shell.set_viewport(width, height);
        pump(&mut shell);
        for destination in ["Player", "Library", "Settings", "Skin Studio"] {
            click_mobile(&mut shell, destination);
            for label in ["Player", "Library", "Skin Studio", "Settings"] {
                let bounds = mobile_control_bounds(&mut shell, label);
                assert!(
                    bounds.x >= insets.left && bounds.x + bounds.width <= width - insets.right,
                    "{label}: {bounds:?}"
                );
                assert!(
                    bounds.y >= insets.top && bounds.y + bounds.height <= height - insets.bottom,
                    "{label}: {bounds:?}"
                );
            }
            let top_control = match destination {
                "Player" => "Close player",
                "Library" => "Add music",
                "Settings" => "Choose skin",
                _ => "New project",
            };
            let bounds = mobile_control_bounds(&mut shell, top_control);
            assert!(
                bounds.y >= insets.top,
                "{top_control} overlaps status bar: {bounds:?}"
            );
        }
    }
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
    assert!(contains(&visible_texts(&mut shell), "APPEARANCE"));
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
        for label in ["Player", "Library", "Skin Studio", "Settings"] {
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

fn seed_library(titles: &[&str]) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/audio/tone.wav");
    let hex = |text: &str| {
        text.bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let mut saved = "version=1\ncurrent_index=0\n".to_owned();
    for title in titles {
        saved.push_str(&format!(
            "track={}\t{}\t90\n",
            hex(title),
            hex(&path.to_string_lossy())
        ));
    }
    cranpose_services::preferences()
        .set("cranamp.player", &saved)
        .unwrap();
}

fn wait_for_saved_playlists(
    shell: &mut AppShell<HitGraphRenderer>,
    ready: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        shell.update();
        pump(shell);
        if let Some(value) = cranpose_services::preferences()
            .get("cranamp.playlists.v1")
            .and_then(|text| serde_json::from_str(&text).ok())
            .filter(&ready)
        {
            return value;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Saved playlists did not settle"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn enter_mobile_text(shell: &mut AppShell<HitGraphRenderer>, label: &str, value: &str) {
    // Semantics expose the final sheet bounds while its entrance is animated.
    // Allow the surface to reach those bounds before sending a physical tap.
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(16));
        shell.update();
        pump(shell);
    }
    let bounds = mobile_control_bounds(shell, label);
    shell.set_cursor(bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.);
    shell.pointer_pressed();
    shell.pointer_released();
    pump(shell);
    assert!(
        shell.ime_editor_state().is_some(),
        "No focus for {value}: {bounds:?}; {:?}",
        visible_texts(shell)
    );
    shell.on_key_event(&cranpose_ui::KeyEvent::key_down_with_modifiers(
        cranpose_ui::KeyCode::A,
        "a",
        cranpose_ui::Modifiers {
            ctrl: true,
            meta: true,
            ..cranpose_ui::Modifiers::NONE
        },
    ));
    assert!(shell.on_paste(value), "Cannot enter {value}");
    pump(shell);
}

fn theme_selected(shell: &mut AppShell<HitGraphRenderer>, label: &str) -> bool {
    fn find(node: &cranpose_ui::SemanticsNode, label: &str) -> bool {
        !node.hidden
            && ((node.description.as_deref() == Some(label) && node.selected == Some(true))
                || node.children.iter().any(|child| find(child, label)))
    }
    shell.set_semantics_enabled(true);
    find(shell.semantics_tree().unwrap().root(), label)
}

#[test]
fn mobile_theme_choice_is_saved_and_restored_after_rapid_changes() {
    let _application = TestApplication::new("theme-choice");
    let preferences = cranpose_services::preferences();
    preferences
        .set("cranamp.appearance.v1", "unknown-future-mode")
        .unwrap();
    let launch = || {
        let mut shell = AppShell::new(
            HitGraphRenderer::default(),
            location_key(file!(), line!(), column!()),
            cranamp::winamp::WinampMobileApp,
        );
        shell.set_viewport(393., 852.);
        pump(&mut shell);
        click_mobile(&mut shell, "Settings");
        click_mobile(&mut shell, "Theme");
        shell
    };
    let mut shell = launch();
    assert!(theme_selected(&mut shell, "Use Auto theme"));
    for label in [
        "Day",
        "Night",
        "Auto",
        "From skin background",
        "Day",
        "Night",
    ] {
        click_mobile(&mut shell, &format!("Use {label} theme"));
        assert!(theme_selected(&mut shell, &format!("Use {label} theme")));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while preferences.get("cranamp.appearance.v1").as_deref() != Some("night") {
        assert!(std::time::Instant::now() < deadline, "Theme was not saved");
        std::thread::sleep(std::time::Duration::from_millis(10));
        shell.update();
        pump(&mut shell);
    }
    drop(shell);
    let mut shell = launch();
    while !theme_selected(&mut shell, "Use Night theme") {
        assert!(
            std::time::Instant::now() < deadline,
            "Theme was not restored"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
        shell.update();
        pump(&mut shell);
    }
    click_mobile(&mut shell, "Back to Settings");
    assert!(contains(&visible_texts(&mut shell), "Night"));
    click_mobile(&mut shell, "Library");
    click_mobile(&mut shell, "Settings");
    click_mobile(&mut shell, "Theme");
    assert!(theme_selected(&mut shell, "Use Night theme"));
}

fn library_scrollbar(shell: &mut AppShell<HitGraphRenderer>) -> Option<cranpose_ui::SemanticsNode> {
    fn find(node: &cranpose_ui::SemanticsNode) -> Option<cranpose_ui::SemanticsNode> {
        if node.hidden {
            return None;
        }
        if node.description.as_deref() == Some("Library scrollbar") && node.enabled {
            return Some(node.clone());
        }
        node.children.iter().rev().find_map(find)
    }
    shell.set_semantics_enabled(true);
    find(shell.semantics_tree().unwrap().root())
}

fn seek_library(shell: &mut AppShell<HitGraphRenderer>, progress: f32) {
    let bar = library_scrollbar(shell).expect("scrollbar for a long list");
    shell.debug_enter_app_context(|| {
        assert!(bar.details().set_progress.clone().unwrap().invoke(progress))
    });
    pump(shell);
}

#[test]
fn library_scrollbars_jump_swipe_filter_and_fit_every_list() {
    let app = TestApplication::new("library-scrollbars");
    let hex = |text: &str| text.bytes().map(|b| format!("{b:02x}")).collect::<String>();
    let mut saved = "version=1\ncurrent_index=0\n".to_owned();
    let tracks: Vec<_> = (0..1000)
        .map(|i| {
            let title = if i == 0 {
                "Unique Needle".into()
            } else {
                format!("Song {i:04}")
            };
            let path = app
                .directory
                .join(format!("Album {:02}/{title}.wav", i / 25));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::hard_link(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("test/fixtures/audio/tone.wav"),
                &path,
            )
            .unwrap();
            saved.push_str(&format!(
                "track={}\t{}\t90\n",
                hex(&title),
                hex(&path.to_string_lossy())
            ));
            serde_json::json!({"title": title, "path": path, "duration_seconds": 90.})
        })
        .collect();
    let lists: Vec<_> = (1..=60)
        .map(|i| serde_json::json!({"id":i,"name":format!("List {i:02}"),"tracks":if i == 1 { &tracks[..] } else { &tracks[..1] }}))
        .collect();
    let preferences = cranpose_services::preferences();
    preferences.set("cranamp.player", &saved).unwrap();
    preferences
        .set(
            "cranamp.playlists.v1",
            &serde_json::json!({"version":1,"next_id":61,"lists":lists,"available":tracks})
                .to_string(),
        )
        .unwrap();
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_buffer_size(393, 852);
    shell.set_viewport(393., 852.);
    pump(&mut shell);
    click_mobile(&mut shell, "Library");
    let bar = library_scrollbar(&mut shell)
        .unwrap_or_else(|| panic!("Missing bar: {:?}", visible_texts(&mut shell)));
    assert_eq!(bar.bounds.width, 44.);
    assert!(bar.bounds.x > 340.);
    assert!(bar.bounds.y + bar.bounds.height <= 852. - 152.);
    let options = mobile_control_bounds(&mut shell, "Track 1 options");
    assert!(
        options.x + options.width <= bar.bounds.x + 0.5,
        "row buttons overlap scrollbar: {options:?} / {:?}",
        bar.bounds
    );
    // Drag the bar, then return through its accessibility action.
    shell.set_cursor(bar.bounds.x + 36., bar.bounds.y + 12.);
    shell.pointer_pressed();
    for step in 1..=8 {
        shell.set_cursor(
            bar.bounds.x + 36.,
            bar.bounds.y + 12. + step as f32 * (bar.bounds.height - 28.) / 8.,
        );
        pump(&mut shell);
    }
    shell.pointer_released();
    pump(&mut shell);
    assert!(
        library_scrollbar(&mut shell)
            .unwrap()
            .details()
            .progress
            .unwrap()
            .current
            > 0.8
    );
    seek_library(&mut shell, 1.);
    mobile_control_bounds(&mut shell, "Play track 1000: Song 0999");
    seek_library(&mut shell, 0.);
    mobile_control_bounds(&mut shell, "Play track 1: Unique Needle");
    // A normal content swipe must also move the thumb.
    shell.set_cursor(180., 520.);
    shell.pointer_pressed();
    for y in [480., 440., 400., 360., 320.] {
        shell.set_cursor(180., y);
        pump(&mut shell);
    }
    shell.pointer_released();
    pump(&mut shell);
    assert!(
        library_scrollbar(&mut shell)
            .unwrap()
            .details()
            .progress
            .unwrap()
            .current
            > 0.
    );
    seek_library(&mut shell, 0.);
    enter_mobile_text(&mut shell, "Search your playlist", "Unique Needle");
    click_mobile(&mut shell, "Finish search");
    assert!(
        library_scrollbar(&mut shell).is_none(),
        "Filtered list: {:?}; bar: {:?}",
        visible_texts(&mut shell),
        library_scrollbar(&mut shell).map(|n| n.bounds)
    );
    click_mobile(&mut shell, "Clear search");
    assert!(library_scrollbar(&mut shell).is_some());
    click_mobile(&mut shell, "Library Albums tab");
    seek_library(&mut shell, 1.);
    mobile_control_bounds(&mut shell, "Open album Album 39");
    click_mobile(&mut shell, "Open album Album 39");
    seek_library(&mut shell, 1.);
    mobile_control_bounds(&mut shell, "Play track 1000: Song 0999");
    click_mobile(&mut shell, "Library Playlists tab");
    wait_for_saved_playlists(&mut shell, |v| {
        v["lists"].as_array().is_some_and(|v| v.len() == 60)
    });
    seek_library(&mut shell, 1.);
    mobile_control_bounds(&mut shell, "Open playlist List 01");
    click_mobile(&mut shell, "Open playlist List 01");
    seek_library(&mut shell, 1.);
    mobile_control_bounds(&mut shell, "Saved track 1000 options");
    click_mobile(&mut shell, "Add tracks to playlist");
    seek_library(&mut shell, 1.);
    assert!(contains(&visible_texts(&mut shell), "Song 0999"));
    for (width, height) in [(320., 568.), (844., 390.), (768., 1024.)] {
        shell.set_viewport(width, height);
        pump(&mut shell);
        let bounds = library_scrollbar(&mut shell).unwrap().bounds;
        assert!(bounds.width >= 44. && bounds.height > 0.);
        assert!(bounds.x >= 0. && bounds.x + bounds.width <= width);
        assert!(
            bounds.y + bounds.height <= height - 152.,
            "bar under dock: {bounds:?}"
        );
        seek_library(&mut shell, 0.);
        seek_library(&mut shell, 1.);
    }
    shell.set_viewport(393., 852.);
    pump(&mut shell);
    click_mobile(&mut shell, "Cancel adding tracks");
    click_mobile(&mut shell, "Library Tracks tab");
    seek_library(&mut shell, 0.);
    click_mobile(&mut shell, "Track 1 options");
    click_mobile(&mut shell, "Add to saved playlist");
    // This sheet contains 60 destinations and must scroll independently.
    seek_library(&mut shell, 1.);
    assert!(contains(&visible_texts(&mut shell), "List 01"));
    let bar = library_scrollbar(&mut shell).unwrap();
    assert!(bar.details().progress.unwrap().current > 0.99);
    click_mobile(&mut shell, "Cancel");
    // Scrolling must not select or replace the user's playing queue.
    let current = preferences.get("cranamp.player").unwrap();
    assert!(current.contains("current_index=0"));
    assert_eq!(
        current
            .lines()
            .filter(|line| line.starts_with("track="))
            .count(),
        1000
    );
}

#[test]
fn saved_playlists_create_pick_reorder_rename_play_and_survive_restart() {
    let app = TestApplication::new("saved-playlists");
    let hex = |text: &str| text.bytes().map(|b| format!("{b:02x}")).collect::<String>();
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/audio/tone.wav");
    let mut state = "version=1\ncurrent_index=0\n".to_owned();
    for title in ["Cats - Sunrise", "Cats - Moon", "Dogs - River"] {
        let source = app.directory.join(format!("{title}.wav"));
        std::fs::copy(&fixture, &source).unwrap();
        state.push_str(&format!(
            "track={}\t{}\t90\n",
            hex(title),
            hex(&source.to_string_lossy())
        ));
    }
    cranpose_services::preferences()
        .set("cranamp.player", &state)
        .unwrap();
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_buffer_size(393, 780);
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    wait_for_saved_playlists(&mut shell, |v| {
        v["available"].as_array().is_some_and(|a| a.len() == 3)
    });
    click_mobile(&mut shell, "Library");
    click_mobile(&mut shell, "Library Playlists tab");
    click_mobile(&mut shell, "New playlist");
    click_mobile(&mut shell, "Save playlist");
    assert!(contains(
        &visible_texts(&mut shell),
        "Enter a playlist name."
    ));
    enter_mobile_text(&mut shell, "Playlist name", "Night Drive");
    click_mobile(&mut shell, "Save playlist");
    click_mobile(&mut shell, "Add tracks to playlist");
    click_mobile(&mut shell, "Choose track 1: Sunrise");
    click_mobile(&mut shell, "Choose track 3: River");
    click_mobile(&mut shell, "Add selected tracks");
    mobile_control_bounds(&mut shell, "Play saved track 1: Sunrise");
    mobile_control_bounds(&mut shell, "Play saved track 2: River");
    click_mobile(&mut shell, "Saved track 2 options");
    click_mobile(&mut shell, "Move up");
    assert!(
        contains(&visible_texts(&mut shell), "Play saved track 1: River"),
        "After reorder: {:?}",
        visible_texts(&mut shell)
    );
    mobile_control_bounds(&mut shell, "Play saved track 1: River");
    click_mobile(&mut shell, "Saved playlist options");
    click_mobile(&mut shell, "Rename playlist");
    enter_mobile_text(&mut shell, "Playlist name", "After Dark");
    click_mobile(&mut shell, "Save playlist");
    click_mobile(&mut shell, "Saved playlist options");
    click_mobile(&mut shell, "Duplicate playlist");
    click_mobile(&mut shell, "Saved track 1 options");
    click_mobile(&mut shell, "Remove from saved playlist");
    mobile_control_bounds(&mut shell, "Play saved track 1: Sunrise");
    for (width, height) in [(320., 568.), (844., 390.), (768., 1024.)] {
        shell.set_buffer_size(width as u32, height as u32);
        shell.set_viewport(width, height);
        pump(&mut shell);
        let row = mobile_control_bounds(&mut shell, "Play saved track 1: Sunrise");
        let dock = mobile_control_bounds(&mut shell, "Show current track in Player");
        assert!(
            row.y + row.height <= dock.y,
            "Saved track overlaps player dock at {width}x{height}: {row:?} {dock:?}"
        );
        click_mobile(&mut shell, "Add tracks to playlist");
        let add = mobile_control_bounds(&mut shell, "Add selected tracks");
        assert!(
            add.x >= 0. && add.x + add.width <= width,
            "Picker button clipped: {add:?}"
        );
        click_mobile(&mut shell, "Cancel adding tracks");
    }
    shell.set_buffer_size(393, 780);
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    click_mobile(&mut shell, "Back to playlists");
    click_mobile(&mut shell, "Open playlist After Dark");
    click_mobile(&mut shell, "Play playlist");
    click_mobile(&mut shell, "Library Tracks tab");
    mobile_control_bounds(&mut shell, "Play track 1: River");
    mobile_control_bounds(&mut shell, "Play track 2: Sunrise");
    click_mobile(&mut shell, "Library Playlists tab");
    click_mobile(&mut shell, "Open playlist After Dark");
    click_mobile(&mut shell, "Add tracks to playlist");
    mobile_control_bounds(&mut shell, "Choose track 2: Moon");
    click_mobile(&mut shell, "Cancel adding tracks");
    let saved = wait_for_saved_playlists(&mut shell, |v| {
        v["lists"].as_array().is_some_and(|a| {
            a.len() == 2 && a[1]["tracks"].as_array().is_some_and(|t| t.len() == 1)
        })
    });
    assert_eq!(saved["lists"][0]["name"], "After Dark");
    assert_eq!(saved["lists"][0]["tracks"][0]["title"], "Dogs - River");
    assert_eq!(saved["available"].as_array().unwrap().len(), 3);
    drop(shell);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    // Let the asynchronous load finish before using saved-list controls.
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        shell.update();
        pump(&mut shell);
    }
    click_mobile(&mut shell, "Library");
    click_mobile(&mut shell, "Library Playlists tab");
    mobile_control_bounds(&mut shell, "Open playlist After Dark");
    click_mobile(&mut shell, "Open playlist After Dark copy");
    click_mobile(&mut shell, "Saved playlist options");
    click_mobile(&mut shell, "Delete playlist");
    click_mobile(&mut shell, "Cancel");
    mobile_control_bounds(&mut shell, "Play saved track 1: Sunrise");
    click_mobile(&mut shell, "Saved playlist options");
    click_mobile(&mut shell, "Delete playlist");
    click_mobile(&mut shell, "Confirm delete playlist");
    let saved = wait_for_saved_playlists(&mut shell, |v| {
        v["lists"].as_array().is_some_and(|a| a.len() == 1)
    });
    assert_eq!(saved["lists"][0]["name"], "After Dark");
    click_mobile(&mut shell, "Library Tracks tab");
    click_mobile(&mut shell, "Select");
    click_mobile(&mut shell, "Select results");
    click_mobile(&mut shell, "Add selected to playlist");
    click_mobile(&mut shell, "New playlist");
    enter_mobile_text(&mut shell, "Playlist name", "Queue selection");
    click_mobile(&mut shell, "Save playlist");
    wait_for_saved_playlists(&mut shell, |v| {
        v["lists"].as_array().is_some_and(|lists| {
            lists.iter().any(|list| {
                list["name"] == "Queue selection"
                    && list["tracks"]
                        .as_array()
                        .is_some_and(|tracks| tracks.len() == 2)
            })
        })
    });
    click_mobile(&mut shell, "Library Tracks tab");
    mobile_control_bounds(&mut shell, "Play track 1: River");
    assert_eq!(
        std::fs::read_dir(&app.directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "wav"))
            .count(),
        3
    );
}

#[test]
fn mobile_library_search_selection_reorder_and_removal_share_the_player_playlist() {
    let _application = TestApplication::new("library");
    seed_library(&["Cats - Zebra", "Cats - Aurora", "Dogs - Moon"]);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_buffer_size(393, 780);
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    click_mobile(&mut shell, "Library");
    assert!(contains(&visible_texts(&mut shell), "3 tracks"));
    click_mobile(&mut shell, "Track 1 options");
    click_mobile(&mut shell, "Move down");
    mobile_control_bounds(&mut shell, "Play track 2: Zebra");
    click_mobile(&mut shell, "Sort playlist");
    click_mobile(&mut shell, "Track title");
    mobile_control_bounds(&mut shell, "Play track 1: Aurora");
    let search = mobile_control_bounds(&mut shell, "Search your playlist");
    shell.set_cursor(search.x + search.width / 2., search.y + search.height / 2.);
    shell.pointer_pressed();
    shell.pointer_released();
    pump(&mut shell);
    assert!(shell.on_paste("Aurora"));
    pump(&mut shell);
    assert!(contains(&visible_texts(&mut shell), "1 of 3 tracks"));
    click_mobile(&mut shell, "Finish search");
    assert!(shell.ime_editor_state().is_none());
    click_mobile(&mut shell, "Settings");
    click_mobile(&mut shell, "Library");
    assert!(contains(&visible_texts(&mut shell), "1 of 3 tracks"));
    click_mobile(&mut shell, "Select");
    click_mobile(&mut shell, "Select results");
    assert!(contains(&visible_texts(&mut shell), "1 selected"));
    click_mobile(&mut shell, "Remove selected tracks");
    click_mobile(&mut shell, "Cancel");
    assert!(contains(&visible_texts(&mut shell), "3 tracks"));
    click_mobile(&mut shell, "Remove selected tracks");
    click_mobile(&mut shell, "Confirm remove tracks");
    assert!(contains(&visible_texts(&mut shell), "No matching tracks"));
    click_mobile(&mut shell, "Clear search");
    assert!(contains(&visible_texts(&mut shell), "2 tracks"));
    mobile_control_bounds(&mut shell, "Play track 1: Zebra");
    click_mobile(&mut shell, "Player");
    let player = visible_texts(&mut shell);
    assert!(contains(&player, "Cats - Zebra"), "{player:?}");
    assert!(!contains(&player, "Cats - Aurora"), "{player:?}");
    assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test/fixtures/audio/tone.wav")
        .is_file());
}

#[test]
fn mobile_library_empty_state_and_actions_fit_small_phone_and_landscape() {
    let _application = TestApplication::new("empty-library");
    seed_library(&[]);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    for (width, height) in [(320., 568.), (844., 390.), (1024., 1366.)] {
        shell.set_buffer_size(width as u32, height as u32);
        shell.set_viewport(width, height);
        pump(&mut shell);
        click_mobile(&mut shell, "Library");
        if !cfg!(feature = "store") && width == 320. {
            click_mobile(&mut shell, "Library options");
            click_mobile(&mut shell, "Clear playlist");
            click_mobile(&mut shell, "Confirm remove tracks");
        }
        assert!(contains(&visible_texts(&mut shell), "Add your music"));
        for label in ["Add music", "Library options", "Search your playlist"] {
            let rect = mobile_control_bounds(&mut shell, label);
            assert!(
                rect.x >= 0.
                    && rect.y >= 0.
                    && rect.x + rect.width <= width
                    && rect.y + rect.height <= height - 86.,
                "{label}: {rect:?}"
            );
        }
        click_mobile(&mut shell, "Add music");
        mobile_control_bounds(&mut shell, "Choose a music folder");
        click_mobile(&mut shell, "Cancel");
    }
}

#[test]
fn mobile_library_keyboard_keeps_tabs_visible_and_tab_change_clears_focus() {
    let _application = TestApplication::new("library-keyboard");
    seed_library(&["Cats - Zebra"]);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        || {
            cranpose_core::CompositionLocalProvider(
                vec![
                    cranpose_ui::local_safe_area_insets()
                        .provides(cranpose_ui::EdgeInsets::from_components(0., 59., 0., 34.)),
                    cranpose_ui::local_ime_insets()
                        .provides(cranpose_ui::EdgeInsets::from_components(0., 0., 0., 300.)),
                ],
                cranamp::winamp::WinampMobileApp,
            );
        },
    );
    shell.set_buffer_size(393, 780);
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    click_mobile(&mut shell, "Library");
    let search = mobile_control_bounds(&mut shell, "Search your playlist");
    shell.set_cursor(search.x + search.width / 2., search.y + search.height / 2.);
    shell.pointer_pressed();
    shell.pointer_released();
    pump(&mut shell);
    mobile_control_bounds(&mut shell, "Finish search");
    assert!(shell.on_paste("Zebra"));
    pump(&mut shell);
    for label in ["Player", "Library", "Skin Studio", "Settings"] {
        let bounds = mobile_control_bounds(&mut shell, label);
        assert!(bounds.y + bounds.height <= 480., "{label}: {bounds:?}");
    }
    click_mobile(&mut shell, "Settings");
    assert!(shell.ime_editor_state().is_none());
    click_mobile(&mut shell, "Library");
    mobile_control_bounds(&mut shell, "Clear search");
}

#[test]
fn mobile_skin_gallery_filters_and_the_player_dock_keeps_controls_reachable() {
    let _application = TestApplication::new("skin-gallery-dock");
    seed_library(&[
        "Cats - Aurora",
        "Cats - A very long song title that must be clipped",
    ]);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    for (width, height) in [(320., 568.), (393., 780.), (844., 390.)] {
        shell.set_buffer_size(width as u32, height as u32);
        shell.set_viewport(width, height);
        pump(&mut shell);
        for screen in ["Library", "Settings"] {
            click_mobile(&mut shell, screen);
            let tab = mobile_control_bounds(&mut shell, screen);
            for label in [
                "Show current track in Player",
                "Play library selection",
                "Next library track",
            ] {
                let bounds = mobile_control_bounds(&mut shell, label);
                assert!(
                    bounds.height >= 44. && bounds.width >= 44.,
                    "{label}: {bounds:?}"
                );
                assert!(
                    bounds.x >= 0.
                        && bounds.y >= 0.
                        && bounds.x + bounds.width <= width
                        && bounds.y + bounds.height <= tab.y,
                    "{label} overlaps tabs at {width}x{height}: {bounds:?}"
                );
            }
        }
        click_mobile(&mut shell, "Choose skin");
        let first = mobile_control_bounds(&mut shell, "Silverplay · Selected");
        let second = mobile_control_bounds(&mut shell, "Feral Night");
        assert!(first.x + first.width < second.x, "Skin previews overlap");
        let search = mobile_control_bounds(&mut shell, "Find a skin");
        shell.set_cursor(search.x + search.width / 2., search.y + search.height / 2.);
        shell.pointer_pressed();
        shell.pointer_released();
        pump(&mut shell);
        assert!(shell.on_paste("no-skin-with-this-name"));
        pump(&mut shell);
        assert!(contains(&visible_texts(&mut shell), "No matching skins"));
        click_mobile(&mut shell, "Clear skin search");
        mobile_control_bounds(&mut shell, "Silverplay · Selected");
        click_mobile(&mut shell, "Finish skin search");
        assert!(shell.ime_editor_state().is_none());
        click_mobile(&mut shell, "Feral Night");
        assert!(
            contains(&visible_texts(&mut shell), "Feral Night · Selected"),
            "Skin selection at {width}x{height}: {:?}",
            visible_texts(&mut shell)
        );
        mobile_control_bounds(&mut shell, "Feral Night · Selected");
        click_mobile(&mut shell, "Silverplay");
        mobile_control_bounds(&mut shell, "Silverplay · Selected");
        click_mobile(&mut shell, "Back to Settings");
        click_mobile(&mut shell, "Show current track in Player");
        assert!(contains(&visible_texts(&mut shell), "Play"));
    }
}

#[test]
fn library_indexes_tags_folder_art_and_albums_without_changing_original_audio() {
    let app = TestApplication::new("album-index");
    let album = app.directory.join("Original album");
    std::fs::create_dir(&album).unwrap();
    let tagged = album.join("01 - Wrong filename.flac");
    let untagged = album.join("02 - Folder artist - Untagged.wav");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::copy(fixture.join("test/fixtures/library/tagged.flac"), &tagged).unwrap();
    std::fs::copy(fixture.join("test/fixtures/audio/tone.wav"), &untagged).unwrap();
    let original = std::fs::read(&tagged).unwrap();
    image::RgbImage::from_pixel(48, 32, image::Rgb([12, 34, 56]))
        .save(album.join("Front.JPG"))
        .unwrap();
    let hex = |text: &str| text.bytes().map(|b| format!("{b:02x}")).collect::<String>();
    let mut saved = "version=1\ncurrent_index=0\n".to_owned();
    for path in [&tagged, &untagged] {
        saved.push_str(&format!(
            "track={}\t{}\t\n",
            hex(&path.file_stem().unwrap().to_string_lossy()),
            hex(&path.to_string_lossy())
        ));
    }
    cranpose_services::preferences()
        .set("cranamp.player", &saved)
        .unwrap();
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        cranamp::winamp::WinampMobileApp,
    );
    shell.set_viewport(393., 780.);
    pump(&mut shell);
    click_mobile(&mut shell, "Library");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        shell.update();
        pump(&mut shell);
        if contains(&visible_texts(&mut shell), "Tagged title") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Metadata worker did not update the Library"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    mobile_control_bounds(&mut shell, "Play track 1: Tagged title");
    // The glass lens commits the new view when a horizontal drag is released.
    let tracks = mobile_control_bounds(&mut shell, "Library Tracks tab");
    let albums = mobile_control_bounds(&mut shell, "Library Albums tab");
    let start = tracks.x + tracks.width * 0.5;
    let end = albums.x + albums.width * 0.5;
    let y = tracks.y + tracks.height * 0.5;
    shell.set_cursor(start, y);
    shell.pointer_pressed();
    pump(&mut shell);
    for step in 1..=4 {
        shell.set_cursor(start + (end - start) * step as f32 / 4., y);
        pump(&mut shell);
    }
    shell.pointer_released();
    pump(&mut shell);
    mobile_control_bounds(&mut shell, "Open album Tagged album");
    mobile_control_bounds(&mut shell, "Open album Original album");
    click_mobile(&mut shell, "Open album Tagged album");
    mobile_control_bounds(&mut shell, "Play track 1: Tagged title");
    assert!(!contains(
        &visible_texts(&mut shell),
        "Play track 2: Untagged"
    ));
    click_mobile(&mut shell, "Back to albums");
    click_mobile(&mut shell, "Open album Original album");
    mobile_control_bounds(&mut shell, "Play track 2: Untagged");
    click_mobile(&mut shell, "Library Tracks tab");
    assert_eq!(std::fs::read(&tagged).unwrap(), original);
    assert_eq!(
        std::fs::read_dir(&album).unwrap().count(),
        3,
        "No sidecar or audio copy may be written to the source folder"
    );
    let cache = cranpose_services::application_directories()
        .unwrap()
        .cache
        .join("library-v1");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !cache.join("index.json").is_file() {
        shell.update();
        pump(&mut shell);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(std::fs::read_dir(cache)
        .unwrap()
        .flatten()
        .any(|e| e.path().extension().is_some_and(|e| e == "png")));
}
