use super::super::model::View;
use super::*;

fn primary_height(width: f32) -> f32 {
    if width < 360. {
        128.
    } else {
        64.
    }
}
fn extra_numbers(v: &View) -> Vec<(&'static str, &'static str, i32, [i32; 2], &'static str)> {
    match v.brush.as_str() {
        "curve" | "tuft" | "glass" => vec![("Opacity", "opacity", v.opacity as i32, [1, 255], "%")],
        "rect" | "ellipse" if !v.filled => {
            vec![("Opacity", "opacity", v.opacity as i32, [1, 255], "%")]
        }
        "text" => vec![
            ("Spacing", "text_spacing", v.text_spacing, [-2, 8], " px"),
            ("Opacity", "opacity", v.opacity as i32, [1, 255], "%"),
        ],
        _ => vec![],
    }
}
fn switches(v: &View) -> Vec<(&'static str, &'static str, bool)> {
    let mut rows = vec![
        ("Mirror horizontally", "mirror_x", v.mirror_x),
        ("Mirror vertically", "mirror_y", v.mirror_y),
    ];
    if v.paint_layer.is_some() {
        rows.push(("Lock transparency", "alpha_lock", v.alpha_lock));
    }
    if matches!(v.brush.as_str(), "pencil" | "line" | "curve" | "tuft")
        || (matches!(v.brush.as_str(), "rect" | "ellipse") && !v.filled)
    {
        rows.push(("Clean corners", "clean_corners", v.clean_corners));
    }
    rows
}
fn gradient(v: &View) -> bool {
    !matches!(v.brush.as_str(), "pencil" | "text" | "lift" | "stamp")
}
pub(super) fn height(v: &View, width: f32) -> f32 {
    let primary = if v.brush == "lift" {
        64.
    } else {
        primary_height(width)
    };
    if matches!(v.brush.as_str(), "lift" | "stamp") {
        return 64. + primary + if v.brush == "stamp" { 48. } else { 0. } + 12.;
    }
    let columns = if width >= 380. { 2usize } else { 1 };
    let number_rows = extra_numbers(v).len().div_ceil(columns);
    64. + primary
        + 8.
        + number_rows as f32 * 68.
        + if v.brush == "text" { 48. } else { 0. }
        + 24.
        + switches(v).len().div_ceil(columns) as f32 * 44.
        + 24.
        + if columns == 2 { 68. } else { 136. }
        + if gradient(v) {
            72. + if v.ramp_to.is_some() { 48. } else { 0. }
        } else {
            0.
        }
        + 56.
}

#[composable]
pub(super) fn ToolProperties(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let v = ui.shared.lock().unwrap().view.clone();
    let content_h = height(&v, area.w);
    ScrollArea(area, content_h, move || {
        let has_colour = workbench::uses_colour(&v.brush);
        previews::ToolSample(
            ui.clone(),
            Area::new(12., 0., if has_colour { 100. } else { area.w - 24. }, 56.),
        );
        if has_colour {
            quick::Palette(ui.clone(), Area::new(120., 6., area.w - 132., 44.));
        }
        let primary = if v.brush == "lift" {
            64.
        } else {
            primary_height(area.w)
        };
        workbench::Properties(
            ui.clone(),
            Area::new(4., 64., area.w - 8., primary),
            area.w < 360. && v.brush != "lift",
        );
        let mut y = 64. + primary + 8.;
        if matches!(v.brush.as_str(), "lift" | "stamp") {
            if v.brush == "stamp" {
                Switch(
                    ui.clone(),
                    "Sweep states",
                    "stamp_sweep",
                    v.stamp_sweep,
                    Area::new(4., y, area.w - 8., 44.),
                );
            }
            return;
        }
        let columns = if area.w >= 380. { 2usize } else { 1 };
        let cell = (area.w - 8.) / columns as f32;
        let extra = extra_numbers(&v);
        for (index, (label, key, value, range, suffix)) in extra.iter().copied().enumerate() {
            workbench::ValueSlider(
                ui.clone(),
                label,
                key,
                value,
                range,
                suffix,
                Area::new(
                    4. + (index % columns) as f32 * cell,
                    y + (index / columns) as f32 * 68.,
                    cell,
                    64.,
                ),
            );
        }
        y += extra.len().div_ceil(columns) as f32 * 68.;
        if v.brush == "text" {
            for (i, (key, name)) in [("5x7", "5 x 7 font"), ("small", "Small font")]
                .into_iter()
                .enumerate()
            {
                let next = ui.clone();
                desktop::ListRow(
                    None,
                    name.into(),
                    String::new(),
                    String::new(),
                    Area::new(
                        4. + i as f32 * (area.w - 8.) / 2.,
                        y,
                        (area.w - 8.) / 2.,
                        44.,
                    ),
                    v.face == key,
                    ui.hints,
                    move || update(&next.shared, next.tick, json!({"face":key})),
                );
            }
            y += 48.;
        }
        Label(
            "Stroke".into(),
            12.,
            y + 4.,
            area.w - 24.,
            11.,
            studio_dim(),
        );
        y += 24.;
        let toggles = switches(&v);
        for (index, (label, key, on)) in toggles.iter().copied().enumerate() {
            Switch(
                ui.clone(),
                label,
                key,
                on,
                Area::new(
                    4. + (index % columns) as f32 * cell,
                    y + (index / columns) as f32 * 44.,
                    cell,
                    44.,
                ),
            );
        }
        y += toggles.len().div_ceil(columns) as f32 * 44.;
        Label(
            "Texture".into(),
            12.,
            y + 4.,
            area.w - 24.,
            11.,
            studio_dim(),
        );
        y += 24.;
        for (i, (label, key, value, range, suffix)) in [
            ("Grain", "grain", v.grain, [0, 64], ""),
            ("Grain size", "grain_size", v.grain_size, [1, 16], " px"),
        ]
        .into_iter()
        .enumerate()
        {
            workbench::ValueSlider(
                ui.clone(),
                label,
                key,
                value as i32,
                range,
                suffix,
                Area::new(
                    4. + (i % columns) as f32 * cell,
                    y + (i / columns) as f32 * 68.,
                    cell,
                    64.,
                ),
            );
        }
        y += if columns == 2 { 68. } else { 136. };
        if gradient(&v) {
            Label(
                "Gradient".into(),
                12.,
                y + 4.,
                area.w - 24.,
                11.,
                studio_dim(),
            );
            for (i, (axis, name)) in [("", "Solid"), ("down", "Down"), ("across", "Across")]
                .into_iter()
                .enumerate()
            {
                let next = ui.clone();
                let to = v.ramp_to.clone().unwrap_or_else(|| "#ffffff".into());
                desktop::ListRow(
                    None,
                    name.into(),
                    String::new(),
                    String::new(),
                    Area::new(
                        4. + i as f32 * (area.w - 8.) / 3.,
                        y + 24.,
                        (area.w - 8.) / 3.,
                        44.,
                    ),
                    if axis.is_empty() {
                        v.ramp_to.is_none()
                    } else {
                        v.ramp_to.is_some() && v.ramp_axis == axis
                    },
                    ui.hints,
                    move || {
                        update(
                            &next.shared,
                            next.tick,
                            if axis.is_empty() {
                                json!({"ramp_to":null})
                            } else {
                                json!({"ramp_to":to,"ramp_axis":axis})
                            },
                        )
                    },
                );
            }
            y += 72.;
            if let Some(to) = &v.ramp_to {
                Label("To".into(), 12., y + 12., 32., 12., studio_dim());
                let next = ui.clone();
                colour::ExactField(
                    "Gradient colour",
                    to.clone(),
                    Area::new(44., y, area.w - 56., 40.),
                    move |text| {
                        let hex = text.trim().trim_start_matches('#');
                        if hex.len() != 6 || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
                            return false;
                        }
                        update(
                            &next.shared,
                            next.tick,
                            json!({"ramp_to":format!("#{hex}")}),
                        );
                        true
                    },
                );
                y += 48.;
            }
        }
        let next = ui.clone();
        let on = !v.mask_colors.is_empty();
        let current = v.color.clone();
        controls::SwitchRow(
            "Paint only this colour".into(),
            Area::new(4., y + 4., area.w - 8., 44.),
            on,
            move || {
                update(
                    &next.shared,
                    next.tick,
                    json!({"mask_colors":if on{vec![]}else{vec![current.clone()]}}),
                )
            },
        );
    });
}

#[composable]
pub(super) fn Switch(ui: Ui, label: &'static str, key: &'static str, on: bool, area: Area) {
    controls::SwitchRow(label.into(), area, on, move || {
        update(&ui.shared, ui.tick, json!({key:!on}))
    });
}

pub(super) fn essentials_height(v: &View, available: f32) -> f32 {
    if matches!(v.brush.as_str(), "lift" | "stamp") || available < 44. {
        return 0.;
    }
    let numbers = extra_numbers(v).len() as f32 * 68.;
    let number_h = numbers.min((available / 68.).floor() * 68.);
    let switch_h = ((available - number_h) / 44.).floor() * 44.;
    number_h + switch_h.min(switches(v).len() as f32 * 44.)
}

#[composable]
pub(super) fn Essentials(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let v = ui.shared.lock().unwrap().view.clone();
    let mut y = 0.;
    for (label, key, value, range, suffix) in extra_numbers(&v) {
        if y + 68. > area.h {
            return;
        }
        workbench::ValueSlider(
            ui.clone(),
            label,
            key,
            value,
            range,
            suffix,
            Area::new(area.x + 4., area.y + y, area.w - 8., 64.),
        );
        y += 68.;
    }
    for (label, key, on) in switches(&v) {
        if y + 44. > area.h {
            return;
        }
        Switch(
            ui.clone(),
            label,
            key,
            on,
            Area::new(area.x + 4., area.y + y, area.w - 8., 44.),
        );
        y += 44.;
    }
}
