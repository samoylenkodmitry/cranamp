#[test]
fn every_svg_icon_renders_visible_pixels() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/studio/icons");
    let mut count = 0;
    let mut drawings = std::collections::HashSet::new();
    for file in std::fs::read_dir(root).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|e| e != "svg") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let painter = cranpose_ui::SvgPainter::from_bytes(&bytes)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let bitmap = painter.rasterize(cranpose_ui::Size::new(48., 48.)).unwrap();
        assert!(
            bitmap
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] > 0)
                .count()
                > 15,
            "{} is blank",
            path.display()
        );
        assert!(
            drawings.insert(bitmap.pixels().to_vec()),
            "{} duplicates another icon",
            path.display()
        );
        count += 1;
    }
    assert_eq!(count, 47);
}

#[test]
fn canvas_instruments_have_distinct_legible_cursors_with_a_visible_tip() {
    use super::{instrument_cursor, Icon};
    let mut images = std::collections::HashSet::new();
    for icon in [
        Icon::Brush,
        Icon::Line,
        Icon::Rectangle,
        Icon::Ellipse,
        Icon::Select,
        Icon::Text,
        Icon::Stamp,
        Icon::Curve,
        Icon::Glass,
        Icon::Taper,
        Icon::Hand,
    ] {
        let (image, tip) = instrument_cursor(icon);
        let at = ((tip[1] * image.width() + tip[0]) * 4) as usize;
        assert_eq!(
            image.pixels()[at + 3],
            255,
            "the drawing point must be visible"
        );
        assert!(
            images.insert(image.pixels().to_vec()),
            "{icon:?} repeats another instrument"
        );
        assert!(
            image
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] > 0)
                .count()
                > 100
        );
    }
}
