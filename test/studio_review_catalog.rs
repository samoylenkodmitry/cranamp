use super::*;

#[path = "studio_design_preview.rs"]
mod design_system;
pub use design_system::DesignSystem;

#[derive(Clone)]
pub struct ReviewCase {
    pub name: String,
    screen: Screen,
    sheet: Sheet,
    tool: &'static str,
    patch: serde_json::Value,
    copied: bool,
    paint_layer: bool,
    pan: bool,
}

pub fn review_cases() -> Vec<ReviewCase> {
    let mut cases = Vec::new();
    for (scope, screen) in [("canvas", Screen::Canvas), ("sprite", Screen::Sprites)] {
        for tool in [
            "pencil", "line", "rect", "ellipse", "lift", "text", "stamp", "curve", "glass", "tuft",
        ] {
            for (suffix, sheet) in [("quick", Sheet::None), ("settings", Sheet::Advanced(4))] {
                cases.push(ReviewCase {
                    name: format!("{scope}-{tool}-{suffix}"),
                    screen,
                    sheet,
                    tool,
                    patch: json!({}),
                    copied: false,
                    paint_layer: false,
                    pan: false,
                });
            }
        }
    }
    for (name, tool, patch, copied) in [
        (
            "pencil-maximum",
            "pencil",
            json!({"brush_size":32,"opacity":51}),
            false,
        ),
        ("line-maximum", "line", json!({"brush_size":32}), false),
        ("rectangle-filled", "rect", json!({"filled":true}), false),
        (
            "rectangle-gradient",
            "rect",
            json!({"filled":true,"ramp_to":"#56c9dc"}),
            false,
        ),
        ("ellipse-filled", "ellipse", json!({"filled":true}), false),
        (
            "ellipse-gradient",
            "ellipse",
            json!({"filled":true,"ramp_to":"#56c9dc"}),
            false,
        ),
        (
            "text-small-font",
            "text",
            json!({"face":"small","text":"Catamp","text_scale":3,"text_spacing":2}),
            false,
        ),
        (
            "text-large",
            "text",
            json!({"text":"Catamp","text_scale":6,"text_spacing":-1}),
            false,
        ),
        ("curve-left", "curve", json!({"curve_bend":-100}), false),
        (
            "curve-right",
            "curve",
            json!({"curve_bend":100,"brush_size":32}),
            false,
        ),
        (
            "glass-wide",
            "glass",
            json!({"bevel":128,"refraction":32}),
            false,
        ),
        (
            "taper-wide",
            "tuft",
            json!({"brush_size":32,"grain":64,"grain_size":6}),
            false,
        ),
        ("lift-copied", "lift", json!({}), true),
        ("stamp-copied", "stamp", json!({}), true),
        (
            "stamp-repeat",
            "stamp",
            json!({"stamp_repeat":12,"stamp_sweep":true}),
            true,
        ),
    ] {
        for (suffix, sheet) in [("quick", Sheet::None), ("settings", Sheet::Advanced(4))] {
            cases.push(ReviewCase {
                name: format!("{name}-{suffix}"),
                screen: Screen::Canvas,
                sheet,
                tool,
                patch: patch.clone(),
                copied,
                paint_layer: false,
                pan: false,
            });
        }
    }
    for (name, sheet) in [
        ("parts", Sheet::Parts),
        ("targets", Sheet::Targets),
        ("sources", Sheet::Sources),
        ("tools", Sheet::Tools),
        ("colour", Sheet::Colour),
        ("canvas-state", Sheet::CanvasState),
        ("layers", Sheet::Layers),
        ("project-actions", Sheet::More),
        ("edit-history", Sheet::Advanced(2)),
        ("pixel-study", Sheet::Advanced(5)),
        ("sprite-guides", Sheet::Advanced(6)),
        ("workspace-selector", Sheet::Scopes),
        ("equalizer", Sheet::Advanced(10)),
    ] {
        cases.push(ReviewCase {
            name: name.into(),
            screen: Screen::Canvas,
            sheet,
            tool: "pencil",
            patch: json!({}),
            copied: false,
            paint_layer: false,
            pan: false,
        });
    }
    for (name, screen) in [
        ("projects", Screen::Projects),
        ("save", Screen::Save),
        ("help", Screen::Help),
        ("new-project", Screen::ConfirmBlank),
        ("states", Screen::States),
        ("preview", Screen::Preview),
    ] {
        cases.push(ReviewCase {
            name: name.into(),
            screen,
            sheet: Sheet::None,
            tool: "pencil",
            patch: json!({}),
            copied: false,
            paint_layer: false,
            pan: false,
        });
    }
    for (name, patch, sheet) in [
        ("paint-layer", json!({}), Sheet::None),
        ("paint-layer-settings", json!({}), Sheet::Layers),
        (
            "paint-mask",
            json!({"alpha_lock":true,"mask_colors":["#ffffff"]}),
            Sheet::Advanced(4),
        ),
        (
            "current-state",
            json!({"states":"current","active":false,"pressed":true}),
            Sheet::CanvasState,
        ),
        (
            "colour-full-palette",
            json!({"color":"#e37e66","swatches":["#ffffff","#ff0000","#00ff00","#0000ff","#ffff00","#00ffff","#ff00ff","#000000","#e37e66","#56c9dc","#8fbb8f","#e5c777"]}),
            Sheet::Colour,
        ),
    ] {
        cases.push(ReviewCase {
            name: name.into(),
            screen: Screen::Canvas,
            sheet,
            tool: "pencil",
            patch,
            copied: false,
            paint_layer: true,
            pan: false,
        });
    }
    for (name, patch) in [
        ("pan-grid", json!({"grid":true})),
        ("pan-clean", json!({"grid":false})),
    ] {
        cases.push(ReviewCase {
            name: name.into(),
            screen: Screen::Canvas,
            sheet: Sheet::None,
            tool: "pencil",
            patch,
            copied: false,
            paint_layer: false,
            pan: true,
        });
    }
    for part in ["main", "equalizer", "playlist", "shade", "cursors"] {
        cases.push(ReviewCase {
            name: format!("part-{part}"),
            screen: Screen::Canvas,
            sheet: Sheet::None,
            tool: "pencil",
            patch: json!({"panel":part}),
            copied: false,
            paint_layer: false,
            pan: false,
        });
    }
    for scope in ["all", "current", "onward", "up-to"] {
        for (active, pressed) in [(true, false), (true, true), (false, false), (false, true)] {
            cases.push(ReviewCase {
                name: format!(
                    "state-{scope}-{}-{}",
                    if active { "active" } else { "inactive" },
                    if pressed { "pressed" } else { "released" }
                ),
                screen: Screen::Canvas,
                sheet: Sheet::CanvasState,
                tool: "pencil",
                patch: json!({"states":scope,"active":active,"pressed":pressed}),
                copied: false,
                paint_layer: false,
                pan: false,
            });
        }
    }
    for (name, patch) in [
        ("mirror-x", json!({"mirror_x":true})),
        ("mirror-y", json!({"mirror_y":true})),
        ("mirror-both", json!({"mirror_x":true,"mirror_y":true})),
        ("grain-maximum", json!({"grain":64,"grain_size":16})),
        ("clean-corners", json!({"clean_corners":true})),
        ("alpha-locked", json!({"alpha_lock":true})),
    ] {
        cases.push(ReviewCase {
            name: name.into(),
            screen: Screen::Canvas,
            sheet: Sheet::Advanced(4),
            tool: "pencil",
            patch,
            copied: false,
            paint_layer: true,
            pan: false,
        });
    }
    for (section, name) in [
        "playlist",
        "visualizer",
        "font",
        "shape",
        "cursors",
        "format",
    ]
    .into_iter()
    .enumerate()
    {
        cases.push(ReviewCase {
            name: format!("skin-{name}"),
            screen: Screen::Skin,
            sheet: Sheet::None,
            tool: "pencil",
            patch: json!({"skin_section":section}),
            copied: false,
            paint_layer: false,
            pan: false,
        });
    }
    cases.push(ReviewCase {
        name: "skin-cursors-drawn".into(),
        screen: Screen::Skin,
        sheet: Sheet::None,
        tool: "pencil",
        patch: json!({"skin_section":4,"draw_cursors":true}),
        copied: false,
        paint_layer: false,
        pan: false,
    });
    cases
}

fn case_document(case: &ReviewCase) -> SharedDocument {
    let shared = open_document(None).expect("review skin");
    let mut doc = shared.lock().expect("document");
    doc.state(json!({"drawer":"none","panel":"canvas","zoom":1,"brush":case.tool,"color":"#e37e66","text":"Catamp"})).expect("review defaults");
    let mut patch = case.patch.clone();
    patch.as_object_mut().unwrap().remove("skin_section");
    patch.as_object_mut().unwrap().remove("draw_cursors");
    doc.state(patch).expect("review variant");
    if case.name == "skin-cursors" {
        doc.remove_cursors(&[], "review").unwrap();
    }
    if case.patch["draw_cursors"] == true {
        doc.draw_cursors(&[], None, false, "review").unwrap();
    }
    if case.copied {
        doc.capture_cluster([16, 24, 56, 20])
            .expect("review clipboard");
    }
    if case.paint_layer {
        doc.paint_layer_command(&json!({"action":"add","name":"Highlights"}), "Review")
            .expect("review layer");
    }
    if matches!(case.screen, Screen::Sprites | Screen::States) {
        let layer = doc
            .skin_layers()
            .into_iter()
            .find(|layer| layer.id == "main.play")
            .expect("play sprite");
        doc.state(json!({"sheet":layer.sheet,"layer":layer.id,"layers":["main.play"]}))
            .expect("review sprite");
    }
    drop(doc);
    shared
}

fn case_ui(case: &ReviewCase, shared: SharedDocument, width: f32, height: f32) -> Ui {
    let crop = (case.screen == Screen::Sprites).then(|| {
        shared
            .lock()
            .expect("document")
            .skin_layers()
            .into_iter()
            .find(|layer| layer.id == "main.play")
            .expect("play sprite")
            .destination
    });
    let revision = shared.lock().expect("document").revision;
    Ui {
        tick: cranpose_core::mutableStateOf(revision),
        shared,
        layout: cranpose_core::mutableStateOf(Layout::new(width, height)),
        screen: cranpose_core::mutableStateOf(case.screen),
        previous: cranpose_core::mutableStateOf(Screen::Canvas),
        sheet: cranpose_core::mutableStateOf(case.sheet),
        viewport: cranpose_core::mutableStateOf(Viewport::default()),
        fit_requested: cranpose_core::mutableStateOf(true),
        desktop_seen: cranpose_core::mutableStateOf(false),
        sprite_crop: cranpose_core::mutableStateOf(crop),
        sprite_id: cranpose_core::mutableStateOf("main.play".into()),
        hand: cranpose_core::mutableStateOf(case.pan),
        skin_section: cranpose_core::mutableStateOf(
            case.patch["skin_section"].as_u64().unwrap_or(0) as usize,
        ),
        hints: cranpose_core::mutableStateOf(None),
        review: cranpose_core::mutableStateOf(false),
        live: cranpose_core::mutableStateOf(false),
    }
}

#[composable]
pub fn ReviewGallery(width: f32, height: f32, start: usize) {
    ReviewCases(width, height, start, false);
}

#[composable]
fn ReviewCases(width: f32, height: f32, start: usize, automatic: bool) {
    let cases = cranpose_core::remember(review_cases);
    let index = cranpose_core::rememberMutableStateOf(move || start);
    let count = cases.with(Vec::len);
    let at = index.get().min(count - 1);
    let case = cases.with(|cases| cases[at].clone());
    cranpose_core::LaunchedEffectAsync((at, automatic), move |_| {
        Box::pin(async move {
            if automatic {
                cranpose_core::delay(Duration::from_millis(350)).await;
                #[cfg(target_os = "android")]
                android::progress(&format!("case {}/{}", at + 1, count));
                if at + 1 < count {
                    index.set(at + 1);
                }
                #[cfg(target_os = "android")]
                if at + 1 == count {
                    android::progress("complete");
                }
            }
        })
    });
    let ui = cranpose_core::rememberKeyed(at, |_| {
        let shared = case_document(&case);
        case_ui(&case, shared, width, height)
    });
    let actions = Actions {
        open: Rc::new(|| {}),
        project: Rc::new(|| {}),
        export: Rc::new(|| {}),
        apply: Rc::new(|| {}),
        restore: Rc::new(|| {}),
        close: Rc::new(|| {}),
    };
    cranpose_core::key(at, || {
        cranpose_core::CompositionLocalProvider(
            vec![controls::hint_context().provides(Some(ui.hints))],
            || {
                Workspace(ui.clone(), actions, Layout::new(width, height), true, false);
            },
        );
    });
    Box(
        Modifier::empty()
            .absolute_offset(0., height)
            .size_points(width, 40.)
            .background(Color(0.12, 0.14, 0.18, 1.)),
        BoxSpec::default(),
        move || {
            Label(
                format!("{} / {}  {}", at + 1, count, case.name),
                8.,
                12.,
                width - 64.,
                10.,
                FG,
            );
            Button(
                Modifier::empty()
                    .absolute_offset(width - 52., 0.)
                    .size_points(52., 40.)
                    .content_description("Next review case"),
                ButtonSpec::default(),
                move || index.set((at + 1) % count),
                || {
                    Text("Next", Modifier::empty(), text_style(12., FG));
                },
            );
        },
    );
}

#[cfg(target_os = "android")]
#[path = "studio_review_android.rs"]
mod android;

#[cfg(test)]
#[test]
fn every_review_case_uses_valid_document_state() {
    let cases = review_cases();
    let mut names = std::collections::HashSet::new();
    for case in &cases {
        assert!(names.insert(&case.name), "Repeated case {}", case.name);
        let document = case_document(case);
        assert_eq!(
            document.lock().expect("document").view.brush,
            case.tool,
            "{}",
            case.name
        );
    }
    assert!(cases.len() > 90);
}
