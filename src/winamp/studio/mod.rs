#![allow(unused_braces)]
mod brush;
pub(crate) mod cursor_art;
mod draft;
mod equalizer;
mod guides;
mod mapping;
mod material;
mod mcp;
mod model;
pub(crate) use draft::new_mobile_document;
use draft::{load_draft, publish, save_draft, store_draft};
mod skin_studio;
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
const BG: Color = Color(0.065, 0.077, 0.10, 1.);
const CARD: Color = Color(0.105, 0.125, 0.16, 1.);
const PANEL_BACK: Color = Color(0.079, 0.094, 0.122, 1.);
const FG: Color = Color(0.89, 0.93, 0.98, 1.);
const DIM: Color = Color(0.55, 0.63, 0.73, 1.);
const ACCENT: Color = Color(0.13, 0.40, 0.53, 1.);
const DANGER: Color = Color(0.38, 0.14, 0.16, 1.);
const CARD_ON: Color = Color(0.10, 0.19, 0.24, 1.);
const ACCENT_LIT: Color = Color(0.36, 0.72, 0.88, 1.);
const PIP_OFF: Color = Color(0.22, 0.26, 0.32, 1.);
const ALERT: Color = Color(0.98, 0.62, 0.55, 1.);
const CANVAS_ORIGIN: (f32, f32) = (230., 128.);
const SCENE_MIN: (f32, f32) = (1160., 850.);
const SCENE_FLOOR: (f32, f32) = (320., 480.);
const SIDEBAR_MIN: f32 = 900.;
const DRAWER_WIDTH: f32 = 380.;
const DRAWER_MAX: f32 = 480.;
const CANVAS_MIN: f32 = 320.;
const CANVAS_BOTTOM_RESERVE: f32 = 102.;
const SCROLLBAR: f32 = 14.;
const PREVIEW_COLUMN: f32 = PREVIEW_BOX.0 + 12.;
const ACTION_ROW: f32 = 40.;
const ROW_PITCH: f32 = 38.;
const CANVAS_HEADER: f32 = 12.;
struct Rows {
    left: f32,
    right: f32,
    x: f32,
    y: f32,
}
impl Rows {
    fn new(left: f32, top: f32, right: f32) -> Self {
        Self {
            left,
            right,
            x: left,
            y: top,
        }
    }
    fn take(&mut self, width: f32) -> (f32, f32) {
        self.take_gap(width, 8.)
    }
    fn take_gap(&mut self, width: f32, gap: f32) -> (f32, f32) {
        if self.x > self.left && self.x + width > self.right {
            self.wrap();
        }
        let at = (self.x, self.y);
        self.x += width + gap;
        at
    }
    fn wrap(&mut self) {
        self.x = self.left;
        self.y += ROW_PITCH;
    }
    fn bottom(&self) -> f32 {
        self.y + ROW_PITCH
    }
}
#[derive(Clone, Copy, PartialEq)]
struct Scene {
    width: f32,
    height: f32,
    top: f32,
    footer: f32,
}
impl Scene {
    fn new(width: f32, height: f32) -> Self {
        Self {
            width: width.max(SCENE_FLOOR.0),
            height: height.max(SCENE_FLOOR.1),
            top: CANVAS_ORIGIN.1,
            footer: CANVAS_BOTTOM_RESERVE,
        }
    }
    fn under_chrome(self, top: f32) -> Self {
        Self { top, ..self }
    }
    fn over_footer(self, footer: f32) -> Self {
        Self { footer, ..self }
    }
    fn footer_top(&self) -> f32 {
        self.height - self.footer + 6.
    }
    fn sidebar(&self) -> bool {
        self.width >= SIDEBAR_MIN
    }
    fn left(&self) -> f32 {
        if self.sidebar() {
            CANVAS_ORIGIN.0
        } else {
            20.
        }
    }
    fn right(&self, inset_at_min_width: f32) -> f32 {
        self.width - (SCENE_MIN.0 - inset_at_min_width)
    }
    fn canvas(&self) -> (f32, f32, f32, f32) {
        let left = self.left();
        (
            left,
            self.top,
            self.width - left - 30.,
            self.height - self.top - self.footer,
        )
    }
    fn drawer(&self) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = self.painting();
        if w >= DRAWER_WIDTH {
            (x + w - DRAWER_WIDTH, y, DRAWER_WIDTH, h)
        } else {
            self.canvas()
        }
    }
    fn painting(&self) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = self.canvas();
        (x, y, w - PREVIEW_COLUMN, h)
    }
    fn split(&self) -> (f32, f32) {
        let area = self.painting().2;
        if area - DRAWER_WIDTH - 12. < CANVAS_MIN {
            return (area, 0.);
        }
        let drawer = (area * 0.36).clamp(DRAWER_WIDTH, DRAWER_MAX);
        (area - drawer - 12., drawer)
    }
    fn message_y(&self) -> f32 {
        self.height - 22.
    }
}
fn fit_zoom(canvas: (f32, f32), document: (u32, u32)) -> u8 {
    (1..=8u8)
        .rev()
        .find(|z| {
            document.0 as f32 * *z as f32 <= canvas.0 && document.1 as f32 * *z as f32 <= canvas.1
        })
        .unwrap_or(1)
}
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/fit_tests.rs"]
mod fit_tests;
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
        target_arch = "wasm32"
    ))]
    {
        let _ = revision;
        anyhow::bail!("Scene capture requires the WGPU renderer on a native window")
    }
}
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
pub fn launch(path: Option<&str>) -> std::io::Result<()> {
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command.arg("--skin-studio");
    if let Some(path) = path {
        command.arg(path);
    }
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
fn open_on_whole_skin(doc: &mut Document) {
    doc.open_on_whole_skin();
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
        doc.view.zoom = 2;
        open_on_whole_skin(&mut doc);
        return Ok(doc);
    };
    #[cfg(not(target_arch = "wasm32"))]
    let bytes = std::fs::read(path)?;
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
pub fn AdaptiveSkinStudio(shared: SharedDocument, host: StudioHost) {
    SkinStudio(shared, Some(host));
}
pub fn open_document(path: Option<&str>) -> anyhow::Result<SharedDocument> {
    let document = initial_document(path)?;
    Ok(SharedDocument(Arc::new(Mutex::new(document))))
}
fn default_export_path() -> std::path::PathBuf {
    let documents = cranpose::application_directories()
        .ok()
        .and_then(|directories| directories.documents);
    #[cfg(target_arch = "wasm32")]
    let base = documents.unwrap_or_default();
    #[cfg(not(target_arch = "wasm32"))]
    let base = documents.unwrap_or_else(std::env::temp_dir);
    base.join("Skin edited.wsz")
}
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
pub fn run(path: Option<&str>) {
    let doc = initial_document(path).unwrap_or_else(|e| panic!("Open skin: {e:#}"));
    let shared = SharedDocument(Arc::new(Mutex::new(doc)));
    match mcp::start(shared.clone()) {
        Ok(()) => println!(
            "Cranamp Skin Studio: MCP on http://{}, editing {}",
            mcp::ADDRESS,
            path.unwrap_or("the bundled Catamp")
        ),
        Err(e) => {
            eprintln!("Cranamp Skin Studio: no MCP -- {e:#}");
            shared.lock().unwrap().message = format!("MCP unavailable: {e:#}");
        }
    }
    let launcher = crate::create_surface_app()
        .with_title("Cranamp · Skin Studio")
        .with_size(1388, 1000);
    #[cfg(all(feature = "renderer-wgpu", not(target_os = "android")))]
    let launcher = launcher
        .with_frame_pacing_mode(cranpose::FramePacingMode::Vsync)
        .with_test_driver(|robot| {
            let _ = CAPTURE.set(Mutex::new(robot));
        });
    launcher.run(move || SkinStudio(shared.clone(), None));
}
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
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
            SkinStudio(document.clone(), None);
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
            .background(PANEL_BACK)
            .rounded_corners(5.),
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
fn Danger(label: String, x: f32, y: f32, w: f32, click: impl Fn() + 'static) {
    Slab(label, x, y, w, DANGER, click);
}
#[composable]
fn Slab(label: String, x: f32, y: f32, w: f32, fill: Color, click: impl Fn() + 'static) {
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(fill)
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
#[composable]
fn Panel(label: String, x: f32, y: f32, w: f32, open: bool, click: impl Fn() + 'static) {
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(if open { ACCENT } else { CARD })
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
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(if on { CARD_ON } else { CARD })
            .rounded_corners(5.),
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
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(if current { ACCENT } else { CARD })
            .rounded_corners(15.),
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
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(if current { ACCENT } else { CARD })
            .rounded_corners(15.),
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
    Button(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, 30.)
            .background(if on { CARD_ON } else { CARD })
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
fn CanvasScrollbar(
    vertical: bool,
    rect: (f32, f32, f32, f32),
    visible: f32,
    span: f32,
    value: i32,
    max: i32,
    set: impl Fn(i32) + 'static,
) {
    let set = Rc::new(set);
    let (x, y, w, h) = rect;
    let length = if vertical { h } else { w };
    let fraction = (visible / span.max(1.)).clamp(0.08, 1.);
    let thumb = (length * fraction).max(24.).min(length);
    let travel = (length - thumb).max(0.);
    let offset = if max > 0 {
        travel * (value.clamp(0, max) as f32 / max as f32)
    } else {
        0.
    };
    Box(
        Modifier::empty()
            .absolute_offset(x, y)
            .size_points(w, h)
            .background(CARD)
            .rounded_corners(SCROLLBAR / 2.)
            .pointer_input(
                (max, vertical, thumb.to_bits(), travel.to_bits()),
                move |scope: PointerInputScope| {
                    let set = set.clone();
                    async move {
                        scope
                            .await_pointer_event_scope(|events| async move {
                                loop {
                                    let event = events.await_pointer_event().await;
                                    let moving = event.kind == PointerEventKind::Down
                                        || (event.kind == PointerEventKind::Move
                                            && event.buttons.contains(PointerButton::Primary));
                                    if !moving || max <= 0 || travel <= 0. {
                                        continue;
                                    }
                                    let along = if vertical {
                                        event.position.y
                                    } else {
                                        event.position.x
                                    };
                                    let t = ((along - thumb / 2.) / travel).clamp(0., 1.);
                                    set((t * max as f32).round() as i32);
                                    event.consume();
                                }
                            })
                            .await;
                    }
                },
            ),
        BoxSpec::default(),
        move || {
            Box(
                Modifier::empty()
                    .absolute_offset(
                        if vertical { 0. } else { offset },
                        if vertical { offset } else { 0. },
                    )
                    .size_points(
                        if vertical { w } else { thumb },
                        if vertical { thumb } else { h },
                    )
                    .background(ACCENT_LIT)
                    .rounded_corners(SCROLLBAR / 2.),
                BoxSpec::default(),
                || {},
            );
        },
    );
}
fn note(shared: &SharedDocument, message: String) {
    let mut d = shared.lock().unwrap();
    d.message = message;
    d.revision += 1;
    ALERT_REVISION.store(d.revision, Ordering::Release);
}
#[composable]
fn CursorReadout(hover: cranpose_core::MutableState<Option<[i32; 2]>>, x: f32, y: f32) {
    Label(
        hover
            .get()
            .map(|p| format!("pointer  {} , {}", p[0], p[1]))
            .unwrap_or_default(),
        x,
        y,
        190.,
        11.,
        DIM,
    );
}
#[composable]
fn GuideHint(
    hover: cranpose_core::MutableState<Option<[i32; 2]>>,
    rects: Rc<Vec<(String, [u32; 4])>>,
    zoom: f32,
    ox: f32,
    oy: f32,
    label_at: (f32, f32),
) {
    let Some(point) = hover.get() else { return };
    if point[0] < 0 || point[1] < 0 {
        return;
    }
    let (at, id) = {
        let (x, y) = (point[0] as u32, point[1] as u32);
        let mut best: Option<(&str, [u32; 4])> = None;
        for (id, r) in rects.iter() {
            if x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3] {
                let area = r[2] as u64 * r[3] as u64;
                if best.is_none_or(|(_, b)| area < b[2] as u64 * b[3] as u64) {
                    best = Some((id, *r));
                }
            }
        }
        match best {
            Some((id, r)) => (r, id.to_string()),
            None => return,
        }
    };
    let x = ox + at[0] as f32 * zoom;
    let y = oy + at[1] as f32 * zoom;
    let (w, h) = (at[2] as f32 * zoom, at[3] as f32 * zoom);
    for (dx, dy, bw, bh) in [
        (0., 0., w, 1.),
        (0., h - 1., w, 1.),
        (0., 0., 1., h),
        (w - 1., 0., 1., h),
    ] {
        Box(
            Modifier::empty()
                .absolute_offset(x + dx, y + dy)
                .size_points(bw, bh)
                .background(Color(0.22, 0.89, 0.87, 0.85)),
            BoxSpec::default(),
            || {},
        );
    }
    Label(
        format!("{id}   {} × {}", at[2], at[3]),
        label_at.0,
        label_at.1,
        300.,
        11.,
        Color(0.22, 0.89, 0.87, 1.),
    );
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
#[composable]
fn SkinPreview(bitmap: ImageBitmap, size: (u32, u32), x: f32, y: f32, room: (f32, f32)) {
    let (w, h) = size;
    let shrink = preview_divisor(size, room);
    let (pw, ph) = ((w / shrink).max(1) as f32, (h / shrink).max(1) as f32);
    Box(
        Modifier::empty()
            .absolute_offset(x - 5., y - 17.)
            .size_points(pw + 10., ph + 22.)
            .background(PANEL_BACK)
            .rounded_corners(4.),
        BoxSpec::default(),
        move || {
            Label(format!("SKIN 1:{shrink}"), 5., 3., 100., 10., DIM);
        },
    );
    cranpose_ui::Image(
        cranpose_ui::BitmapRegionPainter(
            bitmap,
            Rect {
                x: 0.,
                y: 0.,
                width: w as f32,
                height: h as f32,
            },
            cranpose_ui::ImageSampling::Nearest,
        ),
        None,
        Modifier::empty()
            .absolute_offset(x, y)
            .required_size(cranpose_ui::Size::new(pw, ph)),
        Alignment::TOP_START,
        cranpose_ui::ContentScale::FillBounds,
        1.,
        None,
    );
}
fn preview_divisor(size: (u32, u32), room: (f32, f32)) -> u32 {
    (2..=8)
        .find(|d| size.0 / d <= room.0 as u32 && size.1 / d <= room.1 as u32)
        .unwrap_or(8)
}
const PREVIEW_BOX: (f32, f32) = (96., 132.);
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
        _ => "none",
    }
}
type Tick = cranpose_core::MutableState<u64>;
fn set_drawer(shared: &SharedDocument, tick: Tick, id: u8) {
    state(shared, json!({ "drawer": drawer_name(id) }));
    tick.set(shared.lock().unwrap().revision);
}
fn close_painting_drawer(shared: &SharedDocument, tick: Tick, open: u8) {
    if matches!(open, 4..=8) {
        set_drawer(shared, tick, 0);
    }
}
fn state(shared: &SharedDocument, patch: serde_json::Value) {
    let mut d = shared.lock().unwrap();
    if let Err(e) = d.state(patch) {
        d.message = format!("{e:#}");
        d.revision += 1;
        ALERT_REVISION.store(d.revision, Ordering::Release);
    }
}
pub use skin_studio::SkinStudio;
#[composable]
fn BrushChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    let word = cranpose_core::remember(|| TextFieldState::new("CATAMP")).with(|f| *f);
    let to_color = cranpose_core::remember(|| TextFieldState::new("#000000")).with(|f| *f);
    let shared = shared.clone();
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(0., 0.)
            .size_points(room.0 + 24., room.1)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            let shared = shared.clone();
            Box(
                Modifier::empty().size_points(room.0 + 24., 1040.),
                BoxSpec::default(),
                move || {
                    let v = shared.lock().unwrap().view.clone();
                    Label("DRAWING TOOLS".into(), 12., 17., 270., 14., FG);
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
                        ("text", "Text"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        Choice(
                            label.into(),
                            12. + (i % 3) as f32 * 118.,
                            82. + (i / 3) as f32 * 45.,
                            110.,
                            v.brush == tool,
                            move || state(&d, json!({"brush":tool})),
                        );
                    }
                    Label("STROKE WIDTH".into(), 12., 255., 172., 12., DIM);
                    for (i, size) in [1, 2, 3, 4, 8, 16].into_iter().enumerate() {
                        let d = shared.clone();
                        Choice(
                            format!("{size}px"),
                            12. + i as f32 * 59.,
                            277.,
                            52.,
                            v.brush_size == size,
                            move || state(&d, json!({"brush_size":size})),
                        );
                    }
                    if ["curve", "tuft"].contains(&v.brush.as_str()) {
                        for (x, delta, label) in [(12., -10, "− Bend"), (248., 10, "+ Bend")] {
                            let d = shared.clone();
                            let bend = v.curve_bend;
                            Action(label.into(), x, 316., 110., move || {
                                state(&d, json!({"curve_bend":(bend + delta).clamp(-100, 100)}))
                            });
                        }
                        Label(format!("{}%", v.curve_bend), 170., 322., 60., 12., FG);
                    } else if v.brush == "glass" {
                        Label("BEVEL".into(), 12., 316., 172., 11., DIM);
                        for (i, (amount, label)) in [
                            (0u32, "Drag"),
                            (4, "4px"),
                            (8, "8px"),
                            (16, "16px"),
                            (32, "32px"),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let d = shared.clone();
                            Choice(
                                label.into(),
                                12. + i as f32 * 71.,
                                334.,
                                64.,
                                v.bevel == amount,
                                move || state(&d, json!({ "bevel": amount })),
                            );
                        }
                        Label("REFRACTION".into(), 12., 372., 172., 11., DIM);
                        for (i, amount) in [0u32, 2, 4, 8, 16].into_iter().enumerate() {
                            let d = shared.clone();
                            Choice(
                                format!("{amount}px"),
                                12. + i as f32 * 71.,
                                390.,
                                64.,
                                v.refraction == amount,
                                move || state(&d, json!({ "refraction": amount })),
                            );
                        }
                    } else if v.brush == "text" {
                        Label("WORD".into(), 12., 316., 60., 11., DIM);
                        cranpose_ui::BasicTextField(
                            word,
                            Modifier::empty()
                                .absolute_offset(12., 332.)
                                .size_points(238., 26.)
                                .background(BG),
                            text_style(12., FG),
                        );
                        let d = shared.clone();
                        Action("Set".into(), 258., 331., 104., move || {
                            state(&d, json!({ "text": word.text() }))
                        });
                        Label("FACE".into(), 12., 366., 60., 11., DIM);
                        for (i, (face, label)) in [("5x7", "5×7"), ("small", "4×5 small caps")]
                            .into_iter()
                            .enumerate()
                        {
                            let d = shared.clone();
                            Choice(
                                label.into(),
                                12. + i as f32 * 122.,
                                384.,
                                114.,
                                v.face == face,
                                move || state(&d, json!({ "face": face })),
                            );
                        }
                        for (i, scale) in [1u32, 2, 3].into_iter().enumerate() {
                            let d = shared.clone();
                            Choice(
                                format!("{scale}×"),
                                258. + i as f32 * 36.,
                                384.,
                                32.,
                                v.text_scale == scale,
                                move || state(&d, json!({ "text_scale": scale })),
                            );
                        }
                        Label("LETTER SPACING".into(), 12., 410., 172., 11., DIM);
                        for (i, spacing) in [-1i32, 0, 1, 2].into_iter().enumerate() {
                            let d = shared.clone();
                            Choice(
                                format!("{spacing}"),
                                12. + i as f32 * 36.,
                                428.,
                                32.,
                                v.text_spacing == spacing,
                                move || state(&d, json!({ "text_spacing": spacing })),
                            );
                        }
                        Label(
                            format!("Click the canvas to place “{}”.", v.text),
                            166.,
                            431.,
                            199.,
                            10.,
                            DIM,
                        );
                    }
                    Label("GRADIENT".into(), 12., 455., 172., 12., DIM);
                    let ramping = v.ramp_to.is_some();
                    for (i, (axis, label, on)) in [
                        ("", "Off", !ramping),
                        ("down", "Down", ramping && v.ramp_axis == "down"),
                        ("across", "Across", ramping && v.ramp_axis == "across"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        Choice(
                            label.into(),
                            12. + i as f32 * 89.,
                            477.,
                            82.,
                            on,
                            move || {
                                if axis.is_empty() {
                                    state(&d, json!({"ramp_to":serde_json::Value::Null}))
                                } else {
                                    let to = to_color.text();
                                    state(&d, json!({"ramp_to":to,"ramp_axis":axis}))
                                }
                            },
                        );
                    }
                    Label("TO".into(), 279., 483., 22., 11., DIM);
                    cranpose_ui::BasicTextField(
                        to_color,
                        Modifier::empty()
                            .absolute_offset(300., 477.)
                            .size_points(72., 26.)
                            .background(BG),
                        text_style(12., FG),
                    );
                    Label(
                        if ramping {
                            format!(
                                "{} → {}, {}",
                                v.color,
                                v.ramp_to.clone().unwrap_or_default(),
                                v.ramp_axis
                            )
                        } else {
                            "A filled shape runs from the brush colour to this one.".into()
                        },
                        12.,
                        511.,
                        353.,
                        10.,
                        DIM,
                    );
                    Label("PAPER GRAIN".into(), 12., 538., 172., 12., DIM);
                    for (i, (amount, label)) in
                        [(0u32, "Smooth"), (5, "Fine"), (9, "Paper"), (16, "Coarse")]
                            .into_iter()
                            .enumerate()
                    {
                        let d = shared.clone();
                        Choice(
                            label.into(),
                            12. + i as f32 * 89.,
                            560.,
                            82.,
                            v.grain == amount,
                            move || state(&d, json!({ "grain": amount })),
                        );
                    }
                    Label("STRENGTH".into(), 12., 598., 172., 12., DIM);
                    for (i, (amount, label)) in
                        [(255u32, "Solid"), (190, "75%"), (128, "50%"), (64, "25%")]
                            .into_iter()
                            .enumerate()
                    {
                        let d = shared.clone();
                        Choice(
                            label.into(),
                            12. + i as f32 * 89.,
                            620.,
                            82.,
                            v.opacity == amount,
                            move || state(&d, json!({ "opacity": amount })),
                        );
                    }
                    Label(
                        "STAMP COPIES ALONG A DRAG".into(),
                        12.,
                        658.,
                        348.,
                        11.,
                        DIM,
                    );
                    for (i, n) in [1u32, 2, 3, 5, 9, 11].into_iter().enumerate() {
                        let d = shared.clone();
                        Choice(
                            format!("{n}×"),
                            12. + (i % 6) as f32 * 59.,
                            674.,
                            53.,
                            v.stamp_repeat == n,
                            move || state(&d, json!({ "stamp_repeat": n })),
                        );
                    }
                    {
                        let d = shared.clone();
                        let on = v.stamp_sweep;
                        Toggle(
                            if on {
                                "One copy per sprite state"
                            } else {
                                "Every copy in the edit scope"
                            }
                            .into(),
                            12.,
                            708.,
                            348.,
                            on,
                            move || state(&d, json!({ "stamp_sweep": !on })),
                        );
                    }
                    Label("EDIT SCOPE".into(), 12., 750., 348., 11., DIM);
                    for (i, scope) in model::SCOPES.iter().enumerate() {
                        let d = shared.clone();
                        let scope = *scope;
                        Choice(
                            model::scope_label(scope).into(),
                            12. + (i % 2) as f32 * 178.,
                            766. + (i / 2) as f32 * 34.,
                            170.,
                            v.states == scope,
                            move || state(&d, json!({ "states": scope })),
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
                        Toggle(
                            label.into(),
                            12. + (i % 2) as f32 * 178.,
                            842. + (i / 2) as f32 * 42.,
                            170.,
                            on,
                            move || state(&d, json!({field:!on})),
                        );
                    }
                    let d = shared.clone();
                    let mask_on = !v.mask_colors.is_empty();
                    let picked = v.color.clone();
                    Toggle(
                        if mask_on {
                            "Clear color mask".into()
                        } else {
                            "Mask picked color".into()
                        },
                        12.,
                        972.,
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
                        1006.,
                        353.,
                        10.,
                        DIM,
                    );
                },
            );
        },
    );
}
#[composable]
fn SkinOptionsChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    let shared = shared.clone();
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(0., 0.)
            .size_points(room.0 + 24., room.1)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            let shared = shared.clone();
            Box(
                Modifier::empty().size_points(room.0 + 24., 1050.),
                BoxSpec::default(),
                move || {
                    let report = shared.lock().unwrap().classic_report().unwrap_or_else(|e| json!({"exportable":false,"divergences":[{"entry":"Skin","problem":e.to_string(),"fix":"Review source sheets"}]}));
                    Label("CLASSIC WINAMP SKIN".into(), 12., 17., 300., 14., FG);
                    Label("Opaque BMPs and explicit palettes.\nEditor, player and export use the same pixels.".into(), 12., 41., 355., 11., DIM);
                    Label("EXPORT FORMAT CHECK".into(), 12., 86., 260., 11., DIM);
                    if report["exportable"] == true {
                        Label("Ready to export. Review the art and control states\nin the GPU and the target players before publishing.".into(), 12., 104., 355., 11., FG);
                    } else if let Some(errors) = report["divergences"].as_array() {
                        for (i, error) in errors.iter().take(4).enumerate() {
                            Label(
                                format!(
                                    "{} — {}\n{}",
                                    error["entry"].as_str().unwrap_or(""),
                                    error["problem"].as_str().unwrap_or(""),
                                    error["fix"].as_str().unwrap_or("")
                                ),
                                12.,
                                104. + i as f32 * 34.,
                                355.,
                                11.,
                                FG,
                            );
                        }
                    }
                    Label("OPAQUE SPRITES\nMagenta is a color, not transparency.\nMoving handles carry their complete rectangle.\nInspect every state and travel endpoint.".into(), 12., 260., 355., 11., DIM);
                    let font_doc = shared.clone();
                    Action(
                        "Build classic text font".into(),
                        12.,
                        340.,
                        200.,
                        move || {
                            let mut doc = font_doc.lock().unwrap();
                            let (colors, _) = doc.text_palettes();
                            let ink = colors
                                .iter()
                                .find(|(k, _)| *k == "Normal")
                                .unwrap()
                                .1
                                .clone();
                            let background = colors
                                .iter()
                                .find(|(k, _)| *k == "NormalBG")
                                .unwrap()
                                .1
                                .clone();
                            if let Err(e) = doc.classic_font(&ink, &background) {
                                doc.message = e.to_string();
                                doc.revision += 1;
                            }
                        },
                    );
                    Label(
                        "Replaces text.bmp on the active paint layer. Undo restores it.".into(),
                        12.,
                        375.,
                        355.,
                        10.,
                        DIM,
                    );
                    let (playlist_colours, visualizer_colours) =
                        { shared.lock().unwrap().text_palettes() };
                    let brush = { shared.lock().unwrap().view.color.clone() };
                    Label(
                        "PLAYLIST TEXT  ·  PLEDIT.TXT".into(),
                        12.,
                        408.,
                        300.,
                        11.,
                        DIM,
                    );
                    Label(
                        format!("Click a slot to set it to the brush colour, {brush}."),
                        12.,
                        426.,
                        340.,
                        10.,
                        DIM,
                    );
                    for (i, (key, value)) in playlist_colours.iter().enumerate() {
                        let rgba = parse_color(value).unwrap_or([0, 0, 0, 255]);
                        let d = shared.clone();
                        let key = *key;
                        let brush = brush.clone();
                        Box(
                            Modifier::empty()
                                .absolute_offset(
                                    12. + (i % 3) as f32 * 118.,
                                    448. + (i / 3) as f32 * 44.,
                                )
                                .size_points(110., 22.)
                                .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                                .rounded_corners(3.)
                                .clickable(move |_| {
                                    let mut doc = d.lock().unwrap();
                                    if let Err(e) = doc.set_palette(&json!({ key: brush.clone() }))
                                    {
                                        doc.message = e.to_string();
                                        doc.revision += 1;
                                    }
                                }),
                            BoxSpec::default(),
                            || {},
                        );
                        Label(
                            key.to_string(),
                            12. + (i % 3) as f32 * 118.,
                            472. + (i / 3) as f32 * 44.,
                            110.,
                            10.,
                            DIM,
                        );
                    }
                    Label(
                        "VISUALIZER  ·  VISCOLOR.TXT".into(),
                        12.,
                        542.,
                        300.,
                        11.,
                        DIM,
                    );
                    Label(
                        "0 opaque background, 1 dots, 2-17 analyzer.\n18-22 oscilloscope, 23 peak. Click a slot to use the brush color."
                            .into(),
                        12.,
                        560.,
                        340.,
                        10.,
                        DIM,
                    );
                    for (i, value) in visualizer_colours.iter().enumerate() {
                        let rgba = parse_color(value).unwrap_or([0, 0, 0, 255]);
                        let d = shared.clone();
                        let brush = brush.clone();
                        let mut next = visualizer_colours.clone();
                        let slot = format!("{} · {brush}", mcp::VISUALIZER_SLOTS[i]);
                        next[i] = brush;
                        Box(
                            Modifier::empty()
                                .absolute_offset(
                                    12. + (i % 8) as f32 * 43.,
                                    592. + (i / 8) as f32 * 30.,
                                )
                                .size_points(38., 24.)
                                .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                                .rounded_corners(3.)
                                .clickable(move |_| {
                                    let mut doc = d.lock().unwrap();
                                    match doc.visualizer_palette(&json!({ "colors": next.clone() }))
                                    {
                                        Ok(_) => doc.message = slot.clone(),
                                        Err(e) => {
                                            doc.message = e.to_string();
                                            doc.revision += 1;
                                        }
                                    }
                                }),
                            BoxSpec::default(),
                            || {},
                        );
                    }
                    Label("AUTOMATION".into(), 12., 694., 200., 11., DIM);
                    Label(
                        "WINDOW CUTOUTS · REGION.TXT".into(),
                        12.,
                        800.,
                        350.,
                        11.,
                        DIM,
                    );
                    Label("Choose the exterior color with the brush. Cutouts reveal\nthe window beneath ALL artwork and controls. Undo restores them.".into(), 12., 820., 350., 10., DIM);
                    for (i, section) in ["Normal", "Equalizer"].into_iter().enumerate() {
                        let d = shared.clone();
                        Action(
                            format!("Cut {section} exterior"),
                            12.,
                            860. + i as f32 * 42.,
                            205.,
                            move || {
                                let mut doc = d.lock().unwrap();
                                let color = doc.view.color.clone();
                                match doc.window_regions(&json!({"action":"generate","section":section,"transparent_color":color,"exterior_only":true})) {
                                Ok(_) => doc.message = format!("Generated {section} window mask; review GPU cutouts"),
                                Err(e) => {
                                    doc.message = e.to_string();
                                    doc.revision += 1;
                                }
                            }
                            },
                        );
                        let d = shared.clone();
                        Action(
                            "Reset".into(),
                            230.,
                            860. + i as f32 * 42.,
                            90.,
                            move || {
                                let mut doc = d.lock().unwrap();
                                if let Err(e) = doc
                                    .window_regions(&json!({"action":"remove","section":section}))
                                {
                                    doc.message = e.to_string();
                                    doc.revision += 1;
                                }
                            },
                        );
                    }
                    Label(
                        "This editor is also an MCP server, so an agent can paint\ninto the same document and share its undo history.\n\n  127.0.0.1:18765\n  cranamp --skin-studio-mcp"
                            .into(),
                        12.,
                        712.,
                        350.,
                        11.,
                        DIM,
                    );
                },
            );
        },
    );
}
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
            .background(BG),
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
fn HistoryChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
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
            .size_points(room.0 + 4., (room.1 - 122.).max(60.))
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
                    Modifier::empty().size_points(room.0, 54.),
                    BoxSpec::default(),
                    move || {
                        let d = d.clone();
                        ListChoice(caption.clone(), 2., 0., room.0 - 4., current, move || {
                            let _ = d.lock().unwrap().history_goto(cursor);
                        });
                        Label(source.clone(), 10., 33., room.0 - 20., 10., DIM);
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
fn ColorChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    Label("COLOUR PICKER".into(), 12., 17., 270., 14., FG);
    let current = { shared.lock().unwrap().view.color.clone() };
    let rgb = parse_color(&current).unwrap_or([255, 255, 255, 255]);
    let (h0, s0, v0) = rgb_hsv([rgb[0], rgb[1], rgb[2]]);
    let hue = cranpose_core::rememberMutableStateOf(|| h0);
    if s0 > 0.02 && v0 > 0.02 {
        let h = h0;
        if (hue.get_non_reactive() - h).abs() > 0.001 {
            cranpose_core::SideEffect(move || hue.set(h));
        }
    }
    let h = hue.get();
    Box(
        Modifier::empty()
            .absolute_offset(12., 40.)
            .size_points(58., 28.)
            .background(Color::from_rgba_u8(rgb[0], rgb[1], rgb[2], 255))
            .rounded_corners(4.),
        BoxSpec::default(),
        || {},
    );
    let hex = cranpose_core::remember(|| TextFieldState::new("")).with(|f| *f);
    let showing = current.clone();
    cranpose_core::LaunchedEffect(showing.clone(), move |_| {
        hex.set_text(showing.clone());
    });
    cranpose_ui::BasicTextField(
        hex,
        Modifier::empty()
            .absolute_offset(78., 41.)
            .size_points(110., 26.)
            .background(BG),
        text_style(12., FG),
    );
    {
        let d = shared.clone();
        Action("Set".into(), 196., 39., 62., move || {
            state(&d, json!({ "color": hex.text() }))
        });
    }
    Label(
        "drag the field,\nthen the hue strip".into(),
        268.,
        42.,
        room.0 - 258.,
        10.,
        DIM,
    );
    let field_size: (f32, f32) = (room.0, 196.);
    const STRIP_H: f32 = 22.;
    let field = {
        let (fw, fh) = (68u32, 40u32);
        let mut im = image::RgbaImage::new(fw, fh);
        for y in 0..fh {
            for x in 0..fw {
                let c = hsv_rgb(
                    h,
                    x as f32 / (fw - 1) as f32,
                    1. - y as f32 / (fh - 1) as f32,
                );
                im.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], 255]));
            }
        }
        ImageBitmap::from_rgba8(fw, fh, im.into_raw()).expect("sv field")
    };
    let strip = {
        let sw = 120u32;
        let mut im = image::RgbaImage::new(sw, 1);
        for x in 0..sw {
            let c = hsv_rgb(x as f32 / (sw - 1) as f32, 1., 1.);
            im.put_pixel(x, 0, image::Rgba([c[0], c[1], c[2], 255]));
        }
        ImageBitmap::from_rgba8(sw, 1, im.into_raw()).expect("hue strip")
    };
    let d = shared.clone();
    Box(
        Modifier::empty()
            .absolute_offset(12., 82.)
            .size_points(field_size.0, field_size.1)
            .pointer_input(h.to_bits(), move |scope: PointerInputScope| {
                let d = d.clone();
                async move {
                    scope
                        .await_pointer_event_scope(|events| async move {
                            loop {
                                let event = events.await_pointer_event().await;
                                let holding = event.kind == PointerEventKind::Down
                                    || (event.kind == PointerEventKind::Move
                                    && event.buttons.contains(PointerButton::Primary));
                                if !holding {
                                    continue;
                                }
                                let s = (event.position.x / field_size.0).clamp(0., 1.);
                                let v = 1. - (event.position.y / field_size.1).clamp(0., 1.);
                                let c = hsv_rgb(h, s, v);
                                state(
                                    &d,
                                    json!({"color":format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])}),
                                );
                                event.consume();
                            }
                        })
                        .await;
                }
            }),
        BoxSpec::default(),
        move || {
            cranpose_ui::Image(
                cranpose_ui::BitmapPainter(field.clone()),
                None,
                Modifier::empty()
                    .required_size(cranpose_ui::Size::new(field_size.0, field_size.1)),
                Alignment::TOP_START,
                cranpose_ui::ContentScale::FillBounds,
                1.,
                None,
            );
            Box(
                Modifier::empty()
                    .absolute_offset((s0 * field_size.0 - 4.).clamp(0., field_size.0 - 8.), ((1. - v0) * field_size.1 - 4.).clamp(0., field_size.1 - 8.))
                    .size_points(8., 8.)
                    .background(if v0 > 0.55 { BG } else { FG })
                    .rounded_corners(4.),
                BoxSpec::default(),
                || {},
            );
        },
    );
    let d = shared.clone();
    Box(
        Modifier::empty()
            .absolute_offset(12., 82. + field_size.1 + 10.)
            .size_points(field_size.0, STRIP_H)
            .pointer_input((), move |scope: PointerInputScope| {
                let d = d.clone();
                async move {
                    scope
                        .await_pointer_event_scope(|events| async move {
                            loop {
                                let event = events.await_pointer_event().await;
                                let holding = event.kind == PointerEventKind::Down
                                    || (event.kind == PointerEventKind::Move
                                    && event.buttons.contains(PointerButton::Primary));
                                if !holding {
                                    continue;
                                }
                                let next = (event.position.x / field_size.0).clamp(0., 1.);
                                hue.set(next);
                                let (_, s, v) = {
                                    let doc = d.lock().unwrap();
                                    let c = parse_color(&doc.view.color).unwrap_or([255; 4]);
                                    rgb_hsv([c[0], c[1], c[2]])
                                };
                                let c = hsv_rgb(next, if s < 0.02 { 1. } else { s }, if v < 0.02 { 1. } else { v });
                                state(
                                    &d,
                                    json!({"color":format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])}),
                                );
                                event.consume();
                            }
                        })
                        .await;
                }
            }),
        BoxSpec::default(),
        move || {
            cranpose_ui::Image(
                cranpose_ui::BitmapPainter(strip.clone()),
                None,
                Modifier::empty().required_size(cranpose_ui::Size::new(field_size.0, STRIP_H)),
                Alignment::TOP_START,
                cranpose_ui::ContentScale::FillBounds,
                1.,
                None,
            );
            Box(
                Modifier::empty()
                    .absolute_offset((h * field_size.0 - 2.).clamp(0., field_size.0 - 4.), 0.)
                    .size_points(4., STRIP_H)
                    .background(FG),
                BoxSpec::default(),
                || {},
            );
        },
    );
    Label(
        "IN THIS SKIN".into(),
        12.,
        82. + field_size.1 + 46.,
        200.,
        11.,
        DIM,
    );
    let sampled = shared.clone();
    let palette = cranpose_core::remember(move || sampled.lock().unwrap().palette_sample(24))
        .with(|p| p.clone());
    for (i, colour) in palette.iter().enumerate() {
        let d = shared.clone();
        let c = colour.clone();
        let pick = c.clone();
        Box(
            Modifier::empty()
                .absolute_offset(
                    12. + (i % 8) as f32 * 43.,
                    82. + field_size.1 + 66. + (i / 8) as f32 * 32.,
                )
                .size_points(38., 26.)
                .background({
                    let v = parse_color(&c).unwrap_or([0, 0, 0, 255]);
                    Color::from_rgba_u8(v[0], v[1], v[2], 255)
                })
                .rounded_corners(3.)
                .clickable(move |_| state(&d, json!({"color":pick.clone()}))),
            BoxSpec::default(),
            || {},
        );
    }
    let d = shared.clone();
    Choice(
        "Magenta ink (#ff00ff)".into(),
        12.,
        82. + field_size.1 + 172.,
        200.,
        current.eq_ignore_ascii_case("#ff00ff"),
        move || state(&d, json!({"color":"#ff00ff"})),
    );
    let d = shared.clone();
    Choice(
        "Erase this paint layer".into(),
        12.,
        82. + field_size.1 + 200.,
        200.,
        current == "transparent",
        move || state(&d, json!({"color":"transparent"})),
    );
}
#[composable]
fn StudyChooser(
    shared: SharedDocument,
    _revision: u64,
    room: (f32, f32),
    hover: cranpose_core::MutableState<Option<[i32; 2]>>,
) {
    Label("PIXEL STUDY".into(), 12., 17., 270., 14., FG);
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
            format!(
                "Live {},{} · {}×{} · surrounding pixels included when on",
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
            "Read-only magnifier. Move the pointer over the canvas\nand it follows; lift a region to pin it here instead."
                .into()
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
        let detail_h = (room.1 - 404.).max(100.);
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
    let transforms = room.1 - 96.;
    if flat_count > 0 {
        Label(
            format!("{flat_count} flat drawable areas — inspect before leaving blank"),
            12.,
            transforms - 62.,
            room.0,
            10.,
            FG,
        );
    }
    if let Some(coverage) = coverage {
        let c = &coverage["counts"];
        Label(
            format!(
                "{} paintable · {} runtime-hidden · {} palette-only",
                c["paintable"], c["opaque_runtime"], c["palette_only"]
            ),
            12.,
            transforms - 44.,
            room.0,
            10.,
            DIM,
        );
    }
    Label(
        "CLIPBOARD  ·  NOT THE STUDY".into(),
        12.,
        transforms - 20.,
        300.,
        11.,
        DIM,
    );
    let third = (room.0 - 16.) / 3.;
    let has_clipboard = { shared.lock().unwrap().cluster.is_some() };
    for (i, label) in ["Flip H", "Flip V", "Turn 90°"].into_iter().enumerate() {
        let d = shared.clone();
        MaybeAction(
            label.into(),
            12. + i as f32 * (third + 8.),
            transforms,
            third,
            has_clipboard,
            move || {
                let mut doc = d.lock().unwrap();
                if let Err(e) = doc.transform_cluster(i == 0, i == 1, u32::from(i == 2)) {
                    doc.message = e.to_string();
                    doc.revision += 1;
                }
            },
        );
    }
    Label(
        if has_clipboard {
            "Transforms are exact; paint the result with the Stamp brush."
        } else {
            "Lift a region with the Lift pixels brush to fill the clipboard."
        }
        .into(),
        12.,
        transforms + 38.,
        room.0,
        10.,
        DIM,
    );
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
        doc.message = e.to_string();
        doc.revision += 1;
    }
}
#[composable]
fn PaintChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    let (width, height) = room;
    let info = shared.lock().unwrap().paint_layer_info();
    let planes = info["layers"].as_array().unwrap().clone();
    let active = info["active"].as_str().map(str::to_owned);
    let armed = cranpose_core::rememberMutableStateOf(|| false);
    Label("PAINTING LAYERS".into(), 12., 17., 270., 14., FG);
    let third = (width - 16.) / 3.;
    {
        let d = shared.clone();
        Action("+ New layer".into(), 12., 50., third, move || {
            plane_action(&d, json!({"action":"add"}))
        });
    }
    {
        let d = shared.clone();
        Choice(
            "Base atlases".into(),
            12. + third + 8.,
            50.,
            third,
            active.is_none(),
            move || plane_action(&d, json!({"action":"select","id":"base"})),
        );
    }
    {
        let d = shared.clone();
        Action(
            "Save project".into(),
            12. + (third + 8.) * 2.,
            50.,
            third,
            move || {
                let mut doc = d.lock().unwrap();
                let p = std::path::PathBuf::from(doc.path.as_deref().unwrap_or("Untitled.wsz"))
                    .with_extension("cstudio");
                if let Err(e) = doc.save_project(&p) {
                    doc.message = e.to_string();
                    doc.revision += 1;
                }
            },
        );
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
            .absolute_offset(12., 94.)
            .size_points(width - 104., 26.)
            .background(BG),
        text_style(12., FG),
    );
    {
        let d = shared.clone();
        Action("Rename".into(), width - 84., 92., 96., move || {
            plane_action(&d, json!({"action":"set","name":field.text()}))
        });
    }
    let controls = height - 168.;
    let layer_doc = shared.clone();
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(12., 132.)
            .size_points(width, (controls - 144.).max(60.))
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
                let name = if name.chars().count() > 22 {
                    format!("{}…", name.chars().take(21).collect::<String>())
                } else {
                    name
                };
                Box(
                    Modifier::empty().size_points(width, 38.),
                    BoxSpec::default(),
                    move || {
                        {
                            let d = d.clone();
                            let id = id.clone();
                            ListChoice(name.clone(), 0., 0., width - 192., on, move || {
                                plane_action(&d, json!({"action":"select","id":id}))
                            });
                        }
                        {
                            let d = d.clone();
                            let id = id.clone();
                            Toggle(
                                if visible { "Shown" } else { "Hidden" }.into(),
                                width - 186.,
                                0.,
                                90.,
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
                            Toggle(
                                if locked { "Locked" } else { "Free" }.into(),
                                width - 90.,
                                0.,
                                88.,
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
    let Some(id) = info["active"].as_str().map(str::to_owned) else {
        Label(
            "Painting planes sit over the original atlases, and a skin exports\nas the picture they make together. Choose one to rename,\nreorder or fade it."
                .into(),
            12.,
            controls + 8.,
            width,
            11.,
            DIM,
        );
        return;
    };
    let index = info["layers"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["id"] == id)
        .unwrap();
    let opacity = info["layers"][index]["opacity"].as_u64().unwrap();
    let quarter = (width - 24.) / 4.;
    for (i, label) in ["Down", "Up", "Merge", "Delete"].into_iter().enumerate() {
        let d = shared.clone();
        let id = id.clone();
        let n = info["layers"].as_array().unwrap().len();
        let x = 12. + i as f32 * (quarter + 8.);
        let act = move || {
            plane_action(
                &d,
                match i {
                    0 => json!({"action":"move","id":id,"index":index.saturating_sub(1)}),
                    1 => json!({"action":"move","id":id,"index":(index+1).min(n-1)}),
                    2 => json!({"action":"merge_down","id":id}),
                    _ => json!({"action":"delete","id":id}),
                },
            )
        };
        if i == 3 {
            let armed_now = armed.get();
            Danger(
                if armed_now { "Discard it" } else { "Delete" }.into(),
                x,
                controls,
                quarter,
                move || {
                    if armed_now {
                        armed.set(false);
                        act();
                    } else {
                        armed.set(true);
                    }
                },
            );
        } else {
            Action(label.into(), x, controls, quarter, act);
        }
    }
    if armed.get() {
        Label(
            "Delete discards this plane's artwork. Press it again, or:".into(),
            12.,
            controls + 44.,
            width,
            11.,
            ALERT,
        );
        Action(
            "Keep the layer".into(),
            12.,
            controls + 62.,
            140.,
            move || armed.set(false),
        );
        return;
    }
    Label(
        format!("OPACITY  ·  {}%", opacity * 100 / 255),
        12.,
        controls + 44.,
        160.,
        11.,
        DIM,
    );
    let d = shared.clone();
    let target = id.clone();
    let end_doc = shared.clone();
    let track = width - 24.;
    cranpose_ui::Slider(
        Modifier::empty()
            .absolute_offset(12., controls + 62.)
            .size_points(track, 22.),
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
        cranpose_ui::SliderSpec::new().thumb_extent(14.),
        move |scope| {
            Box(
                Modifier::empty()
                    .absolute_offset(0., 8.)
                    .size_points(track, 6.)
                    .background(CARD)
                    .rounded_corners(3.),
                BoxSpec::default(),
                || {},
            );
            Box(
                Modifier::empty()
                    .absolute_offset(scope.thumb_offset(), 2.)
                    .size_points(14., 18.)
                    .background(ACCENT_LIT)
                    .rounded_corners(3.),
                BoxSpec::default(),
                || {},
            );
        },
    );
    let d = shared.clone();
    let clipped = info["layers"][index]["clip_below"] == true;
    let target = id.clone();
    Toggle(
        if clipped {
            "Clipped to the layer below"
        } else {
            "Independent of the layer below"
        }
        .into(),
        12.,
        controls + 98.,
        width,
        clipped,
        move || {
            plane_action(
                &d,
                json!({"action":"set","id":target,"clip_below":!clipped}),
            );
        },
    );
}
