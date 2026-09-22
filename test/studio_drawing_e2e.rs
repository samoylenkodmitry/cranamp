#![cfg(not(target_arch = "wasm32"))]
mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_core::location_key;
const WINDOW: (u32, u32) = (1160, 850);
// At this viewport the EQ workbench wraps onto a second panel-toolbar row.
const CANVAS_ORIGIN: (f32, f32) = (230.0, 166.0);
const INK_SWATCH: (f32, f32) = (89.0, 344.0);
const COLOUR_PICKER: (f32, f32) = (114.0, 379.0);
const PICK_PIXEL: (f32, f32) = (157.0, 173.0);
const NEW_BLANK: (f32, f32) = (1093.0, 55.0);
const PANEL_ROW: [(f32, f32); 9] = [
    (79.0, 93.0),
    (210.0, 93.0),
    (342.0, 93.0),
    (481.0, 93.0),
    (618.0, 93.0),
    (738.0, 93.0),
    (852.0, 93.0),
    (970.0, 93.0),
    (82.0, 131.0),
];
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
fn painted(shell: &mut AppShell<HitGraphRenderer>) -> bool {
    contains(&visible_texts(shell), "pencil · canvas")
}
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
#[test]
fn every_window_s_options_are_reachable_without_scrolling_the_canvas() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);
    click(&mut shell, PANEL_ROW[7]);
    let texts = visible_texts(&mut shell);
    for option in [
        "CLASSIC WINAMP SKIN",
        "EXPORT FORMAT CHECK",
        "WINDOW CUTOUTS",
        "PLEDIT.TXT",
        "VISCOLOR.TXT",
    ] {
        assert!(
            contains(&texts, option),
            "{option:?} belongs to a different window but must still be here; visible={texts:?}"
        );
    }
}
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
fn count(texts: &[String], needle: &str) -> usize {
    texts.iter().filter(|text| *text == needle).count()
}
#[test]
fn every_painting_layer_is_listed_and_no_row_sits_under_delete() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    for plane in 1..=7 {
        document
            .lock()
            .unwrap()
            .paint_layer_command(
                &serde_json::json!({"action":"add","name":format!("plane {plane}")}),
                "test",
            )
            .expect("a painting plane");
    }
    let mut shell = studio(document);
    click(&mut shell, PANEL_ROW[1]);
    let texts = visible_texts(&mut shell);
    let planes = count(&texts, "Shown") + count(&texts, "Hidden");
    assert_eq!(
        planes, 7,
        "seven painting layers, and all of them belong in the list"
    );
    click(&mut shell, (844.0, 507.0));
    let texts = visible_texts(&mut shell);
    assert_eq!(
        count(&texts, "Shown") + count(&texts, "Hidden"),
        7,
        "a click that deep in the list selects a layer; it must not delete one"
    );
}
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
#[test]
fn the_state_sheet_asks_for_a_sprite_instead_of_picking_one() {
    let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
    let mut shell = studio(document);
    click(&mut shell, (589.0, 55.0));
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
#[test]
fn the_same_editor_lays_out_on_a_handset() {
    for size in [(393u32, 780u32), (820, 1180), (1160, 850), (1680, 1050)] {
        let document = cranamp::winamp::studio::open_document(None).expect("bundled document");
        let root_key = location_key(file!(), line!(), column!());
        let mut shell = AppShell::new(HitGraphRenderer::default(), root_key, {
            let document = document.clone();
            move || cranamp::winamp::studio::SkinStudio(document.clone(), None)
        });
        shell.set_buffer_size(size.0, size.1);
        shell.set_viewport(size.0 as f32, size.1 as f32);
        pump(&mut shell);
        let texts = visible_texts(&mut shell);
        for button in [
            "Drawing tools",
            "Painting layers",
            "Sprite targets",
            "Sprite rectangles",
            "Skin atlases",
            "Edit history",
            "Pixel study",
            "Skin options",
            "Undo",
            "New blank",
        ] {
            assert!(
                contains(&texts, button),
                "{button:?} belongs to this editor at every size; at {size:?} \
                 visible={texts:?}"
            );
        }
    }
}
