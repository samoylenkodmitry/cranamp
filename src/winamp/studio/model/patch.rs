//! Isolated native drawing handoffs for artists working on disjoint atlas cells.
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
// A stable optimistic-concurrency token, not a cryptographic authenticity check.
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
    /// Inspect without changing selection; validate/draw on a private document,
    /// then transfer one new plane in a single shared history transaction.
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
            // Replacing a whole plane must not discard art outside ownership.
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
            // Rasterize the complete original geometry, then use the same native
            // per-pixel clip as human painting. Cutting curve geometry first
            // would alter sampling, corner cleanup and the palette ramp.
            stage.view.clip = clipped.then_some(r);
            stage.view.sheet = region["sheet"].as_str().unwrap().into();
            let result = stage.draw(&json!({"operations":ops,"label":part["label"]}))?;
            pixels += result["pixels_written"].as_u64().unwrap_or(0);
        }
        // Defense in depth: even an explicitly clipped part never owns pixels
        // outside its declared rectangles or any unrelated painting plane.
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
mod tests {
    use super::*;
    fn revision_for(d: &mut Document, id: &str, name: &str, r: [u32; 4], ops: Value) -> Value {
        let found=d.patch(&json!({"action":"inspect","replace_layer":id,"parts":[{"sheet":"main.bmp","rect":r}]})).unwrap();
        let mut result = found;
        result["action"] = json!("apply");
        result["name"] = json!(name);
        result["parts"][0]["operations"] = ops;
        result
    }
    #[test]
    fn revision_replaces_only_its_middle_plane_and_is_one_undo() {
        let mut d = Document::blank();
        let a = patch_for(
            &mut d,
            "A",
            "main.bmp",
            [10, 10, 5, 5],
            json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#abcdef"}]),
        );
        d.patch(&a).unwrap();
        let b = patch_for(
            &mut d,
            "B",
            "pledit.bmp",
            [0, 72, 5, 5],
            json!([{"op":"pixel","x":0,"y":72,"color":"#123456"}]),
        );
        d.patch(&b).unwrap();
        let before = d.snapshot();
        let other = d.planes[1].clone();
        d.view.paint_layer = Some("paint-1".into());
        let view = serde_json::to_value(&d.view).unwrap();
        let mut revised = revision_for(
            &mut d,
            "paint-1",
            "A",
            [10, 10, 5, 5],
            json!([{"op":"pixel","x":11,"y":11,"color":"#112233"}]),
        );
        revised["action"] = json!("validate");
        d.patch(&revised).unwrap();
        assert!(d.snapshot() == before);
        revised["action"] = json!("apply");
        d.patch(&revised).unwrap();
        assert_eq!(d.planes.len(), 2);
        assert!(d.planes[1] == other);
        assert_eq!(d.planes[0].id, "paint-1");
        assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
        assert_eq!(d.undo.len(), 3);
        assert_eq!(
            d.composite_images()["main.bmp"].get_pixel(10, 10)[3],
            0,
            "Old marks must be removed, not overpainted"
        );
        assert_eq!(
            d.composite_images()["main.bmp"].get_pixel(11, 11).0,
            [17, 34, 51, 255]
        );
        d.undo();
        assert!(d.snapshot() == before);
        d.redo();
        assert!(d.planes[1] == other);
    }
    #[test]
    fn revision_rejects_incomplete_ownership_and_stale_raw_paint_under_cover() {
        let mut d = Document::blank();
        let a = patch_for(
            &mut d,
            "A",
            "main.bmp",
            [10, 10, 5, 5],
            json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#abcdef"}]),
        );
        d.patch(&a).unwrap();
        let bad = revision_for(
            &mut d,
            "paint-1",
            "A",
            [10, 10, 1, 1],
            json!([{"op":"pixel","x":10,"y":10}]),
        );
        let before = d.snapshot();
        assert!(d.patch(&bad).is_err());
        assert!(d.snapshot() == before);
        let cover = patch_for(
            &mut d,
            "Cover",
            "main.bmp",
            [10, 10, 5, 5],
            json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#000000"}]),
        );
        d.patch(&cover).unwrap();
        let stale = revision_for(
            &mut d,
            "paint-1",
            "A",
            [10, 10, 5, 5],
            json!([{"op":"pixel","x":10,"y":10}]),
        );
        let visible_before = d.composite_images();
        d.planes[0]
            .images
            .get_mut("main.bmp")
            .unwrap()
            .put_pixel(10, 10, Rgba([12, 34, 56, 255]));
        assert_eq!(d.composite_images(), visible_before);
        let before = d.snapshot();
        let rev = d.revision;
        let undo = d.undo.len();
        assert!(d.patch(&stale).is_err());
        assert!(d.snapshot() == before);
        assert_eq!(d.revision, rev);
        assert_eq!(d.undo.len(), undo);
    }
    fn patch_for(d: &mut Document, name: &str, sheet: &str, region: [u32; 4], ops: Value) -> Value {
        let found = d
            .patch(&json!({"parts":[{"sheet":sheet,"rect":region}]}))
            .unwrap();
        let mut part = found["parts"][0].clone();
        part["operations"] = ops;
        json!({"action":"apply","name":name,"parts":[part]})
    }
    #[test]
    fn disjoint_artists_merge_with_unchanged_view_and_one_undo_each() {
        let mut d = Document::blank();
        d.view.mirror_x = true;
        d.view.all_states = true;
        d.view.brush_size = 8;
        d.view.layers = vec!["main.close".into()];
        let view = serde_json::to_value(&d.view).unwrap();
        let a = patch_for(
            &mut d,
            "Portrait",
            "main.bmp",
            [10, 10, 5, 5],
            json!([{"op":"rect","x":10,"y":10,"width":5,"height":5,"color":"#123456"}]),
        );
        let b = patch_for(
            &mut d,
            "Footer",
            "pledit.bmp",
            [0, 72, 10, 10],
            json!([{"op":"pixel","x":4,"y":75,"color":"#abcdef"}]),
        );
        d.patch(&a).unwrap();
        let first = d.snapshot();
        d.patch(&b).unwrap();
        assert_eq!(d.undo.len(), 2);
        assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
        assert_eq!(
            d.composite_images()["pledit.bmp"].get_pixel(4, 75).0,
            [171, 205, 239, 255]
        );
        assert!(d.undo());
        assert!(d.snapshot() == first);
        assert!(d.redo());
    }
    #[test]
    fn stale_and_out_of_region_patches_cannot_partially_modify_art_or_history() {
        let mut d = Document::blank();
        let a = patch_for(
            &mut d,
            "A",
            "main.bmp",
            [10, 10, 5, 5],
            json!([{"op":"pixel","x":10,"y":10,"color":"#abcdef"}]),
        );
        let mut stale = a.clone();
        stale["name"] = json!("stale");
        d.patch(&a).unwrap();
        let before = d.snapshot();
        let rev = d.revision;
        assert!(d.patch(&stale).is_err());
        let mut bad = patch_for(
            &mut d,
            "bad",
            "eqmain.bmp",
            [10, 10, 5, 5],
            json!([{"op":"pixel","x":10,"y":10},{"op":"line","x":14,"y":14,"x2":16,"y2":14}]),
        );
        // The first part stages successfully; failure in a later sheet rolls back all.
        let second = bad["parts"][0].clone();
        bad["parts"] = json!([a["parts"][0].clone(), second]);
        bad["parts"][0]["expected"] = d
            .patch(&json!({"parts":[{"sheet":"main.bmp","rect":[10,10,5,5]}]}))
            .unwrap()["parts"][0]["expected"]
            .clone();
        assert!(d.patch(&bad).is_err());
        assert!(d.snapshot() == before);
        assert_eq!(d.revision, rev);
        assert_eq!(d.undo.len(), 1);
        assert!(validate_bounds(
            &json!({"op":"stamp","x":14,"y":14,"rows":["xx"],"palette":{"x":"#ffffff"}}),
            [10, 10, 5, 5],
            false
        )
        .is_err());
        assert!(validate_bounds(
            &json!({"op":"line","x":10,"y":10,"x2":14,"y2":10,"brush_size":3}),
            [10, 10, 5, 5],
            false
        )
        .is_err());
    }
    #[test]
    fn clipped_curve_parts_assemble_identically_and_preserve_transaction_state() {
        let mut d = Document::blank();
        let existing = patch_for(
            &mut d,
            "Other",
            "main.bmp",
            [80, 80, 1, 1],
            json!([{"op":"pixel","x":80,"y":80,"color":"#aabbcc"}]),
        );
        d.patch(&existing).unwrap();
        let other = d.planes[0].clone();
        d.view.clip = Some([1, 1, 1, 1]);
        d.view.mirror_x = true;
        let view = serde_json::to_value(&d.view).unwrap();
        let mut curve = json!({"op":"curve","x":4,"y":8,"x2":35,"y2":7,
            "control":[20,24],"brush_size":3,"color":"#ffffff",
            "ramp":["#102030","#406080","#aabbcc"],"ramp_axis":[0,0,39,0]});
        let mut reference = Document::blank();
        reference.view.panel = "atlas".into();
        reference.view.sheet = "main.bmp".into();
        reference
            .draw(&json!({"operations":[curve.clone()]}))
            .unwrap();
        let mut patch = patch_for(
            &mut d,
            "Bridge",
            "main.bmp",
            [0, 0, 20, 32],
            json!([curve.clone()]),
        );
        let before = d.snapshot();
        assert!(
            d.patch(&patch).is_err(),
            "Strict default must reject crossing geometry"
        );
        assert!(d.snapshot() == before);
        curve["x"] = json!(34);
        curve["y"] = json!(58);
        curve["x2"] = json!(65);
        curve["y2"] = json!(57);
        curve["control"] = json!([50, 74]);
        curve["ramp_axis"] = json!([30, 50, 69, 50]);
        let second = patch_for(
            &mut d,
            "Unused",
            "eqmain.bmp",
            [50, 50, 20, 32],
            json!([curve]),
        );
        patch["parts"]
            .as_array_mut()
            .unwrap()
            .push(second["parts"][0].clone());
        for part in patch["parts"].as_array_mut().unwrap() {
            part["clip_to_rect"] = json!(true);
        }
        patch["action"] = json!("validate");
        d.patch(&patch).unwrap();
        assert!(d.snapshot() == before);
        assert_eq!(d.undo.len(), 1);
        patch["action"] = json!("apply");
        d.patch(&patch).unwrap();
        assert!(d.planes[0] == other);
        assert_eq!(serde_json::to_value(&d.view).unwrap(), view);
        let composite = d.composite_images();
        for y in 0..32 {
            for x in 0..40 {
                let actual = if x < 20 {
                    composite["main.bmp"].get_pixel(x, y)
                } else {
                    composite["eqmain.bmp"].get_pixel(x + 30, y + 50)
                };
                assert_eq!(
                    actual,
                    reference.images["main.bmp"].get_pixel(x, y),
                    "assembled {x},{y}"
                );
            }
        }
        assert_eq!(d.undo.len(), 2);
        d.undo();
        assert!(d.snapshot() == before);
        d.redo();
        assert!(d.planes[0] == other);
    }
    #[test]
    fn explicit_clip_contains_stamps_at_sheet_edge_and_rejects_invalid_flags() {
        let mut d = Document::blank();
        let mut patch = patch_for(
            &mut d,
            "Edge",
            "main.bmp",
            [0, 0, 2, 2],
            json!([{"op":"stamp","x":-1,"y":-1,"rows":["xxxx","xxxx","xxxx","xxxx"],
                "palette":{"x":"#abcdef"}}]),
        );
        patch["parts"][0]["clip_to_rect"] = json!("true");
        assert!(d.patch(&patch).is_err());
        patch["parts"][0]["clip_to_rect"] = json!(true);
        d.patch(&patch).unwrap();
        let plane = &d.planes[0];
        assert_eq!(plane.images.len(), 1);
        let paint: Vec<_> = plane.images["main.bmp"]
            .enumerate_pixels()
            .filter(|(_, _, p)| p[3] > 0)
            .collect();
        assert_eq!(paint.len(), 4);
        assert!(paint.iter().all(|(x, y, _)| *x < 2 && *y < 2));
        let before = d.snapshot();
        patch["name"] = json!("Bad rect");
        patch["parts"][0]["rect"] = json!([274, 114, 2, 2]);
        assert!(
            d.patch(&patch).is_err(),
            "Clip never permits an out-of-sheet ownership rectangle"
        );
        assert!(d.snapshot() == before);
        assert!(
            validate_bounds(&json!({"op":"pixel","mirror_x":true}), [0, 0, 2, 2], true).is_err()
        );
    }
    #[test]
    fn validation_is_read_only_and_ignores_unrelated_source_changes() {
        let mut d = Document::blank();
        let mut a = patch_for(
            &mut d,
            "A",
            "main.bmp",
            [10, 10, 5, 5],
            json!([{"op":"pixel","x":10,"y":10,"color":"#abcdef"}]),
        );
        let before = d.snapshot();
        a["action"] = json!("validate");
        assert_eq!(d.patch(&a).unwrap()["valid"], true);
        assert!(d.snapshot() == before);
        assert!(d.undo.is_empty());
        assert!(!d.dirty);
        d.images
            .get_mut("main.bmp")
            .unwrap()
            .put_pixel(20, 20, Rgba([1, 2, 3, 255]));
        a["action"] = json!("apply");
        assert!(d.patch(&a).is_ok());
    }
}
