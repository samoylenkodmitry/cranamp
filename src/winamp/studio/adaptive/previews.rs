use super::*;

pub(super) fn bitmap(image: image::RgbaImage) -> ImageBitmap {
    ImageBitmap::from_rgba8(image.width(), image.height(), image.into_raw())
        .expect("preview RGBA pixels")
}

pub(super) type StateCaptions = Vec<(String, [u32; 4])>;

/// Only the sprite pixels scale with the canvas. State names remain UI text.
pub(super) fn state_sheet(doc: &Document, columns: usize) -> (image::RgbaImage, StateCaptions) {
    let Ok(layer) = doc.state_sheet_asked(None) else {
        return (image::RgbaImage::new(1, 1), Vec::new());
    };
    let columns = columns.min(layer.variants.len()).max(1) as u32;
    let cw = layer.source[2] + 4;
    let ch = layer.source[3] + 6;
    let rows = (layer.variants.len() as u32).div_ceil(columns);
    let mut image =
        image::RgbaImage::from_pixel(columns * cw, rows * ch, image::Rgba([12, 14, 18, 255]));
    let sheets = doc.composite_images();
    let Some(source) = sheets.get(&layer.sheet) else {
        return (image, Vec::new());
    };
    let mut captions = Vec::new();
    for (i, [x, y, w, h]) in layer.variants.iter().copied().enumerate() {
        let ox = i as u32 % columns * cw;
        let oy = i as u32 / columns * ch;
        for sy in 0..h.min(source.height().saturating_sub(y)) {
            for sx in 0..w.min(source.width().saturating_sub(x)) {
                image.put_pixel(ox + 2 + sx, oy + 4 + sy, *source.get_pixel(x + sx, y + sy));
            }
        }
        captions.push((
            layer
                .labels
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("State {}", i + 1)),
            [ox, oy, cw, ch],
        ));
    }
    (image, captions)
}

#[composable]
pub(super) fn Image(image: ImageBitmap, area: Area, label: String) {
    let bounds = Rect {
        x: 0.,
        y: 0.,
        width: image.width() as f32,
        height: image.height() as f32,
    };
    cranpose_ui::Image(
        cranpose_ui::BitmapRegionPainter(image, bounds, cranpose_ui::ImageSampling::Nearest),
        Some(label),
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h),
        Alignment::CENTER,
        cranpose_ui::ContentScale::Fit,
        1.,
        None,
    );
}

#[composable]
pub(super) fn Divider(area: Area) {
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, 1.)
            .background(Color(0.16, 0.18, 0.22, 1.)),
        BoxSpec::default(),
        || {},
    );
}

#[allow(clippy::too_many_arguments)]
#[composable]
pub(super) fn LayerRow(
    ui: Ui,
    id: String,
    name: String,
    area: Area,
    selected: bool,
    action: impl Fn() + 'static,
) {
    let revision = ui.tick.get();
    let shared = ui.shared.clone();
    let base = id == "base";
    let (image, empty) =
        cranpose_core::rememberKeyed((shared.clone(), revision, id.clone()), move |_| {
            let image = shared.lock().unwrap().paint_layer_thumbnail(&id);
            let empty = image.pixels().all(|p| p[3] == 0);
            (bitmap(image), empty)
        });
    AssetRow(
        ui,
        image,
        name,
        if base {
            "Original pixels"
        } else if empty {
            "Empty layer"
        } else {
            "Paint layer"
        }
        .into(),
        area,
        selected,
        action,
    );
}

#[composable]
pub(super) fn ToolSample(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let shared = ui.shared.clone();
    let key = {
        let doc = shared.lock().unwrap();
        (
            serde_json::to_string(&doc.view).unwrap_or_default(),
            if matches!(doc.view.brush.as_str(), "lift" | "stamp") {
                doc.revision
            } else {
                0
            },
        )
    };
    let (image, empty) = cranpose_core::rememberKeyed((shared.clone(), key), move |_| {
        let doc = shared.lock().unwrap();
        (
            bitmap(doc.tool_sample()),
            matches!(doc.view.brush.as_str(), "lift" | "stamp") && doc.cluster.is_none(),
        )
    });
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .clip_to_bounds()
            .background(Color(0.065, 0.075, 0.09, 1.))
            .content_description("Live tool sample"),
        BoxSpec::default(),
        move || {
            if empty {
                desktop::RowLabel(
                    "No copied pixels".into(),
                    8.,
                    (area.h - 14.) / 2.,
                    area.w - 16.,
                    11.,
                    studio_dim(),
                );
            } else {
                cranpose_ui::Image(
                    cranpose_ui::BitmapPainter(image.clone()),
                    Some("Tool result preview".into()),
                    Modifier::empty()
                        .absolute_offset(6., 4.)
                        .size_points(area.w - 12., area.h - 8.),
                    Alignment::CENTER,
                    cranpose_ui::ContentScale::Fit,
                    1.,
                    None,
                );
            }
        },
    );
}

#[allow(clippy::too_many_arguments)]
#[composable]
pub(super) fn ToolCard(
    ui: Ui,
    icon: Icon,
    name: String,
    sample: ImageBitmap,
    area: Area,
    selected: bool,
    action: impl Fn() + 'static,
) {
    let hover = controls::feedback(name.clone());
    let empty = sample.pixels().as_chunks::<4>().0.iter().all(|p| p[3] == 0);
    Button(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .then(design::surface(
                design::Surface::Row,
                selected,
                false,
                false,
            ))
            .content_description(name.clone())
            .semantics(move |s| s.selected = selected.then_some(true))
            .then(hover),
        ButtonSpec::default(),
        move || {
            ui.hints.set(None);
            action();
        },
        move || {
            let sample = sample.clone();
            let name = name.clone();
            Box(
                Modifier::empty().size_points(area.w, area.h),
                BoxSpec::default(),
                move || {
                    cranpose_ui::Canvas(
                        Modifier::empty()
                            .absolute_offset(8., 6.)
                            .size_points(24., 24.),
                        {
                            let color = studio_fg();
                            move |scope| controls::draw_icon(scope, icon, color)
                        },
                    );
                    desktop::RowLabel(name.clone(), 40., 11., area.w - 48., 12., studio_fg());
                    if empty {
                        desktop::RowLabel(
                            if name == "Stamp" {
                                "Copy pixels first"
                            } else {
                                "Drag to copy pixels"
                            }
                            .into(),
                            8.,
                            38.,
                            area.w - 16.,
                            10.,
                            studio_dim(),
                        );
                    } else {
                        Image(
                            sample.clone(),
                            Area::new(8., 34., area.w - 16., area.h - 38.),
                            format!("{name} sample"),
                        );
                    }
                },
            );
        },
    );
}

#[allow(clippy::too_many_arguments)]
#[composable]
pub(super) fn AssetRow(
    ui: Ui,
    image: ImageBitmap,
    title: String,
    detail: String,
    area: Area,
    selected: bool,
    action: impl Fn() + 'static,
) {
    let source = cranpose_ui::rememberMutableInteractionSource();
    let hovered = cranpose_ui::collect_is_hovered_as_state(&source).get();
    Button(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .content_description(title.clone())
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
            ui.hints.set(None);
            action();
        },
        move || {
            let image = image.clone();
            let title = title.clone();
            let detail = detail.clone();
            Box(
                Modifier::empty().size_points(area.w, area.h),
                BoxSpec::default(),
                move || {
                    Image(
                        image.clone(),
                        Area::new(8., 6., 48., area.h - 12.),
                        format!("Preview {title}"),
                    );
                    desktop::RowLabel(
                        title
                            .strip_prefix("Sprite ")
                            .or_else(|| title.strip_prefix("Edit "))
                            .unwrap_or(&title)
                            .into(),
                        64.,
                        8.,
                        area.w - 76.,
                        12.,
                        if selected {
                            studio_accent_lit()
                        } else {
                            studio_fg()
                        },
                    );
                    desktop::RowLabel(detail.clone(), 64., 26., area.w - 76., 10., studio_dim());
                },
            );
        },
    );
}

/// Extra desktop space holds a reference sheet while the drawing canvas stays
/// focused on the current zoom. These previews also select the viewed state.
#[composable]
pub(super) fn StateReferences(ui: Ui, area: Area) {
    let revision = ui.tick.get();
    let shared = ui.shared.clone();
    let images = cranpose_core::rememberKeyed((shared.clone(), revision), move |_| {
        let doc = shared.lock().unwrap();
        [(true, false), (true, true), (false, false), (false, true)]
            .into_iter()
            .map(|(active, pressed)| {
                let mut v = doc.view.clone();
                v.panel = "main".into();
                v.layers.clear();
                v.active = active;
                v.pressed = pressed;
                bitmap(doc.render_patch_for(&v, [0, 0, 275, 116]).image)
            })
            .collect::<Vec<_>>()
    });
    let view = ui.shared.lock().unwrap().view.clone();
    let height = (area.w - 24.) * 116. / 275. + 30.;
    ScrollArea(area, 32. + height * 4., move || {
        Label(
            "State previews".into(),
            12.,
            10.,
            area.w - 24.,
            12.,
            studio_dim(),
        );
        for (i, ((active, pressed), name)) in
            [(true, false), (true, true), (false, false), (false, true)]
                .into_iter()
                .zip([
                    "Active · released",
                    "Active · pressed",
                    "Inactive · released",
                    "Inactive · pressed",
                ])
                .enumerate()
        {
            let d = ui.shared.clone();
            let tick = ui.tick;
            let image = images[i].clone();
            let selected = view.active == active && view.pressed == pressed;
            let hover = controls::feedback(format!("View {name}"));
            Button(
                Modifier::empty()
                    .absolute_offset(8., 32. + i as f32 * height)
                    .size_points(area.w - 16., height - 4.)
                    .content_description(format!("View {name}"))
                    .semantics(move |s| s.selected = selected.then_some(true))
                    .then(design::surface(
                        design::Surface::Row,
                        selected,
                        false,
                        false,
                    ))
                    .then(hover),
                ButtonSpec::default(),
                move || update(&d, tick, json!({"active":active,"pressed":pressed})),
                move || {
                    let image = image.clone();
                    Box(
                        Modifier::empty().size_points(area.w - 16., height - 4.),
                        BoxSpec::default(),
                        move || {
                            Image(
                                image.clone(),
                                Area::new(4., 4., area.w - 24., height - 32.),
                                name.into(),
                            );
                            desktop::RowLabel(
                                name.into(),
                                8.,
                                height - 24.,
                                area.w - 32.,
                                11.,
                                studio_dim(),
                            );
                        },
                    );
                },
            );
        }
    });
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/preview_tests.rs"]
mod preview_tests;
