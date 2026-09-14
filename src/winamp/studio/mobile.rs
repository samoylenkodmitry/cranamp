//! Touch layout for the same native document, brushes and history as desktop.
use super::*;
use cranpose_ui::{Column, ColumnSpec, Row, RowSpec};
use std::rc::Rc;

pub(crate) fn new_mobile_document(path: Option<&str>) -> anyhow::Result<SharedDocument> {
    let mut doc = initial_document(path)?;
    doc.view.panel = "canvas".into();
    doc.view.zoom = 2;
    let shared = SharedDocument(Arc::new(Mutex::new(doc)));
    Ok(shared)
}

/// Name the applied skin keeps in the player's library, on disk or in the
/// browser's scoped storage.
const APPLIED: &str = "Studio edited.wsz";

// Draft and applied-skin storage. Native keeps real files; the browser has no
// filesystem, so both go through the same scoped preferences the player already
// uses for its skin library.
#[cfg(not(target_arch = "wasm32"))]
fn draft_path() -> std::path::PathBuf {
    super::super::app_config_dir().join("skin-studio/draft.cstudio")
}
#[cfg(not(target_arch = "wasm32"))]
fn store_draft(bytes: &[u8], previous: bool) -> anyhow::Result<()> {
    let path = draft_path();
    let path = if previous {
        path.with_file_name("previous.cstudio")
    } else {
        path
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("cstudio.tmp");
    std::fs::write(&temp, bytes)?;
    std::fs::rename(temp, path)?;
    Ok(())
}
#[cfg(not(target_arch = "wasm32"))]
fn load_draft() -> anyhow::Result<Vec<u8>> {
    Ok(std::fs::read(draft_path())?)
}

#[cfg(target_arch = "wasm32")]
const DRAFT_KEY: &str = "cranamp.studio.draft.v1";
#[cfg(target_arch = "wasm32")]
fn store_draft(bytes: &[u8], previous: bool) -> anyhow::Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let key = if previous {
        format!("{DRAFT_KEY}.previous")
    } else {
        DRAFT_KEY.to_string()
    };
    cranpose_services::preferences()
        .set(&key, &STANDARD.encode(bytes))
        .map_err(|e| anyhow::anyhow!("Browser storage is full or blocked: {e}"))
}
#[cfg(target_arch = "wasm32")]
fn load_draft() -> anyhow::Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encoded = cranpose_services::preferences()
        .get(DRAFT_KEY)
        .ok_or_else(|| anyhow::anyhow!("No saved draft in this browser yet"))?;
    Ok(STANDARD.decode(encoded)?)
}

/// Persist the edited skin where the player can find it again, returning the
/// bytes to apply now and the path (or browser key) to remember.
pub(super) fn publish(shared: &SharedDocument) -> anyhow::Result<(Vec<u8>, String)> {
    let mut doc = shared.lock().unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = super::super::skins_library_dir().join(APPLIED);
        doc.export(&path)?;
        Ok((doc.archive()?, path.to_string_lossy().into_owned()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        doc.finish_stroke();
        let bytes = doc.archive()?;
        // Validate before writing, exactly as the native export does: a broken
        // archive must not replace the entry the player already has.
        super::super::skin::load_skin(&bytes)?;
        let key = super::super::browser_skins::save(
            cranpose_services::preferences().as_ref(),
            APPLIED,
            &bytes,
        )
        .map_err(anyhow::Error::msg)?;
        doc.path = Some(key.clone());
        doc.mark_project_saved(format!("Applied {APPLIED}"));
        Ok((bytes, key))
    }
}

fn message(shared: &SharedDocument, text: impl ToString) {
    let mut doc = shared.lock().unwrap();
    doc.message = text.to_string();
    doc.revision += 1;
}
fn save_draft(shared: &SharedDocument) -> anyhow::Result<()> {
    let mut doc = shared.lock().unwrap();
    let bytes = doc.project_bytes()?;
    store_draft(&bytes, false)?;
    doc.mark_project_saved("Saved draft".into());
    Ok(())
}

#[composable]
fn TouchButton(label: String, width: f32, selected: bool, click: impl Fn() + 'static) {
    Button(
        Modifier::empty()
            .size_points(width, 44.)
            .padding(2.)
            .background(if selected { ACCENT } else { CARD }),
        ButtonSpec::default(),
        click,
        move || {
            Text(
                label.clone(),
                Modifier::empty().padding(5.),
                text_style(12., FG),
            );
        },
    );
}

#[composable]
pub fn MobileSkinStudio(
    shared: SharedDocument,
    on_close: impl Fn() + 'static,
    on_apply: impl Fn(Vec<u8>, String) + 'static,
) {
    if cfg!(target_os = "android") || std::env::args().any(|arg| arg == "--touch-preview") {
        let active = shared.clone();
        cranpose_core::LaunchedEffect(shared.clone(), move |_| {
            if let Err(error) = mcp::start(active.clone()) {
                message(&active, error);
            }
        });
    }
    let tick = cranpose_core::rememberMutableStateOf(|| 0u64);
    let drawer = cranpose_core::rememberMutableStateOf(|| "".to_string());
    let pan_mode = cranpose_core::rememberMutableStateOf(|| false);
    // The opening zoom fits the skin to whatever surface this is, once.
    let fitted = cranpose_core::rememberMutableStateOf(|| false);
    let color_field = cranpose_core::remember(|| TextFieldState::new("#ffffff")).with(|f| *f);
    let pan = cranpose_core::rememberMutableStateOf(|| [0f32; 2]);
    let pending_export = cranpose_core::rememberMutableStateOf(|| None::<Vec<u8>>);
    let poll = shared.clone();
    cranpose_core::LaunchedEffectAsync(0u8, move |_| {
        Box::pin(async move {
            cranpose_core::interval(Duration::from_millis(32), move || {
                let revision = poll.lock().unwrap().revision;
                if tick.get_non_reactive() != revision {
                    tick.set(revision);
                }
            })
            .await;
        })
    });
    let _ = tick.get();
    let d = shared.clone();
    let close = Rc::new(on_close);
    let back = close.clone();
    cranpose::BackHandler(true, move || {
        if !drawer.get_non_reactive().is_empty() {
            drawer.set(String::new());
        } else if let Err(e) = save_draft(&d) {
            message(&d, e);
        } else {
            back();
        }
    });
    let apply = Rc::new(on_apply);

    let d = shared.clone();
    let open = cranpose_services::rememberOpenFileLauncher("cranamp.studio.open", move |result| {
        let d = d.clone();
        match result {
            Ok(Some(entry)) => {
                // Keep a recoverable layered draft before replacing the document.
                if let Err(e) = save_draft(&d) {
                    message(&d, e);
                    return;
                }
                cranpose_core::spawn_ui_task(async move {
                    let result = async {
                        let bytes = entry.read_all().await?;
                        let mut doc = if entry
                            .metadata()
                            .name
                            .to_ascii_lowercase()
                            .ends_with(".cstudio")
                        {
                            Document::open_project(&bytes)?
                        } else {
                            Document::open(&bytes, None)?
                        };
                        doc.path = None;
                        doc.view.panel = "canvas".into();
                        doc.view.zoom = 2;
                        Ok::<_, anyhow::Error>(doc)
                    }
                    .await;
                    match result {
                        Ok(mut doc) => {
                            let mut old = d.lock().unwrap();
                            doc.revision = old.revision + 1;
                            *old = doc;
                            pan.set([0., 0.]);
                        }
                        Err(e) => message(&d, e),
                    }
                });
            }
            Err(e) => message(&d, e),
            _ => {}
        }
    });
    let d = shared.clone();
    let export =
        cranpose_services::rememberSaveDocumentLauncher("cranamp.studio.export", move |result| {
            let d = d.clone();
            match result {
                Ok(Some(sink)) => {
                    if let Some(bytes) = pending_export.get_non_reactive() {
                        cranpose_core::spawn_ui_task(async move {
                            match cranpose::write_all(&sink, bytes).await {
                                Ok(()) => message(&d, "Exported Studio document"),
                                Err(e) => message(&d, e),
                            }
                            pending_export.set(None);
                        });
                    }
                }
                Err(e) => message(&d, e),
                _ => {
                    pending_export.set(None);
                }
            }
        });

    let (view, revision, status, bitmap, size) = {
        let d = shared.lock().unwrap();
        let image = d.editor_render();
        let size = [image.width() as f32, image.height() as f32];
        (
            d.view.clone(),
            d.revision,
            d.message.clone(),
            ImageBitmap::from_rgba8(image.width(), image.height(), image.into_raw()).unwrap(),
            size,
        )
    };
    BoxWithConstraints(
        Modifier::empty().fill_max_size().background(BG),
        move |scope| {
            let view = view.clone();
            let bitmap = bitmap.clone();
            let open = open.clone();
            let export = export.clone();
            let w = scope.max_width().0;
            let h = scope.max_height().0;
            let col = w / 4.;
            let body_y = 132.;
            let body_h = (h - body_y - 68.).max(44.);
            let shared = shared.clone();
            let close = close.clone();
            let apply = apply.clone();
            if !fitted.get() {
                let fit = (((w - 8.) / size[0]).floor().max(1.) as u32).min(8);
                let d = shared.clone();
                cranpose_core::SideEffect(move || {
                    fitted.set(true);
                    if fit != d.lock().unwrap().view.zoom {
                        state(&d, json!({"zoom": fit}));
                    }
                });
            }
            Column(Modifier::empty().fill_max_width(), ColumnSpec::default(), {
                let d = shared.clone();
                move || {
                    Row(Modifier::empty(), RowSpec::default(), {
                        let d = d.clone();
                        let close = close.clone();
                        move || {
                            let c = d.clone();
                            let close = close.clone();
                            TouchButton("Player".into(), col, false, move || {
                                match save_draft(&c) {
                                    Ok(()) => close(),
                                    Err(e) => message(&c, e),
                                }
                            });
                            let c = d.clone();
                            TouchButton("Undo".into(), col, false, move || {
                                c.lock().unwrap().undo();
                            });
                            let c = d.clone();
                            TouchButton("Redo".into(), col, false, move || {
                                c.lock().unwrap().redo();
                            });
                            TouchButton("Files".into(), col, drawer.get() == "Files", move || {
                                drawer.set(
                                    if drawer.get_non_reactive() == "Files" {
                                        ""
                                    } else {
                                        "Files"
                                    }
                                    .into(),
                                )
                            });
                        }
                    });
                    Row(Modifier::empty(), RowSpec::default(), {
                        let d = d.clone();
                        move || {
                            TouchButton(
                                if pan_mode.get() { "Pan ✓" } else { "Draw" }.into(),
                                col,
                                pan_mode.get(),
                                move || pan_mode.set(!pan_mode.get_non_reactive()),
                            );
                            for name in ["Layers", "Tools", "States"] {
                                TouchButton(name.into(), col, drawer.get() == name, move || {
                                    drawer.set(
                                        if drawer.get_non_reactive() == name {
                                            ""
                                        } else {
                                            name
                                        }
                                        .into(),
                                    )
                                });
                            }
                            let _ = &d;
                        }
                    });
                    Row(Modifier::empty(), RowSpec::default(), {
                        let d = d.clone();
                        move || {
                            let c = d.clone();
                            TouchButton("− zoom".into(), col, false, move || {
                                let z = c.lock().unwrap().view.zoom;
                                state(&c, json!({"zoom":z.saturating_sub(1).max(1)}));
                            });
                            let c = d.clone();
                            TouchButton("+ zoom".into(), col, false, move || {
                                let z = c.lock().unwrap().view.zoom;
                                state(&c, json!({"zoom":(z+1).min(8)}));
                            });
                            let c = d.clone();
                            TouchButton("Whole skin".into(), col, false, move || {
                                state(&c, json!({"panel":"canvas"}));
                                pan.set([0., 0.]);
                            });
                            TouchButton("Parts".into(), col, drawer.get() == "Parts", move || {
                                drawer.set(
                                    if drawer.get_non_reactive() == "Parts" {
                                        ""
                                    } else {
                                        "Parts"
                                    }
                                    .into(),
                                )
                            });
                        }
                    });
                }
            });
            if drawer.get().is_empty() {
                // Logical points per skin pixel, the same meaning `zoom` has in
                // the desktop layout. It used to be divided by the display
                // density, which quietly made the canvas a 1:1 postage stamp in
                // the corner of any high-density surface bigger than a phone --
                // and only that stamp accepted a stroke.
                let scale = view.zoom as f32;
                let limits = [
                    (size[0] - w / scale).max(0.),
                    (size[1] - body_h / scale).max(0.),
                ];
                let offset = [
                    pan.get()[0].clamp(0., limits[0]),
                    pan.get()[1].clamp(0., limits[1]),
                ];
                let d = shared.clone();
                Box(
                    Modifier::empty()
                        .absolute_offset(0., body_y)
                        .size_points(w, body_h)
                        .clip_to_bounds(),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
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
                            Some("Skin drawing canvas".into()),
                            Modifier::empty()
                                .absolute_offset(-offset[0] * scale, -offset[1] * scale)
                                .required_size(cranpose_ui::Size::new(
                                    size[0] * scale,
                                    size[1] * scale,
                                )),
                            Alignment::TOP_START,
                            cranpose_ui::ContentScale::FillBounds,
                            1.,
                            None,
                        );
                        Box(
                            Modifier::empty().fill_max_size().pointer_input(
                                (view.panel.clone(), view.zoom, pan_mode.get()),
                                move |scope: PointerInputScope| {
                                    let d = d.clone();
                                    async move {
                                        scope
                                            .await_pointer_event_scope(|events| async move {
                                                let mut last = None;
                                                let mut origin = None;
                                                let mut drag = None;
                                                loop {
                                                    let event = events.await_pointer_event().await;
                                                    let raw = [
                                                        event.position.x / scale,
                                                        event.position.y / scale,
                                                    ];
                                                    let at = pan.get_non_reactive();
                                                    let point = [
                                                        (raw[0] + at[0].clamp(0., limits[0]))
                                                            .floor()
                                                            as i32,
                                                        (raw[1] + at[1].clamp(0., limits[1]))
                                                            .floor()
                                                            as i32,
                                                    ];
                                                    match event.kind {
                                                        PointerEventKind::Down => {
                                                            if pan_mode.get_non_reactive() {
                                                                drag = Some((
                                                                    raw,
                                                                    pan.get_non_reactive(),
                                                                ));
                                                            } else {
                                                                let mut doc = d.lock().unwrap();
                                                                doc.checkpoint();
                                                                origin = Some(point);
                                                                last = Some(point);
                                                                paint(
                                                                    &mut doc, point, point, point,
                                                                );
                                                            }
                                                            event.consume();
                                                        }
                                                        PointerEventKind::Move => {
                                                            if let Some((start, at)) = drag {
                                                                pan.set([
                                                                    (at[0] + start[0] - raw[0])
                                                                        .clamp(0., limits[0]),
                                                                    (at[1] + start[1] - raw[1])
                                                                        .clamp(0., limits[1]),
                                                                ]);
                                                                event.consume();
                                                            } else if let (
                                                                Some(previous),
                                                                Some(start),
                                                            ) = (last, origin)
                                                            {
                                                                paint(
                                                                    &mut d.lock().unwrap(),
                                                                    previous,
                                                                    point,
                                                                    start,
                                                                );
                                                                last = Some(point);
                                                                event.consume();
                                                            }
                                                        }
                                                        PointerEventKind::Up
                                                        | PointerEventKind::Cancel => {
                                                            if origin.is_some() {
                                                                let mut doc = d.lock().unwrap();
                                                                if event.kind
                                                                    == PointerEventKind::Cancel
                                                                {
                                                                    doc.cancel_stroke();
                                                                } else {
                                                                    doc.finish_stroke();
                                                                }
                                                            }
                                                            last = None;
                                                            origin = None;
                                                            drag = None;
                                                            event.consume();
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
                            || {},
                        );
                    },
                );
            } else {
                let scroll = cranpose_ui::rememberScrollState!(0.);
                let d = shared.clone();
                let tab = drawer.get();
                let apply = apply.clone();
                Column(
                    Modifier::empty()
                        .absolute_offset(0., body_y)
                        .size_points(w, body_h)
                        .vertical_scroll(scroll, false)
                        .background(CARD),
                    ColumnSpec::default(),
                    move || {
                        Text(
                            tab.clone(),
                            Modifier::empty().padding(8.),
                            text_style(15., FG),
                        );
                        match tab.as_str() {
                            "Files" => {
                                let open = open.clone();
                                TouchButton("Open WSZ / project…".into(), w, false, move || {
                                    open.launch(cranpose::FilePickerOptions::default().with_filter(
                                        cranpose::FileFilter::new(
                                            "Skin/project",
                                            &["wsz", "zip", "cstudio"],
                                        ),
                                    ))
                                });
                                for project in [false, true] {
                                    let d = d.clone();
                                    let export = export.clone();
                                    TouchButton(
                                        if project {
                                            "Export layered project…"
                                        } else {
                                            "Export Winamp skin…"
                                        }
                                        .into(),
                                        w,
                                        false,
                                        move || {
                                            let data = if project {
                                                d.lock().unwrap().project_bytes()
                                            } else {
                                                d.lock().unwrap().archive()
                                            };
                                            match data {
                                                Ok(bytes) => {
                                                    pending_export.set(Some(bytes));
                                                    export.launch(
                                                        cranpose::SaveDocumentRequest::new(
                                                            if project {
                                                                "Catamp edited.cstudio"
                                                            } else {
                                                                "Catamp edited.wsz"
                                                            },
                                                            "application/zip",
                                                        ),
                                                    );
                                                }
                                                Err(e) => message(&d, e),
                                            }
                                        },
                                    );
                                }
                                let c = d.clone();
                                let apply = apply.clone();
                                TouchButton("Apply to player".into(), w, false, move || {
                                    match publish(&c) {
                                        Ok((bytes, path)) => apply(bytes, path),
                                        Err(e) => message(&c, e),
                                    }
                                });
                                let c = d.clone();
                                TouchButton("Restore last draft".into(), w, false, move || {
                                    let result = (|| {
                                        let bytes = load_draft()?;
                                        let mut restored = Document::open_project(&bytes)?;
                                        restored.path = None;
                                        restored.view.panel = "canvas".into();
                                        let mut doc = c.lock().unwrap();
                                        // Capture the outgoing work before replacing it; the restored
                                        // bytes above are already owned, so this cannot erase them.
                                        let outgoing = doc.project_bytes()?;
                                        store_draft(&outgoing, true)?;
                                        restored.revision = doc.revision + 1;
                                        *doc = restored;
                                        Ok::<_, anyhow::Error>(())
                                    })();
                                    if let Err(e) = result {
                                        message(&c, e);
                                    } else {
                                        pan.set([0., 0.]);
                                    }
                                });
                                let c = d.clone();
                                TouchButton("Save draft".into(), w, false, move || {
                                    if let Err(e) = save_draft(&c) {
                                        message(&c, e)
                                    }
                                });
                            }
                            "Tools" => {
                                cranpose_ui::BasicTextField(
                                    color_field,
                                    Modifier::empty()
                                        .fill_max_width()
                                        .height(44.)
                                        .padding(8.)
                                        .background(BG),
                                    text_style(14., FG),
                                );
                                let c = d.clone();
                                TouchButton("Use HEX color".into(), w, false, move || {
                                    state(&c, json!({"color":color_field.text()}))
                                });
                                for brush in [
                                    "pencil", "line", "curve", "tuft", "rect", "ellipse", "lift",
                                    "stamp", "glass",
                                ] {
                                    let c = d.clone();
                                    TouchButton(brush.into(), w, view.brush == brush, move || {
                                        state(&c, json!({"brush":brush}))
                                    });
                                }
                                for color in [
                                    "#ffffff",
                                    "#d4ecea",
                                    "#85bdcd",
                                    "#456e81",
                                    "#193e55",
                                    "#101e2c",
                                    "#daa0b2",
                                    "#00000000",
                                ] {
                                    let c = d.clone();
                                    TouchButton(color.into(), w, view.color == color, move || {
                                        state(&c, json!({"color":color}))
                                    });
                                }
                                for size in [1, 2, 3, 5, 8] {
                                    let c = d.clone();
                                    TouchButton(
                                        format!("Brush {size}px"),
                                        w,
                                        view.brush_size == size,
                                        move || state(&c, json!({"brush_size":size})),
                                    );
                                }
                                let c = d.clone();
                                let filled = view.filled;
                                TouchButton("Filled shapes".into(), w, filled, move || {
                                    state(&c, json!({"filled":!filled}))
                                });
                            }
                            "Layers" => {
                                let c = d.clone();
                                TouchButton("+ Painting layer".into(), w, false, move || {
                                    plane_action(&c, json!({"action":"add"}))
                                });
                                let c = d.clone();
                                TouchButton(
                                    "Base atlases".into(),
                                    w,
                                    view.paint_layer.is_none(),
                                    move || {
                                        plane_action(&c, json!({"action":"select","id":"base"}))
                                    },
                                );
                                let layers = d.lock().unwrap().planes.clone();
                                for layer in layers.iter().rev() {
                                    let c = d.clone();
                                    let id = layer.id.clone();
                                    TouchButton(
                                        layer.name.clone(),
                                        w,
                                        view.paint_layer.as_deref() == Some(&id),
                                        move || {
                                            plane_action(&c, json!({"action":"select","id":id}))
                                        },
                                    );
                                    let c = d.clone();
                                    let id = layer.id.clone();
                                    let visible = layer.visible;
                                    TouchButton(
                                        if visible { "Visible ✓" } else { "Hidden" }.into(),
                                        w,
                                        visible,
                                        move || {
                                            plane_action(
                                                &c,
                                                json!({"action":"set","id":id,"visible":!visible}),
                                            )
                                        },
                                    );
                                    let c = d.clone();
                                    let id = layer.id.clone();
                                    let locked = layer.locked;
                                    TouchButton(
                                        if locked { "Locked" } else { "Unlocked" }.into(),
                                        w,
                                        locked,
                                        move || {
                                            plane_action(
                                                &c,
                                                json!({"action":"set","id":id,"locked":!locked}),
                                            )
                                        },
                                    );
                                }
                            }
                            "Parts" => {
                                let c = d.clone();
                                let all = view.all_states;
                                TouchButton("Paint every state".into(), w, all, move || {
                                    state(&c, json!({"all_states":!all}))
                                });
                                let c = d.clone();
                                let guides = view.guides;
                                TouchButton("Show part rectangles".into(), w, guides, move || {
                                    state(&c, json!({"guides":!guides}))
                                });
                                let c = d.clone();
                                TouchButton(
                                    "Auto · topmost".into(),
                                    w,
                                    view.layers.is_empty(),
                                    move || state(&c, json!({"layer":"auto","layers":[]})),
                                );
                                for layer in d.lock().unwrap().layers() {
                                    let selected = view.layers.contains(&layer.id);
                                    let id = layer.id;
                                    let c = d.clone();
                                    TouchButton(id.clone(), w, selected, move || {
                                        let mut targets = c.lock().unwrap().view.layers.clone();
                                        if selected {
                                            targets.retain(|s| s != &id);
                                        } else {
                                            targets.push(id.clone());
                                        }
                                        state(&c, json!({"layers":targets}));
                                    });
                                }
                            }
                            "States" => {
                                let c = d.clone();
                                let pressed = view.pressed;
                                TouchButton("Pressed".into(), w, pressed, move || {
                                    state(&c, json!({"pressed":!pressed}))
                                });
                                let c = d.clone();
                                let active = view.active;
                                TouchButton("Active".into(), w, active, move || {
                                    state(&c, json!({"active":!active}))
                                });
                                for kind in ["volume", "balance", "position", "scroll"] {
                                    let c = d.clone();
                                    let frame = match kind {
                                        "volume" => view.volume,
                                        "balance" => view.balance,
                                        "position" => view.position,
                                        _ => view.scroll,
                                    };
                                    TouchButton(
                                        format!("{kind}: {frame}/27 →"),
                                        w,
                                        false,
                                        move || state(&c, json!({kind:(frame+1)%28})),
                                    );
                                }
                                for (i, frame) in view.eq.iter().copied().enumerate() {
                                    let c = d.clone();
                                    TouchButton(
                                        format!("EQ {i}: {frame}/27 →"),
                                        w,
                                        false,
                                        move || {
                                            let mut eq = c.lock().unwrap().view.eq;
                                            eq[i] = (frame + 1) % 28;
                                            state(&c, json!({"eq":eq}));
                                        },
                                    );
                                }
                                for entry in d.lock().unwrap().history()["entries"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                {
                                    let c = d.clone();
                                    let cursor = entry["cursor"].as_u64().unwrap_or(0) as usize;
                                    TouchButton(
                                        entry["label"].as_str().unwrap_or("History").into(),
                                        w,
                                        entry["current"] == true,
                                        move || {
                                            let _ = c.lock().unwrap().history_goto(cursor);
                                        },
                                    );
                                }
                            }
                            _ => {}
                        }
                    },
                );
            }
            Text(
                format!(
                    "Skin Studio · {}× · {}\n{}",
                    view.zoom,
                    if pan_mode.get() {
                        "Drag to pan"
                    } else {
                        "Draw on native pixels"
                    },
                    status
                ),
                Modifier::empty()
                    .absolute_offset(8., h - 64.)
                    .size_points((w - 16.).max(1.), 64.),
                text_style(11., DIM),
            );
            cranpose_core::SideEffect(move || {
                COMPOSED_REVISION.fetch_max(revision, Ordering::Release);
            });
        },
    );
}

fn paint(doc: &mut Document, from: [i32; 2], to: [i32; 2], origin: [i32; 2]) {
    let view = doc.view.clone();
    let result = if view.brush == "pencil" {
        parse_color(&view.color)
            .and_then(|color| doc.paint_line(from, to, color, "selection", view.all_states))
    } else {
        doc.shape_stroke(origin, to)
    };
    if let Err(e) = result {
        doc.message = e.to_string();
        doc.revision += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod harness {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/mod.rs"));
    }

    #[test]
    fn touch_canvas_draws_a_connected_stroke_and_undoes_it() {
        use cranpose_app_shell::AppShell;
        let mut doc = Document::blank();
        doc.state(
            json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","zoom":1,"color":"#ff1122"}),
        )
        .unwrap();
        let before = doc.render().get_pixel(44, 100).0;
        let shared = SharedDocument(Arc::new(Mutex::new(doc)));
        let d = shared.clone();
        let mut shell = AppShell::new(
            harness::HitGraphRenderer::default(),
            cranpose_core::location_key(file!(), line!(), column!()),
            move || MobileSkinStudio(d.clone(), || {}, |_, _| {}),
        );
        shell.set_buffer_size(393, 780);
        shell.set_viewport(393., 780.);
        harness::pump(&mut shell);
        let labels = harness::visible_texts(&mut shell);
        assert!(labels.iter().any(|t| t == "Layers"));
        assert!(labels.iter().any(|t| t == "Files"));
        shell.set_cursor(40., 232.);
        shell.pointer_pressed();
        harness::pump(&mut shell);
        shell.set_cursor(47., 232.);
        harness::pump(&mut shell);
        shell.pointer_released();
        harness::pump(&mut shell);
        {
            let doc = shared.lock().unwrap();
            let image = doc.render();
            for x in 40..=47 {
                assert_eq!(
                    image.get_pixel(x, 100).0,
                    [255, 17, 34, 255],
                    "missing stroke pixel {x}"
                );
            }
            assert_eq!(doc.history()["cursor"], 1);
        }
        // Undo through the actual button hit target, not Document::undo.
        shell.set_cursor(147., 22.);
        shell.pointer_pressed();
        harness::pump(&mut shell);
        shell.pointer_released();
        harness::pump(&mut shell);
        assert_eq!(shared.lock().unwrap().render().get_pixel(44, 100).0, before);
    }
}
