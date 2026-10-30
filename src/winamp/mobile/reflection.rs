//! Project the player's lower third onto glass using the live GPU backdrop.
use super::*;
use cranpose_ui_graphics::{RuntimeShader, RUNTIME_SHADER_PRELUDE_WGSL};

const PLAYER_NAV_MARGIN: f32 = 6.;
// The plane widens toward the viewer and shears to the left. At its contact
// edge it has exactly the player's width and position.
const SPREAD: f32 = 0.24;
const SKEW: f32 = -0.18;

pub(super) fn player_layout(width: f32, height: f32, state: &WinampState) -> StackedLayout {
    resizable_stacked_layout(
        width,
        (height - NAV_HEIGHT + NAV_TOP_INSET - PLAYER_NAV_MARGIN).max(1.),
        0.,
        0.,
        state,
        ui_scale(),
    )
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
    top: f32,
    width: f32,
    source_left: f32,
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
    let source_height = snap(height / 3.);
    let depth = (viewport_height - bottom).max(0.);
    let source_width = snap(MAIN_WIDTH * layout.scale);
    ReflectionBounds {
        top: bottom - source_height,
        // Capture the full viewport so the projected plane can extend to the
        // left of a centered player in landscape and on tablets.
        width: viewport_width,
        source_left: snap(layout.content_left_inset),
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
    shader.set_float(11, area.source_left);
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
