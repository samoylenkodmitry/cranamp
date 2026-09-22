//! Verify that requested native ink survived component mapping and composition.
use super::*;

impl Document {
    pub fn stroke_continuity(&self, all_states: bool) -> Value {
        if self.view.panel != "canvas" {
            return json!({"continuous":null,"note":"Joined-canvas check only; atlas/cursor edits are source-local."});
        }
        if self
            .view
            .paint_layer
            .as_ref()
            .and_then(|id| self.planes.iter().find(|p| &p.id == id))
            .is_some_and(|p| !p.visible || p.opacity != 255 || p.clip_below)
        {
            return json!({"continuous":null,"note":"Exact ink comparison needs a visible, full-opacity, unclipped paint layer."});
        }
        let ink: Vec<_> = self
            .stroke_intent
            .iter()
            .filter(|(_, c)| c[3] == 255)
            .collect();
        let mut samples = Vec::new();
        let mut mismatches = 0;
        let mut checked = 0;
        let states = if all_states {
            vec![(true, false), (true, true), (false, false), (false, true)]
        } else {
            vec![(self.view.active, self.view.pressed)]
        };
        if !ink.is_empty() {
            let x0 = ink.iter().map(|(p, _)| p[0]).min().unwrap();
            let y0 = ink.iter().map(|(p, _)| p[1]).min().unwrap();
            let x1 = ink.iter().map(|(p, _)| p[0]).max().unwrap();
            let y1 = ink.iter().map(|(p, _)| p[1]).max().unwrap();
            for (active, pressed) in &states {
                let mut view = self.view.clone();
                view.active = *active;
                view.pressed = *pressed;
                let image = self
                    .render_patch_for(&view, [x0, y0, x1 - x0 + 1, y1 - y0 + 1])
                    .image;
                let layers = self.layers_for(&view);
                for (at, wanted) in &ink {
                    checked += 1;
                    let actual = image.get_pixel((at[0] - x0) as u32, (at[1] - y0) as u32).0;
                    if actual == **wanted {
                        continue;
                    }
                    mismatches += 1;
                    if samples.len() < 16 {
                        let sources: Vec<_> = layers.iter().rev().filter_map(|l| {
                            let (x,y) = l.map(at[0] as u32,at[1] as u32)?;
                            let p = self.sheet_stack(&l.sheet)?.at(l.source[0]+x,l.source[1]+y)?;
                            if p[3] == 0 { return None; }
                            Some(json!({"id":l.id,"sheet":l.sheet,"source":[l.source[0]+x,l.source[1]+y],"rgba":p.0}))
                        }).collect();
                        samples.push(json!({"at":at,"wanted":wanted,"visible":actual,
                            "active":active,"pressed":pressed,"covering_sources":sources}));
                    }
                }
            }
        }
        let continuous =
            if mismatches > 0 || self.clipped_pixels > 0 || !self.unmapped_pixels.is_empty() {
                Some(false)
            } else if checked > 0 {
                Some(true)
            } else {
                None
            };
        json!({"continuous":continuous,
            "checked":checked,"states":states.len(),"mismatches":mismatches,"samples":samples,
            "clipped_pixels":self.clipped_pixels,"shared_source_overwrites":self.overwrites,
            "unmapped_pixels":self.unmapped_pixels.len(),
            "note":"Checks final opaque native ink against the composed editor, after source writes. Magenta is opaque ink. Layer erasure and masked-out pixels are not ink checks. Intentional occlusion may warn. This cannot judge pre-existing artistic seams; inspect the halo and GPU, including moving states. Author a crossing once in canvas coordinates, without restarting texture phase or clipping it per component."})
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/continuity_tests.rs"]
mod tests;
