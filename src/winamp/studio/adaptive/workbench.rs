use super::*;

pub(super) const BRUSHES: [(&str, &str, Icon); 8] = [
    ("pencil", "Pencil", Icon::Brush),
    ("line", "Line", Icon::Line),
    ("rect", "Rectangle", Icon::Rectangle),
    ("ellipse", "Ellipse", Icon::Ellipse),
    ("lift", "Lift pixels", Icon::Select),
    ("text", "Text", Icon::Text),
    ("stamp", "Stamp", Icon::Stamp),
    ("curve", "Curve", Icon::Curve),
];

pub(super) fn brush_icon(tool: &str) -> Icon {
    BRUSHES
        .into_iter()
        .chain([
            ("glass", "Glass lens", Icon::Glass),
            ("tuft", "Taper", Icon::Taper),
        ])
        .find(|b| b.0 == tool)
        .map(|b| b.2)
        .unwrap_or(Icon::Brush)
}
pub(super) fn uses_colour(tool: &str) -> bool {
    !matches!(tool, "lift" | "stamp")
}

fn tool_hint(key: &str) -> &'static str {
    match key {
        "pencil" => "Drag to draw a continuous stroke",
        "line" => "Drag from the start to the end of a line",
        "rect" | "ellipse" => "Drag between opposite corners of the shape",
        "lift" => "Drag a rectangle to copy its pixels",
        "stamp" => "Drag to place the copied pixels",
        "text" => "Drag to place the text from the properties",
        "curve" => "Drag the endpoints; Bend controls the curve",
        "pan" => "Drag to move the canvas without painting",
        "colour" => "Enter an exact colour or edit saved swatches",
        "layers" => "Choose the destination for new paint",
        _ => "",
    }
}

fn visible_brush(index: usize, count: usize, active: &str) -> (&'static str, &'static str, Icon) {
    if index == count - 1 && !BRUSHES[..count].iter().any(|b| b.0 == active) {
        BRUSHES
            .into_iter()
            .chain([
                ("glass", "Glass lens", Icon::Glass),
                ("tuft", "Taper", Icon::Taper),
            ])
            .find(|b| b.0 == active)
            .unwrap_or(BRUSHES[index])
    } else {
        BRUSHES[index]
    }
}

pub(super) fn tool_items(layout: Layout, active: &str) -> Vec<(&'static str, &'static str, Icon)> {
    let extras = if layout.inspector.is_some() { 2 } else { 3 };
    let capacity = (if layout.side_tools {
        layout.tools.h
    } else {
        layout.tools.w
    } / 44.) as usize;
    let preferred = if layout.side_tools {
        8
    } else if layout.class == WidthClass::SmallPhone {
        2
    } else {
        4
    };
    let count = preferred.min(capacity.saturating_sub(extras).max(1));
    let mut items: Vec<_> = (0..count)
        .map(|i| visible_brush(i, count, active))
        .collect();
    if layout.inspector.is_none() {
        items.push(("layers", "Painting layers", Icon::Layers));
    }
    items.extend([
        ("pan", "Pan canvas", Icon::Hand),
        ("tools", "Drawing tools", Icon::More),
    ]);
    items
}

#[composable]
pub(super) fn Tools(ui: Ui, layout: Layout) {
    let _ = ui.tick.get();
    let view = ui.shared.lock().unwrap().view.clone();
    let items = tool_items(layout, &view.brush);
    let step = if layout.side_tools {
        44.
    } else {
        layout.tools.w / items.len() as f32
    };
    ScrollArea(
        layout.tools,
        if layout.side_tools {
            items.len() as f32 * step
        } else {
            layout.tools.h
        },
        move || {
            for (i, (key, name, icon)) in items.iter().copied().enumerate() {
                let button = if layout.side_tools {
                    Area::new(4., i as f32 * step, 44., 44.)
                } else {
                    Area::new(i as f32 * step + (step - 44.) / 2., 2., 44., 44.)
                };
                let next = ui.clone();
                Control(
                    icon,
                    name.into(),
                    tool_hint(key).into(),
                    button,
                    if key == "pan" {
                        ui.hand.get()
                    } else {
                        !ui.hand.get() && view.brush == key
                    },
                    true,
                    false,
                    ui.hints,
                    move || match key {
                        "colour" => next.open_sheet(Sheet::Colour),
                        "layers" => next.open_sheet(Sheet::Layers),
                        "pan" => next.hand.set(!next.hand.get_non_reactive()),
                        "tools" => next.open_sheet(Sheet::Tools),
                        _ => {
                            next.hand.set(false);
                            update(&next.shared, next.tick, json!({"brush":key}));
                            next.open_sheet(Sheet::None);
                        }
                    },
                );
                if key == "colour" {
                    ColourMark(
                        view.color.clone(),
                        Area::new(button.x + 29., button.y + 29., 10., 10.),
                    );
                }
            }
        },
    );
}

#[composable]
fn ColourMark(value: String, area: Area) {
    let color = parse_color(&value).unwrap_or([255, 255, 255, 255]);
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .background(Color(
                color[0] as f32 / 255.,
                color[1] as f32 / 255.,
                color[2] as f32 / 255.,
                1.,
            ))
            .rounded_corners(2.),
        BoxSpec::default(),
        || {},
    );
}

pub(super) fn switch_scope(ui: &Ui, screen: Screen) {
    if screen == Screen::Canvas {
        state(
            &ui.shared,
            json!({"panel":"canvas","layer":"auto","layers":[],"presentation":false}),
        );
    } else if matches!(screen, Screen::Sprites | Screen::States) {
        let id = ui.sprite_id.get_non_reactive();
        let layer = ui
            .shared
            .lock()
            .unwrap()
            .skin_layers()
            .into_iter()
            .find(|l| l.id == id);
        if let Some(layer) = layer {
            state(
                &ui.shared,
                json!({"panel":"canvas","sheet":layer.sheet,"layer":id,"layers":[id],"presentation":false}),
            );
            ui.sprite_crop
                .set((screen == Screen::Sprites).then_some(layer.destination));
        }
    }
    ui.fit_requested.set(true);
    ui.show(screen);
}

#[composable]
pub(super) fn ContextBar(ui: Ui, layout: Layout) {
    let _ = ui.tick.get();
    let area = layout.context;
    let screen = ui.screen.get();
    let desktop = layout.class == WidthClass::Desktop;
    if desktop {
        if screen != Screen::States {
            Label(
                "Canvas state".into(),
                area.x + 8.,
                area.y + 16.,
                100.,
                12.,
                studio_dim(),
            );
            if layout.references.is_none() {
                CanvasToggles(
                    ui.clone(),
                    Area::new(area.x + 112., area.y, (area.w - 216.).min(300.), 48.),
                );
            } else {
                let view = ui.shared.lock().unwrap().view.clone();
                desktop::RowLabel(
                    format!(
                        "{} / {}",
                        if view.active { "Active" } else { "Inactive" },
                        if view.pressed { "Pressed" } else { "Released" }
                    ),
                    area.x + 112.,
                    area.y + 16.,
                    220.,
                    12.,
                    studio_fg(),
                );
            }
            let next = ui.clone();
            Control(
                Icon::State,
                "Canvas frames".into(),
                "Slider positions, digits and paint scope".into(),
                Area::new(area.x + area.w - 92., area.y + 2., 44., 44.),
                false,
                true,
                false,
                ui.hints,
                move || next.open_sheet(Sheet::CanvasState),
            );
        } else {
            Label(
                "Compare sprite states".into(),
                area.x + 8.,
                area.y + 16.,
                area.w - 60.,
                12.,
                studio_dim(),
            );
        }
    } else {
        let panel = ui.shared.lock().unwrap().view.panel.clone();
        let label = if screen == Screen::Canvas {
            part_name(&panel).into()
        } else {
            "Sprite targets".into()
        };
        let next = ui.clone();
        desktop::ListRow(
            Some(Icon::Parts),
            label,
            "Choose what to edit".into(),
            String::new(),
            Area::new(
                area.x,
                area.y + 2.,
                area.w - if screen == Screen::States { 44. } else { 88. },
                44.,
            ),
            false,
            ui.hints,
            move || {
                next.open_sheet(if screen == Screen::Canvas {
                    Sheet::Parts
                } else {
                    Sheet::Targets
                })
            },
        );
        if screen != Screen::States {
            let next = ui.clone();
            let view = ui.shared.lock().unwrap().view.clone();
            Control(
                if screen == Screen::States {
                    Icon::Brush
                } else {
                    Icon::State
                },
                if screen == Screen::States {
                    "Edit sprite"
                } else {
                    "Canvas state"
                }
                .into(),
                format!(
                    "{} / {}",
                    if view.active { "Active" } else { "Inactive" },
                    if view.pressed { "Pressed" } else { "Released" }
                ),
                Area::new(area.x + area.w - 88., area.y + 2., 44., 44.),
                false,
                true,
                false,
                ui.hints,
                move || {
                    if screen == Screen::States {
                        switch_scope(&next, Screen::Sprites);
                    } else {
                        next.open_sheet(Sheet::CanvasState);
                    }
                },
            );
        }
    }
    let next = ui.clone();
    Control(
        Icon::Save,
        "Save and share".into(),
        "Save project or export skin".into(),
        Area::new(area.x + area.w - 48., area.y + 2., 44., 44.),
        false,
        true,
        false,
        ui.hints,
        move || next.show(Screen::Save),
    );
}

#[composable]
pub(super) fn CanvasToggles(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let tick = ui.tick;
    let v = ui.shared.lock().unwrap().view.clone();
    for (i, (key, name, on)) in [
        (
            "active",
            if v.active { "Active" } else { "Inactive" },
            v.active,
        ),
        (
            "pressed",
            if v.pressed { "Pressed" } else { "Released" },
            v.pressed,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let d = ui.shared.clone();
        desktop::ListRow(
            None,
            name.into(),
            if key == "active" {
                "Switch active / inactive artwork"
            } else {
                "Switch released / pressed artwork"
            }
            .into(),
            String::new(),
            Area::new(
                area.x + i as f32 * area.w / 2.,
                area.y + 2.,
                area.w / 2. - 4.,
                44.,
            ),
            on,
            ui.hints,
            move || update(&d, tick, json!({key:!on})),
        );
    }
}

#[composable]
pub(super) fn CanvasSettings(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let tick = ui.tick;
    let v = ui.shared.lock().unwrap().view.clone();
    if v.panel == "cursors" {
        skin::CursorCanvas(ui, area);
        return;
    }
    let desktop = ui.layout.get().class == WidthClass::Desktop;
    ScrollArea(area, 600., move || {
        let mut y = 0.;
        if !desktop {
            CanvasToggles(ui.clone(), Area::new(8., y, area.w - 16., 48.));
            y += 56.;
        }
        Label(
            "Paint into".into(),
            12.,
            y + 8.,
            area.w - 24.,
            12.,
            studio_dim(),
        );
        for (i, (key, label)) in [
            ("current", "This state"),
            ("all", "All states"),
            ("onward", "This and later"),
            ("up-to", "This and earlier"),
        ]
        .into_iter()
        .enumerate()
        {
            let d = ui.shared.clone();
            desktop::ListRow(
                None,
                label.into(),
                model::scope_label(key).into(),
                String::new(),
                Area::new(
                    (i % 2) as f32 * area.w / 2.,
                    y + 28. + (i / 2) as f32 * 44.,
                    area.w / 2.,
                    44.,
                ),
                v.states == key,
                ui.hints,
                move || update(&d, tick, json!({"states":key})),
            );
        }
        y += 124.;
        for (i, (name, key, val, max)) in [
            ("Volume frame", "volume", v.volume, 27),
            ("Balance frame", "balance", v.balance, 27),
            ("Position frame", "position", v.position, 27),
            ("Scroll frame", "scroll", v.scroll, 27),
            ("Digit", "digit", v.digit, 9),
        ]
        .into_iter()
        .enumerate()
        {
            ValueSlider(
                ui.clone(),
                name,
                key,
                val as i32,
                [0, max],
                "",
                Area::new(4., y + i as f32 * 64., area.w - 8., 64.),
            );
        }
        Label(
            "Playback".into(),
            12.,
            y + 328.,
            area.w - 24.,
            12.,
            studio_dim(),
        );
        for (i, name) in ["Stopped", "Playing", "Paused"].into_iter().enumerate() {
            let d = ui.shared.clone();
            desktop::ListRow(
                None,
                name.into(),
                format!("Show {name} artwork"),
                String::new(),
                Area::new(i as f32 * area.w / 3., y + 348., area.w / 3., 44.),
                v.playback as usize == i,
                ui.hints,
                move || update(&d, tick, json!({"playback":i})),
            );
        }
    });
}

#[composable]
pub(super) fn PropertyHeading(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let brush = ui.shared.lock().unwrap().view.brush.clone();
    let name = BRUSHES
        .into_iter()
        .chain([
            ("glass", "Glass lens", Icon::Glass),
            ("tuft", "Taper", Icon::Taper),
        ])
        .find(|b| b.0 == brush)
        .map(|b| b.1)
        .unwrap_or("Tool");
    desktop::RowLabel(
        if area.w < 220. {
            name.into()
        } else {
            format!("{name} properties")
        },
        area.x + 8.,
        area.y + 14.,
        area.w - 60.,
        14.,
        studio_fg(),
    );
    let next = ui.clone();
    Control(
        Icon::Settings,
        "Tool settings".into(),
        String::new(),
        Area::new(area.x + area.w - 48., area.y, 44., 44.),
        false,
        true,
        false,
        ui.hints,
        move || next.open_sheet(Sheet::Advanced(4)),
    );
}

#[allow(clippy::too_many_arguments)]
#[composable]
pub(super) fn ValueSlider(
    ui: Ui,
    label: &'static str,
    key: &'static str,
    value: i32,
    range: [i32; 2],
    suffix: &'static str,
    area: Area,
) {
    let tick = ui.tick;
    let displayed = if key == "opacity" {
        (value as f32 / 255. * 100.).round() as i32
    } else {
        value
    };
    let displayed = if key == "opacity" {
        displayed.max(1)
    } else {
        displayed
    };
    let compact = area.w < 120.;
    let visible = match label {
        "Brush size" => "Size",
        "Text size" => "Scale",
        _ => label,
    };
    desktop::RowLabel(
        visible.into(),
        area.x + 8.,
        area.y + if compact { 0. } else { 9. },
        if compact {
            area.w - 16.
        } else {
            (area.w - 76.).max(28.)
        },
        12.,
        studio_fg(),
    );
    let _ = suffix;
    let d = ui.shared.clone();
    colour::ExactField(
        match key {
            "brush_size" => "Brush size value",
            "opacity" => "Opacity value",
            "curve_bend" => "Bend value",
            "text_scale" => "Text size value",
            "stamp_repeat" => "Repeat value",
            "bevel" => "Bevel value",
            "refraction" => "Refraction value",
            "grain" => "Grain value",
            "grain_size" => "Grain size value",
            "text_spacing" => "Spacing value",
            "volume" => "Volume frame value",
            "balance" => "Balance frame value",
            "position" => "Position frame value",
            "scroll" => "Scroll frame value",
            "digit" => "Digit value",
            _ => "Playback value",
        },
        displayed.to_string(),
        if compact {
            Area::new(area.x + 8., area.y + 16., area.w - 16., 24.)
        } else {
            Area::new(area.x + area.w - 60., area.y, 52., 32.)
        },
        move |text| {
            let Ok(mut v) = text.trim().parse::<i32>() else {
                return false;
            };
            if key == "opacity" {
                if !(1..=100).contains(&v) {
                    return false;
                }
                v = (v as f32 * 255. / 100.).round() as i32;
            }
            if !(range[0]..=range[1]).contains(&v) {
                return false;
            }
            update(&d, tick, json!({key:v}));
            true
        },
    );
    let feedback = controls::feedback(format!("Adjust {label}"));
    let width = area.w - 16.;
    let shared = ui.shared;
    cranpose_ui::Slider(
        Modifier::empty()
            .absolute_offset(area.x + 8., area.y + if compact { 40. } else { 32. })
            .size_points(width, 24.)
            .content_description(label)
            .then(feedback),
        (value - range[0]) as f32 / (range[1] - range[0]) as f32,
        move |v| {
            update(
                &shared,
                tick,
                json!({key:range[0] + (v * (range[1] - range[0]) as f32).round() as i32}),
            )
        },
        || {},
        cranpose_ui::SliderSpec::default().thumb_extent(16.),
        move |scope| {
            design::SliderTrack(width, 24., scope.thumb_offset());
        },
    );
}

#[composable]
pub(super) fn Properties(ui: Ui, area: Area, vertical: bool) {
    let _ = ui.tick.get();
    let tick = ui.tick;
    let view = ui.shared.lock().unwrap().view.clone();
    let first = Area::new(
        area.x,
        area.y,
        if vertical { area.w } else { area.w / 2. },
        64.,
    );
    let second = if vertical {
        Area::new(area.x, area.y + 64., area.w, 64.)
    } else {
        Area::new(area.x + area.w / 2., area.y, area.w / 2., 64.)
    };
    if view.brush == "lift" {
        let selection = ui.shared.lock().unwrap().selection;
        Label(
            selection
                .map(|r| format!("{} x {} pixels", r[2], r[3]))
                .unwrap_or_else(|| "Drag to copy pixels".into()),
            first.x + 8.,
            first.y + 16.,
            area.w - 68.,
            13.,
            studio_fg(),
        );
        let next = ui.clone();
        Control(
            Icon::Stamp,
            "Use copied pixels".into(),
            "Switch to Stamp; the original stays in place".into(),
            Area::new(area.x + area.w - 52., area.y + 8., 44., 44.),
            false,
            ui.shared.lock().unwrap().cluster.is_some(),
            false,
            ui.hints,
            move || update(&next.shared, next.tick, json!({"brush":"stamp"})),
        );
        return;
    }
    if view.brush == "stamp" {
        ValueSlider(
            ui.clone(),
            "Repeat",
            "stamp_repeat",
            view.stamp_repeat as i32,
            [1, 64],
            "",
            first,
        );
        let captured = ui.shared.lock().unwrap().cluster.is_some();
        for (i, (icon, label)) in [
            (Icon::FlipH, "Flip copied pixels horizontally"),
            (Icon::FlipV, "Flip copied pixels vertically"),
            (Icon::Rotate, "Rotate copied pixels"),
        ]
        .into_iter()
        .enumerate()
        {
            let next = ui.clone();
            Control(
                icon,
                label.into(),
                "Changes the copy for the next stamp".into(),
                Area::new(
                    second.x + 4. + i as f32 * ((second.w - 8.) / 3.).min(44.),
                    second.y + 8.,
                    ((second.w - 8.) / 3.).min(44.),
                    44.,
                ),
                false,
                captured,
                false,
                ui.hints,
                move || {
                    let mut d = next.shared.lock().unwrap();
                    if let Err(e) = d.transform_cluster(i == 0, i == 1, u32::from(i == 2)) {
                        d.notify(e.to_string());
                        d.revision += 1;
                    }
                    next.tick.set(d.revision);
                },
            );
        }
        return;
    }
    if view.brush == "text" {
        let text = cranpose_core::remember(|| TextFieldState::new(view.text.clone())).with(|f| *f);
        let applied = cranpose_core::rememberMutableStateOf(|| view.text.clone());
        let value = text.text();
        let shared = ui.shared.clone();
        if value != applied.get() {
            cranpose_core::SideEffect(move || {
                update(&shared, tick, json!({"text":value}));
                applied.set(value.clone());
            });
        } else if view.text != value {
            let value = view.text.clone();
            cranpose_core::SideEffect(move || {
                text.set_text(value.clone());
                applied.set(value.clone());
            });
        }
        Label(
            "Text".into(),
            first.x + 8.,
            first.y + 4.,
            first.w - 16.,
            12.,
            studio_dim(),
        );
        let feedback = controls::feedback("Text to paint".into());
        cranpose_ui::BasicTextField(
            text,
            Modifier::empty()
                .absolute_offset(first.x + 8., first.y + 24.)
                .size_points(first.w - 16., 40.)
                .then(design::surface(design::Surface::Field, false, false, false))
                .content_description("Text to paint")
                .then(feedback)
                .padding_symmetric(8., 6.),
            text_style(14., studio_fg()),
        );
        ValueSlider(
            ui,
            "Text size",
            "text_scale",
            view.text_scale as i32,
            [1, 8],
            "x",
            second,
        );
    } else {
        match view.brush.as_str() {
            "rect" | "ellipse" if view.filled => ValueSlider(
                ui.clone(),
                "Opacity",
                "opacity",
                view.opacity as i32,
                [1, 255],
                "%",
                first,
            ),
            "curve" | "tuft" => ValueSlider(
                ui.clone(),
                "Brush size",
                "brush_size",
                view.brush_size as i32,
                [1, 32],
                " px",
                first,
            ),
            "stamp" => ValueSlider(
                ui.clone(),
                "Repeat",
                "stamp_repeat",
                view.stamp_repeat as i32,
                [1, 64],
                "",
                first,
            ),
            "glass" => ValueSlider(
                ui.clone(),
                "Bevel",
                "bevel",
                view.bevel as i32,
                [0, 128],
                " px",
                first,
            ),
            _ => ValueSlider(
                ui.clone(),
                "Brush size",
                "brush_size",
                view.brush_size as i32,
                [1, 32],
                " px",
                first,
            ),
        }
        if matches!(view.brush.as_str(), "curve" | "tuft") {
            ValueSlider(
                ui,
                "Bend",
                "curve_bend",
                view.curve_bend,
                [-100, 100],
                "%",
                second,
            );
        } else if view.brush == "glass" {
            ValueSlider(
                ui,
                "Refraction",
                "refraction",
                view.refraction as i32,
                [0, 32],
                " px",
                second,
            );
        } else if matches!(view.brush.as_str(), "rect" | "ellipse") {
            let d = ui.shared.clone();
            desktop::ListRow(
                Some(Icon::Fill),
                "Fill shape".into(),
                "Fill or outline".into(),
                String::new(),
                Area::new(second.x + 4., second.y + 16., second.w - 8., 44.),
                view.filled,
                ui.hints,
                move || update(&d, tick, json!({"filled":!view.filled})),
            );
        } else {
            ValueSlider(
                ui,
                "Opacity",
                "opacity",
                view.opacity as i32,
                [1, 255],
                "%",
                second,
            );
        }
    }
}

#[composable]
pub(super) fn FullProperties(ui: Ui, area: Area) {
    properties::ToolProperties(ui, area);
}

#[composable]
fn Toggle(ui: Ui, label: &'static str, key: &'static str, on: bool, area: Area) {
    properties::Switch(ui, label, key, on, area);
}
