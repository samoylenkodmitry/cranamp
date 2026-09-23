//! How the player's windows keep together when they share one canvas, the way
//! Winamp's did on a desktop: an edge put down near another window's edge, or
//! near the canvas edge, lands on it, and the windows touching the main window
//! travel with it.

use cranpose_ui::{Point, Rect, Size};

/// How near an edge has to come to another to land on it, Winamp's own
/// distance.
pub(crate) const SNAP_DISTANCE: f32 = 10.0;

const TOUCH_TOLERANCE: f32 = 0.5;

/// Where a window dragged to `proposed` settles: each axis moves to the
/// nearest edge of `others` or of the canvas within [`SNAP_DISTANCE`], and
/// keeps the dragged position when none is near.
pub(crate) fn snapped_origin(proposed: Rect, others: &[Rect], canvas: Option<Size>) -> Point {
    let mut x_targets = Vec::new();
    let mut y_targets = Vec::new();
    for other in others {
        if spans_overlap(
            proposed.y,
            proposed.height,
            other.y,
            other.height,
            SNAP_DISTANCE,
        ) {
            x_targets.extend([
                other.x + other.width,
                other.x - proposed.width,
                other.x,
                other.x + other.width - proposed.width,
            ]);
        }
        if spans_overlap(
            proposed.x,
            proposed.width,
            other.x,
            other.width,
            SNAP_DISTANCE,
        ) {
            y_targets.extend([
                other.y + other.height,
                other.y - proposed.height,
                other.y,
                other.y + other.height - proposed.height,
            ]);
        }
    }
    if let Some(canvas) = canvas {
        x_targets.extend([0.0, canvas.width - proposed.width]);
        y_targets.extend([0.0, canvas.height - proposed.height]);
    }
    Point::new(
        nearest_within(proposed.x, &x_targets),
        nearest_within(proposed.y, &y_targets),
    )
}

/// The windows that travel with window `lead`: every window that touches it,
/// and every window touching one of those, edge to edge.
pub(crate) fn attached_to(lead: usize, windows: &[Rect]) -> Vec<usize> {
    let mut group = vec![lead];
    let mut next = 0;
    while next < group.len() {
        let current = windows[group[next]];
        for (index, window) in windows.iter().enumerate() {
            if !group.contains(&index) && touching(current, *window) {
                group.push(index);
            }
        }
        next += 1;
    }
    group.retain(|index| *index != lead);
    group.sort_unstable();
    group
}

/// Whether two windows share an edge: one ends where the other starts, and
/// they sit side by side along that edge.
pub(crate) fn touching(a: Rect, b: Rect) -> bool {
    let near = |first: f32, second: f32| (first - second).abs() <= TOUCH_TOLERANCE;
    let side_by_side = (near(a.x + a.width, b.x) || near(b.x + b.width, a.x))
        && spans_overlap(a.y, a.height, b.y, b.height, 0.0);
    let stacked = (near(a.y + a.height, b.y) || near(b.y + b.height, a.y))
        && spans_overlap(a.x, a.width, b.x, b.width, 0.0);
    side_by_side || stacked
}

fn spans_overlap(start: f32, length: f32, other_start: f32, other_length: f32, slack: f32) -> bool {
    start < other_start + other_length + slack && other_start < start + length + slack
}

fn nearest_within(value: f32, targets: &[f32]) -> f32 {
    targets
        .iter()
        .copied()
        .filter(|target| (target - value).abs() <= SNAP_DISTANCE)
        .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
        .unwrap_or(value)
}

#[cfg(test)]
#[path = "../../test/unit/winamp/inline_windows/tests.rs"]
mod tests;
