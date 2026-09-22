use super::*;

fn document() -> Document {
    let mut d = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap();
    d.open_on_whole_skin();
    d
}

#[test]
fn artwork_mode_preserves_layers_undo_and_project_roundtrip() {
    let mut d = document();
    d.paint_layer_command(&json!({"action":"add","name":"EQ art"}), "test")
        .unwrap();
    d.draw(&json!({"origin":"equalizer.background","operations":[{"op":"pixel","x":79,"y":45,"color":"#abcdef"}]})).unwrap();
    let full = d.composite_images();
    d.set_eq_artwork_only(true, "test").unwrap();
    assert_eq!(d.composite_images(), full);
    assert_eq!(d.export_images()["eqmain.bmp"].height(), 163);
    assert!(!d
        .skin_layers()
        .iter()
        .any(|l| l.id.starts_with("equalizer.band")));
    let saved = d.project_bytes().unwrap();
    let mut reopened = Document::open_project(&saved).unwrap();
    assert!(reopened.eq_artwork_only);
    assert_eq!(reopened.composite_images(), full);
    reopened.set_eq_artwork_only(false, "test").unwrap();
    assert_eq!(reopened.composite_images(), full);
    assert_eq!(
        reopened
            .skin_layers()
            .iter()
            .filter(|l| l.id.ends_with(".track") && l.id.starts_with("equalizer.band"))
            .count(),
        11
    );
    assert!(d.undo());
    assert!(!d.eq_artwork_only);
    assert_eq!(d.composite_images(), full);
    assert!(d.redo());
    assert!(d.eq_artwork_only);
}

#[test]
fn cropped_export_has_same_composition_and_explicit_warning() {
    let mut d = document();
    d.set_eq_artwork_only(true, "test").unwrap();
    let bytes = d.export_archive().unwrap();
    let audit = crate::winamp::skin::audit_classic_archive(&bytes).unwrap();
    assert!(audit.exportable, "{:?}", audit.errors);
    assert!(audit.warnings.iter().any(|s| s.contains("Artwork-only EQ")));
    let mut reopened = Document::open(&bytes, None).unwrap();
    reopened.open_on_whole_skin();
    assert!(reopened.eq_artwork_only);
    assert_eq!(d.render(), reopened.render());
    assert!(reopened.set_eq_artwork_only(false, "test").is_err());
    assert_eq!(
        d.transparency_report()["incomplete_moving_sprites"],
        json!([])
    );
}

#[test]
fn only_exact_legacy_eq_omission_is_accepted() {
    let d = document();
    let entries = crate::winamp::skin::entries_of(&d.archive().unwrap()).unwrap();
    for height in [116, 162, 163, 164, 229, 314, 315] {
        let mut entries = entries.clone();
        entries
            .iter_mut()
            .find(|(name, _)| name == "eqmain.bmp")
            .unwrap()
            .1 = Some((275, height));
        let errors = crate::winamp::skin::divergences(&entries);
        assert_eq!(
            errors.iter().any(|e| e.entry == "eqmain.bmp"),
            height != 163 && height < 315,
            "height {height}"
        );
    }
}

#[test]
fn workbench_selects_shared_cells_and_exercises_independent_levels() {
    let mut d = document();
    let before = d.snapshot();
    let report = d
        .eq_workbench(&json!({"action":"edit_frame","frame":7}), "test")
        .unwrap();
    assert_eq!(report["shared_by"], 11);
    assert_eq!(d.view.eq, [7; 11]);
    assert_eq!(d.view.layers, vec!["equalizer.band1.track"]);
    assert_eq!(d.view.states, "current");
    d.eq_workbench(&json!({"action":"preview","pattern":"alternating"}), "test")
        .unwrap();
    assert_eq!(d.view.eq, [0, 27, 0, 27, 0, 27, 0, 27, 0, 27, 0]);
    assert!(d.snapshot() == before);
    let state = serde_json::to_value(&d.view).unwrap();
    assert!(d
        .eq_workbench(&json!({"action":"edit_frame","frame":28}), "test")
        .is_err());
    assert_eq!(serde_json::to_value(&d.view).unwrap(), state);
}

#[test]
fn copying_one_frame_uses_the_native_pen_preserves_other_art_and_undoes() {
    let mut d = document();
    d.eq_workbench(&json!({"action":"edit_frame","frame":7}), "test")
        .unwrap();
    d.draw(&json!({"origin":"equalizer.band1.track","states":"current","operations":[{"op":"rect","x":0,"y":0,"width":14,"height":63,"color":"#123456","fill":true},{"op":"line","x":0,"y":0,"x2":13,"y2":62,"color":"#abcdef"}]})).unwrap();
    let before = d.composite_images();
    let target = d
        .skin_layers()
        .into_iter()
        .find(|l| l.id == "equalizer.band1.track")
        .unwrap();
    d.view.mask_colors = vec!["#ffffff".into()];
    d.view.opacity = 128;
    let view = serde_json::to_value(&d.view).unwrap();
    d.eq_workbench(&json!({"action":"copy_frame_to_all","frame":7}), "test")
        .unwrap();
    assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
    let after = d.composite_images();
    assert_eq!(before["main.bmp"], after["main.bmp"]);
    for cell in target.variants {
        for y in 0..63 {
            for x in 0..14 {
                assert_eq!(
                    after["eqmain.bmp"].get_pixel(cell[0] + x, cell[1] + y),
                    before["eqmain.bmp"].get_pixel(target.source[0] + x, target.source[1] + y)
                );
            }
        }
    }
    assert!(d.undo());
    assert_eq!(d.composite_images(), before);
}
