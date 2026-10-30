use cranpose::liquid::LiquidColors;
use cranpose_ui_graphics::Color;

pub(super) fn luminance(color: Color) -> f32 {
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.0) + 0.7152 * linear(color.1) + 0.0722 * linear(color.2)
}

pub(super) fn contrast(a: Color, b: Color) -> f32 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    Color(
        a.0 + (b.0 - a.0) * amount,
        a.1 + (b.1 - a.1) * amount,
        a.2 + (b.2 - a.2) * amount,
        1.,
    )
}

pub(super) fn from_background(background: Color) -> LiquidColors {
    let dark = super::theme::prefers_dark(background);
    let label = if dark { Color::WHITE } else { Color::BLACK };
    let depth = if dark { Color::BLACK } else { Color::WHITE };
    let surface = mix(background, depth, 0.12);
    // Keep the skin hue while reaching readable contrast for interactive text.
    let mut amount = 0.;
    let mut accent = background;
    while contrast(accent, background) < 7. && amount < 1. {
        amount = (amount + 0.025_f32).min(1.);
        accent = mix(background, label, amount);
    }
    let mut colors = if dark {
        LiquidColors::dark(accent)
    } else {
        LiquidColors::light(accent)
    };
    colors.background = background;
    colors.surface = surface;
    colors.surface_pressed = mix(surface, depth, 0.12);
    colors.label = label;
    let mut secondary_alpha = 0.76;
    while contrast(mix(background, label, secondary_alpha), background) < 4.5
        && secondary_alpha < 1.
    {
        secondary_alpha = (secondary_alpha + 0.02_f32).min(1.);
    }
    colors.secondary_label = label.with_alpha(secondary_alpha);
    colors.tertiary_label = label.with_alpha(secondary_alpha);
    colors.separator = accent.with_alpha(0.26);
    colors.fill = surface.with_alpha(0.65);
    colors.secondary_fill = surface.with_alpha(0.40);
    colors.surface_glass = surface.with_alpha(0.76);
    colors.glass_tint = background.with_alpha(0.18);
    colors.glass_stroke = accent.with_alpha(0.32);
    colors.on_accent = if super::theme::prefers_dark(accent) {
        Color::WHITE
    } else {
        Color::BLACK
    };
    colors.toggle_off = accent.with_alpha(0.30);
    colors
}
