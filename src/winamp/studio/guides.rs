//! Editor-only sprite guides. Rectangles never enter the exported artwork.
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Guide {
    pub id: String,
    pub label: String,
    pub sheet: String,
    pub rect: [u32; 4],
    pub source: [u32; 4],
    pub variant: usize,
    pub active: bool,
    /// A live readout: Cranamp writes over these pixels, and they have no
    /// source cell to paint.
    pub runtime: bool,
    /// A control Cranamp hit-tests but draws nothing for. The classic playlist
    /// footer's five menus and six transport keys are the whole of this: the
    /// artist has to draw a button there, and until these rectangles existed
    /// the only way to find out where was to read the player's source.
    pub hit: bool,
}
pub fn intersection(a: [u32; 4], b: [u32; 4]) -> Option<[u32; 4]> {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    let right = a[0].saturating_add(a[2]).min(b[0].saturating_add(b[2]));
    let bottom = a[1].saturating_add(a[3]).min(b[1].saturating_add(b[3]));
    (right > x && bottom > y).then(|| [x, y, right - x, bottom - y])
}
