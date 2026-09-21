//! Color-key semantics and moving-cell review; raw sheets retain the opaque key.
use super::*;

impl Document {
    pub fn transparency_report(&self) -> Value {
        let images = self.composite_images();
        let key_pixels: usize = images
            .iter()
            .filter(|(name, _)| name.ends_with(".bmp"))
            .map(|(_, im)| {
                im.pixels()
                    .filter(|p| crate::winamp::skin::is_sprite_key(p.0))
                    .count()
            })
            .sum();
        let mut view = self.view.clone();
        view.panel = "canvas".into();
        let mut cells = Vec::new();
        let mut warnings = Vec::new();
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
                let (mut keyed, mut alpha_clear, mut opaque, mut missing) = (0, 0, 0, 0);
                for y in rect[1]..rect[1] + rect[3] {
                    for x in rect[0]..rect[0] + rect[2] {
                        match image.get_pixel_checked(x, y) {
                            Some(p) if crate::winamp::skin::is_sprite_key(p.0) => keyed += 1,
                            Some(p) if p[3] < 255 => alpha_clear += 1,
                            Some(_) => opaque += 1,
                            None => missing += 1,
                        }
                    }
                }
                let cell = json!({"id":layer.id,"sheet":layer.sheet,"variant":index,"source":rect,
                    "key_pixels":keyed,"alpha_pixels":alpha_clear,"opaque_pixels":opaque,"missing_pixels":missing});
                if opaque == rect[2] * rect[3] {
                    warnings.push(cell.clone());
                }
                cells.push(cell);
            }
        }
        json!({"key":"#ff00ff","key_pixels":key_pixels,"moving_cells":cells,
            "opaque_moving_sprites":warnings,
            "meaning":"Opaque #ff00ff is a sprite hole after paint layers are composed. Alpha 0 erases only the active paint layer. Unmapped stamp characters skip existing paint. Atlas/clipboard views retain the literal key.",
            "review":"Opaque moving cells carry their entire rectangle over every position. For a silhouette, key the whole cell then draw the subject; inspect every state and travel endpoint in the GPU player. Box-shaped controls may be intentional.",
            "compatibility":"Cranamp interprets exact opaque magenta as a sprite key. BMP export preserves it; identical transparency in other players is not verified."})
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/transparency_tests.rs"]
mod tests;
