use super::*;

fn canvas() -> Document {
    let mut doc = Document::blank();
    doc.open_on_whole_skin();
    doc.view.playback = 1;
    doc
}
#[test]
fn marked_gaps_are_real_paintable_pixels_and_not_runtime_exclusions() {
    let mut doc = canvas();
    for [x, y] in [
        [44, 27],
        [58, 30],
        [105, 51],
        [189, 37],
        [264, 29],
        [18, 73],
        [7, 344],
        [125, 354],
        [130, 370],
    ] {
        let (report, _) = doc.coverage([x, y, 1, 1], true).unwrap();
        assert_eq!(report["counts"]["paintable"], 1, "{x},{y}: {report}");
        assert_eq!(report["counts"]["palette_only"], 0);
        doc.draw(
            &json!({"operations":[{"op":"pixel","x":x,"y":y,"color":"#ff33aa","brush_size":1}]}),
        )
        .unwrap();
        assert_eq!(
            doc.render().get_pixel(x, y).0,
            [255, 51, 170, 255],
            "{x},{y}"
        );
    }
}
#[test]
fn runtime_footprints_keep_their_bitmap_backdrop_paintable() {
    let doc = canvas();
    let (report, _) = doc.coverage([24, 43, 76, 16], true).unwrap();
    assert_eq!(report["counts"]["paintable"], 76 * 16);
    assert_eq!(report["counts"]["runtime_overlay"], 76 * 16);
    assert!(report["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == "main.background"));
    assert!(report["rows"].as_array().unwrap().iter().all(|r| r
        .as_str()
        .unwrap()
        .chars()
        .all(|c| c == 'O')));
    assert_eq!(report["counts"]["opaque_runtime"], 76 * 16);
}
#[test]
fn playlist_fill_is_distinct_from_its_editable_footer_at_every_height() {
    let mut doc = canvas();
    for height in [145, 146, 261, 384, 522] {
        doc.view.preview_playlist_height = height;
        let (fill, _) = doc.coverage([20, 252, 1, 1], false).unwrap();
        assert_eq!(fill["counts"]["palette_only"], 1);
        assert_eq!(fill["counts"]["paintable"], 0);
        let (footer, _) = doc.coverage([7, 232 + height - 30, 1, 1], false).unwrap();
        assert_eq!(footer["counts"]["paintable"], 1);
    }
}

#[test]
fn keyed_spectrum_coverage_and_real_writer_probe_agree_after_export_reload() {
    let mut doc = canvas();
    let mut colors = vec!["#ffe8b1"; 24];
    colors[0] = "#ff00ff";
    doc.visualizer_palette(&json!({"colors":colors})).unwrap();
    let mut reloaded = Document::open(&doc.archive().unwrap(), None).unwrap();
    reloaded.open_on_whole_skin();
    reloaded.view.playback = 1;
    let (report, _) = reloaded.coverage([24, 43, 76, 16], true).unwrap();
    assert_eq!(report["counts"]["opaque_runtime"], 76 * 16);
    assert_eq!(report["counts"]["paintable"], 76 * 16);
    assert!(report["rows"].as_array().unwrap().iter().all(|r| r
        .as_str()
        .unwrap()
        .chars()
        .all(|c| c == 'O')));
    let probe = reloaded.probe_pixels([24, 43, 76, 16], false).unwrap();
    assert_eq!(probe["mismatches"], 0);
    assert!(probe["states"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["coverage"]["counts"]["opaque_runtime"] == 76 * 16));
}
#[test]
fn coverage_reports_repeats_and_never_changes_art_history_or_view() {
    let doc = canvas();
    let before = doc.status();
    let pixels = doc.render();
    let (report, image) = doc.coverage([25, 232, 25, 20], true).unwrap();
    assert_eq!(image.dimensions(), (25, 20));
    assert_eq!(report["counts"]["stateful_or_repeated"], 500);
    assert!(report["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["id"] == "playlist.top.tile" && t["repeated"] == true));
    assert_eq!(doc.status(), before);
    assert_eq!(doc.render(), pixels);
}
#[test]
fn coverage_validates_bounds_without_overflow() {
    let doc = canvas();
    for rect in [
        [0, 0, 0, 2],
        [275, 0, 1, 1],
        [0, 377, 1, 1],
        [u32::MAX, 0, 2, 1],
        [0, 0, u32::MAX, 2],
    ] {
        assert!(doc.coverage(rect, true).is_err(), "{rect:?}");
    }
}
#[test]
fn stopped_spectrum_reveals_editable_background() {
    let mut doc = canvas();
    doc.view.playback = 0;
    let (coverage, _) = doc.coverage([24, 43, 76, 16], false).unwrap();
    assert_eq!(coverage["counts"]["opaque_runtime"], 0);
    assert_eq!(coverage["counts"]["paintable"], 76 * 16);
}
