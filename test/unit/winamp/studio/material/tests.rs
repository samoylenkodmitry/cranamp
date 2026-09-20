use super::*;
#[test]
fn native_glass_is_bounded_and_refracts_without_resizing() {
    let mut under = RgbaImage::new(24, 24);
    for (x, y, p) in under.enumerate_pixels_mut() {
        *p = image::Rgba([(x * 9) as u8, (y * 9) as u8, 30, 255]);
    }
    let original = under.clone();
    let pts = super::super::brush::rasterize(
        &serde_json::json!({"op":"ellipse","x":3,"y":4,"width":16,"height":14,"fill":true}),
    )
    .unwrap();
    let a = glass(
        &pts,
        &under,
        [120, 200, 220, 255],
        &serde_json::json!({"op":"ellipse","x":3,"y":4,"width":16,"height":14,"bevel":7,"refraction":0}),
    )
        .unwrap();
    let b = glass(
        &pts,
        &under,
        [120, 200, 220, 255],
        &serde_json::json!({"op":"ellipse","x":3,"y":4,"width":16,"height":14,"bevel":7,"refraction":5}),
    )
        .unwrap();
    assert_eq!(under, original);
    assert_eq!(a.len(), pts.len());
    assert_ne!(a, b);
    assert!(b.values().all(|c| c[3] == 255));
    assert!(b.keys().all(|p| pts.contains(p)));
    assert!(glass(&pts, &under, [0; 4], &serde_json::json!({"bevel":0})).is_err());
}
