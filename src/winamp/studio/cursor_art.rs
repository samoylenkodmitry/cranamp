//! Drawing a cursor set in a skin's own colours.
//!
//! A classic skin's pointers are hand-drawn, and a skin that ships none looks
//! unfinished the moment the mouse crosses it. The editor therefore knows how
//! to draw a complete set from the artwork already on the sheets: the shapes
//! say what a region does, and the colours come from the skin, so a derived set
//! belongs to its skin instead of looking like every other one.

use std::collections::BTreeMap;

use image::{Rgba, RgbaImage};

use crate::winamp::cursors::SkinCursor;
mod paw;

/// The side of every cursor this module draws.
///
/// Classic skins are drawn at this size and platforms scale from it, so a
/// derived set matches what a hand-drawn one would be.
pub const CURSOR_SIDE: u32 = 32;

/// The three colours a cursor set is drawn from.
///
/// `ink` outlines every shape so it stays visible over the artwork, `body`
/// fills the plain pointers, and `accent` marks the regions that do something
/// when dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// The darkest colour the skin uses often, for outlines.
    pub ink: [u8; 4],
    /// The lightest colour the skin uses often, for the pointer body.
    pub body: [u8; 4],
    /// The most colourful thing on the sheets, for the draggable regions.
    pub accent: [u8; 4],
    /// The colour the skin covers most of itself in, which a pointer has to
    /// stay visible against.
    pub ground: [u8; 4],
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            ink: [0, 0, 0, 255],
            body: [255, 255, 255, 255],
            accent: [255, 255, 255, 255],
            ground: [255, 255, 255, 255],
        }
    }
}

const WHITE: [u8; 4] = [255, 255, 255, 255];

const BLACK: [u8; 4] = [0, 0, 0, 255];

const LEGIBLE: f64 = 3.0;

fn relative_luminance(colour: [u8; 4]) -> f64 {
    let channel = |value: u8| {
        let part = f64::from(value) / 255.0;
        if part <= 0.03928 {
            part / 12.92
        } else {
            ((part + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(colour[0]) + 0.7152 * channel(colour[1]) + 0.0722 * channel(colour[2])
}

/// How far apart two colours are to the eye, as the WCAG contrast ratio.
///
/// A pointer that does not clear [`LEGIBLE`] against what it floats over is
/// invisible half the time it is needed, so the palette replaces it rather than
/// drawing it.
pub fn contrast(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (first, second) = (relative_luminance(a), relative_luminance(b));
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn reads_everywhere(grounds: &[[u8; 4]], colour: [u8; 4], ink: [u8; 4]) -> bool {
    grounds
        .iter()
        .all(|ground| contrast(*ground, colour).max(contrast(*ground, ink)) >= LEGIBLE)
}

fn luminance(colour: [u8; 4]) -> u32 {
    u32::from(colour[0]) * 299 + u32::from(colour[1]) * 587 + u32::from(colour[2]) * 114
}

fn saturation(colour: [u8; 4]) -> u32 {
    let high = colour[0].max(colour[1]).max(colour[2]);
    let low = colour[0].min(colour[1]).min(colour[2]);
    u32::from(high - low)
}

/// Reads the colours a cursor set should be drawn in off a skin's sheets.
///
/// Only the sheets a player always shows are sampled, and only pixels the skin
/// actually paints, so the pointer picks up the window's own palette rather
/// than whatever sits in an unused corner of an atlas. A skin with nothing
/// drawn yet falls back to black on white, which is legible on anything.
pub fn palette(images: &BTreeMap<String, RgbaImage>) -> Palette {
    let mut counts: BTreeMap<[u8; 4], u32> = BTreeMap::new();
    let mut sheet_grounds: Vec<[u8; 4]> = Vec::new();
    for sheet in ["main.bmp", "eqmain.bmp", "pledit.bmp", "titlebar.bmp"] {
        let Some(image) = images.get(sheet) else {
            continue;
        };
        let mut own: BTreeMap<[u8; 4], u32> = BTreeMap::new();
        for pixel in image.pixels() {
            if pixel.0[3] >= 128 {
                *counts.entry(pixel.0).or_default() += 1;
                *own.entry(pixel.0).or_default() += 1;
            }
        }
        if let Some((colour, _)) = own.iter().max_by_key(|(_, count)| **count) {
            sheet_grounds.push(*colour);
        }
    }
    let total: u32 = counts.values().sum();
    if total == 0 {
        return Palette::default();
    }
    let common: Vec<[u8; 4]> = counts
        .iter()
        .filter(|(_, count)| **count * 400 >= total)
        .map(|(colour, _)| *colour)
        .collect();
    let pool = if common.is_empty() {
        counts.keys().copied().collect()
    } else {
        common
    };
    let ground = counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(colour, _)| *colour)
        .unwrap_or(WHITE);
    let mut grounds = vec![ground];
    grounds.extend(sheet_grounds);
    grounds.sort();
    grounds.dedup();

    let darkest = pool
        .iter()
        .copied()
        .min_by_key(|c| luminance(*c))
        .unwrap_or(BLACK);
    let lightest = pool
        .iter()
        .copied()
        .max_by_key(|c| luminance(*c))
        .unwrap_or(WHITE);
    let colourful = pool
        .iter()
        .copied()
        .max_by_key(|c| (saturation(*c), luminance(*c)))
        .unwrap_or(lightest);

    let (body, ink) = if contrast(lightest, darkest) >= LEGIBLE
        && reads_everywhere(&grounds, lightest, darkest)
    {
        (lightest, darkest)
    } else {
        (WHITE, BLACK)
    };
    let accent = if saturation(colourful) >= 24 && reads_everywhere(&grounds, colourful, ink) {
        colourful
    } else {
        body
    };
    Palette {
        ink,
        body,
        accent,
        ground,
    }
}

/// The shape a region's pointer takes.
///
/// The shape says what the region does; two regions that behave differently
/// never share one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A plain pointer, for a window with nothing more specific under it.
    Arrow,
    /// A left-right bar, for something dragged sideways.
    SlideX,
    /// A four-way arrow, for a bar that drags its whole window about.
    Move,
    /// An up-down bar, for something dragged up and down.
    SlideY,
    /// A corner-to-corner bar, for a corner that resizes.
    Resize,
    /// A pointer marked with a cross, for a button that closes a window.
    /// Winamp falls these back to `IDC_DANGER` rather than the plain arrow.
    Danger,
}

impl Shape {
    /// The shape that suits `role`.
    pub fn of(role: SkinCursor) -> Self {
        match role {
            SkinCursor::MainWindow
            | SkinCursor::EqualizerWindow
            | SkinCursor::PlaylistWindow
            | SkinCursor::MainMenu
            | SkinCursor::MainMinimize
            | SkinCursor::MainWindowshade => Self::Arrow,
            SkinCursor::MainClose | SkinCursor::EqualizerClose | SkinCursor::PlaylistClose => {
                Self::Danger
            }
            SkinCursor::MainTitleBar
            | SkinCursor::EqualizerTitleBar
            | SkinCursor::PlaylistTitleBar => Self::Move,
            SkinCursor::SongName | SkinCursor::VolumeBalance | SkinCursor::PositionBar => {
                Self::SlideX
            }
            SkinCursor::EqualizerSlider | SkinCursor::PlaylistScrollBar => Self::SlideY,
            SkinCursor::PlaylistResize => Self::Resize,
        }
    }

    /// Where the pointer actually points. The ones drawn as a pointer point
    /// at their own tip; the bars point at their middle.
    pub fn hotspot(self) -> [u32; 2] {
        match self {
            Self::Arrow | Self::Danger => [0, 0],
            _ => [CURSOR_SIDE / 2, CURSOR_SIDE / 2],
        }
    }

    /// The points this shape covers and the colour it is filled with.
    fn stroke(self, palette: &Palette, style: Style) -> (Vec<(i32, i32)>, [u8; 4]) {
        match self {
            Self::Arrow => (arrow_points(style.silhouette), palette.body),
            Self::SlideX => (slide_points(false, style.weight()), palette.accent),
            Self::Move => (move_points(style.weight()), palette.accent),
            Self::SlideY => (slide_points(true, style.weight()), palette.accent),
            Self::Resize => (resize_points(style.weight()), palette.body),
            Self::Danger => (
                danger_points(style.silhouette, style.weight()),
                palette.accent,
            ),
        }
    }
}

/// A pointer with a cross beside it: the shape a close button takes.
fn danger_points(silhouette: Silhouette, weight: i32) -> Vec<(i32, i32)> {
    let mut points = arrow_points(silhouette);
    for step in 0..9 {
        for offset in -weight..=weight {
            points.push((16 + step + offset, 16 + step));
            points.push((24 - step + offset, 16 + step));
        }
    }
    points
}

/// The silhouette a skin's pointers are cut to.
///
/// The shape says what a region does; the silhouette says whose skin it is, so
/// two skins never hand out the same pointer even where their colours agree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Silhouette {
    /// A thin classic arrow.
    Sharp,
    /// A wide arrow with a blunt tail.
    Chisel,
    /// A long slender arrow.
    Needle,
    /// A chunky arrow with stepped edges.
    Block,
    /// A cat paw with toe beans and role-specific directional marks.
    Paw,
}

/// How a skin draws its whole cursor set: a silhouette, and whether the
/// outline is drawn heavy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// The arrow this skin cuts.
    pub silhouette: Silhouette,
    /// A two-pixel outline rather than one, for artwork that is busy enough to
    /// swallow a thin one.
    pub bold: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            silhouette: Silhouette::Sharp,
            bold: false,
        }
    }
}

impl Style {
    /// Every style, in the order the editor offers them.
    pub const ALL: [Self; 10] = [
        Self {
            silhouette: Silhouette::Sharp,
            bold: false,
        },
        Self {
            silhouette: Silhouette::Chisel,
            bold: false,
        },
        Self {
            silhouette: Silhouette::Needle,
            bold: false,
        },
        Self {
            silhouette: Silhouette::Block,
            bold: false,
        },
        Self {
            silhouette: Silhouette::Sharp,
            bold: true,
        },
        Self {
            silhouette: Silhouette::Chisel,
            bold: true,
        },
        Self {
            silhouette: Silhouette::Needle,
            bold: true,
        },
        Self {
            silhouette: Silhouette::Block,
            bold: true,
        },
        Self {
            silhouette: Silhouette::Paw,
            bold: false,
        },
        Self {
            silhouette: Silhouette::Paw,
            bold: true,
        },
    ];

    /// The name this style answers to.
    pub fn name(self) -> &'static str {
        match (self.silhouette, self.bold) {
            (Silhouette::Sharp, false) => "sharp",
            (Silhouette::Chisel, false) => "chisel",
            (Silhouette::Needle, false) => "needle",
            (Silhouette::Block, false) => "block",
            (Silhouette::Sharp, true) => "sharp-bold",
            (Silhouette::Chisel, true) => "chisel-bold",
            (Silhouette::Needle, true) => "needle-bold",
            (Silhouette::Block, true) => "block-bold",
            (Silhouette::Paw, false) => "paw",
            (Silhouette::Paw, true) => "paw-bold",
        }
    }

    /// The style of that name, if there is one.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.name() == name)
    }

    /// The style this palette falls to when the author names none.
    ///
    /// It is picked from the colours themselves, so two skins that look
    /// different get pointers cut differently without anyone choosing, and one
    /// skin picks the same style every time it is drawn.
    pub fn for_palette(palette: &Palette) -> Self {
        let signature: u32 = [palette.ink, palette.body, palette.accent, palette.ground]
            .iter()
            .flat_map(|colour| colour.iter().take(3))
            .enumerate()
            .map(|(index, channel)| u32::from(*channel) * (index as u32 * 7 + 13))
            .sum();
        // Explicit themes are opt-in; keep the established automatic arrow selection.
        Self::ALL[(signature % 8) as usize]
    }

    fn ring(self) -> i32 {
        if self.bold {
            2
        } else {
            1
        }
    }

    fn weight(self) -> i32 {
        if self.bold {
            1
        } else {
            0
        }
    }
}

struct Canvas {
    image: RgbaImage,
}

impl Canvas {
    fn new() -> Self {
        Self {
            image: RgbaImage::from_pixel(CURSOR_SIDE, CURSOR_SIDE, Rgba([0, 0, 0, 0])),
        }
    }

    fn put(&mut self, x: i32, y: i32, colour: [u8; 4]) {
        if (0..CURSOR_SIDE as i32).contains(&x) && (0..CURSOR_SIDE as i32).contains(&y) {
            self.image.put_pixel(x as u32, y as u32, Rgba(colour));
        }
    }

    fn body(&mut self, points: &[(i32, i32)], colour: [u8; 4], ink: [u8; 4], ring: i32) {
        for (x, y) in points {
            for dx in -ring..=ring {
                for dy in -ring..=ring {
                    self.put(x + dx, y + dy, ink);
                }
            }
        }
        for (x, y) in points {
            self.put(*x, *y, colour);
        }
    }
}

struct ArrowCut {
    head: i32,
    tail: (i32, i32),
    tail_end: i32,
    edge: fn(i32) -> i32,
}

fn arrow_cut(silhouette: Silhouette) -> ArrowCut {
    match silhouette {
        Silhouette::Sharp => ArrowCut {
            head: 16,
            tail: (5, 8),
            tail_end: 20,
            edge: |y| if y <= 11 { y } else { 11 - (y - 11) * 2 },
        },
        Silhouette::Chisel | Silhouette::Paw => ArrowCut {
            head: 18,
            tail: (5, 10),
            tail_end: 22,
            edge: |y| {
                if y <= 13 {
                    y * 3 / 2
                } else {
                    19 - (y - 13) * 3
                }
            },
        },
        Silhouette::Needle => ArrowCut {
            head: 22,
            tail: (3, 5),
            tail_end: 26,
            edge: |y| {
                if y <= 18 {
                    y / 2
                } else {
                    9 - (y - 18) * 2
                }
            },
        },
        Silhouette::Block => ArrowCut {
            head: 16,
            tail: (4, 9),
            tail_end: 22,
            edge: |y| {
                if y <= 12 {
                    (y / 2) * 2
                } else {
                    12 - (y - 12) * 3
                }
            },
        },
    }
}

fn arrow_points(silhouette: Silhouette) -> Vec<(i32, i32)> {
    let cut = arrow_cut(silhouette);
    let mut points = Vec::new();
    for y in 0..=cut.head {
        for x in 0..=(cut.edge)(y).max(0) {
            points.push((x, y));
        }
    }
    for y in (cut.head - 5).max(0)..=cut.tail_end {
        for x in cut.tail.0..=cut.tail.1 {
            points.push((x, y));
        }
    }
    points
}

fn slide_points(vertical: bool, weight: i32) -> Vec<(i32, i32)> {
    let middle = (CURSOR_SIDE / 2) as i32;
    let mut points = Vec::new();
    for i in 6..=26 {
        for offset in -weight..=weight {
            points.push(if vertical {
                (middle + offset, i)
            } else {
                (i, middle + offset)
            });
        }
    }
    for step in 0..6 {
        for (end, direction) in [(6, 1), (26, -1)] {
            let along = end + direction * step;
            if vertical {
                points.push((middle - step, along));
                points.push((middle + step, along));
            } else {
                points.push((along, middle - step));
                points.push((along, middle + step));
            }
        }
    }
    points
}

fn move_points(weight: i32) -> Vec<(i32, i32)> {
    let mut points = slide_points(false, weight);
    points.extend(slide_points(true, weight));
    points
}

fn resize_points(weight: i32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    for i in 7..=24 {
        for offset in -weight..=weight {
            points.push((i + offset, i));
        }
    }
    for step in 0..6 {
        points.push((7 + step, 7));
        points.push((7, 7 + step));
        points.push((24 - step, 24));
        points.push((24, 24 - step));
    }
    points
}

/// Draws `role`'s pointer in `palette` and `style`, with the hotspot its shape
/// asks for.
///
/// The same role, palette and style always draw the same way, so deriving a set
/// twice over one skin changes nothing, and two skins differ wherever either
/// their palette or their style does.
pub fn draw(role: SkinCursor, palette: &Palette, style: Style) -> (RgbaImage, [u32; 2]) {
    let shape = Shape::of(role);
    if style.silhouette == Silhouette::Paw {
        return paw::draw(shape, palette, style.bold);
    }
    let mut canvas = Canvas::new();
    let (points, colour) = shape.stroke(palette, style);
    canvas.body(&points, colour, palette.ink, style.ring());
    (canvas.image, shape.hotspot())
}

#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/cursor_art/tests.rs"]
mod tests;
