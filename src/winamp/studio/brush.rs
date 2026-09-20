use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;
type Point = [i32; 2];
fn integer(v: &Value, key: &str, default: i32) -> Result<i32> {
    let n = v
        .get(key)
        .map(|x| {
            x.as_i64()
                .with_context(|| format!("{key} is a whole number of pixels (got {x})"))
        })
        .transpose()?
        .unwrap_or(default as i64);
    anyhow::ensure!(
        (-4096..=4096).contains(&n),
        "{key} is -4096..4096 (got {n})"
    );
    Ok(n as i32)
}
fn number(v: &Value, key: &str, default: f64) -> Result<f64> {
    let n = v
        .get(key)
        .map(|x| {
            x.as_f64()
                .with_context(|| format!("{key} is a number of pixels (got {x})"))
        })
        .transpose()?
        .unwrap_or(default);
    anyhow::ensure!(
        n.is_finite() && (-4096. ..=4096.).contains(&n),
        "{key} is -4096..4096 (got {n})"
    );
    Ok(n)
}
fn segment(a: Point, b: Point, out: &mut BTreeSet<Point>) {
    walk_segment(a, b, |p| {
        out.insert(p);
    });
}
fn walk_segment(a: Point, b: Point, mut visit: impl FnMut(Point)) {
    let [mut x, mut y] = a;
    let dx = (b[0] - x).abs();
    let dy = -(b[1] - y).abs();
    let sx = if x < b[0] { 1 } else { -1 };
    let sy = if y < b[1] { 1 } else { -1 };
    let mut e = dx + dy;
    loop {
        visit([x, y]);
        if [x, y] == b {
            break;
        }
        let e2 = 2 * e;
        if e2 >= dy {
            e += dy;
            x += sx;
        }
        if e2 <= dx {
            e += dx;
            y += sy;
        }
    }
}
fn clean_corners(points: &[Point]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    for &p in points {
        if out.last() == Some(&p) {
            continue;
        }
        if out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            let orthogonal = |u: Point, v: Point| (u[0] - v[0]).abs() + (u[1] - v[1]).abs() == 1;
            if orthogonal(a, b)
                && orthogonal(b, p)
                && (a[0] - p[0]).abs() == 1
                && (a[1] - p[1]).abs() == 1
            {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}
fn polygon(points: &[Point], out: &mut BTreeSet<Point>) {
    if points.len() < 3 {
        return;
    }
    let lo = points.iter().map(|p| p[1]).min().unwrap();
    let hi = points.iter().map(|p| p[1]).max().unwrap();
    for y in lo..=hi {
        let mut cuts = Vec::new();
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if (a[1] <= y && b[1] > y) || (b[1] <= y && a[1] > y) {
                cuts.push(
                    a[0] as f64 + (y - a[1]) as f64 * (b[0] - a[0]) as f64 / (b[1] - a[1]) as f64,
                );
            }
        }
        cuts.sort_by(f64::total_cmp);
        for pair in cuts.as_chunks::<2>().0 {
            for x in pair[0].ceil() as i32..=pair[1].floor() as i32 {
                out.insert([x, y]);
            }
        }
    }
}
pub fn rasterize(op: &Value) -> Result<Vec<Point>> {
    let kind = op["op"].as_str().context("op required")?;
    let width = integer(op, "brush_size", 1)?;
    anyhow::ensure!((1..=32).contains(&width), "brush_size must be 1..32");
    if kind == "curve" || kind == "tuft" {
        let (x, y) = (number(op, "x", 0.)?, number(op, "y", 0.)?);
        let end = [number(op, "x2", x)?, number(op, "y2", y)?];
        let bend = integer(op, "curve_bend", 35)?;
        anyhow::ensure!((-100..=100).contains(&bend), "curve_bend is -100..100");
        let dx = end[0] - x;
        let dy = end[1] - y;
        let control: [f64; 2] = op
            .get("control")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?
            .unwrap_or([
                (x + end[0]) / 2. - dy * bend as f64 / 100.,
                (y + end[1]) / 2. + dx * bend as f64 / 100.,
            ]);
        anyhow::ensure!(
            control
                .iter()
                .all(|v| v.is_finite() && (-4096. ..=4096.).contains(v)),
            "control is a finite point in -4096..4096 (got {control:?})"
        );
        let mut path = op.clone();
        path["op"] = serde_json::json!("path");
        path["x"] = serde_json::json!(0);
        path["y"] = serde_json::json!(0);
        path["fill"] = serde_json::json!(false);
        path["points"] = serde_json::json!([[x, y], [control[0], control[1], end[0], end[1]]]);
        if kind == "tuft" && width > 1 && (dx != 0. || dy != 0.) {
            let mut vx = control[0] - x;
            let mut vy = control[1] - y;
            if vx == 0. && vy == 0. {
                vx = dx;
                vy = dy;
            }
            let length = vx.hypot(vy);
            let half = (width - 1) as f64 / 2.;
            let nx = -vy / length * half;
            let ny = vx / length * half;
            path["points"] = serde_json::json!([
                [x + nx, y + ny],
                [
                    control[0] + nx * 0.55,
                    control[1] + ny * 0.55,
                    end[0],
                    end[1]
                ],
                [
                    control[0] - nx * 0.55,
                    control[1] - ny * 0.55,
                    x - nx,
                    y - ny
                ]
            ]);
            path["brush_size"] = serde_json::json!(1);
            path["fill"] = serde_json::json!(true);
        }
        return rasterize(&path);
    }
    let x = integer(op, "x", 0)?;
    let y = integer(op, "y", 0)?;
    let fill = op.get("fill").and_then(Value::as_bool).unwrap_or(false);
    let mut out = BTreeSet::new();
    let mut contour = Vec::new();
    match kind {
        "path" => {
            for p in path_contour(op)? {
                let q = [
                    x + (p[0] - x as f64).round() as i32,
                    y + (p[1] - y as f64).round() as i32,
                ];
                if contour.last() != Some(&q) {
                    contour.push(q);
                }
            }
        }
        "ellipse" => {
            let w = integer(op, "width", 1)?;
            let h = integer(op, "height", 1)?;
            anyhow::ensure!(
                (1..=1024).contains(&w) && (1..=1024).contains(&h),
                "ellipse dimensions must be 1..1024"
            );
            for yy in 0..h {
                for xx in 0..w {
                    let nx = (2 * xx + 1 - w) as f64 / w as f64;
                    let ny = (2 * yy + 1 - h) as f64 / h as f64;
                    let inside = nx * nx + ny * ny <= 1.;
                    let iw = w - 2 * width;
                    let ih = h - 2 * width;
                    let interior = iw > 0
                        && ih > 0
                        && ((2 * xx + 1 - w) as f64 / iw as f64).powi(2)
                            + ((2 * yy + 1 - h) as f64 / ih as f64).powi(2)
                            <= 1.;
                    if inside && (fill || !interior) {
                        out.insert([x + xx, y + yy]);
                    }
                }
            }
            return Ok(out.into_iter().collect());
        }
        "line" => contour.extend([[x, y], [integer(op, "x2", x)?, integer(op, "y2", y)?]]),
        "pixel" => contour.push([x, y]),
        "rect" => {
            let w = integer(op, "width", 1)?;
            let h = integer(op, "height", 1)?;
            anyhow::ensure!(
                (1..=1024).contains(&w) && (1..=1024).contains(&h),
                "rectangle dimensions must be 1..1024"
            );
            contour.extend([
                [x, y],
                [x + w - 1, y],
                [x + w - 1, y + h - 1],
                [x, y + h - 1],
                [x, y],
            ]);
        }
        "text" => {
            let body = op.get("text").and_then(Value::as_str).unwrap_or_default();
            let small = op.get("face").and_then(Value::as_str) == Some("small");
            let scale = op
                .get("scale")
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .clamp(1, 8) as i32;
            let spacing = op.get("spacing").and_then(Value::as_i64).unwrap_or(1) as i32;
            crate::winamp::pixel_text::layout(body, small, scale, spacing, |dx, dy| {
                out.insert([x + dx, y + dy]);
            });
            return Ok(out.into_iter().collect());
        }
        _ => bail!("unsupported brush shape {kind}"),
    }
    if contour.len() == 1 {
        out.insert(contour[0]);
    }
    let clean = op
        .get("clean_corners")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if clean
        && width == 1
        && !fill
        && contour.first() != contour.last()
        && ["line", "path"].contains(&kind)
    {
        let mut ordered = Vec::new();
        for p in contour.windows(2) {
            walk_segment(p[0], p[1], |q| {
                if ordered.last() != Some(&q) {
                    ordered.push(q);
                }
            });
        }
        out.extend(clean_corners(&ordered));
    } else {
        for p in contour.windows(2) {
            segment(p[0], p[1], &mut out);
        }
    }
    if fill {
        polygon(&contour, &mut out);
        if let (Some(a), Some(b)) = (contour.first(), contour.last()) {
            segment(*a, *b, &mut out);
        }
    }
    if width > 1 {
        let mut thick = BTreeSet::new();
        let lo = -(width / 2);
        let hi = lo + width;
        for p in out {
            for dy in lo..hi {
                for dx in lo..hi {
                    thick.insert([p[0] + dx, p[1] + dy]);
                }
            }
        }
        out = thick;
    }
    Ok(out.into_iter().collect())
}
pub fn path_contour(op: &Value) -> Result<Vec<[f64; 2]>> {
    let x = integer(op, "x", 0)?;
    let y = integer(op, "y", 0)?;
    let mut contour = Vec::new();
    let commands = op["points"].as_array().context("path points required")?;
    anyhow::ensure!(
        !commands.is_empty() && commands.len() <= 256,
        "path needs 1..256 commands"
    );
    let mut p = [0., 0.];
    for (i, c) in commands.iter().enumerate() {
        let c = c
            .as_array()
            .context("path commands must be coordinate arrays")?;
        anyhow::ensure!(
            [2, 4, 6].contains(&c.len()) && (i > 0 || c.len() == 2),
            "path starts with [x,y]; commands have 2,4,6 coordinates"
        );
        let v: Vec<f64> = c
            .iter()
            .map(|n| {
                let n = n.as_f64().context("finite coordinate required")?;
                anyhow::ensure!(
                    n.is_finite() && (-2048.0..=2048.0).contains(&n),
                    "path coordinates must be -2048..2048"
                );
                Ok(n)
            })
            .collect::<Result<_>>()?;
        if v.len() == 2 {
            p = [v[0], v[1]];
            contour.push([x as f64 + p[0], y as f64 + p[1]]);
            continue;
        }
        let end = [v[v.len() - 2], v[v.len() - 1]];
        let length = v
            .as_chunks::<2>()
            .0
            .iter()
            .scan(p, |last, q| {
                let d = (q[0] - last[0]).hypot(q[1] - last[1]);
                *last = [q[0], q[1]];
                Some(d)
            })
            .sum::<f64>();
        let steps = (length * 2.).ceil().max(1.) as usize;
        for j in 1..=steps {
            let t = j as f64 / steps as f64;
            let u = 1. - t;
            let q = if v.len() == 4 {
                [
                    u * u * p[0] + 2. * u * t * v[0] + t * t * end[0],
                    u * u * p[1] + 2. * u * t * v[1] + t * t * end[1],
                ]
            } else {
                [
                    u * u * u * p[0]
                        + 3. * u * u * t * v[0]
                        + 3. * u * t * t * v[2]
                        + t * t * t * end[0],
                    u * u * u * p[1]
                        + 3. * u * u * t * v[1]
                        + 3. * u * t * t * v[3]
                        + t * t * t * end[1],
                ]
            };
            let q = [x as f64 + q[0], y as f64 + q[1]];
            if contour.last() != Some(&q) {
                contour.push(q);
            }
        }
        p = end;
    }
    Ok(contour)
}
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/brush/curve_tests.rs"]
mod curve_tests;
