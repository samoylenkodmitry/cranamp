//! Read-only pixel ownership. Runtime footprints are overlays, never exclusion masks.
use super::*;

fn contains(rect: [u32; 4], x: u32, y: u32) -> bool {
    x >= rect[0] && y >= rect[1] && x - rect[0] < rect[2] && y - rect[1] < rect[3]
}
fn intersects(a: [u32; 4], b: [u32; 4]) -> bool {
    u64::from(a[0]) < u64::from(b[0]) + u64::from(b[2])
        && u64::from(b[0]) < u64::from(a[0]) + u64::from(a[2])
        && u64::from(a[1]) < u64::from(b[1]) + u64::from(b[3])
        && u64::from(b[1]) < u64::from(a[1]) + u64::from(a[3])
}
impl Document {
    pub fn coverage(&self, rect: [u32; 4], rows: bool) -> Result<(Value, RgbaImage)> {
        let [x, y, w, h] = rect;
        let (sw, sh) = self.canvas_size();
        anyhow::ensure!(
            w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 1_048_576,
            "coverage requires a nonempty rectangle of at most 1048576 pixels"
        );
        anyhow::ensure!(
            x.checked_add(w).is_some_and(|end| end <= sw)
                && y.checked_add(h).is_some_and(|end| end <= sh),
            "coverage {rect:?} must fit inside {} ({sw}x{sh})",
            self.surface()
        );
        let layers = self.layers();
        let guides = self.guides();
        let runtime: Vec<_> = guides
            .iter()
            .filter(|g| g.runtime && self.view.panel != "atlas" && intersects(rect, g.rect))
            .collect();
        let mut targets = BTreeMap::<String, Value>::new();
        for layer in layers.iter().filter(|l| intersects(rect, l.destination)) {
            let entry = targets.entry(layer.id.clone()).or_insert_with(|| {
                json!({
                    "id":layer.id, "sheet":layer.sheet, "source":layer.source,
                    "states":layer.variants.len(), "destinations":[],
                    "repeated":layers.iter().filter(|l| l.id == layer.id).count() > 1,
                })
            });
            entry["destinations"]
                .as_array_mut()
                .unwrap()
                .push(json!(layer.destination));
            if w == 1 && h == 1 {
                if let Some((sx, sy)) = layer.map(x, y) {
                    entry["atlas_pixel"] = json!([layer.source[0] + sx, layer.source[1] + sy]);
                }
            }
        }
        let mut counts = [0u32; 6];
        let mut map = Vec::new();
        let mut image = RgbaImage::new(w, h);
        for yy in y..y + h {
            let mut row = String::with_capacity(w as usize);
            for xx in x..x + w {
                let top = layers.iter().rev().find(|l| l.map(xx, yy).is_some());
                let under_runtime = runtime.iter().any(|g| contains(g.rect, xx, yy));
                // draw_visualizer fills its entire rectangle even while stopped.
                let opaque_runtime = runtime
                    .iter()
                    .any(|g| g.label == "SPECTRUM" && contains(g.rect, xx, yy));
                let palette_only = match self.view.panel.as_str() {
                    "canvas" => contains([12, 252, 243, sh.saturating_sub(290)], xx, yy),
                    "playlist" => contains([12, 20, 243, sh.saturating_sub(58)], xx, yy),
                    _ => false,
                };
                let index = if let Some(layer) = top {
                    if opaque_runtime {
                        5
                    } else if under_runtime {
                        2
                    } else if layer.variants.len() > 1 || targets[&layer.id]["repeated"] == true {
                        1
                    } else {
                        0
                    }
                } else if palette_only {
                    3
                } else {
                    4
                };
                counts[index] += 1;
                row.push(['D', 'S', 'R', 'P', 'X', 'O'][index]);
                image.put_pixel(
                    xx - x,
                    yy - y,
                    Rgba(
                        [
                            [76, 174, 130, 255],
                            [80, 139, 212, 255],
                            [218, 172, 81, 255],
                            [124, 98, 155, 255],
                            [53, 57, 66, 255],
                            [190, 102, 76, 255],
                        ][index],
                    ),
                );
            }
            if rows {
                map.push(row);
            }
        }
        let mut report = json!({
            "surface":self.surface(), "rect":rect, "revision":self.revision,
            "counts":{
                "paintable":counts[0]+counts[1]+counts[2]+counts[5], "direct":counts[0],
                "stateful_or_repeated":counts[1], "runtime_overlay":counts[2]+counts[5],
                "opaque_runtime":counts[5],
                "palette_only":counts[3], "unmapped":counts[4],
            },
            "legend":{
                "D":"Paintable bitmap; no runtime footprint at this pixel.",
                "S":"Paintable sprite with states or repeated cells. Inspect variants and shared destinations.",
                "R":"Paintable bitmap beneath a runtime footprint. Only live content may cover the artwork; this is NOT an inaccessible region.",
                "O":"Paintable bitmap, but the GPU player covers it with the solid VISCOLOR spectrum background even while stopped. The editor can show paint the player hides.",
                "P":"No bitmap source: playlist color fill. Use PLEDIT.TXT colors.",
                "X":"No mapped bitmap source on this surface.",
            },
            "targets":targets.into_values().collect::<Vec<_>>(),
            "runtime_footprints":runtime.iter().map(|g|json!({"id":g.id,"rect":g.rect,"opaque_background":g.label=="SPECTRUM"})).collect::<Vec<_>>(),
            "note":"Coverage describes source ownership, independent of selection, clip, masks and locked paint planes. Runtime footprints are not exclusion masks: inspect the exact pixels, keep live content legible, and verify the actual GPU player. Bitmap fonts and shared cells may affect other states or locations.",
        });
        if rows {
            report["rows"] = json!(map);
        }
        Ok((report, image))
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/coverage_tests.rs"]
mod tests;
