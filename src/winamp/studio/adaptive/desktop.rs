use super::*;

#[composable]
pub(super) fn ScopeSelector(ui: Ui, layout: Layout) {
    let desktop = layout.class == WidthClass::Desktop;
    if !desktop && layout.height < 500. {
        let next = ui.clone();
        Control(
            Icon::Canvas,
            "Workspaces".into(),
            "Canvas, sprites, states, skin settings and preview".into(),
            Area::new(56., 6., 44., 44.),
            false,
            true,
            false,
            ui.hints,
            move || next.open_sheet(Sheet::Scopes),
        );
        return;
    }
    let x = if desktop { 312. } else { 8. };
    let y = if desktop { 10. } else { 56. };
    let width = if desktop { 520. } else { layout.width - 16. };
    for (i, (screen, icon, name)) in [
        (Screen::Canvas, Icon::Canvas, "Canvas"),
        (Screen::Sprites, Icon::Parts, "Sprites"),
        (Screen::States, Icon::State, "States"),
        (Screen::Skin, Icon::Settings, "Skin"),
        (Screen::Preview, Icon::Eye, "Preview"),
    ]
    .into_iter()
    .enumerate()
    {
        let next = ui.clone();
        let area = Area::new(x + i as f32 * width / 5., y, width / 5., 44.);
        if desktop || layout.width >= 600. {
            ListRow(
                Some(icon),
                name.into(),
                String::new(),
                String::new(),
                area,
                ui.screen.get() == screen,
                ui.hints,
                move || workbench::switch_scope(&next, screen),
            );
        } else {
            Control(
                icon,
                name.into(),
                String::new(),
                area,
                ui.screen.get() == screen,
                true,
                false,
                ui.hints,
                move || workbench::switch_scope(&next, screen),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[composable]
pub(super) fn ListRow(
    icon: Option<Icon>,
    label: String,
    hint: String,
    metadata: String,
    area: Area,
    selected: bool,
    hints: Hints,
    action: impl Fn() + 'static,
) {
    let source = cranpose_ui::rememberMutableInteractionSource();
    let hovered = cranpose_ui::collect_is_hovered_as_state(&source).get();
    let help = Hint {
        label: label.clone(),
        text: hint,
        x: area.x,
        y: area.y,
    };
    cranpose_core::LaunchedEffect((hovered, label.clone()), move |_| {
        if hovered && !help.text.is_empty() && help.text != help.label {
            hints.set(Some(help.clone()));
        } else if hints
            .get_non_reactive()
            .as_ref()
            .is_some_and(|h| h.label == help.label)
        {
            hints.set(None);
        }
    });
    let fg = motion::colour(if selected || hovered { ACCENT_LIT } else { FG });
    Button(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .content_description(label.clone())
            .semantics(move |s| s.selected = selected.then_some(true))
            .pointer_icon(control_cursor())
            .hoverable(source, true)
            .then(design::surface(
                design::Surface::Row,
                selected,
                hovered,
                false,
            ))
            .then(motion::feedback()),
        ButtonSpec::default().interaction_source(source),
        move || {
            hints.set(None);
            action();
        },
        move || {
            Box(
                Modifier::empty().size_points(area.w, area.h),
                BoxSpec::default(),
                {
                    let label = label.clone();
                    let metadata = metadata.clone();
                    move || {
                        if let Some(icon) = icon {
                            cranpose_ui::Canvas(
                                Modifier::empty()
                                    .absolute_offset(8., (area.h - 24.) / 2.)
                                    .size_points(24., 24.),
                                move |scope| controls::draw_icon(scope, icon, fg),
                            );
                        }
                        let left = if icon.is_some() { 40. } else { 12. };
                        let meta_w = if metadata.is_empty() { 0. } else { 78. };
                        RowLabel(
                            label
                                .strip_prefix("Choose a part: ")
                                .or_else(|| {
                                    icon.is_none()
                                        .then(|| label.strip_prefix("Sprite "))
                                        .flatten()
                                })
                                .or_else(|| {
                                    icon.is_none()
                                        .then(|| label.strip_prefix("Edit "))
                                        .flatten()
                                })
                                .unwrap_or(&label)
                                .into(),
                            left,
                            (area.h - 16.) / 2.,
                            area.w - left - meta_w - 8.,
                            12.,
                            fg,
                        );
                        if !metadata.is_empty() {
                            RowLabel(
                                metadata.clone(),
                                area.w - meta_w,
                                (area.h - 14.) / 2.,
                                meta_w - 4.,
                                11.,
                                DIM,
                            );
                        }
                    }
                },
            );
        },
    );
}

#[composable]
pub(super) fn RowLabel(text: String, x: f32, y: f32, width: f32, size: f32, color: Color) {
    cranpose_ui::BasicText(
        text,
        Modifier::empty().absolute_offset(x, y).width(width.max(1.)),
        text_style(size, color),
        cranpose_ui::text::TextOverflow::Ellipsis,
        false,
        1,
        1,
    );
}

#[composable]
pub(super) fn Sources(ui: Ui, area: Area) {
    let revision = ui.tick.get();
    let shared = ui.shared.clone();
    let sheets = cranpose_core::rememberKeyed((shared.clone(), revision), move |_| {
        shared
            .lock()
            .unwrap()
            .composite_images()
            .into_iter()
            .map(|(name, image)| {
                let detail = format!("{} x {} px", image.width(), image.height());
                (name, detail, previews::bitmap(image))
            })
            .collect::<Vec<_>>()
    });
    let selected = ui.shared.lock().unwrap().view.sheet.clone();
    let heading = if matches!(ui.sheet.get(), Sheet::Sources | Sheet::Advanced(3)) {
        0.
    } else {
        32.
    };
    ScrollArea(area, sheets.len() as f32 * 48. + heading, move || {
        if heading > 0. {
            Label("Source bitmaps".into(), 12., 10., area.w - 24., 12., DIM);
        }
        for (i, (name, detail, image)) in sheets.iter().enumerate() {
            let next = ui.clone();
            let source = name.clone();
            previews::AssetRow(
                ui.clone(),
                image.clone(),
                name.clone(),
                detail.clone(),
                Area::new(0., heading + i as f32 * 48., area.w - 6., 48.),
                name == &selected,
                move || {
                    state(
                        &next.shared,
                        json!({"panel":"atlas","sheet":source,"layer":"sheet"}),
                    );
                    next.sprite_crop.set(None);
                    next.fit_requested.set(true);
                    next.show(Screen::Sprites);
                },
            );
        }
    });
}

#[composable]
pub(super) fn Parts(ui: Ui, area: Area, editing: bool) {
    let revision = ui.tick.get();
    let shared = ui.shared.clone();
    let images = cranpose_core::rememberKeyed((shared.clone(), revision), move |_| {
        let doc = shared.lock().unwrap();
        PARTS
            .into_iter()
            .map(|(part, _, _, _)| {
                let mut view = doc.view.clone();
                view.panel = part.into();
                view.layers.clear();
                let (w, h) = if part == "canvas" {
                    (275, 377)
                } else {
                    mapping::size(part)
                };
                previews::bitmap(
                    doc.render_patch_for(&view, [0, 0, w as i32, h as i32])
                        .image,
                )
            })
            .collect::<Vec<_>>()
    });
    let view = ui.shared.lock().unwrap().view.clone();
    let part_h = if editing {
        (area.h * 0.48).min(368.)
    } else {
        area.h
    };
    ScrollArea(Area::new(area.x, area.y, area.w, part_h), 368., {
        let ui = ui.clone();
        move || {
            Label("Skin windows".into(), 12., 10., area.w - 24., 12., DIM);
            for (i, (part, label, _, _)) in PARTS.into_iter().enumerate() {
                let next = ui.clone();
                let detail = if part == "canvas" {
                    "All windows".into()
                } else {
                    let (w, h) = mapping::size(part);
                    format!("{w} x {h} px")
                };
                previews::AssetRow(
                    ui.clone(),
                    images[i].clone(),
                    format!("Edit {label}"),
                    detail,
                    Area::new(0., 32. + i as f32 * 48., area.w - 6., 48.),
                    view.panel == part,
                    move || next.select_part(part),
                );
            }
        }
    });
    if editing {
        previews::Divider(Area::new(area.x + 8., area.y + part_h, area.w - 16., 1.));
        Sources(
            ui,
            Area::new(area.x, area.y + part_h + 8., area.w, area.h - part_h - 8.),
        );
    }
}

#[composable]
pub(super) fn StateDetails(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let id = ui.sprite_id.get();
    let layer = ui
        .shared
        .lock()
        .unwrap()
        .skin_layers()
        .into_iter()
        .find(|layer| layer.id == id);
    if let Some(layer) = layer {
        ScrollArea(area, 268. + layer.variants.len() as f32 * 48., move || {
            Label("SELECTED SPRITE".into(), 12., 12., area.w - 24., 11., DIM);
            Label(layer.id.clone(), 12., 36., area.w - 24., 13., FG);
            Label(
                format!(
                    "{}\n{} × {} px\nCanvas position {}, {}",
                    layer.sheet,
                    layer.source[2],
                    layer.source[3],
                    layer.destination[0],
                    layer.destination[1]
                ),
                12.,
                68.,
                area.w - 24.,
                12.,
                DIM,
            );
            Label("SOURCE STATES".into(), 12., 144., area.w - 24., 11., DIM);
            for (i, rect) in layer.variants.iter().enumerate() {
                let label = layer.labels.get(i).map(String::as_str).unwrap_or("Variant");
                Label(
                    format!("{}. {label}", i + 1),
                    12.,
                    172. + i as f32 * 48.,
                    area.w - 24.,
                    12.,
                    FG,
                );
                Label(
                    format!("x {}   y {}", rect[0], rect[1]),
                    12.,
                    190. + i as f32 * 48.,
                    area.w - 24.,
                    11.,
                    DIM,
                );
            }
        });
    }
}

#[composable]
pub(super) fn Inspector(ui: Ui, area: Area, screen: Screen, _revision: u64, _preview: bool) {
    if screen == Screen::States {
        SpriteTargets(ui, area, screen);
        return;
    }
    let view = ui.shared.lock().unwrap().view.clone();
    let brush = view.brush.clone();
    let hand = ui.hand.get();
    let colour_h = if !hand && workbench::uses_colour(&brush) {
        colour::height(ui.shared.lock().unwrap().view.swatches.len(), area.w)
    } else {
        0.
    };
    let extra_h = if hand {
        0.
    } else {
        properties::essentials_height(
            &view,
            (area.h
                - 204.
                - colour_h
                - if screen == Screen::Sprites {
                    200.
                } else {
                    144.
                })
            .max(0.),
        )
    };
    let top_h = if hand { 76. } else { 204. + extra_h };
    let body_h = (top_h + colour_h + 144.).max(area.h);
    ScrollArea(area, body_h, move || {
        if hand {
            quick::Navigation(ui.clone(), Area::new(0., 0., area.w, 68.));
        } else {
            workbench::PropertyHeading(ui.clone(), Area::new(0., 0., area.w, 40.));
            previews::ToolSample(ui.clone(), Area::new(12., 44., area.w - 24., 52.));
            workbench::Properties(ui.clone(), Area::new(4., 104., area.w - 8., 64.), false);
            if extra_h > 0. {
                properties::Essentials(ui.clone(), Area::new(0., 168., area.w, extra_h));
            }
            layers::Destination(ui.clone(), Area::new(0., 164. + extra_h, area.w, 36.));
        }
        previews::Divider(Area::new(12., top_h - 2., area.w - 24., 1.));
        if colour_h > 0. {
            colour::Editor(ui.clone(), Area::new(0., top_h, area.w, colour_h));
        }
        let y = top_h + colour_h;
        previews::Divider(Area::new(12., y, area.w - 24., 1.));
        if screen == Screen::Sprites {
            SpriteTargets(
                ui.clone(),
                Area::new(0., y + 4., area.w, (body_h - y).max(200.)),
                screen,
            );
        } else {
            CompactLayers(
                ui.clone(),
                Area::new(0., y + 4., area.w, (body_h - y).max(140.)),
            );
        }
    });
}

#[composable]
fn CompactLayers(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let info = ui.shared.lock().unwrap().paint_layer_info();
    let planes = info["layers"].as_array().cloned().unwrap_or_default();
    let selected = info["active"].as_str().unwrap_or("base").to_owned();
    ScrollArea(area, 100. + planes.len() as f32 * 48., move || {
        Label("Layers".into(), 12., 16., area.w - 80., 11., DIM);
        let next = ui.clone();
        Control(
            Icon::Add,
            "Add paint layer".into(),
            "Add a layer above the source artwork".into(),
            Area::new(area.w - 56., 0., 48., 48.),
            false,
            true,
            false,
            ui.hints,
            move || plane_action(&next.shared, json!({"action":"add"})),
        );
        for (index, (id, name, visible, locked)) in
            std::iter::once(("base".to_owned(), "Source artwork".to_owned(), true, false))
                .chain(planes.iter().map(|p| {
                    (
                        p["id"].as_str().unwrap_or("").to_owned(),
                        p["name"].as_str().unwrap_or("Layer").to_owned(),
                        p["visible"].as_bool().unwrap_or(true),
                        p["locked"].as_bool().unwrap_or(false),
                    )
                }))
                .enumerate()
        {
            let top = 52. + index as f32 * 48.;
            let next = ui.clone();
            let key = id.clone();
            previews::LayerRow(
                ui.clone(),
                id.clone(),
                name,
                Area::new(0., top, area.w - if id == "base" { 6. } else { 80. }, 48.),
                selected == id,
                move || plane_action(&next.shared, json!({"action":"select","id":key})),
            );
            if id != "base" {
                for (i, (icon, property, on)) in [
                    (Icon::Eye, "visible", visible),
                    (Icon::Lock, "locked", locked),
                ]
                .into_iter()
                .enumerate()
                {
                    let next = ui.clone();
                    let key = id.clone();
                    Control(
                        icon,
                        format!("{} {}", if i == 0 { "Visibility" } else { "Lock" }, id),
                        String::new(),
                        Area::new(area.w - 76. + i as f32 * 38., top, 36., 36.),
                        on,
                        true,
                        false,
                        ui.hints,
                        move || {
                            plane_action(
                                &next.shared,
                                json!({"action":"set","id":key,property:!on}),
                            )
                        },
                    );
                }
            }
        }
    });
}

#[composable]
pub(super) fn PreviewInspector(ui: Ui, area: Area, revision: u64) {
    let shared = ui.shared.clone();
    let report = cranpose_core::rememberKeyed((shared.clone(), revision), move |_| {
        shared.lock().unwrap().classic_report().unwrap_or_else(
            |error| json!({"exportable":false,"divergences":[{"problem":error.to_string()}]}),
        )
    });
    let states_h = 32. + ((area.w - 24.) * 116. / 275. + 30.) * 4.;
    ScrollArea(area, states_h + 144., move || {
        previews::StateReferences(ui.clone(), Area::new(0., 0., area.w, states_h));
        previews::Divider(Area::new(12., states_h + 8., area.w - 24., 1.));
        Label(
            "Export format".into(),
            16.,
            states_h + 24.,
            area.w - 32.,
            12.,
            DIM,
        );
        let text = if report["exportable"] == true {
            "Classic format check passed.".to_owned()
        } else {
            report["divergences"]
                .as_array()
                .map(|errors| {
                    errors
                        .iter()
                        .take(4)
                        .map(|error| {
                            format!(
                                "{}: {}",
                                error["entry"].as_str().unwrap_or("Skin"),
                                error["problem"].as_str().unwrap_or("Review source sheets")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|| "Review source sheets before export.".into())
        };
        Label(text, 16., states_h + 48., area.w - 32., 13., FG);
    });
}

#[composable]
pub(super) fn SpriteTargets(ui: Ui, area: Area, screen: Screen) {
    let _ = ui.tick.get();
    let row_height = 48.;
    let revision = ui.tick.get();
    let shared = ui.shared.clone();
    let sheets = cranpose_core::rememberKeyed((shared.clone(), revision), move |_| {
        shared.lock().unwrap().composite_images()
    });
    let filter = cranpose_core::remember(|| TextFieldState::new("")).with(|field| *field);
    let query = filter.text().to_ascii_lowercase();
    let mut seen = std::collections::BTreeSet::new();
    let layers = ui
        .shared
        .lock()
        .unwrap()
        .skin_layers()
        .into_iter()
        .filter(|layer| seen.insert(layer.id.clone()))
        .filter(|layer| query.is_empty() || layer.id.to_ascii_lowercase().contains(&query))
        .collect::<Vec<_>>();
    let layers = Rc::new(layers);
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h),
        BoxSpec::default(),
        move || {
            Label("SPRITE TARGETS".into(), 16., 12., area.w - 32., 12., DIM);
            let feedback = controls::feedback("Filter sprite names".into());
            cranpose_ui::BasicTextField(
                filter,
                Modifier::empty()
                    .absolute_offset(16., 36.)
                    .size_points(area.w - 32., 32.)
                    .then(design::surface(design::Surface::Field, false, false, false))
                    .content_description("Filter sprite names")
                    .then(feedback)
                    .padding_symmetric(8., 6.),
                text_style(14., FG),
            );
            if query.is_empty() {
                Label(
                    "Search sprite names".into(),
                    24.,
                    43.,
                    area.w - 40.,
                    12.,
                    DIM,
                );
            }
            let ui = ui.clone();
            let layers = layers.clone();
            let sheets = sheets.clone();
            ScrollArea(
                Area::new(0., 76., area.w, area.h - 76.),
                layers.len() as f32 * row_height,
                move || {
                    let selected = if screen == Screen::Sprites && ui.sprite_crop.get().is_none() {
                        String::new()
                    } else {
                        ui.sprite_id.get()
                    };
                    for (index, layer) in layers.iter().enumerate() {
                        let next = ui.clone();
                        let layer = layer.clone();
                        let id = layer.id.clone();
                        let image = sheets
                            .get(&layer.sheet)
                            .map(|im| {
                                let [x, y, w, h] = layer.source;
                                if x + w <= im.width() && y + h <= im.height() {
                                    image::imageops::crop_imm(im, x, y, w, h).to_image()
                                } else {
                                    image::RgbaImage::new(1, 1)
                                }
                            })
                            .unwrap_or_else(|| image::RgbaImage::new(1, 1));
                        previews::AssetRow(
                            ui.clone(),
                            previews::bitmap(image),
                            format!("Sprite {}", id),
                            format!(
                                "{} x {} px · {} states",
                                layer.source[2],
                                layer.source[3],
                                layer.variants.len()
                            ),
                            Area::new(0., index as f32 * row_height, area.w - 6., row_height),
                            selected == id,
                            move || {
                                next.sprite_id.set(layer.id.clone());
                                state(
                                    &next.shared,
                                    json!({"panel":"canvas","sheet":layer.sheet,"layer":layer.id,"layers":[layer.id]}),
                                );
                                next.sprite_crop
                                    .set((screen != Screen::States).then_some(layer.destination));
                                next.fit_requested.set(true);
                                next.show(if screen == Screen::States {
                                    Screen::States
                                } else {
                                    Screen::Sprites
                                });
                            },
                        );
                    }
                },
            );
        },
    );
}
