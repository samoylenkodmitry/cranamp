use super::*;
#[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
mod agent_connection;
mod canvas;
mod colour;
pub(crate) mod controls;
mod desktop;
mod help;
mod layers;
mod layout;
mod motion;
mod notification;
mod panels;
mod previews;
mod properties;
mod quick;
#[cfg(feature = "studio-review")]
#[path = "../../../../test/studio_review_catalog.rs"]
pub mod review;
mod skin;
mod viewport;
mod workbench;
use canvas::{CanvasState, EditorCanvas};
use controls::{control_cursor, Control, Hint, HintOverlay, Hints, Icon};
use layout::{Area, Layout, WidthClass};
use panels::*;
use viewport::Viewport;

fn update(shared: &SharedDocument, tick: Tick, patch: serde_json::Value) {
    state(shared, patch);
    tick.set(shared.lock().unwrap().revision);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Projects,
    Canvas,
    Preview,
    States,
    Sprites,
    Skin,
    Save,
    Help,
    ConfirmBlank,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sheet {
    None,
    Scopes,
    Parts,
    Targets,
    Sources,
    Tools,
    Colour,
    CanvasState,
    Layers,
    More,
    Advanced(u8),
}
#[derive(Clone)]
struct Actions {
    open: Rc<dyn Fn()>,
    project: Rc<dyn Fn()>,
    export: Rc<dyn Fn()>,
    apply: Rc<dyn Fn()>,
    restore: Rc<dyn Fn()>,
    close: Rc<dyn Fn()>,
}
impl PartialEq for Actions {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.open, &other.open)
            && Rc::ptr_eq(&self.project, &other.project)
            && Rc::ptr_eq(&self.export, &other.export)
            && Rc::ptr_eq(&self.apply, &other.apply)
            && Rc::ptr_eq(&self.restore, &other.restore)
            && Rc::ptr_eq(&self.close, &other.close)
    }
}
#[derive(Clone, PartialEq)]
struct Ui {
    shared: SharedDocument,
    layout: cranpose_core::MutableState<Layout>,
    tick: Tick,
    screen: cranpose_core::MutableState<Screen>,
    previous: cranpose_core::MutableState<Screen>,
    sheet: cranpose_core::MutableState<Sheet>,
    viewport: CanvasState,
    fit_requested: cranpose_core::MutableState<bool>,
    desktop_seen: cranpose_core::MutableState<bool>,
    sprite_crop: cranpose_core::MutableState<Option<[u32; 4]>>,
    sprite_id: cranpose_core::MutableState<String>,
    hand: cranpose_core::MutableState<bool>,
    skin_section: cranpose_core::MutableState<usize>,
    hints: Hints,
    review: cranpose_core::MutableState<bool>,
    live: cranpose_core::MutableState<bool>,
}
impl Ui {
    fn open_sheet(&self, sheet: Sheet) {
        self.hints.set(None);
        if sheet == Sheet::None {
            set_drawer(&self.shared, self.tick, 0);
        }
        self.sheet.set(sheet);
    }
    fn show(&self, screen: Screen) {
        if screen == Screen::States {
            let result = self.shared.lock().unwrap().state_sheet(None);
            if let Err(error) = result {
                note(&self.shared, format!("Sprite states: {error:#}"));
                return;
            }
        }
        self.hints.set(None);
        self.open_sheet(Sheet::None);
        self.previous.set(self.screen.get_non_reactive());
        self.screen.set(screen);
    }
    fn back(&self) {
        if self.sheet.get_non_reactive() != Sheet::None {
            self.open_sheet(Sheet::None);
        } else {
            let target = match self.screen.get_non_reactive() {
                Screen::Canvas | Screen::ConfirmBlank => Screen::Projects,
                Screen::Preview | Screen::States | Screen::Sprites | Screen::Skin => Screen::Canvas,
                _ => self.previous.get_non_reactive(),
            };
            self.show(target);
        }
    }
    fn select_part(&self, part: &str) {
        self.fit_requested.set(true);
        state(&self.shared, json!({"panel":part,"presentation":false}));
        self.viewport.update(|v| {
            v.scroll = [0.; 2];
        });
        self.show(Screen::Canvas);
    }
}
#[composable]
pub fn SkinStudio(shared: SharedDocument, host: Option<StudioHost>) {
    MountedSkinStudio(shared, host, true, 0.0);
}

/// Keep editor navigation and the viewport while a host tab is hidden.
#[composable]
pub(super) fn MountedSkinStudio(
    shared: SharedDocument,
    host: Option<StudioHost>,
    visible: bool,
    bottom_inset: f32,
) {
    let ui = cranpose_core::rememberKeyed(shared.clone(), {
        let shared = shared.clone();
        move |_| Ui {
            shared: shared.clone(),
            layout: cranpose_core::mutableStateOf(Layout::new(430., 820.)),
            tick: cranpose_core::mutableStateOf(shared.lock().unwrap().revision),
            screen: cranpose_core::mutableStateOf(Screen::Projects),
            previous: cranpose_core::mutableStateOf(Screen::Projects),
            sheet: cranpose_core::mutableStateOf(Sheet::None),
            viewport: cranpose_core::mutableStateOf(Viewport::default()),
            fit_requested: cranpose_core::mutableStateOf(false),
            desktop_seen: cranpose_core::mutableStateOf(false),
            sprite_crop: cranpose_core::mutableStateOf(None),
            sprite_id: cranpose_core::mutableStateOf("main.play".into()),
            hand: cranpose_core::mutableStateOf(false),
            skin_section: cranpose_core::mutableStateOf(0),
            hints: cranpose_core::mutableStateOf(None),
            review: cranpose_core::mutableStateOf(false),
            live: cranpose_core::mutableStateOf(false),
        }
    });
    if !visible {
        return;
    }
    let poll = shared.clone();
    let tick = ui.tick;
    cranpose_core::LaunchedEffectAsync(shared.clone(), move |_| {
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
    #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
    {
        let active = shared.clone();
        cranpose_core::DisposableEffect(shared.clone(), move |scope| {
            scope.on_dispose(move || agent_server::stop_for(&active))
        });
    }
    let _ = ui.tick.get();
    let (drawer, requested_zoom, presentation) = {
        let d = shared.lock().unwrap();
        (d.view.drawer.clone(), d.view.zoom, d.view.presentation)
    };
    let select = ui.clone();
    cranpose_core::LaunchedEffect(drawer.clone(), move |_| {
        let id = drawer_id(&drawer);
        if id == 9 {
            select.screen.set(Screen::Skin);
            select.sheet.set(Sheet::None);
        } else if id > 0 {
            select.screen.set(Screen::Canvas);
            select.sheet.set(match id {
                8 => Sheet::Colour,
                _ => Sheet::Advanced(id),
            });
        }
    });
    let viewport = ui.viewport;
    cranpose_core::LaunchedEffect(requested_zoom, move |_| {
        viewport.update(|v| {
            v.scale = requested_zoom as f32;
            v.constrain();
        })
    });
    let screen = ui.screen;
    cranpose_core::LaunchedEffect(presentation, move |_| {
        if presentation {
            screen.set(Screen::Preview);
        }
    });
    let review = ui.review.get();
    let review_ui = ui.clone();
    cranpose_core::LaunchedEffect(review, move |_| {
        if review {
            review_ui.review.set(false);
            review_ui.show(Screen::States);
        }
    });
    let pending = cranpose_core::rememberMutableStateOf(|| None::<(Vec<u8>, bool, u64)>);
    let busy = cranpose_core::rememberMutableStateOf(|| false);
    let opened = ui.clone();
    let open =
        cranpose_services::rememberOpenFileLauncher("cranamp.studio.desktop.open", move |result| {
            match result {
                Ok(Some(entry)) => {
                    busy.set(true);
                    let ui = opened.clone();
                    cranpose_core::spawn_ui_task(async move {
                        let result = async {
                            let bytes = crate::content_io::read_content(
                                &entry,
                                crate::content_io::MAX_DOCUMENT_BYTES,
                            )
                            .await?;
                            let mut incoming = if entry
                                .metadata()
                                .name
                                .to_ascii_lowercase()
                                .ends_with(".cstudio")
                            {
                                Document::open_project(&bytes)?
                            } else {
                                Document::open(&bytes, None)?
                            };
                            incoming.path = None;
                            incoming.open_on_whole_skin();
                            incoming.view.drawer = "none".into();
                            incoming.view.zoom = 1;
                            incoming.notify(format!("Opened {}", entry.metadata().name));
                            let mut old = ui.shared.lock().unwrap();
                            if old.dirty {
                                store_draft(&old.project_bytes()?, false)?;
                            }
                            incoming.revision = old.revision + 1;
                            *old = incoming;
                            Ok::<_, anyhow::Error>(())
                        }
                        .await;
                        busy.set(false);
                        match result {
                            Ok(()) => {
                                ui.viewport.set(Viewport::default());
                                ui.show(Screen::Canvas);
                                ui.fit_requested.set(true);
                            }
                            Err(e) => note(&ui.shared, format!("Open: {e:#}")),
                        }
                    });
                }
                Err(e) => note(&opened.shared, format!("Open: {e:#}")),
                _ => {}
            }
        });
    let saved = ui.clone();
    let save = cranpose_services::rememberSaveDocumentLauncher(
        "cranamp.studio.desktop.export",
        move |result| match result {
            Ok(Some(sink)) => {
                if let Some((bytes, project, revision)) = pending.get_non_reactive() {
                    let ui = saved.clone();
                    busy.set(true);
                    cranpose_core::spawn_ui_task(async move {
                        let outcome = cranpose::write_all(&sink, bytes).await;
                        busy.set(false);
                        pending.set(None);
                        finish_save(
                            &ui.shared,
                            project,
                            revision,
                            outcome.map_err(|e| e.to_string()),
                        );
                    });
                }
            }
            Err(e) => {
                pending.set(None);
                note(&saved.shared, format!("Save: {e:#}"));
            }
            _ => pending.set(None),
        },
    );
    let actions = Actions {
        open: Rc::new(move || open.launch(studio_open_options())),
        project: {
            let ui = ui.clone();
            let launcher = save.clone();
            Rc::new(move || save_file(&ui, pending, launcher.clone(), true))
        },
        export: {
            let ui = ui.clone();
            Rc::new(move || save_file(&ui, pending, save.clone(), false))
        },
        apply: {
            let ui = ui.clone();
            let host = host.clone();
            Rc::new(move || {
                let result = save_draft(&ui.shared).and_then(|_| publish(&ui.shared));
                match result {
                    Ok((bytes, path)) => {
                        if let Some(host) = &host {
                            (host.apply)(bytes, path)
                        }
                    }
                    Err(e) => note(&ui.shared, format!("Apply: {e:#}")),
                }
            })
        },
        restore: {
            let ui = ui.clone();
            Rc::new(move || {
                let result = (|| {
                    let mut incoming = Document::open_project(&load_draft()?)?;
                    let mut old = ui.shared.lock().unwrap();
                    store_draft(&old.project_bytes()?, true)?;
                    incoming.path = None;
                    incoming.revision = old.revision + 1;
                    *old = incoming;
                    Ok::<_, anyhow::Error>(())
                })();
                match result {
                    Ok(()) => {
                        ui.viewport.set(Viewport::default());
                        ui.show(Screen::Canvas);
                    }
                    Err(e) => note(&ui.shared, format!("Restore: {e:#}")),
                }
            })
        },
        close: {
            let ui = ui.clone();
            let host = host.clone();
            Rc::new(move || match save_draft(&ui.shared) {
                Ok(()) => {
                    if let Some(host) = &host {
                        (host.close)()
                    }
                }
                Err(e) => note(&ui.shared, format!("Draft: {e:#}")),
            })
        },
    };
    let back = ui.clone();
    let closing = actions.close.clone();
    cranpose::BackHandler(
        host.is_some() || ui.screen.get() != Screen::Projects || ui.sheet.get() != Sheet::None,
        move || {
            if back.sheet.get_non_reactive() != Sheet::None
                || back.screen.get_non_reactive() != Screen::Projects
            {
                back.back();
            } else {
                closing();
            }
        },
    );
    let hosted = host.is_some();
    cranpose_core::CompositionLocalProvider(
        vec![
            controls::hint_context().provides(Some(ui.hints)),
            design::palette_local()
                .provides((bottom_inset > 0.0).then(cranpose::liquid::liquid_colors)),
        ],
        move || {
            BoxWithConstraints(
                Modifier::empty().fill_max_size().background(studio_bg()),
                move |scope| {
                    Workspace(
                        ui.clone(),
                        actions.clone(),
                        Layout::new(
                            scope.constraints().max_width,
                            (scope.constraints().max_height - bottom_inset).max(1.0),
                        ),
                        hosted,
                        busy.get(),
                    );
                },
            );
        },
    );
}
fn save_file(
    ui: &Ui,
    pending: cranpose_core::MutableState<Option<(Vec<u8>, bool, u64)>>,
    launcher: cranpose_services::SaveDocumentLauncher,
    project: bool,
) {
    let result = {
        let mut doc = ui.shared.lock().unwrap();
        let bytes = if project {
            doc.project_bytes()
        } else {
            doc.export_archive()
        };
        bytes.map(|bytes| (bytes, doc.revision))
    };
    match result {
        Ok((bytes, revision)) => {
            #[cfg(all(target_os = "macos", feature = "store"))]
            {
                let shared = ui.shared.clone();
                cranpose_core::spawn_ui_task(async move {
                    if let Some(file) = rfd::AsyncFileDialog::new()
                        .set_file_name(if project { "Skin.cstudio" } else { "Skin.wsz" })
                        .add_filter(
                            if project {
                                "Studio project"
                            } else {
                                "WSZ skin"
                            },
                            &[if project { "cstudio" } else { "wsz" }],
                        )
                        .save_file()
                        .await
                    {
                        let result = file.write(&bytes).await.map_err(|e| e.to_string());
                        finish_save(&shared, project, revision, result);
                    }
                });
                let _ = (pending, launcher);
            }
            #[cfg(not(all(target_os = "macos", feature = "store")))]
            {
                pending.set(Some((bytes, project, revision)));
                launcher.launch(cranpose::SaveDocumentRequest::new(
                    if project { "Skin.cstudio" } else { "Skin.wsz" },
                    "application/zip",
                ));
            }
        }
        Err(e) => note(&ui.shared, format!("Save: {e:#}")),
    }
}
fn finish_save(shared: &SharedDocument, project: bool, revision: u64, result: Result<(), String>) {
    let mut doc = shared.lock().unwrap();
    match result {
        Ok(()) if project && doc.revision == revision => {
            doc.mark_project_saved("Saved Studio project".into())
        }
        outcome => {
            let message = match outcome {
                Ok(()) if project => "Saved project copy; newer edits remain unsaved".into(),
                Ok(()) => "Exported WSZ skin".into(),
                Err(error) => format!("Save: {error}"),
            };
            doc.notify(message);
            doc.revision += 1;
        }
    }
}
fn sheet_area(ui: &Ui, layout: Layout, sheet: Sheet) -> Area {
    #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
    if sheet == Sheet::Advanced(11) {
        let width = (layout.width - 16.).min(408.);
        let height = (layout.height - 72.).clamp(1., 688.);
        return Area::new(layout.width - width - 8., 64., width, height);
    }
    let mut panel = layout.panel(matches!(sheet, Sheet::Advanced(_)) && layout.inspector.is_none());
    if layout.inspector.is_none() {
        let needed = match sheet {
            Sheet::Parts => Some(
                panels::PARTS
                    .len()
                    .div_ceil(panels::part_columns(layout.width, true)) as f32
                    * 72.,
            ),
            Sheet::Tools => Some(panels::tool_height(10, layout.width, layout.height < 500.)),
            Sheet::Colour => Some(colour::height(
                ui.shared.lock().unwrap().view.swatches.len(),
                layout.width,
            )),
            Sheet::Layers | Sheet::Advanced(7) => Some(layers::height(
                &ui.shared.lock().unwrap().paint_layer_info(),
            )),
            _ => None,
        };
        if let Some(needed) = needed {
            let height = (needed + 48.).min(layout.height);
            panel = Area::new(0., layout.height - height, layout.width, height);
        }
    }
    if sheet == Sheet::Advanced(4) && layout.inspector.is_none() {
        let needed = properties::height(&ui.shared.lock().unwrap().view, layout.width) + 48.;
        let height = needed.min(layout.height);
        panel = Area::new(0., layout.height - height, layout.width, height);
    }
    if sheet == Sheet::Advanced(2) && layout.inspector.is_none() {
        let entries = ui.shared.lock().unwrap().history()["entries"]
            .as_array()
            .map_or(0, Vec::len);
        let height = (92. + entries as f32 * 52.).min(layout.height);
        panel = Area::new(0., layout.height - height, layout.width, height);
    }
    if sheet == Sheet::Tools && layout.inspector.is_some() {
        let view = ui.shared.lock().unwrap().view.clone();
        let visible = workbench::tool_items(layout, &view.brush);
        let hidden = 10usize.saturating_sub(
            visible
                .iter()
                .filter(|item| !matches!(item.0, "pan" | "tools"))
                .count(),
        );
        let h = (48. + panels::tool_height(hidden, 304., false)).min(layout.height - 80.);
        panel = Area::new(
            layout.tools.x + layout.tools.w + 8.,
            (layout.tools.y + visible.len() as f32 * 44. - h).max(56.),
            304.,
            h,
        );
    }
    panel
}

#[composable]
fn Workspace(ui: Ui, actions: Actions, layout: Layout, hosted: bool, busy: bool) {
    let layout = if ui.hand.get() && matches!(ui.screen.get(), Screen::Canvas | Screen::Sprites) {
        layout.for_navigation()
    } else {
        layout
    };

    let layout_state = ui.layout;
    if layout_state.get_non_reactive() != layout {
        cranpose_core::SideEffect(move || layout_state.set(layout));
    }
    let screen = ui.screen.get();
    let requested_sheet = ui.sheet.get();
    let sheet = if layout.rail.is_some() && requested_sheet == Sheet::Parts {
        let next = ui.clone();
        cranpose_core::SideEffect(move || next.open_sheet(Sheet::None));
        Sheet::None
    } else {
        requested_sheet
    };
    let panel = sheet_area(&ui, layout, sheet);
    let covered =
        busy || (sheet != Sheet::None && (layout.inspector.is_none() || sheet == Sheet::Tools));
    let covered_hints = cranpose_core::rememberMutableStateOf(|| None);
    let revision = ui.tick.get();
    // The player acknowledges after its skin reload. Other workspaces compose
    // the document directly and must also release pending GPU captures.
    if screen != Screen::Preview {
        cranpose_core::SideEffect(move || {
            super::COMPOSED_REVISION.fetch_max(revision, std::sync::atomic::Ordering::Release);
        });
    }
    let (name, message, dirty) = {
        let doc = ui.shared.lock().unwrap();
        (
            doc.path
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_stem())
                .and_then(|p| p.to_str())
                .unwrap_or("Working copy")
                .to_string(),
            doc.notice.clone(),
            doc.dirty,
        )
    };
    Box(
        Modifier::empty()
            .size_points(layout.width, layout.height)
            .background(studio_bg())
            .content_description(format!("Skin Studio {} layout", layout.class.label())),
        BoxSpec::default(),
        move || {
            let mut base = Modifier::empty()
                .size_points(layout.width, layout.height)
                .then(motion::entrance(screen as u64, 0.));
            if covered {
                base = base.hide_from_accessibility();
            }
            Box(base, BoxSpec::default(), {
                let mut ui = ui.clone();
                if covered {
                    ui.hints = covered_hints;
                }
                let actions = actions.clone();
                let name = name.clone();
                move || {
                    let ui = ui.clone();
                    let actions = actions.clone();
                    let name = name.clone();
                    cranpose_core::CompositionLocalProvider(
                        vec![controls::hint_context().provides(Some(ui.hints))],
                        move || {
                            if matches!(screen, Screen::Canvas | Screen::Sprites | Screen::States) {
                                let inspector = if sheet == Sheet::Advanced(11) {
                                    None
                                } else {
                                    layout.inspector
                                };
                                for area in [layout.rail, inspector, layout.references]
                                    .into_iter()
                                    .flatten()
                                {
                                    Box(
                                        Modifier::empty()
                                            .absolute_offset(area.x + 2., area.y + 2.)
                                            .size_points(area.w - 4., area.h - 4.)
                                            .then(design::surface(
                                                design::Surface::Panel,
                                                false,
                                                false,
                                                false,
                                            )),
                                        BoxSpec::default(),
                                        || {},
                                    );
                                }
                            }
                            Header(ui.clone(), layout, name.clone(), dirty);
                            match screen {
                                Screen::Projects => {
                                    Projects(ui.clone(), actions.clone(), layout, hosted)
                                }
                                Screen::Canvas | Screen::States | Screen::Sprites => {
                                    if let Some(rail) = layout.rail {
                                        if screen == Screen::States {
                                            desktop::StateDetails(ui.clone(), rail);
                                        } else if screen == Screen::Sprites {
                                            desktop::Sources(ui.clone(), rail);
                                        } else if layout.class == WidthClass::Desktop {
                                            desktop::Parts(ui.clone(), rail, true);
                                        } else {
                                            PartList(ui.clone(), rail, false);
                                        }
                                    }
                                    workbench::ContextBar(ui.clone(), layout);
                                    if let Some(reference) = layout.references {
                                        previews::StateReferences(ui.clone(), reference);
                                    }
                                    let mut canvas = layout.canvas;
                                    if screen == Screen::States {
                                        canvas.h += layout.properties.map_or(0., |p| p.h)
                                            + if layout.side_tools {
                                                0.
                                            } else {
                                                layout.tools.h
                                            };
                                        if layout.side_tools {
                                            canvas.x -= layout.tools.w;
                                            canvas.w += layout.tools.w;
                                        }
                                    }
                                    if sheet != Sheet::None
                                        && sheet != Sheet::Advanced(11)
                                        && layout.inspector.is_none()
                                        && layout.class != WidthClass::SmallPhone
                                        && layout.height >= 500.
                                    {
                                        canvas.h = (panel.y - canvas.y).max(1.);
                                    }
                                    EditorCanvas(
                                        ui.clone(),
                                        canvas,
                                        !busy
                                            && (sheet == Sheet::None
                                                || (layout.inspector.is_some()
                                                    && sheet != Sheet::Tools)),
                                        screen == Screen::States,
                                        layout.class == WidthClass::Desktop,
                                    );
                                    if screen != Screen::States {
                                        workbench::Tools(ui.clone(), layout);
                                        if let Some(properties) = layout.properties {
                                            quick::Properties(ui.clone(), properties);
                                        }
                                    }
                                    if let Some(inspector) = layout.inspector {
                                        if sheet == Sheet::None || sheet == Sheet::Tools {
                                            desktop::Inspector(
                                                ui.clone(),
                                                inspector,
                                                screen,
                                                revision,
                                                layout.class == WidthClass::Desktop,
                                            );
                                        }
                                    }
                                    let y = canvas.y + canvas.h;
                                    for (i, (icon, label)) in [
                                        (Icon::Minus, "Zoom out"),
                                        (Icon::Fit, "Fit canvas"),
                                        (Icon::Add, "Zoom in"),
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    {
                                        let viewport = ui.viewport;
                                        Control(
                                            icon,
                                            label.into(),
                                            "Zoom the canvas".into(),
                                            Area::new(
                                                canvas.x + canvas.w - 132. + i as f32 * 44.,
                                                y,
                                                44.,
                                                44.,
                                            ),
                                            false,
                                            true,
                                            false,
                                            ui.hints,
                                            move || {
                                                viewport.update(|v| {
                                                    if i == 1 {
                                                        v.fit();
                                                    } else {
                                                        v.transform(
                                                            cranpose_ui_graphics::Point::new(
                                                                v.room[0] / 2.,
                                                                v.room[1] / 2.,
                                                            ),
                                                            cranpose_ui_graphics::Point::ZERO,
                                                            if i == 0 { 0.8 } else { 1.25 },
                                                        );
                                                    }
                                                })
                                            },
                                        );
                                    }
                                    let palette = layout.inspector.is_none()
                                        && screen != Screen::States
                                        && !ui.hand.get()
                                        && workbench::uses_colour(
                                            &ui.shared.lock().unwrap().view.brush,
                                        );
                                    if palette {
                                        quick::Palette(
                                            ui.clone(),
                                            Area::new(canvas.x + 4., y, canvas.w - 140., 44.),
                                        );
                                    } else if ui.hand.get()
                                        && layout.inspector.is_none()
                                        && screen != Screen::States
                                    {
                                        quick::NavigationBar(
                                            ui.clone(),
                                            Area::new(canvas.x + 4., y, canvas.w - 140., 44.),
                                        );
                                    } else {
                                        Label(
                                            format!("{:.0}%", ui.viewport.get().scale * 100.),
                                            canvas.x + 12.,
                                            y + 14.,
                                            80.,
                                            12.,
                                            studio_dim(),
                                        );
                                    }
                                }
                                Screen::Skin => skin::Screen(ui.clone(), layout),
                                Screen::Preview => Preview(ui.clone(), layout, revision),
                                Screen::Save => {
                                    SaveScreen(ui.clone(), actions.clone(), layout, hosted)
                                }
                                Screen::Help => help::HelpScreen(ui.clone(), layout),
                                Screen::ConfirmBlank => ConfirmBlank(ui.clone(), layout),
                            }
                        },
                    );
                }
            });
            if sheet != Sheet::None {
                if layout.inspector.is_none() || sheet == Sheet::Tools {
                    let close = ui.clone();
                    Box(
                        Modifier::empty()
                            .absolute_offset(0., 0.)
                            .size_points(layout.width, layout.height - 24.)
                            .background(Color(0., 0., 0., 0.45))
                            .clickable(move |_| close.open_sheet(Sheet::None)),
                        BoxSpec::default(),
                        || {},
                    );
                }
                SheetPanel(ui.clone(), panel, sheet, revision);
            }
            notification::Notification(message.clone(), layout.width, layout.height);
            HintOverlay(ui.hints, layout.width, layout.height);
            if busy {
                Box(
                    Modifier::empty()
                        .fill_max_size()
                        .background(Color(0., 0., 0., 0.7))
                        .clickable(|_| {}),
                    BoxSpec::default(),
                    move || {
                        Label(
                            "Preparing document...".into(),
                            24.,
                            layout.height / 2.,
                            layout.width - 48.,
                            18.,
                            studio_fg(),
                        )
                    },
                );
            }
        },
    );
}
#[composable]
fn Header(ui: Ui, layout: Layout, name: String, dirty: bool) {
    let _ = ui.tick.get();
    let screen = ui.screen.get();
    let title = match screen {
        Screen::Projects => "Skin Studio",
        Screen::Canvas => &name,
        Screen::Preview => "Preview",
        Screen::States => "Sprite states",
        Screen::Sprites => "Sprite sources",
        Screen::Skin => "Skin settings",
        Screen::Save => "Save and share",
        Screen::Help => "Studio help",
        Screen::ConfirmBlank => "New project",
    };
    let button = ui.clone();
    Control(
        if screen == Screen::Projects {
            Icon::Help
        } else {
            Icon::Back
        },
        if screen == Screen::Projects {
            "Studio help"
        } else {
            "Back"
        }
        .into(),
        if screen == Screen::Projects {
            "Instructions for editing and saving"
        } else {
            ""
        }
        .into(),
        Area::new(4., 6., 44., 44.),
        false,
        true,
        false,
        ui.hints,
        move || {
            if screen == Screen::Projects {
                button.show(Screen::Help)
            } else {
                button.back()
            }
        },
    );
    let compact_header = layout.class == WidthClass::SmallPhone;
    let short_header = layout.height < 500. && layout.class != WidthClass::Desktop;
    let title_width = if layout.class == WidthClass::Desktop {
        240.
    } else {
        (layout.width
            - if compact_header { 152. } else { 196. }
            - if short_header { 48. } else { 0. })
        .max(24.)
    };
    if !matches!(
        screen,
        Screen::Projects | Screen::ConfirmBlank | Screen::Help | Screen::Save
    ) {
        desktop::ScopeSelector(ui.clone(), layout);
    }
    desktop::RowLabel(
        format!(
            "{}{}",
            title,
            if dirty && screen == Screen::Canvas {
                " *"
            } else {
                ""
            }
        ),
        if short_header { 104. } else { 56. },
        10.,
        title_width,
        14.,
        studio_fg(),
    );
    if matches!(
        screen,
        Screen::Canvas | Screen::Sprites | Screen::States | Screen::Skin
    ) {
        let panel = ui.shared.lock().unwrap().view.panel.clone();
        let scope_name = if screen == Screen::Skin {
            name.clone()
        } else if screen == Screen::States
            || (screen == Screen::Sprites && ui.sprite_crop.get().is_some())
        {
            ui.sprite_id.get()
        } else if screen == Screen::Sprites {
            ui.shared.lock().unwrap().view.sheet.clone()
        } else {
            part_name(&panel).into()
        };
        let scope_name = if screen == Screen::Canvas && layout.class != WidthClass::Desktop {
            let v = ui.shared.lock().unwrap().view.clone();
            format!(
                "{}, {}",
                if v.active { "Active" } else { "Inactive" },
                if v.pressed { "pressed" } else { "released" }
            )
        } else {
            scope_name
        };
        desktop::RowLabel(
            scope_name,
            if short_header { 104. } else { 56. },
            30.,
            title_width,
            10.,
            studio_dim(),
        );
        let (undo, redo) = ui.shared.lock().unwrap().history_depth();
        let header_items = vec![
            (Icon::Undo, "Undo", undo > 0),
            (Icon::Redo, "Redo", redo > 0),
            (Icon::More, "Project actions", true),
        ];
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        let header_items = {
            let mut items = header_items;
            if layout.class == WidthClass::Desktop {
                items.push((Icon::Agent, "Agent connection", true));
            }
            items
        };
        let slots = header_items.len() - usize::from(compact_header);
        let mut slot = 0;
        for (i, (icon, label, enabled)) in header_items.into_iter().enumerate() {
            if compact_header && i == 1 {
                continue;
            }
            let x = layout.width - 4. - slots as f32 * 44. + slot as f32 * 44.;
            slot += 1;
            let ui = ui.clone();
            #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
            let selected =
                i == 3 && (agent_server::status().running || ui.sheet.get() == Sheet::Advanced(11));
            #[cfg(any(target_os = "ios", target_os = "android", target_arch = "wasm32"))]
            let selected = false;
            Control(
                icon,
                label.into(),
                match i {
                    0 => "Undo the last edit",
                    1 => "Redo the last undone edit",
                    2 => "Project tools and help",
                    _ => "Connect an MCP agent to this canvas",
                }
                .into(),
                Area::new(x, 6., 44., 44.),
                selected,
                enabled,
                false,
                ui.hints,
                move || {
                    if i == 3 {
                        ui.open_sheet(Sheet::Advanced(11));
                    } else if i == 2 {
                        ui.open_sheet(Sheet::More);
                    } else {
                        let mut doc = ui.shared.lock().unwrap();
                        if i == 0 {
                            doc.undo();
                        } else {
                            doc.redo();
                        }
                        ui.tick.set(doc.revision);
                    }
                },
            );
        }
    } else if screen == Screen::Preview {
        let ui = ui.clone();
        Control(
            Icon::Share,
            "Save and share".into(),
            "Save the layered project or export a skin".into(),
            Area::new(layout.width - 56., 8., 48., 48.),
            false,
            true,
            false,
            ui.hints,
            move || ui.show(Screen::Save),
        );
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/fit_tests.rs"]
mod fit_tests;
