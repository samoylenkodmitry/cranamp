//! Framework-level end-to-end coverage for the desktop Skin Studio, driven
//! through cranpose's real hit-dispatch pipeline: synthesized pointer drags and
//! clicks on the actual editor composable, asserting on what a real gesture
//! produces rather than on `Document` methods directly.
//!
//! These guard the things a reader of the editor has to be able to trust:
//! that the tools are on screen when it opens, that the tool column never eats
//! a stroke, that a switch's label says which way round it is, that the one
//! button able to destroy work asks first, and that no control hides itself
//! based on where the canvas happens to be scrolled.

#![cfg(not(target_arch = "wasm32"))]

mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;

/// The smallest window the editor lays out for, so the assertions below also
/// prove the layout fits at its minimum rather than only at the default size.
const WINDOW: (u32, u32) = (1160, 850);
/// The canvas at that window size: origin (230,128), 508 x 620, with the tool
/// column docked to its right. The skin is larger than the canvas at the
/// default 2x, so it sits at the canvas origin and a source pixel is at
/// `(230 + 2x, 132 + 2y)`.
const CANVAS_ORIGIN: (f32, f32) = (230.0, 128.0);
/// The near-black preset swatch (#09121d): the 6th of a 4-wide grid at (32,282),
/// 32x25 cells at (32,298) spaced 41x34. Deliberately NOT the 8th swatch, #ff00ff -- that is
/// the classic transparency key, which the renderer drops, so a stroke made with
/// it is recorded and invisible.
const INK_SWATCH: (f32, f32) = (89.0, 344.0);
/// "Colour picker…" in the sidebar (30,364,168,30), centred.
const COLOUR_PICKER: (f32, f32) = (114.0, 379.0);
/// "Pick pixel" in the sidebar (115,158,84,30), centred.
const PICK_PIXEL: (f32, f32) = (157.0, 173.0);
/// "New blank" at the right of the action row (1046,40,94,30), centred.
const NEW_BLANK: (f32, f32) = (1093.0, 55.0);
/// The eight drawer buttons of the panel row, centred. Laid out left to right
/// from x=20 with an 8pt gap, exactly as `SkinStudio` lays them out.
const PANEL_ROW: [(f32, f32); 8] = [
    (79.0, 93.0),  // Drawing tools
    (210.0, 93.0), // Painting layers
    (342.0, 93.0), // Sprite targets
    (481.0, 93.0), // Sprite rectangles
    (618.0, 93.0), // Skin atlases
    (738.0, 93.0), // Edit history
    (852.0, 93.0), // Pixel study
    (970.0, 93.0), // Skin options
];
/// A source pixel of the main window that is opaque artwork.
const INK_SOURCE: (u32, u32) = (140, 106);

fn canvas_point(source: (u32, u32)) -> (f32, f32) {
    (
        CANVAS_ORIGIN.0 + source.0 as f32 * 2.0,
        CANVAS_ORIGIN.1 + source.1 as f32 * 2.0,
    )
}

fn contains(texts: &[String], needle: &str) -> bool {
    texts.iter().any(|text| text.contains(needle))
}

fn studio(document: cranamp::winamp::studio::SharedDocument) -> AppShell<HitGraphRenderer> {
    let root_key = location_key(file!(), line!(), column!());
    let mut shell = AppShell::new(HitGraphRenderer::default(), root_key, move || {
        cranamp::winamp::studio::SkinStudio(document.clone(), None)
    });
    shell.set_buffer_size(WINDOW.0, WINDOW.1);
    shell.set_viewport(WINDOW.0 as f32, WINDOW.1 as f32);
    pump(&mut shell);
    shell
}

fn click(shell: &mut AppShell<HitGraphRenderer>, (x, y): (f32, f32)) {
    shell.set_cursor(x, y);
    shell.pointer_pressed();
    pump(shell);
    shell.pointer_released();
    pump(shell);
}

/// A press, two moves and a release -- the shape of a real freehand stroke.
fn drag(shell: &mut AppShell<HitGraphRenderer>, (x, y): (f32, f32)) {
    shell.set_cursor(x, y);
    shell.pointer_pressed();
    pump(shell);
    for step in 1..=2 {
        shell.set_cursor(x + step as f32 * 6.0, y + step as f32 * 6.0);
        pump(shell);
    }
    shell.pointer_released();
    pump(shell);
}

/// The status line reads "<brush> · <panel> · <target>" once a stroke is
/// recorded, so it is an unambiguous marker that the drag actually painted.
/// The panel is always the joined whole skin, never a single window.
fn painted(shell: &mut AppShell<HitGraphRenderer>) -> bool {
    contains(&visible_texts(shell), "pencil · canvas")
}

/// An editor that opens with nothing but a canvas has to be searched before it
/// can be used. The tool column is docked beside the canvas rather than laid
/// over it, so it can simply be there from the start.
#[test]
fn the_editor_opens_on_the_whole_skin_with_its_tools_already_on_screen() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "WHOLE SKIN"),
        "the editor should open on the joined canvas, not one window; visible={texts:?}"
    );
    assert!(
        contains(&texts, "DRAWING TOOLS") && contains(&texts, "STROKE WIDTH"),
        "the drawing tools should be on screen before anything is pressed; visible={texts:?}"
    );
}

/// Every switch in the editor names the state it is in, never the state that
/// pressing it would move to. Mixing the two conventions -- which the editor
/// used to do, in the same row -- makes a switch unreadable without pressing it.
#[test]
fn every_switch_names_the_state_it_is_in_rather_than_the_one_it_would_reach() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "Canvas editing"),
        "at rest the editor is editing the canvas and should say so; visible={texts:?}"
    );
    assert!(
        !contains(&texts, "Player preview"),
        "\"Player preview\" is a state, not an instruction, so it must not \
         show while the editor is editing; visible={texts:?}"
    );
    assert!(
        !contains(&texts, "Sprite state sheet"),
        "the state sheet is off, so its name must not be on a button; visible={texts:?}"
    );
}

/// A stroke that runs from the main window into the equalizer is one stroke on
/// one surface. The windows are sections of the joined canvas, not modes, so
/// the editor has to route the pixels to each window's own sheets itself.
#[test]
fn a_stroke_across_the_window_seam_paints() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);
    let seam_y = CANVAS_ORIGIN.1 + 232.0;

    click(&mut shell, INK_SWATCH);
    shell.set_cursor(canvas_point(INK_SOURCE).0, seam_y - 24.0);
    shell.pointer_pressed();
    pump(&mut shell);
    for step in 1..=4 {
        shell.set_cursor(
            canvas_point(INK_SOURCE).0,
            seam_y - 24.0 + step as f32 * 12.0,
        );
        pump(&mut shell);
    }
    shell.pointer_released();
    pump(&mut shell);

    assert!(
        painted(&mut shell),
        "a stroke crossing the main/equalizer seam should paint; visible={:?}",
        visible_texts(&mut shell)
    );
}

#[test]
fn a_drag_on_the_canvas_paints_a_pixel_the_viewer_can_see() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let before = document
        .rendered_pixel(INK_SOURCE.0, INK_SOURCE.1)
        .expect("canvas pixel");
    let mut shell = studio(document.clone());

    assert!(
        !painted(&mut shell),
        "nothing is painted before the first drag"
    );
    click(&mut shell, INK_SWATCH);
    drag(&mut shell, canvas_point(INK_SOURCE));

    assert!(
        painted(&mut shell),
        "dragging on the canvas should record a stroke; visible={:?}",
        visible_texts(&mut shell)
    );
    let after = document
        .rendered_pixel(INK_SOURCE.0, INK_SOURCE.1)
        .expect("canvas pixel");
    assert_ne!(
        before, after,
        "the stroke has to change the picture, not just the atlas behind it"
    );
}

/// The tool column is docked beside the canvas at every window size the desktop
/// layout accepts, so no panel can swallow a stroke the way the old overlaid
/// drawer did. Every drawer in turn, with the brush over the same artwork.
#[test]
fn no_open_panel_ever_covers_the_canvas() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document.clone());
    click(&mut shell, INK_SWATCH);

    for (index, button) in PANEL_ROW.iter().enumerate() {
        let source = (100 + index as u32 * 10, INK_SOURCE.1);
        let before = document
            .rendered_pixel(source.0, source.1)
            .expect("canvas pixel");
        click(&mut shell, *button);
        drag(&mut shell, canvas_point(source));
        let after = document
            .rendered_pixel(source.0, source.1)
            .expect("canvas pixel");
        assert_ne!(
            before,
            after,
            "panel {index} must not stop the canvas painting; visible={:?}",
            visible_texts(&mut shell)
        );
    }
}

#[test]
fn the_colour_picker_opens_with_a_field_and_the_skin_palette() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    click(&mut shell, COLOUR_PICKER);
    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "COLOUR PICKER") && contains(&texts, "IN THIS SKIN"),
        "the picker should show a colour field and the skin's own palette; visible={texts:?}"
    );

    drag(&mut shell, canvas_point(INK_SOURCE));
    assert!(
        painted(&mut shell),
        "the canvas stays drawable with the picker open; visible={:?}",
        visible_texts(&mut shell)
    );
}

/// An eyedropper that stays armed after it has been used swallows the next
/// stroke. One sample, then back to the pencil, the way every other pixel
/// editor behaves.
#[test]
fn picking_a_pixel_hands_the_colour_back_to_the_pencil() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    click(&mut shell, PICK_PIXEL);
    assert!(
        contains(
            &visible_texts(&mut shell),
            "Click a pixel to take its colour"
        ),
        "the picker should say what it is waiting for"
    );

    click(&mut shell, canvas_point(INK_SOURCE));
    assert!(
        contains(&visible_texts(&mut shell), "Drag to paint"),
        "one sample returns the pencil; visible={:?}",
        visible_texts(&mut shell)
    );

    drag(&mut shell, canvas_point((150, 100)));
    assert!(
        painted(&mut shell),
        "and the next drag paints instead of sampling again; visible={:?}",
        visible_texts(&mut shell)
    );
}

/// The only button that can destroy unexported artwork refuses the first time
/// and says so on itself, rather than failing silently into the status line
/// with no way to proceed.
#[test]
fn new_blank_will_not_discard_unexported_work_without_a_second_press() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document.clone());
    click(&mut shell, INK_SWATCH);
    drag(&mut shell, canvas_point(INK_SOURCE));
    let painted_pixel = document
        .rendered_pixel(INK_SOURCE.0, INK_SOURCE.1)
        .expect("canvas pixel");

    click(&mut shell, NEW_BLANK);
    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "Discard & start blank"),
        "the refusal has to offer a way through, on the button itself; visible={texts:?}"
    );
    assert!(
        contains(&texts, "Keep editing"),
        "and a way out beside it; visible={texts:?}"
    );
    assert_eq!(
        document.rendered_pixel(INK_SOURCE.0, INK_SOURCE.1),
        Some(painted_pixel),
        "the first press must not have thrown the artwork away"
    );

    click(&mut shell, NEW_BLANK);
    assert_ne!(
        document.rendered_pixel(INK_SOURCE.0, INK_SOURCE.1),
        Some(painted_pixel),
        "the second press is the one that starts over"
    );
}

/// The per-window options used to appear and disappear from the toolbar as the
/// canvas scrolled past the window they belonged to, so a control could only be
/// found by already knowing where to scroll. They are all in one panel now.
#[test]
fn every_window_s_options_are_reachable_without_scrolling_the_canvas() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    click(&mut shell, PANEL_ROW[7]);
    let texts = visible_texts(&mut shell);
    for option in ["SKIN OPTIONS", "Playlist", "Equalizer", "Visualizer"] {
        assert!(
            contains(&texts, option),
            "{option:?} belongs to a different window but must still be here; visible={texts:?}"
        );
    }
}

/// Aiming at a native pixel is guesswork without being told which one is under
/// the pointer.
#[test]
fn the_pointer_position_is_reported_in_the_skin_s_own_pixels() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    let (x, y) = canvas_point(INK_SOURCE);
    shell.set_cursor(x, y);
    pump(&mut shell);

    let texts = visible_texts(&mut shell);
    assert!(
        contains(
            &texts,
            &format!("pointer  {} , {}", INK_SOURCE.0, INK_SOURCE.1)
        ),
        "the pointer readout should name the source pixel under it; visible={texts:?}"
    );
}

/// cranpose delivers key events only to a focused text field, so the editor has
/// no shortcuts to offer; every way of getting around the canvas has to be a
/// gesture the pointer alone can make. These are those gestures.
#[test]
fn the_wheel_scrolls_the_canvas_and_ctrl_wheel_zooms_at_the_pointer() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);
    let (x, y) = canvas_point(INK_SOURCE);

    shell.set_cursor(x, y);
    pump(&mut shell);
    assert!(
        contains(
            &visible_texts(&mut shell),
            &format!("pointer  {} , {}", INK_SOURCE.0, INK_SOURCE.1)
        ),
        "the readout should start on the pixel the pointer is over"
    );

    // One wheel notch downward brings lower rows of the skin under the pointer.
    shell.pointer_scrolled(0.0, -40.0);
    pump(&mut shell);
    shell.set_cursor(x, y + 1.0);
    pump(&mut shell);
    shell.set_cursor(x, y);
    pump(&mut shell);
    assert!(
        contains(
            &visible_texts(&mut shell),
            &format!("pointer  {} , {}", INK_SOURCE.0, INK_SOURCE.1 + 20)
        ),
        "the wheel should move the canvas by the notch divided by the zoom; visible={:?}",
        visible_texts(&mut shell)
    );

    // And a ctrl+wheel notch, which the shell converts into a zoom gesture,
    // magnifies without moving the pixel that was under the pointer.
    shell.pointer_zoomed(1.2);
    pump(&mut shell);
    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "3×"),
        "ctrl+wheel should step the zoom; visible={texts:?}"
    );
    shell.set_cursor(x, y + 1.0);
    pump(&mut shell);
    shell.set_cursor(x, y);
    pump(&mut shell);
    assert!(
        contains(
            &visible_texts(&mut shell),
            &format!("pointer  {} , {}", INK_SOURCE.0, INK_SOURCE.1 + 20)
        ),
        "zooming about the pointer must keep the same pixel under it; visible={:?}",
        visible_texts(&mut shell)
    );
}

/// Counts how many times `needle` appears among the editor's texts.
fn count(texts: &[String], needle: &str) -> usize {
    texts.iter().filter(|text| *text == needle).count()
}

/// The controls for the chosen plane used to be pinned at a fixed offset, which
/// cut the list to four rows in a seven-plane document with nothing to say the
/// other three existed -- and put Delete where the fifth row should have been.
#[test]
fn every_painting_layer_is_listed_and_no_row_sits_under_delete() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    click(&mut shell, PANEL_ROW[1]);
    let texts = visible_texts(&mut shell);
    let planes = count(&texts, "Shown") + count(&texts, "Hidden");
    assert_eq!(
        planes, 7,
        "the bundled skin has seven painting layers and all of them belong in the list"
    );

    // The seventh row, at the depth the old pinned Delete button occupied.
    click(&mut shell, (844.0, 507.0));
    let texts = visible_texts(&mut shell);
    assert_eq!(
        count(&texts, "Shown") + count(&texts, "Hidden"),
        7,
        "a click that deep in the list selects a layer; it must not delete one"
    );
}

/// Undo and Redo used to look identical whether or not there was anything to
/// undo, and never said how much there was.
#[test]
fn undo_is_unavailable_until_there_is_something_to_undo() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    assert!(
        !contains(&visible_texts(&mut shell), "Undo 1"),
        "nothing has been done yet, so Undo counts nothing"
    );
    click(&mut shell, INK_SWATCH);
    drag(&mut shell, canvas_point(INK_SOURCE));
    assert!(
        contains(&visible_texts(&mut shell), "Undo 1"),
        "one stroke is one step back; visible={:?}",
        visible_texts(&mut shell)
    );
}

/// Every slider a classic skin has, reachable from one row. Which of them the
/// row offered used to depend on where the canvas happened to be scrolled.
#[test]
fn every_slider_state_is_reachable_without_scrolling_the_canvas() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    let texts = visible_texts(&mut shell);
    for slider in ["volume", "balance", "position", "eq", "scroll"] {
        assert!(
            contains(&texts, slider),
            "{slider:?} belongs to one of the three windows but must always be here; \
             visible={texts:?}"
        );
    }
}

/// Both modes that are not drawing surfaces put their brushes away. Preview used
/// to leave a live, completely inert sidebar and tool panel on screen.
#[test]
fn the_mode_that_cannot_paint_puts_the_drawing_tools_away() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);
    click(&mut shell, (256.0, 55.0));

    let texts = visible_texts(&mut shell);
    assert!(
        !contains(&texts, "STROKE WIDTH"),
        "the brush controls must go away when they cannot be used; visible={texts:?}"
    );
    assert!(
        !contains(&texts, "DRAWING TOOLS"),
        "and so must the panel of them; visible={texts:?}"
    );
    assert!(
        contains(&texts, "Back to the canvas"),
        "and there has to be a way back; visible={texts:?}"
    );
}

/// A state sheet is one sprite's variants. With nothing chosen it used to show
/// an arbitrary sprite, rendered as flat magenta because it copied the classic
/// transparency key verbatim.
#[test]
fn the_state_sheet_asks_for_a_sprite_instead_of_picking_one() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);

    click(&mut shell, (409.0, 55.0));
    let texts = visible_texts(&mut shell);
    assert!(
        contains(&texts, "Choose one sprite in Sprite targets"),
        "it should say what it needs; visible={texts:?}"
    );
    assert!(
        contains(&texts, "SPRITE TARGETS") && contains(&texts, "Solo"),
        "and open the panel that provides it; visible={texts:?}"
    );
    assert!(
        !contains(&texts, "SPRITE STATE SHEET"),
        "without a sprite it must not enter the mode at all; visible={texts:?}"
    );
}

/// Sprite rectangles used to be drawn into the artwork's own pixels -- every
/// cell at once, with hairlines that grew with the zoom, burying the picture
/// they were meant to point at. Now the editor outlines the one being pointed
/// at, over the canvas, and names it.
#[test]
fn the_sprite_under_the_pointer_is_outlined_and_named() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let before = document
        .rendered_pixel(INK_SOURCE.0, INK_SOURCE.1)
        .expect("canvas pixel");
    let mut shell = studio(document.clone());

    click(&mut shell, PANEL_ROW[3]);
    assert_eq!(
        document.rendered_pixel(INK_SOURCE.0, INK_SOURCE.1),
        Some(before),
        "turning the outline on must not change one pixel of the picture"
    );

    let (x, y) = canvas_point(INK_SOURCE);
    shell.set_cursor(x, y);
    pump(&mut shell);
    let texts = visible_texts(&mut shell);
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("main.") && t.contains(" × ")),
        "the sprite under the pointer should be named with its size; visible={texts:?}"
    );
}
