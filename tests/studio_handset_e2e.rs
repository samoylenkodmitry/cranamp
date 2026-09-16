//! End-to-end coverage for the Skin Studio on a handset-sized surface,
//! driven through cranpose's real composition.
//!
//! It is the same composable a desktop window gets: these assert that the
//! panels a narrow surface opens are the same panels, with the same controls
//! in them, rather than a reduced second editor.
//!
//! The rule this layout has to keep is that anything the drawing engine grows
//! appears here too: Android and the web are the two platforms with no MCP to
//! reach a capability with instead, so a setting that exists only in the
//! desktop panel does not exist for anybody drawing there.

#![cfg(not(target_arch = "wasm32"))]

mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;

/// A handset: narrower than the 1140x820 the desktop layout needs.
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

/// A word is set in a face, at a scale, and at a letter spacing, and spacing is
/// the one of the three that decides whether it fits its cell: `STEREO` in the
/// 4x5 face is 29 pixels of ink at spacing 1 and the stereo lamp is 29 pixels
/// wide. It was a `text` operation field from the day the operation landed, so
/// it was reachable from a script and from nothing a finger could press.
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

/// Every drawer is a field of the document rather than of each layout's own
/// state, so a caller with no pointer -- MCP, or a test -- can open one, and
/// the panel that opens is the one it named.
#[test]
fn a_drawer_named_on_the_document_is_the_drawer_this_layout_opens() {
    for (drawer, marker) in [
        ("tools", "DRAWING TOOLS"),
        ("layers", "PAINTING LAYERS"),
        ("targets", "SPRITE TARGETS"),
        ("options", "SKIN OPTIONS"),
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

/// Ten brushes one to a row pushed every setting a brush has below the fold,
/// where the only way to find out they existed was to scroll past all ten.
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
