//! Guided editing for the shared classic equalizer cells.
use super::*;

fn perform(shared: &SharedDocument, args: serde_json::Value) {
    let mut doc = shared.lock().unwrap();
    if let Err(error) = doc.eq_workbench(&args, "Human") {
        doc.notify(error.to_string());
        doc.revision += 1;
    }
}

#[composable]
fn Cell(label: String, x: f32, y: f32, width: f32, on: bool, action: impl Fn() + 'static) {
    let feedback = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(width, 44.)
            .then(design::surface(design::Surface::Button, on, false, false))
            .content_description(label.clone())
            .semantics(move |s| s.selected = on.then_some(true))
            .then(feedback),
        ButtonSpec::default(),
        action,
        move || {
            Text(
                label.clone(),
                Modifier::empty().padding(10.),
                text_style(12., FG),
            );
        },
    );
}

#[composable]
pub(super) fn EqualizerChooser(
    shared: SharedDocument,
    _revision: u64,
    room: (f32, f32),
    review: cranpose_core::MutableState<bool>,
    live: cranpose_core::MutableState<bool>,
) {
    let scroll = cranpose_ui::rememberScrollState!(0.);
    let (artwork, frame, scope, pressed) = {
        let doc = shared.lock().unwrap();
        (
            doc.eq_artwork_only,
            doc.view.eq[0],
            doc.view.states.clone(),
            doc.view.pressed,
        )
    };
    let width = room.0;
    let columns = ((width + 4.) / 48.).floor().clamp(4., 8.) as u8;
    let cell = (width + 4.) / f32::from(columns);
    let mode_rows = if width < 300. { 2. } else { 1. };
    let modes_h = mode_rows * 48.;
    let grid_y = 72. + modes_h;
    let after_grid = grid_y + f32::from(28u8.div_ceil(columns)) * 48.;
    let scopes_y = after_grid + 36.;
    let actions_y = scopes_y + modes_h + 12.;
    let preview_y = actions_y + 4. * 48. + 32.;
    let content_h = preview_y + 5. * 48. + 56.;
    cranpose_ui::Column(
        Modifier::empty()
            .size_points(width + 24., room.1)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            let shared = shared.clone();
            let scope = scope.clone();
            Box(
                Modifier::empty().size_points(width + 24., content_h),
                BoxSpec::default(),
                move || {
                    Label(
                        "The 11 bands share one drawing.".into(),
                        12.,
                        8.,
                        width,
                        12.,
                        DIM,
                    );
                    for (i, (mode, label, on)) in [
                        ("controls", "Visible controls", !artwork),
                        ("artwork", "Artwork only", artwork),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        let x = if mode_rows == 2. {
                            12.
                        } else {
                            12. + i as f32 * (width + 4.) / 2.
                        };
                        let y = 32. + if mode_rows == 2. { i as f32 * 48. } else { 0. };
                        Cell(
                            label.into(),
                            x,
                            y,
                            if mode_rows == 2. {
                                width
                            } else {
                                (width - 4.) / 2.
                            },
                            on,
                            move || perform(&d, json!({"action":"set_mode","mode":mode})),
                        );
                    }
                    Label("Level to paint".into(), 12., grid_y - 24., width, 12., DIM);
                    for n in 0..28u8 {
                        let d = shared.clone();
                        Cell(
                            format!("{n:02}"),
                            12. + f32::from(n % columns) * cell,
                            grid_y + f32::from(n / columns) * 48.,
                            cell - 4.,
                            frame == n,
                            move || {
                                perform(&d, json!({"action":"edit_frame","frame":n}));
                                review.set(false);
                                live.set(false);
                            },
                        );
                    }
                    Label(
                        "0: cut    14: centre    27: boost".into(),
                        12.,
                        after_grid + 4.,
                        width,
                        11.,
                        DIM,
                    );
                    for (i, (value, label)) in [("current", "This frame"), ("all", "All 28 frames")]
                        .into_iter()
                        .enumerate()
                    {
                        let d = shared.clone();
                        Cell(
                            label.into(),
                            12. + if mode_rows == 2. {
                                0.
                            } else {
                                i as f32 * (width + 4.) / 2.
                            },
                            scopes_y + if mode_rows == 2. { i as f32 * 48. } else { 0. },
                            if mode_rows == 2. {
                                width
                            } else {
                                (width - 4.) / 2.
                            },
                            scope == value,
                            move || state(&d, json!({"states":value})),
                        );
                    }
                    for (i, (action, label)) in [
                        ("edit_frame", "Paint shared track"),
                        ("edit_handle", "Paint shared handle"),
                        ("edit_background", "Paint background"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        let scope = scope.clone();
                        Cell(
                            label.into(),
                            12.,
                            actions_y + i as f32 * 48.,
                            width,
                            false,
                            move || {
                                perform(
                                    &d,
                                    json!({"action":action,"frame":frame,"scope":if scope=="all" {"all"}else{"current"}}),
                                );
                                review.set(false);
                                live.set(false);
                            },
                        );
                    }
                    let d = shared.clone();
                    Cell(
                        "Handle pressed".into(),
                        12.,
                        actions_y + 144.,
                        width,
                        pressed,
                        move || state(&d, json!({"pressed":!pressed})),
                    );
                    Label(
                        "Check the result".into(),
                        12.,
                        preview_y - 24.,
                        width,
                        12.,
                        DIM,
                    );
                    for (i, (pattern, label)) in [
                        ("flat", "Same level"),
                        ("ramp", "Rising levels"),
                        ("alternating", "High / low"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        Cell(
                            label.into(),
                            12.,
                            preview_y + i as f32 * 48.,
                            width,
                            false,
                            move || {
                                perform(
                                    &d,
                                    json!({"action":"preview","frame":frame,"pattern":pattern}),
                                )
                            },
                        );
                    }
                    let d = shared.clone();
                    Cell(
                        "Compare all 28 frames".into(),
                        12.,
                        preview_y + 144.,
                        width,
                        false,
                        move || {
                            let mut doc = d.lock().unwrap();
                            match doc.eq_workbench(
                                &json!({"action":"edit_frame","frame":frame}),
                                "Human",
                            ) {
                                Ok(_) => {
                                    review.set(true);
                                    live.set(false);
                                }
                                Err(error) => {
                                    doc.notify(error.to_string());
                                    doc.revision += 1;
                                }
                            }
                        },
                    );
                    let d = shared.clone();
                    Cell(
                        "Copy this frame to all 28".into(),
                        12.,
                        preview_y + 192.,
                        width,
                        false,
                        move || perform(&d, json!({"action":"copy_frame_to_all","frame":frame})),
                    );
                    Label(
                        "Copy replaces the shared track. Undo restores it.".into(),
                        12.,
                        preview_y + 244.,
                        width,
                        11.,
                        DIM,
                    );
                },
            );
        },
    );
    cranpose_ui::widgets::Scrollbar(
        Modifier::empty()
            .absolute_offset(width + 18., 0.)
            .size_points(6., room.1)
            .content_description("Scroll equalizer controls")
            .then(adaptive::controls::feedback(
                "Scroll equalizer controls".into(),
            )),
        scroll,
        cranpose_ui_layout::Axis::Vertical,
        cranpose_ui::widgets::ScrollbarSpec::default().thickness(6.),
    );
}
