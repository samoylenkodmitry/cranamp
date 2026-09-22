//! Guided editing for the shared classic equalizer cells.
use super::*;

fn perform(shared: &SharedDocument, args: serde_json::Value) {
    let mut doc = shared.lock().unwrap();
    if let Err(error) = doc.eq_workbench(&args, "Human") {
        doc.message = error.to_string();
        doc.revision += 1;
    }
}

#[composable]
pub(super) fn EqualizerChooser(
    shared: SharedDocument,
    _revision: u64,
    room: (f32, f32),
    review: cranpose_core::MutableState<bool>,
    live: cranpose_core::MutableState<bool>,
) {
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    let (artwork, frame, scope, pressed) = {
        let doc = shared.lock().unwrap();
        (
            doc.eq_artwork_only,
            doc.view.eq[0],
            doc.view.states.clone(),
            doc.view.pressed,
        )
    };
    cranpose_ui::Column(
        Modifier::empty()
            .size_points(room.0 + 24., room.1)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            let shared = shared.clone();
            let scope = scope.clone();
            Box(
                Modifier::empty().size_points(room.0 + 24., 800.),
                BoxSpec::default(),
                move || {
                    Label("EQ WORKBENCH".into(), 12., 17., 260., 14., FG);
                    Label(
                        "One shared drawing appears in all eleven bands.
28 frames are levels, not individual band pictures."
                            .into(),
                        12.,
                        50.,
                        350.,
                        11.,
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
                        Choice(
                            label.into(),
                            12. + i as f32 * 178.,
                            96.,
                            170.,
                            on,
                            move || perform(&d, json!({"action":"set_mode","mode":mode})),
                        );
                    }
                    Label(
                        if artwork {
                            "Tracks, handles, graph and preset graphics are hidden.
Invisible controls still respond. Source artwork is preserved."
                        } else {
                            "Paint a repeating track once, then refine each level.
Use mixed levels to catch seams between different frames."
                        }
                        .into(),
                        12.,
                        135.,
                        350.,
                        11.,
                        DIM,
                    );
                    Label("CHOOSE A LEVEL TO PAINT".into(), 12., 184., 350., 11., DIM);
                    for n in 0..28u8 {
                        let d = shared.clone();
                        Choice(
                            format!("{n:02}"),
                            12. + f32::from(n % 7) * 50.,
                            204. + f32::from(n / 7) * 34.,
                            44.,
                            frame == n,
                            move || {
                                perform(&d, json!({"action":"edit_frame","frame":n}));
                                review.set(false);
                                live.set(false);
                            },
                        );
                    }
                    Label(
                        "0 = full cut  ·  14 = center  ·  27 = full boost".into(),
                        12.,
                        350.,
                        350.,
                        10.,
                        DIM,
                    );
                    for (i, (value, label)) in [("current", "This frame"), ("all", "All 28 frames")]
                        .into_iter()
                        .enumerate()
                    {
                        let d = shared.clone();
                        Choice(
                            label.into(),
                            12. + i as f32 * 178.,
                            379.,
                            170.,
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
                        Action(label.into(), 12., 425. + i as f32 * 37., 220., move || {
                            perform(
                                &d,
                                json!({"action":action,"frame":frame,"scope":if scope=="all" {"all"}else{"current"}}),
                            );
                            review.set(false);
                            live.set(false);
                        });
                    }
                    let d = shared.clone();
                    Toggle(
                        "Handle pressed".into(),
                        238.,
                        462.,
                        120.,
                        pressed,
                        move || state(&d, json!({"pressed":!pressed})),
                    );
                    Label("CHECK THE COMPOSITION".into(), 12., 548., 350., 11., DIM);
                    for (i, (pattern, label)) in [
                        ("flat", "Same level"),
                        ("ramp", "Rising levels"),
                        ("alternating", "High / low"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        Action(label.into(), 12. + i as f32 * 118., 569., 110., move || {
                            perform(
                                &d,
                                json!({"action":"preview","frame":frame,"pattern":pattern}),
                            )
                        });
                    }
                    let d = shared.clone();
                    Action("Compare all 28 frames".into(), 12., 615., 220., move || {
                        let mut doc = d.lock().unwrap();
                        match doc
                            .eq_workbench(&json!({"action":"edit_frame","frame":frame}), "Human")
                        {
                            Ok(_) => {
                                review.set(true);
                                live.set(false);
                            }
                            Err(error) => {
                                doc.message = error.to_string();
                                doc.revision += 1;
                            }
                        }
                    });
                    let d = shared.clone();
                    Action(
                        "Copy this frame to all 28".into(),
                        12.,
                        660.,
                        220.,
                        move || perform(&d, json!({"action":"copy_frame_to_all","frame":frame})),
                    );
                    Label(
                        "Copy replaces the shared bank on the active paint layer.
Undo restores it. Artwork mode exports a cropped EQ bitmap;
check the result in the players you intend to support."
                            .into(),
                        12.,
                        704.,
                        350.,
                        10.,
                        DIM,
                    );
                },
            );
        },
    );
}
