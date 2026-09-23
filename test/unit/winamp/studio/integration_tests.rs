use super::*;
#[test]
fn the_editor_opens_on_the_skin_the_player_wears() {
    use std::io::{Cursor, Read};
    fn entries(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..zip.len())
            .map(|i| {
                let mut file = zip.by_index(i).unwrap();
                let name = file.name().to_owned();
                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();
                (name, data)
            })
            .collect()
    }
    let doc = initial_document(None).unwrap();
    assert!(doc.path.is_none());
    assert!(!doc.dirty);
    assert!(!doc.view.presentation);
    assert!(doc.planes.is_empty(), "the skin, not a layered copy of it");
    let exported = entries(&doc.archive().unwrap());
    let drawn_for = crate::winamp::cursors::SkinCursor::files()
        .into_iter()
        .filter(|(role, _)| role.stand_in().is_none())
        .count();
    assert_eq!(
        exported.len(),
        15 + drawn_for,
        "thirteen sheets, two text files, and one cursor per region of the full \
         window; the rolled-up window's regions borrow theirs"
    );
    assert_eq!(exported, entries(super::super::BUNDLED_SKINS[0].bytes));
    super::super::bundled_skin().expect("Catamp must load in the player");
}
#[test]
fn explicit_studio_input_does_not_fall_back_to_bundled_art() {
    assert!(initial_document(Some("missing-skin-for-startup-test.wsz")).is_err());
}
#[test]
fn blank_atlases_have_no_inherited_art_and_share_native_canvas_history() {
    let mut d = Document::blank();
    super::super::skin::load_skin(&d.archive().unwrap()).unwrap();
    assert_eq!(d.sheets().len(), 13);
    d.state(json!({"panel":"atlas","sheet":"volume.bmp","layer":"sheet"}))
        .unwrap();
    assert_eq!(d.render().dimensions(), (68, 433));
    assert_eq!(
        d.inspect([67, 432, 1, 1], None)["hits"][0]["rgba"],
        json!([0, 0, 0, 0])
    );
    d.draw(&json!({"operations":[{"op":"pixel","x":67,"y":432,"color":"#abcdef"}]}))
        .unwrap();
    assert_eq!(
        d.inspect([67, 432, 1, 1], None)["hits"][0]["rgba"],
        json!([171, 205, 239, 255])
    );
    d.undo();
    assert_eq!(
        d.inspect([67, 432, 1, 1], None)["hits"][0]["rgba"],
        json!([0, 0, 0, 0])
    );
    d.redo();
    d.state(json!({"panel":"main","layer":"auto"})).unwrap();
    assert_eq!(d.render().dimensions(), (275, 116));
    assert!(d
        .state(json!({"panel":"atlas","sheet":"missing.bmp"}))
        .is_err());
}
#[test]
fn mcp_and_human_edits_share_the_same_undo_history() {
    let shared = SharedDocument(Arc::new(Mutex::new(
        Document::open(include_bytes!("../../../../assets/winamp.wsz"), None).unwrap(),
    )));
    let original = shared.lock().unwrap().archive().unwrap();
    {
        let mut d = shared.lock().unwrap();
        d.checkpoint();
        d.paint_line(
            [40, 90],
            [44, 90],
            [17, 34, 51, 255],
            "play",
            model::Scope::Current.into(),
        )
        .unwrap();
    }
    let response = mcp::dispatch(
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"studio_draw","arguments":{"layer":"play","all_states":true,"operations":[{"op":"pixel","x":42,"y":91,"color":"#abcdef"}]}}}),
        &shared,
    );
    assert_eq!(response["id"], 7);
    assert!(response["result"]["isError"].is_null());
    assert_eq!(shared.lock().unwrap().status()["undo"], 2);
    mcp::call("studio_undo", json!({}), &shared).unwrap();
    assert_eq!(
        shared.lock().unwrap().inspect([40, 90, 1, 1], None)["hits"][0]["rgba"],
        json!([17, 34, 51, 255])
    );
    mcp::call("studio_undo", json!({}), &shared).unwrap();
    assert_eq!(shared.lock().unwrap().archive().unwrap(), original);
}
#[test]
fn a_state_sheet_needs_a_sprite_and_shows_every_variant_of_it() {
    let mut d = Document::open(include_bytes!("../../../../assets/winamp.wsz"), None).unwrap();
    d.state(json!({"layer":"auto"})).unwrap();
    assert!(
        d.state_sheet(None).is_err(),
        "with no sprite chosen there is nothing to lay out"
    );
    d.state(json!({"layer":"volume.track"})).unwrap();
    let sheet = d.state_sheet(None).expect("a chosen sprite has a sheet");
    assert_eq!(sheet.dimensions(), (546, 124));
    assert!(
        sheet.pixels().all(|p| p.0[..3] != [255, 0, 255]),
        "the transparency key must not be painted as a colour"
    );
    d.state(json!({"panel":"equalizer","layer":"band1.track"}))
        .unwrap();
    assert_eq!(
        d.state_sheet(None).unwrap().dimensions(),
        (168, 324),
        "every variant, numbered, in a grid"
    );
}
