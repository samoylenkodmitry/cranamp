use super::*;

#[test]
fn player_background_uses_the_exact_skin_corner_rgb() {
    let mut skin = bundled_skin().unwrap();
    for rgba in [[17, 34, 51, 255], [255, 0, 255, 255], [240, 230, 210, 255]] {
        skin.main = ImageBitmap::from_rgba8(1, 1, rgba.to_vec()).unwrap();
        let color = skin_corner_color(&skin);
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
                navigation: Navigation::Mobile(true),
                ..WinampState::default()
            };
            let layout = player_layout(width, height, &state);
            let bottom_inset = 34.;
            let area = bounds(
                player_bounds(&state, layout),
                width,
                height + bottom_inset,
                3.,
            );
            let skin_height = ((main_height(main_shaded)
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
            let bottom = skin_height + layout.content_top_inset;
            assert!((area.top + area.source_height - bottom).abs() < 0.001);
            assert!(area.top >= -0.001 && area.depth > 0.);
            assert!((bottom + area.depth - height - bottom_inset).abs() < 0.001);
            assert!((area.source_height - skin_height / 3.).abs() <= 1. / 6.);
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
    let state = WinampState {
        navigation: Navigation::Mobile(true),
        ..WinampState::default()
    };
    let layout = player_layout(440., 863., &state);
    let full = bounds(player_bounds(&state, layout), 440., 897., 3.);
    let contact = full.top + full.source_height;
    let shallow = bounds(player_bounds(&state, layout), 440., contact + 20., 3.);
    assert_eq!(shallow.source_height, full.source_height);
    assert_eq!(shallow.depth, 20.);
    assert!((full.source_height - (contact - layout.content_top_inset) / 3.).abs() <= 1. / 6.);
    assert_eq!(full.depth, 897. - contact);
}

#[test]
fn centered_player_reflection_has_room_to_expand_on_both_sides() {
    let state = WinampState {
        navigation: Navigation::Mobile(true),
        ..WinampState::default()
    };
    for (width, height) in [(393., 780.), (844., 390.), (1024., 1366.)] {
        let layout = player_layout(width, height, &state);
        let area = bounds(player_bounds(&state, layout), width, height + 34., 3.);
        let right_margin = area.width - area.source_left - area.source_width;
        assert!(area.source_left >= 24. && right_margin >= 24. - 0.34);
        assert!((area.source_left - right_margin).abs() <= 0.34);
        assert_eq!(area.width, width);
    }
}

#[test]
fn hiding_navigation_removes_player_margins_and_uses_the_available_height() {
    for (width, height) in [(320., 568.), (393., 780.), (440., 900.)] {
        let state = WinampState {
            navigation: Navigation::Mobile(false),
            ..WinampState::default()
        };
        let player = player_bounds(&state, player_layout(width, height, &state));
        assert_eq!(player.x, 0.);
        assert_eq!(player.y, 0.);
        assert!((player.width - width).abs() < 0.001);
        assert!((player.height - height).abs() < 0.001);
    }
}

#[test]
fn desktop_reflection_uses_the_inset_skin_and_reaches_below_navigation() {
    for (width, height) in [(275., 377.), (500., 600.)] {
        let (player, window_height, scale) = navigation::desktop_frame(width, height, true);
        assert_eq!(player.x, SIDE_MARGIN);
        assert!((width - player.x - player.width - SIDE_MARGIN).abs() < 0.001);
        assert!(scale < 1.);
        let area = bounds(player, width, window_height, 2.);
        assert_eq!(area.source_left, SIDE_MARGIN);
        assert!((area.source_width - player.width).abs() <= 0.25);
        assert!((area.top + area.source_height + area.depth - window_height).abs() < 0.001);
        assert!(area.depth > NAV_HEIGHT - NAV_TOP_INSET);
        let (plain, plain_height, plain_scale) = navigation::desktop_frame(width, height, false);
        assert_eq!(plain, Rect::from_size(Size::new(width, height)));
        assert_eq!(plain_height, height);
        assert_eq!(plain_scale, 1.);
    }
}
