use super::*;
#[test]
fn tiled_edges_cover_the_same_pixels_as_one_continuous_region() {
    for density in [1., 1.5, 2., 2.625, 3.5] {
        for physical_width in [573., 720., 1080., 1280., 1344.] {
            let scale = physical_width / (275. * density);
            for oy in [0., 116., 232.] {
                let region = bounds([0., 20., 12., 500.], [0., oy], scale, density);
                let mut end = region.y;
                for (_, _, dy) in super::super::native_sprite_tiles((0., 42., 12., 29.), 12., 500.)
                {
                    let h = (500. - dy).min(29.);
                    let tile = bounds([0., 20. + dy, 12., h], [0., oy], scale, density);
                    assert!(
                        (tile.y - end).abs() < 0.001,
                        "gap at {dy}, density {density}, width {physical_width}"
                    );
                    end = tile.y + tile.height;
                }
                assert!((end - region.y - region.height).abs() < 0.001);
            }
        }
    }
}
#[test]
fn stacked_panels_share_the_same_physical_edge() {
    for density in [1., 2.625, 3.] {
        for physical_width in [573., 1080., 1344.] {
            let scale = physical_width / (275. * density);
            let main = bounds([0., 0., 275., 116.], [0., 0.], scale, density);
            let eq = bounds([0., 116., 275., 116.], [0., 0.], scale, density);
            let pl = bounds([0., 232., 275., 384.], [0., 0.], scale, density);
            assert_eq!(main.y + main.height, eq.y);
            assert!((eq.y + eq.height - pl.y).abs() < 0.001);
            let local = bounds([0., 0., 275., 116.], [0., 116.], scale, density);
            assert!((local.height - eq.height).abs() < 0.001);
        }
    }
}
