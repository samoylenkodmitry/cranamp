use super::*;

#[test]
fn classic_font_paints_real_cells_preserves_view_and_undo() {
    let mut doc = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap();
    let before = doc.images["text.bmp"].clone();
    let view = serde_json::to_value(&doc.view).unwrap();
    doc.classic_font("#fff0c4", "#1d2031").unwrap();
    assert_eq!(serde_json::to_value(&doc.view).unwrap(), view);
    let atlas = &doc.images["text.bmp"];
    for row in 0..3 {
        for column in 0..31 {
            for y in 0..6 {
                assert_eq!(
                    atlas.get_pixel(column * 5 + 4, row * 6 + y).0,
                    [29, 32, 49, 255]
                );
            }
            for x in 0..5 {
                assert_eq!(
                    atlas.get_pixel(column * 5 + x, row * 6 + 5).0,
                    [29, 32, 49, 255]
                );
            }
        }
    }
    assert_eq!(atlas.get_pixel(151, 2).0, [29, 32, 49, 255]);
    assert_eq!(atlas.get_pixel(1, 0).0, [255, 240, 196, 255]);
    assert_eq!(atlas.get_pixel(61, 7).0, [255, 240, 196, 255]);
    assert!(doc.export_archive().is_ok());
    doc.undo();
    assert_eq!(doc.images["text.bmp"], before);
}
