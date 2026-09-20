//! A native-pixel paw, with small directional marks for each cursor role.
use super::*;

pub(super) fn draw(shape: Shape, palette: &Palette, bold: bool) -> (RgbaImage, [u32; 2]) {
    // Some skins choose the same light color for body and accent. Keep toe beans visible.
    let pad = if contrast(palette.body, palette.accent) < 1.8 {
        [
            ((u16::from(palette.body[0]) + u16::from(palette.ink[0])) / 2) as u8,
            ((u16::from(palette.body[1]) + u16::from(palette.ink[1])) / 2) as u8,
            ((u16::from(palette.body[2]) + u16::from(palette.ink[2])) / 2) as u8,
            255,
        ]
    } else {
        palette.accent
    };
    let rows = [
        "        oo    oo       ",
        "       obbo  obbo      ",
        "       obpbo obpbo     ",
        "  oo   obpbo obpbo oo  ",
        " obbo  obbo  obbo obbo ",
        "obpbo   oo    oo  obpbo",
        "obpbo             obpbo",
        " obbo             obbo ",
        "  oo    oo   oo    oo  ",
        "       obbooobbo       ",
        "      obbbbbbbbbo      ",
        "     obbbppppbbbbo     ",
        "    obbbppppppbbbbo    ",
        "    obbppppppppbbbo    ",
        "    obbppppppppbbbo    ",
        "     obbppppppbbbo     ",
        "      obbppppbbbo      ",
        "       obbbbbbo        ",
        "        oooooo         ",
    ];
    let mut canvas = Canvas::new();
    if bold {
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate().filter(|(_, c)| *c != ' ') {
                let _ = c;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        canvas.put(x as i32 + 4 + dx, y as i32 + 3 + dy, palette.ink);
                    }
                }
            }
        }
    }
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let color = match c {
                'o' => palette.ink,
                'b' => palette.body,
                'p' => pad,
                _ => continue,
            };
            canvas.put(x as i32 + 4, y as i32 + 3, color);
        }
    }
    let mut marks = Vec::new();
    let horizontal = matches!(shape, Shape::SlideX | Shape::Move);
    let vertical = matches!(shape, Shape::SlideY | Shape::Move);
    if horizontal {
        for step in 0..3 {
            marks.extend([
                (1 + step, 16 - step),
                (1 + step, 16 + step),
                (30 - step, 16 - step),
                (30 - step, 16 + step),
            ]);
        }
    }
    if vertical {
        for step in 0..3 {
            marks.extend([
                (16 - step, step),
                (16 + step, step),
                (16 - step, 30 - step),
                (16 + step, 30 - step),
            ]);
        }
    }
    if shape == Shape::Resize {
        for step in 0..4 {
            marks.extend([
                (1 + step, 1),
                (1, 1 + step),
                (29 - step, 29),
                (29, 29 - step),
            ]);
        }
    }
    if shape == Shape::Danger {
        for step in 0..6 {
            marks.extend([(23 + step, 24 + step), (28 - step, 24 + step)]);
        }
    }
    canvas.body(&marks, palette.accent, palette.ink, 1);
    let hotspot = if matches!(shape, Shape::Arrow | Shape::Danger) {
        [12, 3]
    } else {
        [16, 16]
    };
    (canvas.image, hotspot)
}
