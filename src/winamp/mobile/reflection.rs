//! Project the player's lower third onto glass using the live GPU backdrop.
use super::*;
use cranpose_ui_graphics::{RuntimeShader, RUNTIME_SHADER_PRELUDE_WGSL};

use super::super::navigation::{PLAYER_GAP, REFLECTION_GAP, SIDE_MARGIN, TOP_MARGIN};
pub(super) fn player_layout(width: f32, height: f32, state: &WinampState) -> StackedLayout {
    let visible = state.navigation.visible();
    let side = if visible { SIDE_MARGIN } else { 0. };
    let top = if visible { TOP_MARGIN } else { 0. };
    let bottom = if visible {
        NAV_HEIGHT - NAV_TOP_INSET + PLAYER_GAP
    } else {
        0.
    };
    let mut layout = resizable_stacked_layout(
        (width - side * 2.).max(1.),
        height,
        top,
        bottom,
        state,
        ui_scale(),
    );
    layout.content_left_inset += side;
    layout
}

#[derive(Clone, Copy, Debug)]
struct ReflectionBounds {
    top: f32,
    width: f32,
    source_left: f32,
    source_width: f32,
    source_height: f32,
    depth: f32,
}

pub(super) fn player_bounds(state: &WinampState, layout: StackedLayout) -> Rect {
    Rect::from_origin_size(
        Point::new(layout.content_left_inset, layout.content_top_inset),
        Size::new(
            MAIN_WIDTH * layout.scale,
            stacked_skin_height(state, layout.playlist_height) * layout.scale,
        ),
    )
}

fn bounds(
    player: Rect,
    viewport_width: f32,
    viewport_height: f32,
    density: f32,
) -> ReflectionBounds {
    let snap = |value: f32| (value * density).round() / density;
    let height = snap(player.height);
    let bottom = snap(player.y) + height;
    let source_height = snap(height / 3.);
    let depth = (viewport_height - bottom).max(0.);
    ReflectionBounds {
        top: bottom - source_height,
        width: viewport_width,
        source_left: snap(player.x),
        source_width: snap(player.width),
        source_height,
        depth,
    }
}

fn shader(area: ReflectionBounds, background: Color) -> RuntimeShader {
    static SOURCE: std::sync::OnceLock<RuntimeShader> = std::sync::OnceLock::new();
    let mut shader = SOURCE
        .get_or_init(|| {
            let mut shader = RuntimeShader::new(&format!(
                "{RUNTIME_SHADER_PRELUDE_WGSL}\n{}",
                include_str!("reflection.wgsl")
            ));
            shader.set_position_independent(true);
            shader
        })
        .clone();
    shader.set_float2(0, area.width, area.source_height + area.depth);
    shader.set_float(2, area.source_height);
    shader.set_float(3, area.depth);
    shader.set_float4(4, background.0, background.1, background.2, 1.);
    shader.set_float(8, area.source_width);
    shader.set_float(9, area.source_left);
    shader.set_float(10, REFLECTION_GAP);
    shader
}

#[composable]
pub(in crate::winamp) fn FloorReflection(
    background: Color,
    player: Rect,
    viewport_width: f32,
    viewport_height: f32,
) {
    let area = bounds(player, viewport_width, viewport_height, current_density());
    if area.depth <= 0. {
        return;
    }
    // Capture the complete lower third, regardless of the available floor depth.
    // Preserve it above the contact edge; project its mirror onto the floor below.
    // This empty drawing layer has no input handlers or accessibility nodes.
    Box(
        Modifier::empty()
            .absolute_offset(0., area.top)
            .size_points(area.width, area.source_height + area.depth)
            .shader_background(shader(area, background)),
        BoxSpec::default(),
        || {},
    );
}

#[cfg(test)]
#[path = "../../../test/unit/winamp/mobile_reflection.rs"]
mod tests;
