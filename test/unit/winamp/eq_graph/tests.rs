use super::*;
fn atlas() -> ImageBitmap {
    let mut pixels = vec![0; 116 * 313 * 4];
    for row in 0..19 {
        let i = ((294 + row) * 116 + 115) * 4;
        pixels[i..i + 4].copy_from_slice(&[row as u8, 128, 255, 255]);
    }
    ImageBitmap::from_rgba8(116, 313, pixels).unwrap()
}
#[test]
fn native_curve_uses_the_eq_atlas_gradient_and_full_band_range() {
    for (value, row) in [(0.0, 18), (0.5, 9), (1.0, 0)] {
        let image = render([value; 11], &atlas());
        for x in 2..=110 {
            let i = (row * 113 + x) * 4;
            assert_eq!(&image.pixels()[i..i + 4], &[row as u8, 128, 255, 255]);
        }
        assert_eq!(
            image
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] != 0)
                .count(),
            109
        );
    }
    assert_eq!(preamp_y(0.0), 0);
    assert_eq!(preamp_y(0.5), 9);
    assert_eq!(preamp_y(1.0), 18);
}
#[test]
fn extreme_bands_stay_inside_native_graph_and_cross_each_control_point() {
    let values = std::array::from_fn(|i| if i % 2 == 0 { 1.0 } else { 0.0 });
    let image = render(values, &atlas());
    assert_eq!(image.pixels().len(), 113 * 19 * 4);
    for band in 0..10 {
        let row = if values[band + 1] == 1.0 { 0 } else { 18 };
        let i = (row * 113 + 2 + band * 12) * 4;
        assert_eq!(image.pixels()[i + 3], 255);
    }
}
