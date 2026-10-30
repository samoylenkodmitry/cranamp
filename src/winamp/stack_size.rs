//! Desktop stack dimensions in skin pixels. Width controls the common zoom;
//! height is the playlist's height at that zoom.
use cranpose_ui::{Point, Size};

pub const WIDTH: f32 = 275.;
pub const MIN_WIDTH: f32 = WIDTH / 1.5;
pub const MIN_PLAYLIST_HEIGHT: f32 = 116.;

pub fn valid(size: Size) -> Size {
    Size::new(
        if size.width.is_finite() {
            size.width.clamp(MIN_WIDTH, WIDTH * 4.)
        } else {
            WIDTH
        },
        if size.height.is_finite() {
            size.height.clamp(MIN_PLAYLIST_HEIGHT, 8192.)
        } else {
            261.
        },
    )
}

pub fn stretched(held: Size, travel: Point, base_scale: f32, above: f32, side_margin: f32) -> Size {
    let old_scale = (held.width * base_scale - 2. * side_margin) / WIDTH;
    let width = valid(Size::new(held.width + travel.x / base_scale, held.height)).width;
    let scale = (width * base_scale - 2. * side_margin) / WIDTH;
    let height = ((above + held.height) * old_scale + travel.y) / scale - above;
    valid(Size::new(width, height))
}

#[cfg(test)]
#[path = "../../test/unit/winamp/stack_size_tests.rs"]
mod tests;
