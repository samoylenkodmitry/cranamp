use super::*;

#[test]
fn classic_preview_switch_is_exposed_and_never_rewrites_exported_art() {
    let shared = document();
    let before = shared.lock().unwrap().archive().unwrap();
    call("studio_options", json!({"classic_preview":true}), &shared).unwrap();
    assert!(shared.lock().unwrap().view.classic_preview);
    assert_eq!(shared.lock().unwrap().archive().unwrap(), before);
    assert!(call("studio_options", json!({"classic_preview":false}), &shared).is_err());
    assert!(shared.lock().unwrap().view.classic_preview);
    assert_eq!(shared.lock().unwrap().archive().unwrap(), before);
}

#[test]
fn pixel_palette_reports_composited_colors_without_changing_document_or_history() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_atlas", json!({"sheet":"main.bmp"}), &shared).unwrap();
    call(
        "studio_layers",
        json!({"action":"add","name":"Palette probe"}),
        &shared,
    )
    .unwrap();
    call(
        "studio_draw",
        json!({"operations":[{"op":"rect","x":0,"y":0,"width":2,"height":2,"color":"#fff0ca"}]}),
        &shared,
    )
    .unwrap();
    let before = shared.lock().unwrap().status();
    let response = call(
        "studio_pixel",
        json!({"x":0,"y":0,"width":2,"height":2,"palette":true}),
        &shared,
    )
    .unwrap();
    let report: Value =
        serde_json::from_str(response["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["palette"]["unique_colors"], 1);
    assert_eq!(report["palette"]["colors"][0]["hex"], "#fff0caff");
    assert_eq!(report["palette"]["colors"][0]["pixels"], 4);
    assert_eq!(shared.lock().unwrap().status(), before);
}
use crate::winamp::studio::model::Document;
use std::sync::{Arc, Mutex};
fn document() -> SharedDocument {
    crate::winamp::studio::open_document(None).expect("bundled document")
}
fn names() -> Vec<String> {
    tools()
        .into_iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect()
}
#[test]
fn the_offered_tools_are_the_panels_a_human_works_in() {
    let offered = names();
    for panel in [
        "studio_canvas",
        "studio_atlas",
        "studio_targets",
        "studio_rectangles",
        "studio_layers",
        "studio_options",
        "studio_states",
        "studio_study",
        "studio_pixel",
        "studio_draw",
        "studio_history",
    ] {
        assert!(offered.iter().any(|n| n == panel), "{panel} is not offered");
    }
    for retired in [
        "studio_state",
        "studio_patch",
        "studio_inspect_region",
        "studio_render",
        "studio_set_palette",
        "studio_visualizer_palette",
    ] {
        assert!(
            !offered.iter().any(|n| n == retired),
            "{retired} describes an editor this one no longer is"
        );
    }
}
#[test]
fn the_retired_tools_still_answer() {
    let shared = document();
    for retired in ["studio_state", "studio_palette"] {
        call(retired, json!({}), &shared)
            .unwrap_or_else(|e| panic!("{retired} should still answer: {e:#}"));
    }
}
#[test]
fn studio_options_reads_and_writes_everything_outside_the_sheets() {
    let shared = document();
    let before = call("studio_options", json!({}), &shared).unwrap();
    let before: Value =
        serde_json::from_str(before["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(before["playlist_colors"].as_object().unwrap().len(), 6);
    assert_eq!(before["visualizer_colors"].as_array().unwrap().len(), 24);
    let after = call(
        "studio_options",
        json!({ "playlist_colors": {"Normal": "#010203"} }),
        &shared,
    )
    .unwrap();
    let after: Value = serde_json::from_str(after["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(after["playlist_colors"]["Normal"], "#010203");
}
#[test]
fn setting_the_pencil_does_not_close_an_open_atlas() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_atlas", json!({"sheet": "monoster.bmp"}), &shared).unwrap();
    for pencil in [
        json!({"face": "small"}),
        json!({"text_scale": 2}),
        json!({"color": "#ffcc66"}),
        json!({"brush_size": 3}),
        json!({"measure": ["MONO", "STEREO"]}),
    ] {
        call("studio_canvas", pencil.clone(), &shared).unwrap();
        let view = shared.lock().unwrap().view.clone();
        assert_eq!(
            view.panel, "atlas",
            "{pencil} moved the pencil off monoster.bmp"
        );
        assert_eq!(view.sheet, "monoster.bmp");
    }
    assert_eq!(shared.lock().unwrap().view.face, "small");
    call("studio_canvas", json!({"zoom": 3}), &shared).unwrap();
    assert_eq!(shared.lock().unwrap().view.panel, "canvas");
    call("studio_atlas", json!({"sheet": "monoster.bmp"}), &shared).unwrap();
    call("studio_canvas", json!({}), &shared).unwrap();
    assert_eq!(
        shared.lock().unwrap().view.panel,
        "canvas",
        "an empty call is still show me the canvas"
    );
}
#[test]
fn reading_an_open_sheet_back_does_not_close_it() {
    let dir = std::env::temp_dir().join("cranamp-atlas-look-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("look.png");
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_atlas", json!({"sheet": "posbar.bmp"}), &shared).unwrap();
    let answer = call(
        "studio_atlas",
        json!({"crop": [0, 0, 307, 10], "magnify": 2, "path": path.to_str().unwrap()}),
        &shared,
    )
    .unwrap();
    let answer: Value =
        serde_json::from_str(answer["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        answer["size"],
        json!([614, 20]),
        "the sheet, not the canvas"
    );
    assert_eq!(shared.lock().unwrap().view.panel, "atlas");
    assert_eq!(shared.lock().unwrap().view.sheet, "posbar.bmp");
    call("studio_atlas", json!({}), &shared).unwrap();
    assert_eq!(shared.lock().unwrap().view.panel, "canvas");
    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn a_committed_transaction_reads_the_surface_back_like_a_preview() {
    let dir = std::env::temp_dir().join("cranamp-draw-readback-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("stroke.png");
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    let before = shared.lock().unwrap().revision;
    let answer = call(
        "studio_draw",
        json!({
            "operations": [{"op":"rect","x":16,"y":88,"width":23,"height":18,"color":"#ff8800"}],
            "crop": [16, 88, 23, 18],
            "magnify": 4,
            "path": path.to_str().unwrap(),
        }),
        &shared,
    )
    .unwrap();
    let answer: Value =
        serde_json::from_str(answer["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(answer["size"], json!([92, 72]));
    assert!(
        answer["path"].as_str().unwrap().ends_with("stroke.png"),
        "the answer names where it wrote: {}",
        answer["path"]
    );
    assert!(answer["pixels_written"].as_u64().unwrap() > 0);
    assert!(path.exists(), "the file the answer named");
    assert!(
        shared.lock().unwrap().revision > before,
        "a committed stroke, not a dry run"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
#[test]
fn refusing_to_discard_unsaved_edits_names_the_argument_that_does_it() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call(
        "studio_draw",
        json!({"operations":[{"op":"rect","x":0,"y":0,"width":4,"height":4,
                              "color":"#ffffff"}]}),
        &shared,
    )
    .unwrap();
    let refusal = call("studio_new", json!({}), &shared)
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("discard"), "{refusal}");
    assert!(refusal.contains("studio_new"), "{refusal}");
    call("studio_new", json!({"discard": true}), &shared).unwrap();
}
#[test]
fn clip_can_be_set_without_leaving_the_sheet_it_is_for() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_atlas", json!({"sheet": "cbuttons.bmp"}), &shared).unwrap();
    call("studio_canvas", json!({"clip": [23, 18, 23, 18]}), &shared).unwrap();
    {
        let view = shared.lock().unwrap().view.clone();
        assert_eq!(view.panel, "atlas");
        assert_eq!(view.clip, Some([23, 18, 23, 18]));
    }
    call(
        "studio_draw",
        json!({"operations":[{"op":"rect","x":0,"y":0,"width":136,"height":36,
                              "color":"#abcdef"}]}),
        &shared,
    )
    .unwrap();
    let doc = shared.lock().unwrap();
    assert_eq!(
        doc.inspect([23, 18, 1, 1], None)["hits"][0]["rgba"],
        json!([171, 205, 239, 255])
    );
    assert_eq!(
        doc.inspect([22, 18, 1, 1], None)["hits"][0]["rgba"],
        json!([0, 0, 0, 0]),
        "the column beside the cell is outside the clip"
    );
}
#[test]
fn the_history_is_seekable_by_the_cursor_it_answers_with() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    for colour in ["#111111", "#222222", "#333333"] {
        call(
            "studio_draw",
            json!({"label":format!("step {colour}"),
                   "operations":[{"op":"rect","x":0,"y":0,"width":8,"height":8,
                                  "color":colour}]}),
            &shared,
        )
        .unwrap();
    }
    let read = |args: Value| -> Value {
        let out = call("studio_history", args, &shared).unwrap();
        serde_json::from_str(out["content"][0]["text"].as_str().unwrap()).unwrap()
    };
    let before = read(json!({}));
    assert_eq!(before["cursor"], 3);
    assert_eq!(before["entries"][2]["label"], "step #222222");
    let after = read(json!({"cursor": 1}));
    assert_eq!(after["cursor"], 1);
    assert_eq!(
        shared.lock().unwrap().render().get_pixel(0, 0).0[..3],
        [17, 17, 17]
    );
    read(json!({"cursor": 3}));
    assert_eq!(
        shared.lock().unwrap().render().get_pixel(0, 0).0[..3],
        [51, 51, 51]
    );
}
#[test]
fn the_pixel_probe_answers_the_ground_and_whether_a_colour_reads_on_it() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_atlas", json!({"sheet": "main.bmp"}), &shared).unwrap();
    call(
        "studio_draw",
        json!({"operations":[{"op":"rect","x":0,"y":0,"width":275,"height":115,
                              "color":"#1b1526"}]}),
        &shared,
    )
    .unwrap();
    let probe = |args: Value| -> Value {
        let out = call("studio_pixel", args, &shared).unwrap();
        serde_json::from_str(out["content"][0]["text"].as_str().unwrap()).unwrap()
    };
    let sheet = probe(json!({"x": 40, "y": 40}));
    assert_eq!(sheet["surface"], "atlas main.bmp");
    call("studio_atlas", json!({}), &shared).unwrap();
    let one = probe(json!({"x": 40, "y": 40}));
    assert_eq!(one["surface"], "canvas");
    assert_eq!(one["ground"], "#1b1526");
    assert_eq!(one["hits"][0]["layer"], "main.background");
    let region = probe(json!({"x": 20, "y": 20, "width": 40, "height": 20,
                              "ink": "#ffdf9c"}));
    assert_eq!(region["rect"], json!([20, 20, 40, 20]));
    assert_eq!(region["readable"], true);
    assert!(region["contrast"].as_f64().unwrap() > 10.0, "{region}");
    let hidden = probe(json!({"x": 20, "y": 20, "width": 40, "height": 20,
                              "ink": "#221c30"}));
    assert_eq!(hidden["readable"], false, "{hidden}");
    let outside = probe(json!({"x": 400, "y": 400}));
    assert!(
        outside["nothing_at"]
            .as_str()
            .is_some_and(|s| s.contains("canvas") && s.contains("275x377")),
        "{outside}"
    );
}
#[test]
fn studio_canvas_puts_the_editor_on_the_whole_skin() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_atlas", json!({"sheet": "main.bmp"}), &shared).unwrap();
    assert_eq!(shared.lock().unwrap().view.panel, "atlas");
    call("studio_canvas", json!({"zoom": 3}), &shared).unwrap();
    let view = shared.lock().unwrap().view.clone();
    assert_eq!(view.panel, "canvas");
    assert_eq!(view.zoom, 3);
    let listed = call("studio_targets", json!({}), &shared).unwrap();
    let listed: Value =
        serde_json::from_str(listed["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(listed["sprites"].as_array().unwrap().len() > 40);
    assert_eq!(
        listed["chosen"],
        json!([]),
        "auto by default, as for a human"
    );
}

#[test]
fn pixel_inspection_explains_coverage_and_the_coverage_tool_is_read_only() {
    let shared = document();
    call("studio_canvas", json!({}), &shared).unwrap();
    let before = shared.lock().unwrap().status();
    let response = call(
        "studio_coverage",
        json!({"rect":[100,35,10,20],"rows":true}),
        &shared,
    )
    .unwrap();
    let report: Value =
        serde_json::from_str(response["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["counts"]["paintable"], 200);
    assert_eq!(report["rows"].as_array().unwrap().len(), 20);
    assert_eq!(shared.lock().unwrap().status(), before);
    let response = call("studio_pixel", json!({"x":105,"y":52}), &shared).unwrap();
    let report: Value =
        serde_json::from_str(response["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["paintability"]["counts"]["paintable"], 1);
    assert_eq!(shared.lock().unwrap().status(), before);
}
#[test]
fn cursor_schema_advertises_every_drawable_style() {
    let offered = tools()
        .into_iter()
        .find(|t| t["name"] == "studio_cursors")
        .unwrap();
    let names = offered["inputSchema"]["properties"]["style"]["enum"]
        .as_array()
        .unwrap();
    for style in super::super::cursor_art::Style::ALL {
        assert!(names.iter().any(|name| name == style.name()));
    }
}

#[test]
fn generated_paws_sample_the_visible_paint_planes() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call(
        "studio_layers",
        json!({"action":"add","name":"Visible theme"}),
        &shared,
    )
    .unwrap();
    for (sheet, w, h) in [
        ("main.bmp", 275, 116),
        ("eqmain.bmp", 275, 315),
        ("pledit.bmp", 280, 190),
        ("titlebar.bmp", 344, 87),
    ] {
        call("studio_atlas", json!({"sheet":sheet}), &shared).unwrap();
        call(
            "studio_draw",
            json!({"operations":[
                {"op":"rect","x":0,"y":0,"width":w,"height":h,"color":"#112233"},
                {"op":"rect","x":0,"y":0,"width":w/4,"height":h,"color":"#ffdd99"}
            ]}),
            &shared,
        )
        .unwrap();
    }
    let response = call(
        "studio_cursors",
        json!({"action":"draw","style":"paw","overwrite":true}),
        &shared,
    )
    .unwrap();
    let report: Value =
        serde_json::from_str(response["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["palette"]["body"], "#ffdd99");
    assert_eq!(report["palette"]["ink"], "#112233");
    assert_eq!(
        report["drawn"].as_array().unwrap().len(),
        crate::winamp::cursors::SkinCursor::COUNT
    );
}

#[test]
fn contextual_state_study_shows_overlapping_variants_without_mutating_the_document() {
    use base64::Engine;
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call(
        "studio_canvas",
        json!({"active":false,"pressed":false}),
        &shared,
    )
    .unwrap();
    call(
        "studio_draw",
        json!({"origin":"main.options","states":"current","operations":[
            {"op":"rect","x":0,"y":0,"width":9,"height":9,"color":"#ff0000"}
        ]}),
        &shared,
    )
    .unwrap();
    call("studio_canvas", json!({"pressed":true}), &shared).unwrap();
    call(
        "studio_draw",
        json!({"origin":"main.options","states":"current","operations":[
            {"op":"rect","x":0,"y":0,"width":9,"height":9,"color":"#00ff00"}
        ]}),
        &shared,
    )
    .unwrap();
    let before = shared.lock().unwrap().status();
    let response = call(
        "studio_study",
        json!({"rect":[6,3,9,9],"states":true,"zoom":1}),
        &shared,
    )
    .unwrap();
    let report: Value =
        serde_json::from_str(response["content"][1]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report["study"]["context_rect"], json!([2, 0, 17, 16]));
    assert_eq!(report["study"]["state_order"].as_array().unwrap().len(), 4);
    let png = base64::engine::general_purpose::STANDARD
        .decode(response["content"][0]["data"].as_str().unwrap())
        .unwrap();
    let image = image::load_from_memory(&png).unwrap().to_rgba8();
    // Both native and enlarged crops show each real sprite state, twice for focus.
    assert_eq!(
        image.pixels().filter(|p| p.0 == [255, 0, 0, 255]).count(),
        324
    );
    assert_eq!(
        image.pixels().filter(|p| p.0 == [0, 255, 0, 255]).count(),
        324
    );
    assert_eq!(shared.lock().unwrap().status(), before);
    assert!(call(
        "studio_study",
        json!({"rect":[274,0,2,2],"states":true}),
        &shared
    )
    .is_err());
    assert_eq!(shared.lock().unwrap().status(), before);
}
