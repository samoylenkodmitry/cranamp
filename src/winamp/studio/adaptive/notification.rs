use super::*;
use cranpose::liquid::{
    Glass, LiquidModifierExt, LiquidShape, LiquidTheme, LiquidThemeSpec, SchemeMode,
};

#[composable]
pub(super) fn Notification(notice: Option<(u64, String)>, width: f32, height: f32) {
    let seen = cranpose_core::rememberMutableStateOf(|| notice.as_ref().map(|n| n.0));
    let visible = cranpose_core::rememberMutableStateOf(|| false);
    let key = notice.as_ref().map(|n| n.0);
    cranpose_core::LaunchedEffectAsync(key, move |_| {
        Box::pin(async move {
            if seen.get_non_reactive() == key {
                return;
            }
            seen.set(key);
            visible.set(key.is_some());
            cranpose_core::delay(Duration::from_secs(5)).await;
            visible.set(false);
        })
    });
    let (transition, retained) = motion::surface(visible.get(), 14.);
    let Some((_, message)) = notice.filter(|_| retained) else {
        return;
    };
    let w = (width - 24.).min(320.);
    let columns = ((w - 68.) / 6.).max(1.) as usize;
    let lines = message
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(columns))
        .sum::<usize>();
    let panel_h = (lines as f32 * 16. + 24.).clamp(56., 160.);
    let palette = design::palette_local().current();
    LiquidTheme(
        LiquidThemeSpec {
            scheme: if palette.is_some_and(|colors| !colors.is_dark) {
                SchemeMode::Light
            } else {
                SchemeMode::Dark
            },
            ..Default::default()
        },
        move || {
            Box(
                Modifier::empty()
                    .absolute_offset(width - w - 12., (height - panel_h - 12.).max(8.))
                    .size_points(w, panel_h)
                    .glass_effect(
                        Glass::regular()
                            .shape(LiquidShape::RoundedRect(10.))
                            .tint(palette.map_or(Color(0.06, 0.10, 0.14, 0.82), |colors| {
                                colors.surface_glass
                            }))
                            .blur_radius(12.),
                    )
                    .content_description("Studio notification")
                    .then(transition.clone()),
                BoxSpec::default(),
                move || {
                    Text(
                        message.clone(),
                        Modifier::empty().width(w - 44.).padding(12.),
                        text_style(12., studio_fg()),
                    );
                    Control(
                        Icon::Close,
                        "Dismiss notification".into(),
                        String::new(),
                        Area::new(w - 44., 6., 36., 44.),
                        false,
                        true,
                        false,
                        controls::hint_context().current().expect("Studio hints"),
                        move || visible.set(false),
                    );
                },
            );
        },
    );
}
