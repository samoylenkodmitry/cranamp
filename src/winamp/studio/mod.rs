#![allow(unused_braces)]
#[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
mod agent_server;
mod brush;
pub(crate) mod cursor_art;
mod design;
mod draft;
mod equalizer;
mod guides;
mod mapping;
mod material;
#[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
mod mcp;
mod model;
pub(crate) use draft::new_mobile_document;
use draft::{load_draft, publish, save_draft, store_draft};
mod adaptive;
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
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
#[derive(Clone)]
pub struct StudioHost {
    pub close: Rc<dyn Fn()>,
    pub apply: Rc<dyn Fn(Vec<u8>, String)>,
}
impl PartialEq for StudioHost {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.close, &other.close) && Rc::ptr_eq(&self.apply, &other.apply)
    }
}
#[derive(Clone)]
pub struct SharedDocument(Arc<Mutex<Document>>);
impl PartialEq for SharedDocument {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl SharedDocument {
    pub fn rendered_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let doc = self.0.lock().ok()?;
        doc.render().get_pixel_checked(x, y).map(|p| p.0)
    }
}
impl std::ops::Deref for SharedDocument {
    type Target = Arc<Mutex<Document>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
use design::{ACCENT, ACCENT_LIT, BG, CARD, DIM, FG, PIP_OFF};
fn presentation_origin(scene: [f32; 2], player: [f32; 2]) -> [f32; 2] {
    [
        ((scene[0] - player[0]) / 2.).floor(),
        ((scene[1] - player[1]) / 2.).floor(),
    ]
}
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/presentation_tests.rs"]
mod presentation_tests;
#[cfg(all(
    feature = "renderer-wgpu",
    not(target_os = "android"),
    not(target_os = "ios"),
    not(target_arch = "wasm32")
))]
static CAPTURE: std::sync::OnceLock<Mutex<cranpose::Robot>> = std::sync::OnceLock::new();
static COMPOSED_REVISION: AtomicU64 = AtomicU64::new(0);
static PLAYER_SCENE: Mutex<Option<[f32; 3]>> = Mutex::new(None);
fn note_player_scene(origin: [f32; 2], zoom: f32) {
    if let Ok(mut at) = PLAYER_SCENE.lock() {
        *at = Some([origin[0], origin[1], zoom]);
    }
}
pub fn player_scene_rect(canvas: [u32; 4]) -> Option<[u32; 4]> {
    let [x, y, zoom] = (*PLAYER_SCENE.lock().ok()?)?;
    Some([
        (x + canvas[0] as f32 * zoom).round().max(0.) as u32,
        (y + canvas[1] as f32 * zoom).round().max(0.) as u32,
        (canvas[2] as f32 * zoom).round().max(1.) as u32,
        (canvas[3] as f32 * zoom).round().max(1.) as u32,
    ])
}
pub fn player_scene() -> Option<[f32; 3]> {
    *PLAYER_SCENE.lock().ok()?
}
static ALERT_REVISION: AtomicU64 = AtomicU64::new(u64::MAX);
pub fn capture_scene(revision: u64) -> anyhow::Result<image::RgbaImage> {
    #[cfg(all(
        feature = "renderer-wgpu",
        not(target_os = "android"),
        not(target_os = "ios"),
        not(target_arch = "wasm32")
    ))]
    {
        let robot = CAPTURE
            .get()
            .ok_or_else(|| anyhow::anyhow!("Studio capture is not ready"))?
            .lock()
            .map_err(|_| anyhow::anyhow!("Studio capture lock"))?;
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
    #[cfg(any(
        not(feature = "renderer-wgpu"),
        target_os = "android",
        target_os = "ios",
        target_arch = "wasm32"
    ))]
    {
        let _ = revision;
        anyhow::bail!("Scene capture requires the WGPU renderer on a native window")
    }
}
fn studio_open_options() -> cranpose::FilePickerOptions {
    let options = cranpose::FilePickerOptions::default().with_title("Open skin or Studio project");
    // Document providers do not consistently label custom archive extensions.
    // Read the selected bytes and let the archive loader validate the contents.
    if cfg!(any(target_os = "android", target_os = "ios")) {
        options
    } else {
        options.with_filter(cranpose::FileFilter::new(
            "Skin/project",
            &["wsz", "zip", "cstudio"],
        ))
    }
}
fn open_on_whole_skin(doc: &mut Document) {
    doc.open_on_whole_skin();
}
#[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
pub fn launch(path: Option<&str>) -> std::io::Result<()> {
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command
        .arg("--skin-studio-owned")
        .stdin(std::process::Stdio::piped());
    if let Some(path) = path {
        command.arg(path);
    }
    let mut child = command.spawn()?;
    let parent = child.stdin.take();
    std::thread::Builder::new()
        .name("studio-lifetime".into())
        .spawn(move || {
            let _ = child.wait();
            drop(parent);
        })?;
    Ok(())
}
fn initial_document(path: Option<&str>) -> anyhow::Result<Document> {
    let Some(path) = path else {
        let mut doc = Document::open(
            include_bytes!("../../../assets/skins/Catamp Silverplay.wsz"),
            None,
        )?;
        doc.path = None;
        doc.view = model::View::default();
        doc.message = "Catamp Silverplay · editable copy".into();
        doc.view.zoom = 1;
        doc.view.drawer = "none".into();
        open_on_whole_skin(&mut doc);
        return Ok(doc);
    };
    #[cfg(not(target_arch = "wasm32"))]
    let bytes = crate::content_io::read_document(path)?;
    #[cfg(target_arch = "wasm32")]
    let bytes = super::browser_skins::load(cranpose_services::preferences().as_ref(), path)
        .map_err(anyhow::Error::msg)?;
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
        open_on_whole_skin(&mut doc);
        Ok(doc)
    } else {
        let mut doc = Document::open(&bytes, Some(path.into()))?;
        open_on_whole_skin(&mut doc);
        Ok(doc)
    }
}
pub const DESKTOP_LAYOUT_MIN: (f32, f32) = (1140., 820.);
#[composable]
pub fn AdaptiveSkinStudio(
    shared: SharedDocument,
    host: StudioHost,
    visible: bool,
    bottom_inset: f32,
) {
    adaptive::MountedSkinStudio(shared, Some(host), visible, bottom_inset);
}
pub fn open_document(path: Option<&str>) -> anyhow::Result<SharedDocument> {
    let document = initial_document(path)?;
    Ok(SharedDocument(Arc::new(Mutex::new(document))))
}
#[cfg(all(
    not(target_os = "android"),
    not(target_os = "ios"),
    not(target_arch = "wasm32")
))]
pub fn run(path: Option<&str>, owned: bool) {
    let doc = initial_document(path).unwrap_or_else(|e| panic!("Open skin: {e:#}"));
    let shared = SharedDocument(Arc::new(Mutex::new(doc)));
    let launcher = crate::create_surface_app()
        .with_title("Cranamp · Skin Studio")
        .with_size(1388, 1000);
    #[cfg(all(feature = "renderer-wgpu", not(target_os = "android")))]
    let launcher = launcher
        .with_frame_pacing_mode(cranpose::FramePacingMode::Vsync)
        .with_test_driver(|robot| {
            let _ = CAPTURE.set(Mutex::new(robot));
        });
    launcher.run(move || {
        cranpose_core::LaunchedEffect(owned, move |_| {
            if owned {
                std::thread::spawn(|| {
                    let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink());
                    cranpose_services::request_exit();
                });
            }
        });
        SkinStudio(shared.clone(), None);
    });
}
#[cfg(all(
    not(target_os = "android"),
    not(target_os = "ios"),
    not(target_arch = "wasm32")
))]
pub fn run_touch_preview(editor: bool) {
    let document = new_mobile_document(None).expect("Bundled Studio document");
    let launcher = crate::create_surface_app()
        .with_title("Cranamp · Touch preview")
        .with_developer_inspector(false)
        .with_size(393, 780);
    #[cfg(feature = "renderer-wgpu")]
    let launcher = launcher.with_test_driver(|robot| {
        let _ = CAPTURE.set(Mutex::new(robot));
    });
    launcher.run(move || {
        if editor {
            SkinStudio(document.clone(), None);
        } else {
            super::WinampMobileApp();
        }
    });
}
#[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
pub fn stdio_bridge() {
    agent_server::bridge();
}
fn text_style(size: f32, color: Color) -> TextStyle {
    TextStyle::from_span_style(SpanStyle {
        color: Some(color),
        font_size: TextUnit::Sp(size),
        font_weight: Some(if size >= 14. {
            cranpose_ui::text::FontWeight::MEDIUM
        } else {
            cranpose_ui::text::FontWeight::NORMAL
        }),
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
fn Action(label: String, x: f32, y: f32, w: f32, click: impl Fn() + 'static) {
    Slab(label, x, y, w, CARD, click);
}
#[composable]
fn MaybeAction(label: String, x: f32, y: f32, w: f32, enabled: bool, click: impl Fn() + 'static) {
    if enabled {
        Slab(label, x, y, w, CARD, click);
        return;
    }
    Box(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(
                design::Surface::Button,
                false,
                false,
                false,
            )),
        BoxSpec::default(),
        move || {
            Text(
                label.clone(),
                Modifier::empty().padding(6.),
                text_style(12., PIP_OFF),
            );
        },
    );
}
#[composable]
fn Slab(label: String, x: f32, y: f32, w: f32, fill: Color, click: impl Fn() + 'static) {
    let hover = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(
                design::Surface::Button,
                fill == ACCENT,
                false,
                false,
            ))
            .then(hover),
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
#[composable]
fn Panel(label: String, x: f32, y: f32, w: f32, open: bool, click: impl Fn() + 'static) {
    let hover = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(design::Surface::Button, open, false, false))
            .then(hover),
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
    Box(
        Modifier::empty()
            .absolute_offset(x + 7., y + 24.)
            .size_points(w - 14., 2.)
            .background(if open { FG } else { DIM })
            .rounded_corners(1.),
        BoxSpec::default(),
        || {},
    );
}
#[composable]
fn ListToggle(label: String, x: f32, y: f32, w: f32, on: bool, click: impl Fn() + 'static) {
    let hover = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(design::Surface::Button, on, false, false))
            .then(hover),
        ButtonSpec::default(),
        click,
        move || {
            Text(
                label.clone(),
                Modifier::empty().width(w - 34.),
                text_style(12., FG),
            );
        },
    );
    Box(
        Modifier::empty()
            .absolute_offset(x + w - 17., y + 11.)
            .size_points(8., 8.)
            .background(if on { ACCENT_LIT } else { PIP_OFF })
            .rounded_corners(4.),
        BoxSpec::default(),
        || {},
    );
}
#[composable]
fn ListChoice(label: String, x: f32, y: f32, w: f32, current: bool, click: impl Fn() + 'static) {
    let hover = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(
                design::Surface::Button,
                current,
                false,
                false,
            ))
            .then(hover),
        ButtonSpec::default(),
        click,
        move || {
            Text(
                label.clone(),
                Modifier::empty().width(w - 28.),
                text_style(12., FG),
            );
        },
    );
}
#[composable]
fn Choice(label: String, x: f32, y: f32, w: f32, current: bool, click: impl Fn() + 'static) {
    let hover = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(
                design::Surface::Button,
                current,
                false,
                false,
            ))
            .then(hover),
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
#[composable]
fn Toggle(label: String, x: f32, y: f32, w: f32, on: bool, click: impl Fn() + 'static) {
    let hover = adaptive::controls::feedback(label.clone());
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .then(design::surface(design::Surface::Button, on, false, false))
            .then(hover),
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
    Box(
        Modifier::empty()
            .absolute_offset(x + w - 17., y + 11.)
            .size_points(8., 8.)
            .background(if on { ACCENT_LIT } else { PIP_OFF })
            .rounded_corners(4.),
        BoxSpec::default(),
        || {},
    );
}
fn note(shared: &SharedDocument, message: String) {
    let mut d = shared.lock().unwrap();
    d.notify(message);
    d.revision += 1;
    ALERT_REVISION.store(d.revision, Ordering::Release);
}
#[composable]
fn BrushCursor(
    hover: cranpose_core::MutableState<Option<[i32; 2]>>,
    zoom: f32,
    size: u32,
    ox: f32,
    oy: f32,
) {
    let Some(point) = hover.get() else { return };
    let half = (size.max(1) - 1) / 2;
    let x = ox + (point[0] - half as i32) as f32 * zoom;
    let y = oy + (point[1] - half as i32) as f32 * zoom;
    let side = size.max(1) as f32 * zoom;
    for (dx, dy, w, h) in [
        (0., 0., side, 1.),
        (0., side - 1., side, 1.),
        (0., 0., 1., side),
        (side - 1., 0., 1., side),
    ] {
        Box(
            Modifier::empty()
                .absolute_offset(x + dx, y + dy)
                .size_points(w, h)
                .background(Color(1., 1., 1., 0.75)),
            BoxSpec::default(),
            || {},
        );
    }
}
fn drawer_id(name: &str) -> u8 {
    match name {
        "targets" => 1,
        "history" => 2,
        "atlases" => 3,
        "tools" => 4,
        "study" => 5,
        "rectangles" => 6,
        "layers" => 7,
        "picker" => 8,
        "options" => 9,
        "equalizer" => 10,
        #[cfg(not(any(target_os = "ios", target_os = "android", target_arch = "wasm32")))]
        "agent" => 11,
        _ => 0,
    }
}
fn drawer_name(id: u8) -> &'static str {
    match id {
        1 => "targets",
        2 => "history",
        3 => "atlases",
        4 => "tools",
        5 => "study",
        6 => "rectangles",
        7 => "layers",
        8 => "picker",
        9 => "options",
        10 => "equalizer",
        11 => "agent",
        _ => "none",
    }
}
type Tick = cranpose_core::MutableState<u64>;
fn set_drawer(shared: &SharedDocument, tick: Tick, id: u8) {
    state(shared, json!({ "drawer": drawer_name(id) }));
    tick.set(shared.lock().unwrap().revision);
}
fn state(shared: &SharedDocument, patch: serde_json::Value) {
    let mut d = shared.lock().unwrap();
    if let Err(e) = d.state(patch) {
        d.notify(format!("{e:#}"));
        d.revision += 1;
        ALERT_REVISION.store(d.revision, Ordering::Release);
    }
}
#[cfg(feature = "studio-review")]
pub use adaptive::review::{review_cases, DesignSystem, ReviewGallery};
pub use adaptive::SkinStudio;

#[composable]
fn LayerChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    let (width, height) = room;
    let filter = cranpose_core::remember(|| TextFieldState::new("")).with(|f| *f);
    let needle = filter.text().to_ascii_lowercase();
    let (layers, selected, total) = {
        let d = shared.lock().unwrap();
        let mut seen = std::collections::BTreeSet::new();
        let all: Vec<_> = d
            .layers()
            .into_iter()
            .filter(|l| seen.insert(l.id.clone()))
            .collect();
        let total = all.len();
        let shown = all
            .into_iter()
            .filter(|l| needle.is_empty() || l.id.to_ascii_lowercase().contains(&needle))
            .collect::<Vec<_>>();
        (shown, d.view.layers.clone(), total)
    };
    Label("SPRITE TARGETS".into(), 12., 17., 260., 14., FG);
    Label(
        "Which sprites a stroke paints into.\nAny combination; Solo isolates one.".into(),
        12.,
        43.,
        width,
        11.,
        DIM,
    );
    for (i, (label, all)) in [
        ("Auto · every sprite", false),
        ("Every sprite listed", true),
    ]
    .into_iter()
    .enumerate()
    {
        let d = shared.clone();
        Choice(
            label.into(),
            12. + i as f32 * (width / 2. + 4.),
            84.,
            width / 2. - 4.,
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
    cranpose_ui::BasicTextField(
        filter,
        Modifier::empty()
            .absolute_offset(12., 124.)
            .size_points(width - 140., 26.)
            .then(design::surface(design::Surface::Field, false, false, false))
            .padding_symmetric(6., 4.),
        text_style(12., FG),
    );
    Label(
        if needle.is_empty() {
            format!("type to filter {total}")
        } else {
            format!("{} of {total}", layers.len())
        },
        width - 130.,
        130.,
        140.,
        11.,
        DIM,
    );
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(10., 162.)
            .size_points(width + 4., (height - 178.).max(60.))
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
                let caption = id.clone();
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
                    Modifier::empty().size_points(width, 52.),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
                        let id = id.clone();
                        ListToggle(caption.clone(), 2., 0., width - 74., checked, move || {
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
                        Action("Solo".into(), width - 66., 0., 64., move || {
                            state(&d, json!({"layers":[id]}))
                        });
                        Label(info.clone(), 8., 32., width - 16., 10., DIM);
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
    let initial = {
        let doc = shared.lock().unwrap();
        doc.preview_archive().unwrap()
    };
    let skin_state = cranpose_core::rememberMutableStateOf(move || {
        super::skin::load_skin(&initial).map_err(|e| format!("{e:#}"))
    });
    cranpose_core::LaunchedEffect(revision, move |_| {
        let d = shared.lock().unwrap();
        let bytes = d.preview_archive().unwrap();
        skin_state.set(super::skin::load_skin(&bytes).map_err(|e| format!("{e:#}")));
        let v = d.view.clone();
        state.update(|s| {
            s.playback = match v.playback {
                1 => super::PlaybackState::Playing,
                2 => super::PlaybackState::Paused,
                _ => super::PlaybackState::Stopped,
            };
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
#[path = "../../../test/unit/winamp/studio/integration_tests.rs"]
mod integration_tests;
fn hsv_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = (h.rem_euclid(1.)) * 6.;
    let c = v * s;
    let x = c * (1. - ((h % 2.) - 1.).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.),
        1 => (x, c, 0.),
        2 => (0., c, x),
        3 => (0., x, c),
        4 => (x, 0., c),
        _ => (c, 0., x),
    };
    let m = v - c;
    [
        ((r + m) * 255.).round().clamp(0., 255.) as u8,
        ((g + m) * 255.).round().clamp(0., 255.) as u8,
        ((b + m) * 255.).round().clamp(0., 255.) as u8,
    ]
}
fn rgb_hsv(c: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255., c[1] as f32 / 255., c[2] as f32 / 255.);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= f32::EPSILON {
        0.
    } else if max == r {
        ((g - b) / d).rem_euclid(6.)
    } else if max == g {
        (b - r) / d + 2.
    } else {
        (r - g) / d + 4.
    } / 6.;
    (h, if max > 0. { d / max } else { 0. }, max)
}

#[composable]
fn StudyChooser(
    shared: SharedDocument,
    _revision: u64,
    room: (f32, f32),
    hover: cranpose_core::MutableState<Option<[i32; 2]>>,
) {
    let values = cranpose_core::rememberMutableStateOf(|| false);
    let context = cranpose_core::rememberMutableStateOf(|| true);
    let at = hover.get().map(|p| {
        [
            (p[0].max(0) / 16 * 16) as u32,
            (p[1].max(0) / 16 * 16) as u32,
        ]
    });
    let (im, rect, chosen, coverage, flat_count) = {
        let d = shared.lock().unwrap();
        let chosen = d.selection.is_some();
        let mut r = d.selection.unwrap_or_else(|| match at {
            Some([x, y]) => [x.saturating_sub(40), y.saturating_sub(30), 80, 60],
            None => [0, 0, 80, 60],
        });
        let image = d.render();
        r[0] = r[0].min(image.width().saturating_sub(1));
        r[1] = r[1].min(image.height().saturating_sub(1));
        r[2] = r[2].min(image.width() - r[0]);
        r[3] = r[3].min(image.height() - r[1]);
        let r = study::context_rect(r, image.dimensions(), if context.get() { 4 } else { 0 })
            .unwrap_or(r);
        let coverage = d.coverage(r, false).ok().map(|(report, _)| report);
        let flat_count = d
            .flat_regions(r)
            .ok()
            .and_then(|v| v["regions"].as_array().map(Vec::len))
            .unwrap_or(0);
        (study::crop(&image, r).ok(), r, chosen, coverage, flat_count)
    };
    Label(
        if chosen {
            "Selected pixels"
        } else if at.is_some() {
            "Pointer preview"
        } else {
            "Top-left preview"
        }
        .into(),
        12.,
        17.,
        room.0,
        14.,
        FG,
    );
    Label(
        if chosen {
            format!(
                "Selection at {},{} · {}×{} pixels",
                rect[0],
                rect[1],
                im.as_ref().map_or(rect[2], |i| i.width()),
                im.as_ref().map_or(rect[3], |i| i.height())
            )
        } else if at.is_some() {
            format!(
                "Following the pointer at {},{}. Lift a region with the\nLift pixels brush to pin it here instead.",
                rect[0], rect[1]
            )
        } else {
            "Use Lift pixels to inspect a region.\nThis preview does not change the skin.".into()
        },
        12.,
        46.,
        room.0,
        11.,
        DIM,
    );
    for (i, label) in ["Color", "Values"].into_iter().enumerate() {
        Choice(
            label.into(),
            12. + i as f32 * (room.0 / 2. + 4.),
            96.,
            room.0 / 2. - 4.,
            values.get() == (i == 1),
            move || values.set(i == 1),
        );
    }
    Toggle(
        "Include 4px surroundings".into(),
        12.,
        128.,
        room.0,
        context.get(),
        move || context.set(!context.get()),
    );
    if let Some(im) = im {
        let im = if values.get() {
            study::value_view(&im)
        } else {
            im
        };
        let (w, h) = im.dimensions();
        let detail_h = (room.1 - 320.).max(100.);
        let z = ((room.0 as u32 - 8) / w)
            .min(detail_h as u32 / h)
            .clamp(1, 6);
        let bitmap = ImageBitmap::from_rgba8(w, h, im.into_raw()).unwrap();
        for (scale, y, available_h) in [(1, 168., 76.), (z, 254., detail_h)] {
            let bitmap = bitmap.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(12., y)
                    .size_points(room.0, available_h)
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
    let summary = room.1 - 52.;
    if flat_count > 0 {
        Label(
            format!("{flat_count} flat regions"),
            12.,
            summary,
            room.0,
            10.,
            FG,
        );
    }
    if let Some(coverage) = coverage {
        let c = &coverage["counts"];
        Label(
            format!(
                "Paintable {} · hidden {} · palette {}",
                c["paintable"], c["opaque_runtime"], c["palette_only"]
            ),
            12.,
            summary + 20.,
            room.0,
            10.,
            DIM,
        );
    }
}

#[composable]
fn GuideChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    let filter = cranpose_core::rememberMutableStateOf(|| false);
    let doc = shared.lock().unwrap();
    let selected = doc.selection;
    let mut guides: Vec<_> = doc.guides().into_iter().enumerate().collect();
    if filter.get() {
        if let Some(r) = selected {
            guides.retain(|(_, g)| guides::intersection(g.rect, r).is_some());
        }
    }
    drop(doc);
    let outlining = { shared.lock().unwrap().view.guides };
    Label("SPRITE RECTANGLES".into(), 12., 17., 270., 14., FG);
    Label(
        "Where each sprite lives in the joined skin. The editor\noutlines the one under the pointer; it never draws into\nthe artwork."
            .into(),
        12.,
        40.,
        room.0,
        11.,
        DIM,
    );
    {
        let d = shared.clone();
        Toggle(
            if outlining {
                "Outlining the sprite under the pointer"
            } else {
                "Outline is off"
            }
            .into(),
            12.,
            94.,
            room.0 / 2. + 20.,
            outlining,
            move || state(&d, json!({ "guides": !outlining })),
        );
    }
    {
        let d = shared.clone();
        Action(
            "Clear paint clip".into(),
            room.0 / 2. + 40.,
            94.,
            room.0 / 2. - 28.,
            move || state(&d, json!({"clip":null})),
        );
    }
    Toggle(
        if filter.get() {
            "Only the parts in the lifted region".into()
        } else {
            "Every part".into()
        },
        12.,
        132.,
        room.0,
        filter.get(),
        move || filter.set(!filter.get()),
    );
    Label(
        if selected.is_some() {
            "Pink regions carry live text and timer digits.".into()
        } else {
            "Lift a region to narrow this to its own parts.".into()
        },
        12.,
        170.,
        room.0,
        10.,
        DIM,
    );
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(12., 192.)
            .size_points(room.0, (room.1 - 208.).max(60.))
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
                    Modifier::empty().size_points(room.0, 52.),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
                        let id = id.clone();
                        ListChoice(name.clone(), 0., 0., room.0 - 6., false, move || {
                            let _ = d.lock().unwrap().select_guide(&id);
                        });
                        Label(text.clone(), 8., 32., room.0 - 16., 10., DIM);
                    },
                );
            }
        },
    );
}
fn plane_action(d: &SharedDocument, args: serde_json::Value) {
    let mut doc = d.lock().unwrap();
    if let Err(e) = doc.paint_layer_command(&args, "Human") {
        doc.notify(e.to_string());
        doc.revision += 1;
    }
}
