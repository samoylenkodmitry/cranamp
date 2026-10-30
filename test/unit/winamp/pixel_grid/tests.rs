use super::*;
#[test]
fn native_size_rounding_does_not_request_another_resize() {
    let density = 130. / 96.;
    let wanted = Size::new(275., 493.);
    let actual = Size::new(
        (wanted.width * density).round() / density,
        (wanted.height * density).round() / density,
    );
    assert_ne!(actual, wanted);
    assert!(same_surface_size(actual, wanted, density));
    assert!(!same_surface_size(actual, Size::new(275., 377.), density));
}
#[test]
fn desktop_art_defaults_to_one_and_a_half_of_the_previous_size() {
    for density in [1., 130. / 96., 1.5, 1.75, 2., 2.5, 3.] {
        let physical = desktop_scale(density) * density;
        assert!((physical - 1.5 * density.round().max(1.)).abs() < 0.0001);
    }
    assert_eq!(desktop_scale(2.), 1.5);
    assert_eq!(desktop_scale(f32::NAN), 1.5);
}
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
