#[cfg(test)]
mod join_tests;
mod patch;
use super::mapping::{self, Layer};
use anyhow::{bail, Context, Result};
use image::{Pixel, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
    path::Path,
};

/// Shared capacity for human edits, isolated patches, and layered projects.
const MAX_PAINT_LAYERS: usize = 64;

/// The variant in hand alone.
pub const SCOPE_CURRENT: &str = "current";
/// The variant in hand and every one after it.
pub const SCOPE_ONWARD: &str = "onward";
/// Every variant up to and including the one in hand.
pub const SCOPE_UP_TO: &str = "up-to";
/// Every variant of every target.
pub const SCOPE_ALL: &str = "all";
/// The four edit scopes, in the order the pills are drawn in.
pub const SCOPES: &[&str] = &[SCOPE_CURRENT, SCOPE_ONWARD, SCOPE_UP_TO, SCOPE_ALL];
/// What each scope is called where a person reads it.
pub fn scope_label(scope: &str) -> &'static str {
    match scope {
        SCOPE_ONWARD => "This state onward",
        SCOPE_UP_TO => "Up to this state",
        SCOPE_ALL => "All sprite states",
        _ => "This state only",
    }
}

/// One pass of a transaction: the scope, and which single step of it this pass
/// is painting.
///
/// A sweep draws the same operation once per variant with its numbers moved a
/// little each time -- a slider's twenty-eight frames are one shape whose
/// endpoint walks across the cell -- so each pass has to land in exactly one
/// variant of the scope rather than in all of them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Variants {
    pub scope: Scope,
    /// Which cell of `scope`, when a sweep is painting them one at a time.
    pub pick: Option<usize>,
}
impl From<Scope> for Variants {
    fn from(scope: Scope) -> Self {
        Variants { scope, pick: None }
    }
}
impl Variants {
    pub fn cells(self, layer: &Layer) -> Vec<[u32; 4]> {
        let cells = self.scope.cells(layer);
        match self.pick {
            Some(k) => cells.get(k).copied().into_iter().collect(),
            None => cells,
        }
    }
}

/// The operation fields a sweep may walk. Every one of them is a number the
/// rasteriser reads; `control` is the one that is a pair, and it sweeps as two
/// pairs rather than as two numbers.
const SWEEPABLE: &[&str] = &[
    "x",
    "y",
    "x2",
    "y2",
    "width",
    "height",
    "brush_size",
    "curve_bend",
    "bevel",
    "refraction",
    "opacity",
    "grain",
    "grain_size",
    "grain_seed",
    "spacing",
    "scale",
    "text_scale",
];

/// Which fields of these operations are written as `[from, to]`.
fn sweep_fields(operations: &[Value]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for op in operations {
        let Some(fields) = op.as_object() else {
            continue;
        };
        for (key, value) in fields {
            let is_pair = match (key.as_str(), value) {
                ("control", Value::Array(a)) => a.len() == 2 && a.iter().all(Value::is_array),
                (k, Value::Array(a)) => {
                    SWEEPABLE.contains(&k) && a.len() == 2 && a.iter().all(Value::is_number)
                }
                _ => false,
            };
            if is_pair && !found.contains(key) {
                found.push(key.clone());
            }
        }
    }
    found
}

/// One operation, moved to a place and stepped to a point in its sweep.
///
/// `t` runs 0..1 across the variants the transaction writes. Swept numbers land
/// on whole pixels: a curve may be fractional, but a slider frame that is half
/// a pixel along is the same picture as the one before it, and
/// `identical_variants` would rightly complain about it.
fn place_operation(op: &Value, shift: [i32; 2], t: f64) -> Value {
    let mut op = op.clone();
    let lerp = |a: f64, b: f64| (a + (b - a) * t).round();
    if let Some(fields) = op.as_object_mut() {
        for (key, value) in fields.iter_mut() {
            match (key.as_str(), &value) {
                ("control", Value::Array(a)) if a.len() == 2 && a.iter().all(Value::is_array) => {
                    let point =
                        |i: usize, j: usize| a[i].get(j).and_then(Value::as_f64).unwrap_or(0.0);
                    *value = json!([
                        lerp(point(0, 0), point(1, 0)),
                        lerp(point(0, 1), point(1, 1))
                    ]);
                }
                (k, Value::Array(a))
                    if SWEEPABLE.contains(&k) && a.len() == 2 && a.iter().all(Value::is_number) =>
                {
                    let from = a[0].as_f64().unwrap_or(0.0);
                    let to = a[1].as_f64().unwrap_or(0.0);
                    *value = json!(lerp(from, to) as i64);
                }
                _ => {}
            }
        }
    }
    if shift != [0, 0] {
        for (key, d) in [("x", shift[0]), ("y", shift[1])] {
            let at = op.get(key).and_then(Value::as_f64).unwrap_or(0.0);
            op[key] = json!(at + d as f64);
        }
        for (key, d) in [("x2", shift[0]), ("y2", shift[1])] {
            if let Some(at) = op.get(key).and_then(Value::as_f64) {
                op[key] = json!(at + d as f64);
            }
        }
        if let Some(c) = op.get_mut("control").and_then(Value::as_array_mut) {
            for (i, d) in [shift[0], shift[1]].into_iter().enumerate() {
                if let Some(v) = c.get(i).and_then(Value::as_f64) {
                    c[i] = json!(v + d as f64);
                }
            }
        }
        if let Some(points) = op.get_mut("points").and_then(Value::as_array_mut) {
            let _ = points;
        }
    }
    // Whole-pixel operations want whole numbers back after a float shift.
    for key in ["x", "y", "x2", "y2"] {
        if let Some(v) = op.get(key).and_then(Value::as_f64) {
            if v.fract() == 0.0 {
                op[key] = json!(v as i64);
            }
        }
    }
    op
}

/// Which variants of one sprite a stroke lands in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Current,
    Onward,
    UpTo,
    All,
}
impl Scope {
    pub fn named(scope: &str) -> Self {
        match scope {
            SCOPE_ONWARD => Scope::Onward,
            SCOPE_UP_TO => Scope::UpTo,
            SCOPE_ALL => Scope::All,
            _ => Scope::Current,
        }
    }
    /// The source cells this scope writes, for one sprite. `onward` and `up-to`
    /// are measured from the variant the sprite is drawn in now, which is the
    /// one the canvas is showing, so what a stroke does is what the canvas says
    /// it will do.
    pub fn cells(self, layer: &Layer) -> Vec<[u32; 4]> {
        match self {
            Scope::Current => vec![layer.source],
            Scope::All => layer.variants.clone(),
            Scope::Onward | Scope::UpTo => {
                let at = layer
                    .variants
                    .iter()
                    .position(|v| *v == layer.source)
                    .unwrap_or(0);
                match self {
                    Scope::Onward => layer.variants[at..].to_vec(),
                    _ => layer.variants[..=at].to_vec(),
                }
            }
        }
    }
}

/// The tool panels, by name, in the one vocabulary both layouts answer to.
///
/// The desktop calls them Drawing tools, Painting layers, Sprite targets,
/// Sprite rectangles, Skin atlases, Edit history, Pixel study, Skin options
/// and the colour picker; the touch layout calls its six Tools, Layers, Parts,
/// States, Sheets and Files. A name that a layout does not have simply opens
/// nothing there.
pub const DRAWERS: &[&str] = &[
    "none",
    "tools",
    "layers",
    "targets",
    "rectangles",
    "atlases",
    "history",
    "study",
    "options",
    "picker",
    "files",
    "states",
];

/// The composed skin over one rectangle. `at` takes surface coordinates, so a
/// caller that asked for a small region reads it with the same numbers it
/// would have read a whole render with, and gets `None` outside it rather than
/// a wrong pixel from the wrong place.
pub(super) struct Patch {
    pub(super) image: RgbaImage,
    origin: (i32, i32),
}

impl Patch {
    pub(super) fn at(&self, x: i32, y: i32) -> Option<[u8; 4]> {
        let (ox, oy) = self.origin;
        if x < ox || y < oy {
            return None;
        }
        self.image
            .get_pixel_checked((x - ox) as u32, (y - oy) as u32)
            .map(|p| p.0)
    }
}

/// A sheet and the painting planes over it, as `composite_images` would leave
/// it but answered one pixel at a time.
struct SheetStack<'a> {
    base: &'a RgbaImage,
    planes: Vec<(Option<&'a RgbaImage>, u8, bool)>,
}

impl SheetStack<'_> {
    fn at(&self, x: u32, y: u32) -> Option<Rgba<u8>> {
        let mut out = *self.base.get_pixel_checked(x, y)?;
        // What `clip_below` masks against is the plane immediately below --
        // the base sheet for the lowest one -- after that plane's own clip and
        // opacity have been applied, which is what `effective_planes` walks.
        let mut below = out.0[3];
        for (image, opacity, clip_below) in &self.planes {
            let Some(pixel) = image.and_then(|im| im.get_pixel_checked(x, y)) else {
                below = 0;
                continue;
            };
            let mut pixel = *pixel;
            let alpha = (pixel.0[3] as u32 * *opacity as u32 + 127) / 255;
            pixel.0[3] = if *clip_below {
                ((alpha * below as u32 + 127) / 255) as u8
            } else {
                alpha as u8
            };
            out.blend(&pixel);
            below = pixel.0[3];
        }
        Some(out)
    }
}

/// One pixel of one atlas sheet: the sheet's position in `images`, then x, y.
type AtlasPixel = (usize, u32, u32);

/// A deterministic clump of noise for one pixel, smoothed across a lattice.
///
/// Paper is not flat, and a flat rectangle of kraft reads as plastic. Noise
/// sampled per pixel is invisible from any distance a skin is looked at -- the
/// first attempt read as suede -- while clumps of two or three pixels read as
/// paper immediately, so the value is taken on a lattice and averaged with its
/// two neighbours. Integer hashing throughout: the same skin recipe has to
/// produce the same bytes on every machine.
fn grain_at(x: i32, y: i32, size: i32, seed: i64, amplitude: f64) -> f64 {
    fn value(cx: i32, cy: i32, seed: i64) -> f64 {
        let mut h = (cx as i64)
            .wrapping_mul(0x27d4_eb2d)
            .wrapping_add((cy as i64).wrapping_mul(0x1656_67b1))
            .wrapping_add(seed.wrapping_mul(0x2545_f491));
        h ^= h >> 15;
        h = h.wrapping_mul(0x2545_f491_4f6c_dd1d);
        h ^= h >> 17;
        // -1.0 ..= 1.0, from the top bits only: the low ones of a multiply
        // hash are the least mixed.
        ((h >> 24) & 0xffff) as f64 / 32767.5 - 1.0
    }
    let size = size.max(1);
    let (cx, cy) = (x.div_euclid(size), y.div_euclid(size));
    let mixed =
        (value(cx, cy, seed) * 2.0 + value(cx + 1, cy, seed) + value(cx, cy + 1, seed)) / 4.0;
    mixed * amplitude
}

/// The colour a transaction put there, and the canvas pixel that sent it.
type AtlasInk = ([u8; 4], [i32; 2]);
type AtlasWrite = (AtlasPixel, [u8; 4], [i32; 2]);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct View {
    pub panel: String,
    pub sheet: String,
    pub layer: String,
    /// Empty selects the topmost sprite; otherwise paint every selected footprint.
    pub layers: Vec<String>,
    pub zoom: u32,
    pub preview_playlist_height: u32,
    pub presentation: bool,
    pub color: String,
    pub brush_size: u32,
    pub brush: String,
    pub curve_bend: i32,
    /// Paper grain and a bakeable opacity, held on the view like every other
    /// brush setting so a mouse stroke and an MCP operation get the same
    /// material. An operation may still override either for itself.
    pub grain: u32,
    pub grain_size: u32,
    pub opacity: u32,
    pub clean_corners: bool,
    /// A gradient between the brush colour and a second one, along the shape's
    /// own axis. The engine has taken exact palette ramps from the start and
    /// nothing in either panel could ask for one, so every gradient in every
    /// skin so far came from a recipe -- which meant a flat rectangle was the
    /// only thing a person drawing by hand could make.
    pub ramp_to: Option<String>,
    /// "down" or "across".
    pub ramp_axis: String,
    /// The glass lens' own two numbers. They are 1..128 and 0..32 over MCP and
    /// were fixed at a drag-derived bevel and a refraction of four for a hand
    /// stroke, so a person got exactly one glass. Zero keeps the old
    /// behaviour: take the bevel from the height of the drag.
    pub bevel: u32,
    pub refraction: u32,
    /// What the text brush writes, in which of the two faces, at what scale
    /// and at what letter spacing.
    ///
    /// Spacing is the one of the four that decides whether a word *fits*. The
    /// `text` operation has taken it from the day it landed and the brush did
    /// not, so it was fixed at one for every hand and for every operation that
    /// did not name it: `STEREO` in the small face is six cells at five
    /// pixels each, twenty-nine pixels of ink, and the stereo lamp is
    /// twenty-nine pixels wide. At spacing zero it is twenty-three and sits in
    /// the cell with room either side. Without this the only way to label that
    /// lamp was to carry a glyph table in a build script, which is the mistake
    /// the small face was added to stop.
    pub text: String,
    pub face: String,
    pub text_scale: u32,
    pub text_spacing: i32,
    /// Which tool panel is open, shared by both layouts.
    ///
    /// It is on the document rather than in each layout's own local state so
    /// that a panel can be opened by a caller that has no pointer: every
    /// capability this editor grows has to appear in both panels, and until
    /// this existed the only way to see that it had was to be a person sitting
    /// in front of one of them. An agent could add a control and screenshot
    /// everything except the drawer it was in.
    pub drawer: String,
    pub filled: bool,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub grid: bool,
    pub alpha_lock: bool,
    pub mask_colors: Vec<String>,
    /// Which variants of each target a stroke lands in: `current`, `onward`,
    /// `up-to` or `all`. It was a bool, and a bool is the wrong shape for the
    /// thing it is for -- a twenty-eight frame slider whose frames differ by
    /// one object is neither one frame nor all of them, and drawing one cost
    /// twenty-eight transactions. `onward` and `up-to` run from the variant in
    /// hand to an end, which is the shape every slider's artwork actually has.
    pub states: String,
    /// How many copies a Stamp drag places, evenly along the drag. The pointer's
    /// half of `at`: a sheet is a grid of cells that mostly hold the same
    /// drawing, and lifting it once and dragging across them is how a hand does
    /// what `at` does from a socket.
    pub stamp_repeat: u32,
    /// A Stamp drag places one copy per variant of the edit scope, stepping
    /// along the drag. The pointer's half of a swept `[from, to]`: lift a tail,
    /// set the scope to All, and drag from where frame 0 wants it to where
    /// frame 27 does.
    pub stamp_sweep: bool,
    pub pressed: bool,
    pub active: bool,
    pub volume: u8,
    pub balance: u8,
    pub position: u8,
    pub scroll: u8,
    pub eq: [u8; 11],
    pub digit: u8,
    pub playback: u8,
    pub guides: bool,
    pub clip: Option<[u32; 4]>,
    pub paint_layer: Option<String>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            // A fresh View addresses the main window, which is what the
            // per-window addressing tests construct one for. Every path that
            // hands a document to a *user* -- the GUI's, and MCP's -- puts it
            // on the joined canvas with `open_on_whole_skin` instead.
            panel: "main".into(),
            sheet: "text.bmp".into(),
            layer: "auto".into(),
            layers: vec![],
            zoom: 3,
            preview_playlist_height: 145,
            presentation: false,
            color: "#ffffff".into(),
            brush_size: 1,
            brush: "pencil".into(),
            curve_bend: 35,
            grain: 0,
            grain_size: 2,
            opacity: 255,
            clean_corners: false,
            ramp_to: None,
            ramp_axis: "down".into(),
            bevel: 0,
            refraction: 4,
            text: "CATAMP".into(),
            face: "5x7".into(),
            text_scale: 1,
            text_spacing: 1,
            drawer: "tools".into(),
            filled: false,
            mirror_x: false,
            mirror_y: false,
            grid: true,
            alpha_lock: false,
            mask_colors: vec![],
            states: SCOPE_CURRENT.into(),
            stamp_repeat: 1,
            stamp_sweep: false,
            pressed: false,
            active: true,
            volume: 20,
            balance: 14,
            position: 0,
            scroll: 0,
            eq: [14; 11],
            digit: 0,
            playback: 0,
            guides: false,
            clip: None,
            paint_layer: None,
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct PaintLayer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: u8,
    pub clip_below: bool,
    images: BTreeMap<String, RgbaImage>,
}
#[derive(Clone, PartialEq)]
struct Snapshot {
    planes: Vec<PaintLayer>,
    images: BTreeMap<String, RgbaImage>,
    files: BTreeMap<String, Vec<u8>>,
}
#[derive(Clone)]
struct HistoryItem {
    snapshot: Snapshot,
    label: String,
    source: String,
}
pub struct Document {
    pub planes: Vec<PaintLayer>,
    pub view: View,
    pub revision: u64,
    pub message: String,
    pub path: Option<String>,
    pub dirty: bool,
    images: BTreeMap<String, RgbaImage>,
    files: BTreeMap<String, Vec<u8>>,
    undo: Vec<HistoryItem>,
    redo: Vec<HistoryItem>,
    saved: Snapshot,
    stroke: Option<Snapshot>,
    discarded_history: usize,
    pub selection: Option<[u32; 4]>,
    pub cluster: Option<RgbaImage>,
    /// The last dry run, waiting to be handed back as an image. A preview is
    /// the whole surface as the operations would leave it; the document itself
    /// has already been put back.
    pub preview: Option<RgbaImage>,
    /// Attempted Auto pixels with no bitmap source (for example classic list fill).
    unmapped_pixels: BTreeSet<[i32; 2]>,
    /// Atlas pixels the stroke in progress has changed. Reported when it ends,
    /// because a stroke can be entirely correct and entirely invisible -- white
    /// on pale artwork, or the magenta transparency key -- and silence then
    /// reads as a broken editor.
    stroke_pixels: usize,
    /// Canvas bounds of everything written since the last checkpoint, so a
    /// caller can check a stroke landed where it meant to without rendering.
    painted_bounds: Option<[i32; 4]>,
    /// Pixels a stroke could not reach because no chosen sprite covers them.
    clipped_pixels: usize,
    /// Atlas pixels this transaction has written, with the colour and the canvas
    /// pixel that wrote them. Sprites share source cells -- the four timer
    /// digits are one cell of `numbers.bmp`, the playlist top is one tile drawn
    /// nine times -- so a stroke crossing them lands on top of itself and the
    /// last colour wins in every position at once. That used to be reported as
    /// a clean write.
    atlas_writes: BTreeMap<AtlasPixel, AtlasInk>,
    /// The sheet box one operation wrote, so a transaction of ten thousand
    /// can say which one crossed out of the cell it was aimed at.
    operation_box: Option<(usize, [u32; 4])>,
    /// Where this operation blended with the transparency key.
    ///
    /// `#ff00ff` erases everywhere else in this engine, and `opacity`, an
    /// `image`'s alpha and `material: "glass"` all read it as a colour: a warm
    /// glow laid into a cleared sprite cell comes out as brown mud and a glass
    /// jar drawn in one comes out hot pink, the cell quietly stops being
    /// transparent, and every other report is clean. Nobody has ever wanted a
    /// colour blended toward magenta.
    keyed_blend: Vec<[i32; 2]>,
    overwrites: usize,
    overwrite_note: Option<String>,
    /// Sprites this transaction wrote into, and how many variants of each. Only
    /// collected when the scope is wider than the variant in hand, because that
    /// is the only case where the answer is not "one".
    scope_writes: BTreeMap<String, usize>,
    /// Ink that landed in a control's own part, with the variant it landed in,
    /// so the transaction can say afterwards whether the control's other part
    /// is drawn on top of it in that same state.
    covered_probe: Vec<(String, usize, [i32; 2])>,
    /// Sprite ids that share a control with another sprite -- a slider's track
    /// and its thumb. Recomputed per transaction, because it is the only way
    /// the per-pixel check can be a set lookup.
    control_parts: BTreeSet<String>,
}
/// The WCAG relative luminance of a colour.
fn relative_luminance(c: [u8; 4]) -> f64 {
    let channel = |v: u8| {
        let s = v as f64 / 255.;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2])
}

/// How well one colour reads on another, to one decimal place.
///
/// Four and a half is comfortable at the size a skin is looked at; below three
/// is a readout you have to hunt for, and below two is one that is not there.
/// One implementation, because the export check and `studio_pixel` answering
/// for an arbitrary rectangle are the same question asked twice.
fn contrast_ratio(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (x, y) = (relative_luminance(a), relative_luminance(b));
    ((x.max(y) + 0.05) / (x.min(y) + 0.05) * 10.).round() / 10.
}

impl Document {
    /// A new classic atlas set with zero inherited artwork or metadata.
    pub fn blank() -> Self {
        let mut images = BTreeMap::new();
        let mut files = BTreeMap::new();
        for (name, w, h) in [
            ("main.bmp", 275, 115),
            ("titlebar.bmp", 344, 87),
            ("cbuttons.bmp", 136, 36),
            ("posbar.bmp", 307, 10),
            ("shufrep.bmp", 92, 85),
            ("volume.bmp", 68, 433),
            ("balance.bmp", 68, 433),
            ("playpaus.bmp", 42, 9),
            ("monoster.bmp", 56, 24),
            ("numbers.bmp", 99, 13),
            ("eqmain.bmp", 275, 315),
            ("pledit.bmp", 280, 190),
            ("text.bmp", 155, 18),
        ] {
            images.insert(
                name.to_string(),
                RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0])),
            );
            files.insert(name.to_string(), vec![]);
        }
        let saved = Snapshot {
            planes: vec![],
            images: images.clone(),
            files: files.clone(),
        };
        Self {
            planes: vec![],
            view: View::default(),
            revision: 0,
            message: "Blank skin. Every sprite starts transparent.".into(),
            path: None,
            dirty: false,
            images,
            files,
            undo: vec![],
            redo: vec![],
            saved,
            stroke: None,
            discarded_history: 0,
            selection: None,
            cluster: None,
            unmapped_pixels: BTreeSet::new(),
            stroke_pixels: 0,
            painted_bounds: None,
            clipped_pixels: 0,
            atlas_writes: BTreeMap::new(),
            operation_box: None,
            keyed_blend: Vec::new(),
            preview: None,
            overwrites: 0,
            overwrite_note: None,
            scope_writes: BTreeMap::new(),
            covered_probe: Vec::new(),
            control_parts: BTreeSet::new(),
        }
    }
    pub fn open(bytes: &[u8], path: Option<String>) -> Result<Self> {
        // Reuse the production validator before accepting a document.
        crate::winamp::skin::load_skin(bytes).map_err(|e| anyhow::anyhow!("{e:#}"))?;
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
        let mut files = BTreeMap::new();
        let mut images = BTreeMap::new();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let name = entry
                .name()
                .replace('\\', "/")
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)?;
            if name.ends_with(".bmp") {
                let mut image = image::load_from_memory(&data)?.to_rgba8();
                for p in image.pixels_mut() {
                    if p.0[..3] == [255, 0, 255] {
                        p.0[3] = 0;
                    }
                }
                images.insert(name.clone(), image);
            }
            files.insert(name, data);
        }
        Ok(Self {
            planes: vec![],
            view: View::default(),
            revision: 0,
            message: "Draw on assembled bitmap surfaces. Shared tiles repeat; classic list fill has no bitmap source.".into(),
            path,
            dirty: false,
            saved: Snapshot {
                planes: vec![],
                images: images.clone(),
                files: files.clone(),
            },
            discarded_history: 0,
            selection: None,
            cluster: None,
            unmapped_pixels: BTreeSet::new(),
            stroke_pixels: 0,
            painted_bounds: None,
            clipped_pixels: 0,
            atlas_writes: BTreeMap::new(),
            operation_box: None,
            keyed_blend: Vec::new(),
            preview: None,
            overwrites: 0,
            overwrite_note: None,
            scope_writes: BTreeMap::new(),
            covered_probe: Vec::new(),
            control_parts: BTreeSet::new(),
            stroke: None,
            images,
            files,
            undo: vec![],
            redo: vec![],
        })
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            planes: self.planes.clone(),
            images: self.images.clone(),
            files: self.files.clone(),
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.images = s.images;
        self.planes = s.planes;
        if self
            .view
            .paint_layer
            .as_ref()
            .is_some_and(|id| !self.planes.iter().any(|p| &p.id == id))
        {
            self.view.paint_layer = None;
        }
        self.files = s.files;
        if self.view.panel == "atlas" && !self.images.contains_key(&self.view.sheet) {
            self.view.sheet = "main.bmp".into();
        }
        let available = self.layers();
        self.view
            .layers
            .retain(|id| available.iter().any(|l| &l.id == id));
        if self.view.layer != "auto" && !available.iter().any(|l| l.id == self.view.layer) {
            self.view.layer = self
                .view
                .layers
                .first()
                .cloned()
                .unwrap_or_else(|| "auto".into());
        }
        self.changed();
    }
    fn changed(&mut self) {
        self.revision += 1;
        self.dirty = self.snapshot() != self.saved;
    }
    fn record(&mut self, snapshot: Snapshot, label: String, source: &str) {
        self.undo.push(HistoryItem {
            snapshot,
            label,
            source: source.into(),
        });
        if self.undo.len() > 32 {
            self.undo.remove(0);
            self.discarded_history += 1;
        }
        self.redo.clear();
    }
    pub fn checkpoint(&mut self) {
        self.finish_stroke();
        self.unmapped_pixels.clear();
        self.stroke_pixels = 0;
        self.painted_bounds = None;
        self.clipped_pixels = 0;
        self.forget_atlas_writes();
        self.stroke = Some(self.snapshot());
    }
    pub fn finish_stroke(&mut self) {
        if self.view.brush == "lift" && self.stroke.is_some() {
            if let Some(rect) = self.selection {
                let _ = self.capture_cluster(rect);
            }
        }
        if let Some(before) = self.stroke.take() {
            if before != self.snapshot() {
                let target = match self.view.layers.len() {
                    0 => "Auto".into(),
                    1 => self.view.layers[0].clone(),
                    n => format!("{n} layers"),
                };
                self.record(
                    before,
                    format!("{} · {} · {target}", self.view.brush, self.view.panel),
                    "Human",
                );
                self.message = format!(
                    "{} · {} · {target} · {} px",
                    self.view.brush, self.view.panel, self.stroke_pixels
                );
                self.revision += 1;
            } else {
                let target = match self.view.layers.len() {
                    0 => "Auto".into(),
                    1 => self.view.layers[0].clone(),
                    n => format!("{n} layers"),
                };
                self.message = format!(
                    "{} · {} · {target} changed no pixels — that colour is already there, or the layer is hidden or locked",
                    self.view.brush, self.view.panel
                );
                self.revision += 1;
            }
            if self.clipped_pixels > 0 {
                self.message = format!(
                    "{} · {} px landed, {} fell outside the chosen sprites",
                    self.view.brush, self.stroke_pixels, self.clipped_pixels
                );
                self.revision += 1;
            }
            if self.overwrites > 0 {
                self.message = format!(
                    "{} · {} px landed on a shared source cell twice — the last colour wins in every position it is drawn",
                    self.view.brush, self.overwrites
                );
                self.revision += 1;
            }
            if !self.unmapped_pixels.is_empty() {
                self.message = format!(
                    "{} pixels have no bitmap source; the classic playlist fill is a palette colour, not artwork — turn on “List canvas” to paint there",
                    self.unmapped_pixels.len()
                );
                self.revision += 1;
            }
        }
    }
    pub fn undo(&mut self) -> bool {
        self.finish_stroke();
        if let Some(item) = self.undo.pop() {
            self.redo.push(HistoryItem {
                snapshot: self.snapshot(),
                label: item.label.clone(),
                source: item.source,
            });
            self.restore(item.snapshot);
            self.message = format!("Undo · {}", item.label);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        self.finish_stroke();
        if let Some(item) = self.redo.pop() {
            self.undo.push(HistoryItem {
                snapshot: self.snapshot(),
                label: item.label.clone(),
                source: item.source,
            });
            self.restore(item.snapshot);
            self.message = format!("Redo · {}", item.label);
            true
        } else {
            false
        }
    }
    /// How many steps back and forward there are. The editor greys its Undo
    /// and Redo out when there are none, and says how many there are, rather
    /// than offering two buttons that look identical whether or not they work.
    pub(super) fn history_depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
    pub fn history(&self) -> Value {
        let mut entries = vec![
            json!({"cursor":0,"label":if self.discarded_history == 0 {"Opened skin"} else {"Earlier retained state"},"source":"","current":self.undo.is_empty()}),
        ];
        for (i, item) in self.undo.iter().chain(self.redo.iter().rev()).enumerate() {
            entries.push(json!({"cursor":i+1,"label":item.label,"source":item.source,"current":i+1==self.undo.len(),"undone":i>=self.undo.len()}));
        }
        json!({"cursor":self.undo.len(),"limit":32,"discarded":self.discarded_history,"entries":entries})
    }
    pub fn history_goto(&mut self, cursor: usize) -> Result<Value> {
        self.finish_stroke();
        if cursor > self.undo.len() + self.redo.len() {
            bail!("History cursor out of range");
        }
        while self.undo.len() > cursor {
            self.undo();
        }
        while self.undo.len() < cursor {
            self.redo();
        }
        Ok(self.history())
    }
    pub fn state(&mut self, patch: Value) -> Result<Value> {
        self.finish_stroke();
        let mut value = serde_json::to_value(&self.view)?;
        let object = value.as_object_mut().context("view")?;
        for (k, v) in patch.as_object().context("state must be an object")? {
            // The bool `states` grew out of. Scripts and the sidebar still set
            // it, and both ends of it still mean what they meant.
            if k == "all_states" {
                let on = v
                    .as_bool()
                    .with_context(|| format!("all_states is true or false (got {v})"))?;
                object.insert(
                    "states".into(),
                    json!(if on { SCOPE_ALL } else { SCOPE_CURRENT }),
                );
                continue;
            }
            if !object.contains_key(k) {
                bail!("Unknown state field: {k}");
            }
            object.insert(k.clone(), v.clone());
        }
        let mut view: View = serde_json::from_value(value)?;
        if (view.panel != self.view.panel || view.sheet != self.view.sheet)
            && patch.get("clip").is_none()
        {
            view.clip = None;
        }
        if patch.get("layers").is_some() {
            let mut seen = BTreeSet::new();
            view.layers.retain(|id| seen.insert(id.clone()));
            view.layer = view
                .layers
                .first()
                .cloned()
                .unwrap_or_else(|| "auto".into());
        } else if patch.get("layer").is_some() {
            view.layers = if view.layer == "auto" {
                vec![]
            } else {
                vec![view.layer.clone()]
            };
        } else if view.panel != self.view.panel {
            view.layers.clear();
            view.layer = "auto".into();
        }
        let available = self.layers_for(&view);
        for id in &view.layers {
            if !available.iter().any(|l| &l.id == id) {
                bail!("Unknown layer {id}");
            }
        }
        if !["main", "equalizer", "playlist", "atlas", "canvas"].contains(&view.panel.as_str()) {
            bail!("panel must be main, equalizer, playlist, atlas, or canvas");
        }
        if view.panel == "atlas" && !self.images.contains_key(&view.sheet) {
            bail!("Unknown atlas {}", view.sheet);
        }
        // One refusal listing every range at once told a caller which ranges
        // exist and not which field it had got wrong, or what it had sent --
        // and the field is the whole of what it needs to fix it.
        for (name, value, low, high) in [
            (
                "preview_playlist_height",
                view.preview_playlist_height,
                145,
                522,
            ),
            ("zoom", view.zoom, 1, 8),
            ("volume", u32::from(view.volume), 0, 27),
            ("balance", u32::from(view.balance), 0, 27),
            ("position", u32::from(view.position), 0, 27),
            ("scroll", u32::from(view.scroll), 0, 27),
            ("digit", u32::from(view.digit), 0, 9),
            ("playback", u32::from(view.playback), 0, 2),
        ] {
            anyhow::ensure!(
                (low..=high).contains(&value),
                "{name} is {low}..{high} (got {value})"
            );
        }
        for (band, value) in view.eq.iter().enumerate() {
            anyhow::ensure!(*value <= 27, "eq[{band}] is 0..27 (got {value})");
        }
        anyhow::ensure!(
            (1..=32).contains(&view.brush_size),
            "brush_size must be 1..32"
        );
        anyhow::ensure!(view.grain <= 64, "grain is an amplitude 0..64");
        if let Some(to) = &view.ramp_to {
            parse_color(to).context("ramp_to is the colour a gradient runs to")?;
        }
        anyhow::ensure!(
            ["down", "across"].contains(&view.ramp_axis.as_str()),
            "ramp_axis is down or across (got {})",
            view.ramp_axis
        );
        anyhow::ensure!(
            view.bevel <= 128,
            "bevel is 0..128 native pixels, 0 to take it from the drag (got {})",
            view.bevel
        );
        anyhow::ensure!(
            view.refraction <= 32,
            "refraction is 0..32 native pixels (got {})",
            view.refraction
        );
        anyhow::ensure!(
            ["5x7", "small"].contains(&view.face.as_str()),
            "face is 5x7 or small (got {})",
            view.face
        );
        anyhow::ensure!(
            (1..=8).contains(&view.text_scale),
            "text_scale is 1..8 (got {})",
            view.text_scale
        );
        anyhow::ensure!(
            (-2..=8).contains(&view.text_spacing),
            "text_spacing is -2..8 (got {})",
            view.text_spacing
        );
        anyhow::ensure!(
            (1..=64).contains(&view.stamp_repeat),
            "stamp_repeat is 1..64 (got {})",
            view.stamp_repeat
        );
        anyhow::ensure!(
            SCOPES.contains(&view.states.as_str()),
            "states is one of {} (got {})",
            SCOPES.join(", "),
            view.states
        );
        anyhow::ensure!(
            DRAWERS.contains(&view.drawer.as_str()),
            "drawer is one of {} (got {})",
            DRAWERS.join(", "),
            view.drawer
        );
        anyhow::ensure!(view.text.len() <= 256, "text is at most 256 characters");
        anyhow::ensure!(
            (1..=16).contains(&view.grain_size),
            "grain_size is a lattice 1..16 pixels"
        );
        anyhow::ensure!((1..=255).contains(&view.opacity), "opacity is 1..255");
        anyhow::ensure!(
            [
                "pencil", "line", "rect", "ellipse", "lift", "stamp", "glass", "curve", "tuft",
                "text",
            ]
            .contains(&view.brush.as_str()),
            "unknown brush"
        );
        anyhow::ensure!(
            (-100..=100).contains(&view.curve_bend),
            "curve_bend must be -100..100"
        );
        if let Some(id) = &view.paint_layer {
            anyhow::ensure!(
                self.planes.iter().any(|p| &p.id == id),
                "Unknown paint layer"
            );
        }
        if let Some([x, y, w, h]) = view.clip {
            anyhow::ensure!(
                w > 0 && h > 0 && x.checked_add(w).is_some() && y.checked_add(h).is_some(),
                "Invalid clip rectangle"
            );
        }
        parse_color(&view.color)?;
        anyhow::ensure!(view.mask_colors.len() <= 256, "At most 256 mask colors");
        for c in &view.mask_colors {
            parse_color(c)?;
        }
        // A sheet with something surprising about it says so the moment it is
        // opened. The note used to arrive only in a draw result, which is one
        // stroke after it would have been useful.
        let opened = self.view.panel != "atlas" || self.view.sheet != view.sheet;
        self.view = view;
        if opened && self.view.panel == "atlas" {
            // Always this call's own note, never the document's last message.
            // A sheet with nothing surprising about it used to leave the
            // message alone, so opening `main.bmp` answered with whatever the
            // call before it had said -- for a recipe that had just turned on
            // three sheets, opening one answered with a paragraph about the
            // other three, which reads exactly like the note this field is for.
            let (w, h) = self
                .images
                .get(&self.view.sheet)
                .map(|im| im.dimensions())
                .unwrap_or((0, 0));
            self.message = match Self::sheet_note(&self.view.sheet) {
                Some(note) => format!("{} · {note}", self.view.sheet),
                None => format!("{} · {w}x{h}", self.view.sheet),
            };
        }
        self.revision += 1;
        Ok(self.brief())
    }
    /// Everything about the document. The explicit read, so it carries the
    /// sheet list as well.
    pub fn status(&self) -> Value {
        let mut value = self.brief();
        value["sheets"] = json!(self.sheets());
        value
    }
    /// What a call that only touched the view answers with.
    ///
    /// The sprite catalogue used to ride along here, and so did the sheet list,
    /// and every call that touched the view -- a colour, a zoom, a cropped read
    /// -- carried all sixty-odd sprites and their 28 slider variants back with
    /// it: eighteen kilobytes to set a brush colour. The sprites have two
    /// panels of their own -- studio_targets says which exist,
    /// studio_rectangles says where each variant lives -- and the sheet list
    /// changes only when a sheet is added, so studio_status keeps it.
    pub fn brief(&self) -> Value {
        let mut view = serde_json::to_value(&self.view).unwrap_or_else(|_| json!({}));
        // Answered as well as `states`, so a script written when this was a
        // bool still reads a true answer out of it rather than nothing.
        view["all_states"] = json!(self.view.states == SCOPE_ALL);
        json!({"path":self.path,"revision":self.revision,"dirty":self.dirty,"view":view,"undo":self.undo.len(),"redo":self.redo.len(),"message":self.message,"canvas":self.canvas_size(),"surface":self.surface(),"sprites":self.layers().len(),"paint_layers":self.paint_layer_info()})
    }
    /// Which surface a stroke's coordinates mean right now. The assembled
    /// canvas and a single sheet share one pencil and one history, so a draw
    /// aimed at the wrong one still succeeds -- somewhere else.
    pub fn surface(&self) -> String {
        match self.view.panel.as_str() {
            "atlas" => format!("atlas {}", self.view.sheet),
            "canvas" => "canvas".into(),
            panel => panel.into(),
        }
    }
    /// What a sheet is for, when it is not what it looks like.
    ///
    /// `text.bmp` looks exactly like the classic glyph sheet and is not one
    /// here: Cranamp sets every readout in its own 5x7 face and opens this
    /// sheet only to sample one colour from it. Drawing thirty-one legible
    /// glyphs into it is a day's work that changes nothing but that colour.
    pub fn sheet_note(sheet: &str) -> Option<&'static str> {
        match sheet {
            "text.bmp" => Some(
                "text.bmp is not drawn. Cranamp sets titles and readouts in its \
                 own 5x7 face and reads this sheet only to sample the display \
                 ink: the most common opaque colour, or the second most common \
                 when more than two thirds of the sheet is opaque. Paint it in \
                 the colour the readouts should be.",
            ),
            "plbg.bmp" => Some(
                "The player draws one track row every 11 pixels from the top of \
                 this sheet, and the selection strip is one row of the same \
                 height. 243x203 covers a tall playlist exactly; a taller one \
                 tiles this sheet from the top without stretching it, so a \
                 pattern whose period does not divide 203 steps at the seam.",
            ),
            "plselection.bmp" => Some(
                "One track row, 243x11, drawn under the selected row's text. It \
                 has to stay clear of the playlist ink: Cranamp writes the row's \
                 own colours over it, from PLEDIT.TXT.",
            ),
            "titlebar.bmp" => Some(
                "Most of this sheet is classic shade-mode artwork Cranamp does \
                 not draw. Only the two 275x14 title rows and the four 9x9 \
                 window buttons are sampled; the rest is reported as unsampled.",
            ),
            _ => None,
        }
    }
    /// What a sheet that has just come into existence holds.
    ///
    /// Three skin options add a drawing surface rather than change a setting,
    /// and a new surface that nothing names is a surface nobody finds.
    pub fn new_sheet_note(sheet: &str) -> &'static str {
        match sheet {
            "eqhandles.bmp" => {
                "eleven 14x25 handles, one per band, normal above pressed. \
                 Equalizer travel is limited to 38 pixels while these exist, \
                 because the handle itself takes 25 of the 63-pixel groove. \
                 The eleven band *tracks* are untouched and still share one set \
                 of 28 cells, so nothing that is true of only one band can be \
                 said in its groove -- this sheet is the only place eleven \
                 bands can differ from each other."
            }
            "plbg.bmp" => {
                "the ground under the track list, one row every 11 pixels, tiled \
                 from the top without stretching when the list is taller."
            }
            "plselection.bmp" => "one 243x11 track row, drawn under the selected row's text.",
            _ => "a new drawing surface",
        }
    }
    /// Which pixels of a sheet any sprite samples, in any panel, in any state.
    ///
    /// A sheet is not a picture: it is a bag of cells, and the space between
    /// them is never drawn. `pledit.bmp` column 125 falls between the two
    /// footer flaps and `titlebar.bmp` is mostly shade-mode bars Cranamp does
    /// not use, so a band painted straight across either one quietly loses
    /// part of itself.
    /// The parts of a sheet no sprite ever samples, as rectangles.
    ///
    /// A sheet is a bag of cells and the space between them is never drawn:
    /// `pledit.bmp` column 125 falls between the two footer flaps, so a band
    /// painted straight across the sheet loses a column and a button laid over
    /// it lands a pixel out of step with its own hit area. `unsampled_pixels`
    /// reports that after the ink is down, which is one stroke later than it is
    /// useful; this is the same mask asked before drawing into it.
    pub fn sheet_gaps(&self, sheet: &str) -> Result<Value> {
        let (mask, width) = self
            .sampled_mask(sheet)
            .with_context(|| format!("no sheet called {sheet}"))?;
        let height = mask.len() as u32 / width.max(1);
        if !mask.iter().any(|sampled| *sampled) {
            return Ok(json!({"sheet":sheet,"size":[width,height],
                             "never_drawn":true,"gaps":[],"of":0,
                             "note":Self::sheet_note(sheet)}));
        }
        // Maximal horizontal runs, then merged downward while they line up, so
        // a column nothing draws is one rectangle rather than thirty-eight.
        let mut rows: Vec<Vec<(u32, u32)>> = Vec::with_capacity(height as usize);
        for y in 0..height {
            let mut runs = Vec::new();
            let mut x = 0;
            while x < width {
                if mask[(y * width + x) as usize] {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < width && !mask[(y * width + x) as usize] {
                    x += 1;
                }
                runs.push((start, x - start));
            }
            rows.push(runs);
        }
        let mut gaps: Vec<[u32; 4]> = Vec::new();
        let mut open: Vec<(u32, u32, u32)> = Vec::new(); // x, w, top
        for (y, runs) in rows.iter().enumerate() {
            let y = y as u32;
            open.retain(|(x, w, top)| {
                if runs.contains(&(*x, *w)) {
                    true
                } else {
                    gaps.push([*x, *top, *w, y - top]);
                    false
                }
            });
            for run in runs {
                if !open.iter().any(|(x, w, _)| (*x, *w) == *run) {
                    open.push((run.0, run.1, y));
                }
            }
        }
        for (x, w, top) in open {
            gaps.push([x, top, w, height - top]);
        }
        gaps.sort_by_key(|g| (g[1], g[0]));
        let of = gaps.iter().map(|g| (g[2] * g[3]) as u64).sum::<u64>();
        gaps.truncate(64);
        Ok(
            json!({"sheet":sheet,"size":[width,height],"gaps":gaps,"of":of,
                  "note":Self::sheet_note(sheet)}),
        )
    }
    fn sampled_mask(&self, sheet: &str) -> Option<(Vec<bool>, u32)> {
        let (w, h) = self.images.get(sheet)?.dimensions();
        let mut mask = vec![false; (w as usize) * (h as usize)];
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        // "Will anything ever show this pixel" is a question about the skin,
        // not about the preview. The playlist background tiles from the top of
        // a 243x203 sheet and the preview was 145 pixels tall, so painting the
        // whole sheet -- which is the correct thing to do -- reported 28,188
        // pixels as ink nothing will ever show. At the tallest playlist the
        // player uses all of it.
        view.preview_playlist_height = 522;
        for layer in self.layers_for(&view) {
            if layer.sheet != sheet {
                continue;
            }
            for rect in &layer.variants {
                for y in rect[1]..(rect[1] + rect[3]).min(h) {
                    for x in rect[0]..(rect[0] + rect[2]).min(w) {
                        mask[(y * w + x) as usize] = true;
                    }
                }
            }
        }
        Some((mask, w))
    }
    pub fn layers(&self) -> Vec<Layer> {
        self.layers_for(&self.view)
    }
    /// Put a freshly loaded document on the only drawing surface the editor
    /// has.
    ///
    /// The GUI did this after every document swap of its own and MCP did it
    /// after none, so `studio_new` and `studio_open` handed back a retired
    /// single-window panel: a canvas 275x115 instead of 275x377, a sprite
    /// catalogue with 29 of the 81 sprites in it, and a `surface` that is
    /// neither of the two values the schema promises. A skin drawn from New
    /// blank over MCP is the documented way to start one.
    pub fn open_on_whole_skin(&mut self) {
        self.view.panel = "canvas".into();
        self.view.layer = "auto".into();
        self.view.layers.clear();
        self.view.zoom = self.view.zoom.clamp(1, 4);
        // Paint onto the top of the picture, not underneath it. A layered
        // project composites its painting planes over the base atlases, so a
        // document that arrives with planes and no selection would take every
        // stroke into the atlases below them -- landing correctly, recorded in
        // history, and visible only in the gaps where no plane covers the art.
        if self.view.paint_layer.is_none() {
            self.view.paint_layer = self
                .planes
                .iter()
                .rev()
                .find(|plane| plane.visible && !plane.locked)
                .map(|plane| plane.id.clone());
        }
    }
    /// Every sprite the skin has, whatever surface the view is on.
    ///
    /// The catalogue is a property of the skin and not of the open sheet. With
    /// `studio_atlas` holding one BMP, `layers()` answers with a single
    /// pseudo-sprite called `sheet` -- so "where do this slider's twenty-eight
    /// frames live" could not be asked from the one place the answer is needed,
    /// which is a recipe drawing that sheet at its own coordinates. It had to
    /// leave the sheet to ask, and leaving the sheet is what it was avoiding.
    pub fn skin_layers(&self) -> Vec<Layer> {
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        self.layers_for(&view)
    }
    fn layers_for(&self, view: &View) -> Vec<Layer> {
        if view.panel == "atlas" {
            return self
                .images
                .get(&view.sheet)
                .map(|im| {
                    let rect = [0, 0, im.width(), im.height()];
                    vec![Layer {
                        id: "sheet".into(),
                        sheet: view.sheet.clone(),
                        source: rect,
                        destination: rect,
                        variants: vec![rect],
                        labels: vec!["always".into()],
                    }]
                })
                .unwrap_or_default();
        }
        if view.panel == "canvas" {
            let mut all = Vec::new();
            for (panel, offset) in [("main", 0), ("equalizer", 116), ("playlist", 232)] {
                let mut panel_view = view.clone();
                panel_view.panel = panel.into();
                let panel_layers = self.layers_for(&panel_view);
                for mut layer in mapping::native_panel_layers(
                    panel_layers,
                    panel,
                    view.preview_playlist_height,
                    view.scroll,
                ) {
                    layer.id = format!("{panel}.{}", layer.id);
                    layer.destination[1] += offset;
                    all.push(layer);
                }
            }
            // The host's docking row is an exact edge copy, not a resized
            // background. Its inverse mapping intentionally aliases row 114.
            all.push(Layer {
                id: "main.docking.edge".into(),
                sheet: "main.bmp".into(),
                source: [0, 114, 275, 1],
                destination: [0, 115, 275, 1],
                variants: vec![[0, 114, 275, 1]],
                labels: vec!["always · the same row as main's last".into()],
            });
            return all;
        }
        let mut layers = mapping::layers(view, self.layout());
        if view.panel == "playlist" && self.images.contains_key("plselection.bmp") {
            layers.push(Layer {
                id: "list.selection".into(),
                sheet: "plselection.bmp".into(),
                source: [0, 0, 243, 11],
                destination: [12, 21, 243, 11],
                variants: vec![[0, 0, 243, 11]],
                labels: vec!["always · one track row".into()],
            });
        }
        if view.panel == "equalizer" && self.images.contains_key("eqhandles.bmp") {
            for i in 0..11 {
                if let Some(layer) = layers.iter_mut().find(|l| l.id == format!("band{i}.thumb")) {
                    layer.sheet = "eqhandles.bmp".into();
                    layer.variants = vec![[i * 14, 0, 14, 25], [i * 14, 25, 14, 25]];
                    layer.source = layer.variants[usize::from(view.pressed)];
                    layer.destination[0] -= 1;
                    layer.destination[2] = 14;
                    layer.destination[3] = 25;
                }
            }
        }
        layers
            .into_iter()
            .filter(|layer| self.images.contains_key(&layer.sheet))
            .collect()
    }
    pub fn sheets(&self) -> Vec<(String, u32, u32)> {
        self.images
            .iter()
            .map(|(n, im)| (n.clone(), im.width(), im.height()))
            .collect()
    }
    pub(super) fn canvas_size(&self) -> (u32, u32) {
        self.canvas_size_for(&self.view)
    }
    fn canvas_size_for(&self, view: &View) -> (u32, u32) {
        if view.panel == "canvas" {
            (275, 232 + view.preview_playlist_height)
        } else if view.panel == "atlas" {
            self.images
                .get(&view.sheet)
                .map(|im| im.dimensions())
                .unwrap_or((1, 1))
        } else {
            mapping::size(&view.panel)
        }
    }
    /// The whole skin, whatever surface the editor has open.
    ///
    /// The always-on preview is a picture of the *skin*, not of the workspace:
    /// with one sheet open the canvas is that sheet, and a corner of the window
    /// that claims to show how everything looks has to keep showing how
    /// everything looks.
    pub fn skin_render(&self) -> RgbaImage {
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        view.layers.clear();
        let (w, h) = self.canvas_size_for(&view);
        self.render_patch_for(&view, [0, 0, w as i32, h as i32])
            .image
    }
    fn forget_atlas_writes(&mut self) {
        self.atlas_writes.clear();
        self.overwrites = 0;
        self.overwrite_note = None;
    }
    /// Record where a transaction put each atlas pixel, and notice when it puts
    /// two different colours in the same one.
    fn note_atlas_writes(&mut self, touched: Vec<AtlasWrite>) {
        for (key, colour, at) in touched {
            self.operation_box = match self.operation_box {
                Some((sheet, [x0, y0, w, h])) if sheet == key.0 => {
                    let (nx, ny) = (x0.min(key.1), y0.min(key.2));
                    Some((
                        sheet,
                        [
                            nx,
                            ny,
                            (x0 + w).max(key.1 + 1) - nx,
                            (y0 + h).max(key.2 + 1) - ny,
                        ],
                    ))
                }
                Some(other) => Some(other),
                None => Some((key.0, [key.1, key.2, 1, 1])),
            };
            if let Some((previous, from)) = self.atlas_writes.get(&key) {
                // Only a *different* canvas pixel landing in the same source
                // cell is the trap worth naming: one tile drawn nine times,
                // four timer digits sharing one cell. Two operations painting
                // the same canvas pixel -- fill a panel, then draw on it -- is
                // ordinary drawing, and counting it buried the real signal.
                if *previous != colour && *from != at {
                    self.overwrites += 1;
                    if self.overwrite_note.is_none() {
                        let from = *from;
                        let sheet = self
                            .images
                            .keys()
                            .nth(key.0)
                            .cloned()
                            .unwrap_or_else(|| "?".into());
                        self.overwrite_note = Some(format!(
                            "canvas {:?} and {:?} both write {sheet} at {:?}: \
                             that source cell is shared, so the last colour wins \
                             everywhere it is drawn. Name one target in `layers`.",
                            from,
                            at,
                            [key.1, key.2]
                        ));
                    }
                }
            }
            if self.atlas_writes.len() < 400_000 {
                self.atlas_writes.insert(key, (colour, at));
            }
        }
    }
    /// Variants of one sprite that have come out as the same picture.
    ///
    /// Twenty-eight frames of a slider track, drawn by a recipe that forgot to
    /// offset each frame by its own y, all land on frame 0. Every report says
    /// the transaction succeeded: `bounds` is sensible, nothing was clipped,
    /// nothing was unsampled, and `overwrites` is a canvas-to-source measure
    /// that does not apply to a sheet open on its own. The only thing wrong is
    /// that twenty-seven frames are now the same picture, so that is the thing
    /// to say.
    ///
    /// An empty cell is not a duplicate, it is an unpainted cell, so fully
    /// transparent variants are left out -- otherwise the first stroke on a
    /// blank sheet reports every sprite on it.
    fn identical_variants(&self) -> Vec<Value> {
        if self.atlas_writes.is_empty() {
            return Vec::new();
        }
        let names: Vec<String> = self.images.keys().cloned().collect();
        // The bounding box of what this transaction wrote, per sheet. A box is
        // enough to ask "did this touch that sprite" and costs one comparison
        // per variant rather than one per written pixel.
        let mut touched: BTreeMap<usize, [u32; 4]> = BTreeMap::new();
        for (sheet, x, y) in self.atlas_writes.keys() {
            let box_ = touched.entry(*sheet).or_insert([*x, *y, 1, 1]);
            let (x0, y0) = (box_[0].min(*x), box_[1].min(*y));
            let (x1, y1) = (
                (box_[0] + box_[2]).max(*x + 1),
                (box_[1] + box_[3]).max(*y + 1),
            );
            *box_ = [x0, y0, x1 - x0, y1 - y0];
        }
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        let mut out: Vec<Value> = Vec::new();
        for layer in self.layers_for(&view) {
            if layer.variants.len() < 2 {
                continue;
            }
            let Some(index) = names.iter().position(|n| *n == layer.sheet) else {
                continue;
            };
            let Some(box_) = touched.get(&index) else {
                continue;
            };
            // The box is the cheap filter and not the answer. One transaction
            // usually draws every cell a sheet has, scattered over the whole of
            // it, so the union of its writes is most of the sheet and a box
            // test reports every sprite on it: the equalizer's PRESETS plate
            // and its close key, drawn together, made a box that swallowed the
            // ON and AUTO cells in between and named both as having duplicate
            // variants the stroke had not touched a pixel of. Confirm against
            // the writes themselves, which the transaction is already holding.
            if !layer.variants.iter().any(|r| {
                super::guides::intersection(*r, *box_).is_some()
                    && (r[1]..r[1] + r[3]).any(|y| {
                        (r[0]..r[0] + r[2]).any(|x| self.atlas_writes.contains_key(&(index, x, y)))
                    })
            }) {
                continue;
            }
            let Some(image) = self.images.get(&layer.sheet) else {
                continue;
            };
            let mut same: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
            for (i, r) in layer.variants.iter().enumerate() {
                let mut pixels = Vec::with_capacity((r[2] * r[3] * 4) as usize);
                for y in r[1]..(r[1] + r[3]).min(image.height()) {
                    for x in r[0]..(r[0] + r[2]).min(image.width()) {
                        pixels.extend_from_slice(&image.get_pixel(x, y).0);
                    }
                }
                if pixels.is_empty() || pixels.chunks(4).all(|p| p[3] == 0) {
                    continue;
                }
                same.entry(pixels).or_default().push(i);
            }
            let groups: Vec<Vec<usize>> = same.into_values().filter(|g| g.len() > 1).collect();
            if groups.is_empty() {
                continue;
            }
            // Sprites that share one set of source cells share the answer too.
            // The equalizer's eleven bands all draw their groove from the same
            // rectangle, so one duplicate frame was reported eleven times, each
            // with its own copy of all twenty-eight labels -- four and a half
            // kilobytes of JSON for one fact.
            if let Some(shared) = out.iter_mut().find(|e| {
                e["sheet"] == json!(layer.sheet)
                    && e["groups"] == json!(groups)
                    && e["variants"] == json!(layer.variants)
            }) {
                if let Some(ids) = shared["also"].as_array_mut() {
                    ids.push(json!(layer.id));
                }
                continue;
            }
            out.push(json!({"id":layer.id,"sheet":layer.sheet,
                            "of":layer.variants.len(),"groups":groups,
                            "labels":layer.labels,"variants":layer.variants,
                            "also":[]}));
        }
        for entry in out.iter_mut() {
            if let Some(object) = entry.as_object_mut() {
                object.remove("variants");
                if object["also"].as_array().map(Vec::is_empty).unwrap_or(true) {
                    object.remove("also");
                }
            }
        }
        out
    }
    pub fn render(&self) -> RgbaImage {
        let (w, h) = self.canvas_size();
        self.render_patch([0, 0, w as i32, h as i32]).image
    }
    /// The composed surface over one rectangle, and nothing outside it.
    ///
    /// `opacity`, a half-transparent `image` and `material: "glass"` all have
    /// to read the artwork as it stands, and the artwork as it stands is
    /// thirteen sheets composed under every painting plane and then assembled
    /// into the canvas. Composing all of it to answer for one pixel of marine
    /// snow costs exactly what composing it for the whole playlist costs, and
    /// a skin whose light is made of many small blended marks pays that once
    /// per mark: Catamp Salvage's caustic net is 886 one-pixel blends and took
    /// nineteen seconds to write 1,537 pixels, all of it re-rendering the
    /// whole skin 886 times. A blended operation reads the artwork only where
    /// it is about to write.
    ///
    /// Freshness is unchanged -- this is composed per operation like the whole
    /// render was, so a glow laid over a picture still blends into the picture
    /// that is there rather than into the one that was.
    pub(super) fn render_patch(&self, bounds: [i32; 4]) -> Patch {
        self.render_patch_for(&self.view, bounds)
    }
    fn render_patch_for(&self, view: &View, bounds: [i32; 4]) -> Patch {
        let (w, h) = self.canvas_size_for(view);
        let x0 = bounds[0].clamp(0, w as i32) as u32;
        let y0 = bounds[1].clamp(0, h as i32) as u32;
        let x1 = bounds[0]
            .saturating_add(bounds[2])
            .clamp(0, w as i32)
            .max(x0 as i32) as u32;
        let y1 = bounds[1]
            .saturating_add(bounds[3])
            .clamp(0, h as i32)
            .max(y0 as i32) as u32;
        let ground = if view.panel == "playlist" {
            self.files
                .get("pledit.txt")
                .map(|bytes| crate::winamp::skin::parse_pledit_txt(bytes).normal_bg)
                .unwrap_or([0, 0, 0, 255])
        } else {
            [30, 34, 42, 255]
        };
        let origin = (x0 as i32, y0 as i32);
        if x1 <= x0 || y1 <= y0 {
            return Patch {
                image: RgbaImage::new(0, 0),
                origin,
            };
        }
        let mut image = RgbaImage::from_pixel(x1 - x0, y1 - y0, Rgba(ground));
        if view.panel == "canvas" {
            let bg = self
                .files
                .get("pledit.txt")
                .map(|b| crate::winamp::skin::parse_pledit_txt(b).normal_bg)
                .unwrap_or([0, 0, 0, 255]);
            for y in y0.max(252)..y1.min(h - 38) {
                for x in x0.max(12)..x1.min(255) {
                    image.put_pixel(x - x0, y - y0, Rgba(bg));
                }
            }
        }
        for layer in self.layers_for(view) {
            let [dx, dy, dw, dh] = layer.destination;
            let (lx0, ly0) = (dx.max(x0), dy.max(y0));
            let (lx1, ly1) = ((dx + dw).min(w).min(x1), (dy + dh).min(h).min(y1));
            if lx1 <= lx0 || ly1 <= ly0 {
                continue;
            }
            let Some(stack) = self.sheet_stack(&layer.sheet) else {
                continue;
            };
            for y in ly0..ly1 {
                for x in lx0..lx1 {
                    let Some((sx, sy)) = layer.map(x, y) else {
                        continue;
                    };
                    if let Some(pixel) = stack.at(layer.source[0] + sx, layer.source[1] + sy) {
                        if pixel.0[3] > 0
                            && (view.panel == "atlas" || pixel.0[..3] != [255, 0, 255])
                        {
                            image.put_pixel(x - x0, y - y0, pixel);
                        }
                    }
                }
            }
        }
        Patch { image, origin }
    }
    /// One sheet's base bitmap and every painting plane's copy of it, resolved
    /// once so that composing a pixel is an index rather than a map lookup per
    /// plane per pixel.
    fn sheet_stack(&self, sheet: &str) -> Option<SheetStack<'_>> {
        Some(SheetStack {
            base: self.images.get(sheet)?,
            planes: self
                .planes
                .iter()
                .map(|plane| {
                    (
                        plane.images.get(sheet),
                        if plane.visible { plane.opacity } else { 0 },
                        plane.clip_below,
                    )
                })
                .collect(),
        })
    }
    /// Transparent composition of only selected sprites; Auto uses the visible canvas.
    pub fn selected_image(&self) -> RgbaImage {
        let composite = self.composite_images();
        if self.view.layers.is_empty() {
            return self.render();
        }
        let (w, h) = self.canvas_size();
        let mut im = RgbaImage::new(w, h);
        for l in self
            .layers()
            .iter()
            .filter(|l| self.view.layers.contains(&l.id))
        {
            let Some(sheet) = composite.get(&l.sheet) else {
                continue;
            };
            let [dx, dy, dw, dh] = l.destination;
            for y in dy..(dy + dh).min(h) {
                for x in dx..(dx + dw).min(w) {
                    if let Some((sx, sy)) = l.map(x, y) {
                        if let Some(p) = sheet.get_pixel_checked(l.source[0] + sx, l.source[1] + sy)
                        {
                            if p[3] > 0 && (self.view.panel == "atlas" || p.0[..3] != [255, 0, 255])
                            {
                                im.put_pixel(x, y, *p);
                            }
                        }
                    }
                }
            }
        }
        im
    }
    pub fn capture_cluster(&mut self, rect: [u32; 4]) -> Result<Value> {
        let im = self.selected_image();
        let [x, y, w, h] = rect;
        anyhow::ensure!(
            w > 0
                && h > 0
                && w <= 512
                && h <= 512
                && x.checked_add(w).is_some_and(|v| v <= im.width())
                && y.checked_add(h).is_some_and(|v| v <= im.height()),
            "Cluster rectangle must fit canvas and be at most 512×512"
        );
        self.cluster = Some(image::imageops::crop_imm(&im, x, y, w, h).to_image());
        self.selection = Some(rect);
        self.message =
            format!("Lifted {w}×{h} native pixels. Choose Stamp; no skin pixels changed.");
        self.revision += 1;
        self.cluster_data()
    }
    pub fn cluster_data(&self) -> Result<Value> {
        let im = self
            .cluster
            .as_ref()
            .context("Lift a pixel cluster first")?;
        let mut palette = BTreeMap::new();
        let mut colors = BTreeMap::new();
        let mut rows = Vec::new();
        for y in 0..im.height() {
            let mut row = String::new();
            for x in 0..im.width() {
                let p = im.get_pixel(x, y).0;
                if p[3] == 0 {
                    row.push('.');
                    continue;
                }
                let next = char::from_u32(0xE000 + colors.len() as u32)
                    .context("Too many cluster colours")?;
                let key = *colors.entry(p).or_insert(next);
                palette.insert(
                    key.to_string(),
                    format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]),
                );
                row.push(key);
            }
            rows.push(row);
        }
        Ok(
            json!({"width":im.width(),"height":im.height(),"rows":rows,"palette":palette,"revision":self.revision}),
        )
    }
    pub fn transform_cluster(&mut self, flip_x: bool, flip_y: bool, turns: u32) -> Result<Value> {
        anyhow::ensure!(turns < 4, "quarter_turns is 0..3");
        let mut im = self.cluster.clone().context("Lift a pixel cluster first")?;
        if flip_x {
            im = image::imageops::flip_horizontal(&im);
        }
        if flip_y {
            im = image::imageops::flip_vertical(&im);
        }
        for _ in 0..turns {
            im = image::imageops::rotate90(&im);
        }
        self.cluster = Some(im);
        self.revision += 1;
        self.cluster_data()
    }
    fn paint_cluster(
        &mut self,
        im: &RgbaImage,
        at: [i32; 2],
        layer: &str,
        all: Variants,
        layers: &[Layer],
    ) -> Result<usize> {
        let mut n = 0;
        for (x, y, p) in im.enumerate_pixels() {
            if p[3] > 0 {
                let q = [at[0] + x as i32, at[1] + y as i32];
                n += self.paint_line_untracked(q, q, p.0, layer, all, layers)?;
            }
        }
        Ok(n)
    }
    pub fn editor_render(&self) -> RgbaImage {
        let mut im = self.render();
        // Sprite rectangles are no longer drawn into the picture. Outlining
        // every cell in the artwork's own pixels buried the artwork under the
        // hints that were supposed to point at it, and the outlines scaled with
        // the zoom because they were pixels. The editor draws the one under the
        // pointer as an overlay instead; see `GuideHint`.
        if self.view.brush == "lift" {
            if let Some([x, y, w, h]) = self.selection {
                for yy in y..y + h {
                    for xx in x..x + w {
                        if (xx == x || yy == y || xx == x + w - 1 || yy == y + h - 1)
                            && xx < im.width()
                            && yy < im.height()
                        {
                            im.put_pixel(
                                xx,
                                yy,
                                Rgba(if (xx + yy) % 2 == 0 {
                                    [255, 255, 255, 255]
                                } else {
                                    [16, 24, 36, 255]
                                }),
                            );
                        }
                    }
                }
            }
        }
        im
    }
    /// What the artwork is at a place, and whether a colour would read on it.
    ///
    /// Three things, and they used to be one. `hits` settles a mapping: every
    /// sprite under the pixel, where each keeps it, and what shares it.
    /// `ground` is the artwork itself, averaged over the rectangle's opaque
    /// pixels -- a skin whose whole grammar is "how far is this from a flame"
    /// has to know what a cat is standing in front of before it can put one
    /// down, and answering that by rendering a crop and looking at it is a file
    /// and two round trips for a number the document is already holding.
    /// `contrast` is the export check's own arithmetic pointed anywhere: that
    /// check covers the eight readouts Cranamp writes and nothing else, and a
    /// skin is full of hand-drawn marks that have to read on hand-drawn
    /// artwork.
    ///
    /// The coordinates mean the surface the view is on, like every stroke, and
    /// the answer says which -- asked about a canvas pixel while a sheet was
    /// open, this used to answer `{"hits": []}`, which reads as "no sprite
    /// there" and means "not on the surface you have open".
    pub fn inspect(&self, rect: [u32; 4], ink: Option<[u8; 4]>) -> Value {
        let [x, y, w, h] = rect;
        let composite = self.composite_images();
        let hits:Vec<Value>=self.layers().iter().rev().filter_map(|l|l.map(x,y).map(|(sx,sy)|json!({"layer":l.id,"sheet":l.sheet,"atlas_pixel":[l.source[0]+sx,l.source[1]+sy],"local_pixel":[sx,sy],"variants":l.variants.len(),"shared_or_stretched":l.stretched(),"rgba":composite.get(&l.sheet).and_then(|im|im.get_pixel_checked(l.source[0]+sx,l.source[1]+sy)).map(|p|p.0)}))).collect();
        let mut out = json!({"canvas_pixel":[x,y],"surface":self.surface(),"hits":hits});
        if w != 1 || h != 1 {
            out["rect"] = json!(rect);
        }
        let hex = |c: [u8; 4]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        let ground = self.ground_under(rect);
        if let Some(ground) = ground {
            out["ground"] = json!(hex(ground));
            if let Some(ink) = ink {
                let r = contrast_ratio(ink, ground);
                out["ink"] = json!(hex(ink));
                out["contrast"] = json!(r);
                out["readable"] = json!(r >= 3.0);
            }
        }
        // Nothing there is an answer, and it is a different answer from "no
        // sprite there". Say what was actually looked in.
        if out["hits"].as_array().is_some_and(Vec::is_empty) && ground.is_none() {
            let (sw, sh) = if self.view.panel == "atlas" {
                self.images
                    .get(&self.view.sheet)
                    .map(|i| i.dimensions())
                    .unwrap_or((0, 0))
            } else {
                self.canvas_size()
            };
            out["nothing_at"] = json!(format!(
                "{} is {sw}x{sh} and holds no artwork at {x},{y}",
                self.surface()
            ));
        }
        out
    }

    /// The sprite a state sheet would show, or `None` when nothing names one.
    ///
    /// There is no such thing as the state sheet of "every sprite under the
    /// brush", and the arbitrary sprite this used to fall back to meant the
    /// first press of the button always showed something nobody asked for.
    ///
    /// A named sprite is looked up in the whole skin, whatever surface is
    /// open. A sprite's variants are a fact about the skin rather than about
    /// the surface: a recipe that draws twenty-eight slider frames draws them
    /// in `studio_atlas` at the sheet's own coordinates, and that was the one
    /// place it could not then ask whether they had come out twenty-eight
    /// different pictures -- the catalogue there is a single pseudo-sprite
    /// called `sheet`, so `studio_states` answered with the whole sheet as one
    /// variant and said nothing was repeated. Leaving the sheet to ask is the
    /// one thing having the sheet open was avoiding. `studio_targets` and
    /// `studio_pixel` were both fixed for this same sentence.
    pub(super) fn state_sheet_layer(&self) -> Option<Layer> {
        let named = |wanted: &str| self.layers().into_iter().find(|l| l.id == wanted);
        // One sprite picked by hand is a target as much as a named one is.
        if self.view.layers.len() == 1 {
            return named(&self.view.layers[0]);
        }
        if self.view.layer == "auto" || self.view.layer.is_empty() {
            return None;
        }
        named(&self.view.layer)
    }
    /// The sprite a state sheet is being asked for: the one this call names,
    /// or the one the view has chosen.
    ///
    /// A named one is looked up in the whole skin whatever surface is open,
    /// because a sprite's variants are a fact about the skin rather than about
    /// the surface. A recipe draws twenty-eight slider frames in
    /// `studio_atlas` at the sheet's own coordinates, because that is where
    /// the frames are -- and that was the one place it could not then ask
    /// whether they had come out twenty-eight different pictures. Opening a
    /// sheet puts the catalogue on a single pseudo-sprite called `sheet`, so
    /// the answer was the whole sheet as one variant with nothing repeated: a
    /// clean report about a question nobody asked. Leaving the sheet to ask
    /// was the one thing having the sheet open was avoiding, and
    /// `studio_targets` and `studio_pixel` were both fixed for this same
    /// sentence.
    pub(super) fn state_sheet_asked(&self, id: Option<&str>) -> Result<Layer> {
        let Some(id) = id else {
            return self.state_sheet_layer().context(
                // Naming only the GUI panel leaves an MCP caller with the right
                // diagnosis and no call to make: the panel is a tool.
                "A state sheet is one sprite's variants, so choose one sprite first: \
                 studio_states {\"id\":\"main.play\"}, studio_targets \
                 {\"solo\":\"main.play\"}, or the editor's Sprite targets panel",
            );
        };
        let wanted = id.to_ascii_lowercase();
        let skin = self.skin_layers();
        let mut hit: Vec<Layer> = skin
            .iter()
            .filter(|l| l.id.eq_ignore_ascii_case(id))
            .cloned()
            .collect();
        if hit.is_empty() {
            hit = skin
                .iter()
                .filter(|l| l.id.to_ascii_lowercase().contains(&wanted))
                .cloned()
                .collect();
        }
        match hit.len() {
            1 => Ok(hit.remove(0)),
            0 => bail!(
                "no sprite matching {id} in this skin's {} -- studio_targets lists them",
                skin.len()
            ),
            _ => bail!(
                "{id} matches {} sprites ({}); name one exactly",
                hit.len(),
                hit.iter()
                    .take(4)
                    .map(|l| l.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
    /// For each variant, the earliest variant it is the same picture as.
    ///
    /// A fully transparent cell is left out: it is unpainted, not a copy.
    fn repeated_variants(source: &RgbaImage, variants: &[[u32; 4]]) -> BTreeMap<usize, usize> {
        let cell = |r: &[u32; 4]| -> Option<Vec<u8>> {
            let mut pixels = Vec::with_capacity((r[2] * r[3] * 4) as usize);
            for y in r[1]..(r[1] + r[3]).min(source.height()) {
                for x in r[0]..(r[0] + r[2]).min(source.width()) {
                    pixels.extend_from_slice(&source.get_pixel(x, y).0);
                }
            }
            (!pixels.is_empty() && pixels.chunks(4).any(|p| p[3] > 0)).then_some(pixels)
        };
        let mut first: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        let mut twins = BTreeMap::new();
        for (i, r) in variants.iter().enumerate() {
            let Some(pixels) = cell(r) else { continue };
            match first.get(&pixels) {
                Some(earlier) => {
                    twins.insert(i, *earlier);
                }
                None => {
                    first.insert(pixels, i);
                }
            }
        }
        twins
    }
    /// What each variant of the chosen sprite is, and how far it is from the
    /// one before it.
    ///
    /// Two variants that differ by a hundredth of their pixels are a control
    /// whose pressed state nobody will see, and two that differ by none are a
    /// frame that was never drawn. Neither is visible on a contact sheet.
    pub fn variant_differences(&self, id: Option<&str>) -> Result<Value> {
        let layer = self.state_sheet_asked(id)?;
        let composite = self.composite_images();
        let source = &composite[&layer.sheet];
        let twins = Self::repeated_variants(source, &layer.variants);
        let at = |r: &[u32; 4], x: u32, y: u32| -> [u8; 4] {
            if r[0] + x < source.width() && r[1] + y < source.height() {
                source.get_pixel(r[0] + x, r[1] + y).0
            } else {
                [0; 4]
            }
        };
        let mut out = Vec::new();
        for (i, r) in layer.variants.iter().enumerate() {
            let mut painted = 0u32;
            for y in 0..r[3] {
                for x in 0..r[2] {
                    painted += u32::from(at(r, x, y)[3] > 0);
                }
            }
            let mut entry = json!({"index":i,"label":layer.labels.get(i),
                                   "rect":r,"painted_pixels":painted});
            if let Some(previous) = i.checked_sub(1).and_then(|p| layer.variants.get(p)) {
                let (mut differing, mut delta) = (0u32, 0i32);
                for y in 0..r[3].min(previous[3]) {
                    for x in 0..r[2].min(previous[2]) {
                        let (a, b) = (at(r, x, y), at(previous, x, y));
                        if a != b {
                            differing += 1;
                            for c in 0..4 {
                                delta = delta.max((a[c] as i32 - b[c] as i32).abs());
                            }
                        }
                    }
                }
                entry["differs_from_previous"] = json!(differing);
                entry["largest_channel_change"] = json!(delta);
            }
            if let Some(first) = twins.get(&i) {
                entry["same_picture_as"] = json!(first);
            }
            out.push(entry);
        }
        Ok(
            json!({"id":layer.id,"sheet":layer.sheet,"of":layer.variants.len(),
                  "variants":out}),
        )
    }
    /// Every variant of one sprite, laid out in a grid and numbered.
    pub fn state_sheet(&self, id: Option<&str>) -> Result<RgbaImage> {
        let layer = self.state_sheet_asked(id)?;
        let columns = layer.variants.len().clamp(1, 7) as u32;
        // A band above each cell for its number, and a margin so neighbouring
        // sprites do not read as one piece of artwork.
        const BAND: u32 = 12;
        let cw = layer.source[2] + 10;
        let ch = layer.source[3] + BAND + 6;
        let rows = (layer.variants.len() as u32).div_ceil(columns);
        let mut image = RgbaImage::from_pixel(columns * cw, rows * ch, Rgba([20, 24, 30, 255]));
        let composite = self.composite_images();
        let source = &composite[&layer.sheet];
        let twins = Self::repeated_variants(source, &layer.variants);
        for (i, r) in layer.variants.iter().enumerate() {
            let ox = (i as u32 % columns) * cw;
            let oy = (i as u32 / columns) * ch;
            // A cell ground a shade off the board, so a sprite that is mostly
            // transparent still shows where its bounds are.
            for y in 1..ch - 1 {
                for x in 1..cw - 1 {
                    image.put_pixel(ox + x, oy + y, Rgba([34, 40, 50, 255]));
                }
            }
            // The number, and -- when this cell is the same picture as an
            // earlier one -- which. A contact sheet of 28 frames that differ
            // by two pixels each looks exactly like one where 27 are copies.
            let caption = match twins.get(&i) {
                Some(first) => format!("{} = {}", i + 1, first + 1),
                None => (i + 1).to_string(),
            };
            let ink = if twins.contains_key(&i) {
                Rgba([214, 138, 104, 255])
            } else {
                Rgba([120, 146, 170, 255])
            };
            for (k, letter) in caption.chars().enumerate() {
                let Some(glyph) = crate::winamp::pixel_text::glyph(letter) else {
                    continue;
                };
                for (row, bits) in glyph.iter().enumerate() {
                    for column in 0..5 {
                        if bits & (1 << (4 - column)) == 0 {
                            continue;
                        }
                        let px = ox + 3 + k as u32 * 6 + column;
                        let py = oy + 3 + row as u32;
                        if px < ox + cw - 1 && py < oy + ch - 1 {
                            image.put_pixel(px, py, ink);
                        }
                    }
                }
            }
            for y in 0..r[3] {
                for x in 0..r[2] {
                    let pixel = source.get_pixel(r[0] + x, r[1] + y);
                    // The same rule the player renders by: the classic
                    // transparency key is not a colour. Copying it verbatim
                    // turned sheets like the volume track into flat magenta.
                    if pixel.0[3] > 0 && pixel.0[..3] != [255, 0, 255] {
                        image.put_pixel(ox + 5 + x, oy + BAND + 3 + y, *pixel);
                    }
                }
            }
        }
        Ok(image)
    }
    pub fn paint_line(
        &mut self,
        from: [i32; 2],
        to: [i32; 2],
        color: [u8; 4],
        layer: &str,
        all: Variants,
    ) -> Result<usize> {
        let op = json!({"op":"line","x":from[0],"y":from[1],"x2":to[0],"y2":to[1],"brush_size":self.view.brush_size});
        let count = self.paint_shape(&op, color, layer, all, &self.layers())?;
        if count > 0 {
            self.changed();
        }
        Ok(count)
    }
    fn paint_shape(
        &mut self,
        op: &Value,
        color: [u8; 4],
        layer: &str,
        all: Variants,
        layers: &[Layer],
    ) -> Result<usize> {
        let (w, h) = self.canvas_size();
        let mx = op
            .get("mirror_x")
            .and_then(Value::as_bool)
            .unwrap_or(self.view.mirror_x);
        let my = op
            .get("mirror_y")
            .and_then(Value::as_bool)
            .unwrap_or(self.view.mirror_y);
        let ramp = op
            .get("ramp")
            .map(|r| r.as_array().context("ramp is a colour array"))
            .transpose()?;
        let colors: Vec<_> = ramp
            .map(|r| {
                r.iter()
                    .map(|c| parse_color(c.as_str().context("ramp colour required")?))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        anyhow::ensure!(
            ramp.is_none() || (2..=256).contains(&colors.len()),
            "ramp needs 2..256 colours"
        );
        let axis: Option<[f64; 4]> = op
            .get("ramp_axis")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;
        anyhow::ensure!(
            colors.is_empty() || axis.is_some(),
            "ramp_axis [x1,y1,x2,y2] required"
        );
        if let Some(a) = axis {
            anyhow::ensure!(
                a.iter().all(|x| x.is_finite()) && (a[0] != a[2] || a[1] != a[3]),
                "ramp axis must be finite and nonzero"
            );
        }
        // Paper grain and a bakeable opacity. Both were being composed in an
        // image library and stamped in through the `image` operation, which
        // works and needs Pillow, so neither material could be drawn from the
        // editor itself, from Android, or from any recipe without Python.
        let grain = op
            .get("grain")
            .and_then(Value::as_f64)
            .unwrap_or(self.view.grain as f64);
        anyhow::ensure!((0.0..=64.0).contains(&grain), "grain is an amplitude 0..64");
        let grain_size = op
            .get("grain_size")
            .and_then(Value::as_i64)
            .unwrap_or(self.view.grain_size as i64) as i32;
        anyhow::ensure!(
            (1..=16).contains(&grain_size),
            "grain_size is a lattice 1..16 pixels"
        );
        let grain_seed = op.get("grain_seed").and_then(Value::as_i64).unwrap_or(0);
        let opacity = op
            .get("opacity")
            .and_then(Value::as_u64)
            .unwrap_or(self.view.opacity as u64);
        anyhow::ensure!((1..=255).contains(&opacity), "opacity is 1..255");
        let mut geometry = op.clone();
        if geometry.get("clean_corners").is_none() {
            geometry["clean_corners"] = json!(self.view.clean_corners);
        }
        let points = super::brush::rasterize(&geometry)?;
        // Composed per operation, so it cannot go stale the way the image
        // operation's backdrop once did -- and composed over the operation's
        // own bounds, because a blend reads the artwork only where it is about
        // to write it.
        let under = (opacity < 255 && !points.is_empty()).then(|| {
            let x0 = points.iter().map(|p| p[0]).min().unwrap_or(0);
            let y0 = points.iter().map(|p| p[1]).min().unwrap_or(0);
            let x1 = points.iter().map(|p| p[0]).max().unwrap_or(0);
            let y1 = points.iter().map(|p| p[1]).max().unwrap_or(0);
            self.render_patch([x0, y0, x1 - x0 + 1, y1 - y0 + 1])
        });
        let material = op.get("material").and_then(Value::as_str);
        anyhow::ensure!(
            material.is_none() || material == Some("glass"),
            "Unknown material"
        );
        let shaded = if material == Some("glass") {
            // Glass refracts what is under it, and what is under a cleared
            // sprite cell is the transparency key, so the engine's own glass
            // drawn into one comes out hot pink. Count it the same way the
            // opacity blend does rather than let it ship.
            let beneath = self.selected_image();
            for [x, y] in points.iter().copied() {
                if let Some(p) = beneath
                    .get_pixel_checked(x.max(0) as u32, y.max(0) as u32)
                    .filter(|_| x >= 0 && y >= 0)
                {
                    if p.0[3] == 0 || p.0[..3] == [255, 0, 255] {
                        self.keyed_blend.push([x, y]);
                    }
                }
            }
            Some(super::material::glass(&points, &beneath, color, op)?)
        } else {
            None
        };
        let mut count = 0;
        for [x, y] in points {
            let color = if let Some(c) = shaded.as_ref().and_then(|s| s.get(&[x, y])) {
                *c
            } else if let Some(a) = axis.filter(|_| !colors.is_empty()) {
                let dx = a[2] - a[0];
                let dy = a[3] - a[1];
                let t = (((x as f64 - a[0]) * dx + (y as f64 - a[1]) * dy) / (dx * dx + dy * dy))
                    .clamp(0., 1.);
                colors[(t * (colors.len() - 1) as f64).round() as usize]
            } else {
                color
            };
            let mut color = color;
            if grain > 0.0 {
                let g = grain_at(x, y, grain_size, grain_seed, grain);
                for channel in color.iter_mut().take(3) {
                    *channel = (*channel as f64 + g).round().clamp(0.0, 255.0) as u8;
                }
            }
            if let Some(base) = under.as_ref() {
                let a = opacity as f64 / 255.0;
                let beneath = base.at(x, y).unwrap_or([0, 0, 0, 255]);
                if beneath[3] == 0 || beneath[..3] == [255, 0, 255] {
                    self.keyed_blend.push([x, y]);
                }
                for c in 0..3 {
                    color[c] = (color[c] as f64 * a + beneath[c] as f64 * (1.0 - a))
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
                color[3] = 255;
            }
            let xs = if mx {
                vec![x, w as i32 - 1 - x]
            } else {
                vec![x]
            };
            let ys = if my {
                vec![y, h as i32 - 1 - y]
            } else {
                vec![y]
            };
            for x in xs {
                for &y in &ys {
                    count +=
                        self.paint_line_untracked([x, y], [x, y], color, layer, all, layers)?;
                }
            }
        }
        Ok(count)
    }
    pub fn shape_stroke(&mut self, from: [i32; 2], to: [i32; 2]) -> Result<usize> {
        self.unmapped_pixels.clear();
        if self.view.brush == "lift" {
            let (w, h) = self.canvas_size();
            let x = from[0].min(to[0]).clamp(0, w as i32 - 1) as u32;
            let y = from[1].min(to[1]).clamp(0, h as i32 - 1) as u32;
            let x2 = from[0].max(to[0]).clamp(0, w as i32 - 1) as u32;
            let y2 = from[1].max(to[1]).clamp(0, h as i32 - 1) as u32;
            self.selection = Some([x, y, x2 - x + 1, y2 - y + 1]);
            self.revision += 1;
            return Ok(0);
        }
        if self.view.brush == "stamp" {
            if let Some(before) = &self.stroke {
                self.images = before.images.clone();
                self.planes = before.planes.clone();
            }
            let cluster = self.cluster.clone().context("Lift a pixel cluster first")?;
            let layers = self.layers();
            let scope = Scope::named(&self.view.states);
            // One copy per variant when sweeping, otherwise as many as asked
            // for. Both walk the drag: the first copy lands where the drag
            // started and the last where it ended, so a slider's whole travel
            // is one gesture.
            let variants = self
                .view
                .layers
                .iter()
                .filter_map(|id| layers.iter().find(|l| &l.id == id))
                .map(|l| scope.cells(l).len())
                .max()
                .unwrap_or(1);
            let copies = if self.view.stamp_sweep {
                variants.max(1)
            } else {
                self.view.stamp_repeat as usize
            };
            let mut count = 0;
            for k in 0..copies.max(1) {
                let t = if copies > 1 {
                    k as f64 / (copies - 1) as f64
                } else {
                    1.0
                };
                let at = [
                    (from[0] as f64 + (to[0] - from[0]) as f64 * t).round() as i32,
                    (from[1] as f64 + (to[1] - from[1]) as f64 * t).round() as i32,
                ];
                count += self.paint_cluster(
                    &cluster,
                    at,
                    "selection",
                    Variants {
                        scope,
                        pick: (self.view.stamp_sweep && variants > 1).then_some(k),
                    },
                    &layers,
                )?;
            }
            if count > 0 {
                self.changed();
            }
            return Ok(count);
        }
        // Repaint from the checkpoint for a live, reversible shape preview.
        if let Some(before) = &self.stroke {
            self.images = before.images.clone();
            self.planes = before.planes.clone();
        }
        let v = self.view.clone();
        let mut op = if v.brush == "text" {
            // A word is placed, not dragged: the gesture's start is where it
            // begins and the rest of the drag is ignored.
            json!({"op":"text","x":from[0],"y":from[1],"text":v.text,
                   "face":v.face,"scale":v.text_scale,"spacing":v.text_spacing})
        } else if ["line", "curve", "tuft"].contains(&v.brush.as_str()) {
            json!({"op":v.brush,"x":from[0],"y":from[1],"x2":to[0],"y2":to[1],"brush_size":v.brush_size,"curve_bend":v.curve_bend})
        } else {
            json!({"op":if v.brush=="glass" {"ellipse"}else{&v.brush},"x":from[0].min(to[0]),"y":from[1].min(to[1]),"width":(to[0]-from[0]).abs()+1,"height":(to[1]-from[1]).abs()+1,"brush_size":v.brush_size,"fill":v.filled||v.brush=="glass","material":if v.brush=="glass" {Some("glass")}else{None},
                   "bevel":if v.bevel > 0 {v.bevel as f64} else {((from[1]-to[1]).abs() as f64/2.).clamp(1.,24.)},
                   "refraction":v.refraction})
        };
        // A gradient runs along the shape the gesture drew, so the axis comes
        // from the drag rather than from anything the artist has to type.
        if let Some(to_color) = v.ramp_to.as_deref() {
            if v.brush != "text" {
                let (a, b) = (parse_color(&v.color)?, parse_color(to_color)?);
                let stops: Vec<String> = (0..24)
                    .map(|i| {
                        let t = i as f64 / 23.;
                        let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t) as u8;
                        format!(
                            "#{:02x}{:02x}{:02x}",
                            mix(a[0], b[0]),
                            mix(a[1], b[1]),
                            mix(a[2], b[2])
                        )
                    })
                    .collect();
                let (x0, y0) = (from[0].min(to[0]), from[1].min(to[1]));
                let (x1, y1) = (from[0].max(to[0]), from[1].max(to[1]));
                op["ramp"] = json!(stops);
                op["ramp_axis"] = if v.ramp_axis == "across" {
                    json!([x0, y0, x1, y0])
                } else {
                    json!([x0, y0, x0, y1])
                };
            }
        }
        let count = self.paint_shape(
            &op,
            parse_color(&v.color)?,
            "selection",
            Scope::named(&v.states).into(),
            &self.layers(),
        )?;
        self.changed();
        Ok(count)
    }
    pub fn cancel_stroke(&mut self) {
        if let Some(before) = self.stroke.take() {
            self.restore(before);
            self.revision += 1;
        }
    }
    fn paint_line_untracked(
        &mut self,
        from: [i32; 2],
        to: [i32; 2],
        color: [u8; 4],
        layer: &str,
        all: Variants,
        layers: &[Layer],
    ) -> Result<usize> {
        let (w, h) = self.canvas_size();
        let active = self
            .view
            .paint_layer
            .as_ref()
            .and_then(|id| self.planes.iter().position(|p| &p.id == id));
        if let Some(i) = active {
            anyhow::ensure!(!self.planes[i].locked, "Paint layer is locked");
        }
        let mut count = 0;
        let mask = self
            .view
            .mask_colors
            .iter()
            .map(|c| parse_color(c))
            .collect::<Result<Vec<_>>>()?;
        for (x, y) in line(from, to) {
            if let Some([cx, cy, cw, ch]) = self.view.clip {
                if x < cx as i32
                    || y < cy as i32
                    || x >= cx.saturating_add(cw) as i32
                    || y >= cy.saturating_add(ch) as i32
                {
                    continue;
                }
            }

            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                continue;
            }
            let selection = if layer == "selection" {
                self.view.layers.clone()
            } else if layer == "auto" {
                vec![]
            } else {
                vec![layer.to_string()]
            };
            let hits: Vec<&Layer> = if selection.is_empty() {
                // Every sprite the brush is over, not just the one on top. The
                // joined canvas stacks a control over the window background it
                // sits on, and both are artwork the stroke is crossing: paint
                // only the top one and the illustration breaks apart the moment
                // that control moves, changes state, or is drawn somewhere else
                // from the same shared source. Selecting targets explicitly in
                // TARGET LAYERS still narrows this.
                layers
                    .iter()
                    .filter(|l| l.map(x as u32, y as u32).is_some())
                    .collect()
            } else {
                layers
                    .iter()
                    .filter(|l| selection.contains(&l.id) && l.map(x as u32, y as u32).is_some())
                    .collect()
            };
            if hits.is_empty() {
                if selection.is_empty() {
                    self.unmapped_pixels.insert([x, y]);
                } else {
                    self.clipped_pixels += 1;
                }
            }
            // Shared source pixels may be reached through multiple selected instances.
            let mut written = BTreeSet::new();
            let mut touched = Vec::new();
            for l in hits {
                let (sx, sy) = l.map(x as u32, y as u32).unwrap();
                let targets = all.cells(l);
                // A slider's track and its thumb move together: both are read
                // from the same value, so ink put at the thumb's position in
                // frame k is under the thumb in frame k, in every frame. That
                // is invisible artwork and nothing said so.
                if self.control_parts.contains(&l.id) && self.covered_probe.len() < 20000 {
                    for (index, cell) in targets.iter().enumerate() {
                        let variant = l.variants.iter().position(|v| v == cell).unwrap_or(index);
                        self.covered_probe.push((l.id.clone(), variant, [x, y]));
                    }
                }
                if all.scope != Scope::Current {
                    // The scope's own count, not this pass's. A sweep paints one
                    // variant per pass and still wrote the whole run.
                    self.scope_writes
                        .insert(l.id.clone(), all.scope.cells(l).len());
                }
                let dimensions = self
                    .images
                    .get(&l.sheet)
                    .context("missing atlas")?
                    .dimensions();
                let sheet_ix = self
                    .images
                    .keys()
                    .position(|k| *k == l.sheet)
                    .unwrap_or(usize::MAX);
                let im = if let Some(i) = active {
                    self.planes[i]
                        .images
                        .entry(l.sheet.clone())
                        .or_insert_with(|| RgbaImage::new(dimensions.0, dimensions.1))
                } else {
                    self.images.get_mut(&l.sheet).unwrap()
                };
                for r in targets {
                    if written.insert((l.sheet.clone(), r[0] + sx, r[1] + sy)) {
                        if let Some(p) = im.get_pixel_mut_checked(r[0] + sx, r[1] + sy) {
                            if (self.view.alpha_lock && p[3] == 0)
                                || (!mask.is_empty() && !mask.contains(&p.0))
                            {
                                continue;
                            }
                            touched.push(((sheet_ix, r[0] + sx, r[1] + sy), color, [x, y]));
                            if p.0 != color {
                                *p = Rgba(color);
                                count += 1;
                                self.stroke_pixels += 1;
                                self.painted_bounds = Some(match self.painted_bounds {
                                    Some([x0, y0, x1, y1]) => {
                                        [x0.min(x), y0.min(y), x1.max(x), y1.max(y)]
                                    }
                                    None => [x, y, x, y],
                                });
                            }
                        }
                    }
                }
            }
            self.note_atlas_writes(touched);
        }
        Ok(count)
    }
    pub fn draw(&mut self, args: &Value) -> Result<Value> {
        // Every draw says how long it took. Nothing in this surface did, and
        // the one thing a caller cannot see from the other end of a socket is
        // which of its own operations is expensive: a caustic net of 886
        // one-pixel blends took nineteen seconds of a three-second build and
        // was found by wall-clocking the client, after a day of assuming the
        // client was the slow half.
        let started = web_time::Instant::now();
        self.finish_stroke();
        // Per transaction, not per session. A human stroke clears these in
        // checkpoint(); an MCP caller has no equivalent, so without this every
        // draw reported the union of every draw before it.
        self.painted_bounds = None;
        self.clipped_pixels = 0;
        self.forget_atlas_writes();
        let before = self.snapshot();
        let before_revision = self.revision;
        let before_dirty = self.dirty;
        let selected: Vec<String> = if let Some(ids) = args.get("layers") {
            serde_json::from_value(ids.clone()).context("layers must be an array of layer IDs")?
        } else if let Some(layer) = args.get("layer") {
            let id = layer.as_str().context("layer must be a string")?;
            if id == "auto" {
                vec![]
            } else {
                vec![id.into()]
            }
        } else {
            self.view.layers.clone()
        };
        for id in &selected {
            if !self.layers().iter().any(|l| &l.id == id) {
                bail!("Unknown layer {id}");
            }
        }
        // `origin` moves the coordinate system onto a sprite. Absolute canvas
        // coordinates for a sprite that moves -- a slider thumb, whose
        // destination follows its own frame -- are correct only for the state
        // they were read in, and a stale one paints a second copy of the art at
        // an offset without tripping anything.
        let origin = args
            .get("origin")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let offset = match &origin {
            Some(id) => {
                let layer = self
                    .layers()
                    .into_iter()
                    .find(|l| &l.id == id)
                    .with_context(|| format!("Unknown layer {id}"))?;
                [layer.destination[0] as i32, layer.destination[1] as i32]
            }
            None => [0, 0],
        };
        let mut selected = selected;
        if let Some(id) = &origin {
            if selected.is_empty() {
                selected.push(id.clone());
            }
        }
        // `states` is the scope this transaction paints in; `all_states` is the
        // bool it grew out of and still answers to, because scripts were
        // written against it before there was anything between one and all.
        let scope = match args.get("states") {
            Some(Value::String(named)) => {
                anyhow::ensure!(
                    SCOPES.contains(&named.as_str()),
                    "states is one of {} (got {named})",
                    SCOPES.join(", ")
                );
                named.clone()
            }
            Some(other) => bail!("states is one of {} (got {other})", SCOPES.join(", ")),
            None => match args.get("all_states").and_then(Value::as_bool) {
                Some(true) => SCOPE_ALL.into(),
                Some(false) => SCOPE_CURRENT.into(),
                None => self.view.states.clone(),
            },
        };
        let base_scope = Scope::named(&scope);
        let layer = "selection";
        let operations = args
            .get("operations")
            .and_then(Value::as_array)
            .context("operations array required")?;
        // Where to run the whole operation list. One place is an ordinary
        // transaction; `at` repeats it, which is the difference between drawing
        // ten identical berths and writing a loop outside the editor to emit
        // ten copies of the same nine operations.
        let places: Vec<[i32; 2]> = match args.get("at") {
            None => vec![[0, 0]],
            Some(Value::String(word)) => {
                anyhow::ensure!(
                    word == "targets",
                    "at is a list of [x, y] offsets or \"targets\" (got {word})"
                );
                anyhow::ensure!(
                    !selected.is_empty(),
                    "at \"targets\" runs the operations once at each chosen \
                     sprite's own destination, so it needs targets: name them in \
                     layers, or give explicit [x, y] offsets"
                );
                let known = self.layers();
                selected
                    .iter()
                    .map(|id| {
                        known
                            .iter()
                            .find(|l| &l.id == id)
                            .map(|l| [l.destination[0] as i32, l.destination[1] as i32])
                            .with_context(|| format!("Unknown layer {id}"))
                    })
                    .collect::<Result<_>>()?
            }
            Some(Value::Array(list)) => {
                anyhow::ensure!(!list.is_empty(), "at is at least one [x, y] offset");
                anyhow::ensure!(list.len() <= 256, "at is at most 256 places");
                list.iter()
                    .enumerate()
                    .map(|(i, place)| {
                        let pair: [i32; 2] = serde_json::from_value(place.clone())
                            .with_context(|| format!("at[{i}] is [x, y] (got {place})"))?;
                        Ok(pair)
                    })
                    .collect::<Result<_>>()?
            }
            Some(other) => bail!("at is a list of [x, y] offsets or \"targets\" (got {other})"),
        };
        // Numbers that walk across the variants this transaction writes. A
        // slider's twenty-eight frames are one shape whose endpoint moves, and
        // spelling that as `[from, to]` is the difference between one call and
        // twenty-eight.
        let swept = sweep_fields(operations);
        let steps = if swept.is_empty() {
            1
        } else {
            anyhow::ensure!(
                !selected.is_empty(),
                "a sweep writes one variant per step, so it needs a named target: \
                 {} is swept and the targets are Auto",
                swept.join(", ")
            );
            let known = self.layers();
            let mut counts: Vec<(String, usize)> = Vec::new();
            for id in &selected {
                let layer = known
                    .iter()
                    .find(|l| &l.id == id)
                    .with_context(|| format!("Unknown layer {id}"))?;
                counts.push((id.clone(), Scope::named(&scope).cells(layer).len()));
            }
            let n = counts[0].1;
            anyhow::ensure!(
                counts.iter().all(|(_, c)| *c == n),
                "a sweep steps through one run of variants, and these do not \
                 agree: {}",
                counts
                    .iter()
                    .map(|(id, c)| format!("{id} has {c}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            anyhow::ensure!(
                n > 1,
                "a sweep needs more than one variant to walk across; \
                 {} writes {n} in scope {scope}",
                counts[0].0
            );
            n
        };
        let passes = places.len().saturating_mul(steps);
        anyhow::ensure!(
            operations.len().saturating_mul(passes) <= 10000,
            "At most 10000 operations per transaction ({} operations at {} places \
             over {steps} sweep steps is {})",
            operations.len(),
            places.len(),
            operations.len().saturating_mul(passes)
        );
        let prepared: Vec<Vec<Value>> = places
            .iter()
            .flat_map(|place| {
                let shift = [place[0] + offset[0], place[1] + offset[1]];
                (0..steps).map(move |step| {
                    let t = if steps > 1 {
                        step as f64 / (steps - 1) as f64
                    } else {
                        0.0
                    };
                    operations
                        .iter()
                        .map(|op| place_operation(op, shift, t))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mask: Vec<String> = args
            .get("mask_colors")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or_else(|| self.view.mask_colors.clone());
        anyhow::ensure!(mask.len() <= 256, "At most 256 mask colors");
        for c in &mask {
            parse_color(c)?;
        }
        let previous_mask = std::mem::replace(&mut self.view.mask_colors, mask);
        let previous_unmapped = std::mem::take(&mut self.unmapped_pixels);
        self.scope_writes.clear();
        self.covered_probe.clear();
        // Sprites with three or more segments in their id share a control with
        // whatever else has their first two: `main.balance.track` and
        // `main.balance.thumb`. Two segments is a window's own furniture, and a
        // background being under a button is not news.
        self.control_parts = {
            let known = self.layers();
            let mut by_control: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for l in &known {
                let mut parts = l.id.split('.');
                if let (Some(a), Some(b), Some(_)) = (parts.next(), parts.next(), parts.next()) {
                    by_control
                        .entry(format!("{a}.{b}"))
                        .or_default()
                        .push(l.id.clone());
                }
            }
            by_control
                .into_values()
                .filter(|ids| ids.len() > 1)
                .flatten()
                .collect()
        };
        let previous_selection = std::mem::replace(&mut self.view.layers, selected.clone());
        let paint_layers = self.layers();
        // Every cell of the open sheet, and how many places the player draws
        // it. A sheet is a bag of cells and an operation is nearly always
        // aimed at one of them; the one that is worth reporting is the one
        // that crossed out of its own cell into a cell drawn more than once,
        // because that repeats the accident everywhere the tile goes.
        let mut cells: Vec<(String, usize, [u32; 4])> = Vec::new();
        let sheet_size = self.canvas_size();
        if self.view.panel == "atlas" {
            let mut times: BTreeMap<String, usize> = BTreeMap::new();
            let layers = self.skin_layers();
            for l in &layers {
                if l.sheet == self.view.sheet {
                    *times.entry(l.id.clone()).or_default() += 1;
                }
            }
            let mut seen = BTreeSet::new();
            for l in &layers {
                if l.sheet != self.view.sheet {
                    continue;
                }
                for v in &l.variants {
                    if seen.insert((l.id.clone(), *v)) {
                        cells.push((l.id.clone(), times[&l.id], *v));
                    }
                }
            }
        }
        let mut crossed: Vec<Value> = Vec::new();
        let mut keyed: Vec<Value> = Vec::new();
        let mut count = 0;
        // Which operation is in hand, so a refusal can say so. A transaction is
        // allowed ten thousand operations; "No glyph for '@'" with no index is
        // a needle in ten thousand.
        let mut at: Option<(usize, String)> = None;
        let mut skipped: Vec<char> = Vec::new();
        let result = (|| -> Result<()> {
            for (pass, operations) in prepared.iter().enumerate() {
                // Which variant this pass paints. Without a sweep there is one
                // pass and it paints whatever the scope says; with one, each
                // pass paints exactly one step of it.
                let all = Variants {
                    scope: base_scope,
                    pick: (steps > 1).then_some(pass % steps),
                };
                for (index, op) in operations.iter().enumerate() {
                    let kind = op.get("op").and_then(Value::as_str).unwrap_or("pixel");
                    at = Some((index, kind.to_owned()));
                    self.operation_box = None;
                    self.keyed_blend.clear();
                    let color = parse_color(
                        op.get("color")
                            .and_then(Value::as_str)
                            .unwrap_or(&self.view.color),
                    )?;
                    // A stamp, a cluster, an image and a set word all land on a
                    // whole pixel. A shape does not: its geometry stays fractional
                    // until the rasteriser walks it, so reading its origin as an
                    // integer here refused a curve the engine was about to turn
                    // into f64 on its next line.
                    let placed = |key: &str| integer(op, key, 0);
                    match kind {
                        "pixel" | "line" | "rect" | "ellipse" | "path" | "curve" | "tuft" => {
                            let mut shape = op.clone();
                            shape["op"] = json!(kind);
                            if shape.get("brush_size").is_none() {
                                shape["brush_size"] = json!(self.view.brush_size);
                            }
                            if kind == "rect" && shape.get("fill").is_none() {
                                shape["fill"] = json!(true);
                            }
                            count += self.paint_shape(&shape, color, layer, all, &paint_layers)?;
                        }
                        "cluster" => {
                            let (x, y) = (placed("x")?, placed("y")?);
                            let im = self.cluster.clone().context("Lift a pixel cluster first")?;
                            count += self.paint_cluster(&im, [x, y], layer, all, &paint_layers)?;
                        }
                        "stamp" => {
                            let (x, y) = (placed("x")?, placed("y")?);
                            let rows = op
                                .get("rows")
                                .and_then(Value::as_array)
                                .context("stamp rows required")?;
                            let palette = op
                                .get("palette")
                                .and_then(Value::as_object)
                                .context("stamp palette required")?;
                            for (j, row) in rows.iter().enumerate() {
                                for (i, ch) in row
                                    .as_str()
                                    .context("stamp row string")?
                                    .chars()
                                    .enumerate()
                                {
                                    if let Some(c) =
                                        palette.get(&ch.to_string()).and_then(Value::as_str)
                                    {
                                        count += self.paint_line_untracked(
                                            [x + i as i32, y + j as i32],
                                            [x + i as i32, y + j as i32],
                                            parse_color(c)?,
                                            layer,
                                            all,
                                            &paint_layers,
                                        )?;
                                    }
                                }
                            }
                        }
                        "image" => {
                            let (x, y) = (placed("x")?, placed("y")?);
                            use base64::Engine as _;
                            let data = op
                                .get("data")
                                .and_then(Value::as_str)
                                .context("image op needs base64 PNG data")?;
                            let bytes = base64::engine::general_purpose::STANDARD
                                .decode(data.trim())
                                .context("image data is not valid base64")?;
                            let mut im = image::load_from_memory(&bytes)
                                .context("image data is not a readable PNG")?
                                .to_rgba8();
                            anyhow::ensure!(
                                im.width() <= 2048 && im.height() <= 2048,
                                "Image is at most 2048x2048"
                            );
                            // A sheet has no alpha channel to keep, so a half
                            // transparent pixel has to become an opaque blend with
                            // what it lands on or it arrives as full-strength
                            // colour: soft glows used to stamp as hard speckle.
                            if im.pixels().any(|p| p[3] > 0 && p[3] < 255) {
                                // The artwork a half-transparent image blends
                                // into, composed now rather than reused: a glow
                                // placed after the picture beneath it used to
                                // blend into the picture that was there *before*
                                // and write that back, which silently undid the
                                // work underneath it. Only the image's own
                                // footprint is composed, because that is all it
                                // reads.
                                let base = self.render_patch([
                                    x,
                                    y,
                                    im.width() as i32,
                                    im.height() as i32,
                                ]);
                                for (ix, iy, p) in im.enumerate_pixels_mut() {
                                    if p[3] == 0 || p[3] == 255 {
                                        continue;
                                    }
                                    let (bx, by) = (x + ix as i32, y + iy as i32);
                                    let under = base.at(bx, by).unwrap_or([0, 0, 0, 255]);
                                    if under[3] == 0 || under[..3] == [255, 0, 255] {
                                        self.keyed_blend.push([bx, by]);
                                    }
                                    let a = p[3] as f32 / 255.0;
                                    let mut out = [0u8; 4];
                                    for c in 0..3 {
                                        out[c] = (p[c] as f32 * a + under[c] as f32 * (1.0 - a))
                                            .round()
                                            .clamp(0.0, 255.0)
                                            as u8;
                                    }
                                    out[3] = 255;
                                    *p = Rgba(out);
                                }
                            }
                            count += self.paint_cluster(&im, [x, y], layer, all, &paint_layers)?;
                        }
                        "text" => {
                            let (x, y) = (placed("x")?, placed("y")?);
                            let body = op
                                .get("text")
                                .and_then(Value::as_str)
                                .context("text op needs text")?;
                            let scale = op
                                .get("scale")
                                .and_then(Value::as_u64)
                                .unwrap_or(1)
                                .clamp(1, 8) as i32;
                            let spacing = op
                                .get("spacing")
                                .and_then(Value::as_i64)
                                .unwrap_or(self.view.text_spacing as i64)
                                as i32;
                            // Two faces, because a classic skin has cells a word
                            // has to fit in and the 5x7 one does not: 14 pixels for
                            // an equalizer caption, 27 for the mono lamp.
                            let small = op.get("face").and_then(Value::as_str) == Some("small");
                            // Where the word starts, when the caller knows the box
                            // it has to sit in rather than the pixel it starts at.
                            // Centring a caption is (cell - ink) / 2 and the ink is
                            // (4*n + n-1) for the small face -- arithmetic every
                            // caller was carrying its own copy of, next to the
                            // `measure` call that exists to answer it.
                            let x = match op.get("align").and_then(Value::as_str) {
                                None => x,
                                Some(side) => {
                                    let width = op.get("width").and_then(Value::as_i64).context(
                                        "align needs width: the box the word is \
                                         placed in, starting at x",
                                    )? as i32;
                                    let ink = crate::winamp::pixel_text::measure(
                                        body, small, scale, spacing,
                                    )
                                    .width;
                                    match side {
                                        "left" => x,
                                        "center" | "centre" => x + (width - ink) / 2,
                                        "right" => x + width - ink,
                                        other => {
                                            bail!("align is left, center or right (got {other})")
                                        }
                                    }
                                }
                            };
                            let mut points: Vec<[i32; 2]> = Vec::new();
                            let missing = crate::winamp::pixel_text::layout(
                                body,
                                small,
                                scale,
                                spacing,
                                |dx, dy| points.push([x + dx, y + dy]),
                            );
                            for ch in missing {
                                if !skipped.contains(&ch) {
                                    skipped.push(ch);
                                }
                            }
                            for at in points {
                                count += self.paint_line_untracked(
                                    at,
                                    at,
                                    color,
                                    layer,
                                    all,
                                    &paint_layers,
                                )?;
                            }
                        }
                        _ => bail!("Unknown drawing operation {kind}"),
                    }
                    if let Some((_, box_)) = self.operation_box {
                        // An operation that covers the whole sheet has no cell of
                        // its own to have crossed out of. Clearing one to the
                        // transparency key is how a recipe starts a sheet, and a
                        // wash over the whole of one is how it starts a
                        // background; both touch every cell, so both answered
                        // "crossed into a cell the player draws nine times" --
                        // true, not a spill, and printed above the one spill that
                        // was real. It is the same distinction `unsampled_pixels`
                        // draws when it counts ink rather than clearing.
                        let whole_sheet = box_[0] == 0
                            && box_[1] == 0
                            && box_[2] >= sheet_size.0
                            && box_[3] >= sheet_size.1;
                        if let Some(note) = (!whole_sheet)
                            .then(|| crossed_into_repeat(&cells, box_, index, kind))
                            .flatten()
                        {
                            if crossed.len() < 8
                                && !crossed.iter().any(|c| c["cell"] == note["cell"])
                            {
                                crossed.push(note);
                            }
                        }
                    }
                    if !self.keyed_blend.is_empty() && keyed.len() < 8 {
                        keyed.push(json!({"operation":index,"op":kind,
                        "pixels":self.keyed_blend.len(),
                        "at":self.keyed_blend.iter().take(4).collect::<Vec<_>>()}));
                    }
                }
            }
            Ok(())
        })();
        self.view.layers = previous_selection;
        self.view.mask_colors = previous_mask;
        let result = result.map_err(|error| match &at {
            Some((index, kind)) => error.context(format!("operations[{index}] \"{kind}\"")),
            None => error,
        });
        if let Err(error) = result {
            self.restore(before);
            self.revision = before_revision;
            self.dirty = before_dirty;
            self.unmapped_pixels = previous_unmapped;
            return Err(error);
        }
        // A dry run: the operations are applied, the surface is rendered, and
        // the document is put back. The only way to see a stroke before making
        // it used to be to make it, look, and undo -- or to reimplement the
        // rasteriser in an image library and hope the two agreed.
        if args.get("preview") == Some(&json!(true)) {
            let bounds = self.painted_bounds;
            let mut report = json!({"preview":true,"bounds":bounds,
                "pixels_written":count,"surface":self.surface(),
                "clipped_pixels":self.clipped_pixels,
                "overwrites":self.overwrites,
                "overwrite_sample":self.overwrite_note,
                "unmapped_pixels":self.unmapped_pixels.len()});
            // A dry run says what it did as fully as a real one. Without these
            // the one call that exists to check a transaction before making it
            // could not confirm that `at` had repeated anything.
            if places.len() > 1 {
                report["repeated"] = json!({"places": places.len()});
            }
            if !swept.is_empty() {
                report["swept"] = json!({"fields": swept.clone(), "steps": steps});
            }
            // Checking a transaction before making it is the one place this
            // matters most: invisible ink is cheapest to find before it is ink.
            if let Some(covered) = self.covered_by_a_sibling_part() {
                report["covered_pixels"] = covered;
            }
            self.preview = Some(self.render());
            self.restore(before);
            self.revision = before_revision;
            self.dirty = before_dirty;
            self.unmapped_pixels = previous_unmapped;
            self.painted_bounds = bounds;
            report["ms"] = json!(started.elapsed().as_millis() as u64);
            return Ok(report);
        }
        if before != self.snapshot() {
            self.changed();
            self.record(
                before,
                args.get("label")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        format!(
                            "Draw · {} · {} ops · {}",
                            self.view.panel,
                            operations.len(),
                            if selected.is_empty() {
                                "Auto".into()
                            } else {
                                selected.join(" + ")
                            }
                        )
                    }),
                "MCP",
            );
        }
        self.message = format!(
            "Painted {count} atlas pixels{}",
            match base_scope {
                Scope::Current => "",
                Scope::Onward => " into this state and every one after it",
                Scope::UpTo => " into every state up to this one",
                Scope::All => " across all variants",
            }
        );
        if !self.unmapped_pixels.is_empty() {
            self.message.push_str(&format!(
                "; {} pixels have no bitmap source (classic playlist fill)",
                self.unmapped_pixels.len()
            ));
            if before_revision == self.revision {
                self.revision += 1;
            }
        }
        // Ink that landed in a part of the sheet no sprite ever samples. Only
        // a sheet open on its own can be painted there; the assembled canvas
        // has no way to address the gaps between cells.
        let mut unsampled: Vec<[u32; 2]> = Vec::new();
        let mut never_drawn = false;
        if self.view.panel == "atlas" {
            if let Some(index) = self.images.keys().position(|k| k == &self.view.sheet) {
                if let Some((mask, width)) = self.sampled_mask(&self.view.sheet) {
                    // A sheet nothing samples at all is not a sheet with ink in
                    // the wrong place on it. `text.bmp` is read for one colour
                    // and never drawn, so painting it -- which is the correct
                    // thing to do -- reported every one of its 2,790 pixels,
                    // every time, forever. That is a fact about the sheet and
                    // the sheet already has a note saying it.
                    never_drawn = !mask.iter().any(|sampled| *sampled);
                    if !never_drawn {
                        for ((sheet, x, y), (colour, _)) in self.atlas_writes.iter() {
                            // Erasing a gap is not ink nothing will ever show.
                            // Clearing a sheet to the transparency key before
                            // painting it is the ordinary way to start, and it
                            // reported every gutter between every cell.
                            if *sheet == index
                                && colour[..3] != [255, 0, 255]
                                && !mask[(y * width + x) as usize]
                            {
                                unsampled.push([*x, *y]);
                            }
                        }
                    }
                }
            }
        }
        if !unsampled.is_empty() {
            self.message
                .push_str(&format!("; {} never sampled", unsampled.len()));
        }
        // `bounds` is what the caller asked for made real: where the ink
        // actually landed. Checking a stroke used to mean rendering the whole
        // canvas and looking at it.
        let mut result = json!({"pixels_written":count,"revision":self.revision,
            "surface":self.surface(),
            "bounds":self.painted_bounds,
            "clipped_pixels":self.clipped_pixels,
            "overwrites":self.overwrites,
            "overwrite_sample":self.overwrite_note,
            "unmapped_pixels":self.unmapped_pixels.len(),
            "unmapped_sample":self.unmapped_pixels.iter().take(8).collect::<Vec<_>>(),
            "unsampled_pixels":unsampled.len(),
            "unsampled_sample":unsampled.iter().take(8).collect::<Vec<_>>()});
        if never_drawn {
            result["sheet_is_never_drawn"] = json!(true);
        }
        // What a scope other than "this frame" actually did. A transaction that
        // writes one object into twenty-three of a slider's twenty-eight frames
        // looks exactly like one that wrote it into one, and the difference is
        // the whole of what the caller asked for.
        if places.len() > 1 {
            result["repeated"] = json!({"places": places.len()});
        }
        if !swept.is_empty() {
            result["swept"] = json!({"fields": swept, "steps": steps});
        }
        if let Some(covered) = self.covered_by_a_sibling_part() {
            let total = covered["pixels"].as_u64().unwrap_or(0);
            self.message.push_str(&format!(
                "; {total} pixels are under the control's own thumb"
            ));
            result["covered_pixels"] = covered;
        }
        let wrote = std::mem::take(&mut self.scope_writes);
        if !wrote.is_empty() {
            result["states_written"] = json!({
                "scope": scope,
                "variants": wrote
                    .into_iter()
                    .map(|(id, n)| (id, json!(n)))
                    .collect::<serde_json::Map<String, Value>>(),
            });
        }
        if !skipped.is_empty() {
            result["unsupported_characters"] =
                json!(skipped.iter().map(|c| c.to_string()).collect::<Vec<_>>());
        }
        if !crossed.is_empty() {
            self.message
                .push_str(&format!("; {} crossed into a repeated cell", crossed.len()));
            result["crossed_cells"] = json!(crossed);
        }
        if !keyed.is_empty() {
            let total: u64 = keyed.iter().filter_map(|k| k["pixels"].as_u64()).sum();
            self.message.push_str(&format!(
                "; {total} blended with the transparency key and were written opaque"
            ));
            result["keyed_blends"] = json!(keyed);
        }
        let same = self.identical_variants();
        if !same.is_empty() {
            let first = &same[0];
            let biggest = first["groups"][0].as_array().map(Vec::len).unwrap_or(0);
            self.message.push_str(&format!(
                "; {} of {}'s {} variants are the same picture",
                biggest,
                first["id"].as_str().unwrap_or_default(),
                first["of"]
            ));
            result["identical_variants"] = json!(same);
        }
        if let Some(note) = Self::sheet_note(&self.view.sheet) {
            if self.view.panel == "atlas" {
                result["note"] = json!(note);
            }
        }
        result["ms"] = json!(started.elapsed().as_millis() as u64);
        Ok(result)
    }

    /// Ink a control draws its own other half on top of, in the same state.
    ///
    /// A slider's track and its thumb are read from one value, so a mark put at
    /// the thumb's position in frame k is behind the thumb in frame k -- and in
    /// every frame, because both moved together. The balance tail in Catamp
    /// Freefall was drawn that way: twenty-eight frames of a tail whose tip was
    /// never once visible, and the only way to find out was to look at the live
    /// player and notice the tip was missing.
    ///
    /// Only parts of the same control count. A window background being under a
    /// button is how a skin is built and is not news.
    fn covered_by_a_sibling_part(&mut self) -> Option<Value> {
        let probes = std::mem::take(&mut self.covered_probe);
        if probes.is_empty() {
            return None;
        }
        let control_of =
            |id: &str| -> String { id.split('.').take(2).collect::<Vec<_>>().join(".") };
        let mut by_variant: BTreeMap<usize, Vec<(String, [i32; 2])>> = BTreeMap::new();
        for (id, variant, at) in probes {
            by_variant.entry(variant).or_default().push((id, at));
        }
        // Hidden in *every* state it was drawn into, not in some. A thumb
        // always hides part of its own track in any one frame -- that is what a
        // slider is -- so per-frame coverage is not news. Ink that no frame
        // shows is.
        let mut seen: BTreeMap<(String, [i32; 2]), (u32, u32)> = BTreeMap::new();
        let mut sample: Vec<Value> = Vec::new();
        for (variant, hits) in by_variant {
            let value = variant.min(27) as u8;
            let mut view = self.view.clone();
            view.panel = "canvas".into();
            view.volume = value;
            view.balance = value;
            view.position = value;
            view.scroll = value;
            view.eq = [value; 11];
            let layers = self.layers_for(&view);
            for (id, at) in hits {
                let Some(mine) = layers.iter().position(|l| l.id == id) else {
                    continue;
                };
                // Later in the list is nearer the front: this is the order the
                // player draws them in, which `ground_under` reads the same way.
                let over = layers.iter().skip(mine + 1).find(|l| {
                    if control_of(&l.id) != control_of(&id) || l.id == id {
                        return false;
                    }
                    let Some((sx, sy)) = l.map(at[0].max(0) as u32, at[1].max(0) as u32) else {
                        return false;
                    };
                    // The rectangle is not the artwork. A slider's thumb is a
                    // 14x11 cell with a three-row grip in it and nothing above:
                    // counting its whole rectangle called the visible part of
                    // the tail hidden. Opaque in *every* variant of the part on
                    // top, because ink a pressed thumb covers and a released
                    // one does not is ink somebody sees.
                    let Some(image) = self.images.get(&l.sheet) else {
                        return false;
                    };
                    l.variants.iter().all(|cell| {
                        image
                            .get_pixel_checked(cell[0] + sx, cell[1] + sy)
                            .is_some_and(|p| p[3] > 0 && p.0[..3] != [255, 0, 255])
                    })
                });
                let tally = seen.entry((id.clone(), at)).or_insert((0, 0));
                tally.0 += 1;
                if let Some(over) = over {
                    tally.1 += 1;
                    if sample.len() < 8
                        && !sample.iter().any(|s| s["behind"] == json!(over.id.clone()))
                    {
                        sample.push(json!({
                            "at": at,
                            "in": id.clone(),
                            "behind": over.id.clone(),
                        }));
                    }
                }
            }
        }
        let pixels = seen
            .values()
            .filter(|(drawn, hidden)| drawn == hidden)
            .count() as u64;
        (pixels > 0).then(|| json!({"pixels": pixels, "sample": sample}))
    }
    pub fn recolor(&mut self, args: &Value) -> Result<Value> {
        self.finish_stroke();
        let map = args
            .get("colors")
            .and_then(Value::as_object)
            .context("colors is an exact source-to-destination hex map")?;
        let mut replacements = BTreeMap::new();
        for (a, b) in map {
            replacements.insert(
                parse_color(a)?,
                parse_color(b.as_str().context("color string")?)?,
            );
        }
        let sheet = args.get("sheet").and_then(Value::as_str);
        if let Some(s) = sheet {
            if !self.images.contains_key(s) {
                bail!("Unknown sheet {s}");
            }
        }
        if let Some(id) = &self.view.paint_layer {
            anyhow::ensure!(
                !self.planes.iter().find(|p| &p.id == id).unwrap().locked,
                "Paint layer is locked"
            );
        }
        self.record(self.snapshot(), "Recolor atlas palette".into(), "MCP");
        let mut n = 0;
        let active = self
            .view
            .paint_layer
            .as_ref()
            .and_then(|id| self.planes.iter().position(|p| &p.id == id));
        let images = if let Some(i) = active {
            &mut self.planes[i].images
        } else {
            &mut self.images
        };
        for (name, image) in images {
            if sheet.is_none_or(|s| s == name) {
                for pixel in image.pixels_mut() {
                    if let Some(c) = replacements.get(&pixel.0) {
                        pixel.0 = *c;
                        n += 1;
                    }
                }
            }
        }
        self.changed();
        Ok(json!({"pixels_changed":n}))
    }
    /// The most-used opaque colours in the skin, most used first: a palette hint
    /// for the picker, not the exhaustive list `palette` returns.
    pub fn palette_sample(&self, limit: usize) -> Vec<String> {
        let mut counts: BTreeMap<[u8; 4], usize> = BTreeMap::new();
        for image in self.composite_images().values() {
            for p in image.pixels() {
                if p.0[3] == 0 || p.0[..3] == [255, 0, 255] {
                    continue;
                }
                *counts.entry(p.0).or_insert(0) += 1;
            }
        }
        let mut ranked: Vec<_> = counts.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked
            .into_iter()
            .take(limit)
            .map(|(c, _)| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
            .collect()
    }
    pub fn palette(&self) -> Value {
        let mut colors = BTreeSet::new();
        for image in self.composite_images().values() {
            for p in image.pixels() {
                colors.insert(format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    p.0[0], p.0[1], p.0[2], p.0[3]
                ));
            }
        }
        json!({"colors":colors})
    }

    pub fn layout(&self) -> crate::winamp::skin::SkinLayout {
        self.files
            .get("cranamp.json")
            .and_then(|data| serde_json::from_slice(data).ok())
            .unwrap_or_default()
    }
    pub fn set_playlist_selection(&mut self, enabled: bool, source: &str) -> Value {
        self.finish_stroke();
        if enabled != self.images.contains_key("plselection.bmp") {
            self.record(self.snapshot(), "Playlist selection artwork".into(), source);
            if enabled {
                self.images
                    .insert("plselection.bmp".into(), RgbaImage::new(243, 11));
                self.files.insert("plselection.bmp".into(), Vec::new());
            } else {
                self.images.remove("plselection.bmp");
                self.files.remove("plselection.bmp");
                if self.view.sheet == "plselection.bmp" {
                    self.view.sheet = "pledit.bmp".into();
                }
                self.view.layers.retain(|id| id != "list.selection");
                if self.view.layer == "list.selection" {
                    self.view.layer = "auto".into();
                }
            }
            self.changed();
        }
        json!({"enabled":enabled,"revision":self.revision})
    }
    pub fn set_eq_handles(&mut self, enabled: bool, source: &str) -> Result<Value> {
        self.finish_stroke();
        if enabled != self.images.contains_key("eqhandles.bmp") {
            self.record(self.snapshot(), "Independent EQ handles".into(), source);
            if enabled {
                self.images
                    .insert("eqhandles.bmp".into(), RgbaImage::new(154, 50));
                self.files.insert("eqhandles.bmp".into(), Vec::new());
                let mut layout = self.layout();
                layout.eq_travel = layout.eq_travel.min(38);
                self.files
                    .insert("cranamp.json".into(), serde_json::to_vec(&layout)?);
            } else {
                self.images.remove("eqhandles.bmp");
                self.files.remove("eqhandles.bmp");
                if self.view.sheet == "eqhandles.bmp" {
                    self.view.sheet = "eqmain.bmp".into();
                }
            }
            self.changed();
        }
        Ok(json!({"enabled":enabled,"revision":self.revision}))
    }
    pub fn has_playlist_background(&self) -> bool {
        self.images.contains_key("plbg.bmp")
    }
    pub fn set_playlist_background(&mut self, enabled: bool, source: &str) -> Value {
        self.finish_stroke();
        if enabled != self.has_playlist_background() {
            self.record(self.snapshot(), "Playlist canvas".into(), source);
            if enabled {
                let color = self
                    .files
                    .get("pledit.txt")
                    .map(|bytes| crate::winamp::skin::parse_pledit_txt(bytes).normal_bg)
                    .unwrap_or([0, 0, 0, 255]);
                self.images.insert(
                    "plbg.bmp".into(),
                    RgbaImage::from_pixel(243, 203, Rgba(color)),
                );
                self.files.insert("plbg.bmp".into(), Vec::new());
            } else {
                self.images.remove("plbg.bmp");
                self.files.remove("plbg.bmp");
                self.view.layers.retain(|id| id != "list.background");
                if self.view.layer == "list.background" {
                    self.view.layer = self
                        .view
                        .layers
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "auto".into());
                }
            }
            self.changed();
        }
        json!({"enabled":enabled,"revision":self.revision})
    }
    pub fn set_layout(&mut self, args: &Value, source: &str) -> Result<Value> {
        self.finish_stroke();
        let mut fields = serde_json::to_value(self.layout())?;
        for (key, value) in args.as_object().context("layout object required")? {
            fields
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), value.clone());
        }
        let layout: crate::winamp::skin::SkinLayout = serde_json::from_value(fields)?;
        layout.validate()?;
        anyhow::ensure!(
            !self.images.contains_key("eqhandles.bmp") || layout.eq_travel <= 38,
            "independent EQ handles need eq_travel <= 38"
        );
        if layout != self.layout() {
            let bytes = serde_json::to_vec(&layout)?;
            self.record(self.snapshot(), "Skin layout".into(), source);
            self.files.insert("cranamp.json".into(), bytes);
            self.changed();
        }
        Ok(json!({"layout":layout,"revision":self.revision}))
    }
    /// The six PLEDIT.TXT colours and the 24 VISCOLOR.TXT colours, as hex.
    /// Skin options edits both, so it has to be able to read them first.
    pub(super) fn text_palettes(&self) -> (Vec<(&'static str, String)>, Vec<String>) {
        let hex = |c: [u8; 4]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        let playlist = self
            .files
            .get("pledit.txt")
            .map(|b| crate::winamp::skin::parse_pledit_txt(b))
            .unwrap_or_default();
        let visualizer = self
            .files
            .get("viscolor.txt")
            .map(|b| crate::winamp::skin::parse_viscolor_txt(b))
            .unwrap_or_default();
        (
            vec![
                ("Normal", hex(playlist.normal)),
                ("Current", hex(playlist.current)),
                ("NormalBG", hex(playlist.normal_bg)),
                ("SelectedBG", hex(playlist.selected_bg)),
                ("MbFG", hex(playlist.marquee_fg)),
                ("MbBG", hex(playlist.marquee_bg)),
            ],
            visualizer.0.iter().map(|c| hex(*c)).collect(),
        )
    }
    pub fn set_palette(&mut self, args: &Value) -> Result<Value> {
        self.finish_stroke();
        let values = args
            .as_object()
            .context("playlist palette object required")?;
        let allowed = [
            "Normal",
            "Current",
            "NormalBG",
            "SelectedBG",
            "MbFG",
            "MbBG",
        ];
        for (key, value) in values {
            if !allowed.contains(&key.as_str()) {
                bail!("Unknown playlist palette key {key}");
            }
            parse_color(value.as_str().context("hex color required")?)?;
        }
        let mut lines = String::from_utf8_lossy(
            self.files
                .get("pledit.txt")
                .map(Vec::as_slice)
                .unwrap_or(b"[Text]\n"),
        )
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
        for (key, value) in values {
            let replacement = format!("{key}={}", value.as_str().unwrap());
            if let Some(line) = lines.iter_mut().find(|line| {
                line.split_once('=')
                    .is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            }) {
                *line = replacement;
            } else {
                lines.push(replacement);
            }
        }
        self.record(self.snapshot(), "Playlist colors".into(), "MCP");
        self.files
            .insert("pledit.txt".into(), (lines.join("\n") + "\n").into_bytes());
        self.changed();
        Ok(json!({"palette":values,"revision":self.revision}))
    }
    pub fn visualizer_palette(&mut self, args: &Value) -> Result<Value> {
        self.finish_stroke();
        if let Some(values) = args.get("colors") {
            let values = values.as_array().context("colors must be 24 hex colors")?;
            if values.len() != 24 {
                bail!("Visualizer needs exactly 24 colors");
            }
            let colors = values
                .iter()
                .map(|v| parse_color(v.as_str().context("hex color required")?))
                .collect::<Result<Vec<_>>>()?;
            let content = colors
                .iter()
                .map(|c| format!("{},{},{}\n", c[0], c[1], c[2]))
                .collect::<String>()
                .into_bytes();
            if self.files.get("viscolor.txt") != Some(&content) {
                self.record(self.snapshot(), "Visualizer colors".into(), "MCP");
                self.files.insert("viscolor.txt".into(), content);
                self.changed();
            }
        }
        let palette = crate::winamp::skin::load_skin(&self.archive()?)?.viscolor.0;
        Ok(
            json!({"colors":palette.iter().map(|c|format!("#{:02x}{:02x}{:02x}",c[0],c[1],c[2])).collect::<Vec<_>>()}),
        )
    }
    pub fn guides(&self) -> Vec<super::guides::Guide> {
        use super::guides::Guide;
        let atlas = self.view.panel == "atlas";
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let panels: Vec<&str> = if atlas || self.view.panel == "canvas" {
            vec!["main", "equalizer", "playlist"]
        } else {
            vec![&self.view.panel]
        };
        for panel in panels {
            let mut v = self.view.clone();
            v.panel = panel.into();
            let offset = if self.view.panel == "canvas" {
                match panel {
                    "equalizer" => 116,
                    "playlist" => 232,
                    _ => 0,
                }
            } else {
                0
            };
            let layers = if self.view.panel == "canvas" {
                mapping::native_panel_layers(
                    self.layers_for(&v),
                    panel,
                    self.view.preview_playlist_height,
                    self.view.scroll,
                )
            } else {
                self.layers_for(&v)
            };
            for l in layers {
                if atlas && l.sheet != self.view.sheet {
                    continue;
                }
                // An atlas shows every cell a sprite has; the assembled canvas
                // shows the one the player is drawing right now -- and that
                // one has an index of its own. Numbering it 0 because it is
                // the only entry in the list labelled every rectangle on the
                // canvas with the first variant's name: the volume track at
                // frame 20 read `0 · silent`, the balance track at centre read
                // `hard left`, and the channel lamp drawn from its ON cell read
                // `off · not this channel mode`. Which is the exact mistake the
                // labels were added to catch, made by the thing reporting them.
                let variants: Vec<(usize, [u32; 4])> = if atlas {
                    l.variants.iter().copied().enumerate().collect()
                } else {
                    vec![(
                        l.variants.iter().position(|v| *v == l.source).unwrap_or(0),
                        l.source,
                    )]
                };
                for (i, r) in variants.iter().map(|(i, r)| (*i, r)) {
                    let mut rect = if atlas { *r } else { l.destination };
                    rect[1] += offset;
                    if !seen.insert((l.sheet.clone(), rect)) {
                        continue;
                    }
                    let id = if atlas {
                        format!("{panel}.{}#{i}", l.id)
                    } else if self.view.panel == "canvas" {
                        format!("{panel}.{}", l.id)
                    } else {
                        l.id.clone()
                    };
                    // A rectangle labelled `play:1` says which variant it is
                    // and not what that variant is for. The mapping knows;
                    // there is no reason for the canvas hint and the list not
                    // to say it.
                    let suffix = l.id.rsplit('.').next().unwrap_or(&l.id);
                    let label = match l.labels.get(i) {
                        Some(what) if l.variants.len() > 1 => format!("{suffix} · {what}"),
                        _ => suffix.to_string(),
                    };
                    out.push(Guide {
                        id,
                        label,
                        sheet: l.sheet.clone(),
                        rect,
                        source: *r,
                        variant: i,
                        active: *r == l.source,
                        runtime: false,
                        hit: false,
                    });
                }
            }
            let reserved: Vec<(&str, [u32; 4])> = match panel {
                "main" => vec![
                    ("TIMER DIGIT 0", [48, 26, 9, 13]),
                    ("TIMER DIGIT 1", [60, 26, 9, 13]),
                    ("TIMER DIGIT 2", [78, 26, 9, 13]),
                    ("TIMER DIGIT 3", [90, 26, 9, 13]),
                    ("SPECTRUM", [24, 43, 76, 16]),
                    ("SONG TEXT", [111, 27, 150, 8]),
                    ("BITRATE", [111, 41, 18, 8]),
                    ("SAMPLE RATE", [156, 41, 12, 8]),
                ],
                "equalizer" => vec![("EQ CURVE", [86, 17, 113, 19])],
                "playlist" => {
                    let h = if self.view.panel == "canvas" {
                        self.view.preview_playlist_height
                    } else {
                        261
                    };
                    vec![
                        ("TRACK ROWS", [12, 20, 243, h - 58]),
                        ("TIME / TOTAL", [132, h - 28, 72, 8]),
                        ("ELAPSED", [192, h - 14, 30, 8]),
                    ]
                }
                _ => vec![],
            };
            let sheet = match panel {
                "main" => "main.bmp",
                "equalizer" => "eqmain.bmp",
                _ => "pledit.bmp",
            };
            if atlas && (self.view.sheet != sheet || panel == "playlist") {
                continue;
            }
            for (name, mut r) in reserved {
                r[1] += offset;
                out.push(Guide {
                    id: format!("runtime.{panel}.{name}"),
                    label: name.into(),
                    sheet: sheet.into(),
                    rect: r,
                    source: r,
                    variant: 0,
                    active: true,
                    runtime: true,
                    hit: false,
                });
            }
            // Controls the player hit-tests and draws nothing for. Every other
            // button in a classic skin is a sprite and says where it is; the
            // playlist footer's eleven are not, and an artist drawing a footer
            // from blank had no way to find them short of reading Cranamp's
            // own source. A cat was laid across the elapsed-time readout that
            // way, and a button a pixel out of step with its own hit area is
            // the same mistake with no cat to show for it.
            let targets: Vec<(&str, [u32; 4])> = match panel {
                "main" => vec![("SKIN CHOOSER", [249, 79, 26, 33])],
                "playlist" => {
                    let h = if self.view.panel == "canvas" {
                        self.view.preview_playlist_height
                    } else {
                        261
                    };
                    vec![
                        ("ADD", [10, h - 31, 28, 18]),
                        ("REM", [39, h - 31, 28, 18]),
                        ("SEL", [69, h - 31, 28, 18]),
                        ("MISC", [99, h - 31, 36, 18]),
                        ("LIST", [228, h - 31, 28, 18]),
                        ("PREV", [139, h - 13, 8, 8]),
                        ("PLAY", [148, h - 13, 8, 8]),
                        ("PAUSE", [157, h - 13, 8, 8]),
                        ("STOP", [166, h - 13, 8, 8]),
                        ("NEXT", [175, h - 13, 8, 8]),
                        ("EJECT", [185, h - 13, 12, 8]),
                    ]
                }
                _ => vec![],
            };
            for (name, mut r) in targets {
                r[1] += offset;
                out.push(Guide {
                    id: format!("hit.{panel}.{name}"),
                    label: name.into(),
                    sheet: sheet.into(),
                    rect: r,
                    source: r,
                    variant: 0,
                    active: true,
                    runtime: false,
                    hit: true,
                });
            }
        }
        if self.view.panel == "canvas" {
            out.push(Guide {
                id: "main.docking.edge".into(),
                label: "Dock edge · shares main bottom row".into(),
                sheet: "main.bmp".into(),
                rect: [0, 115, 275, 1],
                source: [0, 114, 275, 1],
                variant: 0,
                active: true,
                runtime: false,
                hit: false,
            });
        }
        out
    }
    /// Map a native review crop to all intersecting sprite sources and live
    /// reservations. Underlying parts are included; this is not a visible mask.
    pub fn inspect_region(&self, r: [u32; 4]) -> Result<Value> {
        let (w, h) = self.canvas_size();
        anyhow::ensure!(
            r[2] > 0
                && r[3] > 0
                && r[0].checked_add(r[2]).is_some_and(|x| x <= w)
                && r[1].checked_add(r[3]).is_some_and(|y| y <= h),
            "Review rectangle must fit the current native panel"
        );
        let parts:Vec<_>=self.guides().into_iter().filter_map(|g| {
            let cut=super::guides::intersection(r,g.rect)?;
            let source=(!g.runtime&&!g.hit).then(||[g.source[0]+cut[0]-g.rect[0],g.source[1]+cut[1]-g.rect[1],cut[2],cut[3]]);
            Some(json!({"id":g.id,"label":g.label,"sheet":g.sheet,"overlap":cut,"source_overlap":source,"runtime":g.runtime,"hit":g.hit,"active":g.active}))
        }).collect();
        Ok(
            json!({"panel":self.view.panel,"rect":r,"parts":parts,"note":"Includes underlays and live reservations. Source overlaps refer to the current preview state; inspect atlas guides for alternate cells."}),
        )
    }
    pub fn select_guide(&mut self, id: &str) -> Result<Value> {
        let g = self
            .guides()
            .into_iter()
            .find(|g| g.id == id)
            .context("Unknown part guide")?;
        if self.view.panel == "atlas" {
            self.view.clip = Some(g.rect);
            self.view.layers = vec!["sheet".into()];
            self.view.layer = "sheet".into();
        } else if !g.runtime && !g.hit {
            self.view.layers = vec![g.id.clone()];
            self.view.layer = g.id.clone();
            self.view.clip = None;
        } else {
            self.view.clip = Some(g.rect);
        }
        self.message = format!(
            "{} · {} {:?}{}",
            g.id,
            g.sheet,
            g.source,
            if g.runtime {
                " · runtime draws here"
            } else if g.hit {
                " · the player hit-tests here and draws nothing"
            } else {
                ""
            }
        );
        self.revision += 1;
        Ok(json!(g))
    }
    /// Prepare independent planes with their effective per-pixel alpha. A clipped
    /// plane inherits the immediately lower plane's alpha (or the atlas base).
    /// The source pixels remain untouched and editable when clipping is disabled.
    fn effective_planes(&self) -> Vec<BTreeMap<String, RgbaImage>> {
        let mut planes: Vec<BTreeMap<String, RgbaImage>> = Vec::new();
        for plane in &self.planes {
            let below = planes.last().unwrap_or(&self.images);
            let mut effective = plane.images.clone();
            for (name, im) in &mut effective {
                let mask = below.get(name);
                for (x, y, p) in im.enumerate_pixels_mut() {
                    let alpha = if plane.visible {
                        (p[3] as u32 * plane.opacity as u32 + 127) / 255
                    } else {
                        0
                    };
                    p[3] = if plane.clip_below {
                        ((alpha * mask.map_or(0, |m| m.get_pixel(x, y)[3]) as u32 + 127) / 255)
                            as u8
                    } else {
                        alpha as u8
                    };
                }
            }
            planes.push(effective);
        }
        planes
    }
    pub fn composite_images(&self) -> BTreeMap<String, RgbaImage> {
        let mut out = self.images.clone();
        for plane in self.effective_planes() {
            for (name, im) in &plane {
                let Some(dst) = out.get_mut(name) else {
                    continue;
                };
                for (d, s) in dst.pixels_mut().zip(im.pixels()) {
                    d.blend(s);
                }
            }
        }
        out
    }
    pub fn paint_layer_info(&self) -> Value {
        json!({"active":self.view.paint_layer,"base":"Original atlases", "layers":self.planes.iter().map(|p|json!({"id":p.id,"name":p.name,"visible":p.visible,"locked":p.locked,"opacity":p.opacity,"clip_below":p.clip_below})).collect::<Vec<_>>()})
    }
    pub fn paint_layer_command(&mut self, args: &Value, source: &str) -> Result<Value> {
        self.finish_stroke();
        let before = self.snapshot();
        let previous_active = self.view.paint_layer.clone();
        let result = (|| -> Result<()> {
            let action = args["action"].as_str().unwrap_or("list");
            if action == "list" {
                return Ok(());
            }
            if action == "add" {
                anyhow::ensure!(
                    self.planes.len() < MAX_PAINT_LAYERS,
                    "At most {MAX_PAINT_LAYERS} painting layers"
                );
                let id = (1..)
                    .map(|i| format!("paint-{i}"))
                    .find(|id| !self.planes.iter().any(|p| &p.id == id))
                    .unwrap();
                let name = args["name"]
                    .as_str()
                    .unwrap_or("Paint layer")
                    .chars()
                    .take(80)
                    .collect();
                self.planes.push(PaintLayer {
                    id: id.clone(),
                    name,
                    visible: true,
                    locked: false,
                    opacity: 255,
                    clip_below: false,
                    images: BTreeMap::new(),
                });
                self.view.paint_layer = Some(id);
                return Ok(());
            }
            if action == "select" && args["id"].as_str() == Some("base") {
                self.view.paint_layer = None;
                return Ok(());
            }
            let id = args["id"]
                .as_str()
                .or(self.view.paint_layer.as_deref())
                .context("Painting layer id required")?;
            let i = self
                .planes
                .iter()
                .position(|p| p.id == id)
                .context("Unknown painting layer")?;
            match action {
                "select" => self.view.paint_layer = Some(self.planes[i].id.clone()),
                "set" => {
                    if let Some(name) = args["name"].as_str() {
                        self.planes[i].name = name.chars().take(80).collect();
                    }
                    if let Some(v) = args["visible"].as_bool() {
                        self.planes[i].visible = v;
                    }
                    if let Some(v) = args["locked"].as_bool() {
                        self.planes[i].locked = v;
                    }
                    if let Some(v) = args.get("clip_below") {
                        self.planes[i].clip_below =
                            v.as_bool().context("clip_below must be boolean")?;
                    }
                    if let Some(v) = args.get("opacity") {
                        let n = v.as_u64().context("Opacity is 0..255")?;
                        anyhow::ensure!(n <= 255, "Opacity is 0..255");
                        self.planes[i].opacity = n as u8;
                    }
                }
                "move" => {
                    let to = args["index"].as_u64().context("Layer index required")? as usize;
                    anyhow::ensure!(to < self.planes.len(), "Layer index out of bounds");
                    let p = self.planes.remove(i);
                    self.planes.insert(to, p);
                }
                "delete" => {
                    self.planes.remove(i);
                    if self.view.paint_layer.as_deref() == Some(id) {
                        self.view.paint_layer = None;
                    }
                }
                "merge_down" => {
                    anyhow::ensure!(self.planes[i].visible, "Show layer before merging");
                    anyhow::ensure!(
                        !self.planes.get(i + 1).is_some_and(|p| p.clip_below),
                        "Merge the clipped layer above first"
                    );
                    let effective = self.effective_planes().remove(i);
                    let _p = self.planes.remove(i);
                    let target = if i == 0 {
                        &mut self.images
                    } else {
                        anyhow::ensure!(
                            !self.planes[i - 1].locked
                                && self.planes[i - 1].visible
                                && self.planes[i - 1].opacity == 255
                                && !self.planes[i - 1].clip_below,
                            "Destination must be visible, unlocked, fully opaque and not clipped"
                        );
                        &mut self.planes[i - 1].images
                    };
                    for (name, im) in effective {
                        let dst = target
                            .entry(name)
                            .or_insert_with(|| RgbaImage::new(im.width(), im.height()));
                        for (d, s) in dst.pixels_mut().zip(im.pixels()) {
                            d.blend(s);
                        }
                    }
                    self.view.paint_layer = if i == 0 {
                        None
                    } else {
                        Some(self.planes[i - 1].id.clone())
                    };
                }
                _ => bail!("Unknown painting layer action"),
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.images = before.images;
            self.files = before.files;
            self.planes = before.planes;
            self.view.paint_layer = previous_active;
            return Err(e);
        }
        if self.snapshot() != before {
            self.record(
                before,
                format!(
                    "Paint layers · {}",
                    args["action"].as_str().unwrap_or("list")
                ),
                source,
            );
            self.changed();
        } else if self.view.paint_layer != previous_active {
            self.revision += 1;
        }
        Ok(self.paint_layer_info())
    }
    pub fn finish_opacity_stroke(&mut self) {
        if let Some(before) = self.stroke.take() {
            if self.snapshot() != before {
                self.record(before, "Layer opacity".into(), "Human");
                self.message = "Layer opacity".into();
            }
        }
    }
    pub fn opacity_stroke(&mut self, id: &str, opacity: u8) -> Result<()> {
        let i = self
            .planes
            .iter()
            .position(|p| p.id == id)
            .context("Unknown paint layer")?;
        if self.stroke.is_none() {
            self.checkpoint();
        }
        self.planes[i].opacity = opacity;
        self.changed();
        Ok(())
    }
    /// Serialize the layered project. `save_project` writes these bytes to disk;
    /// the browser build has no filesystem and stores the same bytes itself.
    pub fn project_bytes(&mut self) -> Result<Vec<u8>> {
        self.finish_stroke();
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut bytes);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            z.start_file("base.wsz", opts)?;
            z.write_all(&self.archive_with_images(&self.images)?)?;
            let manifest = json!({"version":1,"view":self.view,"skin_path":self.path,"layers":self.paint_layer_info()["layers"]});
            z.start_file("project.json", opts)?;
            z.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
            for (i, p) in self.planes.iter().enumerate() {
                for (name, im) in &p.images {
                    z.start_file(format!("layers/{i}/{name}.png"), opts)?;
                    let mut data = Cursor::new(Vec::new());
                    im.write_to(&mut data, image::ImageFormat::Png)?;
                    z.write_all(data.get_ref())?;
                }
            }
            z.finish()?;
        }
        Ok(bytes.into_inner())
    }
    /// Record that the document as it stands has been persisted as `label`.
    pub fn mark_project_saved(&mut self, label: String) {
        self.saved = self.snapshot();
        self.dirty = false;
        self.message = label;
        self.revision += 1;
    }
    pub fn save_project(&mut self, path: &Path) -> Result<Value> {
        let bytes = self.project_bytes()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("cstudio.tmp");
        std::fs::write(&temp, &bytes)?;
        std::fs::rename(temp, path)?;
        self.mark_project_saved(format!("Saved layered project {}", path.display()));
        Ok(json!({"path":path,"layers":self.planes.len(),"bytes":bytes.len()}))
    }
    pub fn open_project(bytes: &[u8]) -> Result<Self> {
        let mut z = zip::ZipArchive::new(Cursor::new(bytes))?;
        let mut base = Vec::new();
        z.by_name("base.wsz")?.read_to_end(&mut base)?;
        let mut meta = String::new();
        z.by_name("project.json")?.read_to_string(&mut meta)?;
        let meta: Value = serde_json::from_str(&meta)?;
        anyhow::ensure!(meta["version"] == 1, "Unsupported project version");
        let mut d = Self::open(&base, meta["skin_path"].as_str().map(str::to_owned))?;
        let planes = meta["layers"]
            .as_array()
            .context("Project layers missing")?;
        anyhow::ensure!(
            planes.len() <= MAX_PAINT_LAYERS,
            "At most {MAX_PAINT_LAYERS} project layers"
        );
        let mut ids = BTreeSet::new();
        for (i, p) in planes.iter().enumerate() {
            let id = p["id"].as_str().context("Layer id missing")?.to_owned();
            anyhow::ensure!(ids.insert(id.clone()), "Duplicate layer id");
            let mut images = BTreeMap::new();
            for (name, base) in &d.images {
                if let Ok(mut entry) = z.by_name(&format!("layers/{i}/{name}.png")) {
                    let mut b = Vec::new();
                    entry.read_to_end(&mut b)?;
                    let im = image::load_from_memory(&b)?.to_rgba8();
                    anyhow::ensure!(
                        im.dimensions() == base.dimensions(),
                        "Layer dimensions differ from atlas"
                    );
                    images.insert(name.clone(), im);
                }
            }
            let opacity = p["opacity"].as_u64().context("Layer opacity missing")?;
            anyhow::ensure!(opacity <= 255, "Invalid opacity");
            d.planes.push(PaintLayer {
                id,
                name: p["name"].as_str().unwrap_or("Layer").into(),
                visible: p["visible"] == true,
                locked: p["locked"] == true,
                opacity: opacity as u8,
                clip_below: p["clip_below"].as_bool().unwrap_or(false),
                images,
            });
        }
        d.state(meta["view"].clone())?;
        d.saved = d.snapshot();
        d.dirty = false;
        d.message = "Opened layered project".into();
        Ok(d)
    }
    pub fn archive(&self) -> Result<Vec<u8>> {
        self.archive_with_images(&self.composite_images())
    }
    /// Render the requested pressed and title-focus artwork through the real
    /// player without changing the editable sheets, exported bytes, or history.
    pub fn preview_archive(&self) -> Result<Vec<u8>> {
        let composite = self.composite_images();
        let mut images = composite.clone();
        for panel in ["main", "equalizer", "playlist"] {
            for active in [false, true] {
                let mut normal = self.view.clone();
                normal.panel = panel.into();
                normal.active = active;
                normal.pressed = false;
                let mut desired = normal.clone();
                desired.pressed = self.view.pressed;
                desired.active = self.view.active;
                let normal_layers = self.layers_for(&normal);
                let desired_layers = self.layers_for(&desired);
                for (target, source) in normal_layers.iter().zip(desired_layers.iter()) {
                    if target.source == source.source {
                        continue;
                    }
                    let Some(original) = composite.get(&source.sheet) else {
                        continue;
                    };
                    let Some(destination) = images.get_mut(&target.sheet) else {
                        continue;
                    };
                    for y in 0..target.source[3] {
                        for x in 0..target.source[2] {
                            if let (Some(pixel), Some(dest)) = (
                                original
                                    .get_pixel_checked(source.source[0] + x, source.source[1] + y),
                                destination.get_pixel_mut_checked(
                                    target.source[0] + x,
                                    target.source[1] + y,
                                ),
                            ) {
                                *dest = *pixel;
                            }
                        }
                    }
                }
            }
        }
        self.archive_with_images(&images)
    }
    fn archive_with_images(&self, images: &BTreeMap<String, RgbaImage>) -> Result<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut output);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, data) in &self.files {
                writer.start_file(name, opts)?;
                if let Some(image) = images.get(name) {
                    let mut im = image.clone();
                    for p in im.pixels_mut() {
                        if p.0[3] < 128 {
                            p.0 = [255, 0, 255, 255];
                        }
                    }
                    let mut bytes = Cursor::new(Vec::new());
                    image::DynamicImage::ImageRgba8(im)
                        .to_rgb8()
                        .write_to(&mut bytes, image::ImageFormat::Bmp)?;
                    writer.write_all(bytes.get_ref())?;
                } else {
                    writer.write_all(data)?;
                }
            }
            writer.finish()?;
        }
        Ok(output.into_inner())
    }
    /// Sprites with nothing in them at all.
    ///
    /// A skin drawn from **New blank** with twelve of its thirteen sheets still
    /// untouched exports in six kilobytes and answers exactly as a finished one
    /// does: validating through the loader only proves the archive parses. The
    /// transport keys, the timer, the title bar and the whole equalizer are
    /// simply invisible in the player, and nothing said so.
    ///
    /// It is a report and never a refusal: a skin with no channel lamps, or no
    /// playlist selection artwork, is a choice somebody may have made.
    pub fn undrawn_sprites(&self) -> Vec<String> {
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        let composite = self.composite_images();
        let mut out = Vec::new();
        for layer in self.layers_for(&view) {
            let Some(image) = composite.get(&layer.sheet) else {
                continue;
            };
            let painted = layer.variants.iter().any(|r| {
                (r[1]..(r[1] + r[3]).min(image.height())).any(|y| {
                    (r[0]..(r[0] + r[2]).min(image.width())).any(|x| image.get_pixel(x, y).0[3] > 0)
                })
            });
            if !painted && !out.contains(&layer.id) {
                out.push(layer.id.clone());
            }
        }
        out
    }
    /// The colour Cranamp will write its readouts in, by the loader's own rule.
    fn display_ink(&self) -> [u8; 4] {
        self.images
            .get("text.bmp")
            .and_then(|im| {
                let total = (im.width() as usize).saturating_mul(im.height() as usize);
                crate::winamp::skin::sample_display_ink(im.as_raw(), total)
            })
            .unwrap_or([153, 204, 236, 255])
    }
    /// The artwork a canvas rectangle sits on, averaged.
    ///
    /// Walked from the topmost sprite down, because that is the order the
    /// player draws them in and a readout sits on whatever is nearest it.
    /// The artwork under a rectangle, averaged over its opaque pixels.
    ///
    /// Public because it is the answer to two questions, not one: the checker
    /// asks it about the eight readouts Cranamp writes, and `studio_pixel` asks
    /// it about anywhere at all -- which is what a skin whose whole grammar is
    /// "how far is this from a flame" needs before it can put a cat down.
    pub fn ground_under(&self, rect: [u32; 4]) -> Option<[u8; 4]> {
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        let layers = self.layers_for(&view);
        let composite = self.composite_images();
        let (mut sum, mut seen) = ([0u64; 3], 0u64);
        for y in rect[1]..rect[1] + rect[3] {
            for x in rect[0]..rect[0] + rect[2] {
                for layer in layers.iter().rev() {
                    let Some((sx, sy)) = layer.map(x, y) else {
                        continue;
                    };
                    let Some(image) = composite.get(&layer.sheet) else {
                        continue;
                    };
                    let (px, py) = (layer.source[0] + sx, layer.source[1] + sy);
                    if px >= image.width() || py >= image.height() {
                        continue;
                    }
                    let pixel = image.get_pixel(px, py).0;
                    // The transparency key is not a colour, here as everywhere
                    // else. Averaged in as one, a sprite cell that is mostly
                    // cleared -- which every control drawn on top of a window
                    // is -- answers magenta, and the artwork the player will
                    // actually show through it is the thing that was asked
                    // about. A transport key on dark cloth came back as
                    // `#963384`, and with it the wrong answer to whether the
                    // mark on it reads.
                    if pixel[3] == 0 || pixel[..3] == [255, 0, 255] {
                        continue;
                    }
                    for c in 0..3 {
                        sum[c] += pixel[c] as u64;
                    }
                    seen += 1;
                    break;
                }
            }
        }
        (seen > 0).then(|| {
            [
                (sum[0] / seen) as u8,
                (sum[1] / seen) as u8,
                (sum[2] / seen) as u8,
                255,
            ]
        })
    }
    /// Whether each thing the player writes can be read on the artwork under it.
    /// The ratio itself is `contrast_ratio`, so `studio_pixel` answers the same
    /// question about any rectangle with the same arithmetic.
    ///
    /// Studio already knows both halves and never put them together: the
    /// runtime rectangles say where Cranamp writes, `text.bmp` says what colour
    /// the display ink will be, and PLEDIT.TXT says what colour the playlist
    /// text will be. A skin whose title is one shade off its own background is
    /// the mistake that actually ships, and the only way to catch it was to
    /// render the player and squint.
    ///
    /// The ratio is the WCAG one. Four and a half is comfortable at this size;
    /// below three is a readout you have to hunt for, and below two is one that
    /// is not there.
    pub fn readability(&mut self) -> Vec<Value> {
        let ratio = contrast_ratio;
        let hex = |c: [u8; 4]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        let ink = self.display_ink();
        let (palette, _) = self.text_palettes();
        let colour = |name: &str| -> Option<[u8; 4]> {
            palette
                .iter()
                .find(|(k, _)| *k == name)
                .and_then(|(_, v)| parse_color(v).ok())
        };
        // The runtime rectangles are the canvas panel's, whatever the editor is
        // showing at the moment, so the view goes there and straight back.
        let showing = std::mem::replace(&mut self.view.panel, "canvas".into());
        let runtime: Vec<(String, [u32; 4])> = self
            .guides()
            .into_iter()
            .filter(|g| g.runtime)
            .map(|g| (g.id.clone(), g.rect))
            .collect();
        self.view.panel = showing;
        let mut out = Vec::new();
        let mut check = |what: &str, ink: [u8; 4], ground: [u8; 4]| {
            let r = ratio(ink, ground);
            out.push(json!({"reads":what,"ink":hex(ink),"ground":hex(ground),
                            "contrast":r,"readable":r >= 3.0}));
        };
        for (id, rect) in &runtime {
            // Not `else { continue }`. The classic playlist fill has no bitmap
            // under it at all, so `ground_under` answers None there -- and that
            // skipped the whole of TRACK ROWS, including the three checks below
            // that do not want the artwork's ground but `NormalBG`. Every skin
            // that does not add `plbg.bmp`, which is every skin by default, was
            // told nothing at all about whether its playlist reads, by the one
            // call that exists to say so.
            let ground = self.ground_under(*rect);
            let Some(ground) = ground else {
                if id == "runtime.playlist.TRACK ROWS" {
                    let bg = colour("NormalBG");
                    if let (Some(normal), Some(bg)) = (colour("Normal"), bg) {
                        check("a playlist row", normal, bg);
                    }
                    if let (Some(current), Some(bg)) = (colour("Current"), bg) {
                        check("the playing row", current, bg);
                    }
                    if let (Some(normal), Some(on)) = (colour("Normal"), colour("SelectedBG")) {
                        check("a selected row", normal, on);
                    }
                }
                continue;
            };
            match id.as_str() {
                "runtime.main.SONG TEXT" => check("the track title", ink, ground),
                "runtime.main.BITRATE" | "runtime.main.SAMPLE RATE" => {
                    check(id.rsplit('.').next().unwrap_or(id), ink, ground)
                }
                // The equalizer curve is drawn in the display ink -- text.bmp's
                // sampled colour, the same one the title is written in -- over
                // whatever the skin painted in the graph rectangle. A skin with
                // a dark graph and a dark ink draws its curve and nobody ever
                // sees it, and until this line the check that exists for
                // exactly this said nothing about the one readout that is a
                // picture rather than words.
                "runtime.equalizer.EQ CURVE" => check("the equalizer curve", ink, ground),
                // The footer's two readouts are the one place in the playlist
                // that does *not* use PLEDIT.TXT: Cranamp writes them in the
                // display ink, the same colour as the main window's title.
                // Checking them against `Normal` reported dark-on-dark for a
                // skin whose footer reads perfectly.
                "runtime.playlist.TIME / TOTAL" | "runtime.playlist.ELAPSED" => {
                    check(id.rsplit('.').next().unwrap_or(id), ink, ground)
                }
                "runtime.playlist.TRACK ROWS" => {
                    // The list has a background of its own only when the skin
                    // gives it one; otherwise the player fills it with NormalBG.
                    let ground = if self.images.contains_key("plbg.bmp") {
                        ground
                    } else {
                        colour("NormalBG").unwrap_or(ground)
                    };
                    if let Some(normal) = colour("Normal") {
                        check("a playlist row", normal, ground);
                    }
                    if let Some(current) = colour("Current") {
                        check("the playing row", current, ground);
                    }
                    let selected = self
                        .images
                        .get("plselection.bmp")
                        .and_then(|_| self.ground_under([12, rect[1], 243, 11]))
                        .or_else(|| colour("SelectedBG"));
                    if let (Some(normal), Some(on)) = (colour("Normal"), selected) {
                        check("a selected row", normal, on);
                    }
                }
                _ => {}
            }
        }
        out
    }
    pub fn export(&mut self, path: &Path) -> Result<Value> {
        self.finish_stroke();
        let bytes = self.archive()?;
        crate::winamp::skin::load_skin(&bytes)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("wsz.tmp");
        std::fs::write(&temp, &bytes)?;
        std::fs::rename(temp, path)?;
        self.path = Some(path.to_string_lossy().into());
        if !self.planes.is_empty() {
            self.save_project(&path.with_extension("cstudio"))?;
        }
        self.dirty = false;
        self.saved = self.snapshot();
        let undrawn = self.undrawn_sprites();
        let unreadable: Vec<Value> = self
            .readability()
            .into_iter()
            .filter(|r| r["readable"] == false)
            .collect();
        self.message = format!("Saved {}", path.display());
        if !undrawn.is_empty() {
            self.message
                .push_str(&format!("; {} sprites are still blank", undrawn.len()));
        }
        if !unreadable.is_empty() {
            self.message.push_str(&format!(
                "; {} readouts are hard to read on the artwork",
                unreadable.len()
            ));
        }
        self.revision += 1;
        let mut out = json!({"path":path,"bytes":bytes.len()});
        if !undrawn.is_empty() {
            out["undrawn_sprites"] = json!(undrawn);
        }
        if !unreadable.is_empty() {
            out["hard_to_read"] = json!(unreadable);
        }
        Ok(out)
    }
}
pub fn parse_color(s: &str) -> Result<[u8; 4]> {
    if s == "transparent" {
        return Ok([255, 0, 255, 0]);
    }
    let s = s
        .strip_prefix('#')
        .context("Color needs #RRGGBB or #RRGGBBAA")?;
    if !s.is_ascii() || (s.len() != 6 && s.len() != 8) {
        bail!("Color needs 6 or 8 hex digits");
    }
    let mut c = [255; 4];
    for (i, v) in c.iter_mut().enumerate().take(s.len() / 2) {
        *v = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)?;
    }
    Ok(c)
}
/// One operation's ink, having left the cell it was aimed at and landed in a
/// cell the player draws in more than one place.
///
/// A sheet is a bag of cells and crossing between two of them is often exactly
/// what an artist means -- a band along a footer, a wash over a background. It
/// is never what they mean when the cell on the other side is a *tile*: the
/// playlist header's is 25 pixels wide and drawn nine times, so a caption that
/// runs four pixels past the end of the title cell does not spill into empty
/// sheet, it spills into the tile, and the player writes it nine times across
/// the top of the window. Nothing reported that -- every pixel of it is a legal
/// part of some cell, `overwrites` is a canvas-to-source measure that does not
/// apply to a sheet open on its own, and the artist reads CASE NOTES with SCAN
/// SCAN SCAN either side of it and goes looking for a bug in the tiling.
fn crossed_into_repeat(
    cells: &[(String, usize, [u32; 4])],
    box_: [u32; 4],
    index: usize,
    kind: &str,
) -> Option<Value> {
    let mut touched: BTreeSet<&str> = BTreeSet::new();
    let mut repeated: Option<(&str, usize)> = None;
    for (id, times, r) in cells {
        let hit = box_[0] < r[0] + r[2]
            && r[0] < box_[0] + box_[2]
            && box_[1] < r[1] + r[3]
            && r[1] < box_[1] + box_[3];
        if !hit {
            continue;
        }
        touched.insert(id.as_str());
        if *times > 1 {
            repeated = Some((id.as_str(), *times));
        }
    }
    let (id, times) = repeated?;
    if touched.len() < 2 {
        return None;
    }
    Some(json!({
        "operation": index,
        "op": kind,
        "cell": id,
        "drawn": times,
        "note": format!(
            "operations[{index}] \"{kind}\" crossed out of its own cell into {id}, \
             which the player draws {times} times -- so whatever landed there is \
             repeated in every one of them"
        ),
    }))
}
fn integer(v: &Value, key: &str, default: i32) -> Result<i32> {
    match v.get(key) {
        None => Ok(default),
        Some(n) => {
            // Name the field and the value, the way every other refusal here
            // does. "Coordinates must be integers" is true of eleven fields
            // and says which of them was wrong about none of them.
            let n = n
                .as_i64()
                .with_context(|| format!("{key} is a whole number of pixels (got {n})"))?;
            if !(-4096..=4096).contains(&n) {
                bail!("{key} is -4096..4096 (got {n})");
            }
            Ok(n as i32)
        }
    }
}
fn line(a: [i32; 2], b: [i32; 2]) -> Vec<(i32, i32)> {
    let (mut x, mut y) = (a[0], a[1]);
    let dx = (b[0] - x).abs();
    let dy = -(b[1] - y).abs();
    let sx = if x < b[0] { 1 } else { -1 };
    let sy = if y < b[1] { 1 } else { -1 };
    let mut e = dx + dy;
    let mut points = Vec::new();
    loop {
        points.push((x, y));
        if x == b[0] && y == b[1] {
            break;
        }
        let e2 = 2 * e;
        if e2 >= dy {
            e += dy;
            x += sx;
        }
        if e2 <= dx {
            e += dx;
            y += sy;
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Document {
        Document::open(include_bytes!("../../../assets/winamp.wsz"), None).unwrap()
    }
    #[test]
    fn painted_transparency_key_matches_player_in_canvas_but_remains_editable_in_atlas() {
        let mut d = Document::blank();
        for p in d.images.get_mut("main.bmp").unwrap().pixels_mut() {
            *p = Rgba([20, 30, 40, 255]);
        }
        for p in d.images.get_mut("cbuttons.bmp").unwrap().pixels_mut() {
            *p = Rgba([255, 0, 255, 255]);
        }
        d.state(json!({"panel":"canvas"})).unwrap();
        assert_eq!(d.render().get_pixel(40, 90), &Rgba([20, 30, 40, 255]));
        d.state(json!({"layers":["main.play"]})).unwrap();
        assert_eq!(d.selected_image().get_pixel(40, 90)[3], 0);
        d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layers":[]}))
            .unwrap();
        assert_eq!(d.render().get_pixel(23, 0), &Rgba([255, 0, 255, 255]));
    }
    /// A blended operation composes the artwork under the pixels it is about
    /// to write and nowhere else, which is the difference between a caustic
    /// net taking a tenth of a second and taking twenty.
    ///
    /// Two things can go wrong with composing a window instead of everything.
    /// The window can be read at the wrong offset, which the crop half of this
    /// checks; and the per-pixel walk over a sheet's painting planes, which is
    /// what replaced compositing whole sheets, can disagree with
    /// `composite_images` -- the same arithmetic the export writes with, which
    /// is why it is the reference here. Hidden planes, partial opacity and
    /// `clip_below` are the three things that walk has to reproduce, and each
    /// plane is offset from the one under it so a clipped plane has ink both
    /// on and off its own mask. Drawn in the same place they agree however the
    /// mask is applied and the assertion stops being one.
    #[test]
    fn a_patch_of_the_canvas_is_the_same_picture_as_the_whole_render_of_it() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#204050"},
        ]}))
        .unwrap();
        d.state(json!({"panel":"canvas"})).unwrap();
        for (i, (opacity, clip_below, visible)) in
            [(255u8, false, true), (140, true, true), (255, false, false)]
                .into_iter()
                .enumerate()
        {
            let made = d
                .paint_layer_command(&json!({"action":"add","name":"plane"}), "MCP")
                .unwrap();
            let id = made["active"].as_str().unwrap().to_string();
            let left = 20 + i as i64 * 34;
            d.draw(&json!({"operations":[
                {"op":"ellipse","x":left,"y":20,"width":90,"height":60,"color":"#c08040","fill":true},
                {"op":"rect","x":left + 20,"y":30,"width":30,"height":10,"color":"#ffffff","opacity":128},
            ]}))
            .unwrap();
            d.paint_layer_command(
                &json!({"action":"set","id":id,"opacity":opacity,
                        "clip_below":clip_below,"visible":visible}),
                "MCP",
            )
            .unwrap();
        }

        let composite = d.composite_images();
        let mut differs = 0;
        for (name, sheet) in &composite {
            let stack = d.sheet_stack(name).expect("every sheet has a stack");
            for (x, y, pixel) in sheet.enumerate_pixels() {
                assert_eq!(stack.at(x, y), Some(*pixel), "{name} at {x},{y}");
                if d.images[name].get_pixel(x, y) != pixel {
                    differs += 1;
                }
            }
        }
        assert!(
            differs > 2000,
            "the planes have to actually change the sheets or this proves nothing: {differs}"
        );

        let whole = d.render();
        for area in [
            [0, 0, 275, 377],
            [30, 25, 40, 30],
            [-6, -6, 20, 20],
            [260, 360, 40, 40],
        ] {
            let patch = d.render_patch(area);
            for y in area[1].max(0)..(area[1] + area[3]).min(377) {
                for x in area[0].max(0)..(area[0] + area[2]).min(275) {
                    assert_eq!(
                        patch.at(x, y),
                        Some(whole.get_pixel(x as u32, y as u32).0),
                        "patch {area:?} disagrees at {x},{y}"
                    );
                }
            }
        }
        // And outside its own bounds it says nothing rather than something
        // from the wrong place.
        assert_eq!(d.render_patch([30, 25, 4, 4]).at(29, 25), None);
    }
    /// A half-transparent image blends into the skin as it stands. The backdrop
    /// it blends into used to be rendered once per transaction and then reused,
    /// so a glow placed after the picture beneath it was painted blended into
    /// the picture that was there before -- and wrote it back, silently undoing
    /// earlier operations in the same transaction. Two glows are needed to see
    /// it: the first is what populated the stale backdrop.
    #[test]
    fn a_blended_image_sees_what_the_same_transaction_painted_under_it() {
        use base64::Engine as _;
        let mut d = Document::blank();
        let translucent = |w: u32, h: u32, alpha: u8| {
            let mut im = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, alpha]));
            im.put_pixel(0, 0, Rgba([255, 255, 255, alpha]));
            let mut out = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(im)
                .write_to(&mut out, image::ImageFormat::Png)
                .unwrap();
            json!({"op":"image","x":0,"y":0,
                   "data":base64::engine::general_purpose::STANDARD.encode(out.into_inner())})
        };
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":40,"height":8,"color":"#ffffff"},
            translucent(8, 8, 128),
            {"op":"rect","x":0,"y":0,"width":40,"height":8,"color":"#000000"},
            translucent(40, 8, 128),
        ]}))
        .unwrap();
        // Black, then white at half alpha: the row is mid grey everywhere. It
        // used to come back white, because the second stamp blended into the
        // backdrop the first one had cached, from before the black rectangle.
        for x in [0, 9, 20, 39] {
            assert_eq!(
                d.images["main.bmp"].get_pixel(x, 4),
                &Rgba([128, 128, 128, 255]),
                "column {x} blended into a stale backdrop"
            );
        }
    }
    /// Three things a draw result has to say that it used to keep to itself: a
    /// refusal names the operation that caused it, a character the 5x7 face
    /// does not have is skipped and named instead of aborting ten thousand
    /// operations, and ink that lands between a sheet's cells -- where no
    /// sprite will ever sample it -- is counted.
    #[test]
    fn a_draw_reports_the_operation_that_failed_the_glyphs_it_skipped_and_ink_nothing_samples() {
        let mut d = document();
        let error = d
            .draw(&json!({"operations":[
                {"op":"pixel","x":1,"y":1,"color":"#123456"},
                {"op":"rect","x":2,"y":2,"width":4,"height":4,"color":"#123456"},
                {"op":"nonsense","x":3,"y":3},
            ]}))
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("operations[2] \"nonsense\""),
            "{error:#}"
        );
        assert!(!d.dirty, "a refused transaction leaves nothing behind");

        d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
            .unwrap();
        let result = d
            .draw(&json!({"operations":[
                {"op":"text","x":1,"y":1,"text":"OK@ \u{00e9}","color":"#ffffff"},
            ]}))
            .unwrap();
        assert_eq!(result["unsupported_characters"], json!(["@", "é"]));
        assert!(result["pixels_written"].as_u64().unwrap() > 0);
        assert!(result["note"].as_str().unwrap().contains("not drawn"));

        // pledit.bmp column 125 lies between the two footer flaps: bottom.left
        // is sheet 0..124 and bottom.right is 126..275, so nothing samples it.
        d.state(json!({"panel":"atlas","sheet":"pledit.bmp","layer":"sheet"}))
            .unwrap();
        let result = d
            .draw(&json!({"operations":[
                {"op":"rect","x":120,"y":72,"width":10,"height":38,"color":"#123456"},
            ]}))
            .unwrap();
        assert_eq!(result["unsampled_pixels"], json!(38));
        assert!(result["unsampled_sample"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p[0] == json!(125)));
    }

    /// The two materials that used to need an image library, and the dry run
    /// that used to need one reimplemented. Grain has to be identical on every
    /// machine, because a recipe's whole claim is that it reproduces the skin.
    #[test]
    fn grain_is_deterministic_opacity_bakes_a_blend_and_preview_leaves_no_trace() {
        let field = json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":60,"height":20,"color":"#808080",
             "grain":12,"grain_seed":7},
        ]});
        let mut a = Document::blank();
        let mut b = Document::blank();
        for d in [&mut a, &mut b] {
            d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
                .unwrap();
            d.draw(&field).unwrap();
        }
        assert_eq!(a.images["main.bmp"], b.images["main.bmp"]);
        let grained = (0..60)
            .map(|x| a.images["main.bmp"].get_pixel(x, 4)[0])
            .collect::<std::collections::BTreeSet<_>>();
        assert!(grained.len() > 6, "grain should vary: {grained:?}");
        assert!(
            grained.iter().all(|v| (116..=140).contains(v)),
            "grain should stay inside its amplitude: {grained:?}"
        );

        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":10,"height":10,"color":"#000000"},
            {"op":"rect","x":0,"y":0,"width":10,"height":10,"color":"#ffffff","opacity":128},
        ]}))
        .unwrap();
        assert_eq!(
            d.images["main.bmp"].get_pixel(5, 5),
            &Rgba([128, 128, 128, 255])
        );

        let before = d.snapshot();
        let revision = d.revision;
        let undo = d.undo.len();
        let report = d
            .draw(&json!({"preview":true,"operations":[
                {"op":"rect","x":20,"y":0,"width":8,"height":8,"color":"#ff0000"},
            ]}))
            .unwrap();
        assert_eq!(report["preview"], json!(true));
        assert_eq!(report["bounds"], json!([20, 0, 27, 7]));
        let image = d.preview.take().expect("a preview answers with an image");
        assert_eq!(image.get_pixel(24, 4), &Rgba([255, 0, 0, 255]));
        assert!(d.snapshot() == before, "a preview changes nothing");
        assert_eq!((d.revision, d.undo.len()), (revision, undo));
        assert_eq!(d.images["main.bmp"].get_pixel(24, 4)[3], 0);
    }

    #[test]
    fn brush_batch_updates_revision_and_history_once() {
        let mut d = document();
        let before = d.snapshot();
        let revision = d.revision;
        d.draw(&json!({"layer":"background","operations":[
            {"op":"rect","x":1,"y":1,"width":200,"height":80,"color":"#123456"},
            {"op":"stamp","x":20,"y":20,"rows":["abba","baab"],"palette":{"a":"#abcdef","b":"#fedcba"}}
        ]})).unwrap();
        assert_eq!(d.revision, revision + 1);
        assert_eq!(d.undo.len(), 1);
        assert!(d.dirty);
        assert!(d.undo());
        assert!(d.snapshot() == before);
        assert!(!d.dirty);
    }
    #[test]
    fn canvas_stroke_maps_to_play_button_and_all_pressed_variants() {
        let mut d = document();
        let before = d.images["cbuttons.bmp"].clone();
        d.draw(&json!({"layer":"play","all_states":true,"operations":[{"op":"line","x":40,"y":90,"x2":44,"y2":90,"color":"#123456"}]})).unwrap();
        for y in [2, 20] {
            for x in 24..=28 {
                assert_eq!(
                    d.images["cbuttons.bmp"].get_pixel(x, y).0,
                    [18, 52, 86, 255]
                );
            }
        }
        assert_eq!(
            d.images["cbuttons.bmp"].get_pixel(0, 0),
            before.get_pixel(0, 0)
        );
        assert!(d.undo());
        assert_eq!(d.images["cbuttons.bmp"], before);
        assert!(d.redo());
        assert_eq!(
            d.images["cbuttons.bmp"].get_pixel(24, 2).0,
            [18, 52, 86, 255]
        );
    }
    #[test]
    fn every_volume_frame_receives_the_same_local_pixel() {
        let mut d = document();
        d.draw(&json!({"layer":"volume.track","all_states":true,"operations":[{"op":"pixel","x":110,"y":58,"color":"#192837"}]})).unwrap();
        for frame in 0..28 {
            assert_eq!(
                d.images["volume.bmp"].get_pixel(3, frame * 15 + 1).0,
                [25, 40, 55, 255]
            );
        }
    }
    #[test]
    fn shared_eq_frames_and_heads_cover_both_rows_of_the_atlas() {
        let mut d = document();
        d.state(json!({"panel":"equalizer","eq":vec![14;11]}))
            .unwrap();
        d.draw(&json!({"layer":"band1.track","all_states":true,"operations":[{"op":"pixel","x":79,"y":40,"color":"#aabbcc"}]})).unwrap();
        for frame in 0..28 {
            let x = 14 + (frame % 14) * 15;
            let y = if frame < 14 { 166 } else { 231 };
            assert_eq!(
                d.images["eqmain.bmp"].get_pixel(x, y).0,
                [170, 187, 204, 255]
            );
        }
    }
    #[test]
    fn failed_multi_operation_transaction_is_atomic() {
        let mut d = document();
        let before = d.archive().unwrap();
        let rev = d.revision;
        assert!(d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"},{"op":"unknown"}]})).is_err());
        assert_eq!(d.archive().unwrap(), before);
        assert_eq!(d.revision, rev);
        assert!(!d.dirty);
        assert_eq!(d.undo.len(), 0);
    }
    #[test]
    fn renderer_pixel_and_inverse_mapping_agree_at_all_28_positions() {
        let mut d = document();
        for frame in 0..28 {
            d.state(json!({"volume":frame,"balance":frame,"position":frame,"scroll":frame,"eq":vec![frame;11]})).unwrap();
            for panel in ["main", "equalizer", "playlist"] {
                d.state(json!({"panel":panel})).unwrap();
                let rendered = d.render();
                for l in d.layers() {
                    let x = l.destination[0];
                    let y = l.destination[1];
                    assert!(l.map(x, y).is_some());
                    let im = &d.images[&l.sheet];
                    assert!(
                        l.source[0] + l.source[2] <= im.width(),
                        "{} {:?}",
                        l.id,
                        l.source
                    );
                    assert!(
                        l.source[1] + l.source[3] <= im.height(),
                        "{} {:?}",
                        l.id,
                        l.source
                    );
                    assert!(x < rendered.width() && y < rendered.height());
                }
            }
        }
    }
    #[test]
    fn exported_skin_round_trips_through_production_loader() {
        let mut d = document();
        d.draw(&json!({"operations":[{"op":"pixel","x":18,"y":90,"color":"#cc44aa"}]}))
            .unwrap();
        let bytes = d.archive().unwrap();
        crate::winamp::skin::load_skin(&bytes).unwrap();
        let restored = Document::open(&bytes, None).unwrap();
        assert_eq!(d.images, restored.images);
    }
    #[test]
    fn a_stroke_shows_on_every_pixel_of_the_joined_canvas() {
        // The editor's own promise: drag anywhere on the whole skin and the
        // pixel under the brush changes. Asserted on `render` -- what the user
        // is looking at -- not on the atlases, because a write can be perfectly
        // correct and still invisible.
        let mut d = document();
        d.state(json!({"panel":"canvas","layer":"auto","zoom":1}))
            .unwrap();
        let (w, h) = d.canvas_size();
        // Not #ff00ff: that is the classic transparency key and the renderer
        // drops it on purpose.
        let ink = [18, 255, 52, 255];
        for y in 0..h {
            d.checkpoint();
            let _ = d.paint_line(
                [0, y as i32],
                [w as i32 - 1, y as i32],
                ink,
                "selection",
                Scope::Current.into(),
            );
            d.finish_stroke();
        }
        let im = d.render();
        // The playlist interior is a palette colour in the classic format, not
        // artwork, so only its frame can hold a stroke until "List canvas" is on.
        let list_fill = 252..(h - 38);
        for y in 0..h {
            let seen = (0..w).filter(|&x| im.get_pixel(x, y).0 == ink).count();
            if list_fill.contains(&y) {
                assert!(seen > 0, "row {y} of the playlist frame took nothing");
            } else {
                assert_eq!(seen, w as usize, "row {y} did not take a full-width stroke");
            }
        }
    }

    #[test]
    fn auto_paints_every_sprite_under_the_brush_not_only_the_top_one() {
        let mut d = document();
        let original = d.snapshot();
        assert!(d.view.layers.is_empty(), "no explicit target means Auto");
        d.checkpoint();
        d.paint_line(
            [40, 90],
            [40, 90],
            [4, 5, 6, 255],
            "selection",
            Scope::Current.into(),
        )
        .unwrap();
        // The control the brush is over, and the window background underneath
        // it: one stroke on the joined canvas is one stroke on the artwork, so
        // it cannot stop at whichever sprite happens to be drawn last.
        assert_eq!(d.images["cbuttons.bmp"].get_pixel(24, 2).0, [4, 5, 6, 255]);
        assert_eq!(d.images["main.bmp"].get_pixel(40, 90).0, [4, 5, 6, 255]);
        assert_ne!(
            original.images["main.bmp"].get_pixel(40, 90).0,
            [4, 5, 6, 255]
        );
        d.undo();
        assert_eq!(d.images["main.bmp"], original.images["main.bmp"]);
        assert_eq!(d.images["cbuttons.bmp"], original.images["cbuttons.bmp"]);
    }
    #[test]
    fn selected_layers_paint_occluded_background_and_both_control_states() {
        let mut d = document();
        let original = d.snapshot();
        d.state(json!({"layers":["background","play","play"],"all_states":true}))
            .unwrap();
        assert_eq!(d.view.layers, vec!["background", "play"]);
        d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"}]}))
            .unwrap();
        assert_eq!(d.images["main.bmp"].get_pixel(40, 90).0, [18, 52, 86, 255]);
        for y in [2, 20] {
            assert_eq!(
                d.images["cbuttons.bmp"].get_pixel(24, y).0,
                [18, 52, 86, 255]
            );
        }
        assert_eq!(d.images["titlebar.bmp"], original.images["titlebar.bmp"]);
        d.undo();
        assert!(!d.dirty);
        d.state(json!({"layers":["play"]})).unwrap();
        d.checkpoint();
        d.paint_line(
            [40, 90],
            [40, 90],
            [4, 5, 6, 255],
            "selection",
            Scope::Current.into(),
        )
        .unwrap();
        assert_eq!(d.images["main.bmp"], original.images["main.bmp"]);
        assert_eq!(d.images["cbuttons.bmp"].get_pixel(24, 2).0, [4, 5, 6, 255]);
    }
    #[test]
    fn selection_validation_and_draw_override_are_atomic() {
        let mut d = document();
        d.state(json!({"layers":["background","play"]})).unwrap();
        assert!(d.state(json!({"layers":["play","invalid"]})).is_err());
        assert_eq!(d.view.layers, vec!["background", "play"]);
        let before = d.snapshot();
        assert!(d.draw(&json!({"layers":["play"],"operations":[{"x":40,"y":90,"color":"#123456"},{"op":"invalid"}]})).is_err());
        assert!(before == d.snapshot());
        assert_eq!(d.view.layers, vec!["background", "play"]);
        d.draw(&json!({"layers":["play"],"operations":[{"x":40,"y":90,"color":"#123456"}]}))
            .unwrap();
        assert_eq!(d.images["main.bmp"], before.images["main.bmp"]);
        assert_eq!(d.view.layers, vec!["background", "play"]);
        d.state(json!({"panel":"equalizer"})).unwrap();
        assert!(d.view.layers.is_empty());
    }
    #[test]
    fn history_navigation_branching_and_saved_state() {
        let mut d = document();
        let initial = d.snapshot();
        for color in ["#111111", "#222222", "#333333"] {
            d.draw(&json!({"label":color,"operations":[{"x":40,"y":90,"color":color}]}))
                .unwrap();
        }
        d.history_goto(1).unwrap();
        let temp = std::env::temp_dir().join(format!("cranamp-history-{}.wsz", std::process::id()));
        d.export(&temp).unwrap();
        assert!(!d.dirty);
        d.history_goto(3).unwrap();
        assert!(d.dirty);
        d.history_goto(1).unwrap();
        assert!(!d.dirty);
        d.history_goto(0).unwrap();
        assert!(initial == d.snapshot());
        assert!(d.dirty);
        d.history_goto(1).unwrap();
        d.draw(&json!({"label":"New branch","operations":[{"x":40,"y":90,"color":"#445566"}]}))
            .unwrap();
        let history = d.history();
        assert_eq!(history["cursor"], 2);
        assert_eq!(history["entries"].as_array().unwrap().len(), 3);
        assert_eq!(history["entries"][2]["source"], "MCP");
        assert_eq!(history["entries"][2]["label"], "New branch");
        assert!(!d.redo());
        assert!(d.history_goto(3).is_err());
        std::fs::remove_file(temp).unwrap();
    }
    #[test]
    fn empty_human_gesture_preserves_redo_and_does_not_add_history() {
        let mut d = document();
        d.draw(&json!({"operations":[{"x":40,"y":90,"color":"#123456"}]}))
            .unwrap();
        d.undo();
        d.state(json!({"layers":["play"]})).unwrap();
        d.checkpoint();
        d.paint_line(
            [0, 0],
            [2, 2],
            [1, 2, 3, 255],
            "selection",
            Scope::Current.into(),
        )
        .unwrap();
        d.finish_stroke();
        assert_eq!(d.history()["cursor"], 0);
        assert!(!d.dirty);
        assert!(d.redo());
        d.checkpoint();
        d.paint_line(
            [40, 90],
            [41, 90],
            [1, 2, 3, 255],
            "selection",
            Scope::Current.into(),
        )
        .unwrap();
        d.paint_line(
            [41, 90],
            [43, 90],
            [1, 2, 3, 255],
            "selection",
            Scope::Current.into(),
        )
        .unwrap();
        d.finish_stroke();
        assert_eq!(d.history()["cursor"], 2);
        assert_eq!(d.history()["entries"][2]["source"], "Human");
    }
    #[test]
    fn visualizer_palette_round_trips_and_shares_history() {
        let mut d = document();
        let original = d.archive().unwrap();
        assert!(d
            .visualizer_palette(&json!({"colors":["#123456"]}))
            .is_err());
        assert_eq!(d.archive().unwrap(), original);
        d.visualizer_palette(&json!({"colors":vec!["#123456";24]}))
            .unwrap();
        let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
        assert_eq!(skin.viscolor.0, [[18, 52, 86, 255]; 24]);
        assert_eq!(d.history()["entries"][1]["label"], "Visualizer colors");
        d.undo();
        assert_eq!(d.archive().unwrap(), original);
    }
    #[test]
    fn footer_layout_round_trips_without_changing_classic_defaults() {
        use crate::winamp::skin::FooterLayout;
        let mut d = document();
        assert_eq!(d.layout().footer, FooterLayout::Classic);
        let original = d.archive().unwrap();
        assert!(d.set_layout(&json!({"footer":"unknown"}), "MCP").is_err());
        assert_eq!(original, d.archive().unwrap());
        d.set_layout(&json!({"footer":"time-total"}), "Human")
            .unwrap();
        let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
        assert_eq!(skin.layout.footer, FooterLayout::TimeTotal);
        assert_eq!(d.history()["entries"][1]["source"], "Human");
        d.undo();
        assert_eq!(d.layout().footer, FooterLayout::Classic);
        assert_eq!(original, d.archive().unwrap());
    }
    #[test]
    fn color_parser_rejects_non_ascii_without_panicking() {
        assert!(parse_color("#ééé").is_err());
        assert_eq!(parse_color("transparent").unwrap(), [255, 0, 255, 0]);
    }
    #[test]
    fn playlist_canvas_draws_and_round_trips_with_shared_history() {
        let mut d = document();
        let original = d.archive().unwrap();
        d.state(json!({"panel":"playlist"})).unwrap();
        assert!(!d.layers().iter().any(|l| l.id == "list.background"));
        d.set_playlist_background(true, "Human");
        d.state(json!({"layer":"list.background"})).unwrap();
        d.draw(&json!({"operations":[{"op":"pixel","x":12,"y":20,"color":"#123456"}]}))
            .unwrap();
        assert_eq!(d.render().get_pixel(12, 20).0, [18, 52, 86, 255]);
        let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
        let bitmap = skin.playlist_background.unwrap();
        assert_eq!(&bitmap.pixels()[..4], &[18, 52, 86, 255]);
        assert_eq!(d.history()["entries"][1]["source"], "Human");
        assert_eq!(d.history()["entries"][2]["source"], "MCP");
        d.undo();
        d.undo();
        assert!(!d.has_playlist_background());
        assert_eq!(original, d.archive().unwrap());
        // A selected optional layer disappearing through undo must remain safe.
        let _ = d.state_sheet(None);
        d.state(json!({"pressed":true})).unwrap();
        d.redo();
        d.redo();
        d.set_playlist_background(false, "MCP");
        assert_eq!(d.view.layer, "auto");
        d.undo();
        assert_eq!(d.images["plbg.bmp"].get_pixel(0, 0).0, [18, 52, 86, 255]);
    }
    #[test]
    fn playlist_canvas_rejects_non_native_dimensions() {
        let mut d = document();
        d.set_playlist_background(true, "MCP");
        d.images.insert("plbg.bmp".into(), RgbaImage::new(486, 406));
        assert!(crate::winamp::skin::load_skin(&d.archive().unwrap()).is_err());
    }
    #[test]
    fn eq_travel_round_trips_and_keeps_pencil_mapping_on_native_pixels() {
        let mut d = document();
        d.set_layout(&json!({"eq_travel":40}), "Human").unwrap();
        d.set_layout(&json!({"footer":"time-total"}), "MCP")
            .unwrap();
        assert_eq!(d.layout().eq_travel, 40);
        for invalid in [0, 53] {
            assert!(d.set_layout(&json!({"eq_travel":invalid}), "MCP").is_err());
        }
        for frame in 0..28 {
            d.state(json!({"panel":"equalizer","layer":"band1.thumb","eq":vec![frame;11]}))
                .unwrap();
            let thumb = d
                .layers()
                .into_iter()
                .find(|l| l.id == "band1.thumb")
                .unwrap();
            let y = 38 + (40.0 * (1.0 - frame as f32 / 27.0)).round() as u32;
            assert_eq!(thumb.destination, [79, y, 11, 11]);
            assert!(!thumb.stretched());
            assert_eq!(thumb.map(79, y), Some((0, 0)));
        }
        let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
        assert_eq!(skin.layout.eq_travel, 40);
        assert_eq!(
            skin.layout.footer,
            crate::winamp::skin::FooterLayout::TimeTotal
        );
        d.undo();
        assert_eq!(d.layout().eq_travel, 40);
        d.undo();
        assert_eq!(d.layout().eq_travel, 52);
    }
    #[test]
    fn pressed_player_preview_uses_pressed_art_without_mutating_the_document() {
        let mut d = document();
        d.state(json!({"panel":"main","layer":"play","pressed":true}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#123456"}]}))
            .unwrap();
        let original = d.archive().unwrap();
        let history = d.history();
        let preview = Document::open(&d.preview_archive().unwrap(), None).unwrap();
        assert_eq!(
            preview.images["cbuttons.bmp"].get_pixel(24, 2).0,
            [18, 52, 86, 255]
        );
        assert_eq!(d.archive().unwrap(), original);
        assert_eq!(d.history(), history);
        d.state(json!({"pressed":false,"active":true})).unwrap();
        let preview = Document::open(&d.preview_archive().unwrap(), None).unwrap();
        assert_eq!(
            preview.images["cbuttons.bmp"].get_pixel(24, 2),
            d.images["cbuttons.bmp"].get_pixel(24, 2)
        );
        assert_eq!(d.archive().unwrap(), original);
    }
    #[test]
    fn independent_eq_handles_paint_per_band_and_preview_pressed_without_mutation() {
        let mut d = Document::blank();
        d.set_eq_handles(true, "MCP").unwrap();
        assert_eq!(d.layout().eq_travel, 38);
        assert!(d.set_layout(&json!({"eq_travel":39}), "MCP").is_err());
        d.state(json!({"panel":"equalizer","layer":"band1.thumb","eq":vec![27;11],"pressed":true}))
            .unwrap();
        let layer = d
            .layers()
            .into_iter()
            .find(|l| l.id == "band1.thumb")
            .unwrap();
        assert_eq!(layer.destination, [78, 38, 14, 25]);
        d.draw(&json!({"operations":[{"op":"pixel","x":82,"y":54,"color":"#123456"}]}))
            .unwrap();
        assert_eq!(
            d.images["eqhandles.bmp"].get_pixel(18, 41).0,
            [18, 52, 86, 255]
        );
        assert_eq!(d.images["eqhandles.bmp"].get_pixel(32, 41).0, [0; 4]);
        let archive = d.archive().unwrap();
        let preview = Document::open(&d.preview_archive().unwrap(), None).unwrap();
        assert_eq!(
            preview.images["eqhandles.bmp"].get_pixel(18, 16).0,
            [18, 52, 86, 255]
        );
        assert_eq!(d.archive().unwrap(), archive);
        let loaded = crate::winamp::skin::load_skin(&archive).unwrap();
        assert!(loaded.eq_handles.is_some());
        d.undo();
        assert_eq!(d.images["eqhandles.bmp"].get_pixel(18, 41).0, [0; 4]);
        d.redo();
        assert_eq!(
            d.images["eqhandles.bmp"].get_pixel(18, 41).0,
            [18, 52, 86, 255]
        );
    }
    #[test]
    fn native_selection_art_and_glass_visualizer_export_and_undo() {
        let mut d = Document::blank();
        d.set_playlist_selection(true, "Human");
        d.set_layout(&json!({"visualizer_glass":true}), "Human")
            .unwrap();
        d.state(json!({"panel":"playlist","layer":"list.selection"}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"pixel","x":14,"y":24,"color":"#765432"}]}))
            .unwrap();
        assert_eq!(
            d.images["plselection.bmp"].get_pixel(2, 3).0,
            [118, 84, 50, 255]
        );
        let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
        assert!(skin.layout.visualizer_glass);
        assert_eq!(skin.playlist_selection.unwrap().width(), 243);
        d.undo();
        assert_eq!(d.images["plselection.bmp"].get_pixel(2, 3).0, [0; 4]);
        d.redo();
        d.set_playlist_selection(false, "MCP");
        assert_eq!(d.view.layer, "auto");
        d.undo();
        assert_eq!(
            d.images["plselection.bmp"].get_pixel(2, 3).0,
            [118, 84, 50, 255]
        );
    }
    #[test]
    fn whole_skin_canvas_keeps_definitions_and_maps_cross_panel_history() {
        let mut d = Document::blank();
        let archive = d.archive().unwrap();
        d.state(json!({"panel":"canvas","preview_playlist_height":145,"layers":["main.background","equalizer.background"]})).unwrap();
        assert_eq!(d.canvas_size(), (275, 377));
        assert_eq!(
            d.archive().unwrap(),
            archive,
            "An editor view must not change skin definitions"
        );
        assert!(d.layers().iter().all(|l| !l.stretched()));
        d.draw(&json!({"operations":[{"op":"line","x":100,"y":113,"x2":100,"y2":118,"color":"#abcdef"}]})).unwrap();
        assert_eq!(
            d.images["main.bmp"].get_pixel(100, 114).0,
            [171, 205, 239, 255]
        );
        assert_eq!(
            d.images["eqmain.bmp"].get_pixel(100, 0).0,
            [171, 205, 239, 255]
        );
        assert_eq!(
            d.images["eqmain.bmp"].get_pixel(100, 2).0,
            [171, 205, 239, 255]
        );
        d.undo();
        assert_eq!(d.archive().unwrap(), archive);
        d.redo();
        let skin = crate::winamp::skin::load_skin(&d.archive().unwrap()).unwrap();
        assert_eq!(skin.main.width(), 275);
        assert_eq!(skin.eqmain.height(), 315);
        assert!(!d.files.contains_key("cranamp.json"));
        assert!(!d.files.contains_key("canvas.bmp"));
        d.state(json!({"layers":["playlist.top.tile"],"all_states":true}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"pixel","x":51,"y":236,"color":"#123456"}]}))
            .unwrap();
        assert_eq!(
            d.images["pledit.bmp"].get_pixel(128, 4).0,
            [18, 52, 86, 255]
        );
        assert_eq!(
            d.images["pledit.bmp"].get_pixel(128, 25).0,
            [18, 52, 86, 255]
        );
        let layers = d.layers();
        let scroll = layers
            .iter()
            .find(|l| l.id == "playlist.scroll.thumb")
            .unwrap();
        assert_eq!(scroll.destination, [260, 252, 8, 18]);
        let footer = layers
            .iter()
            .find(|l| l.id == "playlist.bottom.left")
            .unwrap();
        assert_eq!(footer.destination, [0, 339, 125, 38]);
    }
    #[test]
    fn thirty_three_independent_planes_roundtrip_without_flattening() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        for i in 1..=33 {
            d.paint_layer_command(
                &json!({"action":"add","name":format!("Detail {i}")}),
                "Human",
            )
            .unwrap();
        }
        d.draw(&json!({"operations":[{"op":"pixel","x":20,"y":30,"color":"#123456"}]}))
            .unwrap();
        assert_eq!(d.view.paint_layer.as_deref(), Some("paint-33"));
        let file = std::env::temp_dir().join(format!(
            "cranamp-33-plane-test-{}.cstudio",
            std::process::id()
        ));
        d.save_project(&file).unwrap();
        let mut reopened = Document::open_project(&std::fs::read(&file).unwrap()).unwrap();
        std::fs::remove_file(file).unwrap();
        assert!(
            reopened.planes == d.planes,
            "Every ID, property and independent pixel plane survives"
        );
        assert_eq!(reopened.planes.len(), 33);
        assert_eq!(
            reopened.planes[32].images["main.bmp"].get_pixel(20, 30).0,
            [18, 52, 86, 255]
        );
        assert_eq!(
            reopened.images["main.bmp"].get_pixel(20, 30)[3],
            0,
            "Project must not flatten paint into original atlas"
        );
        let mut patch = reopened
            .patch(&json!({"action":"inspect","parts":[{"sheet":"main.bmp","rect":[40,40,1,1]}]}))
            .unwrap();
        patch["action"] = json!("apply");
        patch.as_object_mut().unwrap().remove("replace_layer");
        patch.as_object_mut().unwrap().remove("layer_expected");
        patch["name"] = json!("Independent patch 34");
        patch["parts"][0]["operations"] = json!([{"op":"pixel","x":40,"y":40,"color":"#abcdef"}]);
        reopened.patch(&patch).unwrap();
        assert_eq!(reopened.planes[33].id, "paint-34");
        assert_eq!(
            reopened.planes[33].images["main.bmp"].get_pixel(40, 40).0,
            [171, 205, 239, 255]
        );
        assert!(reopened.planes[..33] == d.planes);
        for i in 35..=MAX_PAINT_LAYERS {
            reopened
                .paint_layer_command(
                    &json!({"action":"add","name":format!("Detail {i}")}),
                    "Human",
                )
                .unwrap();
        }
        let before = reopened.snapshot();
        assert!(reopened
            .paint_layer_command(&json!({"action":"add"}), "Human")
            .is_err());
        patch["name"] = json!("Over capacity");
        patch["parts"][0]["expected"] = reopened
            .patch(&json!({"action":"inspect","parts":[{"sheet":"main.bmp","rect":[40,40,1,1]}]}))
            .unwrap()["parts"][0]["expected"]
            .clone();
        assert!(reopened.patch(&patch).is_err());
        assert!(reopened.snapshot() == before);
    }
    #[test]
    fn clipped_planes_preserve_source_pixels_history_export_and_project() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.paint_layer_command(&json!({"action":"add","name":"Silhouette"}), "MCP")
            .unwrap();
        d.draw(&json!({"operations":[{"op":"rect","x":10,"y":10,"width":2,"height":2,"color":"#ffffff"}]})).unwrap();
        d.paint_layer_command(&json!({"action":"add","name":"Shading"}), "MCP")
            .unwrap();
        d.paint_layer_command(&json!({"action":"set","clip_below":true}), "MCP")
            .unwrap();
        d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":32,"height":32,"color":"#123456"}]})).unwrap();
        assert_eq!(d.composite_images()["main.bmp"].get_pixel(0, 0).0, [0; 4]);
        assert_eq!(
            d.composite_images()["main.bmp"].get_pixel(10, 10).0,
            [18, 52, 86, 255]
        );
        assert_eq!(
            d.planes[1].images["main.bmp"].get_pixel(0, 0).0,
            [18, 52, 86, 255],
            "clipping must not erase stored painting"
        );
        d.paint_layer_command(&json!({"action":"set","clip_below":false}), "Human")
            .unwrap();
        assert_eq!(
            d.composite_images()["main.bmp"].get_pixel(0, 0).0,
            [18, 52, 86, 255]
        );
        d.undo();
        let original = d.archive().unwrap();
        d.paint_layer_command(
            &json!({"action":"set","id":"paint-1","visible":false}),
            "Human",
        )
        .unwrap();
        assert_eq!(d.composite_images()["main.bmp"].get_pixel(10, 10).0, [0; 4]);
        d.undo();
        let file = std::env::temp_dir().join(format!(
            "cranamp-clipped-test-{}.cstudio",
            std::process::id()
        ));
        d.save_project(&file).unwrap();
        let reopened = Document::open_project(&std::fs::read(&file).unwrap()).unwrap();
        std::fs::remove_file(file).unwrap();
        assert!(reopened.planes[1].clip_below);
        assert_eq!(reopened.archive().unwrap(), original);
        assert!(reopened.planes == d.planes);
        d.paint_layer_command(&json!({"action":"merge_down"}), "Human")
            .unwrap();
        assert_eq!(d.archive().unwrap(), original);
        d.undo();
        assert!(d.planes[1].clip_below);
        d.paint_layer_command(&json!({"action":"add","name":"Glints"}), "MCP")
            .unwrap();
        d.paint_layer_command(&json!({"action":"set","clip_below":true}), "MCP")
            .unwrap();
        d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":32,"height":32,"color":"#aabbcc"}]})).unwrap();
        assert_eq!(d.composite_images()["main.bmp"].get_pixel(0, 0).0, [0; 4]);
        assert_eq!(
            d.composite_images()["main.bmp"].get_pixel(10, 10).0,
            [170, 187, 204, 255]
        );
        assert!(d
            .paint_layer_command(&json!({"action":"merge_down","id":"paint-2"}), "Human")
            .is_err());
    }
    #[test]
    fn a_hand_stroke_can_make_a_gradient_a_word_and_a_glass_of_its_own() {
        // Every one of these was an engine capability with no control anywhere
        // in either panel, so it existed for a recipe with a Python script and
        // for nobody drawing by hand, on Android, or in a browser.
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();

        // A gradient: the same pixels an MCP ramp puts down.
        d.state(json!({"brush":"rect","filled":true,"color":"#ffffff",
                       "ramp_to":"#000000","ramp_axis":"down"}))
            .unwrap();
        d.checkpoint();
        d.shape_stroke([0, 0], [39, 39]).unwrap();
        d.finish_stroke();
        let top = d.images["main.bmp"].get_pixel(20, 1).0;
        let bottom = d.images["main.bmp"].get_pixel(20, 38).0;
        assert!(top[0] > 200 && bottom[0] < 60, "{top:?} -> {bottom:?}");
        // Off means off, and a flat fill comes back.
        d.state(json!({"ramp_to":null})).unwrap();
        d.checkpoint();
        d.shape_stroke([0, 0], [39, 39]).unwrap();
        d.finish_stroke();
        assert_eq!(
            d.images["main.bmp"].get_pixel(20, 1).0,
            d.images["main.bmp"].get_pixel(20, 38).0
        );

        // A word, in the face the cell has room for.
        d.state(
            json!({"brush":"text","text":"16K","face":"small","text_scale":1,
                       "color":"#ff0000"}),
        )
        .unwrap();
        d.checkpoint();
        d.shape_stroke([50, 50], [50, 50]).unwrap();
        d.finish_stroke();
        let painted = (50..64)
            .flat_map(|x| (50..56).map(move |y| (x, y)))
            .filter(|(x, y)| d.images["main.bmp"].get_pixel(*x, *y).0 == [255, 0, 0, 255])
            .count();
        assert!(painted > 10, "the word landed: {painted} pixels");
        assert_eq!(
            d.images["main.bmp"].get_pixel(64, 52).0[3],
            0,
            "and it fits the fourteen pixels an equalizer caption has"
        );

        // A glass lens with its own bevel rather than one taken from the drag.
        d.state(json!({"brush":"glass","color":"#66ccff","bevel":4,"refraction":0}))
            .unwrap();
        d.checkpoint();
        d.shape_stroke([100, 20], [160, 80]).unwrap();
        d.finish_stroke();
        let narrow = d.archive().unwrap();
        d.undo();
        d.state(json!({"bevel":32})).unwrap();
        d.checkpoint();
        d.shape_stroke([100, 20], [160, 80]).unwrap();
        d.finish_stroke();
        assert_ne!(
            d.archive().unwrap(),
            narrow,
            "a bevel the artist chose has to change the glass"
        );
    }

    #[test]
    fn clean_curve_gui_preview_matches_mcp_and_undo() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","brush":"curve","brush_size":1,"curve_bend":45,"clean_corners":true,"color":"#abcdef"})).unwrap();
        d.paint_layer_command(&json!({"action":"add","name":"Whiskers"}), "Human")
            .unwrap();
        let before = d.archive().unwrap();
        d.checkpoint();
        d.shape_stroke([10, 40], [60, 48]).unwrap();
        d.shape_stroke([10, 40], [30, 48]).unwrap();
        d.finish_stroke();
        let human = d.archive().unwrap();
        assert_ne!(before, human);
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
        d.draw(&json!({"operations":[{"op":"curve","x":10,"y":40,"x2":30,"y2":48,"curve_bend":45,"color":"#abcdef"}]})).unwrap();
        assert_eq!(d.archive().unwrap(), human);
        d.undo();
        d.draw(&json!({"operations":[{"op":"curve","x":10,"y":40,"x2":30,"y2":48,"curve_bend":45,"clean_corners":false,"color":"#abcdef"}]})).unwrap();
        assert_ne!(
            d.archive().unwrap(),
            human,
            "operation overrides shared brush setting"
        );
    }
    #[test]
    fn human_tuft_preview_and_mcp_share_pixels_and_atomic_layer_history() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","brush":"tuft","brush_size":5,"curve_bend":0,"color":"#abcdef"})).unwrap();
        d.paint_layer_command(&json!({"action":"add","name":"Fur"}), "MCP")
            .unwrap();
        let before = d.archive().unwrap();
        d.checkpoint();
        d.shape_stroke([10, 40], [60, 40]).unwrap();
        d.shape_stroke([10, 40], [30, 40]).unwrap();
        assert_eq!(d.composite_images()["main.bmp"].get_pixel(50, 40).0, [0; 4]);
        d.finish_stroke();
        let human = d.archive().unwrap();
        assert_eq!(d.images["main.bmp"].get_pixel(10, 40).0, [0; 4]);
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
        d.draw(&json!({"operations":[{"op":"tuft","x":10,"y":40,"x2":30,"y2":40,"brush_size":5,"curve_bend":0,"color":"#abcdef"}]})).unwrap();
        assert_eq!(d.archive().unwrap(), human);
        assert!(d.draw(&json!({"operations":[{"op":"pixel","x":80,"y":80},{"op":"curve","x":0,"y":0,"curve_bend":101}]})).is_err());
        assert_eq!(
            d.archive().unwrap(),
            human,
            "invalid operations restore all layers atomically"
        );
    }
    #[test]
    fn native_paths_ramps_mirrors_and_shape_preview_share_history() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet","brush":"ellipse","filled":true})).unwrap();
        d.draw(&json!({"operations":[{"op":"path","x":0,"y":0,"points":[[10,10],[10,25,30,25,30,10],[10,10]],"fill":true,"mirror_x":true,"ramp":["#123456","#abcdef"],"ramp_axis":[10,10,30,10]}]})).unwrap();
        assert_eq!(d.images["main.bmp"].get_pixel(10, 10).0, [18, 52, 86, 255]);
        assert_eq!(d.images["main.bmp"].get_pixel(264, 10).0, [18, 52, 86, 255]);
        assert_eq!(
            d.images["main.bmp"].get_pixel(30, 10).0,
            [171, 205, 239, 255]
        );
        assert_eq!(d.images["main.bmp"].get_pixel(20, 30).0, [0; 4]);
        let before = d.archive().unwrap();
        let history = d.undo.len();
        d.checkpoint();
        d.shape_stroke([40, 40], [60, 60]).unwrap();
        assert_ne!(d.images["main.bmp"].get_pixel(50, 50).0, [0; 4]);
        d.shape_stroke([40, 40], [44, 44]).unwrap();
        assert_eq!(
            d.images["main.bmp"].get_pixel(50, 50).0,
            [0; 4],
            "old shape preview must be erased"
        );
        d.finish_stroke();
        assert_eq!(d.undo.len(), history + 1);
        assert_eq!(d.undo.last().unwrap().source, "Human");
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
        d.checkpoint();
        d.shape_stroke([40, 40], [60, 60]).unwrap();
        d.cancel_stroke();
        assert_eq!(d.archive().unwrap(), before);
        assert!(d
            .draw(
                &json!({"operations":[{"op":"path","x":0,"y":0,"points":[[0,0],[5]],"fill":true}]})
            )
            .is_err());
        assert_eq!(d.archive().unwrap(), before, "invalid paths are atomic");
    }
    #[test]
    fn lifted_clusters_preserve_transparency_and_native_history() {
        let mut d = Document::blank();
        d.state(json!({"panel":"main","layers":["background"]}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"stamp","x":30,"y":40,"rows":["ab.",".ba"],"palette":{"a":"#aabbcc","b":"#123456"}}]})).unwrap();
        let before = d.archive().unwrap();
        let history = d.undo.len();
        d.capture_cluster([30, 40, 3, 2]).unwrap();
        assert_eq!(d.cluster.as_ref().unwrap().get_pixel(2, 0).0, [0; 4]);
        assert_eq!(d.archive().unwrap(), before);
        assert_eq!(d.undo.len(), history);
        d.transform_cluster(true, false, 1).unwrap();
        assert_eq!(d.cluster.as_ref().unwrap().dimensions(), (2, 3));
        d.state(json!({"panel":"main","layers":["play"],"all_states":true}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"cluster","x":40,"y":90}]}))
            .unwrap();
        assert_eq!(d.undo.len(), history + 1);
        for y in 0..3 {
            for x in 0..2 {
                let expected = d.cluster.as_ref().unwrap().get_pixel(x, y);
                assert_eq!(d.images["cbuttons.bmp"].get_pixel(24 + x, 2 + y), expected);
                assert_eq!(d.images["cbuttons.bmp"].get_pixel(24 + x, 20 + y), expected);
            }
        }
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
        d.state(json!({"panel":"main","layers":["background"],"brush":"lift"}))
            .unwrap();
        d.checkpoint();
        d.shape_stroke([30, 40], [32, 41]).unwrap();
        d.finish_stroke();
        assert_eq!(d.archive().unwrap(), before);
        assert_eq!(d.undo.len(), history);
        d.state(json!({"brush":"stamp"})).unwrap();
        d.checkpoint();
        d.shape_stroke([50, 40], [50, 40]).unwrap();
        d.shape_stroke([50, 40], [60, 40]).unwrap();
        d.finish_stroke();
        assert_eq!(
            d.images["main.bmp"].get_pixel(50, 40).0,
            [0; 4],
            "old stamp preview erased"
        );
        assert_eq!(
            d.images["main.bmp"].get_pixel(60, 40).0,
            [170, 187, 204, 255]
        );
        assert_eq!(d.undo.last().unwrap().source, "Human");
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
    }
    #[test]
    fn alpha_and_palette_masks_protect_each_source_variant() {
        let mut d = Document::blank();
        d.state(json!({"panel":"main","layer":"play","all_states":true}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"pixel","x":42,"y":92,"color":"#112233"}]}))
            .unwrap();
        let before = d.archive().unwrap();
        d.state(json!({"alpha_lock":true})).unwrap();
        d.draw(&json!({"mask_colors":["#112233"],"operations":[{"op":"rect","x":39,"y":88,"width":23,"height":18,"color":"#abcdef"}]})).unwrap();
        assert!(d.view.mask_colors.is_empty());
        for yy in [4, 22] {
            assert_eq!(
                d.images["cbuttons.bmp"].get_pixel(26, yy).0,
                [171, 205, 239, 255]
            );
            assert_eq!(d.images["cbuttons.bmp"].get_pixel(27, yy).0, [0; 4]);
        }
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
        let rev = d.revision;
        assert!(d
            .draw(&json!({"mask_colors":["invalid"],"operations":[]}))
            .is_err());
        assert_eq!(d.revision, rev);
    }
    #[test]
    fn glass_brush_uses_selected_underlay_and_one_human_undo() {
        let mut d = Document::blank();
        d.state(json!({"panel":"main","layers":["background"]}))
            .unwrap();
        d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#203050"}]})).unwrap();
        let before = d.archive().unwrap();
        let history = d.undo.len();
        d.state(json!({"brush":"glass","color":"#aaccdd"})).unwrap();
        d.checkpoint();
        d.shape_stroke([15, 20], [70, 50]).unwrap();
        d.shape_stroke([15, 20], [60, 45]).unwrap();
        d.finish_stroke();
        assert_eq!(d.undo.len(), history + 1);
        assert_eq!(d.undo.last().unwrap().source, "Human");
        assert_eq!(d.images["main.bmp"].get_pixel(65, 40).0, [32, 48, 80, 255]);
        assert_ne!(d.images["main.bmp"].get_pixel(35, 33).0, [32, 48, 80, 255]);
        d.undo();
        assert_eq!(d.archive().unwrap(), before);
    }
    #[test]
    fn painting_planes_compose_lock_undo_and_roundtrip_as_project() {
        let mut d = Document::blank();
        d.state(json!({"panel":"main","layer":"play","all_states":true}))
            .unwrap();
        let blank = d.archive().unwrap();
        d.paint_layer_command(&json!({"action":"add","name":"Fur"}), "MCP")
            .unwrap();
        let fur = d.view.paint_layer.clone().unwrap();
        d.draw(&json!({"operations":[{"op":"rect","x":39,"y":88,"width":23,"height":18,"color":"#e0d0c0"}]})).unwrap();
        assert_eq!(d.images["cbuttons.bmp"].get_pixel(23, 0).0, [0; 4]);
        assert_eq!(
            d.composite_images()["cbuttons.bmp"].get_pixel(23, 0).0,
            [224, 208, 192, 255]
        );
        assert_eq!(
            d.composite_images()["cbuttons.bmp"].get_pixel(23, 18).0,
            [224, 208, 192, 255]
        );
        d.paint_layer_command(&json!({"action":"set","visible":false}), "Human")
            .unwrap();
        assert_eq!(d.archive().unwrap(), blank);
        d.undo();
        d.paint_layer_command(&json!({"action":"set","locked":true}), "Human")
            .unwrap();
        let locked = d.archive().unwrap();
        assert!(d
            .draw(&json!({"operations":[{"op":"pixel","x":40,"y":90}]}))
            .is_err());
        assert_eq!(d.archive().unwrap(), locked);
        d.undo();
        d.paint_layer_command(&json!({"action":"add","name":"Highlights"}), "MCP")
            .unwrap();
        let top = d.view.paint_layer.clone().unwrap();
        d.draw(&json!({"operations":[{"op":"pixel","x":40,"y":90,"color":"#ffffff"}]}))
            .unwrap();
        let n = d.undo.len();
        d.opacity_stroke(&top, 128).unwrap();
        d.opacity_stroke(&top, 200).unwrap();
        d.finish_opacity_stroke();
        assert_eq!(d.undo.len(), n + 1);
        assert_eq!(d.undo.last().unwrap().label, "Layer opacity");
        let original = d.archive().unwrap();
        let project =
            std::env::temp_dir().join(format!("cranamp-layer-test-{}.cstudio", std::process::id()));
        d.save_project(&project).unwrap();
        let restored = Document::open_project(&std::fs::read(&project).unwrap()).unwrap();
        assert_eq!(restored.archive().unwrap(), original);
        assert_eq!(restored.planes.len(), 2);
        assert_eq!(restored.view.paint_layer, Some(top.clone()));
        std::fs::remove_file(project).unwrap();
        d.paint_layer_command(&json!({"action":"merge_down","id":top}), "MCP")
            .unwrap();
        assert_eq!(d.archive().unwrap(), original);
        assert_eq!(d.view.paint_layer, Some(fur));
        d.undo();
        assert_eq!(d.planes.len(), 2);
        d.paint_layer_command(&json!({"action":"move","id":top,"index":0}), "MCP")
            .unwrap();
        assert_eq!(
            d.composite_images()["cbuttons.bmp"].get_pixel(24, 2).0,
            [224, 208, 192, 255]
        );
    }
    #[test]
    fn the_footer_buttons_that_have_no_sprite_still_say_where_they_are() {
        // The playlist footer's five menus and six transport keys are drawn by
        // the artist and hit-tested by the player, and nothing owns their
        // pixels. Until they were listed here the only way to find them was to
        // read Cranamp's own source, and a cat got laid across the elapsed
        // readout for want of a rectangle.
        let mut d = Document::blank();
        d.state(json!({"panel":"canvas","preview_playlist_height":145}))
            .unwrap();
        let guides = d.guides();
        let add = guides.iter().find(|g| g.id == "hit.playlist.ADD").unwrap();
        assert!(add.hit && !add.runtime);
        assert_eq!(add.rect, [10, 346, 28, 18]);
        let eject = guides
            .iter()
            .find(|g| g.id == "hit.playlist.EJECT")
            .unwrap();
        assert_eq!(eject.rect, [185, 364, 12, 8]);
        // They move with the playlist, like every other footer rectangle.
        let elapsed = guides
            .iter()
            .find(|g| g.id == "runtime.playlist.ELAPSED")
            .unwrap();
        assert_eq!(elapsed.rect[1] + 1, eject.rect[1]);
        assert!(guides.iter().any(|g| g.id == "hit.main.SKIN CHOOSER"));
        // Choosing one clips painting to it rather than targeting a sprite
        // that does not exist.
        d.select_guide("hit.playlist.ADD").unwrap();
        assert_eq!(d.view.clip, Some([10, 346, 28, 18]));
    }
    #[test]
    fn a_skin_that_was_never_drawn_says_so_when_it_is_exported() {
        // Validating through the loader proves the archive parses. A blank skin
        // with one sheet painted exports in six kilobytes and answers exactly
        // as a finished one does.
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#204060"}]}))
            .unwrap();
        let blank = d.undrawn_sprites();
        assert!(blank.contains(&"main.play".to_string()), "{blank:?}");
        assert!(blank.contains(&"equalizer.background".to_string()));
        assert!(
            !blank.contains(&"main.background".to_string()),
            "the one sheet that was painted is not blank"
        );
        let dir = std::env::temp_dir().join("cranamp-undrawn-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blank.wsz");
        let out = d.export(&path).unwrap();
        assert!(
            out["undrawn_sprites"].as_array().unwrap().len() > 50,
            "export has to say so: {out}"
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn ink_that_cannot_be_read_on_its_own_artwork_is_reported() {
        let mut d = Document::blank();
        // A dark main window, and a display ink sampled from text.bmp that is
        // nearly the same colour: the title is there and nobody can read it.
        d.state(json!({"panel":"atlas","sheet":"main.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#202430"}]}))
            .unwrap();
        d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#2a2f3c"}]}))
            .unwrap();
        let title = d
            .readability()
            .into_iter()
            .find(|r| r["reads"] == "the track title")
            .expect("the title is checked");
        assert_eq!(title["readable"], false, "{title}");
        assert!(title["contrast"].as_f64().unwrap() < 1.5, "{title}");
        // Paint the ink pale and the same readout passes.
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#f4ecd4"}]}))
            .unwrap();
        let title = d
            .readability()
            .into_iter()
            .find(|r| r["reads"] == "the track title")
            .unwrap();
        assert_eq!(title["readable"], true, "{title}");
    }

    /// Two cells far apart on one sheet make a box that covers everything
    /// between them, and the box used to be the whole answer to "did this
    /// stroke touch that sprite".
    /// `unsampled_pixels` is "ink nothing will ever show". Erasing a gap is not
    /// ink, and a sheet nothing samples at all is not a sheet with ink in the
    /// wrong place on it -- both used to be counted, and between them they
    /// fired on almost every transaction a recipe makes.
    #[test]
    fn unsampled_pixels_counts_ink_rather_than_clearing_and_never_drawn_sheets() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"numbers.bmp","layer":"sheet"}))
            .unwrap();
        // Clearing the whole sheet to the transparency key is how a recipe
        // starts. numbers.bmp is 99 wide and holds ten 9-pixel cells, so the
        // eleventh is a gap -- and erasing it says nothing about anything.
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":0,"y":0,"width":99,"height":13,"color":"#ff00ff"}]}))
            .unwrap();
        assert_eq!(out["unsampled_pixels"], 0, "{out}");
        // Actual ink in the same gap is still reported, with where.
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":90,"y":0,"width":9,"height":13,"color":"#ffcc00"}]}))
            .unwrap();
        assert_eq!(out["unsampled_pixels"], 117, "{out}");
        assert_eq!(out["unsampled_sample"][0], json!([90, 0]));

        // text.bmp is read for one colour and never drawn, so painting it --
        // the correct thing to do -- reported all 2,790 of its pixels every
        // time. That is a fact about the sheet, and the sheet says it.
        d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
            .unwrap();
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#ffdf9c"}]}))
            .unwrap();
        assert_eq!(out["unsampled_pixels"], 0, "{out}");
        assert_eq!(out["sheet_is_never_drawn"], true, "{out}");

        // And a real gap on a sheet that is drawn still reports: pledit.bmp
        // column 125 falls between the two footer flaps.
        d.state(json!({"panel":"atlas","sheet":"pledit.bmp","layer":"sheet"}))
            .unwrap();
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":0,"y":72,"width":276,"height":38,"color":"#334455"}]}))
            .unwrap();
        assert_eq!(out["unsampled_pixels"], 38, "{out}");
        assert!(out["sheet_is_never_drawn"].is_null());
    }

    #[test]
    fn identical_variants_names_only_sprites_the_stroke_actually_wrote() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"eqmain.bmp","layer":"sheet"}))
            .unwrap();
        // Leave ON and AUTO with two variants each that are the same picture.
        // They live between the close key and the PRESETS plate on this sheet.
        let mut ops: Vec<Value> = Vec::new();
        for x in [10, 36, 69, 95, 128, 154, 187, 213] {
            ops.push(json!({"op":"rect","x":x,"y":119,"width":26,"height":12,
                            "color":"#445566"}));
        }
        d.draw(&json!({ "operations": ops })).unwrap();
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":10,"y":119,"width":4,"height":4,"color":"#889900"}]}))
            .unwrap();
        assert!(
            out["identical_variants"]
                .as_array()
                .is_some_and(|v| v.iter().any(|e| e["id"] == "equalizer.on")),
            "a stroke inside ON's own cell still reports it: {out}"
        );
        // The close key and the PRESETS plate, together, in one transaction.
        // Their bounding box swallows ON and AUTO; neither is written.
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":0,"y":116,"width":9,"height":9,"color":"#112233"},
                {"op":"rect","x":224,"y":164,"width":44,"height":12,"color":"#112233"}]}))
            .unwrap();
        assert!(
            out["identical_variants"].is_null(),
            "the box covers ON and AUTO and the stroke wrote neither: {out}"
        );
    }

    /// Eleven equalizer bands draw their groove from one rectangle, so one
    /// duplicate frame used to be reported eleven times over with eleven
    /// copies of all twenty-eight labels.
    #[test]
    fn sprites_that_share_their_cells_share_one_report() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"eqmain.bmp","layer":"sheet"}))
            .unwrap();
        // Paint every track frame the same, which is the defect being caught.
        let ops: Vec<Value> = (0..28)
            .map(|f| {
                let (x, y) = if f < 14 {
                    (13 + f * 15, 164)
                } else {
                    (13 + (f - 14) * 15, 229)
                };
                json!({"op":"rect","x":x,"y":y,"width":14,"height":63,"color":"#334455"})
            })
            .collect();
        let out = d.draw(&json!({ "operations": ops })).unwrap();
        let same = out["identical_variants"].as_array().expect("a report");
        assert_eq!(same.len(), 1, "one fact, said once: {out}");
        assert_eq!(same[0]["id"], "equalizer.band0.track");
        let also = same[0]["also"].as_array().expect("the other ten");
        assert_eq!(also.len(), 10, "{same:?}");
        assert!(also.contains(&json!("equalizer.band10.track")));
    }

    /// `#ff00ff` erases everywhere else in this engine; `opacity` and glass
    /// read it as magenta, write the result opaque, and say nothing.
    #[test]
    fn a_blend_against_the_transparency_key_is_reported() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":23,"height":18,"color":"#ff00ff"}]}))
            .unwrap();
        let out = d
            .draw(&json!({"operations":[
                {"op":"ellipse","x":4,"y":4,"width":10,"height":10,"color":"#ffcc66",
                 "fill":true,"opacity":120}]}))
            .unwrap();
        let keyed = out["keyed_blends"].as_array().expect("a report");
        assert_eq!(keyed[0]["operation"], 0);
        assert_eq!(keyed[0]["op"], "ellipse");
        assert!(keyed[0]["pixels"].as_u64().unwrap() > 40, "{out}");
        // Over artwork rather than over the key, the same blend is silent.
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":23,"height":18,"color":"#203040"}]}))
            .unwrap();
        let out = d
            .draw(&json!({"operations":[
                {"op":"ellipse","x":4,"y":4,"width":10,"height":10,"color":"#ffcc66",
                 "fill":true,"opacity":120}]}))
            .unwrap();
        assert!(out["keyed_blends"].is_null(), "{out}");
    }

    #[test]
    fn frames_that_came_out_the_same_picture_are_reported() {
        // The bug this exists for: a recipe draws 28 slider frames and forgets
        // to offset each by its own y, so 27 of them keep whatever the last
        // full-width pass left and only frame 0 gets the work. Bounds, clipped,
        // unsampled and overwrites are all clean; the frames are just the same
        // picture now.
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"volume.bmp","layer":"sheet"}))
            .unwrap();
        // Every frame gets a band, and only frame 0 gets the stitches.
        let mut ops: Vec<Value> = (0..28)
            .map(|f| json!({"op":"rect","x":0,"y":f*15,"width":68,"height":13,"color":"#203040"}))
            .collect();
        ops.push(json!({"op":"rect","x":2,"y":3,"width":30,"height":3,"color":"#ffcc00"}));
        let out = d.draw(&json!({ "operations": ops })).unwrap();
        assert_eq!(out["overwrites"], 0, "nothing else reports this");
        assert_eq!(out["clipped_pixels"], 0);
        let same = out["identical_variants"].as_array().expect("a report");
        let track = same
            .iter()
            .find(|v| v["id"] == "main.volume.track")
            .expect("the track");
        assert_eq!(track["of"], 28);
        assert_eq!(track["groups"][0].as_array().unwrap().len(), 27);
        assert_eq!(track["labels"][0], "0 · silent");
        assert_eq!(track["labels"][27], "27 · full volume");
        // And it is silent once every frame is different.
        let ops: Vec<Value> = (0..28)
            .map(|f| json!({"op":"rect","x":2,"y":f*15+3,"width":2+f,"height":3,"color":"#ffcc00"}))
            .collect();
        let out = d.draw(&json!({ "operations": ops })).unwrap();
        assert!(
            out["identical_variants"]
                .as_array()
                .is_none_or(|v| v.iter().all(|s| s["id"] != "main.volume.track")),
            "28 different frames are not a defect: {}",
            out["identical_variants"]
        );
    }

    #[test]
    fn an_unpainted_cell_is_not_a_duplicate() {
        // On a blank sheet every variant is transparent and identical. Saying
        // so on the first stroke would fire on every sprite of every sheet for
        // the whole early part of a skin.
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layer":"sheet"}))
            .unwrap();
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":23,"y":0,"width":23,"height":18,"color":"#2f6b64"}]}))
            .unwrap();
        assert!(
            out["identical_variants"].is_null(),
            "the untouched pressed cell is empty, not a duplicate: {}",
            out["identical_variants"]
        );
        // Paint the pressed cell the same and it is a duplicate, which is the
        // other half of the same defect.
        let out = d
            .draw(&json!({"operations":[
                {"op":"rect","x":23,"y":18,"width":23,"height":18,"color":"#2f6b64"}]}))
            .unwrap();
        let play = out["identical_variants"]
            .as_array()
            .expect("a report")
            .iter()
            .find(|v| v["id"] == "main.play")
            .expect("play");
        assert_eq!(play["groups"][0], json!([0, 1]));
        assert_eq!(play["labels"], json!(["released", "pressed"]));
    }

    #[test]
    fn the_small_face_fits_a_word_in_a_cell_the_large_one_overruns() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"eqmain.bmp","layer":"sheet"}))
            .unwrap();
        let wide = d
            .draw(&json!({"operations":[{"op":"text","x":0,"y":0,"text":"16K","color":"#ffffff"}]}))
            .unwrap();
        assert!(d.undo());
        let narrow = d
            .draw(&json!({"operations":[{"op":"text","x":0,"y":0,"text":"16K","color":"#ffffff","face":"small"}]}))
            .unwrap();
        let width = |v: &Value| v["bounds"][2].as_i64().unwrap() - v["bounds"][0].as_i64().unwrap();
        assert!(width(&wide) > 14, "the 5x7 face overruns a band caption");
        assert!(
            width(&narrow) < 14,
            "the small face has to fit one: {}",
            narrow["bounds"]
        );
        // No lower case at five pixels tall, so a-z are capitals, not gaps.
        let lower = d
            .draw(&json!({"operations":[{"op":"text","x":0,"y":40,"text":"khz","color":"#ffffff","face":"small"}]}))
            .unwrap();
        assert!(
            lower["unsupported_characters"].is_null(),
            "a-z must be drawn as capitals, not reported missing: {}",
            lower["unsupported_characters"]
        );
    }
    #[test]
    fn atlas_guides_cover_states_and_clip_only_the_chosen_cell() {
        let mut d = Document::blank();
        d.state(json!({"panel":"atlas","sheet":"cbuttons.bmp","layer":"sheet"}))
            .unwrap();
        let before = d.archive().unwrap();
        let guides = d.guides();
        let play = guides.iter().find(|g| g.id == "main.play#1").unwrap();
        assert_eq!(play.rect, [23, 18, 23, 18]);
        let artwork = d.render();
        d.state(json!({"guides":true})).unwrap();
        // Rectangles are an overlay the editor draws over the canvas, never
        // pixels in it: outlining every cell in the artwork's own pixels buried
        // the artwork under the hints meant to point at it.
        assert_eq!(
            d.editor_render(),
            artwork,
            "rectangles must not be painted into the picture"
        );
        assert_eq!(d.render(), artwork, "guides must not enter the artwork");
        assert_eq!(d.archive().unwrap(), before);
        d.select_guide(&play.id).unwrap();
        assert_eq!(
            d.archive().unwrap(),
            before,
            "guide selection is not artwork"
        );
        d.draw(&json!({"operations":[{"op":"rect","x":0,"y":0,"width":136,"height":36,"color":"#abcdef"}]})).unwrap();
        assert_eq!(
            d.images["cbuttons.bmp"].get_pixel(23, 18).0,
            [171, 205, 239, 255]
        );
        assert_eq!(d.images["cbuttons.bmp"].get_pixel(22, 18).0, [0; 4]);
        assert_eq!(d.images["cbuttons.bmp"].get_pixel(23, 17).0, [0; 4]);
        d.state(json!({"sheet":"volume.bmp"})).unwrap();
        assert_eq!(d.view.clip, None);
        let guides = d.guides();
        assert_eq!(
            guides
                .iter()
                .filter(|g| g.id.starts_with("main.volume.track#"))
                .count(),
            28
        );
        d.state(json!({"sheet":"main.bmp"})).unwrap();
        assert!(d
            .guides()
            .iter()
            .any(|g| g.runtime && g.rect == [24, 43, 76, 16]));
    }
    #[test]
    fn review_crop_maps_sources_and_identifies_live_timer_pixels_without_mutation() {
        let mut d = Document::blank();
        d.state(json!({"panel":"canvas","pressed":true,"digit":7}))
            .unwrap();
        let before = d.snapshot();
        let view = serde_json::to_value(&d.view).unwrap();
        let rev = d.revision;
        let crop = d.inspect_region([48, 26, 9, 13]).unwrap();
        let parts = crop["parts"].as_array().unwrap();
        assert!(parts.iter().any(|p| p["runtime"] == true
            && p["label"] == "TIMER DIGIT 0"
            && p["source_overlap"].is_null()));
        assert!(parts
            .iter()
            .any(|p| p["sheet"] == "numbers.bmp" && p["source_overlap"] == json!([63, 0, 9, 13])));
        let crop = d.inspect_region([40, 90, 5, 4]).unwrap();
        assert!(crop["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["sheet"] == "cbuttons.bmp" && p["source_overlap"] == json!([24, 20, 5, 4])));
        assert!(d.snapshot() == before);
        assert_eq!(d.revision, rev);
        assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
        assert!(d.inspect_region([274, 0, 2, 1]).is_err());
    }
    #[test]
    fn review_crop_preserves_repeated_tile_source_offsets() {
        let mut d = Document::blank();
        d.state(json!({"panel":"canvas","active":true})).unwrap();
        let crop = d.inspect_region([26, 234, 4, 3]).unwrap();
        assert!(crop["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["sheet"] == "pledit.bmp" && p["source_overlap"] == json!([128, 23, 4, 3])));
    }
    #[test]
    fn dock_row_copies_native_edge_and_inverse_maps_without_resizing() {
        let mut d = Document::blank();
        d.state(json!({"panel":"canvas","layers":["main.docking.edge"]}))
            .unwrap();
        assert!(d.layers().iter().all(|l| !l.stretched()));
        d.draw(&json!({"operations":[{"op":"pixel","x":137,"y":115,"color":"#72aabb"}]}))
            .unwrap();
        assert_eq!(d.images["main.bmp"].dimensions(), (275, 115));
        assert_eq!(
            d.images["main.bmp"].get_pixel(137, 114).0,
            [114, 170, 187, 255]
        );
        let rendered = d.render();
        assert_eq!(rendered.get_pixel(137, 114), rendered.get_pixel(137, 115));
        let crop = d.inspect_region([137, 115, 1, 1]).unwrap();
        assert_eq!(crop["parts"][0]["source_overlap"], json!([137, 114, 1, 1]));
        d.undo();
        assert_eq!(d.images["main.bmp"].get_pixel(137, 114).0, [0; 4]);
    }

    /// The scope between one variant and all of them.
    ///
    /// A slider is twenty-eight frames that differ by one object, and with a
    /// bool there was no way to say "from this frame on": every frame was its
    /// own transaction. The run from the frame in hand to an end is the shape
    /// the artwork actually has, so it is the shape the setting has.
    #[test]
    fn a_stroke_lands_in_a_run_of_variants_and_the_report_says_how_many() {
        let mut d = Document::blank();
        d.open_on_whole_skin();
        d.state(json!({"volume": 20})).unwrap();
        let report = d
            .draw(
                &json!({"layers":["main.volume.track"],"states":"onward","operations":[
                    {"op":"pixel","x":110,"y":58,"color":"#123456"}
                ]}),
            )
            .unwrap();
        assert_eq!(report["states_written"]["scope"], json!("onward"));
        assert_eq!(
            report["states_written"]["variants"]["main.volume.track"],
            json!(8),
            "volume 20 onward is variants 20..27"
        );
        let cells = |d: &Document| {
            (0..28)
                .map(|v| d.images["volume.bmp"].get_pixel(3, v * 15 + 1).0)
                .collect::<Vec<_>>()
        };
        let after = cells(&d);
        assert!(
            after[..20].iter().all(|p| p[3] == 0),
            "nothing below the frame in hand"
        );
        assert!(
            after[20..].iter().all(|p| *p == [0x12, 0x34, 0x56, 255]),
            "the frame in hand and every one after it"
        );

        d.state(json!({"volume": 3})).unwrap();
        let report = d
            .draw(
                &json!({"layers":["main.volume.track"],"states":"up-to","operations":[
                    {"op":"pixel","x":111,"y":58,"color":"#abcdef"}
                ]}),
            )
            .unwrap();
        assert_eq!(
            report["states_written"]["variants"]["main.volume.track"],
            json!(4),
            "volume 3 up-to is variants 0..3"
        );

        // The bool it grew out of still answers, on the way in and on the way
        // back, because scripts were written against it.
        d.state(json!({"all_states": true})).unwrap();
        assert_eq!(d.view.states, SCOPE_ALL);
        assert_eq!(d.brief()["view"]["all_states"], json!(true));
        d.state(json!({"states": SCOPE_CURRENT})).unwrap();
        assert_eq!(d.brief()["view"]["all_states"], json!(false));
        let refusal = d.state(json!({"states": "sometimes"})).unwrap_err();
        assert!(
            refusal.to_string().contains("up-to"),
            "a refusal names the scopes: {refusal}"
        );

        // A scope of one is the old default and says nothing extra.
        let report = d
            .draw(&json!({"layers":["main.volume.track"],"operations":[
                {"op":"pixel","x":112,"y":58,"color":"#010203"}
            ]}))
            .unwrap();
        assert!(report["states_written"].is_null());
    }

    /// Both halves of the readability answer that were never given.
    ///
    /// `TRACK ROWS` sits over the classic playlist fill, which has no bitmap
    /// under it at all, so the ground was None and the whole entry was skipped
    /// -- taking three of the eight checks with it, for every skin that does
    /// not add `plbg.bmp`. And the equalizer curve is drawn in the display ink
    /// over the graph, and was not checked at all: the one readout that is a
    /// picture rather than words.
    #[test]
    fn readability_answers_for_the_playlist_without_plbg_and_for_the_eq_curve() {
        let mut d = Document::blank();
        d.open_on_whole_skin();
        d.state(json!({"panel":"atlas","sheet":"text.bmp","layer":"sheet"}))
            .unwrap();
        d.draw(&json!({"operations":[
            {"op":"rect","x":0,"y":0,"width":155,"height":18,"color":"#101010"}
        ]}))
        .unwrap();
        d.open_on_whole_skin();
        d.draw(&json!({"layers":["equalizer.background"],"operations":[
            {"op":"rect","x":86,"y":133,"width":113,"height":19,"color":"#0c0c0c"}
        ]}))
        .unwrap();
        d.draw(&json!({"layers":["main.background"],"operations":[
            {"op":"rect","x":0,"y":0,"width":275,"height":115,"color":"#f0f0f0"}
        ]}))
        .unwrap();
        d.set_palette(&json!({"Normal":"#0a0a0a","Current":"#ffffff",
                              "NormalBG":"#080808","SelectedBG":"#111111"}))
            .unwrap();
        assert!(!d.images.contains_key("plbg.bmp"), "no playlist background");
        let says = |d: &mut Document, what: &str| -> Value {
            d.readability()
                .into_iter()
                .find(|r| r["reads"] == json!(what))
                .unwrap_or_else(|| panic!("readability never mentioned {what}"))
        };
        assert_eq!(says(&mut d, "a playlist row")["readable"], json!(false));
        assert_eq!(says(&mut d, "the playing row")["readable"], json!(true));
        assert_eq!(says(&mut d, "a selected row")["readable"], json!(false));
        let title = says(&mut d, "the track title");
        let curve = says(&mut d, "the equalizer curve");
        assert_eq!(
            curve["ink"], title["ink"],
            "the curve is written in the display ink, like the title"
        );
        assert_eq!(curve["ground"], json!("#0c0c0c"));
        assert_eq!(curve["readable"], json!(false), "dark ink on a dark graph");
    }

    /// The two shapes a sheet actually has, and the loops they replace.
    ///
    /// A sheet is a grid of cells that mostly hold the same drawing, and a
    /// slider is one shape whose numbers walk across twenty-eight frames.
    /// Without `at` and a swept `[from, to]` the only way to draw either was to
    /// emit the operations N times from outside the editor -- which is a
    /// drawing program living in whatever wrote the loop.
    #[test]
    fn at_repeats_a_transaction_and_a_swept_field_walks_the_variants() {
        let mut d = Document::blank();
        d.open_on_whole_skin();

        // Five transport berths, one operation, no coordinates typed out.
        let report = d
            .draw(&json!({
                "layers":["main.previous","main.play","main.pause","main.stop","main.next"],
                "at":"targets",
                "operations":[{"op":"rect","x":2,"y":2,"width":3,"height":3,"color":"#ff0000"}]
            }))
            .unwrap();
        assert_eq!(report["repeated"]["places"], json!(5));
        assert_eq!(
            report["pixels_written"],
            json!(45),
            "nine pixels, five berths"
        );
        for (x, _) in [(16, 0), (39, 1), (62, 2), (85, 3), (108, 4)] {
            assert_eq!(
                d.render().get_pixel(x + 3, 88 + 3).0,
                [255, 0, 0, 255],
                "berth at {x} should have its own copy"
            );
        }

        // Explicit offsets, and the count is places x operations.
        let report = d
            .draw(&json!({
                "layers":["main.background"],
                "at":[[0,0],[0,2],[0,4]],
                "operations":[{"op":"pixel","x":200,"y":100,"color":"#00ff00"}]
            }))
            .unwrap();
        assert_eq!(report["repeated"]["places"], json!(3));
        for dy in 0..3 {
            assert_eq!(d.render().get_pixel(200, 100 + dy * 2).0, [0, 255, 0, 255]);
        }

        // One call, twenty-eight frames, an endpoint that moves.
        let mut d = Document::blank();
        d.open_on_whole_skin();
        let report = d
            .draw(&json!({
                "layers":["main.balance.track"],"states":"all",
                "operations":[{"op":"pixel","x":[178,213],"y":60,"color":"#0000ff"}]
            }))
            .unwrap();
        assert_eq!(report["swept"]["steps"], json!(28));
        assert_eq!(report["swept"]["fields"], json!(["x"]));
        assert_eq!(
            report["states_written"]["variants"]["main.balance.track"],
            json!(28),
            "the scope's own count, not one per pass"
        );
        let blue = |d: &mut Document, frame: u8| {
            d.state(json!({ "balance": frame })).unwrap();
            let im = d.render();
            (177..215)
                .find(|x| im.get_pixel(*x, 60).0 == [0, 0, 255, 255])
                .unwrap_or_else(|| panic!("frame {frame} has no swept pixel"))
        };
        assert_eq!(blue(&mut d, 0), 178, "the first frame is where it starts");
        assert_eq!(blue(&mut d, 27), 213, "the last frame is where it ends");
        let middle = blue(&mut d, 13);
        assert!(
            (194..=197).contains(&middle),
            "frame 13 is about halfway, not {middle}"
        );

        // A sweep needs a target to count variants on, and more than one.
        let refusal = d
            .draw(&json!({"operations":[{"op":"pixel","x":[1,9],"y":1,"color":"#ffffff"}]}))
            .unwrap_err();
        let refusal = format!("{refusal:#}");
        assert!(refusal.contains("named target"), "{refusal}");
        let refusal = d
            .draw(&json!({"layers":["main.background"],"states":"all",
                          "operations":[{"op":"pixel","x":[1,9],"y":1,"color":"#ffffff"}]}))
            .unwrap_err();
        let refusal = format!("{refusal:#}");
        assert!(refusal.contains("more than one variant"), "{refusal}");
    }

    /// Where a caption starts, from the walk that draws it.
    ///
    /// Centring a word in a cell is (cell - ink) / 2, and the ink of the small
    /// face is 4n + (n-1) -- arithmetic every caller was keeping its own copy
    /// of, beside the `measure` call that exists to answer it.
    #[test]
    fn a_word_can_be_placed_in_a_box_instead_of_at_a_pixel() {
        let mut d = Document::blank();
        d.open_on_whole_skin();
        let ink = crate::winamp::pixel_text::measure("EQ", true, 1, 1).width;
        assert_eq!(ink, 9, "two small cells and one space");
        for (align, expected) in [
            ("left", 20),
            ("center", 20 + (30 - ink) / 2),
            ("right", 20 + 30 - ink),
        ] {
            let mut d = Document::blank();
            d.open_on_whole_skin();
            d.draw(&json!({"layers":["main.background"],"operations":[
                {"op":"text","x":20,"y":60,"width":30,"align":align,
                 "text":"EQ","face":"small","color":"#ffffff"}
            ]}))
            .unwrap();
            let im = d.render();
            let first = (0..275)
                .find(|x| (60..67).any(|y| im.get_pixel(*x, y).0 == [255, 255, 255, 255]))
                .expect("some ink");
            assert_eq!(first as i32, expected, "align {align}");
        }
        let refusal = d
            .draw(&json!({"layers":["main.background"],"operations":[
                {"op":"text","x":20,"y":60,"align":"center","text":"EQ","face":"small"}
            ]}))
            .unwrap_err();
        // The whole chain: the operation index is the outer context and the
        // reason is under it.
        let refusal = format!("{refusal:#}");
        assert!(refusal.contains("width"), "{refusal}");
    }

    /// Ink a control draws its own thumb on top of, in the same state.
    ///
    /// The balance slider in Catamp Freefall was first drawn with a thumb that
    /// filled its cell and a tail whose tip ran to the same value the thumb
    /// does. Both are read from one number, so the tip was behind the thumb in
    /// frame 0, and in frame 27, and in all twenty-six between: twenty-eight
    /// frames of a tail nobody could ever see the end of. Nothing said so --
    /// it was found by looking at the live player and noticing the tip missing.
    #[test]
    fn ink_a_control_hides_under_its_own_thumb_is_reported() {
        let mut d = Document::blank();
        d.open_on_whole_skin();
        // A thumb that fills its cell, as the first one did.
        d.draw(
            &json!({"layers":["main.balance.thumb"],"states":"all","operations":[
                {"op":"rect","x":189,"y":58,"width":14,"height":11,"color":"#c9c2ac"}
            ]}),
        )
        .unwrap();
        // A mark in the track that runs to the same value the thumb does.
        let report = d
            .draw(
                &json!({"layers":["main.balance.track"],"states":"all","operations":[
                    {"op":"pixel","x":[183,207],"y":62,"color":"#5d5d6e"}
                ]}),
            )
            .unwrap();
        let covered = &report["covered_pixels"];
        // Distinct pixels, not frames: the sweep walks 183..207, so twenty-eight
        // steps land on twenty-five places, and every one of them is hidden in
        // every frame it was drawn into.
        assert_eq!(covered["pixels"], json!(25), "{report}");
        assert_eq!(covered["sample"][0]["behind"], json!("main.balance.thumb"));
        assert_eq!(covered["sample"][0]["in"], json!("main.balance.track"));

        // The same mark under a thumb that is transparent there is visible ink
        // and says nothing: the rectangle is not the artwork.
        let mut d = Document::blank();
        d.open_on_whole_skin();
        d.draw(
            &json!({"layers":["main.balance.thumb"],"states":"all","operations":[
                {"op":"rect","x":189,"y":66,"width":14,"height":3,"color":"#c9c2ac"}
            ]}),
        )
        .unwrap();
        let report = d
            .draw(
                &json!({"layers":["main.balance.track"],"states":"all","operations":[
                    {"op":"pixel","x":[183,207],"y":62,"color":"#5d5d6e"}
                ]}),
            )
            .unwrap();
        assert!(
            report["covered_pixels"].is_null(),
            "a three-row grip hides three rows: {report}"
        );

        // A thumb hiding part of its own track in *one* frame is what a slider
        // is. Ink that some frame shows is not invisible artwork.
        let report = d
            .draw(
                &json!({"layers":["main.balance.track"],"states":"all","operations":[
                    {"op":"rect","x":177,"y":66,"width":38,"height":3,"color":"#5d5d6e"}
                ]}),
            )
            .unwrap();
        assert!(
            report["covered_pixels"].is_null(),
            "a bar across the whole track is visible at both ends: {report}"
        );

        // And a window background under a button is how a skin is built.
        let report = d
            .draw(&json!({"layers":["main.background"],"operations":[
                {"op":"rect","x":39,"y":88,"width":23,"height":18,"color":"#101010"}
            ]}))
            .unwrap();
        assert!(report["covered_pixels"].is_null(), "not news: {report}");
    }
}
