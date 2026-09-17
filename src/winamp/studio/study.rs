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
pub fn board(doc: &Document, args: &Value) -> Result<RgbaImage> {
    let rect: [u32; 4] = args
        .get("rect")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()?
        .or(doc.selection)
        .unwrap_or([0, 0, 80, 60]);
    let zoom = args["zoom"].as_u64().unwrap_or(4);
    anyhow::ensure!((1..=8).contains(&zoom), "Study zoom is 1..8");
    let zoom = zoom as u32;
    let mut im = if args["selected"] == true {
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
mod tests {
    use super::*;
    #[test]
    fn value_preview_preserves_native_geometry_alpha_and_source() {
        let im =
            RgbaImage::from_raw(3, 1, vec![255, 0, 0, 23, 0, 255, 0, 127, 0, 0, 255, 255]).unwrap();
        let before = im.clone();
        let v = value_view(&im);
        assert_eq!(im, before);
        assert_eq!(v.dimensions(), im.dimensions());
        assert_eq!(v.get_pixel(0, 0), &Rgba([54, 54, 54, 23]));
        assert_eq!(v.get_pixel(1, 0), &Rgba([182, 182, 182, 127]));
        assert_eq!(v.get_pixel(2, 0), &Rgba([19, 19, 19, 255]));
        let d = Document::blank();
        let status = d.status();
        board(&d, &serde_json::json!({"rect":[0,0,20,20],"values":true})).unwrap();
        assert_eq!(d.status(), status);
    }
    #[test]
    fn studies_are_native_and_read_only() {
        let d = Document::blank();
        let before = d.status();
        let b = board(&d, &serde_json::json!({"rect":[2,3,2,3],"zoom":4})).unwrap();
        assert_eq!(d.status(), before);
        assert_eq!(b.dimensions(), (40, 63));
        for y in 0..3 {
            for x in 0..2 {
                for yy in 0..4 {
                    for xx in 0..4 {
                        assert_eq!(
                            b.get_pixel(16 + x * 4 + xx, 35 + y * 4 + yy),
                            b.get_pixel(16 + x, 16 + y)
                        );
                    }
                }
            }
        }
        assert!(board(&d, &serde_json::json!({"rect":[274,114,2,2]})).is_err());
    }
}
