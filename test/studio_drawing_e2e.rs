#![cfg(not(target_arch = "wasm32"))]
mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranamp::winamp::studio::{open_document, SharedDocument, SkinStudio};
use cranpose_app_shell::{AppShell, PointerSource};
use cranpose_core::location_key;
use cranpose_ui::SemanticsNode;
use cranpose_ui_graphics::Rect;

fn bounds(shell: &mut AppShell<HitGraphRenderer>, label: &str) -> Rect {
    fn find(node: &SemanticsNode, label: &str, pass: u8) -> Option<Rect> {
        if node.hidden {
            return None;
        }
        let matches = match pass {
            0 => node.description.as_deref() == Some(label),
            1 => node
                .description
                .as_deref()
                .is_some_and(|s| s.starts_with(label)),
            _ => {
                matches!(&node.role,cranpose_ui::SemanticsRole::Text {value} if value.as_str()==label)
            }
        };
        if matches {
            return Some(node.bounds);
        }
        node.children.iter().find_map(|n| find(n, label, pass))
    }
    shell.set_semantics_enabled(true);
    (0..3)
        .find_map(|pass| {
            find(
                shell.semantics_tree().expect("semantics").root(),
                label,
                pass,
            )
        })
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", visible_texts(shell)))
}
fn click(shell: &mut AppShell<HitGraphRenderer>, label: &str) {
    let rect = bounds(shell, label);
    shell.set_pointer_source(PointerSource::Mouse);
    shell.set_cursor(rect.x + rect.width / 2., rect.y + rect.height / 2.);
    shell.pointer_pressed();
    pump(shell);
    shell.pointer_released();
    pump(shell);
}
fn setup(width: u32, height: u32) -> (SharedDocument, AppShell<HitGraphRenderer>) {
    let document = open_document(None).expect("skin");
    document.lock().unwrap().state(serde_json::json!({"drawer":"none","zoom":1,"panel":"canvas","color":"#fa1133","brush":"pencil"})).unwrap();
    let root = document.clone();
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        location_key(file!(), line!(), column!()),
        move || SkinStudio(root.clone(), None),
    );
    shell.set_buffer_size(width, height);
    shell.set_viewport(width as f32, height as f32);
    pump(&mut shell);
    click(&mut shell, "Open working copy");
    (document, shell)
}
fn at(shell: &mut AppShell<HitGraphRenderer>, point: [u32; 2], _zoom: f32) -> (f32, f32) {
    let r = bounds(shell, "Skin canvas.");
    let zoom = visible_texts(shell)
        .iter()
        .find_map(|text| {
            text.strip_prefix("Skin canvas. Zoom ")
                .and_then(|s| s.split('%').next())
                .and_then(|s| s.parse::<f32>().ok())
        })
        .expect("canvas zoom")
        / 100.;
    (
        r.x + ((r.width - 275. * zoom) / 2.).max(0.) + (point[0] as f32 + 0.5) * zoom,
        r.y + ((r.height - 377. * zoom) / 2.).max(0.) + (point[1] as f32 + 0.5) * zoom,
    )
}
fn stroke(shell: &mut AppShell<HitGraphRenderer>, p: (f32, f32), q: (f32, f32)) {
    shell.set_cursor(p.0, p.1);
    shell.pointer_pressed();
    pump(shell);
    shell.set_cursor(q.0, q.1);
    pump(shell);
    shell.pointer_released();
    pump(shell);
}
#[test]
fn paint_and_undo_work_on_every_width_class() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 900);
        let p = [140, 106];
        let before = doc.rendered_pixel(p[0], p[1]).unwrap();
        let position = at(&mut shell, p, 1.);
        stroke(&mut shell, position, (position.0 + 8., position.1));
        assert_eq!(
            doc.rendered_pixel(p[0], p[1]),
            Some([250, 17, 51, 255]),
            "width {width}"
        );
        click(&mut shell, "Undo");
        assert_eq!(doc.rendered_pixel(p[0], p[1]), Some(before));
        if width < 380 {
            click(&mut shell, "Project actions");
        }
        click(&mut shell, "Redo");
        if width < 380 {
            click(&mut shell, "Close panel");
        }
        assert_eq!(doc.rendered_pixel(p[0], p[1]), Some([250, 17, 51, 255]));
    }
}
#[test]
fn stroke_across_a_window_seam_remains_continuous() {
    let (doc, mut shell) = setup(1100, 900);
    let p = at(&mut shell, [140, 106], 1.);
    let q = at(&mut shell, [140, 126], 1.);
    stroke(&mut shell, p, q);
    for y in [106, 115, 116, 126] {
        assert_eq!(
            doc.rendered_pixel(140, y),
            Some([250, 17, 51, 255]),
            "row {y}"
        );
    }
}
#[test]
fn two_fingers_cancel_the_tentative_stroke_and_never_paint() {
    let (doc, mut shell) = setup(430, 900);
    let p = at(&mut shell, [140, 106], 1.);
    let before = doc.lock().unwrap().render();
    shell.set_pointer_source(PointerSource::Touch);
    shell.set_cursor(p.0, p.1);
    shell.pointer_pressed();
    pump(&mut shell);
    shell.secondary_pointer_pressed(7, p.0 + 60., p.1, Some(10));
    pump(&mut shell);
    shell.secondary_pointer_moved(7, p.0 + 90., p.1 + 20., Some(20));
    pump(&mut shell);
    shell.secondary_pointer_released(7, p.0 + 90., p.1 + 20., Some(30));
    pump(&mut shell);
    shell.pointer_released();
    pump(&mut shell);
    assert!(
        doc.lock().unwrap().render() == before,
        "pinch must roll back the first touch mark"
    );
    assert!(
        !visible_texts(&mut shell).iter().any(|s| s == "100%"),
        "pinch changes the view scale"
    );
}
#[test]
fn stylus_hover_shows_feedback_and_does_not_paint() {
    let (doc, mut shell) = setup(1100, 900);
    let before = doc.lock().unwrap().render();
    let r = bounds(&mut shell, "Hue");
    shell.set_pointer_source(PointerSource::Stylus);
    shell.set_cursor(r.x + 24., r.y + 24.);
    pump(&mut shell);
    assert!(matches!(
        shell.take_pointer_icon_change(),
        Some(cranpose_ui::PointerIcon::Custom(_))
    ));
    assert!(!visible_texts(&mut shell)
        .iter()
        .any(|s| s.contains("Drag to choose hue")));
    std::thread::sleep(std::time::Duration::from_millis(500));
    pump(&mut shell);
    assert_eq!(
        visible_texts(&mut shell)
            .iter()
            .filter(|s| s.contains("Drag to choose hue"))
            .count(),
        1
    );
    assert!(
        !visible_texts(&mut shell).contains(&doc.lock().unwrap().message),
        "routine status must not repeat the hover hint"
    );
    let p = at(&mut shell, [140, 106], 1.);
    shell.set_cursor(p.0, p.1);
    pump(&mut shell);
    assert!(doc.lock().unwrap().render() == before);
    shell.cancel_gesture_unless_pressed();
    pump(&mut shell);
    assert!(doc.lock().unwrap().render() == before);
}
#[test]
fn hand_mode_and_canvas_scrollbars_do_not_paint() {
    let (doc, mut shell) = setup(430, 900);
    let before = doc.lock().unwrap().render();
    click(&mut shell, "Zoom in");
    click(&mut shell, "Zoom in");
    assert!(
        doc.lock().unwrap().render() == before,
        "zoom controls must not paint"
    );
    click(&mut shell, "Pan canvas");
    assert!(
        doc.lock().unwrap().render() == before,
        "pan control must not paint"
    );
    let r = bounds(&mut shell, "Skin canvas.");
    stroke(
        &mut shell,
        (r.x + 100., r.y + 200.),
        (r.x + 60., r.y + 140.),
    );
    assert!(
        doc.lock().unwrap().render() == before,
        "pan drag must not paint"
    );
    let bar = bounds(&mut shell, "Canvas vertical scrollbar");
    stroke(
        &mut shell,
        (bar.x + 10., bar.y + 20.),
        (bar.x + 10., bar.y + 100.),
    );
    assert!(doc.lock().unwrap().render() == before);
}
#[test]
fn part_selection_and_preview_keep_the_document_and_undo_history() {
    let (doc, mut shell) = setup(1100, 900);
    let p = at(&mut shell, [140, 106], 1.);
    stroke(&mut shell, p, (p.0 + 8., p.1));
    let painted = doc.lock().unwrap().skin_render();
    click(&mut shell, "Joined canvas");
    click(&mut shell, "Edit Equalizer");
    assert_eq!(doc.lock().unwrap().view.panel, "equalizer");
    assert!(visible_texts(&mut shell).iter().any(|s| s == "Equalizer"));
    click(&mut shell, "Preview");
    assert!(visible_texts(&mut shell).iter().any(|s| s == "Preview"));
    click(&mut shell, "Back");
    assert_eq!(doc.lock().unwrap().view.panel, "equalizer");
    assert!(visible_texts(&mut shell).iter().any(|s| s == "Equalizer"));
    assert!(doc.lock().unwrap().skin_render() == painted);
    click(&mut shell, "Undo");
    assert!(doc.lock().unwrap().skin_render() != painted);
}
#[test]
fn resize_changes_layout_without_losing_document_state() {
    let (doc, mut shell) = setup(430, 900);
    let p = at(&mut shell, [140, 106], 1.);
    stroke(&mut shell, p, (p.0 + 6., p.1));
    let before = doc.lock().unwrap().render();
    for (w, h, label) in [
        (1440, 900, "Desktop"),
        (1100, 900, "Wide tablet"),
        (768, 900, "Tablet"),
        (320, 600, "Small phone"),
        (600, 360, "Tablet"),
        (430, 300, "Phone"),
    ] {
        shell.set_buffer_size(w, h);
        shell.set_viewport(w as f32, h as f32);
        pump(&mut shell);
        assert!(visible_texts(&mut shell)
            .iter()
            .any(|s| s == &format!("Skin Studio {label} layout")));
        assert!(doc.lock().unwrap().render() == before);
        let r = bounds(&mut shell, "Skin canvas.");
        assert!(
            r.width > 0.
                && r.height >= if h < 360 { 48. } else { 96. }
                && r.x + r.width <= w as f32
                && r.y + r.height <= h as f32,
            "canvas at {w}x{h}: {r:?}"
        );
        if h < 500 {
            click(&mut shell, "Workspaces");
            bounds(&mut shell, "Skin");
            click(&mut shell, "Close panel");
        }
    }
}
#[test]
fn a_small_phone_sheet_blocks_painting_beneath_it() {
    let (doc, mut shell) = setup(320, 900);
    let before = doc.lock().unwrap().render();
    click(&mut shell, "Painting layers");
    stroke(&mut shell, (160., 260.), (170., 280.));
    assert!(doc.lock().unwrap().render() == before);
    click(&mut shell, "Close panel");
}

#[test]
fn desktop_workspaces_have_their_own_scopes_and_keep_edits() {
    let (doc, mut shell) = setup(1440, 1000);
    assert!(visible_texts(&mut shell)
        .iter()
        .any(|s| s == "Pencil properties"));
    assert!(!visible_texts(&mut shell).iter().any(|s| s == "100%"));
    let p = at(&mut shell, [140, 106], 1.);
    stroke(&mut shell, p, (p.0 + 10., p.1));
    let painted = doc.lock().unwrap().skin_render();
    click(&mut shell, "Sprites");
    assert_eq!(doc.lock().unwrap().view.layer, "main.play");
    assert!(visible_texts(&mut shell)
        .iter()
        .any(|s| s == "Source bitmaps"));
    click(&mut shell, "Filter sprite names");
    shell.on_paste("main.play");
    pump(&mut shell);
    click(&mut shell, "Sprite main.play");
    assert_eq!(doc.lock().unwrap().view.layer, "main.play");
    let r = bounds(&mut shell, "Skin canvas.");
    let centre = (r.x + r.width / 2., r.y + r.height / 2.);
    stroke(&mut shell, centre, (centre.0 + 5., centre.1));
    assert!(doc.lock().unwrap().skin_render() != painted);
    click(&mut shell, "Undo");
    assert!(doc.lock().unwrap().skin_render() == painted);
    click(&mut shell, "States");
    assert!(visible_texts(&mut shell)
        .iter()
        .any(|s| s == "Sprite states"));
    let r = bounds(&mut shell, "Skin canvas.");
    stroke(&mut shell, (r.x + 80., r.y + 80.), (r.x + 100., r.y + 110.));
    assert!(doc.lock().unwrap().skin_render() == painted);
    click(&mut shell, "Preview");
    assert!(visible_texts(&mut shell)
        .iter()
        .any(|s| s == "Export format"));
    click(&mut shell, "Canvas");
    assert_eq!(doc.lock().unwrap().view.panel, "canvas");
    assert!(doc.lock().unwrap().skin_render() == painted);
    click(&mut shell, "Undo");
    assert!(doc.lock().unwrap().skin_render() != painted);
}

#[test]
fn primary_controls_are_visible_and_clear_of_the_canvas_at_each_size() {
    for (width, height) in [
        (240, 568),
        (320, 568),
        (379, 640),
        (380, 640),
        (599, 640),
        (600, 640),
        (899, 700),
        (900, 700),
        (1279, 800),
        (1280, 800),
        (1920, 1080),
        (430, 820),
        (768, 900),
        (1100, 800),
        (1440, 900),
        (768, 390),
        (1440, 390),
    ] {
        let (_, mut shell) = setup(width, height);
        let canvas = bounds(&mut shell, "Skin canvas.");
        if width < 900 {
            bounds(&mut shell, "Brush colour");
            bounds(&mut shell, "Painting layers");
        }
        for name in [
            "Pencil",
            "Pan canvas",
            "Drawing tools",
            "Brush size",
            "Opacity",
            "Fit canvas",
        ] {
            let r = bounds(&mut shell, name);
            assert!(
                r.x >= 0.
                    && r.y >= 0.
                    && r.x + r.width <= width as f32 + 1.
                    && r.y + r.height <= height as f32 + 1.,
                "{name} outside {width}x{height}: {r:?}"
            );
            assert!(
                r.x + r.width <= canvas.x
                    || r.x >= canvas.x + canvas.width
                    || r.y + r.height <= canvas.y
                    || r.y >= canvas.y + canvas.height,
                "{name} covers the canvas at {width}x{height}"
            );
        }
    }
}

#[test]
fn tools_take_two_taps_and_expose_their_current_settings() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 900);
        if width < 900 {
            click(&mut shell, "Drawing tools");
        }
        click(&mut shell, "Text");
        assert_eq!(
            doc.lock().unwrap().view.brush,
            "text",
            "width {width}; {:?}",
            visible_texts(&mut shell)
        );
        let field = bounds(&mut shell, "Text to paint");
        assert!(field.y + field.height <= 900.);
        click(&mut shell, "Text to paint");
        shell.on_paste("NEW");
        pump(&mut shell);
        assert!(doc.lock().unwrap().view.text.contains("NEW"));
        if width < 900 {
            click(&mut shell, "Drawing tools");
        }
        click(&mut shell, "Curve");
        assert_eq!(doc.lock().unwrap().view.brush, "curve");
        assert!(bounds(&mut shell, "Bend").y < 900.);
        click(&mut shell, "Tool settings");
        assert!(
            visible_texts(&mut shell).iter().any(|s| s == "Stroke"),
            "width {width}: {:?}",
            visible_texts(&mut shell)
        );
    }
}

#[test]
fn sprite_editing_keeps_settings_visible_and_states_reclaims_paint_space() {
    for width in [320, 430, 768, 1100, 1440] {
        let (_, mut shell) = setup(width, 900);
        click(&mut shell, "Sprites");
        let paint = bounds(&mut shell, "Skin canvas.");
        for label in ["Brush size", "Opacity", "Drawing tools"] {
            let r = bounds(&mut shell, label);
            assert!(r.x >= 0. && r.x + r.width <= width as f32 + 1.);
            assert!(r.y >= 0. && r.y + r.height <= 900.);
        }
        click(&mut shell, "States");
        let review = bounds(&mut shell, "Skin canvas.");
        assert!(review.width >= paint.width && review.height >= paint.height);
        assert!(!visible_texts(&mut shell)
            .iter()
            .any(|s| s == "Drawing tools"));
    }
}

fn enter_value(shell: &mut AppShell<HitGraphRenderer>, label: &str, value: &str) {
    click(shell, label);
    shell.on_key_event(&cranpose_ui::KeyEvent::key_down_with_modifiers(
        cranpose_ui::KeyCode::A,
        "a",
        cranpose_ui::Modifiers {
            ctrl: true,
            meta: true,
            ..cranpose_ui::Modifiers::NONE
        },
    ));
    shell.on_paste(value);
    pump(shell);
}

#[test]
fn brush_size_uses_the_whole_slider_and_accepts_exact_values() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 900);
        let r = bounds(&mut shell, "Brush size");
        for (x, expected) in [(r.x + r.width - 1., 32), (r.x, 1)] {
            shell.set_cursor(x, r.y + r.height / 2.);
            shell.pointer_pressed();
            pump(&mut shell);
            shell.pointer_released();
            pump(&mut shell);
            assert_eq!(
                doc.lock().unwrap().view.brush_size,
                expected,
                "width {width}"
            );
        }
        enter_value(&mut shell, "Brush size value", "17");
        assert_eq!(doc.lock().unwrap().view.brush_size, 17, "width {width}");
        enter_value(&mut shell, "Brush size value", "64");
        assert_eq!(doc.lock().unwrap().view.brush_size, 17);
        enter_value(&mut shell, "Brush size value", "32");
        assert_eq!(doc.lock().unwrap().view.brush_size, 32);
    }
}

#[test]
fn exact_colour_and_saved_swatches_can_be_edited() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 1200);
        if width < 900 {
            click(&mut shell, "Brush colour");
        }
        enter_value(&mut shell, "HEX colour", "#123456");
        assert_eq!(doc.lock().unwrap().view.color, "#123456", "width {width}");
        enter_value(&mut shell, "Red channel", "17");
        assert_eq!(doc.lock().unwrap().view.color, "#113456");
        enter_value(&mut shell, "HEX colour", "#xx3456");
        assert_eq!(doc.lock().unwrap().view.color, "#113456");
        click(&mut shell, "Save colour");
        assert_eq!(doc.lock().unwrap().view.swatches.last().unwrap(), "#113456");
        enter_value(&mut shell, "HEX colour", "#abcdef");
        click(&mut shell, "Replace swatch");
        assert_eq!(doc.lock().unwrap().view.swatches.last().unwrap(), "#abcdef");
        click(&mut shell, "Swatch 1:");
        assert_eq!(doc.lock().unwrap().view.color, "#ffffff");
        click(&mut shell, "Remove swatch");
        assert_eq!(doc.lock().unwrap().view.swatches.len(), 8);
        assert_eq!(doc.lock().unwrap().view.swatches[0], "#a7c5d5");
    }
}

#[test]
fn joined_canvas_states_change_in_place_and_preserve_paint() {
    for width in [320, 430, 768, 1100, 1440, 1920] {
        let (doc, mut shell) = setup(width, 1200);
        let p = at(&mut shell, [140, 106], 1.);
        stroke(&mut shell, p, (p.0 + 4., p.1));
        let painted = doc.lock().unwrap().skin_render();
        if width < 1280 {
            click(&mut shell, "Canvas state");
        }
        if width >= 1680 {
            click(&mut shell, "View Inactive · pressed");
            assert!(!doc.lock().unwrap().view.active);
            assert!(doc.lock().unwrap().view.pressed);
            click(&mut shell, "View Active · released");
        } else {
            click(&mut shell, "Active");
            assert!(!doc.lock().unwrap().view.active);
            click(&mut shell, "Released");
            assert!(doc.lock().unwrap().view.pressed);
            click(&mut shell, "Inactive");
            click(&mut shell, "Pressed");
        }
        assert_eq!(doc.lock().unwrap().view.panel, "canvas");
        assert_eq!(doc.lock().unwrap().skin_render(), painted);
        if width >= 1280 {
            click(&mut shell, "Canvas frames");
        }
        click(&mut shell, "All states");
        assert_eq!(doc.lock().unwrap().view.states, "all");
        enter_value(&mut shell, "Volume frame value", "27");
        assert_eq!(doc.lock().unwrap().view.volume, 27);
    }
}

fn buttons(shell: &mut AppShell<HitGraphRenderer>) -> Vec<String> {
    fn walk(node: &SemanticsNode, out: &mut Vec<String>) {
        if node.hidden {
            return;
        }
        // Ignore full-panel input barriers; collect the actual compact click targets.
        if !node.actions.is_empty() && node.bounds.height <= 80. {
            if let Some(label) = &node.description {
                out.push(label.clone());
            }
        }
        for child in &node.children {
            walk(child, out);
        }
    }
    shell.set_semantics_enabled(true);
    let mut out = Vec::new();
    walk(shell.semantics_tree().unwrap().root(), &mut out);
    out
}

#[test]
fn an_action_has_only_one_button_in_each_editing_layout() {
    for width in [320, 430, 768, 1100, 1440] {
        let (_, mut shell) = setup(width, 1000);
        for panel in [None, Some("Drawing tools"), Some("Project actions")] {
            if let Some(label) = panel {
                click(&mut shell, label);
            }
            let labels = buttons(&mut shell);
            assert!(labels.len() > 2, "No button evidence at {width}");
            let mut unique = std::collections::HashSet::new();
            for label in &labels {
                assert!(
                    unique.insert(label),
                    "duplicate {label} at {width}: {labels:?}"
                );
            }
            assert!(
                labels
                    .iter()
                    .filter(|s| matches!(
                        s.as_str(),
                        "Preview" | "Open live preview" | "Live preview"
                    ))
                    .count()
                    <= 1,
                "duplicate preview at {width}"
            );
            if panel.is_some() {
                click(&mut shell, "Close panel");
            }
        }
    }
}

#[test]
fn sprite_state_and_preview_screens_do_not_repeat_navigation() {
    for width in [320, 430, 768, 1100, 1440] {
        let (_, mut shell) = setup(width, 1000);
        click(&mut shell, "Sprites");
        click(&mut shell, "States");
        let labels = buttons(&mut shell);
        assert_eq!(
            labels
                .iter()
                .filter(|s| matches!(s.as_str(), "Sprites" | "Edit sprite"))
                .count(),
            1,
            "width {width}: {labels:?}"
        );
        click(&mut shell, "Preview");
        let labels = buttons(&mut shell);
        assert_eq!(
            labels
                .iter()
                .filter(|s| s.as_str() == "Save and share")
                .count(),
            1,
            "width {width}: {labels:?}"
        );
    }
}

#[test]
fn layer_editing_is_a_complete_workflow_at_every_width() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 1000);
        click(&mut shell, "Painting layers");
        click(&mut shell, "Add layer");
        enter_value(&mut shell, "Layer name", "Ink");
        click(&mut shell, "Rename layer");
        enter_value(&mut shell, "Layer opacity value", "50");
        let info = doc.lock().unwrap().paint_layer_info();
        assert_eq!(info["layers"][0]["name"], "Ink", "width {width}");
        assert_eq!(info["layers"][0]["opacity"], 128);
        click(&mut shell, "Add layer");
        enter_value(&mut shell, "Layer name", "Highlights");
        click(&mut shell, "Rename layer");
        click(&mut shell, "Move layer down");
        let info = doc.lock().unwrap().paint_layer_info();
        assert_eq!(info["layers"][0]["name"], "Highlights");
        click(&mut shell, "Delete layer");
        assert_eq!(
            doc.lock().unwrap().paint_layer_info()["layers"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        click(&mut shell, "Close panel");
        click(&mut shell, "Undo");
        assert_eq!(
            doc.lock().unwrap().paint_layer_info()["layers"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}

#[test]
fn help_opens_a_specific_task_and_returns_without_changing_the_document() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 1000);
        let before = doc.lock().unwrap().skin_render();
        click(&mut shell, "Back");
        click(&mut shell, "Studio help");
        click(&mut shell, "Change what a stroke affects");
        assert!(visible_texts(&mut shell)
            .iter()
            .any(|s| s.contains("Under Paint into")));
        if width < 700 {
            click(&mut shell, "All help topics");
        }
        click(&mut shell, "Copy, flip and stamp pixels");
        assert!(visible_texts(&mut shell)
            .iter()
            .any(|s| s.contains("original pixels stay in place")));
        click(&mut shell, "Back");
        click(&mut shell, "Open working copy");
        assert!(doc.lock().unwrap().skin_render() == before);
        bounds(&mut shell, "Skin canvas.");
    }
}

#[test]
fn selection_leads_directly_to_stamp_and_its_own_transforms() {
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 1000);
        let before = doc.lock().unwrap().skin_render();
        if width < 600 {
            click(&mut shell, "Drawing tools");
        }
        click(&mut shell, "Lift pixels");
        assert!(!visible_texts(&mut shell)
            .iter()
            .any(|s| s == "Brush size" || s == "Opacity"));
        let a = at(&mut shell, [100, 40], 1.);
        let b = at(&mut shell, [112, 48], 1.);
        stroke(&mut shell, a, b);
        assert!(doc.lock().unwrap().skin_render() == before);
        let copied = doc
            .lock()
            .unwrap()
            .cluster
            .clone()
            .expect("captured pixels");
        click(&mut shell, "Use copied pixels");
        assert_eq!(doc.lock().unwrap().view.brush, "stamp");
        assert!(!visible_texts(&mut shell).iter().any(|s| s == "Opacity"));
        click(&mut shell, "Rotate copied pixels");
        assert_eq!(
            doc.lock().unwrap().cluster.as_ref(),
            Some(&image::imageops::rotate90(&copied))
        );
        click(&mut shell, "Flip copied pixels horizontally");
        assert_eq!(
            doc.lock().unwrap().cluster.as_ref(),
            Some(&image::imageops::flip_horizontal(
                &image::imageops::rotate90(&copied)
            ))
        );
        let p = at(&mut shell, [160, 60], 1.);
        stroke(&mut shell, p, (p.0 + 2., p.1));
        assert!(doc.lock().unwrap().skin_render() != before);
        click(&mut shell, "Undo");
        assert!(doc.lock().unwrap().skin_render() == before);
    }
}

#[test]
fn tool_choices_fit_in_short_and_narrow_windows() {
    for (width, height) in [
        (240, 568),
        (320, 568),
        (430, 820),
        (600, 360),
        (768, 390),
        (1100, 800),
        (1920, 1080),
    ] {
        let (_, mut shell) = setup(width, height);
        click(&mut shell, "Drawing tools");
        for label in ["Glass lens", "Taper"] {
            let r = bounds(&mut shell, label);
            assert!(
                r.x >= 0.
                    && r.y >= 0.
                    && r.x + r.width <= width as f32 + 1.
                    && r.y + r.height <= height as f32 - 20.,
                "{label} needs scrolling at {width}x{height}: {r:?}"
            );
        }
        if width < 600 {
            click(&mut shell, "Stamp");
            for label in [
                "Flip copied pixels horizontally",
                "Flip copied pixels vertically",
                "Rotate copied pixels",
            ] {
                let r = bounds(&mut shell, label);
                assert!(r.x >= 0. && r.x + r.width <= width as f32);
            }
        }
    }
}

#[test]
fn quick_palette_changes_paint_without_opening_a_sheet_at_each_width() {
    for width in [320, 488, 768] {
        let (document, mut shell) = setup(width, 820);
        let swatch = document.lock().unwrap().view.swatches[2].clone();
        click(&mut shell, &format!("Quick colour {swatch}"));
        assert_eq!(document.lock().unwrap().view.color, swatch);
        bounds(&mut shell, "Skin canvas.");
        bounds(&mut shell, "Pencil");
    }
}

#[test]
fn pan_replaces_paint_properties_with_navigation() {
    for width in [320, 488, 768, 1100, 1440] {
        let (_document, mut shell) = setup(width, 820);
        click(&mut shell, "Pan canvas");
        let texts = visible_texts(&mut shell);
        bounds(&mut shell, "Canvas zoom");
        assert!(!texts.iter().any(|s| s == "Brush size value"));
        assert!(!texts.iter().any(|s| s == "Brush colour"));
        click(&mut shell, "Pencil");
        bounds(&mut shell, "Brush size value");
    }
}

#[test]
fn canvas_hover_uses_the_selected_instrument_cursor() {
    let (_document, mut shell) = setup(488, 700);
    let mut cursors = std::collections::HashSet::new();
    for (brush, name) in [
        ("pencil", "Pencil"),
        ("line", "Line"),
        ("rect", "Rectangle"),
        ("ellipse", "Ellipse"),
        ("lift", "Lift pixels"),
        ("text", "Text"),
        ("stamp", "Stamp"),
        ("curve", "Curve"),
        ("glass", "Glass lens"),
        ("tuft", "Taper"),
    ] {
        click(&mut shell, "Drawing tools");
        click(&mut shell, name);
        let canvas = bounds(&mut shell, "Skin canvas.");
        shell.set_cursor(canvas.x + 50., canvas.y + 50.);
        shell.refresh_pointer_icon();
        pump(&mut shell);
        let Some(cranpose_ui::PointerIcon::Custom(cursor)) = shell.take_pointer_icon_change()
        else {
            panic!("{brush} has no custom canvas cursor");
        };
        assert_eq!(cursor.hotspot_x(), 4, "{brush} inherited a control cursor");
        assert!(
            cursors.insert(cursor.image().pixels().to_vec()),
            "{brush} repeats another cursor"
        );
    }
}

#[test]
fn drawing_stays_quiet_and_errors_use_one_dismissible_bottom_right_notice() {
    let (_document, mut shell) = setup(488, 700);
    let p = at(&mut shell, [140, 106], 1.);
    stroke(&mut shell, p, p);
    assert!(!visible_texts(&mut shell)
        .iter()
        .any(|s| s == "Studio notification"));
    click(&mut shell, "Drawing tools");
    click(&mut shell, "Stamp");
    stroke(&mut shell, p, p);
    let notice = bounds(&mut shell, "Studio notification");
    assert!((notice.x + notice.width - 476.).abs() < 1.);
    assert!((notice.y + notice.height - 688.).abs() < 1.);
    click(&mut shell, "Dismiss notification");
    assert!(!visible_texts(&mut shell)
        .iter()
        .any(|s| s == "Studio notification"));
}

#[test]
fn equalizer_frames_fit_every_width_and_keep_touch_targets() {
    for width in [240, 320, 430, 768, 900, 1280, 1920] {
        let document = open_document(None).unwrap();
        document
            .lock()
            .unwrap()
            .state(serde_json::json!({"drawer":"equalizer"}))
            .unwrap();
        let mut shell = AppShell::new(
            HitGraphRenderer::default(),
            location_key(file!(), line!(), column!()),
            move || SkinStudio(document.clone(), None),
        );
        shell.set_buffer_size(width, 900);
        shell.set_viewport(width as f32, 900.);
        pump(&mut shell);
        for frame in 0..28 {
            let rect = bounds(&mut shell, &format!("{frame:02}"));
            assert!(
                rect.x >= 0. && rect.x + rect.width <= width as f32 + 1.,
                "frame {frame} exceeds width {width}: {rect:?}"
            );
            assert!(
                rect.width >= 44. && rect.height >= 44.,
                "frame {frame} is too small at {width}: {rect:?}"
            );
        }
    }
}

#[test]
fn history_panel_restores_the_selected_edit() {
    let (doc, mut shell) = setup(430, 820);
    let before = doc.lock().unwrap().render();
    let point = at(&mut shell, [120, 100], 1.);
    stroke(&mut shell, point, point);
    assert!(doc.lock().unwrap().render() != before);
    let first = doc.lock().unwrap().history()["entries"][0]["label"]
        .as_str()
        .unwrap()
        .to_owned();
    click(&mut shell, "Project actions");
    click(&mut shell, "Edit history");
    click(&mut shell, &first);
    assert!(doc.lock().unwrap().render() == before);
}

#[test]
fn a_sheet_does_not_show_help_for_a_covered_brush_control() {
    let (_doc, mut shell) = setup(600, 360);
    let field = bounds(&mut shell, "Brush size value");
    click(&mut shell, "Project actions");
    click(&mut shell, "Pixel study");
    bounds(&mut shell, "Scroll panel");
    shell.set_cursor(field.x + field.width / 2., field.y + field.height / 2.);
    pump(&mut shell);
    std::thread::sleep(std::time::Duration::from_millis(500));
    pump(&mut shell);
    assert!(!visible_texts(&mut shell)
        .iter()
        .any(|text| text == "1 to 32 pixels"));
}

#[test]
fn skin_palette_edits_change_the_skin_and_undo_without_changing_brush_colour() {
    use std::io::Read;
    fn palette(doc: &SharedDocument) -> String {
        let bytes = doc.lock().unwrap().archive().unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        zip.by_name("pledit.txt")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        text
    }
    for width in [320, 430, 768, 1100, 1440] {
        let (doc, mut shell) = setup(width, 900);
        let before = palette(&doc);
        let brush = doc.lock().unwrap().view.color.clone();
        click(&mut shell, "Skin");
        let field = bounds(&mut shell, "HEX colour");
        assert!(
            field.x >= 0.
                && field.x + field.width <= width as f32
                && field.y + field.height <= 900.,
            "width {width}: {field:?}"
        );
        enter_value(&mut shell, "HEX colour", "#112233");
        assert!(palette(&doc).contains("Normal=#112233"), "width {width}");
        assert_eq!(doc.lock().unwrap().view.color, brush);
        click(&mut shell, "Undo");
        assert_eq!(palette(&doc), before);
    }
}

#[test]
fn skin_cursor_controls_generate_missing_files_and_edit_the_click_point() {
    for width in [320, 768, 1440] {
        let (doc, mut shell) = setup(width, 900);
        doc.lock().unwrap().remove_cursors(&[], "test").unwrap();
        click(&mut shell, "Skin");
        click(&mut shell, "Cursors");
        click(&mut shell, "Draw missing cursors");
        let report = doc.lock().unwrap().cursors_report();
        assert_eq!(report["drawn"], report["of"]);
        enter_value(&mut shell, "Cursor X", "2");
        enter_value(&mut shell, "Cursor Y", "3");
        assert_eq!(
            doc.lock().unwrap().cursors_report()["regions"][0]["hotspot"],
            serde_json::json!([2, 3])
        );
    }
}
