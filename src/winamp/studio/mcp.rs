use super::{model::Document, SharedDocument};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    io::{BufRead, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::Duration,
};
pub const ADDRESS: &str = "127.0.0.1:18765";
static ACTIVE: std::sync::OnceLock<std::sync::Mutex<SharedDocument>> = std::sync::OnceLock::new();
pub fn start(shared: SharedDocument) -> Result<()> {
    if let Some(active) = ACTIVE.get() {
        *active.lock().unwrap() = shared;
        return Ok(());
    }
    let listener =
        TcpListener::bind(ADDRESS).context("Skin Studio MCP port 18765 is already in use")?;
    let _ = ACTIVE.set(std::sync::Mutex::new(shared.clone()));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let shared = ACTIVE.get().unwrap().lock().unwrap().clone();
            let _ = serve(&mut stream, &shared);
        }
    });
    Ok(())
}
fn serve(stream: &mut TcpStream, shared: &SharedDocument) -> Result<()> {
    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let valid = line.starts_with("POST /mcp HTTP/1.");
    let mut length = 0;
    let mut origin = false;
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            length = v.trim().parse()?;
        }
        if lower.starts_with("origin:") {
            origin = true;
        }
    }
    if !valid || origin || length > 8 * 1024 * 1024 {
        stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let request: Value = serde_json::from_slice(&body)?;
    if request.get("id").is_none() {
        stream.write_all(
            b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )?;
        return Ok(());
    }
    let response = dispatch(request, shared);
    let bytes = serde_json::to_vec(&response)?;
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len())?;
    stream.write_all(&bytes)?;
    Ok(())
}
pub fn dispatch(request: Value, shared: &SharedDocument) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(
            json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"cranamp-skin-studio","version":env!("CARGO_PKG_VERSION")}}),
        ),
        "notifications/initialized" | "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools":tools()})),
        "tools/call" => {
            let p = &request["params"];
            let name = p["name"].as_str().unwrap_or("");
            let args = p.get("arguments").cloned().unwrap_or(json!({}));
            call(name, args, shared)
        }
        _ => Err(anyhow::anyhow!("Unknown method {method}")),
    };
    match result {
        Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
        Err(e) => {
            json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":format!("{e:#}")}]}})
        }
    }
}
fn panel_canvas(panel: &str, crop: Option<Value>, playlist_height: u32) -> Result<[u32; 4]> {
    let window: [u32; 4] = match panel {
        "main" => [0, 0, 275, 116],
        "equalizer" => [0, 116, 275, 116],
        "playlist" => [0, 232, 275, playlist_height],
        "all" => [0, 0, 275, 232 + playlist_height],
        other => bail!("panel is main, equalizer, playlist or all, not {other}"),
    };
    let Some(inner) = crop else {
        return Ok(window);
    };
    let r: [u32; 4] = serde_json::from_value(inner).context("crop must be [x,y,width,height]")?;
    anyhow::ensure!(
        r[2] > 0 && r[3] > 0 && r[0] + r[2] <= window[2] && r[1] + r[3] <= window[3],
        "crop {r:?} must fit inside the {panel} panel, which is {}x{} native",
        window[2],
        window[3]
    );
    Ok([window[0] + r[0], window[1] + r[1], r[2], r[3]])
}
fn needles(args: &Value, key: &str) -> Result<Option<Vec<String>>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(one)) => Ok(Some(vec![one.to_lowercase()])),
        Some(Value::Array(many)) => Ok(Some(
            many.iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_lowercase)
                        .with_context(|| format!("{key} is a string or an array of strings"))
                })
                .collect::<Result<_>>()?,
        )),
        Some(other) => bail!("{key} is a string or an array of strings (got {other})"),
    }
}
fn matches(needles: &Option<Vec<String>>, value: &str) -> bool {
    match needles {
        None => true,
        Some(list) => {
            let lower = value.to_lowercase();
            list.iter().any(|n| lower.contains(n.as_str()))
        }
    }
}
fn overlaps(box_: [u32; 4], rect: &[u32; 4]) -> bool {
    box_[0] < rect[0] + rect[2]
        && rect[0] < box_[0] + box_[2]
        && box_[1] < rect[1] + rect[3]
        && rect[1] < box_[1] + box_[3]
}
fn enlargement(args: &Value, native: (u32, u32), fallback: u32) -> Result<u32> {
    const MOST: u32 = 2048;
    let Some(asked) = args.get("magnify") else {
        return Ok(fallback);
    };
    let m = asked
        .as_u64()
        .with_context(|| format!("magnify is a whole number 1..64 (got {asked})"))?
        as u32;
    anyhow::ensure!((1..=64).contains(&m), "magnify is 1..64 (got {m})");
    let (w, h) = (native.0.max(1), native.1.max(1));
    if w * m > MOST || h * m > MOST {
        let fits = (MOST / w).min(MOST / h).max(1);
        bail!(
            "magnify {m} would return {}x{}, over the {MOST} pixel limit; \
             {w}x{h} fits {fits} times -- crop first to go closer",
            w * m,
            h * m
        );
    }
    Ok(m)
}
fn empty_catalogue(
    surface: Option<String>,
    kept: Option<&Vec<Value>>,
    total: usize,
    sheet: &Option<Vec<String>>,
    needle: &Option<Vec<String>>,
) -> Option<String> {
    if kept.is_some_and(|k| !k.is_empty()) || (sheet.is_none() && needle.is_none()) {
        return None;
    }
    let listed = |what: &Option<Vec<String>>| {
        what.as_ref()
            .map(|v| v.join("\" or \""))
            .unwrap_or_default()
    };
    let asked = match (sheet.is_some(), needle.is_some()) {
        (true, true) => format!(
            "sheet \"{}\" and id containing \"{}\"",
            listed(sheet),
            listed(needle)
        ),
        (true, false) => format!("sheet \"{}\"", listed(sheet)),
        (false, true) => format!("id containing \"{}\"", listed(needle)),
        (false, false) => unreachable!(),
    };
    let where_from = match surface {
        Some(s) if s != "canvas" => {
            format!("{s} -- studio_targets answers for the whole skin whatever is open")
        }
        _ => "the whole skin".to_string(),
    };
    Some(format!(
        "Nothing matched {asked} among the {total} on {where_from}."
    ))
}
fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
}
fn operation_schema() -> Value {
    json!({"type":"array","items":{"type":"object","required":["op"],"properties":{
        "op":{"enum":["pixel","line","rect","ellipse","path","curve","tuft","stamp","cluster","text","image"],
              "description":"tuft is a curve that tapers to a point. cluster drops the studio_cluster clipboard."},
        "x":{"type":["integer","array"],"description":"Native pixels on the surface the view is on, never zoomed. Write it as [from, to] for a sweep: the operation lands once per state the transaction writes, and the number steps from the first to the last. A slider is one shape whose end moves, so all its frames take one call. These sweep: x, y, x2, y2, width, height, brush_size, curve_bend, bevel, refraction, opacity, grain, grain_size, grain_seed, spacing, scale, and control as two points."},
        "y":{"type":["integer","array"]},"x2":{"type":["integer","array"]},"y2":{"type":["integer","array"]},
        "width":{"type":["integer","array"]},"height":{"type":["integer","array"]},"fill":{"type":"boolean"},
        "brush_size":{"type":["integer","array"],"minimum":1,"maximum":32},
        "curve_bend":{"type":["integer","array"],"minimum":-100,"maximum":100,"description":"How far a curve bows. control overrides it."},
        "control":{"type":"array","items":{},"minItems":2,"maxItems":2,
                   "description":"The quadratic control point, in absolute coordinates. It may be fractional. Two points rather than two numbers sweep it."},
        "align":{"enum":["left","center","right"],"description":"Put the word in a box width wide that starts at x, rather than at x itself. A centred caption is (cell - ink) / 2."},
        "points":{"type":"array","items":{"type":"array","items":{"type":"number"}},
                  "description":"Relative to x/y, first point is the start: [x,y] line, [cx,cy,x,y] quadratic, [c1x,c1y,c2x,c2y,x,y] cubic."},
        "color":{"type":"string","description":"#rrggbb; #ff00ff erases."},
        "ramp":{"type":"array","items":{"type":"string"},"description":"Exact palette colours along ramp_axis. A short ramp gives bands, not a gradient."},
        "ramp_axis":{"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4},
        "rows":{"type":"array","items":{"type":"string"},"description":"One character per pixel. A character the palette lacks is skipped, which is how you spell transparency."},
        "palette":{"type":"object","additionalProperties":{"type":"string"}},
        "text":{"type":"string","description":"The word to set, in one of the two faces. The pen moves (cell+spacing)*scale per character, where cell is 5 or 4. studio_canvas measure gives the width first. It has A-Z a-z 0-9 - : . , / \\ \" ( ) [ ] + = _ ! ? & # % * < > | only. unsupported_characters names the rest."},
        "face":{"enum":["5x7","small"],"description":"small is a 4x5 small-caps face for the cells a classic skin gives a word and no room for one -- a 14-pixel equalizer caption, a 27-pixel mono lamp. It has no lower case, so a-z are drawn as capitals rather than skipped."},
        "scale":{"type":"integer","minimum":1,"maximum":8},
        "spacing":{"type":"integer","minimum":-2,"maximum":8},
        "data":{"type":"string","description":"Base64 PNG, at most 2048x2048, one source pixel per skin pixel. Alpha 0 stays put. Part alpha blends with the surface as it stands and lands opaque. Good to add light to art that is there, bad for a redraw."},
        "material":{"enum":["glass"],"description":"Lay glass into a filled shape, tinted with the brush colour."},
        "bevel":{"type":"number","minimum":1,"maximum":128},
        "refraction":{"type":"number","minimum":0,"maximum":32},
        "mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},
        "clean_corners":{"type":"boolean","description":"Drop spare elbows from an open 1px curve or path. This is not antialiasing."},
        "grain":{"type":"number","minimum":0,"maximum":64,"description":"Fixed noise on this shape's colours, so a large panel reads as paper, not plastic. grain_size is the lattice it clumps on, default 2. grain_seed varies it. The same numbers always give the same pixels."},
        "grain_size":{"type":"integer","minimum":1,"maximum":16},
        "grain_seed":{"type":"integer"},
        "opacity":{"type":"integer","minimum":1,"maximum":255,"description":"Lay this shape over what is there at this strength. A sheet keeps no alpha, so the blend lands opaque. Good to add light to art that exists, bad for a redraw."}
    }}})
}
fn tools() -> Vec<Value> {
    vec![
        tool("studio_canvas", "The whole skin as one canvas. Main, equalizer and playlist sit at their own positions. This is the only surface you draw on. Reads it back as a PNG at a whole-number zoom, cropped to [x,y,width,height] if you ask. With path it writes a file and says where. It also sets what the canvas shows: zoom, brush, colour, width, the state each sprite is in, and which panel is open.", json!({"zoom":{"type":"integer","minimum":1,"maximum":8,"description":"The canvas zoom in the editor, and the image enlargement when magnify is absent."},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"path":{"type":"string"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"measure":{"type":["string","array"],"items":{"type":"string"},"description":"How wide these words come out in the current face and scale, instead of a canvas read. width and height are the ink. advance is where the pen ends. The engine walks the glyphs, so you need no copy of the sums."},"spacing":{"type":"integer","minimum":-2,"maximum":8,"description":"Letter spacing for this measure only. text_spacing is the one the brush and the operations use."},"color":{"type":"string"},"brush":{"enum":["pencil","line","rect","ellipse","lift","stamp","glass","curve","tuft","text"]},"ramp_to":{"type":["string","null"],"description":"A gradient from the brush colour to this one, along the shape the gesture drew. null turns it off."},"ramp_axis":{"enum":["down","across"]},"bevel":{"type":"integer","minimum":0,"maximum":128,"description":"The glass lens bevel. 0 takes it from the height of the drag."},"refraction":{"type":"integer","minimum":0,"maximum":32},"text":{"type":"string","description":"What the text brush writes."},"face":{"enum":["5x7","small"]},"text_scale":{"type":"integer","minimum":1,"maximum":8},"drawer":{"enum":["none","tools","layers","targets","rectangles","atlases","history","study","options","picker","files","states"],"description":"Which panel is open. It sits on the document, so a caller with no pointer opens one. A name this layout does not have opens nothing."},"text_spacing":{"type":"integer","minimum":-2,"maximum":8,"description":"Letter spacing for the text brush and for any text operation with none of its own. It decides whether a word fits: STEREO in the small face is 29 pixels of ink at spacing 1, and the stereo lamp is 29 pixels wide."},"brush_size":{"type":"integer","minimum":1,"maximum":32},"curve_bend":{"type":"integer","minimum":-100,"maximum":100},"grain":{"type":"integer","minimum":0,"maximum":64},"grain_size":{"type":"integer","minimum":1,"maximum":16},"opacity":{"type":"integer","minimum":1,"maximum":255},"clean_corners":{"type":"boolean"},"filled":{"type":"boolean"},"mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},"grid":{"type":"boolean"},"guides":{"type":"boolean"},"alpha_lock":{"type":"boolean"},"mask_colors":{"type":"array","items":{"type":"string"}},"states":{"enum":["current","onward","up-to","all"],"description":"Which states of each target this writes into: the one the canvas shows, that one and every later one, every earlier one, or all of them. A slider has 28 frames that differ by one object, so a run from the frame in hand to an end fits its art. all_states true is the old name for all."},"stamp_repeat":{"type":"integer","minimum":1,"maximum":64,"description":"How many copies a Stamp drag drops, evenly along the drag. It is the pointer's half of studio_draw at."},"stamp_sweep":{"type":"boolean","description":"A Stamp drag drops one copy per state of the edit scope, a step at a time along the drag. It is the pointer's half of a swept [from, to]."},"all_states":{"type":"boolean","description":"The old name for states."},"clip":{"type":["array","null"],"items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"pressed":{"type":"boolean","description":"Which state the canvas shows, and so which state a stroke lands in. A four-state switch is one call run four ways with active and pressed."},"active":{"type":"boolean","description":"Focused titles, and on or off for the channel lamps, shuffle, repeat and the equalizer switches. It picks the state a stroke lands in, as pressed does."},"volume":{"type":"integer","minimum":0,"maximum":27},"balance":{"type":"integer","minimum":0,"maximum":27},"position":{"type":"integer","minimum":0,"maximum":27},"scroll":{"type":"integer","minimum":0,"maximum":27},"eq":{"type":"array","items":{"type":"integer","minimum":0,"maximum":27},"minItems":11,"maxItems":11},"digit":{"type":"integer","minimum":0,"maximum":9},"playback":{"type":"integer","minimum":0,"maximum":2},"presentation":{"type":"boolean"},"preview_playlist_height":{"type":"integer","minimum":145,"maximum":522}}), &[]),
        tool("studio_atlas", "One BMP on its own, at its own coordinates. It shares the brush and the history with the canvas. path, crop and zoom read it back the way studio_canvas reads the skin. This is the only way to see a sheet the canvas never shows, such as numbers.bmp or text.bmp. Leave out sheet to go back to the whole skin.", json!({"sheet":{"type":"string"},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"path":{"type":"string"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4}}), &[]),
        tool("studio_targets", "The sprites a stroke goes into. It lists the whole skin whatever surface is open. Leave out everything for the full list. Narrow it with sheet or id, which take one substring or a list. Each sprite says its sheet, its cell, how many states it has, and whether anything is drawn in it yet.", json!({"layers":{"type":"array","items":{"type":"string"}},"solo":{"type":"string"},"auto":{"type":"boolean"},"paint_layer":{"type":["string","null"]},"sheet":{"type":["string","array"],"items":{"type":"string"}},"id":{"type":["string","array"],"items":{"type":"string"},"description":"One substring, or several. Case does not matter. Six transport keys take one call."},"variants":{"type":"boolean"}}), &[]),
        tool("studio_rectangles", "Where each sprite state sits on the canvas, plus the two kinds of rectangle that have no sprite. Narrow it with at, sheet, id, runtime for the readouts the player writes, or hit for controls the player hit-tests but never draws. gaps lists the parts of a sheet no cell uses.", json!({"select":{"type":"string"},"sheet":{"type":["string","array"],"items":{"type":"string"}},"id":{"type":["string","array"],"items":{"type":"string"},"description":"One substring, or several. Case does not matter."},"at":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4,"description":"Everything that overlaps this [x,y,width,height] box on the surface in hand: what a new band would cross. A plain list says where each one is, not what sits next to what."},"runtime":{"type":"boolean"},"gaps":{"type":"boolean","description":"The parts of one sheet no sprite reads, as rectangles, so you can keep ink out of them. pledit.bmp column 125 falls between the two footer flaps. Takes sheet, or uses the open one."},"hit":{"type":"boolean"}}), &[]),
        tool("studio_layers", "Paint planes above the original sheets. list, add, select, set, move, merge_down and delete. A plane has a name, an opacity, and switches for shown, locked and clipped to the plane below.", json!({"action":{"enum":["list","add","select","set","move","merge_down","delete"]},"id":{"type":"string"},"name":{"type":"string"},"visible":{"type":"boolean"},"locked":{"type":"boolean"},"clip_below":{"type":"boolean"},"opacity":{"type":"integer","minimum":0,"maximum":255},"index":{"type":"integer","minimum":0}}), &[]),
        tool("studio_options", "The skin apart from its sheets: the six PLEDIT.TXT colours and the 24 VISCOLOR.TXT colours. It answers readability: the contrast of every colour the player writes against the art behind it. Leave everything out to read them all.", json!({"playlist_colors":{"type":"object","additionalProperties":{"type":"string"}},"visualizer_colors":{"type":"array","items":{"type":"string"},"minItems":24,"maxItems":24}}), &[]),
        tool("studio_validate", "Whether this skin looks the same in every player that reads .wsz. It names each entry no player reads, each sheet the format needs and the skin lacks, and each sheet too small for its own sprites. An empty list means the skin is portable. fix repairs what it can: it drops the entries no player reads and grows the sheets that are too small.", json!({"fix":{"type":"boolean","description":"Repair instead of report. A grown sheet repeats its own edge into the new rows, so it looks the way it looked before."}}), &[]),
        tool("studio_cursors", "The pointers a skin shows over its own regions: the 18 .cur files a classic player reads, from the main window's plain arrow to the playlist's resize corner. Leave everything out to list which regions are drawn. draw makes the missing ones in the skin's own colours and opens them on the canvas, where every brush works on them as on any sheet. regions narrows any action to the ones you name. overwrite redraws regions that already have a cursor. remove drops them. hotspot moves the point a region's pointer actually points at.", json!({"action":{"enum":["list","draw","remove","hotspot"]},"regions":{"type":"array","items":{"type":"string"},"description":"Region names such as normal, titlebar, posbar, volbar, eqslid, psize, with or without .cur. Leave it out for every region."},"overwrite":{"type":"boolean","description":"Redraw a region that already has a cursor instead of leaving the artwork alone."},"style":{"enum":super::cursor_art::Style::ALL.iter().map(|style|style.name()).collect::<Vec<_>>(),"description":"The silhouette the whole set is cut to. paw and paw-bold draw cat paws with functional direction marks. Left out, the established arrow styles are chosen from the skin palette."},"hotspot":{"type":"array","items":{"type":"integer","minimum":0},"minItems":2,"maxItems":2,"description":"Where in the 32x32 image the pointer actually points. An arrow points at its own tip, a slider at its middle."}}), &[]),
        tool("studio_status", "The shared document: path, revision, unsaved edits, history depth, sheets, paint planes and the whole view. surface says which surface a stroke lands on.", json!({}), &[]),
        tool("studio_coverage", "Inspect exact pixel ownership before treating a region as unavailable. Separates paintable pixels, stateful/repeated sprites, paintable backgrounds beneath runtime footprints, and palette-only playlist fill. Runtime footprints are NOT exclusion masks. Read-only; rows adds a character map, image or path adds a color map.", json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"rows":{"type":"boolean"},"image":{"type":"boolean"},"path":{"type":"string"}}), &["rect"]),
        tool("studio_new", "Make an empty classic skin. Nothing is kept from the old one. Set discard to true if there are unsaved edits.", json!({"discard":{"type":"boolean"}}), &[]),
        tool("studio_open", "Load a WSZ into the open Studio. Set discard to true if there are unsaved edits.", json!({"path":{"type":"string"},"discard":{"type":"boolean"}}), &["path"]),
        tool("studio_draw", "One undoable transaction, up to 10000 operations, all or nothing, on the surface the view is on. The answer names that surface, and a refusal names the operation that failed. It reports bounds, clipped_pixels for ink outside the chosen sprites, unsampled_pixels for ink in a part of a sheet nothing draws, unmapped_pixels for ink with no sheet under it, overwrites when two canvas pixels write one shared cell, keyed_blends when a blend read the transparency key as a colour, and covered_pixels for ink another part of the same control hides in every state.", json!({"operations":operation_schema(),"layers":{"type":"array","items":{"type":"string"},"description":"The sprites every pixel goes into. [] means Auto: every sprite under the brush."},"layer":{"type":"string"},"states":{"enum":["current","onward","up-to","all"],"description":"Which states of each target this writes into: the one the canvas shows, that one and every later one, every earlier one, or all of them. A slider has 28 frames that differ by one object, so a run from the frame in hand to an end fits its art. all_states true is the old name for all."},"at":{"type":["array","string"],"description":"Run the whole operation list once at each [x, y] offset, or once at each chosen target's own position with \"targets\". A sheet is a grid of cells that mostly hold the same drawing, so this fills one in a single call. At most 256 places."},"all_states":{"type":"boolean","description":"The old name for states. true is all, false is current."},"origin":{"type":"string","description":"Put 0,0 on this sprite's own position and target it. This is the safe way to aim at a sprite that moves with its frame."},"label":{"type":"string"},"mask_colors":{"type":"array","items":{"type":"string"}},"preview":{"type":"boolean","description":"A dry run. It applies the operations, answers with the surface they would leave, and puts the document back. It records nothing and the revision holds. crop, zoom and path work as on studio_canvas."},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"path":{"type":"string"}}), &["operations"]),
        tool("studio_project", "Save or open a layered .cstudio project. WSZ stays the flat export. Set discard to true if there are unsaved edits.", json!({"action":{"enum":["save","open"]},"path":{"type":"string"},"discard":{"type":"boolean"}}), &["action", "path"]),
        tool("studio_cluster", "Pick up pixels from the chosen sprites. In Auto it takes what the canvas shows. Leave out rect to read the clipboard back as stamp rows and a palette. flip_x, flip_y and quarter_turns turn what it holds.", json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"flip_x":{"type":"boolean"},"flip_y":{"type":"boolean"},"quarter_turns":{"type":"integer","minimum":0,"maximum":3}}), &[]),
        tool("studio_study", "Inspect an edit and its boundary in one call: native pixels, enlargement, four-pixel context by default, and exact ownership/runtime coverage. states adds a labelled 2x2 board of active/inactive and released/pressed editor composites without changing the view or history. Check the GPU separately with studio_screenshot.", json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"padding":{"type":"integer","minimum":0,"maximum":32,"default":4,"description":"Exterior pixels around rect, clipped to the surface. Use zero only for an exact isolated crop."},"states":{"type":"boolean","description":"Show all four active/pressed combinations in a labelled 2x2 editor board, read-only."},"selected":{"type":"boolean"},"values":{"type":"boolean"},"grid":{"type":"boolean"},"geometry":{"type":"boolean"},"reference":{"type":"string"},"reference_rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"path":{"type":"string"}}), &[]),
        tool("studio_pixel", "What sits under one pixel: each sprite there, where each one keeps it, its colour, and what shares that cell. With width and height it answers ground, the average colour under the box. With ink it answers contrast and whether that ink reads. Always includes exact paintability, runtime footprints and opaque spectrum coverage. Use studio_coverage for a visual map; never treat broad readout rectangles as exclusion masks.", json!({"palette":{"type":"boolean","description":"Count exact RGBA colours in the rendered crop. Returns up to 32 colours, sorted by frequency, plus the total number of unique colours. Useful for keeping pixel-art ramps deliberate."},"x":{"type":"integer","minimum":0},"y":{"type":"integer","minimum":0},"width":{"type":"integer","minimum":1},"height":{"type":"integer","minimum":1},"ink":{"type":"string","description":"#rrggbb. How well does this colour read on what is there?"}}), &["x", "y"]),
        tool("studio_states", "Every state of one sprite, as a contact sheet and as numbers. Each state says its index, its label, its cell, how many pixels are painted, how far it differs from the one before, and which other state it matches. Set image to false for the numbers alone.", json!({"id":{"type":"string","description":"The sprite to show: an exact id or a substring, case does not matter. It looks in the whole skin whatever surface is open. Leave it out for the sprite the view has."},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"differences":{"type":"boolean"},"image":{"type":"boolean","description":"false answers with the numbers alone. Whether 28 frames came out as 28 different pictures is a question the numbers answer."},"path":{"type":"string"}}), &[]),
        tool("studio_history", "The shared history. It lists what each step did, and cursor moves the document to a step.", json!({"cursor":{"type":"integer","minimum":0,"description":"Move the document to this entry. It undoes or redoes as far as it must."}}), &[]),
        tool("studio_undo", "Undo one step of the shared history.", json!({}), &[]),
        tool("studio_redo", "Redo one step of the shared history.", json!({}), &[]),
        tool("studio_screenshot", "The player's own scene, as the GPU draws it, once the revision you ask for is on screen. panel crops to one window in that window's coordinates. crop takes scene pixels, or that panel's own skin pixels when panel is given too. presentation puts the window into the live stack first.", json!({"path":{"type":"string"},"panel":{"enum":["main","equalizer","playlist","all"]},"presentation":{"type":"boolean"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"Enlarge the capture, nearest neighbour, up to 2048 pixels a side. One transport key is 23x18 of a 1280x1000 scene."}}), &[]),
        tool("studio_export", "Write a WSZ. It runs through the player's own skin loader first and writes the file in one step. It reports undrawn_sprites for sprites still empty and hard_to_read for a readout under 3:1. It never refuses for either. Left without a path it writes back over the file the skin was opened from, so a run of open, edit, export cannot put one skin's artwork under another skin's name.", json!({"path":{"type":"string","description":"Where to write. Left out, the skin is written back where it came from."}}), &["path"]),
    ]
}
fn text(value: Value) -> Value {
    json!({"content":[{"type":"text","text":value.to_string()}]})
}
fn only_the_pencil(args: &Value) -> bool {
    const PENCIL: &[&str] = &[
        "color",
        "brush",
        "brush_size",
        "ramp_to",
        "ramp_axis",
        "bevel",
        "refraction",
        "text",
        "face",
        "text_scale",
        "text_spacing",
        "drawer",
        "curve_bend",
        "grain",
        "grain_size",
        "opacity",
        "clean_corners",
        "filled",
        "mirror_x",
        "mirror_y",
        "alpha_lock",
        "mask_colors",
        "measure",
        "spacing",
        "clip",
        "states",
        "stamp_repeat",
        "stamp_sweep",
        "all_states",
    ];
    match args.as_object() {
        Some(fields) => !fields.is_empty() && fields.keys().all(|k| PENCIL.contains(&k.as_str())),
        None => false,
    }
}
fn only_a_look(args: &Value) -> bool {
    const LOOK: &[&str] = &["path", "crop", "zoom", "magnify"];
    match args.as_object() {
        Some(fields) => !fields.is_empty() && fields.keys().all(|k| LOOK.contains(&k.as_str())),
        None => false,
    }
}
fn reject_unknown_arguments(name: &str, args: &Value) -> Result<()> {
    let Some(object) = args.as_object() else {
        return Ok(());
    };
    let Some(schema) = tools()
        .into_iter()
        .find(|t| t["name"] == name)
        .and_then(|t| t["inputSchema"]["properties"].as_object().cloned())
    else {
        return Ok(());
    };
    let unknown: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|k| !schema.contains_key(*k))
        .collect();
    if unknown.is_empty() {
        return Ok(());
    }
    let mut known: Vec<&str> = schema.keys().map(String::as_str).collect();
    known.sort_unstable();
    let elsewhere: Vec<String> = unknown
        .iter()
        .filter_map(|field| {
            tools()
                .into_iter()
                .find(|t| t["name"] != name && t["inputSchema"]["properties"].get(field).is_some())
                .map(|t| format!("{field} is {}'s", t["name"].as_str().unwrap_or_default()))
        })
        .collect();
    bail!(
        "{name} has no {}{}. It takes: {}",
        unknown.join(", "),
        if elsewhere.is_empty() {
            String::new()
        } else {
            format!(" -- {}", elsewhere.join("; "))
        },
        known.join(", ")
    );
}
pub fn call(name: &str, args: Value, shared: &SharedDocument) -> Result<Value> {
    reject_unknown_arguments(name, &args)?;
    if name == "studio_screenshot" {
        let (revision, playlist_height) = {
            let mut doc = shared
                .lock()
                .map_err(|_| anyhow::anyhow!("Studio document lock"))?;
            if args["presentation"] == json!(true) && !doc.view.presentation {
                doc.state(json!({"presentation": true}))?;
            }
            (doc.revision, doc.view.preview_playlist_height)
        };
        let mut im = super::capture_scene(revision)?;
        let mut crop = args.get("crop").cloned();
        if let Some(panel) = args["panel"].as_str() {
            let canvas = panel_canvas(panel, crop.take(), playlist_height)?;
            let rect = super::player_scene_rect(canvas).context(
                "The window is showing the editor's canvas, not the player, so there is no \
                 panel to crop to. Add presentation:true, or turn on Player preview.",
            )?;
            crop = Some(json!(rect));
        }
        if let Some(crop) = crop {
            let r: [u32; 4] =
                serde_json::from_value(crop.clone()).context("crop must be [x,y,width,height]")?;
            if r[2] == 0
                || r[3] == 0
                || r[0].checked_add(r[2]).is_none_or(|v| v > im.width())
                || r[1].checked_add(r[3]).is_none_or(|v| v > im.height())
            {
                bail!(
                    "Crop {r:?} must fit inside the captured scene, which is {}x{}",
                    im.width(),
                    im.height()
                );
            }
            im = image::imageops::crop_imm(&im, r[0], r[1], r[2], r[3]).to_image();
        }
        let magnify = enlargement(&args, im.dimensions(), 1)?;
        if magnify > 1 {
            im = image::imageops::resize(
                &im,
                im.width() * magnify,
                im.height() * magnify,
                image::imageops::FilterType::Nearest,
            );
        }
        let (showing, player) = match super::player_scene() {
            Some([x, y, zoom]) => (
                "player",
                json!({"x":x.round() as i64,"y":y.round() as i64,"zoom":zoom}),
            ),
            None => ("editor", Value::Null),
        };
        let mut bytes = std::io::Cursor::new(Vec::new());
        im.write_to(&mut bytes, image::ImageFormat::Png)?;
        if let Some(path) = args["path"].as_str() {
            std::fs::write(path, bytes.get_ref())?;
            return Ok(text(
                json!({"path":wrote(path),"size":[im.width(),im.height()],"revision":revision,
                       "showing":showing,"player":player}),
            ));
        }
        return Ok(
            json!({"content":[{"type":"image","mimeType":"image/png","data":base64(bytes.get_ref())},{"type":"text","text":format!("Cranamp GPU scene, showing the {showing}: {}×{} pixels; player {player}",im.width(),im.height())}]}),
        );
    }
    let mut doc = shared
        .lock()
        .map_err(|_| anyhow::anyhow!("Studio document lock"))?;
    let value = match name {
        "studio_status" => doc.status(),
        "studio_new" => {
            if doc.dirty && args["discard"] != true {
                bail!("There are unsaved edits. studio_new {{\"discard\":true}} throws them away; studio_export writes them first");
            }
            let revision = doc.revision + 1;
            *doc = Document::blank();
            doc.revision = revision;
            doc.open_on_whole_skin();
            doc.status()
        }
        "studio_state" => doc.state(args)?,
        "studio_open" => {
            if doc.dirty && args["discard"] != true {
                bail!("There are unsaved edits. studio_open {{\"path\":..., \"discard\":true}} throws them away; studio_export writes them first");
            }
            let p = args["path"].as_str().context("path required")?;
            let revision = doc.revision + 1;
            *doc = if p.ends_with(".cstudio") {
                Document::open_project(&std::fs::read(p)?)?
            } else {
                Document::open(&std::fs::read(p)?, Some(p.into()))?
            };
            doc.revision = revision;
            doc.open_on_whole_skin();
            doc.status()
        }
        "studio_inspect_region" => {
            doc.inspect_region(serde_json::from_value(args["rect"].clone())?)?
        }
        "studio_guides" | "studio_rectangles" => {
            if let Some(id) = args["select"].as_str() {
                doc.select_guide(id)?
            } else if args["gaps"] == json!(true) {
                let sheet = args["sheet"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| doc.view.sheet.clone());
                doc.sheet_gaps(&sheet)?
            } else {
                let sheet = needles(&args, "sheet")?;
                let needle = needles(&args, "id")?;
                let runtime = args["runtime"].as_bool();
                let hit = args["hit"].as_bool();
                let at: Option<[u32; 4]> = args
                    .get("at")
                    .map(|v| {
                        serde_json::from_value(v.clone())
                            .context("at is [x, y, width, height] on the surface in hand")
                    })
                    .transpose()?;
                let all = doc.guides();
                let total = all.len();
                let kept = all
                    .into_iter()
                    .filter(|g| {
                        matches(&sheet, &g.sheet)
                            && matches(&needle, &g.id)
                            && runtime.is_none_or(|r| g.runtime == r)
                            && hit.is_none_or(|r| g.hit == r)
                            && at.is_none_or(|box_| overlaps(box_, &g.rect))
                    })
                    .collect::<Vec<_>>();
                let mut answer = json!({"rectangles":kept,"of":total});
                if let Some(note) = empty_catalogue(
                    Some(doc.surface()),
                    answer["rectangles"].as_array(),
                    total,
                    &sheet,
                    &needle,
                ) {
                    answer["note"] = json!(note);
                }
                answer
            }
        }
        "studio_patch" => doc.patch(&args)?,
        "studio_paint_layers" | "studio_layers" => doc.paint_layer_command(&args, "MCP")?,
        "studio_project" => {
            let p = Path::new(args["path"].as_str().context("Project path required")?);
            if args["action"] == "save" {
                doc.save_project(p)?
            } else if args["action"] == "open" {
                anyhow::ensure!(
                    !doc.dirty || args["discard"] == true,
                    "There are unsaved edits. studio_project {{\"action\":\"open\", \
                     \"path\":..., \"discard\":true}} throws them away; \
                     studio_project {{\"action\":\"save\"}} writes them first"
                );
                let revision = doc.revision + 1;
                *doc = Document::open_project(&std::fs::read(p)?)?;
                doc.revision = revision;
                doc.open_on_whole_skin();
                doc.status()
            } else {
                bail!("Unknown project action")
            }
        }
        "studio_cluster" => {
            let turns = args["quarter_turns"].as_u64().unwrap_or(0);
            anyhow::ensure!(turns < 4, "quarter_turns is 0..3");
            if let Some(rect) = args.get("rect") {
                doc.capture_cluster(serde_json::from_value(rect.clone())?)?;
            }
            doc.transform_cluster(args["flip_x"] == true, args["flip_y"] == true, turns as u32)?
        }
        "studio_draw" => {
            let report = doc.draw(&args)?;
            let looked = ["path", "crop", "zoom", "magnify"]
                .iter()
                .any(|k| args.get(*k).is_some());
            if doc.preview.is_none() && looked {
                doc.preview = Some(doc.render());
            }
            if let Some(image) = doc.preview.take() {
                let zoom = args["zoom"].as_u64().unwrap_or(1).clamp(1, 8) as u32;
                let mut image = image;
                if let Some(crop) = args.get("crop") {
                    let r: [u32; 4] = serde_json::from_value(crop.clone())
                        .context("crop must be [x,y,width,height]")?;
                    let (x, y) = (r[0].min(image.width()), r[1].min(image.height()));
                    let w = r[2].min(image.width() - x).max(1);
                    let h = r[3].min(image.height() - y).max(1);
                    image = image::imageops::crop_imm(&image, x, y, w, h).to_image();
                }
                let zoom = enlargement(&args, image.dimensions(), zoom)?;
                let image = image::imageops::resize(
                    &image,
                    image.width() * zoom,
                    image.height() * zoom,
                    image::imageops::FilterType::Nearest,
                );
                let mut bytes = std::io::Cursor::new(Vec::new());
                image.write_to(&mut bytes, image::ImageFormat::Png)?;
                if let Some(path) = args["path"].as_str() {
                    std::fs::write(path, bytes.get_ref())?;
                    let mut report = report;
                    report["path"] = json!(wrote(path));
                    report["size"] = json!([image.width(), image.height()]);
                    return Ok(text(report));
                }
                return Ok(
                    json!({"content":[{"type":"image","mimeType":"image/png","data":base64(bytes.get_ref())},{"type":"text","text":report.to_string()}]}),
                );
            }
            report
        }
        "studio_coverage" => {
            let rect = serde_json::from_value(args["rect"].clone())
                .context("rect must be [x,y,width,height]")?;
            let (mut report, image) = doc.coverage(rect, args["rows"] == true)?;
            if let Some(path) = args["path"].as_str() {
                image.save(path)?;
                report["path"] = json!(wrote(path));
            } else if args["image"] == true {
                let mut bytes = std::io::Cursor::new(Vec::new());
                image.write_to(&mut bytes, image::ImageFormat::Png)?;
                return Ok(json!({"content":[
                    {"type":"text","text":report.to_string()},
                    {"type":"image","mimeType":"image/png","data":base64(bytes.get_ref())}
                ]}));
            }
            report
        }
        "studio_pixel" => {
            let whole = |key: &str, default: u64, what: &str| -> Result<u32> {
                match args.get(key) {
                    None | Some(Value::Null) => Ok(default as u32),
                    Some(v) => {
                        let n = v.as_u64().with_context(|| {
                            format!("{key} is a whole number of {what} (got {v})")
                        })?;
                        anyhow::ensure!(n >= 1, "{key} is at least 1 (got {n})");
                        Ok(n as u32)
                    }
                }
            };
            let rect = [
                args["x"].as_u64().context("x required")? as u32,
                args["y"].as_u64().context("y required")? as u32,
                whole("width", 1, "pixels")?,
                whole("height", 1, "pixels")?,
            ];
            let ink = match args.get("ink").and_then(Value::as_str) {
                Some(colour) => Some(super::model::parse_color(colour)?),
                None => None,
            };
            let mut report = doc.inspect(rect, ink);
            if let Ok((coverage, _)) = doc.coverage(rect, false) {
                report["paintability"] = coverage;
            }
            if args["palette"] == true {
                report["palette"] = super::study::palette(&doc.render(), rect)?;
            }
            report
        }
        "studio_visualizer_palette" => doc.visualizer_palette(&args)?,
        "studio_canvas" | "studio_atlas" => {
            let mut view = args.clone();
            if name == "studio_atlas" {
                match args.get("sheet").and_then(Value::as_str) {
                    Some(sheet) => {
                        view["panel"] = json!("atlas");
                        view["sheet"] = json!(sheet);
                        view["layer"] = json!("sheet");
                    }
                    None if only_a_look(&args) || only_the_pencil(&args) => {}
                    None => {
                        view["panel"] = json!("canvas");
                        view["layer"] = json!("auto");
                    }
                }
            } else if !only_the_pencil(&args) {
                view["panel"] = json!("canvas");
            }
            let path = view.as_object_mut().and_then(|v| v.remove("path"));
            let crop = view.as_object_mut().and_then(|v| v.remove("crop"));
            let magnify = view.as_object_mut().and_then(|v| v.remove("magnify"));
            let measure = view.as_object_mut().and_then(|v| v.remove("measure"));
            let asked_spacing = view
                .as_object_mut()
                .and_then(|v| v.remove("spacing"))
                .and_then(|v| v.as_i64());
            let _ = &magnify;
            let zoom_out = view.as_object_mut().and_then(|v| v.remove("zoom"));
            if let Some(zoom) = zoom_out.clone() {
                view["zoom"] = zoom;
            }
            let state = doc.state(view)?;
            if let Some(measure) = measure {
                let words: Vec<String> = match &measure {
                    Value::String(one) => vec![one.clone()],
                    Value::Array(many) => many
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(str::to_owned)
                                .context("measure is a string or an array of strings")
                        })
                        .collect::<Result<_>>()?,
                    other => bail!("measure is a string or an array of strings (got {other})"),
                };
                anyhow::ensure!(words.len() <= 256, "at most 256 words per measure");
                let small = doc.view.face == "small";
                let scale = doc.view.text_scale as i32;
                let spacing = asked_spacing.unwrap_or(doc.view.text_spacing as i64) as i32;
                let mut missing: Vec<char> = Vec::new();
                let measured: Vec<Value> = words
                    .iter()
                    .map(|word| {
                        let e = crate::winamp::pixel_text::measure(word, small, scale, spacing);
                        for ch in e.missing {
                            if !missing.contains(&ch) {
                                missing.push(ch);
                            }
                        }
                        json!({"text":word,"width":e.width,"height":e.height,
                               "advance":e.advance})
                    })
                    .collect();
                let mut out = json!({"measured":measured,"face":doc.view.face,
                                     "scale":scale,"spacing":spacing});
                if !missing.is_empty() {
                    out["unsupported_characters"] =
                        json!(missing.iter().map(|c| c.to_string()).collect::<Vec<_>>());
                }
                return Ok(text(out));
            }
            match path.and_then(|p| p.as_str().map(str::to_owned)) {
                Some(path) => {
                    let zoom = zoom_out.and_then(|z| z.as_u64()).unwrap_or(1) as u32;
                    let mut im = doc.render();
                    if let Some(crop) = crop.as_ref().and_then(Value::as_array) {
                        let r: Vec<u32> = crop
                            .iter()
                            .filter_map(|v| v.as_u64().map(|n| n as u32))
                            .collect();
                        anyhow::ensure!(r.len() == 4, "crop is [x, y, width, height]");
                        let (x, y) = (r[0].min(im.width()), r[1].min(im.height()));
                        let w = r[2].min(im.width() - x).max(1);
                        let h = r[3].min(im.height() - y).max(1);
                        im = image::imageops::crop_imm(&im, x, y, w, h).to_image();
                    }
                    let zoom = enlargement(&args, im.dimensions(), zoom)?;
                    let im = image::imageops::resize(
                        &im,
                        im.width() * zoom,
                        im.height() * zoom,
                        image::imageops::FilterType::Nearest,
                    );
                    im.save(&path)?;
                    json!({"view":state,"path":wrote(&path),"size":[im.width(),im.height()]})
                }
                None => state,
            }
        }
        "studio_targets" => {
            if args.get("auto") == Some(&json!(true)) {
                doc.state(json!({"layers":[],"layer":"auto"}))?
            } else if let Some(solo) = args.get("solo").and_then(Value::as_str) {
                if doc.view.panel == "atlas" && !doc.layers().iter().any(|l| l.id == solo) {
                    bail!(
                        "sprite targets are the joined canvas, and {} is open: \
                         studio_atlas {{}} to go back to it, or set clip to keep \
                         a stroke inside one cell of this sheet. studio_states \
                         takes an id and needs neither.",
                        doc.view.sheet
                    );
                }
                doc.state(json!({ "layers": [solo] }))?
            } else if args.get("layers").is_some() || args.get("paint_layer").is_some() {
                doc.state(args.clone())?
            } else {
                let sheet = needles(&args, "sheet")?;
                let needle = needles(&args, "id")?;
                let variants = args["variants"] == json!(true);
                let blank = doc.undrawn_sprites();
                let all = doc.skin_layers();
                let total = all.len();
                let sprites = all
                    .into_iter()
                    .filter(|l| matches(&sheet, &l.sheet) && matches(&needle, &l.id))
                    .map(|l| {
                        let mut value = json!({
                            "id": l.id,
                            "sheet": l.sheet,
                            "source": l.source,
                            "states": l.variants.len(),
                            "drawn": !blank.contains(&l.id),
                        });
                        if variants {
                            value["variants"] = json!(l.variants);
                            value["labels"] = json!(l.labels);
                        }
                        value
                    })
                    .collect::<Vec<_>>();
                let mut answer = json!({"sprites":sprites,"of":total,"chosen":doc.view.layers});
                if let Some(note) =
                    empty_catalogue(None, answer["sprites"].as_array(), total, &sheet, &needle)
                {
                    answer["note"] = json!(note);
                }
                answer
            }
        }
        "studio_options" => {
            if let Some(colors) = args.get("playlist_colors") {
                doc.set_palette(colors)?;
            }
            if let Some(colors) = args.get("visualizer_colors") {
                doc.visualizer_palette(&json!({ "colors": colors }))?;
            }
            let readability = doc.readability();
            let (playlist, visualizer) = doc.text_palettes();
            json!({
                "playlist_colors": playlist
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), json!(v)))
                    .collect::<serde_json::Map<String, Value>>(),
                "visualizer_colors": visualizer,
                "visualizer_slots": VISUALIZER_SLOTS,
                "readability": readability,
            })
        }
        "studio_cursors" => {
            let regions: Vec<String> = match args.get("regions") {
                Some(value) => serde_json::from_value(value.clone())
                    .context("regions must be an array of region names")?,
                None => Vec::new(),
            };
            match args["action"].as_str().unwrap_or("list") {
                "draw" => {
                    let drawn = doc.draw_cursors(
                        &regions,
                        args["style"].as_str(),
                        args["overwrite"].as_bool() == Some(true),
                        "MCP",
                    )?;
                    doc.state(json!({"panel": "cursors", "layer": "auto"}))?;
                    drawn
                }
                "remove" => doc.remove_cursors(&regions, "MCP")?,
                "hotspot" => {
                    let region = regions
                        .first()
                        .context("hotspot needs one region, as regions: [\"posbar\"]")?;
                    let at: [u32; 2] = serde_json::from_value(args["hotspot"].clone())
                        .context("hotspot must be [x, y]")?;
                    doc.set_cursor_hotspot(region, at[0], at[1], "MCP")?
                }
                _ => doc.cursors_report(),
            }
        }
        "studio_validate" => {
            if args.get("fix").and_then(Value::as_bool) == Some(true) {
                doc.make_portable("MCP")?
            } else {
                let divergences = doc.divergences();
                json!({
                    "plays_the_same_elsewhere": divergences.is_empty(),
                    "divergences": divergences,
                })
            }
        }
        "studio_palette" => doc.palette(),
        "studio_set_palette" => doc.set_palette(&args)?,
        "studio_recolor" => doc.recolor(&args)?,
        "studio_history" => match args.get("cursor") {
            Some(cursor) => doc.history_goto(
                cursor
                    .as_u64()
                    .with_context(|| format!("cursor is a whole number of steps (got {cursor})"))?
                    as usize,
            )?,
            None => doc.history(),
        },
        "studio_history_goto" => {
            doc.history_goto(args["cursor"].as_u64().context("cursor required")? as usize)?
        }
        "studio_undo" => json!({"changed":doc.undo(),"revision":doc.revision}),
        "studio_redo" => json!({"changed":doc.redo(),"revision":doc.revision}),
        "studio_export" => {
            let path = match args["path"].as_str() {
                Some(path) => path.to_string(),
                None => doc.path.clone().context(
                    "This skin has never been written, so studio_export needs a path to write it to",
                )?,
            };
            doc.export(Path::new(&path))?
        }
        "studio_render" | "studio_states" | "studio_study" => {
            let zoom = if name == "studio_study" {
                1
            } else {
                args["zoom"].as_u64().unwrap_or(1) as u32
            };
            if !(1..=8).contains(&zoom) {
                bail!("integer zoom 1..8");
            }
            if name == "studio_states" && args["image"] == json!(false) {
                return Ok(text(json!({
                    "sprite": doc.variant_differences(args["id"].as_str())?,
                    "revision": doc.revision,
                })));
            }
            let im = if name == "studio_study" {
                super::study::board(&doc, &args)?
            } else if name == "studio_states" {
                doc.state_sheet(args["id"].as_str())?
            } else {
                doc.render()
            };
            let zoom = enlargement(&args, im.dimensions(), zoom)?;
            let im = image::imageops::resize(
                &im,
                im.width() * zoom,
                im.height() * zoom,
                image::imageops::FilterType::Nearest,
            );
            let mut bytes = std::io::Cursor::new(Vec::new());
            im.write_to(&mut bytes, image::ImageFormat::Png)?;
            let differences = (name == "studio_states" && args["differences"] != json!(false))
                .then(|| doc.variant_differences(args["id"].as_str()))
                .transpose()?;
            let study = (name == "studio_study")
                .then(|| super::study::report(&doc, &args))
                .transpose()?;
            if let Some(p) = args["path"].as_str() {
                std::fs::write(p, bytes.get_ref())?;
                let mut out = json!({"path":wrote(p),"size":[im.width(),im.height()],"native":[im.width()/zoom,im.height()/zoom],"zoom":zoom,"revision":doc.revision});
                if let Some(d) = differences {
                    out["sprite"] = d;
                }
                if let Some(study) = study {
                    out["study"] = study;
                }
                return Ok(text(out));
            }
            let mut note = format!(
                "{}x{} native panel, {}x integer zoom; revision {}",
                im.width() / zoom,
                im.height() / zoom,
                zoom,
                doc.revision
            );
            if let Some(d) = differences {
                note.push('\n');
                note.push_str(&d.to_string());
            }
            if let Some(study) = study {
                note =
                    json!({"study":study,"size":[im.width(),im.height()],"revision":doc.revision})
                        .to_string();
            }
            return Ok(
                json!({"content":[{"type":"image","mimeType":"image/png","data":base64(bytes.get_ref())},{"type":"text","text":note}]}),
            );
        }
        _ => bail!("Unknown Studio tool {name}"),
    };
    Ok(text(value))
}
pub(super) const VISUALIZER_SLOTS: [&str; 24] = [
    "background",
    "grid dots",
    "analyzer 1 (top of the bar)",
    "analyzer 2",
    "analyzer 3",
    "analyzer 4",
    "analyzer 5",
    "analyzer 6",
    "analyzer 7",
    "analyzer 8",
    "analyzer 9",
    "analyzer 10",
    "analyzer 11",
    "analyzer 12",
    "analyzer 13",
    "analyzer 14",
    "analyzer 15",
    "analyzer 16 (foot of the bar)",
    "oscilloscope 1",
    "oscilloscope 2",
    "oscilloscope 3",
    "oscilloscope 4",
    "oscilloscope 5",
    "analyzer peak dot",
];
fn wrote(path: &str) -> String {
    std::path::Path::new(path)
        .canonicalize()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.to_string())
}
fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16)
            | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
            | c.get(2).copied().unwrap_or(0) as u32;
        s.push(A[((n >> 18) & 63) as usize] as char);
        s.push(A[((n >> 12) & 63) as usize] as char);
        s.push(if c.len() > 1 {
            A[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            A[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    s
}
pub fn bridge() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = (|| -> Result<String> {
            let mut s =
                TcpStream::connect(ADDRESS).context("Launch cranamp --skin-studio first")?;
            write!(s, "POST /mcp HTTP/1.1\r\nHost: {ADDRESS}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{line}", line.len())?;
            let mut body = String::new();
            s.read_to_string(&mut body)?;
            Ok(body
                .split_once("\r\n\r\n")
                .context("HTTP response")?
                .1
                .to_string())
        })();
        match response {
            Ok(r) if !r.is_empty() => println!("{r}"),
            Ok(_) => {}
            Err(e) => eprintln!("{e:#}"),
        }
    }
}
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/mcp/drawing_tests.rs"]
mod drawing_tests;
#[cfg(test)]
#[path = "../../../test/unit/winamp/studio/mcp/tests.rs"]
mod tests;
