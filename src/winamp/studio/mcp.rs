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
    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len())?;
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
 tool("studio_canvas","The whole skin as one canvas. Main, equalizer and playlist sit at their own positions. This is the only surface you draw on. Reads it back as a PNG at a whole-number zoom, cropped to [x,y,width,height] if you ask. With path it writes a file and says where. It also sets what the canvas shows: zoom, brush, colour, width, the state each sprite is in, and which panel is open.",json!({"zoom":{"type":"integer","minimum":1,"maximum":8,"description":"The canvas zoom in the editor, and the image enlargement when magnify is absent."},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"path":{"type":"string"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"measure":{"type":["string","array"],"items":{"type":"string"},"description":"How wide these words come out in the current face and scale, instead of a canvas read. width and height are the ink. advance is where the pen ends. The engine walks the glyphs, so you need no copy of the sums."},"spacing":{"type":"integer","minimum":-2,"maximum":8,"description":"Letter spacing for this measure only. text_spacing is the one the brush and the operations use."},"color":{"type":"string"},"brush":{"enum":["pencil","line","rect","ellipse","lift","stamp","glass","curve","tuft","text"]},"ramp_to":{"type":["string","null"],"description":"A gradient from the brush colour to this one, along the shape the gesture drew. null turns it off."},"ramp_axis":{"enum":["down","across"]},"bevel":{"type":"integer","minimum":0,"maximum":128,"description":"The glass lens bevel. 0 takes it from the height of the drag."},"refraction":{"type":"integer","minimum":0,"maximum":32},"text":{"type":"string","description":"What the text brush writes."},"face":{"enum":["5x7","small"]},"text_scale":{"type":"integer","minimum":1,"maximum":8},"drawer":{"enum":["none","tools","layers","targets","rectangles","atlases","history","study","options","picker","files","states"],"description":"Which panel is open. It sits on the document, so a caller with no pointer opens one. A name this layout does not have opens nothing."},"text_spacing":{"type":"integer","minimum":-2,"maximum":8,"description":"Letter spacing for the text brush and for any text operation with none of its own. It decides whether a word fits: STEREO in the small face is 29 pixels of ink at spacing 1, and the stereo lamp is 29 pixels wide."},"brush_size":{"type":"integer","minimum":1,"maximum":32},"curve_bend":{"type":"integer","minimum":-100,"maximum":100},"grain":{"type":"integer","minimum":0,"maximum":64},"grain_size":{"type":"integer","minimum":1,"maximum":16},"opacity":{"type":"integer","minimum":1,"maximum":255},"clean_corners":{"type":"boolean"},"filled":{"type":"boolean"},"mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},"grid":{"type":"boolean"},"guides":{"type":"boolean"},"alpha_lock":{"type":"boolean"},"mask_colors":{"type":"array","items":{"type":"string"}},"states":{"enum":["current","onward","up-to","all"],"description":"Which states of each target this writes into: the one the canvas shows, that one and every later one, every earlier one, or all of them. A slider has 28 frames that differ by one object, so a run from the frame in hand to an end fits its art. all_states true is the old name for all."},"stamp_repeat":{"type":"integer","minimum":1,"maximum":64,"description":"How many copies a Stamp drag drops, evenly along the drag. It is the pointer's half of studio_draw at."},"stamp_sweep":{"type":"boolean","description":"A Stamp drag drops one copy per state of the edit scope, a step at a time along the drag. It is the pointer's half of a swept [from, to]."},"all_states":{"type":"boolean","description":"The old name for states."},"clip":{"type":["array","null"],"items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"pressed":{"type":"boolean","description":"Which state the canvas shows, and so which state a stroke lands in. A four-state switch is one call run four ways with active and pressed."},"active":{"type":"boolean","description":"Focused titles, and on or off for the channel lamps, shuffle, repeat and the equalizer switches. It picks the state a stroke lands in, as pressed does."},"volume":{"type":"integer","minimum":0,"maximum":27},"balance":{"type":"integer","minimum":0,"maximum":27},"position":{"type":"integer","minimum":0,"maximum":27},"scroll":{"type":"integer","minimum":0,"maximum":27},"eq":{"type":"array","items":{"type":"integer","minimum":0,"maximum":27},"minItems":11,"maxItems":11},"digit":{"type":"integer","minimum":0,"maximum":9},"playback":{"type":"integer","minimum":0,"maximum":2},"presentation":{"type":"boolean"},"preview_playlist_height":{"type":"integer","minimum":145,"maximum":522}}),&[]),
 tool("studio_atlas","One BMP on its own, at its own coordinates. It shares the brush and the history with the canvas. path, crop and zoom read it back the way studio_canvas reads the skin. This is the only way to see a sheet the canvas never shows, such as numbers.bmp or text.bmp. Leave out sheet to go back to the whole skin.",json!({"sheet":{"type":"string"},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"path":{"type":"string"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4}}),&[]),
 tool("studio_targets","The sprites a stroke goes into. It lists the whole skin whatever surface is open. Leave out everything for the full list. Narrow it with sheet or id, which take one substring or a list. Each sprite says its sheet, its cell, how many states it has, and whether anything is drawn in it yet.",json!({"layers":{"type":"array","items":{"type":"string"}},"solo":{"type":"string"},"auto":{"type":"boolean"},"paint_layer":{"type":["string","null"]},"sheet":{"type":["string","array"],"items":{"type":"string"}},"id":{"type":["string","array"],"items":{"type":"string"},"description":"One substring, or several. Case does not matter. Six transport keys take one call."},"variants":{"type":"boolean"}}),&[]),
 tool("studio_rectangles","Where each sprite state sits on the canvas, plus the two kinds of rectangle that have no sprite. Narrow it with at, sheet, id, runtime for the readouts the player writes, or hit for controls the player hit-tests but never draws. gaps lists the parts of a sheet no cell uses.",json!({"select":{"type":"string"},"sheet":{"type":["string","array"],"items":{"type":"string"}},"id":{"type":["string","array"],"items":{"type":"string"},"description":"One substring, or several. Case does not matter."},"at":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4,"description":"Everything that overlaps this [x,y,width,height] box on the surface in hand: what a new band would cross. A plain list says where each one is, not what sits next to what."},"runtime":{"type":"boolean"},"gaps":{"type":"boolean","description":"The parts of one sheet no sprite reads, as rectangles, so you can keep ink out of them. pledit.bmp column 125 falls between the two footer flaps. Takes sheet, or uses the open one."},"hit":{"type":"boolean"}}),&[]),
 tool("studio_layers","Paint planes above the original sheets. list, add, select, set, move, merge_down and delete. A plane has a name, an opacity, and switches for shown, locked and clipped to the plane below.",json!({"action":{"enum":["list","add","select","set","move","merge_down","delete"]},"id":{"type":"string"},"name":{"type":"string"},"visible":{"type":"boolean"},"locked":{"type":"boolean"},"clip_below":{"type":"boolean"},"opacity":{"type":"integer","minimum":0,"maximum":255},"index":{"type":"integer","minimum":0}}),&[]),
 tool("studio_options","Everything about the skin that is not painted into a sheet. Three of them add or remove a sheet: playlist_background, playlist_selection and eq_handles. The answer names any sheet that appears or goes. It also holds the time readout, the equalizer travel, the six PLEDIT.TXT colours and the 24 VISCOLOR.TXT colours. It answers readability: the contrast of every colour the player writes against the art behind it. Leave everything out to read them all.",json!({"footer":{"enum":["classic","time-total"]},"eq_travel":{"type":"integer","minimum":1,"maximum":52},"visualizer_glass":{"type":"boolean","description":"Unlit visualizer pixels stay clear. Off, the player fills the whole spectrum rectangle with the first VISCOLOR colour. No sheet holds that box and no canvas render shows it, so only the live player reveals it."},"playlist_background":{"type":"boolean"},"eq_handles":{"type":"boolean"},"playlist_selection":{"type":"boolean"},"playlist_colors":{"type":"object","additionalProperties":{"type":"string"}},"visualizer_colors":{"type":"array","items":{"type":"string"},"minItems":24,"maxItems":24}}),&[]),
 tool("studio_status","The shared document: path, revision, unsaved edits, history depth, sheets, paint planes and the whole view. surface says which surface a stroke lands on.",json!({}),&[]),
 tool("studio_new","Make an empty classic skin. Nothing is kept from the old one. Set discard to true if there are unsaved edits.",json!({"discard":{"type":"boolean"}}),&[]),
 tool("studio_open","Load a WSZ into the open Studio. Set discard to true if there are unsaved edits.",json!({"path":{"type":"string"},"discard":{"type":"boolean"}}),&["path"]),
 tool("studio_draw","One undoable transaction, up to 10000 operations, all or nothing, on the surface the view is on. The answer names that surface, and a refusal names the operation that failed. It reports bounds, clipped_pixels for ink outside the chosen sprites, unsampled_pixels for ink in a part of a sheet nothing draws, unmapped_pixels for ink with no sheet under it, overwrites when two canvas pixels write one shared cell, keyed_blends when a blend read the transparency key as a colour, and covered_pixels for ink another part of the same control hides in every state.",json!({"operations":operation_schema(),"layers":{"type":"array","items":{"type":"string"},"description":"The sprites every pixel goes into. [] means Auto: every sprite under the brush."},"layer":{"type":"string"},"states":{"enum":["current","onward","up-to","all"],"description":"Which states of each target this writes into: the one the canvas shows, that one and every later one, every earlier one, or all of them. A slider has 28 frames that differ by one object, so a run from the frame in hand to an end fits its art. all_states true is the old name for all."},"at":{"type":["array","string"],"description":"Run the whole operation list once at each [x, y] offset, or once at each chosen target's own position with \"targets\". A sheet is a grid of cells that mostly hold the same drawing, so this fills one in a single call. At most 256 places."},"all_states":{"type":"boolean","description":"The old name for states. true is all, false is current."},"origin":{"type":"string","description":"Put 0,0 on this sprite's own position and target it. This is the safe way to aim at a sprite that moves with its frame."},"label":{"type":"string"},"mask_colors":{"type":"array","items":{"type":"string"}},"preview":{"type":"boolean","description":"A dry run. It applies the operations, answers with the surface they would leave, and puts the document back. It records nothing and the revision holds. crop, zoom and path work as on studio_canvas."},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"path":{"type":"string"}}),&["operations"]),
 tool("studio_project","Save or open a layered .cstudio project. WSZ stays the flat export. Set discard to true if there are unsaved edits.",json!({"action":{"enum":["save","open"]},"path":{"type":"string"},"discard":{"type":"boolean"}}),&["action","path"]),
 tool("studio_cluster","Pick up pixels from the chosen sprites. In Auto it takes what the canvas shows. Leave out rect to read the clipboard back as stamp rows and a palette. flip_x, flip_y and quarter_turns turn what it holds.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"flip_x":{"type":"boolean"},"flip_y":{"type":"boolean"},"quarter_turns":{"type":"integer","minimum":0,"maximum":3}}),&[]),
 tool("studio_study","A read-only board: the crop at native size above a whole-number enlargement. values shows grey levels, geometry marks sprite outlines, grid adds a display grid, and reference puts another image beside it. It changes nothing.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"selected":{"type":"boolean"},"values":{"type":"boolean"},"grid":{"type":"boolean"},"geometry":{"type":"boolean"},"reference":{"type":"string"},"reference_rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"path":{"type":"string"}}),&[]),
 tool("studio_pixel","What sits under one pixel: each sprite there, where each one keeps it, its colour, and what shares that cell. With width and height it answers ground, the average colour under the box. With ink it answers contrast and whether that ink reads.",json!({"x":{"type":"integer","minimum":0},"y":{"type":"integer","minimum":0},"width":{"type":"integer","minimum":1},"height":{"type":"integer","minimum":1},"ink":{"type":"string","description":"#rrggbb. How well does this colour read on what is there?"}}),&["x","y"]),
 tool("studio_states","Every state of one sprite, as a contact sheet and as numbers. Each state says its index, its label, its cell, how many pixels are painted, how far it differs from the one before, and which other state it matches. Set image to false for the numbers alone.",json!({"id":{"type":"string","description":"The sprite to show: an exact id or a substring, case does not matter. It looks in the whole skin whatever surface is open. Leave it out for the sprite the view has."},"zoom":{"type":"integer","minimum":1,"maximum":8},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"How much to enlarge the image, nearest neighbour, instead of zoom. A 14x25 handle wants more than 8x. The limit is 2048 pixels a side, so crop first for a closer look."},"differences":{"type":"boolean"},"image":{"type":"boolean","description":"false answers with the numbers alone. Whether 28 frames came out as 28 different pictures is a question the numbers answer."},"path":{"type":"string"}}),&[]),
 tool("studio_history","The shared history. It lists what each step did, and cursor moves the document to a step.",json!({"cursor":{"type":"integer","minimum":0,"description":"Move the document to this entry. It undoes or redoes as far as it must."}}),&[]),
 tool("studio_undo","Undo one step of the shared history.",json!({}),&[]),
 tool("studio_redo","Redo one step of the shared history.",json!({}),&[]),
 tool("studio_screenshot","The player's own scene, as the GPU draws it, once the revision you ask for is on screen. panel crops to one window in that window's coordinates. crop takes scene pixels, or that panel's own skin pixels when panel is given too. presentation puts the window into the live stack first.",json!({"path":{"type":"string"},"panel":{"enum":["main","equalizer","playlist","all"]},"presentation":{"type":"boolean"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"magnify":{"type":"integer","minimum":1,"maximum":64,"description":"Enlarge the capture, nearest neighbour, up to 2048 pixels a side. One transport key is 23x18 of a 1280x1000 scene."}}),&[]),
 tool("studio_export","Write a WSZ. It runs through the player's own skin loader first and writes the file in one step. It reports undrawn_sprites for sprites still empty and hard_to_read for a readout under 3:1. It never refuses for either.",json!({"path":{"type":"string"}}),&["path"]),
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
            doc.inspect(rect, ink)
        }
        "studio_visualizer_palette" => doc.visualizer_palette(&args)?,
        "studio_playlist_selection" => {
            if let Some(enabled) = args.get("enabled") {
                doc.set_playlist_selection(
                    enabled.as_bool().context("enabled must be boolean")?,
                    "MCP",
                )
            } else {
                json!({"enabled":doc.sheets().iter().any(|(n,_,_)| n == "plselection.bmp")})
            }
        }
        "studio_eq_handles" => {
            if let Some(enabled) = args.get("enabled") {
                doc.set_eq_handles(enabled.as_bool().context("enabled must be boolean")?, "MCP")?
            } else {
                json!({"enabled":doc.sheets().iter().any(|(n,_,_)| n == "eqhandles.bmp")})
            }
        }
        "studio_playlist_background" => {
            if let Some(enabled) = args.get("enabled") {
                doc.set_playlist_background(
                    enabled.as_bool().context("enabled must be boolean")?,
                    "MCP",
                )
            } else {
                json!({"enabled":doc.has_playlist_background()})
            }
        }
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
            let mut appeared: Vec<String> = Vec::new();
            let before: Vec<String> = doc.sheets().into_iter().map(|(n, _, _)| n).collect();
            for (key, call) in [
                ("playlist_background", 0u8),
                ("eq_handles", 1),
                ("playlist_selection", 2),
            ] {
                let Some(on) = args.get(key).and_then(Value::as_bool) else {
                    continue;
                };
                match call {
                    0 => {
                        doc.set_playlist_background(on, "MCP");
                    }
                    1 => {
                        doc.set_eq_handles(on, "MCP")?;
                    }
                    _ => {
                        doc.set_playlist_selection(on, "MCP");
                    }
                }
            }
            let layout: Value = ["footer", "eq_travel", "visualizer_glass"]
                .into_iter()
                .filter_map(|k| args.get(k).map(|v| (k.to_string(), v.clone())))
                .collect::<serde_json::Map<String, Value>>()
                .into();
            if !layout.as_object().unwrap().is_empty() {
                doc.set_layout(&layout, "MCP")?;
            }
            if let Some(colors) = args.get("playlist_colors") {
                doc.set_palette(colors)?;
            }
            if let Some(colors) = args.get("visualizer_colors") {
                doc.visualizer_palette(&json!({ "colors": colors }))?;
            }
            let readability = doc.readability();
            let (playlist, visualizer) = doc.text_palettes();
            let sheets = doc.sheets();
            for (name, w, h) in &sheets {
                if !before.contains(name) {
                    appeared.push(format!(
                        "{name} is now a sheet, {w}x{h}: {}",
                        super::model::Document::new_sheet_note(name)
                    ));
                }
            }
            for name in &before {
                if !sheets.iter().any(|(n, _, _)| n == name) {
                    appeared.push(format!("{name} is gone and will not be exported"));
                }
            }
            let layout = doc.layout();
            let mut value = json!({
                "footer": layout.footer,
                "eq_travel": layout.eq_travel,
                "visualizer_glass": layout.visualizer_glass,
                "playlist_background": doc.has_playlist_background(),
                "eq_handles": sheets.iter().any(|(n, _, _)| n == "eqhandles.bmp"),
                "playlist_selection": sheets.iter().any(|(n, _, _)| n == "plselection.bmp"),
                "playlist_colors": playlist
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), json!(v)))
                    .collect::<serde_json::Map<String, Value>>(),
                "visualizer_colors": visualizer,
                "visualizer_slots": VISUALIZER_SLOTS,
                "readability": readability,
            });
            if !appeared.is_empty() {
                doc.message = appeared.join("; ");
                value["sheets_changed"] = json!(appeared);
            }
            value
        }
        "studio_layout" => {
            if args.get("footer").is_some()
                || args.get("eq_travel").is_some()
                || args.get("visualizer_glass").is_some()
            {
                doc.set_layout(&args, "MCP")?
            } else {
                json!({"layout":doc.layout()})
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
            doc.export(Path::new(args["path"].as_str().context("path required")?))?
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
            if let Some(p) = args["path"].as_str() {
                std::fs::write(p, bytes.get_ref())?;
                let mut out = json!({"path":wrote(p),"size":[im.width(),im.height()],"native":[im.width()/zoom,im.height()/zoom],"zoom":zoom,"revision":doc.revision});
                if let Some(d) = differences {
                    out["sprite"] = d;
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
            write!(s,"POST /mcp HTTP/1.1\r\nHost: {ADDRESS}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{line}",line.len())?;
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
mod tests {
    use super::*;
    use crate::winamp::studio::model::Document;
    use std::sync::{Arc, Mutex};
    fn document() -> SharedDocument {
        crate::winamp::studio::open_document(None).expect("bundled document")
    }
    fn names() -> Vec<String> {
        tools()
            .into_iter()
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect()
    }
    #[test]
    fn the_offered_tools_are_the_panels_a_human_works_in() {
        let offered = names();
        for panel in [
            "studio_canvas",
            "studio_atlas",
            "studio_targets",
            "studio_rectangles",
            "studio_layers",
            "studio_options",
            "studio_states",
            "studio_study",
            "studio_pixel",
            "studio_draw",
            "studio_history",
        ] {
            assert!(offered.iter().any(|n| n == panel), "{panel} is not offered");
        }
        for retired in [
            "studio_state",
            "studio_patch",
            "studio_inspect_region",
            "studio_render",
            "studio_layout",
            "studio_set_palette",
            "studio_visualizer_palette",
        ] {
            assert!(
                !offered.iter().any(|n| n == retired),
                "{retired} describes an editor this one no longer is"
            );
        }
    }
    #[test]
    fn the_retired_tools_still_answer() {
        let shared = document();
        for retired in ["studio_state", "studio_layout", "studio_palette"] {
            call(retired, json!({}), &shared)
                .unwrap_or_else(|e| panic!("{retired} should still answer: {e:#}"));
        }
    }
    #[test]
    fn studio_options_reads_and_writes_everything_outside_the_sheets() {
        let shared = document();
        let before = call("studio_options", json!({}), &shared).unwrap();
        let before: Value =
            serde_json::from_str(before["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(before["footer"], "classic");
        assert_eq!(before["eq_travel"], 52);
        assert_eq!(before["visualizer_glass"], false);
        assert_eq!(before["playlist_colors"].as_object().unwrap().len(), 6);
        assert_eq!(before["visualizer_colors"].as_array().unwrap().len(), 24);
        let after = call(
            "studio_options",
            json!({
                "footer": "time-total",
                "eq_travel": 40,
                "playlist_colors": {"Normal": "#010203"},
            }),
            &shared,
        )
        .unwrap();
        let after: Value =
            serde_json::from_str(after["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(after["footer"], "time-total");
        assert_eq!(after["eq_travel"], 40);
        assert_eq!(after["playlist_colors"]["Normal"], "#010203");
    }
    #[test]
    fn setting_the_pencil_does_not_close_an_open_atlas() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_atlas", json!({"sheet": "monoster.bmp"}), &shared).unwrap();
        for pencil in [
            json!({"face": "small"}),
            json!({"text_scale": 2}),
            json!({"color": "#ffcc66"}),
            json!({"brush_size": 3}),
            json!({"measure": ["MONO", "STEREO"]}),
        ] {
            call("studio_canvas", pencil.clone(), &shared).unwrap();
            let view = shared.lock().unwrap().view.clone();
            assert_eq!(
                view.panel, "atlas",
                "{pencil} moved the pencil off monoster.bmp"
            );
            assert_eq!(view.sheet, "monoster.bmp");
        }
        assert_eq!(shared.lock().unwrap().view.face, "small");
        call("studio_canvas", json!({"zoom": 3}), &shared).unwrap();
        assert_eq!(shared.lock().unwrap().view.panel, "canvas");
        call("studio_atlas", json!({"sheet": "monoster.bmp"}), &shared).unwrap();
        call("studio_canvas", json!({}), &shared).unwrap();
        assert_eq!(
            shared.lock().unwrap().view.panel,
            "canvas",
            "an empty call is still show me the canvas"
        );
    }
    #[test]
    fn reading_an_open_sheet_back_does_not_close_it() {
        let dir = std::env::temp_dir().join("cranamp-atlas-look-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("look.png");
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_atlas", json!({"sheet": "posbar.bmp"}), &shared).unwrap();
        let answer = call(
            "studio_atlas",
            json!({"crop": [0, 0, 307, 10], "magnify": 2, "path": path.to_str().unwrap()}),
            &shared,
        )
        .unwrap();
        let answer: Value =
            serde_json::from_str(answer["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            answer["size"],
            json!([614, 20]),
            "the sheet, not the canvas"
        );
        assert_eq!(shared.lock().unwrap().view.panel, "atlas");
        assert_eq!(shared.lock().unwrap().view.sheet, "posbar.bmp");
        call("studio_atlas", json!({}), &shared).unwrap();
        assert_eq!(shared.lock().unwrap().view.panel, "canvas");
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn a_committed_transaction_reads_the_surface_back_like_a_preview() {
        let dir = std::env::temp_dir().join("cranamp-draw-readback-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stroke.png");
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        let before = shared.lock().unwrap().revision;
        let answer = call(
            "studio_draw",
            json!({
                "operations": [{"op":"rect","x":16,"y":88,"width":23,"height":18,"color":"#ff8800"}],
                "crop": [16, 88, 23, 18],
                "magnify": 4,
                "path": path.to_str().unwrap(),
            }),
            &shared,
        )
        .unwrap();
        let answer: Value =
            serde_json::from_str(answer["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(answer["size"], json!([92, 72]));
        assert!(
            answer["path"].as_str().unwrap().ends_with("stroke.png"),
            "the answer names where it wrote: {}",
            answer["path"]
        );
        assert!(answer["pixels_written"].as_u64().unwrap() > 0);
        assert!(path.exists(), "the file the answer named");
        assert!(
            shared.lock().unwrap().revision > before,
            "a committed stroke, not a dry run"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn refusing_to_discard_unsaved_edits_names_the_argument_that_does_it() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call(
            "studio_draw",
            json!({"operations":[{"op":"rect","x":0,"y":0,"width":4,"height":4,
                                  "color":"#ffffff"}]}),
            &shared,
        )
        .unwrap();
        let refusal = call("studio_new", json!({}), &shared)
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("discard"), "{refusal}");
        assert!(refusal.contains("studio_new"), "{refusal}");
        call("studio_new", json!({"discard": true}), &shared).unwrap();
    }
    #[test]
    fn clip_can_be_set_without_leaving_the_sheet_it_is_for() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_atlas", json!({"sheet": "cbuttons.bmp"}), &shared).unwrap();
        call("studio_canvas", json!({"clip": [23, 18, 23, 18]}), &shared).unwrap();
        {
            let view = shared.lock().unwrap().view.clone();
            assert_eq!(view.panel, "atlas");
            assert_eq!(view.clip, Some([23, 18, 23, 18]));
        }
        call(
            "studio_draw",
            json!({"operations":[{"op":"rect","x":0,"y":0,"width":136,"height":36,
                                  "color":"#abcdef"}]}),
            &shared,
        )
        .unwrap();
        let doc = shared.lock().unwrap();
        assert_eq!(
            doc.inspect([23, 18, 1, 1], None)["hits"][0]["rgba"],
            json!([171, 205, 239, 255])
        );
        assert_eq!(
            doc.inspect([22, 18, 1, 1], None)["hits"][0]["rgba"],
            json!([0, 0, 0, 0]),
            "the column beside the cell is outside the clip"
        );
    }
    #[test]
    fn the_history_is_seekable_by_the_cursor_it_answers_with() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        for colour in ["#111111", "#222222", "#333333"] {
            call(
                "studio_draw",
                json!({"label":format!("step {colour}"),
                       "operations":[{"op":"rect","x":0,"y":0,"width":8,"height":8,
                                      "color":colour}]}),
                &shared,
            )
            .unwrap();
        }
        let read = |args: Value| -> Value {
            let out = call("studio_history", args, &shared).unwrap();
            serde_json::from_str(out["content"][0]["text"].as_str().unwrap()).unwrap()
        };
        let before = read(json!({}));
        assert_eq!(before["cursor"], 3);
        assert_eq!(before["entries"][2]["label"], "step #222222");
        let after = read(json!({"cursor": 1}));
        assert_eq!(after["cursor"], 1);
        assert_eq!(
            shared.lock().unwrap().render().get_pixel(0, 0).0[..3],
            [17, 17, 17]
        );
        read(json!({"cursor": 3}));
        assert_eq!(
            shared.lock().unwrap().render().get_pixel(0, 0).0[..3],
            [51, 51, 51]
        );
    }
    #[test]
    fn the_pixel_probe_answers_the_ground_and_whether_a_colour_reads_on_it() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_atlas", json!({"sheet": "main.bmp"}), &shared).unwrap();
        call(
            "studio_draw",
            json!({"operations":[{"op":"rect","x":0,"y":0,"width":275,"height":115,
                                  "color":"#1b1526"}]}),
            &shared,
        )
        .unwrap();
        let probe = |args: Value| -> Value {
            let out = call("studio_pixel", args, &shared).unwrap();
            serde_json::from_str(out["content"][0]["text"].as_str().unwrap()).unwrap()
        };
        let sheet = probe(json!({"x": 40, "y": 40}));
        assert_eq!(sheet["surface"], "atlas main.bmp");
        call("studio_atlas", json!({}), &shared).unwrap();
        let one = probe(json!({"x": 40, "y": 40}));
        assert_eq!(one["surface"], "canvas");
        assert_eq!(one["ground"], "#1b1526");
        assert_eq!(one["hits"][0]["layer"], "main.background");
        let region = probe(json!({"x": 20, "y": 20, "width": 40, "height": 20,
                                  "ink": "#ffdf9c"}));
        assert_eq!(region["rect"], json!([20, 20, 40, 20]));
        assert_eq!(region["readable"], true);
        assert!(region["contrast"].as_f64().unwrap() > 10.0, "{region}");
        let hidden = probe(json!({"x": 20, "y": 20, "width": 40, "height": 20,
                                  "ink": "#221c30"}));
        assert_eq!(hidden["readable"], false, "{hidden}");
        let outside = probe(json!({"x": 400, "y": 400}));
        assert!(
            outside["nothing_at"]
                .as_str()
                .is_some_and(|s| s.contains("canvas") && s.contains("275x377")),
            "{outside}"
        );
    }
    #[test]
    fn studio_canvas_puts_the_editor_on_the_whole_skin() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_atlas", json!({"sheet": "main.bmp"}), &shared).unwrap();
        assert_eq!(shared.lock().unwrap().view.panel, "atlas");
        call("studio_canvas", json!({"zoom": 3}), &shared).unwrap();
        let view = shared.lock().unwrap().view.clone();
        assert_eq!(view.panel, "canvas");
        assert_eq!(view.zoom, 3);
        let listed = call("studio_targets", json!({}), &shared).unwrap();
        let listed: Value =
            serde_json::from_str(listed["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(listed["sprites"].as_array().unwrap().len() > 40);
        assert_eq!(
            listed["chosen"],
            json!([]),
            "auto by default, as for a human"
        );
    }
}
#[cfg(test)]
mod drawing_tests {
    use super::*;
    use crate::winamp::studio::model::Document;
    use std::sync::{Arc, Mutex};
    fn blank() -> SharedDocument {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_canvas", json!({}), &shared).unwrap();
        shared
    }
    fn result(shared: &SharedDocument, name: &str, args: Value) -> Value {
        let out = call(name, args, shared).unwrap();
        serde_json::from_str(out["content"][0]["text"].as_str().unwrap()).unwrap()
    }
    #[test]
    fn text_draws_a_label_in_the_editors_own_face() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"text","x":20,"y":30,"text":"CAT","color":"#f5c06a"}]}),
        );
        assert!(out["pixels_written"].as_u64().unwrap() > 20);
        assert_eq!(out["bounds"][0], 20);
        assert_eq!(out["bounds"][2], 36);
        assert_eq!(out["bounds"][3], 36);
    }
    #[test]
    fn origin_puts_the_coordinates_on_the_sprite_wherever_it_currently_sits() {
        let shared = blank();
        call("studio_canvas", json!({"volume": 5}), &shared).unwrap();
        let out = result(
            &shared,
            "studio_draw",
            json!({"origin":"main.volume.thumb",
                   "operations":[{"op":"pixel","x":0,"y":0,"color":"#4488cc"}]}),
        );
        assert_eq!(out["pixels_written"], 1);
        assert_eq!(out["clipped_pixels"], 0);
        for frame in (0..28).step_by(9) {
            let mut doc = shared.0.lock().unwrap();
            doc.state(json!({"volume": frame})).unwrap();
            let at = doc
                .layers()
                .into_iter()
                .find(|l| l.id == "main.volume.thumb")
                .unwrap()
                .destination;
            assert_eq!(
                doc.render().get_pixel(at[0], at[1]).0,
                [0x44, 0x88, 0xcc, 0xff],
                "frame {frame}"
            );
        }
    }
    #[test]
    fn writing_one_shared_source_cell_twice_says_so() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"pixel","x":48,"y":26,"color":"#112233"},
                                 {"op":"pixel","x":60,"y":26,"color":"#ccddee"}]}),
        );
        assert!(out["overwrites"].as_u64().unwrap() > 0);
        let note = out["overwrite_sample"].as_str().unwrap();
        assert!(note.contains("numbers.bmp"), "{note}");
        assert!(note.contains("layers"), "{note}");
        let aimed = result(
            &shared,
            "studio_draw",
            json!({"layers":["main.digit0"],
                   "operations":[{"op":"pixel","x":48,"y":26,"color":"#112233"},
                                 {"op":"pixel","x":60,"y":26,"color":"#ccddee"}]}),
        );
        assert_eq!(aimed["overwrites"], 0);
        assert!(aimed["overwrite_sample"].is_null());
    }
    #[test]
    fn a_half_transparent_stamp_blends_with_what_is_under_it() {
        let shared = blank();
        result(
            &shared,
            "studio_draw",
            json!({"layers":["main.background"],
                   "operations":[{"op":"rect","x":8,"y":8,"width":4,"height":4,
                                  "color":"#202060"}]}),
        );
        let mut im = image::RgbaImage::new(1, 1);
        im.put_pixel(0, 0, image::Rgba([0xff, 0xff, 0xff, 0x80]));
        let mut png = std::io::Cursor::new(Vec::new());
        im.write_to(&mut png, image::ImageFormat::Png).unwrap();
        result(
            &shared,
            "studio_draw",
            json!({"layers":["main.background"],
                   "operations":[{"op":"image","x":9,"y":9,"data":base64(png.get_ref())}]}),
        );
        let doc = shared.0.lock().unwrap();
        let [r, g, b, _] = doc.render().get_pixel(9, 9).0;
        assert!((r as i32 - 0x90).abs() <= 2, "r {r:#x}");
        assert!((g as i32 - 0x90).abs() <= 2, "g {g:#x}");
        assert!((b as i32 - 0xb0).abs() <= 2, "b {b:#x}");
    }
    #[test]
    fn what_a_draw_reports_covers_that_draw_and_no_earlier_one() {
        let shared = blank();
        let first = result(
            &shared,
            "studio_draw",
            json!({"layers":["main.play"],
                   "operations":[{"op":"rect","x":0,"y":0,"width":40,"height":40,
                                  "color":"#112233"}]}),
        );
        assert!(
            first["clipped_pixels"].as_u64().unwrap() > 0,
            "aimed past the sprite"
        );
        let second = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"pixel","x":44,"y":92,"color":"#445566"}]}),
        );
        assert_eq!(second["bounds"], json!([44, 92, 44, 92]));
        assert_eq!(second["clipped_pixels"], 0);
    }
    #[test]
    fn an_image_stamps_its_opaque_pixels_and_leaves_the_transparent_ones_alone() {
        let shared = blank();
        let before = {
            let doc = shared.0.lock().unwrap();
            doc.render().get_pixel(41, 31).0
        };
        let mut im = image::RgbaImage::new(2, 2);
        im.put_pixel(0, 0, image::Rgba([0x11, 0x22, 0x33, 0xff]));
        im.put_pixel(1, 0, image::Rgba([0x11, 0x22, 0x33, 0xff]));
        let mut png = std::io::Cursor::new(Vec::new());
        im.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let out = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"image","x":40,"y":30,"data":base64(png.get_ref())}]}),
        );
        assert_eq!(out["pixels_written"], 2);
        let doc = shared.0.lock().unwrap();
        assert_eq!(doc.render().get_pixel(40, 30).0, [0x11, 0x22, 0x33, 0xff]);
        assert_eq!(doc.render().get_pixel(41, 31).0, before);
    }
    #[test]
    fn a_stroke_reports_what_fell_outside_the_sprites_it_was_aimed_at() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_draw",
            json!({
                "layers": ["main.close"],
                "operations": [{"op":"rect","x":260,"y":0,"width":20,"height":20,
                                "color":"#f5c06a","fill":true}],
            }),
        );
        assert!(
            out["clipped_pixels"].as_u64().unwrap() > 0,
            "most of that rectangle is outside a 9x9 button and the caller has to be told"
        );
        assert!(
            out["pixels_written"].as_u64().unwrap() > 0,
            "the rest still landed"
        );
    }
    #[test]
    fn an_argument_a_tool_does_not_have_is_refused_by_name() {
        let refusal = reject_unknown_arguments("studio_screenshot", &json!({"brush": "pencil"}))
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("brush"), "{refusal}");
        assert!(
            refusal.contains("studio_canvas"),
            "say where the field does belong: {refusal}"
        );
        assert!(
            refusal.contains("crop"),
            "and what this one takes: {refusal}"
        );
        let refusal = reject_unknown_arguments("studio_undo", &json!({"nonesuch": 1}))
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("nonesuch"), "{refusal}");
        reject_unknown_arguments(
            "studio_screenshot",
            &json!({"crop":[0,0,8,8],"panel":"main"}),
        )
        .unwrap();
        reject_unknown_arguments("studio_layout", &json!({"anything": 1})).unwrap();
    }
    #[test]
    fn the_canvas_can_be_read_back_cropped() {
        let shared = blank();
        let dir = std::env::temp_dir().join("cranamp-crop-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("crop.png");
        let out = result(
            &shared,
            "studio_canvas",
            json!({"zoom": 2, "crop": [16, 88, 23, 18], "path": path.to_str().unwrap()}),
        );
        assert_eq!(
            out["size"],
            json!([46, 36]),
            "the crop, at the asked-for zoom"
        );
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn a_blank_document_opens_on_the_whole_skin() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        let out = result(&shared, "studio_new", json!({ "discard": true }));
        assert_eq!(out["surface"], "canvas");
        assert_eq!(out["canvas"], json!([275, 377]));
        assert_eq!(out["sprites"], json!(81));
    }
    #[test]
    fn a_curve_may_start_and_end_between_pixels() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"curve","x":20.5,"y":30.25,"x2":60.5,"y2":50.75,
                                 "control":[24.5,48.5],"color":"#ffffff","brush_size":2}]}),
        );
        assert!(out["pixels_written"].as_u64().unwrap() > 0);
        let refused = call(
            "studio_draw",
            json!({"operations":[{"op":"rect","x":1.5,"y":2,"width":3,"height":3}]}),
            &shared,
        )
        .unwrap_err();
        let refused = format!("{refused:#}");
        assert!(
            refused.contains("x is a whole number of pixels"),
            "a refusal names the field and the value it got: {refused}"
        );
    }
    #[test]
    fn the_catalogue_answers_for_the_skin_while_one_sheet_is_open() {
        let shared = blank();
        call("studio_atlas", json!({"sheet":"volume.bmp"}), &shared).unwrap();
        let out = result(
            &shared,
            "studio_targets",
            json!({"id":"volume.track","variants":true}),
        );
        assert_eq!(out["sprites"][0]["id"], "main.volume.track");
        assert_eq!(out["sprites"][0]["variants"].as_array().unwrap().len(), 28);
        let empty = result(&shared, "studio_rectangles", json!({"id":"main.play"}));
        assert!(
            empty["note"]
                .as_str()
                .unwrap_or_default()
                .contains("atlas volume.bmp"),
            "an empty match says what it looked in: {empty}"
        );
    }
    #[test]
    fn a_canvas_rectangle_is_labelled_with_the_variant_it_is_showing() {
        let shared = blank();
        call("studio_canvas", json!({ "volume": 20 }), &shared).unwrap();
        let out = result(&shared, "studio_rectangles", json!({"id":"volume.track"}));
        assert_eq!(out["rectangles"][0]["variant"], json!(20));
        assert_eq!(out["rectangles"][0]["label"], "track · 20");
    }
    #[test]
    fn ink_that_crosses_into_a_repeated_cell_says_so() {
        let shared = blank();
        call("studio_atlas", json!({"sheet":"pledit.bmp"}), &shared).unwrap();
        let spilled = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"text","x":98,"y":32,"text":"/ CAT SCAN",
                                 "color":"#ffffff","face":"small"}]}),
        );
        assert_eq!(spilled["crossed_cells"][0]["cell"], "playlist.top.tile");
        assert_eq!(spilled["crossed_cells"][0]["drawn"], json!(9));
        let inside = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"rect","x":127,"y":21,"width":25,"height":20,
                                 "color":"#203040"}]}),
        );
        assert!(
            inside.get("crossed_cells").is_none(),
            "a cell painted on its own is not a crossing: {inside}"
        );
        for color in ["#ff00ff", "#203040"] {
            let whole = result(
                &shared,
                "studio_draw",
                json!({"operations":[{"op":"rect","x":0,"y":0,"width":280,"height":190,
                                     "color":color}]}),
            );
            assert!(
                whole.get("crossed_cells").is_none(),
                "clearing or washing a whole sheet is not a crossing: {whole}"
            );
        }
    }
    #[test]
    fn the_playlist_background_is_sampled_to_its_last_row() {
        let shared = blank();
        result(
            &shared,
            "studio_options",
            json!({"playlist_background":true}),
        );
        call(
            "studio_canvas",
            json!({ "preview_playlist_height": 145 }),
            &shared,
        )
        .unwrap();
        call("studio_atlas", json!({"sheet":"plbg.bmp"}), &shared).unwrap();
        let out = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"rect","x":0,"y":0,"width":243,"height":203,
                                 "color":"#101418"}]}),
        );
        assert_eq!(out["unsampled_pixels"], json!(0));
    }
    #[test]
    fn a_state_sheet_answers_for_a_named_sprite_whatever_surface_is_open() {
        let shared = blank();
        let dir = std::env::temp_dir().join("cranamp-state-sheet-test");
        std::fs::create_dir_all(&dir).unwrap();
        result(
            &shared,
            "studio_targets",
            json!({"solo":"main.volume.track"}),
        );
        let on_canvas = result(
            &shared,
            "studio_states",
            json!({"path":dir.join("a.png").to_str().unwrap()}),
        );
        assert_eq!(on_canvas["sprite"]["id"], "main.volume.track");
        assert_eq!(on_canvas["sprite"]["of"], json!(28));
        result(&shared, "studio_atlas", json!({"sheet":"volume.bmp"}));
        let on_sheet = result(
            &shared,
            "studio_states",
            json!({"id":"volume.track","path":dir.join("b.png").to_str().unwrap()}),
        );
        assert_eq!(
            on_sheet["sprite"]["id"], "main.volume.track",
            "the sprite is the one that was chosen, not the sheet it lives on"
        );
        assert_eq!(on_sheet["sprite"]["of"], json!(28));
        assert_eq!(on_sheet["native"], on_canvas["native"]);
        let missing = call("studio_states", json!({"id":"kettle"}), &shared)
            .unwrap_err()
            .to_string();
        assert!(missing.contains("in this skin's"), "{missing}");
        let several = call("studio_states", json!({"id":"track"}), &shared)
            .unwrap_err()
            .to_string();
        assert!(several.contains("name one exactly"), "{several}");
    }
    #[test]
    fn the_engine_measures_its_own_face() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_canvas",
            json!({"face":"small","text_scale":1,"measure":["MONO","STEREO","16K"]}),
        );
        assert_eq!(out["measured"][0]["width"], json!(19), "the 27-pixel lamp");
        assert_eq!(out["measured"][1]["width"], json!(29), "the 29-pixel lamp");
        let tight = result(
            &shared,
            "studio_canvas",
            json!({"text_spacing":0,"measure":"STEREO"}),
        );
        assert_eq!(tight["measured"][0]["width"], json!(24), "at spacing 0");
        assert_eq!(tight["spacing"], json!(0), "measure reports what it used");
        let now = result(&shared, "studio_status", json!({}));
        assert_eq!(now["view"]["text_spacing"], json!(0));
        let drawn = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"text","x":0,"y":0,"text":"II","color":"#ffffff",
                                 "face":"small"}]}),
        );
        let narrow = drawn["bounds"][2].as_i64().unwrap();
        let wider = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"text","x":0,"y":20,"text":"II","color":"#ffffff",
                                 "face":"small","spacing":2}]}),
        );
        assert_eq!(
            wider["bounds"][2].as_i64().unwrap(),
            narrow + 2,
            "an operation that names its own spacing still wins"
        );
        result(&shared, "studio_canvas", json!({"text_spacing": 1}));
        assert_eq!(out["measured"][2]["width"], json!(14), "a band caption");
        assert_eq!(out["measured"][0]["advance"], json!(20));
        let scaled = result(
            &shared,
            "studio_canvas",
            json!({"face":"5x7","text_scale":2,"measure":"CATAMP"}),
        );
        assert_eq!(scaled["measured"][0]["width"], json!(70));
        assert_eq!(scaled["measured"][0]["height"], json!(14));
        let missing = result(
            &shared,
            "studio_canvas",
            json!({"face":"small","measure":["A@B"]}),
        );
        assert_eq!(missing["unsupported_characters"], json!(["@"]));
    }
    #[test]
    fn a_crop_may_be_magnified_past_the_zoom_cap() {
        let shared = blank();
        let dir = std::env::temp_dir().join("cranamp-magnify-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.png");
        let out = result(
            &shared,
            "studio_canvas",
            json!({"crop":[16,88,23,18],"magnify":32,"path":path.to_str().unwrap()}),
        );
        assert_eq!(out["size"], json!([736, 576]));
        let refused = call(
            "studio_canvas",
            json!({"magnify":32,"path":path.to_str().unwrap()}),
            &shared,
        )
        .unwrap_err();
        let refused = format!("{refused:#}");
        assert!(
            refused.contains("over the 2048 pixel limit") && refused.contains("fits 5 times"),
            "a refusal says how far it would fit: {refused}"
        );
        std::fs::remove_file(&path).ok();
    }
    #[test]
    fn a_catalogue_filter_takes_several_needles() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_targets",
            json!({"id":["volume.track","balance.track","position.thumb"]}),
        );
        let ids: Vec<&str> = out["sprites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            vec![
                "main.position.thumb",
                "main.volume.track",
                "main.balance.track"
            ]
        );
    }
    #[test]
    fn a_panel_and_a_crop_compose_in_native_coordinates() {
        assert_eq!(
            panel_canvas("main", Some(json!([14, 86, 146, 22])), 145).unwrap(),
            [14, 86, 146, 22]
        );
        assert_eq!(
            panel_canvas("equalizer", Some(json!([21, 38, 14, 63])), 145).unwrap(),
            [21, 154, 14, 63]
        );
        assert_eq!(
            panel_canvas("playlist", None, 261).unwrap(),
            [0, 232, 275, 261]
        );
        let outside = panel_canvas("main", Some(json!([14, 110, 146, 22])), 145)
            .unwrap_err()
            .to_string();
        assert!(
            outside.contains("inside the main panel, which is 275x116 native"),
            "{outside}"
        );
    }
    #[test]
    fn rectangles_answer_what_a_band_would_cross() {
        let shared = blank();
        let out = result(&shared, "studio_rectangles", json!({"at":[0,52,275,20]}));
        let ids: Vec<&str> = out["rectangles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"runtime.main.SPECTRUM"), "{ids:?}");
        assert!(ids.contains(&"main.volume.track"), "{ids:?}");
        assert!(!ids.contains(&"main.position.track"), "{ids:?}");
        assert!(!ids.contains(&"main.play"), "{ids:?}");
    }
}
