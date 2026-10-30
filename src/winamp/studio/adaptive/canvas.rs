use super::*;
use cranpose_foundation::{
    nodes::input::gestures::{TransformGesture, TransformGestureEvent},
    PointerSource,
};
use cranpose_ui_graphics::{Brush, Point, Stroke};

pub(super) type CanvasState = cranpose_core::MutableState<Viewport>;
fn paint(
    doc: &mut Document,
    from: [i32; 2],
    to: [i32; 2],
    origin: [i32; 2],
    crop: Option<[u32; 4]>,
) {
    let translate = |p: [i32; 2]| {
        if let Some([x, y, _, _]) = crop {
            [p[0] + x as i32, p[1] + y as i32]
        } else {
            p
        }
    };
    let (from, to, origin) = (translate(from), translate(to), translate(origin));
    let result = if doc.view.brush == "pencil" {
        let color = parse_color(&doc.view.color).unwrap_or([255, 255, 255, 255]);
        doc.paint_line(
            from,
            to,
            color,
            "selection",
            model::Scope::named(&doc.view.states).into(),
        )
        .map(|_| ())
    } else {
        doc.shape_stroke(origin, to).map(|_| ())
    };
    if let Err(error) = result {
        doc.notify(error.to_string());
        doc.revision += 1;
    }
}
#[composable]
pub(super) fn EditorCanvas(
    ui: super::Ui,
    area: Area,
    enabled: bool,
    read_only: bool,
    desktop: bool,
) {
    let crop = if ui.screen.get() == Screen::Sprites {
        ui.sprite_crop.get()
    } else {
        None
    };
    let fit_requested = ui.fit_requested;
    let desktop_seen = ui.desktop_seen;
    let shared = ui.shared;
    let tick = ui.tick;
    let viewport = ui.viewport;
    let hand = ui.hand;
    let hints = ui.hints;
    let revision = tick.get();
    let columns = (area.w / 200.).floor().clamp(1., 7.) as usize;
    let cached =
        cranpose_core::rememberKeyed((shared.clone(), revision, read_only, crop, columns), {
            let d = shared.clone();
            move |_| {
                let doc = d.lock().unwrap();
                let (image, captions) = if read_only {
                    previews::state_sheet(&doc, columns)
                } else {
                    (doc.editor_render(), Vec::new())
                };
                let image = if let Some([x, y, w, h]) = crop {
                    if x.saturating_add(w) <= image.width() && y.saturating_add(h) <= image.height()
                    {
                        image::imageops::crop_imm(&image, x, y, w, h).to_image()
                    } else {
                        image
                    }
                } else {
                    image
                };
                let size = [image.width() as f32, image.height() as f32];
                let bitmap =
                    ImageBitmap::from_rgba8(image.width(), image.height(), image.into_raw())
                        .expect("editor RGBA image");
                (
                    bitmap,
                    size,
                    doc.view.panel.clone(),
                    doc.view.grid && !read_only,
                    doc.view.brush_size,
                    captions,
                )
            }
        });
    let (bitmap, size, panel, grid, brush_size, captions) = cached;
    let mut view = viewport.get();
    view.resize([(area.w - 20.).max(1.), (area.h - 20.).max(1.)], size);
    if fit_requested.get() || (desktop && !desktop_seen.get()) {
        view.fit();
        cranpose_core::SideEffect(move || {
            fit_requested.set(false);
            desktop_seen.set(true);
        });
    }
    let opened = cranpose_core::rememberMutableStateOf(|| false);
    if !opened.get_non_reactive() {
        if area.h < 220. && !read_only {
            view.scale = view.scale.max(1.);
            view.scroll = [0., 0.];
        }
        cranpose_core::SideEffect(move || opened.set(true));
    }
    if view != viewport.get_non_reactive() {
        cranpose_core::SideEffect(move || viewport.set(view));
    }
    let origin = view.origin();
    let hover = cranpose_core::rememberMutableStateOf(|| None::<[i32; 2]>);
    let brush = shared.lock().unwrap().view.brush.clone();
    let show_footprint =
        !hand.get() && matches!(brush.as_str(), "pencil" | "line" | "curve" | "tuft");
    let d = shared.clone();
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w - 20., area.h - 20.)
            .clip_to_bounds()
            .then(design::surface(design::Surface::Well, false, false, false))
            .content_description(format!(
                "Skin canvas. Zoom {:.4}%. Pinch to zoom; use two fingers to pan.",
                view.scale * 100.
            ))
            .pointer_icon(super::controls::canvas_cursor(
                &brush,
                hand.get() || read_only,
            ))
            .pointer_input(
                (
                    Arc::as_ptr(&shared.0) as usize,
                    panel,
                    enabled,
                    read_only,
                    crop,
                ),
                move |scope: PointerInputScope| {
                    let d = d.clone();
                    async move {
                        scope
                            .await_pointer_event_scope(|events| async move {
                                let mut gestures = TransformGesture::new();
                                let mut navigating = false;
                                let mut pen_down = false;
                                let mut last = None;
                                let mut start = None;
                                let mut pan_from: Option<([f32; 2], [f32; 2])> = None;
                                loop {
                                    let event = events.await_pointer_event().await;
                                    if !enabled {
                                        event.consume();
                                        continue;
                                    }
                                    let current = viewport.get_non_reactive();
                                    let point = current.point(event.position);
                                    let contact = event.source == PointerSource::Touch
                                        || event.source == PointerSource::Unknown;
                                    if pen_down && contact {
                                        event.consume();
                                        continue;
                                    }
                                    let transform = if contact {
                                        gestures.handle_event(&event)
                                    } else {
                                        TransformGestureEvent::None
                                    };
                                    if gestures.pointer_count() >= 2 {
                                        if !navigating {
                                            let mut doc = d.lock().unwrap();
                                            doc.cancel_stroke();
                                            tick.set(doc.revision);
                                            navigating = true;
                                            last = None;
                                            start = None;
                                            pan_from = None;
                                        }
                                        if let TransformGestureEvent::Transform {
                                            centroid,
                                            pan,
                                            zoom,
                                            ..
                                        } = transform
                                        {
                                            viewport.update(|v| v.transform(centroid, pan, zoom));
                                        }
                                        hover.set(None);
                                        event.consume();
                                        continue;
                                    }
                                    if navigating {
                                        if gestures.pointer_count() == 0 {
                                            navigating = false;
                                        }
                                        event.consume();
                                        continue;
                                    }
                                    match event.kind {
                                        PointerEventKind::Zoom => {
                                            viewport.update(|v| {
                                                v.transform(
                                                    event.position,
                                                    Point::ZERO,
                                                    event.zoom_delta,
                                                )
                                            });
                                            event.consume();
                                        }
                                        PointerEventKind::Scroll => {
                                            viewport.update(|v| {
                                                v.scroll[0] -= event.scroll_delta.x;
                                                v.scroll[1] -= event.scroll_delta.y;
                                                v.constrain();
                                            });
                                            event.consume();
                                        }
                                        PointerEventKind::Enter | PointerEventKind::Move => {
                                            if event.source != PointerSource::Touch {
                                                hover.set(Some(point));
                                            }
                                            if let Some((from, base)) = pan_from {
                                                if event.buttons
                                                    != cranpose_foundation::PointerButtons::NONE
                                                {
                                                    viewport.update(|v| {
                                                        v.scroll = [
                                                            base[0] - (event.position.x - from[0]),
                                                            base[1] - (event.position.y - from[1]),
                                                        ];
                                                        v.constrain();
                                                    });
                                                    event.consume();
                                                }
                                            } else if let (Some(previous), Some(origin)) =
                                                (last, start)
                                            {
                                                if event.buttons.contains(PointerButton::Primary) {
                                                    let mut doc = d.lock().unwrap();
                                                    paint(&mut doc, previous, point, origin, crop);
                                                    last = Some(point);
                                                    tick.set(doc.revision);
                                                    event.consume();
                                                }
                                            }
                                        }
                                        PointerEventKind::Down => {
                                            hints.set(None);
                                            if event.source == PointerSource::Stylus {
                                                pen_down = true;
                                            }
                                            if read_only
                                                || hand.get_non_reactive()
                                                || event.buttons.contains(PointerButton::Middle)
                                            {
                                                pan_from = Some((
                                                    [event.position.x, event.position.y],
                                                    current.scroll,
                                                ));
                                                event.consume();
                                                continue;
                                            }
                                            if point[0] < 0
                                                || point[1] < 0
                                                || point[0] >= size[0] as i32
                                                || point[1] >= size[1] as i32
                                            {
                                                continue;
                                            }
                                            let mut doc = d.lock().unwrap();
                                            if event.buttons.contains(PointerButton::Secondary) {
                                                if let Some(pixel) = doc.render().get_pixel_checked(
                                                    point[0] as u32 + crop.map_or(0, |r| r[0]),
                                                    point[1] as u32 + crop.map_or(0, |r| r[1]),
                                                ) {
                                                    doc.view.color = format!(
                                                        "#{:02x}{:02x}{:02x}",
                                                        pixel[0], pixel[1], pixel[2]
                                                    );
                                                    doc.revision += 1;
                                                }
                                            } else if event.buttons.contains(PointerButton::Primary)
                                            {
                                                doc.checkpoint();
                                                start = Some(point);
                                                last = Some(point);
                                                paint(&mut doc, point, point, point, crop);
                                            }
                                            tick.set(doc.revision);
                                            event.consume();
                                        }
                                        PointerEventKind::Up | PointerEventKind::Cancel => {
                                            pen_down = false;
                                            pan_from = None;
                                            last = None;
                                            start = None;
                                            let mut doc = d.lock().unwrap();
                                            if event.kind == PointerEventKind::Cancel {
                                                doc.cancel_stroke();
                                            } else {
                                                doc.finish_stroke();
                                            }
                                            tick.set(doc.revision);
                                            event.consume();
                                        }
                                        PointerEventKind::Exit => {
                                            hover.set(None);
                                        }
                                        _ => {}
                                    }
                                }
                            })
                            .await;
                    }
                },
            ),
        BoxSpec::default(),
        move || {
            cranpose_ui::Image(
                cranpose_ui::BitmapRegionPainter(
                    bitmap.clone(),
                    Rect {
                        x: 0.,
                        y: 0.,
                        width: size[0],
                        height: size[1],
                    },
                    cranpose_ui::ImageSampling::Nearest,
                ),
                None,
                Modifier::empty()
                    .absolute_offset(origin[0], origin[1])
                    .required_size(cranpose_ui::Size::new(
                        size[0] * view.scale,
                        size[1] * view.scale,
                    )),
                Alignment::TOP_START,
                cranpose_ui::ContentScale::FillBounds,
                1.,
                None,
            );
            for (label, [x, y, w, _]) in &captions {
                desktop::RowLabel(
                    label.clone(),
                    origin[0] + *x as f32 * view.scale + 8.,
                    origin[1] + *y as f32 * view.scale + 5.,
                    (*w as f32 * view.scale - 16.).max(1.),
                    12.,
                    studio_fg(),
                );
            }
            if grid && view.scale >= 4. {
                cranpose_ui::Canvas(Modifier::empty().fill_max_size(), move |scope| {
                    let color = Brush::solid(Color(0.35, 0.65, 0.75, 0.25));
                    let begin = [
                        (-origin[0] / view.scale).floor().max(0.) as u32,
                        (-origin[1] / view.scale).floor().max(0.) as u32,
                    ];
                    let end = [
                        ((view.room[0] - origin[0]) / view.scale)
                            .ceil()
                            .min(size[0]) as u32,
                        ((view.room[1] - origin[1]) / view.scale)
                            .ceil()
                            .min(size[1]) as u32,
                    ];
                    for x in begin[0]..=end[0] {
                        let x = origin[0] + x as f32 * view.scale;
                        scope.draw_line(
                            color.clone(),
                            Point::new(x, 0.),
                            Point::new(x, view.room[1]),
                            Stroke::new(1.),
                        );
                    }
                    for y in begin[1]..=end[1] {
                        let y = origin[1] + y as f32 * view.scale;
                        scope.draw_line(
                            color.clone(),
                            Point::new(0., y),
                            Point::new(view.room[0], y),
                            Stroke::new(1.),
                        );
                    }
                });
            }
            if !read_only && show_footprint {
                BrushCursor(hover, view.scale, brush_size, origin[0], origin[1]);
            }
            if let Some([x, y]) = hover.get().filter(|p| p[0] >= 0 && p[1] >= 0) {
                let x = x + crop.map_or(0, |r| r[0] as i32);
                let y = y + crop.map_or(0, |r| r[1] as i32);
                Box(
                    Modifier::empty()
                        .absolute_offset(8., 8.)
                        .size_points(104., 24.)
                        .background(Color(0.03, 0.05, 0.08, 0.85))
                        .rounded_corners(4.),
                    BoxSpec::default(),
                    move || Label(format!("x {x}  y {y}"), 6., 5., 92., 11., studio_fg()),
                );
            }
        },
    );
    for axis in 0..2 {
        let bar = if axis == 0 {
            Area::new(area.x, area.y + area.h - 20., area.w - 20., 20.)
        } else {
            Area::new(area.x + area.w - 20., area.y, 20., area.h - 20.)
        };
        Scrollbar(axis, bar, viewport, hints);
    }
}
#[composable]
fn Scrollbar(axis: usize, area: Area, state: CanvasState, hints: Hints) {
    let view = state.get();
    let span = if axis == 0 { area.w } else { area.h };
    let thumb =
        (span * view.room[axis] / (view.content[axis] * view.scale)).clamp(24f32.min(span), span);
    let travel = (span - thumb).max(0.);
    let offset = if view.limits()[axis] > 0. {
        view.scroll[axis] / view.limits()[axis] * travel
    } else {
        0.
    };
    let hover = cranpose_core::rememberMutableStateOf(|| false);
    Box(Modifier::empty().absolute_offset(area.x,area.y).size_points(area.w,area.h).background(studio_bg())
        .content_description(if axis==0 {"Canvas horizontal scrollbar"}else{"Canvas vertical scrollbar"})
        .pointer_icon(control_cursor())
        .pointer_input((axis,area.w.to_bits(),area.h.to_bits()),move |scope:PointerInputScope|async move {
            scope.await_pointer_event_scope(|events|async move {
                let mut grab=None;
                loop {
                    let e=events.await_pointer_event().await;
                    let v=state.get_non_reactive();
                    let thumb=(span*v.room[axis]/(v.content[axis]*v.scale)).clamp(24f32.min(span),span);
                    let travel=(span-thumb).max(0.);
                    let max=v.limits()[axis];
                    let offset=if max>0. {v.scroll[axis]/max*travel}else{0.};
                    let along=if axis==0 {e.position.x}else{e.position.y};
                    match e.kind {
                        PointerEventKind::Enter=>{hover.set(true);hints.set(Some(Hint{label:"Canvas scrollbar".into(),text:"Drag to scroll. The thumb shows the visible part of the artwork.".into(),x:area.x,y:area.y}));}
                        PointerEventKind::Exit=>{hover.set(false);hints.set(None);}
                        PointerEventKind::Down=>{grab=Some(if along>=offset && along<=offset+thumb {along-offset}else{thumb/2.});}
                        PointerEventKind::Up | PointerEventKind::Cancel=>{grab=None;}
                        _=>{}
                    }
                    if matches!(e.kind,PointerEventKind::Down|PointerEventKind::Move) && e.buttons.contains(PointerButton::Primary) {
                        if let Some(grab)=grab {if travel>0. {state.update(|v|v.scroll[axis]=((along-grab)/travel).clamp(0.,1.)*max);e.consume();}}
                    }
                }
            }).await;
        }),BoxSpec::default(),move || {
            Box(Modifier::empty().absolute_offset(if axis==0 {offset}else{8.},if axis==0 {8.}else{offset})
                .size_points(if axis==0 {thumb}else{4.},if axis==0 {4.}else{thumb}).rounded_corners(6.).background(if view.limits()[axis] <= 0. {Color(0.,0.,0.,0.)} else if hover.get(){studio_fg()}else{studio_dim()}),BoxSpec::default(),||{});
        });
}
