use super::*;
#[test]
fn consistent_overpainting_of_a_repeated_rail_is_not_a_conflict() {
    let mut d = document();
    let out = d
        .draw(
            &json!({"states":"all","require_continuity":true,"operations":[
                {"op":"line","x":3,"y":223,"x2":3,"y2":376,"color":"#182735","brush_size":3},
                {"op":"line","x":3,"y":223,"x2":3,"y2":376,"color":"#526868","brush_size":1}
            ]}),
        )
        .unwrap();
    assert_eq!(out["continuity"]["continuous"], true);
    assert_eq!(out["continuity"]["mismatches"], 0);
    assert!(
        out["continuity"]["shared_source_overwrites"]
            .as_u64()
            .unwrap()
            > 0
    );
}
fn document() -> Document {
    let mut d = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap();
    d.open_on_whole_skin();
    d
}
fn stroke() -> Value {
    json!({"operations":[{"op":"curve","x":90,"y":135,"x2":108,"y2":150,"control":[97,143],"color":"#ee4791","brush_size":2}],"states":"all"})
}
#[test]
fn omitted_preamp_and_graph_are_reported_even_when_source_writes_succeed() {
    let mut d = document();
    let mut args = stroke();
    args["layers"] = json!(["equalizer.background", "equalizer.graph"]);
    let out = d.draw(&args).unwrap();
    assert!(out["pixels_written"].as_u64().unwrap() > 0);
    assert_eq!(out["continuity"]["continuous"], false);
    assert!(out["continuity"]["mismatches"].as_u64().unwrap() > 0);
    assert!(out["continuity"]["samples"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["covering_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "equalizer.graph" || p["id"] == "equalizer.preamp.line")));
}
#[test]
fn strict_join_refuses_hidden_strokes_atomically_and_auto_passes_in_four_states() {
    let mut d = document();
    let before = d.status();
    let pixels = d.render();
    let mut args = stroke();
    args["require_continuity"] = json!(true);
    args["layers"] = json!(["equalizer.background", "equalizer.graph"]);
    assert!(d
        .draw(&args)
        .unwrap_err()
        .to_string()
        .contains("Joined stroke refused"));
    assert_eq!(d.status(), before);
    assert_eq!(d.render(), pixels);
    args["layers"] = json!([]);
    let out = d.draw(&args).unwrap();
    assert_eq!(out["continuity"]["continuous"], true);
    assert_eq!(out["continuity"]["states"], 4);
    d.undo();
    assert_eq!(d.render(), pixels);
}
#[test]
fn shared_cell_conflicts_and_clipped_crossings_cannot_claim_continuity() {
    let mut d = document();
    let before = d.render();
    let out = d.draw(&json!({"preview":true,"states":"all","operations":[
        {"op":"line","x":4,"y":110,"x2":4,"y2":137,"color":"#ee4791"}],"layers":["main.background"]})).unwrap();
    assert_eq!(out["continuity"]["continuous"], false);
    assert!(out["continuity"]["clipped_pixels"].as_u64().unwrap() > 0);
    assert_eq!(d.render(), before);
    let out = d.draw(&json!({"states":"all","operations":[
        {"op":"rect","x":25,"y":233,"width":50,"height":6,"color":"#123456","ramp":["#123456","#ffaaff"],"ramp_axis":[25,233,74,233]}]})).unwrap();
    assert_eq!(out["continuity"]["continuous"], false);
    assert!(
        out["continuity"]["shared_source_overwrites"]
            .as_u64()
            .unwrap()
            > 0
    );
}
#[test]
fn gui_stroke_surfaces_the_same_occlusion_warning() {
    let mut d = document();
    d.view.layers = vec!["equalizer.background".into()];
    d.view.states = SCOPE_ALL.into();
    d.checkpoint();
    d.paint_line(
        [90, 135],
        [108, 150],
        [238, 71, 145, 255],
        "selection",
        Scope::All.into(),
    )
    .unwrap();
    d.finish_stroke();
    assert!(d.message.contains("stroke interrupted"));
}

#[test]
fn key_only_draws_are_unverified_and_strict_mode_preserves_the_document() {
    let mut d = document();
    let pixels = d.render();
    let status = d.status();
    let mut args = json!({"preview":true,"states":"all","operations":[
        {"op":"line","x":90,"y":135,"x2":108,"y2":150,"color":"#ff00ff"}]});
    let out = d.draw(&args).unwrap();
    assert_eq!(out["continuity"]["continuous"], Value::Null);
    assert_eq!(out["continuity"]["checked"], 0);
    args["preview"] = json!(false);
    args["require_continuity"] = json!(true);
    assert!(d.draw(&args).is_err());
    assert_eq!(d.render(), pixels);
    assert_eq!(d.status(), status);
}
