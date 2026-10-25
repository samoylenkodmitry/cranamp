#![cfg(not(target_arch = "wasm32"))]
mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
const WINDOW: (u32, u32) = (393, 780);
fn touch(document: cranamp::winamp::studio::SharedDocument) -> AppShell<HitGraphRenderer> {
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(HitGraphRenderer::default(), root_key, move || {
        cranamp::winamp::studio::SkinStudio(document.clone(), None)
    });
    shell.set_buffer_size(WINDOW.0, WINDOW.1);
    shell.set_viewport(WINDOW.0 as f32, WINDOW.1 as f32);
    pump(&mut shell);
    shell
}
fn contains(texts: &[String], needle: &str) -> bool {
    texts.iter().any(|text| text.contains(needle))
}
#[test]
fn desktop_agent_panel_explains_local_access_and_offers_connection_controls() {
    let document = cranamp::winamp::studio::open_document(None).expect("a document");
    open(&document, "agent", "pencil");
    let mut shell = touch(document);
    let texts = visible_texts(&mut shell);
    for control in [
        "AGENT CONNECTION",
        "Stopped",
        "127.0.0.1:18765/mcp",
        "Start server",
        "Test connection",
        "Copy connection details",
        "local programs can edit this skin",
    ] {
        assert!(contains(&texts, control), "Missing {control}: {texts:?}");
    }
}
fn open(document: &cranamp::winamp::studio::SharedDocument, drawer: &str, brush: &str) {
    document
        .lock()
        .unwrap()
        .state(serde_json::json!({ "drawer": drawer, "brush": brush }))
        .expect("the view takes a drawer and a brush");
}
#[test]
fn the_text_brush_offers_its_face_its_scale_and_its_letter_spacing() {
    let document = cranamp::winamp::studio::open_document(None).expect("a document");
    open(&document, "tools", "text");
    let mut shell = touch(document);
    let texts = visible_texts(&mut shell);
    for control in ["Text", "Text size", "5 x 7 font", "Small font", "Spacing"] {
        assert!(
            contains(&texts, control),
            "{control:?} is a text brush setting and has to be pressable here; visible={texts:?}"
        );
    }
}
#[test]
fn a_drawer_named_on_the_document_is_the_drawer_this_layout_opens() {
    for (drawer, marker) in [
        ("tools", "Tool properties"),
        ("layers", "Layers, top first"),
        ("targets", "SPRITE TARGETS"),
        ("options", "Playlist colours"),
        ("equalizer", "Equalizer"),
        ("history", "Edit history"),
    ] {
        let document = cranamp::winamp::studio::open_document(None).expect("a document");
        open(&document, drawer, "pencil");
        let mut shell = touch(document);
        let texts = visible_texts(&mut shell);
        assert!(
            contains(&texts, marker),
            "{drawer:?} should have opened the panel holding {marker:?}; visible={texts:?}"
        );
    }
}
#[test]
fn the_brushes_do_not_fill_the_drawer_before_their_own_settings() {
    let document = cranamp::winamp::studio::open_document(None).expect("a document");
    open(&document, "tools", "glass");
    let mut shell = touch(document);
    let texts = visible_texts(&mut shell);
    for control in ["Bevel", "Refraction"] {
        assert!(
            contains(&texts, control),
            "{control:?} is the glass lens' own setting; visible={texts:?}"
        );
    }
}
#[test]
fn the_edit_scope_is_reachable_where_there_is_no_sidebar() {
    let document = cranamp::winamp::studio::open_document(None).expect("a document");
    open(&document, "tools", "pencil");
    let mut shell = touch(document);
    click_text(&mut shell, "Close panel");
    click_text(&mut shell, "Canvas state");
    let texts = visible_texts(&mut shell);
    for control in [
        "Paint into",
        "This state",
        "This and later",
        "This and earlier",
        "All states",
    ] {
        assert!(
            contains(&texts, control),
            "{control:?} decides which variants a stroke lands in and has to be \
             pressable here; visible={texts:?}"
        );
    }
}
#[test]
fn stamp_repeat_and_sweep_are_reachable_where_there_is_no_sidebar() {
    let document = cranamp::winamp::studio::open_document(None).expect("a document");
    open(&document, "tools", "stamp");
    let mut shell = touch(document);
    let texts = visible_texts(&mut shell);
    for control in ["Repeat", "Sweep states"] {
        assert!(
            contains(&texts, control),
            "{control:?} is how a hand does what studio_draw's at does; visible={texts:?}"
        );
    }
}
#[test]
fn the_export_format_check_is_reachable_where_there_is_no_sidebar() {
    let document = cranamp::winamp::studio::open_document(None).expect("a document");
    open(&document, "options", "pencil");
    let mut shell = touch(document);
    click_text(&mut shell, "Format check");
    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "Ready for WSZ export")
            || contains(&texts, "Fix these items before WSZ export"),
        "the export format check must be available on a handset; visible={texts:?}"
    );
}

fn click_text(shell: &mut AppShell<HitGraphRenderer>, label: &str) {
    fn find(node: &cranpose_ui::SemanticsNode, label: &str) -> Option<(f32, f32)> {
        if node.hidden {
            return None;
        }
        if node.description.as_deref() == Some(label) {
            return Some((
                node.bounds.x + node.bounds.width / 2.,
                node.bounds.y + node.bounds.height / 2.,
            ));
        }
        if let cranpose_ui::SemanticsRole::Text { value } = &node.role {
            if value.as_str() == label {
                return Some((
                    node.bounds.x + node.bounds.width / 2.,
                    node.bounds.y + node.bounds.height / 2.,
                ));
            }
        }
        node.children.iter().find_map(|child| find(child, label))
    }
    shell.set_semantics_enabled(true);
    let (x, y) = find(shell.semantics_tree().unwrap().root(), label)
        .unwrap_or_else(|| panic!("Missing {label}"));
    assert!(
        x >= 0. && x < WINDOW.0 as f32 && y >= 0. && y < WINDOW.1 as f32,
        "{label} is outside the screen at {x},{y}"
    );
    shell.set_cursor(x, y);
    shell.pointer_pressed();
    pump(shell);
    shell.pointer_released();
    pump(shell);
}

struct PickDocument(Option<cranpose_services::ContentHandle>);
impl cranpose_services::FilePicker for PickDocument {
    fn pick_file(
        &self,
        _: cranpose_services::FilePickerOptions,
    ) -> cranpose_services::PickerFuture<
        Result<Option<cranpose_services::ContentHandle>, cranpose_services::FilePickerError>,
    > {
        let content = self.0.clone();
        Box::pin(async move { Ok(content) })
    }
    fn pick_folder(
        &self,
        _: cranpose_services::FilePickerOptions,
    ) -> cranpose_services::PickerFuture<
        Result<Option<cranpose_services::ContentFolderRef>, cranpose_services::FilePickerError>,
    > {
        Box::pin(async { Ok(None) })
    }
}

#[test]
fn phone_navigation_opens_a_file_and_returns_to_canvas() {
    let source = cranamp::winamp::studio::open_document(None).unwrap();
    let bytes = source.lock().unwrap().project_bytes().unwrap();
    for (name, bytes) in [
        ("test.CSTUDIO", bytes),
        (
            "test.wsz",
            include_bytes!("../assets/skins/Catamp Moon Garden.wsz").to_vec(),
        ),
        (
            "test.zip",
            include_bytes!("../assets/skins/Catamp Moon Garden.wsz").to_vec(),
        ),
    ] {
        let picked = cranpose_services::BytesContent::named(name, bytes).handle();
        cranpose_services::set_platform_file_picker(std::rc::Rc::new(PickDocument(Some(picked))));
        let document = cranamp::winamp::studio::open_document(None).unwrap();
        document
            .lock()
            .unwrap()
            .state(serde_json::json!({"drawer":"none", "zoom":8, "panel":"atlas"}))
            .unwrap();
        let mut shell = touch(document.clone());
        click_text(&mut shell, "Open skin or project");
        pump(&mut shell);
        assert_eq!(document.lock().unwrap().view.panel, "canvas");
        assert_eq!(document.lock().unwrap().view.zoom, 1);
        click_text(&mut shell, "Joined canvas");
        click_text(&mut shell, "Edit Player");
        assert_eq!(document.lock().unwrap().view.panel, "main");
        click_text(&mut shell, "Brush colour");
        assert!(contains(&visible_texts(&mut shell), "HEX"));
        cranpose_services::clear_platform_file_picker();
    }
}

#[test]
fn cancelled_and_invalid_picks_preserve_the_document() {
    for content in [
        None,
        Some(cranpose_services::BytesContent::named("bad.wsz", vec![0; 32]).handle()),
    ] {
        cranpose_services::set_platform_file_picker(std::rc::Rc::new(PickDocument(content)));
        let document = cranamp::winamp::studio::open_document(None).unwrap();
        document
            .lock()
            .unwrap()
            .state(serde_json::json!({"drawer":"none"}))
            .unwrap();
        let before = document.lock().unwrap().project_bytes().unwrap();
        let mut shell = touch(document.clone());
        click_text(&mut shell, "Open skin or project");
        assert_eq!(document.lock().unwrap().project_bytes().unwrap(), before);
        click_text(&mut shell, "Open working copy");
        click_text(&mut shell, "Brush colour");
        assert!(contains(&visible_texts(&mut shell), "HEX"));
        click_text(&mut shell, "Close panel");
        click_text(&mut shell, "Back");
        click_text(&mut shell, "Open working copy");
        click_text(&mut shell, "Brush size value");
        assert_eq!(document.lock().unwrap().project_bytes().unwrap(), before);
        cranpose_services::clear_platform_file_picker();
    }
}
