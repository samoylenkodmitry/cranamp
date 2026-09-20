use cranpose_core::{compositionLocalOf, CompositionLocal, CompositionLocalProvider};
use cranpose_ui_graphics::Rect;
fn origin() -> CompositionLocal<[f32; 2]> {
    thread_local! {
        static LOCAL: std::cell::RefCell<Option<CompositionLocal<[f32; 2]>>> =
            const { std::cell::RefCell::new(None) };
    }
    LOCAL.with(|cell| {
        cell.borrow_mut()
            .get_or_insert_with(|| compositionLocalOf(|| [0., 0.]))
            .clone()
    })
}
pub fn provide(at: [f32; 2], content: impl FnOnce()) {
    CompositionLocalProvider(vec![origin().provides(at)], content);
}
pub fn rect(x: f32, y: f32, width: f32, height: f32, scale: f32) -> Rect {
    bounds(
        [x, y, width, height],
        origin().current(),
        scale,
        cranpose_ui::current_density(),
    )
}
fn bounds(r: [f32; 4], origin: [f32; 2], scale: f32, density: f32) -> Rect {
    let density = if density > 0. { density } else { 1. };
    let edge = |v: f32| (v * scale * density).round() / density;
    let left = edge(origin[0] + r[0]);
    let top = edge(origin[1] + r[1]);
    Rect {
        x: left - edge(origin[0]),
        y: top - edge(origin[1]),
        width: edge(origin[0] + r[0] + r[2]) - left,
        height: edge(origin[1] + r[1] + r[3]) - top,
    }
}
#[cfg(test)]
#[path = "../../test/unit/winamp/pixel_grid/tests.rs"]
mod tests;
