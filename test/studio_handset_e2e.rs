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
    for control in ["WORD", "FACE", "5×7", "4×5 small caps", "LETTER SPACING"] {
        assert!(
            contains(&texts, control),
            "{control:?} is a text brush setting and has to be pressable here; visible={texts:?}"
        );
    }
}
#[test]
fn a_drawer_named_on_the_document_is_the_drawer_this_layout_opens() {
    for (drawer, marker) in [
        ("tools", "DRAWING TOOLS"),
        ("layers", "PAINTING LAYERS"),
        ("targets", "SPRITE TARGETS"),
        ("options", "CLASSIC WINAMP SKIN"),
        ("equalizer", "EQ WORKBENCH"),
        ("history", "EDIT HISTORY"),
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
    for control in ["BEVEL", "REFRACTION"] {
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
    let texts = visible_texts(&mut shell);
    for control in [
        "EDIT SCOPE",
        "This state only",
        "This state onward",
        "Up to this state",
        "All sprite states",
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
    for control in [
        "STAMP COPIES ALONG A DRAG",
        "5×",
        "11×",
        "Every copy in the edit scope",
    ] {
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
    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "EXPORT FORMAT CHECK"),
        "the export format check must be available on a handset; visible={texts:?}"
    );
}
