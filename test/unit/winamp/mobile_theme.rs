use super::*;
use crate::winamp::test_common::{pump, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use cranpose_services::SystemTheme;

#[test]
fn skin_contrast_uses_luminance_for_saturated_and_neutral_colors() {
    assert!(prefers_dark(Color::BLACK));
    assert!(!prefers_dark(Color::WHITE));
    assert!(prefers_dark(Color(0., 0., 1., 1.)));
    assert!(!prefers_dark(Color(0., 1., 0., 1.)));
    assert!(prefers_dark(Color(0.3, 0.3, 0.3, 1.)));
    assert!(!prefers_dark(Color(0.7, 0.7, 0.7, 1.)));
}

#[test]
fn theme_reacts_to_system_and_skin_changes_without_overriding_manual_choice() {
    let handles = Rc::new(Cell::new(None));
    let captured = handles.clone();
    let observed = Rc::new(RefCell::new(None));
    let drawn = observed.clone();
    let first = load_skin(BUNDLED_SKINS[0].bytes).unwrap();
    let corner = reflection::corner_color(&first);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        cranpose_core::location_key(file!(), line!(), column!()),
        move || {
            let mode = cranpose_core::rememberMutableStateOf(|| Mode::Auto);
            let system = cranpose_core::rememberMutableStateOf(|| SystemTheme::Light);
            let skin = cranpose_core::rememberMutableStateOf(|| Ok(first.clone()));
            let ui = cranpose_core::remember(|| Ui {
                mode,
                error: cranpose_core::mutableStateOf(String::new()),
                loaded: cranpose_core::mutableStateOf(true),
                revision: cranpose_core::mutableStateOf(0),
                attempted: cranpose_core::mutableStateOf(0),
            })
            .with(|ui| *ui);
            captured.set(Some((mode, system, skin)));
            let drawn = drawn.clone();
            cranpose_core::CompositionLocalProvider(
                [cranpose_services::local_system_theme().provides(system.get())],
                move || {
                    Theme(ui, skin, move || {
                        drawn.replace(Some(colors()));
                        Box(
                            Modifier::empty()
                                .fill_max_size()
                                .background(colors().background),
                            BoxSpec::default(),
                            || {},
                        );
                    });
                },
            );
        },
    );
    shell.set_viewport(320., 568.);
    pump(&mut shell);
    let (mode, system, skin) = handles.get().unwrap();
    let palette = || observed.borrow().unwrap();
    assert!(!palette().is_dark);
    shell.debug_enter_app_context(|| system.set(SystemTheme::Dark));
    pump(&mut shell);
    assert!(palette().is_dark);
    shell.debug_enter_app_context(|| mode.set(Mode::Day));
    pump(&mut shell);
    assert!(!palette().is_dark);
    shell.debug_enter_app_context(|| {
        system.set(SystemTheme::Light);
        mode.set(Mode::Night);
    });
    pump(&mut shell);
    assert!(palette().is_dark);
    shell.debug_enter_app_context(|| mode.set(Mode::Skin));
    pump(&mut shell);
    assert_eq!(palette().background, corner);
    assert_eq!(palette().is_dark, prefers_dark(corner));
    let second = load_skin(BUNDLED_SKINS[1].bytes).unwrap();
    let second_corner = reflection::corner_color(&second);
    shell.debug_enter_app_context(|| skin.set(Ok(second)));
    pump(&mut shell);
    assert_eq!(palette().background, second_corner);
    shell.debug_enter_app_context(|| skin.set(Err("unavailable skin".into())));
    pump(&mut shell);
    assert!(!palette().is_dark);
    assert_eq!(
        palette().background,
        LiquidColors::light(LiquidThemeSpec::default().accent).background
    );
}
