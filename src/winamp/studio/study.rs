use super::model::Document;
use anyhow::Result;
use image::{Rgba, RgbaImage};
use serde_json::Value;
pub fn crop(im: &RgbaImage, rect: [u32; 4]) -> Result<RgbaImage> {
    let [x, y, w, h] = rect;
    anyhow::ensure!(
        w > 0
            && h > 0
            && x.checked_add(w).is_some_and(|n| n <= im.width())
            && y.checked_add(h).is_some_and(|n| n <= im.height()),
        "Study rectangle must fit image"
    );
    Ok(image::imageops::crop_imm(im, x, y, w, h).to_image())
}
/// Exact colours in a native crop, ordered by coverage with deterministic ties.
pub fn palette(im: &RgbaImage, rect: [u32; 4]) -> Result<Value> {
    let region = crop(im, rect)?;
    let mut counts = std::collections::BTreeMap::<[u8; 4], u32>::new();
    for pixel in region.pixels() {
        *counts.entry(pixel.0).or_default() += 1;
    }
    let unique_colors = counts.len();
    let mut colors: Vec<_> = counts.into_iter().collect();
    colors.sort_by_key(|(rgba, count)| (std::cmp::Reverse(*count), *rgba));
    let colors: Vec<_> = colors.into_iter().take(32).map(|(rgba, count)| {
        serde_json::json!({"rgba": rgba, "hex": format!("#{:02x}{:02x}{:02x}{:02x}", rgba[0], rgba[1], rgba[2], rgba[3]), "pixels": count})
    }).collect();
    Ok(
        serde_json::json!({"rect": rect, "pixels": u64::from(rect[2]) * u64::from(rect[3]), "unique_colors": unique_colors, "truncated": unique_colors > 32, "colors": colors}),
    )
}

pub fn enlarged(im: &RgbaImage, zoom: u32, grid: bool) -> RgbaImage {
    let mut out = image::imageops::resize(
        im,
        im.width() * zoom,
        im.height() * zoom,
        image::imageops::FilterType::Nearest,
    );
    if grid && zoom >= 4 {
        for (x, y, p) in out.enumerate_pixels_mut() {
            if x % zoom == 0 || y % zoom == 0 {
                for c in &mut p.0[..3] {
                    *c = ((*c as u16 * 3 + 24) / 4) as u8;
                }
            }
        }
    }
    out
}
pub fn value_view(im: &RgbaImage) -> RgbaImage {
    let mut out = im.clone();
    for p in out.pixels_mut() {
        let value = ((54 * p[0] as u32 + 183 * p[1] as u32 + 19 * p[2] as u32 + 128) / 256) as u8;
        p[0] = value;
        p[1] = value;
        p[2] = value;
    }
    out
}
pub fn context_rect(rect: [u32; 4], size: (u32, u32), padding: u32) -> Result<[u32; 4]> {
    let [x, y, w, h] = rect;
    anyhow::ensure!(
        w > 0
            && h > 0
            && x.checked_add(w).is_some_and(|n| n <= size.0)
            && y.checked_add(h).is_some_and(|n| n <= size.1),
        "Study rectangle must fit image"
    );
    let left = x.saturating_sub(padding);
    let top = y.saturating_sub(padding);
    Ok([
        left,
        top,
        (x + w).saturating_add(padding).min(size.0) - left,
        (y + h).saturating_add(padding).min(size.1) - top,
    ])
}

pub fn region(doc: &Document, args: &Value) -> Result<([u32; 4], [u32; 4], u32)> {
    let size = doc.canvas_size();
    let rect: [u32; 4] = args
        .get("rect")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()?
        .or(doc.selection)
        .unwrap_or([0, 0, 80.min(size.0), 60.min(size.1)]);
    let padding = args
        .get("padding")
        .map(|v| {
            v.as_u64()
                .filter(|n| *n <= 32)
                .ok_or_else(|| anyhow::anyhow!("Study padding is an integer 0..32"))
        })
        .transpose()?
        .unwrap_or(4) as u32;
    Ok((rect, context_rect(rect, size, padding)?, padding))
}

pub fn report(doc: &Document, args: &Value) -> Result<Value> {
    let (requested, rect, padding) = region(doc, args)?;
    let (coverage, _) = doc.coverage(rect, false)?;
    Ok(serde_json::json!({
        "requested_rect": requested, "context_rect": rect, "padding": padding,
        "paintability": coverage,
        "flat_drawable_areas": doc.flat_regions(rect)?,
        "transparency": doc.transparency_report(),
        "state_order": if args["states"] == true {
            serde_json::json!([{"active":true,"pressed":false},{"active":true,"pressed":true},
                {"active":false,"pressed":false},{"active":false,"pressed":true}])
        } else { serde_json::json!([{"active":doc.view.active,"pressed":doc.view.pressed}]) },
        "note": "Editor composites, not GPU screenshots. Read the exterior halo and every overlapping target/state; runtime footprints are not exclusion masks. Use studio_screenshot for the live player. No pixels, selection, view or history are changed."
    }))
}

pub fn board(doc: &Document, args: &Value) -> Result<RgbaImage> {
    if args["states"] != true {
        return board_view(doc, args, None);
    }
    anyhow::ensure!(
        args["selected"] != true,
        "State studies show the full composite; omit selected"
    );
    anyhow::ensure!(
        doc.view.panel != "atlas",
        "State studies need the canvas, not an atlas"
    );
    let mut panels = Vec::new();
    for (active, pressed, label) in [
        (true, false, "ON / UP"),
        (true, true, "ON / DOWN"),
        (false, false, "OFF / UP"),
        (false, true, "OFF / DOWN"),
    ] {
        let mut view = doc.view.clone();
        view.active = active;
        view.pressed = pressed;
        panels.push((board_view(doc, args, Some(&view))?, label));
    }
    let cell_w = panels[0].0.width().max(96);
    let cell_h = panels[0].0.height() + 12;
    anyhow::ensure!(
        cell_w * 2 <= 8192 && cell_h * 2 <= 8192,
        "State study too large; use a smaller rectangle"
    );
    let mut out = RgbaImage::from_pixel(cell_w * 2, cell_h * 2, Rgba([23, 28, 38, 255]));
    for (index, (panel, label)) in panels.iter().enumerate() {
        let x = index as u32 % 2 * cell_w;
        let y = index as u32 / 2 * cell_h;
        image::imageops::overlay(&mut out, panel, x as i64, (y + 12) as i64);
        for (i, ch) in label.chars().enumerate() {
            for (dy, bits) in crate::winamp::pixel_text::glyph(ch)
                .unwrap_or([0; 7])
                .iter()
                .enumerate()
            {
                for dx in 0..5 {
                    if bits & (1 << (4 - dx)) != 0 {
                        out.put_pixel(
                            x + 16 + i as u32 * 6 + dx,
                            y + 3 + dy as u32,
                            Rgba([197, 208, 219, 255]),
                        );
                    }
                }
            }
        }
    }
    Ok(out)
}

fn board_view(
    doc: &Document,
    args: &Value,
    view: Option<&super::model::View>,
) -> Result<RgbaImage> {
    let (_, rect, _) = region(doc, args)?;
    let zoom = args["zoom"].as_u64().unwrap_or(4);
    anyhow::ensure!((1..=8).contains(&zoom), "Study zoom is 1..8");
    let zoom = zoom as u32;
    let mut im = if let Some(view) = view {
        let (w, h) = doc.canvas_size();
        doc.render_patch_for(view, [0, 0, w as i32, h as i32]).image
    } else if args["selected"] == true {
        doc.selected_image()
    } else {
        doc.render()
    };
    if args["geometry"] == true {
        for l in doc.layers() {
            let [x, y, w, h] = l.destination;
            for yy in y..(y + h).min(im.height()) {
                for xx in x..(x + w).min(im.width()) {
                    if (xx == x || yy == y || xx == x + w - 1 || yy == y + h - 1)
                        && (xx + yy) % 2 == 0
                    {
                        im.put_pixel(xx, yy, Rgba([245, 115, 146, 255]));
                    }
                }
            }
        }
    }
    let mut part = crop(&im, rect)?;
    if args["values"] == true {
        part = value_view(&part);
    }
    let detail = enlarged(&part, zoom, args["grid"] == true);
    let reference = if let Some(path) = args["reference"].as_str() {
        let reader = image::ImageReader::open(path)?.with_guessed_format()?;
        let image = reader.decode()?.to_rgba8();
        let r = args
            .get("reference_rect")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or([0, 0, image.width(), image.height()]);
        let part = crop(&image, r)?;
        Some(if args["values"] == true {
            value_view(&part)
        } else {
            part
        })
    } else {
        None
    };
    let gap = 16;
    let native_h = part
        .height()
        .max(reference.as_ref().map_or(0, |i| i.height()));
    let width = detail.width() + reference.as_ref().map_or(0, |i| gap + i.width() * zoom) + gap * 2;
    let height = gap * 3
        + native_h
        + detail
            .height()
            .max(reference.as_ref().map_or(0, |i| i.height() * zoom));
    anyhow::ensure!(
        width <= 8192 && height <= 8192,
        "Study board too large; crop the reference"
    );
    let mut out = RgbaImage::from_pixel(width, height, Rgba([23, 28, 38, 255]));
    image::imageops::overlay(&mut out, &part, gap as i64, gap as i64);
    image::imageops::overlay(&mut out, &detail, gap as i64, (gap * 2 + native_h) as i64);
    if let Some(r) = reference {
        let x = gap * 2 + detail.width();
        image::imageops::overlay(&mut out, &r, x as i64, gap as i64);
        image::imageops::overlay(
            &mut out,
            &enlarged(&r, zoom, args["grid"] == true),
            x as i64,
            (gap * 2 + native_h) as i64,
        );
    }
    Ok(out)
}
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/study/tests.rs"]
mod tests;
