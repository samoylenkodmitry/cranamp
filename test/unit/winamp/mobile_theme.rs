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
    let corner = skin_corner_color(&first);
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        cranpose_core::location_key(file!(), line!(), column!()),
        move || {
            let mode = cranpose_core::rememberMutableStateOf(|| Mode::Auto);
            let system = cranpose_core::rememberMutableStateOf(|| SystemTheme::Light);
            let options = cranpose_core::rememberMutableStateOf(
                cranpose_services::AccessibilityOptions::default,
            );
            let skin = cranpose_core::rememberMutableStateOf(|| Ok(first.clone()));
            let ui = cranpose_core::remember(|| Ui {
                mode,
                error: cranpose_core::mutableStateOf(String::new()),
                loaded: cranpose_core::mutableStateOf(true),
                revision: cranpose_core::mutableStateOf(0),
                attempted: cranpose_core::mutableStateOf(0),
            })
            .with(|ui| *ui);
            captured.set(Some((mode, system, skin, options)));
            let drawn = drawn.clone();
            cranpose_core::CompositionLocalProvider(
                [
                    cranpose_services::local_system_theme().provides(system.get()),
                    cranpose_services::local_accessibility_options().provides(options.get()),
                ],
                move || {
                    Theme(ui, skin, move || {
                        assert_eq!(colors(), cranpose::liquid::liquid_colors());
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
    let (mode, system, skin, options) = handles.get().unwrap();
    let palette = || observed.borrow().unwrap();
    assert!(!palette().is_dark);
    let default_day = palette();
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
    assert_ne!(palette().accent, default_day.accent);
    let first_palette = palette();
    let second = load_skin(BUNDLED_SKINS[1].bytes).unwrap();
    let second_corner = skin_corner_color(&second);
    shell.debug_enter_app_context(|| skin.set(Ok(second)));
    pump(&mut shell);
    assert_eq!(palette().background, second_corner);
    assert_ne!(palette().surface, first_palette.surface);
    assert_ne!(palette().glass_tint, first_palette.glass_tint);
    assert_ne!(palette().accent, first_palette.accent);
    let normal_label_opacity = palette().secondary_label.a();
    shell.debug_enter_app_context(|| options.update(|options| options.increase_contrast = true));
    pump(&mut shell);
    assert!(palette().secondary_label.a() > normal_label_opacity);
    assert_eq!(palette().background, second_corner);
    shell.debug_enter_app_context(|| options.update(|options| options.invert_colors = true));
    pump(&mut shell);
    let inverted = Color(
        1. - second_corner.0,
        1. - second_corner.1,
        1. - second_corner.2,
        1.,
    );
    assert_eq!(palette().background, inverted);
    assert_eq!(palette().is_dark, prefers_dark(inverted));
    shell.debug_enter_app_context(|| options.set(Default::default()));
    shell.debug_enter_app_context(|| skin.set(Err("unavailable skin".into())));
    pump(&mut shell);
    assert!(!palette().is_dark);
    assert_eq!(palette().background, default_day.background);
}

#[test]
fn skin_palettes_keep_text_and_controls_readable_for_light_dark_and_saturated_skins() {
    for background in [
        Color::BLACK,
        Color::WHITE,
        Color(0.46, 0.46, 0.46, 1.),
        Color(1., 0., 0., 1.),
        Color(0., 1., 0., 1.),
        Color(0., 0., 1., 1.),
        Color(0.10, 0.27, 0.34, 1.),
        Color(0.97, 0.85, 0.75, 1.),
    ] {
        let colors = palette::from_background(background);
        assert_eq!(colors.background, background);
        for surface in [colors.background, colors.surface, colors.surface_pressed] {
            assert!(palette::contrast(colors.label, surface) >= 4.5);
            assert!(palette::contrast(colors.accent, surface) >= 4.5);
        }
        assert!(palette::contrast(colors.on_accent, colors.accent) >= 4.5);
        assert_eq!(colors.toggle_on, colors.accent);
        assert_eq!(colors.surface_glass.with_alpha(1.), colors.surface);
        assert!(colors.surface_glass.3 < 1.);
    }
}
