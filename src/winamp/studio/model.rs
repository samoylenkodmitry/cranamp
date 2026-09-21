mod audit;
mod continuity;
mod coverage;
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/model/join_tests.rs"]
mod join_tests;
mod patch;
mod transparency;
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
const MAX_PAINT_LAYERS: usize = 64;
pub const SCOPE_CURRENT: &str = "current";
pub const SCOPE_ONWARD: &str = "onward";
pub const SCOPE_UP_TO: &str = "up-to";
pub const SCOPE_ALL: &str = "all";
pub const SCOPES: &[&str] = &[SCOPE_CURRENT, SCOPE_ONWARD, SCOPE_UP_TO, SCOPE_ALL];
pub fn scope_label(scope: &str) -> &'static str {
    match scope {
        SCOPE_ONWARD => "This state onward",
        SCOPE_UP_TO => "Up to this state",
        SCOPE_ALL => "All sprite states",
        _ => "This state only",
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Variants {
    pub scope: Scope,
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
    for key in ["x", "y", "x2", "y2"] {
        if let Some(v) = op.get(key).and_then(Value::as_f64) {
            if v.fract() == 0.0 {
                op[key] = json!(v as i64);
            }
        }
    }
    op
}
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
struct SheetStack<'a> {
    base: &'a RgbaImage,
    planes: Vec<(Option<&'a RgbaImage>, u8, bool)>,
}
impl SheetStack<'_> {
    fn at(&self, x: u32, y: u32) -> Option<Rgba<u8>> {
        let mut out = *self.base.get_pixel_checked(x, y)?;
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
type AtlasPixel = (usize, u32, u32);
fn grain_at(x: i32, y: i32, size: i32, seed: i64, amplitude: f64) -> f64 {
    fn value(cx: i32, cy: i32, seed: i64) -> f64 {
        let mut h = (cx as i64)
            .wrapping_mul(0x27d4_eb2d)
            .wrapping_add((cy as i64).wrapping_mul(0x1656_67b1))
            .wrapping_add(seed.wrapping_mul(0x2545_f491));
        h ^= h >> 15;
        h = h.wrapping_mul(0x2545_f491_4f6c_dd1d);
        h ^= h >> 17;
        ((h >> 24) & 0xffff) as f64 / 32767.5 - 1.0
    }
    let size = size.max(1);
    let (cx, cy) = (x.div_euclid(size), y.div_euclid(size));
    let mixed =
        (value(cx, cy, seed) * 2.0 + value(cx + 1, cy, seed) + value(cx, cy + 1, seed)) / 4.0;
    mixed * amplitude
}
type AtlasInk = ([u8; 4], [i32; 2]);
type AtlasWrite = (AtlasPixel, [u8; 4], [i32; 2]);
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct View {
    pub panel: String,
    pub sheet: String,
    pub layer: String,
    pub layers: Vec<String>,
    pub zoom: u32,
    pub preview_playlist_height: u32,
    pub presentation: bool,
    pub color: String,
    pub brush_size: u32,
    pub brush: String,
    pub curve_bend: i32,
    pub grain: u32,
    pub grain_size: u32,
    pub opacity: u32,
    pub clean_corners: bool,
    pub ramp_to: Option<String>,
    pub ramp_axis: String,
    pub bevel: u32,
    pub refraction: u32,
    pub text: String,
    pub face: String,
    pub text_scale: u32,
    pub text_spacing: i32,
    pub drawer: String,
    pub filled: bool,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub grid: bool,
    pub alpha_lock: bool,
    pub mask_colors: Vec<String>,
    pub states: String,
    pub stamp_repeat: u32,
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
    pub preview: Option<RgbaImage>,
    unmapped_pixels: BTreeSet<[i32; 2]>,
    stroke_pixels: usize,
    painted_bounds: Option<[i32; 4]>,
    clipped_pixels: usize,
    atlas_writes: BTreeMap<AtlasPixel, AtlasInk>,
    stroke_intent: BTreeMap<[i32; 2], [u8; 4]>,
    operation_box: Option<(usize, [u32; 4])>,
    overwrites: usize,
    overwrite_note: Option<String>,
    scope_writes: BTreeMap<String, usize>,
    covered_probe: Vec<(String, usize, [i32; 2])>,
    control_parts: BTreeSet<String>,
}
fn cursor_role(role: &str) -> Result<(crate::winamp::cursors::SkinCursor, &'static str)> {
    let lower = role.to_ascii_lowercase();
    let stem = lower.strip_suffix(".cur").unwrap_or(lower.as_str());
    crate::winamp::cursors::SkinCursor::files()
        .into_iter()
        .find(|(_, name)| name.trim_end_matches(".cur") == stem)
        .with_context(|| format!("Unknown cursor region {role}"))
}

fn cursor_roles(
    roles: &[String],
) -> Result<Vec<(crate::winamp::cursors::SkinCursor, &'static str)>> {
    if roles.is_empty() {
        return Ok(crate::winamp::cursors::SkinCursor::files().to_vec());
    }
    roles.iter().map(|role| cursor_role(role)).collect()
}

/// The archive entry a removal names. Wider than [`cursor_role`]: any file in
/// the classic vocabulary can be dropped, including the vestigial ones like
/// `volbar.cur` that skins carry but no player reads.
fn cursor_file(role: &str) -> Result<&'static str> {
    let lower = role.to_ascii_lowercase();
    let stem = lower.strip_suffix(".cur").unwrap_or(lower.as_str());
    crate::winamp::cursors::CLASSIC_CURSORS
        .into_iter()
        .find(|name| name.trim_end_matches(".cur") == stem)
        .with_context(|| format!("Unknown cursor {role}"))
}

fn cursor_files_to_remove(roles: &[String]) -> Result<Vec<&'static str>> {
    if roles.is_empty() {
        return Ok(crate::winamp::cursors::SkinCursor::files()
            .iter()
            .map(|(_, name)| *name)
            .collect());
    }
    roles.iter().map(|role| cursor_file(role)).collect()
}

fn cursor_image(name: &str, data: &[u8]) -> Option<RgbaImage> {
    if !name.ends_with(".cur") {
        return None;
    }
    let icon = crate::winamp::cursors::decode_cur(data).ok()?;
    let bitmap = icon.image();
    RgbaImage::from_raw(bitmap.width(), bitmap.height(), bitmap.pixels().to_vec())
}

fn cursor_hotspot(data: &[u8]) -> [u32; 2] {
    crate::winamp::cursors::decode_cur(data)
        .map(|icon| [icon.hotspot_x(), icon.hotspot_y()])
        .unwrap_or_default()
}

fn archive_entry(name: &str, data: &[u8], image: Option<&RgbaImage>) -> Result<Vec<u8>> {
    let Some(image) = image else {
        return Ok(data.to_vec());
    };
    if name.ends_with(".cur") {
        let [x, y] = cursor_hotspot(data);
        return crate::winamp::cursors::encode_cur(
            image.width(),
            image.height(),
            image.as_raw(),
            x,
            y,
        );
    }
    let mut opaque = image.clone();
    for pixel in opaque.pixels_mut() {
        if pixel.0[3] < 128 {
            pixel.0 = [0, 0, 0, 255];
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(opaque)
        .to_rgb8()
        .write_to(&mut bytes, image::ImageFormat::Bmp)?;
    Ok(bytes.into_inner())
}

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
fn contrast_ratio(a: [u8; 4], b: [u8; 4]) -> f64 {
    let (x, y) = (relative_luminance(a), relative_luminance(b));
    ((x.max(y) + 0.05) / (x.min(y) + 0.05) * 10.).round() / 10.
}
impl Document {
    pub fn blank() -> Self {
        let mut images = BTreeMap::new();
        let mut files = BTreeMap::new();
        for (name, w, h) in [
            ("main.bmp", 275, 116),
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
            stroke_intent: BTreeMap::new(),
            operation_box: None,
            preview: None,
            overwrites: 0,
            overwrite_note: None,
            scope_writes: BTreeMap::new(),
            covered_probe: Vec::new(),
            control_parts: BTreeSet::new(),
        }
    }
    pub fn open(bytes: &[u8], path: Option<String>) -> Result<Self> {
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
                let image = image::load_from_memory(&data)?.to_rgba8();
                images.insert(name.clone(), image);
            } else if let Some(image) = cursor_image(&name, &data) {
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
            stroke_intent: BTreeMap::new(),
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
                    "{} pixels have no bitmap source; the classic playlist fill is one flat PLEDIT.TXT colour, not artwork — turn on “Playlist has its own background” in Skin options to paint there",
                    self.unmapped_pixels.len()
                );
                self.revision += 1;
            }
            let review = self.stroke_continuity(self.view.states == SCOPE_ALL);
            if review["continuous"] == false {
                self.message.push_str(&format!("; stroke interrupted: {} hidden/replaced pixel-state checks; inspect overlapping sprites", review["mismatches"]));
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
        if ![
            "main",
            "equalizer",
            "playlist",
            "atlas",
            "canvas",
            "cursors",
        ]
        .contains(&view.panel.as_str())
        {
            bail!("panel must be main, equalizer, playlist, atlas, canvas, or cursors");
        }
        if view.panel == "atlas" && !self.images.contains_key(&view.sheet) {
            bail!("Unknown atlas {}", view.sheet);
        }
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
        let opened = self.view.panel != "atlas" || self.view.sheet != view.sheet;
        self.view = view;
        if opened && self.view.panel == "atlas" {
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
    pub fn status(&self) -> Value {
        let mut value = self.brief();
        value["sheets"] = json!(self.sheets());
        value
    }
    pub fn brief(&self) -> Value {
        let mut view = serde_json::to_value(&self.view).unwrap_or_else(|_| json!({}));
        view["all_states"] = json!(self.view.states == SCOPE_ALL);
        json!({"path":self.path,"revision":self.revision,"dirty":self.dirty,"view":view,"undo":self.undo.len(),"redo":self.redo.len(),"message":self.message,"canvas":self.canvas_size(),"surface":self.surface(),"sprites":self.layers().len(),"paint_layers":self.paint_layer_info()})
    }
    pub fn surface(&self) -> String {
        match self.view.panel.as_str() {
            "atlas" => format!("atlas {}", self.view.sheet),
            "canvas" => "canvas".into(),
            panel => panel.into(),
        }
    }
    pub fn sheet_note(sheet: &str) -> Option<&'static str> {
        match sheet {
            "text.bmp" => Some(
                "text.bmp is not drawn. Cranamp sets titles and readouts in its \
                 own 5x7 face and reads this sheet only to sample the display \
                 ink: the most common opaque colour, or the second most common \
                 when more than two thirds of the sheet is opaque. Paint it in \
                 the colour the readouts should be.",
            ),
            "titlebar.bmp" => Some(
                "Most of this sheet is classic shade-mode artwork Cranamp does \
                 not draw. Only the two 275x14 title rows and the four 9x9 \
                 window buttons are sampled; the rest is reported as unsampled.",
            ),
            _ => None,
        }
    }
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
        let mut open: Vec<(u32, u32, u32)> = Vec::new();
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
    pub fn open_on_whole_skin(&mut self) {
        self.view.panel = "canvas".into();
        self.view.layer = "auto".into();
        self.view.layers.clear();
        self.view.zoom = self.view.zoom.clamp(1, 4);
        if self.view.paint_layer.is_none() {
            self.view.paint_layer = self
                .planes
                .iter()
                .rev()
                .find(|plane| plane.visible && !plane.locked)
                .map(|plane| plane.id.clone());
        }
    }
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
        if view.panel == "cursors" {
            return self.cursor_layers();
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
            return all;
        }
        let layers = mapping::layers(view);
        layers
            .into_iter()
            .filter(|layer| self.images.contains_key(&layer.sheet))
            .collect()
    }
    fn cursor_layers(&self) -> Vec<Layer> {
        let mut out = Vec::new();
        for (index, (_, name)) in crate::winamp::cursors::SkinCursor::files()
            .into_iter()
            .enumerate()
        {
            let Some(image) = self.images.get(name) else {
                continue;
            };
            let (width, height) = image.dimensions();
            let rect = [0, 0, width, height];
            let (x, y) = mapping::cursor_cell(index as u32);
            out.push(Layer {
                id: name.trim_end_matches(".cur").to_string(),
                sheet: name.to_string(),
                source: rect,
                destination: [x, y, width, height],
                variants: vec![rect],
                labels: vec!["always".into()],
            });
        }
        out
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
        self.stroke_intent.clear();
        self.overwrites = 0;
        self.overwrite_note = None;
    }
    fn note_atlas_writes(&mut self, touched: Vec<AtlasWrite>) {
        for (key, colour, at) in touched {
            if self.view.panel == "canvas" {
                self.stroke_intent.insert(at, colour);
            }
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
    fn identical_variants(&self) -> Vec<Value> {
        if self.atlas_writes.is_empty() {
            return Vec::new();
        }
        let names: Vec<String> = self.images.keys().cloned().collect();
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
    pub(super) fn render_patch(&self, bounds: [i32; 4]) -> Patch {
        self.render_patch_for(&self.view, bounds)
    }
    pub(super) fn render_patch_for(&self, view: &View, bounds: [i32; 4]) -> Patch {
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
                            && (view.panel == "atlas"
                                || layer.sheet.ends_with(".cur")
                                || !crate::winamp::skin::is_sprite_key(pixel.0))
                        {
                            image.put_pixel(x - x0, y - y0, pixel);
                        }
                    }
                }
            }
        }
        Patch { image, origin }
    }
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
                            if p[3] > 0 {
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
    pub fn inspect(&self, rect: [u32; 4], ink: Option<[u8; 4]>) -> Value {
        let [x, y, w, h] = rect;
        let composite = self.composite_images();
        let hits: Vec<Value> = self.layers().iter().rev().filter_map(|l| l.map(x, y).map(|(sx, sy)| json!({"layer":l.id,"sheet":l.sheet,"atlas_pixel":[l.source[0]+sx,l.source[1]+sy],"local_pixel":[sx,sy],"variants":l.variants.len(),"shared_or_stretched":l.stretched(),"rgba":composite.get(&l.sheet).and_then(|im|im.get_pixel_checked(l.source[0]+sx,l.source[1]+sy)).map(|p|p.0)}))).collect();
        let mut out = json!({"canvas_pixel":[x,y],"surface":self.surface(),"hits":hits});
        for hit in out["hits"].as_array_mut().unwrap() {
            let rgba = serde_json::from_value::<[u8; 4]>(hit["rgba"].clone()).ok();
            hit["sprite_key"] = json!(
                rgba.is_some_and(crate::winamp::skin::is_sprite_key)
                    && hit["sheet"].as_str().is_some_and(|s| s.ends_with(".bmp"))
            );
        }
        out["visible_rgba"] = json!(self
            .render_patch([x as i32, y as i32, 1, 1])
            .image
            .get_pixel_checked(0, 0)
            .map(|p| p.0));
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
    pub(super) fn state_sheet_layer(&self) -> Option<Layer> {
        let named = |wanted: &str| self.layers().into_iter().find(|l| l.id == wanted);
        if self.view.layers.len() == 1 {
            return named(&self.view.layers[0]);
        }
        if self.view.layer == "auto" || self.view.layer.is_empty() {
            return None;
        }
        named(&self.view.layer)
    }
    pub(super) fn state_sheet_asked(&self, id: Option<&str>) -> Result<Layer> {
        let Some(id) = id else {
            return self.state_sheet_layer().context(
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
    pub fn state_sheet(&self, id: Option<&str>) -> Result<RgbaImage> {
        let layer = self.state_sheet_asked(id)?;
        let columns = layer.variants.len().clamp(1, 7) as u32;
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
            for y in 1..ch - 1 {
                for x in 1..cw - 1 {
                    image.put_pixel(ox + x, oy + y, Rgba([34, 40, 50, 255]));
                }
            }
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
                    if pixel.0[3] > 0 {
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
            let beneath = self.selected_image();
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
        if let Some(before) = &self.stroke {
            self.images = before.images.clone();
            self.planes = before.planes.clone();
        }
        let v = self.view.clone();
        let mut op = if v.brush == "text" {
            json!({"op":"text","x":from[0],"y":from[1],"text":v.text,
                   "face":v.face,"scale":v.text_scale,"spacing":v.text_spacing})
        } else if ["line", "curve", "tuft"].contains(&v.brush.as_str()) {
            json!({"op":v.brush,"x":from[0],"y":from[1],"x2":to[0],"y2":to[1],"brush_size":v.brush_size,"curve_bend":v.curve_bend})
        } else {
            json!({"op":if v.brush=="glass" {"ellipse"}else{&v.brush},"x":from[0].min(to[0]),"y":from[1].min(to[1]),"width":(to[0]-from[0]).abs()+1,"height":(to[1]-from[1]).abs()+1,"brush_size":v.brush_size,"fill":v.filled||v.brush=="glass","material":if v.brush=="glass" {Some("glass")}else{None},
                   "bevel":if v.bevel > 0 {v.bevel as f64} else {((from[1]-to[1]).abs() as f64/2.).clamp(1.,24.)},
                   "refraction":v.refraction})
        };
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
            let mut written = BTreeSet::new();
            let mut touched = Vec::new();
            for l in hits {
                let (sx, sy) = l.map(x as u32, y as u32).unwrap();
                let targets = all.cells(l);
                if self.control_parts.contains(&l.id) && self.covered_probe.len() < 20000 {
                    for (index, cell) in targets.iter().enumerate() {
                        let variant = l.variants.iter().position(|v| v == cell).unwrap_or(index);
                        self.covered_probe.push((l.id.clone(), variant, [x, y]));
                    }
                }
                if all.scope != Scope::Current {
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
        let started = web_time::Instant::now();
        self.finish_stroke();
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
        let mut count = 0;
        let mut at: Option<(usize, String)> = None;
        let mut skipped: Vec<char> = Vec::new();
        let result = (|| -> Result<()> {
            for (pass, operations) in prepared.iter().enumerate() {
                let all = Variants {
                    scope: base_scope,
                    pick: (steps > 1).then_some(pass % steps),
                };
                for (index, op) in operations.iter().enumerate() {
                    let kind = op.get("op").and_then(Value::as_str).unwrap_or("pixel");
                    at = Some((index, kind.to_owned()));
                    self.operation_box = None;
                    let color = parse_color(
                        op.get("color")
                            .and_then(Value::as_str)
                            .unwrap_or(&self.view.color),
                    )?;
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
                            if let Some(rect) = op.get("source_rect") {
                                let rect = rect
                                    .as_array()
                                    .context("image source_rect needs [x,y,width,height]")?;
                                anyhow::ensure!(
                                    rect.len() == 4,
                                    "image source_rect needs four integers"
                                );
                                let r: Vec<u32> = rect
                                    .iter()
                                    .map(|v| {
                                        v.as_u64()
                                            .and_then(|n| u32::try_from(n).ok())
                                            .context("image source_rect needs nonnegative integers")
                                    })
                                    .collect::<Result<_>>()?;
                                anyhow::ensure!(
                                    r[2] > 0
                                        && r[3] > 0
                                        && r[0]
                                            .checked_add(r[2])
                                            .is_some_and(|end| end <= im.width())
                                        && r[1]
                                            .checked_add(r[3])
                                            .is_some_and(|end| end <= im.height()),
                                    "image source_rect must be inside the source image"
                                );
                                im = image::imageops::crop_imm(&im, r[0], r[1], r[2], r[3])
                                    .to_image();
                            }
                            if op.get("width").is_some() || op.get("height").is_some() {
                                let (width, height) =
                                    (integer(op, "width", 0)?, integer(op, "height", 0)?);
                                anyhow::ensure!(
                                    (1..=2048).contains(&width) && (1..=2048).contains(&height),
                                    "image resizing needs both width and height in 1..=2048"
                                );
                                im = image::imageops::resize(
                                    &im,
                                    width as u32,
                                    height as u32,
                                    image::imageops::FilterType::Nearest,
                                );
                            }
                            if im.pixels().any(|p| p[3] > 0 && p[3] < 255) {
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
                            let small = op.get("face").and_then(Value::as_str) == Some("small");
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
        if args.get("preview") == Some(&json!(true)) {
            let bounds = self.painted_bounds;
            let mut report = json!({"preview":true,"bounds":bounds,
                "pixels_written":count,"surface":self.surface(),
                "clipped_pixels":self.clipped_pixels,
                "overwrites":self.overwrites,
                "overwrite_sample":self.overwrite_note,
                "unmapped_pixels":self.unmapped_pixels.len()});
            if places.len() > 1 {
                report["repeated"] = json!({"places": places.len()});
            }
            report["continuity"] = self.stroke_continuity(base_scope == Scope::All);
            if !swept.is_empty() {
                report["swept"] = json!({"fields": swept.clone(), "steps": steps});
            }
            if let Some(covered) = self.covered_by_a_sibling_part() {
                report["covered_pixels"] = covered;
            }
            if let Some([x0, y0, x1, y1]) = bounds {
                if let Ok(review) = self.flat_regions([
                    x0 as u32,
                    y0 as u32,
                    (x1 - x0 + 1) as u32,
                    (y1 - y0 + 1) as u32,
                ]) {
                    report["flat_drawable_areas"] = review;
                }
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
        let continuity = self.stroke_continuity(base_scope == Scope::All);
        if args["require_continuity"] == true && continuity["continuous"] != true {
            self.restore(before);
            self.revision = before_revision;
            self.dirty = before_dirty;
            self.unmapped_pixels = previous_unmapped;
            bail!("Joined stroke refused without changing artwork/history: {continuity}");
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
                "; {} pixels have no bitmap source — the classic playlist fill is one flat PLEDIT.TXT colour, not artwork. Turn on “Playlist has its own background” in Skin options to paint there",
                self.unmapped_pixels.len()
            ));
            if before_revision == self.revision {
                self.revision += 1;
            }
        }
        let mut unsampled: Vec<[u32; 2]> = Vec::new();
        let mut never_drawn = false;
        if self.view.panel == "atlas" {
            if let Some(index) = self.images.keys().position(|k| k == &self.view.sheet) {
                if let Some((mask, width)) = self.sampled_mask(&self.view.sheet) {
                    never_drawn = !mask.iter().any(|sampled| *sampled);
                    if !never_drawn {
                        for ((sheet, x, y), (colour, _)) in self.atlas_writes.iter() {
                            if *sheet == index && !mask[(y * width + x) as usize] && colour[3] != 0
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
        if let Some([x0, y0, x1, y1]) = self.painted_bounds {
            if (x1 - x0 + 1) as i64 * (y1 - y0 + 1) as i64 >= 256 {
                let review = self.flat_regions([
                    x0 as u32,
                    y0 as u32,
                    (x1 - x0 + 1) as u32,
                    (y1 - y0 + 1) as u32,
                ]);
                match review {
                    Ok(review) => {
                        let count = review["regions"].as_array().unwrap().len();
                        if count > 0 {
                            self.message
                                .push_str(&format!("; {count} flat drawable areas need review"));
                            result["flat_drawable_areas"] = review;
                        }
                    }
                    Err(error) => {
                        result["flat_area_review_unavailable"] = json!(error.to_string());
                    }
                }
            }
        }
        if continuity["continuous"] == false {
            self.message.push_str(
                "; stroke interrupted: inspect continuity samples and overlapping sprites",
            );
        }
        result["continuity"] = continuity;
        result["transparency"] = self.transparency_report();
        if !result["transparency"]["opaque_moving_sprites"]
            .as_array()
            .unwrap()
            .is_empty()
        {
            self.message
                .push_str("; opaque moving sprite cells need review");
        }
        result["ms"] = json!(started.elapsed().as_millis() as u64);
        Ok(result)
    }
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
                let over = layers.iter().skip(mine + 1).find(|l| {
                    if control_of(&l.id) != control_of(&id) || l.id == id {
                        return false;
                    }
                    let Some((sx, sy)) = l.map(at[0].max(0) as u32, at[1].max(0) as u32) else {
                        return false;
                    };
                    let Some(image) = self.images.get(&l.sheet) else {
                        return false;
                    };
                    l.variants.iter().all(|cell| {
                        image
                            .get_pixel_checked(cell[0] + sx, cell[1] + sy)
                            .is_some_and(|p| p[3] > 0)
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
    pub fn palette_sample(&self, limit: usize) -> Vec<String> {
        let mut counts: BTreeMap<[u8; 4], usize> = BTreeMap::new();
        for image in self.composite_images().values() {
            for p in image.pixels() {
                if p.0[3] == 0 {
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
    pub fn divergences(&self) -> Vec<crate::winamp::skin::Divergence> {
        let mut entries: Vec<crate::winamp::skin::SkinEntry> = self
            .files
            .keys()
            .map(|name| {
                let size = self
                    .images
                    .get(name)
                    .map(|image| (image.width(), image.height()));
                (name.clone(), size)
            })
            .collect();
        entries.sort();
        let mut found = crate::winamp::skin::divergences(&entries);
        for (sheet, image) in &self.composite_images() {
            let Some((mask, width)) = self.sampled_mask(sheet) else {
                continue;
            };
            let clear = mask
                .iter()
                .enumerate()
                .filter(|(i, sampled)| {
                    **sampled && image.get_pixel(*i as u32 % width, *i as u32 / width).0[3] < 128
                })
                .count();
            if clear > 0 {
                found.push(crate::winamp::skin::Divergence {
                    entry: sheet.clone(),
                    problem: format!(
                        "{clear} pixels a sprite reads are clear, and a .wsz sheet \
                         cannot hold alpha; unpainted pixels export as black"
                    ),
                    fix: "paint them; use opaque #ff00ff for deliberate Cranamp sprite holes"
                        .into(),
                });
            }
        }
        found.sort_by(|a, b| a.entry.cmp(&b.entry));
        found
    }
    fn flatten_clear_sprite_pixels(&mut self) -> Vec<String> {
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        view.preview_playlist_height = 522;
        let (w, h) = self.canvas_size_for(&view);
        let canvas = self
            .render_patch_for(&view, [0, 0, w as i32, h as i32])
            .image;
        let layers = self.layers_for(&view);
        let mut filled: std::collections::BTreeMap<String, u32> = Default::default();
        for layer in &layers {
            let Some(image) = self.images.get(&layer.sheet) else {
                continue;
            };
            let (w, h) = image.dimensions();
            let [dx, dy, dw, dh] = layer.destination;
            let mut paint: Vec<(u32, u32, Rgba<u8>)> = Vec::new();
            for variant in &layer.variants {
                for y in 0..dh {
                    for x in 0..dw {
                        let sx = variant[0] + x * variant[2] / dw.max(1);
                        let sy = variant[1] + y * variant[3] / dh.max(1);
                        if sx >= w || sy >= h || image.get_pixel(sx, sy).0[3] >= 128 {
                            continue;
                        }
                        let under = canvas
                            .get_pixel_checked(dx + x, dy + y)
                            .copied()
                            .filter(|p| p.0[3] >= 128)
                            .unwrap_or(Rgba([0, 0, 0, 255]));
                        paint.push((sx, sy, Rgba([under.0[0], under.0[1], under.0[2], 255])));
                    }
                }
            }
            if paint.is_empty() {
                continue;
            }
            let sheet = layer.sheet.clone();
            let image = self.images.get_mut(&sheet).expect("the sheet is there");
            let mut count = 0;
            for (x, y, colour) in paint {
                if image.get_pixel(x, y).0[3] < 128 {
                    image.put_pixel(x, y, colour);
                    count += 1;
                }
            }
            *filled.entry(sheet).or_default() += count;
        }
        filled
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(sheet, count)| {
                format!("painted {count} clear pixels in {sheet} the colour the player already showed there")
            })
            .collect()
    }
    pub fn cursors_report(&self) -> Value {
        let drawn: Vec<Value> = crate::winamp::cursors::SkinCursor::files()
            .into_iter()
            .map(|(_, name)| {
                let image = self.images.get(name);
                let hotspot = self
                    .files
                    .get(name)
                    .map(|data| cursor_hotspot(data))
                    .unwrap_or_default();
                json!({
                    "region": name.trim_end_matches(".cur"),
                    "file": name,
                    "drawn": image.is_some(),
                    "size": image.map(|i| [i.width(), i.height()]),
                    "hotspot": image.map(|_| hotspot),
                })
            })
            .collect();
        let present = drawn.iter().filter(|c| c["drawn"] == true).count();
        json!({
            "regions": drawn,
            "drawn": present,
            "of": crate::winamp::cursors::SkinCursor::COUNT,
        })
    }
    pub fn draw_cursors(
        &mut self,
        roles: &[String],
        style: Option<&str>,
        overwrite: bool,
        source: &str,
    ) -> Result<Value> {
        self.finish_stroke();
        let wanted = cursor_roles(roles)?;
        let todo: Vec<_> = wanted
            .into_iter()
            .filter(|(_, name)| overwrite || !self.images.contains_key(*name))
            .collect();
        if todo.is_empty() {
            return Ok(json!({"drawn": [], "note": "every region asked for already has a cursor"}));
        }
        let palette = super::cursor_art::palette(&self.composite_images());
        let style = match style {
            Some(name) => super::cursor_art::Style::named(name).with_context(|| {
                let known: Vec<&str> = super::cursor_art::Style::ALL
                    .iter()
                    .map(|style| style.name())
                    .collect();
                format!(
                    "Unknown cursor style {name}; the styles are {}",
                    known.join(", ")
                )
            })?,
            None => super::cursor_art::Style::for_palette(&palette),
        };
        self.record(self.snapshot(), "Draw cursors".into(), source);
        let mut drawn = Vec::new();
        for (role, name) in todo {
            let (image, hotspot) = super::cursor_art::draw(role, &palette, style);
            let bytes = crate::winamp::cursors::encode_cur(
                image.width(),
                image.height(),
                image.as_raw(),
                hotspot[0],
                hotspot[1],
            )?;
            self.images.insert(name.to_string(), image);
            self.files.insert(name.to_string(), bytes);
            drawn.push(name);
        }
        self.changed();
        let hex = |c: [u8; 4]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        Ok(json!({
            "drawn": drawn,
            "style": style.name(),
            "palette": {
                "ink": hex(palette.ink),
                "body": hex(palette.body),
                "accent": hex(palette.accent),
            },
        }))
    }
    pub fn remove_cursors(&mut self, roles: &[String], source: &str) -> Result<Value> {
        self.finish_stroke();
        let todo: Vec<&'static str> = cursor_files_to_remove(roles)?
            .into_iter()
            .filter(|name| self.images.contains_key(*name) || self.files.contains_key(*name))
            .collect();
        if todo.is_empty() {
            return Ok(json!({"removed": []}));
        }
        self.record(self.snapshot(), "Remove cursors".into(), source);
        for name in &todo {
            self.images.remove(*name);
            self.files.remove(*name);
        }
        self.changed();
        Ok(json!({"removed": todo}))
    }
    pub fn set_cursor_hotspot(
        &mut self,
        role: &str,
        x: u32,
        y: u32,
        source: &str,
    ) -> Result<Value> {
        self.finish_stroke();
        let (_, name) = cursor_role(role)?;
        let image = self
            .images
            .get(name)
            .with_context(|| format!("{name} has no cursor to aim yet"))?
            .clone();
        let bytes = crate::winamp::cursors::encode_cur(
            image.width(),
            image.height(),
            image.as_raw(),
            x,
            y,
        )?;
        self.record(self.snapshot(), format!("Aim {name}"), source);
        self.files.insert(name.to_string(), bytes);
        self.changed();
        Ok(json!({"file": name, "hotspot": [x, y]}))
    }
    pub fn nudge_cursor_hotspot(
        &mut self,
        role: &str,
        dx: i32,
        dy: i32,
        source: &str,
    ) -> Result<Value> {
        let (_, name) = cursor_role(role)?;
        let image = self
            .images
            .get(name)
            .with_context(|| format!("{name} has no cursor to aim yet"))?;
        let (width, height) = image.dimensions();
        let at = self
            .files
            .get(name)
            .map(|data| cursor_hotspot(data))
            .unwrap_or_default();
        let x = (at[0] as i32 + dx).clamp(0, width.saturating_sub(1) as i32) as u32;
        let y = (at[1] as i32 + dy).clamp(0, height.saturating_sub(1) as i32) as u32;
        self.set_cursor_hotspot(role, x, y, source)
    }
    pub fn make_portable(&mut self, source: &str) -> Result<Value> {
        self.finish_stroke();
        let found = self.divergences();
        if found.is_empty() {
            let transparency = self.transparency_report();
            return Ok(
                json!({"plays_the_same_elsewhere":transparency["key_pixels"] == 0,"changed":[],"transparency":transparency}),
            );
        }
        self.record(self.snapshot(), "Make the skin portable".into(), source);
        let mut changed: Vec<String> = Vec::new();
        let clear: Vec<String> = found
            .iter()
            .filter(|d| d.problem.contains("clear"))
            .map(|d| d.entry.clone())
            .collect();
        for divergence in found.iter().filter(|d| !clear.contains(&d.entry)) {
            let entry = divergence.entry.clone();
            let classic = crate::winamp::skin::CLASSIC_SHEETS
                .iter()
                .find(|(sheet, _, _)| *sheet == entry);
            let Some((_, width, height)) = classic else {
                self.images.remove(&entry);
                self.files.remove(&entry);
                if self.view.sheet == entry {
                    self.view.sheet = "main.bmp".into();
                }
                changed.push(format!("dropped {entry}, which no player reads"));
                continue;
            };
            let grown = match self.images.get(&entry) {
                Some(old) => {
                    let (was, tall) = old.dimensions();
                    let mut image = RgbaImage::new((*width).max(was), (*height).max(tall));
                    for y in 0..image.height() {
                        for x in 0..image.width() {
                            let pixel = *old.get_pixel(x.min(was - 1), y.min(tall - 1));
                            image.put_pixel(x, y, pixel);
                        }
                    }
                    changed.push(format!(
                        "grew {entry} from {was}x{tall} to {}x{}, repeating its edge",
                        image.width(),
                        image.height()
                    ));
                    image
                }
                None => {
                    changed.push(format!("added {entry} at {width}x{height}, empty"));
                    RgbaImage::new(*width, *height)
                }
            };
            self.images.insert(entry.clone(), grown);
            self.files.insert(entry, Vec::new());
        }
        changed.extend(self.flatten_clear_sprite_pixels());
        self.changed();
        Ok(json!({
            "plays_the_same_elsewhere": self.divergences().is_empty() && self.transparency_report()["key_pixels"] == 0,
            "transparency": self.transparency_report(),
            "changed": changed,
        }))
    }
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
                        ("TRACK ROWS", [16, 20, 227, h - 58]),
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
        out
    }
    pub fn inspect_region(&self, r: [u32; 4]) -> Result<Value> {
        let (w, h) = self.canvas_size();
        anyhow::ensure!(
            r[2] > 0
                && r[3] > 0
                && r[0].checked_add(r[2]).is_some_and(|x| x <= w)
                && r[1].checked_add(r[3]).is_some_and(|y| y <= h),
            "Review rectangle must fit the current native panel"
        );
        let parts: Vec<_> = self.guides().into_iter().filter_map(|g| {
            let cut = super::guides::intersection(r, g.rect)?;
            let source = (!g.runtime && !g.hit).then(|| [g.source[0] + cut[0] - g.rect[0], g.source[1] + cut[1] - g.rect[1], cut[2], cut[3]]);
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
                writer.write_all(&archive_entry(name, data, images.get(name))?)?;
            }
            writer.finish()?;
        }
        Ok(output.into_inner())
    }
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
    fn display_ink(&self) -> [u8; 4] {
        self.composite_images()
            .get("text.bmp")
            .and_then(|im| {
                let total = (im.width() as usize).saturating_mul(im.height() as usize);
                crate::winamp::skin::sample_display_ink(im.as_raw(), total)
            })
            .unwrap_or([153, 204, 236, 255])
    }
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
                    if pixel[3] == 0 {
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
                "runtime.equalizer.EQ CURVE" => check("the equalizer curve", ink, ground),
                "runtime.playlist.TIME / TOTAL" | "runtime.playlist.ELAPSED" => {
                    check(id.rsplit('.').next().unwrap_or(id), ink, ground)
                }
                "runtime.playlist.TRACK ROWS" => {
                    let ground = colour("NormalBG").unwrap_or(ground);
                    if let Some(normal) = colour("Normal") {
                        check("a playlist row", normal, ground);
                    }
                    if let Some(current) = colour("Current") {
                        check("the playing row", current, ground);
                    }
                    if let (Some(normal), Some(on)) = (colour("Normal"), colour("SelectedBG")) {
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
        let flat = self.canvas_flat_review()?;
        if !flat.as_array().unwrap().is_empty() {
            self.message
                .push_str("; flat drawable areas need visual review");
        }
        self.revision += 1;
        let transparency = self.transparency_report();
        if !transparency["opaque_moving_sprites"]
            .as_array()
            .unwrap()
            .is_empty()
        {
            self.message
                .push_str("; opaque moving sprite cells need review");
        }
        let mut out = json!({"path":path,"bytes":bytes.len(),"flat_drawable_areas":flat,"transparency":transparency});
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
#[path = "../../../test/unit/winamp/studio/model/tests.rs"]
mod tests;
