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
    pub runtime: bool,
    pub hit: bool,
    /// The player paints the whole rectangle while music plays, as it does
    /// the visualizer's field: the art under it shows only when stopped.
    pub opaque: bool,
}
pub fn intersection(a: [u32; 4], b: [u32; 4]) -> Option<[u32; 4]> {
    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    let right = a[0].saturating_add(a[2]).min(b[0].saturating_add(b[2]));
    let bottom = a[1].saturating_add(a[3]).min(b[1].saturating_add(b[3]));
    (right > x && bottom > y).then(|| [x, y, right - x, bottom - y])
}
