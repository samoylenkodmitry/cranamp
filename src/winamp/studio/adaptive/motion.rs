use super::*;
use cranpose_animation::{animateColorAsState, animateFloatAsState, tween, Easing};
use cranpose_ui_graphics::{Brush, CompositingStrategy, CornerRadii, GraphicsLayer, Stroke};

#[track_caller]
pub(super) fn colour(target: Color) -> Color {
    animateColorAsState(target, tween(130, Easing::EaseOut), "Studio control colour").get()
}

#[track_caller]
pub(super) fn press(pressed: bool) -> Modifier {
    let scale = animateFloatAsState(
        if pressed { 0.90 } else { 1. },
        tween(if pressed { 70 } else { 140 }, Easing::EaseOut),
        "Studio press",
    );
    Modifier::empty().graphics_layer(move || GraphicsLayer {
        scale: scale.get(),
        ..Default::default()
    })
}

#[track_caller]
pub(super) fn surface(visible: bool, distance: f32) -> (Modifier, bool) {
    let progress = animateFloatAsState(
        if visible { 1. } else { 0. },
        tween(180, Easing::EaseOut),
        "Studio surface",
    );
    let retained = visible || progress.get() > 0.001;
    (
        Modifier::empty().graphics_layer(move || {
            let p = progress.get();
            GraphicsLayer {
                alpha: p,
                translation_y: (1. - p) * distance,
                compositing_strategy: CompositingStrategy::ModulateAlpha,
                ..Default::default()
            }
        }),
        retained,
    )
}

#[track_caller]
pub(super) fn entrance(key: u64, distance: f32) -> Modifier {
    cranpose_core::key(key, || {
        let entered = cranpose_core::rememberMutableStateOf(|| false);
        cranpose_core::LaunchedEffectAsync((), move |_| {
            Box::pin(async move {
                cranpose_core::delay(Duration::from_millis(16)).await;
                entered.set(true);
            })
        });
        surface(entered.get(), distance).0
    })
}

#[track_caller]
pub(super) fn feedback() -> Modifier {
    feedback_colour(studio_accent_lit())
}

#[track_caller]
pub(super) fn feedback_colour(tint: Color) -> Modifier {
    let hovered = cranpose_core::rememberMutableStateOf(|| false);
    let pressed = cranpose_core::rememberMutableStateOf(|| false);
    let focused = cranpose_core::rememberMutableStateOf(|| false);
    let strength = animateFloatAsState(
        if focused.get() {
            1.
        } else if pressed.get() {
            0.8
        } else if hovered.get() {
            0.5
        } else {
            0.
        },
        tween(110, Easing::EaseOut),
        "Studio control feedback",
    );
    Modifier::empty()
        .pointer_icon(control_cursor())
        .on_focus_changed(move |state| focused.set(state.is_focused()))
        .pointer_input(0u8, move |scope: PointerInputScope| async move {
            scope
                .await_pointer_event_scope(|events| async move {
                    loop {
                        let event = events.await_pointer_event().await;
                        match event.kind {
                            PointerEventKind::Enter => hovered.set(true),
                            PointerEventKind::Exit => {
                                hovered.set(false);
                                pressed.set(false);
                            }
                            PointerEventKind::Down => pressed.set(true),
                            PointerEventKind::Up | PointerEventKind::Cancel => pressed.set(false),
                            _ => {}
                        }
                    }
                })
                .await;
        })
        .draw_with_content(move |scope| {
            scope.draw_content();
            let p = strength.get();
            if p > 0.001 {
                scope.draw_round_rect_stroked(
                    Brush::solid(Color(tint.0, tint.1, tint.2, p)),
                    CornerRadii::uniform(design::CONTROL_RADIUS),
                    Stroke::new(1.5),
                );
            }
        })
}
