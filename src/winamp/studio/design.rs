//! Shared Studio control surfaces. Values are logical pixels on every platform.
use super::*;
use cranpose_animation::{animateColorAsState, tween, Easing};
use cranpose_ui_graphics::{Brush, CornerRadii, Point, Stroke};

macro_rules! palette_color {
    ($name:ident, $field:ident) => {
        pub(super) fn $name() -> Color {
            cranpose::liquid::liquid_colors().$field
        }
    };
}
palette_color!(studio_bg, background);
palette_color!(studio_card, surface);
palette_color!(studio_fg, label);
palette_color!(studio_dim, secondary_label);
palette_color!(studio_accent, accent);
palette_color!(studio_accent_lit, accent);
palette_color!(studio_pip_off, fill);
palette_color!(studio_error, destructive);

pub(super) const CONTROL_RADIUS: f32 = 8.;
pub(super) const PANEL_RADIUS: f32 = 12.;

#[derive(Clone, Copy)]
pub(super) enum Surface {
    Panel,
    Well,
    Button,
    Icon,
    Row,
    Field,
    Invalid,
    Primary,
}

#[track_caller]
pub(super) fn surface(role: Surface, selected: bool, hovered: bool, pressed: bool) -> Modifier {
    let colors = cranpose::liquid::liquid_colors();
    let base = match role {
        Surface::Primary => colors.accent,
        Surface::Well | Surface::Field => colors.background,
        Surface::Row => Color::TRANSPARENT,
        _ => colors.surface,
    };
    let (mut top, mut bottom, mut border) = (base, base, colors.separator);
    if selected || hovered || pressed {
        let tint = if matches!(role, Surface::Primary) {
            colors.on_accent
        } else {
            colors.accent
        };
        top = mix(base, tint, if pressed { 0.22 } else { 0.10 });
        bottom = mix(base, tint, if pressed { 0.16 } else { 0.06 });
        border = colors.accent;
    }
    if matches!(role, Surface::Invalid) {
        border = colors.destructive;
    }
    let selection_glow = colors.accent.with_alpha(0.14);
    let top = animateColorAsState(top, tween(130, Easing::EaseOut), "Studio surface top").get();
    let bottom =
        animateColorAsState(bottom, tween(130, Easing::EaseOut), "Studio surface base").get();
    let border =
        animateColorAsState(border, tween(130, Easing::EaseOut), "Studio surface edge").get();
    let inset = match role {
        Surface::Icon | Surface::Row => 2.,
        _ => 0.5,
    };
    let radius = if matches!(role, Surface::Panel) {
        PANEL_RADIUS
    } else {
        CONTROL_RADIUS
    };
    Modifier::empty().draw_with_content(move |scope| {
        let size = scope.size();
        let rect = Rect {
            x: inset,
            y: inset,
            width: (size.width - inset * 2.).max(0.),
            height: (size.height - inset * 2.).max(0.),
        };
        let radii = CornerRadii::uniform(radius.min(rect.height / 2.));
        if bottom.3 > 0. {
            scope.draw_round_rect_at(
                rect,
                Brush::linear_gradient_range(
                    vec![top, bottom],
                    Point::new(0., 0.),
                    Point::new(0., size.height),
                ),
                radii,
            );
        }
        scope.draw_content();
        if border.3 > 0. {
            scope.draw_round_rect_at_stroked(rect, Brush::solid(border), radii, Stroke::new(1.));
            if selected {
                let inner = Rect {
                    x: rect.x + 2.,
                    y: rect.y + 2.,
                    width: (rect.width - 4.).max(0.),
                    height: (rect.height - 4.).max(0.),
                };
                scope.draw_round_rect_at_stroked(
                    inner,
                    Brush::solid(selection_glow),
                    CornerRadii::uniform((radius - 2.).max(0.)),
                    Stroke::new(1.),
                );
            }
        }
    })
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    Color(
        a.0 + (b.0 - a.0) * amount,
        a.1 + (b.1 - a.1) * amount,
        a.2 + (b.2 - a.2) * amount,
        a.3 + (b.3 - a.3) * amount,
    )
}

/// One rail and thumb drawing shared by brush and layer sliders.
#[composable]
pub(super) fn SliderTrack(width: f32, height: f32, thumb_offset: f32) {
    let rail_color = studio_pip_off();
    let accent = studio_accent();
    let accent_lit = studio_accent_lit();
    let edge = studio_dim();
    let thumb = studio_fg();
    cranpose_ui::Canvas(Modifier::empty().size_points(width, height), move |scope| {
        let y = height / 2.;
        let rail = Rect {
            x: 1.,
            y: y - 2.5,
            width: (width - 2.).max(0.),
            height: 5.,
        };
        scope.draw_round_rect_at(rail, Brush::solid(rail_color), CornerRadii::uniform(2.5));
        scope.draw_round_rect_at(
            Rect {
                width: (thumb_offset + 8.).max(0.),
                ..rail
            },
            Brush::linear_gradient_range(
                vec![accent, accent_lit],
                Point::new(0., y),
                Point::new(width, y),
            ),
            CornerRadii::uniform(2.5),
        );
        let centre = Point::new(thumb_offset + 8., y);
        scope.draw_circle(
            Brush::solid(Color(0., 0., 0., 0.35)),
            Point::new(centre.x, centre.y + 1.),
            8.5,
        );
        scope.draw_circle(Brush::solid(edge), centre, 8.);
        scope.draw_circle(
            Brush::linear_gradient_range(
                vec![thumb, mix(thumb, accent, 0.18)],
                Point::new(centre.x, y - 7.),
                Point::new(centre.x, y + 7.),
            ),
            centre,
            7.,
        );
    });
}
