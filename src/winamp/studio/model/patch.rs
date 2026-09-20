use super::*;
fn rect(part: &Value, images: &BTreeMap<String, RgbaImage>) -> Result<(String, [u32; 4])> {
    let sheet = part["sheet"]
        .as_str()
        .context("Patch sheet required")?
        .to_owned();
    let r: [u32; 4] =
        serde_json::from_value(part["rect"].clone()).context("Patch rect is [x,y,width,height]")?;
    let im = images.get(&sheet).context("Unknown patch sheet")?;
    anyhow::ensure!(
        r[2] > 0
            && r[3] > 0
            && r[0].checked_add(r[2]).is_some_and(|v| v <= im.width())
            && r[1].checked_add(r[3]).is_some_and(|v| v <= im.height()),
        "Patch rectangle outside {sheet}"
    );
    Ok((sheet, r))
}
fn fingerprint(im: &RgbaImage, r: [u32; 4]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for y in r[1]..r[1] + r[3] {
        for x in r[0]..r[0] + r[2] {
            for b in im.get_pixel(x, y).0 {
                hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
            }
        }
    }
    format!("rgba-fnv1a64:{hash:016x}")
}
fn layer_fingerprint(plane: &PaintLayer, index: usize) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
        }
    };
    feed(
        format!(
            "{index}|{}|{}|{}|{}|{}|{}",
            plane.id, plane.name, plane.visible, plane.locked, plane.opacity, plane.clip_below
        )
        .as_bytes(),
    );
    for (name, im) in &plane.images {
        feed(name.as_bytes());
        feed(&im.width().to_le_bytes());
        feed(&im.height().to_le_bytes());
        feed(im.as_raw());
    }
    format!("plane-fnv1a64:{hash:016x}")
}
fn validate_bounds(op: &Value, r: [u32; 4], clip_to_rect: bool) -> Result<()> {
    let inside = |[x, y]: [i32; 2]| {
        x >= r[0] as i32 && y >= r[1] as i32 && x < (r[0] + r[2]) as i32 && y < (r[1] + r[3]) as i32
    };
    anyhow::ensure!(
        !op["mirror_x"].as_bool().unwrap_or(false) && !op["mirror_y"].as_bool().unwrap_or(false),
        "Patch mirrors must be authored explicitly"
    );
    let kind = op["op"].as_str().context("Patch operation kind required")?;
    if kind == "stamp" {
        let x = integer(op, "x", 0)?;
        let y = integer(op, "y", 0)?;
        let pal = op["palette"]
            .as_object()
            .context("Stamp palette required")?;
        let rows = op["rows"].as_array().context("Stamp rows required")?;
        anyhow::ensure!(rows.len() <= 1024, "Stamp too tall");
        for (yy, row) in rows.iter().enumerate() {
            let row = row.as_str().context("Stamp row string required")?;
            anyhow::ensure!(row.chars().count() <= 1024, "Stamp too wide");
            for (xx, ch) in row.chars().enumerate() {
                if pal.contains_key(&ch.to_string()) {
                    anyhow::ensure!(
                        clip_to_rect || inside([x + xx as i32, y + yy as i32]),
                        "Stamp pixel outside assigned patch rectangle"
                    );
                }
            }
        }
    } else {
        anyhow::ensure!(
            ["pixel", "line", "rect", "ellipse", "path", "curve", "tuft"].contains(&kind),
            "Patch accepts native geometry and explicit stamps only"
        );
        let mut shape = op.clone();
        if kind == "rect" && shape.get("fill").is_none() {
            shape["fill"] = json!(true);
        }
        for p in crate::winamp::studio::brush::rasterize(&shape)? {
            anyhow::ensure!(
                clip_to_rect || inside(p),
                "Native stroke pixel {p:?} outside assigned patch rectangle {r:?}"
            );
        }
    }
    Ok(())
}
impl Document {
    pub fn patch(&mut self, args: &Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("inspect");
        anyhow::ensure!(
            ["inspect", "validate", "apply"].contains(&action),
            "Unknown patch action"
        );
        anyhow::ensure!(
            self.stroke.is_none(),
            "Finish the current human stroke before a patch"
        );
        let parts = args["parts"].as_array().context("Patch parts required")?;
        anyhow::ensure!(
            !parts.is_empty() && parts.len() <= 64,
            "Patch needs 1..64 parts"
        );
        let composite = self.composite_images();
        let replacement = args
            .get("replace_layer")
            .map(|v| -> Result<usize> {
                let id = v
                    .as_str()
                    .context("replace_layer must be a painting layer ID")?;
                self.planes
                    .iter()
                    .position(|p| p.id == id)
                    .context("Unknown replacement painting layer")
            })
            .transpose()?;
        let layer_token = replacement.map(|i| layer_fingerprint(&self.planes[i], i));
        let mut regions = Vec::new();
        for part in parts {
            let (sheet, r) = rect(part, &composite)?;
            let clip_to_rect = part
                .get("clip_to_rect")
                .map(|v| v.as_bool().context("clip_to_rect must be boolean"))
                .transpose()?
                .unwrap_or(false);
            let token = fingerprint(&composite[&sheet], r);
            if action != "inspect" {
                anyhow::ensure!(part["expected"].as_str()==Some(&token),"Patch baseline changed in {sheet} {r:?}; inspect and review against the current art");
            }
            regions
                .push(json!({"sheet":sheet,"rect":r,"expected":token,"clip_to_rect":clip_to_rect}));
        }
        if action == "inspect" {
            return Ok(
                json!({"parts":regions,"replace_layer":replacement.map(|i|&self.planes[i].id),"layer_expected":layer_token}),
            );
        }
        if let Some(i) = replacement {
            anyhow::ensure!(
                args["layer_expected"].as_str() == layer_token.as_deref(),
                "Replacement layer changed; inspect and review its current paint"
            );
            let plane = &self.planes[i];
            anyhow::ensure!(
                plane.visible && !plane.locked && plane.opacity == 255 && !plane.clip_below,
                "Replacement needs a visible, unlocked, opaque, unclipped plane"
            );
            anyhow::ensure!(
                !self.planes.get(i + 1).is_some_and(|p| p.clip_below),
                "Replace a clipped follower before its mask source"
            );
            for (sheet, im) in &plane.images {
                for (x, y, _) in im.enumerate_pixels().filter(|(_, _, p)| p[3] > 0) {
                    anyhow::ensure!(regions.iter().any(|v| {
                        let r: [u32;4]=serde_json::from_value(v["rect"].clone()).unwrap();
                        v["sheet"].as_str()==Some(sheet.as_str()) && x>=r[0] && y>=r[1] && x<r[0]+r[2] && y<r[1]+r[3]
                    }),"Replacement rectangles do not cover existing paint on {sheet} at [{x},{y}]");
                }
            }
        }
        let name = args["name"]
            .as_str()
            .context("Patch painting layer name required")?;
        anyhow::ensure!(
            !name.trim().is_empty() && name.chars().count() <= 80,
            "Patch name needs 1..80 characters"
        );
        anyhow::ensure!(
            !self
                .planes
                .iter()
                .enumerate()
                .any(|(i, p)| p.name == name && Some(i) != replacement),
            "A painting layer named {name} already exists"
        );
        anyhow::ensure!(
            replacement.is_some() || self.planes.len() < MAX_PAINT_LAYERS,
            "At most {MAX_PAINT_LAYERS} painting layers"
        );
        let total: usize = parts
            .iter()
            .map(|p| p["operations"].as_array().map_or(0, Vec::len))
            .sum();
        anyhow::ensure!(
            total > 0 && total <= 10000,
            "Patch needs 1..10000 operations"
        );
        let mut stage = Document::blank();
        stage.restore(self.snapshot());
        let id = replacement
            .map(|i| self.planes[i].id.clone())
            .unwrap_or_else(|| {
                (1..)
                    .map(|i| format!("paint-{i}"))
                    .find(|id| !stage.planes.iter().any(|p| &p.id == id))
                    .unwrap()
            });
        let fresh = PaintLayer {
            id: id.clone(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 255,
            clip_below: false,
            images: BTreeMap::new(),
        };
        let plane_index = if let Some(i) = replacement {
            stage.planes[i] = fresh;
            i
        } else {
            stage.planes.push(fresh);
            stage.planes.len() - 1
        };
        stage.view.paint_layer = Some(id.clone());
        stage.view.panel = "atlas".into();
        stage.view.layer = "sheet".into();
        let mut pixels = 0;
        for (part, region) in parts.iter().zip(&regions) {
            let ops = part["operations"]
                .as_array()
                .context("Every patch part needs operations")?;
            let r = serde_json::from_value(region["rect"].clone())?;
            let clipped = region["clip_to_rect"].as_bool().unwrap();
            for op in ops {
                validate_bounds(op, r, clipped)?;
            }
            stage.view.clip = clipped.then_some(r);
            stage.view.sheet = region["sheet"].as_str().unwrap().into();
            let result = stage.draw(&json!({"operations":ops,"label":part["label"]}))?;
            pixels += result["pixels_written"].as_u64().unwrap_or(0);
        }
        for (sheet, im) in &stage.planes[plane_index].images {
            for (x, y, _) in im.enumerate_pixels().filter(|(_, _, p)| p[3] > 0) {
                anyhow::ensure!(
                    regions.iter().any(|v| {
                        let r: [u32; 4] = serde_json::from_value(v["rect"].clone()).unwrap();
                        v["sheet"].as_str() == Some(sheet.as_str())
                            && x >= r[0]
                            && y >= r[1]
                            && x < r[0] + r[2]
                            && y < r[1] + r[3]
                    }),
                    "Staged paint escaped declared ownership on {sheet} at [{x},{y}]"
                );
            }
        }
        if action == "validate" {
            return Ok(json!({"valid":true,"pixels_written":pixels,"parts":regions}));
        }
        let before = self.snapshot();
        let finished = stage.planes.remove(plane_index);
        if let Some(i) = replacement {
            self.planes[i] = finished;
        } else {
            self.planes.push(finished);
        }
        self.changed();
        let verb = if replacement.is_some() {
            "Revise"
        } else {
            "Regional patch"
        };
        self.record(before, format!("{verb} · {name}"), "MCP");
        self.message = format!("{verb} · {name} · {} regions · one undo", parts.len());
        Ok(
            json!({"applied":true,"paint_layer":id,"pixels_written":pixels,"revision":self.revision}),
        )
    }
}
#[cfg(test)]
#[path = "../../../../test/unit/winamp/studio/model/patch/tests.rs"]
mod tests;
