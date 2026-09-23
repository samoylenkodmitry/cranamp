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
    connected(lead, windows, |_| true)
}

/// The windows hanging under window `lead`: those joined to it through
/// windows whose tops are all at or below its bottom edge, which move with
/// that edge when the window changes height.
pub(crate) fn hanging_below(lead: usize, windows: &[Rect]) -> Vec<usize> {
    let bottom = windows[lead].y + windows[lead].height;
    connected(lead, windows, |index| {
        windows[index].y >= bottom - TOUCH_TOLERANCE
    })
}

/// The windows joined to `lead` edge to edge, through windows `admit` lets
/// in.
fn connected(lead: usize, windows: &[Rect], admit: impl Fn(usize) -> bool) -> Vec<usize> {
    let mut group = vec![lead];
    let mut next = 0;
    while next < group.len() {
        let current = windows[group[next]];
        for (index, window) in windows.iter().enumerate() {
            if !group.contains(&index) && admit(index) && touching(current, *window) {
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

/// Where each window goes so that every one is inside `canvas`: each group of
/// windows touching one another moves as a whole, by the least that brings
/// the group in, so a docked stack stays docked. A group larger than the
/// canvas keeps its top left corner on screen.
pub(crate) fn kept_inside(windows: &[Rect], canvas: Size) -> Vec<Point> {
    let mut origins: Vec<Point> = windows
        .iter()
        .map(|rect| Point::new(rect.x, rect.y))
        .collect();
    let mut placed = vec![false; windows.len()];
    for lead in 0..windows.len() {
        if placed[lead] {
            continue;
        }
        let mut group = attached_to(lead, windows);
        group.push(lead);
        let left = group
            .iter()
            .map(|index| windows[*index].x)
            .fold(f32::INFINITY, f32::min);
        let top = group
            .iter()
            .map(|index| windows[*index].y)
            .fold(f32::INFINITY, f32::min);
        let right = group
            .iter()
            .map(|index| windows[*index].x + windows[*index].width)
            .fold(f32::NEG_INFINITY, f32::max);
        let bottom = group
            .iter()
            .map(|index| windows[*index].y + windows[*index].height)
            .fold(f32::NEG_INFINITY, f32::max);
        let dx = shift_into(left, right, canvas.width);
        let dy = shift_into(top, bottom, canvas.height);
        for index in group {
            placed[index] = true;
            origins[index] = Point::new(windows[index].x + dx, windows[index].y + dy);
        }
    }
    origins
}

fn shift_into(start: f32, end: f32, extent: f32) -> f32 {
    if end > extent {
        (extent - end).max(-start)
    } else if start < 0.0 {
        -start
    } else {
        0.0
    }
}

/// The windows' places as the text the preferences keep.
pub(crate) fn encode_positions(positions: &[Point]) -> String {
    positions
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(";")
}

/// The windows' places read back from [`encode_positions`], or nothing when
/// the text is not three places.
pub(crate) fn decode_positions(text: &str) -> Option<[Point; 3]> {
    let points: Vec<Point> = text
        .split(';')
        .map(|pair| {
            let (x, y) = pair.split_once(',')?;
            let (x, y) = (x.trim().parse::<f32>().ok()?, y.trim().parse::<f32>().ok()?);
            (x.is_finite() && y.is_finite()).then(|| Point::new(x, y))
        })
        .collect::<Option<_>>()?;
    points.try_into().ok()
}

/// The playlist's size as the text the preferences keep.
pub(crate) fn encode_size(size: Size) -> String {
    format!("{},{}", size.width, size.height)
}

/// The playlist's size read back from [`encode_size`], or nothing when the
/// text is not a size.
pub(crate) fn decode_size(text: &str) -> Option<Size> {
    let (width, height) = text.split_once(',')?;
    let (width, height) = (
        width.trim().parse::<f32>().ok()?,
        height.trim().parse::<f32>().ok()?,
    );
    (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
        .then(|| Size::new(width, height))
}

/// A window held at `held` and stretched from its corner by `travel`, never
/// smaller than `minimum`.
pub(crate) fn stretched(held: Size, travel: Point, minimum: Size) -> Size {
    Size::new(
        (held.width + travel.x).max(minimum.width),
        (held.height + travel.y).max(minimum.height),
    )
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
