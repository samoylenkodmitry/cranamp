//! Conservative, self-contained classic WSZ export profile shared by Studio and CI.
//! Passing this audit is format validation, not evidence of testing another player.
use super::*;

#[derive(Clone, Debug, serde::Serialize)]
pub struct ClassicAudit {
    pub profile: &'static str,
    pub exportable: bool,
    pub errors: Vec<Divergence>,
    pub warnings: Vec<String>,
    pub magenta_pixels: usize,
}

pub fn audit_classic_archive(bytes: &[u8]) -> Result<ClassicAudit> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut names = std::collections::BTreeSet::new();
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut magenta_pixels = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_dir() {
            continue;
        }
        let name = normalize_name(file.name());
        if !names.insert(name.clone()) {
            errors.push(Divergence {
                entry: name.clone(),
                problem:
                    "Duplicate case-insensitive skin filename; players may select different copies"
                        .into(),
                fix: "Keep exactly one copy of each sheet or palette".into(),
            });
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let mut size = None;
        if name.ends_with(".bmp") {
            if !data.starts_with(b"BM") {
                errors.push(Divergence {
                    entry: name.clone(),
                    problem: "The .bmp entry is not a Windows BMP".into(),
                    fix: "Export as an opaque Windows BMP, preferably 24-bit RGB".into(),
                });
            }
            match image::load_from_memory(&data) {
                Ok(image) => {
                    size = Some((image.width(), image.height()));
                    let rgba = image.to_rgba8();
                    magenta_pixels += rgba.pixels().filter(|p| is_sprite_key(p.0)).count();
                    if rgba.pixels().any(|p| p[3] != 255) {
                        errors.push(Divergence { entry: name.clone(), problem: "BMP contains alpha that classic players do not interpret consistently".into(), fix: "Composite paint layers onto an opaque background and export 24-bit RGB".into() });
                    }
                }
                Err(error) => errors.push(Divergence {
                    entry: name.clone(),
                    problem: format!("Unreadable bitmap: {error}"),
                    fix: "Replace the damaged bitmap".into(),
                }),
            }
        }
        if name == "region.txt" {
            match regions::Regions::parse(&data) {
                Ok(region) if region.portable() => {}
                Ok(_) => errors.push(Divergence {
                    entry: name.clone(),
                    problem: "Audacious interprets non-rectangular region polygons differently"
                        .into(),
                    fix: "Regenerate this region from its pixel mask as merged rectangles".into(),
                }),
                Err(error) => errors.push(Divergence {
                    entry: name.clone(),
                    problem: format!("Invalid window regions: {error}"),
                    fix: "Generate REGION.TXT from a Studio window mask".into(),
                }),
            }
        }
        if matches!(name.as_str(), "pledit.txt" | "viscolor.txt") && !complete_palette(&name, &data)
        {
            errors.push(Divergence {
                entry: name.clone(),
                problem: "Incomplete or ambiguous classic palette".into(),
                fix: "Write all six [Text] colors or exactly 24 RGB visualizer rows explicitly"
                    .into(),
            });
        }
        entries.push((name, size));
    }
    errors.extend(divergences(&entries));
    if entries
        .iter()
        .any(|(name, size)| name == "eqmain.bmp" && size.is_some_and(|(w, h)| w >= 275 && h == 163))
    {
        warnings.push("Artwork-only EQ: the 163-row bitmap intentionally omits slider tracks, handles, presets graphics and the graph. Controls remain hit-testable but have no visible position feedback. Compare this legacy cropped-bitmap technique in your target players.".into());
    }
    for palette in ["pledit.txt", "viscolor.txt"] {
        if !names.contains(palette) {
            errors.push(Divergence {
                entry: palette.into(),
                problem: "Missing palette would use player-specific defaults".into(),
                fix: "Supply an explicit classic palette".into(),
            });
        }
    }
    if let Some((_, Some((w, h)))) = entries.iter().find(|(name, _)| name == "nums_ex.bmp") {
        if *w < 108 || *h < 13 {
            errors.push(Divergence {
                entry: "nums_ex.bmp".into(),
                problem: "Extended digits need twelve 9x13 cells, including blank and minus".into(),
                fix: "Use at least 108x13 pixels".into(),
            });
        }
    }
    if magenta_pixels > 0 {
        warnings.push(format!("{magenta_pixels} literal magenta pixels. #ff00ff is opaque ink, never a sprite hole. Review projects created by older Cranamp versions."));
    }
    errors.sort_by(|a, b| a.entry.cmp(&b.entry).then(a.problem.cmp(&b.problem)));
    errors.dedup();
    Ok(ClassicAudit {
        profile: "classic-winamp-v1",
        exportable: errors.is_empty(),
        errors,
        warnings,
        magenta_pixels,
    })
}
fn complete_palette(name: &str, bytes: &[u8]) -> bool {
    let text = decode_text(bytes);
    let lines = text
        .lines()
        .map(|line| {
            line.split("//")
                .next()
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
        })
        .filter(|line| !line.is_empty());
    if name == "viscolor.txt" {
        let rows: Vec<_> = lines.collect();
        return rows.len() == 24
            && rows.iter().all(|row| {
                let channels: Vec<_> = row.split(',').map(str::trim).collect();
                channels.len() == 3 && channels.iter().all(|channel| channel.parse::<u8>().is_ok())
            });
    }
    let mut section = false;
    let mut found = std::collections::BTreeSet::new();
    let required = [
        "normal",
        "current",
        "normalbg",
        "selectedbg",
        "mbfg",
        "mbbg",
    ];
    for line in lines {
        if line.starts_with('[') {
            section = line.eq_ignore_ascii_case("[Text]");
            continue;
        }
        if !section {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let key = key.trim().to_ascii_lowercase();
        if required.contains(&key.as_str()) {
            let value = value.trim();
            if value.len() != 7
                || !value.starts_with('#')
                || !value[1..].bytes().all(|b| b.is_ascii_hexdigit())
                || !found.insert(key)
            {
                return false;
            }
        }
    }
    found.len() == required.len()
}
#[cfg(test)]
#[path = "../../../test/unit/winamp/skin/compatibility_tests.rs"]
mod tests;
