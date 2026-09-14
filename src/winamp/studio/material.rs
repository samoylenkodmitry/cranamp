//! Native, silhouette-driven glass shading. Readback and refraction use nearest
//! native pixels. This paints colors; it never resizes or filters an image.
use anyhow::{bail, Result};
use image::RgbaImage;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type Point = [i32; 2];
fn unit(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).max(0.)
}
pub fn glass(
    points: &[Point],
    under: &RgbaImage,
    tint: [u8; 4],
    op: &Value,
) -> Result<BTreeMap<Point, [u8; 4]>> {
    let bevel = op["bevel"].as_f64().unwrap_or(8.);
    let refract = op["refraction"].as_f64().unwrap_or(4.);
    anyhow::ensure!(
        bevel.is_finite() && (1. ..=128.).contains(&bevel),
        "bevel is 1..128 native pixels"
    );
    anyhow::ensure!(
        refract.is_finite() && (0. ..=32.).contains(&refract),
        "refraction is 0..32 native pixels"
    );
    let inside: BTreeSet<_> = points
        .iter()
        .copied()
        .filter(|p| {
            p[0] >= 0
                && p[1] >= 0
                && p[0] < (under.width() as i32)
                && p[1] < (under.height() as i32)
        })
        .collect();
    anyhow::ensure!(inside.len() <= 150_000, "Glass shape too large");
    let x = op["x"].as_f64().unwrap_or(0.);
    let y = op["y"].as_f64().unwrap_or(0.);
    let w = op["width"].as_f64().unwrap_or(1.);
    let h = op["height"].as_f64().unwrap_or(1.);
    let contour: Vec<[f64; 2]> = match op["op"].as_str() {
        Some("path") => super::brush::path_contour(op)?,
        Some("ellipse") => (0..720)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 720.;
                [
                    x + (w - 1.) / 2. + a.cos() * w / 2.,
                    y + (h - 1.) / 2. + a.sin() * h / 2.,
                ]
            })
            .collect(),
        Some("rect") => vec![
            [x - 0.5, y - 0.5],
            [x + w - 0.5, y - 0.5],
            [x + w - 0.5, y + h - 0.5],
            [x - 0.5, y + h - 0.5],
        ],
        _ => bail!("Glass material needs a filled ellipse, rectangle or path"),
    };
    anyhow::ensure!(
        contour.len() >= 3 && inside.len().saturating_mul(contour.len()) <= 100_000_000,
        "Glass outline too complex; shade smaller regions"
    );
    let light = unit([-0.45, -0.7, 1.]);
    let soft = unit([-0.1, -0.7, 1.]);
    let back = unit([0.4, 0.65, 0.7]);
    let mut out = BTreeMap::new();
    for p in &inside {
        // Surface normals come from the unrounded construction curve, not
        // the staircase of the raster mask. No image/height blur is required.
        let px = p[0] as f64;
        let py = p[1] as f64;
        let mut nearest = (f64::MAX, 0., 0.);
        for i in 0..contour.len() {
            let a = contour[i];
            let b = contour[(i + 1) % contour.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let l = dx * dx + dy * dy;
            let t = if l > 0. {
                ((px - a[0]) * dx + (py - a[1]) * dy) / l
            } else {
                0.
            }
            .clamp(0., 1.);
            let vx = px - (a[0] + t * dx);
            let vy = py - (a[1] + t * dy);
            let ds = vx * vx + vy * vy;
            if ds < nearest.0 {
                nearest = (ds, vx, vy);
            }
        }
        let dist = nearest.0.sqrt().max(0.05);
        let t = (dist / bevel).min(1.);
        let slope = 0.65 * (1. - t) / (2. * t - t * t).sqrt();
        let n = unit([-nearest.1 / dist * slope, -nearest.2 / dist * slope, 1.]);
        let xx = (p[0] as f64 + n[0] * refract)
            .round()
            .clamp(0., under.width() as f64 - 1.) as u32;
        let yy = (p[1] as f64 + n[1] * refract)
            .round()
            .clamp(0., under.height() as f64 - 1.) as u32;
        let base = under.get_pixel(xx, yy).0;
        let fresnel = (1. - n[2]).powi(3);
        let shine = dot(n, light).powi(44) * 0.95 + dot(n, soft).powi(10) * 0.16;
        let secondary = dot(n, back).powi(28) * 0.26;
        let edge = !inside.contains(&[p[0] - 1, p[1]])
            || !inside.contains(&[p[0] + 1, p[1]])
            || !inside.contains(&[p[0], p[1] - 1])
            || !inside.contains(&[p[0], p[1] + 1]);
        let mut color = [0, 0, 0, 255];
        for i in 0..3 {
            let transmitted = base[i] as f64 * 0.68 + tint[i] as f64 * 0.20;
            let reflection = tint[i] as f64 * (0.05 + fresnel * 0.35)
                + [243., 255., 253.][i] * (shine + secondary);
            let mut c = transmitted + reflection;
            if edge {
                c *= 0.56 + dot(n, light) * 0.28;
            }
            color[i] = c.round().clamp(0., 255.) as u8;
        }
        out.insert(*p, color);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_glass_is_bounded_and_refracts_without_resizing() {
        let mut under = RgbaImage::new(24, 24);
        for (x, y, p) in under.enumerate_pixels_mut() {
            *p = image::Rgba([(x * 9) as u8, (y * 9) as u8, 30, 255]);
        }
        let original = under.clone();
        let pts = super::super::brush::rasterize(
            &serde_json::json!({"op":"ellipse","x":3,"y":4,"width":16,"height":14,"fill":true}),
        )
        .unwrap();
        let a = glass(
            &pts,
            &under,
            [120, 200, 220, 255],
            &serde_json::json!({"op":"ellipse","x":3,"y":4,"width":16,"height":14,"bevel":7,"refraction":0}),
        )
        .unwrap();
        let b = glass(
            &pts,
            &under,
            [120, 200, 220, 255],
            &serde_json::json!({"op":"ellipse","x":3,"y":4,"width":16,"height":14,"bevel":7,"refraction":5}),
        )
        .unwrap();
        assert_eq!(under, original);
        assert_eq!(a.len(), pts.len());
        assert_ne!(a, b);
        assert!(b.values().all(|c| c[3] == 255));
        assert!(b.keys().all(|p| pts.contains(p)));
        assert!(glass(&pts, &under, [0; 4], &serde_json::json!({"bevel":0})).is_err());
    }
}
