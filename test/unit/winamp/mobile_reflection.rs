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
fn reflection_meets_the_player_and_reaches_the_physical_screen_bottom() {
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
            let bottom_inset = 34.;
            let area = bounds(&state, layout, width, height + bottom_inset, 3.);
            let bottom = ((main_height(main_shaded)
                + if eq_visible { EQ_HEIGHT } else { 0. }
                + if playlist_visible {
                    layout.playlist_height
                } else {
                    0.
                })
                * layout.scale
                * 3.)
                .round()
                / 3.;
            assert!((area.top + area.source_height - bottom).abs() < 0.001);
            assert!(area.top >= -0.001 && area.depth > 0.);
            assert!((bottom + area.depth - height - bottom_inset).abs() < 0.001);
            assert!((area.source_height - bottom / 3.).abs() <= 1. / 6.);
            assert_eq!(area.width, width);
            assert!(area.source_left >= 0.);
            assert!(area.source_left + area.source_width <= width + 0.34);
            if playlist_visible {
                let nav_top = height - NAV_HEIGHT + NAV_TOP_INSET;
                assert!((nav_top - bottom - 6.).abs() <= 1. / 6.);
            }
        }
    }
}

#[test]
fn reflection_captures_the_lower_third_even_when_the_floor_is_shallow() {
    let state = WinampState::default();
    let layout = player_layout(440., 863., &state);
    let full = bounds(&state, layout, 440., 897., 3.);
    let contact = full.top + full.source_height;
    let shallow = bounds(&state, layout, 440., contact + 20., 3.);
    assert_eq!(shallow.source_height, full.source_height);
    assert_eq!(shallow.depth, 20.);
    assert!((full.source_height - contact / 3.).abs() <= 1. / 6.);
    assert_eq!(full.depth, 897. - contact);
}

#[test]
fn centered_player_reflection_can_extend_left_of_the_skin() {
    let state = WinampState::default();
    let layout = player_layout(844., 390., &state);
    let area = bounds(&state, layout, 844., 424., 3.);
    assert!(area.source_left > 0.);
    let near_left = area.source_left + area.source_width * (SKEW - SPREAD / 2.);
    let near_right = area.source_left + area.source_width * (1. + SKEW + SPREAD / 2.);
    assert!(near_left < area.source_left);
    assert!(near_right < area.source_left + area.source_width);
}
