//! Local MCP transport. Both the native GUI and tools mutate SharedDocument.
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
fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
}
fn tools() -> Vec<Value> {
    vec![
 tool("studio_status","Read the shared document, active state, layers and source rectangles.",json!({}),&[]),
 tool("studio_new","Create a transparent classic skin from scratch. No artwork or metadata is inherited. Unsaved edits require discard=true.",json!({"discard":{"type":"boolean"}}),&[]),
 tool("studio_open","Load a WSZ into the running native Studio. Existing unsaved edits require discard=true.",json!({"path":{"type":"string"},"discard":{"type":"boolean"}}),&["path"]),
 tool("studio_state","Set GUI panel, brush, zoom, layer and preview states. Panel canvas stitches main, EQ and playlist at their original positions; layer IDs have main., equalizer., playlist. prefixes. Panel atlas plus sheet edits a complete bitmap at native coordinates using the same pencil/history. Frame values are integers 0..27. Use layers for any combination of layer IDs; an empty array or layer auto picks the topmost sprite. All_states paints every variant of the targeted sprite.",json!({"panel":{"enum":["main","equalizer","playlist","atlas","canvas"]},"sheet":{"type":"string"},"layer":{"type":"string"},"layers":{"type":"array","items":{"type":"string"}},"zoom":{"type":"integer","minimum":1,"maximum":8},"preview_playlist_height":{"type":"integer","minimum":145,"maximum":522},"presentation":{"type":"boolean"},"color":{"type":"string"},"brush_size":{"type":"integer","minimum":1,"maximum":32},"curve_bend":{"type":"integer","minimum":-100,"maximum":100},"clean_corners":{"type":"boolean"},"brush":{"enum":["pencil","line","rect","ellipse","lift","stamp","glass","curve","tuft"]},"filled":{"type":"boolean"},"mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},"grid":{"type":"boolean"},"guides":{"type":"boolean"},"clip":{"type":["array","null"],"items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"paint_layer":{"type":["string","null"]},"alpha_lock":{"type":"boolean"},"mask_colors":{"type":"array","items":{"type":"string"}},"all_states":{"type":"boolean"},"pressed":{"type":"boolean"},"active":{"type":"boolean"},"volume":{"type":"integer","minimum":0,"maximum":27},"balance":{"type":"integer","minimum":0,"maximum":27},"position":{"type":"integer","minimum":0,"maximum":27},"scroll":{"type":"integer","minimum":0,"maximum":27},"eq":{"type":"array","items":{"type":"integer","minimum":0,"maximum":27},"minItems":11,"maxItems":11},"digit":{"type":"integer","minimum":0,"maximum":9},"playback":{"type":"integer","minimum":0,"maximum":2}}),&[]),
 tool("studio_draw","Paint assembled panel pixels. One atomic undoable transaction. Optional layers targets every selected sprite beneath each pixel; omitted uses GUI selection. Optional label names the history entry. Pixel/line/rect/stamp operations map into their owning atlas; all_states maps the same local pixels into every pressed/frame variant. Coordinates are native pixels, never zoomed coordinates. Path points start [x,y], then line [x,y], quadratic [cx,cy,x,y], cubic [c1x,c1y,c2x,c2y,x,y], relative to x/y. Filled paths use native scanlines. brush_size controls solid stroke width; mirror_x/y reflect around canvas centre. Optional ramp colour array and ramp_axis [x1,y1,x2,y2] shade geometry with exact palette colours. Curve and tuft use endpoints x/y and x2/y2, curve_bend -100..100 or an explicit absolute control [x,y]. Tuft tapers brush_size to a pointed tip. Optional clean_corners removes redundant elbows from open 1px curves/paths only; preserves endpoints, fills and thick strokes. No antialiasing. Stamp rows contain palette-character pixel art, unmapped characters are skipped.",json!({"mask_colors":{"type":"array","items":{"type":"string"}},"label":{"type":"string"},"layer":{"type":"string"},"layers":{"type":"array","items":{"type":"string"}},"all_states":{"type":"boolean"},"operations":{"type":"array","items":{"type":"object","properties":{"op":{"enum":["pixel","line","rect","ellipse","path","curve","tuft","stamp","cluster"]},"x":{"type":"integer"},"y":{"type":"integer"},"x2":{"type":"integer"},"y2":{"type":"integer"},"width":{"type":"integer"},"height":{"type":"integer"},"fill":{"type":"boolean"},"brush_size":{"type":"integer","minimum":1,"maximum":32},"curve_bend":{"type":"integer","minimum":-100,"maximum":100},"clean_corners":{"type":"boolean"},"mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},"control":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2},"points":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"ramp":{"type":"array","items":{"type":"string"}},"material":{"enum":["glass"]},"bevel":{"type":"number","minimum":1,"maximum":128},"refraction":{"type":"number","minimum":0,"maximum":32},"ramp_axis":{"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4},"color":{"type":"string"},"rows":{"type":"array","items":{"type":"string"}},"palette":{"type":"object","additionalProperties":{"type":"string"}}},"required":["op","x","y"]}}}),&["operations"]),
 tool("studio_inspect_region","Map a native review rectangle on the current panel to intersecting source rectangles and live runtime reservations, including timer digits. Read-only; includes underlying parts rather than a visibility mask.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4}}),&["rect"]),
 tool("studio_guides","Read labeled sprite rectangles. Atlas mode includes every source state; assembled modes show current destinations and runtime text/spectrum reservations. Optional select sets the target, and clips atlas painting to that source rectangle. Guides are not exported.",json!({"select":{"type":"string"}}),&[]),
 tool("studio_patch","Inspect region baseline tokens or validate/apply an isolated native drawing handoff. Each part has a sheet and assigned source rect. Validate/apply require expected tokens from inspect and explicit operations; geometry outside its part is rejected by default. Per-part clip_to_rect=true explicitly rasterizes whole geometry then clips pixels to the assigned rect using the human paint clip; this supports continuous drawing across atlas sections without resizing. Uses deterministic drawing defaults, preserves GUI view, and creates one new painting layer and one undo. Disjoint patches may share a baseline; overlapping changed regions are rejected. No clipboard clusters or implicit mirrors. Optional replace_layer revises an existing standalone plane in place; inspect with its ID to obtain layer_expected, then submit its complete replacement geometry and cover every existing painted pixel. Keeps other planes/order/view intact.",json!({"action":{"enum":["inspect","validate","apply"]},"replace_layer":{"type":"string"},"layer_expected":{"type":"string"},"name":{"type":"string"},"parts":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"object","properties":{"sheet":{"type":"string"},"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"expected":{"type":"string"},"clip_to_rect":{"type":"boolean","default":false},"label":{"type":"string"},"operations":{"type":"array","items":{"type":"object"}}},"required":["sheet","rect"]}}}),&["parts"]),
 tool("studio_paint_layers","Independent artwork planes above the original atlases. Add/select/name/show/hide/lock/set opacity, move (bottom=0), clip to the immediately lower painting plane, merge down or delete; all changes share undo. Sprite target selection remains independent. WSZ exports the composite, project files retain editable planes.",json!({"action":{"enum":["list","add","select","set","move","merge_down","delete"]},"id":{"type":"string"},"name":{"type":"string"},"visible":{"type":"boolean"},"locked":{"type":"boolean"},"clip_below":{"type":"boolean"},"opacity":{"type":"integer","minimum":0,"maximum":255},"index":{"type":"integer","minimum":0}}),&[]),
 tool("studio_project","Save or open a layered .cstudio project. WSZ remains the flattened skin export. Opening unsaved work requires discard=true.",json!({"action":{"enum":["save","open"]},"path":{"type":"string"},"discard":{"type":"boolean"}}),&["action","path"]),
 tool("studio_cluster","Pick up a native pixel region from selected layers (Auto captures visible canvas). No skin mutation. Omit rect to read clipboard as reusable stamp rows/palette. Optional flip_x, flip_y and quarter_turns transform the clipboard losslessly. Paint it using studio_draw op cluster, or human Stamp brush. Clipboard is an editor tool, never added to WSZ.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"flip_x":{"type":"boolean"},"flip_y":{"type":"boolean"},"quarter_turns":{"type":"integer","minimum":0,"maximum":3}}),&[]),
 tool("studio_study","Read-only material study board: native crop above integer enlarged detail, optional grayscale value preview, registered layer geometry, pixel grid, and reference image alongside. Does not edit skin or change view. rect defaults to last lifted region. Reference pixels are never imported into the skin.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"zoom":{"type":"integer","minimum":1,"maximum":8},"selected":{"type":"boolean"},"values":{"type":"boolean"},"grid":{"type":"boolean"},"geometry":{"type":"boolean"},"reference":{"type":"string"},"reference_rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"path":{"type":"string"}}),&[]),
 tool("studio_pixel","Inspect a canvas pixel, all underlying sprites, atlas coordinates and sharing.",json!({"x":{"type":"integer","minimum":0},"y":{"type":"integer","minimum":0}}),&["x","y"]),
 tool("studio_render","Return the assembled editor panel as an image at an integer nearest-neighbor zoom. Optional output path saves a PNG.",json!({"zoom":{"type":"integer","minimum":1,"maximum":8},"path":{"type":"string"}}),&[]),
 tool("studio_states","Return every source variant of the selected layer as a nearest-neighbor contact sheet. Inspect all 28 track frames or both pressed/released sprites side by side.",json!({"zoom":{"type":"integer","minimum":1,"maximum":8},"path":{"type":"string"}}),&[]),
 tool("studio_visualizer_palette","Read or replace the 24 VISCOLOR.TXT colors. Entry 0 is the runtime visualizer background. Undoable with shared history.",json!({"colors":{"type":"array","items":{"type":"string"},"minItems":24,"maxItems":24}}),&[]),
 tool("studio_playlist_selection","Enable a native 243×11 selection row, paintable as list.selection or plselection.bmp in the atlas workspace. The player reserves an 8-pixel marker gutter. Shared GUI/MCP history.",json!({"enabled":{"type":"boolean"}}),&[]),
 tool("studio_eq_handles","Enable independent normal/pressed 14×25 artwork for each EQ slider; shared pencil/layer selection/history; native eqhandles.bmp atlas. Travel is limited to 38 pixels.",json!({"enabled":{"type":"boolean"}}),&[]),
 tool("studio_playlist_background","Enable a paintable playlist interior. Paint list.background at native coordinates; exports optional 243×203 plbg.bmp, tiled without stretching. Shared human/MCP history.",json!({"enabled":{"type":"boolean"}}),&[]),
 tool("studio_layout","Read or set optional Cranamp skin layout metadata. Footer classic preserves standard WSZ placement; time-total uses two centered elapsed/total readouts. Eq_travel reserves room within the 63px track for full-size artwork. Changes share human/MCP undo and export in cranamp.json.",json!({"footer":{"enum":["classic","time-total"]},"eq_travel":{"type":"integer","minimum":1,"maximum":52},"visualizer_glass":{"type":"boolean"}}),&[]),
 tool("studio_palette","List exact RGBA colors for manual palette editing.",json!({}),&[]),
 tool("studio_set_palette","Edit the six PLEDIT.TXT palette colors. This controls the solid playlist background, normal/current text and selection. Undoable and shared with the GUI.",json!({"Normal":{"type":"string"},"Current":{"type":"string"},"NormalBG":{"type":"string"},"SelectedBG":{"type":"string"},"MbFG":{"type":"string"},"MbBG":{"type":"string"}}),&[]),
 tool("studio_recolor","Undoable exact palette substitutions. Preserves every pixel and geometry; no resampling. Optionally restrict to one BMP sheet.",json!({"sheet":{"type":"string"},"colors":{"type":"object","additionalProperties":{"type":"string"}}}),&["colors"]),
 tool("studio_history","List the shared human/MCP history, current cursor and retained steps.",json!({}),&[]),
 tool("studio_history_goto","Restore any retained history cursor. New edits after undo replace the redo branch.",json!({"cursor":{"type":"integer","minimum":0}}),&["cursor"]),
 tool("studio_undo","Undo the last human or MCP drawing transaction.",json!({}),&[]),
 tool("studio_redo","Redo the last undone transaction.",json!({}),&[]),
 tool("studio_screenshot","Capture the actual Cranamp GPU-rendered Studio scene after the requested document revision is composed, at native logical pixels without thumbnail scaling. Optional crop is [x,y,width,height] in scene pixels. Does not alter the skin or GUI state.",json!({"path":{"type":"string"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4}}),&[]),
 tool("studio_export","Validate through Cranamp's loader and write the edited classic WSZ atomically.",json!({"path":{"type":"string"}}),&["path"]),
]
}
fn text(value: Value) -> Value {
    json!({"content":[{"type":"text","text":value.to_string()}]})
}
pub fn call(name: &str, args: Value, shared: &SharedDocument) -> Result<Value> {
    // GPU capture must run without holding the document lock: the UI needs it to draw.
    if name == "studio_screenshot" {
        let revision = shared
            .lock()
            .map_err(|_| anyhow::anyhow!("Studio document lock"))?
            .revision;
        let mut im = super::capture_scene(revision)?;
        if let Some(crop) = args.get("crop") {
            let r: [u32; 4] =
                serde_json::from_value(crop.clone()).context("crop must be [x,y,width,height]")?;
            if r[2] == 0
                || r[3] == 0
                || r[0].checked_add(r[2]).is_none_or(|v| v > im.width())
                || r[1].checked_add(r[3]).is_none_or(|v| v > im.height())
            {
                bail!("Crop must fit inside the captured scene");
            }
            im = image::imageops::crop_imm(&im, r[0], r[1], r[2], r[3]).to_image();
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        im.write_to(&mut bytes, image::ImageFormat::Png)?;
        if let Some(path) = args["path"].as_str() {
            std::fs::write(path, bytes.get_ref())?;
        }
        return Ok(
            json!({"content":[{"type":"image","mimeType":"image/png","data":base64(bytes.get_ref())},{"type":"text","text":format!("Actual Cranamp GPU scene: {}×{} pixels",im.width(),im.height())}]}),
        );
    }
    let mut doc = shared
        .lock()
        .map_err(|_| anyhow::anyhow!("Studio document lock"))?;
    let value = match name {
        "studio_status" => doc.status(),
        "studio_new" => {
            if doc.dirty && args["discard"] != true {
                bail!("Export or undo unsaved edits before creating a blank skin");
            }
            let revision = doc.revision + 1;
            *doc = Document::blank();
            doc.revision = revision;
            doc.status()
        }
        "studio_state" => doc.state(args)?,
        "studio_open" => {
            if doc.dirty && args["discard"] != true {
                bail!("Export or undo unsaved edits first, or explicitly set discard=true");
            }
            let p = args["path"].as_str().context("path required")?;
            let revision = doc.revision + 1;
            *doc = if p.ends_with(".cstudio") {
                Document::open_project(&std::fs::read(p)?)?
            } else {
                Document::open(&std::fs::read(p)?, Some(p.into()))?
            };
            doc.revision = revision;
            doc.status()
        }
        "studio_inspect_region" => {
            doc.inspect_region(serde_json::from_value(args["rect"].clone())?)?
        }
        "studio_guides" => {
            if let Some(id) = args["select"].as_str() {
                doc.select_guide(id)?
            } else {
                json!({"guides":doc.guides()})
            }
        }
        "studio_patch" => doc.patch(&args)?,
        "studio_paint_layers" => doc.paint_layer_command(&args, "MCP")?,
        "studio_project" => {
            let p = Path::new(args["path"].as_str().context("Project path required")?);
            if args["action"] == "save" {
                doc.save_project(p)?
            } else if args["action"] == "open" {
                anyhow::ensure!(
                    !doc.dirty || args["discard"] == true,
                    "Save or export unsaved edits first"
                );
                let revision = doc.revision + 1;
                *doc = Document::open_project(&std::fs::read(p)?)?;
                doc.revision = revision;
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
        "studio_draw" => doc.draw(&args)?,
        "studio_pixel" => doc.inspect(
            args["x"].as_u64().context("x required")? as u32,
            args["y"].as_u64().context("y required")? as u32,
        ),
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
        "studio_history" => doc.history(),
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
            let im = if name == "studio_study" {
                super::study::board(&doc, &args)?
            } else if name == "studio_states" {
                doc.state_sheet()
            } else {
                doc.render()
            };
            let im = image::imageops::resize(
                &im,
                im.width() * zoom,
                im.height() * zoom,
                image::imageops::FilterType::Nearest,
            );
            let mut bytes = std::io::Cursor::new(Vec::new());
            im.write_to(&mut bytes, image::ImageFormat::Png)?;
            if let Some(p) = args["path"].as_str() {
                std::fs::write(p, bytes.get_ref())?;
            }
            return Ok(
                json!({"content":[{"type":"image","mimeType":"image/png","data":base64(bytes.get_ref())},{"type":"text","text":format!("{}x{} native panel, {}x integer zoom; revision {}",im.width()/zoom,im.height()/zoom,zoom,doc.revision)}]}),
            );
        }
        _ => bail!("Unknown Studio tool {name}"),
    };
    Ok(text(value))
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
