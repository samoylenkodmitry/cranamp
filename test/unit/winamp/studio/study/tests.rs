use super::*;

#[test]
fn palette_counts_exact_rgba_in_the_crop_without_mutating_pixels() {
    let im = RgbaImage::from_raw(
        4,
        1,
        vec![255, 0, 0, 255, 1, 2, 3, 255, 1, 2, 3, 255, 1, 2, 3, 0],
    )
    .unwrap();
    let before = im.clone();
    let report = palette(&im, [1, 0, 3, 1]).unwrap();
    assert_eq!(report["pixels"], 3);
    assert_eq!(report["unique_colors"], 2);
    assert_eq!(report["colors"][0]["hex"], "#010203ff");
    assert_eq!(report["colors"][0]["pixels"], 2);
    assert_eq!(report["colors"][1]["rgba"], serde_json::json!([1, 2, 3, 0]));
    assert_eq!(im, before);
    assert!(palette(&im, [4, 0, 1, 1]).is_err());
    assert!(palette(&im, [0, 0, 0, 1]).is_err());
}

#[test]
fn palette_limits_output_and_sorts_ties_by_rgba() {
    let im = RgbaImage::from_fn(40, 1, |x, _| Rgba([39 - x as u8, 0, 0, 255]));
    let report = palette(&im, [0, 0, 40, 1]).unwrap();
    assert_eq!(report["unique_colors"], 40);
    assert_eq!(report["truncated"], true);
    assert_eq!(report["colors"].as_array().unwrap().len(), 32);
    assert_eq!(report["colors"][0]["hex"], "#000000ff");
}
#[test]
fn value_preview_preserves_native_geometry_alpha_and_source() {
    let im =
        RgbaImage::from_raw(3, 1, vec![255, 0, 0, 23, 0, 255, 0, 127, 0, 0, 255, 255]).unwrap();
    let before = im.clone();
    let v = value_view(&im);
    assert_eq!(im, before);
    assert_eq!(v.dimensions(), im.dimensions());
    assert_eq!(v.get_pixel(0, 0), &Rgba([54, 54, 54, 23]));
    assert_eq!(v.get_pixel(1, 0), &Rgba([182, 182, 182, 127]));
    assert_eq!(v.get_pixel(2, 0), &Rgba([19, 19, 19, 255]));
    let d = Document::blank();
    let status = d.status();
    board(&d, &serde_json::json!({"rect":[0,0,20,20],"values":true})).unwrap();
    assert_eq!(d.status(), status);
}
#[test]
fn studies_are_native_and_read_only() {
    let d = Document::blank();
    let before = d.status();
    let b = board(
        &d,
        &serde_json::json!({"rect":[2,3,2,3],"zoom":4,"padding":0}),
    )
    .unwrap();
    assert_eq!(d.status(), before);
    assert_eq!(b.dimensions(), (40, 63));
    for y in 0..3 {
        for x in 0..2 {
            for yy in 0..4 {
                for xx in 0..4 {
                    assert_eq!(
                        b.get_pixel(16 + x * 4 + xx, 35 + y * 4 + yy),
                        b.get_pixel(16 + x, 16 + y)
                    );
                }
            }
        }
    }
    assert!(board(&d, &serde_json::json!({"rect":[274,114,2,2]})).is_err());
}

#[test]
fn context_includes_only_real_neighbor_pixels_and_rejects_invalid_centers() {
    assert_eq!(
        context_rect([2, 3, 2, 3], (275, 377), 4).unwrap(),
        [0, 0, 8, 10]
    );
    assert_eq!(
        context_rect([271, 374, 4, 3], (275, 377), 4).unwrap(),
        [267, 370, 8, 7]
    );
    assert_eq!(
        context_rect([0, 0, 275, 377], (275, 377), 32).unwrap(),
        [0, 0, 275, 377]
    );
    assert!(context_rect([u32::MAX, 1, 2, 2], (275, 377), 4).is_err());
    assert!(context_rect([0, 0, 0, 1], (275, 377), 4).is_err());
}

#[test]
fn study_defaults_to_context_and_reports_overlapping_control_sources() {
    let mut d = Document::blank();
    d.view.panel = "canvas".into();
    let before = d.status();
    let r = report(&d, &serde_json::json!({"rect":[212,41,27,12]})).unwrap();
    assert_eq!(r["padding"], 4);
    assert_eq!(r["context_rect"], serde_json::json!([208, 37, 35, 20]));
    assert!(r["paintability"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["id"] == "main.mono" && v["states"] == 2));
    assert!(r["paintability"]["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["id"] == "main.stereo"));
    assert_eq!(d.status(), before);
    for padding in [
        serde_json::json!(-1),
        serde_json::json!(33),
        serde_json::json!("4"),
    ] {
        assert!(report(&d, &serde_json::json!({"padding":padding})).is_err());
    }
}
