//! Cursors from a classic Winamp skin.
//!
//! A `.wsz` may carry a Windows `.cur` file per region of the player — the
//! title bar, the volume slider, the playlist scroll bar and so on. Winamp
//! swaps the pointer as it crosses those regions, and a skin that draws a
//! knife for a cursor is as much a part of the skin as its bitmaps.
//!
//! This module reads those files and hands them to the framework as
//! [`PointerIcon`]s. A skin with no cursor for a region simply keeps the
//! platform arrow, which is what Winamp does too.

use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use cranpose_ui::{CustomPointerIcon, ImageBitmap, PointerIcon};

/// The parts of the player a classic skin can name a cursor for.
///
/// Winamp's set also covers windowshade mode and the playlist's own title-bar
/// buttons, neither of which Cranamp draws; those files stay in the archive
/// unread until there is a region to put them on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkinCursor {
    /// `NORMAL.CUR` — anywhere on the main window with nothing more specific.
    MainWindow,
    /// `TITLEBAR.CUR` — the main window's title bar.
    MainTitleBar,
    /// `MAINMENU.CUR` — the options button that opens the main menu.
    MainMenu,
    /// `MIN.CUR` — the main window's minimize button.
    MainMinimize,
    /// `WINBUT.CUR` — the main window's windowshade button.
    MainWindowshade,
    /// `CLOSE.CUR` — the main window's close button.
    MainClose,
    /// `SONGNAME.CUR` — the scrolling track title.
    SongName,
    /// `POSBAR.CUR` — the seek bar.
    PositionBar,
    /// `VOLBAL.CUR` — the volume and balance sliders, which Winamp treats as
    /// one region rather than two. `VOLBAR.CUR` is carried in skins but no
    /// player reads it.
    VolumeBalance,
    /// `EQNORMAL.CUR` — anywhere on the equalizer with nothing more specific.
    EqualizerWindow,
    /// `EQTITLE.CUR` — the equalizer's title bar.
    EqualizerTitleBar,
    /// `EQCLOSE.CUR` — the equalizer's close button.
    EqualizerClose,
    /// `EQSLID.CUR` — the preamp and band sliders.
    EqualizerSlider,
    /// `PNORMAL.CUR` — anywhere on the playlist with nothing more specific.
    PlaylistWindow,
    /// `PTBAR.CUR` — the playlist's title bar.
    PlaylistTitleBar,
    /// `PCLOSE.CUR` — the playlist's close button.
    PlaylistClose,
    /// `PVSCROLL.CUR` — the playlist's scroll bar.
    PlaylistScrollBar,
    /// `PSIZE.CUR` — the playlist's resize corner.
    PlaylistResize,
}

impl SkinCursor {
    /// Every region, paired with the archive entry it reads.
    ///
    /// The names are the ones Winamp has used since 2.x; the lookup is by
    /// lower-cased file name, so an archive that shouts `NORMAL.CUR` and one
    /// that whispers `cursors/normal.cur` both land here.
    const FILES: [(Self, &'static str); 18] = [
        (Self::MainWindow, "normal.cur"),
        (Self::MainTitleBar, "titlebar.cur"),
        (Self::MainMenu, "mainmenu.cur"),
        (Self::MainMinimize, "min.cur"),
        (Self::MainWindowshade, "winbut.cur"),
        (Self::MainClose, "close.cur"),
        (Self::SongName, "songname.cur"),
        (Self::PositionBar, "posbar.cur"),
        (Self::VolumeBalance, "volbal.cur"),
        (Self::EqualizerWindow, "eqnormal.cur"),
        (Self::EqualizerTitleBar, "eqtitle.cur"),
        (Self::EqualizerClose, "eqclose.cur"),
        (Self::EqualizerSlider, "eqslid.cur"),
        (Self::PlaylistWindow, "pnormal.cur"),
        (Self::PlaylistTitleBar, "ptbar.cur"),
        (Self::PlaylistClose, "pclose.cur"),
        (Self::PlaylistScrollBar, "pvscroll.cur"),
        (Self::PlaylistResize, "psize.cur"),
    ];

    /// How many regions a skin can name a cursor for.
    pub const COUNT: usize = Self::FILES.len();

    /// Every region paired with its archive entry, in the order an editor
    /// should lay them out: the main window, then the equalizer, then the
    /// playlist, each from its most general region to its most specific.
    pub const fn files() -> [(Self, &'static str); Self::COUNT] {
        Self::FILES
    }

    /// The archive entry this region reads, lower-cased.
    pub fn file_name(self) -> &'static str {
        Self::FILES
            .iter()
            .find_map(|(cursor, name)| (*cursor == self).then_some(*name))
            .expect("every region names a file")
    }
}

/// Every cursor file name a classic player reads.
///
/// This is wider than [`SkinCursor`]: Winamp also swaps the pointer in
/// windowshade mode and over the playlist's own title-bar buttons, which
/// Cranamp does not draw. A skin carrying one of those files is still carrying
/// something a player uses, so the portability check must not offer to drop it.
pub const CLASSIC_CURSORS: [&str; 28] = [
    "close.cur",
    "eqclose.cur",
    "eqnormal.cur",
    "eqslid.cur",
    "eqtitle.cur",
    "mainmenu.cur",
    "min.cur",
    "mmenu.cur",
    "normal.cur",
    "pclose.cur",
    "pnormal.cur",
    "posbar.cur",
    "psize.cur",
    "ptbar.cur",
    "pvscroll.cur",
    "pwinbut.cur",
    "pwsnorm.cur",
    "pwssize.cur",
    "songname.cur",
    "titlebar.cur",
    "volbal.cur",
    "volbar.cur",
    "winbut.cur",
    "wsclose.cur",
    "wsmin.cur",
    "wsnormal.cur",
    "wsposbar.cur",
    "wswinbut.cur",
];

/// The cursors one skin carries, by region.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinCursors {
    icons: HashMap<SkinCursor, PointerIcon>,
}

impl SkinCursors {
    /// The cursor for `region`, or `None` when the skin names none and the
    /// platform's own pointer should stay.
    pub fn get(&self, region: SkinCursor) -> Option<PointerIcon> {
        self.icons.get(&region).cloned()
    }

    /// Whether the skin carries no cursors at all, which is the common case:
    /// most skins ship bitmaps only.
    pub fn is_empty(&self) -> bool {
        self.icons.is_empty()
    }

    /// How many regions the skin drew a cursor for.
    pub fn len(&self) -> usize {
        self.icons.len()
    }
}

/// Reads every cursor a skin archive carries.
///
/// `files` is the archive indexed by lower-cased file name. A `.cur` that
/// cannot be decoded is logged and skipped: one malformed cursor should not
/// cost the user the skin.
pub(super) fn load_cursors(files: &HashMap<String, Vec<u8>>) -> SkinCursors {
    let mut icons = HashMap::new();
    for (region, name) in SkinCursor::FILES {
        let Some(bytes) = files.get(name) else {
            continue;
        };
        match decode_cur(bytes) {
            Ok(icon) => {
                icons.insert(region, PointerIcon::Custom(icon));
            }
            Err(error) => log::warn!("skin cursor {name} could not be decoded: {error:#}"),
        }
    }
    SkinCursors { icons }
}

/// The largest cursor a platform will draw. Classic skins draw 32x32, so this
/// only ever rejects something unusual.
const MAX_CURSOR_SIZE: u32 = 128;

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const CURSOR_RESOURCE_TYPE: u16 = 2;
const DIRECTORY_ENTRY_LEN: usize = 16;

/// Decodes a Windows `.cur` file into a pointer icon with its hotspot.
///
/// The format is an icon directory followed by one image per size; cursors add
/// the hotspot to each directory entry. The image itself is either a PNG or a
/// bottom-up DIB whose declared height covers a colour bitmap stacked on a
/// 1-bit transparency mask.
pub fn decode_cur(bytes: &[u8]) -> Result<CustomPointerIcon> {
    let entry = best_directory_entry(bytes)?;
    let image = bytes
        .get(entry.offset..entry.offset.saturating_add(entry.length))
        .context("cursor image runs past the end of the file")?;
    let bitmap = if image.starts_with(&PNG_MAGIC) {
        decode_png_image(image)?
    } else {
        decode_dib_image(image)?
    };
    CustomPointerIcon::new(bitmap, entry.hotspot_x.into(), entry.hotspot_y.into())
        .context("cursor hotspot or size is out of range")
}

/// Encodes an RGBA image as a Windows `.cur` file with its hotspot.
///
/// The output is the shape every modern cursor takes: one directory entry
/// followed by a bottom-up 32-bit DIB whose alpha channel carries the
/// transparency. The 1-bit mask the format still demands is written to agree
/// with that alpha, so a reader that ignores the channel cuts the same shape,
/// and a cursor whose every pixel is transparent survives the round trip
/// instead of coming back as an opaque rectangle.
pub fn encode_cur(
    width: u32,
    height: u32,
    rgba: &[u8],
    hotspot_x: u32,
    hotspot_y: u32,
) -> Result<Vec<u8>> {
    ensure!(
        (1..=MAX_CURSOR_SIZE).contains(&width) && (1..=MAX_CURSOR_SIZE).contains(&height),
        "a cursor is between 1 and {MAX_CURSOR_SIZE} pixels on a side, not {width}x{height}"
    );
    let expected = width as usize * height as usize * 4;
    ensure!(
        rgba.len() == expected,
        "image is {} bytes, not the {expected} a {width}x{height} RGBA image needs",
        rgba.len()
    );
    ensure!(
        hotspot_x < width && hotspot_y < height,
        "hotspot ({hotspot_x}, {hotspot_y}) sits outside a {width}x{height} cursor"
    );

    let mask_stride = row_stride(width, 1);
    let mut colors = Vec::with_capacity(expected);
    let mut mask = vec![0u8; mask_stride * height as usize];
    for (target_row, row) in (0..height).rev().enumerate() {
        for column in 0..width {
            let at = ((row * width + column) * 4) as usize;
            let pixel = &rgba[at..at + 4];
            colors.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
            if pixel[3] < 128 {
                mask[target_row * mask_stride + (column / 8) as usize] |= 1 << (7 - (column % 8));
            }
        }
    }

    let mut image = Vec::with_capacity(40 + colors.len() + mask.len());
    image.extend_from_slice(&40u32.to_le_bytes());
    image.extend_from_slice(&(width as i32).to_le_bytes());
    image.extend_from_slice(&(height as i32 * 2).to_le_bytes());
    image.extend_from_slice(&1u16.to_le_bytes());
    image.extend_from_slice(&32u16.to_le_bytes());
    image.extend_from_slice(&[0; 24]);
    image.extend_from_slice(&colors);
    image.extend_from_slice(&mask);

    let mut out = Vec::with_capacity(6 + DIRECTORY_ENTRY_LEN + image.len());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&CURSOR_RESOURCE_TYPE.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.push(u8::try_from(width).unwrap_or_default());
    out.push(u8::try_from(height).unwrap_or_default());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&(hotspot_x as u16).to_le_bytes());
    out.extend_from_slice(&(hotspot_y as u16).to_le_bytes());
    out.extend_from_slice(&(image.len() as u32).to_le_bytes());
    out.extend_from_slice(&((6 + DIRECTORY_ENTRY_LEN) as u32).to_le_bytes());
    out.extend_from_slice(&image);
    Ok(out)
}

/// One entry of the icon directory: where an image lives and where its hotspot
/// sits inside it.
struct DirectoryEntry {
    hotspot_x: u16,
    hotspot_y: u16,
    offset: usize,
    length: usize,
    pixels: u32,
}

/// The largest image in the directory that a platform will still draw.
///
/// Classic skins carry exactly one, but the format allows several sizes and
/// the biggest is the one worth showing.
fn best_directory_entry(bytes: &[u8]) -> Result<DirectoryEntry> {
    ensure!(bytes.len() >= 6, "file is too short to be a cursor");
    ensure!(
        read_u16(bytes, 0) == 0 && read_u16(bytes, 2) == CURSOR_RESOURCE_TYPE,
        "file is not a Windows cursor"
    );
    let count = usize::from(read_u16(bytes, 4));
    ensure!(count > 0, "cursor holds no images");

    let mut best: Option<DirectoryEntry> = None;
    for index in 0..count {
        let at = 6 + index * DIRECTORY_ENTRY_LEN;
        if bytes.len() < at + DIRECTORY_ENTRY_LEN {
            break;
        }
        let width = dimension(bytes[at]);
        let height = dimension(bytes[at + 1]);
        if width > MAX_CURSOR_SIZE || height > MAX_CURSOR_SIZE {
            continue;
        }
        let entry = DirectoryEntry {
            hotspot_x: read_u16(bytes, at + 4),
            hotspot_y: read_u16(bytes, at + 6),
            length: read_u32(bytes, at + 8) as usize,
            offset: read_u32(bytes, at + 12) as usize,
            pixels: width * height,
        };
        if best
            .as_ref()
            .is_none_or(|current| entry.pixels > current.pixels)
        {
            best = Some(entry);
        }
    }
    best.context("cursor holds no image a platform can draw")
}

/// A directory entry's stored dimension, where zero stands for 256.
fn dimension(stored: u8) -> u32 {
    if stored == 0 {
        256
    } else {
        u32::from(stored)
    }
}

/// Decodes a PNG-compressed directory image, which Vista-era editors write.
fn decode_png_image(image: &[u8]) -> Result<ImageBitmap> {
    let decoded = image::load_from_memory(image).context("cursor PNG image")?;
    let rgba = decoded.to_rgba8();
    ImageBitmap::from_rgba8(rgba.width(), rgba.height(), rgba.into_raw())
        .context("cursor PNG pixels")
}

/// A DIB's header fields, far enough to find its palette and pixels.
struct DibHeader {
    header_len: usize,
    width: u32,
    height: u32,
    bit_count: u16,
    palette_colors: usize,
}

/// Decodes the bottom-up DIB every classic skin cursor is written as.
fn decode_dib_image(image: &[u8]) -> Result<ImageBitmap> {
    let header = read_dib_header(image)?;
    let palette_at = header.header_len;
    let palette_len = header.palette_colors * 4;
    let pixels_at = palette_at + palette_len;
    let palette = image
        .get(palette_at..pixels_at)
        .context("cursor palette runs past the end of the image")?;

    let color_stride = row_stride(header.width, u32::from(header.bit_count));
    let mask_stride = row_stride(header.width, 1);
    let color_len = color_stride * header.height as usize;
    let colors = image
        .get(pixels_at..pixels_at + color_len)
        .context("cursor pixels run past the end of the image")?;
    let mask = image.get(pixels_at + color_len..).unwrap_or(&[]);

    let stored_alpha_is_real =
        header.bit_count == 32 && colors.iter().skip(3).step_by(4).any(|alpha| *alpha != 0);
    let mut rgba = vec![0u8; (header.width * header.height * 4) as usize];
    for y in 0..header.height {
        let source_row = (header.height - 1 - y) as usize;
        for x in 0..header.width {
            let color = read_dib_pixel(&header, palette, colors, color_stride, source_row, x)?;
            let masked = mask_bit(mask, mask_stride, source_row, x);
            let at = ((y * header.width + x) * 4) as usize;
            rgba[at] = color[0];
            rgba[at + 1] = color[1];
            rgba[at + 2] = color[2];
            rgba[at + 3] = resolve_alpha(stored_alpha_is_real, color[3], masked);
        }
    }
    ImageBitmap::from_rgba8(header.width, header.height, rgba).context("cursor pixels")
}

/// Reads and validates a `BITMAPINFOHEADER`, whose declared height counts the
/// colour bitmap and the transparency mask stacked on top of each other.
fn read_dib_header(image: &[u8]) -> Result<DibHeader> {
    ensure!(image.len() >= 40, "cursor image is too short for a DIB");
    let header_len = read_u32(image, 0) as usize;
    ensure!(
        (40..=image.len()).contains(&header_len),
        "cursor DIB header length {header_len} is out of range"
    );
    let width = read_u32(image, 4);
    let stored_height = read_u32(image, 8);
    let bit_count = read_u16(image, 14);
    let compression = read_u32(image, 16);
    ensure!(
        compression == 0,
        "cursor DIB uses unsupported compression {compression}"
    );
    ensure!(
        stored_height.is_multiple_of(2),
        "cursor DIB height {stored_height} does not stack a mask"
    );
    let height = stored_height / 2;
    ensure!(
        width > 0 && height > 0 && width <= MAX_CURSOR_SIZE && height <= MAX_CURSOR_SIZE,
        "cursor is {width}x{height}, outside the drawable range"
    );
    let declared_colors = read_u32(image, 32) as usize;
    let palette_colors = match bit_count {
        1 | 4 | 8 => {
            let maximum = 1usize << bit_count;
            if declared_colors == 0 || declared_colors > maximum {
                maximum
            } else {
                declared_colors
            }
        }
        24 | 32 => 0,
        other => bail!("cursor DIB has unsupported bit depth {other}"),
    };
    Ok(DibHeader {
        header_len,
        width,
        height,
        bit_count,
        palette_colors,
    })
}

/// One DIB pixel as RGBA, reading the palette for the indexed depths. The
/// alpha byte is meaningful only at 32 bits per pixel.
fn read_dib_pixel(
    header: &DibHeader,
    palette: &[u8],
    colors: &[u8],
    stride: usize,
    row: usize,
    x: u32,
) -> Result<[u8; 4]> {
    let row_at = row * stride;
    let index = match header.bit_count {
        1 | 4 | 8 => {
            let bits = u32::from(header.bit_count);
            let bit_at = x * bits;
            let byte = *colors
                .get(row_at + (bit_at / 8) as usize)
                .context("cursor pixel row is short")?;
            let shift = 8 - bits - (bit_at % 8);
            let mask = (1u16 << bits) - 1;
            Some(usize::from(u16::from(byte >> shift) & mask))
        }
        _ => None,
    };
    match index {
        Some(index) => {
            let at = index * 4;
            let entry = palette
                .get(at..at + 4)
                .context("cursor palette index is out of range")?;
            Ok([entry[2], entry[1], entry[0], 255])
        }
        None => {
            let bytes = usize::from(header.bit_count / 8);
            let at = row_at + x as usize * bytes;
            let pixel = colors
                .get(at..at + bytes)
                .context("cursor pixel row is short")?;
            let alpha = if bytes == 4 { pixel[3] } else { 255 };
            Ok([pixel[2], pixel[1], pixel[0], alpha])
        }
    }
}

/// Whether the 1-bit transparency mask clears this pixel. A cursor whose mask
/// is missing entirely is treated as fully opaque.
fn mask_bit(mask: &[u8], stride: usize, row: usize, x: u32) -> bool {
    let at = row * stride + (x / 8) as usize;
    mask.get(at)
        .is_some_and(|byte| byte >> (7 - (x % 8)) & 1 == 1)
}

/// The alpha a pixel ends up with.
///
/// A 32-bit cursor usually carries its own alpha. Plenty of them carry a
/// channel of zeroes instead, which would make the cursor invisible, so the
/// stored channel is trusted only when some pixel is actually opaque;
/// otherwise — and at every other depth — the 1-bit mask decides, where a set
/// bit means "leave the screen alone".
fn resolve_alpha(stored_alpha_is_real: bool, stored_alpha: u8, masked: bool) -> u8 {
    if stored_alpha_is_real {
        return stored_alpha;
    }
    if masked {
        0
    } else {
        255
    }
}

/// A DIB row's length in bytes: packed pixels rounded up to four bytes.
fn row_stride(width: u32, bits_per_pixel: u32) -> usize {
    ((width * bits_per_pixel).div_ceil(32) * 4) as usize
}

fn read_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// One image inside a cursor file, already laid out as a bottom-up DIB.
    pub(in crate::winamp) struct TestImage {
        width: u32,
        height: u32,
        hotspot: (u16, u16),
        bit_count: u16,
        palette: Vec<[u8; 4]>,
        colors: Vec<u8>,
        mask: Vec<u8>,
    }

    impl TestImage {
        fn dib(&self) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&40u32.to_le_bytes());
            out.extend_from_slice(&self.width.to_le_bytes());
            out.extend_from_slice(&(self.height * 2).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&self.bit_count.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.extend_from_slice(&(self.palette.len() as u32).to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            for entry in &self.palette {
                out.extend_from_slice(entry);
            }
            out.extend_from_slice(&self.colors);
            out.extend_from_slice(&self.mask);
            out
        }
    }

    /// Wraps images in an icon directory with the cursor resource type.
    pub(in crate::winamp) fn cursor_file(images: &[TestImage]) -> Vec<u8> {
        cursor_file_of_type(images, CURSOR_RESOURCE_TYPE)
    }

    fn cursor_file_of_type(images: &[TestImage], resource_type: u16) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&resource_type.to_le_bytes());
        header.extend_from_slice(&(images.len() as u16).to_le_bytes());

        let dibs: Vec<Vec<u8>> = images.iter().map(TestImage::dib).collect();
        let mut offset = 6 + images.len() * DIRECTORY_ENTRY_LEN;
        for (image, dib) in images.iter().zip(&dibs) {
            header.push(image.width as u8);
            header.push(image.height as u8);
            header.push(0);
            header.push(0);
            header.extend_from_slice(&image.hotspot.0.to_le_bytes());
            header.extend_from_slice(&image.hotspot.1.to_le_bytes());
            header.extend_from_slice(&(dib.len() as u32).to_le_bytes());
            header.extend_from_slice(&(offset as u32).to_le_bytes());
            offset += dib.len();
        }
        for dib in dibs {
            header.extend_from_slice(&dib);
        }
        header
    }

    /// A 2x2 two-colour cursor: white and black on the diagonal, with the
    /// top-right pixel cleared by the transparency mask.
    pub(in crate::winamp) fn monochrome_2x2() -> TestImage {
        TestImage {
            width: 2,
            height: 2,
            hotspot: (1, 0),
            bit_count: 1,
            palette: vec![[0, 0, 0, 0], [255, 255, 255, 0]],
            colors: vec![0x40, 0, 0, 0, 0x80, 0, 0, 0],
            mask: vec![0x00, 0, 0, 0, 0x40, 0, 0, 0],
        }
    }

    fn bgra_2x1(pixels: [[u8; 4]; 2], mask: u8) -> TestImage {
        let mut colors = Vec::new();
        for pixel in pixels {
            colors.extend_from_slice(&pixel);
        }
        TestImage {
            width: 2,
            height: 1,
            hotspot: (0, 0),
            bit_count: 32,
            palette: Vec::new(),
            colors,
            mask: vec![mask, 0, 0, 0],
        }
    }

    fn pixel(icon: &CustomPointerIcon, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * icon.image().width() + x) * 4) as usize;
        let pixels = icon.image().pixels();
        [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
    }

    #[test]
    fn a_monochrome_cursor_decodes_bottom_up_with_its_palette() {
        let icon = decode_cur(&cursor_file(&[monochrome_2x2()])).expect("cursor decodes");

        assert_eq!(icon.image().width(), 2);
        assert_eq!(icon.image().height(), 2);
        assert_eq!(pixel(&icon, 0, 0), [255, 255, 255, 255]);
        assert_eq!(pixel(&icon, 0, 1), [0, 0, 0, 255]);
        assert_eq!(pixel(&icon, 1, 1), [255, 255, 255, 255]);
    }

    #[test]
    fn the_transparency_mask_clears_the_pixels_it_covers() {
        let icon = decode_cur(&cursor_file(&[monochrome_2x2()])).expect("cursor decodes");

        assert_eq!(pixel(&icon, 1, 0)[3], 0, "the masked pixel is transparent");
    }

    #[test]
    fn the_hotspot_comes_from_the_directory_entry() {
        let icon = decode_cur(&cursor_file(&[monochrome_2x2()])).expect("cursor decodes");

        assert_eq!(icon.hotspot_x(), 1);
        assert_eq!(icon.hotspot_y(), 0);
    }

    #[test]
    fn a_32_bit_cursor_keeps_the_alpha_channel_it_carries() {
        let image = bgra_2x1([[10, 20, 30, 128], [40, 50, 60, 255]], 0b1100_0000);
        let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

        assert_eq!(
            pixel(&icon, 0, 0),
            [30, 20, 10, 128],
            "the stored alpha wins over a mask that would clear the pixel"
        );
        assert_eq!(pixel(&icon, 1, 0), [60, 50, 40, 255]);
    }

    #[test]
    fn a_32_bit_cursor_with_an_empty_alpha_channel_falls_back_to_the_mask() {
        let image = bgra_2x1([[10, 20, 30, 0], [40, 50, 60, 0]], 0b1000_0000);
        let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

        assert_eq!(pixel(&icon, 0, 0)[3], 0, "the mask clears the first pixel");
        assert_eq!(pixel(&icon, 1, 0), [60, 50, 40, 255]);
    }

    #[test]
    fn the_largest_image_in_the_directory_is_the_one_drawn() {
        let mut large = monochrome_2x2();
        large.width = 4;
        large.height = 4;
        large.colors = vec![0; 16];
        large.mask = vec![0; 16];
        large.hotspot = (2, 2);

        let icon = decode_cur(&cursor_file(&[monochrome_2x2(), large])).expect("cursor decodes");

        assert_eq!(icon.image().width(), 4);
        assert_eq!(icon.hotspot_x(), 2);
    }

    #[test]
    fn an_icon_file_is_not_accepted_as_a_cursor() {
        let icon_resource_type = 1;
        let error = decode_cur(&cursor_file_of_type(
            &[monochrome_2x2()],
            icon_resource_type,
        ))
        .expect_err("an icon is refused");

        assert!(
            format!("{error}").contains("not a Windows cursor"),
            "{error}"
        );
    }

    #[test]
    fn an_unsupported_bit_depth_is_refused_rather_than_guessed() {
        let mut image = monochrome_2x2();
        image.bit_count = 16;
        let error = decode_cur(&cursor_file(&[image])).expect_err("16bpp is refused");

        assert!(format!("{error}").contains("bit depth 16"), "{error}");
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_panicking() {
        let full = cursor_file(&[monochrome_2x2()]);
        for length in 0..full.len() {
            assert!(
                decode_cur(&full[..length]).is_err(),
                "a {length}-byte prefix must not decode"
            );
        }
    }

    #[test]
    fn a_cursor_larger_than_a_platform_draws_is_left_alone() {
        let mut image = monochrome_2x2();
        image.width = MAX_CURSOR_SIZE + 8;
        image.height = 2;
        image.colors = vec![0; 2 * row_stride(image.width, 1)];
        image.mask = vec![0; 2 * row_stride(image.width, 1)];

        assert!(decode_cur(&cursor_file(&[image])).is_err());
    }

    /// A 16-colour row where the two pixels pick palette entries 1 and 2.
    fn indexed_4bpp_2x1() -> TestImage {
        let mut palette = vec![[0, 0, 0, 0]; 16];
        palette[1] = [11, 22, 33, 0];
        palette[2] = [44, 55, 66, 0];
        TestImage {
            width: 2,
            height: 1,
            hotspot: (0, 0),
            bit_count: 4,
            palette,
            colors: vec![0x12, 0, 0, 0],
            mask: vec![0, 0, 0, 0],
        }
    }

    #[test]
    fn a_sixteen_colour_cursor_reads_two_pixels_from_one_byte() {
        let icon = decode_cur(&cursor_file(&[indexed_4bpp_2x1()])).expect("cursor decodes");

        assert_eq!(pixel(&icon, 0, 0), [33, 22, 11, 255]);
        assert_eq!(pixel(&icon, 1, 0), [66, 55, 44, 255]);
    }

    #[test]
    fn a_256_colour_cursor_reads_one_pixel_per_byte() {
        let mut palette = vec![[0, 0, 0, 0]; 256];
        palette[7] = [1, 2, 3, 0];
        palette[9] = [4, 5, 6, 0];
        let image = TestImage {
            width: 2,
            height: 1,
            hotspot: (0, 0),
            bit_count: 8,
            palette,
            colors: vec![7, 9, 0, 0],
            mask: vec![0, 0, 0, 0],
        };

        let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

        assert_eq!(pixel(&icon, 0, 0), [3, 2, 1, 255]);
        assert_eq!(pixel(&icon, 1, 0), [6, 5, 4, 255]);
    }

    #[test]
    fn a_24_bit_cursor_reads_three_bytes_per_pixel_over_a_padded_row() {
        let image = TestImage {
            width: 2,
            height: 1,
            hotspot: (0, 0),
            bit_count: 24,
            palette: Vec::new(),
            colors: vec![10, 20, 30, 40, 50, 60, 0, 0],
            mask: vec![0b0100_0000, 0, 0, 0],
        };

        let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

        assert_eq!(pixel(&icon, 0, 0), [30, 20, 10, 255]);
        assert_eq!(pixel(&icon, 1, 0)[3], 0, "the mask clears the second pixel");
    }

    #[test]
    fn load_cursors_reads_the_regions_the_archive_names() {
        let mut files = HashMap::new();
        files.insert(
            SkinCursor::MainWindow.file_name().to_string(),
            cursor_file(&[monochrome_2x2()]),
        );
        files.insert(
            SkinCursor::VolumeBalance.file_name().to_string(),
            cursor_file(&[monochrome_2x2()]),
        );

        let cursors = load_cursors(&files);

        assert_eq!(cursors.len(), 2);
        assert!(cursors.get(SkinCursor::MainWindow).is_some());
        assert!(cursors.get(SkinCursor::VolumeBalance).is_some());
        assert!(cursors.get(SkinCursor::PlaylistResize).is_none());
    }

    #[test]
    fn a_malformed_cursor_costs_only_its_own_region() {
        let mut files = HashMap::new();
        files.insert(
            SkinCursor::MainWindow.file_name().to_string(),
            b"not a cursor at all".to_vec(),
        );
        files.insert(
            SkinCursor::MainClose.file_name().to_string(),
            cursor_file(&[monochrome_2x2()]),
        );

        let cursors = load_cursors(&files);

        assert!(cursors.get(SkinCursor::MainWindow).is_none());
        assert!(cursors.get(SkinCursor::MainClose).is_some());
    }

    #[test]
    fn a_skin_with_no_cursor_files_carries_none() {
        assert!(load_cursors(&HashMap::new()).is_empty());
    }

    #[test]
    fn every_region_reads_a_file_a_classic_player_reads() {
        for (region, name) in SkinCursor::FILES {
            assert!(
                CLASSIC_CURSORS.contains(&name),
                "{region:?} reads {name}, which the portability check would offer to drop"
            );
        }
    }

    #[test]
    fn the_classic_cursor_names_are_sorted_and_distinct() {
        let mut sorted = CLASSIC_CURSORS;
        sorted.sort_unstable();
        assert_eq!(sorted, CLASSIC_CURSORS, "keep the list in name order");
        let mut unique = CLASSIC_CURSORS.to_vec();
        unique.dedup();
        assert_eq!(unique.len(), CLASSIC_CURSORS.len());
    }

    #[test]
    fn every_region_names_a_distinct_lower_case_file() {
        let mut names: Vec<&str> = SkinCursor::FILES.iter().map(|(_, name)| *name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();

        assert_eq!(names.len(), count, "two regions share a file name");
        assert!(
            SkinCursor::FILES
                .iter()
                .all(|(_, name)| *name == name.to_ascii_lowercase()),
            "archive lookup is by lower-cased name"
        );
    }

    fn gradient(width: u32, height: u32) -> Vec<u8> {
        (0..width * height)
            .flat_map(|i| {
                let x = (i % width) as u8;
                let y = (i / width) as u8;
                [
                    x.wrapping_mul(7),
                    y.wrapping_mul(11),
                    40,
                    if x == y { 0 } else { 255 },
                ]
            })
            .collect()
    }

    #[test]
    fn an_encoded_cursor_reads_back_pixel_for_pixel() {
        let rgba = gradient(16, 24);
        let bytes = encode_cur(16, 24, &rgba, 3, 5).expect("encodes");
        let icon = decode_cur(&bytes).expect("decodes");
        assert_eq!(icon.hotspot_x(), 3);
        assert_eq!(icon.hotspot_y(), 5);
        assert_eq!(icon.image().width(), 16);
        assert_eq!(icon.image().height(), 24);
        assert_eq!(icon.image().pixels(), rgba.as_slice());
    }

    #[test]
    fn an_encoded_cursor_keeps_its_transparent_pixels_transparent() {
        let rgba = vec![0; 32 * 32 * 4];
        let bytes = encode_cur(32, 32, &rgba, 0, 0).expect("encodes");
        let icon = decode_cur(&bytes).expect("decodes");
        assert!(
            icon.image()
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[3] == 0),
            "an all-transparent cursor must not come back opaque"
        );
    }

    #[test]
    fn an_image_of_the_wrong_length_is_refused() {
        assert!(encode_cur(8, 8, &[0; 10], 0, 0).is_err());
    }

    #[test]
    fn a_hotspot_outside_the_image_is_refused() {
        let rgba = vec![0; 8 * 8 * 4];
        assert!(encode_cur(8, 8, &rgba, 8, 0).is_err());
        assert!(encode_cur(8, 8, &rgba, 0, 8).is_err());
    }

    #[test]
    fn a_cursor_larger_than_the_format_allows_is_refused() {
        let side = MAX_CURSOR_SIZE + 1;
        let rgba = vec![0; (side * side * 4) as usize];
        assert!(encode_cur(side, side, &rgba, 0, 0).is_err());
    }
}
