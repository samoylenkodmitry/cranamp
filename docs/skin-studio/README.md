# Cranamp Skin Studio

A pixel editor for classic Winamp `.wsz` skins, built into Cranamp. It is driven
by pointer, by touch, and by MCP; all three act on one document with one undo
history.

Open it from **Settings → Open Skin Studio** on desktop, Android or the web, or
from the command line:

```sh
cargo run -- --skin-studio
cargo run -- --skin-studio '/absolute/path/skin.wsz'
cargo run -- --skin-studio '/absolute/path/project.cstudio'
cargo run -- --skin-studio-mcp          # stdio MCP relay into a running Studio
cargo run -- --touch-preview --studio   # the editor at handset size
```

On desktop the editor opens in a separate native window. Android and the browser
open it inside the running player. Playback continues in every case. With no path
it opens the skin the player is wearing.

## Platform matrix

| | Desktop | Android | Web |
| --- | --- | --- | --- |
| Drawing engine, reports, materials, history | yes | yes | yes |
| MCP server | yes | yes | no (no listening socket) |
| `studio_screenshot` GPU capture | yes | no | no |
| Live player preview | yes | yes | yes |
| File open/export | path or picker | picker | `localStorage` + download |

The web build is checked with:

```sh
cargo check --target wasm32-unknown-unknown --no-default-features --features web,renderer-wgpu
```

The browser keeps the layered draft under `cranamp.studio.draft.v1` and
**Apply to player** under `cranamp.skin.v1/Studio edited.wsz`, which joins the
Settings skin list and survives a reload. **Export Winamp skin…** downloads a
real `.wsz`.

## Layouts

One editor at every size. A desktop window, a browser and a handset get the same
composable, the same buttons with the same names in the same order, and the same
panels with the same controls in them; only the arrangement changes.

| What | Where there is room | Where there is not |
| --- | --- | --- |
| The two toolbar rows | one row each | wrapped at the same left margin |
| The quick-access sidebar | beside the canvas | gone: everything on it is also in a panel — the edit scope reads there and is set in **Drawing tools** — and its zoom pills and **Fit whole skin** join the toolbar |
| The tool column | docked beside the canvas | over the right of it, or over all of it |
| The whole-skin preview | its own column, right of the canvas | the same column, reduced further to fit |
| The sprite-state strip | one row and a slider | wrapped |
| Pan | middle-button drag | a **Draw / Pan** switch, since a finger has no middle button |

The sidebar appears at 900 logical points of width and up; the tool column docks
whenever the canvas can keep 320 points beside it. Below 320×480 the layout
clips rather than reflowing further.

Hosted inside a player (Android, the web), the toolbar also carries **Player**,
**Apply to player**, **Save draft** and **Restore last draft**, and **Open…** /
**Export…** use the platform picker instead of a typed path. Android Back closes
an open panel, and otherwise saves a recoverable draft and returns to playback.

## The canvas

One drawing surface: main, equalizer and playlist joined at their own positions.

| | |
| --- | --- |
| Size | 275×377 at the default playlist height |
| Main | y 0 |
| Equalizer | y 116 |
| Playlist | y 232 |
| Playlist height | `preview_playlist_height`, 145..522 |
| Coordinates | native skin pixels, never enlarged GUI pixels |

A stroke may cross a window boundary; each pixel is routed to whichever BMP
sheet owns it. Layer IDs are qualified: `main.background`,
`equalizer.background`, `playlist.bottom.right`.

Main row 115 aliases source row 114 and cannot hold different pixels. Classic
playlist rails repeat every 29 rows; the header tile is 25×20 and is drawn nine
times; the footer follows the final cropped tile at any height. Tiles are drawn
at native size and cropped, never stretched.

**Skin atlases** opens one BMP on its own at its own coordinates,
sharing the pencil and the history. `← The whole skin` returns.

### The thirteen sheets

| Sheet | Size | Sheet | Size |
| --- | --- | --- | --- |
| `main.bmp` | 275×115 | `balance.bmp` | 68×433 |
| `titlebar.bmp` | 344×87 | `playpaus.bmp` | 42×9 |
| `cbuttons.bmp` | 136×36 | `monoster.bmp` | 56×24 |
| `posbar.bmp` | 307×10 | `numbers.bmp` | 99×13 |
| `shufrep.bmp` | 92×85 | `eqmain.bmp` | 275×315 |
| `volume.bmp` | 68×433 | `pledit.bmp` | 280×190 |
| | | `text.bmp` | 155×18 |

Plus `pledit.txt` and `viscolor.txt`. Three options add a sheet:
`plbg.bmp` 243×203, `plselection.bmp` 243×11, `eqhandles.bmp` 154×50.

Sheets with something non-obvious about them carry a note, shown in the status
line the moment the sheet is opened and repeated in draw results:

| Sheet | Note |
| --- | --- |
| `text.bmp` | read for one colour, never drawn as artwork |
| `plbg.bmp` | one track row every 11 pixels; tiles from the top above 203 |
| `plselection.bmp` | one row of the same height |
| `titlebar.bmp` | mostly shade-mode art Cranamp never draws |

## Pointer and touch controls

cranpose delivers key events only to a focused text field, so the editor has no
keyboard shortcuts.

| Gesture | Effect |
| --- | --- |
| Wheel | Scroll |
| Alt + wheel | Scroll sideways |
| Ctrl + wheel | Zoom a whole step, keeping the pixel under the pointer in place |
| Middle-button drag | Pan |
| Right-click on the canvas | Sample the colour under the pointer |
| Canvas edge scrollbars | Scroll; click to jump |
| **Draw / Pan** | Turns a drag into a pan, for surfaces with no middle button |

## Control shapes

| Shape | Class | Example |
| --- | --- | --- |
| Plain slab | Runs once | **Undo**, **Export**, **Fit whole skin** |
| Red slab | Runs once and can lose work | **New blank**, **Delete** |
| Slab with a rule under the label | Opens the panel named on it | **Drawing tools** |
| Pill | One value out of a set | `2x`, **volume**, **Pencil** |
| Slab with a pip on the right | On/off switch | **Fill shapes** |

A switch names the state the editor is in now, not the state pressing it would
move to. **New blank** and **Delete** ask a second time before acting. Refusals
are coloured in the status line. **Undo**/**Redo** count their steps (`Undo 3`)
and are drawn as unavailable at zero.

## Whole-skin preview

A live render of the entire skin — every sheet, every painting plane, the whole
joined canvas — sits in a column of its own at the top right of the canvas, at
every size. The column is reserved before the canvas and the tool column divide
what is left, so the preview is never over the artwork. It
follows the document revision, so it is current after every stroke, undo, MCP
transaction, option change and sheet edit, including while `studio_atlas` has a
single sheet open and the canvas is showing that one sheet.

| | |
| --- | --- |
| Column | 108 logical points, reserved beside the canvas at every size |
| Scale | the smallest whole-number reduction 2..8 that fits the room, so the preview is the largest one made of whole pixels; the label reads `SKIN 1:3` |
| Sampling | nearest neighbour |
| Position | top of its own column, right of the canvas |
| MCP equivalent | `Document::skin_render()` — the canvas surface, all planes composited |
| Pointer | none: it registers no hit test, so a stroke under it lands on the canvas |

## Panels

One panel is open at a time. The panel is a field on the document
(`view.drawer`), so `studio_canvas {"drawer": ...}` opens one without a pointer,
on any surface.

| `drawer` | Panel |
| --- | --- |
| `none` | closed |
| `tools` | Drawing tools |
| `layers` | Painting layers |
| `targets` | Sprite targets |
| `rectangles` | Sprite rectangles |
| `atlases` | Skin atlases |
| `history` | Edit history |
| `states` | Sprite state sheet |
| `study` | Pixel study |
| `options` | Skin options |
| `picker` | Colour picker |
| `files` | — (the toolbar carries the file actions) |

Where the canvas is wide enough the column is docked beside it, so an open panel
can never swallow a stroke; where it is not, the column is over the canvas and
the canvas it leaves visible stays drawable.

## Drawing

| Brush | `brush` | Operation `op` |
| --- | --- | --- |
| Pencil | `pencil` | `pixel` |
| Line | `line` | `line` |
| Rectangle | `rect` | `rect` |
| Ellipse | `ellipse` | `ellipse` |
| Curve | `curve` | `curve` |
| Fur / taper | `tuft` | `tuft` |
| Glass lens | `glass` | any filled shape with `material:"glass"` |
| Lift | `lift` | — |
| Stamp | `stamp` | `cluster` |
| Text | `text` | `text` |
| — | — | `path`, `image` |

Shapes preview while dragging and commit one history transaction on release;
cancelling restores the original pixels. One completed drag is one history entry;
an empty gesture creates none and does not discard the redo branch.

### Brush settings on the view

Every one of these is a field of `view`, set by `studio_canvas` and by pills in
both panels, and every drawing operation may override its own.

| Field | Range | Panel control |
| --- | --- | --- |
| `color` | `#rrggbb`; `#ff00ff` erases | colour pills, HEX box, **Colour picker…** |
| `brush_size` | 1..32 | WIDTH |
| `filled` | bool | **Fill shapes** |
| `ramp_to`, `ramp_axis` | colour or null; `down`/`across` | GRADIENT |
| `bevel` | 0..128, 0 = from the drag | BEVEL |
| `refraction` | 0..32 | REFRACTION |
| `curve_bend` | −100..100 | BEND |
| `grain`, `grain_size` | 0..64, 1..16 | PAPER GRAIN |
| `opacity` | 1..255 | STRENGTH |
| `text`, `face`, `text_scale` | string; `5x7`/`small`; 1..8 | TEXT, FACE, SCALE |
| `text_spacing` | −2..8 | LETTER SPACING |
| `clean_corners` | bool | **Clean 1px corners** |
| `mirror_x`, `mirror_y` | bool | mirrors |
| `alpha_lock` | bool | **Lock transparent pixels** |
| `mask_colors` | list of colours | **Mask picked color** |
| `states` | `current`, `onward`, `up-to`, `all` | **EDIT SCOPE** |
| `stamp_repeat` | 1..64 | **STAMP COPIES ALONG A DRAG** |
| `stamp_sweep` | bool | **One copy per sprite state** |
| `clip` | `[x,y,w,h]` or null | selected rectangle / **Clear paint clip** |
| `grid`, `guides` | bool | pixel grid (4× and up), rectangle outlines |

Brush settings and `measure` are the pencil, not the canvas: setting one does
not close an open atlas. Neither does reading the surface back — `crop`, `zoom`,
`magnify` and `path` alone are a look at the sheet in hand, not a way out of it.
Asking anything of the canvas, or an empty call, returns to it.

### Edit scope

Which variants of each target a stroke lands in.

| `states` | Writes into |
| --- | --- |
| `current` | the variant the canvas is showing |
| `onward` | that one and every one after it |
| `up-to` | every one up to and including it |
| `all` | every variant of every target |

A slider is twenty-eight frames that differ by one object, so the run from the
frame in hand to an end is the shape its artwork has: the stowage shelf in
Catamp Freefall is one `all` for the shelf and nine `onward`s, one per object,
instead of twenty-eight drawings. `all_states: true` is the retired name for
`all` and still answers, on the way in and in `view`. A draw whose scope is
wider than one variant answers `states_written` with the scope and how many
variants each target got.

### Repeating and sweeping

A sheet is a grid of cells that mostly hold the same drawing, and a slider is one
shape whose numbers walk across its frames. Both used to be loops outside the
editor emitting the same operations N times.

| Want | Write |
| --- | --- |
| the same drawing in five berths | `at: "targets"` with the five sprites in `layers` |
| the same drawing at chosen places | `at: [[0,0],[23,0],[46,0],…]` |
| a slider's 28 frames | `states: "all"` and `[from, to]` on the numbers that move |

A numeric operation field written as `[from, to]` is a **sweep**: the operation
is drawn once per variant the transaction writes, with that number walking from
the first variant to the last and landing on whole pixels. Sweepable: `x`, `y`,
`x2`, `y2`, `width`, `height`, `brush_size`, `curve_bend`, `bevel`,
`refraction`, `opacity`, `grain`, `grain_size`, `grain_seed`, `spacing`,
`scale` — and `control`, written as two points rather than two numbers. A sweep
needs a named target to count variants on, and more than one variant to walk
across; it says so when it has neither.

```json
{"layers":["main.balance.track"],"states":"all","operations":[
  {"op":"curve","x":195,"y":65,"x2":[180,210],"y2":58,"curve_bend":[55,-55],"color":"#5d5d6e"}]}
```

That is one call for all twenty-eight frames of a balance tail.

The pointer does the same two things with one lift and one drag: **STAMP COPIES
ALONG A DRAG** places N copies of the clipboard evenly between where the drag
started and where it ended, and **One copy per sprite state** puts one in each
variant of the edit scope instead — lift a tail, set the scope to All, and drag
from where frame 0 wants it to where frame 27 does.

### Text

Two faces, both walked by the same code that draws them:

| `face` | Cell | Notes |
| --- | --- | --- |
| `5x7` | 5×7 | full case |
| `small` | 4×5 | small caps; `a`-`z` drawn as capitals |

The pen advances `(cell + spacing) * scale`. `studio_canvas {"measure": ...}`
answers `width`/`height` (the ink) and `advance` (where the pen ends) from that
same walk. Supported characters: `A-Z a-z 0-9 - : . , / \ " ( ) [ ] + = _ ! ? &
# % * < > |`. Anything else is skipped and named in `unsupported_characters`.

### Materials

| Field | Effect |
| --- | --- |
| `grain` | Deterministic noise on the shape's own colours, clumped on a `grain_size` lattice, varied by `grain_seed`. Reproducible byte for byte. |
| `opacity` | Bakes the shape over what is already there at that strength, written opaque. Reads the surface as it stands, this transaction's earlier operations included. |
| `material:"glass"` | Baked glass in a filled shape, tinted with the brush colour. Normals come from the construction curves, not the stepped raster. `refraction` samples the underlay at integer coordinates. Nothing is blurred or resampled. |
| `image` | Base64 PNG, at most 2048×2048, one source pixel per skin pixel. Alpha 0 is left alone; partial alpha blends and is written opaque. |

A sheet has no alpha channel, so every blend is resolved and written opaque.
Blending is right for adding light to artwork that is already there and wrong
for a redraw, which compounds. `#ff00ff` is a colour to `opacity`, to an image's
alpha and to glass, and blending toward it is reported as `keyed_blends`.

## Sprite targets and rectangles

| Term | Means |
| --- | --- |
| **Sprite target** | which sprites a stroke is routed into |
| **Painting layer** | an independent artwork plane over the atlases |

With no target (Auto) a stroke paints **every sprite the brush covers**, as a
human stroke does. `studio_targets {"layers": [...]}` narrows it,
`{"solo": "main.play"}` picks one, `{"auto": true}` restores it.

`studio_states {"id": ...}` names a sprite exactly or by case-insensitive
substring and looks it up in the whole skin whatever surface is open, since a
recipe draws a slider's 28 frames in `studio_atlas` — the one place the view's
own catalogue is a single pseudo-sprite called `sheet`. A name that matches
nothing, or several, says which. `studio_targets {"solo": ...}` is the other
way round: it refuses a sprite that is not on the sheet `studio_atlas` has open,
rather than silently soloing something the surface cannot paint.

**Sprite rectangles** outlines the rectangle under the pointer and names it
beside the canvas; it never draws into the artwork. **Sprite targets** lists
them with their coordinates, which is how they are read on a surface with no
pointer to hover one with.
Rectangle colours: cyan sprite cells, gold the selected paint clip, pink runtime
text/graph/spectrum areas. Guides never enter preview artwork or the export.

Three kinds of rectangle:

| Kind | Flag | What it is |
| --- | --- | --- |
| Sprite variant | — | a source cell and where it is drawn |
| Runtime | `runtime: true` | where Cranamp writes live text, the graph, the spectrum, the four timer digits |
| Hit | `hit: true` | hit-tested, nothing drawn: the playlist footer's `ADD`, `REM`, `SEL`, `MISC`, `LIST` and its six transport keys, plus the main window's skin-chooser corner |

The artist has to draw a button in each `hit` rectangle. They move with the
playlist's height. Selecting one clips painting to it.

### Variant labels

`studio_targets {"variants": true}` answers `labels`, saying what each variant
is. Three are worth reading before drawing a skin:

- the classic playlist header keeps two rows and **Cranamp draws the lower one
  always** — there is no unfocused playlist;
- the main and equalizer titles are the other way round: focused first;
- `balance.track` runs frame 0 hard left to frame 27 hard right, not out from
  the centre as classic Winamp does.

### Shared source cells

Some pixels cannot be told apart, and no setting changes that:

| Cell | Drawn |
| --- | --- |
| the timer digit cell | 4 times |
| the playlist header tile | 9 times |
| each equalizer band's groove | 11 times (one set of 28 frames) |
| main row 115 | aliases row 114 |

**Unique EQ art** (`eq_handles`) is the one separation available: eleven
independent 14×25 handles in `eqhandles.bmp`, which costs 25 pixels of the
63-pixel track and limits travel to 38. It does not separate the band tracks.

A stroke across a shared cell lands on top of itself and the last colour wins
in every position; `overwrites` reports it. Name one target in `layers` to cure
it.

### Gaps

A sheet is a bag of cells, and the space between them is never drawn —
`pledit.bmp` column 125 falls between the two footer flaps.

- `studio_rectangles {"gaps": true}` lists those areas as rectangles, before
  painting.
- `unsampled_pixels` in a draw result counts ink that landed in one, with
  coordinates. It counts ink, not clearing, and answers `sheet_is_never_drawn`
  for a sheet like `text.bmp` rather than a number.

## Painting layers

Independent artwork planes over the original atlases. Select, name, show, hide,
lock, reorder, set opacity 0..255, clip to the plane below, merge down, delete.
Bottom-to-top; move index zero is above the base. Locked planes reject drawing;
hidden planes do not export. One opacity drag is one history step.

Choosing **Base atlases** paints underneath the planes. The editor selects the
topmost visible plane when it opens a layered project.

**Clip to layer below** constrains a plane to the effective alpha of the plane
immediately under it — opacity included. Source pixels are retained; switching
clipping off reveals them. Chains are supported. For the bottom plane the
original atlas provides the mask. Merging a clipped plane bakes its effective
pixels into a visible, unlocked, opaque, unclipped lower plane; merge clipped
followers first, and Studio rejects a merge that would change a follower's mask.

`studio_layers` actions: `list`, `add`, `select`, `set`, `move`, `merge_down`,
`delete`. `studio_targets {"paint_layer": ...}` selects one by ID; `null`
selects the base.

## Skin options

Everything about the skin that is not painted into a sheet.

| Option | Values | Effect |
| --- | --- | --- |
| `footer` | `classic`, `time-total` | the time readout |
| `eq_travel` | 1..52 | equalizer slider travel; default 52 |
| `visualizer_glass` | bool | off, the player fills the spectrum's whole rectangle with VISCOLOR slot 0 — an opaque box no sheet contains and no canvas render shows |
| `playlist_background` | bool | adds `plbg.bmp` 243×203 and an editable `list.background` |
| `playlist_selection` | bool | adds `plselection.bmp` 243×11; the runtime reserves an eight-pixel marker gutter |
| `eq_handles` | bool | adds `eqhandles.bmp` 154×50, eleven columns, normal above pressed |
| `playlist_colors` | six keys | `PLEDIT.TXT` |
| `visualizer_colors` | 24 colours | `VISCOLOR.TXT` |

Turning one of the three surface options on or off answers `sheets_changed`,
naming the sheet, its size and what its cells are.

### PLEDIT.TXT

| Key | Colour of |
| --- | --- |
| `Normal` | track text |
| `Current` | the playing track's text |
| `NormalBG` | the list background |
| `SelectedBG` | a selected row |
| `MbFG` | the marquee / status text |
| `MbBG` | the marquee background |

The playlist footer's two readouts are the one place in that window that does
**not** use PLEDIT.TXT.

### VISCOLOR.TXT

24 slots, answered by `studio_options` as `visualizer_slots`:

| Slot | Is |
| --- | --- |
| 0 | background |
| 1 | grid dots |
| 2..17 | analyzer bar, top of the bar (2) down to its foot (17) |
| 18..22 | oscilloscope |
| 23 | analyzer peak dot |

Both palettes are grids of swatches in **Skin options**; click a slot to set it
to the brush colour.

### Readability

`studio_options` answers `readability` — for each of the nine readouts Cranamp
writes: `reads`, `ink`, `ground`, `contrast`, `readable`. Eight are words and
the ninth is a picture: **the equalizer curve is drawn in text.bmp's ink**, the same
colour as the title, so a skin that wants dark ink on a pale title strip must
give the graph a pale background too or draw its curve invisibly. The three
playlist checks use `NormalBG` where the skin has no `plbg.bmp`, which is where
they are needed most — the classic playlist fill has no bitmap under it at all.

`studio_export` repeats anything below 3:1 as `hard_to_read`. 4.5 is comfortable at this size;
below 2 is a readout that is not there. `studio_pixel {"ink": "#..."}` points
the same WCAG arithmetic at any rectangle.

## Preview and export

**Player preview** runs the actual Cranamp composables against the current
document at integer zoom and pan. **Tall playlist / Compact playlist** switches
between 261 and 145 native pixels. **Presentation** shows the whole live stack
at the largest fitting integer zoom, up to 2×. The Pressed and On/Off selectors
change the preview through preview-only copies; the document and the export are
untouched. The editing canvas assembles skin bitmaps only — no runtime text, no
visualizer.

**Export** validates through Cranamp's own skin loader and writes atomically.
The file you opened is not touched unless you export over it. Export marks the
saved state, which undo and redo can return to. History keeps 32 transactions
for the session; a new edit after an undo replaces the redo branch. History is
in memory and is never written into a WSZ.

Export reports, never refuses:

| Report | Means |
| --- | --- |
| `undrawn_sprites` | sprites still blank — a skin from **New blank** with twelve untouched sheets exports in six kilobytes and parses perfectly |
| `hard_to_read` | a readout below 3:1 against the artwork behind it |

**Save project** writes a layered `.cstudio` ZIP: the original base WSZ, lossless
layer PNGs, and layer/view metadata. Export writes a flattened WSZ and, when
painting layers exist, an adjacent editable `.cstudio`. `studio_project` supports
`save` and `open`. Project files preserve layers and current artwork, not
history.

## MCP

The running Studio serves JSON-RPC at `http://127.0.0.1:18765/mcp`. It binds
only to loopback, rejects browser `Origin` headers, and accepts at most 8 MiB
per request. Run one Studio for that endpoint.

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

The stdio relay connects to that same native window rather than opening a
document of its own. On Android, forward the port:

```sh
adb -s DEVICE_SERIAL forward tcp:18766 tcp:18765
```

A thin development client (Node 18+) takes a tool name and its arguments:

```sh
node tools/skin-studio/client.mjs studio_status
node tools/skin-studio/client.mjs studio_targets '{"id":"band0.track","variants":true}'
node tools/skin-studio/client.mjs studio_draw \
  '{"operations":[{"op":"line","x":41,"y":90,"x2":51,"y2":90,"color":"#d5f2fa"}]}'
node tools/skin-studio/client.mjs studio_canvas \
  '{"zoom":4,"crop":[16,88,23,18],"path":"/tmp/play.png"}'
node tools/skin-studio/client.mjs studio_undo
```

### Tools

One tool per panel, named for it.

| Tool | Is |
| --- | --- |
| `studio_canvas` | the joined skin: reads it back as a PNG, and sets zoom, brush, colour, width, sprite state, open panel |
| `studio_draw` | one atomic transaction of up to 10000 operations on the surface in hand |
| `studio_atlas` | the one-BMP detour, and the way back |
| `studio_targets` | sprite targets: list, choose, solo, auto |
| `studio_rectangles` | sprite rectangles, runtime and hit rectangles, and sheet gaps |
| `studio_states` | one sprite's variants, as a contact sheet and as numbers |
| `studio_layers` | painting layers |
| `studio_options` | everything outside the sheets, both palettes included |
| `studio_pixel` | what is under a pixel: mapping, colour, contrast |
| `studio_study` | the read-only study board |
| `studio_cluster` | the clipboard |
| `studio_history`, `studio_undo`, `studio_redo` | the shared history |
| `studio_status` | the document |
| `studio_new`, `studio_open`, `studio_project`, `studio_export` | the document's life |
| `studio_screenshot` | the GPU scene |

Retired but still answering, for existing scripts: `studio_state`,
`studio_render`, `studio_guides`, `studio_paint_layers`, `studio_layout`,
`studio_patch`, `studio_inspect_region`, the one-off skin switches and the
palette pair. They are not offered in the tool list.

### Conventions

- Every schema declares `additionalProperties: false`. An argument a tool does
  not have is refused by name, with the tool's own field list.
- A refusal names the field and the value it got (`zoom is 1..8 (got 10)`,
  `eq[3] is 0..27`), and for a transaction the operation index
  (`operations[2] "text": ...`). Rollback is all or nothing.
- Coordinates are always native pixels, of the assembled canvas or of the sheet
  `studio_atlas` has open. Every result names which in `surface`.
- Every image tool takes `path` — it writes the file and answers with where and
  how big. Omit it and the PNG comes back inline (a full `studio_screenshot` is
  about 420 KB of base64). A relative path resolves against the Studio process,
  not the caller, and the answer is the resolved one.
- Every image tool takes `magnify`, 1..64, which enlarges the returned image
  nearest-neighbour instead of `zoom` and is capped at 2048 pixels a side.
- `studio_new`, `studio_open` and `studio_project` refuse over unsaved edits and
  name `discard: true` in the refusal. All three land on the whole skin.
- Both catalogue filters (`id`, `sheet`) take one case-insensitive substring or
  a list. When a filter matches nothing, the answer says what it looked in.

### Drawing operations

`op` is one of `pixel`, `line`, `rect`, `ellipse`, `path`, `curve`, `tuft`,
`stamp`, `cluster`, `text`, `image`.

| Field | For |
| --- | --- |
| `x`, `y`, `x2`, `y2`, `width`, `height` | geometry, in native pixels |
| `control` | absolute quadratic control point; may be fractional |
| `points` | relative to `x`/`y`: `[x,y]` line, `[cx,cy,x,y]` quadratic, `[c1x,c1y,c2x,c2y,x,y]` cubic |
| `curve_bend` | −100..100; `control` overrides it |
| `fill`, `brush_size` | bool; 1..32 |
| `color`, `ramp`, `ramp_axis` | `#rrggbb`; exact palette colours along an axis |
| `rows`, `palette` | one character per pixel; a character absent from the palette is skipped, which is how transparency is spelled |
| `text`, `face`, `scale`, `spacing` | see **Text** |
| `align`, `width` | place the word in a box `width` wide starting at `x` — `left`, `center` or `right` — instead of starting it at `x` |
| `data` | base64 PNG for `image` |
| `material`, `bevel`, `refraction` | glass |
| `grain`, `grain_size`, `grain_seed`, `opacity` | materials |
| `mirror_x`, `mirror_y`, `clean_corners` | geometry |

Curve endpoints, path points and control points may be fractional.

Transaction-level fields on `studio_draw`:

| Field | Effect |
| --- | --- |
| `layers` | sprites to route every pixel into; `[]` is Auto |
| `origin` | put 0,0 on this sprite's destination as it stands, and target it — the only safe way to aim at a sprite that moves with its frame |
| `states` | the edit scope for this transaction; `all_states: true` is the retired name for `all` |
| `mask_colors` | a temporary mask, without replacing the persistent one |
| `label` | names the history entry |
| `preview` | dry run: applied, answered, and put back; nothing recorded, revision unmoved |
| `states` | the edit scope: `current`, `onward`, `up-to`, `all` |
| `at` | run the whole operation list once at each `[x, y]` offset, or at each chosen target's own destination with `"targets"`; at most 256 places |
| `crop`, `zoom`, `magnify`, `path` | read the surface back, as `studio_canvas` does — on a committed transaction as well as a `preview` |

### Draw reports

All describe the transaction in hand.

| Report | Means |
| --- | --- |
| `pixels_written`, `bounds` | how much ink, and the rectangle it landed in |
| `ms` | how long the transaction took |
| `clipped_pixels` | fell outside the chosen sprites |
| `unmapped_pixels` | no bitmap source at all — the classic playlist fill. Turn on **List canvas** to paint there |
| `unsampled_pixels` | landed in a gap between a sheet's cells, where nothing will ever show it |
| `overwrites` | two *different* canvas pixels wrote one shared source cell; `overwrite_sample` says which |
| `keyed_blends` | an `opacity`, an image's alpha or glass read `#ff00ff` as a colour, turning a glow to mud and glass to magenta and taking the cell's transparency with it |
| `crossed_cells` | ink left its own cell and landed in a **repeated** one, which the player then draws n times. Silent for a cell painted on its own, and for an operation covering the whole sheet |
| `identical_variants` | variants that came out the same picture — 28 slider frames all on frame 0, or a pressed state identical to its released one. Fully transparent variants are excluded; sprites sharing source cells share one entry, with `also` |
| `unsupported_characters` | characters the face does not have; the rest of the text still landed |
| `covered_pixels` | ink hidden, in **every** state it was drawn into, behind another part of the same control — a slider's track and its thumb are read from one value, so a mark that runs to the thumb's position is behind the thumb in every frame. Per-frame coverage is what a slider is and is not reported; a background under a button is not either |
| `repeated` | how many places `at` ran the operations at |
| `swept` | which fields walked, and over how many steps |
| `states_written` | the scope, and how many variants each target got, when it was more than the one in hand |
| `surface`, `revision`, `note` | which surface, which revision, the sheet's note |

### Response shapes

The two catalogue tools are different shapes on purpose: one entry per sprite,
one entry per variant.

| Tool | Answers |
| --- | --- |
| `studio_status` | `path`, `revision`, `dirty`, `undo`, `redo`, `message`, `canvas`, `surface`, `sprites` (a count), `sheets` (name, width, height), the whole `view` |
| `studio_canvas`, `studio_atlas` | the same without `sheets`; with `path`, `{view, path, size}`; with `measure`, `{measured, face, scale, spacing}` and `unsupported_characters` |
| `studio_targets` | `{sprites, of, chosen}` — per sprite: `id`, `sheet`, `source`, `states`, `drawn`; with `variants: true` also `variants` and `labels` |
| `studio_rectangles` | `{rectangles, of}` — per **variant**: `id`, `label`, `sheet`, `rect`, `source`, `variant`, `active`, `runtime`, `hit` |
| `studio_rectangles {"gaps":true}` | `{sheet, size, gaps, of, note}`, or `never_drawn: true`; capped at 64 entries |
| `studio_states` | the contact sheet, plus `sprite`: `id`, `sheet`, `of`, and per variant `index`, `label`, `rect`, `painted_pixels`, `differs_from_previous`, `largest_channel_change`, `same_picture_as`. `image: false` answers the numbers alone |
| `studio_pixel` | `surface`, `hits` (each sprite under the pixel, where it keeps it, its `rgba`, what shares it), `ground` over the rectangle's opaque pixels, `contrast`/`readable` with `ink`, `nothing_at` when there is no artwork |
| `studio_options` | every option flat, as this table names them, plus `visualizer_slots`, `readability`, and `sheets_changed` when one added or removed a sheet |
| `studio_screenshot` | `path`/`size` or the PNG, plus `showing` (`player` or `editor`) and `player` (`x`, `y`, `zoom`) |
| `studio_export` | `path`, `bytes`, and when there is something to say `undrawn_sprites` and `hard_to_read` |
| `studio_layers`, `studio_history`, `studio_project` | the planes, the history (seekable with `cursor`), the project file |

`ground` and `readability` treat `#ff00ff` as transparent rather than as a
colour.

### studio_screenshot

Captures the Cranamp GPU scene at native logical pixels, once the requested
revision is composed — including the live player's atlas reload, so a client
never needs a sleep after painting. It uses offscreen GPU snapshots rather than
`Robot::pump_frames`, which a hidden macOS window cannot satisfy.

| Field | Effect |
| --- | --- |
| `panel` | `main`, `equalizer`, `playlist`, `all` — crops to one window in the player's own coordinates |
| `crop` | scene pixels alone; the panel's **own native skin coordinates** when `panel` is given too |
| `presentation` | put the window into the live stack first |
| `magnify` | 1..64, capped at 2048 a side |

### The clipboard and the study board

```json
{"name":"studio_cluster","arguments":{"rect":[10,12,86,55]}}
{"name":"studio_cluster","arguments":{"flip_x":true,"quarter_turns":1}}
{"name":"studio_draw","arguments":{"operations":[{"op":"cluster","x":110,"y":15}]}}
```

`studio_cluster` returns reusable palette-character stamp rows as well as
storing an in-memory clipboard. **Lift pixels** captures only the selected
layers with their transparency, or the visible composition in Auto; picking up
pixels does not modify the skin or add a history step.

`studio_study` produces a read-only board: the crop at native size above an
integer enlargement. `values: true` is a grayscale view, `(54R + 183G + 19B)/256`.
`geometry: true` marks sprite footprints; `grid: true` adds a display-only grid;
`reference` and `reference_rect` place a reference alongside without importing
its pixels. It changes nothing.

## Source layout

| File | Holds |
| --- | --- |
| `src/winamp/studio/mapping.rs` | source/destination mappings from Cranamp's own sprite constants; state variants and shared-tile metadata |
| `src/winamp/studio/model.rs` | editable bitmaps, transactional drawing, history, composition, palette edits, WSZ serialization |
| `src/winamp/studio/mod.rs` | desktop Cranpose UI and the production-player preview |
| `src/winamp/studio/draft.rs` | the recoverable draft, and the skin a hosted editor hands back to the player |
| `src/winamp/studio/mcp.rs` | MCP schemas, the JSON-RPC endpoint, the stdio relay |
| `tools/skin-studio/client.mjs` | the thin development client |

## Verification

```sh
cargo test
cargo test --lib join_tests
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo check --target wasm32-unknown-unknown --no-default-features --features web,renderer-wgpu
```

`join_tests` is 600 MCP cases comparing painting through the joined canvas with
painting one unsplit bitmap: pixels, lines, filled rectangles, ellipses, paths,
curves, tufts, stamps, palette ramps and glass at panel/title/tile/footer joins,
both borders, and five playlist heights. Further tests cover GUI preview, lifted
stamps, cancellation, selected parts, masks, mirrors, layers, undo/redo and WSZ
reload, both pressed variants, all 28 slider frames, every state mapping's
bounds, atomic rollback, shared human/MCP undo, and round trips through the
production skin loader. They verify pixel routing, not illustration quality.

`tests/studio_drawing_e2e.rs` and `tests/studio_handset_e2e.rs` drive the real
editor through cranpose's hit dispatch — the first at the reference window size,
the second at 393×780 — so the claim that one editor lays out at every size is a
test rather than an intention: the same eight panel buttons, the same panels,
the same controls in them.

Physical sprite edges must be snapped from their absolute native coordinates,
with widths derived from the snapped endpoints; rounding positions and widths
independently leaves gaps at fractional phone scales. Check `--touch-preview` as
well as the integer-zoom desktop window.

Playlist borders must use `TiledSprite`, not `StretchSprite`: nearest-neighbour
sampling alone does not preserve sprite geometry when a source cell is enlarged
to fill a border.
