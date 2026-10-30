use super::*;
use cranpose_foundation::PointerSource;
use cranpose_ui::{Canvas, CustomPointerIcon, PointerIcon};
use cranpose_ui_graphics::DrawScope;

pub(super) fn hint_context() -> cranpose_core::CompositionLocal<Option<Hints>> {
    thread_local! {
        static LOCAL: std::cell::RefCell<Option<cranpose_core::CompositionLocal<Option<Hints>>>> = const { std::cell::RefCell::new(None) };
    }
    LOCAL.with(|local| {
        local
            .borrow_mut()
            .get_or_insert_with(|| cranpose_core::compositionLocalOf(|| None))
            .clone()
    })
}
pub(crate) fn feedback(_label: String) -> Modifier {
    super::motion::feedback()
}

pub(super) fn hint_feedback(label: String) -> Modifier {
    hint_feedback_colour(label, studio_accent_lit())
}

pub(super) fn hint_feedback_colour(label: String, tint: Color) -> Modifier {
    let hint = hint_context().current();
    super::motion::feedback_colour(tint).then(hint.map_or_else(Modifier::empty, |hints| {
        hover_hint(hints, label.clone(), label, true)
    }))
}

fn hover_hint(hints: Hints, label: String, text: String, enabled: bool) -> Modifier {
    Modifier::empty().pointer_input(
        (
            label.clone(),
            text.clone(),
            enabled,
            hints.runtime_state_id(),
        ),
        move |scope: PointerInputScope| {
            let label = label.clone();
            let text = text.clone();
            async move {
                scope
                    .await_pointer_event_scope(|events| async move {
                        loop {
                            let event = events.await_pointer_event().await;
                            if enabled
                                && !text.is_empty()
                                && event.source != PointerSource::Touch
                                && !event.buttons.contains(PointerButton::Primary)
                                && matches!(
                                    event.kind,
                                    PointerEventKind::Enter | PointerEventKind::Move
                                )
                            {
                                hints.set(Some(Hint {
                                    label: label.clone(),
                                    text: text.clone(),
                                    x: event.global_position.x,
                                    y: event.global_position.y,
                                }));
                            } else if matches!(
                                event.kind,
                                PointerEventKind::Exit | PointerEventKind::Down
                            ) && hints
                                .get_non_reactive()
                                .is_some_and(|h| h.label == label)
                            {
                                hints.set(None);
                            }
                        }
                    })
                    .await;
            }
        },
    )
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Hint {
    pub label: String,
    pub text: String,
    pub x: f32,
    pub y: f32,
}
pub(super) type Hints = cranpose_core::MutableState<Option<Hint>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Icon {
    #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
    Agent,
    #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
    Copy,
    #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
    Power,
    #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
    Test,
    Back,
    Add,
    Close,
    Undo,
    Redo,
    More,
    Brush,
    Canvas,
    Colour,
    Select,
    Layers,
    Play,
    Open,
    Save,
    Share,
    Hand,
    Parts,
    Grid,
    Minus,
    Eye,
    Lock,
    Help,
    Settings,
    Line,
    Rectangle,
    Ellipse,
    Text,
    Fit,
    History,
    Curve,
    Stamp,
    Fill,
    State,
    Up,
    Down,
    Merge,
    Delete,
    Check,
    FlipH,
    FlipV,
    Rotate,
    Taper,
    Glass,
}
fn icon_source(icon: Icon) -> &'static [u8] {
    match icon {
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        Icon::Agent => include_bytes!("../../../../assets/studio/icons/agent.svg"),
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        Icon::Copy => include_bytes!("../../../../assets/studio/icons/copy.svg"),
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        Icon::Power => include_bytes!("../../../../assets/studio/icons/power.svg"),
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        Icon::Test => include_bytes!("../../../../assets/studio/icons/test.svg"),
        Icon::Canvas => include_bytes!("../../../../assets/studio/icons/canvas.svg"),
        Icon::Colour => include_bytes!("../../../../assets/studio/icons/colour.svg"),
        Icon::Taper => include_bytes!("../../../../assets/studio/icons/taper.svg"),
        Icon::Glass => include_bytes!("../../../../assets/studio/icons/glass.svg"),
        Icon::Up => include_bytes!("../../../../assets/studio/icons/up.svg"),
        Icon::Down => include_bytes!("../../../../assets/studio/icons/down.svg"),
        Icon::Merge => include_bytes!("../../../../assets/studio/icons/merge.svg"),
        Icon::Delete => include_bytes!("../../../../assets/studio/icons/delete.svg"),
        Icon::Check => include_bytes!("../../../../assets/studio/icons/check.svg"),
        Icon::FlipH => include_bytes!("../../../../assets/studio/icons/flip-h.svg"),
        Icon::FlipV => include_bytes!("../../../../assets/studio/icons/flip-v.svg"),
        Icon::Rotate => include_bytes!("../../../../assets/studio/icons/rotate.svg"),
        Icon::State => include_bytes!("../../../../assets/studio/icons/state.svg"),
        Icon::Back => include_bytes!("../../../../assets/studio/icons/back.svg"),
        Icon::Add => include_bytes!("../../../../assets/studio/icons/add.svg"),
        Icon::Close => include_bytes!("../../../../assets/studio/icons/close.svg"),
        Icon::Undo => include_bytes!("../../../../assets/studio/icons/undo.svg"),
        Icon::Redo => include_bytes!("../../../../assets/studio/icons/redo.svg"),
        Icon::More => include_bytes!("../../../../assets/studio/icons/more.svg"),
        Icon::Brush => include_bytes!("../../../../assets/studio/icons/brush.svg"),
        Icon::Select => include_bytes!("../../../../assets/studio/icons/select.svg"),
        Icon::Layers => include_bytes!("../../../../assets/studio/icons/layers.svg"),
        Icon::Play => include_bytes!("../../../../assets/studio/icons/play.svg"),
        Icon::Open => include_bytes!("../../../../assets/studio/icons/open.svg"),
        Icon::Save => include_bytes!("../../../../assets/studio/icons/save.svg"),
        Icon::Share => include_bytes!("../../../../assets/studio/icons/share.svg"),
        Icon::Hand => include_bytes!("../../../../assets/studio/icons/hand.svg"),
        Icon::Parts => include_bytes!("../../../../assets/studio/icons/parts.svg"),
        Icon::Grid => include_bytes!("../../../../assets/studio/icons/grid.svg"),
        Icon::Minus => include_bytes!("../../../../assets/studio/icons/minus.svg"),
        Icon::Eye => include_bytes!("../../../../assets/studio/icons/eye.svg"),
        Icon::Lock => include_bytes!("../../../../assets/studio/icons/lock.svg"),
        Icon::Help => include_bytes!("../../../../assets/studio/icons/help.svg"),
        Icon::Settings => {
            include_bytes!("../../../../assets/studio/icons/settings.svg")
        }
        Icon::Line => include_bytes!("../../../../assets/studio/icons/line.svg"),
        Icon::Rectangle => {
            include_bytes!("../../../../assets/studio/icons/rectangle.svg")
        }
        Icon::Ellipse => include_bytes!("../../../../assets/studio/icons/ellipse.svg"),
        Icon::Text => include_bytes!("../../../../assets/studio/icons/text.svg"),
        Icon::Fit => include_bytes!("../../../../assets/studio/icons/fit.svg"),
        Icon::History => include_bytes!("../../../../assets/studio/icons/history.svg"),
        Icon::Curve => include_bytes!("../../../../assets/studio/icons/curve.svg"),
        Icon::Stamp => include_bytes!("../../../../assets/studio/icons/stamp.svg"),
        Icon::Fill => include_bytes!("../../../../assets/studio/icons/fill.svg"),
    }
}
pub(super) fn draw_icon(scope: &mut dyn DrawScope, icon: Icon, color: Color) {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<Icon, ImageBitmap>> = std::cell::RefCell::new(std::collections::HashMap::new());
    }
    let bitmap = CACHE.with(|cache| {
        cache
            .borrow_mut()
            .entry(icon)
            .or_insert_with(|| {
                let source = icon_source(icon);
                cranpose_ui::SvgPainter::from_bytes(source)
                    .expect("bundled studio icon")
                    .rasterize(cranpose_ui::Size::new(96., 96.))
                    .expect("studio icon pixels")
            })
            .clone()
    });
    scope.draw_image_at(
        Rect {
            x: 0.,
            y: 0.,
            width: 24.,
            height: 24.,
        },
        bitmap,
        1.,
        Some(cranpose_ui_graphics::ColorFilter::tint(color)),
    );
}
pub(super) fn control_cursor() -> PointerIcon {
    static ICON: std::sync::OnceLock<PointerIcon> = std::sync::OnceLock::new();
    ICON.get_or_init(|| {
        let palette = cursor_art::Palette {
            ink: [7, 18, 29, 255],
            body: [225, 244, 250, 255],
            accent: [53, 216, 233, 255],
            ground: [14, 27, 40, 255],
        };
        let (image, hotspot) = cursor_art::draw(
            crate::winamp::cursors::SkinCursor::MainMenu,
            &palette,
            cursor_art::Style::named("sharp").expect("built-in cursor style"),
        );
        ImageBitmap::from_rgba8(image.width(), image.height(), image.into_raw())
            .ok()
            .and_then(|bitmap| CustomPointerIcon::new(bitmap, hotspot[0], hotspot[1]).ok())
            .map(PointerIcon::Custom)
            .unwrap_or(PointerIcon::POINTER)
    })
    .clone()
}
pub(super) fn canvas_cursor(tool: &str, hand: bool) -> PointerIcon {
    let icon = if hand {
        Icon::Hand
    } else {
        super::workbench::brush_icon(tool)
    };
    thread_local! {
        static CURSORS: std::cell::RefCell<std::collections::HashMap<Icon, PointerIcon>> = Default::default();
    }
    CURSORS.with(|cache| {
        cache
            .borrow_mut()
            .entry(icon)
            .or_insert_with(|| {
                let (bitmap, hotspot) = instrument_cursor(icon);
                PointerIcon::Custom(
                    CustomPointerIcon::new(bitmap, hotspot[0], hotspot[1])
                        .expect("instrument cursor hotspot"),
                )
            })
            .clone()
    })
}

fn instrument_cursor(icon: Icon) -> (ImageBitmap, [u32; 2]) {
    // A precise tip at the upper left; the tool stays below and to its right.
    // A dark halo keeps the white instrument visible over both light and dark art.
    let image = cranpose_ui::SvgPainter::from_bytes(icon_source(icon))
        .expect("instrument SVG")
        .rasterize(cranpose_ui::Size::new(24., 24.))
        .expect("instrument pixels");
    let mut pixels = vec![0; 36 * 36 * 4];
    for y in 0..24usize {
        for x in 0..24usize {
            if image.pixels()[(y * 24 + x) * 4 + 3] > 60 {
                for dy in 0..3 {
                    for dx in 0..3 {
                        let at = ((y + 8 + dy) * 36 + x + 8 + dx) * 4;
                        pixels[at..at + 4].copy_from_slice(&[8, 15, 23, 255]);
                    }
                }
            }
        }
    }
    for y in 0..24usize {
        for x in 0..24usize {
            let alpha = image.pixels()[(y * 24 + x) * 4 + 3];
            if alpha > 60 {
                let at = ((y + 9) * 36 + x + 9) * 4;
                pixels[at..at + 4].copy_from_slice(&[235, 249, 255, 255]);
            }
        }
    }
    for y in 0..9usize {
        for x in 0..9usize {
            if x.abs_diff(4) <= 1 || y.abs_diff(4) <= 1 {
                let at = (y * 36 + x) * 4;
                pixels[at..at + 4].copy_from_slice(if x == 4 || y == 4 {
                    &[74, 227, 233, 255]
                } else {
                    &[8, 15, 23, 255]
                });
            }
        }
    }
    (
        ImageBitmap::from_rgba8(36, 36, pixels).expect("cursor pixels"),
        [4, 4],
    )
}
#[allow(clippy::too_many_arguments)]
#[composable]
pub(super) fn Control(
    icon: Icon,
    label: String,
    detail: String,
    area: Area,
    selected: bool,
    enabled: bool,
    caption: bool,
    hints: Hints,
    click: impl Fn() + 'static,
) {
    let source = cranpose_ui::rememberMutableInteractionSource();
    let hovered = cranpose_ui::collect_is_hovered_as_state(&source).get();
    let pressed = cranpose_ui::collect_is_pressed_as_state(&source).get();
    let help = Hint {
        label: label.clone(),
        text: if caption {
            if detail == label {
                String::new()
            } else {
                detail
            }
        } else if detail.is_empty() || detail == label {
            label.clone()
        } else {
            format!("{label}\n{detail}")
        },
        x: area.x,
        y: area.y,
    };
    let help_modifier = hover_hint(hints, help.label, help.text, enabled);
    let fg = super::motion::colour(if enabled {
        if selected || hovered {
            studio_accent_lit()
        } else {
            studio_fg()
        }
    } else {
        studio_dim()
    });
    let press = super::motion::press(pressed);
    let mut modifier = Modifier::empty()
        .absolute_offset(area.x, area.y)
        .size_points(area.w, area.h)
        .content_description(label.clone())
        .semantics(move |s| s.selected = selected.then_some(true))
        .pointer_icon(control_cursor())
        .hoverable(source, true)
        .then(help_modifier)
        .then(design::surface(
            design::Surface::Icon,
            selected,
            hovered && enabled,
            pressed && enabled,
        ));
    if !enabled {
        modifier = modifier.semantics(|s| s.enabled = false);
    }
    Button(
        modifier,
        ButtonSpec::default().interaction_source(source),
        move || {
            if enabled {
                hints.set(None);
                click();
            }
        },
        move || {
            Canvas(
                Modifier::empty().size_points(24., 24.).then(press.clone()),
                move |scope| draw_icon(scope, icon, fg),
            );
            if caption {
                Text(
                    label.clone(),
                    Modifier::empty().padding(4.),
                    text_style(12., fg),
                );
            }
        },
    );
}

#[composable]
pub(super) fn PrimaryAction(icon: Icon, label: String, area: Area, action: impl Fn() + 'static) {
    let source = cranpose_ui::rememberMutableInteractionSource();
    let hovered = cranpose_ui::collect_is_hovered_as_state(&source).get();
    let pressed = cranpose_ui::collect_is_pressed_as_state(&source).get();
    Button(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .content_description(label.clone())
            .hoverable(source, true)
            .then(design::surface(
                design::Surface::Primary,
                false,
                hovered,
                pressed,
            ))
            .then(super::motion::feedback()),
        ButtonSpec::default().interaction_source(source),
        action,
        move || {
            let label = label.clone();
            Box(
                Modifier::empty().size_points(area.w, area.h),
                BoxSpec::default(),
                move || {
                    Canvas(
                        Modifier::empty()
                            .absolute_offset(12., (area.h - 24.) / 2.)
                            .size_points(24., 24.),
                        {
                            let color = Color::WHITE;
                            move |scope| draw_icon(scope, icon, color)
                        },
                    );
                    super::desktop::RowLabel(
                        label.clone(),
                        48.,
                        (area.h - 18.) / 2.,
                        area.w - 60.,
                        14.,
                        Color::WHITE,
                    );
                },
            );
        },
    );
}
#[composable]
pub(super) fn HintOverlay(hints: Hints, width: f32, height: f32) {
    let hint = hints.get();
    let visible = cranpose_core::rememberMutableStateOf(|| false);
    let key = hint.as_ref().map(|h| h.label.clone());
    cranpose_core::LaunchedEffectAsync(key, move |_| {
        Box::pin(async move {
            visible.set(false);
            cranpose_core::delay(Duration::from_millis(450)).await;
            visible.set(true);
        })
    });
    if let Some(hint) = hint {
        let (transition, retained) = super::motion::surface(visible.get(), 4.);
        if retained {
            let text = hint.text.clone();
            let w = (hint
                .text
                .lines()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0) as f32
                * 6.
                + 20.)
                .clamp(96., (width - 24.).min(360.));
            Box(
                Modifier::empty()
                    .absolute_offset(
                        hint.x.clamp(8., (width - w - 8.).max(8.)),
                        (hint.y - 64.).clamp(8., (height - 84.).max(8.)),
                    )
                    .width(w)
                    .then(design::surface(design::Surface::Panel, false, false, false))
                    .content_description("Control help")
                    .then(transition),
                BoxSpec::default(),
                move || {
                    Text(
                        text.clone(),
                        Modifier::empty().width(w).padding(10.),
                        text_style(12., studio_fg()),
                    );
                },
            );
        }
    }
}

#[composable]
pub(super) fn SwitchRow(label: String, area: Area, on: bool, action: impl Fn() + 'static) {
    let thumb = cranpose_animation::animateFloatAsState(
        if on { 1. } else { 0. },
        cranpose_animation::tween(140, cranpose_animation::Easing::EaseOut),
        "Studio switch",
    );
    let track = super::motion::colour(if on {
        studio_accent()
    } else {
        studio_pip_off()
    });
    Button(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .content_description(label.clone())
            .semantics(move |s| s.selected = Some(on))
            .then(super::motion::feedback()),
        ButtonSpec::default(),
        action,
        move || {
            let label = label.clone();
            Box(
                Modifier::empty().size_points(area.w, area.h),
                BoxSpec::default(),
                move || {
                    super::desktop::RowLabel(
                        label.clone(),
                        8.,
                        (area.h - 16.) / 2.,
                        area.w - 64.,
                        12.,
                        studio_fg(),
                    );
                    let thumb_color = Color::WHITE;
                    Canvas(
                        Modifier::empty()
                            .absolute_offset(area.w - 48., (area.h - 20.) / 2.)
                            .size_points(36., 20.),
                        move |scope| {
                            scope.draw_round_rect(
                                cranpose_ui_graphics::Brush::solid(track),
                                cranpose_ui_graphics::CornerRadii::uniform(10.),
                            );
                            scope.draw_circle(
                                cranpose_ui_graphics::Brush::solid(thumb_color),
                                cranpose_ui_graphics::Point::new(10. + 16. * thumb.get(), 10.),
                                7.,
                            );
                        },
                    );
                },
            );
        },
    );
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/icon_tests.rs"]
mod icon_tests;
