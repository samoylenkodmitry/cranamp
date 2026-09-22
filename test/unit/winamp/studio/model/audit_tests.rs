use super::*;

#[test]
fn every_joined_pixel_is_probed_in_all_four_states_without_touching_the_document() {
    let mut doc = Document::blank();
    doc.open_on_whole_skin();
    doc.view.playback = 1;
    doc.view.clip = Some([0, 0, 1, 1]);
    doc.view.alpha_lock = true;
    doc.view.mask_colors = vec!["#ffffff".into()];
    let before = doc.status();
    let pixels = doc.render();
    let audit = doc.probe_pixels([0, 0, 275, 377], true).unwrap();
    assert_eq!(audit["pixels_probed"], 275 * 377 * 4);
    assert_eq!(audit["mismatches"], 0);
    for state in audit["states"].as_array().unwrap() {
        assert_eq!(state["verified_bitmap_pixels"], 82534);
        assert_eq!(state["coverage"]["counts"]["palette_only"], 21141);
        assert_eq!(state["coverage"]["counts"]["opaque_runtime"], 1216);
        for (map, verified) in state["coverage"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .zip(state["verified_rows"].as_array().unwrap())
        {
            for (a, b) in map
                .as_str()
                .unwrap()
                .bytes()
                .zip(verified.as_str().unwrap().bytes())
            {
                assert_eq!(matches!(a, b'D' | b'S' | b'R' | b'O'), b == b'W');
            }
        }
    }
    assert_eq!(doc.status(), before);
    assert_eq!(doc.render(), pixels);
}

#[test]
fn flat_warnings_include_text_backdrops_but_exclude_opaque_runtime_and_palette_fill() {
    let mut doc = Document::blank();
    doc.open_on_whole_skin();
    doc.view.playback = 1;
    doc.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":275,"height":377,"fill":true,"color":"#19182c"}],"layers":[],"states":"all"})).unwrap();
    let text = doc.flat_regions([110, 24, 151, 19]).unwrap();
    assert!(!text["regions"].as_array().unwrap().is_empty());
    assert!(
        text["regions"][0]["runtime_overlay_pixels"]
            .as_u64()
            .unwrap()
            > 0
    );
    for rect in [[24, 43, 76, 16], [12, 252, 243, 87]] {
        assert!(doc.flat_regions(rect).unwrap()["regions"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    let result = doc.draw(&json!({"operations":[{"op":"rect","x":36,"y":155,"width":40,"height":60,"fill":true,"color":"#26253c"}],"layers":["equalizer.background"]})).unwrap();
    assert!(!result["flat_drawable_areas"]["regions"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn a_speck_does_not_hide_a_large_blank_and_report_points_inside_the_blank() {
    let mut doc = Document::blank();
    doc.open_on_whole_skin();
    doc.draw(&json!({"operations":[
        {"op":"rect","x":36,"y":155,"width":40,"height":60,"fill":true,"color":"#19182c"},
        {"op":"pixel","x":55,"y":185,"color":"#ffffff"}
    ],"layers":["equalizer.background"]}))
        .unwrap();
    let report = doc.flat_regions([36, 155, 40, 60]).unwrap();
    let square: [u32; 4] =
        serde_json::from_value(report["regions"][0]["largest_flat_square"].clone()).unwrap();
    let pixels = doc.render();
    assert!(square[2] >= 8);
    for y in square[1]..square[1] + square[3] {
        for x in square[0]..square[0] + square[2] {
            assert_eq!(pixels.get_pixel(x, y).0, [25, 24, 44, 255]);
        }
    }
}

#[test]
fn probe_rejects_invalid_rectangles_and_respects_playlist_resize() {
    let mut doc = Document::blank();
    doc.open_on_whole_skin();
    assert!(doc.probe_pixels([u32::MAX, 0, 2, 1], false).is_err());
    doc.view.preview_playlist_height = 261;
    let audit = doc.probe_pixels([0, 483, 275, 10], false).unwrap();
    assert_eq!(audit["mismatches"], 0);
    for state in audit["states"].as_array().unwrap() {
        assert_eq!(state["verified_bitmap_pixels"], 2750);
        assert!(state.get("verified_rows").is_none());
    }
}

#[test]
fn optional_review_limits_do_not_turn_a_successful_draw_into_a_partial_failure() {
    let mut doc = Document::blank();
    doc.images
        .insert("main.bmp".into(), RgbaImage::new(2048, 513));
    doc.view.panel = "atlas".into();
    doc.view.sheet = "main.bmp".into();
    let result = doc
        .draw(&json!({"operations":[
            {"op":"pixel","x":0,"y":0,"color":"#123456"},
            {"op":"pixel","x":2047,"y":512,"color":"#123456"}
        ]}))
        .unwrap();
    assert_eq!(result["pixels_written"], 2);
    assert!(result["flat_area_review_unavailable"]
        .as_str()
        .unwrap()
        .contains("1048576"));
    assert_eq!(
        doc.images["main.bmp"].get_pixel(2047, 512).0,
        [18, 52, 86, 255]
    );
}
