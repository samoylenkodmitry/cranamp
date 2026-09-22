//! Paint a correctly indexed classic font through the same pen used by the GUI/MCP.
use super::*;

impl Document {
    pub fn classic_font(&mut self, ink: &str, background: &str) -> Result<Value> {
        anyhow::ensure!(
            parse_color(ink)?[3] == 255 && parse_color(background)?[3] == 255,
            "Classic font ink and background must be opaque"
        );
        let image = self.images.get("text.bmp").context("Missing text.bmp")?;
        anyhow::ensure!(
            image.width() >= 155 && image.height() >= 18,
            "Classic font builder needs text.bmp at least 155x18"
        );
        let mut operations = vec![
            json!({"op":"rect","x":0,"y":0,"width":155,"height":18,"color":background,"fill":true}),
        ];
        for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZ\"@0123456789\u{1}.:()-'!_+\\/[]~&%,=$#ÅÖÄ?*".chars()
        {
            let glyph = match ch {
                '@' => [6, 9, 11, 8, 7],
                '\u{1}' => [0, 6, 6, 0, 0],
                '\'' => [4, 4, 0, 0, 0],
                '~' => [0, 5, 10, 0, 0],
                '$' => [7, 12, 6, 3, 14],
                'Å' => [6, 0, 6, 15, 9],
                'Ö' => [9, 0, 6, 9, 6],
                'Ä' => [9, 0, 6, 15, 9],
                _ => crate::winamp::pixel_text::small_glyph(ch)
                    .context("Missing font-builder glyph")?,
            };
            let (x, y) = crate::winamp::bitmap_font::source(ch);
            let rows: Vec<String> = glyph
                .iter()
                .map(|bits| {
                    (0..4)
                        .map(|x| if bits & (1 << (3 - x)) != 0 { 'x' } else { ' ' })
                        .collect()
                })
                .collect();
            operations.push(json!({"op":"stamp","x":x,"y":y,"rows":rows,"palette":{"x":ink}}));
        }
        let saved = self.view.clone();
        self.view.panel = "atlas".into();
        self.view.sheet = "text.bmp".into();
        self.view.layers.clear();
        self.view.layer = "auto".into();
        self.view.clip = None;
        self.view.alpha_lock = false;
        self.view.mask_colors.clear();
        self.view.opacity = 255;
        self.view.grain = 0;
        self.view.mirror_x = false;
        self.view.mirror_y = false;
        let result = self
            .draw(&json!({"operations":operations,"layers":[],"label":"Classic 5x6 font atlas"}));
        self.view = saved;
        result
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/font_tests.rs"]
mod tests;
