use super::*;
use crate::winamp::skin::regions::{Regions, SECTIONS};
impl Document {
    pub fn window_regions(&mut self, args: &Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("list");
        let mut regions = self
            .files
            .get("region.txt")
            .map(|b| Regions::parse(b))
            .transpose()?
            .unwrap_or_default();
        if action != "list" {
            let requested = args["section"]
                .as_str()
                .context("Region section required")?;
            let name = *SECTIONS
                .iter()
                .find(|s| s.eq_ignore_ascii_case(requested))
                .context("Use Normal, WindowShade, Equalizer or EqualizerWS")?;
            let height = if name.ends_with("Shade") || name.ends_with("WS") {
                14
            } else {
                116
            };
            match action {
                "remove" => {
                    regions.sections.remove(name);
                }
                "normalize" => {
                    let mask = regions.mask(name, 275, height);
                    regions.set_mask(name, 275, height, &mask)?;
                }
                "generate" => {
                    let mask = if let Some(rows) = args["rows"].as_array() {
                        ensure_region_rows(rows, height)?
                    } else {
                        anyhow::ensure!(
                            name == "Normal" || name == "Equalizer",
                            "For shade regions supply 14 mask rows"
                        );
                        let color = parse_color(
                            args["transparent_color"]
                                .as_str()
                                .context("Supply mask rows or an explicit transparent_color")?,
                        )?;
                        let sheet = if name == "Normal" {
                            "main.bmp"
                        } else {
                            "eqmain.bmp"
                        };
                        let image = self
                            .composite_images()
                            .remove(sheet)
                            .context("Missing window background sheet")?;
                        anyhow::ensure!(
                            image.width() >= 275 && image.height() >= 116,
                            "Region color extraction needs a complete 275x116 background"
                        );
                        let mut mask = vec![true; 275 * 116];
                        let matches =
                            |x: usize, y: usize| image.get_pixel(x as u32, y as u32).0 == color;
                        if args["exterior_only"].as_bool().unwrap_or(true) {
                            let mut pending = Vec::new();
                            for x in 0..275 {
                                pending.push((x, 0));
                                pending.push((x, 115));
                            }
                            for y in 0..116 {
                                pending.push((0, y));
                                pending.push((274, y));
                            }
                            while let Some((x, y)) = pending.pop() {
                                let i = y * 275 + x;
                                if !mask[i] || !matches(x, y) {
                                    continue;
                                }
                                mask[i] = false;
                                if x > 0 {
                                    pending.push((x - 1, y));
                                }
                                if x < 274 {
                                    pending.push((x + 1, y));
                                }
                                if y > 0 {
                                    pending.push((x, y - 1));
                                }
                                if y < 115 {
                                    pending.push((x, y + 1));
                                }
                            }
                        } else {
                            for y in 0..116 {
                                for x in 0..275 {
                                    mask[y * 275 + x] = !matches(x, y);
                                }
                            }
                        }
                        mask
                    };
                    regions.set_mask(name, 275, height, &mask)?;
                }
                _ => bail!("Unknown region action {action}"),
            }
            let bytes = regions.encode()?;
            self.finish_stroke();
            self.record(self.snapshot(), format!("Window region: {name}"), "Studio");
            if regions.sections.is_empty() {
                self.files.remove("region.txt");
            } else {
                self.files.insert("region.txt".into(), bytes);
            }
            self.changed();
        }
        Ok(
            json!({"regions":regions,"portable":regions.portable(),"revision":self.revision,
            "note":"REGION.TXT clips the entire window, including controls and live text. Rectangular mask pieces preserve pixel cutouts in Winamp, Webamp and Audacious. It never makes a moving sprite reveal artwork. Shade masks are saved for other players; Cranamp currently displays full-height windows."}),
        )
    }
    pub(super) fn clip_region_preview(
        &self,
        view: &View,
        image: &mut RgbaImage,
        origin: (i32, i32),
    ) {
        if !matches!(view.panel.as_str(), "canvas" | "main" | "equalizer") {
            return;
        }
        let Some(regions) = self
            .files
            .get("region.txt")
            .and_then(|b| Regions::parse(b).ok())
        else {
            return;
        };
        for (name, offset) in [
            ("Normal", 0),
            ("Equalizer", if view.panel == "canvas" { 116 } else { 0 }),
        ] {
            if (name == "Normal" && view.panel == "equalizer")
                || (name == "Equalizer" && view.panel == "main")
            {
                continue;
            }
            if !regions.sections.contains_key(name) {
                continue;
            }
            let mask = regions.mask(name, 275, 116);
            for (x, y, pixel) in image.enumerate_pixels_mut() {
                let sx = x as i32 + origin.0;
                let sy = y as i32 + origin.1 - offset;
                if (0..275).contains(&sx)
                    && (0..116).contains(&sy)
                    && !mask[(sy * 275 + sx) as usize]
                {
                    *pixel = Rgba([0, 0, 0, 0]);
                }
            }
        }
    }
}
fn ensure_region_rows(rows: &[Value], height: u32) -> Result<Vec<bool>> {
    anyhow::ensure!(rows.len() == height as usize, "Mask needs {height} rows");
    let mut mask = Vec::with_capacity(275 * height as usize);
    for row in rows {
        let row = row.as_str().context("Mask rows must be strings")?;
        anyhow::ensure!(
            row.len() == 275 && row.bytes().all(|c| c == b'#' || c == b'.'),
            "Each mask row needs 275 characters: # visible, . transparent"
        );
        mask.extend(row.bytes().map(|c| c == b'#'));
    }
    Ok(mask)
}
#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/regions_tests.rs"]
mod tests;
