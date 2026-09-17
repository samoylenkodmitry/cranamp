use super::cursors::{self, SkinCursors};
use anyhow::{Context, Result};
use cranpose_ui::ImageBitmap;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::io::{Cursor, Read};
#[derive(Clone, PartialEq)]
pub struct WinampSkin {
    pub main: ImageBitmap,
    pub titlebar: ImageBitmap,
    pub cbuttons: ImageBitmap,
    pub posbar: ImageBitmap,
    pub shufrep: ImageBitmap,
    pub volume: ImageBitmap,
    pub balance: ImageBitmap,
    pub playpaus: ImageBitmap,
    pub monoster: ImageBitmap,
    pub numbers: ImageBitmap,
    pub eqmain: ImageBitmap,
    pub pledit: ImageBitmap,
    pub text: ImageBitmap,
    pub display_text_color: [u8; 4],
    pub palette: SkinPalette,
    pub viscolor: VisColor,
    /// The `.cur` files the archive carried, one per region of the player.
    /// Empty for a skin that ships bitmaps only, which is most of them.
    pub cursors: SkinCursors,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkinPalette {
    pub normal: [u8; 4],
    pub current: [u8; 4],
    pub normal_bg: [u8; 4],
    pub selected_bg: [u8; 4],
    pub marquee_fg: [u8; 4],
    pub marquee_bg: [u8; 4],
}
impl Default for SkinPalette {
    fn default() -> Self {
        Self {
            normal: [255, 200, 108, 255],
            current: [255, 255, 255, 255],
            normal_bg: [0, 0, 0, 255],
            selected_bg: [0x42, 0x35, 0x1e, 255],
            marquee_fg: [255, 200, 108, 255],
            marquee_bg: [0, 0, 0, 255],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisColor(pub [[u8; 4]; 24]);
#[allow(dead_code)]
impl VisColor {
    pub fn analyzer_gradient(&self) -> &[[u8; 4]] {
        &self.0[2..18]
    }
    pub fn oscilloscope(&self) -> &[[u8; 4]] {
        &self.0[18..23]
    }
    pub fn peak(&self) -> [u8; 4] {
        self.0[23]
    }
    pub fn background(&self) -> [u8; 4] {
        self.0[0]
    }
    pub fn dots(&self) -> [u8; 4] {
        self.0[1]
    }
}
impl Default for VisColor {
    fn default() -> Self {
        let bg = [0, 0, 0, 255];
        let fg = [153, 204, 236, 255];
        let mut palette = [fg; 24];
        palette[0] = bg;
        palette[1] = bg;
        palette[4] = bg;
        palette[7] = bg;
        palette[10] = bg;
        palette[13] = bg;
        palette[16] = bg;
        Self(palette)
    }
}
pub fn load_skin(wsz_bytes: &[u8]) -> Result<WinampSkin> {
    let mut archive = zip::ZipArchive::new(Cursor::new(wsz_bytes))
        .context("failed to open winamp .wsz archive")?;
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx).context("failed to read zip entry")?;
        if file.is_dir() {
            continue;
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .with_context(|| format!("failed to read entry {}", file.name()))?;
        files.insert(normalize_name(file.name()), data);
    }
    let decode = |name: &str| -> Result<ImageBitmap> {
        let bytes = files
            .get(name)
            .with_context(|| format!("missing required skin entry: {name}"))?;
        decode_bmp(bytes).with_context(|| format!("failed to decode {name}"))
    };
    let palette = files
        .get("pledit.txt")
        .map(|bytes| parse_pledit_txt(bytes))
        .unwrap_or_default();
    let viscolor = files
        .get("viscolor.txt")
        .map(|bytes| parse_viscolor_txt(bytes))
        .unwrap_or_default();
    let text = match files.get("text.bmp") {
        Some(bytes) => decode_bmp(bytes).context("failed to decode text.bmp")?,
        None => default_text_bitmap(),
    };
    let display_text_color =
        sample_text_bitmap_color(&text).unwrap_or_else(|| default_display_text_color(viscolor));
    let cursors = cursors::load_cursors(&files);
    Ok(WinampSkin {
        main: decode("main.bmp")?,
        titlebar: decode("titlebar.bmp")?,
        cbuttons: decode("cbuttons.bmp")?,
        posbar: decode("posbar.bmp")?,
        shufrep: decode("shufrep.bmp")?,
        volume: decode("volume.bmp")?,
        balance: decode("balance.bmp")?,
        playpaus: decode("playpaus.bmp")?,
        monoster: decode("monoster.bmp")?,
        numbers: decode("numbers.bmp")?,
        eqmain: decode("eqmain.bmp")?,
        pledit: decode("pledit.bmp")?,
        text,
        display_text_color,
        palette,
        viscolor,
        cursors,
    })
}
fn default_text_bitmap() -> ImageBitmap {
    let width = 155;
    let height = 12;
    ImageBitmap::from_rgba8(width, height, vec![0; width as usize * height as usize * 4])
        .expect("default transparent text atlas should be valid")
}
fn default_display_text_color(viscolor: VisColor) -> [u8; 4] {
    viscolor
        .analyzer_gradient()
        .iter()
        .copied()
        .find(|color| color[3] > 0 && (color[0] != 0 || color[1] != 0 || color[2] != 0))
        .unwrap_or([153, 204, 236, 255])
}
fn sample_text_bitmap_color(bitmap: &ImageBitmap) -> Option<[u8; 4]> {
    let total = (bitmap.width() as usize).saturating_mul(bitmap.height() as usize);
    sample_display_ink(bitmap.pixels(), total)
}
pub(crate) fn sample_display_ink(pixels: &[u8], total_pixels: usize) -> Option<[u8; 4]> {
    let mut opaque_pixels = 0usize;
    let mut counts: HashMap<[u8; 3], usize> = HashMap::new();
    for pixel in pixels.as_chunks::<4>().0 {
        if pixel[3] < 128 {
            continue;
        }
        opaque_pixels += 1;
        let key = [
            quantize_color_channel(pixel[0]),
            quantize_color_channel(pixel[1]),
            quantize_color_channel(pixel[2]),
        ];
        *counts.entry(key).or_insert(0) += 1;
    }
    if counts.is_empty() {
        return None;
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by_key(|entry| Reverse(entry.1));
    let likely_has_opaque_background =
        total_pixels > 0 && opaque_pixels > total_pixels.saturating_mul(2) / 3 && ranked.len() > 1;
    let color = ranked
        .get(usize::from(likely_has_opaque_background))
        .or_else(|| ranked.first())?
        .0;
    Some([color[0], color[1], color[2], 255])
}
fn quantize_color_channel(channel: u8) -> u8 {
    (channel & 0xf8).saturating_add(4)
}
pub(super) fn parse_pledit_txt(bytes: &[u8]) -> SkinPalette {
    let text = decode_text(bytes);
    let mut palette = SkinPalette::default();
    let mut in_text_section = true;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with("//") {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_text_section = line.eq_ignore_ascii_case("[Text]");
            continue;
        }
        if !in_text_section {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let Some(rgba) = parse_hex_color(value) else {
            continue;
        };
        if key.eq_ignore_ascii_case("Normal") {
            palette.normal = rgba;
        } else if key.eq_ignore_ascii_case("Current") {
            palette.current = rgba;
        } else if key.eq_ignore_ascii_case("NormalBG") {
            palette.normal_bg = rgba;
        } else if key.eq_ignore_ascii_case("SelectedBG") {
            palette.selected_bg = rgba;
        } else if key.eq_ignore_ascii_case("MbFG") {
            palette.marquee_fg = rgba;
        } else if key.eq_ignore_ascii_case("MbBG") {
            palette.marquee_bg = rgba;
        }
    }
    palette
}
pub(super) fn parse_viscolor_txt(bytes: &[u8]) -> VisColor {
    let text = decode_text(bytes);
    let mut palette = VisColor::default().0;
    let mut idx = 0;
    for raw in text.lines() {
        if idx >= palette.len() {
            break;
        }
        let line = raw
            .split("//")
            .next()
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        if parts.len() < 3 {
            continue;
        }
        let Ok(r) = parts[0].parse::<u8>() else {
            continue;
        };
        let Ok(g) = parts[1].parse::<u8>() else {
            continue;
        };
        let Ok(b) = parts[2].parse::<u8>() else {
            continue;
        };
        palette[idx] = [r, g, b, 255];
        idx += 1;
    }
    VisColor(palette)
}
fn parse_hex_color(value: &str) -> Option<[u8; 4]> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some([r, g, b, 255])
}
fn decode_text(bytes: &[u8]) -> String {
    let stripped = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(stripped).into_owned()
}
fn normalize_name(name: &str) -> String {
    name.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(name)
        .trim()
        .to_ascii_lowercase()
}
fn decode_bmp(bytes: &[u8]) -> Result<ImageBitmap> {
    let dynamic = image::load_from_memory(bytes).context("image decode")?;
    let rgba = dynamic.to_rgba8();
    ImageBitmap::from_rgba8(rgba.width(), rgba.height(), rgba.into_raw())
        .context("failed to create image bitmap")
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::winamp::cursors::SkinCursor;
    use cranpose_ui::PointerIcon;
    use std::io::{Cursor, Read, Write};
    #[test]
    fn normalize_name_extracts_file_name() {
        assert_eq!(normalize_name("SKINS\\MAIN.BMP"), "main.bmp");
        assert_eq!(normalize_name("foo/bar/PLAYPAUS.BMP"), "playpaus.bmp");
    }
    #[test]
    fn load_bundled_skin_dimensions_match_classic_template() {
        let wsz = include_bytes!("../../assets/winamp.wsz");
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
    #[test]
    fn load_skin_allows_missing_text_bitmap() {
        let mut source =
            zip::ZipArchive::new(Cursor::new(include_bytes!("../../assets/winamp.wsz")))
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
        assert_eq!(skin.text.height(), 12);
        assert_eq!(
            skin.display_text_color,
            default_display_text_color(skin.viscolor)
        );
    }
    /// The bundled skin with extra entries written alongside it.
    fn bundled_skin_plus(extra: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut source =
            zip::ZipArchive::new(Cursor::new(include_bytes!("../../assets/winamp.wsz")))
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
            ("cursors/VOLBAR.CUR", sample_cursor()),
        ]);

        let skin = load_skin(&wsz).expect("skin with cursors should load");

        assert_eq!(skin.cursors.len(), 2);
        let volume = skin
            .cursors
            .get(SkinCursor::VolumeBar)
            .expect("VOLBAR.CUR reaches the volume slider");
        let PointerIcon::Custom(volume) = volume else {
            panic!("a skin cursor is a custom pointer icon");
        };
        assert_eq!(volume.image().width(), 2);
        assert_eq!(volume.hotspot_x(), 1);
        assert!(skin.cursors.get(SkinCursor::MainWindow).is_some());
        assert!(skin.cursors.get(SkinCursor::PositionBar).is_none());
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
    fn the_bundled_skin_ships_no_cursors_and_keeps_the_platform_arrow() {
        let skin =
            load_skin(include_bytes!("../../assets/winamp.wsz")).expect("bundled skin should load");

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
    fn load_bundled_skin_parses_pledit_palette() {
        let wsz = include_bytes!("../../assets/winamp.wsz");
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
        let wsz = include_bytes!("../../assets/winamp.wsz");
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
}
pub const CLASSIC_SHEETS: &[(&str, u32, u32)] = &[
    ("main.bmp", 275, 116),
    ("titlebar.bmp", 275, 87),
    ("cbuttons.bmp", 136, 36),
    ("shufrep.bmp", 92, 85),
    ("posbar.bmp", 307, 10),
    ("volume.bmp", 68, 433),
    ("balance.bmp", 38, 433),
    ("monoster.bmp", 56, 24),
    ("playpaus.bmp", 42, 9),
    ("numbers.bmp", 99, 13),
    ("text.bmp", 155, 18),
    ("eqmain.bmp", 275, 315),
    ("pledit.bmp", 280, 186),
];
pub const CLASSIC_EXTRAS: &[&str] = &[
    "pledit.txt",
    "viscolor.txt",
    "region.txt",
    "nums_ex.bmp",
    "eq_ex.bmp",
    "avs.bmp",
    "mb.bmp",
    "video.bmp",
    "gen.bmp",
    "genex.bmp",
    "winampmb.bmp",
];
pub type SkinEntry = (String, Option<(u32, u32)>);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Divergence {
    pub entry: String,
    pub problem: String,
    pub fix: String,
}
pub fn divergences(entries: &[SkinEntry]) -> Vec<Divergence> {
    let mut out = Vec::new();
    let named = |name: &str| entries.iter().any(|(entry, _)| entry == name);
    for (sheet, width, height) in CLASSIC_SHEETS {
        let Some((_, size)) = entries.iter().find(|(entry, _)| entry == sheet) else {
            if *sheet == "numbers.bmp" && named("nums_ex.bmp") {
                continue;
            }
            out.push(Divergence {
                entry: (*sheet).into(),
                problem: "every player reads this sheet and the skin does not carry it".into(),
                fix: format!("add {sheet} at {width}x{height}"),
            });
            continue;
        };
        if let Some((w, h)) = size {
            if w < width || h < height {
                out.push(Divergence {
                    entry: (*sheet).into(),
                    problem: format!(
                        "{w}x{h} is smaller than the sprites the format reads, so \
                         part of every state falls outside the sheet"
                    ),
                    fix: format!("grow it to at least {width}x{height}"),
                });
            }
        }
    }
    for (entry, size) in entries {
        let classic = CLASSIC_SHEETS.iter().any(|(sheet, _, _)| sheet == entry)
            || CLASSIC_EXTRAS.contains(&entry.as_str())
            || cursors::CLASSIC_CURSORS.contains(&entry.as_str());
        let prose = entry.rsplit_once('.').is_some_and(|(_, extension)| {
            matches!(extension, "txt" | "md" | "nfo" | "html" | "diz")
        });
        if classic || prose {
            continue;
        }
        out.push(Divergence {
            entry: entry.clone(),
            problem: if size.is_some() {
                "no player reads this sheet, so whatever it holds is invisible everywhere".into()
            } else {
                "no player reads this entry".into()
            },
            fix: "drop it, and say the same thing in a sheet every player reads".into(),
        });
    }
    out.sort_by(|a, b| a.entry.cmp(&b.entry));
    out
}
pub fn entries_of(wsz_bytes: &[u8]) -> Result<Vec<SkinEntry>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(wsz_bytes)).context("not a .wsz archive")?;
    let mut out = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).context("unreadable zip entry")?;
        if file.is_dir() {
            continue;
        }
        let name = normalize_name(file.name());
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let size = decode_bmp(&data)
            .ok()
            .map(|image| (image.width(), image.height()));
        out.push((name, size));
    }
    Ok(out)
}
