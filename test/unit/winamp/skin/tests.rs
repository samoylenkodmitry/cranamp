use super::*;
use crate::winamp::cursors::SkinCursor;
use cranpose_ui::PointerIcon;
use std::io::{Cursor, Read, Write};
#[test]
fn default_bmp_loader_preserves_magenta_and_neighboring_purple_as_opaque_ink() {
    let mut bytes = Cursor::new(Vec::new());
    let image =
        image::RgbImage::from_raw(3, 1, vec![255, 0, 255, 254, 0, 255, 128, 0, 128]).unwrap();
    image.write_to(&mut bytes, image::ImageFormat::Bmp).unwrap();
    let decoded = decode_bmp(bytes.get_ref()).unwrap();
    assert_eq!(
        decoded.pixels(),
        &[255, 0, 255, 255, 254, 0, 255, 255, 128, 0, 128, 255]
    );
}
#[test]
fn classic_bitmaps_keep_magenta_opaque_including_font_backgrounds() {
    let mut bytes = Cursor::new(Vec::new());
    image::RgbImage::from_raw(2, 1, vec![255, 0, 255, 17, 29, 43])
        .unwrap()
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .unwrap();
    let decoded = decode_bmp_with_mode(bytes.get_ref(), BitmapMode::Classic).unwrap();
    assert_eq!(decoded.pixels(), &[255, 0, 255, 255, 17, 29, 43, 255]);
    let mut palette = VisColor::default();
    palette.0[0] = [255, 0, 255, 255];
    assert_eq!(
        BitmapMode::Classic.spectrum_background(palette),
        [255, 0, 255, 255]
    );
    assert_eq!(palette.background(), [255, 0, 255, 255]);
}
#[test]
fn classic_bitmap_copy_ignores_embedded_alpha() {
    let mut bytes = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(1, 1, image::Rgba([17, 29, 43, 0]))
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .unwrap();
    let bitmap = decode_bmp_with_mode(bytes.get_ref(), BitmapMode::Classic).unwrap();
    assert_eq!(bitmap.pixels(), &[17, 29, 43, 255]);
}
#[test]
fn extended_number_atlas_takes_precedence_over_numbers_bmp() {
    let mut bytes = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(108, 13, image::Rgb([17, 29, 43]))
        .write_to(&mut bytes, image::ImageFormat::Bmp)
        .unwrap();
    let skin = load_skin(&bundled_skin_plus(&[("nums_ex.bmp", bytes.into_inner())])).unwrap();
    assert_eq!((skin.numbers.width(), skin.numbers.height()), (108, 13));
    assert_eq!(&skin.numbers.pixels()[0..4], &[17, 29, 43, 255]);
}
#[test]
fn normalize_name_extracts_file_name() {
    assert_eq!(normalize_name("SKINS\\MAIN.BMP"), "main.bmp");
    assert_eq!(normalize_name("foo/bar/PLAYPAUS.BMP"), "playpaus.bmp");
}
#[test]
fn load_bundled_skin_dimensions_match_classic_template() {
    let wsz = include_bytes!("../../../../assets/winamp.wsz");
    let skin = load_skin(wsz).expect("bundled skin should load");
    assert_eq!(skin.main.width(), 275);
    assert_eq!(skin.main.height(), 116);
    assert_eq!(skin.titlebar.width(), 344);
    assert_eq!(skin.cbuttons.width(), 136);
    assert_eq!(skin.cbuttons.height(), 36);
    assert_eq!(skin.posbar.width(), 307);
    assert_eq!(skin.posbar.height(), 10);
    assert_eq!(skin.text.width(), 155);
    assert_eq!(skin.text.height(), 18);
}
/// The classic skin with its windowshade strip painted over in one colour,
/// as a skin drawn before the rolled-up window was looks.
fn classic_skin_with_a_blank_strip() -> Vec<u8> {
    let mut source =
        zip::ZipArchive::new(Cursor::new(include_bytes!("../../../../assets/winamp.wsz")))
            .expect("bundled skin should be a zip");
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for index in 0..source.len() {
            let mut file = source.by_index(index).expect("zip entry");
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data).expect("zip entry bytes");
            if normalize_name(&name) == "titlebar.bmp" {
                let mut titlebar = image::load_from_memory(&data).expect("bmp").to_rgb8();
                for y in 29..43 {
                    for x in 27..302 {
                        titlebar.put_pixel(x, y, image::Rgb([0, 0, 0]));
                    }
                }
                let mut bytes = Cursor::new(Vec::new());
                titlebar
                    .write_to(&mut bytes, image::ImageFormat::Bmp)
                    .expect("bmp encodes");
                data = bytes.into_inner();
            }
            writer.start_file(name, options).expect("zip entry");
            writer.write_all(&data).expect("zip entry bytes");
        }
        writer.finish().expect("zip should finish");
    }
    output.into_inner()
}
#[test]
fn a_skin_without_windowshade_art_rolls_up_in_the_classic_strip() {
    let classic = load_skin(include_bytes!("../../../../assets/winamp.wsz")).unwrap();
    assert!(!shade_strip_is_blank(&classic.titlebar));
    assert_eq!(classic.shade_titlebar.pixels(), classic.titlebar.pixels());
    let blank = load_skin(&classic_skin_with_a_blank_strip()).unwrap();
    assert!(shade_strip_is_blank(&blank.titlebar));
    assert_eq!(blank.shade_titlebar.pixels(), classic.titlebar.pixels());
    assert_eq!(blank.shade_text.pixels(), classic.text.pixels());
}
#[test]
fn every_bundled_skin_draws_its_own_windowshade_strip() {
    for skin in crate::winamp::BUNDLED_SKINS {
        let loaded = load_skin(skin.bytes).unwrap();
        assert!(
            !shade_strip_is_blank(&loaded.titlebar),
            "{} rolls up into the classic strip",
            skin.id
        );
        assert_eq!(loaded.shade_titlebar.pixels(), loaded.titlebar.pixels());
    }
}
#[test]
fn load_skin_allows_missing_text_bitmap() {
    let mut source =
        zip::ZipArchive::new(Cursor::new(include_bytes!("../../../../assets/winamp.wsz")))
            .expect("bundled skin should be a zip");
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for index in 0..source.len() {
            let mut file = source
                .by_index(index)
                .expect("zip entry should be readable");
            let name = file.name().to_string();
            if normalize_name(&name) == "text.bmp" {
                continue;
            }
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .expect("zip entry bytes should be readable");
            writer
                .start_file(name, options)
                .expect("zip entry should be writable");
            writer
                .write_all(&data)
                .expect("zip entry bytes should be writable");
        }
        writer.finish().expect("zip should finish");
    }
    let skin = load_skin(&output.into_inner()).expect("skin without text.bmp should load");
    assert_eq!(skin.text.width(), 155);
    assert_eq!(skin.text.height(), 18);
    let reference = load_skin(include_bytes!("../../../../assets/winamp.wsz")).unwrap();
    assert_eq!(skin.text.pixels(), reference.text.pixels());
}
/// The bundled skin with extra entries written alongside it.
fn bundled_skin_plus(extra: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut source =
        zip::ZipArchive::new(Cursor::new(include_bytes!("../../../../assets/winamp.wsz")))
            .expect("bundled skin should be a zip");
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut output);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for index in 0..source.len() {
            let mut file = source
                .by_index(index)
                .expect("zip entry should be readable");
            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .expect("zip entry bytes should be readable");
            writer
                .start_file(name, options)
                .expect("zip entry should be writable");
            writer
                .write_all(&data)
                .expect("zip entry should be written");
        }
        for (name, data) in extra {
            writer
                .start_file(*name, options)
                .expect("extra entry should be writable");
            writer
                .write_all(data)
                .expect("extra entry should be written");
        }
        writer.finish().expect("zip should finish");
    }
    output.into_inner()
}

fn sample_cursor() -> Vec<u8> {
    cursors::tests::cursor_file(&[cursors::tests::monochrome_2x2()])
}

#[test]
fn a_skin_that_ships_cursors_hands_them_to_the_regions_that_read_them() {
    let wsz = bundled_skin_plus(&[
        ("NORMAL.CUR", sample_cursor()),
        ("cursors/VOLBAL.CUR", sample_cursor()),
    ]);

    let skin = load_skin(&wsz).expect("skin with cursors should load");

    assert_eq!(skin.cursors.len(), 2);
    let volume = skin
        .cursors
        .get(SkinCursor::VolumeBalance)
        .expect("VOLBAL.CUR reaches the volume and balance sliders");
    let PointerIcon::Custom(volume) = volume else {
        panic!("a skin cursor is a custom pointer icon");
    };
    assert_eq!(volume.image().width(), 2);
    assert_eq!(volume.hotspot_x(), 1);
    assert!(skin.cursors.get(SkinCursor::MainWindow).is_some());
    assert!(skin.cursors.get(SkinCursor::PositionBar).is_none());
}

#[test]
fn volbar_is_carried_but_never_read() {
    let wsz = bundled_skin_plus(&[("VOLBAR.CUR", sample_cursor())]);

    let skin = load_skin(&wsz).expect("skin with cursors should load");

    assert!(
        skin.cursors.get(SkinCursor::VolumeBalance).is_none(),
        "Winamp's cursor table names VolBal.cur for the volume and balance \
         sliders and never loads VolBar.cur, so a skin shipping only \
         VOLBAR.CUR leaves those sliders with the window's own pointer"
    );
    assert_eq!(skin.cursors.len(), 0);
}

/// The portability check reports entries no player reads, and the Studio's
/// repair drops exactly what it reports. Cursors are read, so they have to
/// survive both.
#[test]
fn a_cursor_file_is_not_an_entry_no_player_reads() {
    let entries: Vec<SkinEntry> = cursors::CLASSIC_CURSORS
        .iter()
        .map(|name| ((*name).to_string(), None))
        .collect();

    let reported = divergences(&entries);

    for (name, _) in &entries {
        assert!(
            !reported.iter().any(|divergence| divergence.entry == *name),
            "{name} was reported as an entry no player reads"
        );
    }
}

#[test]
fn a_cursor_name_no_player_knows_is_still_reported() {
    let reported = divergences(&[("mystery.cur".to_string(), None)]);

    assert!(
        reported
            .iter()
            .any(|divergence| divergence.entry == "mystery.cur"),
        "an unknown cursor name should still be reported: {reported:?}"
    );
}

#[test]
fn the_classic_reference_skin_ships_no_cursors_and_keeps_the_platform_arrow() {
    let skin = load_skin(include_bytes!("../../../../assets/winamp.wsz"))
        .expect("bundled skin should load");

    assert!(skin.cursors.is_empty());
}

#[test]
fn a_broken_cursor_does_not_cost_the_skin() {
    let wsz = bundled_skin_plus(&[("NORMAL.CUR", b"not a cursor".to_vec())]);

    let skin = load_skin(&wsz).expect("a skin with one broken cursor still loads");

    assert!(skin.cursors.is_empty());
    assert_eq!(skin.main.width(), 275);
}

#[test]
fn sample_text_bitmap_color_uses_visible_glyph_pixels() {
    let bitmap = ImageBitmap::from_rgba8(
        2,
        2,
        vec![
            0, 0, 0, 255, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 0,
        ],
    )
    .expect("test bitmap should be valid");
    assert_eq!(sample_text_bitmap_color(&bitmap), Some([4, 4, 4, 255]));
}
#[test]
fn sample_text_bitmap_color_skips_opaque_background() {
    let bitmap = ImageBitmap::from_rgba8(
        2,
        2,
        vec![
            248, 248, 248, 255, 248, 248, 248, 255, 248, 248, 248, 255, 8, 16, 24, 255,
        ],
    )
    .expect("test bitmap should be valid");
    assert_eq!(sample_text_bitmap_color(&bitmap), Some([12, 20, 28, 255]));
}
#[test]
fn display_ink_ignores_literal_sprite_keys_in_editor_atlases() {
    assert_eq!(
        sample_display_ink(&[255, 0, 255, 255, 8, 16, 24, 255], 2),
        Some([12, 20, 28, 255])
    );
}
#[test]
fn load_bundled_skin_parses_pledit_palette() {
    let wsz = include_bytes!("../../../../assets/winamp.wsz");
    let skin = load_skin(wsz).expect("bundled skin should load");
    assert_eq!(skin.palette.normal, [0xff, 0xc8, 0x6c, 255]);
    assert_eq!(skin.palette.current, [0xff, 0xff, 0xff, 255]);
    assert_eq!(skin.palette.normal_bg, [0, 0, 0, 255]);
    assert_eq!(skin.palette.selected_bg, [0x42, 0x35, 0x1e, 255]);
    assert_eq!(skin.palette.marquee_fg, [0xff, 0xc8, 0x6c, 255]);
    assert_eq!(skin.palette.marquee_bg, [0, 0, 0, 255]);
}
#[test]
fn load_bundled_skin_parses_viscolor() {
    let wsz = include_bytes!("../../../../assets/winamp.wsz");
    let skin = load_skin(wsz).expect("bundled skin should load");
    assert_eq!(skin.viscolor.0[0], [0, 0, 0, 255]);
    assert_eq!(skin.viscolor.0[2], [153, 204, 236, 255]);
    assert_eq!(skin.viscolor.0[23], [153, 204, 236, 255]);
}
#[test]
fn parse_pledit_handles_missing_section_header_and_casing() {
    let body = b"Normal=#abcdef\r\ncurrent =  #112233 \r\nNORMALBG=#000000\r\n";
    let palette = parse_pledit_txt(body);
    assert_eq!(palette.normal, [0xab, 0xcd, 0xef, 255]);
    assert_eq!(palette.current, [0x11, 0x22, 0x33, 255]);
    assert_eq!(palette.normal_bg, [0, 0, 0, 255]);
}
#[test]
fn parse_pledit_skips_keys_outside_text_section() {
    let body = b"[Text]\nNormal=#aabbcc\n[Marquee]\nNormal=#ffffff\n";
    let palette = parse_pledit_txt(body);
    assert_eq!(palette.normal, [0xaa, 0xbb, 0xcc, 255]);
}
#[test]
fn parse_pledit_ignores_malformed_hex() {
    let body = b"[Text]\nNormal=#zz0000\nCurrent=#abcdef\n";
    let palette = parse_pledit_txt(body);
    let default = SkinPalette::default();
    assert_eq!(palette.normal, default.normal);
    assert_eq!(palette.current, [0xab, 0xcd, 0xef, 255]);
}
#[test]
fn parse_viscolor_strips_comments_and_short_lines() {
    let body = b"  10, 20, 30, // first\n40,50,60 ; second\nbroken\n70,80,90\n";
    let palette = parse_viscolor_txt(body);
    assert_eq!(palette.0[0], [10, 20, 30, 255]);
    assert_eq!(palette.0[1], [40, 50, 60, 255]);
    assert_eq!(palette.0[2], [70, 80, 90, 255]);
}
