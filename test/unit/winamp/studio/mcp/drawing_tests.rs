use super::*;
use crate::winamp::studio::model::Document;
use std::sync::{Arc, Mutex};
fn blank() -> SharedDocument {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    call("studio_canvas", json!({}), &shared).unwrap();
    shared
}
fn result(shared: &SharedDocument, name: &str, args: Value) -> Value {
    let out = call(name, args, shared).unwrap();
    serde_json::from_str(out["content"][0]["text"].as_str().unwrap()).unwrap()
}
#[test]
fn text_draws_a_label_in_the_editors_own_face() {
    let shared = blank();
    let out = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"text","x":20,"y":30,"text":"CAT","color":"#f5c06a"}]}),
    );
    assert!(out["pixels_written"].as_u64().unwrap() > 20);
    assert_eq!(out["bounds"][0], 20);
    assert_eq!(out["bounds"][2], 36);
    assert_eq!(out["bounds"][3], 36);
}
#[test]
fn origin_puts_the_coordinates_on_the_sprite_wherever_it_currently_sits() {
    let shared = blank();
    call("studio_canvas", json!({"volume": 5}), &shared).unwrap();
    let out = result(
        &shared,
        "studio_draw",
        json!({"origin":"main.volume.thumb",
               "operations":[{"op":"pixel","x":0,"y":0,"color":"#4488cc"}]}),
    );
    assert_eq!(out["pixels_written"], 1);
    assert_eq!(out["clipped_pixels"], 0);
    for frame in (0..28).step_by(9) {
        let mut doc = shared.0.lock().unwrap();
        doc.state(json!({"volume": frame})).unwrap();
        let at = doc
            .layers()
            .into_iter()
            .find(|l| l.id == "main.volume.thumb")
            .unwrap()
            .destination;
        assert_eq!(
            doc.render().get_pixel(at[0], at[1]).0,
            [0x44, 0x88, 0xcc, 0xff],
            "frame {frame}"
        );
    }
}
#[test]
fn writing_one_shared_source_cell_twice_says_so() {
    let shared = blank();
    let out = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"pixel","x":48,"y":26,"color":"#112233"},
                             {"op":"pixel","x":60,"y":26,"color":"#ccddee"}]}),
    );
    assert!(out["overwrites"].as_u64().unwrap() > 0);
    let note = out["overwrite_sample"].as_str().unwrap();
    assert!(note.contains("numbers.bmp"), "{note}");
    assert!(note.contains("layers"), "{note}");
    let aimed = result(
        &shared,
        "studio_draw",
        json!({"layers":["main.digit0"],
               "operations":[{"op":"pixel","x":48,"y":26,"color":"#112233"},
                             {"op":"pixel","x":60,"y":26,"color":"#ccddee"}]}),
    );
    assert_eq!(aimed["overwrites"], 0);
    assert!(aimed["overwrite_sample"].is_null());
}
#[test]
fn a_half_transparent_stamp_blends_with_what_is_under_it() {
    let shared = blank();
    result(
        &shared,
        "studio_draw",
        json!({"layers":[],
               "operations":[{"op":"rect","x":8,"y":8,"width":4,"height":4,
                              "color":"#202060"}]}),
    );
    let mut im = image::RgbaImage::new(1, 1);
    im.put_pixel(0, 0, image::Rgba([0xff, 0xff, 0xff, 0x80]));
    let mut png = std::io::Cursor::new(Vec::new());
    im.write_to(&mut png, image::ImageFormat::Png).unwrap();
    result(
        &shared,
        "studio_draw",
        json!({"layers":[],
               "operations":[{"op":"image","x":9,"y":9,"data":base64(png.get_ref())}]}),
    );
    let doc = shared.0.lock().unwrap();
    let [r, g, b, _] = doc.render().get_pixel(9, 9).0;
    assert!((r as i32 - 0x90).abs() <= 2, "r {r:#x}");
    assert!((g as i32 - 0x90).abs() <= 2, "g {g:#x}");
    assert!((b as i32 - 0xb0).abs() <= 2, "b {b:#x}");
}
#[test]
fn what_a_draw_reports_covers_that_draw_and_no_earlier_one() {
    let shared = blank();
    let first = result(
        &shared,
        "studio_draw",
        json!({"layers":["main.play"],
               "operations":[{"op":"rect","x":0,"y":0,"width":40,"height":40,
                              "color":"#112233"}]}),
    );
    assert!(
        first["clipped_pixels"].as_u64().unwrap() > 0,
        "aimed past the sprite"
    );
    let second = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"pixel","x":44,"y":92,"color":"#445566"}]}),
    );
    assert_eq!(second["bounds"], json!([44, 92, 44, 92]));
    assert_eq!(second["clipped_pixels"], 0);
}
#[test]
fn an_image_stamps_its_opaque_pixels_and_leaves_the_transparent_ones_alone() {
    let shared = blank();
    let before = {
        let doc = shared.0.lock().unwrap();
        doc.render().get_pixel(41, 31).0
    };
    let mut im = image::RgbaImage::new(2, 2);
    im.put_pixel(0, 0, image::Rgba([0x11, 0x22, 0x33, 0xff]));
    im.put_pixel(1, 0, image::Rgba([0x11, 0x22, 0x33, 0xff]));
    let mut png = std::io::Cursor::new(Vec::new());
    im.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let out = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"image","x":40,"y":30,"data":base64(png.get_ref())}]}),
    );
    assert_eq!(out["pixels_written"], 2);
    let doc = shared.0.lock().unwrap();
    assert_eq!(doc.render().get_pixel(40, 30).0, [0x11, 0x22, 0x33, 0xff]);
    assert_eq!(doc.render().get_pixel(41, 31).0, before);
}

#[test]
fn an_image_crop_resizes_with_exact_pixels_and_keeps_transparency() {
    let shared = blank();
    result(
        &shared,
        "studio_draw",
        json!({"layers":["main.background"],
        "operations":[{"op":"rect","x":120,"y":20,"width":4,"height":4,"color":"#202060"}]}),
    );
    let mut im = image::RgbaImage::from_pixel(4, 2, image::Rgba([255, 0, 0, 255]));
    im.put_pixel(1, 0, image::Rgba([0x11, 0x22, 0x33, 255]));
    im.put_pixel(2, 0, image::Rgba([0x44, 0x55, 0x66, 255]));
    im.put_pixel(1, 1, image::Rgba([0, 0, 0, 0]));
    im.put_pixel(2, 1, image::Rgba([255, 255, 255, 128]));
    let mut png = std::io::Cursor::new(Vec::new());
    im.write_to(&mut png, image::ImageFormat::Png).unwrap();
    result(
        &shared,
        "studio_draw",
        json!({"layers":["main.background"],
        "operations":[{"op":"image","x":120,"y":20,"data":base64(png.get_ref()),
            "source_rect":[1,0,2,2],"width":4,"height":4}]}),
    );
    let doc = shared.lock().unwrap();
    let rendered = doc.render();
    for y in 0..4 {
        for x in 0..4 {
            let expected = match (x / 2, y / 2) {
                (0, 0) => [0x11, 0x22, 0x33, 255],
                (1, 0) => [0x44, 0x55, 0x66, 255],
                (0, 1) => [0x20, 0x20, 0x60, 255],
                _ => [0x90, 0x90, 0xb0, 255],
            };
            assert_eq!(rendered.get_pixel(120 + x, 20 + y).0, expected);
        }
    }
    drop(doc);
    result(&shared, "studio_undo", json!({}));
    assert_eq!(
        shared.lock().unwrap().render().get_pixel(120, 20).0,
        [0x20, 0x20, 0x60, 255]
    );
}

#[test]
fn invalid_image_geometry_rolls_back_the_entire_transaction() {
    let shared = blank();
    let im = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    im.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let before = shared.lock().unwrap().render();
    let status = result(&shared, "studio_status", json!({}));
    for geometry in [
        json!({"width":4}),
        json!({"width":0,"height":4}),
        json!({"width":2049,"height":4}),
        json!({"width":2.5,"height":4}),
        json!({"source_rect":[1,0,2,2]}),
        json!({"source_rect":[0,0,0,2]}),
        json!({"source_rect":[0,0,2]}),
        json!({"source_rect":[-1,0,1,1]}),
        json!({"source_rect":[4294967295u64,0,2,2]}),
    ] {
        let mut op = json!({"op":"image","x":120,"y":20,"data":base64(png.get_ref())});
        op.as_object_mut()
            .unwrap()
            .extend(geometry.as_object().unwrap().clone());
        let refused = call(
            "studio_draw",
            json!({"layers":["main.background"],
            "operations":[{"op":"pixel","x":120,"y":20,"color":"#123456"},op]}),
            &shared,
        );
        assert!(refused.is_err(), "{geometry}");
        assert_eq!(shared.lock().unwrap().render(), before, "{geometry}");
        let after = result(&shared, "studio_status", json!({}));
        assert_eq!(after["revision"], status["revision"]);
        assert_eq!(after["undo"], status["undo"]);
    }
}
#[test]
fn a_stroke_reports_what_fell_outside_the_sprites_it_was_aimed_at() {
    let shared = blank();
    let out = result(
        &shared,
        "studio_draw",
        json!({
            "layers": ["main.close"],
            "operations": [{"op":"rect","x":260,"y":0,"width":20,"height":20,
                            "color":"#f5c06a","fill":true}],
        }),
    );
    assert!(
        out["clipped_pixels"].as_u64().unwrap() > 0,
        "most of that rectangle is outside a 9x9 button and the caller has to be told"
    );
    assert!(
        out["pixels_written"].as_u64().unwrap() > 0,
        "the rest still landed"
    );
}
#[test]
fn an_argument_a_tool_does_not_have_is_refused_by_name() {
    let refusal = reject_unknown_arguments("studio_screenshot", &json!({"brush": "pencil"}))
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("brush"), "{refusal}");
    assert!(
        refusal.contains("studio_canvas"),
        "say where the field does belong: {refusal}"
    );
    assert!(
        refusal.contains("crop"),
        "and what this one takes: {refusal}"
    );
    let refusal = reject_unknown_arguments("studio_undo", &json!({"nonesuch": 1}))
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("nonesuch"), "{refusal}");
    reject_unknown_arguments(
        "studio_screenshot",
        &json!({"crop":[0,0,8,8],"panel":"main"}),
    )
    .unwrap();
    reject_unknown_arguments("studio_patch", &json!({"anything": 1})).unwrap();
}
#[test]
fn the_canvas_can_be_read_back_cropped() {
    let shared = blank();
    let dir = std::env::temp_dir().join("cranamp-crop-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("crop.png");
    let out = result(
        &shared,
        "studio_canvas",
        json!({"zoom": 2, "crop": [16, 88, 23, 18], "path": path.to_str().unwrap()}),
    );
    assert_eq!(
        out["size"],
        json!([46, 36]),
        "the crop, at the asked-for zoom"
    );
    std::fs::remove_file(&path).ok();
}
#[test]
fn a_blank_document_opens_on_the_whole_skin() {
    let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
    let out = result(&shared, "studio_new", json!({ "discard": true }));
    assert_eq!(out["surface"], "canvas");
    assert_eq!(out["canvas"], json!([275, 377]));
    assert_eq!(out["sprites"], json!(80));
}
#[test]
fn a_curve_may_start_and_end_between_pixels() {
    let shared = blank();
    let out = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"curve","x":20.5,"y":30.25,"x2":60.5,"y2":50.75,
                             "control":[24.5,48.5],"color":"#ffffff","brush_size":2}]}),
    );
    assert!(out["pixels_written"].as_u64().unwrap() > 0);
    let refused = call(
        "studio_draw",
        json!({"operations":[{"op":"rect","x":1.5,"y":2,"width":3,"height":3}]}),
        &shared,
    )
    .unwrap_err();
    let refused = format!("{refused:#}");
    assert!(
        refused.contains("x is a whole number of pixels"),
        "a refusal names the field and the value it got: {refused}"
    );
}
#[test]
fn the_catalogue_answers_for_the_skin_while_one_sheet_is_open() {
    let shared = blank();
    call("studio_atlas", json!({"sheet":"volume.bmp"}), &shared).unwrap();
    let out = result(
        &shared,
        "studio_targets",
        json!({"id":"volume.track","variants":true}),
    );
    assert_eq!(out["sprites"][0]["id"], "main.volume.track");
    assert_eq!(out["sprites"][0]["variants"].as_array().unwrap().len(), 28);
    let empty = result(&shared, "studio_rectangles", json!({"id":"main.play"}));
    assert!(
        empty["note"]
            .as_str()
            .unwrap_or_default()
            .contains("atlas volume.bmp"),
        "an empty match says what it looked in: {empty}"
    );
}
#[test]
fn a_canvas_rectangle_is_labelled_with_the_variant_it_is_showing() {
    let shared = blank();
    call("studio_canvas", json!({ "volume": 20 }), &shared).unwrap();
    let out = result(&shared, "studio_rectangles", json!({"id":"volume.track"}));
    assert_eq!(out["rectangles"][0]["variant"], json!(20));
    assert_eq!(out["rectangles"][0]["label"], "track · 20");
}
#[test]
fn ink_that_crosses_into_a_repeated_cell_says_so() {
    let shared = blank();
    call("studio_atlas", json!({"sheet":"pledit.bmp"}), &shared).unwrap();
    let spilled = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"text","x":98,"y":32,"text":"/ CAT SCAN",
                             "color":"#ffffff","face":"small"}]}),
    );
    assert_eq!(spilled["crossed_cells"][0]["cell"], "playlist.top.tile");
    assert_eq!(spilled["crossed_cells"][0]["drawn"], json!(9));
    let inside = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"rect","x":127,"y":21,"width":25,"height":20,
                             "color":"#203040"}]}),
    );
    assert!(
        inside.get("crossed_cells").is_none(),
        "a cell painted on its own is not a crossing: {inside}"
    );
    for color in ["#ff00ff", "#203040"] {
        let whole = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"rect","x":0,"y":0,"width":280,"height":190,
                                 "color":color}]}),
        );
        assert!(
            whole.get("crossed_cells").is_none(),
            "clearing or washing a whole sheet is not a crossing: {whole}"
        );
    }
}
/// A run of open, edit, export is the shape of every batch job over a
/// folder of skins. If the open fails and the export still needs a path of
/// its own, the edit lands in the skin that was already open and the export
/// writes it under the name that failed to open.
#[test]
fn export_without_a_path_writes_the_skin_back_where_it_came_from() {
    let shared = SharedDocument(Arc::new(Mutex::new(
        Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap(),
    )));
    let first = std::env::temp_dir().join("cranamp-export-in-place.wsz");
    result(
        &shared,
        "studio_export",
        json!({ "path": first.to_str().unwrap() }),
    );
    result(&shared, "studio_cursors", json!({"action":"draw"}));

    let out = result(&shared, "studio_export", json!({}));
    assert_eq!(
        out["path"].as_str(),
        first.to_str(),
        "an export with no path writes the file the skin came from: {out}"
    );
    let written = std::fs::read(&first).expect("the skin was written");
    let skin = crate::winamp::skin::load_skin(&written).expect("it loads");
    assert_eq!(
        skin.cursors.len(),
        crate::winamp::cursors::SkinCursor::COUNT
    );
    std::fs::remove_file(first).ok();
}

#[test]
fn a_skin_that_was_never_written_still_asks_where_to_go() {
    let shared = blank();
    let out = call("studio_export", json!({}), &shared);
    assert!(
        format!("{out:?}").contains("needs a path"),
        "a blank skin has nowhere to be written back to: {out:?}"
    );
}

#[test]
fn studio_cursors_lists_every_region_a_classic_player_reads() {
    let shared = blank();
    let out = result(&shared, "studio_cursors", json!({}));
    assert_eq!(out["of"], json!(crate::winamp::cursors::SkinCursor::COUNT));
    assert_eq!(out["drawn"], json!(0), "a blank skin ships no cursors");
    assert_eq!(
        out["regions"].as_array().unwrap().len(),
        crate::winamp::cursors::SkinCursor::COUNT
    );
}

#[test]
fn studio_cursors_draws_a_set_and_opens_it_on_the_canvas() {
    let shared = blank();
    let out = result(&shared, "studio_cursors", json!({"action":"draw"}));
    assert_eq!(
        out["drawn"].as_array().unwrap().len(),
        crate::winamp::cursors::SkinCursor::COUNT,
        "{out}"
    );
    let status = result(&shared, "studio_status", json!({}));
    assert_eq!(status["view"]["panel"], json!("cursors"), "{status}");
    let listed = result(&shared, "studio_cursors", json!({}));
    assert_eq!(
        listed["drawn"],
        json!(crate::winamp::cursors::SkinCursor::COUNT)
    );
}

#[test]
fn a_surface_puts_the_rolled_up_player_on_the_canvas_and_back() {
    let document = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None);
    let shared = SharedDocument(Arc::new(Mutex::new(document.unwrap())));
    result(&shared, "studio_canvas", json!({"surface": "shade"}));
    let status = result(&shared, "studio_status", json!({}));
    assert_eq!(status["view"]["panel"], json!("shade"), "{status}");
    assert_eq!(status["canvas"], json!([275, 14]), "{status}");
    let targets = result(&shared, "studio_targets", json!({"id": "unshade"}));
    assert!(format!("{targets}").contains("titlebar.bmp"), "{targets}");

    result(&shared, "studio_canvas", json!({"zoom": 2}));
    let status = result(&shared, "studio_status", json!({}));
    assert_eq!(status["view"]["panel"], json!("canvas"), "{status}");
}

#[test]
fn a_drawn_cursor_leaves_the_skin_as_a_cursor_a_player_can_read() {
    let shared = blank();
    result(&shared, "studio_cursors", json!({"action":"draw"}));
    let bytes = shared.lock().unwrap().archive().unwrap();
    let skin = crate::winamp::skin::load_skin(&bytes).expect("the skin still loads");
    assert_eq!(
        skin.cursors.len(),
        crate::winamp::cursors::SkinCursor::COUNT,
        "every region the editor drew is readable by the player"
    );
}

#[test]
fn drawing_again_leaves_hand_drawn_artwork_alone_unless_asked() {
    let shared = blank();
    result(&shared, "studio_cursors", json!({"action":"draw"}));
    let out = result(&shared, "studio_cursors", json!({"action":"draw"}));
    assert!(out["drawn"].as_array().unwrap().is_empty(), "{out}");
    let forced = result(
        &shared,
        "studio_cursors",
        json!({"action":"draw","regions":["posbar"],"overwrite":true}),
    );
    assert_eq!(forced["drawn"], json!(["posbar.cur"]), "{forced}");
}

#[test]
fn a_region_can_be_re_aimed_and_dropped() {
    let shared = blank();
    result(&shared, "studio_cursors", json!({"action":"draw"}));
    let aimed = result(
        &shared,
        "studio_cursors",
        json!({"action":"hotspot","regions":["normal"],"hotspot":[4,9]}),
    );
    assert_eq!(aimed["hotspot"], json!([4, 9]), "{aimed}");
    let listed = result(&shared, "studio_cursors", json!({}));
    let normal = listed["regions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["region"] == json!("normal"))
        .unwrap()
        .clone();
    assert_eq!(normal["hotspot"], json!([4, 9]));

    let dropped = result(
        &shared,
        "studio_cursors",
        json!({"action":"remove","regions":["normal"]}),
    );
    assert_eq!(dropped["removed"], json!(["normal.cur"]), "{dropped}");
}

#[test]
fn an_unknown_region_is_named_rather_than_ignored() {
    let shared = blank();
    let out = call(
        "studio_cursors",
        json!({"action":"draw","regions":["nosuch"]}),
        &shared,
    );
    assert!(
        format!("{out:?}").contains("nosuch"),
        "the error should name the region: {out:?}"
    );
}

#[test]
fn studio_validate_tells_a_blank_skin_its_sprites_are_unpainted() {
    let shared = blank();
    let out = result(&shared, "studio_validate", json!({}));
    assert_eq!(out["exportable"], json!(false), "{out}");
    assert!(out["plays_the_same_elsewhere"].is_null());
    assert!(
        out["divergences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["problem"].as_str().unwrap().contains("clear")),
        "a blank skin must identify its unpainted sources: {out}"
    );
}

#[test]
fn a_state_sheet_answers_for_a_named_sprite_whatever_surface_is_open() {
    let shared = blank();
    let dir = std::env::temp_dir().join("cranamp-state-sheet-test");
    std::fs::create_dir_all(&dir).unwrap();
    result(
        &shared,
        "studio_targets",
        json!({"solo":"main.volume.track"}),
    );
    let on_canvas = result(
        &shared,
        "studio_states",
        json!({"path":dir.join("a.png").to_str().unwrap()}),
    );
    assert_eq!(on_canvas["sprite"]["id"], "main.volume.track");
    assert_eq!(on_canvas["sprite"]["of"], json!(28));
    result(&shared, "studio_atlas", json!({"sheet":"volume.bmp"}));
    let on_sheet = result(
        &shared,
        "studio_states",
        json!({"id":"volume.track","path":dir.join("b.png").to_str().unwrap()}),
    );
    assert_eq!(
        on_sheet["sprite"]["id"], "main.volume.track",
        "the sprite is the one that was chosen, not the sheet it lives on"
    );
    assert_eq!(on_sheet["sprite"]["of"], json!(28));
    assert_eq!(on_sheet["native"], on_canvas["native"]);
    let missing = call("studio_states", json!({"id":"kettle"}), &shared)
        .unwrap_err()
        .to_string();
    assert!(missing.contains("in this skin's"), "{missing}");
    let several = call("studio_states", json!({"id":"track"}), &shared)
        .unwrap_err()
        .to_string();
    assert!(several.contains("name one exactly"), "{several}");
}
#[test]
fn the_engine_measures_its_own_face() {
    let shared = blank();
    let out = result(
        &shared,
        "studio_canvas",
        json!({"face":"small","text_scale":1,"measure":["MONO","STEREO","16K"]}),
    );
    assert_eq!(out["measured"][0]["width"], json!(19), "the 27-pixel lamp");
    assert_eq!(out["measured"][1]["width"], json!(29), "the 29-pixel lamp");
    let tight = result(
        &shared,
        "studio_canvas",
        json!({"text_spacing":0,"measure":"STEREO"}),
    );
    assert_eq!(tight["measured"][0]["width"], json!(24), "at spacing 0");
    assert_eq!(tight["spacing"], json!(0), "measure reports what it used");
    let now = result(&shared, "studio_status", json!({}));
    assert_eq!(now["view"]["text_spacing"], json!(0));
    let drawn = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"text","x":0,"y":0,"text":"II","color":"#ffffff",
                             "face":"small"}]}),
    );
    let narrow = drawn["bounds"][2].as_i64().unwrap();
    let wider = result(
        &shared,
        "studio_draw",
        json!({"operations":[{"op":"text","x":0,"y":20,"text":"II","color":"#ffffff",
                             "face":"small","spacing":2}]}),
    );
    assert_eq!(
        wider["bounds"][2].as_i64().unwrap(),
        narrow + 2,
        "an operation that names its own spacing still wins"
    );
    result(&shared, "studio_canvas", json!({"text_spacing": 1}));
    assert_eq!(out["measured"][2]["width"], json!(14), "a band caption");
    assert_eq!(out["measured"][0]["advance"], json!(20));
    let scaled = result(
        &shared,
        "studio_canvas",
        json!({"face":"5x7","text_scale":2,"measure":"CATAMP"}),
    );
    assert_eq!(scaled["measured"][0]["width"], json!(70));
    assert_eq!(scaled["measured"][0]["height"], json!(14));
    let missing = result(
        &shared,
        "studio_canvas",
        json!({"face":"small","measure":["A@B"]}),
    );
    assert_eq!(missing["unsupported_characters"], json!(["@"]));
}
#[test]
fn a_crop_may_be_magnified_past_the_zoom_cap() {
    let shared = blank();
    let dir = std::env::temp_dir().join("cranamp-magnify-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.png");
    let out = result(
        &shared,
        "studio_canvas",
        json!({"crop":[16,88,23,18],"magnify":32,"path":path.to_str().unwrap()}),
    );
    assert_eq!(out["size"], json!([736, 576]));
    let refused = call(
        "studio_canvas",
        json!({"magnify":32,"path":path.to_str().unwrap()}),
        &shared,
    )
    .unwrap_err();
    let refused = format!("{refused:#}");
    assert!(
        refused.contains("over the 2048 pixel limit") && refused.contains("fits 5 times"),
        "a refusal says how far it would fit: {refused}"
    );
    std::fs::remove_file(&path).ok();
}
#[test]
fn a_catalogue_filter_takes_several_needles() {
    let shared = blank();
    let out = result(
        &shared,
        "studio_targets",
        json!({"id":["volume.track","balance.track","position.thumb"]}),
    );
    let ids: Vec<&str> = out["sprites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec![
            "main.position.thumb",
            "main.volume.track",
            "main.balance.track"
        ]
    );
}
#[test]
fn a_panel_and_a_crop_compose_in_native_coordinates() {
    assert_eq!(
        panel_canvas("main", Some(json!([14, 86, 146, 22])), 145).unwrap(),
        [14, 86, 146, 22]
    );
    assert_eq!(
        panel_canvas("equalizer", Some(json!([21, 38, 14, 63])), 145).unwrap(),
        [21, 154, 14, 63]
    );
    assert_eq!(
        panel_canvas("playlist", None, 261).unwrap(),
        [0, 232, 275, 261]
    );
    let outside = panel_canvas("main", Some(json!([14, 110, 146, 22])), 145)
        .unwrap_err()
        .to_string();
    assert!(
        outside.contains("inside the main panel, which is 275x116 native"),
        "{outside}"
    );
}
#[test]
fn rectangles_answer_what_a_band_would_cross() {
    let shared = blank();
    let out = result(&shared, "studio_rectangles", json!({"at":[0,52,275,20]}));
    let ids: Vec<&str> = out["rectangles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"runtime.main.SPECTRUM"), "{ids:?}");
    assert!(ids.contains(&"main.volume.track"), "{ids:?}");
    assert!(!ids.contains(&"main.position.track"), "{ids:?}");
    assert!(!ids.contains(&"main.play"), "{ids:?}");
}
