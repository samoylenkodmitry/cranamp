fn sort_playlist_tracks_by_title(state: &mut WinampState) -> bool {
    sort_playlist_tracks_by_field(state, PlaylistSortField::Title)
}

fn remove_playlist_track_at(state: &mut WinampState, index: usize) -> bool {
    if index >= state.playlist.len() {
        state.status = "Track Missing".to_string();
        return false;
    }
    let removed_number = index + 1;
    remove_playlist_indices(state, &[index]);
    if !state.playlist.is_empty() {
        state.status = format!("Removed Track {removed_number}");
    }
    true
}

/// Every skin Cranamp ships draws its own pointers. A bundled skin with no
/// cursors falls back to the desktop arrow, which is the one part of the
/// window that would not belong to the skin.
#[test]
fn every_bundled_skin_carries_a_full_cursor_set() {
    for skin in BUNDLED_SKINS {
        let loaded = crate::winamp::skin::load_skin(skin.bytes)
            .unwrap_or_else(|error| panic!("{} does not load: {error:#}", skin.id));
        assert_eq!(
            loaded.cursors.len(),
            crate::winamp::cursors::SkinCursor::COUNT,
            "{} is missing cursors",
            skin.id
        );
    }
}

/// A pointer the skin swallows is worse than no pointer at all, so every
/// cursor has to carry ink that reads against what the skin covers itself
/// in.
#[test]
fn every_bundled_cursor_is_visible_on_the_skin_it_belongs_to() {
    use crate::winamp::studio::cursor_art::contrast;
    for skin in BUNDLED_SKINS {
        let loaded = crate::winamp::skin::load_skin(skin.bytes).expect("loads");
        let mut counts: std::collections::BTreeMap<[u8; 4], u32> =
            std::collections::BTreeMap::new();
        for pixel in loaded.main.pixels().as_chunks::<4>().0 {
            if pixel[3] >= 128 {
                *counts.entry(*pixel).or_default() += 1;
            }
        }
        let ground = counts
            .iter()
            .max_by_key(|(_, count)| **count)
            .map(|(colour, _)| *colour)
            .expect("a bundled skin has paint on its main sheet");

        for (role, name) in crate::winamp::cursors::SkinCursor::files() {
            let icon = loaded
                .cursors
                .get(role)
                .unwrap_or_else(|| panic!("{} has no {name}", skin.id));
            let PointerIcon::Custom(drawn) = icon else {
                panic!("{}'s {name} is a system shape, not the skin's art", skin.id);
            };
            let best = drawn
                .image()
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[3] >= 128)
                .map(|pixel| contrast(ground, *pixel))
                .fold(0.0f64, f64::max);
            assert!(
                best >= 3.0,
                "{}'s {name} is invisible on its own artwork: best contrast {best:.1} against {ground:?}",
                skin.id
            );
        }
    }
}

/// Two skins that share a cursor set would look like one skin the moment
/// the pointer moved, so the derived art has to follow the artwork.
#[test]
fn two_bundled_skins_do_not_share_the_same_pointer() {
    let pointer = |bytes: &'static [u8]| {
        crate::winamp::skin::load_skin(bytes)
            .expect("loads")
            .cursors
            .get(crate::winamp::cursors::SkinCursor::MainWindow)
            .expect("every bundled skin draws its window pointer")
            .clone()
    };
    let first = pointer(BUNDLED_SKINS[0].bytes);
    let second = pointer(BUNDLED_SKINS[1].bytes);
    assert_ne!(
        format!("{first:?}"),
        format!("{second:?}"),
        "{} and {} drew the same pointer",
        BUNDLED_SKINS[0].id,
        BUNDLED_SKINS[1].id
    );
}

#[test]
fn playlist_border_tiles_preserve_source_texels_and_crop_partial_edges() {
    let source = (31., 42., 20., 29.);
    assert_eq!(
        super::native_sprite_tiles(source, 20., 63.),
        vec![
            ((31., 42., 20., 29.), 0., 0.),
            ((31., 42., 20., 29.), 0., 29.),
            ((31., 42., 20., 5.), 0., 58.),
        ]
    );
    assert_eq!(
        super::native_sprite_tiles((127., 21., 25., 20.), 62., 20.),
        vec![
            ((127., 21., 25., 20.), 0., 0.),
            ((127., 21., 25., 20.), 25., 0.),
            ((127., 21., 12., 20.), 50., 0.),
        ]
    );
    assert!(super::native_sprite_tiles(source, 0., 0.).is_empty());
}
use super::*;
fn test_playlist(tracks: Vec<Track>) -> Rc<Vec<Track>> {
    Rc::new(tracks)
}
/// Whether `outer` covers every point of `inner`.
fn contains(outer: SpriteRect, inner: SpriteRect) -> bool {
    inner.0 >= outer.0
        && inner.1 >= outer.1
        && inner.0 + inner.2 <= outer.0 + outer.2
        && inner.1 + inner.3 <= outer.1 + outer.3
}

/// The cursor of the topmost region wins, and a later sibling is drawn on
/// top: a region that swallows another must therefore be listed before it,
/// or the window's own cursor would sit over every control on it.
fn assert_general_to_specific(areas: &[(SkinCursor, SpriteRect)]) {
    for (index, (region, area)) in areas.iter().enumerate() {
        for (earlier_region, earlier) in &areas[..index] {
            assert!(
                !(contains(*area, *earlier) && *area != *earlier),
                "{region:?} {area:?} swallows {earlier_region:?} {earlier:?} \
                 but is listed after it"
            );
        }
    }
}

#[test]
fn main_window_cursor_areas_run_general_to_specific() {
    assert_general_to_specific(&main_window_cursor_areas());
}

#[test]
fn equalizer_cursor_areas_run_general_to_specific() {
    assert_general_to_specific(&equalizer_cursor_areas());
}

#[test]
fn dragging_the_track_title_shows_a_later_part_of_it() {
    let title =
        "A Very Long Track Name That Cannot Possibly Fit In The Display At Once".to_string();

    let resting = track_text_window(title.clone(), true, 0.0);
    let scrolled = track_text_window(title.clone(), true, 6.0);

    assert_ne!(
        resting, scrolled,
        "the marquee phase picks which stretch of the title is on screen"
    );
    assert!(
        title.contains(scrolled.trim_end_matches("...")),
        "the scrolled window is a stretch of the real title"
    );
}

#[test]
fn a_title_nobody_has_scrolled_is_cut_with_an_ellipsis_while_stopped() {
    let title =
        "A Very Long Track Name That Cannot Possibly Fit In The Display At Once".to_string();

    let stopped = track_text_window(title.clone(), false, 0.0);
    let dragged = track_text_window(title, false, 6.0);

    assert!(
        stopped.ends_with("..."),
        "a stopped player cuts the title rather than scrolling it: {stopped}"
    );
    assert!(
        !dragged.ends_with("...") || dragged != stopped,
        "once the pointer has dragged it, the title scrolls even while stopped"
    );
}

#[test]
fn the_playlist_close_button_sits_inside_its_title_bar() {
    let layout = PlaylistCursorLayout {
        width: PLAYLIST_WIDTH,
        height: PLAYLIST_HEIGHT,
        list_height: PLAYLIST_LIST_BG.3,
        scroll_track_x: PLAYLIST_WIDTH - 15.0,
    };
    let close = playlist_close_button_area(layout.width);

    assert!(
        contains((0.0, 0.0, layout.width, PLAYLIST_DRAG_AREA.3), close),
        "the close button {close:?} must sit on the playlist's title bar"
    );
    assert!(
        playlist_cursor_areas(layout)
            .iter()
            .any(|(region, _)| *region == SkinCursor::PlaylistClose),
        "the playlist names a cursor for its close button, as PCLOSE.CUR expects"
    );
}

#[test]
fn playlist_cursor_areas_run_general_to_specific() {
    let layout = PlaylistCursorLayout {
        width: PLAYLIST_WIDTH,
        height: PLAYLIST_HEIGHT,
        list_height: PLAYLIST_LIST_BG.3,
        scroll_track_x: PLAYLIST_WIDTH - 15.0,
    };
    assert_general_to_specific(&playlist_cursor_areas(layout));
}

#[test]
fn every_main_window_cursor_region_lands_on_the_window() {
    for (region, area) in main_window_cursor_areas() {
        assert!(
            contains(MAIN_WINDOW, area),
            "{region:?} at {area:?} falls outside the main window"
        );
    }
}

#[test]
fn the_title_bar_buttons_carry_their_own_cursor_over_the_title_bar() {
    let buttons = [
        SkinCursor::MainMenu,
        SkinCursor::MainMinimize,
        SkinCursor::MainWindowshade,
        SkinCursor::MainClose,
    ];
    for (region, area) in main_window_cursor_areas() {
        if buttons.contains(&region) {
            assert!(
                contains(TITLE_DRAG_AREA, area),
                "{region:?} at {area:?} is not on the title bar"
            );
        }
    }
}

#[test]
fn the_playlist_resize_cursor_covers_the_handle_that_resizes_it() {
    let layout = PlaylistCursorLayout {
        width: 400.0,
        height: 300.0,
        list_height: 200.0,
        scroll_track_x: 385.0,
    };
    let resize = playlist_cursor_areas(layout)
        .into_iter()
        .find_map(|(region, area)| (region == SkinCursor::PlaylistResize).then_some(area))
        .expect("the playlist names a resize region");

    assert_eq!(
        resize,
        (
            layout.width - PLAYLIST_RESIZE_HANDLE,
            layout.height - PLAYLIST_RESIZE_HANDLE,
            PLAYLIST_RESIZE_HANDLE,
            PLAYLIST_RESIZE_HANDLE,
        ),
        "the cursor region must track the handle WindowResizeHandle places"
    );
}

#[test]
fn every_region_the_ui_draws_has_a_cursor_file_to_read() {
    let mut drawn: Vec<SkinCursor> = main_window_cursor_areas()
        .into_iter()
        .chain(shade_cursor_areas())
        .chain(equalizer_cursor_areas())
        .chain(playlist_cursor_areas(PlaylistCursorLayout {
            width: PLAYLIST_WIDTH,
            height: PLAYLIST_HEIGHT,
            list_height: PLAYLIST_LIST_BG.3,
            scroll_track_x: PLAYLIST_WIDTH - 15.0,
        }))
        .map(|(region, _)| region)
        .collect();
    drawn.sort_by_key(|region| region.file_name());
    drawn.dedup();

    assert_eq!(
        drawn.len(),
        cursors::SkinCursor::COUNT,
        "a cursor the loader reads has no region on screen, or the other way round"
    );
}

#[test]
fn time_digits_are_mapped_correctly() {
    assert_eq!(time_digits(0.0), [0, 0, 0, 0]);
    assert_eq!(time_digits(65.0), [0, 1, 0, 5]);
    assert_eq!(time_digits(-1.0), [0, 0, 0, 0]);
}
#[test]
fn the_rolled_up_time_sits_around_the_strips_colon() {
    assert_eq!(
        shade_time_glyphs(754.0),
        [(7.0, '1'), (12.0, '2'), (20.0, '3'), (25.0, '4')]
    );
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn sanitize_skin_file_name_strips_dirs_and_forces_extension() {
    assert_eq!(sanitize_skin_file_name("base-2.91.wsz"), "base-2.91.wsz");
    assert_eq!(sanitize_skin_file_name("Cool.ZIP"), "Cool.ZIP");
    assert_eq!(sanitize_skin_file_name("skins/MyTheme.wsz"), "MyTheme.wsz");
    assert_eq!(
        sanitize_skin_file_name("C:\\Downloads\\Theme.wsz"),
        "Theme.wsz"
    );
    assert_eq!(
        sanitize_skin_file_name("Untitled Skin"),
        "Untitled Skin.wsz"
    );
    assert_eq!(sanitize_skin_file_name("   "), "skin.wsz");
}
#[test]
fn slider_helpers_clamp_values() {
    assert_eq!(slider_frame(-1.0, 28), 0);
    assert_eq!(slider_frame(2.0, 28), 27);
    assert_eq!(slider_thumb_x(-1.0, 248.0, 29.0), 0.0);
    assert_eq!(slider_thumb_x(2.0, 248.0, 29.0), 219.0);
}
#[test]
fn eq_slider_background_uses_skin_frame_for_value() {
    assert_eq!(eq_slider_bg_rect(0.0), (13.0, 164.0, 14.0, 63.0));
    assert_eq!(eq_slider_bg_rect(0.5), (13.0, 229.0, 14.0, 63.0));
    assert_eq!(eq_slider_bg_rect(1.0), (208.0, 229.0, 14.0, 63.0));
}
#[test]
fn eq_presets_are_named_and_clamped() {
    assert_eq!(EQ_PRESETS.first().map(|preset| preset.label), Some("FLAT"));
    assert_eq!(EQ_PRESETS[0].values, DEFAULT_EQ_VALUES);
    assert!(EQ_PRESETS
        .iter()
        .any(|preset| preset.label == "ROCK" && preset.values != DEFAULT_EQ_VALUES));
    assert!(EQ_PRESETS
        .iter()
        .flat_map(|preset| preset.values)
        .all(|value| (0.0..=1.0).contains(&value)));
}
#[test]
fn playlist_duration_column_width_matches_duration_text() {
    assert_eq!(
        playlist_duration_column_width("1:23"),
        (WINAMP_PLAYLIST_ROW_CHAR_WIDTH * 4.0).ceil().max(30.0)
    );
    assert_eq!(
        playlist_duration_column_width("12:34"),
        (WINAMP_PLAYLIST_ROW_CHAR_WIDTH * 5.0).ceil().max(30.0)
    );
}
#[test]
fn playlist_title_column_leaves_right_duration_gap() {
    let line_width = WINAMP_PLAYLIST_ROW_CHAR_WIDTH * 12.0;
    let duration_width = playlist_duration_column_width("1:23");
    let expected = line_width - duration_width - WINAMP_PLAYLIST_ROW_CHAR_WIDTH * 2.0;
    assert!((playlist_title_column_width(line_width, duration_width) - expected).abs() < 0.001);
    assert_eq!(playlist_title_column_width(line_width, 0.0), line_width);
}
#[test]
fn playlist_visible_row_capacity_uses_full_list_area() {
    let default_list_height = PLAYLIST_HEIGHT - PLAYLIST_BOTTOM_LEFT_CORNER.3 - PLAYLIST_LIST_BG.1;
    assert_eq!(playlist_visible_row_capacity(default_list_height), 18);
}
#[test]
fn slider_artwork_stays_on_native_pixel_grid_at_all_skin_frames() {
    for frame in 0..=27 {
        let value = frame as f32 / 27.0;
        for offset in [
            slider_thumb_x(value, 248.0, 29.0),
            slider_thumb_x(value, 68.0, 14.0),
            slider_thumb_x(value, 38.0, 14.0),
            vertical_slider_thumb_y(value, 63.0, 11.0),
            vertical_slider_thumb_y_down(value, 145.0, 18.0),
        ] {
            assert_eq!(
                offset.fract(),
                0.0,
                "frame {frame} drifted off the skin grid"
            );
        }
    }
}

#[test]
fn vertical_slider_helpers_clamp_values() {
    assert_eq!(vertical_slider_thumb_y(-1.0, 63.0, 11.0), 52.0);
    assert_eq!(vertical_slider_thumb_y(2.0, 63.0, 11.0), 0.0);
    assert_eq!(vertical_slider_thumb_y_down(-1.0, 145.0, 18.0), 0.0);
    assert_eq!(vertical_slider_thumb_y_down(2.0, 145.0, 18.0), 127.0);
}
#[test]
fn progress_fraction_uses_duration_when_known() {
    assert_eq!(progress_fraction(30.0, Some(120.0)), 0.25);
    assert_eq!(progress_fraction(130.0, Some(120.0)), 1.0);
    assert_eq!(progress_fraction(30.0, None), 0.0);
}
#[test]
fn playlist_scroll_tracks_current_index() {
    assert_eq!(playlist_scroll_for_track(3, 20, 5, 0.0), 0.0);
    assert_eq!(playlist_scroll_for_track(10, 20, 5, 0.5), 0.5);
    assert!((playlist_scroll_for_track(10, 20, 5, 0.0) - (8.0 / 15.0)).abs() < f32::EPSILON);
    assert_eq!(playlist_scroll_for_track(99, 20, 5, 0.0), 1.0);
    assert_eq!(playlist_scroll_for_track(0, 1, 5, 0.5), 0.0);
}
#[test]
fn marquee_text_ping_pongs_long_titles() {
    let title = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string();
    let width = WINAMP_SYSTEM_MARQUEE_CHAR_WIDTH * 14.0;
    assert!(marquee_system_text(title.clone(), width, 0.0).starts_with("ABCDEFGHIJKLMN"));
    assert!(marquee_system_text(title.clone(), width, 2.0).starts_with("CDEFGHIJKLMNOP"));
    assert!(marquee_system_text(title, width, 18.0).starts_with("GHIJKLMNOPQRST"));
    assert_eq!(
        marquee_system_text("SHORT".to_string(), width, 60.0),
        "SHORT"
    );
}

#[test]
fn every_bundled_skin_loads_from_the_bytes_compiled_into_the_binary() {
    for skin in BUNDLED_SKINS {
        load_skin(skin.bytes)
            .unwrap_or_else(|error| panic!("{} does not load: {error:#}", skin.label));
        assert!(skin.label.ends_with("(Bundled)"), "{}", skin.label);
    }
    assert!(
        BUNDLED_SKINS.len() >= 2,
        "the list is meant to hold more than one"
    );
}
#[test]
fn the_skin_list_offers_each_bundled_skin_and_keeps_the_first_addressed_as_none() {
    let listed = list_library_skins();
    assert_eq!(listed[0].label, BUNDLED_SKIN_LABEL);
    assert_eq!(listed[0].path, None);
    assert_eq!(listed[1].label, "Catamp Feral Night (Bundled)");
    assert_eq!(
        listed[1].path.as_deref(),
        Some(std::path::Path::new("bundled:feral-night"))
    );
    assert_eq!(listed[2].label, "Catamp Cat Scan (Bundled)");
    assert_eq!(
        listed[2].path.as_deref(),
        Some(std::path::Path::new("bundled:cat-scan"))
    );
    assert_eq!(listed[3].label, "Catamp Seance (Bundled)");
    assert_eq!(
        listed[3].path.as_deref(),
        Some(std::path::Path::new("bundled:seance"))
    );
    assert_eq!(listed[4].label, "Catamp Salvage (Bundled)");
    assert_eq!(
        listed[4].path.as_deref(),
        Some(std::path::Path::new("bundled:salvage"))
    );
}
#[test]
fn a_bundled_id_survives_saving_but_is_never_handed_to_the_studio() {
    let id = "bundled:feral-night".to_string();
    assert_eq!(valid_saved_skin_path(Some(id.clone())), Some(id.clone()));
    assert_eq!(valid_saved_skin_path(Some("bundled:nope".into())), None);
    #[cfg(not(target_os = "ios"))]
    {
        assert_eq!(studio_skin_path(Some(id)), None);
        assert_eq!(
            studio_skin_path(Some("/tmp/real.wsz".into())),
            Some("/tmp/real.wsz".to_string())
        );
    }
}
#[test]
fn player_state_config_round_trips_settings_and_playlist() {
    let mut eq_values = [0.5; 11];
    eq_values[0] = 0.1;
    eq_values[10] = 0.9;
    let state = WinampState {
        shuffle: true,
        repeat: true,
        eq_visible: false,
        playlist_visible: false,
        main_shaded: true,
        main_visible: false,
        vis_mode: vis::VisMode::Oscilloscope,
        eq_enabled: false,
        eq_auto: true,
        eq_values,
        skin_path: Some("/tmp/Cranamp Skin.wsz".to_string()),
        playlist_scroll: 0.42,
        volume: 0.33,
        balance: 0.66,
        playlist: test_playlist(vec![test_track("One=Track"), test_track("Two\tTrack")]),
        current_index: Some(1),
        ..WinampState::default()
    };
    let saved = SavedPlayerState::from_state(&state);
    let parsed = parse_player_state(&serialize_player_state(&saved));
    assert_eq!(parsed, saved);
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn restored_player_state_filters_missing_tracks_and_remaps_current_index() {
    let fixture_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test/fixtures/tone-video.mp4");
    let saved = SavedPlayerState {
        volume: 0.25,
        current_index: Some(1),
        skin_path: Some("/tmp/cranamp-definitely-missing.wsz".to_string()),
        tracks: vec![
            SavedTrack {
                title: "Missing".to_string(),
                path: "/tmp/cranamp-definitely-missing.mp3".to_string(),
                duration_seconds: None,
            },
            SavedTrack {
                title: "Present".to_string(),
                path: fixture_path.to_string_lossy().to_string(),
                duration_seconds: Some(1.0),
            },
        ],
        ..SavedPlayerState::default()
    };
    let state = restore_saved_player_state(saved);
    assert_eq!(state.playlist.len(), 1);
    assert_eq!(state.playlist[0].title, "Present");
    assert_eq!(state.playlist[0].duration_seconds, Some(1.0));
    assert_eq!(state.current_index, Some(0));
    assert_eq!(state.volume, 0.25);
    assert_eq!(state.skin_path, None);
}
#[test]
fn elapsed_time_is_clamped_to_track_duration() {
    assert_eq!(normalized_elapsed_seconds(130.0, Some(120.0)), 120.0);
    assert_eq!(normalized_elapsed_seconds(30.0, Some(120.0)), 30.0);
}
#[test]
fn saved_window_config_round_trips() {
    let config = SavedWinampWindowConfig {
        main: SavedWindowConfig {
            position: Some(Point::new(10.0, 20.0)),
            size: Some(Size::new(MAIN_WIDTH, MAIN_HEIGHT)),
        },
        equalizer: SavedWindowConfig {
            position: Some(Point::new(10.0, 136.0)),
            size: Some(Size::new(EQ_WIDTH, EQ_HEIGHT)),
        },
        playlist: SavedWindowConfig {
            position: Some(Point::new(10.0, 252.0)),
            size: Some(Size::new(320.0, 240.0)),
        },
    };
    assert_eq!(
        parse_window_config(&serialize_window_config(config)),
        config
    );
}
#[test]
fn default_native_windows_stack_vertically() {
    assert_eq!(default_main_position(), Point::new(140.0, 120.0));
    assert_eq!(default_equalizer_position(), Point::new(140.0, 236.0));
    assert_eq!(default_playlist_position(), Point::new(140.0, 352.0));
}
#[test]
fn append_playlist_tracks_starts_empty_playlist_at_first_added_track() {
    let mut state = WinampState::default();
    let should_start = append_playlist_tracks(&mut state, vec![test_track("First")]);
    assert!(should_start);
    assert_eq!(state.playlist.as_slice(), &[test_track("First")]);
    assert_eq!(state.current_index, Some(0));
    assert_eq!(state.playlist_scroll, 0.0);
    assert_eq!(state.position, 0.0);
    assert_eq!(state.elapsed_seconds, 0.0);
    assert_eq!(state.status, "Loaded 1 Track(s)");
}
#[test]
fn append_playlist_tracks_preserves_current_track_when_playlist_exists() {
    let mut state = WinampState {
        playback: PlaybackState::Playing,
        playlist: test_playlist(vec![test_track("First")]),
        current_index: Some(0),
        playlist_scroll: 0.25,
        status: "Playing First".to_string(),
        ..WinampState::default()
    };
    let should_start = append_playlist_tracks(&mut state, vec![test_track("Second")]);
    assert!(!should_start);
    assert_eq!(
        state.playlist.as_slice(),
        &[test_track("First"), test_track("Second")]
    );
    assert_eq!(state.current_index, Some(0));
    assert_eq!(state.playback, PlaybackState::Playing);
    assert_eq!(state.playlist_scroll, 0.25);
    assert_eq!(state.status, "Added 1 Track(s)");
}
#[test]
fn playlist_single_click_selects_without_playing() {
    let mut state = WinampState {
        playlist: test_playlist(vec![test_track("First"), test_track("Second")]),
        current_index: Some(0),
        selected_indices: vec![0],
        selection_anchor: Some(0),
        ..WinampState::default()
    };
    let should_play = handle_playlist_row_click_in_state(&mut state, 1, 1000, Modifiers::NONE);
    assert!(!should_play);
    assert_eq!(state.current_index, Some(0));
    assert_eq!(state.playback, PlaybackState::Stopped);
    assert_eq!(state.selected_indices, vec![1]);
    assert_eq!(state.selection_anchor, Some(1));
    assert_eq!(state.playlist_last_click_index, Some(1));
}
#[test]
fn playlist_second_plain_click_requests_play() {
    let mut state = WinampState {
        playlist: test_playlist(vec![test_track("First"), test_track("Second")]),
        playlist_last_click_index: Some(1),
        playlist_last_click_ms: 1000,
        ..WinampState::default()
    };
    let should_play = handle_playlist_row_click_in_state(&mut state, 1, 1200, Modifiers::NONE);
    assert!(should_play);
    assert_eq!(state.playlist_last_click_index, None);
    assert_eq!(state.playlist_last_click_ms, 0);
}
#[test]
fn playlist_shift_and_ctrl_click_match_winamp_selection_rules() {
    let mut state = WinampState {
        playlist: test_playlist(vec![
            test_track("First"),
            test_track("Second"),
            test_track("Third"),
            test_track("Fourth"),
        ]),
        selected_indices: vec![1],
        selection_anchor: Some(1),
        ..WinampState::default()
    };
    select_playlist_row_in_state(
        &mut state,
        3,
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(state.selected_indices, vec![1, 2, 3]);
    assert_eq!(state.selection_anchor, Some(1));
    select_playlist_row_in_state(
        &mut state,
        2,
        Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(state.selected_indices, vec![1, 3]);
    assert_eq!(state.selection_anchor, Some(1));
    select_playlist_row_in_state(
        &mut state,
        0,
        Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        },
    );
    assert_eq!(state.selected_indices, vec![0, 1, 3]);
    assert_eq!(state.selection_anchor, Some(0));
}
#[test]
fn remove_playlist_track_at_stops_removed_current_and_keeps_next_selected() {
    let mut state = WinampState {
        playback: PlaybackState::Playing,
        playlist: test_playlist(vec![
            test_track("First"),
            test_track("Second"),
            test_track("Third"),
        ]),
        current_index: Some(1),
        position: 0.5,
        elapsed_seconds: 12.0,
        duration_seconds: Some(120.0),
        ..WinampState::default()
    };
    assert!(remove_playlist_track_at(&mut state, 1));
    assert_eq!(
        state.playlist.as_slice(),
        &[test_track("First"), test_track("Third")]
    );
    assert_eq!(state.current_index, Some(1));
    assert_eq!(state.playback, PlaybackState::Stopped);
    assert_eq!(state.position, 0.0);
    assert_eq!(state.elapsed_seconds, 0.0);
    assert_eq!(state.duration_seconds, None);
    assert_eq!(state.status, "Removed Track 2");
}
#[test]
fn sort_playlist_tracks_by_title_preserves_current_track() {
    let mut state = WinampState {
        playlist: test_playlist(vec![
            test_track("Bravo"),
            test_track("Alpha"),
            test_track("Charlie"),
        ]),
        current_index: Some(0),
        selected_indices: vec![0, 2],
        selection_anchor: Some(2),
        ..WinampState::default()
    };
    assert!(sort_playlist_tracks_by_title(&mut state));
    assert_eq!(
        state.playlist.as_slice(),
        &[
            test_track("Alpha"),
            test_track("Bravo"),
            test_track("Charlie")
        ]
    );
    assert_eq!(state.current_index, Some(1));
    assert_eq!(state.selected_indices, vec![1, 2]);
    assert_eq!(state.selection_anchor, Some(2));
    assert_eq!(state.status, "Playlist Sorted");
}
#[test]
fn remove_playlist_indices_remaps_current_and_selection() {
    let mut state = WinampState {
        playback: PlaybackState::Playing,
        playlist: test_playlist(vec![
            test_track("First"),
            test_track("Second"),
            test_track("Third"),
            test_track("Fourth"),
        ]),
        current_index: Some(2),
        selected_indices: vec![1, 3],
        selection_anchor: Some(3),
        position: 0.5,
        elapsed_seconds: 12.0,
        duration_seconds: Some(120.0),
        ..WinampState::default()
    };
    assert_eq!(remove_playlist_indices(&mut state, &[1, 3]), 2);
    assert_eq!(
        state.playlist.as_slice(),
        &[test_track("First"), test_track("Third")]
    );
    assert_eq!(state.current_index, Some(1));
    assert_eq!(state.selected_indices, vec![1]);
    assert_eq!(state.playback, PlaybackState::Playing);
}
#[test]
fn remove_playlist_indices_stops_when_current_is_removed() {
    let mut state = WinampState {
        playback: PlaybackState::Playing,
        playlist: test_playlist(vec![test_track("First"), test_track("Second")]),
        current_index: Some(0),
        selected_indices: vec![0],
        position: 0.5,
        elapsed_seconds: 12.0,
        duration_seconds: Some(120.0),
        ..WinampState::default()
    };
    assert_eq!(remove_playlist_indices(&mut state, &[0]), 1);
    assert_eq!(state.playlist.as_slice(), &[test_track("Second")]);
    assert_eq!(state.current_index, Some(0));
    assert_eq!(state.selected_indices, vec![0]);
    assert_eq!(state.playback, PlaybackState::Stopped);
    assert_eq!(state.position, 0.0);
    assert_eq!(state.elapsed_seconds, 0.0);
    assert_eq!(state.duration_seconds, None);
}
#[test]
fn duplicate_playlist_indices_uses_path_when_available() {
    let playlist = vec![
        test_track_with_path("First", "/tmp/one.mp3"),
        test_track_with_path("Copy", "/tmp/one.mp3"),
        test_track_with_path("Other", "/tmp/two.mp3"),
    ];
    assert_eq!(duplicate_playlist_indices(&playlist), vec![1]);
}
#[test]
fn select_search_query_uses_current_artist_prefix() {
    let state = WinampState {
        playlist: test_playlist(vec![
            test_track("Celldweller - Eon"),
            test_track("Celldweller - One Good Reason"),
        ]),
        current_index: Some(1),
        ..WinampState::default()
    };
    assert_eq!(
        playlist_search_query(&state),
        Some("Celldweller".to_string())
    );
}
#[test]
fn playlist_search_filter_selects_matching_title_or_path() {
    let mut state = WinampState {
        playlist: test_playlist(vec![
            test_track_with_path("Blue October - Somebody", "/music/blue.mp3"),
            test_track_with_path("Celldweller - Eon", "/music/eon.flac"),
            test_track_with_path("Other", "/music/celldweller-live.ogg"),
        ]),
        ..WinampState::default()
    };
    apply_playlist_search_filter_in_state(&mut state, "celldweller");
    assert_eq!(state.selected_indices, vec![1, 2]);
    assert_eq!(state.selection_anchor, Some(2));
}
#[test]
fn parse_m3u_playlist_accepts_plain_paths_and_extinf() {
    let input =
        "#EXTM3U\n#EXTINF:195,Broods - Heartlines\nrelative/song.mp3\n/home/s/Music/Other.flac\n";
    let tracks = parse_m3u_playlist(
        input,
        PlaylistBase::Directory(std::path::Path::new("/tmp/list")),
    );
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].title, "Broods - Heartlines");
    assert_eq!(tracks[0].duration_seconds, Some(195.0));
    assert_eq!(
        tracks[0].path.as_deref(),
        Some("/tmp/list/relative/song.mp3")
    );
    assert_eq!(tracks[1].title, "Other");
    assert_eq!(tracks[1].path.as_deref(), Some("/home/s/Music/Other.flac"));
}
#[test]
fn remote_m3u_keeps_absolute_urls_and_resolves_relative_entries() {
    let input = concat!(
        "#EXTM3U\n",
        "#EXTINF:206,Artist - Absolute\n",
        "https://other.example/absolute.mp3\n",
        "#EXTINF:247,Artist - Root Relative\n",
        "/top.mp3\n",
        "#EXTINF:120,Artist - Sibling\n",
        "sibling.mp3\n",
    );
    let tracks = parse_m3u_playlist(
        input,
        PlaylistBase::Url("https://host.example/sets/list.m3u?token=abc"),
    );
    assert_eq!(tracks.len(), 3);
    assert_eq!(
        tracks[0].path.as_deref(),
        Some("https://other.example/absolute.mp3")
    );
    assert_eq!(
        tracks[1].path.as_deref(),
        Some("https://host.example/top.mp3")
    );
    assert_eq!(
        tracks[2].path.as_deref(),
        Some("https://host.example/sets/sibling.mp3")
    );
    assert_eq!(tracks[0].duration_seconds, Some(206.0));
}
#[test]
fn importing_a_playlist_leaves_absolute_urls_alone() {
    let input = "#EXTM3U\n#EXTINF:10,A - B\nhttps://host.example/a.mp3\n";
    let tracks = parse_m3u_playlist(input, PlaylistBase::None);
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        tracks[0].path.as_deref(),
        Some("https://host.example/a.mp3")
    );
}
#[test]
fn a_query_string_does_not_hide_the_extension() {
    assert!(is_supported_playlist_path(
        "https://host.example/track.mp3?token=abc"
    ));
    assert!(is_supported_playlist_path("https://host.example/track.MP3"));
    assert!(!is_supported_playlist_path("https://host.example/stream"));
    assert_eq!(
        media_extension("https://host.example/a/b.m3u8#x"),
        Some("m3u8".to_string())
    );
}
#[test]
fn join_url_follows_the_playlist_not_the_query() {
    let base = "https://host.example/sets/list.m3u?v=2";
    assert_eq!(join_url(base, "a.mp3"), "https://host.example/sets/a.mp3");
    assert_eq!(join_url(base, "/a.mp3"), "https://host.example/a.mp3");
    assert_eq!(
        join_url(base, "https://elsewhere.example/a.mp3"),
        "https://elsewhere.example/a.mp3"
    );
    assert_eq!(
        join_url("https://host.example", "a.mp3"),
        "https://host.example/a.mp3"
    );
}
#[test]
fn pls_playlists_are_read_in_index_order() {
    let input = concat!(
        "[playlist]\n",
        "NumberOfEntries=2\n",
        "File2=https://host.example/second.mp3\n",
        "Title2=Second\n",
        "Length2=-1\n",
        "File1=first.mp3\n",
        "Title1=First\n",
        "Length1=90\n",
    );
    let tracks = parse_pls_playlist(input, PlaylistBase::Url("https://host.example/x/list.pls"));
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].title, "First");
    assert_eq!(
        tracks[0].path.as_deref(),
        Some("https://host.example/x/first.mp3")
    );
    assert_eq!(tracks[0].duration_seconds, Some(90.0));
    assert_eq!(tracks[1].title, "Second");
    assert_eq!(tracks[1].duration_seconds, None);
}
#[test]
fn playlist_formats_are_recognised_by_extension_and_content_type() {
    assert_eq!(
        playlist_format_for_extension("m3u"),
        Some(PlaylistFormat::M3u)
    );
    assert_eq!(
        playlist_format_for_extension("m3u8"),
        Some(PlaylistFormat::M3u)
    );
    assert_eq!(
        playlist_format_for_extension("pls"),
        Some(PlaylistFormat::Pls)
    );
    assert_eq!(playlist_format_for_extension("mp3"), None);
    assert_eq!(
        playlist_format_for_content_type("audio/x-mpegurl; charset=utf-8"),
        Some(PlaylistFormat::M3u)
    );
    assert_eq!(
        playlist_format_for_content_type("audio/x-scpls"),
        Some(PlaylistFormat::Pls)
    );
    assert_eq!(playlist_format_for_content_type("audio/mpeg"), None);
}
#[test]
fn only_playlist_urls_are_resolved_before_playing() {
    for url in [
        "https://host.example/list.m3u",
        "http://host.example/list.m3u8",
        "https://host.example/list.pls",
        "https://host.example/list.M3U",
        "https://host.example/list.m3u?token=abc",
    ] {
        let track = test_track_with_path("Station", url);
        assert_eq!(
            playlist_entry_url(&track),
            Some(url),
            "should resolve before playing: {url}"
        );
    }
    for url in [
        "https://host.example/track.mp3",
        "https://host.example/stream",
        "https://host.example/stream?fmt=mp3",
        "/home/someone/Music/list.m3u",
        "file:///home/someone/list.m3u",
    ] {
        let track = test_track_with_path("Thing", url);
        assert_eq!(
            playlist_entry_url(&track),
            None,
            "should play directly: {url}"
        );
    }
    assert_eq!(
        playlist_entry_url(&Track {
            title: "No Path".to_string(),
            path: None,
            duration_seconds: None,
        }),
        None
    );
}
#[test]
fn the_shipped_station_is_an_entry_that_resolves_on_play() {
    let station = audio::track_from_title_path(DEFAULT_STATION_TITLE, DEFAULT_STATION_URL);
    assert_eq!(playlist_entry_url(&station), Some(DEFAULT_STATION_URL));
}
#[test]
fn a_saved_url_entry_survives_a_restart() {
    let restored = restore_saved_track(SavedTrack {
        title: "Cranamp FM".to_string(),
        path: DEFAULT_STATION_URL.to_string(),
        duration_seconds: None,
    })
    .expect("a URL entry should survive being saved and restored");
    assert_eq!(restored.path.as_deref(), Some(DEFAULT_STATION_URL));
    assert_eq!(restored.title, "Cranamp FM");
    assert!(restore_saved_track(SavedTrack {
        title: "Gone".to_string(),
        path: "/definitely/not/here/song.mp3".to_string(),
        duration_seconds: None,
    })
    .is_none());
}
#[test]
fn format_m3u_playlist_writes_extm3u_and_durations() {
    let playlist = vec![Track {
        title: "Broods - Heartlines".to_string(),
        path: Some("/home/s/Music/Broods - Heartlines.mp3".to_string()),
        duration_seconds: Some(199.0),
    }];
    let text = format_m3u_playlist(&playlist);
    assert!(text.starts_with("#EXTM3U\n"));
    assert!(text.contains("#EXTINF:199,Broods - Heartlines\n"));
    assert!(text.contains("/home/s/Music/Broods - Heartlines.mp3\n"));
}
#[test]
fn scroll_playlist_by_rows_clamps_to_available_rows() {
    let mut state = WinampState {
        playlist: test_playlist(
            (0..20)
                .map(|index| test_track(&format!("Track {index:02}")))
                .collect(),
        ),
        playlist_visible_rows: 5,
        ..WinampState::default()
    };
    scroll_playlist_by_rows_in_state(&mut state, 3);
    assert!((state.playlist_scroll - (3.0 / 15.0)).abs() < f32::EPSILON);
    scroll_playlist_by_rows_in_state(&mut state, 99);
    assert_eq!(state.playlist_scroll, 1.0);
    scroll_playlist_by_rows_in_state(&mut state, -99);
    assert_eq!(state.playlist_scroll, 0.0);
}
#[test]
fn a_track_the_backend_refuses_says_so_instead_of_stopping_the_clock() {
    let mut state = WinampState {
        playlist: test_playlist(vec![test_track("Glass"), test_track("Ping")]),
        current_index: Some(0),
        playback: PlaybackState::Playing,
        elapsed_seconds: 12.0,
        position: 0.5,
        ..WinampState::default()
    };
    apply_playback_failure_in_state(&mut state, "unsupported source");
    assert_eq!(state.playback, PlaybackState::Stopped);
    assert_eq!(state.elapsed_seconds, 0.0);
    assert_eq!(state.position, 0.0);
    assert!(
        state.status.contains("Glass") && state.status.contains("unsupported source"),
        "the refusal must name the track and the reason, got {:?}",
        state.status
    );
}
#[test]
fn automatic_advance_stops_at_playlist_end_without_repeat() {
    let state = WinampState {
        playlist: test_playlist(vec![test_track("First"), test_track("Second")]),
        current_index: Some(1),
        ..WinampState::default()
    };
    let plan = playlist_advance_plan(&state, TrackDirection::Next, TrackAdvanceMode::Automatic);
    assert_eq!(plan, None);
}
#[test]
fn automatic_advance_wraps_at_playlist_end_with_repeat() {
    let state = WinampState {
        repeat: true,
        playlist: test_playlist(vec![test_track("First"), test_track("Second")]),
        current_index: Some(1),
        ..WinampState::default()
    };
    let plan = playlist_advance_plan(&state, TrackDirection::Next, TrackAdvanceMode::Automatic);
    assert_eq!(
        plan,
        Some(TrackAdvancePlan {
            index: 0,
            shuffle_order: None,
        })
    );
}
#[test]
fn manual_advance_wraps_without_repeat() {
    let state = WinampState {
        playlist: test_playlist(vec![test_track("First"), test_track("Second")]),
        current_index: Some(1),
        ..WinampState::default()
    };
    let next = playlist_advance_plan(&state, TrackDirection::Next, TrackAdvanceMode::Manual);
    let previous =
        playlist_advance_plan(&state, TrackDirection::Previous, TrackAdvanceMode::Manual);
    assert_eq!(
        next,
        Some(TrackAdvancePlan {
            index: 0,
            shuffle_order: None,
        })
    );
    assert_eq!(
        previous,
        Some(TrackAdvancePlan {
            index: 0,
            shuffle_order: None,
        })
    );
}
#[test]
fn shuffle_advance_follows_the_existing_order() {
    let state = WinampState {
        shuffle: true,
        playlist: test_playlist(vec![
            test_track("First"),
            test_track("Second"),
            test_track("Third"),
        ]),
        current_index: Some(0),
        shuffle_order: vec![0, 2, 1],
        ..WinampState::default()
    };
    let plan = playlist_advance_plan(&state, TrackDirection::Next, TrackAdvanceMode::Manual);
    assert_eq!(
        plan,
        Some(TrackAdvancePlan {
            index: 2,
            shuffle_order: None,
        })
    );
}
#[test]
fn shuffle_repeat_rebuilds_order_after_exhaustion() {
    let state = WinampState {
        shuffle: true,
        repeat: true,
        playlist: test_playlist(vec![
            test_track("First"),
            test_track("Second"),
            test_track("Third"),
        ]),
        current_index: Some(2),
        shuffle_order: vec![0, 1, 2],
        ..WinampState::default()
    };
    let plan = playlist_advance_plan(&state, TrackDirection::Next, TrackAdvanceMode::Automatic)
        .expect("repeat should continue the playlist");
    let replacement = plan
        .shuffle_order
        .as_ref()
        .expect("shuffle should rebuild order on repeat wrap");
    assert_ne!(plan.index, 2);
    assert_eq!(replacement.first(), Some(&2));
    assert_eq!(replacement.get(1), Some(&plan.index));
    assert!(valid_shuffle_order(replacement, state.playlist.len()));
}
#[test]
fn seeded_shuffle_order_keeps_current_track_first() {
    let order = shuffled_order_with_seed(4, 2, 42);
    assert_eq!(order.first(), Some(&2));
    assert!(valid_shuffle_order(&order, 4));
}
fn test_track(title: &str) -> Track {
    Track {
        title: title.to_string(),
        path: Some(format!("/tmp/{title}.mp3")),
        duration_seconds: None,
    }
}
fn test_track_with_path(title: &str, path: &str) -> Track {
    Track {
        title: title.to_string(),
        path: Some(path.to_string()),
        duration_seconds: None,
    }
}

#[test]
fn the_visualizer_paints_its_field_dots_and_bars_in_the_skins_colours() {
    let mut viscolor = VisColor([[9, 9, 9, 255]; 24]);
    viscolor.0[1] = [1, 1, 1, 255];
    viscolor.0[17] = [17, 17, 17, 255];
    let mut analyzer = vis::Analyzer::default();
    analyzer.advance(&[1.0; vis::BARS], 1);
    let background = [255, 0, 255, 255];
    let rects = vis_rects(&analyzer.runs(), &viscolor, background, vis::WIDTH);
    assert_eq!(rects[0].1, background, "the field is the skin's background");
    assert_eq!(rects[0].0.width, 76.0);
    assert!(rects
        .iter()
        .any(|(rect, colour)| rect.width == 1.0 && *colour == [1, 1, 1, 255]));
    assert!(rects
        .iter()
        .any(|(rect, colour)| rect.y == 15.0 && rect.width == 3.0 && *colour == [17, 17, 17, 255]));

    let mini = vis_rects(&analyzer.runs(), &viscolor, background, 72);
    assert_eq!(mini[0].0.width, 72.0);
    assert!(
        mini.iter().all(|(rect, _)| rect.x + rect.width <= 72.0),
        "the playlist's panel shows 72 of the columns"
    );
}

#[test]
fn alt_w_closes_the_main_window_but_never_the_last_one_open() {
    let mut state = WinampState::default();
    toggle_winamp_window(&mut state, keys::WinampWindowKey::Main);
    assert!(!state.main_visible);
    state.eq_visible = false;
    toggle_winamp_window(&mut state, keys::WinampWindowKey::Playlist);
    assert!(
        state.main_visible,
        "closing the playlist too brings the main window back"
    );
    state.playlist_visible = false;
    toggle_winamp_window(&mut state, keys::WinampWindowKey::Main);
    assert!(state.main_visible, "the main window alone stays open");
}
