use super::*;

/// Keeps incomplete input while typing. External state changes replace the field.
#[composable]
pub(super) fn ExactField(
    label: &'static str,
    value: String,
    area: Area,
    change: impl Fn(String) -> bool + 'static,
) {
    let field = cranpose_core::remember(|| TextFieldState::new(value.clone())).with(|f| *f);
    let model = cranpose_core::rememberMutableStateOf(|| value.clone());
    let input = cranpose_core::rememberMutableStateOf(|| value.clone());
    let invalid = cranpose_core::rememberMutableStateOf(|| false);
    let text = field.text();
    if value != model.get() {
        cranpose_core::SideEffect(move || {
            field.set_text(value.clone());
            model.set(value.clone());
            input.set(value.clone());
            invalid.set(false);
        });
    } else if text != input.get() {
        cranpose_core::SideEffect(move || {
            invalid.set(!change(text.clone()));
            input.set(text.clone());
        });
    }
    let help = match label {
        "Canvas zoom" => "5 to 3200 percent",
        "HEX colour" | "Gradient colour" | "Font ink" | "Font background" | "Cutout colour" => {
            "Six HEX digits, for example #123456"
        }
        "Red channel" | "Green channel" | "Blue channel" => "A whole number from 0 to 255",
        "Brush size value" => "1 to 32 pixels",
        "Opacity value" => "1 to 100 percent",
        "Layer opacity value" => "0 to 100 percent",
        "Bend value" => "-100 to 100 percent",
        "Bevel value" => "1 to 128 pixels; 0 sizes the bevel from the shape",
        "Refraction value" => "0 to 32 pixels",
        "Repeat value" => "1 to 64 copies",
        "Text size value" => "1 to 8 times",
        "Spacing value" => "-2 to 8 pixels",
        _ => "Enter a whole number",
    };
    cranpose_ui::BasicTextField(
        field,
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .then(design::surface(
                if invalid.get() {
                    design::Surface::Invalid
                } else {
                    design::Surface::Field
                },
                false,
                false,
                false,
            ))
            .content_description(label)
            .then(controls::hint_feedback_colour(
                if invalid.get() {
                    format!("{help}. The previous value is unchanged.")
                } else {
                    help.into()
                },
                if invalid.get() {
                    design::ERROR
                } else {
                    ACCENT_LIT
                },
            ))
            .padding_symmetric(8., ((area.h - 18.) / 2.).max(0.)),
        text_style(13., FG),
    );
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Target {
    Brush,
    Playlist(&'static str),
    Visualizer(usize),
}
impl Target {
    fn read(self, doc: &Document) -> String {
        match self {
            Self::Brush => doc.view.color.clone(),
            Self::Playlist(key) => {
                doc.text_palettes()
                    .0
                    .into_iter()
                    .find(|v| v.0 == key)
                    .unwrap()
                    .1
            }
            Self::Visualizer(index) => doc.text_palettes().1[index].clone(),
        }
    }
    fn write(self, shared: &SharedDocument, tick: Tick, value: String) {
        if self == Self::Brush {
            update(shared, tick, json!({"color":value}));
            return;
        }
        let mut doc = shared.lock().unwrap();
        let result = match self {
            Self::Playlist(key) => doc.set_palette(&json!({key:value})),
            Self::Visualizer(index) => {
                let mut colours = doc.text_palettes().1;
                colours[index] = value;
                doc.visualizer_palette(&json!({"colors":colours}))
            }
            Self::Brush => unreachable!(),
        };
        if let Err(error) = result {
            doc.notify(error.to_string());
        }
        tick.set(doc.revision);
    }
}

#[composable]
pub(super) fn SkinColour(ui: Ui, area: Area, target: Target) {
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h),
        BoxSpec::default(),
        move || Body(ui.clone(), area.w, target),
    );
}

pub(super) fn height(count: usize, width: f32) -> f32 {
    let columns = ((width - 24.) / 32.).floor().max(1.) as usize;
    292. + count.div_ceil(columns) as f32 * 32.
}

#[composable]
pub(super) fn Editor(ui: Ui, area: Area) {
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h),
        BoxSpec::default(),
        move || Body(ui.clone(), area.w, Target::Brush),
    );
}

#[composable]
fn Body(ui: Ui, width: f32, target: Target) {
    let _ = ui.tick.get();
    let tick = ui.tick;
    let shared = ui.shared.clone();
    let view = shared.lock().unwrap().view.clone();
    let current = target.read(&shared.lock().unwrap());
    let rgb = parse_color(&current).unwrap_or([255; 4]);
    let (h0, s0, v0) = rgb_hsv([rgb[0], rgb[1], rgb[2]]);
    let hue = cranpose_core::rememberMutableStateOf(|| h0);
    if s0 > 0.02 && v0 > 0.02 && (hue.get_non_reactive() - h0).abs() > 0.001 {
        cranpose_core::SideEffect(move || hue.set(h0));
    }
    let h = hue.get();
    Label(
        if target == Target::Brush {
            "Colour"
        } else {
            "Edit colour"
        }
        .into(),
        12.,
        8.,
        width - 24.,
        13.,
        FG,
    );
    let field_size: (f32, f32) = (width - 24., 88.);
    const STRIP_H: f32 = 28.;
    let field = {
        let (fw, fh) = (68u32, 40u32);
        let mut im = image::RgbaImage::new(fw, fh);
        for y in 0..fh {
            for x in 0..fw {
                let c = hsv_rgb(
                    h,
                    x as f32 / (fw - 1) as f32,
                    1. - y as f32 / (fh - 1) as f32,
                );
                im.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], 255]));
            }
        }
        ImageBitmap::from_rgba8(fw, fh, im.into_raw()).expect("sv field")
    };
    let strip = {
        let sw = 120u32;
        let mut im = image::RgbaImage::new(sw, 1);
        for x in 0..sw {
            let c = hsv_rgb(x as f32 / (sw - 1) as f32, 1., 1.);
            im.put_pixel(x, 0, image::Rgba([c[0], c[1], c[2], 255]));
        }
        ImageBitmap::from_rgba8(sw, 1, im.into_raw()).expect("hue strip")
    };
    let d = shared.clone();
    Box(
        Modifier::empty()
            .absolute_offset(12., 32.)
            .size_points(field_size.0, field_size.1)
            .content_description("Saturation and brightness")
            .then(controls::hint_feedback(
                "Drag to choose saturation and brightness".into(),
            ))
            .pointer_input((h.to_bits(), target), move |scope: PointerInputScope| {
                let d = d.clone();
                async move {
                    scope
                        .await_pointer_event_scope(|events| async move {
                            loop {
                                let event = events.await_pointer_event().await;
                                let holding = event.kind == PointerEventKind::Down
                                    || (event.kind == PointerEventKind::Move
                                        && event.buttons.contains(PointerButton::Primary));
                                if !holding {
                                    continue;
                                }
                                let s = (event.position.x / field_size.0).clamp(0., 1.);
                                let v = 1. - (event.position.y / field_size.1).clamp(0., 1.);
                                let c = hsv_rgb(h, s, v);
                                target.write(
                                    &d,
                                    tick,
                                    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]),
                                );
                                event.consume();
                            }
                        })
                        .await;
                }
            }),
        BoxSpec::default(),
        move || {
            cranpose_ui::Image(
                cranpose_ui::BitmapPainter(field.clone()),
                None,
                Modifier::empty().required_size(cranpose_ui::Size::new(field_size.0, field_size.1)),
                Alignment::TOP_START,
                cranpose_ui::ContentScale::FillBounds,
                1.,
                None,
            );
            Box(
                Modifier::empty()
                    .absolute_offset(
                        (s0 * field_size.0 - 4.).clamp(0., field_size.0 - 8.),
                        ((1. - v0) * field_size.1 - 4.).clamp(0., field_size.1 - 8.),
                    )
                    .size_points(8., 8.)
                    .background(if v0 > 0.55 { BG } else { FG })
                    .rounded_corners(4.),
                BoxSpec::default(),
                || {},
            );
        },
    );
    let d = shared.clone();
    Box(
        Modifier::empty()
            .absolute_offset(12., 32. + field_size.1 + 4.)
            .size_points(field_size.0, STRIP_H)
            .content_description("Hue")
            .then(controls::hint_feedback("Drag to choose hue".into()))
            .pointer_input(target, move |scope: PointerInputScope| {
                let d = d.clone();
                async move {
                    scope
                        .await_pointer_event_scope(|events| async move {
                            loop {
                                let event = events.await_pointer_event().await;
                                let holding = event.kind == PointerEventKind::Down
                                    || (event.kind == PointerEventKind::Move
                                        && event.buttons.contains(PointerButton::Primary));
                                if !holding {
                                    continue;
                                }
                                let next = (event.position.x / field_size.0).clamp(0., 1.);
                                hue.set(next);
                                let (_, s, v) = {
                                    let doc = d.lock().unwrap();
                                    let c = parse_color(&target.read(&doc)).unwrap_or([255; 4]);
                                    rgb_hsv([c[0], c[1], c[2]])
                                };
                                let c = hsv_rgb(
                                    next,
                                    if s < 0.02 { 1. } else { s },
                                    if v < 0.02 { 1. } else { v },
                                );
                                target.write(
                                    &d,
                                    tick,
                                    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]),
                                );
                                event.consume();
                            }
                        })
                        .await;
                }
            }),
        BoxSpec::default(),
        move || {
            cranpose_ui::Image(
                cranpose_ui::BitmapPainter(strip.clone()),
                None,
                Modifier::empty()
                    .absolute_offset(0., 8.)
                    .required_size(cranpose_ui::Size::new(field_size.0, 12.)),
                Alignment::TOP_START,
                cranpose_ui::ContentScale::FillBounds,
                1.,
                None,
            );
            Box(
                Modifier::empty()
                    .absolute_offset((h * field_size.0 - 2.).clamp(0., field_size.0 - 4.), 6.)
                    .size_points(4., 16.)
                    .background(FG),
                BoxSpec::default(),
                || {},
            );
        },
    );

    Label("HEX".into(), 12., 168., 36., 11., DIM);
    let d = shared.clone();
    ExactField(
        "HEX colour",
        current.clone(),
        Area::new(48., 156., width - 100., 36.),
        move |text| {
            let value = text.trim().trim_start_matches('#');
            if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
                return false;
            }
            target.write(&d, tick, format!("#{value}"));
            true
        },
    );
    Box(
        Modifier::empty()
            .absolute_offset(width - 44., 156.)
            .size_points(32., 36.)
            .background(Color::from_rgba_u8(rgb[0], rgb[1], rgb[2], 255))
            .rounded_corners(4.)
            .content_description(format!("Current colour {current}")),
        BoxSpec::default(),
        || {},
    );
    let cell = (width - 24.) / 3.;
    for (i, label) in ["R", "G", "B"].into_iter().enumerate() {
        Label(label.into(), 12. + i as f32 * cell, 212., 16., 11., DIM);
        let d = shared.clone();
        ExactField(
            ["Red channel", "Green channel", "Blue channel"][i],
            rgb[i].to_string(),
            Area::new(28. + i as f32 * cell, 200., cell - 22., 36.),
            move |text| {
                let Ok(value) = text.trim().parse::<u8>() else {
                    return false;
                };
                let mut c = parse_color(&target.read(&d.lock().unwrap())).unwrap_or([255; 4]);
                c[i] = value;
                target.write(&d, tick, format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]));
                true
            },
        );
    }
    if target != Target::Brush {
        return;
    }
    Label("Saved colours".into(), 12., 254., width - 132., 12., FG);
    let selected = cranpose_core::rememberMutableStateOf(|| None::<usize>);
    let slot = selected.get().filter(|i| *i < view.swatches.len());
    for (i, (icon, label)) in [
        (Icon::Add, "Save colour"),
        (Icon::Brush, "Replace swatch"),
        (Icon::Close, "Remove swatch"),
    ]
    .into_iter()
    .enumerate()
    {
        let d = shared.clone();
        Control(
            icon,
            label.into(),
            match i {
                0 => "Add current colour to saved colours",
                1 => "Replace the selected swatch with current colour",
                _ => "Remove the selected swatch",
            }
            .into(),
            Area::new(width - 120. + i as f32 * 36., 240., 36., 36.),
            false,
            if i == 0 {
                view.swatches.len() < 24
            } else {
                slot.is_some()
            },
            false,
            ui.hints,
            move || {
                let v = d.lock().unwrap().view.clone();
                let mut colors = v.swatches;
                match i {
                    0 => {
                        if let Some(index) =
                            colors.iter().position(|c| c.eq_ignore_ascii_case(&v.color))
                        {
                            selected.set(Some(index));
                            return;
                        }
                        colors.push(v.color);
                        selected.set(Some(colors.len() - 1));
                    }
                    1 => {
                        if let Some(index) = slot {
                            if let Some(c) = colors.get_mut(index) {
                                *c = v.color;
                            }
                        }
                    }
                    _ => {
                        if let Some(index) = slot.filter(|n| *n < colors.len()) {
                            colors.remove(index);
                            selected.set(None);
                        }
                    }
                }
                update(&d, tick, json!({"swatches":colors}));
            },
        );
    }
    let columns = ((width - 24.) / 32.).floor().max(1.) as usize;
    let cell = (width - 24.) / columns as f32;
    for (i, value) in view.swatches.iter().enumerate() {
        let d = shared.clone();
        let value = value.clone();
        let paint = value.clone();
        let c = parse_color(&value).unwrap_or([255; 4]);
        let feedback = controls::hint_feedback(format!("Swatch {}: {value}", i + 1));
        Button(
            Modifier::empty()
                .absolute_offset(
                    12. + (i % columns) as f32 * cell,
                    284. + (i / columns) as f32 * 32.,
                )
                .size_points(cell, 32.)
                .content_description(format!("Swatch {}: {value}", i + 1))
                .semantics(move |s| s.selected = Some(slot == Some(i)))
                .then(design::surface(
                    design::Surface::Row,
                    slot == Some(i),
                    false,
                    false,
                ))
                .then(feedback),
            ButtonSpec::default(),
            move || {
                selected.set(Some(i));
                update(&d, tick, json!({"color":paint}));
            },
            move || {
                Box(
                    Modifier::empty()
                        .size_points(cell - 6., 24.)
                        .background(Color::from_rgba_u8(c[0], c[1], c[2], 255))
                        .rounded_corners(3.),
                    BoxSpec::default(),
                    || {},
                );
            },
        );
    }
}
