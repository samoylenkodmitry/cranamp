use super::*;

#[test]
fn readability_samples_visible_paint_layers_like_the_exported_player_skin() {
    let mut d = document();
    d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#101010"}]})).unwrap();
    let base_ink = d.display_ink();
    d.paint_layer_command(&json!({"action":"add","name":"Warm phosphor"}), "MCP")
        .unwrap();
    d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#fff0ca"}]})).unwrap();
    assert_ne!(d.display_ink(), base_ink);
    for opacity in [255, 128, 0] {
        d.paint_layer_command(&json!({"action":"set","opacity":opacity}), "MCP")
            .unwrap();
        let reloaded = Document::open(&d.archive().unwrap(), None).unwrap();
        let ink = reloaded.display_ink();
        let report = d.readability();
        let title = report
            .iter()
            .find(|r| r["reads"] == "the track title")
            .unwrap();
        assert_eq!(
            title["ink"],
            format!("#{:02x}{:02x}{:02x}", ink[0], ink[1], ink[2])
        );
    }
    d.paint_layer_command(
        &json!({"action":"set","opacity":255,"visible":false}),
        "MCP",
    )
    .unwrap();
    assert_eq!(d.display_ink(), base_ink);
}
fn document() -> Document {
    Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap()
}
/// The bundled skin with a `NORMAL.CUR` written alongside it.
fn document_with_a_cursor() -> Document {
    let source = include_bytes!("../../../../../assets/winamp.wsz");
    let cursor = crate::winamp::cursors::tests::cursor_file(&[
        crate::winamp::cursors::tests::monochrome_2x2(),
    ]);
    let mut zip = zip::ZipArchive::new(Cursor::new(source.as_slice())).unwrap();
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).unwrap();
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            writer.start_file(name, opts).unwrap();
            writer.write_all(&data).unwrap();
        }
        writer.start_file("NORMAL.CUR", opts).unwrap();
        writer.write_all(&cursor).unwrap();
        writer.finish().unwrap();
    }
    Document::open(&output.into_inner(), None).unwrap()
}

/// A cursor arrives as an ordinary sheet, so every tool in the editor draws
/// on it, and it leaves as a cursor again rather than as a bitmap.
#[test]
fn a_cursor_edited_here_leaves_the_archive_as_a_cursor() {
    let mut document = document_with_a_cursor();
    document
        .images
        .get_mut("normal.cur")
        .expect("the cursor opened as an editable sheet")
        .put_pixel(0, 0, Rgba([10, 200, 30, 255]));

    let bytes = document.archive().expect("archive");
    let reopened = Document::open(&bytes, None).expect("reopen");
    let cursor = reopened.images.get("normal.cur").expect("cursor survives");
    assert_eq!(cursor.get_pixel(0, 0).0, [10, 200, 30, 255]);
}

/// The hotspot is the one thing a cursor carries that a bitmap does not,
/// and nothing in the editor touches it, so it has to come back unchanged.
#[test]
fn editing_a_cursor_leaves_its_hotspot_where_it_was() {
    let mut document = document_with_a_cursor();
    let before = cursor_hotspot(document.files.get("normal.cur").expect("cursor file"));
    document
        .images
        .get_mut("normal.cur")
        .expect("cursor sheet")
        .put_pixel(1, 1, Rgba([1, 2, 3, 255]));

    let bytes = document.archive().expect("archive");
    let reopened = Document::open(&bytes, None).expect("reopen");
    let after = cursor_hotspot(reopened.files.get("normal.cur").expect("cursor file"));
    assert_eq!(before, [1, 0], "the fixture's hotspot");
    assert_eq!(after, before);
}

/// A skin's cursors are part of the skin. The Studio draws bitmaps, so it
/// must carry the `.cur` files through untouched rather than dropping them
/// on the way out.
#[test]
fn a_skin_edited_here_keeps_the_cursors_it_arrived_with() {
    let mut document = document_with_a_cursor();
    document
        .draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"}]}))
        .unwrap();

    for (what, bytes) in [
        ("the export", document.archive().unwrap()),
        ("the live preview", document.preview_archive().unwrap()),
    ] {
        let skin = crate::winamp::skin::load_skin(&bytes)
            .unwrap_or_else(|error| panic!("{what} should load: {error:#}"));
        assert!(
            skin.cursors
                .get(crate::winamp::cursors::SkinCursor::MainWindow)
                .is_some(),
            "{what} dropped the skin's cursor"
        );
    }
}

/// Making a skin portable drops the entries no player reads. A cursor is
/// read, so the repair has to leave it alone.
#[test]
fn making_a_skin_portable_keeps_its_cursors() {
    let mut document = document_with_a_cursor();

    let report = document.make_portable("test").unwrap();

    assert!(
        document.files.contains_key("normal.cur"),
        "the repair dropped the skin's cursor: {report}"
    );
}

#[test]
fn magenta_is_a_sprite_hole_in_composition_and_literal_ink_in_the_atlas() {
    let mut d = Document::blank();
    for p in d.images.get_mut("main.bmp").unwrap().pixels_mut() {
        *p = Rgba([20, 30, 40, 255]);
    }
    for p in d.images.get_mut("cbuttons.bmp").unwrap().pixels_mut() {
        *p = Rgba([255, 0, 255, 255]);
    }
    d.state(json!({"panel":"canvas"})).unwrap();
    assert_eq!(
        d.render().get_pixel(40, 90),
        &Rgba([20, 30, 40, 255]),
        "Cranamp's sprite key must reveal the local artwork underneath"
    );
    d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layers":[]}))
        .unwrap();
    assert_eq!(d.render().get_pixel(23, 0), &Rgba([255, 0, 255, 255]));
}
#[test]
fn a_patch_of_the_canvas_is_the_same_picture_as_the_whole_render_of_it() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#204050"},
    ]}))
    .unwrap();
    d.state(json!({"panel":"canvas"})).unwrap();
    for (i, (opacity, clip_below, visible)) in
        [(255u8, false, true), (140, true, true), (255, false, false)]
            .into_iter()
            .enumerate()
    {
        let made = d
            .paint_layer_command(&json!({"action":"add","name":"plane"}), "MCP")
            .unwrap();
        let id = made["active"].as_str().unwrap().to_string();
        let left = 20 + i as i64 * 34;
        d.draw(&json!({"operations":[
            {"op":"ellipse","x":left,"y":20,"width":90,"height":60,"color":"#c08040","fill":true},
            {"op":"rect","x":left + 20,"y":30,"width":30,"height":10,"color":"#ffffff","opacity":128},
        ]}))
            .unwrap();
        d.paint_layer_command(
            &json!({"action":"set","id":id,"opacity":opacity,
                    "clip_below":clip_below,"visible":visible}),
            "MCP",
        )
        .unwrap();
    }
    let composite = d.composite_images();
    let mut differs = 0;
    for (name, sheet) in &composite {
        let stack = d.sheet_stack(name).expect("every sheet has a stack");
        for (x, y, pixel) in sheet.enumerate_pixels() {
            assert_eq!(stack.at(x, y), Some(*pixel), "{name} at {x},{y}");
            if d.images[name].get_pixel(x, y) != pixel {
                differs += 1;
            }
        }
    }
    assert!(
        differs > 2000,
        "the planes have to actually change the sheets or this proves nothing: {differs}"
    );
    let whole = d.render();
    for area in [
        [0, 0, 275, 377],
        [30, 25, 40, 30],
        [-6, -6, 20, 20],
        [260, 360, 40, 40],
    ] {
        let patch = d.render_patch(area);
        for y in area[1].max(0)..(area[1] + area[3]).min(377) {
            for x in area[0].max(0)..(area[0] + area[2]).min(275) {
                assert_eq!(
                    patch.at(x, y),
                    Some(whole.get_pixel(x as u32, y as u32).0),
                    "patch {area:?} disagrees at {x},{y}"
                );
            }
        }
    }
    assert_eq!(d.render_patch([30, 25, 4, 4]).at(29, 25), None);
}
#[test]
fn a_blended_image_sees_what_the_same_transaction_painted_under_it() {
    use base64::Engine as _;
    let mut d = Document::blank();
    let translucent = |w: u32, h: u32, alpha: u8| {
        let mut im = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, alpha]));
        im.put_pixel(0, 0, Rgba([255, 255, 255, alpha]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(im)
            .write_to(&mut out, image::ImageFormat::Png)
            .unwrap();
        json!({"op":"image","x":0,"y":0,
               "data":base64::engine::general_purpose::STANDARD.encode(out.into_inner())})
    };
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":40,"height":8,"color":"#ffffff"},
        translucent(8, 8, 128),
        {"op":"rect","x":0,"y":0,"width":40,"height":8,"color":"#000000"},
        translucent(40, 8, 128),
    ]}))
    .unwrap();
    for x in [0, 9, 20, 39] {
        assert_eq!(
            d.images["main.bmp"].get_pixel(x, 4),
            &Rgba([128, 128, 128, 255]),
            "column {x} blended into a stale backdrop"
        );
    }
}
#[test]
fn a_draw_reports_the_operation_that_failed_the_glyphs_it_skipped_and_ink_nothing_samples() {
    let mut d = document();
    let error = d
        .draw(&json!({"operations":[
            {"op":"pixel","x":1,"y":1,"color":"#123456"},
            {"op":"rect","x":2,"y":2,"width":4,"height":4,"color":"#123456"},
            {"op":"nonsense","x":3,"y":3},
        ]}))
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("operations[2] \"nonsense\""),
        "{error:#}"
    );
    assert!(!d.dirty, "a refused transaction leaves nothing behind");
    d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
        .unwrap();
    let result = d
        .draw(&json!({"operations":[
            {"op":"text","x":1,"y":1,"text":"OK@ \u{00e9}","color":"#ffffff"},
        ]}))
        .unwrap();
    assert_eq!(result["unsupported_characters"], json!(["@", "é"]));
    assert!(result["pixels_written"].as_u64().unwrap() > 0);
    assert!(result["note"].as_str().unwrap().contains("not drawn"));
    d.state(json!({"panel":"atlas","sheet":"pledit.bmp","layer":"sheet"}))
        .unwrap();
    let result = d
        .draw(&json!({"operations":[
            {"op":"rect","x":120,"y":72,"width":10,"height":38,"color":"#123456"},
        ]}))
        .unwrap();
    assert_eq!(result["unsampled_pixels"], json!(38));
    assert!(result["unsampled_sample"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p[0] == json!(125)));
}
#[test]
fn grain_is_deterministic_opacity_bakes_a_blend_and_preview_leaves_no_trace() {
    let field = json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":60,"height":20,"color":"#808080",
         "grain":12,"grain_seed":7},
    ]});
    let mut a = Document::blank();
    let mut b = Document::blank();
    for d in [&mut a, &mut b] {
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&field).unwrap();
    }
    assert_eq!(a.images["main.bmp"], b.images["main.bmp"]);
    let grained = (0..60)
        .map(|x| a.images["main.bmp"].get_pixel(x, 4)[0])
        .collect::<std::collections::BTreeSet<_>>();
    assert!(grained.len() > 6, "grain should vary: {grained:?}");
    assert!(
        grained.iter().all(|v| (116..=140).contains(v)),
        "grain should stay inside its amplitude: {grained:?}"
    );
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":10,"height":10,"color":"#000000"},
        {"op":"rect","x":0,"y":0,"width":10,"height":10,"color":"#ffffff","opacity":128},
    ]}))
    .unwrap();
    assert_eq!(
        d.images["main.bmp"].get_pixel(5, 5),
        &Rgba([128, 128, 128, 255])
    );
    let before = d.snapshot();
    let revision = d.revision;
    let undo = d.undo.len();
    let report = d
        .draw(&json!({"preview":true,"operations":[
            {"op":"rect","x":20,"y":0,"width":8,"height":8,"color":"#ff0000"},
        ]}))
        .unwrap();
    assert_eq!(report["preview"], json!(true));
    assert_eq!(report["bounds"], json!([20, 0, 27, 7]));
    let image = d.preview.take().expect("a preview answers with an image");
    assert_eq!(image.get_pixel(24, 4), &Rgba([255, 0, 0, 255]));
    assert!(d.snapshot() == before, "a preview changes nothing");
    assert_eq!((d.revision, d.undo.len()), (revision, undo));
    assert_eq!(d.images["main.bmp"].get_pixel(24, 4)[3], 0);
}
#[test]
fn brush_batch_updates_revision_and_history_once() {
    let mut d = document();
    let before = d.snapshot();
    let revision = d.revision;
    d.draw(&json!({"layer":"background","operations":[
        {"op":"rect","x":1,"y":1,"width":200,"height":80,"color":"#123456"},
        {"op":"stamp","x":20,"y":20,"rows":["abba","baab"],"palette":{"a":"#abcdef","b":"#fedcba"}}
    ]}))
    .unwrap();
    assert_eq!(d.revision, revision + 1);
    assert_eq!(d.undo.len(), 1);
    assert!(d.dirty);
    assert!(d.undo());
    assert!(d.snapshot() == before);
    assert!(!d.dirty);
}
#[test]
fn canvas_stroke_maps_to_play_button_and_all_pressed_variants() {
    let mut d = document();
    let before = d.images["cbuttons.bmp"].clone();
    d.draw(&json!({"layer":"play","all_states":true,"operations":[{"op":"line","x":40,"y":90,"x2":44,"y2":90,"color":"#123456"}]})).unwrap();
    for y in [2, 20] {
        for x in 24..=28 {
            assert_eq!(
                d.images["cbuttons.bmp"].get_pixel(x, y).0,
                [18, 52, 86, 255]
            );
        }
    }
    assert_eq!(
        d.images["cbuttons.bmp"].get_pixel(0, 0),
        before.get_pixel(0, 0)
    );
    assert!(d.undo());
    assert_eq!(d.images["cbuttons.bmp"], before);
    assert!(d.redo());
    assert_eq!(
        d.images["cbuttons.bmp"].get_pixel(24, 2).0,
        [18, 52, 86, 255]
    );
}
#[test]
fn every_volume_frame_receives_the_same_local_pixel() {
    let mut d = document();
    d.draw(&json!({"layer":"volume.track","all_states":true,"operations":[{"op":"pixel","x":110,"y":58,"color":"#192837"}]})).unwrap();
    for frame in 0..28 {
        assert_eq!(
            d.images["volume.bmp"].get_pixel(3, frame * 15 + 1).0,
            [25, 40, 55, 255]
        );
    }
}
#[test]
fn shared_eq_frames_and_heads_cover_both_rows_of_the_atlas() {
    let mut d = document();
    d.state(json!({"panel":"equalizer","eq":vec![14;11]}))
        .unwrap();
    d.draw(&json!({"layer":"band1.track","all_states":true,"operations":[{"op":"pixel","x":79,"y":40,"color":"#aabbcc"}]})).unwrap();
    for frame in 0..28 {
        let x = 14 + (frame % 14) * 15;
        let y = if frame < 14 { 166 } else { 231 };
        assert_eq!(
            d.images["eqmain.bmp"].get_pixel(x, y).0,
            [170, 187, 204, 255]
        );
    }
}
#[test]
fn failed_multi_operation_transaction_is_atomic() {
    let mut d = document();
    let before = d.archive().unwrap();
    let rev = d.revision;
    assert!(d
        .draw(
            &json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"},{"op":"unknown"}]})
        )
        .is_err());
    assert_eq!(d.archive().unwrap(), before);
    assert_eq!(d.revision, rev);
    assert!(!d.dirty);
    assert_eq!(d.undo.len(), 0);
}
#[test]
fn renderer_pixel_and_inverse_mapping_agree_at_all_28_positions() {
    let mut d = document();
    for frame in 0..28 {
        d.state(json!({"volume":frame,"balance":frame,"position":frame,"scroll":frame,"eq":vec![frame;11]})).unwrap();
        for panel in ["main", "equalizer", "playlist"] {
            d.state(json!({"panel":panel})).unwrap();
            let rendered = d.render();
            for l in d.layers() {
                let x = l.destination[0];
                let y = l.destination[1];
                assert!(l.map(x, y).is_some());
                let im = &d.images[&l.sheet];
                assert!(
                    l.source[0] + l.source[2] <= im.width(),
                    "{} {:?}",
                    l.id,
                    l.source
                );
                assert!(
                    l.source[1] + l.source[3] <= im.height(),
                    "{} {:?}",
                    l.id,
                    l.source
                );
                assert!(x < rendered.width() && y < rendered.height());
            }
        }
    }
}
#[test]
fn exported_skin_round_trips_through_production_loader() {
    let mut d = document();
    d.draw(&json!({"operations":[{"op":"pixel","x":18,"y":90,"color":"#cc44aa"}]}))
        .unwrap();
    let bytes = d.archive().unwrap();
    crate::winamp::skin::load_skin(&bytes).unwrap();
    let restored = Document::open(&bytes, None).unwrap();
    assert_eq!(d.images, restored.images);
}
#[test]
fn a_stroke_shows_on_every_pixel_of_the_joined_canvas() {
    let mut d = document();
    d.state(json!({"panel":"canvas","layer":"auto","zoom":1}))
        .unwrap();
    let (w, h) = d.canvas_size();
    let ink = [18, 255, 52, 255];
    for y in 0..h {
        d.checkpoint();
        let _ = d.paint_line(
            [0, y as i32],
            [w as i32 - 1, y as i32],
            ink,
            "selection",
            Scope::Current.into(),
        );
        d.finish_stroke();
    }
    let im = d.render();
    let list_fill = 252..(h - 38);
    for y in 0..h {
        let seen = (0..w).filter(|&x| im.get_pixel(x, y).0 == ink).count();
        if list_fill.contains(&y) {
            assert!(seen > 0, "row {y} of the playlist frame took nothing");
        } else {
            assert_eq!(seen, w as usize, "row {y} did not take a full-width stroke");
        }
    }
}
#[test]
fn auto_paints_every_sprite_under_the_brush_not_only_the_top_one() {
    let mut d = document();
    let original = d.snapshot();
    assert!(d.view.layers.is_empty(), "no explicit target means Auto");
    d.checkpoint();
    d.paint_line(
        [40, 90],
        [40, 90],
        [4, 5, 6, 255],
        "selection",
        Scope::Current.into(),
    )
    .unwrap();
    assert_eq!(d.images["cbuttons.bmp"].get_pixel(24, 2).0, [4, 5, 6, 255]);
    assert_eq!(d.images["main.bmp"].get_pixel(40, 90).0, [4, 5, 6, 255]);
    assert_ne!(
        original.images["main.bmp"].get_pixel(40, 90).0,
        [4, 5, 6, 255]
    );
    d.undo();
    assert_eq!(d.images["main.bmp"], original.images["main.bmp"]);
    assert_eq!(d.images["cbuttons.bmp"], original.images["cbuttons.bmp"]);
}
#[test]
fn selected_layers_paint_occluded_background_and_both_control_states() {
    let mut d = document();
    let original = d.snapshot();
    d.state(json!({"layers":["background","play","play"],"all_states":true}))
        .unwrap();
    assert_eq!(d.view.layers, vec!["background", "play"]);
    d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"}]}))
        .unwrap();
    assert_eq!(d.images["main.bmp"].get_pixel(40, 90).0, [18, 52, 86, 255]);
    for y in [2, 20] {
        assert_eq!(
            d.images["cbuttons.bmp"].get_pixel(24, y).0,
            [18, 52, 86, 255]
        );
    }
    assert_eq!(d.images["titlebar.bmp"], original.images["titlebar.bmp"]);
    d.undo();
    assert!(!d.dirty);
    d.state(json!({"layers":["play"]})).unwrap();
    d.checkpoint();
    d.paint_line(
        [40, 90],
        [40, 90],
        [4, 5, 6, 255],
        "selection",
        Scope::Current.into(),
    )
    .unwrap();
    assert_eq!(d.images["main.bmp"], original.images["main.bmp"]);
    assert_eq!(d.images["cbuttons.bmp"].get_pixel(24, 2).0, [4, 5, 6, 255]);
}
#[test]
fn selection_validation_and_draw_override_are_atomic() {
    let mut d = document();
    d.state(json!({"layers":["background","play"]})).unwrap();
    assert!(d.state(json!({"layers":["play","invalid"]})).is_err());
    assert_eq!(d.view.layers, vec!["background", "play"]);
    let before = d.snapshot();
    assert!(d.draw(&json!({"layers":["play"],"operations":[{"x":40,"y":90,"color":"#123456"},{"op":"invalid"}]})).is_err());
    assert!(before == d.snapshot());
    assert_eq!(d.view.layers, vec!["background", "play"]);
    d.draw(&json!({"layers":["play"],"operations":[{"x":40,"y":90,"color":"#123456"}]}))
        .unwrap();
    assert_eq!(d.images["main.bmp"], before.images["main.bmp"]);
    assert_eq!(d.view.layers, vec!["background", "play"]);
    d.state(json!({"panel":"equalizer"})).unwrap();
    assert!(d.view.layers.is_empty());
}
#[test]
fn history_navigation_branching_and_saved_state() {
    let mut d = document();
    let initial = d.snapshot();
    for color in ["#111111", "#222222", "#333333"] {
        d.draw(&json!({"label":color,"operations":[{"x":40,"y":90,"color":color}]}))
            .unwrap();
    }
    d.history_goto(1).unwrap();
    let temp = std::env::temp_dir().join(format!("cranamp-history-{}.wsz", std::process::id()));
    d.export(&temp).unwrap();
    assert!(!d.dirty);
    d.history_goto(3).unwrap();
    assert!(d.dirty);
    d.history_goto(1).unwrap();
    assert!(!d.dirty);
    d.history_goto(0).unwrap();
    assert!(initial == d.snapshot());
    assert!(d.dirty);
    d.history_goto(1).unwrap();
    d.draw(&json!({"label":"New branch","operations":[{"x":40,"y":90,"color":"#445566"}]}))
        .unwrap();
    let history = d.history();
    assert_eq!(history["cursor"], 2);
    assert_eq!(history["entries"].as_array().unwrap().len(), 3);
    assert_eq!(history["entries"][2]["source"], "MCP");
    assert_eq!(history["entries"][2]["label"], "New branch");
    assert!(!d.redo());
    assert!(d.history_goto(3).is_err());
    std::fs::remove_file(temp).unwrap();
}
#[test]
fn empty_human_gesture_preserves_redo_and_does_not_add_history() {
    let mut d = document();
    d.draw(&json!({"operations":[{"x":40,"y":90,"color":"#123456"}]}))
        .unwrap();
    d.undo();
    d.state(json!({"layers":["play"]})).unwrap();
    d.checkpoint();
    d.paint_line(
        [0, 0],
        [2, 2],
        [1, 2, 3, 255],
        "selection",
        Scope::Current.into(),
    )
    .unwrap();
    d.finish_stroke();
    assert_eq!(d.history()["cursor"], 0);
    assert!(!d.dirty);
    assert!(d.redo());
    d.checkpoint();
    d.paint_line(
        [40, 90],
        [41, 90],
        [1, 2, 3, 255],
        "selection",
        Scope::Current.into(),
    )
    .unwrap();
    d.paint_line(
        [41, 90],
        [43, 90],
        [1, 2, 3, 255],
        "selection",
        Scope::Current.into(),
    )
    .unwrap();
    d.finish_stroke();
    assert_eq!(d.history()["cursor"], 2);
    assert_eq!(d.history()["entries"][2]["source"], "Human");
}
#[test]
fn visualizer_palette_round_trips_and_shares_history() {
    let mut d = document();
    let original = d.archive().unwrap();
    assert!(d
        .visualizer_palette(&json!({"colors":["#123456"]}))
        .is_err());
    assert_eq!(d.archive().unwrap(), original);
    d.visualizer_palette(&json!({"colors":vec!["#123456";24]}))
        .unwrap();
    let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
    assert_eq!(skin.viscolor.0, [[18, 52, 86, 255]; 24]);
    assert_eq!(d.history()["entries"][1]["label"], "Visualizer colors");
    d.undo();
    assert_eq!(d.archive().unwrap(), original);
}
#[test]
fn color_parser_rejects_non_ascii_without_panicking() {
    assert!(parse_color("#ééé").is_err());
    assert_eq!(parse_color("transparent").unwrap(), [255, 0, 255, 0]);
}
#[test]
fn pressed_player_preview_uses_pressed_art_without_mutating_the_document() {
    let mut d = document();
    d.state(json!({"panel":"main","layer":"play","pressed":true}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"}]}))
        .unwrap();
    let original = d.archive().unwrap();
    let history = d.history();
    let preview = Document::open(&d.preview_archive().unwrap(), None).unwrap();
    assert_eq!(
        preview.images["cbuttons.bmp"].get_pixel(24, 2).0,
        [18, 52, 86, 255]
    );
    assert_eq!(d.archive().unwrap(), original);
    assert_eq!(d.history(), history);
    d.state(json!({"pressed":false,"active":true})).unwrap();
    let preview = Document::open(&d.preview_archive().unwrap(), None).unwrap();
    assert_eq!(
        preview.images["cbuttons.bmp"].get_pixel(24, 2),
        d.images["cbuttons.bmp"].get_pixel(24, 2)
    );
    assert_eq!(d.archive().unwrap(), original);
}
#[test]
fn whole_skin_canvas_keeps_definitions_and_maps_cross_panel_history() {
    let mut d = Document::blank();
    let archive = d.archive().unwrap();
    d.state(json!({"panel":"canvas","preview_playlist_height":145,"layers":["main.background","equalizer.background"]})).unwrap();
    assert_eq!(d.canvas_size(), (275, 377));
    assert_eq!(
        d.archive().unwrap(),
        archive,
        "An editor view must not change skin definitions"
    );
    assert!(d.layers().iter().all(|l| !l.stretched()));
    d.draw(
        &json!({"operations":[{"op":"line","x":100,"y":113,"x2":100,"y2":118,"color":"#abcdef"}]}),
    )
    .unwrap();
    assert_eq!(
        d.images["main.bmp"].get_pixel(100, 114).0,
        [171, 205, 239, 255]
    );
    assert_eq!(
        d.images["eqmain.bmp"].get_pixel(100, 0).0,
        [171, 205, 239, 255]
    );
    assert_eq!(
        d.images["eqmain.bmp"].get_pixel(100, 2).0,
        [171, 205, 239, 255]
    );
    d.undo();
    assert_eq!(d.archive().unwrap(), archive);
    d.redo();
    let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
    assert_eq!(skin.main.width(), 275);
    assert_eq!(skin.eqmain.height(), 315);
    assert!(!d.files.contains_key("cranamp.json"));
    assert!(!d.files.contains_key("canvas.bmp"));
    d.state(json!({"layers":["playlist.top.tile"],"all_states":true}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"pixel","x":51,"y":236,"color":"#123456"}]}))
        .unwrap();
    assert_eq!(
        d.images["pledit.bmp"].get_pixel(128, 4).0,
        [18, 52, 86, 255]
    );
    assert_eq!(
        d.images["pledit.bmp"].get_pixel(128, 25).0,
        [18, 52, 86, 255]
    );
    let layers = d.layers();
    let scroll = layers
        .iter()
        .find(|l| l.id == "playlist.scroll.thumb")
        .unwrap();
    assert_eq!(scroll.destination, [260, 252, 8, 18]);
    let footer = layers
        .iter()
        .find(|l| l.id == "playlist.bottom.left")
        .unwrap();
    assert_eq!(footer.destination, [0, 339, 125, 38]);
}
#[test]
fn thirty_three_independent_planes_roundtrip_without_flattening() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    for i in 1..=33 {
        d.paint_layer_command(
            &json!({"action":"add","name":format!("Detail {i}")}),
            "Human",
        )
        .unwrap();
    }
    d.draw(&json!({"operations":[{"op":"pixel","x":20,"y":30,"color":"#123456"}]}))
        .unwrap();
    assert_eq!(d.view.paint_layer.as_deref(), Some("paint-33"));
    let file = std::env::temp_dir().join(format!(
        "cranamp-33-plane-test-{}.cstudio",
        std::process::id()
    ));
    d.save_project(&file).unwrap();
    let mut reopened = Document::open_project(&std::fs::read(&file).unwrap()).unwrap();
    std::fs::remove_file(file).unwrap();
    assert!(
        reopened.planes == d.planes,
        "Every ID, property and independent pixel plane survives"
    );
    assert_eq!(reopened.planes.len(), 33);
    assert_eq!(
        reopened.planes[32].images["main.bmp"].get_pixel(20, 30).0,
        [18, 52, 86, 255]
    );
    assert_ne!(
        reopened.images["main.bmp"].get_pixel(20, 30).0,
        [18, 52, 86, 255],
        "Project must not flatten paint into original atlas"
    );
    let mut patch = reopened
        .patch(&json!({"action":"inspect","parts":[{"sheet":"main.bmp","rect":[40,40,1,1]}]}))
        .unwrap();
    patch["action"] = json!("apply");
    patch.as_object_mut().unwrap().remove("replace_layer");
    patch.as_object_mut().unwrap().remove("layer_expected");
    patch["name"] = json!("Independent patch 34");
    patch["parts"][0]["operations"] = json!([{"op":"pixel","x":40,"y":40,"color":"#abcdef"}]);
    reopened.patch(&patch).unwrap();
    assert_eq!(reopened.planes[33].id, "paint-34");
    assert_eq!(
        reopened.planes[33].images["main.bmp"].get_pixel(40, 40).0,
        [171, 205, 239, 255]
    );
    assert!(reopened.planes[..33] == d.planes);
    for i in 35..=MAX_PAINT_LAYERS {
        reopened
            .paint_layer_command(
                &json!({"action":"add","name":format!("Detail {i}")}),
                "Human",
            )
            .unwrap();
    }
    let before = reopened.snapshot();
    assert!(reopened
        .paint_layer_command(&json!({"action":"add"}), "Human")
        .is_err());
    patch["name"] = json!("Over capacity");
    patch["parts"][0]["expected"] = reopened
        .patch(&json!({"action":"inspect","parts":[{"sheet":"main.bmp","rect":[40,40,1,1]}]}))
        .unwrap()["parts"][0]["expected"]
        .clone();
    assert!(reopened.patch(&patch).is_err());
    assert!(reopened.snapshot() == before);
}
#[test]
fn clipped_planes_preserve_source_pixels_history_export_and_project() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.paint_layer_command(&json!({"action":"add","name":"Silhouette"}), "MCP")
        .unwrap();
    d.draw(
        &json!({"operations":[{"op":"rect","x":10,"y":10,"width":2,"height":2,"color":"#ffffff"}]}),
    )
    .unwrap();
    d.paint_layer_command(&json!({"action":"add","name":"Shading"}), "MCP")
        .unwrap();
    d.paint_layer_command(&json!({"action":"set","clip_below":true}), "MCP")
        .unwrap();
    d.draw(
        &json!({"operations":[{"op":"rect","x":0,"y":0,"width":32,"height":32,"color":"#123456"}]}),
    )
    .unwrap();
    assert_eq!(d.composite_images()["main.bmp"].get_pixel(0, 0).0, [0; 4]);
    assert_eq!(
        d.composite_images()["main.bmp"].get_pixel(10, 10).0,
        [18, 52, 86, 255]
    );
    assert_eq!(
        d.planes[1].images["main.bmp"].get_pixel(0, 0).0,
        [18, 52, 86, 255],
        "clipping must not erase stored painting"
    );
    d.paint_layer_command(&json!({"action":"set","clip_below":false}), "Human")
        .unwrap();
    assert_eq!(
        d.composite_images()["main.bmp"].get_pixel(0, 0).0,
        [18, 52, 86, 255]
    );
    d.undo();
    let original = d.archive().unwrap();
    d.paint_layer_command(
        &json!({"action":"set","id":"paint-1","visible":false}),
        "Human",
    )
    .unwrap();
    assert_eq!(d.composite_images()["main.bmp"].get_pixel(10, 10).0, [0; 4]);
    d.undo();
    let file = std::env::temp_dir().join(format!(
        "cranamp-clipped-test-{}.cstudio",
        std::process::id()
    ));
    d.save_project(&file).unwrap();
    let reopened = Document::open_project(&std::fs::read(&file).unwrap()).unwrap();
    std::fs::remove_file(file).unwrap();
    assert!(reopened.planes[1].clip_below);
    assert_eq!(reopened.archive().unwrap(), original);
    assert!(reopened.planes == d.planes);
    d.paint_layer_command(&json!({"action":"merge_down"}), "Human")
        .unwrap();
    assert_eq!(d.archive().unwrap(), original);
    d.undo();
    assert!(d.planes[1].clip_below);
    d.paint_layer_command(&json!({"action":"add","name":"Glints"}), "MCP")
        .unwrap();
    d.paint_layer_command(&json!({"action":"set","clip_below":true}), "MCP")
        .unwrap();
    d.draw(
        &json!({"operations":[{"op":"rect","x":0,"y":0,"width":32,"height":32,"color":"#aabbcc"}]}),
    )
    .unwrap();
    assert_eq!(d.composite_images()["main.bmp"].get_pixel(0, 0).0, [0; 4]);
    assert_eq!(
        d.composite_images()["main.bmp"].get_pixel(10, 10).0,
        [170, 187, 204, 255]
    );
    assert!(d
        .paint_layer_command(&json!({"action":"merge_down","id":"paint-2"}), "Human")
        .is_err());
}
#[test]
fn a_hand_stroke_can_make_a_gradient_a_word_and_a_glass_of_its_own() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.state(json!({"brush":"rect","filled":true,"color":"#ffffff",
                   "ramp_to":"#000000","ramp_axis":"down"}))
        .unwrap();
    d.checkpoint();
    d.shape_stroke([0, 0], [39, 39]).unwrap();
    d.finish_stroke();
    let top = d.images["main.bmp"].get_pixel(20, 1).0;
    let bottom = d.images["main.bmp"].get_pixel(20, 38).0;
    assert!(top[0] > 200 && bottom[0] < 60, "{top:?} -> {bottom:?}");
    d.state(json!({"ramp_to":null})).unwrap();
    d.checkpoint();
    d.shape_stroke([0, 0], [39, 39]).unwrap();
    d.finish_stroke();
    assert_eq!(
        d.images["main.bmp"].get_pixel(20, 1).0,
        d.images["main.bmp"].get_pixel(20, 38).0
    );
    d.state(
        json!({"brush":"text","text":"16K","face":"small","text_scale":1,
                   "color":"#ff0000"}),
    )
    .unwrap();
    d.checkpoint();
    d.shape_stroke([50, 50], [50, 50]).unwrap();
    d.finish_stroke();
    let painted = (50..64)
        .flat_map(|x| (50..56).map(move |y| (x, y)))
        .filter(|(x, y)| d.images["main.bmp"].get_pixel(*x, *y).0 == [255, 0, 0, 255])
        .count();
    assert!(painted > 10, "the word landed: {painted} pixels");
    assert_eq!(
        d.images["main.bmp"].get_pixel(64, 52).0[3],
        0,
        "and it fits the fourteen pixels an equalizer caption has"
    );
    d.state(json!({"brush":"glass","color":"#66ccff","bevel":4,"refraction":0}))
        .unwrap();
    d.checkpoint();
    d.shape_stroke([100, 20], [160, 80]).unwrap();
    d.finish_stroke();
    let narrow = d.archive().unwrap();
    d.undo();
    d.state(json!({"bevel":32})).unwrap();
    d.checkpoint();
    d.shape_stroke([100, 20], [160, 80]).unwrap();
    d.finish_stroke();
    assert_ne!(
        d.archive().unwrap(),
        narrow,
        "a bevel the artist chose has to change the glass"
    );
}
#[test]
fn clean_curve_gui_preview_matches_mcp_and_undo() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","brush":"curve","brush_size":1,"curve_bend":45,"clean_corners":true,"color":"#abcdef"})).unwrap();
    d.paint_layer_command(&json!({"action":"add","name":"Whiskers"}), "Human")
        .unwrap();
    let before = d.archive().unwrap();
    d.checkpoint();
    d.shape_stroke([10, 40], [60, 48]).unwrap();
    d.shape_stroke([10, 40], [30, 48]).unwrap();
    d.finish_stroke();
    let human = d.archive().unwrap();
    assert_ne!(before, human);
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
    d.draw(&json!({"operations":[{"op":"curve","x":10,"y":40,"x2":30,"y2":48,"curve_bend":45,"color":"#abcdef"}]})).unwrap();
    assert_eq!(d.archive().unwrap(), human);
    d.undo();
    d.draw(&json!({"operations":[{"op":"curve","x":10,"y":40,"x2":30,"y2":48,"curve_bend":45,"clean_corners":false,"color":"#abcdef"}]})).unwrap();
    assert_ne!(
        d.archive().unwrap(),
        human,
        "operation overrides shared brush setting"
    );
}
#[test]
fn human_tuft_preview_and_mcp_share_pixels_and_atomic_layer_history() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","brush":"tuft","brush_size":5,"curve_bend":0,"color":"#abcdef"})).unwrap();
    d.paint_layer_command(&json!({"action":"add","name":"Fur"}), "MCP")
        .unwrap();
    let before = d.archive().unwrap();
    d.checkpoint();
    d.shape_stroke([10, 40], [60, 40]).unwrap();
    d.shape_stroke([10, 40], [30, 40]).unwrap();
    assert_eq!(d.composite_images()["main.bmp"].get_pixel(50, 40).0, [0; 4]);
    d.finish_stroke();
    let human = d.archive().unwrap();
    assert_eq!(d.images["main.bmp"].get_pixel(10, 40).0, [0; 4]);
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
    d.draw(&json!({"operations":[{"op":"tuft","x":10,"y":40,"x2":30,"y2":40,"brush_size":5,"curve_bend":0,"color":"#abcdef"}]})).unwrap();
    assert_eq!(d.archive().unwrap(), human);
    assert!(d.draw(&json!({"operations":[{"op":"pixel","x":80,"y":80},{"op":"curve","x":0,"y":0,"curve_bend":101}]})).is_err());
    assert_eq!(
        d.archive().unwrap(),
        human,
        "invalid operations restore all layers atomically"
    );
}
#[test]
fn native_paths_ramps_mirrors_and_shape_preview_share_history() {
    let mut d = Document::blank();
    d.state(
        json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","brush":"ellipse","filled":true}),
    )
    .unwrap();
    d.draw(&json!({"operations":[{"op":"path","x":0,"y":0,"points":[[10,10],[10,25,30,25,30,10],[10,10]],"fill":true,"mirror_x":true,"ramp":["#123456","#abcdef"],"ramp_axis":[10,10,30,10]}]})).unwrap();
    assert_eq!(d.images["main.bmp"].get_pixel(10, 10).0, [18, 52, 86, 255]);
    assert_eq!(d.images["main.bmp"].get_pixel(264, 10).0, [18, 52, 86, 255]);
    assert_eq!(
        d.images["main.bmp"].get_pixel(30, 10).0,
        [171, 205, 239, 255]
    );
    assert_eq!(d.images["main.bmp"].get_pixel(20, 30).0, [0; 4]);
    let before = d.archive().unwrap();
    let history = d.undo.len();
    d.checkpoint();
    d.shape_stroke([40, 40], [60, 60]).unwrap();
    assert_ne!(d.images["main.bmp"].get_pixel(50, 50).0, [0; 4]);
    d.shape_stroke([40, 40], [44, 44]).unwrap();
    assert_eq!(
        d.images["main.bmp"].get_pixel(50, 50).0,
        [0; 4],
        "old shape preview must be erased"
    );
    d.finish_stroke();
    assert_eq!(d.undo.len(), history + 1);
    assert_eq!(d.undo.last().unwrap().source, "Human");
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
    d.checkpoint();
    d.shape_stroke([40, 40], [60, 60]).unwrap();
    d.cancel_stroke();
    assert_eq!(d.archive().unwrap(), before);
    assert!(d
        .draw(&json!({"operations":[{"op":"path","x":0,"y":0,"points":[[0,0],[5]],"fill":true}]}))
        .is_err());
    assert_eq!(d.archive().unwrap(), before, "invalid paths are atomic");
}
#[test]
fn lifted_clusters_preserve_transparency_and_native_history() {
    let mut d = Document::blank();
    d.state(json!({"panel":"main","layers":["background"]}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"stamp","x":30,"y":40,"rows":["ab.",".ba"],"palette":{"a":"#aabbcc","b":"#123456"}}]})).unwrap();
    let before = d.archive().unwrap();
    let history = d.undo.len();
    d.capture_cluster([30, 40, 3, 2]).unwrap();
    assert_eq!(d.cluster.as_ref().unwrap().get_pixel(2, 0).0, [0; 4]);
    assert_eq!(d.archive().unwrap(), before);
    assert_eq!(d.undo.len(), history);
    d.transform_cluster(true, false, 1).unwrap();
    assert_eq!(d.cluster.as_ref().unwrap().dimensions(), (2, 3));
    d.state(json!({"panel":"main","layers":["play"],"all_states":true}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"cluster","x":40,"y":90}]}))
        .unwrap();
    assert_eq!(d.undo.len(), history + 1);
    for y in 0..3 {
        for x in 0..2 {
            let expected = d.cluster.as_ref().unwrap().get_pixel(x, y);
            assert_eq!(d.images["cbuttons.bmp"].get_pixel(24 + x, 2 + y), expected);
            assert_eq!(d.images["cbuttons.bmp"].get_pixel(24 + x, 20 + y), expected);
        }
    }
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
    d.state(json!({"panel":"main","layers":["background"],"brush":"lift"}))
        .unwrap();
    d.checkpoint();
    d.shape_stroke([30, 40], [32, 41]).unwrap();
    d.finish_stroke();
    assert_eq!(d.archive().unwrap(), before);
    assert_eq!(d.undo.len(), history);
    d.state(json!({"brush":"stamp"})).unwrap();
    d.checkpoint();
    d.shape_stroke([50, 40], [50, 40]).unwrap();
    d.shape_stroke([50, 40], [60, 40]).unwrap();
    d.finish_stroke();
    assert_eq!(
        d.images["main.bmp"].get_pixel(50, 40).0,
        [0; 4],
        "old stamp preview erased"
    );
    assert_eq!(
        d.images["main.bmp"].get_pixel(60, 40).0,
        [170, 187, 204, 255]
    );
    assert_eq!(d.undo.last().unwrap().source, "Human");
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
}
#[test]
fn alpha_and_palette_masks_protect_each_source_variant() {
    let mut d = Document::blank();
    d.state(json!({"panel":"main","layer":"play","all_states":true}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"pixel","x":42,"y":92,"color":"#112233"}]}))
        .unwrap();
    let before = d.archive().unwrap();
    d.state(json!({"alpha_lock":true})).unwrap();
    d.draw(&json!({"mask_colors":["#112233"],"operations":[{"op":"rect","x":39,"y":88,"width":23,"height":18,"color":"#abcdef"}]})).unwrap();
    assert!(d.view.mask_colors.is_empty());
    for yy in [4, 22] {
        assert_eq!(
            d.images["cbuttons.bmp"].get_pixel(26, yy).0,
            [171, 205, 239, 255]
        );
        assert_eq!(d.images["cbuttons.bmp"].get_pixel(27, yy).0, [0; 4]);
    }
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
    let rev = d.revision;
    assert!(d
        .draw(&json!({"mask_colors":["invalid"],"operations":[]}))
        .is_err());
    assert_eq!(d.revision, rev);
}
#[test]
fn glass_brush_uses_selected_underlay_and_one_human_undo() {
    let mut d = Document::blank();
    d.state(json!({"panel":"main","layers":["background"]}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#203050"}]})).unwrap();
    let before = d.archive().unwrap();
    let history = d.undo.len();
    d.state(json!({"brush":"glass","color":"#aaccdd"})).unwrap();
    d.checkpoint();
    d.shape_stroke([15, 20], [70, 50]).unwrap();
    d.shape_stroke([15, 20], [60, 45]).unwrap();
    d.finish_stroke();
    assert_eq!(d.undo.len(), history + 1);
    assert_eq!(d.undo.last().unwrap().source, "Human");
    assert_eq!(d.images["main.bmp"].get_pixel(65, 40).0, [32, 48, 80, 255]);
    assert_ne!(d.images["main.bmp"].get_pixel(35, 33).0, [32, 48, 80, 255]);
    d.undo();
    assert_eq!(d.archive().unwrap(), before);
}
#[test]
fn painting_planes_compose_lock_undo_and_roundtrip_as_project() {
    let mut d = Document::blank();
    d.state(json!({"panel":"main","layer":"play","all_states":true}))
        .unwrap();
    let blank = d.archive().unwrap();
    d.paint_layer_command(&json!({"action":"add","name":"Fur"}), "MCP")
        .unwrap();
    let fur = d.view.paint_layer.clone().unwrap();
    d.draw(&json!({"operations":[{"op":"rect","x":39,"y":88,"width":23,"height":18,"color":"#e0d0c0"}]})).unwrap();
    assert_eq!(d.images["cbuttons.bmp"].get_pixel(23, 0).0, [0; 4]);
    assert_eq!(
        d.composite_images()["cbuttons.bmp"].get_pixel(23, 0).0,
        [224, 208, 192, 255]
    );
    assert_eq!(
        d.composite_images()["cbuttons.bmp"].get_pixel(23, 18).0,
        [224, 208, 192, 255]
    );
    d.paint_layer_command(&json!({"action":"set","visible":false}), "Human")
        .unwrap();
    assert_eq!(d.archive().unwrap(), blank);
    d.undo();
    d.paint_layer_command(&json!({"action":"set","locked":true}), "Human")
        .unwrap();
    let locked = d.archive().unwrap();
    assert!(d
        .draw(&json!({"operations":[{"op":"pixel","x":40,"y":90}]}))
        .is_err());
    assert_eq!(d.archive().unwrap(), locked);
    d.undo();
    d.paint_layer_command(&json!({"action":"add","name":"Highlights"}), "MCP")
        .unwrap();
    let top = d.view.paint_layer.clone().unwrap();
    d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#ffffff"}]}))
        .unwrap();
    let n = d.undo.len();
    d.opacity_stroke(&top, 128).unwrap();
    d.opacity_stroke(&top, 200).unwrap();
    d.finish_opacity_stroke();
    assert_eq!(d.undo.len(), n + 1);
    assert_eq!(d.undo.last().unwrap().label, "Layer opacity");
    let original = d.archive().unwrap();
    let project =
        std::env::temp_dir().join(format!("cranamp-layer-test-{}.cstudio", std::process::id()));
    d.save_project(&project).unwrap();
    let restored = Document::open_project(&std::fs::read(&project).unwrap()).unwrap();
    assert_eq!(restored.archive().unwrap(), original);
    assert_eq!(restored.planes.len(), 2);
    assert_eq!(restored.view.paint_layer, Some(top.clone()));
    std::fs::remove_file(project).unwrap();
    d.paint_layer_command(&json!({"action":"merge_down","id":top}), "MCP")
        .unwrap();
    assert_eq!(d.archive().unwrap(), original);
    assert_eq!(d.view.paint_layer, Some(fur));
    d.undo();
    assert_eq!(d.planes.len(), 2);
    d.paint_layer_command(&json!({"action":"move","id":top,"index":0}), "MCP")
        .unwrap();
    assert_eq!(
        d.composite_images()["cbuttons.bmp"].get_pixel(24, 2).0,
        [224, 208, 192, 255]
    );
}
#[test]
fn the_footer_buttons_that_have_no_sprite_still_say_where_they_are() {
    let mut d = Document::blank();
    d.state(json!({"panel":"canvas","preview_playlist_height":145}))
        .unwrap();
    let guides = d.guides();
    let add = guides.iter().find(|g| g.id == "hit.playlist.ADD").unwrap();
    assert!(add.hit && !add.runtime);
    assert_eq!(add.rect, [10, 346, 28, 18]);
    let eject = guides
        .iter()
        .find(|g| g.id == "hit.playlist.EJECT")
        .unwrap();
    assert_eq!(eject.rect, [185, 364, 12, 8]);
    let elapsed = guides
        .iter()
        .find(|g| g.id == "runtime.playlist.ELAPSED")
        .unwrap();
    assert_eq!(elapsed.rect[1] + 1, eject.rect[1]);
    assert!(guides.iter().any(|g| g.id == "hit.main.SKIN CHOOSER"));
    d.select_guide("hit.playlist.ADD").unwrap();
    assert_eq!(d.view.clip, Some([10, 346, 28, 18]));
}
#[test]
fn a_skin_that_was_never_drawn_says_so_when_it_is_exported() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#204060"}]}))
        .unwrap();
    let blank = d.undrawn_sprites();
    assert!(blank.contains(&"main.play".to_string()), "{blank:?}");
    assert!(blank.contains(&"equalizer.background".to_string()));
    assert!(
        !blank.contains(&"main.background".to_string()),
        "the one sheet that was painted is not blank"
    );
    let dir = std::env::temp_dir().join("cranamp-undrawn-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("blank.wsz");
    let out = d.export(&path).unwrap();
    assert!(
        out["undrawn_sprites"].as_array().unwrap().len() > 50,
        "export has to say so: {out}"
    );
    std::fs::remove_file(&path).ok();
}
#[test]
fn ink_that_cannot_be_read_on_its_own_artwork_is_reported() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#202430"}]}))
        .unwrap();
    d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#2a2f3c"}]}))
        .unwrap();
    let title = d
        .readability()
        .into_iter()
        .find(|r| r["reads"] == "the track title")
        .expect("the title is checked");
    assert_eq!(title["readable"], false, "{title}");
    assert!(title["contrast"].as_f64().unwrap() < 1.5, "{title}");
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#f4ecd4"}]}))
        .unwrap();
    let title = d
        .readability()
        .into_iter()
        .find(|r| r["reads"] == "the track title")
        .unwrap();
    assert_eq!(title["readable"], true, "{title}");
}
#[test]
fn unsampled_pixels_counts_ink_rather_than_clearing_and_never_drawn_sheets() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"numbers.bmp","layer":"sheet"}))
        .unwrap();
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":99,"height":13,"color":"transparent"}]}))
        .unwrap();
    assert_eq!(out["unsampled_pixels"], 0, "{out}");
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":90,"y":0,"width":9,"height":13,"color":"#ffcc00"}]}))
        .unwrap();
    assert_eq!(out["unsampled_pixels"], 117, "{out}");
    assert_eq!(out["unsampled_sample"][0], json!([90, 0]));
    d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
        .unwrap();
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#ffdf9c"}]}))
        .unwrap();
    assert_eq!(out["unsampled_pixels"], 0, "{out}");
    assert_eq!(out["sheet_is_never_drawn"], true, "{out}");
    d.state(json!({"panel":"atlas","sheet":"pledit.bmp","layer":"sheet"}))
        .unwrap();
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":0,"y":72,"width":276,"height":38,"color":"#334455"}]}))
        .unwrap();
    assert_eq!(out["unsampled_pixels"], 38, "{out}");
    assert!(out["sheet_is_never_drawn"].is_null());
}
#[test]
fn identical_variants_names_only_sprites_the_stroke_actually_wrote() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"eqmain.bmp","layer":"sheet"}))
        .unwrap();
    let mut ops: Vec<Value> = Vec::new();
    for x in [10, 36, 69, 95, 128, 154, 187, 213] {
        ops.push(json!({"op":"rect","x":x,"y":119,"width":26,"height":12,
                        "color":"#445566"}));
    }
    d.draw(&json!({ "operations": ops })).unwrap();
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":10,"y":119,"width":4,"height":4,"color":"#889900"}]}))
        .unwrap();
    assert!(
        out["identical_variants"]
            .as_array()
            .is_some_and(|v| v.iter().any(|e| e["id"] == "equalizer.on")),
        "a stroke inside ON's own cell still reports it: {out}"
    );
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":0,"y":116,"width":9,"height":9,"color":"#112233"},
            {"op":"rect","x":224,"y":164,"width":44,"height":12,"color":"#112233"}]}))
        .unwrap();
    assert!(
        out["identical_variants"].is_null(),
        "the box covers ON and AUTO and the stroke wrote neither: {out}"
    );
}
#[test]
fn sprites_that_share_their_cells_share_one_report() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"eqmain.bmp","layer":"sheet"}))
        .unwrap();
    let ops: Vec<Value> = (0..28)
        .map(|f| {
            let (x, y) = if f < 14 {
                (13 + f * 15, 164)
            } else {
                (13 + (f - 14) * 15, 229)
            };
            json!({"op":"rect","x":x,"y":y,"width":14,"height":63,"color":"#334455"})
        })
        .collect();
    let out = d.draw(&json!({ "operations": ops })).unwrap();
    let same = out["identical_variants"].as_array().expect("a report");
    assert_eq!(same.len(), 1, "one fact, said once: {out}");
    assert_eq!(same[0]["id"], "equalizer.band0.track");
    let also = same[0]["also"].as_array().expect("the other ten");
    assert_eq!(also.len(), 10, "{same:?}");
    assert!(also.contains(&json!("equalizer.band10.track")));
}
#[test]
fn frames_that_came_out_the_same_picture_are_reported() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"volume.bmp","layer":"sheet"}))
        .unwrap();
    let mut ops: Vec<Value> = (0..28)
        .map(|f| json!({"op":"rect","x":0,"y":f*15,"width":68,"height":13,"color":"#203040"}))
        .collect();
    ops.push(json!({"op":"rect","x":2,"y":3,"width":30,"height":3,"color":"#ffcc00"}));
    let out = d.draw(&json!({ "operations": ops })).unwrap();
    assert_eq!(out["overwrites"], 0, "nothing else reports this");
    assert_eq!(out["clipped_pixels"], 0);
    let same = out["identical_variants"].as_array().expect("a report");
    let track = same
        .iter()
        .find(|v| v["id"] == "main.volume.track")
        .expect("the track");
    assert_eq!(track["of"], 28);
    assert_eq!(track["groups"][0].as_array().unwrap().len(), 27);
    assert_eq!(track["labels"][0], "0 · silent");
    assert_eq!(track["labels"][27], "27 · full volume");
    let ops: Vec<Value> = (0..28)
        .map(|f| json!({"op":"rect","x":2,"y":f*15+3,"width":2+f,"height":3,"color":"#ffcc00"}))
        .collect();
    let out = d.draw(&json!({ "operations": ops })).unwrap();
    assert!(
        out["identical_variants"]
            .as_array()
            .is_none_or(|v| v.iter().all(|s| s["id"] != "main.volume.track")),
        "28 different frames are not a defect: {}",
        out["identical_variants"]
    );
}
#[test]
fn an_unpainted_cell_is_not_a_duplicate() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layer":"sheet"}))
        .unwrap();
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":23,"y":0,"width":23,"height":18,"color":"#2f6b64"}]}))
        .unwrap();
    assert!(
        out["identical_variants"].is_null(),
        "the untouched pressed cell is empty, not a duplicate: {}",
        out["identical_variants"]
    );
    let out = d
        .draw(&json!({"operations":[
            {"op":"rect","x":23,"y":18,"width":23,"height":18,"color":"#2f6b64"}]}))
        .unwrap();
    let play = out["identical_variants"]
        .as_array()
        .expect("a report")
        .iter()
        .find(|v| v["id"] == "main.play")
        .expect("play");
    assert_eq!(play["groups"][0], json!([0, 1]));
    assert_eq!(play["labels"], json!(["released", "pressed"]));
}
#[test]
fn the_small_face_fits_a_word_in_a_cell_the_large_one_overruns() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"eqmain.bmp","layer":"sheet"}))
        .unwrap();
    let wide = d
        .draw(&json!({"operations":[{"op":"text","x":0,"y":0,"text":"16K","color":"#ffffff"}]}))
        .unwrap();
    assert!(d.undo());
    let narrow = d
        .draw(&json!({"operations":[{"op":"text","x":0,"y":0,"text":"16K","color":"#ffffff","face":"small"}]}))
        .unwrap();
    let width = |v: &Value| v["bounds"][2].as_i64().unwrap() - v["bounds"][0].as_i64().unwrap();
    assert!(width(&wide) > 14, "the 5x7 face overruns a band caption");
    assert!(
        width(&narrow) < 14,
        "the small face has to fit one: {}",
        narrow["bounds"]
    );
    let lower = d
        .draw(&json!({"operations":[{"op":"text","x":0,"y":40,"text":"khz","color":"#ffffff","face":"small"}]}))
        .unwrap();
    assert!(
        lower["unsupported_characters"].is_null(),
        "a-z must be drawn as capitals, not reported missing: {}",
        lower["unsupported_characters"]
    );
}
#[test]
fn atlas_guides_cover_states_and_clip_only_the_chosen_cell() {
    let mut d = Document::blank();
    d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layer":"sheet"}))
        .unwrap();
    let before = d.archive().unwrap();
    let guides = d.guides();
    let play = guides.iter().find(|g| g.id == "main.play#1").unwrap();
    assert_eq!(play.rect, [23, 18, 23, 18]);
    let artwork = d.render();
    d.state(json!({"guides":true})).unwrap();
    assert_eq!(
        d.editor_render(),
        artwork,
        "rectangles must not be painted into the picture"
    );
    assert_eq!(d.render(), artwork, "guides must not enter the artwork");
    assert_eq!(d.archive().unwrap(), before);
    d.select_guide(&play.id).unwrap();
    assert_eq!(
        d.archive().unwrap(),
        before,
        "guide selection is not artwork"
    );
    d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":136,"height":36,"color":"#abcdef"}]})).unwrap();
    assert_eq!(
        d.images["cbuttons.bmp"].get_pixel(23, 18).0,
        [171, 205, 239, 255]
    );
    assert_eq!(d.images["cbuttons.bmp"].get_pixel(22, 18).0, [0; 4]);
    assert_eq!(d.images["cbuttons.bmp"].get_pixel(23, 17).0, [0; 4]);
    d.state(json!({"sheet":"volume.bmp"})).unwrap();
    assert_eq!(d.view.clip, None);
    let guides = d.guides();
    assert_eq!(
        guides
            .iter()
            .filter(|g| g.id.starts_with("main.volume.track#"))
            .count(),
        28
    );
    d.state(json!({"sheet":"main.bmp"})).unwrap();
    assert!(d
        .guides()
        .iter()
        .any(|g| g.runtime && g.rect == [24, 43, 76, 16]));
}
#[test]
fn review_crop_maps_sources_and_identifies_live_timer_pixels_without_mutation() {
    let mut d = Document::blank();
    d.state(json!({"panel":"canvas","pressed":true,"digit":7}))
        .unwrap();
    let before = d.snapshot();
    let view = serde_json::to_value(&d.view).unwrap();
    let rev = d.revision;
    let crop = d.inspect_region([48, 26, 9, 13]).unwrap();
    let parts = crop["parts"].as_array().unwrap();
    assert!(parts.iter().any(|p| p["runtime"] == true
        && p["label"] == "TIMER DIGIT 0"
        && p["source_overlap"].is_null()));
    assert!(parts
        .iter()
        .any(|p| p["sheet"] == "numbers.bmp" && p["source_overlap"] == json!([63, 0, 9, 13])));
    let crop = d.inspect_region([40, 90, 5, 4]).unwrap();
    assert!(crop["parts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["sheet"] == "cbuttons.bmp" && p["source_overlap"] == json!([24, 20, 5, 4])));
    assert!(d.snapshot() == before);
    assert_eq!(d.revision, rev);
    assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
    assert!(d.inspect_region([274, 0, 2, 1]).is_err());
}
#[test]
fn review_crop_preserves_repeated_tile_source_offsets() {
    let mut d = Document::blank();
    d.state(json!({"panel":"canvas","active":true})).unwrap();
    let crop = d.inspect_region([26, 234, 4, 3]).unwrap();
    assert!(crop["parts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["sheet"] == "pledit.bmp" && p["source_overlap"] == json!([128, 23, 4, 3])));
}
#[test]
fn the_main_sheet_is_the_full_classic_height_with_no_aliased_row() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    assert_eq!(d.images["main.bmp"].dimensions(), (275, 116));
    assert!(
        !d.layers().iter().any(|l| l.id.contains("docking")),
        "row 115 is real art now, not a copy of row 114"
    );
    d.state(json!({"panel":"canvas"})).unwrap();
    d.draw(&json!({"operations":[{"op":"pixel","x":137,"y":115,"color":"#72aabb"}]}))
        .unwrap();
    assert_eq!(
        d.images["main.bmp"].get_pixel(137, 115).0,
        [114, 170, 187, 255],
        "the bottom row takes its own ink"
    );
    assert_eq!(d.images["main.bmp"].get_pixel(137, 114).0, [0; 4]);
}
#[test]
fn a_stroke_lands_in_a_run_of_variants_and_the_report_says_how_many() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    d.state(json!({"volume": 20})).unwrap();
    let report = d
        .draw(
            &json!({"layers":["main.volume.track"],"states":"onward","operations":[
                {"op":"pixel","x":110,"y":58,"color":"#123456"}
            ]}),
        )
        .unwrap();
    assert_eq!(report["states_written"]["scope"], json!("onward"));
    assert_eq!(
        report["states_written"]["variants"]["main.volume.track"],
        json!(8),
        "volume 20 onward is variants 20..27"
    );
    let cells = |d: &Document| {
        (0..28)
            .map(|v| d.images["volume.bmp"].get_pixel(3, v * 15 + 1).0)
            .collect::<Vec<_>>()
    };
    let after = cells(&d);
    assert!(
        after[..20].iter().all(|p| p[3] == 0),
        "nothing below the frame in hand"
    );
    assert!(
        after[20..].iter().all(|p| *p == [0x12, 0x34, 0x56, 255]),
        "the frame in hand and every one after it"
    );
    d.state(json!({"volume": 3})).unwrap();
    let report = d
        .draw(
            &json!({"layers":["main.volume.track"],"states":"up-to","operations":[
                {"op":"pixel","x":111,"y":58,"color":"#abcdef"}
            ]}),
        )
        .unwrap();
    assert_eq!(
        report["states_written"]["variants"]["main.volume.track"],
        json!(4),
        "volume 3 up-to is variants 0..3"
    );
    d.state(json!({"all_states": true})).unwrap();
    assert_eq!(d.view.states, SCOPE_ALL);
    assert_eq!(d.brief()["view"]["all_states"], json!(true));
    d.state(json!({"states": SCOPE_CURRENT})).unwrap();
    assert_eq!(d.brief()["view"]["all_states"], json!(false));
    let refusal = d.state(json!({"states": "sometimes"})).unwrap_err();
    assert!(
        refusal.to_string().contains("up-to"),
        "a refusal names the scopes: {refusal}"
    );
    let report = d
        .draw(&json!({"layers":["main.volume.track"],"operations":[
            {"op":"pixel","x":112,"y":58,"color":"#010203"}
        ]}))
        .unwrap();
    assert!(report["states_written"].is_null());
}
#[test]
fn readability_answers_for_the_playlist_without_plbg_and_for_the_eq_curve() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
        .unwrap();
    d.draw(&json!({"operations":[
        {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#101010"}
    ]}))
    .unwrap();
    d.open_on_whole_skin();
    d.draw(&json!({"layers":["equalizer.background"],"operations":[
        {"op":"rect","x":86,"y":133,"width":113,"height":19,"color":"#0c0c0c"}
    ]}))
    .unwrap();
    d.draw(&json!({"layers":["main.background"],"operations":[
        {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#f0f0f0"}
    ]}))
    .unwrap();
    d.set_palette(&json!({"Normal":"#0a0a0a","Current":"#ffffff",
                          "NormalBG":"#080808","SelectedBG":"#111111"}))
        .unwrap();
    assert!(!d.images.contains_key("plbg.bmp"), "no playlist background");
    let says = |d: &mut Document, what: &str| -> Value {
        d.readability()
            .into_iter()
            .find(|r| r["reads"] == json!(what))
            .unwrap_or_else(|| panic!("readability never mentioned {what}"))
    };
    assert_eq!(says(&mut d, "a playlist row")["readable"], json!(false));
    assert_eq!(says(&mut d, "the playing row")["readable"], json!(true));
    assert_eq!(says(&mut d, "a selected row")["readable"], json!(false));
    let title = says(&mut d, "the track title");
    let curve = says(&mut d, "the equalizer curve");
    assert_eq!(
        curve["ink"], title["ink"],
        "the curve is written in the display ink, like the title"
    );
    assert_eq!(curve["ground"], json!("#0c0c0c"));
    assert_eq!(curve["readable"], json!(false), "dark ink on a dark graph");
}
#[test]
fn at_repeats_a_transaction_and_a_swept_field_walks_the_variants() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    let report = d
        .draw(&json!({
            "layers":["main.previous","main.play","main.pause","main.stop","main.next"],
            "at":"targets",
            "operations":[{"op":"rect","x":2,"y":2,"width":3,"height":3,"color":"#ff0000"}]
        }))
        .unwrap();
    assert_eq!(report["repeated"]["places"], json!(5));
    assert_eq!(
        report["pixels_written"],
        json!(45),
        "nine pixels, five berths"
    );
    for (x, _) in [(16, 0), (39, 1), (62, 2), (85, 3), (108, 4)] {
        assert_eq!(
            d.render().get_pixel(x + 3, 88 + 3).0,
            [255, 0, 0, 255],
            "berth at {x} should have its own copy"
        );
    }
    let report = d
        .draw(&json!({
            "layers":["main.background"],
            "at":[[0,0],[0,2],[0,4]],
            "operations":[{"op":"pixel","x":200,"y":100,"color":"#00ff00"}]
        }))
        .unwrap();
    assert_eq!(report["repeated"]["places"], json!(3));
    for dy in 0..3 {
        assert_eq!(d.render().get_pixel(200, 100 + dy * 2).0, [0, 255, 0, 255]);
    }
    let mut d = Document::blank();
    d.open_on_whole_skin();
    let report = d
        .draw(&json!({
            "layers":["main.balance.track"],"states":"all",
            "operations":[{"op":"pixel","x":[178,213],"y":60,"color":"#0000ff"}]
        }))
        .unwrap();
    assert_eq!(report["swept"]["steps"], json!(28));
    assert_eq!(report["swept"]["fields"], json!(["x"]));
    assert_eq!(
        report["states_written"]["variants"]["main.balance.track"],
        json!(28),
        "the scope's own count, not one per pass"
    );
    let blue = |d: &mut Document, frame: u8| {
        d.state(json!({ "balance": frame })).unwrap();
        let im = d.render();
        (177..215)
            .find(|x| im.get_pixel(*x, 60).0 == [0, 0, 255, 255])
            .unwrap_or_else(|| panic!("frame {frame} has no swept pixel"))
    };
    assert_eq!(blue(&mut d, 0), 178, "the first frame is where it starts");
    assert_eq!(blue(&mut d, 27), 213, "the last frame is where it ends");
    let middle = blue(&mut d, 13);
    assert!(
        (194..=197).contains(&middle),
        "frame 13 is about halfway, not {middle}"
    );
    let refusal = d
        .draw(&json!({"operations":[{"op":"pixel","x":[1,9],"y":1,"color":"#ffffff"}]}))
        .unwrap_err();
    let refusal = format!("{refusal:#}");
    assert!(refusal.contains("named target"), "{refusal}");
    let refusal = d
        .draw(&json!({"layers":["main.background"],"states":"all",
                      "operations":[{"op":"pixel","x":[1,9],"y":1,"color":"#ffffff"}]}))
        .unwrap_err();
    let refusal = format!("{refusal:#}");
    assert!(refusal.contains("more than one variant"), "{refusal}");
}
#[test]
fn a_word_can_be_placed_in_a_box_instead_of_at_a_pixel() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    let ink = crate::winamp::pixel_text::measure("EQ", true, 1, 1).width;
    assert_eq!(ink, 9, "two small cells and one space");
    for (align, expected) in [
        ("left", 20),
        ("center", 20 + (30 - ink) / 2),
        ("right", 20 + 30 - ink),
    ] {
        let mut d = Document::blank();
        d.open_on_whole_skin();
        d.draw(&json!({"layers":["main.background"],"operations":[
            {"op":"text","x":20,"y":60,"width":30,"align":align,
             "text":"EQ","face":"small","color":"#ffffff"}
        ]}))
        .unwrap();
        let im = d.render();
        let first = (0..275)
            .find(|x| (60..67).any(|y| im.get_pixel(*x, y).0 == [255, 255, 255, 255]))
            .expect("some ink");
        assert_eq!(first as i32, expected, "align {align}");
    }
    let refusal = d
        .draw(&json!({"layers":["main.background"],"operations":[
            {"op":"text","x":20,"y":60,"align":"center","text":"EQ","face":"small"}
        ]}))
        .unwrap_err();
    let refusal = format!("{refusal:#}");
    assert!(refusal.contains("width"), "{refusal}");
}
#[test]
fn ink_a_control_hides_under_its_own_thumb_is_reported() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    d.draw(
        &json!({"layers":["main.balance.thumb"],"states":"all","operations":[
            {"op":"rect","x":189,"y":58,"width":14,"height":11,"color":"#c9c2ac"}
        ]}),
    )
    .unwrap();
    let report = d
        .draw(
            &json!({"layers":["main.balance.track"],"states":"all","operations":[
                {"op":"pixel","x":[183,207],"y":62,"color":"#5d5d6e"}
            ]}),
        )
        .unwrap();
    let covered = &report["covered_pixels"];
    assert_eq!(covered["pixels"], json!(25), "{report}");
    assert_eq!(covered["sample"][0]["behind"], json!("main.balance.thumb"));
    assert_eq!(covered["sample"][0]["in"], json!("main.balance.track"));
    let mut d = Document::blank();
    d.open_on_whole_skin();
    d.draw(
        &json!({"layers":["main.balance.thumb"],"states":"all","operations":[
            {"op":"rect","x":189,"y":66,"width":14,"height":3,"color":"#c9c2ac"}
        ]}),
    )
    .unwrap();
    let report = d
        .draw(
            &json!({"layers":["main.balance.track"],"states":"all","operations":[
                {"op":"pixel","x":[183,207],"y":62,"color":"#5d5d6e"}
            ]}),
        )
        .unwrap();
    assert!(
        report["covered_pixels"].is_null(),
        "a three-row grip hides three rows: {report}"
    );
    let report = d
        .draw(
            &json!({"layers":["main.balance.track"],"states":"all","operations":[
                {"op":"rect","x":177,"y":66,"width":38,"height":3,"color":"#5d5d6e"}
            ]}),
        )
        .unwrap();
    assert!(
        report["covered_pixels"].is_null(),
        "a bar across the whole track is visible at both ends: {report}"
    );
    let report = d
        .draw(&json!({"layers":["main.background"],"operations":[
            {"op":"rect","x":39,"y":88,"width":23,"height":18,"color":"#101010"}
        ]}))
        .unwrap();
    assert!(report["covered_pixels"].is_null(), "not news: {report}");
}

#[test]
fn the_validator_names_a_stray_sheet_a_missing_one_and_unpainted_sprites() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    let entries = |d: &Document| -> Vec<String> {
        d.divergences()
            .into_iter()
            .filter(|x| !x.problem.contains("clear"))
            .map(|x| x.entry)
            .collect()
    };
    assert_eq!(entries(&d), Vec::<String>::new(), "only classic sheets");
    assert!(
        d.divergences().iter().any(|x| x.problem.contains("clear")),
        "a blank skin is unpainted, and a .wsz cannot hold a clear pixel"
    );
    d.images
        .insert("plbg.bmp".into(), image::RgbaImage::new(243, 203));
    d.files.insert("plbg.bmp".into(), Vec::new());
    assert_eq!(entries(&d), vec!["plbg.bmp".to_string()]);
    d.images.remove("plbg.bmp");
    d.files.remove("plbg.bmp");
    d.files.remove("main.bmp");
    d.images.remove("main.bmp");
    assert_eq!(entries(&d), vec!["main.bmp".to_string()]);
}

#[test]
fn the_track_row_rectangle_has_no_gutter_because_no_marker_sheet_exists() {
    let mut d = Document::blank();
    d.open_on_whole_skin();
    let rows = d
        .guides()
        .into_iter()
        .find(|g| g.id == "runtime.playlist.TRACK ROWS")
        .expect("a track rows rectangle")
        .rect;
    assert_eq!(rows[0], 16, "list at 12, text inset 4");
    assert_eq!(rows[2], 227);
}
