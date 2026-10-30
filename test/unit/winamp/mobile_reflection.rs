use super::*;

#[test]
fn player_background_uses_the_exact_skin_corner_rgb() {
    let mut skin = bundled_skin().unwrap();
    for rgba in [[17, 34, 51, 255], [255, 0, 255, 255], [240, 230, 210, 255]] {
        skin.main = ImageBitmap::from_rgba8(1, 1, rgba.to_vec()).unwrap();
        let color = corner_color(&skin);
        assert_eq!(
            color,
            Color(
                rgba[0] as f32 / 255.,
                rgba[1] as f32 / 255.,
                rgba[2] as f32 / 255.,
                1.
            )
        );
    }
}

#[test]
fn reflection_meets_the_visible_player_bottom_and_stays_inside_the_screen() {
    for (width, height) in [
        (320., 568.),
        (393., 780.),
        (440., 860.),
        (844., 390.),
        (1024., 1366.),
    ] {
        for (eq_visible, playlist_visible, main_shaded) in [
            (true, true, false),
            (true, false, false),
            (false, false, false),
            (false, false, true),
        ] {
            let state = WinampState {
                eq_visible,
                playlist_visible,
                main_shaded,
                ..WinampState::default()
            };
            let layout = player_layout(width, height, &state);
            let area = bounds(&state, layout, height, 1.);
            let bottom = ((main_height(main_shaded)
                + if eq_visible { EQ_HEIGHT } else { 0. }
                + if playlist_visible {
                    layout.playlist_height
                } else {
                    0.
                })
                * layout.scale)
                .round();
            assert!((area.top + area.source_height - bottom).abs() < 0.001);
            assert!(area.top >= -0.001 && area.depth > 0.);
            assert!(bottom + area.depth <= height + 0.001);
            assert!((area.source_height * COMPRESSION - area.depth).abs() < 0.001);
        }
    }
}
