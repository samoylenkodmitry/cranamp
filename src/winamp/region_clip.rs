//! Mask the completed window, so live readouts and controls cannot paint over holes.
use super::*;
use cranpose_ui_graphics::{BlendMode, CompositingStrategy, GraphicsLayer};
pub fn modifier(regions: &skin::regions::Regions, section: &str, scale: f32) -> Modifier {
    if !regions.sections.contains_key(section) {
        return Modifier::empty();
    }
    let mask = regions.mask(section, 275, 116);
    let holes: Vec<Rect> = skin::regions::rectangles(&mask, 275, 116, false)
        .into_iter()
        .map(|[x, y, w, h]| pixel_grid::rect(x as f32, y as f32, w as f32, h as f32, scale))
        .collect();
    Modifier::empty()
        .graphics_layer(|| GraphicsLayer {
            compositing_strategy: CompositingStrategy::Offscreen,
            ..Default::default()
        })
        .draw_with_content(move |scope| {
            scope.draw_content();
            for rect in &holes {
                scope.draw_rect_at_blend(*rect, Brush::solid(Color::BLACK), BlendMode::Clear);
            }
        })
}
#[composable]
pub fn HoleInputShields(regions: skin::regions::Regions, section: &'static str, scale: f32) {
    if !regions.sections.contains_key(section) {
        return;
    }
    let mask = regions.mask(section, 275, 116);
    for [x, y, w, h] in skin::regions::rectangles(&mask, 275, 116, false) {
        let rect = pixel_grid::rect(x as f32, y as f32, w as f32, h as f32, scale);
        // Invisible controls under a cutout must not respond to pointer presses.
        Box(
            Modifier::empty()
                .absolute_offset(rect.x, rect.y)
                .size_points(rect.width, rect.height)
                .clickable(|_| {}),
            BoxSpec::default(),
            || {},
        );
    }
}
