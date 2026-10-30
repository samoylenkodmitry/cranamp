//! Project the player's lower half onto glass using the live GPU backdrop.
use super::*;
use cranpose_ui_graphics::{RuntimeShader, RUNTIME_SHADER_PRELUDE_WGSL};

pub(super) const GAP: f32 = 88.;
const DEPTH: f32 = 128.;
// The plane widens toward the viewer and shears to the right. At its contact
// edge it has exactly the player's width and position.
const SPREAD: f32 = 0.24;
const SKEW: f32 = 0.18;

pub(super) fn player_layout(width: f32, height: f32, state: &WinampState) -> StackedLayout {
    let mut layout = resizable_stacked_layout(
        width,
        (height - NAV_HEIGHT).max(1.),
        0.,
        0.,
        state,
        ui_scale(),
    );
    if state.playlist_visible {
        // Use spare playlist rows for the reflection. Keep the original skin
        // scale when a small screen has already reached the minimum height.
        let spare = (layout.playlist_height - playlist_min_height()).max(0.);
        layout.playlist_height -= (GAP / layout.scale).min(spare);
    }
    layout
}

pub(super) fn corner_color(skin: &WinampSkin) -> Color {
    let pixel = &skin.main.pixels()[..4];
    Color(
        pixel[0] as f32 / 255.,
        pixel[1] as f32 / 255.,
        pixel[2] as f32 / 255.,
        1.,
    )
}

#[derive(Clone, Copy, Debug)]
struct ReflectionBounds {
    left: f32,
    top: f32,
    width: f32,
    source_width: f32,
    source_height: f32,
    depth: f32,
}

fn bounds(
    state: &WinampState,
    layout: StackedLayout,
    viewport_width: f32,
    viewport_height: f32,
    density: f32,
) -> ReflectionBounds {
    let skin_height = main_height(state.main_shaded)
        + if state.eq_visible { EQ_HEIGHT } else { 0. }
        + if state.playlist_visible {
            layout.playlist_height
        } else {
            0.
        };
    let snap = |value: f32| (value * density).round() / density;
    let height = snap(skin_height * layout.scale);
    let bottom = snap(layout.content_top_inset) + height;
    // Align the capture to physical pixels so rounding its offscreen texture
    // cannot shift the projected footer by half a source pixel.
    let source_height = snap(height * 0.5);
    let depth = (viewport_height - bottom)
        .clamp(0., DEPTH)
        .min(source_height * 0.5);
    let left = snap(layout.content_left_inset);
    let source_width = snap(MAIN_WIDTH * layout.scale);
    ReflectionBounds {
        left,
        top: bottom - source_height,
        width: snap(source_width * (1. + SPREAD * 0.5 + SKEW)).min((viewport_width - left).max(0.)),
        source_width,
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
    shader.set_float(9, SPREAD);
    shader.set_float(10, SKEW);
    shader
}

#[composable]
pub(super) fn FloorReflection(
    background: Color,
    state: WinampState,
    layout: StackedLayout,
    viewport_width: f32,
    viewport_height: f32,
) {
    let area = bounds(
        &state,
        layout,
        viewport_width,
        viewport_height,
        current_density(),
    );
    if area.depth <= 0. {
        return;
    }
    // Capture the complete lower half, regardless of the available floor depth.
    // Preserve it above the contact edge; project its mirror onto the floor below.
    // This empty drawing layer has no input handlers or accessibility nodes.
    Box(
        Modifier::empty()
            .absolute_offset(area.left, area.top)
            .size_points(area.width, area.source_height + area.depth)
            .shader_background(shader(area, background)),
        BoxSpec::default(),
        || {},
    );
}

#[cfg(test)]
#[path = "../../../test/unit/winamp/mobile_reflection.rs"]
mod tests;
