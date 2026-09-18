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
/// Classic skins hand out the same pointer nearly everywhere: a title bar you
/// drag, a slider you pull and a list you scroll all keep the plain arrow, in
/// Winamp exactly as on the desktop around it. Directional arrows mean resize
/// and nothing else, so only the corner that resizes gets one. A cursor that
/// changes shape over a control is making a promise about what the control
/// does, and a promise the region cannot keep is worse than no cursor at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// The plain pointer, which is what almost every region gets.
    Arrow,
    /// A corner-to-corner bar, for the one corner that resizes.
    Resize,
}

impl Shape {
    /// The shape that suits `role`.
    pub fn of(role: SkinCursor) -> Self {
        match role {
            SkinCursor::PlaylistResize => Self::Resize,
            _ => Self::Arrow,
        }
    }

    /// Where the pointer actually points.
    pub fn hotspot(self) -> [u32; 2] {
        match self {
            Self::Arrow => [0, 0],
            Self::Resize => [CURSOR_SIDE / 2, CURSOR_SIDE / 2],
        }
    }
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
    pub const ALL: [Self; 8] = [
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
        Self::ALL[(signature % Self::ALL.len() as u32) as usize]
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
        Silhouette::Chisel => ArrowCut {
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
    let mut canvas = Canvas::new();
    let (points, colour) = match shape {
        Shape::Arrow => (arrow_points(style.silhouette), palette.body),
        Shape::Resize => (resize_points(style.weight()), palette.accent),
    };
    canvas.body(&points, colour, palette.ink, style.ring());
    (canvas.image, shape.hotspot())
}

#[cfg(test)]
mod tests {
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
    fn a_pointer_points_at_its_own_tip_and_the_resize_bar_at_its_middle() {
        assert_eq!(Shape::Arrow.hotspot(), [0, 0]);
        assert_eq!(Shape::Resize.hotspot(), [16, 16]);
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

    /// Winamp, macOS and Windows all keep the plain pointer over a title bar
    /// you drag, a slider you pull and a list you scroll. A directional arrow
    /// means resize, so a region that cannot be resized must not show one.
    #[test]
    fn only_the_corner_that_resizes_shows_a_directional_pointer() {
        for (role, name) in SkinCursor::files() {
            let shape = Shape::of(role);
            if role == SkinCursor::PlaylistResize {
                assert_eq!(shape, Shape::Resize, "{name}");
            } else {
                assert_eq!(
                    shape,
                    Shape::Arrow,
                    "{name} promises something the plain pointer does not"
                );
            }
        }
    }

    /// The song title is drawn text with no handler behind it. A pointer that
    /// suggests dragging it is describing a control that is not there.
    #[test]
    fn a_region_that_does_nothing_keeps_the_plain_pointer() {
        for role in [
            SkinCursor::SongName,
            SkinCursor::MainTitleBar,
            SkinCursor::VolumeBar,
            SkinCursor::PlaylistScrollBar,
            SkinCursor::EqualizerSlider,
        ] {
            assert_eq!(Shape::of(role), Shape::Arrow, "{role:?}");
        }
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
}
