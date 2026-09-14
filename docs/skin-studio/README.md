# Cranamp Skin Studio

Open **Settings → Open Skin Studio** on desktop or Android. On desktop the editor
opens in a separate native window. Android opens the editor inside the activity.
Playback continues in both cases. It edits the selected skin;
with the bundled default selected, it opens Catamp Silverplay with seven painting
layers and no output path. Choose an export destination to save your copy.
Add the exported `.wsz` through the player's Settings to apply it.

```sh
cargo run -- --skin-studio
cargo run -- --skin-studio '/absolute/path/skin.wsz'
cargo run -- --skin-studio '/absolute/path/project.cstudio'
```

Catamp Silverplay uses the original thirteen Winamp bitmap sheets plus
`pledit.txt` and `viscolor.txt`. The player and editor embed the same artwork.
Studio uses Cranpose for the GUI; mouse strokes and MCP commands modify the same
document and share undo/redo history. The editor is available on desktop and
Android; the bundled skin is also available on iOS and web.

The whole-canvas drawing helper is documented in
[connected-canvas-api.md](../../tools/skin-studio/connected-canvas-api.md).
Python review helpers require Pillow; the MCP client and drawing helpers use the
standard library. Run helper unit tests with:

```sh
python3 -m unittest discover -s tools/skin-studio -p 'test_*.py'
```

## Android controls

- **Draw / Pan** switches a drag between painting and moving the canvas.
- **−/+ zoom** changes the integer number of screen pixels per skin pixel.
- **Layers** selects, adds, locks or hides painting planes. **Parts** selects any
  combination of sprite targets, shows their rectangles, and enables all-state edits.
- **Tools** selects brushes, width, fill, preset colors or an exact HEX color.
- **States** previews pressed/inactive controls and all slider frames, and lists history.
- **Files** opens WSZ/CSTUDIO files through Android's picker, exports either format,
  saves/restores a layered draft, or applies the edited skin to the player.
- **Player** or Android Back saves a recoverable draft and returns to playback.
  Documents and undo history remain alive when switching between player and Studio.

MCP uses the same active document on Android. For a USB/emulator connection:
`adb -s DEVICE_SERIAL forward tcp:18766 tcp:18765`, then POST MCP requests to
`http://127.0.0.1:18766/mcp`. The server binds only to device loopback. GPU screenshot
capture is currently provided by the desktop preview; Android supports the native
editor image, atlas inspection and drawing commands.

## Reproduce phone scaling on desktop

```sh
cargo run -- --touch-preview
cargo run -- --touch-preview --studio
```

The handset-sized preview uses the real stacked player, Settings and touch editor.
Check it as well as the integer-zoom desktop Studio: physical sprite edges must be
snapped from their absolute native coordinates, with widths derived from the snapped
endpoints. Rounding positions and widths independently leaves gaps at fractional
phone scales. The stacked panels share the same pixel-grid origin.

## Sprite rectangles and painting layers

**Part rectangles** marks each source cell directly on the editor image and lists
its sheet, native rectangle and state ID. Cyan outlines are sprite cells; gold
identifies the selected paint clip; pink marks runtime text/graph/spectrum areas.
Numbers on the canvas match the list. Select a cell to clip atlas painting to it.
In the assembled view, selecting a sprite targets that part. **Clear paint clip**
restores unrestricted painting. Guides never enter preview artwork or WSZ export.

MCP: `studio_guides` lists the rectangles; `{"select":"main.play#1"}` selects
a specific atlas cell. `studio_state` supports `guides` and nullable `clip`.
For example, the pressed play cell is `cbuttons.bmp [23,18,23,18]`; volume has
28 separate track-frame rectangles. Runtime reservations remain visible so
illustrations can be placed around the player-drawn content.

`studio_inspect_region {"rect":[48,26,9,13]}` maps a native review crop to
intersecting parts, source overlaps for the current sprite states, and live
reservations (whose source overlap is null). It includes underlying parts;
it is not a visible-pixel mask and does not mutate the document or view.
In the GUI, lift a region and select **Part rectangles → Parts in lifted
region** to filter the list. **Show all parts** restores it. Pink guides now
include the four exact timer digit cells as well as other live readouts.

**Painting layers** are independent artwork planes, separate from sprite targets.
Select, name, show/hide, lock, reorder, change opacity, merge down, or delete a
plane. The original atlas base stays underneath. Layer actions and painting use
the same Human/MCP undo history. One opacity drag creates one history step.

Sprite targeting still accepts any combination: for example, paint into a
“Highlights” plane while targeting main background and title together, or apply
one stroke to every pressed/unpressed source variant of selected controls.
Alpha lock and color masks apply to pixels already in the active painting plane.

MCP `studio_paint_layers` supports `list`, `add`, `select`, `set`, `move`,
`merge_down`, and `delete`. `studio_state` selects `paint_layer` by ID (`null`
selects the base). Layers are bottom-to-top, with move index zero above the base.
Opacity is 0–255. Locked planes reject drawing. Hidden planes do not export.

**Clip to layer below** constrains a shading plane to the effective alpha of
its immediately lower painting plane. Hidden or transparent pixels below hide
the clipped paint, and opacity participates in the mask. The source pixels are
retained: switching clipping off reveals them again. Clipped chains are supported.
For the bottom painting plane the original atlas provides the mask. Reordering
or deleting layers changes which plane supplies the mask.

Use `studio_paint_layers {"action":"set","id":"paint-6","clip_below":true}`
after placing the shading plane directly above the desired silhouette. The GUI
switch, MCP, undo and project saves share this property. Merging a clipped plane
bakes its effective pixels into a visible, unlocked, fully opaque, unclipped
lower plane. Merge clipped followers first; Studio rejects a merge that would
change a follower's mask. Old projects default to clipping off.

**Save project** writes a layered `.cstudio` ZIP with the original base WSZ,
lossless layer PNGs, and layer/view metadata. **Open** and `--skin-studio` accept
both WSZ and CSTUDIO files. Export writes a standard flattened WSZ and, when
painting layers exist, automatically saves an adjacent editable `.cstudio`.
MCP `studio_project` supports `save` and `open`. Undo history is session-local;
project files preserve layers and current artwork, not historical checkpoints.

## Whole-skin canvas — original definitions

**Whole skin** stitches the main player, equalizer and playlist into one native
editing canvas. It does not add a bitmap, change layout metadata, move controls,
or alter the exported skin format. Main remains at y=0, EQ at y=116, and the
playlist at y=232. The original one-pixel main-window reservation remains.

MCP: `studio_state` with `panel: "canvas"`. Layer IDs are qualified, for example
`main.background`, `equalizer.background`, `playlist.bottom.right`. Select any
combination and draw across panel boundaries in one undoable transaction.
`preview_playlist_height` determines the assembled playlist height (145–522).
Repeated playlist borders are tiled and cropped at native size; painting a shared
tile necessarily affects each repetition. The inspector exposes those mappings.

The Python `CanvasPen` helper rejects an ownership rectangle that includes shared
playlist borders by default (`repeats='error'`). Previously it silently produced
only the non-repeated parts of a patch. Use `repeats='shared'` to edit one tile
copy and every alias, or explicitly use `repeats='skip'` when another operation
owns those borders. Multiple overlapping aliases remain rejected. The GUI and
direct `studio_draw` Auto target paint the visible source at each point, including
shared tiles. Selected parts and masks deliberately restrict that coverage.

Classic playlist rails repeat every 29 rows. The footer follows the final cropped
tile at any playlist height, so an authored rail-to-footer connection must match
every tile phase. The list's solid fill and live text have no static bitmap source.
The main docking row 115 aliases source row 114; they cannot hold different pixels.

Run `cargo test --lib join_tests` to compare painting with one unsplit bitmap:
600 MCP cases cover pixels, lines, filled rectangles, ellipses, paths, curves,
tufts, stamps, palette ramps and glass at panel/title/tile/footer joins, both
borders, and five playlist heights. GUI preview, lifted stamps, cancellation,
selected parts, masks, mirrors, layers, undo/redo and WSZ reload have additional
checks. These tests verify pixel routing; they do not judge whether separate
authored highlights meet visually.

A temporary free-placement renderer was explored and rejected because it changed
the skin definitions. It is not part of the supported skin format.

## Starting from blank and editing atlases

**New blank** creates thirteen transparent classic atlases with no inherited
artwork or metadata. Export or undo dirty work before replacing the document.
MCP exposes the same operation as `studio_new` (`discard: true` explicitly
replaces unsaved work).

**Atlases** opens a sheet chooser. The entire selected bitmap becomes the native
pencil canvas, with the same palette, integer zoom, picker and shared history as
the assembled panel. MCP: `studio_state` with `panel: "atlas"`, `sheet: "text.bmp"`
and `layer: "sheet"`. This also supports painting the previously unreachable
font, status and metadata-associated art sheets.

**Unique EQ art** (Equalizer tab) enables eleven independent 14×25 handles in
`eqhandles.bmp` (154×50: eleven columns, normal above pressed). It reserves 25
pixels of the 63-pixel track and limits travel to 38. Each `bandN.thumb` can be
selected, painted, grouped, and undone independently. MCP: `studio_eq_handles`.
Disabling this returns to the classic shared 11×11 head mapping. The feature button sits above the canvas.

**Glass visualizer** (Main player tab) makes unlit visualizer pixels transparent,
revealing the artist's native background while retaining the live spectrum.
MCP: `studio_layout` with `visualizer_glass: true`.

**Selection artwork** (Playlist tab) enables a native 243×11 `plselection.bmp`.
Edit `list.selection` on the assembled canvas or its atlas. The runtime reserves
an eight-pixel marker gutter before track titles, keeps durations aligned, and
uses the artwork for each selected row. Wider windows crop the native art at
243 pixels and retain the selection palette beyond it; pixels are not stretched.
MCP: `studio_playlist_selection`.

All these operations share human/MCP history and survive WSZ export.

## Drawing

1. Choose Main player, Equalizer, or Playlist.
2. Choose Pencil or Pick pixel, and a swatch or `#RRGGBB` brush color.
3. Draw directly on the assembled canvas at integer zoom. The editor resolves
   each stroke to the underlying BMP and source rectangle.
4. Auto targets the topmost sprite footprint. **Select layers…** opens a native multi-select list. Toggle any combination
   of backgrounds and controls, including occluded pixels. **Solo** selects just
   one layer; **Select all** includes every layer; **Auto / clear** returns to
   topmost targeting. The pencil writes to every selected footprint it crosses.
5. **Current state only** edits the displayed source region. **All sprite
   states** writes the same local pixels into every variant of that control.
   This covers normal/pressed and on/off controls, and all 28 slider frames.
6. Switch Released/Pressed and On/Off. Scrub frames 0–27 for volume, balance,
   seek, equalizer or playlist scroll. **States** shows the selected layer's
   source variants side by side; Auto uses the panel's primary slider.
7. **Player preview** runs the actual Cranamp main/EQ/playlist composables
   against the current document. Integer zoom and pan work here too. Use 1×
   for the whole stack. **Tall playlist / Compact playlist** switches the live preview between 261 and 145 native pixels. MCP `studio_state` accepts `preview_playlist_height` from 145 to 522. **Presentation** shows the entire live stack at the largest fitting integer zoom (up to 2×). MCP can toggle it with `presentation: true/false`. Return to Edit canvas to paint.
8. **List canvas** enables an editable `list.background` layer. It exports a
   native 243×203 `plbg.bmp`, cropped/tiled by the player without stretching.
   **Travel** on the Equalizer tab reserves space for larger fader artwork;
   changes share undo history. The default is 52 pixels; Moonpool uses 38.
9. Enter a destination WSZ path and choose Export. Exports are validated by
   Cranamp's skin loader and written atomically. The input is not changed
   unless you explicitly export to the same path.

Magenta is Winamp's transparent color key. The MCP brush also accepts
`transparent`. **History** lists human and MCP edits; click any step to restore it. Undo/redo
retains 32 transactions during the current session. A new edit after undo replaces
the redo branch. Export marks the saved state, which undo/redo can return to. Shared classic atlas regions necessarily change every place they are used.
Enable Unique EQ art to separate the eleven heads.
Playlist repeat/stretch tiles likewise cannot have independent pixels at each
repeated screen location; the inspector identifies those mappings.

`studio_screenshot` waits for the requested document revision to be composed,
including the live player’s atlas reload, before capturing the GPU scene. Clients
do not need arbitrary sleeps after `studio_state` or painting.

The editing canvas assembles skin bitmaps, without runtime-generated text or
visualizers. Player preview includes Cranamp's runtime controls. The Pressed and On/Off
selectors also affect its artwork: preview-only copies substitute the selected
sprite variants without changing the document or export. Real pointer input
remains available. EQ faces use the same discrete frame as their illustrated
bodies, including during continuous drags.

## MCP

Start the native studio, then connect an MCP client with the stdio relay:

```json
{
  "mcpServers": {
    "cranamp-skin-studio": {
      "command": "/absolute/path/to/cranamp",
      "args": ["--skin-studio-mcp"]
    }
  }
}
```

The running studio also exposes JSON-RPC at
`http://127.0.0.1:18765/mcp`. It binds only to loopback, rejects browser Origin
headers, and accepts at most 8 MiB per request. Run one studio instance for
this endpoint. The stdio relay connects to that same native window; it does
not create a separate document.

Tools:

- `studio_status`, `studio_pixel`: document state and bidirectional pixel mapping.
- `studio_state`: panel, layer, color, zoom, frame positions, pressed/on state.
- `studio_draw`: atomic pixel, line, rectangle, and pixel-stamp transactions.
- `studio_render`, `studio_states`: PNG previews with nearest-neighbor zoom.
- `studio_screenshot`: actual GPU scene capture at native logical pixels; optional `crop: [x, y, width, height]` and PNG `path`. Use this to review runtime text and controls without OS screenshot thumbnail scaling.
- `studio_playlist_background`: enable/read the optional painted playlist canvas.
- `studio_layout`: read/patch footer layout and native EQ travel (1–52 pixels).
- `studio_palette`, `studio_recolor`: exact palette substitutions without resizing.
- `studio_set_palette`, `studio_visualizer_palette`: playlist and runtime visualizer colors.
- `studio_history`, `studio_history_goto` (cursor), `studio_undo`, `studio_redo`: shared history.
- `studio_open`, `studio_export`: import and validated WSZ export.

For direct development calls, use the thin client (Node 18+):

```sh
node tools/skin-studio/client.mjs studio_state \
  '{"panel":"main","layer":"play","all_states":true}'
node tools/skin-studio/client.mjs studio_draw \
  '{"operations":[{"op":"line","x":41,"y":90,"x2":51,"y2":90,"color":"#d5f2fa"}]}'
node tools/skin-studio/client.mjs studio_states \
  '{"zoom":4,"path":"/tmp/play-button-states.png"}'
node tools/skin-studio/client.mjs studio_undo
```

Use `"layers":["background","play"]` in `studio_state` to select multiple layers
for both mouse and MCP strokes. `studio_draw` also accepts a temporary `layers`
override and a `label` for its history entry. Omitted targets use the GUI selection;
`[]` means Auto. `all_states` applies independently to each selected layer. The
legacy singular `layer` argument still selects one layer. Changing panels resets
selection unless the command supplies a new one.

Coordinates always refer to **native panel pixels**, never enlarged GUI pixels.
The play-button example maps to `cbuttons.bmp` at x=25..35, y=2 and y=20.
Use `studio_pixel` to inspect ownership before painting.

Pixel stamps accept `rows` plus a single-character palette. Unmapped characters
are skipped; mapped characters become exact pixels. This supports hand-drawn
cats, lettering, bevels and other motifs without raster scaling or imagegen.

## Implementation and verification

- `src/winamp/studio/mapping.rs`: source/destination mappings, using Cranamp's
  sprite constants; state variants and shared-tile metadata.
- `model.rs`: editable bitmaps, transactional drawing, history, composition,
  palette edits, and WSZ serialization.
- `mod.rs`: native Cranpose UI and production-player preview.
- `mcp.rs`: MCP schemas, JSON-RPC endpoint, and stdio relay.

Tests check drawing into the correct source pixels, both pressed variants,
all 28 slider frames, every state mapping's bounds, atomic error rollback,
shared human/MCP undo, and round trips through the production skin loader.

The Layers and History drawers pause canvas drawing while open. Empty mouse
gestures do not create history entries or discard redo. One completed drag is
one history entry. History is in memory; exported WSZ files contain skin assets,
not editing history.

## Actual-player verification

Player preview uses Cranamp's bundled demo playlist when available, without
starting playback. The preview renders production player text, curves and
controls. State substitutions are temporary and never mutate the exported art.

```sh
python3 tools/skin-studio/check_native_frames.py --output target/catamp-silverplay-frames
python3 tools/skin-studio/check_silverplay_art.py
```

The first command captures all 112 actual GPU states and checks native sampling.
The second compares the complete EQ, volume, balance, seek and scrollbar regions
against the current Silverplay WSZ, including transparent source pixels and all
28 positions. Both are read-only. The capture sweep restores the previous view.

## Pixel craft tools and material studies

**Brush tools** now includes native pencil, line, rectangle and ellipse brushes,
solid widths, filled shapes and reflection about the canvas axes. Shapes preview
while dragging and commit one shared Human history transaction when released.
Cancel restores the original pixels. MCP paths support line, quadratic and cubic
segments rasterized to solid native pixels; palette ramps select exact colors.

**Lift pixels** drags a rectangular pickup region. It captures only the selected
layers with their transparency, or the visible composition in Auto. Picking up
pixels does not modify the skin or add an undo step. **Stamp** places the clipboard
at native size into any selected layer combination and optionally every state.
Dragging previews the placement; release records one undoable transaction.
The **Study** drawer shows the clipboard at 1× and an integer enlargement, with
lossless horizontal/vertical flips and quarter-turn rotations. The pixel grid is
an editor-only overlay visible at 4× and higher. It never enters exported images.

MCP uses the same clipboard:

```json
{"name":"studio_cluster","arguments":{"rect":[10,12,86,55]}}
{"name":"studio_cluster","arguments":{"flip_x":true,"quarter_turns":1}}
{"name":"studio_draw","arguments":{"label":"Place drawn detail","operations":[{"op":"cluster","x":110,"y":15}]}}
```

`studio_cluster` returns reusable palette-character stamp rows as well as storing
an in-memory clipboard. Save those rows in an artwork journal to reproduce a
pickup across sessions. Clipboard, selection and study references are editor
state, not additional WSZ assets or modified skin definitions.

`studio_study` produces a read-only board: the crop at native size above an exact
integer enlargement. Optional `reference` and `reference_rect` place a reference
beside it at its own native resolution, without importing its pixels into the
skin. Optional `geometry: true` marks the registered sprite footprints; `grid:
true` adds a display-only grid to the enlarged copies. Neither changes the view,
history, archive or art. Example:

```json
{"rect":[5,8,98,66],"zoom":6,"selected":true,"grid":false,"path":"/tmp/cat-study.png"}
```

Earlier material studies were drawing exercises, not complete skins. Native
pixel checks prove sampling and state coverage, not illustration quality.

**Lock transparent pixels** protects empty source pixels while shading. **Mask
picked color** locks painting to the current brush color; choose the replacement
color afterward. **Clear color mask** restores unrestricted color painting. These
protections apply separately to each selected source variant, so all-state shading
cannot accidentally fill transparent space in another frame. MCP uses
`studio_state` fields `alpha_lock` and `mask_colors`; `studio_draw` also accepts a
temporary `mask_colors` array without replacing the GUI's persistent mask.


**Glass lens** is a native material brush. Human artists drag an elliptical lens;
MCP can shade an arbitrary hand-drawn filled path, rectangle or ellipse by adding
`material: "glass"`, `bevel` (1–128 native pixels) and `refraction` (0–32 pixels)
to a drawing operation. The brush color supplies the glass tint. Surface normals
come from the original construction curves rather than the stepped raster
silhouette, preventing the repeated edge glints of a pixel-distance bevel.
Refraction samples the selected underlay at integer native coordinates. Neither
underlay pixels nor output colors are blurred, resampled or alpha-filtered.

This is **baked artwork**, not a new runtime material or skin definition. It writes
ordinary BMP colors through the same selected-layer/all-state mapping and shared
undo history. Draw and inspect each state when its underlying artwork differs;
a single all-state stroke intentionally copies the same local shading to each
variant. Studio never adds material metadata to the WSZ.

## Tapered native drawing pen

`tools/skin-studio/pixel_pen.py` provides `Pen.taper(start, control, end, width,
color)`: a pointed quadratic brush mark for fur locks and fine glass facets. It
constructs a closed native path for Studio MCP; Studio rasterizes it once with
solid pixels. Width controls the root, and the end remains a single pointed tip.
This is used directly in the Silverplay refinement. The normal pen remains
available for constant-width lines, Bézier outlines, exact palette ramps and
native stamps.

## Native curve and fur brushes

The GUI brush palette now includes **Curve** and **Fur / taper**. Drag from the
root to the tip; choose a 1–16 pixel width and adjust **Bend** from −100% to
100%. Live previews and mouse release use the same native rasterizer and one
shared undo transaction as MCP. A tapered stroke narrows to a single-pixel tip.

MCP `studio_state` accepts `brush: "curve"` / `"tuft"` and `curve_bend`. Drawing
operations use `op: "curve"` / `"tuft"`, endpoints `x,y,x2,y2`, and either
`curve_bend` or an explicit absolute quadratic `control: [x,y]`. Palette ramps,
selected sprite targets, painting planes, mirrors and masks remain available.

The artist canvas now uses required native dimensions and clips at the viewport
boundary. Previously, a tall whole-skin canvas could be squeezed vertically by
the GUI's layout constraints even though the actual player was correct. The
image and pointer surface now retain identical scale. `check_editor_pixels.py`
compares the actual editor GPU pixels to native artwork at 1×, 2×, 3×, 4×, 6×
and 8×. Run it with the art editor open, drawers closed and pan at the origin.

### Bounded hand-authored sprites

`tools/skin-studio/pixel_pen.py` includes `Pen.sprite_cell(x, y, width, height,
rows, palette, dx=0, dy=0)`. The geometry helper checks source-cell dimensions,
undefined palette symbols, and pixels lost by pressed-state translation before
emitting any Studio drawing operations. It clears the exact cell to the classic
transparency key and stamps the supplied native pixel clusters. Use atlas mode
and keyed sprites. Explicit per-state palettes provide material changes without
scaling; each committed batch remains in the shared drawing history.

### Native playlist borders

The player repeats the 25×20 header tile and 12×29 / 20×29 side tiles at their
original size, cropping only a partial final tile. These must use `TiledSprite`,
not `StretchSprite`: nearest-neighbor sampling alone does not preserve sprite
geometry when a source cell is enlarged to fill an entire border. The original
WSZ rectangles stay unchanged. `check_silverplay_trim.py` compares actual GPU
pixels to the title and repeat-cell definitions in four player states.

### Capturing hidden Studio windows

The MCP screenshot endpoint uses offscreen GPU snapshots to drive composition
until the requested document revision is ready, then captures a fresh frame.
It does not wait for a visible surface presentation. `Robot::pump_frames` has
that requirement and must not be used in this path: a hidden macOS window cannot
satisfy it. The revision guard remains essential for fresh state captures.

This path was exercised after hiding Studio through the native UI, comparing
all transport cells and title/border pixels across released/pressed and active/
inactive previews. `check_silverplay_transport.py` checks the six transport cells,
including transparency over the underlying skin; `check_silverplay_trim.py`
checks exact title and border source mappings.

### Color and value studies

The Study drawer offers Color and Values views. MCP `studio_study` accepts
`values: true` for an inspection-only grayscale board, including its reference.
The native pixels and alpha stay unchanged; the operation never edits a plane
or export. The preview uses a weighted grayscale value (54R + 183G + 19B)/256.

Both GUI previews use required native dimensions with viewport clipping, so a
tall selection is not squeezed to fit a shorter display area. The canvas and
selected-sprite studies also interpret newly painted magenta as the classic
transparency key, matching the running player. Atlas editing keeps the raw key.

Verification included an 80×115 lifted region: native clipped and 2× enlarged
GUI pixels matched the source in both Color and Values modes. The EQ switch
check separately compares keyed corners in the artist canvas, exported atlas,
and actual player across all four on/off and pressed/released combinations.

### Thin contour cleanup

Brush tools → **Clean 1px corners** is shared with MCP `studio_state` and
`studio_draw` through `clean_corners`. Open one-pixel paths and curves remove
redundant right-angle elbows while retaining endpoints and connectivity.
Filled/closed shapes, ellipses and broad strokes are unchanged. An individual
operation can override the shared setting. This is optional native geometry
cleanup, not antialiasing or a resampling filter.

### Regional drawing handoffs

`studio_patch` inspects native source-rectangle baseline tokens, validates drawing
operations inside each owned rectangle, and applies them on one new painting
plane with one shared undo entry. Application is staged independently of the GUI
view and rejects changed baseline pixels. Two artists may prepare disjoint parts
in parallel; their coordinator submits patches sequentially and visually checks
the assembled player. `tools/skin-studio/regional_patch.py --help` exposes the
inspect/prepare/validate/apply workflow. Ordinary GUI layer editing and history
remain available for imported art.

Regional patches can also revise an existing artist plane in place. Inspect with
`replace_layer` to capture its raw-layer token plus current source-region tokens,
then validate/apply a complete replacement using `layer_expected`. The plane's
ID, order and the GUI view stay stable; other artists' planes remain untouched.
The native tool checks complete ownership and rejects changed or locked/clipped
planes. The whole revision is one undo step, so repeated polishing no longer
requires backing out later artists' work.
