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
/// Winamp's set also covers the playlist's own title-bar buttons and its
/// rolled-up window, which Cranamp does not draw; those files stay in the
/// archive unread until there is a region to put them on.
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
    /// `WSNORMAL.CUR` — the main window rolled up into its title bar.
    ShadeWindow,
    /// `WSMIN.CUR` — the rolled-up window's minimize button.
    ShadeMinimize,
    /// `WSWINBUT.CUR` — the rolled-up window's button that unrolls it.
    ShadeWindowshade,
    /// `WSCLOSE.CUR` — the rolled-up window's close button.
    ShadeClose,
    /// `WSPOSBAR.CUR` — the rolled-up window's small seek bar.
    ShadePositionBar,
}

impl SkinCursor {
    /// Every region, paired with the archive entry it reads.
    ///
    /// The names are the ones Winamp has used since 2.x; the lookup is by
    /// lower-cased file name, so an archive that shouts `NORMAL.CUR` and one
    /// that whispers `cursors/normal.cur` both land here.
    const FILES: [(Self, &'static str); 23] = [
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
        (Self::ShadeWindow, "wsnormal.cur"),
        (Self::ShadeMinimize, "wsmin.cur"),
        (Self::ShadeWindowshade, "wswinbut.cur"),
        (Self::ShadeClose, "wsclose.cur"),
        (Self::ShadePositionBar, "wsposbar.cur"),
    ];

    /// How many regions a skin can name a cursor for.
    pub const COUNT: usize = Self::FILES.len();

    /// Every region paired with its archive entry, in the order an editor
    /// should lay them out: the main window, then the equalizer, then the
    /// playlist, then the rolled-up main window, each from its most general
    /// region to its most specific.
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

    /// What the region is, in the words an editor shows beside its cursor.
    pub fn label(self) -> &'static str {
        match self {
            Self::MainWindow => "Main window",
            Self::MainTitleBar => "Title bar",
            Self::MainMenu => "Options button",
            Self::MainMinimize => "Minimize",
            Self::MainWindowshade => "Roll up",
            Self::MainClose => "Close",
            Self::SongName => "Song title",
            Self::PositionBar => "Seek bar",
            Self::VolumeBalance => "Volume and balance",
            Self::EqualizerWindow => "Equalizer",
            Self::EqualizerTitleBar => "EQ title bar",
            Self::EqualizerClose => "EQ close",
            Self::EqualizerSlider => "EQ sliders",
            Self::PlaylistWindow => "Playlist",
            Self::PlaylistTitleBar => "Playlist title bar",
            Self::PlaylistClose => "Playlist close",
            Self::PlaylistScrollBar => "Playlist scroll bar",
            Self::PlaylistResize => "Playlist resize",
            Self::ShadeWindow => "Rolled up",
            Self::ShadeMinimize => "Rolled-up minimize",
            Self::ShadeWindowshade => "Unroll",
            Self::ShadeClose => "Rolled-up close",
            Self::ShadePositionBar => "Rolled-up seek bar",
        }
    }

    /// The region whose cursor stands in when a skin drew none for this one.
    /// Most skins predate the rolled-up window's own set, and its parts do
    /// what the full window's do.
    pub fn stand_in(self) -> Option<Self> {
        match self {
            Self::ShadeWindow => Some(Self::MainTitleBar),
            Self::ShadeMinimize => Some(Self::MainMinimize),
            Self::ShadeWindowshade => Some(Self::MainWindowshade),
            Self::ShadeClose => Some(Self::MainClose),
            Self::ShadePositionBar => Some(Self::PositionBar),
            _ => None,
        }
    }
}

/// Every cursor file name a classic player reads.
///
/// This is wider than [`SkinCursor`]: Winamp also swaps the pointer over the
/// playlist's own title-bar buttons and its rolled-up window, which Cranamp
/// does not draw. A skin carrying one of those files is still carrying
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
    for (region, _) in SkinCursor::FILES {
        if let Some(icon) = region
            .stand_in()
            .filter(|_| !icons.contains_key(&region))
            .and_then(|stand_in| icons.get(&stand_in).cloned())
        {
            icons.insert(region, icon);
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
#[path = "../../test/unit/winamp/cursors/tests.rs"]
pub(super) mod tests;
