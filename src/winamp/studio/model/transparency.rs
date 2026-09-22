//! Classic opaque bitmap semantics and actionable source-cell diagnostics.
use super::*;

impl Document {
    pub fn transparency_report(&self) -> Value {
        let images = self.export_images();
        let mut magenta = 0usize;
        let mut alpha = 0usize;
        for (name, image) in &images {
            if name.ends_with(".bmp") {
                magenta += image
                    .pixels()
                    .filter(|p| crate::winamp::skin::is_sprite_key(p.0))
                    .count();
                alpha += image.pixels().filter(|p| p[3] != 255).count();
            }
        }
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        let mut cells = Vec::new();
        let mut incomplete = Vec::new();
        let mut seen = BTreeSet::new();
        for layer in self
            .layers_for(&view)
            .iter()
            .filter(|l| l.id.ends_with(".thumb"))
        {
            let Some(image) = images.get(&layer.sheet) else {
                continue;
            };
            for (index, rect) in layer.variants.iter().enumerate() {
                if !seen.insert((layer.sheet.clone(), *rect)) {
                    continue;
                }
                let (mut keyed, mut alpha_pixels, mut opaque, mut missing) = (0, 0, 0, 0);
                for y in rect[1]..rect[1] + rect[3] {
                    for x in rect[0]..rect[0] + rect[2] {
                        match image.get_pixel_checked(x, y) {
                            Some(p) => {
                                if crate::winamp::skin::is_sprite_key(p.0) {
                                    keyed += 1;
                                }
                                if p[3] == 255 {
                                    opaque += 1;
                                } else {
                                    alpha_pixels += 1;
                                }
                            }
                            None => missing += 1,
                        }
                    }
                }
                let cell = json!({"id":layer.id,"sheet":layer.sheet,"variant":index,"source":rect,
                    "key_pixels":keyed,"alpha_pixels":alpha_pixels,"opaque_pixels":opaque,"missing_pixels":missing});
                if alpha_pixels > 0 || missing > 0 {
                    incomplete.push(cell.clone());
                }
                cells.push(cell);
            }
        }
        json!({"profile":"classic-winamp-v1", "key":null, "key_pixels":magenta,
            "alpha_pixels":alpha, "keyed_spectrum":false,
            "classic_bitmap_compatible": alpha == 0, "moving_cells":cells,
            "incomplete_moving_sprites":incomplete, "opaque_moving_sprites":[],
            "meaning":"All BMP pixels are opaque, including #ff00ff. Alpha belongs to editable paint layers, not exported sprites. Erasing reveals lower paint layers. Cursor transparency is a separate CUR feature.",
            "review":"A moving handle carries its complete rectangle. Match it to a compatible track background and inspect every state and travel endpoint. Never copy a fixed piece of an illustration into a moving handle.",
            "compatibility":"The canvas, GPU player and 24-bit BMP export use the same opaque composition. REGION.TXT supplies whole-window cutouts; generate merged rectangles for Audacious compatibility. Format validation does not certify external-player screenshots."})
    }

    pub fn classic_report(&self) -> Result<Value> {
        let bytes = self.archive_with_images(&self.export_images())?;
        let audit = crate::winamp::skin::audit_classic_archive(&bytes)?;
        let mut errors = audit.errors;
        errors.extend(self.divergences());
        errors.sort_by(|a, b| a.entry.cmp(&b.entry).then(a.problem.cmp(&b.problem)));
        errors.dedup();
        Ok(
            json!({"profile":audit.profile,"exportable":errors.is_empty(),"divergences":errors,
            "warnings":audit.warnings,"transparency":self.transparency_report(),
            "plays_the_same_elsewhere":null,
            "external_player_verification":"not established by this format audit",
            "review":"Compare the exported WSZ in real players using matching state, scale, fonts and visualization settings. This report checks the archive and source pixels, not artistic quality."}),
        )
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/transparency_tests.rs"]
mod tests;
