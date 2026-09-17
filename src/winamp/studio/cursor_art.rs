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
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            ink: [0, 0, 0, 255],
            body: [255, 255, 255, 255],
            accent: [255, 255, 255, 255],
        }
    }
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
    for sheet in ["main.bmp", "eqmain.bmp", "pledit.bmp", "titlebar.bmp"] {
        let Some(image) = images.get(sheet) else {
            continue;
        };
        for pixel in image.pixels() {
            if pixel.0[3] >= 128 {
                *counts.entry(pixel.0).or_default() += 1;
            }
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
    let ink = pool
        .iter()
        .copied()
        .min_by_key(|c| luminance(*c))
        .unwrap_or([0, 0, 0, 255]);
    let body = pool
        .iter()
        .copied()
        .max_by_key(|c| luminance(*c))
        .unwrap_or([255, 255, 255, 255]);
    let accent = pool
        .iter()
        .copied()
        .max_by_key(|c| (saturation(*c), luminance(*c)))
        .unwrap_or(body);
    Palette {
        ink,
        body,
        accent: if saturation(accent) < 24 {
            body
        } else {
            accent
        },
    }
}

/// The shape a region's pointer takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A plain pointer, for a window with nothing more specific under it.
    Arrow,
    /// A pointer with a raised tip, for a control that reacts to a click.
    Button,
    /// A left-right bar, for something dragged sideways.
    SlideX,
    /// An up-down bar, for something dragged up and down.
    SlideY,
    /// A crosshair, for a bar you aim at rather than nudge.
    Aim,
    /// A diagonal bar, for a corner that resizes.
    Resize,
}

impl Shape {
    /// The shape that suits `role`.
    ///
    /// A window's plain pointer, a button, a slider and a resize corner each
    /// read differently at a glance, which is the whole job of a cursor.
    pub fn of(role: SkinCursor) -> Self {
        match role {
            SkinCursor::MainWindow | SkinCursor::EqualizerWindow | SkinCursor::PlaylistWindow => {
                Self::Arrow
            }
            SkinCursor::MainMenu
            | SkinCursor::MainMinimize
            | SkinCursor::MainWindowshade
            | SkinCursor::MainClose
            | SkinCursor::EqualizerClose => Self::Button,
            SkinCursor::MainTitleBar
            | SkinCursor::EqualizerTitleBar
            | SkinCursor::PlaylistTitleBar
            | SkinCursor::SongName
            | SkinCursor::VolumeBar
            | SkinCursor::BalanceBar => Self::SlideX,
            SkinCursor::EqualizerSlider | SkinCursor::PlaylistScrollBar => Self::SlideY,
            SkinCursor::PositionBar => Self::Aim,
            SkinCursor::PlaylistResize => Self::Resize,
        }
    }

    /// Where the pointer actually points.
    pub fn hotspot(self) -> [u32; 2] {
        match self {
            Self::Arrow | Self::Button => [0, 0],
            _ => [CURSOR_SIDE / 2, CURSOR_SIDE / 2],
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

    fn body(&mut self, points: &[(i32, i32)], colour: [u8; 4], ink: [u8; 4]) {
        for (x, y) in points {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    self.put(x + dx, y + dy, ink);
                }
            }
        }
        for (x, y) in points {
            self.put(*x, *y, colour);
        }
    }
}

fn arrow_points(marked: bool) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    for y in 0..=16 {
        let right = if y <= 11 { y } else { 11 - (y - 11) * 2 };
        for x in 0..=right {
            points.push((x, y));
        }
    }
    for y in 11..=20 {
        for x in 5..=8 {
            points.push((x, y));
        }
    }
    if marked {
        for y in 14..=19 {
            for x in 14..=19 {
                if y == 14 || y == 19 || x == 14 || x == 19 {
                    points.push((x, y));
                }
            }
        }
    }
    points
}

fn slide_points(vertical: bool) -> Vec<(i32, i32)> {
    let middle = (CURSOR_SIDE / 2) as i32;
    let mut points = Vec::new();
    for i in 6..=26 {
        points.push(if vertical { (middle, i) } else { (i, middle) });
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

fn aim_points() -> Vec<(i32, i32)> {
    let middle = (CURSOR_SIDE / 2) as i32;
    let mut points = Vec::new();
    for i in 4..28 {
        points.push((i, middle));
        points.push((middle, i));
    }
    points
}

fn resize_points() -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    for i in 7..25 {
        points.push((i, i));
        points.push((i, 31 - i));
    }
    points
}

/// Draws `role`'s pointer in `palette`, with the hotspot its shape asks for.
///
/// The same role always draws the same way, so deriving a set twice over one
/// skin changes nothing, and two skins differ only where their palettes do.
pub fn draw(role: SkinCursor, palette: &Palette) -> (RgbaImage, [u32; 2]) {
    let shape = Shape::of(role);
    let mut canvas = Canvas::new();
    let (points, colour) = match shape {
        Shape::Arrow => (arrow_points(false), palette.body),
        Shape::Button => (arrow_points(true), palette.accent),
        Shape::SlideX => (slide_points(false), palette.accent),
        Shape::SlideY => (slide_points(true), palette.accent),
        Shape::Aim => (aim_points(), palette.accent),
        Shape::Resize => (resize_points(), palette.body),
    };
    canvas.body(&points, colour, palette.ink);
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

    #[test]
    fn the_palette_takes_its_ink_and_body_from_the_artwork() {
        let found = palette(&sheet(&[[20, 20, 30, 255], [240, 240, 230, 255]]));
        assert_eq!(found.ink, [20, 20, 30, 255]);
        assert_eq!(found.body, [240, 240, 230, 255]);
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
    fn a_grey_skin_keeps_its_body_colour_rather_than_inventing_one() {
        let found = palette(&sheet(&[[30, 30, 30, 255], [200, 200, 200, 255]]));
        assert_eq!(found.accent, found.body);
    }

    #[test]
    fn transparent_pixels_do_not_reach_the_palette() {
        let found = palette(&sheet(&[[9, 9, 9, 0], [40, 40, 40, 255]]));
        assert_ne!(found.ink, [9, 9, 9, 0]);
    }

    #[test]
    fn every_region_draws_something_at_the_size_a_skin_uses() {
        for (role, _) in SkinCursor::files() {
            let (image, hotspot) = draw(role, &Palette::default());
            assert_eq!(image.dimensions(), (CURSOR_SIDE, CURSOR_SIDE));
            assert!(
                image.pixels().any(|p| p.0[3] > 0),
                "{role:?} drew nothing at all"
            );
            assert!(hotspot[0] < CURSOR_SIDE && hotspot[1] < CURSOR_SIDE);
        }
    }

    #[test]
    fn a_pointer_points_at_its_own_tip_and_a_slider_at_its_middle() {
        assert_eq!(Shape::Arrow.hotspot(), [0, 0]);
        assert_eq!(Shape::SlideX.hotspot(), [16, 16]);
    }

    #[test]
    fn drawing_the_same_region_twice_gives_the_same_pointer() {
        let first = draw(SkinCursor::PositionBar, &Palette::default());
        let second = draw(SkinCursor::PositionBar, &Palette::default());
        assert_eq!(first.0.as_raw(), second.0.as_raw());
        assert_eq!(first.1, second.1);
    }

    #[test]
    fn regions_that_do_different_things_do_not_share_a_shape() {
        assert_ne!(
            Shape::of(SkinCursor::MainWindow),
            Shape::of(SkinCursor::PositionBar)
        );
        assert_ne!(
            Shape::of(SkinCursor::EqualizerSlider),
            Shape::of(SkinCursor::VolumeBar)
        );
        assert_eq!(Shape::of(SkinCursor::PlaylistResize), Shape::Resize);
    }
}
