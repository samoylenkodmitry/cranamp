use super::*;

#[composable]
pub fn DesignSystem(width: f32, height: f32) {
    let ui = cranpose_core::remember(|| {
        let case = review_cases().remove(0);
        let document = case_document(&case);
        document
            .lock()
            .unwrap()
            .state(json!({"brush_size":16,"opacity":190,"curve_bend":-20}))
            .unwrap();
        case_ui(&case, document, width, height)
    })
    .with(Clone::clone);
    let columns = if width >= 1100. {
        3
    } else if width >= 700. {
        2
    } else {
        1
    };
    let gap = 16.;
    let card_w = (width - gap * (columns as f32 + 1.)) / columns as f32;
    let card_h = 348.;
    cranpose_core::CompositionLocalProvider(
        vec![controls::hint_context().provides(Some(ui.hints))],
        move || {
            Box(
                Modifier::empty().size_points(width, height).background(BG),
                BoxSpec::default(),
                move || {
                    Label("Studio controls".into(), 20., 18., width - 40., 20., FG);
                    Label(
                        "Shared styles from the two supplied references".into(),
                        20.,
                        48.,
                        width - 40.,
                        12.,
                        DIM,
                    );
                    let ui = ui.clone();
                    let hints = ui.hints;
                    ScrollArea(
                        Area::new(0., 80., width, height - 80.),
                        (6usize.div_ceil(columns)) as f32 * (card_h + gap),
                        move || {
                            for section in 0..6 {
                                let ui = ui.clone();
                                let x = gap + (section % columns) as f32 * (card_w + gap);
                                let y = (section / columns) as f32 * (card_h + gap);
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(x, y)
                                        .size_points(card_w, card_h)
                                        .then(design::surface(
                                            design::Surface::Panel,
                                            false,
                                            false,
                                            false,
                                        )),
                                    BoxSpec::default(),
                                    move || {
                                        let titles = [
                                            "Buttons",
                                            "Fields and switches",
                                            "Sliders",
                                            "Selection and input states",
                                            "Files",
                                            "Tool previews",
                                        ];
                                        Label(
                                            titles[section].into(),
                                            16.,
                                            16.,
                                            card_w - 32.,
                                            16.,
                                            FG,
                                        );
                                        match section {
                                            0 => {
                                                controls::PrimaryAction(
                                                    Icon::Save,
                                                    "Save project".into(),
                                                    Area::new(16., 54., card_w - 32., 44.),
                                                    || {},
                                                );
                                                for (i, icon) in [
                                                    Icon::Open,
                                                    Icon::Undo,
                                                    Icon::Redo,
                                                    Icon::Share,
                                                ]
                                                .into_iter()
                                                .enumerate()
                                                {
                                                    Control(
                                                        icon,
                                                        ["Open", "Undo", "Redo", "Export"][i]
                                                            .into(),
                                                        String::new(),
                                                        Area::new(
                                                            16. + i as f32 * 52.,
                                                            114.,
                                                            44.,
                                                            44.,
                                                        ),
                                                        false,
                                                        i != 2,
                                                        false,
                                                        ui.hints,
                                                        || {},
                                                    );
                                                }
                                                for (i, icon) in [
                                                    Icon::Brush,
                                                    Icon::Line,
                                                    Icon::Rectangle,
                                                    Icon::Ellipse,
                                                ]
                                                .into_iter()
                                                .enumerate()
                                                {
                                                    Control(
                                                        icon,
                                                        ["Pencil", "Line", "Rectangle", "Ellipse"]
                                                            [i]
                                                            .into(),
                                                        String::new(),
                                                        Area::new(
                                                            16. + i as f32 * 52.,
                                                            178.,
                                                            44.,
                                                            44.,
                                                        ),
                                                        i == 0,
                                                        true,
                                                        false,
                                                        ui.hints,
                                                        || {},
                                                    );
                                                }
                                                Choice(
                                                    "Released".into(),
                                                    16.,
                                                    246.,
                                                    (card_w - 40.) / 2.,
                                                    true,
                                                    || {},
                                                );
                                                Choice(
                                                    "Pressed".into(),
                                                    card_w / 2. + 4.,
                                                    246.,
                                                    (card_w - 40.) / 2.,
                                                    false,
                                                    || {},
                                                );
                                            }
                                            1 => {
                                                Label("Name".into(), 16., 62., 64., 13., DIM);
                                                colour::ExactField(
                                                    "Layer name",
                                                    "Highlights".into(),
                                                    Area::new(88., 52., card_w - 104., 40.),
                                                    |_| true,
                                                );
                                                Label("HEX".into(), 16., 116., 64., 13., DIM);
                                                colour::ExactField(
                                                    "HEX colour",
                                                    "#7DDCFF".into(),
                                                    Area::new(88., 106., card_w - 104., 40.),
                                                    |text| parse_color(&text).is_ok(),
                                                );
                                                let visible =
                                                    cranpose_core::rememberMutableStateOf(|| true);
                                                controls::SwitchRow(
                                                    "Visible".into(),
                                                    Area::new(12., 164., card_w - 24., 44.),
                                                    visible.get(),
                                                    move || visible.set(!visible.get()),
                                                );
                                                let locked =
                                                    cranpose_core::rememberMutableStateOf(|| false);
                                                controls::SwitchRow(
                                                    "Locked".into(),
                                                    Area::new(12., 216., card_w - 24., 44.),
                                                    locked.get(),
                                                    move || locked.set(!locked.get()),
                                                );
                                                Label(
                                                    "Type a HEX value to check validation.".into(),
                                                    16.,
                                                    292.,
                                                    card_w - 32.,
                                                    12.,
                                                    DIM,
                                                );
                                            }
                                            2 => {
                                                let _ = ui.tick.get();
                                                let view = ui.shared.lock().unwrap().view.clone();
                                                for (i, label, key, value, range) in [
                                                    (
                                                        0,
                                                        "Brush size",
                                                        "brush_size",
                                                        view.brush_size as i32,
                                                        [1, 32],
                                                    ),
                                                    (
                                                        1,
                                                        "Opacity",
                                                        "opacity",
                                                        view.opacity as i32,
                                                        [1, 255],
                                                    ),
                                                    (
                                                        2,
                                                        "Bend",
                                                        "curve_bend",
                                                        view.curve_bend,
                                                        [-100, 100],
                                                    ),
                                                ] {
                                                    workbench::ValueSlider(
                                                        ui.clone(),
                                                        label,
                                                        key,
                                                        value,
                                                        range,
                                                        "",
                                                        Area::new(
                                                            8.,
                                                            56. + i as f32 * 86.,
                                                            card_w - 16.,
                                                            64.,
                                                        ),
                                                    );
                                                }
                                            }
                                            3 => {
                                                for (i, (name, selected, hovered, pressed)) in [
                                                    ("Normal", false, false, false),
                                                    ("Selected", true, false, false),
                                                    ("Hover", false, true, false),
                                                    ("Pressed", true, false, true),
                                                ]
                                                .into_iter()
                                                .enumerate()
                                                {
                                                    Box(
                                                        Modifier::empty()
                                                            .absolute_offset(
                                                                12.,
                                                                52. + i as f32 * 54.,
                                                            )
                                                            .size_points(card_w - 24., 44.)
                                                            .then(design::surface(
                                                                design::Surface::Button,
                                                                selected,
                                                                hovered,
                                                                pressed,
                                                            )),
                                                        BoxSpec::default(),
                                                        move || {
                                                            Label(
                                                                name.into(),
                                                                12.,
                                                                13.,
                                                                card_w - 48.,
                                                                13.,
                                                                FG,
                                                            );
                                                        },
                                                    );
                                                }
                                                Box(
                                                    Modifier::empty()
                                                        .absolute_offset(12., 278.)
                                                        .size_points(card_w - 24., 44.)
                                                        .then(design::surface(
                                                            design::Surface::Invalid,
                                                            false,
                                                            false,
                                                            false,
                                                        )),
                                                    BoxSpec::default(),
                                                    move || {
                                                        Label(
                                                            "Invalid value".into(),
                                                            12.,
                                                            13.,
                                                            card_w - 48.,
                                                            13.,
                                                            FG,
                                                        );
                                                    },
                                                );
                                            }
                                            4 => desktop::Sources(
                                                ui.clone(),
                                                Area::new(4., 46., card_w - 8., card_h - 54.),
                                            ),
                                            _ => {
                                                previews::ToolSample(
                                                    ui.clone(),
                                                    Area::new(16., 56., card_w - 32., 96.),
                                                );
                                                previews::Divider(Area::new(
                                                    16.,
                                                    174.,
                                                    card_w - 32.,
                                                    1.,
                                                ));
                                                quick::Palette(
                                                    ui.clone(),
                                                    Area::new(12., 196., card_w - 24., 44.),
                                                );
                                            }
                                        }
                                    },
                                );
                            }
                        },
                    );
                    HintOverlay(hints, width, height);
                },
            );
        },
    );
}
