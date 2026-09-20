use super::*;
fn revision_for(d: &mut Document, id: &str, name: &str, r: [u32; 4], ops: Value) -> Value {
    let found = d
        .patch(
            &json!({"action":"inspect","replace_layer":id,"parts":[{"sheet":"main.bmp","rect":r}]}),
        )
        .unwrap();
    let mut result = found;
    result["action"] = json!("apply");
    result["name"] = json!(name);
    result["parts"][0]["operations"] = ops;
    result
}
#[test]
fn revision_replaces_only_its_middle_plane_and_is_one_undo() {
    let mut d = Document::blank();
    let a = patch_for(
        &mut d,
        "A",
        "main.bmp",
        [10, 10, 5, 5],
        json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#abcdef"}]),
    );
    d.patch(&a).unwrap();
    let b = patch_for(
        &mut d,
        "B",
        "pledit.bmp",
        [0, 72, 5, 5],
        json!([{"op":"pixel","x":0,"y":72,"color":"#123456"}]),
    );
    d.patch(&b).unwrap();
    let before = d.snapshot();
    let other = d.planes[1].clone();
    d.view.paint_layer = Some("paint-1".into());
    let view = serde_json::to_value(&d.view).unwrap();
    let mut revised = revision_for(
        &mut d,
        "paint-1",
        "A",
        [10, 10, 5, 5],
        json!([{"op":"pixel","x":11,"y":11,"color":"#112233"}]),
    );
    revised["action"] = json!("validate");
    d.patch(&revised).unwrap();
    assert!(d.snapshot() == before);
    revised["action"] = json!("apply");
    d.patch(&revised).unwrap();
    assert_eq!(d.planes.len(), 2);
    assert!(d.planes[1] == other);
    assert_eq!(d.planes[0].id, "paint-1");
    assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
    assert_eq!(d.undo.len(), 3);
    assert_eq!(
        d.composite_images()["main.bmp"].get_pixel(10, 10)[3],
        0,
        "Old marks must be removed, not overpainted"
    );
    assert_eq!(
        d.composite_images()["main.bmp"].get_pixel(11, 11).0,
        [17, 34, 51, 255]
    );
    d.undo();
    assert!(d.snapshot() == before);
    d.redo();
    assert!(d.planes[1] == other);
}
#[test]
fn revision_rejects_incomplete_ownership_and_stale_raw_paint_under_cover() {
    let mut d = Document::blank();
    let a = patch_for(
        &mut d,
        "A",
        "main.bmp",
        [10, 10, 5, 5],
        json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#abcdef"}]),
    );
    d.patch(&a).unwrap();
    let bad = revision_for(
        &mut d,
        "paint-1",
        "A",
        [10, 10, 1, 1],
        json!([{"op":"pixel","x":10,"y":10}]),
    );
    let before = d.snapshot();
    assert!(d.patch(&bad).is_err());
    assert!(d.snapshot() == before);
    let cover = patch_for(
        &mut d,
        "Cover",
        "main.bmp",
        [10, 10, 5, 5],
        json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#000000"}]),
    );
    d.patch(&cover).unwrap();
    let stale = revision_for(
        &mut d,
        "paint-1",
        "A",
        [10, 10, 5, 5],
        json!([{"op":"pixel","x":10,"y":10}]),
    );
    let visible_before = d.composite_images();
    d.planes[0]
        .images
        .get_mut("main.bmp")
        .unwrap()
        .put_pixel(10, 10, Rgba([12, 34, 56, 255]));
    assert_eq!(d.composite_images(), visible_before);
    let before = d.snapshot();
    let rev = d.revision;
    let undo = d.undo.len();
    assert!(d.patch(&stale).is_err());
    assert!(d.snapshot() == before);
    assert_eq!(d.revision, rev);
    assert_eq!(d.undo.len(), undo);
}
fn patch_for(d: &mut Document, name: &str, sheet: &str, region: [u32; 4], ops: Value) -> Value {
    let found = d
        .patch(&json!({"parts":[{"sheet":sheet,"rect":region}]}))
        .unwrap();
    let mut part = found["parts"][0].clone();
    part["operations"] = ops;
    json!({"action":"apply","name":name,"parts":[part]})
}
#[test]
fn disjoint_artists_merge_with_unchanged_view_and_one_undo_each() {
    let mut d = Document::blank();
    d.view.mirror_x = true;
    d.view.states = super::SCOPE_ALL.into();
    d.view.brush_size = 8;
    d.view.layers = vec!["main.close".into()];
    let view = serde_json::to_value(&d.view).unwrap();
    let a = patch_for(
        &mut d,
        "Portrait",
        "main.bmp",
        [10, 10, 5, 5],
        json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#123456"}]),
    );
    let b = patch_for(
        &mut d,
        "Footer",
        "pledit.bmp",
        [0, 72, 10, 10],
        json!([{"op":"pixel","x":4,"y":75,"color":"#abcdef"}]),
    );
    d.patch(&a).unwrap();
    let first = d.snapshot();
    d.patch(&b).unwrap();
    assert_eq!(d.undo.len(), 2);
    assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
    assert_eq!(
        d.composite_images()["pledit.bmp"].get_pixel(4, 75).0,
        [171, 205, 239, 255]
    );
    assert!(d.undo());
    assert!(d.snapshot() == first);
    assert!(d.redo());
}
#[test]
fn stale_and_out_of_region_patches_cannot_partially_modify_art_or_history() {
    let mut d = Document::blank();
    let a = patch_for(
        &mut d,
        "A",
        "main.bmp",
        [10, 10, 5, 5],
        json!([{"op":"pixel","x":10,"y":10,"color":"#abcdef"}]),
    );
    let mut stale = a.clone();
    stale["name"] = json!("stale");
    d.patch(&a).unwrap();
    let before = d.snapshot();
    let rev = d.revision;
    assert!(d.patch(&stale).is_err());
    let mut bad = patch_for(
        &mut d,
        "bad",
        "eqmain.bmp",
        [10, 10, 5, 5],
        json!([{"op":"pixel","x":10,"y":10},{"op":"line","x":14,"y":14,"x2":16,"y2":14}]),
    );
    let second = bad["parts"][0].clone();
    bad["parts"] = json!([a["parts"][0].clone(), second]);
    bad["parts"][0]["expected"] = d
        .patch(&json!({"parts":[{"sheet":"main.bmp","rect":[10,10,5,5]}]}))
        .unwrap()["parts"][0]["expected"]
        .clone();
    assert!(d.patch(&bad).is_err());
    assert!(d.snapshot() == before);
    assert_eq!(d.revision, rev);
    assert_eq!(d.undo.len(), 1);
    assert!(validate_bounds(
        &json!({"op":"stamp","x":14,"y":14,"rows":["xx"],"palette":{"x":"#ffffff"}}),
        [10, 10, 5, 5],
        false
    )
    .is_err());
    assert!(validate_bounds(
        &json!({"op":"line","x":10,"y":10,"x2":14,"y2":10,"brush_size":3}),
        [10, 10, 5, 5],
        false
    )
    .is_err());
}
#[test]
fn clipped_curve_parts_assemble_identically_and_preserve_transaction_state() {
    let mut d = Document::blank();
    let existing = patch_for(
        &mut d,
        "Other",
        "main.bmp",
        [80, 80, 1, 1],
        json!([{"op":"pixel","x":80,"y":80,"color":"#aabbcc"}]),
    );
    d.patch(&existing).unwrap();
    let other = d.planes[0].clone();
    d.view.clip = Some([1, 1, 1, 1]);
    d.view.mirror_x = true;
    let view = serde_json::to_value(&d.view).unwrap();
    let mut curve = json!({"op":"curve","x":4,"y":8,"x2":35,"y2":7,
        "control":[20,24],"brush_size":3,"color":"#ffffff",
        "ramp":["#102030","#406080","#aabbcc"],"ramp_axis":[0,0,39,0]});
    let mut reference = Document::blank();
    reference.view.panel = "atlas".into();
    reference.view.sheet = "main.bmp".into();
    reference
        .draw(&json!({"operations":[curve.clone()]}))
        .unwrap();
    let mut patch = patch_for(
        &mut d,
        "Bridge",
        "main.bmp",
        [0, 0, 20, 32],
        json!([curve.clone()]),
    );
    let before = d.snapshot();
    assert!(
        d.patch(&patch).is_err(),
        "Strict default must reject crossing geometry"
    );
    assert!(d.snapshot() == before);
    curve["x"] = json!(34);
    curve["y"] = json!(58);
    curve["x2"] = json!(65);
    curve["y2"] = json!(57);
    curve["control"] = json!([50, 74]);
    curve["ramp_axis"] = json!([30, 50, 69, 50]);
    let second = patch_for(
        &mut d,
        "Unused",
        "eqmain.bmp",
        [50, 50, 20, 32],
        json!([curve]),
    );
    patch["parts"]
        .as_array_mut()
        .unwrap()
        .push(second["parts"][0].clone());
    for part in patch["parts"].as_array_mut().unwrap() {
        part["clip_to_rect"] = json!(true);
    }
    patch["action"] = json!("validate");
    d.patch(&patch).unwrap();
    assert!(d.snapshot() == before);
    assert_eq!(d.undo.len(), 1);
    patch["action"] = json!("apply");
    d.patch(&patch).unwrap();
    assert!(d.planes[0] == other);
    assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
    let composite = d.composite_images();
    for y in 0..32 {
        for x in 0..40 {
            let actual = if x < 20 {
                composite["main.bmp"].get_pixel(x, y)
            } else {
                composite["eqmain.bmp"].get_pixel(x + 30, y + 50)
            };
            assert_eq!(
                actual,
                reference.images["main.bmp"].get_pixel(x, y),
                "assembled {x},{y}"
            );
        }
    }
    assert_eq!(d.undo.len(), 2);
    d.undo();
    assert!(d.snapshot() == before);
    d.redo();
    assert!(d.planes[0] == other);
}
#[test]
fn explicit_clip_contains_stamps_at_sheet_edge_and_rejects_invalid_flags() {
    let mut d = Document::blank();
    let mut patch = patch_for(
        &mut d,
        "Edge",
        "main.bmp",
        [0, 0, 2, 2],
        json!([{"op":"stamp","x":-1,"y":-1,"rows":["xxxx","xxxx","xxxx","xxxx"],
            "palette":{"x":"#abcdef"}}]),
    );
    patch["parts"][0]["clip_to_rect"] = json!("true");
    assert!(d.patch(&patch).is_err());
    patch["parts"][0]["clip_to_rect"] = json!(true);
    d.patch(&patch).unwrap();
    let plane = &d.planes[0];
    assert_eq!(plane.images.len(), 1);
    let paint: Vec<_> = plane.images["main.bmp"]
        .enumerate_pixels()
        .filter(|(_, _, p)| p[3] > 0)
        .collect();
    assert_eq!(paint.len(), 4);
    assert!(paint.iter().all(|(x, y, _)| *x < 2 && *y < 2));
    let before = d.snapshot();
    patch["name"] = json!("Bad rect");
    patch["parts"][0]["rect"] = json!([274, 114, 2, 2]);
    assert!(
        d.patch(&patch).is_err(),
        "Clip never permits an out-of-sheet ownership rectangle"
    );
    assert!(d.snapshot() == before);
    assert!(validate_bounds(&json!({"op":"pixel","mirror_x":true}), [0, 0, 2, 2], true).is_err());
}
#[test]
fn validation_is_read_only_and_ignores_unrelated_source_changes() {
    let mut d = Document::blank();
    let mut a = patch_for(
        &mut d,
        "A",
        "main.bmp",
        [10, 10, 5, 5],
        json!([{"op":"pixel","x":10,"y":10,"color":"#abcdef"}]),
    );
    let before = d.snapshot();
    a["action"] = json!("validate");
    assert_eq!(d.patch(&a).unwrap()["valid"], true);
    assert!(d.snapshot() == before);
    assert!(d.undo.is_empty());
    assert!(!d.dirty);
    d.images
        .get_mut("main.bmp")
        .unwrap()
        .put_pixel(20, 20, Rgba([1, 2, 3, 255]));
    a["action"] = json!("apply");
    assert!(d.patch(&a).is_ok());
}
