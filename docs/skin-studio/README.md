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

Fifty-five things make that surface usable at speed, every one of them the
scar of a skin drawn through it:

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
  sees the canvas as it stands *including what earlier operations in the same
  transaction have just painted*, so it is right for adding light to artwork
  that is already there and wrong for a redraw, which compounds: composite a
  redraw onto a clean backdrop yourself. The backdrop used to be composed once
  per transaction and then reused, which meant a glow placed after the picture
  beneath it blended into the picture that was there *before* -- and wrote that
  back, silently undoing the work underneath it. Two cut windows and two glows
  in one transaction were enough to lose one of the windows.
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
  `overwrites` counts only *different* canvas pixels landing in one source cell.
  It used to count any repeat, so filling a panel and then drawing on it --
  ordinary drawing, the same canvas pixel written twice -- reported hundreds of
  collisions and buried the one signal that matters.
- **`studio_canvas` takes a `crop`**, so checking one button costs one small
  image instead of a render of the whole skin, and reports back the path it
  actually wrote: a relative one resolves against the Studio process, not the
  caller, which is how a render ends up somewhere nobody looks.
- **`studio_atlas` takes the same `path`, `crop` and `zoom`**, so the one-BMP
  detour can be looked at and not only drawn into. It is the only way to see a
  sheet the assembled canvas never shows: `numbers.bmp`, `text.bmp`, a pressed
  variant, a slider frame that is not the current one. The capability was there
  from the start and simply not in the schema, which made the whole detour
  write-only to anything reading the tool list.
- **`studio_status` carries the document, not the catalogue.** Every sprite and
  every one of its variants used to ride along with it -- and with every other
  call that touched the view, because they all answer with the status. Setting a
  colour cost eighteen kilobytes; so did reading one cropped button. The sprites
  have two panels of their own: `studio_targets` says which exist,
  `studio_rectangles` says where each variant lives. Status now also names the
  `surface` a stroke's coordinates mean -- `canvas`, or `atlas <sheet>` -- and so
  does every `studio_draw` result, because one pencil and one history serve both
  and a stroke aimed at the wrong one still succeeds, somewhere else.
- **Zoom stopped at 8x, and that was the wrong cap.** Judging a 14x25 sprite
  wants more than 8x, and telling the caller to enlarge at their end costs a
  file and an image library. `magnify` replaced it; see below.
- **A `path` on an image tool means "write it and tell me where".**
  `studio_screenshot`, `studio_states` and `studio_study` used to write the file
  *and* hand back the whole PNG as base64 -- for a full scene, 420 KB, about a
  hundred thousand tokens of a caller's context, spent twice for one look, after
  it had already been given the file. `studio_canvas` always answered with the
  path and the size, and now all of them do; omit `path` and the image still
  comes back inline, which is what a small crop wants.
- **Both catalogue tools filter.** `studio_rectangles` unfiltered is fifteen
  kilobytes and `studio_targets` is seven, and almost nobody wants either whole:
  they want one sheet, or one sprite. Both take `sheet` and `id` (a
  case-insensitive substring), `studio_rectangles` also takes `runtime`, and
  `studio_targets` takes `variants` to include every source rectangle a sprite
  has. "All 28 frames of `band0.track`" went from a 15 KB read to a 571-byte
  one, which is the difference between checking and guessing.
- **A refusal names the operation that caused it.** A transaction may carry ten
  thousand operations, and `No glyph for '@'` named none of them; it is now
  `operations[2] "text": ...`, and the rollback is unchanged -- all or nothing.
- **A refusal names the field and the value it got.** Six of the view's numbers
  were checked together and answered with the whole list of ranges --
  `preview_playlist_height 145..522; zoom 1..8; slider frames 0..27; digit
  0..9; playback 0..2` -- which says which ranges exist and not which field was
  wrong or what was sent. It is `zoom is 1..8 (got 10)`, and `eq[3] is 0..27`
  names the band.
- **An argument a tool does not have is refused, by name.** Every schema here
  declares `additionalProperties: false` and the server was not enforcing it,
  so a field aimed at the wrong tool was accepted, dropped, and answered with a
  perfectly ordinary-looking result. `presentation` belongs to `studio_canvas`;
  sent to `studio_screenshot` it returned a capture of the editor's own window
  described as the player's scene, and nothing in the reply said the argument
  had gone nowhere. The refusal now says `studio_screenshot has no presentation
  -- presentation is studio_canvas's. It takes: crop, path`. The retired tools
  are not listed and so have no schema to check against; they are left alone.
- **Export says what was never drawn.** Validating through the loader proves
  the archive parses and nothing else: a skin from **New blank** with twelve of
  its thirteen sheets untouched writes six kilobytes and answers exactly as a
  finished one does, while the transport keys, the timer, the title bar and the
  whole equalizer are simply invisible in the player. `undrawn_sprites` names
  them, and `studio_targets` carries a `drawn` flag per sprite, which is the
  same question asked halfway through. Both are reports and never refusals -- a
  skin with no channel lamps is a choice somebody may have made.
- **Export says what cannot be read.** Studio knew both halves of this from the
  start and never put them together: the runtime rectangles say where Cranamp
  writes, `text.bmp` says what colour the display ink will be, and PLEDIT.TXT
  says what colour the playlist text will be. A title one shade off its own
  background is the mistake that actually ships, and the only way to catch it
  was to render the player and squint. `studio_options` answers `readability`
  -- every readout, its ink, the artwork under it and the contrast between them
  -- and export repeats anything below three to one. Four and a half is
  comfortable at this size; below two is a readout that is not there. Writing
  this found a bug in the checker before it found one in a skin: the playlist
  footer's two readouts are the one place in that window that does *not* use
  PLEDIT.TXT, and checking them against `Normal` reported dark-on-dark for a
  footer that reads perfectly.
- **`variants` comes with `labels` saying what each one is.** Four rectangles
  for a switch say nothing about which is off and which is pressed; twenty-eight
  for a slider say nothing about which end of the travel frame 0 is. Both were
  answerable only by reading `mapping.rs` and the player's sprite constants, and
  guessing wrong paints the pressed art into the released cell, where nothing
  reports it because nothing is wrong with the drawing. The labels are on the
  rectangles too, so the canvas hint reads `play · pressed` rather than
  `play:1`. Three of them are worth reading before drawing a skin: the classic
  playlist header keeps two rows and **Cranamp draws the lower one always** --
  there is no unfocused playlist, so art in the upper row is never seen by
  anybody; the main and equalizer titles are the other way round, focused
  first; and `balance.track` runs frame 0 hard left to frame 27 hard right
  rather than out from the centre the way classic Winamp does. Catamp Sampler
  had its whole playlist header in the row nobody sees until these existed.
- **A draw reports `identical_variants`: variants that came out the same
  picture.** A recipe that draws twenty-eight slider frames and forgets to
  offset each by its own y puts all of them on frame 0. Every other report says
  it worked -- `bounds` is sensible, nothing is clipped, nothing is unsampled,
  and `overwrites` is a canvas-to-source measure that does not apply to a sheet
  open on its own. The only thing wrong is that twenty-seven frames are now the
  same picture, so that is what is said, in the result and in the status line.
  An empty cell is not a duplicate but an unpainted one, so fully transparent
  variants are left out; otherwise the first stroke on a blank sheet reports
  every sprite on it. It is silent in ordinary work, and it catches the other
  half of the same defect too: a pressed state that came out identical to its
  released one.
- **`studio_states` also answers the numbers.** A contact sheet cannot show
  that two cells differ by four pixels, or by none. Each variant now reports
  what it is, how many pixels it differs from the one before it by, the largest
  channel change between them, and which earlier variant it is a copy of; a
  repeated cell is marked `4 = 1` on the sheet itself. A pressed felt patch
  eleven per cent darker than its released one looks pressed in the code and
  identical on screen, and this is the only thing that says so.
- **A sheet with something surprising about it says so when it is opened.**
  `text.bmp` is not a glyph sheet, `plbg.bmp` is drawn one track row every 11
  pixels and tiles from the top above 203, `plselection.bmp` is one row of the
  same height, and most of `titlebar.bmp` is shade-mode art Cranamp never
  draws. The notes existed and arrived in a *draw* result, which is one stroke
  after they would have been useful; they are in the status line the moment
  `studio_atlas` opens the sheet.
- **An option that adds a drawing surface names it.** `playlist_background`,
  `playlist_selection` and `eq_handles` are not settings, they create or remove
  a sheet, and turning one on used to answer with the option set and no mention
  of the 154x50 surface that had just appeared or what its cells are.
  `sheets_changed` says what arrived, how big it is, and what it holds --
  including that eleven independent handles cost the equalizer fourteen pixels
  of travel.
- **`studio_screenshot` says what it captured and can frame the player.** The
  window shows either the editor or the live player and this call captures
  whichever it is; the description promised the player's scene and the result
  said nothing about which one arrived. It now answers `showing` and where the
  player sits, takes `presentation: true` to put the window into the live stack
  first, and takes `panel` -- `main`, `equalizer`, `playlist`, `all` -- to crop
  to one window in the player's own coordinates. Cutting one window out of a
  capture used to mean measuring it by eye against a full-scene PNG, and
  measuring it again after every resize, because the stack is centred in
  whatever room it has.
- **`studio_rectangles` answers `rectangles`, not `guides`.** A guide is what
  the editor calls its overlay; a caller asked for rectangles and got a key
  named after the implementation. Response shapes are listed under **What each
  tool answers with** below, which is where the two catalogue tools' different
  shapes -- one entry per sprite, one entry per variant -- stop being a
  surprise.
- **`studio_rectangles` also answers `hit`: the controls with no sprite.** Every
  button in a classic skin is a sprite and says where it is, except eleven of
  them: the playlist footer's five menus (`ADD`, `REM`, `SEL`, `MISC`, `LIST`)
  and its six transport keys are rectangles Cranamp hit-tests and draws nothing
  for, and the artist has to put a button in each. Nothing in Studio said
  where, so the only way to find out was to read the player's source -- and
  drawing a footer without them puts a cat across the elapsed-time readout, or
  a button a pixel out of step with its own hit area. They are guides now, with
  `hit: true`, alongside the main window's skin-chooser corner; they move with
  the playlist's height like every other footer rectangle, the desktop canvas
  outlines the one under the pointer, and the touch layout's **Parts** drawer
  lists them with their coordinates because there is no pointer there to hover
  one with. Selecting one clips painting to it, which is the only handle a
  rectangle without a sprite has.
- **A character the 5x7 face does not have is skipped, not fatal.** It comes
  back in `unsupported_characters` alongside the pixels that did land. A
  seven-hundred-operation sheet used to die on one `@` and roll back everything
  before it, which is a poor trade for a character that could simply be absent.
  The set the face does have is named on the `text` field itself.
- **`face: "small"` is a second face, four by five.** A classic skin has cells
  that need a word and have no room for one: the mono lamp is 27 pixels wide,
  the stereo lamp 29, an equalizer band caption 14, a playlist footer button
  28. The 5x7 face runs off the end of all of them, so every skin that wanted a
  word there carried its own glyph table in its build script -- Cardboard's
  seven letters, Sampler's thirty-odd, the same table written twice -- which
  meant the capability existed for a skin with a Python recipe and for nobody
  drawing by hand, on Android or in a browser. Five pixels tall has no room for
  a descender, so it is a small-caps face and `a` is drawn as `A` rather than
  skipped. `M`, `H` and `W` are the same four columns with the bar in a
  different place, so M fills the two rows under its apex and W the two above
  its point; with one bar each, `MISC` came out of the playlist footer reading
  `HISC`.
- **`grain` and `opacity` on any shape.** Paper is not flat, and a flat
  rectangle of kraft reads as plastic; light leaking out of a box has to be
  added to the picture already inside it. Both were being composed in an image
  library and stamped in through the `image` operation, which works and meant a
  skin that wanted paper needed Python and Pillow -- so neither material could
  be drawn from the editor, from Android, or from a recipe with no image
  library. `grain` is deterministic noise on the shape's own colours, clumped on
  a `grain_size` lattice because per-pixel noise is invisible at the size a skin
  is looked at, and seeded by `grain_seed` so a recipe reproduces byte for byte.
  `opacity` bakes the shape over what is there at a fraction of its strength --
  the same blend the `image` operation does, resolved and written opaque,
  because the sheet has no alpha to keep. Both live on the view like every other
  brush setting, as **PAPER GRAIN** and **STRENGTH** in Drawing tools, so a
  mouse stroke and an MCP operation get the same material and an operation can
  still override either for itself; adding an engine capability only a script
  could reach would have been the same mistake in a new place. Catamp Cardboard
  was drawn with an image library first and redrawn with these: the same skin,
  no Pillow, and half the bytes, because native grain has far fewer distinct
  colours than per-pixel noise.
- **`preview: true` on a draw is a dry run.** The operations are applied, the
  surface comes back as it would leave it, and the document is put back --
  nothing recorded, revision unmoved, and `crop`, `zoom` and `path` behave as
  they do on `studio_canvas`. It also reports `bounds` and `overwrites`, so a
  stroke can be checked before it is made. Without it the only ways to see an
  eleven-pixel cat were to draw it, look, and undo, or to reimplement the
  rasteriser in an image library and hope the two agreed; the second is what
  actually happened, and the proofing script it needed is deleted.
- **`unsampled_pixels` counts ink nothing will ever show.** A sheet is a bag of
  cells and the space between them is never drawn: `pledit.bmp` column 125 falls
  between the two footer flaps, so a footer band painted straight across the
  sheet loses a column and a button laid across it arrives a pixel out of step
  with its own hit area. Drawing that band now reports 38 unsampled pixels and
  says they are all at x=125. Sheets that are not artwork at all say so too --
  `text.bmp` is read for one colour and never drawn, which is a day's work to
  discover by hand.
- **`studio_new` and `studio_open` land on the whole skin.** The editor has
  one drawing surface and every GUI path that replaces the document puts the
  view back on it; the two MCP calls that replace it did not, so a skin started
  from **New blank** over MCP -- the documented way to start one -- was handed a
  retired single-window panel instead: a canvas 275x115 rather than 275x377, a
  sprite catalogue with 29 of the 81 sprites in it, and a `surface` that is
  neither of the two values `studio_status` promises. Every coordinate in the
  first stroke of a new skin meant something other than what the documentation
  said it meant.
- **A curve's own endpoints may be fractional.** Its control point always
  could, and so could every point of a path, because construction geometry is
  fractional -- a rib swept round an ellipse, a tail built at two sizes from
  one set of numbers. A curve is exactly where that geometry meets the rest of
  a drawing, and it was the one shape that had to be rounded at the join; the
  rasteriser turns all four corners into `f64` on its next line either way.
  `pixel_pen`'s own `taper` had been emitting a hand-built `path` to get round
  it since the day it was written.
- **A coordinate refusal names the field and the value.** `Coordinates must be
  integers` is true of eleven fields and specific to none of them, which is the
  rule the rest of this server already follows; it is `x is a whole number of
  pixels (got 1.5)` and `zoom is 1..8 (got 10)`.
- **`studio_targets` answers for the whole skin whatever is open.** Both
  catalogues were scoped to the surface, and with `studio_atlas` holding one
  sheet the sprite catalogue is a single pseudo-sprite called `sheet` -- so
  "where do this slider's twenty-eight frames live", which is a question only a
  recipe drawing that sheet at its own coordinates ever asks, could not be
  asked without leaving the sheet, which is what it was avoiding. A recipe that
  computes those rectangles instead lands all twenty-eight frames on frame 0.
  `studio_rectangles` stays scoped, because a source cell is a fact about the
  sheet it is on -- and both now say what they looked in when a filter matches
  nothing, rather than answering `{"sprites":[],"of":1}`, which reads as "no
  such sprite" and means "not on the sheet you have open".
- **A canvas rectangle is labelled with the variant it is actually showing.**
  The labels exist so that four rectangles for a switch say which is pressed
  and twenty-eight for a slider say which end is silence. On the assembled
  canvas every one of them was variant 0's: the volume track at frame 20 read
  `track · 0 · silent`, the balance track at centre read `hard left`, and the
  channel lamp drawn from its ON cell read `off · not this channel mode`. The
  canvas has one rectangle per sprite rather than one per variant, and the
  index of the one entry in that list is not the index of the variant in it.
  It is the mistake the labels were added to catch, made by the thing that
  reports them, and it was in the editor's own canvas hint under the pointer as
  well as in `studio_rectangles`.
- **`crossed_cells`: ink that left its own cell and landed in a repeated one.**
  A sheet is a bag of cells and crossing between two of them is often exactly
  what an artist means -- a band along a footer, a wash over a background. It
  is never what they mean when the cell on the other side is a *tile*. The
  playlist header's is 25 pixels wide and drawn nine times, so a caption four
  pixels too long for the title cell does not spill into empty sheet; it spills
  into the tile, and the player writes it nine times across the top of the
  window. Nothing reported it: every pixel of it is a legal part of some cell,
  and `overwrites` is a canvas-to-source measure that does not apply to a sheet
  open on its own. Cat Scan's playlist header read `CASE NOTES` with `SCAN SCAN
  SCAN` either side of it. The report names the operation, the cell and how
  many times the player draws it, and it is silent for a cell painted on its
  own.
- **`unsampled_pixels` asks about the skin, not about the preview.**
  `plbg.bmp` is 243x203 and tiles from the top, and the mask was built at the
  preview's current playlist height -- so painting the whole sheet, which is
  the correct thing to do, reported 28,188 pixels of ink nothing would ever
  show. At the tallest playlist the player uses all of it.
- **The state sheet's refusal names the call.** "Choose one sprite in Sprite
  targets first" is the right diagnosis and leaves an MCP caller with no call
  to make; the panel is a tool, so it says `studio_targets {"solo":"main.play"}`
  as well.
- **`studio_canvas` measures a word.** How wide a caption comes out is the one
  piece of the engine's arithmetic a recipe always had to reimplement, and all
  three that did -- Cardboard's, Sampler's, Cat Scan's -- reimplemented it the
  same way wrong: the pen advances `(cell + spacing) * scale` and every copy
  computed `cell * scale + spacing`, which agree at scale 1 and at no other
  scale. A caption measured one way and set the other runs past the end of the
  cell it was aimed at, and on a sheet with a repeating tile that error is
  drawn nine times. `measure` takes a word or a list of them and answers
  `width` and `height` -- the ink, which is what centring wants -- and
  `advance`, where the pen ends, which is what setting a second run after the
  first wants, from the same glyph walk that draws them. It names the
  characters the face does not have too, which used to arrive one stroke after
  it would have been useful.
- **The zoom cap moved from the factor to the pixels.** Eight was the ceiling
  everywhere, which bites hardest in the case it was meant to help: eight times
  a 14x25 equalizer handle is a 112x200 thumbnail, and judging one wants more.
  The documented answer was to read at 1:1 and enlarge nearest-neighbour at the
  caller's end, which for an agent is a file, an image library and two more
  round trips per look, in a project whose whole point is that a skin can be
  drawn without one. `magnify` is 1..64 on every image tool and is refused when
  the result would pass 2048 pixels a side, naming the largest that fits: a
  crop may go as close as it likes and the whole canvas may not.
- **Both catalogue filters take a list.** `id` and `sheet` accept one
  case-insensitive substring or several, so "where do these six transport keys
  live" is one call rather than six round trips for one question a recipe asks
  once per sheet.
- **`studio_rectangles` answers `at`: everything overlapping a box.** A flat
  list says where each rectangle is and nothing about what is next to what,
  which is the question an artist actually has -- *if I run a rail across this
  band, what does it cross?* The spectrum and the volume slider sit side by
  side rather than stacked, and Cat Scan's first main window put a steel rail
  the width of the window through the middle of the chest; the spectrum came
  out of a metal bar. `studio_inspect_region` could already answer it and is
  retired and unlisted, so the only way to find the capability was to know it
  was there.
- **`studio_screenshot` takes a `panel` and a `crop` together.** It took both
  and used only the panel: the crop was accepted, dropped, and answered with an
  ordinary-looking result, which is the quiet failure `additionalProperties`
  and the by-name refusals exist to stop. They compose now, and the crop is in
  the panel's **own native skin coordinates** -- the ones `studio_rectangles`
  answers in -- rather than in scene pixels, because the player may be at any
  zoom and working that out by eye is the thing `panel` was added to stop. So
  `{"panel":"main","crop":[14,86,146,22],"magnify":4}` is the live transport
  row, close up, asked for in the coordinates the catalogue gave you.

- **A blend against the transparency key is counted and named.** `#ff00ff`
  erases everywhere in this engine and the `color` field says so, but `opacity`,
  an `image`'s alpha and `material: "glass"` all read it as a colour. A warm glow
  laid into a cleared sprite cell comes back as a brown-purple ellipse, a glass
  jar drawn in one comes back hot pink, the cell quietly stops being transparent,
  and every other report is clean: the pixels are written, nothing is clipped,
  nothing is unsampled. It looks *plausible*, which is the worst kind of wrong,
  and a whole row of pressed transport buttons and two of eleven equalizer
  handles shipped that way before anybody looked at one at 9x. `keyed_blends`
  names the operation, how many pixels it did it to, and where. Nobody has ever
  wanted a colour blended toward magenta; composite the glow onto the colour the
  cell will be seen against and write it opaque.
- **`identical_variants` names only sprites the stroke actually wrote.** It asked
  "did this touch that sprite" with one bounding box per sheet for the whole
  transaction, and a classic sheet is a bag of cells scattered over it -- one
  transaction per sheet is the ordinary way to draw one, so the union box is most
  of the sheet most of the time. The equalizer's PRESETS plate and its close key,
  drawn together, made a box that swallowed the ON and AUTO cells in between and
  reported both as having duplicate variants that the stroke had not written a
  pixel of. Either half of the same transaction on its own reported nothing,
  which is what made it hard to believe: the report appeared when you *added* an
  unrelated operation. The box is the cheap pre-filter now and the writes are the
  answer.
- **Sprites that share their source cells share one report.** The equalizer's
  eleven bands all draw their groove from the same rectangle, so one duplicate
  frame arrived eleven times over, each with its own copy of all twenty-eight
  labels: four and a half kilobytes of JSON for one fact. It is one entry now,
  with `also` naming the other ten.
- **Setting the pencil does not close an open atlas.** `studio_canvas` put the
  editor on the canvas whatever was asked of it, so setting the brush's face --
  or asking `measure`, which is documented as answering *instead of* reading the
  canvas back -- moved the pencil off the sheet a recipe had open. The next
  stroke then landed on the joined canvas at the sheet's own coordinates and
  succeeded: 2,968 pixels of mono and stereo lamp went across the middle of the
  main window, every count looked right, and the sheet came back empty. The
  editor's own tool column does not close **Skin atlases** when you pick a colour
  in it, and MCP and the GUI disagreed. The brush settings and `measure` are the
  pencil rather than the canvas; asking anything of the canvas still goes back to
  it, and so does an empty call.
- **The `text` operation's own schema had the advance formula wrong.** It said
  `cell*scale+spacing` -- which is the mistake `measure` exists to stop, printed
  in the one description a caller reads while writing a `text` operation, and
  right at scale 1, so it read as confirmation. The pen advances
  `(cell + spacing) * scale`.
- **A refusal over unsaved edits names the argument that discards them.**
  "Export or undo unsaved edits before creating a blank skin" leaves a caller
  exporting a half-drawn skin or undoing ninety strokes one call at a time;
  `discard: true` was in the description and not in the refusal, and the refusal
  is what arrives at the moment the question is asked. All three of
  `studio_new`, `studio_open` and `studio_project` say the call now.

- **`studio_pixel` answers the artwork, not only the mapping.** It said which
  sprites are under a pixel and where each keeps it, which settles a mapping;
  the thing a stroke actually has to be chosen against is the *colour* that is
  there, and answering that meant rendering a crop and looking at it -- a file
  and two round trips for a number the document was already holding. It takes a
  `width` and `height` now and answers `ground`, the artwork averaged over the
  rectangle's opaque pixels, and with `ink` the export check's own WCAG
  arithmetic pointed anywhere: `contrast` and `readable`. That check covers the
  eight readouts Cranamp writes and nothing else, which in a skin made of
  hand-drawn marks on hand-drawn artwork is most of nothing. A skin whose whole
  grammar is "how far is this from a flame" hand-passed the answer to every one
  of its cats for a whole build.
- **The transparency key is not a colour to `ground_under` either.** The same
  `#ff00ff` that erases everywhere else was being averaged into the ground as
  magenta, so asking what is under a control -- every one of which is a mostly
  cleared cell -- answered `#963384` and, with it, the wrong answer about
  whether the mark on it reads. The readability check never hit it because its
  eight rectangles sit on backgrounds; the probe hit it in its first minute.
- **`studio_pixel` says which surface the coordinates meant.** Asked about a
  canvas pixel while a sheet was open it answered `{"hits": []}`, which reads as
  "no sprite there" and means "not on the surface you have open" -- the same
  sentence the two catalogue tools were fixed for. It answers `surface` like
  every stroke does, and when there is genuinely nothing there it says what it
  looked in and how big that is.
- **`clip` is the pencil, so it can be set without leaving the sheet it is
  for.** On the canvas `layers` clips a stroke to named sprites and reports how
  much fell outside; in `studio_atlas` there are no sprites to name, so a pool
  of light spills into the cell beside it and only `crossed_cells` catches it,
  and only when the neighbour is a repeated tile. Clipping painting to one cell
  is the answer, and reaching it meant leaving the sheet, which is the one thing
  it was wanted for. `clip` and `all_states` say where a stroke goes rather than
  what the canvas shows; they moved with the brush settings.
- **The history is seekable by the cursor it answers with.** Every entry carries
  the label its transaction was given, and the only way back to one was
  `studio_undo` with no arguments, called a guessed number of times.
  `studio_history {"cursor": 12}` is what the History drawer does when a row is
  clicked; it existed as an unlisted tool and so was invisible to anything
  reading the tool list.
- **`unsampled_pixels` counts ink, not clearing, and not sheets nothing
  draws.** It is "ink nothing will ever show", and it was counting two things
  that are neither. Clearing a sheet to the transparency key before painting it
  is how a recipe starts, and it reported every gutter between every cell --
  117 for `numbers.bmp`'s eleventh cell, every time. And `text.bmp` is read for
  one colour and never drawn, so painting it correctly reported all 2,790 of its
  pixels, for ever; that is a fact about the sheet, the sheet already has a note
  saying it, and the result says `sheet_is_never_drawn` instead of a number. A
  band that loses `pledit.bmp` column 125 still reports 38.
- **The two switches that are invisible until something else draws.**
  `active` and `pressed` choose the variant a stroke lands in, which is the
  cleanest thing in this server and was written down nowhere: with them, a
  four-state switch is the same code run four times, clipped and reported by
  `layers`, and without them it is four rectangles worked out by hand in an
  atlas where nothing clips and nothing reports. And `visualizer_glass` off
  makes the player fill the spectrum's whole rectangle with the first VISCOLOR
  entry -- an opaque box in the middle of a dark main window that no sheet
  contains and no canvas render shows. Both say so in their own schema now, and
  `eqhandles.bmp`'s arrival note says what it does **not** change: the eleven
  band tracks still share one set of 28 cells, so that sheet is the only place
  eleven bands can differ from each other.
- **`cranamp --skin-studio` says it is up, and where.** Every recipe in this
  tree runs against an editor that has to already be running, and the process
  wrote nothing at all -- so a log that is empty because the editor never
  started looked exactly like a log that is empty because it started perfectly,
  and the first one is a build about to fail in forty lines of somebody else's
  stack trace.

### What each tool answers with

The two catalogue tools are different shapes on purpose: one entry per sprite,
one entry per variant. Nothing said so, and both keys had to be discovered by
printing the JSON.

| Tool | Answers |
| --- | --- |
| `studio_status` | the document: `path`, `revision`, `dirty`, `undo`, `redo`, `message`, `canvas`, `surface`, `sprites` (a count), `sheets` (name, width, height) and the whole `view` |
| `studio_canvas`, `studio_atlas` | the same, without `sheets`; with `path`, `{view, path, size}`; with `measure`, `{measured, face, scale, spacing}` and `unsupported_characters` |
| `studio_targets` | `{sprites, of, chosen}` -- one entry per sprite, for the whole skin whatever surface is open: `id`, `sheet`, `source`, `states`, `drawn`, and with `variants: true` also `variants` and `labels` |
| `studio_rectangles` | `{rectangles, of}` -- one entry per **variant**, narrowed by `at`, `sheet`, `id`, `runtime` or `hit`: `id`, `label`, `sheet`, `rect` (where it is drawn), `source` (where it lives), `variant`, `active`, `runtime`, `hit` |
| `studio_draw` | `pixels_written`, `bounds`, `clipped_pixels`, `overwrites`, `overwrite_sample`, `unmapped_pixels`, `unsampled_pixels`, their samples, `surface`, `revision`, and when there is something to say `unsupported_characters`, `identical_variants` (one entry per set of shared cells, with `also`), `keyed_blends`, `crossed_cells` and the sheet's `note` |
| `studio_states` | the contact sheet, plus `sprite`: `id`, `sheet`, `of`, and per variant `index`, `label`, `rect`, `painted_pixels`, `differs_from_previous`, `largest_channel_change`, `same_picture_as` |
| `studio_pixel` | `surface`, `hits` (every sprite under the pixel, where each keeps it, its `rgba` and what shares it), `ground` for the rectangle, `contrast`/`readable` with `ink`, and `nothing_at` when there is no artwork there |
| `studio_options` | every option, `readability` (per readout: `reads`, `ink`, `ground`, `contrast`, `readable`), and `sheets_changed` when one added or removed a sheet |
| `studio_screenshot` | `path`/`size` or the PNG, plus `showing` (`player` or `editor`) and `player` (`x`, `y`, `zoom`); `panel` and `crop` compose, and `magnify` enlarges |
| `studio_export` | `path`, `bytes`, and when there is something to say `undrawn_sprites` and `hard_to_read` |
| `studio_layers`, `studio_history`, `studio_project` | the planes, the history (seekable with `cursor`), the project file |

Every image tool takes `path` and answers with where it wrote rather than the
bytes; omit it and the PNG comes back inline. Every one of them also takes
`magnify`, 1..64, which enlarges the returned image instead of `zoom` and is
capped at 2048 pixels a side rather than at a factor.

The older tools -- `studio_state`, `studio_render`, `studio_guides`,
`studio_paint_layers`, `studio_layout`, `studio_patch`, `studio_inspect_region`,
the one-off skin switches and the palette pair -- still answer, so existing
scripts keep working, but they are no longer offered: they address single windows
and an isolated patch handoff, which is not how this editor edits.

## What runs where

The drawing engine is one crate on every platform, so the materials, the
reports and the history behave identically in a desktop window, in a browser,
and on Android: `grain`, `opacity`, palette ramps, the native brushes, the
shared undo, `unsampled_pixels` and the sheet notes are all the same code, and
the web build is checked with
`cargo check --target wasm32-unknown-unknown --no-default-features --features web,renderer-wgpu`.

Two things are not everywhere, and neither is new:

- **MCP is desktop and Android only.** The browser cannot open a listening
  socket, so a recipe cannot be replayed into the web editor. Everything a
  recipe does is reachable by hand there instead, which is the point of keeping
  the materials in the engine rather than in a Python helper.
- **`studio_screenshot` is the desktop preview's GPU capture.** Android supports
  the native editor image, atlas inspection and drawing; the browser has the
  live player itself.

The touch layout -- under 1140x820 logical points -- has Files, Tools, Layers,
Parts, States and Sheets. **Sheets** is the one-BMP detour, with **← The whole
skin** at the top of it the way the desktop's sheet list has: a skin started
from **New blank** is drawn sheet by sheet, and until that drawer existed this
layout could start one and had no way to finish it. A laptop browser is over
the threshold and gets the full desktop layout instead.

Anything the engine grows has to appear in both panels or it is invisible on the
two platforms that have no MCP to reach it with instead. **PAPER GRAIN** and
**STRENGTH** are in desktop Drawing tools and in the touch Tools drawer, and
both write the same view fields, so a value set in one shows up selected in the
other. So are **GRADIENT**, the glass lens' **BEVEL** and **REFRACTION**, and
the **Text** brush, which spent longer than any of them on the wrong side of
that rule: palette ramps, a bevel and a set label were engine capabilities from
the day they landed and no hand could ask for any of the three. Verified by building the web bundle, serving `dist/`, opening
Settings → Open Skin Studio in a browser at both layout sizes, and dragging a
coarse-grain stroke at half strength across the main window: 194 pixels, no
MCP, no Python. The same session also confirmed the playlist-fill report reads
in a browser -- a stroke through the empty list answered "162 pixels have no
bitmap source ... turn on List canvas to paint there".

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
- **Parts → Show part rectangles** outlines the sprites you have selected, the
  paint clip, the rectangles the player writes over, and the eleven it
  hit-tests and draws nothing for, and lists those eleven with their
  coordinates. It used to set a flag nothing on this layout drew: the overlay
  lives on the desktop canvas and the touch canvas is a bitmap with a pointer
  surface over it, so the switch was on and nothing happened. Outlining all
  ninety-odd cells is not the answer either -- the desktop stopped doing that
  because the hairlines buried the picture they pointed at, and there is no
  pointer here to hover one with.
- **−/+ zoom** changes the integer number of screen pixels per skin pixel.
- **Layers** selects, adds, locks or hides painting planes. **Parts** selects any
  combination of sprite targets, shows their rectangles, and enables all-state edits.
- **Tools** selects brushes, width, fill, preset colors or an exact HEX color,
  and carries the same gradient, glass bevel and refraction, and text brush
  controls the desktop panel does.
- **Sheets** opens one BMP on its own, with **← The whole skin** at the top of
  the list.
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

A third kind of rectangle has no sprite and no source cell at all: the eleven
controls Cranamp hit-tests in the playlist footer -- `ADD`, `REM`, `SEL`,
`MISC`, `LIST` and the six transport keys -- and the main window's skin-chooser
corner. The artist has to draw a button in each, and nothing in the editor used
to say where they were. They are guides now, with `hit: true`, so the canvas
outlines the one under the pointer and names it, and the touch layout's
**Parts** drawer lists them with their coordinates. They move with the
playlist's height like every other footer rectangle. Selecting one clips
painting to it rather than targeting a sprite that does not exist.

MCP: `studio_rectangles` answers `rectangles`, narrowed by `sheet`, `id`,
`runtime` or `hit`; `{"select":"main.play#1"}` selects a specific atlas cell.
`studio_canvas` carries `guides` and a nullable `clip`.
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

MCP `studio_layers` supports `list`, `add`, `select`, `set`, `move`,
`merge_down`, and `delete`. `studio_targets` selects `paint_layer` by ID (`null`
selects the base). Layers are bottom-to-top, with move index zero above the base.
Opacity is 0–255. Locked planes reject drawing. Hidden planes do not export.

**Clip to layer below** constrains a shading plane to the effective alpha of
its immediately lower painting plane. Hidden or transparent pixels below hide
the clipped paint, and opacity participates in the mask. The source pixels are
retained: switching clipping off reveals them again. Clipped chains are supported.
For the bottom painting plane the original atlas provides the mask. Reordering
or deleting layers changes which plane supplies the mask.

Use `studio_layers {"action":"set","id":"paint-6","clip_below":true}`
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

MCP: `studio_canvas`. Layer IDs are qualified, for example
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
the assembled panel. MCP: `studio_atlas {"sheet":"text.bmp"}`, and `path`,
`crop` and `zoom` on the same call read the sheet back as a PNG. This also supports painting the previously unreachable
font, status and metadata-associated art sheets.

**Unique EQ art** (Equalizer tab) enables eleven independent 14×25 handles in
`eqhandles.bmp` (154×50: eleven columns, normal above pressed). It reserves 25
pixels of the 63-pixel track and limits travel to 38. Each `bandN.thumb` can be
selected, painted, grouped, and undone independently. MCP: `studio_options`
with `eq_handles`.
Disabling this returns to the classic shared 11×11 head mapping. The feature button sits above the canvas.

**Glass visualizer** (Main player tab) makes unlit visualizer pixels transparent,
revealing the artist's native background while retaining the live spectrum.
MCP: `studio_options` with `visualizer_glass: true`.

**Selection artwork** (Playlist tab) enables a native 243×11 `plselection.bmp`.
Edit `list.selection` on the assembled canvas or its atlas. The runtime reserves
an eight-pixel marker gutter before track titles, keeps durations aligned, and
uses the artwork for each selected row. Wider windows crop the native art at
243 pixels and retain the selection palette beyond it; pixels are not stretched.
MCP: `studio_options` with `playlist_selection`.

All these operations share human/MCP history and survive WSZ export.

## Player preview, and what a shared cell cannot do

**Player preview** runs the actual Cranamp main, equalizer and playlist
composables against the current document, at integer zoom and pan; 1x shows the
whole stack. **Tall playlist / Compact playlist** switches the live preview
between 261 and 145 native pixels, and `studio_canvas` takes
`preview_playlist_height` anywhere from 145 to 522. **Presentation** shows the
entire live stack at the largest fitting integer zoom, up to 2x, which
`presentation: true` also does. The Pressed and On/Off selectors change the
preview's artwork through preview-only copies, leaving the document and the
export untouched; real pointer input still works, and equalizer faces take the
same discrete frame as their bodies even mid-drag. Return to canvas editing to
paint.

The editing canvas assembles skin bitmaps only -- no runtime text, no
visualizer. `studio_screenshot` waits for the requested document revision to be
composed, the live player's atlas reload included, so a client never needs a
sleep after painting.

**List canvas** enables an editable `list.background` plane and exports a native
243x203 `plbg.bmp`, cropped and tiled by the player without stretching.
**Travel** on the Equalizer tab reserves room for taller fader artwork and
shares the undo history; the default is 52 pixels, and Moonpool uses 38.

Export validates through Cranamp's own skin loader and writes atomically. The
file you opened is not touched unless you export over it. Export also marks the
saved state, which undo and redo can return to; the history keeps 32
transactions for the session, and a new edit after an undo replaces the redo
branch.

Some pixels cannot be told apart, and no editor setting changes that. A shared
classic atlas region is one cell wherever it is drawn: **Unique EQ art**
separates the eleven equalizer heads into cells of their own, and nothing
separates the four timer digits or the nine playlist header tiles.
`studio_pixel` and `studio_rectangles` name those mappings, and `overwrites` in
a draw result catches a transaction writing one of them twice.

A sheet is not a picture either. It is a bag of cells, and the space between
them is never drawn: `pledit.bmp` column 125 falls between the two footer flaps,
and most of `titlebar.bmp` is shade-mode art Cranamp does not use. Ink that
lands there is reported as `unsampled_pixels`, with coordinates, because
otherwise a band painted straight across a sheet quietly loses part of itself.

## Connecting a client

Start the native studio, then point an MCP client at the stdio relay:

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

The running studio also exposes the same JSON-RPC at
`http://127.0.0.1:18765/mcp`. It binds only to loopback, rejects browser Origin
headers, and accepts at most 8 MiB per request. Run one studio for that
endpoint; the stdio relay connects to that same native window rather than
opening a document of its own.

For direct development calls the thin client takes a tool name and its arguments
(Node 18+):

```sh
node tools/skin-studio/client.mjs studio_status
node tools/skin-studio/client.mjs studio_targets '{"id":"band0.track","variants":true}'
node tools/skin-studio/client.mjs studio_draw \
  '{"operations":[{"op":"line","x":41,"y":90,"x2":51,"y2":90,"color":"#d5f2fa"}]}'
node tools/skin-studio/client.mjs studio_canvas \
  '{"zoom":4,"crop":[16,88,23,18],"path":"/tmp/play.png"}'
node tools/skin-studio/client.mjs studio_undo
```

Ask for an image with a `path` and you get the path and its size back; ask
without one and the whole PNG arrives inline as base64, which for a full
`studio_screenshot` is around 420 KB.

Coordinates are always **native pixels** -- of the assembled canvas, or of the
one sheet `studio_atlas` has open -- never enlarged GUI pixels. Every
`studio_draw` result names which of the two it painted, because one pencil and
one history serve both and a stroke aimed at the wrong surface still succeeds
somewhere else. `studio_pixel` maps either direction and is the way to settle an
ownership question rather than derive one: the play button's canvas rows 90 and
108 are `cbuttons.bmp` x=25..35 at y=2 and y=20.

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

**Drawing tools** now includes native pencil, line, rectangle, ellipse, curve,
fur, glass, stamp and **text** brushes, solid widths, filled shapes,
**gradients** and reflection about the canvas axes. Shapes preview
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

### Three things the engine could do and no hand could ask for

The rule is that anything the engine grows has to appear in both panels or it
is invisible on the two platforms with no MCP to reach it with instead. Three
capabilities had been on the wrong side of it since the day they landed, and
every one of them is the difference between artwork and a flat fill:

- **GRADIENT.** The engine has taken exact palette ramps from the start and
  neither panel could ask for one, so every gradient in every skin so far came
  out of a Python recipe -- and a flat rectangle was the only thing a person
  drawing by hand could make. **Off / Down / Across** with a second colour
  beside it: a filled shape runs from the brush colour to that one, along the
  axis of the drag. Both panels have it; `studio_canvas` takes `ramp_to` and
  `ramp_axis`, and an operation's own `ramp` still overrides everything.
- **BEVEL and REFRACTION.** The glass lens has been 1..128 and 0..32 over MCP
  and fixed at a drag-derived bevel and a refraction of four for a hand stroke,
  so a person got exactly one glass. Both are pills now, with **Drag** keeping
  the old behaviour of taking the bevel from the height of the gesture.
- **The text brush.** `text` was an operation with no brush at all, which meant
  every set label in a classic skin -- `MONO`, `AUTO`, `PRESETS`, the wordmark,
  the eleven equalizer band captions -- was reachable only from a script. Pick
  **Text**, type the word, choose the 5x7 face or the 4x5 small-caps one and a
  scale, and click the canvas. It is a shape like any other, so it takes the
  brush colour, the mirrors, the masks, the grain and the gradient with it, and
  one glyph walk serves both the brush and the operation because two would
  drift.

**Lock transparent pixels** protects empty source pixels while shading. **Mask
picked color** locks painting to the current brush color; choose the replacement
color afterward. **Clear color mask** restores unrestricted color painting. These
protections apply separately to each selected source variant, so all-state shading
cannot accidentally fill transparent space in another frame. MCP uses
`studio_canvas` fields `alpha_lock` and `mask_colors`; `studio_draw` also accepts a
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

MCP `studio_canvas` accepts `brush: "curve"` / `"tuft"` and `curve_bend`. Drawing
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

Drawing tools → **Clean 1px corners** is shared with MCP `studio_canvas` and
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

## Catamp Cardboard — a skin drawn from blank through this editor

`assets/skins/Catamp Cardboard.wsz` is the first Catamp built from **New blank**
rather than from another Catamp, and the whole of it is one deterministic
recipe rather than a stroke journal:

```sh
python3 tools/skin-studio/catamp_cardboard.py             # every sheet, then export
python3 tools/skin-studio/catamp_cardboard.py main eq      # one stage at a time
```

- `cardboard_material.py` — the kraft, the cut edges, the flute core, the tape,
  the rubber-stamp lettering, the pools of light, a 4x5 face for the one 29-pixel
  cell the editor's 5x7 face cannot label, and seven-segment timer digits.
- `cardboard_cats.py` — the cats, as hand-authored pixel grids. Grids are
  right-padded on import and checked against the palette, so a row typed a pixel
  short is not a defect to hunt for later.
- `catamp_cardboard.py` — one stage per sheet, each drawn in `studio_atlas` at
  that sheet's own native coordinates.

None of it needs an image library. The grain and the light were composed in
Pillow first and stamped in as PNGs, which is what `grain`, `opacity` and a
palette ramp are for; a motif is proofed with `studio_draw {"preview":true}`
rather than by a second rasteriser written in Python. What is left in Python is
what should be: the shapes, the palette, and the order they go down in.

The skin is one corrugated box with windows cut in it. Both light rules hold
everywhere: the outside of the box is lit from above-left, and the inside is lit
only by the glow that leaks out of it -- which is why every readout Cranamp
draws live sits inside a cut window, where warm light on dark board beats ink on
kraft. The cats are where cats in a box are, which is mostly out of sight: one
asleep beside the transport keys, eleven heads through eleven slots in the lid,
a kitten walking the seek tape, a loaf on each of the volume and balance
grooves, one sitting on the playlist scrollbar, a tail down the left fold, and
one so far back in the box that only its outline and its eyes catch the light.

Three things about the classic format cost a redraw each and are worth knowing
before the next skin from blank:

- **`text.bmp` is not a glyph sheet here.** Cranamp renders titles with its own
  5x7 face and reads this sheet only to sample the display ink -- the second most
  common colour, when two thirds of the sheet is opaque. Paint it as the glyph
  sheet it looks like, in the ink the readouts should use.
- **The playlist footer is two cells a pixel apart.** `bottom.left` is sheet
  0..124 drawn at canvas 0..124; `bottom.right` is sheet 126..275 drawn at canvas
  125..274, and sheet column 125 is never drawn. Nothing continuous may cross
  that seam, so the footer is two flaps meeting at a fold -- which is what the
  bottom of a box looks like anyway. `studio_pixel` settles the mapping; doing
  the arithmetic by eye lands a button a pixel out of step with its own hit area.
- **The header tile is 25 pixels wide and repeats nine times.** Any pattern with
  a period that does not divide 25 shifts at every repeat and reads as a row of
  seams; the side rails repeat every 29 rows, so their flutes run *across* the
  rail and are constant down it, and meet themselves exactly at every phase.

Verification: `studio_export` validates through Cranamp's own loader, and
`check_native_frames.py` swept all 112 live GPU states -- active and inactive,
released and pressed, all 28 slider frames -- with no nonuniform 2x2
source-pixel block, so every sprite samples at exact native pixels.

## Catamp Sampler -- a skin sewn rather than built

`assets/skins/Catamp Sampler.wsz` is the second Catamp drawn from **New blank**
through this editor, and it is one deterministic recipe rather than a stroke
journal:

```sh
python3 tools/skin-studio/catamp_sampler.py             # every sheet, then export
python3 tools/skin-studio/catamp_sampler.py main eq     # one stage at a time
```

- `sampler_cloth.py` -- linen, indigo-dyed cloth, felt, ribbon, twisted cord,
  running and blanket and cross stitch, hems, patchwork seams, sewn buttons,
  and the counted chart for a timer digit.
- `sampler_cats.py` -- the cats. Two ways of making one, and the size decides
  which.
- `catamp_sampler.py` -- one stage per sheet, each drawn in `studio_atlas` at
  that sheet's own native coordinates.

Every other Catamp is made of something hard -- glass, silver, crystal, kraft
board -- and every one of them has a light to glint off. Cloth has none, so
depth comes from three things and only those: a seam casts one pixel of shadow
and the thread beside it catches one of light; a raised thing sits on its own
cast shadow and a pressed thing goes darker all over, because it is now level
with the cloth and in the cloth's shadow; and a run of floss is two values,
never one, because the twist turns over along its length. The grammar is that
no edge is a plain line. A hem is a running stitch, a felt patch is
blanket-stitched down, a patchwork join is a seam with topstitching either
side -- which is what makes eleven different objects read as one piece of
sewing.

Three of the skin's controls say something with the twenty-eight frames a
classic slider track has and a handle cannot:

- **Volume is a sampler band being worked.** Sixteen squares of an evenweave
  chart; the frame says how many have been stitched, and the unworked ones are
  four holes of the weave with nothing in them.
- **Balance is the same band worked outward from the middle**, so centre reads
  as centre with no mark to say so.
- **An equalizer band is worked from its handle down to the bottom of the
  groove**, so a boosted band has more colour in it than a cut one -- and the
  colour runs from cool at full cut to warm at full boost.

The seek bar is a length of yarn couched to the cloth with a stitch every eight
pixels, and the handle is the ball it came off, with its loose end trailing
behind it: the part of the thread that has been unwound is the part that has
been played.

The cats are made two ways, and the size decides which. A big cat is
*embroidered*: a filled silhouette and then thirty or forty tapered strokes
laid along the form in four values, which is what long-and-short satin stitch
is, and which `tuft` -- a curve narrowing to a single-pixel tip -- is exactly
one stitch of. The construction points are fractional, so the same cat is built
at two sizes with no resample anywhere. A small cat is *counted*: a
hand-authored grid checked against its cell and its palette before an operation
is emitted. The embroidered construction was tried at handle size first and does
not survive it -- at fourteen pixels the ears fall outside the cell and the face
has nowhere to sit but the top row. Below about twenty pixels there is no stitch
left to taper.

The equalizer turns on **Unique EQ art**, so the eleven bands get handles of
their own rather than the classic shared head: one kitten grid relit by eleven
coats and eleven collar colours, which costs no more than one kitten and gives
a litter instead of a pattern. A twelfth cat watches them from the gap the
preamp leaves, and the mother of the whole thing sits in the main window's left
margin watching the spectrum.

Five things about the classic format cost a redraw each here, and three of them
were already known:

- **A slider frame is drawn at its own y, not at the sheet's.** Twenty-eight
  frames of cross stitch all landed on frame 0, which is a band that never
  changes and a mistake with no error to report it: the ink went somewhere
  legal.
- **The playlist footer's eleven buttons have no sprites.** They are
  `studio_rectangles {"hit":true}` now; before that they were a hard-coded
  table copied from the player's source, and a sleeping cat went straight over
  the elapsed-time readout.
- **A dyed slider on dyed cloth is a slider; a pale one on pale linen is a
  texture.** The first volume band was worked straight onto the linen and the
  whole control disappeared into the ground it was sewn to.
- **A pressed felt patch has to change value, not just its bevel.** Flipping
  the one-pixel highlight and shadow is two pixels of difference across a
  23x18 button, which is no difference at all; a pressed patch is level with
  the cloth and in its shadow, so it goes darker all over.
- **The playlist header's two rows are the opposite way round from the main
  window's.** Cranamp draws the lower one always -- there is no unfocused
  playlist -- so the whole worked header sat in the row nobody sees, and both
  rows look right on their own. `studio_targets` labels say which is which now;
  before they did, nothing in the editor did.

Verification: `studio_export` validates through Cranamp's own loader, and the
whole recipe replays from **New blank** into an empty document in fifteen
seconds with no image library and no external asset.

## Catamp Cat Scan -- a skin with the light behind it

`assets/skins/Catamp Cat Scan.wsz` is the third Catamp drawn from **New blank**
through this editor, and it is one deterministic recipe rather than a stroke
journal:

```sh
python3 tools/skin-studio/catamp_cat_scan.py          # every sheet, then export
python3 tools/skin-studio/catamp_cat_scan.py main eq  # one stage at a time
```

- `catscan_film.py` -- the backlit film, the brushed steel of the case, the
  slots of bare diffuser, the clips and the tape, grease pencil, dust, the two
  kinds of window Cranamp writes into, and the density ladder everything in the
  skin is coloured from.
- `catscan_cats.py` -- the cats. Two kinds, and where a cat is decides which.
- `catamp_cat_scan.py` -- one stage per sheet, each drawn in `studio_atlas` at
  that sheet's own native coordinates.

The player is a vet's lightbox at two in the morning, with somebody's cat's
films clipped to it. Every other Catamp is lit from above-left -- silver
glints, kraft catches a key light, felt sits on its own cast shadow. This one
has no key light at all: the only light in the skin is *behind* the picture,
and everything visible is something standing in front of it. That inverts every
rule the other skins are drawn by and gives this one four of its own.

- **A bright pixel is a thin one.** Value is density, not illumination. Air is
  black, soft tissue is a broad dim haze, bone is bright and a swallowed staple
  is pure white. Nothing is lit; things are only more or less in the way.
- **An opaque thing has no interior.** The clips, the tape, the grease pencil
  and the cat sitting on the box are flat silhouettes, and the only thing that
  describes their shape is the light leaking past the edge. Shading the inside
  of a silhouette is what turns it into a sticker.
- **Bone is three values and a haze, never one.** The haze goes down first and
  is wider than the bone; then the spongy middle, a speckle one step up; then
  the dense rind, one step up again and only on the outline.
- **Pressed means pressed against the light.** Everywhere else in Catamp a
  pressed control goes darker. Here a chip pushed flat against the diffuser
  loses the shadow it was floating on and the light behind it comes up -- 350
  of a play button's 414 pixels change, with a largest channel change of 226,
  where flipping a bevel would have moved two.

And one more, which is the only warm colour in the skin: the light through a
cat's ears. Everything else is the green-grey of a fluorescent viewer behind a
sheet of polyester, and the ears are the single place the beam passes through
something alive.

Four controls say something with their frames that a handle cannot:

- **The equalizer is eleven things the cat has eaten**, in the order they came
  out: a hair tie, a bottle cap, a spring, a bead, a bell, a paperclip, a
  screw, a button, a battery, a brick and a milk-jug ring, with a fish skeleton
  on the preamp because that one is not the cat's fault. Each band's track is a
  length of gut filling with contrast from the bottom up to its own handle, so
  a boosted band has more in it than a cut one. Eleven copies of one head is a
  pattern; eleven different objects is a story, and **Unique EQ art** is what
  buys the eleven cells.
- **Volume is a step wedge** -- the aluminium staircase a radiographer puts in
  the beam to calibrate an exposure. The level is how many of its thirteen
  steps the beam has got through, so silence is a strip of unexposed film.
- **Balance is the tail.** Six caudal vertebrae leaning together, upright in
  the middle, so centre reads as centre with no mark to say so.
- **The seek bar is a spine survey with a loupe sliding along it**, nose at the
  left and tail tip at the right, and the part already read ringed in wax. It
  is the one piece of glass in a skin made of film and steel, so it is the
  engine's own glass material.

The cats are made two ways and where a cat is decides which. **Inside a film** a
cat is an anatomy: layers of density seen through each other, composed out of
discs, arcs and wedges on a character grid, deepest first. Freehanding one
produces a ring -- the first skull drawn for this skin was one -- because the
hand draws the outline it can see and leaves the inside empty, and on a
radiograph the inside is the picture. **On the box** a cat is a silhouette with
no interior at all, and the ears are cut into its outline rather than laid over
it, because a silhouette with two triangles on top is a cat wearing a hat. The
cat sitting in the equalizer was drawn on the film first, where it was a
perfectly correct black silhouette of nothing; the gap the preamp leaves is
bare diffuser now, so there is a light for it to sit in front of.

Everything Cranamp writes live is written in the brightest colour in the skin
onto the blackest -- `#e3f2e8` on `#0a1112`, sixteen to one. The first draft put
the readouts on clear strips of film with black lettering, which is how the
*printed* parts of a film read and would have put a glaring white block in the
middle of every dark window; the contrast checker passed it and the room did
not. Black on a clear strip is kept for the parts the vet printed.

The recipe measures its own captions against the engine before it exports: 59
words, every one of them handed to `studio_canvas {"measure": [...]}` and
checked against the arithmetic the recipe lays them out with, so the two cannot
drift again in the direction they already drifted once.

Five things about the classic format cost a redraw each here, and two of them
were new:

- **The main title cell is sheet `[27,0,275,14]`, not `[0,0,...]`.** The four
  window keys live in the 27 columns before it, so a bar drawn from the sheet's
  own left edge arrives 27 pixels to the left of where it is seen and loses its
  last 27 columns -- and nothing reports it, because every one of those pixels
  is a legal part of some cell.
- **Five of the six transport keys have their pressed cell one row of 18 below
  the released one, and eject does not.** Eject is 22x16 and its pressed cell
  starts at y=16. Assuming the stride put its pressed art two rows low;
  `unsampled_pixels` named the two rows that fell off the bottom and was the
  only thing that did. Every cell in the recipe is asked for now rather than
  worked out.
- **The playlist header tile is 25 pixels wide and drawn nine times**, so a
  caption that runs four pixels past the end of the title cell is repeated
  across the whole header. `crossed_cells` exists because of this one.
- **The playlist footer is two cells a pixel apart**, so nothing continuous may
  cross sheet column 125 -- which is why the case is two panels butted together
  with a fold between them.
- **A slider frame is drawn at its own y, not at the sheet's**, and the
  twenty-eight are at two different sheet rows with a stride that changes
  halfway. The recipe reads them from `studio_targets` rather than computing
  them.

Verification: `studio_export` validates through Cranamp's own loader and
reports no undrawn sprite and nothing hard to read; `check_native_frames.py`
swept all 112 live GPU states -- active and inactive, released and pressed, all
28 slider frames -- with no nonuniform 2x2 source-pixel block, so every sprite
samples at exact native pixels. The whole recipe replays from **New blank** into
an empty document in forty-five seconds with no image library and no external
asset.

## Catamp Seance -- a skin lit only by what is drawn in it

`assets/skins/Catamp Seance.wsz` is the fourth Catamp drawn from **New blank**
through this editor, and it is one deterministic recipe rather than a stroke
journal:

```sh
python3 tools/skin-studio/catamp_seance.py            # every sheet, then export
python3 tools/skin-studio/catamp_seance.py main eq    # one stage at a time
```

- `seance_room.py` -- the ladder everything in the skin is coloured off, the
  cloth, the pools of candlelight, flames, smoke, wax, salt, eyeshine and the
  dot.
- `seance_cats.py` -- the cats. Two ways of drawing one, and where the cat is
  decides which.
- `catamp_seance.py` -- one stage per sheet or per group of sprites.

It is seven minutes past three in the morning and the cats have got the spirit
board out. There are candles, because a cat will knock a candle over but will
not put one out. There is spilled salt, and the sigils are drawn in it, badly,
by a paw. And the thing they are calling up is the small red dot.

Every other Catamp has a light somewhere outside the picture: silver takes a key
light from above-left, kraft catches one, film has a whole fluorescent box
behind it. **Every light in this skin is in this skin** -- nine candle flames,
all of them drawn, all of them small -- and the rest of the grammar comes out of
that one decision.

- **Falloff is fast and it is coloured.** One step from a flame is cream, two is
  amber, three is a burnt brown, four is the room's violet and there is no
  fifth. A shadow here is never grey and never black. Everything is coloured off
  one twelve-step ladder and the only question ever asked about a pixel is how
  far it is from the nearest flame, so a rim light and the thing it sits on are
  a fixed distance apart rather than two colours picked by eye twice.
- **A bright side faces the flame that is near it, not the same way as its
  neighbour.** There is no global light direction to be consistent with. Two
  objects a hand apart are lit from opposite sides, and getting that wrong is
  what makes a candlelit picture look like a grey picture with an orange filter
  over it.
- **Eyeshine obeys none of it.** It is reflected rather than received, so a cat
  across the room has exactly the two discs the one beside the candle has while
  every other part of it has gone. That is why the dark half of this skin is
  nothing but eyes at different heights, and why it is the one colour in the
  file that is not on the ladder.
- **The dot is not a light.** Two pixels of flat red, no halo, no falloff, no
  shadow, and nothing it lands on gets brighter. Everything else here is warm or
  violet; it is neither, and it is the only thing in the room the cats can see
  properly and you can only nearly.

Salt is the other half of the grammar. Every mark a *cat* made is drawn in
spilled salt -- the sigils, the wordmark, the window keys -- and every mark is
therefore broken, because a salt line is a scatter of grains with a dust of it
either side and some of the path has none. Salt has a ladder of its own, and
that is not a detail: it reflects a flame rather than glowing with it, so it
brightens toward white while everything else brightens toward amber. Run it up
the warm ladder instead and a summoning circle comes out as a ring of embers.

The cats are made two ways and where a cat is decides which. **Near a flame** a
cat is a crescent: the whole animal is a hole in the light, filled one step
darker than whatever it is standing in front of, and the only drawn part is the
band of fur along the side that faces the candle -- brightest where the edge
turns square-on and gone by the time it turns away. `rimlit()` does that
arithmetic from the outline and the flame's position, so a cat put down beside a
different candle relights itself and cannot disagree with the room; there is no
call in that file that fills a cat with fur colour. **Away from every flame** a
cat is two eyes. Not a dim cat: no cat.

Six controls say something with their frames that a handle cannot:

- **Volume is how many cats have woken up and turned round to look at you.**
  Silence is a strip of dark with nothing in it at all and every step up wakes
  one more, somewhere in the room. It works because eyeshine does not fall off,
  so a crowd fits in a space thirteen pixels tall without any of it needing to
  be lit. The handle is the nearest one, and the only cat in the crowd with a
  head as well as a pair of eyes.
- **Balance is the saucer of milk, and it tips.** Centre needs no mark on it:
  milk sitting level is what level looks like. At each end the milk has gone
  over the rim and there is a wet patch on the cloth.
- **The seek bar is the board**, with the alphabet across it, YES at one end and
  NO at the other, and the planchette sliding along covering letters the way a
  planchette does. The dot is in its lens.
- **The equalizer is a mantelpiece with eleven different candles on it** -- a
  pillar, a taper, a birthday candle, a tea light in its tin, a stub in a bottle
  neck, a stick of incense, one in a jar, one shaped like a cat, a beeswax coil,
  a match, and one that has fallen over and is burning sideways -- with a cat
  sitting in the gap the preamp leaves, watching the big one burn.
- **Shuffle is three cups** with the dot under the middle one, and **repeat is a
  cat with its own tail in its mouth**: on is the ring closed and off is the same
  cat having let go.
- **The status lamp is an eye**: open while it plays, half shut while it is
  paused, and asleep when it is stopped.

Pressed is where this skin parts company with every other Catamp. Everywhere
else a pressed control goes darker -- felt sits down in its own shadow, a chip
pushes flat against a diffuser. Here the room is lit by small fires and a
pressed control is one that has **caught**: the salt goes to the top of its own
ladder and a light comes up inside the ring that is not there at rest. It moves
the whole cell rather than a bevel's four pixels, and it is the answer to the
question the cats are asking -- press play and the dot is in the circle.

Six things about this editor and this format cost a redraw each here, and four
of them were new:

- **`opacity` and `material: "glass"` read the transparency key as a colour.**
  A glow laid into a cleared cell came out as brown mud and a glass jar came out
  hot pink, and nothing said so. The pools that land in a sprite cell are
  composited against the colour the cell is seen against and written opaque;
  `keyed_blends` exists because of this one.
- **`studio_canvas` used to close an open atlas.** Setting the face before
  measuring a caption moved the pencil to the joined canvas, and both channel
  lamps were drawn across the middle of the main window at monoster's own
  coordinates.
- **Light does not fit its own cell.** The playlist header's tile is twenty-five
  pixels wide and the player draws it nine times, so a pool four pixels too wide
  for the cell it is lighting is repeated nine times across the top of the
  window -- `crossed_cells` named it. Every pool in a cell is now measured
  against the cell rather than against the thing it is lighting.
- **A control that fills flat reads as a hole.** The volume strip was painted in
  the room's own black, which is correct art and made the one obviously-a-sprite
  rectangle in a lit table. It carries the light the table carries there now: it
  starts two pixels from a candle and ends sixty-eight pixels away from it.
- **A quantised slider has frames that agree.** The milk's surface is a straight
  line rounded to whole pixels, so four frames either side of centre came out the
  same picture and a third of the control's travel did nothing;
  `identical_variants` was the only thing that said so. A glint that slides one
  pixel a frame cannot agree with itself, and the same fix -- wax spattered in
  proportion to the burn -- cured the three lowest equalizer frames.
- **`numbers.bmp` has ten cells and no eleventh for a colon**, so the timer's
  colon is painted on the window behind the digits like every classic skin does
  it. Without it the readout is `00 00`.

Two of the recipe's stages are questions rather than drawing. `measured` hands
every caption to the engine's own glyph walk and checks it against the cell it
is set in. `checked` asks `studio_pixel` the two things the first build guessed
at: what each cat is actually standing in front of -- a cat lit for the wrong
room is the one mistake here that no report catches, because the ink is correct,
in the right cell, in colours that are on the ladder -- and how well every
hand-drawn mark reads on the hand-drawn artwork under it, which is the same
arithmetic the export check runs over the eight readouts and nothing else.

Verification: `studio_export` validates through Cranamp's own loader and reports
no undrawn sprite and nothing hard to read -- every readout is between six and a
half and thirteen to one against its own artwork; `check_native_frames.py` swept
all 112 live GPU states with no nonuniform 2x2 source-pixel block, so every
sprite samples at exact native pixels; and the whole recipe replays from **New
blank** into an empty document in five seconds, byte for byte, with no image
library and no external asset.
