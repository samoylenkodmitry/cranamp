//! Native-pixel classic EQ graph. Ink comes from EQMAIN.BMP, never TEXT.BMP.
use cranpose_ui::ImageBitmap;

pub fn preamp_y(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 18.0).round() as u32
}

pub fn render(values: [f32; 11], atlas: &ImageBitmap) -> ImageBitmap {
    let ys: [f64; 10] =
        std::array::from_fn(|i| 18.0 * (1.0 - f64::from(values[i + 1].clamp(0.0, 1.0))));
    // Natural cubic interpolation at the ten classic band positions (12px apart).
    let mut upper = [0.0; 10];
    let mut rhs = [0.0; 10];
    let mut second = [0.0; 10];
    for i in 1..9 {
        let divisor = 4.0 - upper[i - 1];
        upper[i] = 1.0 / divisor;
        rhs[i] = ((ys[i + 1] - 2.0 * ys[i] + ys[i - 1]) / 24.0 - rhs[i - 1]) / divisor;
    }
    for i in (1..9).rev() {
        second[i] = rhs[i] - upper[i] * second[i + 1];
    }
    let mut pixels = vec![0u8; 113 * 19 * 4];
    let mut previous = ys[0].round().clamp(0.0, 18.0) as usize;
    for x in 0..=108usize {
        let segment = (x / 12).min(8);
        let b = (x - segment * 12) as f64 / 12.0;
        let a = 1.0 - b;
        let y = (a * ys[segment]
            + b * ys[segment + 1]
            + ((a * a * a - a) * second[segment] + (b * b * b - b) * second[segment + 1]) * 24.0)
            .round()
            .clamp(0.0, 18.0) as usize;
        for row in previous.min(y)..=previous.max(y) {
            if atlas.width() > 115 && atlas.height() > 294 + row as u32 {
                let source = ((294 + row) * atlas.width() as usize + 115) * 4;
                let target = (row * 113 + x + 2) * 4;
                pixels[target..target + 4].copy_from_slice(&atlas.pixels()[source..source + 4]);
            }
        }
        previous = y;
    }
    ImageBitmap::from_rgba8(113, 19, pixels).expect("classic EQ curve")
}

#[cfg(test)]
#[path = "../../test/unit/winamp/eq_graph/tests.rs"]
mod tests;
