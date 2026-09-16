//! Bidirectional mapping between the assembled skin and the classic atlases.
use super::model::View;
use crate::winamp::sprites::*;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Layer {
    pub id: String,
    pub sheet: String,
    pub source: [u32; 4],
    pub destination: [u32; 4],
    pub variants: Vec<[u32; 4]>,
    /// What each variant is, in the same order. Four rectangles for a switch
    /// say nothing about which is off and which is pressed, and twenty-eight
    /// say nothing about which end of the travel frame 0 is; both used to be
    /// answerable only by reading this file. Guessing wrong paints the pressed
    /// art into the released cell, and nothing reports it.
    pub labels: Vec<String>,
}
fn rect(r: SpriteRect) -> [u32; 4] {
    [r.0 as u32, r.1 as u32, r.2 as u32, r.3 as u32]
}
impl Layer {
    pub fn map(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        let [dx, dy, dw, dh] = self.destination;
        if x < dx || y < dy || x >= dx + dw || y >= dy + dh {
            return None;
        }
        Some((
            (x - dx) * self.source[2] / dw,
            (y - dy) * self.source[3] / dh,
        ))
    }
    pub fn stretched(&self) -> bool {
        self.source[2..] != self.destination[2..]
    }
}
pub fn size(panel: &str) -> (u32, u32) {
    if panel == "playlist" {
        (275, 261)
    } else if panel == "main" {
        (275, 115)
    } else {
        (275, 116)
    }
}
/// What each of a sprite's variants is.
///
/// Two rectangles are released and pressed almost everywhere, four are a switch
/// that is also off or on, and twenty-eight are a slider's travel -- but which
/// end of the travel frame 0 sits at differs per slider, and three of the
/// two-variant sprites are not pressed states at all. None of that was visible
/// from outside this file.
fn variant_labels(id: &str, sheet: &str, count: usize) -> Vec<String> {
    let ends = |low: &str, high: &str| -> Vec<String> {
        (0..count)
            .map(|i| match i {
                0 => format!("0 · {low}"),
                i if i + 1 == count => format!("{i} · {high}"),
                i => i.to_string(),
            })
            .collect()
    };
    let named =
        |names: &[&str]| -> Vec<String> { names.iter().map(|s| (*s).to_string()).collect() };
    // The playlist header is the trap. Classic pledit.bmp keeps two rows of it
    // and Cranamp draws the lower one always -- there is no unfocused playlist
    // -- so art put in the upper row is never seen by anybody.
    if sheet == "pledit" && matches!(id, "top.left" | "top.tile" | "top.right" | "title") {
        return named(&[
            "unfocused · classic only; Cranamp never draws this row",
            "focused · what the player always draws",
        ]);
    }
    if id == "title" {
        return named(&["focused", "unfocused"]);
    }
    if id.starts_with("band") && id.ends_with(".track") {
        return ends(
            "full cut · handle at the bottom",
            "full boost · handle at the top",
        );
    }
    match (id, count) {
        ("status", 3) => named(&["stopped", "playing", "paused"]),
        ("mono" | "stereo", 2) => named(&["off · not this channel mode", "on"]),
        ("volume.track", _) => ends("silent", "full volume"),
        ("balance.track", _) => ends("hard left", "hard right"),
        ("position.thumb", 2) | ("scroll.thumb", 2) => named(&["released", "dragged"]),
        (_, 4) => named(&["off", "off pressed", "on", "on pressed"]),
        (_, 10) => (0..10).map(|d| d.to_string()).collect(),
        (_, 2) => named(&["released", "pressed"]),
        (_, 1) => named(&["always"]),
        _ => ends("first", "last"),
    }
}
pub fn layers(v: &View, layout: crate::winamp::skin::SkinLayout) -> Vec<Layer> {
    let mut out = Vec::new();
    let mut add = |id: &str,
                   sheet: &str,
                   variants: Vec<SpriteRect>,
                   index: usize,
                   x: u32,
                   y: u32,
                   size: Option<(u32, u32)>| {
        let source = rect(variants[index.min(variants.len() - 1)]);
        let (w, h) = size.unwrap_or((source[2], source[3]));
        out.push(Layer {
            id: id.into(),
            sheet: format!("{sheet}.bmp"),
            source,
            destination: [x, y, w, h],
            labels: variant_labels(id, sheet, variants.len()),
            variants: variants.into_iter().map(rect).collect(),
        });
    };
    let p = usize::from(v.pressed);
    let active = usize::from(v.active);
    match v.panel.as_str() {
        "main" => {
            add(
                "background",
                "main",
                vec![(0., 0., 275., 115.)],
                0,
                0,
                0,
                None,
            );
            add(
                "title",
                "titlebar",
                vec![MAIN_TITLE_BAR_SELECTED, MAIN_TITLE_BAR],
                usize::from(!v.active),
                0,
                0,
                None,
            );
            for (id, a, b, pos) in [
                (
                    "options",
                    MAIN_OPTIONS_BUTTON,
                    MAIN_OPTIONS_BUTTON_SELECTED,
                    POS_OPTIONS_BUTTON,
                ),
                (
                    "minimize",
                    MAIN_MINIMIZE_BUTTON,
                    MAIN_MINIMIZE_BUTTON_SELECTED,
                    POS_MINIMIZE_BUTTON,
                ),
                (
                    "shade",
                    MAIN_SHADE_BUTTON,
                    MAIN_SHADE_BUTTON_SELECTED,
                    POS_SHADE_BUTTON,
                ),
                (
                    "close",
                    MAIN_CLOSE_BUTTON,
                    MAIN_CLOSE_BUTTON_SELECTED,
                    POS_CLOSE_BUTTON,
                ),
            ] {
                add(
                    id,
                    "titlebar",
                    vec![a, b],
                    p,
                    pos.0 as u32,
                    pos.1 as u32,
                    None,
                );
            }
            add(
                "status",
                "playpaus",
                vec![STATUS_STOPPED, STATUS_PLAYING, STATUS_PAUSED],
                v.playback as usize,
                26,
                28,
                None,
            );
            for (i, pos) in POS_TIME_DIGITS.iter().enumerate() {
                add(
                    &format!("digit{i}"),
                    "numbers",
                    (0..10).map(digit_rect).collect(),
                    v.digit as usize,
                    pos.0 as u32,
                    pos.1 as u32,
                    None,
                );
            }
            add(
                "mono",
                "monoster",
                vec![MONO_OFF, MONO_ON],
                active,
                212,
                41,
                None,
            );
            add(
                "stereo",
                "monoster",
                vec![STEREO_OFF, STEREO_ON],
                active,
                239,
                41,
                None,
            );
            add("position.track", "posbar", vec![POSBAR_BG], 0, 17, 72, None);
            add(
                "position.thumb",
                "posbar",
                vec![POSBAR_THUMB, POSBAR_THUMB_ACTIVE],
                p,
                17 + (219. * v.position as f32 / 27.).round() as u32,
                72,
                None,
            );
            for (i, (a, b)) in [
                (PREV_BUTTON, PREV_BUTTON_ACTIVE),
                (PLAY_BUTTON, PLAY_BUTTON_ACTIVE),
                (PAUSE_BUTTON, PAUSE_BUTTON_ACTIVE),
                (STOP_BUTTON, STOP_BUTTON_ACTIVE),
                (NEXT_BUTTON, NEXT_BUTTON_ACTIVE),
                (EJECT_BUTTON, EJECT_BUTTON_ACTIVE),
            ]
            .into_iter()
            .enumerate()
            {
                let x = if i == 5 {
                    136
                } else {
                    16 + [0, 23, 46, 69, 92][i]
                };
                add(
                    ["previous", "play", "pause", "stop", "next", "eject"][i],
                    "cbuttons",
                    vec![a, b],
                    p,
                    x,
                    if i == 5 { 89 } else { 88 },
                    None,
                );
            }
            add(
                "volume.track",
                "volume",
                (0..28).map(|f| (0., f as f32 * 15., 68., 13.)).collect(),
                v.volume as usize,
                107,
                57,
                None,
            );
            add(
                "volume.thumb",
                "volume",
                vec![VOLUME_THUMB, VOLUME_THUMB_ACTIVE],
                p,
                107 + (54. * v.volume as f32 / 27.).round() as u32,
                58,
                None,
            );
            add(
                "balance.track",
                "balance",
                (0..28).map(|f| (9., f as f32 * 15., 38., 13.)).collect(),
                v.balance as usize,
                177,
                57,
                None,
            );
            add(
                "balance.thumb",
                "balance",
                vec![BALANCE_THUMB, BALANCE_THUMB_ACTIVE],
                p,
                177 + (24. * v.balance as f32 / 27.).round() as u32,
                58,
                None,
            );
            add(
                "shuffle",
                "shufrep",
                vec![
                    SHUFFLE_OFF,
                    SHUFFLE_OFF_ACTIVE,
                    SHUFFLE_ON,
                    SHUFFLE_ON_ACTIVE,
                ],
                active * 2 + p,
                164,
                89,
                None,
            );
            add(
                "repeat",
                "shufrep",
                vec![REPEAT_OFF, REPEAT_OFF_ACTIVE, REPEAT_ON, REPEAT_ON_ACTIVE],
                active * 2 + p,
                210,
                89,
                None,
            );
            add(
                "eq.toggle",
                "shufrep",
                vec![
                    EQ_BUTTON_OFF,
                    EQ_BUTTON_OFF_ACTIVE,
                    EQ_BUTTON_ON,
                    EQ_BUTTON_ON_ACTIVE,
                ],
                active * 2 + p,
                219,
                58,
                None,
            );
            add(
                "playlist.toggle",
                "shufrep",
                vec![
                    PL_BUTTON_OFF,
                    PL_BUTTON_OFF_ACTIVE,
                    PL_BUTTON_ON,
                    PL_BUTTON_ON_ACTIVE,
                ],
                active * 2 + p,
                242,
                58,
                None,
            );
        }
        "equalizer" => {
            add("background", "eqmain", vec![EQ_WINDOW], 0, 0, 0, None);
            add(
                "title",
                "eqmain",
                vec![EQ_TITLE_BAR_SELECTED, EQ_TITLE_BAR],
                usize::from(!v.active),
                0,
                0,
                None,
            );
            add("graph", "eqmain", vec![EQ_GRAPH_BG], 0, 86, 17, None);
            add(
                "preamp.line",
                "eqmain",
                vec![EQ_PREAMP_LINE],
                0,
                86,
                26,
                None,
            );
            add(
                "close",
                "eqmain",
                vec![EQ_CLOSE_BUTTON, EQ_CLOSE_BUTTON_SELECTED],
                p,
                264,
                3,
                None,
            );
            add(
                "on",
                "eqmain",
                vec![
                    EQ_ON_BUTTON_OFF,
                    EQ_ON_BUTTON_OFF_SELECTED,
                    EQ_ON_BUTTON_ON,
                    EQ_ON_BUTTON_ON_SELECTED,
                ],
                active * 2 + p,
                14,
                18,
                None,
            );
            add(
                "auto",
                "eqmain",
                vec![
                    EQ_AUTO_BUTTON_OFF,
                    EQ_AUTO_BUTTON_OFF_SELECTED,
                    EQ_AUTO_BUTTON_ON,
                    EQ_AUTO_BUTTON_ON_SELECTED,
                ],
                active * 2 + p,
                40,
                18,
                None,
            );
            add(
                "presets",
                "eqmain",
                vec![EQ_PRESETS_BUTTON, EQ_PRESETS_BUTTON_SELECTED],
                p,
                217,
                18,
                None,
            );
            for i in 0..11 {
                let frame = v.eq[i] as usize;
                add(
                    &format!("band{i}.track"),
                    "eqmain",
                    (0..28)
                        .map(|f| {
                            (
                                13. + (f % 14) as f32 * 15.,
                                if f < 14 { 164. } else { 229. },
                                14.,
                                63.,
                            )
                        })
                        .collect(),
                    frame,
                    EQ_SLIDER_XS[i] as u32,
                    38,
                    None,
                );
                add(
                    &format!("band{i}.thumb"),
                    "eqmain",
                    vec![EQ_SLIDER_THUMB, EQ_SLIDER_THUMB_SELECTED],
                    p,
                    EQ_THUMB_XS[i] as u32,
                    38 + (layout.eq_travel as f32 * (1. - frame as f32 / 27.)).round() as u32,
                    None,
                );
            }
        }
        "playlist" => {
            add(
                "list.background",
                "plbg",
                vec![(0., 0., 243., 203.)],
                0,
                12,
                20,
                None,
            );
            add(
                "top.left",
                "pledit",
                vec![(0., 0., 25., 20.), PLAYLIST_TOP_LEFT_CORNER],
                active,
                0,
                0,
                None,
            );
            add(
                "top.tile",
                "pledit",
                vec![(127., 0., 25., 20.), PLAYLIST_TOP_TILE],
                active,
                25,
                0,
                Some((225, 20)),
            );
            add(
                "title",
                "pledit",
                vec![(26., 0., 100., 20.), PLAYLIST_TITLE_BAR],
                active,
                87,
                0,
                None,
            );
            add(
                "top.right",
                "pledit",
                vec![(153., 0., 25., 20.), PLAYLIST_TOP_RIGHT_CORNER],
                active,
                250,
                0,
                None,
            );
            add(
                "left.rail",
                "pledit",
                vec![PLAYLIST_LEFT_TILE],
                0,
                0,
                20,
                Some((12, 203)),
            );
            add(
                "right.rail",
                "pledit",
                vec![PLAYLIST_RIGHT_TILE],
                0,
                255,
                20,
                Some((20, 203)),
            );
            add(
                "bottom.left",
                "pledit",
                vec![PLAYLIST_BOTTOM_LEFT_CORNER],
                0,
                0,
                223,
                None,
            );
            add(
                "bottom.right",
                "pledit",
                vec![PLAYLIST_BOTTOM_RIGHT_CORNER],
                0,
                125,
                223,
                None,
            );
            add(
                "scroll.thumb",
                "pledit",
                vec![PLAYLIST_SCROLL_HANDLE, PLAYLIST_SCROLL_HANDLE_SELECTED],
                p,
                260,
                20 + (185. * v.scroll as f32 / 27.).round() as u32,
                None,
            );
        }
        _ => {}
    }
    out
}

/// Match the native playlist renderer's repetition and cropping, never stretch
/// its source pixels. Repeated footprints retain their logical layer ID so a
/// shared tile can be selected once in the human/MCP layer picker.
pub fn native_panel_layers(layers: Vec<Layer>, panel: &str, height: u32, scroll: u8) -> Vec<Layer> {
    if panel != "playlist" {
        return layers;
    }
    let mut out = Vec::new();
    let interior = height - 58;
    for mut l in layers {
        match l.id.as_str() {
            "list.background" | "left.rail" | "right.rail" => l.destination[3] = interior,
            "bottom.left" | "bottom.right" => l.destination[1] = height - 38,
            "scroll.thumb" => {
                l.destination[1] =
                    20 + (interior.saturating_sub(18) as f32 * scroll as f32 / 27.).round() as u32
            }
            _ => {}
        }
        let [dx, dy, dw, dh] = l.destination;
        let sw = l.source[2];
        let sh = l.source[3];
        if l.id == "top.tile" || l.id == "list.background" || l.id.ends_with(".rail") {
            for y in (0..dh).step_by(sh as usize) {
                for x in (0..dw).step_by(sw as usize) {
                    let mut tile = l.clone();
                    let w = sw.min(dw - x);
                    let h = sh.min(dh - y);
                    tile.destination = [dx + x, dy + y, w, h];
                    tile.source[2] = w;
                    tile.source[3] = h;
                    for v in &mut tile.variants {
                        v[2] = w;
                        v[3] = h;
                    }
                    out.push(tile);
                }
            }
        } else {
            out.push(l);
        }
    }
    out
}
