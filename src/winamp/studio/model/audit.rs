//! Exhaustive source probes and visible flat-area review, shared by GUI and MCP.
use super::*;

impl Document {
    /// Exercise the real writer on a disposable document. Two different inks avoid
    /// false negatives when a shared source already contains the first probe ink.
    pub fn probe_pixels(&self, rect: [u32; 4], rows: bool) -> Result<Value> {
        self.coverage(rect, false)?;
        let mut scratch = Self::blank();
        scratch.images = self.images.clone();
        scratch.files = self.files.clone();
        scratch.revision = self.revision;
        scratch.view = self.view.clone();
        scratch.view.paint_layer = None;
        scratch.view.clip = None;
        scratch.view.mask_colors.clear();
        scratch.view.alpha_lock = false;
        scratch.view.layers.clear();
        let mut states = Vec::new();
        let mut mismatches = 0usize;
        for (active, pressed) in [(true, false), (true, true), (false, false), (false, true)] {
            scratch.view.active = active;
            scratch.view.pressed = pressed;
            let (mut coverage, _) = scratch.coverage(rect, true)?;
            let layers = scratch.layers();
            let mut verified = 0usize;
            let mut source_writes = 0usize;
            let mut failed = Vec::new();
            let mut verified_rows = Vec::new();
            for y in rect[1]..rect[1] + rect[3] {
                let mut row = String::new();
                for x in rect[0]..rect[0] + rect[2] {
                    let point = [x as i32, y as i32];
                    let mut writes = 0;
                    for ink in [[1, 253, 127, 255], [254, 2, 128, 255]] {
                        writes = writes.max(scratch.paint_line_untracked(
                            point,
                            point,
                            ink,
                            "auto",
                            Scope::All.into(),
                            &layers,
                        )?);
                    }
                    let writable = writes > 0;
                    let expected = coverage["rows"][(y - rect[1]) as usize]
                        .as_str()
                        .unwrap()
                        .as_bytes()[(x - rect[0]) as usize];
                    if writable != matches!(expected, b'D' | b'S' | b'R' | b'O') {
                        mismatches += 1;
                        failed.push([x, y]);
                    }
                    verified += usize::from(writable);
                    source_writes += writes;
                    row.push(if writable { 'W' } else { '-' });
                }
                verified_rows.push(row);
                scratch.forget_atlas_writes();
            }
            if !rows {
                coverage.as_object_mut().unwrap().remove("rows");
            }
            let mut state = json!({
                "active":active, "pressed":pressed, "coverage":coverage,
                "verified_bitmap_pixels":verified, "source_writes":source_writes,
                "mismatches":failed,
            });
            if rows {
                state["verified_rows"] = json!(verified_rows);
            }
            states.push(state);
        }
        Ok(json!({
            "rect":rect, "pixels_probed":u64::from(rect[2])*u64::from(rect[3])*4,
            "states":states, "mismatches":mismatches, "revision":self.revision,
            "method":"Two real paint writes per native pixel, Auto targets, all source variants, four active/pressed views, on a disposable document. W = writable source; - = no source. Coverage rows distinguish runtime overlays, opaque spectrum and palette-only fill.",
            "note":"Intrinsic bitmap capability, not permission under the current clip, color mask, alpha lock or locked paint layer. No artwork, view, clipboard, history or dirty state is changed. Writable does not mean visible through opaque runtime content."
        }))
    }

    /// Exact flat connected regions. This is a review warning, never an aesthetic
    /// verdict. Sparse decoration must not hide a large, still-flat backdrop.
    pub fn flat_regions(&self, rect: [u32; 4]) -> Result<Value> {
        let (coverage, _) = self.coverage(rect, true)?;
        let image = self
            .render_patch([
                rect[0] as i32,
                rect[1] as i32,
                rect[2] as i32,
                rect[3] as i32,
            ])
            .image;
        let w = rect[2] as usize;
        let h = rect[3] as usize;
        let eligible: Vec<bool> = coverage["rows"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|row| {
                row.as_str()
                    .unwrap()
                    .bytes()
                    .map(|c| matches!(c, b'D' | b'S' | b'R'))
            })
            .collect();
        let mut seen = vec![false; w * h];
        let mut square = vec![0u32; w * h];
        // Largest same-color square at each pixel makes the report point at an
        // actual hole, not just the enclosing box of a winding background.
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !eligible[i] {
                    continue;
                }
                square[i] = 1;
                if x > 0 && y > 0 {
                    let color = image.get_pixel(x as u32, y as u32);
                    if [i - 1, i - w, i - w - 1].iter().all(|&j| {
                        eligible[j] && image.get_pixel((j % w) as u32, (j / w) as u32) == color
                    }) {
                        square[i] += square[i - 1].min(square[i - w]).min(square[i - w - 1]);
                    }
                }
            }
        }
        let mut regions = Vec::new();
        for seed in 0..w * h {
            if seen[seed] || !eligible[seed] {
                continue;
            }
            let color = image.get_pixel((seed % w) as u32, (seed / w) as u32).0;
            let mut queue = vec![seed];
            seen[seed] = true;
            let (mut x0, mut y0, mut x1, mut y1) = (seed % w, seed / w, seed % w, seed / w);
            let mut largest = seed;
            let mut runtime = 0;
            let mut cursor = 0;
            while cursor < queue.len() {
                let i = queue[cursor];
                cursor += 1;
                let (x, y) = (i % w, i / w);
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                if square[i] > square[largest] {
                    largest = i;
                }
                runtime += usize::from(coverage["rows"][y].as_str().unwrap().as_bytes()[x] == b'R');
                for j in [
                    (x > 0).then(|| i - 1),
                    (x + 1 < w).then(|| i + 1),
                    (y > 0).then(|| i - w),
                    (y + 1 < h).then(|| i + w),
                ]
                .into_iter()
                .flatten()
                {
                    if !seen[j]
                        && eligible[j]
                        && image.get_pixel((j % w) as u32, (j / w) as u32).0 == color
                    {
                        seen[j] = true;
                        queue.push(j);
                    }
                }
            }
            if queue.len() < 256 || square[largest] < 8 {
                continue;
            }
            let side = square[largest];
            let hole = [
                rect[0] + (largest % w) as u32 + 1 - side,
                rect[1] + (largest / w) as u32 + 1 - side,
                side,
                side,
            ];
            let bounds = [
                rect[0] + x0 as u32,
                rect[1] + y0 as u32,
                (x1 - x0 + 1) as u32,
                (y1 - y0 + 1) as u32,
            ];
            let targets: Vec<_> = coverage["targets"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| {
                    t["destinations"].as_array().unwrap().iter().any(|r| {
                        let r: [u32; 4] = serde_json::from_value(r.clone()).unwrap();
                        r[0] < hole[0] + side
                            && hole[0] < r[0] + r[2]
                            && r[1] < hole[1] + side
                            && hole[1] < r[1] + r[3]
                    })
                })
                .map(|t| t["id"].clone())
                .collect();
            regions.push(json!({"bounds":bounds,"pixels":queue.len(),"rgba":color,
                "largest_flat_square":hole,"runtime_overlay_pixels":runtime,"targets_at_square":targets}));
        }
        regions.sort_by_key(|r| std::cmp::Reverse(r["pixels"].as_u64().unwrap()));
        Ok(json!({
            "rect":rect,"regions":regions,
            "note":"Review exact-color connected areas of at least 256 drawable pixels containing an 8x8 flat square. Runtime text backdrops ARE included; palette-only fill and opaque spectrum are excluded. Bounds may include detail: largest_flat_square is entirely flat. Intentional negative space is allowed, but these regions are not inaccessible and require visual review."
        }))
    }

    pub fn canvas_flat_review(&self) -> Result<Value> {
        let mut scratch = Self::blank();
        scratch.images = self.images.clone();
        scratch.planes = self.planes.clone();
        scratch.files = self.files.clone();
        scratch.view = self.view.clone();
        scratch.view.panel = "canvas".into();
        let (w, h) = scratch.canvas_size();
        let mut states = Vec::new();
        for (active, pressed) in [(true, false), (true, true), (false, false), (false, true)] {
            scratch.view.active = active;
            scratch.view.pressed = pressed;
            let review = scratch.flat_regions([0, 0, w, h])?;
            if !review["regions"].as_array().unwrap().is_empty() {
                states.push(json!({"active":active,"pressed":pressed,"review":review}));
            }
        }
        Ok(json!(states))
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/audit_tests.rs"]
mod tests;
