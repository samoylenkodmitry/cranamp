use super::*;
pub(super) const PARTS: [(&str, &str, f32, f32); 7] = [
    ("main", "Player", 0., 116.),
    ("equalizer", "Equalizer", 116., 116.),
    ("playlist", "Playlist", 232., 145.),
    ("shade", "Windowshade", 0., 14.),
    ("cursors", "Cursors", 0., 0.),
    ("footer", "Playlist footer", 339., 38.),
    ("canvas", "Joined canvas", 0., 377.),
];
pub(super) fn part_name(part: &str) -> &'static str {
    PARTS
        .iter()
        .find(|p| p.0 == part)
        .map(|p| p.1)
        .unwrap_or("Sprite atlas")
}
pub(super) fn part_columns(width: f32, grid: bool) -> usize {
    if grid && width >= 600. {
        3
    } else if grid && width >= 380. {
        2
    } else {
        1
    }
}

pub(super) fn tool_columns(width: f32) -> usize {
    ((width / 140.).floor() as usize).clamp(2, 6)
}

pub(super) fn tool_height(count: usize, width: f32, short: bool) -> f32 {
    count.div_ceil(tool_columns(width)).max(1) as f32 * 80. + if short { 152. } else { 104. }
}
#[composable]
pub(super) fn ScrollArea(area: Area, height: f32, content: impl FnMut() + 'static) {
    let scroll = cranpose_ui::rememberScrollState!(0.);
    let content = Rc::new(std::cell::RefCell::new(content));
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            let content = content.clone();
            Box(
                Modifier::empty().size_points(area.w, height.max(area.h)),
                BoxSpec::default(),
                move || (content.borrow_mut())(),
            );
        },
    );
    if height > area.h {
        cranpose_ui::widgets::Scrollbar(
            Modifier::empty()
                .absolute_offset(area.x + area.w - 6., area.y)
                .size_points(6., area.h)
                .content_description("Scroll panel")
                .then(controls::feedback("Scroll this panel".into())),
            scroll,
            cranpose_ui_layout::Axis::Vertical,
            cranpose_ui::widgets::ScrollbarSpec::default()
                .thickness(6.)
                .colors(cranpose_ui::widgets::ScrollbarColors {
                    track: CARD,
                    thumb: DIM,
                    dragged_thumb: ACCENT_LIT,
                }),
        );
    }
}
#[composable]
fn RowAction(
    icon: Icon,
    label: String,
    detail: &'static str,
    area: Area,
    hints: Hints,
    action: impl Fn() + 'static,
) {
    desktop::ListRow(
        Some(icon),
        label,
        detail.into(),
        String::new(),
        area,
        false,
        hints,
        action,
    );
}
#[composable]
pub(super) fn Projects(ui: Ui, actions: Actions, layout: Layout, hosted: bool) {
    let _ = ui.tick.get();
    let wide = layout.width >= 600.;
    let width = (layout.width - 32.).min(880.);
    let card_w = if wide {
        (width * 0.46).min(360.)
    } else {
        width
    };
    let card_h = if wide {
        (layout.height - 128.).clamp(220., 500.)
    } else {
        card_w / 275. * 116. + 48.
    };
    let action_x = if wide { card_w + 32. } else { 0. };
    let action_y = if wide { 32. } else { card_h + 16. };
    let action_w = if wide { width - action_x } else { width };
    ScrollArea(
        Area::new(
            (layout.width - width) / 2.,
            72.,
            width,
            layout.height - 104.,
        ),
        (action_y + 280.).max(card_h),
        move || {
            let image = ui.shared.lock().unwrap().skin_render();
            let h = if wide { image.height() as f32 } else { 116. };
            let bitmap = ImageBitmap::from_rgba8(image.width(), image.height(), image.into_raw())
                .expect("skin thumbnail");
            let next = ui.clone();
            let feedback = controls::feedback("Continue editing".into());
            Button(
                Modifier::empty()
                    .size_points(card_w, card_h)
                    .then(design::surface(
                        design::Surface::Button,
                        false,
                        false,
                        false,
                    ))
                    .content_description("Open working copy")
                    .then(feedback),
                ButtonSpec::default(),
                move || {
                    next.show(Screen::Canvas);
                    next.fit_requested.set(true);
                },
                move || {
                    let bitmap = bitmap.clone();
                    Box(
                        Modifier::empty().size_points(card_w, card_h),
                        BoxSpec::default(),
                        move || {
                            cranpose_ui::Image(
                                cranpose_ui::BitmapRegionPainter(
                                    bitmap.clone(),
                                    Rect {
                                        x: 0.,
                                        y: 0.,
                                        width: 275.,
                                        height: h,
                                    },
                                    cranpose_ui::ImageSampling::Nearest,
                                ),
                                Some("Working skin".into()),
                                Modifier::empty()
                                    .absolute_offset(8., 8.)
                                    .size_points(card_w - 16., card_h - 56.),
                                Alignment::CENTER,
                                cranpose_ui::ContentScale::Fit,
                                1.,
                                None,
                            );
                            Label(
                                "Continue editing".into(),
                                16.,
                                card_h - 34.,
                                card_w - 32.,
                                14.,
                                FG,
                            );
                        },
                    );
                },
            );
            let open = actions.open.clone();
            RowAction(
                Icon::Open,
                "Open skin or project".into(),
                "WSZ, ZIP or CSTUDIO",
                Area::new(action_x, action_y, action_w, 52.),
                ui.hints,
                move || open(),
            );
            let next = ui.clone();
            RowAction(
                Icon::Add,
                "New project".into(),
                "Start a blank skin",
                Area::new(action_x, action_y + 60., action_w, 52.),
                ui.hints,
                move || next.show(Screen::ConfirmBlank),
            );
            let restore = actions.restore.clone();
            RowAction(
                Icon::History,
                "Restore last draft".into(),
                "Open the saved draft",
                Area::new(action_x, action_y + 120., action_w, 52.),
                ui.hints,
                move || restore(),
            );
            if hosted {
                let close = actions.close.clone();
                RowAction(
                    Icon::Play,
                    "Return to player".into(),
                    "Return to playback",
                    Area::new(action_x, action_y + 180., action_w, 52.),
                    ui.hints,
                    move || close(),
                );
            }
            if wide {
                Label(
                    "CSTUDIO keeps layers.\nWSZ works in players.".into(),
                    action_x + 12.,
                    action_y + 248.,
                    action_w - 24.,
                    12.,
                    DIM,
                );
            }
        },
    );
}
#[composable]
pub(super) fn PartList(ui: Ui, area: Area, grid: bool) {
    let revision = ui.tick.get();
    let d = ui.shared.clone();
    let bitmap = cranpose_core::rememberKeyed((d.clone(), revision), move |_| {
        let image = d.lock().unwrap().skin_render();
        ImageBitmap::from_rgba8(image.width(), image.height(), image.into_raw())
            .expect("skin thumbnail")
    });
    let selected = ui.shared.lock().unwrap().view.panel.clone();
    let columns = part_columns(area.w, grid);
    let cell = area.w / columns as f32;
    ScrollArea(
        area,
        PARTS.len().div_ceil(columns) as f32 * 72.,
        move || {
            for (i, (part, label, y, h)) in PARTS.into_iter().enumerate() {
                let next = ui.clone();
                let bitmap = bitmap.clone();
                let on = selected == part;
                let feedback = controls::feedback(format!("Edit {label}"));
                Button(
                    Modifier::empty()
                        .absolute_offset((i % columns) as f32 * cell, (i / columns) as f32 * 72.)
                        .size_points(cell - 4., 68.)
                        .then(design::surface(design::Surface::Row, on, false, false))
                        .then(feedback)
                        .content_description(format!("Edit {label}")),
                    ButtonSpec::default(),
                    move || next.select_part(part),
                    move || {
                        let bitmap = bitmap.clone();
                        Box(
                            Modifier::empty().size_points(cell - 4., 68.),
                            BoxSpec::default(),
                            move || {
                                if h > 0. {
                                    cranpose_ui::Image(
                                        cranpose_ui::BitmapRegionPainter(
                                            bitmap.clone(),
                                            Rect {
                                                x: 0.,
                                                y,
                                                width: 275.,
                                                height: h,
                                            },
                                            cranpose_ui::ImageSampling::Nearest,
                                        ),
                                        None,
                                        Modifier::empty()
                                            .absolute_offset(8., 10.)
                                            .size_points(68., 48.),
                                        Alignment::CENTER,
                                        cranpose_ui::ContentScale::Fit,
                                        1.,
                                        None,
                                    );
                                } else {
                                    cranpose_ui::Canvas(
                                        Modifier::empty()
                                            .absolute_offset(30., 22.)
                                            .size_points(24., 24.),
                                        move |scope| controls::draw_icon(scope, Icon::Hand, FG),
                                    );
                                }
                                Label(
                                    label.into(),
                                    84.,
                                    16.,
                                    cell - 92.,
                                    12.,
                                    if on { ACCENT_LIT } else { FG },
                                );
                                if h > 0. {
                                    Label(
                                        format!("275 × {}", h as u32),
                                        84.,
                                        38.,
                                        cell - 92.,
                                        10.,
                                        DIM,
                                    );
                                }
                            },
                        );
                    },
                );
            }
        },
    );
}
#[composable]
pub(super) fn ToolSettings(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let view = ui.shared.lock().unwrap().view.clone();
    let columns = tool_columns(area.w);
    let cell = area.w / columns as f32;
    let hidden: Vec<_> = workbench::BRUSHES
        .into_iter()
        .chain([
            ("glass", "Glass lens", Icon::Glass),
            ("tuft", "Taper", Icon::Taper),
        ])
        .filter(|(key, _, _)| {
            ui.layout.get().inspector.is_none()
                || !workbench::tool_items(ui.layout.get(), &view.brush)
                    .iter()
                    .any(|b| b.0 == *key)
        })
        .collect();
    let rows = hidden.len().div_ceil(columns).max(1);
    let row_h = 80.;
    let short = ui.layout.get().height < 500. && ui.layout.get().inspector.is_none();
    let content_height = tool_height(hidden.len(), area.w, short);
    let revision = ui.tick.get();
    let shared = ui.shared.clone();
    let keys = hidden.iter().map(|item| item.0).collect::<Vec<_>>();
    let samples =
        cranpose_core::rememberKeyed((shared.clone(), revision, keys.clone()), move |_| {
            let doc = shared.lock().unwrap();
            keys.iter()
                .map(|key| previews::bitmap(doc.tool_sample_for(key)))
                .collect::<Vec<_>>()
        });
    ScrollArea(area, content_height, move || {
        for (i, (key, name, icon)) in hidden.iter().copied().enumerate() {
            let next = ui.clone();
            let x = (i % columns) as f32 * cell;
            let y = (i / columns) as f32 * row_h;
            let sample = samples[i].clone();
            previews::ToolCard(
                ui.clone(),
                icon,
                name.into(),
                sample,
                Area::new(x + 4., y, cell - 8., row_h - 4.),
                view.brush == key,
                move || {
                    state(&next.shared, json!({"brush":key}));
                    next.hand.set(false);
                    next.open_sheet(Sheet::None);
                },
            );
        }
        let y = rows as f32 * row_h + 8.;
        for (i, (icon, label, sheet)) in [(Icon::Grid, "Guides", Sheet::Advanced(6))]
            .into_iter()
            .enumerate()
        {
            let next = ui.clone();
            RowAction(
                icon,
                label.into(),
                label,
                Area::new(0., y + i as f32 * 48., area.w, 44.),
                ui.hints,
                move || next.open_sheet(sheet),
            );
        }
        let shared = ui.shared.clone();
        desktop::ListRow(
            Some(Icon::Grid),
            "Pixel grid".into(),
            "Show pixel boundaries".into(),
            if view.grid { "On" } else { "Off" }.into(),
            Area::new(0., y + 48., area.w, 44.),
            view.grid,
            ui.hints,
            move || state(&shared, json!({"grid":!view.grid})),
        );
        if short {
            let next = ui.clone();
            RowAction(
                Icon::Settings,
                "Tool settings".into(),
                "Current tool properties",
                Area::new(0., y + 96., area.w, 44.),
                ui.hints,
                move || next.open_sheet(Sheet::Advanced(4)),
            );
        }
    });
}
#[composable]
pub(super) fn SheetPanel(ui: Ui, area: Area, sheet: Sheet, revision: u64) {
    let title = match sheet {
        Sheet::Scopes => "Workspaces",
        Sheet::Parts => "Choose a part",
        Sheet::Targets => "Sprites",
        Sheet::Sources => "Source bitmaps",
        Sheet::Tools => "Drawing tools",
        Sheet::Colour => "Brush colour",
        Sheet::CanvasState => "Canvas state",
        Sheet::Layers => "Painting layers",
        Sheet::More => "Project",
        Sheet::Advanced(4) => "Tool properties",
        Sheet::Advanced(7) => "Painting layers",
        Sheet::Advanced(1) => "Sprite targets",
        Sheet::Advanced(2) => "Edit history",
        Sheet::Advanced(3) => "Source bitmaps",
        Sheet::Advanced(5) => "Pixel study",
        Sheet::Advanced(6) => "Guides",
        Sheet::Advanced(9) => "Skin settings",
        Sheet::Advanced(10) => "Equalizer",
        Sheet::Advanced(11) => "MCP connection",
        Sheet::Advanced(_) => "Settings",
        Sheet::None => "",
    };
    let enter = motion::entrance(
        title
            .bytes()
            .fold(0u64, |key, c| key.wrapping_mul(31).wrapping_add(c as u64)),
        6.,
    );
    let heading = if sheet == Sheet::Advanced(4) {
        let brush = ui.shared.lock().unwrap().view.brush.clone();
        let name = match brush.as_str() {
            "pencil" => "Pencil",
            "line" => "Line",
            "rect" => "Rectangle",
            "ellipse" => "Ellipse",
            "lift" => "Lift pixels",
            "stamp" => "Stamp",
            "text" => "Text",
            "curve" => "Curve",
            "glass" => "Glass lens",
            "tuft" => "Taper",
            _ => "Tool",
        };
        format!("{name} settings")
    } else {
        title.to_owned()
    };
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .then(design::surface(design::Surface::Panel, false, false, false))
            .clickable(|_| {})
            .content_description(title)
            .then(enter),
        BoxSpec::default(),
        move || {
            desktop::RowLabel(heading.clone(), 12., 16., area.w - 64., 16., FG);
            let close = ui.clone();
            Control(
                Icon::Close,
                "Close panel".into(),
                "Return to the canvas".into(),
                Area::new(area.w - 48., 4., 44., 40.),
                false,
                true,
                false,
                ui.hints,
                move || close.open_sheet(Sheet::None),
            );
            let body = Area::new(0., 48., area.w, area.h - 48.);
            match sheet {
                Sheet::Scopes => ScopeList(ui.clone(), body),
                Sheet::Parts => PartList(ui.clone(), body, true),
                Sheet::Targets => desktop::SpriteTargets(ui.clone(), body, ui.screen.get()),
                Sheet::Sources => desktop::Sources(ui.clone(), body),
                Sheet::Tools => ToolSettings(ui.clone(), body),
                Sheet::Layers => layers::Panel(ui.clone(), body),
                Sheet::Colour => {
                    let height =
                        colour::height(ui.shared.lock().unwrap().view.swatches.len(), body.w);
                    let ui = ui.clone();
                    ScrollArea(body, height, move || {
                        colour::Editor(ui.clone(), Area::new(0., 0., body.w, height))
                    });
                }
                Sheet::CanvasState => workbench::CanvasSettings(ui.clone(), body),
                Sheet::More => More(ui.clone(), body),
                Sheet::Advanced(id) => Advanced(ui.clone(), body, id, revision),
                Sheet::None => {}
            }
        },
    );
}
#[composable]
fn ScopeList(ui: Ui, area: Area) {
    ScrollArea(area, 240., move || {
        for (i, (screen, name, icon)) in [
            (Screen::Canvas, "Canvas", Icon::Canvas),
            (Screen::Sprites, "Sprites", Icon::Parts),
            (Screen::States, "States", Icon::State),
            (Screen::Skin, "Skin", Icon::Settings),
            (Screen::Preview, "Preview", Icon::Eye),
        ]
        .into_iter()
        .enumerate()
        {
            let next = ui.clone();
            desktop::ListRow(
                Some(icon),
                name.into(),
                String::new(),
                String::new(),
                Area::new(8., i as f32 * 48., area.w - 16., 44.),
                ui.screen.get() == screen,
                ui.hints,
                move || workbench::switch_scope(&next, screen),
            );
        }
    });
}

#[composable]
fn More(ui: Ui, area: Area) {
    let items = [
        ("Redo", Icon::Redo, 16),
        ("Sprite targets", Icon::Select, 1),
        ("Edit history", Icon::History, 2),
        ("Source bitmaps", Icon::Parts, 3),
        ("Pixel study", Icon::Eye, 5),
        ("Sprite rectangles", Icon::Grid, 6),
        ("EQ workbench", Icon::Settings, 10),
        ("Studio help", Icon::Help, 13),
    ];
    let layout = ui.layout.get();
    let screen = ui.screen.get();
    let items: Vec<_> = items
        .into_iter()
        .filter(|(_, _, id)| match id {
            16 => layout.class == WidthClass::SmallPhone,
            15 => {
                matches!(layout.class, WidthClass::SmallPhone | WidthClass::Phone)
                    && screen != Screen::Sprites
            }
            14 => {
                matches!(layout.class, WidthClass::SmallPhone | WidthClass::Phone)
                    && screen != Screen::States
            }
            3 => layout.rail.is_none(),
            1 => layout.inspector.is_none() || screen == Screen::Canvas,
            _ => true,
        })
        .collect();
    ScrollArea(area, items.len() as f32 * 48. + 64., move || {
        for (i, (label, icon, id)) in items.iter().copied().enumerate() {
            let ui = ui.clone();
            RowAction(
                icon,
                label.into(),
                label,
                Area::new(8., i as f32 * 48., area.w - 16., 44.),
                ui.hints,
                move || match id {
                    16 => {
                        let mut doc = ui.shared.lock().unwrap();
                        doc.redo();
                        ui.tick.set(doc.revision);
                    }
                    13 => ui.show(Screen::Help),
                    14 => workbench::switch_scope(&ui, Screen::States),
                    15 => workbench::switch_scope(&ui, Screen::Sprites),
                    1 => ui.open_sheet(Sheet::Targets),
                    3 => ui.open_sheet(Sheet::Sources),
                    _ => ui.open_sheet(Sheet::Advanced(id)),
                },
            );
        }
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        if layout.class != WidthClass::Desktop {
            let next = ui.clone();
            RowAction(
                Icon::Agent,
                "Agent connection".into(),
                "Connect a local agent to the shared document",
                Area::new(8., items.len() as f32 * 48., area.w - 16., 44.),
                ui.hints,
                move || next.open_sheet(Sheet::Advanced(11)),
            );
        }
    });
}
#[composable]
fn Advanced(ui: Ui, area: Area, id: u8, revision: u64) {
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .clip_to_bounds(),
        BoxSpec::default(),
        move || {
            let room = (area.w - 24., area.h);
            match id {
                1 => LayerChooser(ui.shared.clone(), revision, room),
                2 => History(ui.clone(), Area::new(0., 0., area.w, area.h)),
                4 => workbench::FullProperties(ui.clone(), Area::new(0., 0., room.0, room.1)),
                5 => {
                    let point = cranpose_core::rememberMutableStateOf(|| None);
                    let height = room.1.max(500.);
                    let shared = ui.shared.clone();
                    ScrollArea(Area::new(0., 0., area.w, area.h), height, move || {
                        StudyChooser(shared.clone(), revision, (room.0, height), point);
                    });
                }
                6 => GuideChooser(ui.shared.clone(), revision, room),
                7 => layers::Panel(ui.clone(), Area::new(0., 0., area.w, area.h)),
                9 => skin::Panel(ui.clone(), Area::new(0., 0., area.w, area.h)),
                10 => equalizer::EqualizerChooser(
                    ui.shared.clone(),
                    revision,
                    room,
                    ui.review,
                    ui.live,
                ),
                #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
                11 => agent_connection::Panel(ui.clone(), Area::new(0., 0., area.w, area.h)),
                3 => desktop::Sources(ui.clone(), Area::new(0., 0., area.w, area.h)),
                _ => {}
            }
        },
    );
}
#[composable]
pub(super) fn Preview(ui: Ui, layout: Layout, revision: u64) {
    let desktop = layout.class == WidthClass::Desktop;
    let area = if desktop {
        Area::new(
            0.,
            56.,
            layout.inspector.map_or(layout.width, |a| a.x),
            layout.height - 80.,
        )
    } else {
        Area::new(
            8.,
            if layout.height < 500. { 64. } else { 112. },
            layout.width - 16.,
            layout.height - if layout.height < 500. { 160. } else { 208. },
        )
    };
    if desktop {
        if let Some(inspector) = layout.inspector {
            desktop::PreviewInspector(ui.clone(), inspector, revision);
        }
    }
    let height = ui.shared.lock().unwrap().view.preview_playlist_height;
    let scale = ((area.w - 32.) / 275.)
        .min((area.h - 32.) / (232. + height as f32))
        .clamp(0.25, 8.);
    let origin = presentation_origin(
        [area.w, area.h],
        [275. * scale, (232. + height as f32) * scale],
    );
    let origin = [origin[0] + area.x, origin[1] + area.y];
    note_player_scene(origin, scale);
    let d = ui.shared.clone();
    Box(
        Modifier::empty()
            .absolute_offset(origin[0], origin[1])
            .required_size(cranpose_ui::Size::new(
                275. * scale,
                (232. + height as f32) * scale,
            )),
        BoxSpec::default(),
        move || LivePlayer(d.clone(), revision, scale, height as f32),
    );
    if desktop {
        return;
    }
    for (i, (label, icon, key)) in [
        ("Pressed state", Icon::Hand, "pressed"),
        ("Active window", Icon::Eye, "active"),
    ]
    .into_iter()
    .enumerate()
    {
        let d = ui.shared.clone();
        let on = {
            let d = d.lock().unwrap();
            if i == 0 {
                d.view.pressed
            } else {
                d.view.active
            }
        };
        Control(
            icon,
            label.into(),
            "Inspect the skin in this visual state".into(),
            Area::new(
                layout.width / 2. - 104. + i as f32 * 108.,
                layout.height - 88.,
                100.,
                60.,
            ),
            on,
            true,
            true,
            ui.hints,
            move || state(&d, json!({key:!on})),
        );
    }
}
#[composable]
pub(super) fn SaveScreen(ui: Ui, actions: Actions, layout: Layout, hosted: bool) {
    let width = (layout.width - 32.).min(600.);
    ScrollArea(
        Area::new(
            (layout.width - width) / 2.,
            80.,
            width,
            layout.height - 112.,
        ),
        450.,
        move || {
            Label(
                "Save a project to keep layers.".into(),
                8.,
                4.,
                width - 16.,
                14.,
                DIM,
            );
            let save = actions.project.clone();
            controls::PrimaryAction(
                Icon::Save,
                "Save project".into(),
                Area::new(0., 48., width, 44.),
                move || save(),
            );
            Label(
                "Layers and editable sources (.cstudio)".into(),
                40.,
                94.,
                width - 48.,
                12.,
                DIM,
            );
            let export = actions.export.clone();
            RowAction(
                Icon::Share,
                "Export WSZ skin".into(),
                "Export a player skin",
                Area::new(0., 128., width, 44.),
                ui.hints,
                move || export(),
            );
            Label(
                "Player skin (.wsz)".into(),
                40.,
                174.,
                width - 48.,
                12.,
                DIM,
            );
            if hosted {
                let apply = actions.apply.clone();
                RowAction(
                    Icon::Play,
                    "Apply to player".into(),
                    "Use this skin in the player",
                    Area::new(0., 208., width, 44.),
                    ui.hints,
                    move || apply(),
                );
            }
            let d = ui.shared.clone();
            RowAction(
                Icon::History,
                "Save draft".into(),
                "Keep a copy on this device",
                Area::new(0., if hosted { 288. } else { 208. }, width, 44.),
                ui.hints,
                move || {
                    if let Err(e) = save_draft(&d) {
                        note(&d, format!("Draft: {e:#}"));
                    }
                },
            );
        },
    );
}
#[composable]
pub(super) fn ConfirmBlank(ui: Ui, layout: Layout) {
    Label(
        "The current working copy will be saved as a draft before a blank project opens.".into(),
        24.,
        88.,
        layout.width - 48.,
        15.,
        FG,
    );
    let next = ui.clone();
    RowAction(
        Icon::Add,
        "Create blank project".into(),
        "Save the current working copy first",
        Area::new(24., 200., layout.width - 48., 80.),
        ui.hints,
        move || {
            if let Err(e) = save_draft(&next.shared) {
                note(&next.shared, format!("New project cancelled: {e:#}"));
                return;
            }
            let mut doc = next.shared.lock().unwrap();
            let revision = doc.revision + 1;
            *doc = Document::blank();
            doc.revision = revision;
            doc.view.zoom = 1;
            drop(doc);
            next.viewport.set(Viewport::default());
            next.show(Screen::Canvas);
            next.fit_requested.set(true);
        },
    );
}

#[composable]
fn History(ui: Ui, area: Area) {
    let history = ui.shared.lock().unwrap().history();
    let entries = history["entries"].as_array().cloned().unwrap_or_default();
    ScrollArea(area, 44. + entries.len() as f32 * 52., move || {
        Label(
            "Choose an edit to restore.".into(),
            12.,
            10.,
            area.w - 24.,
            12.,
            DIM,
        );
        for (index, entry) in entries.iter().enumerate() {
            let cursor = entry["cursor"].as_u64().unwrap_or(0) as usize;
            let next = ui.clone();
            desktop::ListRow(
                Some(Icon::History),
                entry["label"].as_str().unwrap_or("Edit").to_owned(),
                entry["source"].as_str().unwrap_or("").to_owned(),
                String::new(),
                Area::new(4., 40. + index as f32 * 52., area.w - 8., 48.),
                entry["current"] == true,
                ui.hints,
                move || {
                    let mut doc = next.shared.lock().unwrap();
                    if let Err(error) = doc.history_goto(cursor) {
                        doc.notify(error.to_string());
                    }
                    next.tick.set(doc.revision);
                },
            );
        }
    });
}
