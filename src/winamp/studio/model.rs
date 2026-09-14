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
    pub clean_corners: bool,
    pub filled: bool,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub grid: bool,
    pub alpha_lock: bool,
    pub mask_colors: Vec<String>,
    pub all_states: bool,
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
            clean_corners: false,
            filled: false,
            mirror_x: false,
            mirror_y: false,
            grid: false,
            alpha_lock: false,
            mask_colors: vec![],
            all_states: false,
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
            message: "Draw on the assembled skin. Every pixel maps back to its atlas.".into(),
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
                self.message = format!("{} · {} · {target}", self.view.brush, self.view.panel);
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
        if !(145..=522).contains(&view.preview_playlist_height)
            || !(1..=8).contains(&view.zoom)
            || [view.volume, view.balance, view.position, view.scroll]
                .iter()
                .chain(view.eq.iter())
                .any(|v| *v > 27)
            || view.digit > 9
            || view.playback > 2
        {
            bail!("preview_playlist_height 145..522; zoom 1..8; slider frames 0..27; digit 0..9; playback 0..2");
        }
        anyhow::ensure!(
            (1..=32).contains(&view.brush_size),
            "brush_size must be 1..32"
        );
        anyhow::ensure!(
            ["pencil", "line", "rect", "ellipse", "lift", "stamp", "glass", "curve", "tuft"]
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
        self.view = view;
        self.revision += 1;
        Ok(self.status())
    }
    pub fn status(&self) -> Value {
        json!({"path":self.path,"revision":self.revision,"dirty":self.dirty,"view":self.view,"undo":self.undo.len(),"redo":self.redo.len(),"message":self.message,"canvas":self.canvas_size(),"layers":self.layers(),"sheets":self.sheets(),"paint_layers":self.paint_layer_info()})
    }
    pub fn layers(&self) -> Vec<Layer> {
        self.layers_for(&self.view)
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
    fn canvas_size(&self) -> (u32, u32) {
        if self.view.panel == "canvas" {
            (275, 232 + self.view.preview_playlist_height)
        } else if self.view.panel == "atlas" {
            self.images
                .get(&self.view.sheet)
                .map(|im| im.dimensions())
                .unwrap_or((1, 1))
        } else {
            mapping::size(&self.view.panel)
        }
    }
    pub fn render(&self) -> RgbaImage {
        let composite = self.composite_images();
        let (w, h) = self.canvas_size();
        let mut image = RgbaImage::from_pixel(w, h, Rgba([30, 34, 42, 255]));
        if self.view.panel == "playlist" {
            let palette = self
                .files
                .get("pledit.txt")
                .map(|bytes| crate::winamp::skin::parse_pledit_txt(bytes).normal_bg)
                .unwrap_or([0, 0, 0, 255]);
            image = RgbaImage::from_pixel(w, h, Rgba(palette));
        }
        if self.view.panel == "canvas" {
            let bg = self
                .files
                .get("pledit.txt")
                .map(|b| crate::winamp::skin::parse_pledit_txt(b).normal_bg)
                .unwrap_or([0, 0, 0, 255]);
            for y in 252..h - 38 {
                for x in 12..255 {
                    image.put_pixel(x, y, Rgba(bg));
                }
            }
        }
        for layer in self.layers() {
            if let Some(sheet) = composite.get(&layer.sheet) {
                let [dx, dy, dw, dh] = layer.destination;
                for y in dy..(dy + dh).min(h) {
                    for x in dx..(dx + dw).min(w) {
                        let Some((sx, sy)) = layer.map(x, y) else {
                            continue;
                        };
                        if let Some(pixel) =
                            sheet.get_pixel_checked(layer.source[0] + sx, layer.source[1] + sy)
                        {
                            if pixel.0[3] > 0
                                && (self.view.panel == "atlas" || pixel.0[..3] != [255, 0, 255])
                            {
                                image.put_pixel(x, y, *pixel);
                            }
                        }
                    }
                }
            }
        }
        image
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
        all: bool,
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
        if self.view.guides {
            for (i, g) in self.guides().iter().enumerate() {
                let [x, y, w, h] = g.rect;
                let color = if g.runtime {
                    [255, 126, 145, 255]
                } else if self.view.clip == Some(g.rect) {
                    [255, 220, 110, 255]
                } else {
                    [56, 226, 223, 255]
                };
                for yy in y..y.saturating_add(h).min(im.height()) {
                    for xx in x..x.saturating_add(w).min(im.width()) {
                        if xx == x || yy == y || xx == x + w - 1 || yy == y + h - 1 {
                            im.put_pixel(xx, yy, Rgba(color));
                        }
                    }
                }
                if w >= 10 && h >= 9 {
                    let label = (i + 1).to_string();
                    for (k, ch) in label.chars().enumerate() {
                        if let Some(rows) = crate::winamp::pixel_text::glyph(ch) {
                            for (yy, bits) in rows.iter().enumerate() {
                                for xx in 0..5 {
                                    let px = x + 2 + k as u32 * 6 + xx;
                                    let py = y + 2 + yy as u32;
                                    if px < x + w - 1
                                        && py < y + h - 1
                                        && px < im.width()
                                        && py < im.height()
                                    {
                                        im.put_pixel(
                                            px,
                                            py,
                                            Rgba(if bits & (1 << (4 - xx)) != 0 {
                                                color
                                            } else {
                                                [17, 27, 40, 255]
                                            }),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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
    pub fn inspect(&self, x: u32, y: u32) -> Value {
        let composite = self.composite_images();
        let hits:Vec<Value>=self.layers().iter().rev().filter_map(|l|l.map(x,y).map(|(sx,sy)|json!({"layer":l.id,"sheet":l.sheet,"atlas_pixel":[l.source[0]+sx,l.source[1]+sy],"local_pixel":[sx,sy],"variants":l.variants.len(),"shared_or_stretched":l.stretched(),"rgba":composite.get(&l.sheet).and_then(|im|im.get_pixel_checked(l.source[0]+sx,l.source[1]+sy)).map(|p|p.0)}))).collect();
        json!({"canvas_pixel":[x,y],"hits":hits})
    }

    pub fn state_sheet(&self) -> RgbaImage {
        let fallback = match self.view.panel.as_str() {
            "atlas" => "sheet",
            "canvas" => "main.volume.track",
            "equalizer" => "band1.track",
            "playlist" => "scroll.thumb",
            _ => "volume.track",
        };
        let layers = self.layers();
        let layer = layers
            .iter()
            .find(|l| l.id == self.view.layer)
            .or_else(|| layers.iter().find(|l| l.id == fallback))
            .unwrap();
        let columns = layer.variants.len().min(7) as u32;
        let cw = layer.source[2] + 8;
        let ch = layer.source[3] + 8;
        let mut image = RgbaImage::from_pixel(
            columns * cw,
            (layer.variants.len() as u32).div_ceil(columns) * ch,
            Rgba([25, 29, 36, 255]),
        );
        let composite = self.composite_images();
        let source = &composite[&layer.sheet];
        for (i, r) in layer.variants.iter().enumerate() {
            for y in 0..r[3] {
                for x in 0..r[2] {
                    let p = source.get_pixel(r[0] + x, r[1] + y);
                    if p.0[3] != 0 {
                        image.put_pixel(
                            (i as u32 % columns) * cw + 4 + x,
                            (i as u32 / columns) * ch + 4 + y,
                            *p,
                        );
                    }
                }
            }
        }
        image
    }
    pub fn paint_line(
        &mut self,
        from: [i32; 2],
        to: [i32; 2],
        color: [u8; 4],
        layer: &str,
        all: bool,
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
        all: bool,
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
        let mut geometry = op.clone();
        if geometry.get("clean_corners").is_none() {
            geometry["clean_corners"] = json!(self.view.clean_corners);
        }
        let points = super::brush::rasterize(&geometry)?;
        let material = op.get("material").and_then(Value::as_str);
        anyhow::ensure!(
            material.is_none() || material == Some("glass"),
            "Unknown material"
        );
        let shaded = if material == Some("glass") {
            Some(super::material::glass(
                &points,
                &self.selected_image(),
                color,
                op,
            )?)
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
            let count = self.paint_cluster(
                &cluster,
                to,
                "selection",
                self.view.all_states,
                &self.layers(),
            )?;
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
        let op = if ["line", "curve", "tuft"].contains(&v.brush.as_str()) {
            json!({"op":v.brush,"x":from[0],"y":from[1],"x2":to[0],"y2":to[1],"brush_size":v.brush_size,"curve_bend":v.curve_bend})
        } else {
            json!({"op":if v.brush=="glass" {"ellipse"}else{&v.brush},"x":from[0].min(to[0]),"y":from[1].min(to[1]),"width":(to[0]-from[0]).abs()+1,"height":(to[1]-from[1]).abs()+1,"brush_size":v.brush_size,"fill":v.filled||v.brush=="glass","material":if v.brush=="glass" {Some("glass")}else{None},"bevel":((from[1]-to[1]).abs() as f64/2.).clamp(1.,24.)})
        };
        let count = self.paint_shape(
            &op,
            parse_color(&v.color)?,
            "selection",
            v.all_states,
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
        all: bool,
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
                    .rev()
                    .find(|l| l.map(x as u32, y as u32).is_some())
                    .into_iter()
                    .collect()
            } else {
                layers
                    .iter()
                    .filter(|l| selection.contains(&l.id) && l.map(x as u32, y as u32).is_some())
                    .collect()
            };
            // Shared source pixels may be reached through multiple selected instances.
            let mut written = BTreeSet::new();
            for l in hits {
                let (sx, sy) = l.map(x as u32, y as u32).unwrap();
                let targets = if all {
                    l.variants.clone()
                } else {
                    vec![l.source]
                };
                let dimensions = self
                    .images
                    .get(&l.sheet)
                    .context("missing atlas")?
                    .dimensions();
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
                            if p.0 != color {
                                *p = Rgba(color);
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        Ok(count)
    }
    pub fn draw(&mut self, args: &Value) -> Result<Value> {
        self.finish_stroke();
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
        let all = args
            .get("all_states")
            .and_then(Value::as_bool)
            .unwrap_or(self.view.all_states);
        let layer = "selection";
        let operations = args
            .get("operations")
            .and_then(Value::as_array)
            .context("operations array required")?;
        if operations.len() > 10000 {
            bail!("At most 10000 operations per transaction");
        }
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
        let previous_selection = std::mem::replace(&mut self.view.layers, selected.clone());
        let paint_layers = self.layers();
        let mut count = 0;
        let result = (|| -> Result<()> {
            for op in operations {
                let kind = op.get("op").and_then(Value::as_str).unwrap_or("pixel");
                let color = parse_color(
                    op.get("color")
                        .and_then(Value::as_str)
                        .unwrap_or(&self.view.color),
                )?;
                let x = integer(op, "x", 0)?;
                let y = integer(op, "y", 0)?;
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
                        let im = self.cluster.clone().context("Lift a pixel cluster first")?;
                        count += self.paint_cluster(&im, [x, y], layer, all, &paint_layers)?;
                    }
                    "stamp" => {
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
                    _ => bail!("Unknown drawing operation {kind}"),
                }
            }
            Ok(())
        })();
        self.view.layers = previous_selection;
        self.view.mask_colors = previous_mask;
        if let Err(error) = result {
            self.restore(before);
            self.revision = before_revision;
            self.dirty = before_dirty;
            return Err(error);
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
            if all { " across all variants" } else { "" }
        );
        Ok(json!({"pixels_written":count,"revision":self.revision}))
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
                let variants = if atlas {
                    l.variants.clone()
                } else {
                    vec![l.source]
                };
                for (i, r) in variants.iter().enumerate() {
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
                    out.push(Guide {
                        id,
                        label: format!("{}:{i}", l.id.rsplit('.').next().unwrap_or(&l.id)),
                        sheet: l.sheet.clone(),
                        rect,
                        source: *r,
                        variant: i,
                        active: *r == l.source,
                        runtime: false,
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
            let source=(!g.runtime).then(||[g.source[0]+cut[0]-g.rect[0],g.source[1]+cut[1]-g.rect[1],cut[2],cut[3]]);
            Some(json!({"id":g.id,"label":g.label,"sheet":g.sheet,"overlap":cut,"source_overlap":source,"runtime":g.runtime,"active":g.active}))
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
        } else if !g.runtime {
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
    pub fn save_project(&mut self, path: &Path) -> Result<Value> {
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
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension("cstudio.tmp");
        std::fs::write(&temp, bytes.get_ref())?;
        std::fs::rename(temp, path)?;
        self.saved = self.snapshot();
        self.dirty = false;
        self.message = format!("Saved layered project {}", path.display());
        self.revision += 1;
        Ok(json!({"path":path,"layers":self.planes.len(),"bytes":bytes.get_ref().len()}))
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
        self.message = format!("Saved {}", path.display());
        self.revision += 1;
        Ok(json!({"path":path,"bytes":bytes.len()}))
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
fn integer(v: &Value, key: &str, default: i32) -> Result<i32> {
    match v.get(key) {
        None => Ok(default),
        Some(n) => {
            let n = n.as_i64().context("Coordinates must be integers")?;
            if !(-4096..=4096).contains(&n) {
                bail!("Coordinate outside supported range");
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
        d.paint_line([40, 90], [40, 90], [4, 5, 6, 255], "selection", false)
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
        d.paint_line([0, 0], [2, 2], [1, 2, 3, 255], "selection", false)
            .unwrap();
        d.finish_stroke();
        assert_eq!(d.history()["cursor"], 0);
        assert!(!d.dirty);
        assert!(d.redo());
        d.checkpoint();
        d.paint_line([40, 90], [41, 90], [1, 2, 3, 255], "selection", false)
            .unwrap();
        d.paint_line([41, 90], [43, 90], [1, 2, 3, 255], "selection", false)
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
        d.state_sheet();
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
        assert_ne!(
            d.editor_render(),
            artwork,
            "guides must be visible in the editor"
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
}
