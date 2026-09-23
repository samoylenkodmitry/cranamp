use super::cursors::{self, SkinCursors};
use anyhow::{Context, Result};
use cranpose_ui::ImageBitmap;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::io::{Cursor, Read};
mod compatibility;
pub mod regions;
pub use compatibility::{audit_classic_archive, ClassicAudit};
/// Classic Winamp BMPs are opaque, including magenta. There is no sprite color key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitmapMode {
    Classic,
}
impl BitmapMode {
    pub fn spectrum_background(self, palette: VisColor) -> [u8; 4] {
        palette.background()
    }
}
#[derive(Clone, PartialEq)]
pub struct WinampSkin {
    pub regions: regions::Regions,
    pub bitmap_mode: BitmapMode,
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
    /// Where the rolled-up main window is drawn from: the skin's own
    /// TITLEBAR.BMP and font, or the classic skin's when the skin left the
    /// windowshade strip blank, the way Winamp filled what a skin lacked.
    pub shade_titlebar: ImageBitmap,
    pub shade_text: ImageBitmap,
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
    load_skin_with_mode(wsz_bytes, BitmapMode::Classic)
}
pub fn load_skin_with_mode(wsz_bytes: &[u8], mode: BitmapMode) -> Result<WinampSkin> {
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
        decode_bmp_with_mode(bytes, mode).with_context(|| format!("failed to decode {name}"))
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
        Some(bytes) => decode_bmp_with_mode(bytes, mode).context("failed to decode text.bmp")?,
        None => default_text_bitmap(),
    };
    let display_text_color =
        sample_text_bitmap_color(&text).unwrap_or_else(|| default_display_text_color(viscolor));
    let cursors = cursors::load_cursors(&files);
    let titlebar = decode("titlebar.bmp")?;
    let (shade_titlebar, shade_text) = if shade_strip_is_blank(&titlebar) {
        (classic_bitmap("titlebar.bmp"), classic_bitmap("text.bmp"))
    } else {
        (titlebar.clone(), text.clone())
    };
    Ok(WinampSkin {
        regions: files
            .get("region.txt")
            .map(|bytes| regions::Regions::parse(bytes))
            .transpose()?
            .unwrap_or_default(),
        bitmap_mode: mode,
        main: decode("main.bmp")?,
        titlebar,
        cbuttons: decode("cbuttons.bmp")?,
        posbar: decode("posbar.bmp")?,
        shufrep: decode("shufrep.bmp")?,
        volume: decode("volume.bmp")?,
        balance: decode("balance.bmp")?,
        playpaus: decode("playpaus.bmp")?,
        monoster: decode("monoster.bmp")?,
        numbers: files
            .get("nums_ex.bmp")
            .and_then(|bytes| decode_bmp_with_mode(bytes, mode).ok())
            .map(Ok)
            .unwrap_or_else(|| decode("numbers.bmp"))?,
        eqmain: decode("eqmain.bmp")?,
        pledit: decode("pledit.bmp")?,
        text,
        shade_titlebar,
        shade_text,
        display_text_color,
        palette,
        viscolor,
        cursors,
    })
}
fn default_text_bitmap() -> ImageBitmap {
    classic_bitmap("text.bmp")
}
/// One bitmap of the classic skin bundled with the player.
fn classic_bitmap(name: &str) -> ImageBitmap {
    let mut archive = zip::ZipArchive::new(Cursor::new(include_bytes!("../../assets/winamp.wsz")))
        .expect("bundled classic skin");
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap_or_else(|_| panic!("bundled {name}"))
        .read_to_end(&mut bytes)
        .expect("bundled bitmap bytes");
    decode_bmp(&bytes).unwrap_or_else(|_| panic!("bundled classic {name}"))
}
/// The windowshade strip in TITLEBAR.BMP, the rolled-up main window.
const SHADE_STRIP: [u32; 4] = [27, 29, 275, 14];
/// Whether a skin drew no windowshade strip: the part of TITLEBAR.BMP that
/// holds it is one flat colour, or the bitmap stops short of it.
pub(crate) fn shade_strip_is_blank(titlebar: &ImageBitmap) -> bool {
    let [x, y, width, height] = SHADE_STRIP;
    if titlebar.width() < x + width || titlebar.height() < y + height {
        return true;
    }
    let stride = titlebar.width() as usize * 4;
    let pixel = |column: u32, row: u32| {
        let start = row as usize * stride + column as usize * 4;
        &titlebar.pixels()[start..start + 3]
    };
    let first = pixel(x, y);
    (y..y + height).all(|row| (x..x + width).all(|column| pixel(column, row) == first))
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
        if pixel[3] < 128 || is_sprite_key(*pixel) {
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
    ranked.sort_by_key(|entry| (Reverse(entry.1), entry.0));
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
/// Detect the obsolete Cranamp color key for migration diagnostics only.
/// This color is ordinary opaque magenta in both the editor and the player.
pub(crate) fn is_sprite_key(pixel: [u8; 4]) -> bool {
    pixel == [255, 0, 255, 255]
}
fn decode_bmp(bytes: &[u8]) -> Result<ImageBitmap> {
    decode_bmp_with_mode(bytes, BitmapMode::Classic)
}
fn decode_bmp_with_mode(bytes: &[u8], _mode: BitmapMode) -> Result<ImageBitmap> {
    let dynamic = image::load_from_memory(bytes).context("image decode")?;
    let mut rgba = dynamic.to_rgba8();
    for pixel in rgba.pixels_mut() {
        pixel.0[3] = 255;
    }
    ImageBitmap::from_rgba8(rgba.width(), rgba.height(), rgba.into_raw())
        .context("failed to create image bitmap")
}
#[cfg(test)]
#[path = "../../test/unit/winamp/skin/tests.rs"]
mod tests;
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
    ("text.bmp", 155, 12),
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
            let artwork_only_eq = *sheet == "eqmain.bmp" && *w >= 275 && *h == 163;
            if !artwork_only_eq && (w < width || h < height) {
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
