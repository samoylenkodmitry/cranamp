use super::*;

#[test]
fn state_previews_keep_exact_sprite_pixels_and_separate_labels() {
    let shared = crate::winamp::studio::open_document(None).unwrap();
    let mut doc = shared.lock().unwrap();
    doc.state(json!({"panel":"canvas", "layer":"main.play", "layers":["main.play"]}))
        .unwrap();
    let layer = doc.state_sheet_asked(None).unwrap();
    let sheets = doc.composite_images();
    let before = doc.project_bytes().unwrap();
    for columns in [1, 2, 7] {
        let (image, captions) = state_sheet(&doc, columns);
        assert_eq!(captions.len(), layer.variants.len());
        for (i, (label, [ox, oy, _, _])) in captions.iter().enumerate() {
            assert_eq!(label, &layer.labels[i]);
            let [x, y, w, h] = layer.variants[i];
            for sy in 0..h {
                for sx in 0..w {
                    assert_eq!(
                        image.get_pixel(ox + 2 + sx, oy + 4 + sy),
                        sheets[&layer.sheet].get_pixel(x + sx, y + sy)
                    );
                }
            }
        }
    }
    assert_eq!(doc.project_bytes().unwrap(), before);
}
