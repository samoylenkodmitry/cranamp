//! The Skin Studio workspace and canvas composition.
use super::*;

#[composable]
pub fn SkinStudio(shared: SharedDocument, host: Option<StudioHost>) {
    if host.is_some() || std::env::args().any(|arg| arg == "--touch-preview") {
        let active = shared.clone();
        cranpose_core::LaunchedEffect(shared.clone(), move |_| {
            if let Err(error) = mcp::start(active.clone()) {
                let mut doc = active.lock().unwrap();
                doc.message = error.to_string();
                doc.revision += 1;
            }
        });
    }
    let tick = cranpose_core::rememberMutableStateOf(|| 0u64);
    let live = cranpose_core::rememberMutableStateOf(|| false);
    let review = cranpose_core::rememberMutableStateOf(|| false);
    let picker = cranpose_core::rememberMutableStateOf(|| false);
    let confirm = cranpose_core::rememberMutableStateOf(|| false);
    let pan_mode = cranpose_core::rememberMutableStateOf(|| false);
    let hover = cranpose_core::rememberMutableStateOf(|| None::<[i32; 2]>);
    let studied = cranpose_core::rememberMutableStateOf(|| None::<[i32; 2]>);
    let pan = cranpose_core::rememberMutableStateOf(|| [0i32; 2]);
    let frame_kind = cranpose_core::rememberMutableStateOf(|| "volume".to_string());
    let scene_state = cranpose_core::rememberMutableStateOf(|| Scene::new(0., 0.));
    let poll = shared.clone();
    cranpose_core::LaunchedEffectAsync(0u8, move |_| {
        Box::pin(async move {
            cranpose_core::interval(Duration::from_millis(16), move || {
                let revision = poll.lock().unwrap().revision;
                if tick.get_non_reactive() != revision {
                    tick.set(revision);
                }
            })
            .await;
        })
    });
    let _ = tick.get();
    let scene = scene_state.get();
    let drawing = !live.get() && !review.get();
    let hosted = host.is_some();
    let armed = confirm.get();
    let mut action = Rows::new(20., ACTION_ROW, scene.width - 20.);
    let undo_at = action.take(76.);
    let redo_at = action.take(76.);
    let live_at = action.take(140.);
    let sheet_at = action.take(150.);
    let pan_at = (!scene.sidebar()).then(|| action.take(96.));
    let (player_at, apply_at) = if hosted {
        if scene.right(640.) < action.x + 8. {
            (action.take(90.), action.take(126.))
        } else {
            (
                (scene.right(640.), ACTION_ROW),
                (scene.right(738.), ACTION_ROW),
            )
        }
    } else {
        ((0., 0.), (0., 0.))
    };
    let draft_at = hosted.then(|| (action.take(104.), action.take(146.)));
    let inline_files = scene.right(872.) < action.x + 8.;
    let (wsz_at, open_at, export_at, blank_at, keep_at) = if inline_files {
        let wsz = if hosted { (0., 0.) } else { action.take(222.) };
        (
            wsz,
            action.take(70.),
            action.take(88.),
            action.take(94.),
            if armed { action.take(100.) } else { (0., 0.) },
        )
    } else {
        (
            (644., ACTION_ROW),
            (scene.right(872.), ACTION_ROW),
            (scene.right(950.), ACTION_ROW),
            (scene.right(1046.), ACTION_ROW),
            (scene.right(940.), ACTION_ROW),
        )
    };
    let zoom_at: Vec<(f32, f32)> = if scene.sidebar() {
        Vec::new()
    } else {
        let mut at: Vec<(f32, f32)> = (0..6).map(|_| action.take(51.)).collect();
        at.push(action.take(120.));
        at
    };
    let mut panels = Rows::new(20., action.bottom(), scene.width - 20.);
    let panel_buttons: Vec<(f32, f32, u8, &'static str, f32)> = {
        let list: &[(u8, &str, f32)] = if !drawing {
            &[
                (1, "Sprite targets", 120.),
                (9, "Skin options", 116.),
                (3, "Skin atlases", 116.),
                (2, "Edit history", 108.),
            ]
        } else {
            &[
                (4, "Drawing tools", 118.),
                (7, "Painting layers", 128.),
                (1, "Sprite targets", 120.),
                (6, "Sprite rectangles", 142.),
                (3, "Skin atlases", 116.),
                (2, "Edit history", 108.),
                (5, "Pixel study", 104.),
                (9, "Skin options", 116.),
            ]
        };
        list.iter()
            .map(|(id, label, width)| {
                let (x, y) = panels.take(*width);
                (x, y, *id, *label, *width)
            })
            .collect()
    };
    let live_extra = live.get().then(|| (panels.take(130.), panels.take(160.)));
    let mut footer = Rows::new(24., 0., scene.width - 12.);
    let state_at = footer.take_gap(198., 8.);
    let pressed_at = footer.take_gap(136., 8.);
    let active_at = footer.take_gap(128., 8.);
    let kind_at: Vec<(f32, f32)> = (0..5).map(|_| footer.take_gap(58., 4.)).collect();
    let value_at = footer.take_gap(92., 8.);
    let minus_at = footer.take_gap(46., 2.);
    let plus_at = footer.take_gap(46., 8.);
    let advance_at = footer.take_gap(118., 8.);
    footer.wrap();
    let slider_at = (24., footer.y);
    let slider_w = (scene.width - 48.).min(500.);
    let scrub_at = if 24. + slider_w + 12. + 240. <= scene.width {
        (24. + slider_w + 12., footer.y + 4.)
    } else {
        (24., footer.y + ROW_PITCH)
    };
    if scrub_at.0 == 24. {
        footer.wrap();
    }
    let scene = scene
        .under_chrome(panels.bottom() + CANVAS_HEADER)
        .over_footer(footer.bottom() + 26.);
    let (canvas_x, canvas_y, canvas_full_w, canvas_h) = scene.painting();
    let (split_w, split_drawer) = scene.split();
    let docked = split_drawer > 0.;
    let canvas_w = if docked { split_w } else { canvas_full_w };
    let (drawer_x, drawer_y, drawer_w, drawer_h) = if docked {
        (
            canvas_x + canvas_w + PREVIEW_COLUMN + 12.,
            canvas_y,
            split_drawer,
            canvas_h,
        )
    } else {
        scene.drawer()
    };
    let drawer_canvas_x = if docked {
        f32::INFINITY
    } else {
        canvas_w - DRAWER_WIDTH
    };
    let (
        view,
        message,
        revision,
        im,
        skin,
        path,
        dirty,
        sheet_subject,
        sheet_refusal,
        sheet_variants,
        undone,
        redoable,
    ) = {
        let d = shared.lock().unwrap();
        let skin = d.skin_render();
        let sheet = review.get().then(|| d.state_sheet(None));
        let (undone, redoable) = d.history_depth();
        let target = d.state_sheet_layer();
        let variants = target.as_ref().map(|l| l.variants.len()).unwrap_or(0);
        let subject = target.map(|l| l.id);
        let refusal = match &sheet {
            Some(Err(e)) => Some(format!("{e:#}")),
            _ => None,
        };
        (
            d.view.clone(),
            d.message.clone(),
            d.revision,
            match sheet {
                Some(Ok(sheet)) => sheet,
                Some(Err(_)) => image::RgbaImage::new(1, 1),
                None => d.editor_render(),
            },
            skin,
            d.path.clone(),
            d.dirty,
            subject,
            refusal,
            variants,
            undone,
            redoable,
        )
    };
    let open_drawer = drawer_id(&view.drawer);
    let export_path = path
        .as_deref()
        .map(std::path::PathBuf::from)
        .map(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Skin");
            p.with_file_name(format!("{stem} edited.wsz"))
        })
        .unwrap_or_else(default_export_path)
        .to_string_lossy()
        .to_string();
    let path_field =
        cranpose_core::remember(move || TextFieldState::new(&export_path)).with(|f| *f);
    let document_path = path.clone();
    cranpose_core::LaunchedEffect(document_path.clone(), move |_| {
        let destination = document_path.clone().unwrap_or_else(|| {
            std::path::PathBuf::from(path_field.text())
                .with_file_name("Untitled.wsz")
                .to_string_lossy()
                .to_string()
        });
        path_field.set_text(destination);
    });
    let pending_export = cranpose_core::rememberMutableStateOf(|| None::<Vec<u8>>);
    let opened = shared.clone();
    let open_launcher =
        cranpose_services::rememberOpenFileLauncher("cranamp.studio.desktop.open", move |result| {
            let d = opened.clone();
            match result {
                Ok(Some(entry)) => {
                    cranpose_core::spawn_ui_task(async move {
                        let loaded = async {
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
                            open_on_whole_skin(&mut doc);
                            Ok::<_, anyhow::Error>(doc)
                        }
                        .await;
                        let mut old = d.lock().unwrap();
                        match loaded {
                            Ok(mut doc) => {
                                doc.revision = old.revision + 1;
                                *old = doc;
                            }
                            Err(e) => {
                                old.message = format!("Open: {e:#}");
                                old.revision += 1;
                            }
                        }
                    });
                }
                Err(e) => {
                    let mut doc = d.lock().unwrap();
                    doc.message = format!("Open: {e:#}");
                    doc.revision += 1;
                }
                _ => {}
            }
        });
    let saved = shared.clone();
    let export_launcher = cranpose_services::rememberSaveDocumentLauncher(
        "cranamp.studio.desktop.export",
        move |result| {
            let d = saved.clone();
            match result {
                Ok(Some(sink)) => {
                    if let Some(bytes) = pending_export.get_non_reactive() {
                        cranpose_core::spawn_ui_task(async move {
                            let outcome = cranpose::write_all(&sink, bytes).await;
                            let mut doc = d.lock().unwrap();
                            doc.message = match outcome {
                                Ok(()) => "Exported skin".into(),
                                Err(e) => format!("Export: {e:#}"),
                            };
                            doc.revision += 1;
                            pending_export.set(None);
                        });
                    }
                }
                Err(e) => {
                    let mut doc = d.lock().unwrap();
                    doc.message = format!("Export: {e:#}");
                    doc.revision += 1;
                    pending_export.set(None);
                }
                _ => pending_export.set(None),
            }
        },
    );
    let (w, h) = (im.width(), im.height());
    let bitmap =
        ImageBitmap::from_rgba8(im.width(), im.height(), im.into_raw()).expect("studio bitmap");
    let skin_size = (skin.width(), skin.height());
    let skin_bitmap =
        ImageBitmap::from_rgba8(skin.width(), skin.height(), skin.into_raw()).expect("skin bitmap");
    let panel = view.panel.clone();
    let section = if panel == "canvas" {
        let rows = ((canvas_h - SCROLLBAR) / view.zoom.max(1) as f32) as i32;
        match pan.get()[1] + rows / 2 {
            y if y >= 232 => "playlist".to_string(),
            y if y >= 116 => "equalizer".to_string(),
            _ => "main".to_string(),
        }
    } else {
        panel.clone()
    };
    let zoom = if review.get() {
        fit_zoom((canvas_w - 24., canvas_h - 24.), (w, h)).min(4) as f32
    } else {
        view.zoom as f32
    };
    let preview_playlist_height = view.preview_playlist_height;
    let preview_size = (275, 232 + preview_playlist_height);
    let (viewport_w, viewport_h) = if live.get() { preview_size } else { (w, h) };
    let pan_value = pan.get();
    let pan_max = [
        (viewport_w as f32 - (canvas_w - SCROLLBAR) / zoom)
            .ceil()
            .max(0.) as i32,
        (viewport_h as f32 - (canvas_h - SCROLLBAR) / zoom)
            .ceil()
            .max(0.) as i32,
    ];
    let px = pan_value[0].clamp(0, pan_max[0]) as f32 * zoom;
    let py = pan_value[1].clamp(0, pan_max[1]) as f32 * zoom;
    let ox = ((canvas_w - SCROLLBAR - viewport_w as f32 * zoom) / 2.).max(0.) - px;
    let oy = ((canvas_h - SCROLLBAR - viewport_h as f32 * zoom) / 2.).max(0.) - py;
    let guide_rects = Rc::new(if view.guides && drawing {
        shared
            .lock()
            .unwrap()
            .guides()
            .into_iter()
            .map(|g| (g.id, g.rect))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    });
    let layers = { shared.lock().unwrap().layers() };
    let layer_info = layers
        .iter()
        .find(|l| l.id == view.layer)
        .map(|l| {
            format!(
                "{}\nsource {:?}\n{} states{}",
                l.sheet,
                l.source,
                l.variants.len(),
                if l.stretched() { " · shared tile" } else { "" }
            )
        })
        .unwrap_or_else(|| "Control art and background\nalike, in one stroke.".into());
    if !view.presentation && !live.get() {
        if let Ok(mut at) = PLAYER_SCENE.lock() {
            *at = None;
        }
        cranpose_core::SideEffect(move || {
            COMPOSED_REVISION.fetch_max(revision, Ordering::Release);
        });
    }
    let document = shared.clone();
    Box(
        Modifier::empty().fill_max_size().background(BG),
        BoxSpec::default(),
        move || {
            BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
                let measured = Scene::new(
                    scope.constraints().max_width,
                    scope.constraints().max_height,
                );
                if scene_state.get_non_reactive() != measured {
                    cranpose_core::SideEffect(move || scene_state.set(measured));
                }
            });
            if view.presentation {
                let d = document.clone();
                let presentation_zoom = view.zoom.min((800 / preview_size.1).clamp(1, 2)) as f32;
                BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
                    let d = d.clone();
                    let player_size = [
                        preview_size.0 as f32 * presentation_zoom,
                        preview_size.1 as f32 * presentation_zoom,
                    ];
                    let constraints = scope.constraints();
                    let origin = presentation_origin(
                        [constraints.max_width, constraints.max_height],
                        player_size,
                    );
                    note_player_scene(origin, presentation_zoom);
                    Box(
                        Modifier::empty()
                            .absolute_offset(origin[0], origin[1])
                            .required_size(cranpose_ui::Size::new(player_size[0], player_size[1])),
                        BoxSpec::default(),
                        move || {
                            LivePlayer(
                                d.clone(),
                                revision,
                                presentation_zoom,
                                preview_playlist_height as f32,
                            )
                        },
                    );
                });
                let d = document.clone();
                Action("Exit presentation".into(), 20., 18., 170., move || {
                    state(&d, json!({"presentation":false}))
                });
                return;
            }
            Label("CRANAMP  /  SKIN STUDIO".into(), 22., 12., 300., 18., FG);
            if scene.width - 380. >= 120. {
                Label(
                    format!(
                        "{}{}",
                        path.clone().unwrap_or_else(|| "Untitled skin".into()),
                        if dirty { "   • unexported edits" } else { "" }
                    ),
                    336.,
                    17.,
                    scene.width - 380.,
                    11.,
                    if dirty { ACCENT_LIT } else { DIM },
                );
            }
            {
                let d = document.clone();
                MaybeAction(
                    if undone > 0 {
                        format!("Undo {undone}")
                    } else {
                        "Undo".into()
                    },
                    undo_at.0,
                    undo_at.1,
                    76.,
                    undone > 0,
                    move || {
                        d.lock().unwrap().undo();
                    },
                );
            }
            {
                let d = document.clone();
                MaybeAction(
                    if redoable > 0 {
                        format!("Redo {redoable}")
                    } else {
                        "Redo".into()
                    },
                    redo_at.0,
                    redo_at.1,
                    76.,
                    redoable > 0,
                    move || {
                        d.lock().unwrap().redo();
                    },
                );
            }
            let closing = document.clone();
            Toggle(
                if live.get() {
                    "Player preview"
                } else {
                    "Canvas editing"
                }
                .into(),
                live_at.0,
                live_at.1,
                140.,
                live.get(),
                move || {
                    let on = !live.get_non_reactive();
                    live.set(on);
                    pan.set([0, 0]);
                    review.set(false);
                    if on {
                        close_painting_drawer(&closing, tick, open_drawer);
                    }
                },
            );
            let sheet_doc = document.clone();
            let has_subject = sheet_subject.is_some();
            Toggle(
                if review.get() {
                    "Sprite state sheet"
                } else {
                    "Canvas"
                }
                .into(),
                sheet_at.0,
                sheet_at.1,
                150.,
                review.get(),
                move || {
                    let on = !review.get_non_reactive();
                    if on && !has_subject {
                        note(
                            &sheet_doc,
                            "Choose one sprite in Sprite targets: a state sheet shows one \
                             sprite's variants."
                                .into(),
                        );
                        set_drawer(&sheet_doc, tick, 1);
                        return;
                    }
                    review.set(on);
                    live.set(false);
                    if on {
                        close_painting_drawer(&sheet_doc, tick, open_drawer);
                    }
                },
            );
            if let [zooms @ .., fit] = zoom_at.as_slice() {
                for (i, z) in [1, 2, 3, 4, 6, 8].into_iter().enumerate() {
                    let d = document.clone();
                    Choice(
                        format!("{z}×"),
                        zooms[i].0,
                        zooms[i].1,
                        51.,
                        view.zoom == z,
                        move || state(&d, json!({"zoom":z})),
                    );
                }
                let d = document.clone();
                let room = (canvas_w - SCROLLBAR - 8., canvas_h - SCROLLBAR - 8.);
                Action("Fit whole skin".into(), fit.0, fit.1, 120., move || {
                    let mut doc = d.lock().unwrap();
                    doc.view.panel = "canvas".into();
                    let best = fit_zoom(room, doc.canvas_size());
                    let _ = doc.state(
                        json!({"panel":"canvas","layer":"auto","zoom":best,"presentation":false}),
                    );
                    live.set(false);
                    review.set(false);
                    pan.set([0, 0]);
                });
            }
            if let Some(at) = pan_at {
                Toggle(
                    if pan_mode.get() { "Pan" } else { "Draw" }.into(),
                    at.0,
                    at.1,
                    96.,
                    pan_mode.get(),
                    move || pan_mode.set(!pan_mode.get_non_reactive()),
                );
            }
            if let Some(host) = host.clone() {
                let leaving = document.clone();
                let back = host.close.clone();
                cranpose::BackHandler(true, move || {
                    if open_drawer != 0 {
                        set_drawer(&leaving, tick, 0);
                    } else if let Err(e) = save_draft(&leaving) {
                        note(&leaving, format!("Draft: {e:#}"));
                    } else {
                        back();
                    }
                });
                let back = host.close.clone();
                let leaving = document.clone();
                Action("Player".into(), player_at.0, player_at.1, 90., move || {
                    if let Err(e) = save_draft(&leaving) {
                        note(&leaving, format!("Draft: {e:#}"));
                    }
                    back()
                });
                if let Some((save_at, restore_at)) = draft_at {
                    let d = document.clone();
                    Action("Save draft".into(), save_at.0, save_at.1, 104., move || {
                        if let Err(e) = save_draft(&d) {
                            note(&d, format!("Draft: {e:#}"));
                        }
                    });
                    let d = document.clone();
                    Action(
                        "Restore last draft".into(),
                        restore_at.0,
                        restore_at.1,
                        146.,
                        move || {
                            let result = (|| {
                                let bytes = load_draft()?;
                                let mut restored = Document::open_project(&bytes)?;
                                restored.path = None;
                                restored.view.panel = "canvas".into();
                                let mut doc = d.lock().unwrap();
                                let outgoing = doc.project_bytes()?;
                                store_draft(&outgoing, true)?;
                                restored.revision = doc.revision + 1;
                                *doc = restored;
                                Ok::<_, anyhow::Error>(())
                            })();
                            if let Err(e) = result {
                                note(&d, format!("Draft: {e:#}"));
                            } else {
                                pan.set([0, 0]);
                            }
                        },
                    );
                }
                let d = document.clone();
                let apply = host.apply.clone();
                Action(
                    "Apply to player".into(),
                    apply_at.0,
                    apply_at.1,
                    126.,
                    move || match publish(&d) {
                        Ok((bytes, path)) => apply(bytes, path),
                        Err(e) => note(&d, format!("Apply: {e:#}")),
                    },
                );
                let open = open_launcher.clone();
                Action("Open…".into(), open_at.0, open_at.1, 70., move || {
                    open.launch(cranpose::FilePickerOptions::default().with_filter(
                        cranpose::FileFilter::new("Skin/project", &["wsz", "zip", "cstudio"]),
                    ))
                });
                let d = document.clone();
                let export = export_launcher.clone();
                Action(
                    "Export…".into(),
                    export_at.0,
                    export_at.1,
                    88.,
                    move || match d.lock().unwrap().archive() {
                        Ok(bytes) => {
                            pending_export.set(Some(bytes));
                            export.launch(cranpose::SaveDocumentRequest::new(
                                "Catamp edited.wsz",
                                "application/zip",
                            ));
                        }
                        Err(e) => note(&d, format!("Export: {e:#}")),
                    },
                );
            } else {
                Label("WSZ".into(), wsz_at.0, wsz_at.1 + 6., 32., 11., DIM);
                cranpose_ui::BasicTextField(
                    path_field,
                    Modifier::empty()
                        .absolute_offset(wsz_at.0 + 32., wsz_at.1 + 3.)
                        .size_points(190., 24.)
                        .background(BG),
                    text_style(12., FG),
                );
                let d = document.clone();
                Action("Open".into(), open_at.0, open_at.1, 70., move || {
                    let p = path_field.text();
                    let result = mcp::call("studio_open", json!({"path":p}), &d);
                    let mut doc = d.lock().unwrap();
                    if let Err(e) = result {
                        doc.message = format!("Open: {e:#}");
                    } else {
                        open_on_whole_skin(&mut doc);
                    }
                    doc.revision += 1;
                });
                let d = document.clone();
                Action("Export".into(), export_at.0, export_at.1, 88., move || {
                    let path = path_field.text();
                    let mut doc = d.lock().unwrap();
                    if let Err(e) = doc.export(std::path::Path::new(&path)) {
                        doc.message = format!("Export: {e:#}");
                        doc.revision += 1;
                    }
                });
            }
            {
                let d = document.clone();
                let armed = confirm.get();
                let label = if armed {
                    "Discard & start blank"
                } else {
                    "New blank"
                };
                Danger(label.into(), blank_at.0, blank_at.1, 94., move || {
                    let unsaved = d.lock().unwrap().dirty;
                    if unsaved && !armed {
                        confirm.set(true);
                        note(
                            &d,
                            "This skin has edits that are not exported. Press again to \
                                 discard them and start blank."
                                .into(),
                        );
                        return;
                    }
                    confirm.set(false);
                    let call = mcp::call("studio_new", json!({"discard":true}), &d);
                    if let Err(e) = call {
                        note(&d, format!("New: {e:#}"));
                    } else {
                        {
                            let mut doc = d.lock().unwrap();
                            open_on_whole_skin(&mut doc);
                            doc.revision += 1;
                        }
                        live.set(false);
                        review.set(false);
                        pan.set([0, 0]);
                    }
                });
                if armed {
                    Action(
                        "Keep editing".into(),
                        keep_at.0,
                        keep_at.1,
                        100.,
                        move || confirm.set(false),
                    );
                }
            }
            {
                for (x, y, id, label, width) in panel_buttons.iter().copied() {
                    let d = document.clone();
                    Panel(
                        label.to_string(),
                        x,
                        y,
                        width,
                        open_drawer == id,
                        move || {
                            if id == 6 {
                                state(&d, json!({"guides":true}));
                            }
                            set_drawer(&d, tick, if open_drawer == id { 0 } else { id });
                        },
                    );
                }
                if let Some((presentation_at, playlist_at)) = live_extra {
                    let d = document.clone();
                    Action(
                        "Presentation".into(),
                        presentation_at.0,
                        presentation_at.1,
                        130.,
                        move || state(&d, json!({"presentation":true})),
                    );
                    let d = document.clone();
                    Toggle(
                        if preview_playlist_height == 145 {
                            "Compact playlist"
                        } else {
                            "Tall playlist"
                        }
                        .into(),
                        playlist_at.0,
                        playlist_at.1,
                        160.,
                        preview_playlist_height != 145,
                        move || {
                            state(
                                &d,
                                json!({"preview_playlist_height": if preview_playlist_height == 145 { 261 } else { 145 }}),
                            );
                            pan.set([0, 0]);
                        },
                    );
                }
            }
            if scene.sidebar() {
                Box(
                    Modifier::empty()
                        .absolute_offset(20., scene.top)
                        .size_points(190., canvas_h)
                        .background(PANEL_BACK)
                        .rounded_corners(6.),
                    BoxSpec::default(),
                    || {},
                );
                if drawing {
                    Label("DRAW".into(), 32., 142., 160., 11., DIM);
                    {
                        let d = document.clone();
                        Choice(
                            view.brush.clone(),
                            30.,
                            158.,
                            78.,
                            !picker.get(),
                            move || {
                                picker.set(false);
                                state(&d, json!({"brush":"pencil"}));
                            },
                        );
                    }
                    Choice(
                        "Pick pixel".into(),
                        115.,
                        158.,
                        84.,
                        picker.get(),
                        move || {
                            picker.set(true);
                        },
                    );
                    Label(
                        if picker.get() {
                            "Click a pixel to take its colour.".into()
                        } else {
                            "Drag to paint · right-click\nsamples the colour under it".into()
                        },
                        32.,
                        192.,
                        175.,
                        11.,
                        DIM,
                    );
                    Label("STROKE WIDTH".into(), 32., 226., 110., 11., DIM);
                    Label(format!("{} px", view.brush_size), 150., 225., 50., 12., FG);
                    for (i, (label, delta)) in [("− Thinner", -1i32), ("+ Thicker", 1)]
                        .into_iter()
                        .enumerate()
                    {
                        let d = document.clone();
                        let size = view.brush_size as i32;
                        Action(label.into(), 30. + i as f32 * 86., 242., 78., move || {
                            state(&d, json!({"brush_size":(size + delta).clamp(1, 32)}))
                        });
                    }
                    Label("COLOUR".into(), 32., 282., 160., 11., DIM);
                    for (i, c) in [
                        "#ffffff", "#d5f2fa", "#8fcae2", "#4382a4", "#15354a", "#09121d",
                        "#ee99b2", "#ff00ff",
                    ]
                    .iter()
                    .enumerate()
                    {
                        let d = document.clone();
                        let c = c.to_string();
                        let rgba = parse_color(&c).unwrap();
                        Box(
                            Modifier::empty()
                                .absolute_offset(
                                    32. + (i % 4) as f32 * 41.,
                                    298. + (i / 4) as f32 * 34.,
                                )
                                .size_points(32., 25.)
                                .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                                .rounded_corners(3.)
                                .clickable(move |_| state(&d, json!({"color":c}))),
                            BoxSpec::default(),
                            || {},
                        );
                    }
                    Panel(
                        "Colour picker…".into(),
                        30.,
                        364.,
                        168.,
                        open_drawer == 8,
                        {
                            let d = document.clone();
                            move || set_drawer(&d, tick, if open_drawer == 8 { 0 } else { 8 })
                        },
                    );
                    {
                        let rgba = parse_color(&view.color).unwrap_or([255, 255, 255, 255]);
                        Box(
                            Modifier::empty()
                                .absolute_offset(32., 402.)
                                .size_points(30., 20.)
                                .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                                .rounded_corners(3.),
                            BoxSpec::default(),
                            || {},
                        );
                    }
                    Label(format!("Brush {}", view.color), 70., 405., 130., 11., FG);
                    Label("EDIT SCOPE".into(), 32., 440., 160., 11., DIM);
                    Label(
                        model::scope_label(&view.states).into(),
                        32.,
                        456.,
                        168.,
                        11.,
                        FG,
                    );
                    Label("Set it in Drawing tools.".into(), 32., 470., 168., 10., DIM);
                    Label("SPRITE TARGETS".into(), 32., 494., 165., 11., DIM);
                    Label(
                        if view.layers.is_empty() {
                            "Auto · everything beneath".into()
                        } else {
                            format!("{} chosen by hand", view.layers.len())
                        },
                        32.,
                        510.,
                        165.,
                        11.,
                        FG,
                    );
                    Label(layer_info.clone(), 32., 530., 165., 10., DIM);
                } else {
                    Label(
                        if live.get() {
                            "PLAYER PREVIEW"
                        } else {
                            "SPRITE STATE SHEET"
                        }
                        .into(),
                        32.,
                        142.,
                        165.,
                        11.,
                        DIM,
                    );
                    Label(
                        if live.get() {
                            "The skin as the player draws it,\nwith live text and a real\nplaylist. Nothing here paints."
                        } else {
                            "Every variant of one sprite,\nnumbered. Read-only: pick the\nsprite in Sprite targets."
                        }
                            .into(),
                        32.,
                        164.,
                        168.,
                        11.,
                        DIM,
                    );
                    Action("Back to the canvas".into(), 30., 230., 168., move || {
                        live.set(false);
                        review.set(false);
                    });
                }
                Label("ZOOM · WHOLE PIXELS".into(), 32., 578., 165., 11., DIM);
                for (i, z) in [1, 2, 3, 4, 6, 8].into_iter().enumerate() {
                    let d = document.clone();
                    Choice(
                        format!("{z}×"),
                        30. + (i % 3) as f32 * 57.,
                        594. + (i / 3) as f32 * 34.,
                        51.,
                        view.zoom == z,
                        move || state(&d, json!({"zoom":z})),
                    );
                }
                {
                    let d = document.clone();
                    let room = (canvas_w - SCROLLBAR - 8., canvas_h - SCROLLBAR - 8.);
                    Action("Fit whole skin".into(), 30., 666., 168., move || {
                        let mut doc = d.lock().unwrap();
                        doc.view.panel = "canvas".into();
                        let best = fit_zoom(room, doc.canvas_size());
                        let _ = doc.state(
                            json!({"panel":"canvas","layer":"auto","zoom":best,"presentation":false}),
                        );
                        live.set(false);
                        review.set(false);
                        pan.set([0, 0]);
                    });
                }
                Label(
                    "Wheel scrolls the canvas, alt+wheel\nsideways, ctrl+wheel zooms at the\n\
                 pointer. Drag with the middle button\nto pan."
                        .into(),
                    32.,
                    706.,
                    168.,
                    10.,
                    DIM,
                );
            }
            Label(
                if live.get() {
                    "CRANAMP PLAYER RENDER  ·  drawing is off".into()
                } else if review.get() {
                    format!(
                        "SPRITE STATE SHEET  ·  {}  ·  {} variants  ·  drawing is off",
                        sheet_subject.clone().unwrap_or_else(|| "no sprite".into()),
                        sheet_variants
                    )
                } else {
                    format!(
                        "{}  ·  {} × {} source pixels  ·  {}×  ·  centre: {}",
                        if panel == "canvas" {
                            "WHOLE SKIN".into()
                        } else if panel == "atlas" {
                            format!("ATLAS  {}", view.sheet)
                        } else {
                            panel.to_uppercase()
                        },
                        w,
                        h,
                        view.zoom,
                        section
                    )
                },
                canvas_x,
                canvas_y - CANVAS_HEADER,
                420.,
                11.,
                DIM,
            );
            CursorReadout(hover, canvas_x + canvas_w - 200., canvas_y - CANVAS_HEADER);
            if let Some(refusal) = sheet_refusal.clone() {
                Label(
                    refusal,
                    canvas_x + 40.,
                    canvas_y + 60.,
                    canvas_w - 80.,
                    13.,
                    DIM,
                );
                let d = document.clone();
                Panel(
                    "Sprite targets".into(),
                    canvas_x + 40.,
                    canvas_y + 120.,
                    150.,
                    open_drawer == 1,
                    move || set_drawer(&d, tick, 1),
                );
            }
            let canvas_doc = document.clone();
            let canvas_bitmap = bitmap.clone();
            let canvas_panel = panel.clone();
            let canvas_guides = guide_rects.clone();
            let nav_doc = document.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(canvas_x, canvas_y)
                    .size_points(canvas_w, canvas_h)
                    .clip_to_bounds()
                    .background(Color(0.04, 0.05, 0.07, 1.))
                    .pointer_input(
                        (
                            view.zoom,
                            canvas_w.to_bits(),
                            canvas_h.to_bits(),
                            ox.to_bits(),
                            oy.to_bits(),
                        ),
                        move |scope: PointerInputScope| {
                            let d = nav_doc.clone();
                            async move {
                                scope
                                    .await_pointer_event_scope(|events| async move {
                                        let mut grab: Option<([f32; 2], [i32; 2])> = None;
                                        let limit = [viewport_w as i32, viewport_h as i32];
                                        loop {
                                            let event = events.await_pointer_event().await;
                                            match event.kind {
                                                PointerEventKind::Zoom => {
                                                    let step: i32 =
                                                        if event.zoom_delta > 1. { 1 } else { -1 };
                                                    let next = (view.zoom as i32 + step).clamp(1, 8)
                                                        as f32;
                                                    if next != zoom {
                                                        let anchor = |along: f32,
                                                                      origin: f32,
                                                                      room: f32,
                                                                      span: f32| {
                                                            let pixel = ((along - origin) / zoom)
                                                                .floor();
                                                            let centre = ((room
                                                                - SCROLLBAR
                                                                - span * next)
                                                                / 2.)
                                                                .max(0.);
                                                            (pixel - (along - centre) / next).ceil()
                                                        };
                                                        pan.set([
                                                            anchor(
                                                                event.position.x,
                                                                ox,
                                                                canvas_w,
                                                                viewport_w as f32,
                                                            )
                                                            .clamp(0., limit[0] as f32)
                                                                as i32,
                                                            anchor(
                                                                event.position.y,
                                                                oy,
                                                                canvas_h,
                                                                viewport_h as f32,
                                                            )
                                                            .clamp(0., limit[1] as f32)
                                                                as i32,
                                                        ]);
                                                        state(&d, json!({ "zoom": next as u8 }));
                                                    }
                                                    event.consume();
                                                }
                                                PointerEventKind::Scroll => {
                                                    pan.update(|p| {
                                                        for axis in 0..2 {
                                                            let delta = if axis == 0 {
                                                                event.scroll_delta.x
                                                            } else {
                                                                event.scroll_delta.y
                                                            };
                                                            p[axis] = (p[axis]
                                                                - (delta / zoom).round() as i32)
                                                                .clamp(0, limit[axis]);
                                                        }
                                                    });
                                                    event.consume();
                                                }
                                                PointerEventKind::Down => {
                                                    if event.buttons.contains(PointerButton::Middle)
                                                        || (pan_mode.get_non_reactive()
                                                            && event
                                                                .buttons
                                                                .contains(PointerButton::Primary))
                                                    {
                                                        grab = Some((
                                                            [event.position.x, event.position.y],
                                                            pan.get_non_reactive(),
                                                        ));
                                                        event.consume();
                                                    }
                                                }
                                                PointerEventKind::Move => {
                                                    if let Some((from, base)) = grab {
                                                        if event
                                                            .buttons
                                                            .contains(PointerButton::Middle)
                                                            || (pan_mode.get_non_reactive()
                                                                && event.buttons.contains(
                                                                    PointerButton::Primary,
                                                                ))
                                                        {
                                                            let travel = [
                                                                event.position.x - from[0],
                                                                event.position.y - from[1],
                                                            ];
                                                            pan.set([
                                                                (base[0]
                                                                    - (travel[0] / zoom).round()
                                                                        as i32)
                                                                    .clamp(0, limit[0]),
                                                                (base[1]
                                                                    - (travel[1] / zoom).round()
                                                                        as i32)
                                                                    .clamp(0, limit[1]),
                                                            ]);
                                                            event.consume();
                                                        } else {
                                                            grab = None;
                                                        }
                                                    }
                                                }
                                                PointerEventKind::Up
                                                | PointerEventKind::Cancel
                                                | PointerEventKind::Exit => {
                                                    grab = None;
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
                    if live.get() {
                        if view.grid && zoom >= 4. && !review.get() {
                            for x in 0..w {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(x as f32 * zoom + ox, oy)
                                        .size_points(1., h as f32 * zoom)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            for y in 0..h {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(ox, y as f32 * zoom + oy)
                                        .size_points(w as f32 * zoom, 1.)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                        }
                        let d = canvas_doc.clone();
                        note_player_scene([canvas_x + ox, canvas_y + oy], zoom);
                        Box(
                            Modifier::empty().absolute_offset(ox, oy).required_size(
                                cranpose_ui::Size::new(275. * zoom, viewport_h as f32 * zoom),
                            ),
                            BoxSpec::default(),
                            move || {
                                LivePlayer(
                                    d.clone(),
                                    revision,
                                    zoom,
                                    preview_playlist_height as f32,
                                )
                            },
                        );
                    } else {
                        cranpose_ui::Image(
                            cranpose_ui::BitmapRegionPainter(
                                canvas_bitmap.clone(),
                                Rect {
                                    x: 0.,
                                    y: 0.,
                                    width: w as f32,
                                    height: h as f32,
                                },
                                cranpose_ui::ImageSampling::Nearest,
                            ),
                            None,
                            Modifier::empty().absolute_offset(ox, oy).required_size(
                                cranpose_ui::Size::new(w as f32 * zoom, h as f32 * zoom),
                            ),
                            Alignment::TOP_START,
                            cranpose_ui::ContentScale::FillBounds,
                            1.,
                            None,
                        );
                        if view.grid && zoom >= 4. && !review.get() {
                            for x in 0..w {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(x as f32 * zoom + ox, oy)
                                        .size_points(1., h as f32 * zoom)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            for y in 0..h {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(ox, y as f32 * zoom + oy)
                                        .size_points(w as f32 * zoom, 1.)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                        }
                        let d = canvas_doc.clone();
                        if !review.get() {
                            Box(
                                Modifier::empty()
                                    .absolute_offset(ox, oy)
                                    .required_size(cranpose_ui::Size::new(w as f32 * zoom, h as f32 * zoom))
                                    .pointer_input(
                                        (
                                            canvas_panel.clone(),
                                            view.zoom,
                                            ox.to_bits(),
                                            oy.to_bits(),
                                            drawer_canvas_x.to_bits(),
                                        ),
                                        move |scope: PointerInputScope| {
                                            let d = d.clone();
                                            async move {
                                                scope
                                                    .await_pointer_event_scope(
                                                        |events| async move {
                                                            let mut last = None;
                                                            let mut origin = None;
                                                            loop {
                                                                let event = events
                                                                    .await_pointer_event()
                                                                    .await;
                                                                let point = [
                                                                    (event.position.x / zoom)
                                                                        .floor()
                                                                        as i32,
                                                                    (event.position.y / zoom)
                                                                        .floor()
                                                                        as i32,
                                                                ];
                                                                if open_drawer != 0
                                                                    && ox + event.position.x
                                                                    >= drawer_canvas_x
                                                                {
                                                                    if origin.take().is_some() {
                                                                        d.lock()
                                                                            .unwrap()
                                                                            .finish_stroke();
                                                                    }
                                                                    last = None;
                                                                    hover.set(None);
                                                                    continue;
                                                                }
                                                                match event.kind {
                                                                    PointerEventKind::Move
                                                                    | PointerEventKind::Enter => {
                                                                        hover.set(Some(point));
                                                                        studied.set(Some(point));
                                                                    }
                                                                    PointerEventKind::Exit => {
                                                                        hover.set(None);
                                                                    }
                                                                    _ => {}
                                                                }
                                                                match event.kind {
                                                                    PointerEventKind::Down => {
                                                                        let sampling = picker
                                                                            .get_non_reactive()
                                                                            || event.buttons.contains(
                                                                            PointerButton::Secondary,
                                                                        );
                                                                        if pan_mode
                                                                            .get_non_reactive()
                                                                            && !sampling
                                                                        {
                                                                            continue;
                                                                        }
                                                                        if !sampling
                                                                            && !event.buttons.contains(
                                                                            PointerButton::Primary,
                                                                        )
                                                                        {
                                                                            continue;
                                                                        }
                                                                        let mut doc =
                                                                            d.lock().unwrap();
                                                                        if sampling {
                                                                            let info = doc.inspect(
                                                                                [
                                                                                    point[0].max(0)
                                                                                        as u32,
                                                                                    point[1].max(0)
                                                                                        as u32,
                                                                                    1,
                                                                                    1,
                                                                                ],
                                                                                None,
                                                                            );
                                                                            let im = doc.render();
                                                                            if let Some(p) = im
                                                                                .get_pixel_checked(
                                                                                    point[0].max(0)
                                                                                        as u32,
                                                                                    point[1].max(0)
                                                                                        as u32,
                                                                                )
                                                                            {
                                                                                doc.view.color =
                                                                                    format!(
                                                                                        "#{:02x}{:02x}{:02x}",
                                                                                        p.0[0], p.0[1], p.0[2]
                                                                                    );
                                                                            }
                                                                            doc.message =
                                                                                info.to_string();
                                                                            doc.revision += 1;
                                                                            picker.set(false);
                                                                        } else {
                                                                            doc.checkpoint();
                                                                            let v =
                                                                                doc.view.clone();
                                                                            origin = Some(point);
                                                                            if v.brush != "pencil" { let _ = doc.shape_stroke(point, point); } else {
                                                                                let _ = doc.paint_line(
                                                                                    point,
                                                                                    point,
                                                                                    parse_color(
                                                                                        &v.color,
                                                                                    )
                                                                                        .unwrap(),
                                                                                    "selection",
                                                                                    model::Scope::named(&v.states).into(),
                                                                                );
                                                                            }
                                                                            last = Some(point);
                                                                        }
                                                                        tick.set(doc.revision);
                                                                        event.consume();
                                                                    }
                                                                    PointerEventKind::Move => {
                                                                        if event.buttons.contains(
                                                                            PointerButton::Primary,
                                                                        ) {
                                                                            if let Some(previous) =
                                                                                last
                                                                            {
                                                                                let mut doc = d
                                                                                    .lock()
                                                                                    .unwrap();
                                                                                let v = doc
                                                                                    .view
                                                                                    .clone();
                                                                                if v.brush != "pencil" { let _ = doc.shape_stroke(origin.unwrap_or(previous), point); } else {
                                                                                    let _ = doc
                                                                                        .paint_line(
                                                                                            previous,
                                                                                            point,
                                                                                            parse_color(
                                                                                                &v.color,
                                                                                            )
                                                                                                .unwrap(),
                                                                                            "selection",
                                                                                            model::Scope::named(&v.states).into(),
                                                                                        );
                                                                                }
                                                                                last = Some(point);
                                                                                tick.set(
                                                                                    doc.revision,
                                                                                );
                                                                                event.consume();
                                                                            }
                                                                        } else {
                                                                            last = None;
                                                                        }
                                                                    }
                                                                    PointerEventKind::Up
                                                                    | PointerEventKind::Cancel => {
                                                                        last = None;
                                                                        let mut doc =
                                                                            d.lock().unwrap();
                                                                        origin = None;
                                                                        if event.kind == PointerEventKind::Cancel { doc.cancel_stroke(); } else { doc.finish_stroke(); }
                                                                        tick.set(doc.revision);
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                        },
                                                    )
                                                    .await;
                                            }
                                        },
                                    ),
                                BoxSpec::default(),
                                || {},
                            );
                        }
                        if !review.get() {
                            GuideHint(
                                hover,
                                canvas_guides.clone(),
                                zoom,
                                ox,
                                oy,
                                (8., canvas_h - 24.),
                            );
                            BrushCursor(hover, zoom, view.brush_size, ox, oy);
                        }
                    }
                },
            );
            if panel != "canvas" && drawing {
                let d = document.clone();
                Action(
                    "← The whole skin".into(),
                    canvas_x + 8.,
                    canvas_y + canvas_h - 40.,
                    150.,
                    move || {
                        let mut doc = d.lock().unwrap();
                        open_on_whole_skin(&mut doc);
                        doc.revision += 1;
                        drop(doc);
                        pan.set([0, 0]);
                    },
                );
            }
            let footer_top = scene.footer_top();
            Label(
                "SPRITE STATE".into(),
                state_at.0,
                footer_top + state_at.1 - 2.,
                180.,
                11.,
                DIM,
            );
            Label(
                "the variant the canvas draws".into(),
                state_at.0,
                footer_top + state_at.1 + 14.,
                190.,
                10.,
                DIM,
            );
            {
                let d = document.clone();
                let pressed = view.pressed;
                Toggle(
                    if pressed {
                        "Buttons pressed"
                    } else {
                        "Buttons released"
                    }
                    .into(),
                    pressed_at.0,
                    footer_top + pressed_at.1,
                    136.,
                    pressed,
                    move || state(&d, json!({"pressed":!pressed})),
                );
            }
            {
                let d = document.clone();
                let active = view.active;
                Toggle(
                    if active {
                        "Window focused"
                    } else {
                        "Window behind"
                    }
                    .into(),
                    active_at.0,
                    footer_top + active_at.1,
                    128.,
                    active,
                    move || state(&d, json!({"active":!active})),
                );
            }
            let kind = frame_kind.get();
            for (i, k) in ["volume", "balance", "position", "eq", "scroll"]
                .iter()
                .enumerate()
            {
                let k = k.to_string();
                Choice(
                    k.clone(),
                    kind_at[i].0,
                    footer_top + kind_at[i].1,
                    58.,
                    kind == k,
                    move || frame_kind.set(k.clone()),
                );
            }
            let value = match kind.as_str() {
                "eq" => view.eq[0],
                "scroll" => view.scroll,
                "balance" => view.balance,
                "position" => view.position,
                _ => view.volume,
            };
            Label(
                format!("{kind}  {value:02}/27"),
                value_at.0,
                footer_top + value_at.1 + 6.,
                150.,
                12.,
                FG,
            );
            for (delta, label, at) in [(-1, "−", minus_at), (1, "+", plus_at)] {
                let d = document.clone();
                let kind = kind.clone();
                Action(label.into(), at.0, footer_top + at.1, 46., move || {
                    let n = (value as i32 + delta).clamp(0, 27) as u8;
                    let patch = if kind == "eq" {
                        json!({"eq":vec![n;11]})
                    } else {
                        json!({kind.clone():n})
                    };
                    state(&d, patch);
                });
            }
            {
                let d = document.clone();
                Action(
                    "Advance every slider".into(),
                    advance_at.0,
                    footer_top + advance_at.1,
                    118.,
                    move || {
                        let mut doc = d.lock().unwrap();
                        let next = (doc.view.volume + 1) % 28;
                        let _ = doc.state(json!({"volume":next,"balance":next,"position":next,"scroll":next,"eq":vec![next;11]}));
                    },
                );
            }
            {
                let d = document.clone();
                let kind = kind.clone();
                cranpose_ui::Slider(
                    Modifier::empty()
                        .absolute_offset(slider_at.0, footer_top + slider_at.1)
                        .size_points(slider_w, 22.),
                    value as f32 / 27.,
                    move |v| {
                        let n = (v * 27.).round() as u8;
                        state(
                            &d,
                            if kind == "eq" {
                                json!({"eq":vec![n;11]})
                            } else {
                                json!({kind.clone():n})
                            },
                        );
                    },
                    || {},
                    cranpose_ui::SliderSpec::new().thumb_extent(14.),
                    move |scope| {
                        Box(
                            Modifier::empty()
                                .absolute_offset(0., 9.)
                                .size_points(slider_w, 3.)
                                .background(CARD),
                            BoxSpec::default(),
                            || {},
                        );
                        Box(
                            Modifier::empty()
                                .absolute_offset(scope.thumb_offset(), 3.)
                                .size_points(14., 16.)
                                .background(ACCENT)
                                .rounded_corners(3.),
                            BoxSpec::default(),
                            || {},
                        );
                    },
                );
            }
            Label(
                "Scrub all 28 native frame positions".into(),
                scrub_at.0,
                footer_top + scrub_at.1,
                370.,
                11.,
                DIM,
            );
            if !review.get() && !live.get() && pan_max[1] > 0 {
                CanvasScrollbar(
                    true,
                    (
                        canvas_x + canvas_w - SCROLLBAR,
                        canvas_y,
                        SCROLLBAR,
                        canvas_h - SCROLLBAR,
                    ),
                    (canvas_h - SCROLLBAR) / zoom,
                    viewport_h as f32,
                    pan_value[1],
                    pan_max[1],
                    move |v| pan.update(|p| p[1] = v),
                );
            }
            if !review.get() && !live.get() && pan_max[0] > 0 {
                CanvasScrollbar(
                    false,
                    (
                        canvas_x,
                        canvas_y + canvas_h - SCROLLBAR,
                        canvas_w - SCROLLBAR,
                        SCROLLBAR,
                    ),
                    (canvas_w - SCROLLBAR) / zoom,
                    viewport_w as f32,
                    pan_value[0],
                    pan_max[0],
                    move |v| pan.update(|p| p[0] = v),
                );
            }
            Label(
                message.clone(),
                230.,
                scene.message_y(),
                scene.width - 265.,
                12.,
                if ALERT_REVISION.load(Ordering::Acquire) == revision {
                    ALERT
                } else {
                    DIM
                },
            );
            SkinPreview(
                skin_bitmap.clone(),
                skin_size,
                canvas_x + canvas_w + 6.,
                canvas_y + 22.,
                (PREVIEW_BOX.0, PREVIEW_BOX.1.min((canvas_h - 34.).max(40.))),
            );
            if open_drawer != 0 {
                let d = document.clone();
                Box(
                    Modifier::empty()
                        .absolute_offset(drawer_x, drawer_y)
                        .size_points(drawer_w, drawer_h)
                        .background(PANEL_BACK)
                        .rounded_corners(6.),
                    BoxSpec::default(),
                    move || {
                        {
                            let d = d.clone();
                            Action("Close".into(), 290., 10., 78., move || {
                                set_drawer(&d, tick, 0)
                            });
                        }
                        let room = (drawer_w - 24., drawer_h);
                        if open_drawer == 1 {
                            LayerChooser(d.clone(), revision, room);
                        } else if open_drawer == 2 {
                            HistoryChooser(d.clone(), revision, room);
                        } else if open_drawer == 6 {
                            GuideChooser(d.clone(), revision, room);
                        } else if open_drawer == 7 {
                            PaintChooser(d.clone(), revision, room);
                        } else if open_drawer == 5 {
                            StudyChooser(d.clone(), revision, room, studied);
                        } else if open_drawer == 4 {
                            BrushChooser(d.clone(), revision, room);
                        } else if open_drawer == 8 {
                            ColorChooser(d.clone(), revision, room);
                        } else if open_drawer == 9 {
                            SkinOptionsChooser(d.clone(), revision, room);
                        } else {
                            Label("SKIN ATLASES".into(), 12., 17., 260., 14., FG);
                            Label(
                                "One BMP at a time, at native coordinates. Same\npencil, pixel picker and undo history."
                                    .into(),
                                12.,
                                51.,
                                355.,
                                11.,
                                DIM,
                            );
                            let sheets = d.lock().unwrap().sheets();
                            let (current_panel, current_sheet) = {
                                let doc = d.lock().unwrap();
                                (doc.view.panel.clone(), doc.view.sheet.clone())
                            };
                            {
                                let target = d.clone();
                                ListChoice(
                                    "The whole skin".into(),
                                    12.,
                                    84.,
                                    drawer_w - 24.,
                                    current_panel == "canvas",
                                    move || {
                                        let mut doc = target.lock().unwrap();
                                        let _ = doc.state(json!({"panel":"canvas","layer":"auto","presentation":false}));
                                        open_on_whole_skin(&mut doc);
                                        doc.revision += 1;
                                        drop(doc);
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
                                    },
                                );
                            }
                            let report = d.lock().unwrap().cursors_report();
                            let drawn = report["drawn"].as_u64().unwrap_or(0);
                            let of = report["of"].as_u64().unwrap_or(0);
                            let mut row = 122.;
                            {
                                let target = d.clone();
                                ListChoice(
                                    format!("The cursor set      {drawn} of {of}"),
                                    12.,
                                    row,
                                    drawer_w - 24.,
                                    current_panel == "cursors",
                                    move || {
                                        state(
                                            &target,
                                            json!({"panel":"cursors","layer":"auto","zoom":2,"presentation":false}),
                                        );
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
                                    },
                                );
                            }
                            row += 38.;
                            if drawn < of {
                                let target = d.clone();
                                ListChoice(
                                    format!("Draw the {} it is missing", of - drawn),
                                    12.,
                                    row,
                                    drawer_w - 24.,
                                    false,
                                    move || {
                                        let mut doc = target.lock().unwrap();
                                        let _ = doc.draw_cursors(&[], None, false, "editor");
                                        let _ =
                                            doc.state(json!({"panel":"cursors","layer":"auto"}));
                                        drop(doc);
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
                                    },
                                );
                            }
                            if drawn < of {
                                row += 38.;
                            }
                            if current_panel == "cursors" {
                                let aiming = d.lock().unwrap().view.layer.clone();
                                for (i, (label, dx, dy)) in [
                                    ("Aim left", -1i32, 0i32),
                                    ("Aim right", 1, 0),
                                    ("Aim up", 0, -1),
                                    ("Aim down", 0, 1),
                                ]
                                .into_iter()
                                .enumerate()
                                {
                                    let target = d.clone();
                                    let region = aiming.clone();
                                    let width = (drawer_w - 32.) / 4.;
                                    ListChoice(
                                        label.into(),
                                        12. + i as f32 * (width + 4.),
                                        row,
                                        width,
                                        false,
                                        move || {
                                            let mut doc = target.lock().unwrap();
                                            let _ =
                                                doc.nudge_cursor_hotspot(&region, dx, dy, "editor");
                                        },
                                    );
                                }
                            }
                            if current_panel == "cursors" {
                                row += 38.;
                            }
                            let sheets_at = row + 10.;
                            for (i, (name, w, h)) in sheets.into_iter().enumerate() {
                                let target = d.clone();
                                let on = current_panel == "atlas" && current_sheet == name;
                                let caption = format!("{name}      {w} × {h}");
                                ListChoice(
                                    caption,
                                    12.,
                                    sheets_at + i as f32 * 38.,
                                    drawer_w - 24.,
                                    on,
                                    move || {
                                        state(
                                            &target,
                                            json!({"panel":"atlas","sheet":name,"layer":"sheet","zoom":2,"presentation":false}),
                                        );
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
                                    },
                                );
                            }
                        }
                    },
                );
            }
        },
    );
}
