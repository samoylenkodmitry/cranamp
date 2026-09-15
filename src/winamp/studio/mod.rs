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
use mobile::publish;
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
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

/// How the editor hands work back to a player it is living inside.
///
/// `None` on the desktop, where the editor is its own window and the player is
/// another process entirely. `Some` on Android and the web, where the editor
/// borrows the player's single surface and has to be able to give it back.
#[derive(Clone)]
pub struct StudioHost {
    pub close: Rc<dyn Fn()>,
    pub apply: Rc<dyn Fn(Vec<u8>, String)>,
}

/// Two hosts are the same host when they carry the same callbacks, which is
/// what the composer needs in order to decide the editor has not changed.
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
    /// The colour a viewer actually sees at a canvas pixel, so the framework
    /// tests can assert on the picture rather than on the atlases behind it.
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
/// The sidebar and the tool column. A shade under `CARD` so that a button
/// resting on one is visibly a button: both were `CARD`, which left every
/// unlit control on them reading as a line of text.
const PANEL_BACK: Color = Color(0.079, 0.094, 0.122, 1.);
const FG: Color = Color(0.89, 0.93, 0.98, 1.);
const DIM: Color = Color(0.55, 0.63, 0.73, 1.);
const ACCENT: Color = Color(0.13, 0.40, 0.53, 1.);
/// Reserved for the two buttons that can discard unexported work.
const DANGER: Color = Color(0.38, 0.14, 0.16, 1.);
/// A switch that is on: lit enough to read across the window, but never
/// mistakable for the solid `ACCENT` of an open drawer.
const CARD_ON: Color = Color(0.10, 0.19, 0.24, 1.);
const ACCENT_LIT: Color = Color(0.36, 0.72, 0.88, 1.);
const PIP_OFF: Color = Color(0.22, 0.26, 0.32, 1.);
/// The status line when it is carrying a refusal or a failure.
const ALERT: Color = Color(0.98, 0.62, 0.55, 1.);

// The desktop editor is an absolutely positioned scene: a fixed sidebar and
// toolbars, and a canvas that takes everything left over. `Scene` is the one
// place that knows the window size, so the drawing hit test and the drawer can
// agree on exactly which strip of canvas is covered at any window size.
/// The canvas starts under two toolbar rows and the canvas' own header line.
/// The skin is a 275x377 portrait document, so every point of chrome above it
/// is a point of zoom it cannot reach: at 2x it needs 754, which only fits in
/// a window this shallow above and below the canvas.
const CANVAS_ORIGIN: (f32, f32) = (230., 128.);
/// The size the scene was drawn for. A smaller window keeps this layout and
/// clips, rather than collapsing every absolute coordinate on top of itself.
const SCENE_MIN: (f32, f32) = (1160., 850.);
/// The docked tool column. Wider than the drawer was when it covered the
/// canvas, because it no longer has to be grudging about the room it takes.
const DRAWER_WIDTH: f32 = 380.;
const DRAWER_MAX: f32 = 480.;
/// Below this the canvas is not worth splitting, and the drawer goes back to
/// covering it.
const CANVAS_MIN: f32 = 320.;
/// Room under the canvas for the sprite-state row, the frame slider and the
/// status line.
const CANVAS_BOTTOM_RESERVE: f32 = 102.;
/// Width of the scrollbars along the canvas' right and bottom edges.
const SCROLLBAR: f32 = 14.;
/// The two toolbar rows: what the document does, then what the drawers show.
const ACTION_ROW: f32 = 40.;
const PANEL_ROW: f32 = 78.;

#[derive(Clone, Copy, PartialEq)]
struct Scene {
    width: f32,
    height: f32,
}

impl Scene {
    fn new(width: f32, height: f32) -> Self {
        Self {
            width: width.max(SCENE_MIN.0),
            height: height.max(SCENE_MIN.1),
        }
    }
    /// Distance in from the right edge, for controls that track the window.
    fn right(&self, inset_at_min_width: f32) -> f32 {
        self.width - (SCENE_MIN.0 - inset_at_min_width)
    }
    fn canvas(&self) -> (f32, f32, f32, f32) {
        (
            CANVAS_ORIGIN.0,
            CANVAS_ORIGIN.1,
            self.width - CANVAS_ORIGIN.0 - 30.,
            self.height - CANVAS_ORIGIN.1 - CANVAS_BOTTOM_RESERVE,
        )
    }
    fn drawer(&self) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = self.canvas();
        (x + w - DRAWER_WIDTH, y, DRAWER_WIDTH, h)
    }
    /// How the canvas panel and the docked tool column divide the room left
    /// over beside the sidebar. A 275-wide document in a 1200-wide panel is
    /// mostly empty panel; the room is worth more as tools that are always on
    /// screen than as margin.
    fn split(&self) -> (f32, f32) {
        let area = self.canvas().2;
        if area - DRAWER_WIDTH - 12. < CANVAS_MIN {
            return (area, 0.);
        }
        let drawer = (area * 0.36).clamp(DRAWER_WIDTH, DRAWER_MAX);
        (area - drawer - 12., drawer)
    }
    fn state_y(&self) -> f32 {
        self.height - 96.
    }
    fn slider_y(&self) -> f32 {
        self.height - 58.
    }
    fn message_y(&self) -> f32 {
        self.height - 28.
    }
}

/// The largest whole-pixel zoom at which the whole skin fits the canvas. Never
/// fractional: a classic skin is native pixels, and half a pixel is a lie.
fn fit_zoom(canvas: (f32, f32), document: (u32, u32)) -> u8 {
    (1..=8u8)
        .rev()
        .find(|z| {
            document.0 as f32 * *z as f32 <= canvas.0 && document.1 as f32 * *z as f32 <= canvas.1
        })
        .unwrap_or(1)
}

#[cfg(test)]
mod fit_tests {
    use super::{fit_zoom, Scene, CANVAS_MIN, DRAWER_WIDTH};

    /// The window the editor opens at has to reach 2x on the joined skin, or
    /// the default zoom is a compromise nobody asked for.
    #[test]
    fn the_default_window_fits_the_whole_skin_at_two_times() {
        let scene = Scene::new(1280., 1000.);
        let (canvas_w, drawer_w) = scene.split();
        let canvas_h = scene.canvas().3;
        assert!(drawer_w >= DRAWER_WIDTH, "the tool column stays docked");
        assert_eq!(fit_zoom((canvas_w, canvas_h), (275, 377)), 2);
    }

    /// A window too small to hold both goes back to one full-width canvas
    /// rather than a canvas too narrow to draw in.
    #[test]
    fn a_narrow_window_keeps_the_canvas_whole() {
        let scene = Scene::new(0., 0.);
        let (canvas_w, drawer_w) = scene.split();
        assert!(canvas_w >= CANVAS_MIN);
        assert!(drawer_w == 0. || canvas_w >= CANVAS_MIN);
    }
}

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

#[cfg(all(
    feature = "renderer-wgpu",
    not(target_os = "android"),
    not(target_arch = "wasm32")
))]
static CAPTURE: std::sync::OnceLock<Mutex<cranpose::Robot>> = std::sync::OnceLock::new();

static COMPOSED_REVISION: AtomicU64 = AtomicU64::new(0);
/// The document revision whose status line is a failure rather than a report.
/// Errors and stroke reports share one line, so the line has to be able to
/// look different when it is carrying bad news.
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

/// Open the editor in its own native window without interrupting playback.
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
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

/// The editor always works on the whole skin: main, equalizer and playlist
/// joined into one surface, so a stroke can cross a window boundary and each
/// pixel is routed to whichever sheet is under it. The individual panels remain
/// as MCP addressing, not as editing modes.
fn open_on_whole_skin(doc: &mut Document) {
    doc.view.panel = "canvas".into();
    doc.view.layer = "auto".into();
    doc.view.layers.clear();
    doc.view.zoom = doc.view.zoom.clamp(1, 4);
    // Paint onto the top of the picture, not underneath it. A layered project
    // composites its painting planes over the base atlases, so a document that
    // arrives with planes and no selection would take every stroke into the
    // atlases below them -- landing correctly, recorded in history, and visible
    // only in the gaps where no plane covers the artwork.
    if doc.view.paint_layer.is_none() {
        doc.view.paint_layer = doc
            .planes
            .iter()
            .rev()
            .find(|plane| plane.visible && !plane.locked)
            .map(|plane| plane.id.clone());
    }
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

/// The desktop layout is an absolutely positioned scene built for the editor
/// window. Below this it does not fit, and the touch layout is the honest
/// answer; at or above it the full editor is simply better, so a browser on a
/// laptop gets the same tools a desktop window does.
pub const DESKTOP_LAYOUT_MIN: (f32, f32) = (1140., 820.);

/// The editor sized to the surface it was given.
///
/// The two layouts are the same document, brushes and history -- only the
/// chrome differs, so this can switch on a window resize without losing work.
#[composable]
pub fn AdaptiveSkinStudio(shared: SharedDocument, host: StudioHost) {
    BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
        let constraints = scope.constraints();
        let roomy = constraints.max_width >= DESKTOP_LAYOUT_MIN.0
            && constraints.max_height >= DESKTOP_LAYOUT_MIN.1;
        if roomy {
            SkinStudio(shared.clone(), Some(host.clone()));
        } else {
            let close = host.close.clone();
            let apply = host.apply.clone();
            MobileSkinStudio(
                shared.clone(),
                move || close(),
                move |bytes, path| apply(bytes, path),
            );
        }
    });
}

/// A shared editor document for `path`, or the bundled editable copy. Lets the
/// framework-level tests mount the real `SkinStudio` against a real document.
pub fn open_document(path: Option<&str>) -> anyhow::Result<SharedDocument> {
    let document = initial_document(path)?;
    Ok(SharedDocument(Arc::new(Mutex::new(document))))
}

/// Where an untitled export lands. The browser has no filesystem, and
/// `std::env::temp_dir()` panics there rather than failing, so a web build that
/// opened the Studio without a file already in hand took the whole player down
/// with it. On the web the name stands alone and the browser places it.
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
    if let Err(e) = mcp::start(shared.clone()) {
        shared.lock().unwrap().message = format!("MCP unavailable: {e:#}");
    }
    let launcher = crate::create_surface_app()
        .with_title("Cranamp · Skin Studio")
        // The joined skin is a 275x377 portrait document, so the window is
        // shaped for height rather than width: this is the smallest window in
        // which the whole skin fits at 2x, with the tool column beside it.
        .with_size(1280, 1000);
    #[cfg(all(feature = "renderer-wgpu", not(target_os = "android")))]
    let launcher = launcher
        .with_frame_pacing_mode(cranpose::FramePacingMode::Vsync)
        .with_test_driver(|robot| {
            let _ = CAPTURE.set(Mutex::new(robot));
        });
    launcher.run(move || SkinStudio(shared.clone(), None));
}
/// A handset-sized native preview for testing fractional scaling and touch UI.
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
/// The editor's button vocabulary.
///
/// Every control belongs to exactly one of five classes, and each class has a
/// shape of its own, so a button says what kind of thing it does before it is
/// pressed: a plain slab runs once, a red slab runs once and can lose work, a
/// slab with a left tab opens the drawer named on it, a pill is one value out
/// of a set, and a slab with a pip on the right is an on/off switch.
///
/// Only a drawer button and a chosen value are ever filled with `ACCENT`, and
/// those two can never be confused because one is a slab and one is a pill.
/// A momentary action: it happens once, and the button looks the same after.
#[composable]
fn Action(label: String, x: f32, y: f32, w: f32, click: impl Fn() + 'static) {
    Slab(label, x, y, w, CARD, click);
}
/// The same, drawn as unavailable and inert when there is nothing for it to do.
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
/// A momentary action that can destroy unexported work.
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
/// Opens the drawer named on it, and stays lit for as long as that drawer is
/// open. The tab on the left edge is what tells a closed drawer button apart
/// from an action that would run immediately.
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
    // The rule is drawn over the button rather than inside it: a `Box` in a
    // button's content is laid out in the content flow, not positioned, and a
    // label with room made for it beside it no longer fits a 30pt button.
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
/// One value out of a set -- a zoom step, which slider the frame scrubber
/// drives. Lit when it is the value in force. A pill, never a slab, so a lit
/// value cannot be mistaken for an open drawer.
/// A switch that is a row in a list. Same rules as `Toggle`, but the label sits
/// against the left edge: a long list is read down one column, and a centred
/// label turns that into a ragged search.
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
/// The same, for a row that selects rather than switches.
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
/// An on/off switch. The label always names the state the editor is in right
/// now, never the state pressing it would move to, and the pip repeats that,
/// so a switch can be read without pressing it to find out which way round it
/// is. Every switch in the editor follows this rule.
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
    // Over the button, for the same reason as the drawer tab above.
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
/// A scrollbar for the canvas: a track the whole length of an edge, and a thumb
/// sized to the fraction of the skin on screen. Dragging anywhere on the track
/// moves the thumb there, so it doubles as jump-to-position.
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
            // Same reason as the canvas: the track geometry is baked into the
            // handler, so it has to be rebuilt when the track moves or resizes.
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

/// Put a sentence in the editor's status line. Everything the editor has to
/// say goes through here, so there is one place to change when it should be
/// said somewhere louder.
fn note(shared: &SharedDocument, message: String) {
    let mut d = shared.lock().unwrap();
    d.message = message;
    d.revision += 1;
    ALERT_REVISION.store(d.revision, Ordering::Release);
}
/// The pixel under the pointer, in the skin's own coordinates.
///
/// Its own composable, reading the hover state itself, so that moving the mouse
/// invalidates this one line and nothing else. `SkinStudio` rebuilds the whole
/// canvas bitmap from the document every time it recomposes, which at pointer
/// rate would cost far more than the readout is worth.
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
/// The sprite rectangle under the pointer, outlined over the canvas and named.
///
/// Only the one being pointed at. Outlining all hundred of them at once -- and
/// in the artwork's own pixels, so the hairlines grew with the zoom -- buried
/// the artwork under the hints that were meant to point at it.
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
        // The smallest rectangle containing the pointer: sprites nest, and the
        // innermost one is the one being pointed at.
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
/// An outline around the pixel the brush would hit, scaled to the stroke width.
/// Same reasoning as `CursorReadout`: it reads the hover state itself so the
/// canvas underneath is not re-rendered to move a four-line box.
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
/// Panels that only mean anything while the canvas is being painted. Leaving
/// the canvas closes one rather than leaving a live, inert set of brushes in
/// the tool column.
fn close_painting_drawer(drawer: cranpose_core::MutableState<u8>) {
    if matches!(drawer.get_non_reactive(), 4..=8) {
        drawer.set(0);
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
#[composable]
pub fn SkinStudio(shared: SharedDocument, host: Option<StudioHost>) {
    let tick = cranpose_core::rememberMutableStateOf(|| 0u64);
    let live = cranpose_core::rememberMutableStateOf(|| false);
    let review = cranpose_core::rememberMutableStateOf(|| false);
    // The tool column is docked, not laid over the canvas, so it costs nothing
    // to start with the drawing tools already in it. An editor that opens with
    // its brushes on screen does not have to be searched for them.
    let drawer = cranpose_core::rememberMutableStateOf(|| 4u8);
    let picker = cranpose_core::rememberMutableStateOf(|| false);
    // Armed by the first press of a button that would discard unexported work.
    let confirm = cranpose_core::rememberMutableStateOf(|| false);
    // The skin pixel under the pointer. Only the readout and the brush outline
    // read it, so a mouse move never rebuilds the canvas.
    let hover = cranpose_core::rememberMutableStateOf(|| None::<[i32; 2]>);
    // Where the pointer last was, kept after it leaves the canvas: the study
    // panel is read by moving the pointer onto it, which would otherwise clear
    // the very thing it is showing.
    let studied = cranpose_core::rememberMutableStateOf(|| None::<[i32; 2]>);
    let pan = cranpose_core::rememberMutableStateOf(|| [0i32; 2]);
    let frame_kind = cranpose_core::rememberMutableStateOf(|| "volume".to_string());
    // The window is not a composition input, so it is polled alongside the
    // document: the canvas is sized from it, and a resize has to recompose.
    let scene_state = cranpose_core::rememberMutableStateOf(|| Scene::new(0., 0.));
    let poll = shared.clone();
    cranpose_core::LaunchedEffectAsync(0u8, move |_| {
        Box::pin(async move {
            // Only a lock and a compare: this recomposes when the document has
            // actually changed, so polling it at frame rate costs nothing and
            // makes every control answer immediately.
            cranpose_core::interval(Duration::from_millis(16), move || {
                let revision = poll.lock().unwrap().revision;
                if tick.get_non_reactive() != revision {
                    tick.set(revision);
                }
            })
            .await;
        })
    });
    let _ = tick.get();
    let scene = scene_state.get();
    // The canvas is a drawing surface only in its own mode; preview and the
    // state sheet are pictures of the skin, not places to paint.
    let drawing = !live.get() && !review.get();
    let (canvas_x, canvas_y, canvas_full_w, canvas_h) = scene.canvas();
    // The tool column is docked beside the canvas rather than laid over it, and
    // the canvas gives up the width it was only using as margin. Covering the
    // drawing surface with the panel that holds the brush was a concession to a
    // narrow window; a window wide enough gets both at once.
    let (split_w, split_drawer) = scene.split();
    let docked = split_drawer > 0.;
    let canvas_w = if docked { split_w } else { canvas_full_w };
    let (drawer_x, drawer_y, drawer_w, drawer_h) = if docked {
        (canvas_x + canvas_w + 12., canvas_y, split_drawer, canvas_h)
    } else {
        scene.drawer()
    };
    // Nothing to exclude from the brush once the drawer is out of the way.
    let drawer_canvas_x = if docked {
        f32::INFINITY
    } else {
        canvas_w - DRAWER_WIDTH
    };
    let (
        view,
        message,
        revision,
        im,
        path,
        dirty,
        sheet_subject,
        sheet_refusal,
        sheet_variants,
        undone,
        redoable,
    ) = {
        let d = shared.lock().unwrap();
        // A state sheet is one sprite's variants, so it can refuse: the editor
        // shows the refusal in place of the canvas rather than a wrong sprite.
        let sheet = review.get().then(|| d.state_sheet());
        let (undone, redoable) = d.history_depth();
        let target = d.state_sheet_layer();
        let variants = target.as_ref().map(|l| l.variants.len()).unwrap_or(0);
        let subject = target.map(|l| l.id);
        let refusal = match &sheet {
            Some(Err(e)) => Some(format!("{e:#}")),
            _ => None,
        };
        (
            d.view.clone(),
            d.message.clone(),
            d.revision,
            match sheet {
                Some(Ok(sheet)) => sheet,
                Some(Err(_)) => image::RgbaImage::new(1, 1),
                None => d.editor_render(),
            },
            d.path.clone(),
            d.dirty,
            subject,
            refusal,
            variants,
            undone,
            redoable,
        )
    };
    let export_path = path
        .as_deref()
        .map(std::path::PathBuf::from)
        .map(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("Skin");
            p.with_file_name(format!("{stem} edited.wsz"))
        })
        .unwrap_or_else(default_export_path)
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
    let pending_export = cranpose_core::rememberMutableStateOf(|| None::<Vec<u8>>);
    let opened = shared.clone();
    let open_launcher =
        cranpose_services::rememberOpenFileLauncher("cranamp.studio.desktop.open", move |result| {
            let d = opened.clone();
            match result {
                Ok(Some(entry)) => {
                    cranpose_core::spawn_ui_task(async move {
                        let loaded = async {
                            let bytes = entry.read_all().await?;
                            let mut doc = if entry
                                .metadata()
                                .name
                                .to_ascii_lowercase()
                                .ends_with(".cstudio")
                            {
                                Document::open_project(&bytes)?
                            } else {
                                Document::open(&bytes, None)?
                            };
                            doc.path = None;
                            open_on_whole_skin(&mut doc);
                            Ok::<_, anyhow::Error>(doc)
                        }
                        .await;
                        let mut old = d.lock().unwrap();
                        match loaded {
                            Ok(mut doc) => {
                                doc.revision = old.revision + 1;
                                *old = doc;
                            }
                            Err(e) => {
                                old.message = format!("Open: {e:#}");
                                old.revision += 1;
                            }
                        }
                    });
                }
                Err(e) => {
                    let mut doc = d.lock().unwrap();
                    doc.message = format!("Open: {e:#}");
                    doc.revision += 1;
                }
                _ => {}
            }
        });
    let saved = shared.clone();
    let export_launcher = cranpose_services::rememberSaveDocumentLauncher(
        "cranamp.studio.desktop.export",
        move |result| {
            let d = saved.clone();
            match result {
                Ok(Some(sink)) => {
                    if let Some(bytes) = pending_export.get_non_reactive() {
                        cranpose_core::spawn_ui_task(async move {
                            let outcome = cranpose::write_all(&sink, bytes).await;
                            let mut doc = d.lock().unwrap();
                            doc.message = match outcome {
                                Ok(()) => "Exported skin".into(),
                                Err(e) => format!("Export: {e:#}"),
                            };
                            doc.revision += 1;
                            pending_export.set(None);
                        });
                    }
                }
                Err(e) => {
                    let mut doc = d.lock().unwrap();
                    doc.message = format!("Export: {e:#}");
                    doc.revision += 1;
                    pending_export.set(None);
                }
                _ => pending_export.set(None),
            }
        },
    );
    let (w, h) = (im.width(), im.height());
    let bitmap =
        ImageBitmap::from_rgba8(im.width(), im.height(), im.into_raw()).expect("studio bitmap");
    let panel = view.panel.clone();
    // Which window the canvas is scrolled over. It only decides which window's
    // own controls to offer -- there is no per-window view to switch into, and
    // the drawing surface is always the whole skin. The atlas panel keeps its
    // own identity so its per-sheet controls still apply.
    let section = if panel == "canvas" {
        // Taken from the middle of the view rather than its top edge: at 6x the
        // top row is nowhere near where the eye is.
        let rows = ((canvas_h - SCROLLBAR) / view.zoom.max(1) as f32) as i32;
        match pan.get()[1] + rows / 2 {
            y if y >= 232 => "playlist".to_string(),
            y if y >= 116 => "equalizer".to_string(),
            _ => "main".to_string(),
        }
    } else {
        panel.clone()
    };
    let zoom = if review.get() {
        fit_zoom((canvas_w - 24., canvas_h - 24.), (w, h)).min(4) as f32
    } else {
        view.zoom as f32
    };

    let preview_playlist_height = view.preview_playlist_height;
    let preview_size = (275, 232 + preview_playlist_height);
    let (viewport_w, viewport_h) = if live.get() { preview_size } else { (w, h) };
    let pan_value = pan.get();
    // How far the canvas can scroll, in source pixels, for this window.
    let pan_max = [
        (viewport_w as f32 - (canvas_w - SCROLLBAR) / zoom)
            .ceil()
            .max(0.) as i32,
        (viewport_h as f32 - (canvas_h - SCROLLBAR) / zoom)
            .ceil()
            .max(0.) as i32,
    ];
    let px = pan_value[0].clamp(0, pan_max[0]) as f32 * zoom;
    let py = pan_value[1].clamp(0, pan_max[1]) as f32 * zoom;
    // Centre the skin in whatever room the canvas has. A tall skin in a wide
    // window otherwise sits jammed against the left edge with the workspace
    // empty beside it.
    let ox = ((canvas_w - SCROLLBAR - viewport_w as f32 * zoom) / 2.).max(0.) - px;
    let oy = ((canvas_h - SCROLLBAR - viewport_h as f32 * zoom) / 2.).max(0.) - py;
    let guide_rects = Rc::new(if view.guides && drawing {
        shared
            .lock()
            .unwrap()
            .guides()
            .into_iter()
            .map(|g| (g.id, g.rect))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    });
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
        .unwrap_or_else(|| "Control art and background\nalike, in one stroke.".into());
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
            // Measures the window for `Scene`. The editor is an absolutely
            // positioned scene, so nothing else here would ever ask how much
            // room it actually has.
            BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
                let measured = Scene::new(
                    scope.constraints().max_width,
                    scope.constraints().max_height,
                );
                if scene_state.get_non_reactive() != measured {
                    cranpose_core::SideEffect(move || scene_state.set(measured));
                }
            });
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
                Action("Exit presentation".into(), 20., 18., 170., move || {
                    state(&d, json!({"presentation":false}))
                });
                return;
            }
            Label("CRANAMP  /  SKIN STUDIO".into(), 22., 12., 300., 18., FG);
            Label(
                format!(
                    "{}{}",
                    path.clone().unwrap_or_else(|| "Untitled skin".into()),
                    if dirty { "   • unexported edits" } else { "" }
                ),
                336.,
                17.,
                scene.width - 380.,
                11.,
                if dirty { ACCENT_LIT } else { DIM },
            );
            // Row one is what happens to the document; row two is what the tool
            // column shows. Both are anchored from the left so the row stays one
            // row at any window width, with only the file group tracking the
            // right edge.
            {
                let d = document.clone();
                MaybeAction(
                    if undone > 0 {
                        format!("Undo {undone}")
                    } else {
                        "Undo".into()
                    },
                    20.,
                    ACTION_ROW,
                    76.,
                    undone > 0,
                    move || {
                        d.lock().unwrap().undo();
                    },
                );
            }
            {
                let d = document.clone();
                MaybeAction(
                    if redoable > 0 {
                        format!("Redo {redoable}")
                    } else {
                        "Redo".into()
                    },
                    102.,
                    ACTION_ROW,
                    76.,
                    redoable > 0,
                    move || {
                        d.lock().unwrap().redo();
                    },
                );
            }
            Toggle(
                if live.get() {
                    "Player preview"
                } else {
                    "Canvas editing"
                }
                .into(),
                186.,
                ACTION_ROW,
                140.,
                live.get(),
                move || {
                    let on = !live.get_non_reactive();
                    live.set(on);
                    pan.set([0, 0]);
                    review.set(false);
                    if on {
                        close_painting_drawer(drawer);
                    }
                },
            );
            let sheet_doc = document.clone();
            let has_subject = sheet_subject.is_some();
            Toggle(
                if review.get() {
                    "Sprite state sheet"
                } else {
                    "Canvas"
                }
                .into(),
                334.,
                ACTION_ROW,
                150.,
                review.get(),
                move || {
                    let on = !review.get_non_reactive();
                    if on && !has_subject {
                        // A state sheet is one sprite's variants, so entering
                        // the mode with nothing chosen used to show an
                        // arbitrary sprite. Ask for one instead.
                        note(
                            &sheet_doc,
                            "Choose one sprite in Sprite targets: a state sheet shows one \
                             sprite's variants."
                                .into(),
                        );
                        drawer.set(1);
                        return;
                    }
                    review.set(on);
                    live.set(false);
                    if on {
                        close_painting_drawer(drawer);
                    }
                },
            );
            if let Some(host) = host.clone() {
                let back = host.close.clone();
                Action(
                    "Player".into(),
                    scene.right(640.),
                    ACTION_ROW,
                    90.,
                    move || back(),
                );
                let d = document.clone();
                let apply = host.apply.clone();
                Action(
                    "Apply to player".into(),
                    scene.right(738.),
                    ACTION_ROW,
                    126.,
                    move || match publish(&d) {
                        Ok((bytes, path)) => apply(bytes, path),
                        Err(e) => note(&d, format!("Apply: {e:#}")),
                    },
                );
                // A hosted editor has no filesystem path to type into, so both
                // file actions go through the platform's own picker.
                let open = open_launcher.clone();
                Action(
                    "Open…".into(),
                    scene.right(872.),
                    ACTION_ROW,
                    70.,
                    move || {
                        open.launch(cranpose::FilePickerOptions::default().with_filter(
                            cranpose::FileFilter::new("Skin/project", &["wsz", "zip", "cstudio"]),
                        ))
                    },
                );
                let d = document.clone();
                let export = export_launcher.clone();
                Action(
                    "Export…".into(),
                    scene.right(950.),
                    ACTION_ROW,
                    88.,
                    move || match d.lock().unwrap().archive() {
                        Ok(bytes) => {
                            pending_export.set(Some(bytes));
                            export.launch(cranpose::SaveDocumentRequest::new(
                                "Catamp edited.wsz",
                                "application/zip",
                            ));
                        }
                        Err(e) => note(&d, format!("Export: {e:#}")),
                    },
                );
            } else {
                Label("WSZ".into(), 644., ACTION_ROW + 6., 32., 11., DIM);
                cranpose_ui::BasicTextField(
                    path_field,
                    Modifier::empty()
                        .absolute_offset(676., ACTION_ROW + 3.)
                        .size_points(190., 24.)
                        .background(BG),
                    text_style(12., FG),
                );
                let d = document.clone();
                Action(
                    "Open".into(),
                    scene.right(872.),
                    ACTION_ROW,
                    70.,
                    move || {
                        let p = path_field.text();
                        let result = mcp::call("studio_open", json!({"path":p}), &d);
                        let mut doc = d.lock().unwrap();
                        if let Err(e) = result {
                            doc.message = format!("Open: {e:#}");
                        } else {
                            // A replaced document arrives on the library's own
                            // per-window default; the editor has no such view.
                            open_on_whole_skin(&mut doc);
                        }
                        doc.revision += 1;
                    },
                );
                let d = document.clone();
                Action(
                    "Export".into(),
                    scene.right(950.),
                    ACTION_ROW,
                    88.,
                    move || {
                        let path = path_field.text();
                        let mut doc = d.lock().unwrap();
                        if let Err(e) = doc.export(std::path::Path::new(&path)) {
                            doc.message = format!("Export: {e:#}");
                            doc.revision += 1;
                        }
                    },
                );
            }
            {
                // The one button that can throw away unexported work. It refuses
                // while there is any, and says so where the eye already is --
                // next to the button, not in the status line at the far bottom.
                let d = document.clone();
                let armed = confirm.get();
                let label = if armed {
                    "Discard & start blank"
                } else {
                    "New blank"
                };
                Danger(
                    label.into(),
                    scene.right(1046.),
                    ACTION_ROW,
                    94.,
                    move || {
                        let unsaved = d.lock().unwrap().dirty;
                        if unsaved && !armed {
                            confirm.set(true);
                            note(
                                &d,
                                "This skin has edits that are not exported. Press again to \
                                 discard them and start blank."
                                    .into(),
                            );
                            return;
                        }
                        confirm.set(false);
                        let call = mcp::call("studio_new", json!({"discard":true}), &d);
                        if let Err(e) = call {
                            note(&d, format!("New: {e:#}"));
                        } else {
                            // A replaced document arrives on the library's own
                            // per-window default. The editor has no per-window
                            // view, so put it back on the whole skin.
                            {
                                let mut doc = d.lock().unwrap();
                                open_on_whole_skin(&mut doc);
                                doc.revision += 1;
                            }
                            live.set(false);
                            review.set(false);
                            pan.set([0, 0]);
                        }
                    },
                );
                if armed {
                    Action(
                        "Keep editing".into(),
                        scene.right(940.),
                        ACTION_ROW,
                        100.,
                        move || confirm.set(false),
                    );
                }
            }
            // Row two: every drawer the editor has, in one row, each button
            // named exactly as the panel it opens.
            {
                let mut x = 20.;
                let mut panel_button = |id: u8, label: &str, width: f32| {
                    let at = x;
                    x += width + 8.;
                    (at, id, label.to_string(), width)
                };
                // Every button is named exactly as the panel it opens, and the
                // whole row fits the narrowest window the editor accepts.
                let buttons: Vec<(f32, u8, String, f32)> = if !drawing {
                    vec![
                        panel_button(1, "Sprite targets", 120.),
                        panel_button(9, "Skin options", 116.),
                        panel_button(3, "Skin atlases", 116.),
                        panel_button(2, "Edit history", 108.),
                    ]
                } else {
                    vec![
                        panel_button(4, "Drawing tools", 118.),
                        panel_button(7, "Painting layers", 128.),
                        panel_button(1, "Sprite targets", 120.),
                        panel_button(6, "Sprite rectangles", 142.),
                        panel_button(3, "Skin atlases", 116.),
                        panel_button(2, "Edit history", 108.),
                        panel_button(5, "Pixel study", 104.),
                        panel_button(9, "Skin options", 116.),
                    ]
                };
                for (at, id, label, width) in buttons {
                    let d = document.clone();
                    Panel(label, at, PANEL_ROW, width, drawer.get() == id, move || {
                        // Part rectangles are only worth a panel when they are
                        // drawn, so opening that one turns them on.
                        if id == 6 {
                            state(&d, json!({"guides":true}));
                        }
                        drawer.set(if drawer.get_non_reactive() == id {
                            0
                        } else {
                            id
                        });
                    });
                }
                if live.get() {
                    let d = document.clone();
                    Action("Presentation".into(), x, PANEL_ROW, 130., move || {
                        state(&d, json!({"presentation":true}))
                    });
                    let d = document.clone();
                    Toggle(
                        if preview_playlist_height == 145 {
                            "Compact playlist"
                        } else {
                            "Tall playlist"
                        }
                        .into(),
                        x + 138.,
                        PANEL_ROW,
                        160.,
                        preview_playlist_height != 145,
                        move || {
                            state(
                                &d,
                                json!({"preview_playlist_height": if preview_playlist_height == 145 { 261 } else { 145 }}),
                            );
                            pan.set([0, 0]);
                        },
                    );
                }
            }
            Box(
                Modifier::empty()
                    .absolute_offset(20., CANVAS_ORIGIN.1)
                    .size_points(190., canvas_h)
                    .background(PANEL_BACK)
                    .rounded_corners(6.),
                BoxSpec::default(),
                || {},
            );
            // Painting controls only while the canvas is the drawing surface.
            // Preview and the state sheet used to leave a full sidebar of live,
            // completely inert brushes, colours and targets on screen with one
            // 11pt line of grey text as the only sign they did nothing.
            if drawing {
                Label("DRAW".into(), 32., 142., 160., 11., DIM);
                {
                    let d = document.clone();
                    Choice(
                        view.brush.clone(),
                        30.,
                        158.,
                        78.,
                        !picker.get(),
                        move || {
                            picker.set(false);
                            state(&d, json!({"brush":"pencil"}));
                        },
                    );
                }
                Choice(
                    "Pick pixel".into(),
                    115.,
                    158.,
                    84.,
                    picker.get(),
                    move || {
                        picker.set(true);
                    },
                );
                Label(
                    if picker.get() {
                        "Click a pixel to take its colour.".into()
                    } else {
                        "Drag to paint · right-click\nsamples the colour under it".into()
                    },
                    32.,
                    192.,
                    175.,
                    11.,
                    DIM,
                );
                // Stroke width is the parameter a painter reaches for most, so it
                // lives in the sidebar rather than behind a panel across the window.
                Label("STROKE WIDTH".into(), 32., 226., 110., 11., DIM);
                Label(format!("{} px", view.brush_size), 150., 225., 50., 12., FG);
                for (i, (label, delta)) in [("− Thinner", -1i32), ("+ Thicker", 1)]
                    .into_iter()
                    .enumerate()
                {
                    let d = document.clone();
                    let size = view.brush_size as i32;
                    Action(label.into(), 30. + i as f32 * 86., 242., 78., move || {
                        state(&d, json!({"brush_size":(size + delta).clamp(1, 32)}))
                    });
                }
                Label("COLOUR".into(), 32., 282., 160., 11., DIM);
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
                            .absolute_offset(
                                32. + (i % 4) as f32 * 41.,
                                298. + (i / 4) as f32 * 34.,
                            )
                            .size_points(32., 25.)
                            .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                            .rounded_corners(3.)
                            .clickable(move |_| state(&d, json!({"color":c}))),
                        BoxSpec::default(),
                        || {},
                    );
                }
                Panel(
                    "Colour picker…".into(),
                    30.,
                    364.,
                    168.,
                    drawer.get() == 8,
                    move || drawer.set(if drawer.get_non_reactive() == 8 { 0 } else { 8 }),
                );
                // The colour in force, as a swatch rather than only as hex.
                // Typing an exact value now lives in the picker, beside the
                // field and the palette it belongs with.
                {
                    let rgba = parse_color(&view.color).unwrap_or([255, 255, 255, 255]);
                    Box(
                        Modifier::empty()
                            .absolute_offset(32., 402.)
                            .size_points(30., 20.)
                            .background(Color::from_rgba_u8(rgba[0], rgba[1], rgba[2], 255))
                            .rounded_corners(3.),
                        BoxSpec::default(),
                        || {},
                    );
                }
                Label(format!("Brush {}", view.color), 70., 405., 130., 11., FG);
                Label("EDIT SCOPE".into(), 32., 440., 160., 11., DIM);
                {
                    let d = document.clone();
                    let all = view.all_states;
                    Toggle(
                        if all {
                            "All sprite states"
                        } else {
                            "Current state only"
                        }
                        .into(),
                        30.,
                        456.,
                        168.,
                        all,
                        move || state(&d, json!({"all_states":!all})),
                    );
                }
                // "Sprite targets" everywhere: these are the sprites a stroke is
                // routed into. The independent stack of artwork planes over the
                // atlases is "painting layers", and never shares the word.
                // Read-only: the panel that changes this is one row up in the
                // toolbar, and a second button here was only a duplicate.
                Label("SPRITE TARGETS".into(), 32., 494., 165., 11., DIM);
                Label(
                    if view.layers.is_empty() {
                        "Auto · everything beneath".into()
                    } else {
                        format!("{} chosen by hand", view.layers.len())
                    },
                    32.,
                    510.,
                    165.,
                    11.,
                    FG,
                );
                Label(layer_info.clone(), 32., 530., 165., 10., DIM);
            } else {
                Label(
                    if live.get() {
                        "PLAYER PREVIEW"
                    } else {
                        "SPRITE STATE SHEET"
                    }
                    .into(),
                    32.,
                    142.,
                    165.,
                    11.,
                    DIM,
                );
                Label(
                    if live.get() {
                        "The skin as the player draws it,\nwith live text and a real\nplaylist. Nothing here paints."
                    } else {
                        "Every variant of one sprite,\nnumbered. Read-only: pick the\nsprite in Sprite targets."
                    }
                    .into(),
                    32.,
                    164.,
                    168.,
                    11.,
                    DIM,
                );
                Action("Back to the canvas".into(), 30., 230., 168., move || {
                    live.set(false);
                    review.set(false);
                });
            }
            Label("ZOOM · WHOLE PIXELS".into(), 32., 578., 165., 11., DIM);
            for (i, z) in [1, 2, 3, 4, 6, 8].into_iter().enumerate() {
                let d = document.clone();
                Choice(
                    format!("{z}×"),
                    30. + (i % 3) as f32 * 57.,
                    594. + (i / 3) as f32 * 34.,
                    51.,
                    view.zoom == z,
                    move || state(&d, json!({"zoom":z})),
                );
            }
            {
                let d = document.clone();
                let room = (canvas_w - SCROLLBAR - 8., canvas_h - SCROLLBAR - 8.);
                Action("Fit whole skin".into(), 30., 666., 168., move || {
                    let mut doc = d.lock().unwrap();
                    doc.view.panel = "canvas".into();
                    let best = fit_zoom(room, doc.canvas_size());
                    let _ = doc.state(
                        json!({"panel":"canvas","layer":"auto","zoom":best,"presentation":false}),
                    );
                    live.set(false);
                    review.set(false);
                    pan.set([0, 0]);
                });
            }
            Label(
                "Wheel scrolls the canvas, alt+wheel\nsideways, ctrl+wheel zooms at the\n\
                 pointer. Drag with the middle button\nto pan."
                    .into(),
                32.,
                706.,
                168.,
                10.,
                DIM,
            );
            Label(
                if live.get() {
                    "CRANAMP PLAYER RENDER  ·  drawing is off".into()
                } else if review.get() {
                    format!(
                        "SPRITE STATE SHEET  ·  {}  ·  {} variants  ·  drawing is off",
                        sheet_subject.clone().unwrap_or_else(|| "no sprite".into()),
                        sheet_variants
                    )
                } else {
                    format!(
                        "{}  ·  {} × {} source pixels  ·  {}×  ·  centre: {}",
                        if panel == "canvas" {
                            "WHOLE SKIN".into()
                        } else if panel == "atlas" {
                            format!("ATLAS  {}", view.sheet)
                        } else {
                            panel.to_uppercase()
                        },
                        w,
                        h,
                        view.zoom,
                        section
                    )
                },
                230.,
                108.,
                420.,
                11.,
                DIM,
            );
            // Where the brush is, in the skin's own pixels. Its own composable,
            // so a mouse move repaints this line and the brush outline and
            // nothing else -- the canvas bitmap is rebuilt only when the
            // picture changes.
            CursorReadout(hover, canvas_x + canvas_w - 200., 108.);
            if let Some(refusal) = sheet_refusal.clone() {
                Label(
                    refusal,
                    canvas_x + 40.,
                    canvas_y + 60.,
                    canvas_w - 80.,
                    13.,
                    DIM,
                );
                let d = document.clone();
                Panel(
                    "Sprite targets".into(),
                    canvas_x + 40.,
                    canvas_y + 120.,
                    150.,
                    drawer.get() == 1,
                    move || {
                        let _ = &d;
                        drawer.set(1)
                    },
                );
            }
            let canvas_doc = document.clone();
            let canvas_bitmap = bitmap.clone();
            let canvas_panel = panel.clone();
            let canvas_guides = guide_rects.clone();
            let nav_doc = document.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(canvas_x, canvas_y)
                    .size_points(canvas_w, canvas_h)
                    .clip_to_bounds()
                    .background(Color(0.04, 0.05, 0.07, 1.))
                    // Navigation sits on the canvas panel, under the drawing
                    // surface, so it works over the artwork and over the margin
                    // around it alike: the brush consumes presses and drags, and
                    // everything it does not consume arrives here.
                    //
                    // cranpose delivers key events only to a focused text field,
                    // so there is no Cmd+Z, no bracket keys and no space-to-pan
                    // to be had; every gesture the editor offers has to be one
                    // the pointer alone can make.
                    .pointer_input(
                        (
                            view.zoom,
                            canvas_w.to_bits(),
                            canvas_h.to_bits(),
                            ox.to_bits(),
                            oy.to_bits(),
                        ),
                        move |scope: PointerInputScope| {
                            let d = nav_doc.clone();
                            async move {
                                scope
                                    .await_pointer_event_scope(|events| async move {
                                        let mut grab: Option<([f32; 2], [i32; 2])> = None;
                                        // Panning is in source pixels, and the
                                        // skin is never larger than its own
                                        // canvas, so these are the only bounds
                                        // either axis can need.
                                        let limit = [viewport_w as i32, viewport_h as i32];
                                        loop {
                                            let event = events.await_pointer_event().await;
                                            match event.kind {
                                                PointerEventKind::Zoom => {
                                                    // ctrl+wheel reaches a
                                                    // handler as a zoom gesture,
                                                    // not as a scroll with a
                                                    // modifier: the shell's
                                                    // wheel policy converts it
                                                    // before dispatch.
                                                    let step: i32 =
                                                        if event.zoom_delta > 1. { 1 } else { -1 };
                                                    let next = (view.zoom as i32 + step).clamp(1, 8)
                                                        as f32;
                                                    if next != zoom {
                                                        // Zoom about the pointer:
                                                        // the pixel under it stays
                                                        // under it, so magnifying
                                                        // does not also lose the
                                                        // reader's place.
                                                        // Panning is in whole
                                                        // source pixels, so the
                                                        // anchor is the pixel the
                                                        // pointer names rather than
                                                        // the exact point under it:
                                                        // the smallest pan that
                                                        // keeps that same pixel
                                                        // under the pointer is the
                                                        // ceiling, not the nearest.
                                                        let anchor = |along: f32,
                                                                      origin: f32,
                                                                      room: f32,
                                                                      span: f32| {
                                                            let pixel = ((along - origin) / zoom)
                                                                .floor();
                                                            let centre = ((room
                                                                - SCROLLBAR
                                                                - span * next)
                                                                / 2.)
                                                                .max(0.);
                                                            (pixel - (along - centre) / next).ceil()
                                                        };
                                                        pan.set([
                                                            anchor(
                                                                event.position.x,
                                                                ox,
                                                                canvas_w,
                                                                viewport_w as f32,
                                                            )
                                                            .clamp(0., limit[0] as f32)
                                                                as i32,
                                                            anchor(
                                                                event.position.y,
                                                                oy,
                                                                canvas_h,
                                                                viewport_h as f32,
                                                            )
                                                            .clamp(0., limit[1] as f32)
                                                                as i32,
                                                        ]);
                                                        state(&d, json!({ "zoom": next as u8 }));
                                                    }
                                                    event.consume();
                                                }
                                                PointerEventKind::Scroll => {
                                                    // The shell has already applied
                                                    // its wheel policy, alt-held
                                                    // axis swap included, so both
                                                    // axes are used as they arrive.
                                                    pan.update(|p| {
                                                        for axis in 0..2 {
                                                            let delta = if axis == 0 {
                                                                event.scroll_delta.x
                                                            } else {
                                                                event.scroll_delta.y
                                                            };
                                                            p[axis] = (p[axis]
                                                                - (delta / zoom).round() as i32)
                                                                .clamp(0, limit[axis]);
                                                        }
                                                    });
                                                    event.consume();
                                                }
                                                PointerEventKind::Down => {
                                                    // The middle button pans. It is
                                                    // the one grab gesture available
                                                    // without a spacebar.
                                                    if event.buttons.contains(PointerButton::Middle)
                                                    {
                                                        grab = Some((
                                                            [event.position.x, event.position.y],
                                                            pan.get_non_reactive(),
                                                        ));
                                                        event.consume();
                                                    }
                                                }
                                                PointerEventKind::Move => {
                                                    if let Some((from, base)) = grab {
                                                        if event
                                                            .buttons
                                                            .contains(PointerButton::Middle)
                                                        {
                                                            let travel = [
                                                                event.position.x - from[0],
                                                                event.position.y - from[1],
                                                            ];
                                                            pan.set([
                                                                (base[0]
                                                                    - (travel[0] / zoom).round()
                                                                        as i32)
                                                                    .clamp(0, limit[0]),
                                                                (base[1]
                                                                    - (travel[1] / zoom).round()
                                                                        as i32)
                                                                    .clamp(0, limit[1]),
                                                            ]);
                                                            event.consume();
                                                        } else {
                                                            grab = None;
                                                        }
                                                    }
                                                }
                                                PointerEventKind::Up
                                                | PointerEventKind::Cancel
                                                | PointerEventKind::Exit => {
                                                    grab = None;
                                                    hover.set(None);
                                                }
                                                _ => {}
                                            }
                                        }
                                    })
                                    .await;
                            }
                        },
                    ),
                BoxSpec::default(),
                move || {
                    if live.get() {
                        if view.grid && zoom >= 4. && !review.get() {
                            for x in 0..w {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(x as f32 * zoom + ox, oy)
                                        .size_points(1., h as f32 * zoom)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            for y in 0..h {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(ox, y as f32 * zoom + oy)
                                        .size_points(w as f32 * zoom, 1.)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                        }
                        let d = canvas_doc.clone();
                        Box(
                            Modifier::empty().absolute_offset(ox, oy).required_size(
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
                            Modifier::empty().absolute_offset(ox, oy).required_size(
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
                                        .absolute_offset(x as f32 * zoom + ox, oy)
                                        .size_points(1., h as f32 * zoom)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            for y in 0..h {
                                Box(
                                    Modifier::empty()
                                        .absolute_offset(ox, y as f32 * zoom + oy)
                                        .size_points(w as f32 * zoom, 1.)
                                        .background(Color(0.05, 0.07, 0.10, 0.28)),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                        }
                        let d = canvas_doc.clone();
                        if !review.get() {
                            Box(
                                Modifier::empty()
                                    .absolute_offset(ox, oy)
                                    .required_size(cranpose_ui::Size::new(w as f32 * zoom, h as f32 * zoom))
                                    .pointer_input(
                                        // The mapping from a pointer to a skin
                                        // pixel is built from the canvas
                                        // geometry, so the handler has to be
                                        // rebuilt whenever that geometry moves:
                                        // a drawer opening or a window resize
                                        // shifts the image, and a stale handler
                                        // then paints somewhere else entirely.
                                        (
                                            canvas_panel.clone(),
                                            view.zoom,
                                            ox.to_bits(),
                                            oy.to_bits(),
                                            drawer_canvas_x.to_bits(),
                                        ),
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
                                                                if drawer.get_non_reactive()
                                                                    != 0
                                                                    && ox + event.position.x
                                                                        >= drawer_canvas_x
                                                                {
                                                                    // Under the open panel: end
                                                                    // any stroke rather than
                                                                    // painting out of sight.
                                                                    if origin.take().is_some() {
                                                                        d.lock()
                                                                            .unwrap()
                                                                            .finish_stroke();
                                                                    }
                                                                    last = None;
                                                                    hover.set(None);
                                                                    continue;
                                                                }
                                                                match event.kind {
                                                                    PointerEventKind::Move
                                                                    | PointerEventKind::Enter => {
                                                                        hover.set(Some(point));
                                                                        studied.set(Some(point));
                                                                    }
                                                                    PointerEventKind::Exit => {
                                                                        hover.set(None);
                                                                    }
                                                                    _ => {}
                                                                }
                                                                match event.kind {
                                                                    PointerEventKind::Down => {
                                                                        // Only the two buttons
                                                                        // that mean something
                                                                        // here. A middle press
                                                                        // belongs to the panel
                                                                        // underneath, which pans
                                                                        // with it; consuming it
                                                                        // as a stroke meant
                                                                        // middle-drag painted
                                                                        // over the artwork and
                                                                        // panned only over the
                                                                        // margin around it.
                                                                        let sampling = picker
                                                                            .get_non_reactive()
                                                                            || event.buttons.contains(
                                                                                PointerButton::Secondary,
                                                                            );
                                                                        if !sampling
                                                                            && !event.buttons.contains(
                                                                                PointerButton::Primary,
                                                                            )
                                                                        {
                                                                            continue;
                                                                        }
                                                                        let mut doc =
                                                                            d.lock().unwrap();
                                                                        // The right button
                                                                        // samples with no mode
                                                                        // to enter and leave.
                                                                        if sampling {
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
                                                                            picker.set(false);
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
                        // Where the next stroke would land, sized to the
                        // stroke width. Drawn over the artwork rather than
                        // into it, so following the pointer costs one outline
                        // and not a re-render of the whole skin.
                        if !review.get() {
                            GuideHint(
                                hover,
                                canvas_guides.clone(),
                                zoom,
                                ox,
                                oy,
                                (8., canvas_h - 24.),
                            );
                            BrushCursor(hover, zoom, view.brush_size, ox, oy);
                        }
                    }
                },
            );
            if panel != "canvas" && drawing {
                // One atlas at a time is a detour, and the way back has to be
                // on the canvas rather than only inside a panel.
                let d = document.clone();
                Action(
                    "← The whole skin".into(),
                    canvas_x + 8.,
                    canvas_y + canvas_h - 40.,
                    150.,
                    move || {
                        let mut doc = d.lock().unwrap();
                        open_on_whole_skin(&mut doc);
                        doc.revision += 1;
                        drop(doc);
                        pan.set([0, 0]);
                    },
                );
            }
            let state_y = scene.state_y();
            // The most consequential strip in the editor had no name at all: it
            // decides which variant of every sprite the canvas is drawing.
            Label("SPRITE STATE".into(), 24., state_y - 2., 180., 11., DIM);
            Label(
                "the variant the canvas draws".into(),
                24.,
                state_y + 14.,
                190.,
                10.,
                DIM,
            );
            {
                let d = document.clone();
                let pressed = view.pressed;
                Toggle(
                    if pressed {
                        "Buttons pressed"
                    } else {
                        "Buttons released"
                    }
                    .into(),
                    230.,
                    state_y,
                    136.,
                    pressed,
                    move || state(&d, json!({"pressed":!pressed})),
                );
            }
            {
                let d = document.clone();
                let active = view.active;
                Toggle(
                    if active {
                        "Window focused"
                    } else {
                        "Window behind"
                    }
                    .into(),
                    374.,
                    state_y,
                    128.,
                    active,
                    move || state(&d, json!({"active":!active})),
                );
            }
            // Every slider the skin has, always. Which of them this row offered
            // used to depend on where the canvas happened to be scrolled, so
            // the playlist scrollbar's frames could only be reached by already
            // knowing to scroll to the playlist.
            let kind = frame_kind.get();
            for (i, k) in ["volume", "balance", "position", "eq", "scroll"]
                .iter()
                .enumerate()
            {
                let k = k.to_string();
                Choice(
                    k.clone(),
                    510. + i as f32 * 62.,
                    state_y,
                    58.,
                    kind == k,
                    move || frame_kind.set(k.clone()),
                );
            }
            let value = match kind.as_str() {
                "eq" => view.eq[0],
                "scroll" => view.scroll,
                "balance" => view.balance,
                "position" => view.position,
                _ => view.volume,
            };
            Label(
                format!("{kind}  {value:02}/27"),
                828.,
                state_y + 6.,
                150.,
                12.,
                FG,
            );
            for (delta, label, x) in [(-1, "−", 920.), (1, "+", 968.)] {
                let d = document.clone();
                let kind = kind.clone();
                Action(label.into(), x, state_y, 46., move || {
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
                Action(
                    "Advance every slider".into(),
                    1022.,
                    state_y,
                    118.,
                    move || {
                        let mut doc = d.lock().unwrap();
                        let next = (doc.view.volume + 1) % 28;
                        let _=doc.state(json!({"volume":next,"balance":next,"position":next,"scroll":next,"eq":vec![next;11]}));
                    },
                );
            }
            {
                let d = document.clone();
                let kind = kind.clone();
                cranpose_ui::Slider(
                    Modifier::empty()
                        .absolute_offset(230., scene.slider_y())
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
                scene.right(746.),
                scene.slider_y() + 4.,
                370.,
                11.,
                DIM,
            );
            // Scrollbars sit over the canvas' own edges, after it, so they take
            // the pointer there instead of the brush.
            if !review.get() && !live.get() && pan_max[1] > 0 {
                CanvasScrollbar(
                    true,
                    (
                        canvas_x + canvas_w - SCROLLBAR,
                        canvas_y,
                        SCROLLBAR,
                        canvas_h - SCROLLBAR,
                    ),
                    (canvas_h - SCROLLBAR) / zoom,
                    viewport_h as f32,
                    pan_value[1],
                    pan_max[1],
                    move |v| pan.update(|p| p[1] = v),
                );
            }
            if !review.get() && !live.get() && pan_max[0] > 0 {
                CanvasScrollbar(
                    false,
                    (
                        canvas_x,
                        canvas_y + canvas_h - SCROLLBAR,
                        canvas_w - SCROLLBAR,
                        SCROLLBAR,
                    ),
                    (canvas_w - SCROLLBAR) / zoom,
                    viewport_w as f32,
                    pan_value[0],
                    pan_max[0],
                    move |v| pan.update(|p| p[0] = v),
                );
            }
            Label(
                message.clone(),
                230.,
                scene.message_y(),
                scene.width - 265.,
                12.,
                if ALERT_REVISION.load(Ordering::Acquire) == revision {
                    ALERT
                } else {
                    DIM
                },
            );
            if drawer.get() != 0 {
                let d = document.clone();
                Box(
                    Modifier::empty()
                        .absolute_offset(drawer_x, drawer_y)
                        .size_points(drawer_w, drawer_h)
                        .background(PANEL_BACK)
                        .rounded_corners(6.),
                    BoxSpec::default(),
                    move || {
                        Action("Close".into(), 290., 10., 78., move || drawer.set(0));
                        let room = (drawer_w - 24., drawer_h);
                        if drawer.get() == 1 {
                            LayerChooser(d.clone(), revision, room);
                        } else if drawer.get() == 2 {
                            HistoryChooser(d.clone(), revision, room);
                        } else if drawer.get() == 6 {
                            GuideChooser(d.clone(), revision, room);
                        } else if drawer.get() == 7 {
                            PaintChooser(d.clone(), revision, room);
                        } else if drawer.get() == 5 {
                            StudyChooser(d.clone(), revision, room, studied);
                        } else if drawer.get() == 4 {
                            BrushChooser(d.clone(), revision);
                        } else if drawer.get() == 8 {
                            ColorChooser(d.clone(), revision, room);
                        } else if drawer.get() == 9 {
                            SkinOptionsChooser(d.clone(), revision, room);
                        } else {
                            Label("SKIN ATLASES".into(), 12., 17., 260., 14., FG);
                            Label(
                                "One BMP at a time, at native coordinates. Same\npencil, pixel picker and undo history."
                                    .into(),
                                12.,
                                51.,
                                355.,
                                11.,
                                DIM,
                            );
                            let sheets = d.lock().unwrap().sheets();
                            let (current_panel, current_sheet) = {
                                let doc = d.lock().unwrap();
                                (doc.view.panel.clone(), doc.view.sheet.clone())
                            };
                            {
                                let target = d.clone();
                                ListChoice(
                                    "The whole skin".into(),
                                    12.,
                                    84.,
                                    drawer_w - 24.,
                                    current_panel == "canvas",
                                    move || {
                                        let mut doc = target.lock().unwrap();
                                        let _ = doc.state(json!({"panel":"canvas","layer":"auto","presentation":false}));
                                        open_on_whole_skin(&mut doc);
                                        doc.revision += 1;
                                        drop(doc);
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
                                    },
                                );
                            }
                            for (i, (name, w, h)) in sheets.into_iter().enumerate() {
                                let target = d.clone();
                                let on = current_panel == "atlas" && current_sheet == name;
                                let caption = format!("{name}      {w} × {h}");
                                ListChoice(
                                    caption,
                                    12.,
                                    130. + i as f32 * 38.,
                                    drawer_w - 24.,
                                    on,
                                    move || {
                                        state(
                                            &target,
                                            json!({"panel":"atlas","sheet":name,"layer":"sheet","zoom":2,"presentation":false}),
                                        );
                                        live.set(false);
                                        review.set(false);
                                        pan.set([0, 0]);
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
    Label("STROKE WIDTH".into(), 12., 210., 172., 12., DIM);
    for (i, size) in [1, 2, 3, 4, 8, 16].into_iter().enumerate() {
        let d = shared.clone();
        Choice(
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
            Action(label.into(), x, 271., 110., move || {
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
    Label("PAPER GRAIN".into(), 12., 300., 172., 12., DIM);
    for (i, (amount, label)) in [(0u32, "Smooth"), (5, "Fine"), (9, "Paper"), (16, "Coarse")]
        .into_iter()
        .enumerate()
    {
        let d = shared.clone();
        Choice(
            label.into(),
            12. + i as f32 * 89.,
            322.,
            82.,
            v.grain == amount,
            move || state(&d, json!({"grain":amount})),
        );
    }
    Label("STRENGTH".into(), 12., 360., 172., 12., DIM);
    for (i, (amount, label)) in [(255u32, "Solid"), (190, "75%"), (128, "50%"), (64, "25%")]
        .into_iter()
        .enumerate()
    {
        let d = shared.clone();
        Choice(
            label.into(),
            12. + i as f32 * 89.,
            382.,
            82.,
            v.opacity == amount,
            move || state(&d, json!({"opacity":amount})),
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
            420. + (i / 2) as f32 * 48.,
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
        595.,
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
        630.,
        353.,
        10.,
        DIM,
    );
}
/// Everything about the skin itself rather than about painting it.
///
/// These used to be buttons in the toolbar that appeared and disappeared as the
/// canvas scrolled past the window they belonged to: a control nobody could
/// find, because finding it meant already knowing to scroll to the playlist.
/// They are all here, all of the time.
#[composable]
fn SkinOptionsChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    // Taller than the column on a small window, so the whole block scrolls
    // rather than losing the visualizer palette off the bottom edge.
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
                Modifier::empty().size_points(room.0 + 24., 780.),
                BoxSpec::default(),
                move || {
                    let (layout, playlist_background, eq_handles, selection) = {
                        let d = shared.lock().unwrap();
                        let sheets = d.sheets();
                        (
                            d.layout(),
                            d.has_playlist_background(),
                            sheets.iter().any(|(n, _, _)| n == "eqhandles.bmp"),
                            sheets.iter().any(|(n, _, _)| n == "plselection.bmp"),
                        )
                    };
                    Label("SKIN OPTIONS".into(), 12., 17., 270., 14., FG);
                    Label(
                "What the skin is, rather than how it is painted. Turning\none on adds the sheet it needs; turning it off removes it."
                    .into(),
                12.,
                41.,
                355.,
                11.,
                DIM,
            );
                    Label("TIME READOUT".into(), 12., 86., 200., 11., DIM);
                    for (i, (caption, value, current)) in [
                        (
                            "Classic",
                            "classic",
                            layout.footer == super::skin::FooterLayout::Classic,
                        ),
                        (
                            "Time / total",
                            "time-total",
                            layout.footer == super::skin::FooterLayout::TimeTotal,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let d = shared.clone();
                        Choice(
                            caption.into(),
                            12. + i as f32 * 130.,
                            104.,
                            122.,
                            current,
                            move || {
                                let mut doc = d.lock().unwrap();
                                if let Err(error) =
                                    doc.set_layout(&json!({ "footer": value }), "Human")
                                {
                                    doc.message = error.to_string();
                                }
                            },
                        );
                    }
                    Label("EQUALIZER SLIDER TRAVEL".into(), 12., 154., 260., 11., DIM);
                    Label(format!("{} px", layout.eq_travel), 12., 174., 60., 12., FG);
                    for (i, (label, delta)) in [("− Shorter", -1_i16), ("+ Longer", 1)]
                        .into_iter()
                        .enumerate()
                    {
                        let d = shared.clone();
                        let travel = layout.eq_travel;
                        Action(label.into(), 76. + i as f32 * 96., 168., 90., move || {
                            let value = (travel as i16 + delta).clamp(1, 52);
                            let mut doc = d.lock().unwrap();
                            if let Err(error) =
                                doc.set_layout(&json!({ "eq_travel": value }), "Human")
                            {
                                doc.message = error.to_string();
                            }
                        });
                    }
                    Label("EXTRA ARTWORK".into(), 12., 222., 200., 11., DIM);
                    {
                        let d = shared.clone();
                        Toggle(
                            if playlist_background {
                                "Playlist has its own background"
                            } else {
                                "Playlist reuses the main background"
                            }
                            .into(),
                            12.,
                            240.,
                            348.,
                            playlist_background,
                            move || {
                                d.lock()
                                    .unwrap()
                                    .set_playlist_background(!playlist_background, "Human");
                            },
                        );
                    }
                    {
                        let d = shared.clone();
                        Toggle(
                            if eq_handles {
                                "Equalizer sliders have their own art"
                            } else {
                                "Equalizer sliders reuse the main art"
                            }
                            .into(),
                            12.,
                            278.,
                            348.,
                            eq_handles,
                            move || {
                                let mut doc = d.lock().unwrap();
                                if let Err(error) = doc.set_eq_handles(!eq_handles, "Human") {
                                    doc.message = error.to_string();
                                }
                            },
                        );
                    }
                    {
                        let d = shared.clone();
                        Toggle(
                            if selection {
                                "Playlist selection has its own art"
                            } else {
                                "Playlist selection is a flat colour"
                            }
                            .into(),
                            12.,
                            316.,
                            348.,
                            selection,
                            move || {
                                d.lock()
                                    .unwrap()
                                    .set_playlist_selection(!selection, "Human");
                            },
                        );
                    }
                    {
                        let d = shared.clone();
                        let glass = layout.visualizer_glass;
                        Toggle(
                            if glass {
                                "Visualizer is drawn under glass"
                            } else {
                                "Visualizer is drawn flat"
                            }
                            .into(),
                            12.,
                            354.,
                            348.,
                            glass,
                            move || {
                                let mut doc = d.lock().unwrap();
                                if let Err(error) =
                                    doc.set_layout(&json!({ "visualizer_glass": !glass }), "Human")
                                {
                                    doc.message = error.to_string();
                                }
                            },
                        );
                    }
                    // The two palettes a classic skin carries outside its bitmaps. They were
                    // reachable only over MCP, which made them invisible to anyone painting by
                    // hand -- and the playlist's own colours are not a detail.
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
                "24 slots: 0 background, 1-16 the bars, 17 the peak,\n18-23 the oscilloscope.".into(),
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
                                    if let Err(e) =
                                        doc.visualizer_palette(&json!({ "colors": next.clone() }))
                                    {
                                        doc.message = e.to_string();
                                        doc.revision += 1;
                                    }
                                }),
                            BoxSpec::default(),
                            || {},
                        );
                    }
                    Label("AUTOMATION".into(), 12., 694., 200., 11., DIM);
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
    // A classic skin has around sixty named sprites. Scrolling to one by eye is
    // the slowest way to reach it.
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
                        // The pip carries the state; the "[ ]" the caption used
                        // to carry said the same thing a second time.
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
        assert_eq!(exported, entries(super::super::BUNDLED_SKINS[0].bytes));
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
    /// A state sheet is one sprite's variants, and the editor has to be told
    /// which sprite. Falling back to an arbitrary one meant the first press of
    /// the button always showed something nobody had asked for.
    #[test]
    fn a_state_sheet_needs_a_sprite_and_shows_every_variant_of_it() {
        let mut d = Document::open(include_bytes!("../../../assets/winamp.wsz"), None).unwrap();
        d.state(json!({"layer":"auto"})).unwrap();
        assert!(
            d.state_sheet().is_err(),
            "with no sprite chosen there is nothing to lay out"
        );

        d.state(json!({"layer":"volume.track"})).unwrap();
        let sheet = d.state_sheet().expect("a chosen sprite has a sheet");
        assert_eq!(sheet.dimensions(), (546, 124));
        // The classic transparency key is not a colour. Copying it verbatim
        // turned sprites like the volume track into flat magenta tiles.
        assert!(
            sheet.pixels().all(|p| p.0[..3] != [255, 0, 255]),
            "the transparency key must not be painted as a colour"
        );
        d.state(json!({"panel":"equalizer","layer":"band1.track"}))
            .unwrap();
        assert_eq!(
            d.state_sheet().unwrap().dimensions(),
            (168, 324),
            "every variant, numbered, in a grid"
        );
    }
}

/// Hue 0..1, saturation 0..1, value 0..1 to 8-bit RGB.
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

/// A real colour picker: a saturation/value field for the chosen hue, a hue
/// strip under it, and the palette already in the skin. Eight preset swatches
/// and a hex box are fine for touching up someone else's artwork and hopeless
/// for choosing a colour.
#[composable]
fn ColorChooser(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    Label("COLOUR PICKER".into(), 12., 17., 270., 14., FG);
    let current = { shared.lock().unwrap().view.color.clone() };
    let rgb = parse_color(&current).unwrap_or([255, 255, 255, 255]);
    let (h0, s0, v0) = rgb_hsv([rgb[0], rgb[1], rgb[2]]);
    // Hue is kept aside: at zero saturation or zero value every hue is the same
    // colour, and reading it back from RGB would snap the strip to red.
    let hue = cranpose_core::rememberMutableStateOf(|| h0);
    if s0 > 0.02 && v0 > 0.02 {
        let h = h0;
        if (hue.get_non_reactive() - h).abs() > 0.001 {
            cranpose_core::SideEffect(move || hue.set(h));
        }
    }
    let h = hue.get();
    // The colour in force, the size of a real swatch, next to the box that
    // types one exactly. Both used to be elsewhere: the only sign of the
    // current colour inside the picker was 11pt of grey hex at the top.
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
    // Scanned once when the picker opens: every pixel of every sheet is too much
    // work to repeat on each recomposition.
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
        "Transparent (#ff00ff)".into(),
        12.,
        82. + field_size.1 + 172.,
        200.,
        current.eq_ignore_ascii_case("#ff00ff"),
        move || state(&d, json!({"color":"#ff00ff"})),
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
    // A magnifier that has to be aimed by picking a brush and dragging a box is
    // a magnifier nobody reaches for. With nothing lifted it simply follows the
    // pointer, quantised so it re-crops every sixteen pixels rather than every
    // one.
    let at = hover.get().map(|p| {
        [
            (p[0].max(0) / 16 * 16) as u32,
            (p[1].max(0) / 16 * 16) as u32,
        ]
    });
    let (im, rect, chosen) = {
        let d = shared.lock().unwrap();
        let chosen = d.selection.is_some() || d.cluster.is_some();
        let r = d.selection.unwrap_or_else(|| match at {
            Some([x, y]) => [x.saturating_sub(40), y.saturating_sub(30), 80, 60],
            None => [0, 0, 80, 60],
        });
        (
            d.cluster
                .clone()
                .or_else(|| study::crop(&d.selected_image(), r).ok()),
            r,
            chosen,
        )
    };
    Label(
        if chosen {
            format!(
                "Source {},{} · {}×{} enlarged with no resampling",
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
    if let Some(im) = im {
        let im = if values.get() {
            study::value_view(&im)
        } else {
            im
        };
        let (w, h) = im.dimensions();
        // The magnified view fits the column rather than a fixed 348 points,
        // which used to run the picture off the right-hand edge of the panel.
        let detail_h = (room.1 - 350.).max(140.);
        let z = ((room.0 as u32 - 8) / w)
            .min(detail_h as u32 / h)
            .clamp(1, 6);
        let bitmap = ImageBitmap::from_rgba8(w, h, im.into_raw()).unwrap();
        for (scale, y, available_h) in [(1, 140., 76.), (z, 226., detail_h)] {
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
    // The study itself is read-only; these three change the lifted clipboard,
    // so they are named and separated as what they are.
    let transforms = room.1 - 96.;
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
    // Preserve the canvas guide numbers when narrowing the list.
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
    // Armed by the first press of Delete, like New blank: a plane is artwork,
    // and it used to go on one click while starting a blank skin asked twice.
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
    // The controls for the chosen plane sit at the foot of the column and the
    // list fills everything between. Pinning them at a fixed offset cut the
    // list to four rows in a seven-plane document, with nothing to say the
    // other three were there.
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
                // One line per row: a name long enough to wrap made its row
                // taller than its neighbours and the list lost its pitch.
                let name = if name.chars().count() > 22 {
                    format!("{}…", name.chars().take(21).collect::<String>())
                } else {
                    name
                };
                Box(
                    Modifier::empty().size_points(width, 38.),
                    BoxSpec::default(),
                    move || {
                        // One row per plane. A full-width switch pair under
                        // every name meant four planes filled the panel.
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
        // Delete throws a whole plane of artwork away, so it asks once first;
        // the other three are reversible rearrangements.
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
