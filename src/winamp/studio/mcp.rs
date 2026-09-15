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
/// The tools an agent is offered.
///
/// One per panel the editor actually has, named for it, so an agent works the
/// way a person does: open the whole skin, draw on it, look at what the stroke
/// hit, look at the sprite rectangles and states, and set the skin's options.
/// The older per-window and isolated-handoff tools still answer for the scripts
/// that use them, but they are no longer offered: they describe a way of
/// editing this editor no longer has.
fn tools() -> Vec<Value> {
    vec![
 tool("studio_canvas","The whole skin as one canvas -- main, equalizer and playlist joined at their own positions -- which is the only drawing surface. Reads it back as an image at an integer zoom, optionally cropped to [x, y, width, height] so one sprite can be checked without the whole skin, and sets what the canvas shows: zoom, brush, colour, stroke width, the sprite state every sprite is drawn in, and whether rectangles are outlined. This is where drawing happens; studio_atlas is a detour to one BMP.",json!({"zoom":{"type":"integer","minimum":1,"maximum":8},"path":{"type":"string"},"crop":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"color":{"type":"string"},"brush":{"enum":["pencil","line","rect","ellipse","lift","stamp","glass","curve","tuft"]},"brush_size":{"type":"integer","minimum":1,"maximum":32},"curve_bend":{"type":"integer","minimum":-100,"maximum":100},"clean_corners":{"type":"boolean"},"filled":{"type":"boolean"},"mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},"grid":{"type":"boolean"},"guides":{"type":"boolean"},"alpha_lock":{"type":"boolean"},"mask_colors":{"type":"array","items":{"type":"string"}},"all_states":{"type":"boolean"},"clip":{"type":["array","null"],"items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"pressed":{"type":"boolean"},"active":{"type":"boolean"},"volume":{"type":"integer","minimum":0,"maximum":27},"balance":{"type":"integer","minimum":0,"maximum":27},"position":{"type":"integer","minimum":0,"maximum":27},"scroll":{"type":"integer","minimum":0,"maximum":27},"eq":{"type":"array","items":{"type":"integer","minimum":0,"maximum":27},"minItems":11,"maxItems":11},"digit":{"type":"integer","minimum":0,"maximum":9},"playback":{"type":"integer","minimum":0,"maximum":2},"presentation":{"type":"boolean"},"preview_playlist_height":{"type":"integer","minimum":145,"maximum":522}}),&[]),
 tool("studio_atlas","Edit one BMP on its own, at native coordinates, with the same pencil and undo history. Omit sheet to go back to the whole skin, which is where drawing normally happens.",json!({"sheet":{"type":"string"},"zoom":{"type":"integer","minimum":1,"maximum":8}}),&[]),
 tool("studio_targets","Which sprites a stroke is routed into -- the editor's Sprite targets panel. Omit everything to list them. Auto (the default) paints every sprite under the brush, control art and window background alike, which is what a human stroke does. Solo one, or choose any combination.",json!({"layers":{"type":"array","items":{"type":"string"}},"solo":{"type":"string"},"auto":{"type":"boolean"},"paint_layer":{"type":["string","null"]}}),&[]),
 tool("studio_rectangles","Where every sprite lives in the joined canvas -- the editor's Sprite rectangles panel. Read-only unless select is given, which clips painting to that rectangle. Rectangles are an overlay, never pixels in the artwork.",json!({"select":{"type":"string"}}),&[]),
 tool("studio_layers","Painting planes over the original atlases -- the editor's Painting layers panel. Add, select, rename, show, hide, lock, set opacity, clip to the plane below, move, merge down or delete. WSZ exports the composite; a project file keeps the planes.",json!({"action":{"enum":["list","add","select","set","move","merge_down","delete"]},"id":{"type":"string"},"name":{"type":"string"},"visible":{"type":"boolean"},"locked":{"type":"boolean"},"clip_below":{"type":"boolean"},"opacity":{"type":"integer","minimum":0,"maximum":255},"index":{"type":"integer","minimum":0}}),&[]),
 tool("studio_options","Everything about the skin that is not painted into a sheet -- the editor's Skin options panel. The time readout, the equalizer's slider travel, whether the playlist, equalizer sliders, playlist selection and visualizer carry their own artwork, the six PLEDIT.TXT text colours and the 24 VISCOLOR.TXT visualizer colours. Omit everything to read them all.",json!({"footer":{"enum":["classic","time-total"]},"eq_travel":{"type":"integer","minimum":1,"maximum":52},"visualizer_glass":{"type":"boolean"},"playlist_background":{"type":"boolean"},"eq_handles":{"type":"boolean"},"playlist_selection":{"type":"boolean"},"playlist_colors":{"type":"object","additionalProperties":{"type":"string"}},"visualizer_colors":{"type":"array","items":{"type":"string"},"minItems":24,"maxItems":24}}),&[]),
 tool("studio_status","Read the shared document, active state, layers and source rectangles.",json!({}),&[]),
 tool("studio_new","Create a transparent classic skin from scratch. No artwork or metadata is inherited. Unsaved edits require discard=true.",json!({"discard":{"type":"boolean"}}),&[]),
 tool("studio_open","Load a WSZ into the running native Studio. Existing unsaved edits require discard=true.",json!({"path":{"type":"string"},"discard":{"type":"boolean"}}),&["path"]),
 tool("studio_draw","Paint assembled panel pixels. One atomic undoable transaction. Optional layers targets every selected sprite beneath each pixel; omitted uses GUI selection. Optional label names the history entry. Pixel/line/rect/stamp operations map into their owning atlas; all_states maps the same local pixels into every pressed/frame variant. Coordinates are native pixels, never zoomed coordinates. Path points start [x,y], then line [x,y], quadratic [cx,cy,x,y], cubic [c1x,c1y,c2x,c2y,x,y], relative to x/y. Filled paths use native scanlines. brush_size controls solid stroke width; mirror_x/y reflect around canvas centre. Optional ramp colour array and ramp_axis [x1,y1,x2,y2] shade geometry with exact palette colours. Curve and tuft use endpoints x/y and x2/y2, curve_bend -100..100 or an explicit absolute control [x,y]. Tuft tapers brush_size to a pointed tip. Optional clean_corners removes redundant elbows from open 1px curves/paths only; preserves endpoints, fills and thick strokes. No antialiasing. Stamp rows contain palette-character pixel art, unmapped characters are skipped. Text draws a string in the editor's own 5x7 face at an integer scale, which is how a classic skin's labels are set. Image stamps a base64 PNG at x/y: fully transparent pixels are left alone and partly transparent ones blend with what they land on, so composed artwork -- gradients, glows, dithered shading -- arrives in one operation looking as it did. Optional origin names a layer and puts 0,0 on that sprite's current destination, which is the only safe way to aim at a sprite that moves with its own frame, and targets that layer unless layers says otherwise. Every result also reports overwrites: how often the transaction wrote one shared source cell twice with different colours, which is what silently repeats artwork across the four timer digits or the nine playlist tiles. Every result reports the bounds the ink actually landed in and how many pixels fell outside the chosen sprites.",json!({"mask_colors":{"type":"array","items":{"type":"string"}},"label":{"type":"string"},"origin":{"type":"string"},"layer":{"type":"string"},"layers":{"type":"array","items":{"type":"string"}},"all_states":{"type":"boolean"},"operations":{"type":"array","items":{"type":"object","properties":{"op":{"enum":["pixel","line","rect","ellipse","path","curve","tuft","stamp","cluster","text","image"]},"data":{"type":"string"},"text":{"type":"string"},"scale":{"type":"integer","minimum":1,"maximum":8},"spacing":{"type":"integer","minimum":-2,"maximum":8},"x":{"type":"integer"},"y":{"type":"integer"},"x2":{"type":"integer"},"y2":{"type":"integer"},"width":{"type":"integer"},"height":{"type":"integer"},"fill":{"type":"boolean"},"brush_size":{"type":"integer","minimum":1,"maximum":32},"curve_bend":{"type":"integer","minimum":-100,"maximum":100},"clean_corners":{"type":"boolean"},"mirror_x":{"type":"boolean"},"mirror_y":{"type":"boolean"},"control":{"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2},"points":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"ramp":{"type":"array","items":{"type":"string"}},"material":{"enum":["glass"]},"bevel":{"type":"number","minimum":1,"maximum":128},"refraction":{"type":"number","minimum":0,"maximum":32},"ramp_axis":{"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4},"color":{"type":"string"},"rows":{"type":"array","items":{"type":"string"}},"palette":{"type":"object","additionalProperties":{"type":"string"}}},"required":["op"]}}}),&["operations"]),
 tool("studio_project","Save or open a layered .cstudio project. WSZ remains the flattened skin export. Opening unsaved work requires discard=true.",json!({"action":{"enum":["save","open"]},"path":{"type":"string"},"discard":{"type":"boolean"}}),&["action","path"]),
 tool("studio_cluster","Pick up a native pixel region from selected layers (Auto captures visible canvas). No skin mutation. Omit rect to read clipboard as reusable stamp rows/palette. Optional flip_x, flip_y and quarter_turns transform the clipboard losslessly. Paint it using studio_draw op cluster, or human Stamp brush. Clipboard is an editor tool, never added to WSZ.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"flip_x":{"type":"boolean"},"flip_y":{"type":"boolean"},"quarter_turns":{"type":"integer","minimum":0,"maximum":3}}),&[]),
 tool("studio_study","Read-only material study board: native crop above integer enlarged detail, optional grayscale value preview, registered layer geometry, pixel grid, and reference image alongside. Does not edit skin or change view. rect defaults to last lifted region. Reference pixels are never imported into the skin.",json!({"rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"zoom":{"type":"integer","minimum":1,"maximum":8},"selected":{"type":"boolean"},"values":{"type":"boolean"},"grid":{"type":"boolean"},"geometry":{"type":"boolean"},"reference":{"type":"string"},"reference_rect":{"type":"array","items":{"type":"integer","minimum":0},"minItems":4,"maxItems":4},"path":{"type":"string"}}),&[]),
 tool("studio_pixel","Inspect a canvas pixel, all underlying sprites, atlas coordinates and sharing.",json!({"x":{"type":"integer","minimum":0},"y":{"type":"integer","minimum":0}}),&["x","y"]),
 tool("studio_states","Return every source variant of the selected layer as a nearest-neighbor contact sheet. Inspect all 28 track frames or both pressed/released sprites side by side.",json!({"zoom":{"type":"integer","minimum":1,"maximum":8},"path":{"type":"string"}}),&[]),
 tool("studio_history","List the shared human/MCP history, current cursor and retained steps.",json!({}),&[]),
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
        "studio_guides" | "studio_rectangles" => {
            if let Some(id) = args["select"].as_str() {
                doc.select_guide(id)?
            } else {
                json!({"guides":doc.guides()})
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
        // The panels a human works in, each answering for itself. The older
        // tools below still work; they are simply no longer offered.
        "studio_canvas" | "studio_atlas" => {
            let mut view = args.clone();
            if name == "studio_atlas" {
                match args.get("sheet").and_then(Value::as_str) {
                    Some(sheet) => {
                        view["panel"] = json!("atlas");
                        view["sheet"] = json!(sheet);
                        view["layer"] = json!("sheet");
                    }
                    None => {
                        view["panel"] = json!("canvas");
                        view["layer"] = json!("auto");
                    }
                }
            } else {
                view["panel"] = json!("canvas");
            }
            let path = view.as_object_mut().and_then(|v| v.remove("path"));
            let crop = view.as_object_mut().and_then(|v| v.remove("crop"));
            let zoom_out = view.as_object_mut().and_then(|v| v.remove("zoom"));
            if let Some(zoom) = zoom_out.clone() {
                view["zoom"] = zoom;
            }
            let state = doc.state(view)?;
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
                doc.state(json!({ "layers": [solo] }))?
            } else if args.get("layers").is_some() || args.get("paint_layer").is_some() {
                doc.state(args.clone())?
            } else {
                json!({
                    "sprites": doc.layers().into_iter().map(|l| json!({
                        "id": l.id,
                        "sheet": l.sheet,
                        "source": l.source,
                        "states": l.variants.len(),
                    })).collect::<Vec<_>>(),
                    "chosen": doc.view.layers,
                })
            }
        }
        "studio_options" => {
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
            // set_layout rejects unknown fields, so hand it only its own.
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
            let (playlist, visualizer) = doc.text_palettes();
            let sheets = doc.sheets();
            json!({
                "layout": doc.layout(),
                "playlist_background": doc.has_playlist_background(),
                "eq_handles": sheets.iter().any(|(n, _, _)| n == "eqhandles.bmp"),
                "playlist_selection": sheets.iter().any(|(n, _, _)| n == "plselection.bmp"),
                "playlist_colors": playlist
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), json!(v)))
                    .collect::<serde_json::Map<String, Value>>(),
                "visualizer_colors": visualizer,
            })
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
                doc.state_sheet()?
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
/// Where a file actually landed. A relative path is resolved against the
/// Studio process's directory, not the caller's, so echoing back what was asked
/// for is how a render ends up somewhere nobody looks.
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

    /// An agent should work the way a person does: one tool per panel the
    /// editor actually has. The older surface addressed single windows and an
    /// isolated patch handoff -- ways of editing this editor no longer offers.
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

    /// Retiring a tool from the offered list must not break the scripts that
    /// already call it.
    #[test]
    fn the_retired_tools_still_answer() {
        let shared = document();
        for retired in ["studio_state", "studio_layout", "studio_palette"] {
            call(retired, json!({}), &shared)
                .unwrap_or_else(|e| panic!("{retired} should still answer: {e:#}"));
        }
    }

    /// Everything a skin carries outside its bitmaps, in the one tool that
    /// matches the panel a human uses for it.
    #[test]
    fn studio_options_reads_and_writes_everything_outside_the_sheets() {
        let shared = document();
        let before = call("studio_options", json!({}), &shared).unwrap();
        let before: Value =
            serde_json::from_str(before["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(before["layout"]["footer"], "classic");
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
        assert_eq!(after["layout"]["footer"], "time-total");
        assert_eq!(after["layout"]["eq_travel"], 40);
        assert_eq!(after["playlist_colors"]["Normal"], "#010203");
    }

    /// The canvas is the drawing surface, and asking for it puts the editor on
    /// it: an agent never has to know the per-window panels exist.
    #[test]
    fn studio_canvas_puts_the_editor_on_the_whole_skin() {
        let shared = SharedDocument(Arc::new(Mutex::new(Document::blank())));
        call("studio_atlas", json!({"sheet": "main.bmp"}), &shared).unwrap();
        assert_eq!(shared.lock().unwrap().view.panel, "atlas");

        call("studio_canvas", json!({"zoom": 3}), &shared).unwrap();
        let view = shared.lock().unwrap().view.clone();
        assert_eq!(view.panel, "canvas");
        assert_eq!(view.zoom, 3);

        // And the sprite targets panel answers for what a stroke would hit.
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

    /// Setting a label used to be one hand-built stamp per letter. The editor
    /// already carries a 5x7 face for its own rectangle numbers; drawing with
    /// it is both faster and correctly spaced.
    #[test]
    fn text_draws_a_label_in_the_editors_own_face() {
        let shared = blank();
        let out = result(
            &shared,
            "studio_draw",
            json!({"operations":[{"op":"text","x":20,"y":30,"text":"CAT","color":"#f5c06a"}]}),
        );
        assert!(out["pixels_written"].as_u64().unwrap() > 20);
        // Three glyphs, five wide with one of spacing: x 20..=36, seven tall.
        assert_eq!(out["bounds"][0], 20);
        assert_eq!(out["bounds"][2], 36);
        assert_eq!(out["bounds"][3], 36);
    }

    /// A sprite that moves -- a slider thumb follows its own frame -- has no
    /// fixed canvas address, so art aimed at one read in another state lands at
    /// an offset and quietly writes a second copy. `origin` removes the
    /// question by putting 0,0 on the sprite.
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
        // Same cell, read back through a different frame: the thumb has moved,
        // and the ink moved with it because it went into the sprite, not a spot.
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

    /// The four timer digits are one cell of numbers.bmp, so a stroke across
    /// them writes the same pixels twice and the last colour wins in all four
    /// positions. It looked like a clean write, and the artwork came out
    /// repeated.
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
        // Naming one target is the cure, and then there is nothing to report.
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

    /// A sheet has no alpha channel, so a half transparent pixel either blends
    /// with what it lands on or arrives at full strength. It used to arrive at
    /// full strength, which turned every soft glow into hard speckle.
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

    /// Both figures describe the transaction in hand. They were session
    /// cumulative at first, which made the second draw onward report the union
    /// of everything before it -- exactly the reassurance they were added to
    /// give, and exactly wrong.
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

    /// Artwork with gradients, dithering and hand-placed noise is composed far
    /// faster outside the editor than one drawing operation at a time; the image
    /// op is the door back in, and transparent pixels have to stay untouched or
    /// every stamp would punch a rectangle through whatever it lands on.
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

    /// A stroke aimed at named sprites used to lose whatever fell outside them
    /// without a word, so a label one pixel too wide simply came out clipped.
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

    /// Checking one sprite should not cost a render of the whole skin.
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
}
