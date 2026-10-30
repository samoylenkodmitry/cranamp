use super::*;
use cranpose_ui_graphics::Point;

#[composable]
pub(super) fn Properties(ui: Ui, area: Area) {
    if ui.hand.get() {
        Navigation(ui, area);
        return;
    }
    let sample_w = if area.w >= 620. { 120. } else { 0. };
    if sample_w > 0. {
        previews::ToolSample(
            ui.clone(),
            Area::new(area.x + 8., area.y + 8., sample_w - 16., 48.),
        );
    }
    workbench::Properties(
        ui.clone(),
        Area::new(area.x + sample_w, area.y, area.w - sample_w - 48., 64.),
        false,
    );
    let next = ui.clone();
    Control(
        Icon::Settings,
        "Tool settings".into(),
        String::new(),
        Area::new(area.x + area.w - 48., area.y + 10., 44., 44.),
        false,
        true,
        false,
        ui.hints,
        move || next.open_sheet(Sheet::Advanced(4)),
    );
}

#[composable]
pub(super) fn Navigation(ui: Ui, area: Area) {
    let view = ui.viewport.get();
    let zoom = ui.viewport;
    Label(
        "Zoom".into(),
        area.x + 12.,
        area.y + 6.,
        70.,
        12.,
        studio_dim(),
    );
    colour::ExactField(
        "Canvas zoom",
        format!("{:.0}%", view.scale * 100.),
        Area::new(area.x + 8., area.y + 25., 72., 32.),
        move |value| {
            let Ok(percent) = value.trim().trim_end_matches('%').trim().parse::<f32>() else {
                return false;
            };
            if !(5. ..=3200.).contains(&percent) {
                return false;
            }
            zoom.update(|v| {
                v.transform(
                    Point::new(v.room[0] / 2., v.room[1] / 2.),
                    Point::ZERO,
                    percent / 100. / v.scale,
                )
            });
            true
        },
    );
    let grid = ui.shared.lock().unwrap().view.grid;
    let next = ui.clone();
    Control(
        Icon::Grid,
        "Pixel grid".into(),
        String::new(),
        Area::new(area.x + 88., area.y + 12., 44., 44.),
        grid,
        true,
        false,
        ui.hints,
        move || update(&next.shared, next.tick, json!({"grid":!grid})),
    );
}

#[composable]
pub(super) fn NavigationBar(ui: Ui, area: Area) {
    let zoom = ui.viewport;
    colour::ExactField(
        "Canvas zoom",
        format!("{:.0}%", zoom.get().scale * 100.),
        Area::new(area.x + 4., area.y + 5., 60., 34.),
        move |value| {
            let Ok(percent) = value.trim().trim_end_matches('%').trim().parse::<f32>() else {
                return false;
            };
            if !(5. ..=3200.).contains(&percent) {
                return false;
            }
            zoom.update(|v| {
                v.transform(
                    Point::new(v.room[0] / 2., v.room[1] / 2.),
                    Point::ZERO,
                    percent / 100. / v.scale,
                )
            });
            true
        },
    );
    let grid = ui.shared.lock().unwrap().view.grid;
    let next = ui.clone();
    Control(
        Icon::Grid,
        "Pixel grid".into(),
        String::new(),
        Area::new(area.x + 68., area.y, 44., 44.),
        grid,
        true,
        false,
        ui.hints,
        move || update(&next.shared, next.tick, json!({"grid":!grid})),
    );
}

#[composable]
pub(super) fn Palette(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let view = ui.shared.lock().unwrap().view.clone();
    let next = ui.clone();
    let current = view.color.clone();
    let c = parse_color(&current).unwrap_or([255; 4]);
    let feedback = controls::hint_feedback(format!("Edit colour {current}"));
    Button(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(40., 44.)
            .then(design::surface(design::Surface::Icon, false, false, false))
            .content_description("Brush colour")
            .pointer_icon(control_cursor())
            .then(feedback),
        ButtonSpec::default(),
        move || next.open_sheet(Sheet::Colour),
        move || {
            Box(
                Modifier::empty().size_points(40., 44.),
                BoxSpec::default(),
                move || {
                    cranpose_ui::Canvas(
                        Modifier::empty()
                            .absolute_offset(9., 6.)
                            .size_points(22., 22.),
                        {
                            let color = studio_fg();
                            move |scope| controls::draw_icon(scope, Icon::Colour, color)
                        },
                    );
                    Box(
                        Modifier::empty()
                            .absolute_offset(9., 32.)
                            .size_points(22., 4.)
                            .background(Color::from_rgba_u8(c[0], c[1], c[2], 255))
                            .rounded_corners(2.),
                        BoxSpec::default(),
                        || {},
                    );
                },
            );
        },
    );
    let count = ((area.w - 44.) / 32.).floor().max(0.) as usize;
    for (i, value) in view.swatches.into_iter().take(count).enumerate() {
        let next = ui.clone();
        let paint = value.clone();
        let selected = value.eq_ignore_ascii_case(&current);
        let c = parse_color(&value).unwrap_or([255; 4]);
        let feedback = controls::hint_feedback(value.clone());
        Button(
            Modifier::empty()
                .absolute_offset(area.x + 44. + i as f32 * 32., area.y)
                .size_points(32., 44.)
                .content_description(format!("Quick colour {value}"))
                .semantics(move |s| s.selected = Some(selected))
                .then(feedback),
            ButtonSpec::default(),
            move || update(&next.shared, next.tick, json!({"color":paint})),
            move || {
                Box(
                    Modifier::empty()
                        .size_points(26., 28.)
                        .background(if selected {
                            studio_accent_lit()
                        } else {
                            design::studio_pip_off()
                        })
                        .rounded_corners(4.),
                    BoxSpec::default(),
                    move || {
                        Box(
                            Modifier::empty()
                                .absolute_offset(1., 1.)
                                .size_points(24., 26.)
                                .background(Color::from_rgba_u8(c[0], c[1], c[2], 255))
                                .rounded_corners(2.),
                            BoxSpec::default(),
                            || {},
                        );
                    },
                );
            },
        );
    }
}
