use super::*;
fn canvas(height: u32) -> Document {
    let mut d = Document::blank();
    d.state(json!({"panel":"canvas","preview_playlist_height":height,
        "layers":[],"all_states":true,"clean_corners":false}))
        .unwrap();
    d
}
fn unsplit(d: &Document) -> Document {
    let mut r = Document::blank();
    r.images.insert("reference.bmp".into(), d.render());
    r.state(json!({"panel":"atlas","sheet":"reference.bmp",
        "clean_corners":false}))
        .unwrap();
    r
}
fn assert_footprint(d: &Document, reference: &Document, before: &RgbaImage, label: &str) {
    let actual = d.render();
    let expected = reference.render();
    let mut checked = 0;
    for (x, y, p) in expected.enumerate_pixels() {
        if y == 114 || y == 115 || p == before.get_pixel(x, y) {
            continue;
        }
        assert_eq!(actual.get_pixel(x, y), p, "{label}: lost pixel {x},{y}");
        checked += 1;
    }
    assert!(checked > 0, "{label}: empty verification footprint");
}
#[test]
fn every_mcp_instrument_survives_panel_tile_and_footer_joins() {
    for height in [145, 146, 261, 384, 522] {
        for seam in [116, 130, 232, 252, 281, 232 + height - 38] {
            for x in [4, 268] {
                let y = seam - 4;
                let ops = [
                    json!({"op":"pixel","x":x+2,"y":seam,"brush_size":5}),
                    json!({"op":"line","x":x+2,"y":y,"x2":x+2,"y2":y+8,"brush_size":3}),
                    json!({"op":"rect","x":x,"y":y,"width":5,"height":9,"fill":true}),
                    json!({"op":"ellipse","x":x,"y":y,"width":5,"height":9,"fill":false}),
                    json!({"op":"curve","x":x+1,"y":y,"x2":x+1,"y2":y+8,"control":[x+4,y+4],"brush_size":2}),
                    json!({"op":"tuft","x":x+2,"y":y,"x2":x+2,"y2":y+8,"curve_bend":0,"brush_size":3}),
                    json!({"op":"path","x":x,"y":y,"points":[[0,0],[4,0],[4,8],[0,8],[0,0]],"fill":true}),
                    json!({"op":"stamp","x":x,"y":y,"rows":[" xxx ","xxxxx","xxxxx","xxxxx","xxxxx","xxxxx","xxxxx","xxxxx"," xxx "],"palette":{"x":"#ed4791"}}),
                    json!({"op":"rect","x":x,"y":y,"width":5,"height":9,"fill":true,"ramp":["#ed4791","#75ccdb","#edf4da"],"ramp_axis":[x,y,x,y+8]}),
                    json!({"op":"ellipse","x":x,"y":y,"width":5,"height":9,"fill":true,"material":"glass","bevel":2}),
                ];
                for (i, mut op) in ops.into_iter().enumerate() {
                    op["color"] = json!("#ed4791");
                    let mut d = canvas(height);
                    let before = d.render();
                    let mut r = unsplit(&d);
                    d.paint_layer_command(&json!({"action":"add","name":"Join audit"}), "MCP")
                        .unwrap();
                    let args = json!({"operations":[op]});
                    d.draw(&args).unwrap();
                    r.draw(&args).unwrap();
                    let label = format!("tool {i}, height {height}, seam {seam}, x {x}");
                    assert_footprint(&d, &r, &before, &label);
                    let painted = d.render();
                    d.undo();
                    assert_eq!(d.render(), before, "{label}: Undo");
                    d.redo();
                    assert_eq!(d.render(), painted, "{label}: Redo");
                    if i == 1 {
                        d.view.active = false;
                        assert_footprint(&d, &r, &before, &label);
                        d.make_portable("test").unwrap();
                        let flattened = d.render();
                        let mut reopened = Document::open(&d.archive().unwrap(), None).unwrap();
                        reopened.view = d.view.clone();
                        assert_eq!(reopened.render(), flattened, "{label}: WSZ reload");
                        d.undo();
                    }
                }
            }
        }
    }
}
#[test]
fn gui_brush_preview_lift_stamp_and_cancel_cross_joins() {
    for seam in [116, 130, 232, 252, 281, 339] {
        for brush in [
            "pencil", "line", "curve", "tuft", "rect", "ellipse", "glass", "stamp",
        ] {
            let mut d = canvas(145);
            let before = d.render();
            let mut r = unsplit(&d);
            let from = [4, seam - 4];
            let to = [8, seam + 4];
            for doc in [&mut d, &mut r] {
                doc.view.brush = brush.into();
                doc.view.color = "#ed4791".into();
                doc.view.filled = true;
                doc.view.curve_bend = 0;
                if brush == "stamp" {
                    doc.draw(&json!({"operations":[{"op":"rect","x":40,"y":70,"width":5,"height":9,"color":"#ed4791"}]})).unwrap();
                    doc.capture_cluster([40, 70, 5, 9]).unwrap();
                    doc.undo();
                }
                doc.checkpoint();
                if brush == "pencil" {
                    doc.paint_line(from, to, [237, 71, 145, 255], "auto", Scope::All.into())
                        .unwrap();
                } else if brush == "stamp" {
                    doc.shape_stroke(from, from).unwrap();
                } else {
                    doc.shape_stroke(from, [6, seam]).unwrap();
                    doc.shape_stroke(from, to).unwrap();
                }
            }
            assert_footprint(&d, &r, &before, &format!("GUI {brush} seam {seam}"));
            d.cancel_stroke();
            assert_eq!(d.render(), before, "Cancel {brush} seam {seam}");
        }
    }
}
#[test]
fn masks_mirrors_and_selected_parts_preserve_join_coverage() {
    let mut d = canvas(145);
    d.paint_layer_command(&json!({"action":"add","name":"Mask"}), "MCP")
        .unwrap();
    d.draw(&json!({"operations":[{"op":"rect","x":4,"y":110,"width":6,"height":24,"color":"#ed4791"}]})).unwrap();
    let before = d.render();
    d.state(json!({"alpha_lock":true,"mask_colors":["#ed4791"],"clip":[6,112,2,19]}))
        .unwrap();
    d.draw(&json!({"operations":[{"op":"rect","x":0,"y":105,"width":14,"height":35,"color":"#75ccdb"}]})).unwrap();
    let after = d.render();
    for y in 105..140 {
        for x in 0..14 {
            let expected = if (6..8).contains(&x) && (112..131).contains(&y) {
                Rgba([117, 204, 219, 255])
            } else {
                *before.get_pixel(x, y)
            };
            assert_eq!(*after.get_pixel(x, y), expected, "mask at {x},{y}");
        }
    }
    d.state(
        json!({"alpha_lock":false,"mask_colors":[],"clip":null,"mirror_x":true,
        "layers":["equalizer.background","equalizer.title"]}),
    )
    .unwrap();
    d.draw(&json!({"operations":[{"op":"line","x":6,"y":116,"x2":6,"y2":137,"color":"#eeaa22"}]}))
        .unwrap();
    for active in [true, false] {
        d.view.active = active;
        let image = d.render();
        for y in 116..138 {
            for x in [6, 268] {
                assert_eq!(
                    image.get_pixel(x, y).0,
                    [238, 170, 34, 255],
                    "selected/mirrored part {x},{y}, active {active}"
                );
            }
        }
    }
}
#[test]
fn classic_list_holes_are_reported_in_mcp_and_gui_instead_of_silent_success() {
    let mut d = canvas(145);
    let result = d
        .draw(
            &json!({"operations":[{"op":"line","x":6,"y":270,"x2":16,"y2":270,"color":"#ed4791"}]}),
        )
        .unwrap();
    assert_eq!(result["unmapped_pixels"], 5);
    assert_eq!(result["unmapped_sample"][0], json!([12, 270]));
    assert!(d.message.contains("no bitmap source"));
    for x in 6..12 {
        assert_eq!(d.render().get_pixel(x, 270).0, [237, 71, 145, 255]);
    }
    d.undo();
    d.checkpoint();
    d.paint_line(
        [6, 270],
        [16, 270],
        [237, 71, 145, 255],
        "auto",
        Scope::All.into(),
    )
    .unwrap();
    d.finish_stroke();
    assert!(d.message.contains("5 pixels have no bitmap source"));
    d.checkpoint();
    d.paint_line(
        [6, 270],
        [6, 275],
        [237, 71, 145, 255],
        "auto",
        Scope::All.into(),
    )
    .unwrap();
    d.finish_stroke();
    assert!(!d.message.contains("no bitmap source"));
    let clean = d
        .draw(&json!({"operations":[{"op":"pixel","x":6,"y":276,"color":"#ed4791"}]}))
        .unwrap();
    assert_eq!(clean["unmapped_pixels"], 0);
}
