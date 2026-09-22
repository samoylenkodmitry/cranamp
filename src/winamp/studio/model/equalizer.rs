//! Shared classic EQ frames and reversible cropped-bitmap artwork mode.
use super::*;

impl Document {
    pub(super) fn export_images(&self) -> BTreeMap<String, RgbaImage> {
        let mut images = self.composite_images();
        if self.eq_artwork_only {
            if let Some(image) = images.get_mut("eqmain.bmp") {
                if image.height() >= 163 {
                    *image = image::imageops::crop_imm(image, 0, 0, image.width(), 163).to_image();
                }
            }
        }
        images
    }

    pub fn set_eq_artwork_only(&mut self, enabled: bool, source: &str) -> Result<()> {
        let image = self
            .images
            .get("eqmain.bmp")
            .context("eqmain.bmp is missing")?;
        anyhow::ensure!(
            image.width() >= 275 && image.height() >= 163,
            "EQ artwork needs the complete 275x163 background and button area"
        );
        if !enabled {
            anyhow::ensure!(image.height() >= 315, "This imported skin omitted its EQ graphics. There are no preserved frames to restore; open a full-size EQ project to edit visible sliders.");
        }
        if enabled != self.eq_artwork_only {
            self.finish_stroke();
            self.record(
                self.snapshot(),
                if enabled {
                    "EQ artwork only"
                } else {
                    "Restore EQ controls"
                }
                .into(),
                source,
            );
            self.eq_artwork_only = enabled;
            self.view.layers.clear();
            self.view.layer = "auto".into();
            self.changed();
        }
        self.message = if enabled {
            "EQ artwork only: tracks, handles, graph and preset graphics are omitted; invisible controls still respond. Full source art stays in the project."
        } else {
            "EQ controls restored. The same 28 track frames and two handle states serve all eleven bands."
        }.into();
        Ok(())
    }

    pub fn eq_workbench(&mut self, args: &Value, source: &str) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("inspect");
        let frame = args
            .get("frame")
            .map(|v| {
                v.as_u64()
                    .filter(|n| *n < 28)
                    .context("frame must be 0..27")
            })
            .transpose()?
            .unwrap_or(self.view.eq[0] as u64) as u8;
        let scope = args["scope"].as_str().unwrap_or("current");
        anyhow::ensure!(
            matches!(scope, "current" | "all"),
            "scope is current or all"
        );
        match action {
            "inspect" => {}
            "set_mode" => {
                let mode = args["mode"]
                    .as_str()
                    .context("mode is controls or artwork")?;
                anyhow::ensure!(
                    matches!(mode, "controls" | "artwork"),
                    "mode is controls or artwork"
                );
                self.set_eq_artwork_only(mode == "artwork", source)?;
            }
            "edit_frame" | "edit_handle" => {
                anyhow::ensure!(
                    !self.eq_artwork_only,
                    "Restore EQ controls before editing shared slider cells"
                );
                let part = if action == "edit_frame" {
                    "track"
                } else {
                    "thumb"
                };
                let pressed = args["pressed"].as_bool().unwrap_or(self.view.pressed);
                self.state(json!({"panel":"canvas","eq":vec![frame;11],"layers":[format!("equalizer.band1.{part}")],"states":scope,"pressed":pressed,"clip":null,"presentation":false,"drawer":"equalizer"}))?;
                self.message = format!("Editing shared EQ {part}, frame {frame}/27: the same source appears in ALL eleven bands. Scope: {scope}.");
            }
            "edit_background" => {
                self.state(json!({"panel":"canvas","layers":["equalizer.background"],"states":"all","clip":null,"presentation":false,"drawer":"equalizer"}))?;
                self.message = "Painting the continuous EQ background. In controls mode, shared opaque sliders cover it; use artwork mode to reveal it.".into();
            }
            "preview" => {
                let values: Vec<u8> = match args["pattern"].as_str().unwrap_or("flat") {
                    "flat" => vec![frame; 11],
                    "alternating" => (0..11).map(|i| if i % 2 == 0 { 0 } else { 27 }).collect(),
                    "ramp" => (0..11).map(|i| (i * 27 / 10) as u8).collect(),
                    _ => bail!("pattern is flat, alternating or ramp"),
                };
                self.state(json!({"eq":values}))?;
            }
            "copy_frame_to_all" => {
                anyhow::ensure!(
                    !self.eq_artwork_only,
                    "Restore EQ controls before copying shared frames"
                );
                let mut view = self.view.clone();
                view.panel = "canvas".into();
                view.eq = [frame; 11];
                let layer = self
                    .layers_for(&view)
                    .into_iter()
                    .find(|l| l.id == "equalizer.band1.track")
                    .context("EQ track missing")?;
                let images = self.composite_images();
                let image = &images[&layer.sheet];
                let [x, y, w, h] = layer.source;
                anyhow::ensure!(
                    x + w <= image.width() && y + h <= image.height(),
                    "The chosen EQ frame is incomplete"
                );
                let mut palette = serde_json::Map::new();
                let mut colors = BTreeMap::new();
                let mut rows = Vec::new();
                for py in y..y + h {
                    let mut row = String::new();
                    for px in x..x + w {
                        let p = image.get_pixel(px, py).0;
                        let next = char::from_u32(0xe000 + colors.len() as u32).unwrap();
                        let c = *colors.entry(p).or_insert(next);
                        palette.insert(
                            c.to_string(),
                            json!(format!("#{:02x}{:02x}{:02x}{:02x}", p[0], p[1], p[2], p[3])),
                        );
                        row.push(c);
                    }
                    rows.push(row);
                }
                let old_view = self.view.clone();
                self.view = view;
                self.view.alpha_lock = false;
                self.view.mask_colors.clear();
                self.view.clip = None;
                self.view.mirror_x = false;
                self.view.mirror_y = false;
                self.view.opacity = 255;
                self.view.grain = 0;
                let result = self.draw(&json!({"layers":[layer.id],"origin":layer.id,"states":"all","label":format!("Copy EQ frame {frame} to all 28 shared frames"),"operations":[{"op":"stamp","x":0,"y":0,"rows":rows,"palette":palette}]}));
                self.view = old_view;
                let result = result?;
                return Ok(
                    json!({"drawing":result,"equalizer":self.eq_workbench(&json!({}),source)?}),
                );
            }
            _ => bail!("Unknown EQ workbench action {action}"),
        }
        let mut view = self.view.clone();
        view.panel = "equalizer".into();
        let layers = mapping::layers(&view);
        let track = layers
            .iter()
            .find(|l| l.id == "band1.track")
            .context("EQ mapping missing")?;
        Ok(json!({
            "mode":if self.eq_artwork_only { "artwork" } else { "controls" },
            "frame":self.view.eq[0], "positions":self.view.eq, "selected":self.view.layers,
            "shared_by":11, "track_size":[14,63], "track_frames":track.variants,
            "handle_states":[[0,164,11,11],[0,176,11,11]],
            "destinations":layers.iter().filter(|l|l.id.ends_with(".track")).map(|l|json!({"id":format!("equalizer.{}",l.id),"rect":[l.destination[0],l.destination[1]+116,l.destination[2],l.destination[3]]})).collect::<Vec<_>>(),
            "explanation":"28 level frames, not 28 independent bands: every band shares this bank and the two handle cells. Paint a repeating track once; use mixed-position previews. Unique continuous background art requires omitting the slider graphics.",
            "artwork_mode":"Exports eqmain.bmp at 275x163, as used by the Björk reference. Slider/graph/preset graphics disappear, but hit targets remain. The project keeps full source pixels for restoration. Verify the cropped-bitmap convention in target players.",
            "can_restore_controls":self.images.get("eqmain.bmp").is_some_and(|im|im.height()>=315)
        }))
    }
}

#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/equalizer_tests.rs"]
mod tests;
