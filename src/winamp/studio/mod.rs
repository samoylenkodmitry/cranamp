//! Built-in Cranpose pixel editor and MCP endpoint for classic Winamp skins.
#![allow(unused_braces)]
mod brush;
mod guides;
mod mapping;
mod material;
mod mcp;
mod mobile;
mod model;
pub(crate) use mobile::new_mobile_document;
pub use mobile::MobileSkinStudio;
mod study;
use cranpose_core;
use cranpose_foundation::{text::TextFieldState, PointerButton};
use cranpose_ui::text::TextUnit;
use cranpose_ui::{
    composable, Alignment, Box, BoxSpec, BoxWithConstraints, BoxWithConstraintsScope, Button,
    ButtonSpec, Color, ImageBitmap, Modifier, PointerEventKind, PointerInputScope, SpanStyle, Text,
    TextStyle,
};
use cranpose_ui_graphics::Rect;
use model::{parse_color, Document};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
#[derive(Clone)]
pub struct SharedDocument(Arc<Mutex<Document>>);
impl PartialEq for SharedDocument {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl std::ops::Deref for SharedDocument {
    type Target = Arc<Mutex<Document>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
const BG: Color = Color(0.065, 0.077, 0.10, 1.);
const CARD: Color = Color(0.105, 0.125, 0.16, 1.);
const FG: Color = Color(0.89, 0.93, 0.98, 1.);
const DIM: Color = Color(0.55, 0.63, 0.73, 1.);
const ACCENT: Color = Color(0.13, 0.40, 0.53, 1.);

fn presentation_origin(scene: [f32; 2], player: [f32; 2]) -> [f32; 2] {
    // Keep native pixels on the logical pixel grid, including scale-1 GPU
    // captures. An odd spare pixel belongs to the right or bottom margin.
    [
        ((scene[0] - player[0]) / 2.).floor(),
        ((scene[1] - player[1]) / 2.).floor(),
    ]
}

#[cfg(test)]
mod presentation_tests {
    use super::presentation_origin;

    #[test]
    fn presentation_centers_on_native_pixels_in_odd_and_even_scenes() {
        assert_eq!(
            presentation_origin([1157., 871.], [550., 754.]),
            [303., 58.]
        );
        assert_eq!(
            presentation_origin([1160., 850.], [550., 754.]),
            [305., 48.]
        );
        for scene_width in [1157., 1158.] {
            for scene_height in [870., 871.] {
                for player in [[275., 377.], [550., 754.], [275., 754.]] {
                    let scene = [scene_width, scene_height];
                    let origin = presentation_origin(scene, player);
                    for axis in 0..2 {
                        let far_margin = scene[axis] - player[axis] - origin[axis];
                        assert_eq!(origin[axis].fract(), 0.);
                        assert!((0. ..=1.).contains(&(far_margin - origin[axis])));
                    }
                }
            }
        }
    }

    #[test]
    fn presentation_keeps_centered_overflow_when_scene_is_smaller() {
        assert_eq!(
            presentation_origin([501., 701.], [550., 754.]),
            [-25., -27.]
        );
        assert_eq!(
            presentation_origin([500., 700.], [550., 754.]),
            [-25., -27.]
        );
    }
}

#[cfg(all(feature = "renderer-wgpu", not(target_os = "android")))]
static CAPTURE: std::sync::OnceLock<Mutex<cranpose::Robot>> = std::sync::OnceLock::new();

static COMPOSED_REVISION: AtomicU64 = AtomicU64::new(0);

pub fn capture_scene(revision: u64) -> anyhow::Result<image::RgbaImage> {
    #[cfg(all(feature = "renderer-wgpu", not(target_os = "android")))]
    {
        let robot = CAPTURE
            .get()
            .ok_or_else(|| anyhow::anyhow!("Studio capture is not ready"))?
            .lock()
            .map_err(|_| anyhow::anyhow!("Studio capture lock"))?;
        // MCP runs on its own thread. Wait for the requested document revision,
        // including LivePlayer's atlas/state reload, without locking the document.
        // An occluded window may not receive ordinary redraws. An offscreen
        // snapshot pumps composition without waiting for a visible presentation.
        // PumpFrames requires that presentation and stalls for hidden windows.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while COMPOSED_REVISION.load(Ordering::Acquire) < revision {
            anyhow::ensure!(
                std::time::Instant::now() < deadline,
                "Studio has not composed requested revision {revision}"
            );
            let _ = robot
                .screenshot_with_scale(1.0)
                .map_err(|e| anyhow::anyhow!(e))?;
            std::thread::sleep(Duration::from_millis(5));
        }
        let shot = robot
            .screenshot_with_scale(1.0)
            .map_err(|e| anyhow::anyhow!(e))?;
        image::RgbaImage::from_raw(shot.width, shot.height, shot.pixels)
            .ok_or_else(|| anyhow::anyhow!("Invalid captured pixel buffer"))
    }
    #[cfg(any(not(feature = "renderer-wgpu"), target_os = "android"))]
    {
        let _ = revision;
        anyhow::bail!("Scene capture requires the WGPU renderer")
    }
}

/// Open the editor in its own native window without interrupting playback.
#[cfg(not(target_os = "android"))]
pub fn launch(path: Option<&str>) -> std::io::Result<()> {
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.arg("--skin-studio");
    if let Some(path) = path {
        command.arg(path);
    }
    let mut child = command.spawn()?;
    // Reap the editor process when its window closes.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn initial_document(path: Option<&str>) -> anyhow::Result<Document> {
    let Some(path) = path else {
        let mut doc = Document::open_project(include_bytes!(
            "../../../assets/skins/Catamp Silverplay.cstudio"
        ))?;
        // The bundled artwork is an unsaved editable copy, never a developer path.
        doc.path = None;
        doc.view = model::View::default();
        doc.message = "Catamp Silverplay · editable copy".into();
        return Ok(doc);
    };
    let bytes = std::fs::read(path)?;
    if std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cstudio"))
    {
        let mut doc = Document::open_project(&bytes)?;
        doc.path = Some(
            std::path::Path::new(path)
                .with_extension("wsz")
                .to_string_lossy()
                .into_owned(),
        );
        Ok(doc)
    } else {
        Document::open(&bytes, Some(path.into()))
    }
}

#[cfg(not(target_os = "android"))]
pub fn run(path: Option<&str>) {
    let doc = initial_document(path).unwrap_or_else(|e| panic!("Open skin: {e:#}"));
    let shared = SharedDocument(Arc::new(Mutex::new(doc)));
    if let Err(e) = mcp::start(shared.clone()) {
        shared.lock().unwrap().message = format!("MCP unavailable: {e:#}");
    }
    let launcher = crate::create_surface_app()
        .with_title("Cranamp · Skin Studio")
        .with_size(1160, 850);
    #[cfg(all(feature = "renderer-wgpu", not(target_os = "android")))]
    let launcher = launcher
        .with_frame_pacing_mode(cranpose::FramePacingMode::Vsync)
        .with_test_driver(|robot| {
            let _ = CAPTURE.set(Mutex::new(robot));
        });
    launcher.run(move || SkinStudio(shared.clone()));
}
/// A handset-sized native preview for testing fractional scaling and touch UI.
#[cfg(not(target_os = "android"))]
pub fn run_touch_preview(editor: bool) {
    let document = new_mobile_document(None).expect("Bundled Studio document");
    if let Err(e) = mcp::start(document.clone()) {
        document.lock().unwrap().message = e.to_string();
    }
    let launcher = crate::create_surface_app()
        .with_title("Cranamp · Touch preview")
        .with_size(393, 780);
    #[cfg(feature = "renderer-wgpu")]
    let launcher = launcher.with_test_driver(|robot| {
        let _ = CAPTURE.set(Mutex::new(robot));
    });
    launcher.run(move || {
        if editor {
            MobileSkinStudio(document.clone(), || {}, |_, _| {});
        } else {
            super::WinampStackedApp();
        }
    });
}

pub fn stdio_bridge() {
    mcp::bridge();
}
fn text_style(size: f32, color: Color) -> TextStyle {
    TextStyle::from_span_style(SpanStyle {
        color: Some(color),
        font_size: TextUnit::Sp(size),
        ..SpanStyle::default()
    })
}
#[composable]
fn Label(text: String, x: f32, y: f32, w: f32, size: f32, color: Color) {
    Text(
        text,
        Modifier::empty().absolute_offset(x, y).width(w),
        text_style(size, color),
    );
}
#[composable]
fn Action(label: String, x: f32, y: f32, w: f32, selected: bool, click: impl Fn() + 'static) {
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(if selected { ACCENT } else { CARD })
            .rounded_corners(5.),
        ButtonSpec::default(),
        click,
        move || {
            Text(
                label.clone(),
                Modifier::empty().padding(6.),
                text_style(12., FG),
            );
        },
    );
}
fn state(shared: &SharedDocument, patch: serde_json::Value) {
    let mut d = shared.lock().unwrap();
    if let Err(e) = d.state(patch) {
        d.message = format!("{e:#}");
        d.revision += 1;
    }
}
#[composable]
pub fn SkinStudio(shared: SharedDocument) {
    let tick = cranpose_core::rememberMutableStateOf(|| 0u64);
    let live = cranpose_core::rememberMutableStateOf(|| false);
    let review = cranpose_core::rememberMutableStateOf(|| false);
    let drawer = cranpose_core::rememberMutableStateOf(|| 0u8);
    let picker = cranpose_core::rememberMutableStateOf(|| false);
    let pan = cranpose_core::rememberMutableStateOf(|| [0i32; 2]);
    let frame_kind = cranpose_core::rememberMutableStateOf(|| "volume".to_string());
    let poll = shared.clone();
    cranpose_core::LaunchedEffectAsync(0u8, move |_| {
        Box::pin(async move {
            cranpose_core::interval(Duration::from_millis(100), move || {
                let revision = poll.lock().unwrap().revision;
                if tick.get_non_reactive() != revision {
                    tick.set(revision);
                }
            })
            .await;
        })
    });
    let _ = tick.get();
    let (view, message, revision, im, path, dirty) = {
        let d = shared.lock().unwrap();
        (
            d.view.clone(),
            d.message.clone(),
            d.revision,
            if review.get() {
                d.state_sheet()
            } else {
                d.editor_render()
            },
            d.path.clone(),
            d.dirty,
        )
    };
    let export_path = path
        .as_deref()
        .map(std::path::PathBuf::from)
        .map(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Skin");
            p.with_file_name(format!("{stem} edited.wsz"))
        })
        .unwrap_or_else(|| {
            cranpose::application_directories()
                .ok()
                .and_then(|d| d.documents)
                .unwrap_or_else(std::env::temp_dir)
                .join("Skin edited.wsz")
        })
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
    let color_field = cranpose_core::remember(|| TextFieldState::new("#ffffff")).with(|f| *f);
    let (w, h) = (im.width(), im.height());
    let bitmap =
        ImageBitmap::from_rgba8(im.width(), im.height(), im.into_raw()).expect("studio bitmap");
    let panel = view.panel.clone();
    let zoom = if review.get() {
        ((900 / w).min(540 / h)).clamp(1, 3) as f32
    } else {
        view.zoom as f32
    };

    let preview_playlist_height = view.preview_playlist_height;
    let preview_size = (275, 232 + preview_playlist_height);
    let (viewport_w, viewport_h) = if live.get() { preview_size } else { (w, h) };
    let pan_value = pan.get();
    let px = pan_value[0].clamp(0, (viewport_w as f32 - 900. / zoom).ceil().max(0.) as i32) as f32
        * zoom;
    let py = pan_value[1].clamp(0, (viewport_h as f32 - 540. / zoom).ceil().max(0.) as i32) as f32
        * zoom;
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
        .unwrap_or_else(|| {
            "Auto picks the topmost sprite.\nEyedropper shows its atlas pixel.".into()
        });
    let skin_layout = shared.lock().unwrap().layout();
    let footer_layout = skin_layout.footer;
    let playlist_background = shared.lock().unwrap().has_playlist_background();
    if !view.presentation && !live.get() {
        cranpose_core::SideEffect(move || {
            COMPOSED_REVISION.fetch_max(revision, Ordering::Release);
        });
    }
    let document = shared.clone();
    Box(
        Modifier::empty().fill_max_size().background(BG),
        BoxSpec::default(),
        move || {
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
                Action(
                    "Exit presentation".into(),
                    20.,
                    18.,
                    170.,
                    false,
                    move || state(&d, json!({"presentation":false})),
                );
                return;
            }
            Label("CRANAMP  /  SKIN STUDIO".into(), 22., 18., 650., 20., FG);
            Label(
                "Native pixels. One canvas. Every sprite state.".into(),
                22.,
                49.,
                700.,
                12.,
                DIM,
            );
            Label(
                format!(
                    "{}{}",
                    path.clone().unwrap_or_else(|| "Untitled skin".into()),
                    if dirty { "  • edited" } else { "" }
                ),
                610.,
                23.,
                525.,
                11.,
                DIM,
            );
            {
                let d = document.clone();
                Action("New blank".into(), 1040., 44., 84., false, move || {
                    if let Err(e) = mcp::call("studio_new", json!({}), &d) {
                        let mut doc = d.lock().unwrap();
                        doc.message = format!("New: {e:#}");
                        doc.revision += 1;
                    } else {
                        live.set(false);
                        review.set(false);
                        pan.set([0, 0]);
                        drawer.set(0);
                    }
                });
            }
            Action(
                "Painting layers".into(),
                394.,
                44.,
                140.,
                drawer.get() == 7,
                move || drawer.set(if drawer.get_non_reactive() == 7 { 0 } else { 7 }),
            );
            {
                let d = document.clone();
                Action(
                    "Part rectangles".into(),
                    542.,
                    44.,
                    145.,
                    drawer.get() == 6,
                    move || {
                        state(&d, json!({"guides":true}));
                        drawer.set(if drawer.get_non_reactive() == 6 { 0 } else { 6 });
                    },
                );
            }
            Action(
                "Brush tools".into(),
                696.,
                44.,
                112.,
                drawer.get() == 4,
                move || drawer.set(if drawer.get_non_reactive() == 4 { 0 } else { 4 }),
            );
            {
                let d = document.clone();
                Action(
                    "Whole skin".into(),
                    816.,
                    44.,
                    112.,
                    panel == "canvas",
                    move || {
                        let mut doc = d.lock().unwrap();
                        let _ = doc.state(
                            json!({"panel":"canvas","layer":"auto","zoom":1,"presentation":false}),
                        );
                        live.set(false);
                        review.set(false);
                        pan.set([0, 0]);
                    },
                );
            }
            Action(
                "Atlases".into(),
                936.,
                44.,
                96.,
                drawer.get() == 3,
                move || {
                    drawer.set(if drawer.get_non_reactive() == 3 { 0 } else { 3 });
                },
            );
            Box(
                Modifier::empty()
                    .absolute_offset(20., 82.)
                    .size_points(1118., 54.)
                    .background(CARD)
                    .rounded_corners(6.),
                BoxSpec::default(),
                || {},
            );
            for (i, (name, title)) in [
                ("main", "Main player"),
                ("equalizer", "Equalizer"),
                ("playlist", "Playlist"),
            ]
            .iter()
            .enumerate()
            {
                let d = document.clone();
                let name = name.to_string();
                Action(
                    title.to_string(),
                    30. + i as f32 * 120.,
                    94.,
                    112.,
                    panel == name,
                    move || {
                        state(
                            &d,
                            json!({"panel":name,"layer":"auto","zoom":if name=="playlist"{2}else{3}}),
                        );
                    },
                );
            }
            {
                let d = document.clone();
                Action("Undo".into(), 410., 94., 65., false, move || {
                    d.lock().unwrap().undo();
                });
            }
            {
                let d = document.clone();
                Action("Redo".into(), 483., 94., 65., false, move || {
                    d.lock().unwrap().redo();
                });
            }
            Action(
                if live.get() {
                    "Edit canvas"
                } else {
                    "Player preview"
                }
                .into(),
                568.,
                94.,
                135.,
                live.get(),
                move || {
                    live.set(!live.get_non_reactive());
                    pan.set([0, 0]);
                    review.set(false);
                },
            );
            Label("WSZ".into(), 724., 100., 32., 11., DIM);
            cranpose_ui::BasicTextField(
                path_field,
                Modifier::empty()
                    .absolute_offset(758., 98.)
                    .size_points(190., 24.)
                    .background(BG),
                text_style(12., FG),
            );
            {
                let d = document.clone();
                Action("Open".into(), 960., 94., 72., false, move || {
                    let p = path_field.text();
                    let result = mcp::call("studio_open", json!({"path":p}), &d);
                    if let Err(e) = result {
                        let mut doc = d.lock().unwrap();
                        doc.message = format!("Open: {e:#}");
                        doc.revision += 1;
                    }
                });
            }
            {
                let d = document.clone();
                Action("Export".into(), 1040., 94., 84., false, move || {
                    let path = path_field.text();
                    let mut doc = d.lock().unwrap();
                    if let Err(e) = doc.export(std::path::Path::new(&path)) {
                        doc.message = format!("Export: {e:#}");
                        doc.revision += 1;
                    }
                });
            }
            Box(
                Modifier::empty()
                    .absolute_offset(20., 153.)
                    .size_points(190., 655.)
                    .background(CARD)
                    .rounded_corners(6.),
                BoxSpec::default(),
                || {},
            );
            Label("DRAW".into(), 32., 166., 160., 12., DIM);
            {
                let d = document.clone();
                Action(
                    view.brush.clone(),
                    30.,
                    190.,
                    78.,
                    !picker.get(),
                    move || {
                        picker.set(false);
                        state(&d, json!({"brush":"pencil"}));
                    },
                );
            }
            Action(
                "Pick pixel".into(),
                115.,
                190.,
                84.,
                picker.get(),
                move || {
                    picker.set(true);
                },
            );
            for (i, c) in [
                "#ffffff", "#d5f2fa", "#8fcae2", "#4382a4", "#15354a", "#09121d", "#ee99b2",
                "#ff00ff",
            ]
            .iter()
            .enumerate()
            {
                let d = document.clone();
                let c = c.to_string();
                let rgba = parse_color(&c).unwrap();
                Box(
                    Modifier::empty()
                        .absolute_offset(32. + (i % 4) as f32 * 41., 232. + (i / 4) as f32 * 34.)
                        .size_points(32., 25.)
                        .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                        .rounded_corners(3.)
                        .clickable(move |_| state(&d, json!({"color":c}))),
                    BoxSpec::default(),
                    || {},
                );
            }
            Label(format!("Brush {}", view.color), 32., 303., 165., 12., FG);
            cranpose_ui::BasicTextField(
                color_field,
                Modifier::empty()
                    .absolute_offset(32., 328.)
                    .size_points(100., 24.)
                    .background(BG),
                text_style(12., FG),
            );
            {
                let d = document.clone();
                Action("Set".into(), 139., 324., 58., false, move || {
                    state(&d, json!({"color":color_field.text()}));
                });
            }
            Label("EDIT SCOPE".into(), 32., 373., 160., 11., DIM);
            {
                let d = document.clone();
                let all = view.all_states;
                Action(
                    if all {
                        "All sprite states"
                    } else {
                        "Current state only"
                    }
                    .into(),
                    30.,
                    395.,
                    168.,
                    all,
                    move || state(&d, json!({"all_states":!all})),
                );
            }
            Label("TARGET LAYERS".into(), 32., 446., 165., 11., DIM);
            Label(
                if view.layers.is_empty() {
                    "Auto · topmost".into()
                } else {
                    format!("{} selected", view.layers.len())
                },
                32.,
                468.,
                165.,
                12.,
                FG,
            );
            Action(
                "Select layers…".into(),
                30.,
                495.,
                168.,
                drawer.get() == 1,
                move || drawer.set(if drawer.get_non_reactive() == 1 { 0 } else { 1 }),
            );
            Label(layer_info.clone(), 32., 542., 165., 11., DIM);
            {
                let d = document.clone();
                let time_total = footer_layout == super::skin::FooterLayout::TimeTotal;
                Action(
                    if time_total { "Time/Total" } else { "Classic" }.into(),
                    30.,
                    589.,
                    81.,
                    time_total,
                    move || {
                        let _ = d.lock().unwrap().set_layout(
                            &json!({"footer":if time_total {"classic"} else {"time-total"}}),
                            "Human",
                        );
                    },
                );
            }
            {
                let d = document.clone();
                Action(
                    "List canvas".into(),
                    117.,
                    589.,
                    81.,
                    playlist_background,
                    move || {
                        d.lock()
                            .unwrap()
                            .set_playlist_background(!playlist_background, "Human");
                    },
                );
            }
            if panel == "equalizer" {
                let d = document.clone();
                let independent = d
                    .lock()
                    .unwrap()
                    .sheets()
                    .iter()
                    .any(|(n, _, _)| n == "eqhandles.bmp");
                Action(
                    "Unique EQ art".into(),
                    742.,
                    151.,
                    180.,
                    independent,
                    move || {
                        let mut doc = d.lock().unwrap();
                        if let Err(error) = doc.set_eq_handles(!independent, "Human") {
                            doc.message = error.to_string();
                        }
                    },
                );
            }
            if panel == "main" {
                let d = document.clone();
                Action(
                    "Glass visualizer".into(),
                    742.,
                    151.,
                    180.,
                    skin_layout.visualizer_glass,
                    move || {
                        let mut doc = d.lock().unwrap();
                        if let Err(error) = doc.set_layout(
                            &json!({"visualizer_glass":!skin_layout.visualizer_glass}),
                            "Human",
                        ) {
                            doc.message = error.to_string();
                        }
                    },
                );
            }
            if panel == "playlist" {
                let d = document.clone();
                let selection = d
                    .lock()
                    .unwrap()
                    .sheets()
                    .iter()
                    .any(|(n, _, _)| n == "plselection.bmp");
                Action(
                    "Selection artwork".into(),
                    742.,
                    151.,
                    180.,
                    selection,
                    move || {
                        d.lock()
                            .unwrap()
                            .set_playlist_selection(!selection, "Human");
                    },
                );
            }
            Label("ZOOM · INTEGER ONLY".into(), 32., 629., 165., 11., DIM);
            for (i, z) in [1, 2, 3, 4, 6, 8].into_iter().enumerate() {
                let d = document.clone();
                Action(
                    format!("{z}×"),
                    30. + (i % 3) as f32 * 57.,
                    654. + (i / 3) as f32 * 34.,
                    51.,
                    view.zoom == z,
                    move || state(&d, json!({"zoom":z})),
                );
            }
            for (i, (label, dx, dy)) in [("←", -24, 0), ("→", 24, 0), ("↑", 0, -24), ("↓", 0, 24)]
                .into_iter()
                .enumerate()
            {
                Action(
                    label.into(),
                    30. + i as f32 * 43.,
                    725.,
                    38.,
                    false,
                    move || {
                        pan.update(|p| {
                            p[0] = (p[0] + dx).clamp(0, 275);
                            p[1] = (p[1] + dy).clamp(0, 493);
                        })
                    },
                );
            }
            Label(
                "MCP • 127.0.0.1:18765\n--skin-studio-mcp".into(),
                32.,
                767.,
                167.,
                10.,
                DIM,
            );
            Label(
                if live.get() {
                    "CRANAMP PLAYER RENDER".into()
                } else {
                    format!(
                        "{}  ·  {} × {} source pixels  ·  {}×",
                        panel.to_uppercase(),
                        w,
                        h,
                        view.zoom
                    )
                },
                230.,
                155.,
                790.,
                12.,
                DIM,
            );
            Action(
                if review.get() { "Canvas" } else { "States" }.into(),
                1040.,
                151.,
                88.,
                review.get(),
                move || {
                    review.set(!review.get_non_reactive());
                    live.set(false);
                },
            );
            if !live.get() {
                Action(
                    "Study".into(),
                    624.,
                    151.,
                    110.,
                    drawer.get() == 5,
                    move || drawer.set(if drawer.get_non_reactive() == 5 { 0 } else { 5 }),
                );
            }
            Action(
                "History".into(),
                936.,
                151.,
                96.,
                drawer.get() == 2,
                move || drawer.set(if drawer.get_non_reactive() == 2 { 0 } else { 2 }),
            );
            if live.get() {
                let d = document.clone();
                Action("Presentation".into(), 624., 151., 132., false, move || {
                    state(&d, json!({"presentation":true}))
                });
                let d = document.clone();
                Action(
                    if preview_playlist_height == 145 {
                        "Tall playlist"
                    } else {
                        "Compact playlist"
                    }
                    .into(),
                    765.,
                    151.,
                    160.,
                    false,
                    move || {
                        state(
                            &d,
                            json!({"preview_playlist_height": if preview_playlist_height == 145 { 261 } else { 145 }}),
                        );
                        pan.set([0, 0]);
                    },
                );
            }
            let canvas_doc = document.clone();
            let canvas_bitmap = bitmap.clone();
            let canvas_panel = panel.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(230., 188.)
                    .size_points(900., 540.)
                    .clip_to_bounds()
                    .background(Color(0.04, 0.05, 0.07, 1.)),
                BoxSpec::default(),
                move || {
                    if live.get() {
                        if view.grid && zoom >= 4. && !review.get() {
                            for x in 0..w {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(x as f32 * zoom - px, -py)
                                        .size_points(1., h as f32 * zoom)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            for y in 0..h {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(-px, y as f32 * zoom - py)
                                        .size_points(w as f32 * zoom, 1.)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                        }
                        let d = canvas_doc.clone();
                        Box(
                            Modifier::empty().absolute_offset(-px, -py).required_size(
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
                            Modifier::empty().absolute_offset(-px, -py).required_size(
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
                                        .absolute_offset(x as f32 * zoom - px, -py)
                                        .size_points(1., h as f32 * zoom)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            for y in 0..h {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(-px, y as f32 * zoom - py)
                                        .size_points(w as f32 * zoom, 1.)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                        }
                        let d = canvas_doc.clone();
                        if !review.get() && drawer.get() == 0 {
                            Box(
                                Modifier::empty()
                                    .absolute_offset(-px, -py)
                                    .required_size(cranpose_ui::Size::new(w as f32 * zoom, h as f32 * zoom))
                                    .pointer_input(
                                        (canvas_panel.clone(), view.zoom),
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
                                                                if drawer.get_non_reactive() != 0 {
                                                                    last = None;
                                                                    continue;
                                                                }
                                                                match event.kind {
                                                                    PointerEventKind::Down => {
                                                                        let mut doc =
                                                                            d.lock().unwrap();
                                                                        if picker.get_non_reactive()
                                                                        {
                                                                            let info = doc.inspect(
                                                                                point[0].max(0)
                                                                                    as u32,
                                                                                point[1].max(0)
                                                                                    as u32,
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
                                                                        } else {
                                                                            doc.checkpoint();
                                                                            let v =
                                                                                doc.view.clone();
                                                                            origin=Some(point);
                                                                            if v.brush!="pencil" {let _=doc.shape_stroke(point,point);} else {
                                                                            let _ = doc.paint_line(
                                                                                point,
                                                                                point,
                                                                                parse_color(
                                                                                    &v.color,
                                                                                )
                                                                                .unwrap(),
                                                                                "selection",
                                                                                v.all_states,
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
                                                                                if v.brush!="pencil" {let _=doc.shape_stroke(origin.unwrap_or(previous),point);} else {
                                                                                let _ = doc
                                                                                    .paint_line(
                                                                                    previous,
                                                                                    point,
                                                                                    parse_color(
                                                                                        &v.color,
                                                                                    )
                                                                                    .unwrap(),
                                                                                    "selection",
                                                                                    v.all_states,
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
                                                                        origin=None;
                                                                        if event.kind==PointerEventKind::Cancel {doc.cancel_stroke();} else {doc.finish_stroke();}
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
                    }
                },
            );
            let state_y = 744.;
            {
                let d = document.clone();
                let pressed = view.pressed;
                Action(
                    if pressed { "Pressed" } else { "Released" }.into(),
                    230.,
                    state_y,
                    110.,
                    pressed,
                    move || state(&d, json!({"pressed":!pressed})),
                );
            }
            {
                let d = document.clone();
                let active = view.active;
                Action(
                    if active {
                        "On / active"
                    } else {
                        "Off / inactive"
                    }
                    .into(),
                    350.,
                    state_y,
                    120.,
                    active,
                    move || state(&d, json!({"active":!active})),
                );
            }
            let kind = if panel == "equalizer" {
                "eq".to_string()
            } else if panel == "playlist" {
                "scroll".to_string()
            } else {
                frame_kind.get()
            };
            if panel == "main" {
                for (i, k) in ["volume", "balance", "position"].iter().enumerate() {
                    let k = k.to_string();
                    Action(
                        k.clone(),
                        488. + i as f32 * 84.,
                        state_y,
                        78.,
                        kind == k,
                        move || frame_kind.set(k.clone()),
                    );
                }
            }
            if panel == "equalizer" {
                Label(
                    format!("Travel: {} px", skin_layout.eq_travel),
                    490.,
                    state_y + 6.,
                    145.,
                    12.,
                    FG,
                );
                for (x, delta, label) in [(640., -1_i16, "−"), (692., 1, "+")] {
                    let d = document.clone();
                    Action(label.into(), x, state_y, 40., false, move || {
                        let value = (skin_layout.eq_travel as i16 + delta).clamp(1, 52);
                        let mut doc = d.lock().unwrap();
                        if let Err(error) = doc.set_layout(&json!({"eq_travel":value}), "Human") {
                            doc.message = error.to_string();
                        }
                    });
                }
            }
            let value = match kind.as_str() {
                "eq" => view.eq[0],
                "scroll" => view.scroll,
                "balance" => view.balance,
                "position" => view.position,
                _ => view.volume,
            };
            Label(
                format!("{}  {value:02}/27", kind),
                760.,
                state_y + 6.,
                150.,
                12.,
                FG,
            );
            for (delta, label, x) in [(-1, "−", 922.), (1, "+", 975.)] {
                let d = document.clone();
                let kind = kind.clone();
                Action(label.into(), x, state_y, 46., false, move || {
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
                Action("Sliders +".into(), 1032., state_y, 96., false, move || {
                    let mut doc = d.lock().unwrap();
                    let next = (doc.view.volume + 1) % 28;
                    let _=doc.state(json!({"volume":next,"balance":next,"position":next,"scroll":next,"eq":vec![next;11]}));
                });
            }
            {
                let d = document.clone();
                let kind = kind.clone();
                cranpose_ui::Slider(
                    Modifier::empty()
                        .absolute_offset(230., 786.)
                        .size_points(500., 22.),
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
                    |scope| {
                        Box(
                            Modifier::empty()
                                .absolute_offset(0., 9.)
                                .size_points(500., 3.)
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
                746.,
                790.,
                370.,
                11.,
                DIM,
            );
            Label(message.clone(), 230., 824., 895., 11., DIM);
            if drawer.get() != 0 {
                let d = document.clone();
                Box(
                    Modifier::empty()
                        .absolute_offset(750., 188.)
                        .size_points(380., 540.)
                        .background(CARD),
                    BoxSpec::default(),
                    move || {
                        Action("Close".into(), 290., 10., 78., false, move || drawer.set(0));
                        if drawer.get() == 1 {
                            LayerChooser(d.clone(), revision);
                        } else if drawer.get() == 2 {
                            HistoryChooser(d.clone(), revision);
                        } else if drawer.get() == 6 {
                            GuideChooser(d.clone(), revision);
                        } else if drawer.get() == 7 {
                            PaintChooser(d.clone(), revision);
                        } else if drawer.get() == 5 {
                            StudyChooser(d.clone(), revision);
                        } else if drawer.get() == 4 {
                            BrushChooser(d.clone(), revision);
                        } else {
                            Label("NATIVE ATLASES".into(), 12., 17., 260., 14., FG);
                            Label(
                                "Same pencil, pixel picker and undo history.".into(),
                                12.,
                                51.,
                                355.,
                                11.,
                                DIM,
                            );
                            let sheets = d.lock().unwrap().sheets();
                            for (i, (name, w, h)) in sheets.into_iter().enumerate() {
                                let target = d.clone();
                                Action(
                                    format!("{name}  {w}×{h}"),
                                    12. + (i % 2) as f32 * 178.,
                                    82. + (i / 2) as f32 * 47.,
                                    170.,
                                    false,
                                    move || {
                                        state(
                                            &target,
                                            json!({"panel":"atlas","sheet":name,"layer":"sheet","zoom":2,"presentation":false}),
                                        );
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
                                        drawer.set(0);
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
#[composable]
fn BrushChooser(shared: SharedDocument, _revision: u64) {
    let v = shared.lock().unwrap().view.clone();
    Label("PIXEL DRAWING TOOLS".into(), 12., 17., 270., 14., FG);
    Label(
        "Solid native pixels. One shared undo history.".into(),
        12.,
        51.,
        355.,
        11.,
        DIM,
    );
    for (i, (tool, label)) in [
        ("pencil", "Pencil"),
        ("line", "Line"),
        ("rect", "Rectangle"),
        ("ellipse", "Ellipse"),
        ("lift", "Lift pixels"),
        ("stamp", "Stamp"),
        ("glass", "Glass lens"),
        ("curve", "Curve"),
        ("tuft", "Fur / taper"),
    ]
    .into_iter()
    .enumerate()
    {
        let d = shared.clone();
        Action(
            label.into(),
            12. + (i % 3) as f32 * 118.,
            82. + (i / 3) as f32 * 45.,
            110.,
            v.brush == tool,
            move || state(&d, json!({"brush":tool})),
        );
    }
    Label("STROKE WIDTH".into(), 12., 210., 172., 12., DIM);
    for (i, size) in [1, 2, 3, 4, 8, 16].into_iter().enumerate() {
        let d = shared.clone();
        Action(
            format!("{size}px"),
            12. + i as f32 * 59.,
            232.,
            52.,
            v.brush_size == size,
            move || state(&d, json!({"brush_size":size})),
        );
    }
    if ["curve", "tuft"].contains(&v.brush.as_str()) {
        for (x, delta, label) in [(12., -10, "− Bend"), (248., 10, "+ Bend")] {
            let d = shared.clone();
            let bend = v.curve_bend;
            Action(label.into(), x, 271., 110., false, move || {
                state(&d, json!({"curve_bend":(bend+delta).clamp(-100,100)}))
            });
        }
        Label(
            format!("Bend {}%", v.curve_bend),
            137.,
            280.,
            108.,
            12.,
            DIM,
        );
    }
    for (i, (field, label, on)) in [
        ("filled", "Fill shapes", v.filled),
        ("mirror_x", "Mirror left / right", v.mirror_x),
        ("mirror_y", "Mirror top / bottom", v.mirror_y),
        ("grid", "Pixel grid at 4× and up", v.grid),
        ("alpha_lock", "Lock transparent pixels", v.alpha_lock),
        ("clean_corners", "Clean 1px corners", v.clean_corners),
    ]
    .into_iter()
    .enumerate()
    {
        let d = shared.clone();
        Action(
            label.into(),
            12. + (i % 2) as f32 * 178.,
            316. + (i / 2) as f32 * 48.,
            170.,
            on,
            move || state(&d, json!({field:!on})),
        );
    }
    let d = shared.clone();
    let mask_on = !v.mask_colors.is_empty();
    let picked = v.color.clone();
    Action(
        if mask_on {
            "Clear color mask".into()
        } else {
            "Mask picked color".into()
        },
        12.,
        491.,
        348.,
        mask_on,
        move || {
            state(
                &d,
                json!({"mask_colors":if mask_on {vec![]}else{vec![picked.clone()]}}),
            )
        },
    );
    Label(
        "Lift a region; Stamp places its exact pixels.".into(),
        12.,
        526.,
        353.,
        10.,
        DIM,
    );
}
#[composable]
fn LayerChooser(shared: SharedDocument, _revision: u64) {
    let (layers, selected) = {
        let d = shared.lock().unwrap();
        let mut seen = std::collections::BTreeSet::new();
        (
            d.layers()
                .into_iter()
                .filter(|l| seen.insert(l.id.clone()))
                .collect::<Vec<_>>(),
            d.view.layers.clone(),
        )
    };
    Label("PENCIL LAYERS".into(), 12., 17., 260., 14., FG);
    Label(
        "Select any combination. Solo isolates one.".into(),
        12.,
        51.,
        355.,
        11.,
        DIM,
    );
    for (i, (label, all)) in [("Auto / clear", false), ("Select all", true)]
        .into_iter()
        .enumerate()
    {
        let d = shared.clone();
        Action(
            label.into(),
            12. + i as f32 * 142.,
            75.,
            134.,
            !all && selected.is_empty(),
            move || {
                let ids = if all {
                    d.lock()
                        .unwrap()
                        .layers()
                        .into_iter()
                        .map(|l| l.id)
                        .collect::<Vec<_>>()
                } else {
                    vec![]
                };
                state(&d, json!({"layers":ids}));
            },
        );
    }
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(10., 116.)
            .size_points(360., 412.)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            for layer in &layers {
                let id = layer.id.clone();
                let checked = selected.contains(&id);
                let d = shared.clone();
                let solo_doc = shared.clone();
                let solo_id = id.clone();
                let caption = format!("{} {}", if checked { "[x]" } else { "[ ]" }, id);
                let info = format!(
                    "{} · {} states{}",
                    layer.sheet,
                    layer.variants.len(),
                    if layer.stretched() {
                        " · shared tile"
                    } else {
                        ""
                    }
                );
                Box(
                    Modifier::empty().size_points(350., 60.),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
                        let id = id.clone();
                        Action(caption.clone(), 0., 0., 267., checked, move || {
                            let mut ids = d.lock().unwrap().view.layers.clone();
                            if ids.contains(&id) {
                                ids.retain(|v| v != &id);
                            } else {
                                ids.push(id.clone());
                            }
                            state(&d, json!({"layers":ids}));
                        });
                        let d = solo_doc.clone();
                        let id = solo_id.clone();
                        Action("Solo".into(), 274., 0., 66., false, move || {
                            state(&d, json!({"layers":[id]}))
                        });
                        Label(info.clone(), 6., 34., 338., 10., DIM);
                    },
                );
            }
        },
    );
}
#[composable]
fn HistoryChooser(shared: SharedDocument, _revision: u64) {
    let history = shared.lock().unwrap().history();
    let entries = history["entries"].as_array().unwrap().clone();
    Label("EDIT HISTORY".into(), 12., 17., 260., 14., FG);
    Label(
        "Click a step to restore it. 32 edits retained.".into(),
        12.,
        51.,
        355.,
        11.,
        DIM,
    );
    Label(
        "New edits replace the undone branch.".into(),
        12.,
        74.,
        355.,
        11.,
        DIM,
    );
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(10., 106.)
            .size_points(360., 422.)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            for entry in &entries {
                let cursor = entry["cursor"].as_u64().unwrap() as usize;
                let current = entry["current"] == true;
                let caption = format!(
                    "{} {} · {}",
                    if current {
                        "●"
                    } else if entry["undone"] == true {
                        "○"
                    } else {
                        "✓"
                    },
                    cursor,
                    entry["label"].as_str().unwrap()
                );
                let caption = if caption.chars().count() > 42 {
                    format!("{}…", caption.chars().take(41).collect::<String>())
                } else {
                    caption
                };
                let source = entry["source"].as_str().unwrap().to_string();
                let d = shared.clone();
                Box(
                    Modifier::empty().size_points(350., 62.),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
                        Action(caption.clone(), 0., 0., 340., current, move || {
                            let _ = d.lock().unwrap().history_goto(cursor);
                        });
                        Label(source.clone(), 8., 35., 330., 10., DIM);
                    },
                );
            }
        },
    );
}
#[composable]
fn LivePlayer(shared: SharedDocument, revision: u64, scale: f32, playlist_height: f32) {
    let applied_revision = cranpose_core::rememberMutableStateOf(move || revision);
    let state = cranpose_core::rememberMutableStateOf(|| {
        let mut tracks = crate::audio::demo_playlist_tracks();
        for track in &mut tracks {
            if track.title.starts_with("Cranamp Demo ") {
                if let Some((_, title)) = track.title.split_once(" - ") {
                    track.title = title.to_string();
                }
            }
        }
        let mut preview = super::WinampState::default();
        if !tracks.is_empty() {
            preview.current_index = Some(0);
            preview.selected_indices = vec![0];
            preview.duration_seconds = tracks[0].duration_seconds;
            preview.playlist = std::rc::Rc::new(tracks);
            preview.status = "Skin Studio preview".into();
        }
        preview
    });
    let initial = shared.lock().unwrap().preview_archive().unwrap();
    let skin_state = cranpose_core::rememberMutableStateOf(move || {
        super::skin::load_skin(&initial).map_err(|e| format!("{e:#}"))
    });
    cranpose_core::LaunchedEffect(revision, move |_| {
        let d = shared.lock().unwrap();
        let bytes = d.preview_archive().unwrap();
        skin_state.set(super::skin::load_skin(&bytes).map_err(|e| format!("{e:#}")));
        let v = d.view.clone();
        state.update(|s| {
            s.volume = v.volume as f32 / 27.;
            s.balance = v.balance as f32 / 27.;
            s.position = v.position as f32 / 27.;
            s.eq_values = v.eq.map(|x| x as f32 / 27.);
            s.playlist_scroll = v.scroll as f32 / 27.;
            s.shuffle = v.active;
            s.repeat = v.active;
            s.eq_enabled = v.active;
            s.eq_auto = v.active;
        });
        applied_revision.set(revision);
    });
    if let Ok(skin) = skin_state.get() {
        let ready_revision = applied_revision.get();
        cranpose_core::SideEffect(move || {
            COMPOSED_REVISION.fetch_max(ready_revision, Ordering::Release);
        });
        super::WinampStackedStage(
            skin,
            state,
            skin_state,
            super::StackedLayout {
                scale,
                playlist_height,
                content_left_inset: 0.,
                content_top_inset: 0.,
            },
            super::StackedDrag::Fixed,
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn bundled_editor_project_matches_the_default_player_skin() {
        use std::io::{Cursor, Read};
        fn entries(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
            let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
            (0..zip.len())
                .map(|i| {
                    let mut file = zip.by_index(i).unwrap();
                    let name = file.name().to_owned();
                    let mut data = Vec::new();
                    file.read_to_end(&mut data).unwrap();
                    (name, data)
                })
                .collect()
        }
        let doc = initial_document(None).unwrap();
        assert!(doc.path.is_none());
        assert!(!doc.dirty);
        assert!(!doc.view.presentation);
        assert_eq!(doc.planes.len(), 7);
        let exported = entries(&doc.archive().unwrap());
        assert_eq!(exported.len(), 15);
        assert_eq!(exported, entries(super::super::BUNDLED_SKIN));
        super::super::bundled_skin().expect("Catamp must load in the player");
    }

    #[test]
    fn explicit_studio_input_does_not_fall_back_to_bundled_art() {
        assert!(initial_document(Some("missing-skin-for-startup-test.wsz")).is_err());
    }

    #[test]
    fn blank_atlases_have_no_inherited_art_and_share_native_canvas_history() {
        let mut d = Document::blank();
        super::super::skin::load_skin(&d.archive().unwrap()).unwrap();
        assert_eq!(d.sheets().len(), 13);
        d.state(json!({"panel":"atlas","sheet":"volume.bmp","layer":"sheet"}))
            .unwrap();
        assert_eq!(d.render().dimensions(), (68, 433));
        assert_eq!(d.inspect(67, 432)["hits"][0]["rgba"], json!([0, 0, 0, 0]));
        d.draw(&json!({"operations":[{"op":"pixel","x":67,"y":432,"color":"#abcdef"}]}))
            .unwrap();
        assert_eq!(
            d.inspect(67, 432)["hits"][0]["rgba"],
            json!([171, 205, 239, 255])
        );
        d.undo();
        assert_eq!(d.inspect(67, 432)["hits"][0]["rgba"], json!([0, 0, 0, 0]));
        d.redo();
        d.state(json!({"panel":"main","layer":"auto"})).unwrap();
        assert_eq!(d.render().dimensions(), (275, 115));
        assert!(d
            .state(json!({"panel":"atlas","sheet":"missing.bmp"}))
            .is_err());
    }
    #[test]
    fn mcp_and_human_edits_share_the_same_undo_history() {
        let shared = SharedDocument(Arc::new(Mutex::new(
            Document::open(include_bytes!("../../../assets/winamp.wsz"), None).unwrap(),
        )));
        let original = shared.lock().unwrap().archive().unwrap();
        // The mouse path calls exactly these operations once per gesture.
        {
            let mut d = shared.lock().unwrap();
            d.checkpoint();
            d.paint_line([40, 90], [44, 90], [17, 34, 51, 255], "play", false)
                .unwrap();
        }
        let response = mcp::dispatch(
            json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"studio_draw","arguments":{"layer":"play","all_states":true,"operations":[{"op":"pixel","x":42,"y":91,"color":"#abcdef"}]}}}),
            &shared,
        );
        assert_eq!(response["id"], 7);
        assert!(response["result"]["isError"].is_null());
        assert_eq!(shared.lock().unwrap().status()["undo"], 2);
        mcp::call("studio_undo", json!({}), &shared).unwrap();
        assert_eq!(
            shared.lock().unwrap().inspect(40, 90)["hits"][0]["rgba"],
            json!([17, 34, 51, 255])
        );
        mcp::call("studio_undo", json!({}), &shared).unwrap();
        assert_eq!(shared.lock().unwrap().archive().unwrap(), original);
    }
    #[test]
    fn frame_review_contains_every_variant() {
        let mut d = Document::open(include_bytes!("../../../assets/winamp.wsz"), None).unwrap();
        d.state(json!({"layer":"volume.track"})).unwrap();
        let sheet = d.state_sheet();
        assert_eq!(sheet.dimensions(), (532, 84));
        d.state(json!({"panel":"equalizer","layer":"band1.track"}))
            .unwrap();
        assert_eq!(d.state_sheet().dimensions(), (154, 284));
    }
}

#[composable]
fn StudyChooser(shared: SharedDocument, _revision: u64) {
    Label("NATIVE PIXEL STUDY".into(), 12., 17., 270., 14., FG);
    let values = cranpose_core::rememberMutableStateOf(|| false);
    let (im, rect) = {
        let d = shared.lock().unwrap();
        let r = d.selection.unwrap_or([0, 0, 80, 60]);
        (
            d.cluster
                .clone()
                .or_else(|| study::crop(&d.selected_image(), r).ok()),
            r,
        )
    };
    Label(
        format!(
            "Source {},{} · clipboard {}×{}",
            rect[0],
            rect[1],
            im.as_ref().map_or(rect[2], |i| i.width()),
            im.as_ref().map_or(rect[3], |i| i.height())
        ),
        12.,
        52.,
        350.,
        11.,
        DIM,
    );
    for (i, label) in ["Color", "Values"].into_iter().enumerate() {
        Action(
            label.into(),
            12. + i as f32 * 176.,
            78.,
            168.,
            values.get() == (i == 1),
            move || values.set(i == 1),
        );
    }
    if let Some(im) = im {
        let im = if values.get() {
            study::value_view(&im)
        } else {
            im
        };
        let (w, h) = im.dimensions();
        let z = (348 / w).min(260 / h).clamp(1, 4);
        let bitmap = ImageBitmap::from_rgba8(w, h, im.into_raw()).unwrap();
        for (scale, y, available_h) in [(1, 124, 76), (z, 210, 260)] {
            let bitmap = bitmap.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(12., y as f32)
                    .size_points(348., available_h as f32)
                    .clip_to_bounds(),
                BoxSpec::default(),
                move || {
                    cranpose_ui::Image(
                        cranpose_ui::BitmapRegionPainter(
                            bitmap.clone(),
                            Rect {
                                x: 0.,
                                y: 0.,
                                width: w as f32,
                                height: h as f32,
                            },
                            cranpose_ui::ImageSampling::Nearest,
                        ),
                        None,
                        Modifier::empty().required_size(cranpose_ui::Size::new(
                            (w * scale) as f32,
                            (h * scale) as f32,
                        )),
                        Alignment::TOP_START,
                        cranpose_ui::ContentScale::FillBounds,
                        1.,
                        None,
                    );
                },
            );
        }
    }
    for (i, label) in ["Flip H", "Flip V", "Turn 90°"].into_iter().enumerate() {
        let d = shared.clone();
        Action(
            label.into(),
            12. + i as f32 * 118.,
            485.,
            110.,
            false,
            move || {
                let mut doc = d.lock().unwrap();
                if let Err(e) = doc.transform_cluster(i == 0, i == 1, u32::from(i == 2)) {
                    doc.message = e.to_string();
                    doc.revision += 1;
                }
            },
        );
    }
    Label("Lift pixels to choose a region. Clipboard transforms\nare exact; use Stamp to paint into selected layers.".into(),12.,527.,350.,10.,DIM);
}

#[composable]
fn GuideChooser(shared: SharedDocument, _revision: u64) {
    let filter = cranpose_core::rememberMutableStateOf(|| false);
    let doc = shared.lock().unwrap();
    let selected = doc.selection;
    // Preserve the canvas guide numbers when narrowing the list.
    let mut guides: Vec<_> = doc.guides().into_iter().enumerate().collect();
    if filter.get() {
        if let Some(r) = selected {
            guides.retain(|(_, g)| guides::intersection(g.rect, r).is_some());
        }
    }
    drop(doc);
    Label("SPRITE RECTANGLES".into(), 12., 17., 270., 14., FG);
    {
        let d = shared.clone();
        Action("Hide guides".into(), 12., 54., 165., false, move || {
            state(&d, json!({"guides":false}))
        });
    }
    {
        let d = shared.clone();
        Action(
            "Clear paint clip".into(),
            190.,
            54.,
            165.,
            false,
            move || state(&d, json!({"clip":null})),
        );
    }
    Action(
        if filter.get() {
            "Show all parts".into()
        } else {
            "Parts in lifted region".into()
        },
        12.,
        94.,
        348.,
        filter.get(),
        move || filter.set(!filter.get()),
    );
    Label(
        if selected.is_some() {
            "Pink regions include live text and timer digits.".into()
        } else {
            "Lift a native region to filter its source parts.".into()
        },
        12.,
        132.,
        350.,
        10.,
        DIM,
    );
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(12., 156.)
            .size_points(354., 376.)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            for (index, g) in &guides {
                let d = shared.clone();
                let id = g.id.clone();
                let text = format!("{} · {:?}", g.sheet, g.source);
                let name = format!("{} · {}", index + 1, g.id);
                Box(
                    Modifier::empty().size_points(348., 60.),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
                        let id = id.clone();
                        Action(name.clone(), 0., 0., 342., false, move || {
                            let _ = d.lock().unwrap().select_guide(&id);
                        });
                        Label(text.clone(), 3., 36., 340., 10., DIM);
                    },
                );
            }
        },
    );
}
fn plane_action(d: &SharedDocument, args: serde_json::Value) {
    let mut doc = d.lock().unwrap();
    if let Err(e) = doc.paint_layer_command(&args, "Human") {
        doc.message = e.to_string();
        doc.revision += 1;
    }
}
#[composable]
fn PaintChooser(shared: SharedDocument, _revision: u64) {
    let info = shared.lock().unwrap().paint_layer_info();
    let planes = info["layers"].as_array().unwrap().clone();
    let active = info["active"].as_str().map(str::to_owned);
    Label("PAINTING LAYERS".into(), 12., 17., 270., 14., FG);
    {
        let d = shared.clone();
        Action("+ New layer".into(), 12., 53., 108., false, move || {
            plane_action(&d, json!({"action":"add"}))
        });
    }
    {
        let d = shared.clone();
        Action(
            "Base atlases".into(),
            130.,
            53.,
            108.,
            active.is_none(),
            move || plane_action(&d, json!({"action":"select","id":"base"})),
        );
    }
    {
        let d = shared.clone();
        Action("Save project".into(), 248., 53., 108., false, move || {
            let mut doc = d.lock().unwrap();
            let p = std::path::PathBuf::from(doc.path.as_deref().unwrap_or("Untitled.wsz"))
                .with_extension("cstudio");
            if let Err(e) = doc.save_project(&p) {
                doc.message = e.to_string();
                doc.revision += 1;
            }
        });
    }
    let initial = active
        .as_ref()
        .and_then(|id| planes.iter().find(|p| p["id"] == *id))
        .and_then(|p| p["name"].as_str())
        .unwrap_or("")
        .to_owned();
    let field = cranpose_core::remember(move || TextFieldState::new(&initial)).with(|f| *f);
    let selected_name = active
        .as_ref()
        .and_then(|id| planes.iter().find(|p| p["id"] == *id))
        .and_then(|p| p["name"].as_str())
        .unwrap_or("")
        .to_owned();
    cranpose_core::LaunchedEffect(active.clone(), move |_| {
        field.set_text(selected_name);
    });
    cranpose_ui::BasicTextField(
        field,
        Modifier::empty()
            .absolute_offset(12., 99.)
            .size_points(240., 24.)
            .background(BG),
        text_style(12., FG),
    );
    {
        let d = shared.clone();
        Action("Rename".into(), 264., 94., 92., false, move || {
            plane_action(&d, json!({"action":"set","name":field.text()}))
        });
    }
    let layer_doc = shared.clone();
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(12., 130.)
            .size_points(354., 298.)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            for p in planes.iter().rev() {
                let d = layer_doc.clone();
                let id = p["id"].as_str().unwrap().to_owned();
                let on = active.as_deref() == Some(&id);
                let visible = p["visible"] == true;
                let locked = p["locked"] == true;
                let name = p["name"].as_str().unwrap().to_owned();
                Box(
                    Modifier::empty().size_points(348., 79.),
                    BoxSpec::default(),
                    move || {
                        {
                            let d = d.clone();
                            let id = id.clone();
                            Action(name.clone(), 0., 0., 340., on, move || {
                                plane_action(&d, json!({"action":"select","id":id}))
                            });
                        }
                        {
                            let d = d.clone();
                            let id = id.clone();
                            Action(
                                if visible { "Visible" } else { "Hidden" }.into(),
                                0.,
                                38.,
                                162.,
                                visible,
                                move || {
                                    plane_action(
                                        &d,
                                        json!({"action":"set","id":id,"visible":!visible}),
                                    )
                                },
                            );
                        }
                        {
                            let d = d.clone();
                            let id = id.clone();
                            Action(
                                if locked { "Locked" } else { "Unlocked" }.into(),
                                172.,
                                38.,
                                168.,
                                locked,
                                move || {
                                    plane_action(
                                        &d,
                                        json!({"action":"set","id":id,"locked":!locked}),
                                    )
                                },
                            );
                        }
                    },
                );
            }
        },
    );
    if let Some(id) = info["active"].as_str() {
        let id = id.to_owned();
        let index = info["layers"]
            .as_array()
            .unwrap()
            .iter()
            .position(|p| p["id"] == id)
            .unwrap();
        let opacity = info["layers"][index]["opacity"].as_u64().unwrap();
        for (i, label) in ["Down", "Up", "Merge", "Delete"].into_iter().enumerate() {
            let d = shared.clone();
            let id = id.clone();
            let n = info["layers"].as_array().unwrap().len();
            Action(
                label.into(),
                12. + i as f32 * 88.,
                435.,
                82.,
                false,
                move || {
                    plane_action(
                        &d,
                        match i {
                            0 => json!({"action":"move","id":id,"index":index.saturating_sub(1)}),
                            1 => json!({"action":"move","id":id,"index":(index+1).min(n-1)}),
                            2 => json!({"action":"merge_down","id":id}),
                            _ => json!({"action":"delete","id":id}),
                        },
                    )
                },
            );
        }
        let d = shared.clone();
        let target = id.clone();
        let end_doc = shared.clone();
        cranpose_ui::Slider(
            Modifier::empty()
                .absolute_offset(12., 484.)
                .size_points(330., 22.),
            opacity as f32 / 255.,
            move |v| {
                let _ = d
                    .lock()
                    .unwrap()
                    .opacity_stroke(&target, (v * 255.).round() as u8);
            },
            move || {
                end_doc.lock().unwrap().finish_opacity_stroke();
            },
            cranpose_ui::SliderSpec::new().thumb_extent(12.),
            |scope| {
                Box(
                    Modifier::empty()
                        .absolute_offset(0., 9.)
                        .size_points(330., 3.)
                        .background(BG),
                    BoxSpec::default(),
                    || {},
                );
                Box(
                    Modifier::empty()
                        .absolute_offset(scope.thumb_offset(), 3.)
                        .size_points(12., 16.)
                        .background(ACCENT),
                    BoxSpec::default(),
                    || {},
                );
            },
        );
        {
            let d = shared.clone();
            let clipped = info["layers"][index]["clip_below"] == true;
            let target = id.clone();
            Action(
                if clipped {
                    "Clipped to layer below"
                } else {
                    "Clip to layer below"
                }
                .into(),
                174.,
                508.,
                182.,
                clipped,
                move || {
                    plane_action(
                        &d,
                        json!({"action":"set","id":target,"clip_below":!clipped}),
                    );
                },
            );
        }
        Label(
            format!("Opacity {}%", opacity * 100 / 255),
            12.,
            513.,
            150.,
            10.,
            DIM,
        );
    }
}
