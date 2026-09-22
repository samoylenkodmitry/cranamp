//! Classic text.bmp cells are 5 by 6 pixels, including their background.
use cranpose_ui::ImageBitmap;

pub const WIDTH: u32 = 5;
pub const HEIGHT: u32 = 6;

/// Winamp's getXYfromChar layout, including punctuation and the optional third row.
pub fn source(ch: char) -> (u32, u32) {
    let (column, row) = match ch.to_ascii_uppercase() {
        'A'..='Z' => (ch.to_ascii_uppercase() as u32 - 'A' as u32, 0),
        '0'..='9' => (ch as u32 - '0' as u32, 1),
        '\u{1}' => (10, 1),
        '.' => (11, 1),
        ':' => (12, 1),
        '(' => (13, 1),
        ')' => (14, 1),
        '-' => (15, 1),
        '\u{27}' | '\u{60}' => (16, 1),
        '!' => (17, 1),
        '_' => (18, 1),
        '+' => (19, 1),
        '\u{5c}' => (20, 1),
        '/' => (21, 1),
        '[' | '{' | '<' => (22, 1),
        ']' | '}' | '>' => (23, 1),
        '~' | '^' => (24, 1),
        '&' => (25, 1),
        '%' => (26, 1),
        ',' => (27, 1),
        '=' => (28, 1),
        '$' => (29, 1),
        '#' => (30, 1),
        'Å' | 'å' => (0, 2),
        'Ö' | 'ö' => (1, 2),
        'Ä' | 'ä' => (2, 2),
        '?' => (3, 2),
        '*' => (4, 2),
        '"' => (26, 0),
        '@' => (27, 0),
        _ => (30, 0),
    };
    (column * WIDTH, row * HEIGHT)
}

/// Copy the actual atlas, without recoloring or substituting a built-in face.
/// Pad with the skin's space cell and clip partial cells at the right edge.
pub fn render(atlas: &ImageBitmap, text: &str, width: u32) -> ImageBitmap {
    let width = width.max(1);
    let mut pixels = vec![0; width as usize * HEIGHT as usize * 4];
    let mut chars = text.chars();
    for x in (0..width).step_by(WIDTH as usize) {
        let (mut sx, mut sy) = source(chars.next().unwrap_or(' '));
        if sx + WIDTH > atlas.width() || sy + HEIGHT > atlas.height() {
            (sx, sy) = source(' ');
        }
        for y in 0..HEIGHT {
            for dx in 0..WIDTH.min(width - x) {
                if sx + dx < atlas.width() && sy + y < atlas.height() {
                    let src = (((sy + y) * atlas.width() + sx + dx) * 4) as usize;
                    let dst = ((y * width + x + dx) * 4) as usize;
                    pixels[dst..dst + 4].copy_from_slice(&atlas.pixels()[src..src + 4]);
                }
            }
        }
    }
    ImageBitmap::from_rgba8(width, HEIGHT, pixels).expect("classic text bitmap")
}

#[cfg(test)]
#[path = "../../test/unit/winamp/bitmap_font/tests.rs"]
mod tests;
