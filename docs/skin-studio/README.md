# Cranamp Skin Studio

Open **Settings → Open Skin Studio** on desktop, Android or the web. On desktop
the editor opens in a separate native window. Android and the browser open the
editor inside the running player; in the browser that is the whole page, because
the canvas is the viewport and the player floats on it. Playback continues in
every case. It edits the selected skin;
with the bundled default selected, it opens Catamp Silverplay with seven painting
layers and no output path. Choose an export destination to save your copy.
Add the exported `.wsz` through the player's Settings to apply it.

```sh
cargo run -- --skin-studio
cargo run -- --skin-studio '/absolute/path/skin.wsz'
cargo run -- --skin-studio '/absolute/path/project.cstudio'
```

## Getting around

cranpose delivers key events only to a focused text field, so the editor has no
keyboard shortcuts -- no Cmd+Z, no bracket keys, no space-to-pan. Every way of
moving around the canvas is a gesture the pointer alone can make:

| Gesture | Effect |
| --- | --- |
| Wheel | Scrolls the canvas |
| Alt + wheel | Scrolls it sideways |
| Ctrl + wheel | Zooms a whole step, keeping the pixel under the pointer in place |
| Middle-button drag | Pans |
| Right-click on the canvas | Samples the colour under the pointer, without leaving the pencil |
| Scrollbars on the canvas' own edges | Scroll, and jump when clicked |

The line above the canvas names the pixel under the pointer, and an outline the
width of the current stroke marks where it would land.

## Reading the editor

Every control belongs to one of five classes, and each class has a shape of its
own, so a button says what kind of thing it does before it is pressed:

| Shape | Class | Example |
| --- | --- | --- |
| Plain slab | Runs once | **Undo**, **Export**, **Fit whole skin** |
| Red slab | Runs once and can lose work | **New blank**, **Delete** |
| Slab with a rule under the label | Opens the panel named on it | **Drawing tools**, **Skin options** |
| Pill | One value out of a set | `2x`, **volume**, **Pencil** |
| Slab with a pip on the right | On/off switch | **Current state only**, **Fill shapes** |

Every switch names the state the editor is in **now**, never the state that
pressing it would move to, and the pip repeats it. Every drawer button is named
exactly as the panel it opens.

Two buttons can destroy work, and both ask first. **New blank** pressed with
edits outstanding refuses, says so, and changes to **Discard & start blank** with
**Keep editing** beside it. **Delete** in **Painting layers** changes to **Discard
it**, with **Keep the layer** beside it. The second press is the one that acts.
Failures and refusals are coloured in the status line, so a refusal does not read
as a report.

**Undo** and **Redo** count the steps they have (`Undo 3`) and are drawn as
unavailable when they have none.

## Drawing

The editor opens on the **whole skin**: main, equalizer and playlist joined into
one canvas, and that is the only drawing surface -- there is no per-window view
to switch into. A stroke can run across a window boundary and each pixel is still
routed to whichever BMP sheet owns it.

The canvas takes the room the skin needs and the tool column takes the rest, so
opening a panel never shrinks the canvas and no panel is ever laid over it. The
editor's window opens at the size where the whole skin fits at 2x with the column
beside it; **Fit whole skin** picks the largest whole-pixel zoom that still fits
whatever the window is now.

There is no separate canvas to open and no shape to commit: **drag on the skin
itself**, with the brush, colour and width shown in the left sidebar. Each
completed stroke is one undo step named in the status line at the bottom.

A layered project composites its painting planes over the base atlases, so the
editor selects the topmost visible plane when it opens one. Strokes then land on
top of the picture. Choosing **Base atlases** in **Painting layers** paints
underneath the planes instead, which is only visible where no plane covers the
artwork.

**Colour picker…** in the sidebar opens a saturation/value field with a hue strip
and the colours already most used in this skin; the hex box beside it still takes
an exact value.

Two colours behave specially: `#ff00ff` is the classic transparency key, so
painting with it erases rather than draws, and a stroke in a colour close to the
artwork under it can be invisible even though it landed. The status line names
every stroke and counts the pixels it changed, which is the reliable signal.

With no explicit target, a stroke paints **every sprite the brush covers**, not
just the one drawn on top -- a control and the window background beneath it are
both artwork the stroke crosses, and painting only the top one breaks the
illustration apart as soon as that control changes state or is drawn elsewhere
from the same shared source. **SPRITE TARGETS → Sprite targets…** narrows that to
named sprites when you want it. The sprites a stroke is routed into are always
"sprite targets"; the independent stack of artwork over the atlases is always
"painting layers", and the two never share a word.

A 1px white stroke on pale artwork is easy to make and easy to miss, so trust the
status line under the canvas rather than your eyes: it names every stroke
(`pencil · canvas · Auto`), and says so when a stroke changed nothing because the
colour was already there or the target layer is hidden or locked. The pixel grid
is on by default and appears from 4x up, so zoom in with the ZOOM column when you
need to aim at individual pixels.
**Pick pixel** samples the colour under the pointer instead of painting, and
hands the pencil back after one sample rather than staying armed.

Two switches show the skin but are not drawing surfaces, and both say so in the
line above the canvas: **Player preview** and **Sprite state sheet**. Each reads
**Canvas editing** / **Canvas** while it is off. Both put the drawing tools away
while they are on, rather than leaving a sidebar of brushes that cannot paint.

A state sheet is **one sprite's** variants, numbered, so it asks for a sprite
before it will show one: pressed with the target on Auto it says so and opens
**Sprite targets** instead of picking a sprite of its own.

The tool column is docked beside the canvas rather than laid over it, so no open
panel can swallow a stroke: pick a brush and keep painting without closing
anything. The column holds one panel at a time and opens on **Drawing tools**, so
the brushes are on screen before anything is pressed.

Catamp Silverplay uses the original thirteen Winamp bitmap sheets plus
`pledit.txt` and `viscolor.txt`. The player and editor embed the same artwork.
Studio uses Cranpose for the GUI; mouse strokes and MCP commands modify the same
document and share undo/redo history. The editor is available on desktop, Android
and the web; the bundled skin is also available on iOS.

The whole-canvas drawing helper is documented in
[connected-canvas-api.md](../../tools/skin-studio/connected-canvas-api.md).
Python review helpers require Pillow; the MCP client and drawing helpers use the
standard library. Run helper unit tests with:

```sh
python3 -m unittest discover -s tools/skin-studio -p 'test_*.py'
```

The bundled Silverplay 22 artwork has continuous sidewall profiles across main,
EQ, playlist header, repeated rails and footer. Its seven native painting planes
separate the case, main illustration, small cats and four control groups. The
hand-authored `atelier22_*` recipes use opaque palette clusters without raster
scaling or blur. `catamp_silverplay_refine22.py` replays them through Studio from
the preserved Silverplay 21 WSZ; it requires clean work, applies seven layers,
and captures the result for review before export.

`check_catamp22_continuity.py` checks all 29 playlist tile phases and three taller
sizes in four focus/pressed states, including actual GPU checks at short and tall
sizes. `check_native_frames.py --output target/catamp22/frames` captures all 112
control states; `check_silverplay_art.py --frames target/catamp22/frames` compares
all 1,680 complete moving-control regions with the exported source sheets.

**Pixel study** magnifies native pixels, read-only, with an optional grayscale
value view. It **follows the pointer** over the canvas and keeps the last place it
was, so it can be read by looking at it. Lift a region with **Lift pixels** to pin
it there instead. The clipboard transforms under it are greyed out until
something is lifted.

**Sprite rectangles** says where each sprite lives in the joined skin. The editor
outlines the **one under the pointer** and names it beside the canvas; it never
draws into the artwork. Outlining every cell at once, in the artwork's own
pixels, buried the picture the hints were meant to point at -- and the hairlines
grew with the zoom, because they were pixels.

**Skin atlases** edits one BMP on its own, at native coordinates, with the same
pencil and undo history. **The whole skin** sits at the top of that list, and the
canvas carries a **← The whole skin** button while a single atlas is open, so the
detour always has a way back.

**Sprite targets** lists all sixty-odd named sprites with a filter box; the row's
pip says whether it is targeted, and **Solo** narrows to one.

**Painting layers** is one row per plane -- name, shown/hidden, locked/free --
with the chosen plane's controls at the foot of the column, so the list always
shows every plane the document has.

**Skin options** holds everything about the skin rather than about painting it --
the time readout, the equalizer's slider travel, whether the playlist, the
equalizer sliders, the playlist selection and the visualizer carry their own
artwork, the six `PLEDIT.TXT` playlist text colours and the 24 `VISCOLOR.TXT`
visualizer colours. The two palettes are grids of swatches: click a slot to set it
to the brush colour. They used to be reachable only over MCP, which made them
invisible to anyone painting by hand. These used to appear and disappear from the toolbar as the canvas
scrolled past the window they belonged to; they are all in one panel now,
whatever the canvas is showing. The sprite-state row under the canvas is the
same: it offers all five sliders -- volume, balance, position, eq, scroll --
whatever the canvas is scrolled over.

## MCP

The editor is also an MCP server, and an agent works it the way a person does:
one tool per panel, named for it.

| Tool | The panel it is |
| --- | --- |
| `studio_canvas` | The joined skin, the only drawing surface; reads it as an image and sets zoom, brush, colour, width and sprite state |
| `studio_draw` | A stroke on that canvas, routed to whichever sheets it crosses |
| `studio_atlas` | The one-BMP detour, and the way back |
| `studio_targets` | Sprite targets: list, choose, solo, or auto |
| `studio_rectangles` | Sprite rectangles |
| `studio_states` | One sprite's variants |
| `studio_layers` | Painting layers |
| `studio_options` | Everything outside the sheets, both palettes included |
| `studio_pixel`, `studio_study`, `studio_cluster` | What is under a pixel, the magnifier, the clipboard |
| `studio_history`, `studio_undo`, `studio_redo` | The shared history |
| `studio_status`, `studio_new`, `studio_open`, `studio_project`, `studio_export`, `studio_screenshot` | The document and the window |

Six things make that surface usable at speed, every one of them the scar of a
skin drawn through it:

- **A `text` operation** draws a string in the editor's own 5x7 face at an
  integer scale. A classic skin is full of set labels -- `MONO`, `AUTO`,
  `PRESETS`, the wordmark -- and every one of them was otherwise a hand-built
  stamp per letter, slow to write and easy to misalign.
- **An `image` operation** stamps a base64 PNG at x/y. Gradients, dithering,
  noise and hand-placed shading are far faster to compose in an image library
  than one drawing operation at a time. Fully transparent pixels are left alone,
  so a stamp never punches a rectangle through what it lands on, and partly
  transparent ones blend with what is under them -- a sheet has no alpha channel
  to keep, so the alternative is a soft glow arriving as hard speckle. Blending
  sees the canvas as it stands, this sprite's current art included, so it is
  right for adding light to artwork that is already there and wrong for a
  redraw, which compounds: composite a redraw onto a clean backdrop yourself.
- **An `origin` on the transaction** names a layer and puts 0,0 on that sprite's
  current destination. A sprite that moves -- a slider thumb follows its own
  frame -- has no fixed canvas address, so art aimed with coordinates read in
  another state lands at an offset and writes a second copy of itself. With
  `origin` the question cannot be asked, and it targets that layer by default.
- **Every `studio_draw` reports `bounds`**, the rectangle the ink actually
  landed in, **`clipped_pixels`**, how much fell outside the sprites it was
  aimed at, and **`overwrites`**, how often the transaction wrote one shared
  source cell twice with different colours. That last one is the trap this
  editor cannot design away: the four timer digits are one cell of
  `numbers.bmp`, the playlist top is one tile drawn nine times, so a stroke
  across them lands on top of itself and the last colour wins in every position
  at once. It used to be reported as a clean write; now it says which two canvas
  pixels collided and where. Name one target in `layers` to cure it. All three
  describe the transaction in hand -- they were session cumulative at first,
  which made every draw after the first report the union of its predecessors.
- **`studio_canvas` takes a `crop`**, so checking one button costs one small
  image instead of a render of the whole skin, and reports back the path it
  actually wrote: a relative one resolves against the Studio process, not the
  caller, which is how a render ends up somewhere nobody looks.
- **Zoom stops at 8x on purpose.** Judging a 14x25 sprite wants more than that;
  read at 1:1 and enlarge nearest-neighbour at your end, which costs nothing and
  has no ceiling.

The older tools -- `studio_state`, `studio_render`, `studio_guides`,
`studio_paint_layers`, `studio_layout`, `studio_patch`, `studio_inspect_region`,
the one-off skin switches and the palette pair -- still answer, so existing
scripts keep working, but they are no longer offered: they address single windows
and an isolated patch handoff, which is not how this editor edits.

## Layouts

The editor picks its layout from the surface it is given. At 1140x820 logical
points or more it uses the full desktop layout -- the same sidebar, drawers and
whole-skin canvas a desktop window gets -- so a browser on a laptop is not
reduced to the touch UI. Below that it uses the touch layout, whose canvas fits
the skin to the width it has on the first frame.

Hosted inside a player (Android, the web), the desktop layout swaps the WSZ path
field for **Player** and **Apply to player**, and its **Open…**/**Export…** go
through the platform's file picker rather than a typed path.

Below the desktop minimum the canvas is not worth splitting, so the tool column
goes back to covering the right of it; the canvas it leaves visible stays
drawable.

## Touch controls (Android and web)

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

The browser has no filesystem, so the web build keeps the same two things in the
scoped `localStorage` the player already uses: the layered draft under
`cranamp.studio.draft.v1`, and **Apply to player** under
`cranamp.skin.v1/Studio edited.wsz`, where it joins the Settings skin list and
survives a reload. Browser storage is finite; **Export Winamp skin…** downloads a
real `.wsz` when you want the work off the machine. MCP is desktop and Android
only -- the browser cannot open a listening socket.

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

**Sprite rectangles** marks each source cell directly on the editor image and lists
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
In the GUI, lift a region and select **Sprite rectangles → Parts in lifted
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
Auto drawing reports attempted pixels in those holes: the GUI shows a status
message; `studio_draw` returns `unmapped_pixels` and up to eight coordinates in
`unmapped_sample`. Intentional part selection and paint masks are not holes.
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
4. Auto targets the topmost sprite footprint. **Sprite targets…** opens a native multi-select list. Toggle any combination
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
   for the whole stack. **Tall playlist / Compact playlist** switches the live preview between 261 and 145 native pixels. MCP `studio_state` accepts `preview_playlist_height` from 145 to 522. **Presentation** shows the entire live stack at the largest fitting integer zoom (up to 2×). MCP can toggle it with `presentation: true/false`. Return to Canvas editing to paint.
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

The `image` operation takes the same ground in one step: `data` is a base64 PNG
laid down at `x`/`y`, at most 2048x2048. There is no scaling and no resampling
-- one source pixel is one skin pixel. Alpha 0 is left untouched; alpha between
blends with the canvas underneath and is written opaque, because the sheet has
nowhere to keep it.

Set `origin` to a layer ID and every coordinate in the transaction becomes
relative to that sprite's destination *as it stands now*, which is the only
reliable way to aim at one that moves with its frame.

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

**Drawing tools** now includes native pencil, line, rectangle and ellipse brushes,
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

Drawing tools → **Clean 1px corners** is shared with MCP `studio_state` and
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
