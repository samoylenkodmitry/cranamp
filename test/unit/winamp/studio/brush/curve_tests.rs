use super::*;
use serde_json::json;
#[test]
fn clean_strokes_remove_elbows_without_breaking_or_moving_endpoints() {
    let path = vec![[0, 0], [1, 0], [1, 1], [2, 1], [3, 1], [3, 2], [4, 2]];
    let cleaned = clean_corners(&path);
    assert_eq!(cleaned.first(), path.first());
    assert_eq!(cleaned.last(), path.last());
    assert!(cleaned.len() < path.len());
    assert!(cleaned
        .windows(2)
        .all(|p| (p[0][0] - p[1][0]).abs() <= 1 && (p[0][1] - p[1][1]).abs() <= 1));
    for x in -8..=8 {
        for y in -8..=8 {
            let op = json!({"op":"curve","x":0,"y":0,"x2":12,"y2":7,"control":[x,y],"clean_corners":true});
            let pixels = rasterize(&op).unwrap();
            assert!(pixels.contains(&[0, 0]) && pixels.contains(&[12, 7]));
            let mut raw_op = op.clone();
            raw_op["clean_corners"] = json!(false);
            let raw = rasterize(&raw_op).unwrap();
            assert!(pixels.iter().all(|p| raw.contains(p)));
            let mut reached = BTreeSet::from([[0, 0]]);
            loop {
                let old = reached.len();
                for p in &pixels {
                    if reached
                        .iter()
                        .any(|q| (p[0] - q[0]).abs() <= 1 && (p[1] - q[1]).abs() <= 1)
                    {
                        reached.insert(*p);
                    }
                }
                if old == reached.len() {
                    break;
                }
            }
            assert_eq!(reached.len(), pixels.len());
        }
    }
}
#[test]
fn cleanup_does_not_modify_filled_closed_or_thick_geometry() {
    for mut op in [
        json!({"op":"path","x":0,"y":0,"points":[[0,0],[8,0],[8,8],[0,0]]}),
        json!({"op":"curve","x":0,"y":0,"x2":12,"y2":7,"brush_size":3}),
        json!({"op":"path","x":0,"y":0,"points":[[0,0],[8,0],[8,8]],"fill":true}),
        json!({"op":"ellipse","x":0,"y":0,"width":11,"height":8}),
    ] {
        let before = rasterize(&op).unwrap();
        op["clean_corners"] = json!(true);
        assert_eq!(before, rasterize(&op).unwrap());
    }
}
#[test]
fn curved_strokes_are_connected_and_tufts_have_a_single_pixel_tip() {
    let curve =
        rasterize(&json!({"op":"curve","x":10,"y":10,"x2":30,"y2":10,"control":[20,30]})).unwrap();
    assert!(curve.contains(&[10, 10]) && curve.contains(&[30, 10]) && curve.contains(&[20, 20]));
    let mut reached = BTreeSet::from([curve[0]]);
    loop {
        let old = reached.len();
        for p in &curve {
            if reached
                .iter()
                .any(|q| (p[0] - q[0]).abs() <= 1 && (p[1] - q[1]).abs() <= 1)
            {
                reached.insert(*p);
            }
        }
        if old == reached.len() {
            break;
        }
    }
    assert_eq!(reached.len(), curve.len());
    let tuft = rasterize(
        &json!({"op":"tuft","x":10,"y":10,"x2":30,"y2":10,"curve_bend":0,"brush_size":5}),
    )
    .unwrap();
    assert_eq!(tuft.iter().filter(|p| p[0] == 10).count(), 5);
    assert_eq!(
        tuft.iter()
            .filter(|p| p[0] == 30)
            .copied()
            .collect::<Vec<_>>(),
        vec![[30, 10]]
    );
    assert!(rasterize(&json!({"op":"curve","x":0,"y":0,"curve_bend":101})).is_err());
}
