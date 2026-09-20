use super::*;

fn sheet(colours: &[[u8; 4]]) -> BTreeMap<String, RgbaImage> {
    let mut image = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
    for (index, colour) in colours.iter().enumerate() {
        for y in 0..10 {
            image.put_pixel(index as u32, y, Rgba(*colour));
        }
    }
    BTreeMap::from([("main.bmp".to_string(), image)])
}

#[test]
fn a_blank_skin_falls_back_to_black_on_white() {
    assert_eq!(palette(&BTreeMap::new()), Palette::default());
}

fn skin(ground: [u8; 4], marks: &[[u8; 4]]) -> BTreeMap<String, RgbaImage> {
    let mut image = RgbaImage::from_pixel(40, 10, Rgba(ground));
    for (index, colour) in marks.iter().enumerate() {
        for y in 0..10 {
            image.put_pixel(index as u32, y, Rgba(*colour));
        }
    }
    BTreeMap::from([("main.bmp".to_string(), image)])
}

#[test]
fn the_palette_takes_its_ink_and_body_from_the_artwork() {
    let found = palette(&skin([20, 20, 30, 255], &[[240, 240, 230, 255]]));
    assert_eq!(found.ground, [20, 20, 30, 255]);
    assert_eq!(
        found.body,
        [240, 240, 230, 255],
        "the skin's own light colour reads on its own dark ground"
    );
    assert_eq!(found.ink, [20, 20, 30, 255]);
}

#[test]
fn the_accent_is_the_most_colourful_thing_on_the_sheet() {
    let found = palette(&sheet(&[
        [20, 20, 30, 255],
        [240, 240, 230, 255],
        [200, 40, 30, 255],
    ]));
    assert_eq!(found.accent, [200, 40, 30, 255]);
}

#[test]
fn transparent_pixels_do_not_reach_the_palette() {
    let found = palette(&sheet(&[[9, 9, 9, 0], [40, 40, 40, 255]]));
    assert_ne!(found.ink, [9, 9, 9, 0]);
}

fn reads(found: &Palette) -> f64 {
    contrast(found.ground, found.body).max(contrast(found.ground, found.ink))
}

#[test]
fn a_pointer_stays_visible_on_a_skin_that_is_all_one_pale_colour() {
    let found = palette(&sheet(&[[236, 236, 232, 255], [240, 240, 238, 255]]));
    assert!(
        reads(&found) >= LEGIBLE,
        "neither body {:?} nor ink {:?} reads on ground {:?}",
        found.body,
        found.ink,
        found.ground
    );
    assert!(contrast(found.body, found.ink) >= LEGIBLE);
}

#[test]
fn a_pointer_stays_visible_on_a_skin_that_is_all_one_dark_colour() {
    let found = palette(&sheet(&[[12, 12, 16, 255], [18, 18, 22, 255]]));
    assert!(reads(&found) >= LEGIBLE);
    assert!(contrast(found.body, found.ink) >= LEGIBLE);
}

#[test]
fn a_pointer_reads_on_every_sheet_not_just_the_busiest_one() {
    let mut main = RgbaImage::from_pixel(20, 10, Rgba([54, 92, 94, 255]));
    for y in 0..10 {
        main.put_pixel(0, y, Rgba([136, 123, 53, 255]));
    }
    let playlist = RgbaImage::from_pixel(20, 10, Rgba([10, 10, 12, 255]));
    let images = BTreeMap::from([
        ("main.bmp".to_string(), main),
        ("pledit.bmp".to_string(), playlist),
    ]);
    let found = palette(&images);
    for ground in [[54, 92, 94, 255], [10, 10, 12, 255]] {
        assert!(
            contrast(ground, found.body).max(contrast(ground, found.ink)) >= LEGIBLE,
            "the pointer vanishes on {ground:?}"
        );
    }
}

#[test]
fn every_region_draws_something_at_the_size_a_skin_uses() {
    for style in Style::ALL {
        for (role, _) in SkinCursor::files() {
            let (image, hotspot) = draw(role, &Palette::default(), style);
            assert_eq!(image.dimensions(), (CURSOR_SIDE, CURSOR_SIDE));
            assert!(
                image.pixels().filter(|p| p.0[3] > 0).count() > 40,
                "{role:?} in {} drew almost nothing",
                style.name()
            );
            assert!(hotspot[0] < CURSOR_SIDE && hotspot[1] < CURSOR_SIDE);
        }
    }
}

#[test]
fn a_pointer_points_at_its_own_tip_and_a_slider_at_its_middle() {
    assert_eq!(Shape::Arrow.hotspot(), [0, 0]);
    assert_eq!(Shape::SlideX.hotspot(), [16, 16]);
}

#[test]
fn drawing_the_same_region_twice_gives_the_same_pointer() {
    let first = draw(
        SkinCursor::PositionBar,
        &Palette::default(),
        Style::default(),
    );
    let second = draw(
        SkinCursor::PositionBar,
        &Palette::default(),
        Style::default(),
    );
    assert_eq!(first.0.as_raw(), second.0.as_raw());
    assert_eq!(first.1, second.1);
}

/// A title bar drags its window in both directions at once. A one-axis
/// slider says it only moves sideways, which is the wrong promise.
#[test]
fn a_title_bar_is_not_drawn_as_a_sideways_slider() {
    for title in [
        SkinCursor::MainTitleBar,
        SkinCursor::EqualizerTitleBar,
        SkinCursor::PlaylistTitleBar,
    ] {
        assert_eq!(Shape::of(title), Shape::Move, "{title:?}");
    }
    assert_eq!(Shape::of(SkinCursor::VolumeBalance), Shape::SlideX);
}

#[test]
fn regions_that_do_different_things_do_not_share_a_shape() {
    assert_ne!(
        Shape::of(SkinCursor::MainWindow),
        Shape::of(SkinCursor::PositionBar)
    );
    assert_eq!(
        Shape::of(SkinCursor::PositionBar),
        Shape::SlideX,
        "seeking drags the thumb sideways"
    );
    assert_ne!(
        Shape::of(SkinCursor::EqualizerSlider),
        Shape::of(SkinCursor::VolumeBalance)
    );
    assert_eq!(Shape::of(SkinCursor::PlaylistResize), Shape::Resize);
    for close in [
        SkinCursor::MainClose,
        SkinCursor::EqualizerClose,
        SkinCursor::PlaylistClose,
    ] {
        assert_eq!(
            Shape::of(close),
            Shape::Danger,
            "Winamp falls a close button back to IDC_DANGER, not the plain arrow: {close:?}"
        );
    }
    assert_ne!(
        Shape::of(SkinCursor::MainClose),
        Shape::of(SkinCursor::MainMinimize),
        "closing a window is not the same as minimising it"
    );
}

#[test]
fn no_two_styles_cut_the_same_arrow() {
    let mut seen: Vec<(String, Vec<u8>)> = Vec::new();
    for style in Style::ALL {
        let (image, _) = draw(SkinCursor::MainWindow, &Palette::default(), style);
        let raw = image.as_raw().clone();
        if let Some((other, _)) = seen.iter().find(|(_, drawn)| *drawn == raw) {
            panic!("{} and {other} cut the same arrow", style.name());
        }
        seen.push((style.name().to_string(), raw));
    }
}

#[test]
fn every_style_answers_to_its_own_name() {
    for style in Style::ALL {
        assert_eq!(Style::named(style.name()), Some(style));
    }
    assert_eq!(Style::named("curly"), None);
}

#[test]
fn a_palette_picks_one_style_and_keeps_picking_it() {
    let first = palette(&sheet(&[[20, 20, 30, 255], [240, 240, 230, 255]]));
    assert_eq!(Style::for_palette(&first), Style::for_palette(&first));
}

#[test]
fn skins_that_look_different_are_cut_differently() {
    let dark = palette(&sheet(&[[11, 17, 32, 255], [201, 194, 172, 255]]));
    let warm = palette(&sheet(&[[60, 30, 10, 255], [250, 210, 120, 255]]));
    assert_ne!(
        draw(SkinCursor::MainWindow, &dark, Style::for_palette(&dark))
            .0
            .as_raw(),
        draw(SkinCursor::MainWindow, &warm, Style::for_palette(&warm))
            .0
            .as_raw()
    );
}

#[test]
fn paw_cursors_have_visible_hotspots_and_distinct_role_hints() {
    let palette = Palette {
        ink: [20, 25, 40, 255],
        body: [248, 231, 197, 255],
        accent: [204, 147, 137, 255],
        ground: [23, 30, 50, 255],
    };
    for name in ["paw", "paw-bold"] {
        let style = Style::named(name).unwrap();
        let mut pictures = Vec::new();
        for role in [
            SkinCursor::MainWindow,
            SkinCursor::MainTitleBar,
            SkinCursor::PositionBar,
            SkinCursor::EqualizerSlider,
            SkinCursor::PlaylistResize,
            SkinCursor::MainClose,
        ] {
            let (image, hotspot) = draw(role, &palette, style);
            assert_eq!(image.get_pixel(hotspot[0], hotspot[1]).0[3], 255);
            assert!(image.pixels().any(|p| p.0 == palette.accent));
            assert!(image.pixels().any(|p| p.0[3] == 0));
            assert!(
                !pictures.contains(image.as_raw()),
                "{name} duplicated role {role:?}"
            );
            pictures.push(image.as_raw().clone());
        }
    }
}

#[test]
fn paw_toe_beans_remain_visible_when_theme_accent_matches_fur() {
    let palette = Palette::default();
    let (image, _) = draw(
        SkinCursor::MainWindow,
        &palette,
        Style::named("paw").unwrap(),
    );
    assert!(image
        .pixels()
        .any(|p| p.0[3] == 255 && p.0 != palette.body && p.0 != palette.ink));
}
