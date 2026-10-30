use super::*;
use cranpose_ui_graphics::Brush;

const SECTIONS: [(&str, Icon); 6] = [
    ("Playlist", Icon::Colour),
    ("Visualizer", Icon::State),
    ("Text font", Icon::Text),
    ("Window shape", Icon::Select),
    ("Cursors", Icon::Hand),
    ("Format check", Icon::Check),
];
const PLAYLIST: [(&str, &str); 6] = [
    ("Normal", "Track text"),
    ("Current", "Playing track"),
    ("NormalBG", "Background"),
    ("SelectedBG", "Selected row"),
    ("MbFG", "Scroll text"),
    ("MbBG", "Scroll background"),
];

#[composable]
pub(super) fn Screen(ui: Ui, layout: Layout) {
    let top = if layout.class == WidthClass::Desktop || layout.height < 500. {
        64.
    } else {
        112.
    };
    let area = Area::new(8., top, layout.width - 16., layout.height - top - 8.);
    Panel(ui, area);
}

#[composable]
pub(super) fn Panel(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let selected = ui.skin_section.get().min(5);
    let side = area.w >= 880.;
    let nav_w = if side { 168. } else { area.w };
    let columns = if side { 1 } else { 5 };
    let row_h = if side { 48. } else { 52. };
    let nav_h = if side { area.h } else { row_h };
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h),
        BoxSpec::default(),
        move || {
            if side {
                Label(
                    "Skin settings".into(),
                    12.,
                    16.,
                    nav_w - 24.,
                    16.,
                    studio_fg(),
                );
            }
            for (i, (name, icon)) in SECTIONS.into_iter().enumerate() {
                if i == 1 {
                    continue;
                }
                let name = if i == 0 { "Colours" } else { name };
                let position = if i > 1 { i - 1 } else { i };
                let on = selected == i || i == 0 && selected == 1;
                let next = ui.skin_section;
                let cell = nav_w / columns as f32;
                let at = Area::new(
                    (position % columns) as f32 * cell,
                    (position / columns) as f32 * row_h + if side { 48. } else { 0. },
                    cell,
                    row_h,
                );
                if side {
                    desktop::ListRow(
                        Some(icon),
                        name.into(),
                        String::new(),
                        String::new(),
                        at,
                        i == selected || i == 0 && selected == 1,
                        ui.hints,
                        move || next.set(i),
                    );
                } else {
                    let caption = ["Colours", "Font", "Shape", "Cursors", "Check"][position];
                    let feedback = controls::hint_feedback(name.into());
                    Button(
                        Modifier::empty()
                            .absolute_offset(at.x, at.y)
                            .size_points(at.w, at.h)
                            .then(design::surface(design::Surface::Row, on, false, false))
                            .content_description(name)
                            .pointer_icon(control_cursor())
                            .semantics(move |s| s.selected = Some(on))
                            .then(feedback),
                        ButtonSpec::default(),
                        move || next.set(i),
                        move || {
                            Box(
                                Modifier::empty().size_points(at.w, at.h),
                                BoxSpec::default(),
                                move || {
                                    let icon_color =
                                        if on { studio_accent_lit() } else { studio_fg() };
                                    cranpose_ui::Canvas(
                                        Modifier::empty()
                                            .absolute_offset((at.w - 20.) / 2., 5.)
                                            .size_points(20., 20.),
                                        move |scope| controls::draw_icon(scope, icon, icon_color),
                                    );
                                    let mut style = text_style(
                                        10.,
                                        if on { studio_fg() } else { studio_dim() },
                                    );
                                    style.paragraph_style.text_align =
                                        cranpose_ui::text::TextAlign::Center;
                                    Text(
                                        caption,
                                        Modifier::empty().absolute_offset(0., 32.).width(at.w),
                                        style,
                                    );
                                },
                            );
                        },
                    );
                }
            }
            let body = if side {
                Area::new(nav_w + 12., 0., area.w - nav_w - 12., area.h)
            } else {
                Area::new(0., nav_h + 8., area.w, area.h - nav_h - 8.)
            };
            let with_preview = body.w >= 680. && selected >= 2;
            let editor_w = if with_preview {
                (body.w * 0.55).clamp(360., 700.)
            } else {
                body.w
            };
            if with_preview {
                SkinPreview(
                    ui.clone(),
                    Area::new(
                        body.x + editor_w + 12.,
                        body.y,
                        body.w - editor_w - 12.,
                        body.h,
                    ),
                );
            }
            let ui = ui.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(body.x, body.y)
                    .size_points(editor_w, body.h)
                    .then(design::surface(design::Surface::Panel, false, false, false)),
                BoxSpec::default(),
                move || {
                    let inner = Area::new(0., 0., editor_w, body.h);
                    match selected {
                        0 | 1 if side => Colours(ui.clone(), inner),
                        0 | 1 => {
                            for (index, title) in ["Playlist", "Visualizer"].into_iter().enumerate()
                            {
                                let next = ui.skin_section;
                                desktop::ListRow(
                                    None,
                                    title.into(),
                                    String::new(),
                                    String::new(),
                                    Area::new(
                                        8. + index as f32 * (inner.w - 16.) / 2.,
                                        4.,
                                        (inner.w - 16.) / 2.,
                                        40.,
                                    ),
                                    selected == index,
                                    ui.hints,
                                    move || next.set(index),
                                );
                            }
                            Palette(
                                ui.clone(),
                                Area::new(0., 48., inner.w, inner.h - 48.),
                                selected == 1,
                            );
                        }
                        2 => Font(ui.clone(), inner, !with_preview),
                        3 => Windows(ui.clone(), inner),
                        4 => Cursors(ui.clone(), inner),
                        _ => Format(ui.clone(), inner, !with_preview),
                    }
                },
            );
        },
    );
}

#[composable]
fn SkinPreview(ui: Ui, area: Area) {
    let revision = ui.tick.get();
    let height = ui.shared.lock().unwrap().view.preview_playlist_height;
    let scale = ((area.w - 24.) / 275.)
        .min((area.h - 52.) / (232. + height as f32))
        .max(0.25);
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .then(design::surface(design::Surface::Well, false, false, false)),
        BoxSpec::default(),
        move || {
            Label(
                "Skin preview".into(),
                12.,
                16.,
                area.w - 24.,
                13.,
                studio_dim(),
            );
            let d = ui.shared.clone();
            Box(
                Modifier::empty()
                    .absolute_offset((area.w - 275. * scale) / 2., 44.)
                    .size_points(275. * scale, (232. + height as f32) * scale),
                BoxSpec::default(),
                move || LivePlayer(d.clone(), revision, scale, height as f32),
            );
        },
    );
}

#[composable]
fn Colours(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let first = if ui.skin_section.get() == 1 {
        6usize
    } else {
        0usize
    };
    let chosen = cranpose_core::rememberMutableStateOf(|| first);
    let index = chosen.get();
    let (playlist, visualizer) = ui.shared.lock().unwrap().text_palettes();
    let split = area.w >= 640.;
    let detached = area.w >= 1000.;
    let list_w = if detached {
        (area.w * 0.22).clamp(240., 320.)
    } else if split {
        (area.w * 0.46).min(380.)
    } else {
        area.w
    };
    let colour_x = if split { list_w + 8. } else { 0. };
    let colour_y = if split { 48. } else { 728. };
    let colour_w = if detached {
        (area.w * 0.26).clamp(280., 360.)
    } else if split {
        area.w - colour_x
    } else {
        area.w
    };
    ScrollArea(
        area,
        if detached {
            area.h.max(680.)
        } else if split {
            824.
        } else {
            1080.
        },
        move || {
            Label("Colours".into(), 16., 16., area.w - 32., 16., studio_fg());
            Label("Playlist".into(), 16., 54., list_w - 32., 13., studio_dim());
            for (i, (key, title)) in PLAYLIST.into_iter().enumerate() {
                let color = paint(&playlist.iter().find(|p| p.0 == key).unwrap().1);
                let y = 80. + i as f32 * 44.;
                let feedback = controls::hint_feedback(title.into());
                let w = list_w - 24.;
                Button(
                    Modifier::empty()
                        .absolute_offset(12., y)
                        .size_points(w, 44.)
                        .then(design::surface(
                            design::Surface::Row,
                            index == i,
                            false,
                            false,
                        ))
                        .content_description(title)
                        .semantics(move |s| s.selected = Some(index == i))
                        .pointer_icon(control_cursor())
                        .then(feedback),
                    ButtonSpec::default(),
                    move || chosen.set(i),
                    move || {
                        Box(
                            Modifier::empty().size_points(w, 44.),
                            BoxSpec::default(),
                            move || {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(12., 10.)
                                        .size_points(24., 24.)
                                        .rounded_corners(5.)
                                        .background(color),
                                    BoxSpec::default(),
                                    || {},
                                );
                                desktop::RowLabel(
                                    title.into(),
                                    48.,
                                    14.,
                                    w - 56.,
                                    12.,
                                    studio_fg(),
                                );
                            },
                        );
                    },
                );
            }

            if !detached {
                PlaylistPreview(playlist.clone(), Area::new(16., 356., list_w - 32., 152.));
            }
            let vis_y = if detached { 356. } else { 528. };
            Label(
                "Visualizer".into(),
                16.,
                vis_y,
                list_w - 32.,
                13.,
                studio_dim(),
            );
            let columns = ((list_w - 24.) / 44.).floor().max(1.) as usize;
            let cell = (list_w - 24.) / columns as f32;
            for (i, value) in visualizer.iter().enumerate() {
                let color = paint(value);
                let at = Area::new(
                    12. + (i % columns) as f32 * cell,
                    vis_y + 24. + (i / columns) as f32 * 44.,
                    cell,
                    44.,
                );
                let feedback = controls::hint_feedback(visualizer_name(i));
                Button(
                    Modifier::empty()
                        .absolute_offset(at.x, at.y)
                        .size_points(at.w, at.h)
                        .then(design::surface(
                            design::Surface::Row,
                            index == i + 6,
                            false,
                            false,
                        ))
                        .content_description(format!("Visualizer colour {i}"))
                        .pointer_icon(control_cursor())
                        .then(feedback),
                    ButtonSpec::default(),
                    move || chosen.set(i + 6),
                    move || {
                        Box(
                            Modifier::empty()
                                .size_points(at.w - 12., 24.)
                                .rounded_corners(4.)
                                .background(color),
                            BoxSpec::default(),
                            || {},
                        );
                    },
                );
            }
            let preview_y = vis_y + 36. + 24usize.div_ceil(columns) as f32 * 44.;
            if detached {
                PlaylistPreview(
                    playlist.clone(),
                    Area::new(colour_x + 12., colour_y + 264., colour_w - 24., 152.),
                );
                VisualizerPreview(
                    visualizer.clone(),
                    Area::new(colour_x + 12., colour_y + 432., colour_w - 24., 96.),
                );
            } else {
                VisualizerPreview(
                    visualizer.clone(),
                    Area::new(16., preview_y, list_w - 32., 68.),
                );
            }
            let title = if index < 6 {
                PLAYLIST[index].1.into()
            } else {
                visualizer_name(index - 6)
            };
            let target = if index < 6 {
                colour::Target::Playlist(PLAYLIST[index].0)
            } else {
                colour::Target::Visualizer(index - 6)
            };
            Label(
                title,
                colour_x + 12.,
                if split { 22. } else { colour_y - 24. },
                colour_w - 24.,
                13.,
                studio_fg(),
            );
            cranpose_core::key(target, || {
                colour::SkinColour(
                    ui.clone(),
                    Area::new(colour_x, colour_y, colour_w, 244.),
                    target,
                )
            });
            if split {
                SkinPreview(
                    ui.clone(),
                    if detached {
                        Area::new(
                            colour_x + colour_w + 12.,
                            16.,
                            area.w - colour_x - colour_w - 28.,
                            area.h - 32.,
                        )
                    } else {
                        Area::new(colour_x + 12., colour_y + 264., colour_w - 24., 408.)
                    },
                );
            }
        },
    );
}

#[composable]
fn Palette(ui: Ui, area: Area, visualizer: bool) {
    let _ = ui.tick.get();
    let selected = cranpose_core::rememberMutableStateOf(|| 0usize);
    let (playlist, colours) = ui.shared.lock().unwrap().text_palettes();
    let index = selected.get().min(if visualizer { 23 } else { 5 });
    let values = if visualizer {
        colours.clone()
    } else {
        playlist.iter().map(|v| v.1.clone()).collect()
    };
    let wide = area.w >= 640.;
    let list_w = if wide {
        (area.w * 0.46).min(420.)
    } else {
        area.w
    };
    let cols = if visualizer {
        ((list_w - 24.) / 44.).floor().max(1.) as usize
    } else if list_w >= 280. {
        2
    } else {
        1
    };
    let rows = values.len().div_ceil(cols);
    let row_h = 44.;
    let list_h = 64. + rows as f32 * row_h;
    let picker_x = if wide { list_w + 8. } else { 0. };
    let picker_y = if wide { 48. } else { list_h + 8. };
    let picker_w = if wide {
        (area.w - picker_x).min(440.)
    } else {
        area.w
    };
    let preview_y = if wide { list_h + 16. } else { picker_y + 252. };
    let content_h = (preview_y + if visualizer { 128. } else { 184. }).max(picker_y + 260.);
    ScrollArea(area, content_h, move || {
        Label(
            if visualizer {
                "Visualizer colours"
            } else {
                "Playlist colours"
            }
            .into(),
            16.,
            16.,
            area.w - 32.,
            16.,
            studio_fg(),
        );
        Label(
            if visualizer {
                "VISCOLOR.TXT"
            } else {
                "PLEDIT.TXT"
            }
            .into(),
            16.,
            40.,
            list_w - 32.,
            11.,
            studio_dim(),
        );
        let cell = (list_w - 24.) / cols as f32;
        for (i, value) in values.iter().enumerate() {
            let c = parse_color(value).unwrap_or([0, 0, 0, 255]);
            let title = if visualizer {
                visualizer_name(i)
            } else {
                PLAYLIST[i].1.into()
            };
            let at = Area::new(
                12. + (i % cols) as f32 * cell,
                64. + (i / cols) as f32 * row_h,
                cell - 4.,
                40.,
            );
            let name = if visualizer {
                format!("Visualizer colour {i}: {title}")
            } else {
                title.clone()
            };
            let feedback = controls::hint_feedback(name.clone());
            Button(
                Modifier::empty()
                    .absolute_offset(at.x, at.y)
                    .size_points(at.w, at.h)
                    .then(design::surface(
                        design::Surface::Row,
                        i == index,
                        false,
                        false,
                    ))
                    .content_description(name)
                    .semantics(move |s| s.selected = Some(i == index))
                    .then(feedback),
                ButtonSpec::default(),
                move || selected.set(i),
                move || {
                    let title = title.clone();
                    Box(
                        Modifier::empty().size_points(at.w, at.h),
                        BoxSpec::default(),
                        move || {
                            Box(
                                Modifier::empty()
                                    .absolute_offset(8., 8.)
                                    .size_points(if visualizer { at.w - 16. } else { 24. }, 24.)
                                    .rounded_corners(5.)
                                    .background(Color::from_rgba_u8(c[0], c[1], c[2], 255)),
                                BoxSpec::default(),
                                || {},
                            );
                            if !visualizer {
                                desktop::RowLabel(
                                    title.clone(),
                                    42.,
                                    12.,
                                    at.w - 50.,
                                    12.,
                                    studio_fg(),
                                );
                            }
                        },
                    );
                },
            );
        }
        let target = if visualizer {
            colour::Target::Visualizer(index)
        } else {
            colour::Target::Playlist(PLAYLIST[index].0)
        };
        if wide {
            Label(
                if visualizer {
                    visualizer_name(index)
                } else {
                    PLAYLIST[index].1.into()
                },
                picker_x + 12.,
                22.,
                picker_w - 24.,
                13.,
                studio_fg(),
            );
        }
        cranpose_core::key(target, || {
            colour::SkinColour(
                ui.clone(),
                Area::new(picker_x, picker_y, picker_w, 244.),
                target,
            )
        });
        let preview_w = if wide { list_w - 32. } else { area.w - 32. };
        if visualizer {
            VisualizerPreview(colours.clone(), Area::new(16., preview_y, preview_w, 96.));
        } else {
            PlaylistPreview(playlist.clone(), Area::new(16., preview_y, preview_w, 152.));
        }
    });
}

fn visualizer_name(index: usize) -> String {
    match index {
        0 => "Background".into(),
        1 => "Grid dots".into(),
        2..=17 => format!("Spectrum row {}", index - 1),
        18..=22 => format!("Waveform row {}", index - 17),
        _ => "Peak marker".into(),
    }
}
fn paint(value: &str) -> Color {
    let c = parse_color(value).unwrap_or([0, 0, 0, 255]);
    Color::from_rgba_u8(c[0], c[1], c[2], 255)
}
#[composable]
fn PlaylistPreview(palette: Vec<(&'static str, String)>, area: Area) {
    let colour = move |key: &str| paint(&palette.iter().find(|p| p.0 == key).unwrap().1);
    let normal = colour("Normal");
    let current = colour("Current");
    let bg = colour("NormalBG");
    let selected = colour("SelectedBG");
    let mbfg = colour("MbFG");
    let mbbg = colour("MbBG");
    Box(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .background(bg)
            .rounded_corners(8.)
            .clip_to_bounds()
            .content_description("Playlist colour preview"),
        BoxSpec::default(),
        move || {
            Box(
                Modifier::empty()
                    .absolute_offset(4., 40.)
                    .size_points(area.w - 8., 32.)
                    .background(selected),
                BoxSpec::default(),
                || {},
            );
            Label(
                "01  First track".into(),
                12.,
                16.,
                area.w - 24.,
                13.,
                normal,
            );
            Label(
                "02  Selected track".into(),
                12.,
                48.,
                area.w - 24.,
                13.,
                normal,
            );
            Label(
                "03  Playing track".into(),
                12.,
                80.,
                area.w - 24.,
                13.,
                current,
            );
            Box(
                Modifier::empty()
                    .absolute_offset(0., 112.)
                    .size_points(area.w, 40.)
                    .background(mbbg),
                BoxSpec::default(),
                move || {
                    Label(
                        "Scroll text preview".into(),
                        12.,
                        12.,
                        area.w - 24.,
                        12.,
                        mbfg,
                    );
                },
            );
        },
    );
}
#[composable]
fn VisualizerPreview(colours: Vec<String>, area: Area) {
    cranpose_ui::Canvas(
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h)
            .content_description("Visualizer colour preview"),
        move |scope| {
            scope.draw_rect(Brush::solid(paint(&colours[0])));
            let step = area.w / 19.;
            let unit = (area.h - 12.) / 16.;
            for i in 0..19 {
                let level = 3 + (i * 7 % 14);
                for y in 0..level {
                    scope.draw_rect_at(
                        Rect {
                            x: i as f32 * step + 2.,
                            y: area.h - (y + 1) as f32 * unit,
                            width: step - 4.,
                            height: (unit - 1.).max(1.),
                        },
                        Brush::solid(paint(&colours[17 - y])),
                    );
                }
                scope.draw_rect_at(
                    Rect {
                        x: i as f32 * step + 2.,
                        y: area.h - (level + 1) as f32 * unit,
                        width: step - 4.,
                        height: 2.,
                    },
                    Brush::solid(paint(&colours[23])),
                );
            }
        },
    );
}

#[composable]
fn Font(ui: Ui, area: Area, preview_below: bool) {
    let revision = ui.tick.get();
    let colours = ui.shared.lock().unwrap().text_palettes().0;
    let ink = cranpose_core::rememberMutableStateOf(|| colours[0].1.clone());
    let background = cranpose_core::rememberMutableStateOf(|| colours[2].1.clone());
    let stacked = area.w < 300.;
    let step = if stacked { 76. } else { 52. };
    let actions_y = 178. + step * 2. + 4.;
    let preview_y = actions_y + 112.;
    let preview_h = (area.h - preview_y - 16.).clamp(260., 420.);
    let content_h = preview_y + if preview_below { preview_h + 16. } else { 0. };
    ScrollArea(area, content_h, move || {
        Label("Text font".into(), 16., 16., area.w - 32., 16., studio_fg());
        Label(
            "TEXT.BMP contains the player's letters.".into(),
            16.,
            44.,
            area.w - 32.,
            12.,
            studio_dim(),
        );
        let image = ui
            .shared
            .lock()
            .unwrap()
            .composite_images()
            .remove("text.bmp");
        if let Some(image) = image {
            let detail = image::imageops::crop_imm(
                &image,
                0,
                0,
                25.min(image.width()),
                6.min(image.height()),
            )
            .to_image();
            Bitmap(
                image,
                Area::new(16., 80., area.w - 32., 32.),
                "Current bitmap font",
            );
            Bitmap(
                detail,
                Area::new(16., 120., area.w - 32., 36.),
                "First five font glyphs, enlarged",
            );
        }
        for (i, (label, state)) in [("Font ink", ink), ("Font background", background)]
            .into_iter()
            .enumerate()
        {
            Label(
                label.into(),
                16.,
                if stacked {
                    178. + i as f32 * step
                } else {
                    190. + i as f32 * step
                },
                if stacked { area.w - 32. } else { area.w - 160. },
                12.,
                studio_dim(),
            );
            colour::ExactField(
                label,
                state.get(),
                Area::new(
                    if stacked { 16. } else { 128. },
                    178. + i as f32 * step + if stacked { 24. } else { 0. },
                    if stacked { area.w - 32. } else { 128. },
                    40.,
                ),
                move |v| {
                    if parse_color(&v).is_err() {
                        return false;
                    }
                    state.set(v);
                    true
                },
            );
        }
        let next = ui.clone();
        desktop::ListRow(
            Some(Icon::Text),
            "Build classic text font".into(),
            "Replace the bitmap font on the active paint layer".into(),
            String::new(),
            Area::new(12., actions_y, area.w - 24., 44.),
            false,
            ui.hints,
            move || {
                change(&next, |doc| {
                    doc.classic_font(&ink.get_non_reactive(), &background.get_non_reactive())
                });
            },
        );
        let next = ui.clone();
        desktop::ListRow(
            Some(Icon::Brush),
            "Edit font pixels".into(),
            String::new(),
            String::new(),
            Area::new(12., actions_y + 52., area.w - 24., 44.),
            false,
            ui.hints,
            move || edit_source(&next, "text.bmp"),
        );
        if preview_below {
            Label(
                "Player preview".into(),
                16.,
                preview_y,
                area.w - 32.,
                13.,
                studio_dim(),
            );
            let scale = ((area.w - 32.) / 275.).min((preview_h - 28.) / 116.);
            let shared = ui.shared.clone();
            Box(
                Modifier::empty()
                    .absolute_offset((area.w - 275. * scale) / 2., preview_y + 28.)
                    .size_points(275. * scale, 116. * scale)
                    .clip_to_bounds()
                    .content_description("Player with current bitmap font"),
                BoxSpec::default(),
                move || LivePlayer(shared.clone(), revision, scale, 150.),
            );
        }
    });
}

#[composable]
fn Windows(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let cut =
        cranpose_core::rememberMutableStateOf(|| ui.shared.lock().unwrap().view.color.clone());
    let row_h = if area.w < 400. { 204. } else { 172. };
    ScrollArea(area, 160. + row_h * 2., move || {
        Label(
            "Window shape".into(),
            16.,
            16.,
            area.w - 32.,
            16.,
            studio_fg(),
        );
        Label(
            "Remove matching pixels that touch the outside edge.".into(),
            16.,
            44.,
            area.w - 32.,
            12.,
            studio_dim(),
        );
        Label(
            "Cutout colour".into(),
            16.,
            100.,
            (area.w - 164.).max(74.),
            12.,
            studio_dim(),
        );
        colour::ExactField(
            "Cutout colour",
            cut.get(),
            Area::new((area.w - 132.).min(128.), 88., 116., 40.),
            move |v| {
                if parse_color(&v).is_err() {
                    return false;
                }
                cut.set(v);
                true
            },
        );
        for (i, (section, label)) in [("Normal", "Player"), ("Equalizer", "Equalizer")]
            .into_iter()
            .enumerate()
        {
            let y = 144. + i as f32 * row_h;
            Label(label.into(), 16., y, area.w - 32., 13., studio_fg());
            let image = ui.shared.lock().unwrap().skin_render();
            let image = image::imageops::crop_imm(&image, 0, i as u32 * 116, 275, 116).to_image();
            Bitmap(
                image,
                Area::new(16., y + 28., area.w - 32., 88.),
                "Window shape preview",
            );
            for (j, (icon, name)) in [(Icon::Select, "Cut edge"), (Icon::Undo, "Reset")]
                .into_iter()
                .enumerate()
            {
                let next = ui.clone();
                let w = (area.w - 24.) / 2.;
                desktop::ListRow(
                    Some(icon),
                    name.into(),
                    String::new(),
                    String::new(),
                    Area::new(12. + j as f32 * w, y + 120., w, 44.),
                    false,
                    ui.hints,
                    move || {
                        change(&next, |doc| {
                            doc.window_regions(&if j==0 {json!({"action":"generate","section":section,"transparent_color":cut.get_non_reactive(),"exterior_only":true})}
                        else {json!({"action":"remove","section":section})})
                        });
                    },
                );
            }
        }
    });
}

#[composable]
fn Cursors(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let report = ui.shared.lock().unwrap().cursors_report();
    let regions = report["regions"].as_array().cloned().unwrap_or_default();
    let selected = cranpose_core::rememberMutableStateOf(|| 0usize);
    let index = selected.get().min(regions.len().saturating_sub(1));
    let current = regions.get(index).cloned();
    let missing = report["drawn"] != report["of"];
    let action_h = if missing { 52. } else { 0. };
    let columns = if area.w >= 600. { 2 } else { 1 };
    let list_y = if current.as_ref().is_some_and(|r| r["drawn"] == true) {
        178. + action_h
    } else {
        94. + action_h
    };
    ScrollArea(
        area,
        list_y + regions.len().div_ceil(columns) as f32 * 52. + 16.,
        move || {
            Label("Cursors".into(), 16., 16., area.w - 32., 16., studio_fg());
            Label(
                format!("{} of {} supplied", report["drawn"], report["of"]),
                16.,
                44.,
                area.w - 32.,
                12.,
                studio_dim(),
            );
            if missing {
                let next = ui.clone();
                desktop::ListRow(
                    Some(Icon::Add),
                    "Draw missing cursors".into(),
                    "Use the skin's colours; existing cursors stay unchanged".into(),
                    String::new(),
                    Area::new(12., 72., area.w - 24., 44.),
                    false,
                    ui.hints,
                    move || change(&next, |doc| doc.draw_cursors(&[], None, false, "editor")),
                );
            }
            if let Some(current) = current.as_ref().filter(|r| r["drawn"] == true) {
                let role = current["region"].as_str().unwrap_or("normal").to_string();
                Hotspot(
                    ui.clone(),
                    role.clone(),
                    current.clone(),
                    Area::new(12., 70. + action_h, area.w - 24., 100.),
                );
            }
            for (i, region) in regions.iter().enumerate() {
                let role = region["region"].as_str().unwrap_or("normal").to_string();
                let file = region["file"].as_str().unwrap_or("");
                let w = (area.w - 24.) / columns as f32;
                let at = Area::new(
                    12. + (i % columns) as f32 * w,
                    list_y + (i / columns) as f32 * 52.,
                    w - 4.,
                    48.,
                );
                let image = ui.shared.lock().unwrap().composite_images().remove(file);
                previews::AssetRow(
                    ui.clone(),
                    previews::bitmap(image.unwrap_or_else(|| image::RgbaImage::new(32, 32))),
                    cursor_name(&role).into(),
                    if region["drawn"] == true {
                        file.into()
                    } else {
                        "Missing".into()
                    },
                    at,
                    i == index,
                    move || selected.set(i),
                );
            }
        },
    );
}

#[composable]
pub(super) fn CursorCanvas(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let doc = ui.shared.lock().unwrap();
    let report = doc.cursors_report();
    let role = doc.view.layer.clone();
    drop(doc);
    let region = report["regions"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["region"] == role))
        .cloned();
    ScrollArea(area, 212., move || {
        if let Some(region) = region.as_ref().filter(|r| r["drawn"] == true) {
            Hotspot(
                ui.clone(),
                role.clone(),
                region.clone(),
                Area::new(12., 16., area.w - 24., 100.),
            );
        } else {
            Label(
                "Choose a cursor in Skin settings.".into(),
                16.,
                20.,
                area.w - 32.,
                12.,
                studio_dim(),
            );
        }
        let next = ui.clone();
        desktop::ListRow(
            Some(Icon::Hand),
            "Choose cursor".into(),
            String::new(),
            String::new(),
            Area::new(12., 124., area.w - 24., 44.),
            false,
            ui.hints,
            move || {
                next.skin_section.set(4);
                next.show(super::Screen::Skin);
            },
        );
    });
}

#[composable]
pub(super) fn Hotspot(ui: Ui, role: String, cursor: serde_json::Value, area: Area) {
    let cell = ((area.w - 48.) / 2.).min(112.);
    Label(
        format!("{} click point", cursor_name(&role)),
        area.x,
        area.y,
        area.w - 44.,
        12.,
        studio_dim(),
    );
    for (i, label) in ["Cursor X", "Cursor Y"].into_iter().enumerate() {
        let next = ui.clone();
        let role = role.clone();
        let other = cursor["hotspot"][1 - i].as_u64().unwrap_or(0) as u32;
        let max = cursor["size"][i].as_u64().unwrap_or(32) as u32;
        Label(
            if i == 0 { "X" } else { "Y" }.into(),
            area.x + i as f32 * cell,
            area.y + 40.,
            16.,
            12.,
            studio_dim(),
        );
        colour::ExactField(
            label,
            cursor["hotspot"][i].as_u64().unwrap_or(0).to_string(),
            Area::new(
                area.x + 18. + i as f32 * cell,
                area.y + 26.,
                cell - 24.,
                40.,
            ),
            move |v| {
                let Ok(n) = v.trim().parse::<u32>() else {
                    return false;
                };
                if n >= max {
                    return false;
                }
                change(&next, |doc| {
                    doc.set_cursor_hotspot(
                        &role,
                        if i == 0 { n } else { other },
                        if i == 0 { other } else { n },
                        "editor",
                    )
                });
                true
            },
        );
    }
    let next = ui.clone();
    Control(
        Icon::Brush,
        "Paint cursor".into(),
        String::new(),
        Area::new(area.x + cell * 2. + 4., area.y + 24., 44., 44.),
        false,
        true,
        false,
        ui.hints,
        move || {
            update(
                &next.shared,
                next.tick,
                json!({"panel":"cursors","layer":role,"layers":[]}),
            );
            next.fit_requested.set(true);
            next.show(super::Screen::Canvas);
        },
    );
    if area.w >= 384. {
        let file = cursor["file"].as_str().unwrap_or("");
        let image = ui.shared.lock().unwrap().composite_images().remove(file);
        if let Some(image) = image {
            Bitmap(
                image,
                Area::new(area.x + cell * 2. + 64., area.y + 8., 64., 64.),
                "Selected cursor",
            );
        }
    }
}

fn cursor_name(role: &str) -> &str {
    match role {
        "normal" => "Player",
        "titlebar" => "Player title bar",
        "mainmenu" => "Player menu",
        "min" => "Minimize",
        "winbut" => "Window buttons",
        "close" => "Close player",
        "songname" => "Track title",
        "posbar" => "Track position",
        "volbal" => "Volume and balance",
        "eqnormal" => "Equalizer",
        "eqtitle" => "Equalizer title bar",
        "eqclose" => "Close equalizer",
        "eqslid" => "Equalizer sliders",
        "pnormal" => "Playlist",
        "ptbar" => "Playlist title bar",
        "pclose" => "Close playlist",
        "pvscroll" => "Playlist scroll",
        "psize" => "Resize playlist",
        "wsnormal" => "Windowshade",
        "wsmin" => "Minimize windowshade",
        "wswinbut" => "Windowshade buttons",
        "wsclose" => "Close windowshade",
        "wsposbar" => "Windowshade position",
        _ => role,
    }
}

#[composable]
fn Format(ui: Ui, area: Area, preview_below: bool) {
    let _ = ui.tick.get();
    let report =
        ui.shared.lock().unwrap().classic_report().unwrap_or_else(
            |e| json!({"exportable":false,"divergences":[{"problem":e.to_string()}]}),
        );
    let problems = report["divergences"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let preview_y = 128. + problems.len() as f32 * 108.;
    let preview_h = (area.h - preview_y - 16.).clamp(260., 560.);
    let content_h = preview_y + if preview_below { preview_h + 16. } else { 32. };
    ScrollArea(area, content_h, move || {
        Label(
            "Format check".into(),
            16.,
            16.,
            area.w - 32.,
            16.,
            studio_fg(),
        );
        Label(
            if report["exportable"] == true {
                "Ready for WSZ export"
            } else {
                "Fix these items before WSZ export"
            }
            .into(),
            16.,
            48.,
            area.w - 32.,
            13.,
            studio_fg(),
        );
        Label(
            "CSTUDIO projects can be saved at any time.".into(),
            16.,
            80.,
            area.w - 32.,
            12.,
            studio_dim(),
        );
        for (i, problem) in problems.iter().enumerate() {
            let y = 128. + i as f32 * 108.;
            Label(
                problem["entry"].as_str().unwrap_or("Skin").into(),
                16.,
                y,
                area.w - 32.,
                13.,
                studio_fg(),
            );
            Label(
                format!(
                    "{}\n{}",
                    problem["problem"].as_str().unwrap_or(""),
                    problem["fix"].as_str().unwrap_or("")
                ),
                16.,
                y + 24.,
                area.w - 32.,
                12.,
                studio_dim(),
            );
        }
        if preview_below {
            SkinPreview(
                ui.clone(),
                Area::new(16., preview_y, area.w - 32., preview_h),
            );
        }
    });
}
#[composable]
fn Bitmap(image: image::RgbaImage, area: Area, label: &'static str) {
    let region = Rect {
        x: 0.,
        y: 0.,
        width: image.width() as f32,
        height: image.height() as f32,
    };
    cranpose_ui::Image(
        cranpose_ui::BitmapRegionPainter(
            previews::bitmap(image),
            region,
            cranpose_ui::ImageSampling::Nearest,
        ),
        Some(label.into()),
        Modifier::empty()
            .absolute_offset(area.x, area.y)
            .size_points(area.w, area.h),
        Alignment::CENTER,
        cranpose_ui::ContentScale::Fit,
        1.,
        None,
    );
}
fn change(ui: &Ui, edit: impl FnOnce(&mut Document) -> anyhow::Result<serde_json::Value>) {
    let mut doc = ui.shared.lock().unwrap();
    if let Err(error) = edit(&mut doc) {
        doc.notify(error.to_string());
        doc.revision += 1;
    }
    ui.tick.set(doc.revision);
}
fn edit_source(ui: &Ui, sheet: &str) {
    update(
        &ui.shared,
        ui.tick,
        json!({"panel":"atlas","sheet":sheet,"layer":"sheet","layers":[],"presentation":false}),
    );
    ui.sprite_crop.set(None);
    ui.fit_requested.set(true);
    ui.show(super::Screen::Sprites);
}
