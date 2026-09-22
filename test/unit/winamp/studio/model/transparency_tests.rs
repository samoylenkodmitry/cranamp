use super::*;

#[test]
fn a_keyed_spectrum_alone_is_a_classic_bitmap_difference() {
    let mut doc = document();
    let colors: Vec<_> = (0..24)
        .map(|i| if i == 0 { "#ff00ff" } else { "#112233" })
        .collect();
    doc.visualizer_palette(&json!({"colors":colors})).unwrap();
    let report = doc.transparency_report();
    assert_eq!(report["key_pixels"], 0);
    assert_eq!(report["keyed_spectrum"], false);
    assert_eq!(report["classic_bitmap_compatible"], true);
    assert_eq!(doc.make_portable("test").unwrap()["exportable"], true);
}

#[test]
fn studio_edits_the_preferred_extended_number_sheet() {
    let mut doc = document();
    let numbers = doc.images.remove("numbers.bmp").unwrap();
    doc.files.remove("numbers.bmp");
    doc.images.insert("nums_ex.bmp".into(), numbers);
    doc.files.insert("nums_ex.bmp".into(), Vec::new());
    let digits: Vec<_> = doc
        .layers()
        .into_iter()
        .filter(|l| l.id.starts_with("main.digit"))
        .collect();
    assert_eq!(digits.len(), 4);
    assert!(digits.iter().all(|l| l.sheet == "nums_ex.bmp"));
    let bytes = doc.archive().unwrap();
    assert!(crate::winamp::skin::load_skin(&bytes).is_ok());
    doc.draw(&json!({"origin":"main.digit0","operations":[{"op":"pixel","x":0,"y":0,"color":"#123456"}]})).unwrap();
    let skin = crate::winamp::skin::load_skin(&doc.archive().unwrap()).unwrap();
    assert_eq!(&skin.numbers.pixels()[..4], &[18, 52, 86, 255]);
}

fn document() -> Document {
    let mut doc = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap();
    doc.open_on_whole_skin();
    doc
}

#[test]
fn keyed_holes_survive_planes_archive_and_every_handle_position() {
    let mut doc = document();
    // Magenta is opaque. Every position must copy the same complete source cell.
    doc.paint_layer_command(&json!({"action":"add","name":"silhouette"}), "test")
        .unwrap();
    doc.draw(
        &json!({"origin":"main.volume.thumb","states":"all","operations":[
        {"op":"rect","x":0,"y":0,"width":14,"height":11,"color":"#ff00ff","fill":true},
        {"op":"pixel","x":7,"y":5,"color":"#f4bd68"}]}),
    )
    .unwrap();
    let report = doc.transparency_report();
    let cells: Vec<_> = report["moving_cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["id"] == "main.volume.thumb")
        .collect();
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().all(|c| c["key_pixels"] == 153));
    let bytes = doc.archive().unwrap();
    let mut reopened = Document::open(&bytes, None).unwrap();
    reopened.open_on_whole_skin();
    for active in [true, false] {
        for pressed in [true, false] {
            for volume in 0..28 {
                doc.state(json!({"volume":volume,"active":active,"pressed":pressed}))
                    .unwrap();
                reopened
                    .state(json!({"volume":volume,"active":active,"pressed":pressed}))
                    .unwrap();
                let thumb = doc
                    .layers()
                    .into_iter()
                    .find(|l| l.id == "main.volume.thumb")
                    .unwrap();
                let [x, y, w, h] = thumb.destination;
                let raw = doc.composite_images();
                assert_eq!(
                    raw[&thumb.sheet]
                        .get_pixel(thumb.source[0], thumb.source[1])
                        .0,
                    [255, 0, 255, 255]
                );
                let mut under = document();
                under.view = doc.view.clone();
                for rect in &thumb.variants {
                    for yy in rect[1]..rect[1] + rect[3] {
                        for xx in rect[0]..rect[0] + rect[2] {
                            under.images.get_mut(&thumb.sheet).unwrap().put_pixel(
                                xx,
                                yy,
                                Rgba([0, 0, 0, 0]),
                            );
                        }
                    }
                }
                let patch = |d: &Document| {
                    d.render_patch([x as i32, y as i32, w as i32, h as i32])
                        .image
                };
                let actual = patch(&doc);
                assert_eq!(actual, patch(&reopened));
                assert_ne!(patch(&under), actual);
                for yy in 0..h {
                    for xx in 0..w {
                        assert_eq!(
                            actual.get_pixel(xx, yy),
                            if [xx, yy] == [7, 5] {
                                &Rgba([244, 189, 104, 255])
                            } else {
                                &Rgba([255, 0, 255, 255])
                            }
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn erase_skip_and_key_are_distinct_and_atlas_exposes_the_key() {
    let mut doc = document();
    doc.paint_layer_command(&json!({"action":"add","name":"holes"}), "test")
        .unwrap();
    let paint = |color: &str| json!({"origin":"main.volume.thumb","states":"all","operations":[{"op":"pixel","x":0,"y":0,"color":color}]});
    doc.draw(&paint("#ff00ff")).unwrap();
    let thumb = doc
        .layers()
        .into_iter()
        .find(|l| l.id == "main.volume.thumb")
        .unwrap();
    let p = [thumb.source[0], thumb.source[1]];
    let raw = |d: &Document| d.composite_images()[&thumb.sheet].get_pixel(p[0], p[1]).0;
    assert_eq!(raw(&doc), [255, 0, 255, 255]);
    doc.draw(&json!({"origin":"main.volume.thumb","operations":[{"op":"stamp","x":0,"y":0,"rows":[" "],"palette":{}}]})).unwrap();
    assert_eq!(
        raw(&doc),
        [255, 0, 255, 255],
        "missing stamp symbol must skip"
    );
    let pixel = doc.inspect([thumb.destination[0], thumb.destination[1], 1, 1], None);
    assert_eq!(pixel["hits"][0]["sprite_key"], false);
    assert_eq!(pixel["hits"][0]["literal_magenta"], true);
    doc.draw(&paint("transparent")).unwrap();
    assert_eq!(
        raw(&doc),
        doc.images[&thumb.sheet].get_pixel(p[0], p[1]).0,
        "erase reveals the older sprite"
    );
    doc.draw(&paint("#ff00ff")).unwrap();
    doc.view.panel = "atlas".into();
    doc.view.sheet = thumb.sheet.clone();
    assert_eq!(doc.render().get_pixel(p[0], p[1]).0, [255, 0, 255, 255]);
}

#[test]
fn textured_small_rectangles_warn_in_both_states_without_mutation() {
    let mut doc = document();
    doc.draw(
        &json!({"origin":"main.volume.thumb","states":"all","operations":[
        {"op":"rect","x":0,"y":0,"width":14,"height":11,"color":"#182735","fill":true},
        {"op":"line","x":0,"y":0,"x2":13,"y2":10,"color":"#526868"}]}),
    )
    .unwrap();
    let before = doc.status();
    let report = doc.transparency_report();
    assert_eq!(
        report["opaque_moving_sprites"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["id"] == "main.volume.thumb")
            .count(),
        0
    );
    assert_eq!(doc.status(), before);
    doc.draw(
        &json!({"origin":"main.volume.thumb","states":"all","operations":[
        {"op":"rect","x":0,"y":0,"width":14,"height":11,"color":"#ff00ff","fill":true}]}),
    )
    .unwrap();
    assert!(!doc.transparency_report()["opaque_moving_sprites"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == "main.volume.thumb"));
    let before = doc.composite_images();
    let report = doc.make_portable("test").unwrap();
    assert_eq!(report["exportable"], true);
    assert_eq!(
        doc.composite_images(),
        before,
        "validation must preserve literal magenta ink"
    );
}
