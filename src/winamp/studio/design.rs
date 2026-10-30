//! Shared Studio control surfaces. Values are logical pixels on every platform.
use super::*;
use cranpose_animation::{animateColorAsState, tween, Easing};
use cranpose_ui_graphics::{Brush, CornerRadii, Point, Stroke};

// Desktop workbench colors stay fixed. Mobile Studio reads the same live
// Liquid palette as Library and Settings, through this composition local.
pub(super) fn palette_local(
) -> cranpose_core::CompositionLocal<Option<cranpose::liquid::LiquidColors>> {
    thread_local! {
        static LOCAL: std::cell::OnceCell<cranpose_core::CompositionLocal<Option<cranpose::liquid::LiquidColors>>> = const { std::cell::OnceCell::new() };
    }
    LOCAL.with(|local| {
        local
            .get_or_init(|| cranpose_core::compositionLocalOf(|| None))
            .clone()
    })
}

macro_rules! palette_color {
    ($name:ident, $field:ident, $fallback:expr) => {
        pub(super) fn $name() -> Color {
            palette_local()
                .current()
                .map_or(rgb($fallback), |colors| colors.$field)
        }
    };
}
palette_color!(studio_bg, background, 0x0b1018);
palette_color!(studio_panel_back, surface, 0x111a26);
palette_color!(studio_card, surface, 0x172437);
palette_color!(studio_card_on, surface_pressed, 0x19365e);
palette_color!(studio_fg, label, 0xe8f6ff);
palette_color!(studio_dim, secondary_label, 0x8ca6c4);
palette_color!(studio_accent, accent, 0x387aff);
palette_color!(studio_accent_lit, accent, 0x7ddcff);
palette_color!(studio_pip_off, fill, 0x2b405b);
palette_color!(studio_border, separator, 0x29425d);
palette_color!(studio_error, destructive, 0xff686b);

pub(super) const CONTROL_RADIUS: f32 = 8.;
pub(super) const PANEL_RADIUS: f32 = 12.;

const fn rgb(value: u32) -> Color {
    Color(
        ((value >> 16) & 255) as f32 / 255.,
        ((value >> 8) & 255) as f32 / 255.,
        (value & 255) as f32 / 255.,
        1.,
    )
}

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
    let (mut top, mut bottom, mut border) = match role {
        Surface::Panel => (rgb(0x172231), studio_panel_back(), rgb(0x24374c)),
        Surface::Well => (rgb(0x0c1520), rgb(0x101c2a), rgb(0x203348)),
        Surface::Field => (rgb(0x101b29), rgb(0x162437), studio_border()),
        Surface::Invalid => (rgb(0x241923), rgb(0x1d1721), studio_error()),
        Surface::Primary => (rgb(0x4386ff), rgb(0x2461db), rgb(0x6a9eff)),
        Surface::Row => (
            Color(0., 0., 0., 0.),
            Color(0., 0., 0., 0.),
            Color(0., 0., 0., 0.),
        ),
        Surface::Button | Surface::Icon => (rgb(0x1c2b3e), rgb(0x121e2e), studio_border()),
    };
    if selected {
        top = rgb(0x234b80);
        bottom = studio_card_on();
        border = studio_accent();
    }
    if hovered {
        top = mix(top, rgb(0x39608b), 0.22);
        bottom = mix(bottom, rgb(0x294568), 0.15);
        border = mix(border, studio_accent_lit(), 0.48);
    }
    if pressed {
        top = rgb(0x112742);
        bottom = rgb(0x1b395d);
        border = studio_accent_lit();
    }
    if let Some(colors) = palette_local().current() {
        let base = match role {
            Surface::Primary => colors.accent,
            Surface::Well | Surface::Field => colors.background,
            Surface::Row => Color::TRANSPARENT,
            _ => colors.surface,
        };
        top = base;
        bottom = base;
        border = colors.separator;
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
    }
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
                    Brush::solid(Color(0.35, 0.65, 1., 0.14)),
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
                vec![Color::WHITE, rgb(0xb9d9ef)],
                Point::new(centre.x, y - 7.),
                Point::new(centre.x, y + 7.),
            ),
            centre,
            7.,
        );
    });
}
