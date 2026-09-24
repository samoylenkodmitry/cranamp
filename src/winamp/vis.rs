//! Winamp 2's visualizer, as its main window drew it and its playlist drew it
//! while the main window was closed: a 76x16 field of `VISCOLOR.TXT`'s colour
//! 0 dotted with colour 1, and on it the spectrum analyzer or the
//! oscilloscope. Clicking it steps through the analyzer, the oscilloscope and
//! nothing.
//!
//! The analyzer is Winamp's default: 19 bars 3 pixels wide with a pixel
//! between, each row in its own colour from 2 at the top to 17 at the
//! bottom, falling back at 12/16 of a pixel a frame, with a peak in colour 23
//! that drops slowly and then faster. The oscilloscope draws the wave as
//! lines, coloured 18 to 22 by how far a row is from the middle.

/// The field Winamp draws in.
pub(crate) const WIDTH: usize = 76;
pub(crate) const HEIGHT: usize = 16;
pub(crate) const BARS: usize = 19;
const BAR_PITCH: usize = 4;
const BAR_WIDTH: usize = 3;
/// Winamp's frames run at about 60 a second; how far a bar and a peak fall is
/// counted in those.
pub(crate) const FRAME_MS: f32 = 1000.0 / 60.0;
const BAR_FALLOFF: f32 = 12.0 / 16.0;
const PEAK_START_SPEED: f32 = 3.0 / 256.0;
const PEAK_ACCELERATION: f32 = 1.1;
const OSCILLOSCOPE_COLUMNS: usize = 75;

/// `VISCOLOR.TXT` colours by what they paint; the field is colour 0, or what
/// the skin's bitmap mode puts there.
pub(crate) const DOTS: u8 = 1;
const ANALYZER_TOP: u8 = 2;
const OSCILLOSCOPE_MIDDLE: u8 = 18;
pub(crate) const PEAK: u8 = 23;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum VisMode {
    #[default]
    Analyzer,
    Oscilloscope,
    Off,
}

impl VisMode {
    /// The mode a click on the visualizer turns to.
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Analyzer => Self::Oscilloscope,
            Self::Oscilloscope => Self::Off,
            Self::Off => Self::Analyzer,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Analyzer => "analyzer",
            Self::Oscilloscope => "oscilloscope",
            Self::Off => "off",
        }
    }

    pub(crate) fn parse(name: &str) -> Option<Self> {
        [Self::Analyzer, Self::Oscilloscope, Self::Off]
            .into_iter()
            .find(|mode| mode.name() == name.trim())
    }
}

/// The analyzer's bars and peaks between frames, in pixels from the bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Analyzer {
    heights: [f32; BARS],
    peaks: [f32; BARS],
    peak_speeds: [f32; BARS],
}

impl Analyzer {
    /// Runs `frames` of Winamp's frames with the bands at `levels`, each from
    /// 0 to 1: a bar rises at once to its level and falls back slowly, and its
    /// peak is left behind to fall on its own.
    pub(crate) fn advance(&mut self, levels: &[f32; BARS], frames: u32) {
        for bar in 0..BARS {
            let level = if levels[bar].is_finite() {
                levels[bar].clamp(0.0, 1.0) * HEIGHT as f32
            } else {
                0.0
            };
            for _ in 0..frames.max(1) {
                self.heights[bar] = (self.heights[bar] - BAR_FALLOFF).max(level);
                if self.peaks[bar] <= self.heights[bar] {
                    self.peaks[bar] = self.heights[bar];
                    self.peak_speeds[bar] = PEAK_START_SPEED;
                } else {
                    self.peaks[bar] = (self.peaks[bar] - self.peak_speeds[bar]).max(0.0);
                    self.peak_speeds[bar] *= PEAK_ACCELERATION;
                }
            }
        }
    }

    /// The frame as runs of one colour: `(x, y, width, colour)`, each a row of
    /// a bar or a peak.
    pub(crate) fn runs(&self) -> Vec<(usize, usize, usize, u8)> {
        let mut out = Vec::new();
        for bar in 0..BARS {
            let x = bar * BAR_PITCH;
            let height = (self.heights[bar].round() as usize).min(HEIGHT);
            for y in HEIGHT - height..HEIGHT {
                out.push((x, y, BAR_WIDTH, ANALYZER_TOP + y as u8));
            }
            let peak = self.peaks[bar].round() as usize;
            if (1..=HEIGHT).contains(&peak) && peak > height {
                out.push((x, HEIGHT - peak, BAR_WIDTH, PEAK));
            }
        }
        out
    }
}

/// The colour of an oscilloscope row: the middle rows 18, and one colour on
/// for every two rows further out, up to 22 at the bottom edge.
fn oscilloscope_colour(y: usize) -> u8 {
    let offset = match y {
        14.. => 4,
        12..=13 => 3,
        10..=11 => 2,
        8..=9 => 1,
        6..=7 => 0,
        4..=5 => 1,
        2..=3 => 2,
        _ => 3,
    };
    OSCILLOSCOPE_MIDDLE + offset
}

/// The wave in `samples`, from -1 to 1, as the oscilloscope's runs of one
/// colour: each column joined to the one before it by a line down its column.
pub(crate) fn oscilloscope_runs(samples: &[f32]) -> Vec<(usize, usize, usize, u8)> {
    if samples.is_empty() {
        return Vec::new();
    }
    let row = |sample: f32| {
        let sample = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        (((1.0 - sample) * 0.5) * (HEIGHT - 1) as f32).round() as usize
    };
    let mut out = Vec::new();
    let mut last = None;
    for x in 0..OSCILLOSCOPE_COLUMNS {
        let y = row(samples[x * samples.len() / OSCILLOSCOPE_COLUMNS]);
        let from = last.unwrap_or(y);
        for run_y in from.min(y)..=from.max(y) {
            out.push((x, run_y, 1, oscilloscope_colour(run_y)));
        }
        last = Some(y);
    }
    out
}

/// The dots of the field behind everything: every other pixel of every other
/// row, from the second row down.
pub(crate) fn dots() -> impl Iterator<Item = (usize, usize)> {
    (1..HEIGHT)
        .step_by(2)
        .flat_map(|y| (0..WIDTH).step_by(2).map(move |x| (x, y)))
}

#[cfg(test)]
#[path = "../../test/unit/winamp/vis/tests.rs"]
mod tests;
