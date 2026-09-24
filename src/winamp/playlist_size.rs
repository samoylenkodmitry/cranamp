//! The sizes classic Winamp gives its playlist: 275 by 116, and from there
//! only whole steps of 25 across and 29 down, a footer tile's width and a side
//! rail tile's height. Resizing lands on the nearest step, so every size shows
//! whole tiles and the footer's pieces fall where the skin drew them to meet.

use cranpose_ui::{Point, Size};

/// The smallest playlist, and the size every step counts from.
pub(crate) const BASE_WIDTH: f32 = 275.0;
pub(crate) const BASE_HEIGHT: f32 = 116.0;
/// One step of width: a footer tile, `PLEDIT.BMP` (179,0,25,38).
pub(crate) const STEP_WIDTH: f32 = 25.0;
/// One step of height: a side rail tile.
pub(crate) const STEP_HEIGHT: f32 = 29.0;
/// The footer's own pieces: the corner at each end, and the visualizer
/// panel Winamp puts beside the right one once the playlist is three steps
/// wider than the base.
pub(crate) const FOOTER_LEFT_WIDTH: f32 = 125.0;
pub(crate) const FOOTER_RIGHT_WIDTH: f32 = 150.0;
pub(crate) const VISUALIZER_WIDTH: f32 = 75.0;
const VISUALIZER_STEPS: f32 = 3.0;

/// `size` moved to the nearest classic size.
pub(crate) fn classic(size: Size) -> Size {
    let steps = |length: f32, base: f32, step: f32| {
        if length.is_finite() {
            ((length - base) / step).round().max(0.0)
        } else {
            0.0
        }
    };
    Size::new(
        BASE_WIDTH + steps(size.width, BASE_WIDTH, STEP_WIDTH) * STEP_WIDTH,
        BASE_HEIGHT + steps(size.height, BASE_HEIGHT, STEP_HEIGHT) * STEP_HEIGHT,
    )
}

/// The classic size a playlist held at `held` takes when its corner has
/// travelled by `travel`.
pub(crate) fn stretched(held: Size, travel: Point) -> Size {
    classic(Size::new(held.width + travel.x, held.height + travel.y))
}

/// Where the footer's visualizer panel starts in a playlist `width` wide, if
/// the playlist is wide enough to have one.
pub(crate) fn visualizer_x(width: f32) -> Option<f32> {
    (width >= BASE_WIDTH + VISUALIZER_STEPS * STEP_WIDTH)
        .then_some(width - FOOTER_RIGHT_WIDTH - VISUALIZER_WIDTH)
}

/// How far the footer's right corner, and everything drawn on it, sits right
/// of where it sits in the narrowest playlist.
pub(crate) fn right_corner_shift(width: f32) -> f32 {
    (width - BASE_WIDTH).max(0.0)
}

#[cfg(test)]
#[path = "../../test/unit/winamp/playlist_size/tests.rs"]
mod tests;
