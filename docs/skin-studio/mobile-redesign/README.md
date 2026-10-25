# Skin Studio workspaces

## Control access audit

The [control audit](control-audit.md) records hidden settings and missing controls found in the adaptive UI. Skin settings now have a visible scope. The [v9 gallery](../../../target/ux-audit/v9/index.html) records the current pass. Older capture sets are superseded; the partial v7 refresh is not a consistent baseline.

## Review status, 25 October 2026

The previous layout pass was rejected by the user. The preview pass below is
under review. Screenshots and passing tests record implementation behavior; they
do not establish accepted design.
The access table below describes the intended budget. It is not proof that every
instrument satisfies that budget. The audit at the end identifies exceptions.

The editor uses five width classes on every platform, with an expanded desktop
arrangement at 1680 pixels. Selecting a tool shows its current settings beside the canvas.

| Logical width | Arrangement |
| --- | --- |
| Below 380 | Current drawing tools, pan and secondary tools below the canvas; current settings below them |
| 380–599 | More direct drawing tools below the canvas, including shape tools |
| 600–899 | Side tool rail; current settings across the bottom; named workspace tabs above |
| 900–1279 | Side tools; current properties above exact colour controls and layers |
| 1280–1679 | Window and source navigation; current properties at the top of the inspector |
| 1680 and above | Adds a column of rendered states beside the canvas |

Short windows reduce the direct tool selection to keep colour, layers, pan and
the tool menu visible. The remaining tools stay in the same menu. Height does not
change the width class. On phones, a tool that does not usually occupy the bar
replaces its last brush slot while active.

## Tool access

Counts start from the editing canvas. Zero means the control is already visible.

| Control | Small phone | Phone | Tablet | Wide tablet | Desktop |
| --- | --- | --- | --- | --- | --- |
| Current size/opacity, text/scale, bend or fill | 0 | 0 | 0 | 0 | 0 |
| Colour picker in Canvas | 1 | 1 | 1 | 0 | 0 |
| Layer controls | 1 | 1 | 1 | 0 | 0 |
| Tool settings | 1 | 1 | 1 | 1 | 1 |
| Choose a tool from the full menu | 2 | 2 | 2 | 2 | 2 |
| Save choices | 1 | 1 | 1 | 1 | 1 |
| History and guides | 2 | 2 | 2 | 2 | 2 |
| Skin workspace | 1 | 1 | 1 | 1 | 1 |

Windows shorter than 500 pixels use a workspace menu; selecting Skin then takes two taps. Skin sections take one further selection; narrow Colours pages have Playlist and Visualizer tabs. Direct brushes take one tap to select. Text fields accept typing immediately.
The text tool exposes its text and scale; shapes expose fill; curves expose bend;
stamps expose repeat. The tool menu closes after selection. When the toolbar stays visible, the menu lists only tools that are absent from that toolbar. More detailed
brush controls remain in Tool settings.

## Editing scopes

Canvas edits the assembled skin. Sprites edits a selected sprite or source
bitmap and keeps current brush settings visible. States compares the selected
sprite without paint controls; it gives that space back to the canvas. Preview
shows the working player and its visual states. Phone navigation uses separate
screens and panels; tablets and desktop use named tabs. Skin contains colour tables, bitmap font generation, window cutouts, cursor files and format checks.

Window choices use small previews. Desktop source and target rows show dimensions.
Target rows also show state counts. Asset lists use 48-pixel rows with rendered thumbnails. Most tool controls use
44-pixel targets. The 240-pixel window uses narrower Stamp transform controls;
colour swatches use 32-pixel targets.

## State, colour and exact values

Desktop shows the joined canvas state above the artwork. The Active/Inactive and
Released/Pressed buttons change that same canvas. Other widths show the current
state in the header and offer a Canvas state panel. Paint scope and frame controls
live there too; they are separate from the sprite comparison workspace.

The current tool has a named properties section. Size sliders cover their actual
accepted range: pencil width is 1 to 32 pixels. Fields beside the sliders accept
exact values. Invalid input leaves the previous value unchanged and shows a hint.
Curve controls expose width and bend; glass controls expose bevel and refraction.
The properties panel retains gradients and colour masks.

Colour entry accepts six HEX digits or RGB channels from 0 to 255. Saved colours
can be added, replaced or removed. They survive CSTUDIO save/reload and participate
in undo history. The colour picker has no second toolbar entry when it is already
visible in the inspector. Preview and Save each have one entry per screen.

## Canvas and feedback

Zoom controls sit outside the drawing surface. A control click cannot paint
beneath them. The canvas has scrollbars, anchored zoom and two-finger pan/pinch.
A second touch cancels tentative one-finger painting. Hand mode and middle-button
dragging pan. Hover does not paint.

The [40 SVG icons](../../../assets/studio/icons) share a 24-pixel view box,
1.65-pixel strokes and rounded joins. Pencil, curve and stamp have distinct shapes.
Selected tools expose selection through accessibility. The state icon has its own drawing, separate from the layers icon. Hints omit repeated text.
Modal panels hide the covered editor from accessibility navigation.

## Files and platforms

The picker opens WSZ, ZIP and CSTUDIO files. Cancelled or invalid imports preserve
the working document. Replacement saves edited work as a draft first. Project and
WSZ exports use the platform save service; sandboxed macOS saves use the selected
file directly.

Cranpose revision `d052dda2eefa396f008af86e8ae5a322cd57ee7b` includes Android
hover routing, native cursors and the scroll-panel disposal fix. It also reduces
thread-local key use to fix the Galaxy Tab S7 crash described below.

The normal debug build is installed on the connected Galaxy Tab S7. The device
regression and Android window configuration tests passed. S Pen hover, cursor
appearance and physical pinch behavior still need a device check. UIKit currently
ignores custom cursor shapes. Keyboard focus and safe areas still need physical
mobile review.

## Shared controls, reference pass

The supplied `Winamp Skin Studio Dark UI Mockup.png` and
`Cyber Blue Winamp Skin Studio.png` define the control styling for this pass.
The styles live in `src/winamp/studio/design.rs`. Controls use these shared
surfaces across platforms; the width rules still choose their placement.

| Token | Value | Use |
| --- | --- | --- |
| Background | #0B1018 | Workspace |
| Panel | #111A26 | Inspector, navigation, sheets |
| Control surface | #172437 | Buttons and fields |
| Selected surface | #19365E | Current item |
| Primary | #387AFF | Main action, selection edge, slider fill |
| Hover and focus | #7DDCFF | Pointer and keyboard feedback |
| Text | #E8F6FF | Values and labels |
| Muted text | #8CA6C4 | Dimensions and secondary labels |
| Error | #FF686B | Invalid input |

Control corners use an 8-pixel radius. Panel corners use 12 pixels. Borders are
one logical pixel. Selected controls add a second inset edge. Surface gradients
use a small brightness change to separate controls from their panel. Icon faces
have a two-pixel inset inside the existing touch target. List rows use the same
selection edge without a permanent box around each idle row.

Brush and layer sliders share a five-pixel rail and a 16-pixel thumb. The rail
fills through the thumb centre. Numeric fields retain exact entry and validation.
Switches keep their animated thumb. Save project is the primary action on the Save
screen. Undo and other unavailable actions expose their disabled state.

The native control sheet uses the production widgets. Run
`cargo run --features studio-review --bin cranamp-studio-review -- --design 1280 860`.
It covers buttons, editable fields, sliders, switches, file rows and tool samples.
The static interaction samples use the production surface renderer. The actual
controls accept pointer and keyboard input. Widths below 700 stack the groups;
700–1099 uses two columns; wider windows use three.

## Control review and Android crash, 25 October 2026

The [current screenshot sheets](../../../target/ux-audit/v9/index.html) contain
2,470 native GPU captures: 130 named control states at 19 window sizes. Each width
has seven sheets with links to the original captures. The catalogue covers every
brush in Canvas and Sprites, quick controls and full settings, minimum and maximum
brush values, fill modes, clipboard states, panels and workspaces. It also covers
layers, palettes, window states and mirror settings. Skin cases include both colour tables, font generation, window cutouts, format checks, and missing and populated cursor files. All 133 overview sheets were inspected, with selected full-size captures for detailed review. It is a finite catalogue;
it does not enumerate every possible number or combination of independent values.

The captured sizes are 240x568, 320x640, 379x740, 380x740, 430x820, 488x538,
599x800, 600x360, 600x900, 768x900, 899x900, 900x900, 1100x800, 1279x900,
1280x900, 1440x900, 1679x1000, 1680x1000 and 1920x1080. These include both sides
of each width boundary and a short landscape window.

The visual review found an overflowing equalizer frame grid at 240 pixels,
oversized history panels, wasted space in tablet sheets, and overlapping Pixel
study content at 600x360. The grid now keeps 44-pixel frame targets inside the
panel. History uses selectable rows. Parts, Tools, Colour and Layers sheets use
their content height. Pixel study scrolls when the window is short. Wider tool
settings use two columns; Lift and Stamp show the copied pixels.

Shared controls now animate hover, focus and press feedback. Selected colours
transition over 130 ms; switch thumbs move over 140 ms. Screens and sheets enter
over 180 ms. Notifications retain their surface through the exit transition.
Reduced motion is respected. Sheet dismissal remains immediate. The review
captures use reduced motion to show settled layouts; screenshots do not verify
the quality of movement.

The Tab S7 log reported `fatal runtime error: out of TLS keys, aborting`. The
symbolized stack reached the modal provider through `BasicTextField` in the
inspector. The framework change groups UI and theme provider values into shared
thread-local records and removes 15 OS key allocations from that path. Android
has a 128-key limit in the installed NDK.

The device regression opens the real file picker, cancels it with Android Back,
then enters the production workspace through a test root and renders all 130
control states. It passed in 133.278 seconds. A separate desktop regression
clicks Continue editing after cancellation and verifies the unchanged document.
The device test does not tap that button through the Android accessibility tree.

Validation includes 419 unit tests, 30 drawing tests and nine handset tests. The
two Android window configuration tests passed. Strict Clippy passed for all
host targets with the review feature. Later panel height corrections received
targeted checks and refreshed GPU captures. The final palette and cursor UI checks passed after the cursor row changes. Device logs and the coverage
manifest are beside the screenshot sheets.

The screenshot pass records specific fixes. It does not establish accepted
visual quality. Physical pen and gesture checks remain open.

## Rendered preview pass, 25 October 2026

The seven supplied screenshots were reviewed in Preview. They showed blank source
rows, large colour fields, repeated panel labels, and unused space around a
canvas that retained its old fitted scale after resizing.

Source files, skin windows, sprite targets and paint layers now show their actual
pixels. The active tool shows a sample produced by its paint engine with the
current settings. Lift and Stamp show the copied pixels, or the action needed to
create a copy. Tool choices have distinct SVG symbols and rendered samples.

The inspector groups the current tool above colour and layers. Idle icons have
no button box. Lists share their row height and text offsets. The desktop tool
menu opens beside the tool rail and keeps the inspector visible. A larger
desktop adds four clickable state previews. The Preview workspace uses those
same state choices. Sprite comparison uses normal text labels over exact source
pixels; it no longer enlarges bitmap numbers into headings.

A fitted canvas stays fitted after resizing. Fit can reduce below 50 percent in
short windows. Pixel rounding no longer creates a scrollbar on fitted content.
Manually zoomed canvases preserve the centre pixel when their size changes.

The [native screenshot gallery](native-review-v5.html) contains the captures.
The GPU review covers these actual window sizes:

| Size | Capture |
| --- | --- |
| 240 x 568 | [Small window](native-tiny-window-v5.png), [tool choices](native-tiny-tools-v5.png) |
| 320 x 640 | [Small phone](native-small-phone-v5.png) |
| 430 x 820 | [Phone](native-phone-v5.png), [source files](native-phone-sources-v5.png), [colour](native-phone-colour-v5.png) |
| 600 x 360 | [Short landscape](native-short-window-v5.png), [tool choices](native-short-tools-v5.png) |
| 768 x 900 | [Tablet](native-tablet-v5.png) |
| 1100 x 800 | [Wide tablet](native-wide-tablet-v5.png) |
| 1440 x 900 | [Desktop](native-desktop-v5.png), [Sprites](native-sprites-v5.png), [States](native-states-v5.png), [Preview](native-preview-v5.png) |
| 1920 x 1080 | [Expanded desktop](native-expanded-desktop-v5.png), [tool menu](native-desktop-tools-v5.png) |

The [generated design board](preview-workspaces-v5.png) informed this pass. It is
not a native screenshot. The accepted skin artwork was unchanged.

Interaction checks cover both sides of the width breakpoints, controls at 240
pixels, tool access in short windows, exact colour entry and layer editing.
Additional checks cover the rendered tool samples, exact sprite pixels in the
comparison sheet, state selection without lost paint, and fitting after resize.
The icon check renders all 42 SVGs and rejects blank or duplicate images.

Validation for this pass: all 30 Studio interaction tests pass, along with the
42-icon check and the preview pixel tests. Strict native lint, formatting, the
state-holder gate and seven packaging checks pass. The native build, web and iOS
compile checks, and Android debug APK build pass. The full unit run found one
rounding failure among 416 tests. After the correction, all five fit tests and
the scrollbar interaction test pass; the other 415 unit tests had passed.

The preview-pass APK was installed on the reconnected Samsung SM-T970 on
25 October. Android reported a successful cold launch of `com.cranamp.app.debug`
and a visible DeX freeform window with bounds `[219,164][2521,1512]`.
The window uses `adjustResize` and has no fullscreen flag. Screen mirroring
connected to the tablet but produced no desktop window. Corner dragging,
S Pen hover and physical gestures remain unverified with this build.

## Earlier review

- [Width design board](width-workspaces-v2.png), generated after scoping the layouts.
- Native GPU captures: [small phone](native-small-phone-v3.png),
  [phone](native-phone-v3.png), [tablet](native-tablet-v3.png),
  [wide tablet](native-wide-tablet-v3.png), [desktop](native-desktop-v3.png).
- Small phone panels: [exact colour and saved colours](native-phone-colour-v3.png),
  [joined canvas states](native-phone-canvas-state-v3.png).
- Earlier scope captures: [Sprites](native-sprites-v2.png), [States](native-states-v2.png).
- Earlier [small-phone state review](native-phone-states-v2.png), with direct sprite editing.
- [Icon catalogue](icon-catalog.html).
- [Earlier design brief](brief.md) and [generation prompts](prompts.md).
- [Earlier browser prototype](prototype.html). Native code contains the current flow.
- [Implemented audio coverage](../../audio-formats.md).

The design board is illustrative. Native screenshots show GPU output with the
accepted skin assets. No skin artwork was changed.

The 29 drawing and handset interaction tests pass. Strict lint checks, the desktop
build, and web, iOS and Android compile checks pass. Interaction tests exercise
drawing, file selection, modal isolation and gestures
at all five widths. New checks cover primary control bounds, tool selection in
two taps, immediate text editing and direct return from state review to sprite editing. SVG checks render every icon and reject
blank or duplicate drawings. The full model run passed all 413 tests.

For native review at a chosen size:

```sh
cargo run --features studio-review --bin cranamp-studio-review -- 430 820
```

The review helper is under `test/` and is excluded from default builds.

## Changes after the workflow audit

Hover help appears once, after a delay. It no longer replaces the operation
message. Labeled controls do not repeat their label in a tooltip. Help has
specific tasks and recovery steps, with a topic list on phones and a side list
on wider screens.

Painting layers has one panel on every layout. The active layer's name, exact
opacity and order controls stay together. The inspector shows the current paint
destination near the tool properties. The older layer panel was removed.

Lift pixels shows the captured dimensions and a direct route to Stamp. Stamp
shows repeat and flip/rotate controls. These tools no longer offer ignored brush
properties. Pencil and Text no longer offer unsupported gradient controls; filled
shapes expose opacity in place of unused outline width.

The Android build uses a non-fullscreen Activity theme through Cranpose's Gradle
configuration. Normal Activity windows no longer register an app-owned host size.
The merged manifest and runtime window flags were checked on the SM-T970. A
physical corner-drag result remains pending; removing the fullscreen flag alone
is not proof of that gesture.

The final debug APK built successfully. It has not yet been installed on the
tablet. The 29 Studio interaction tests and the 40-icon rendering check pass;
settings, cursor and test-placement regression checks also pass. The native
review app remains open with a clean document.

Native review: [task help](native-help-v4.png),
[phone task help](native-phone-help-v4.png),
[phone layers](native-phone-layers-v4.png),
[desktop layers](native-desktop-layers-v4.png).

## Workflow audit, 25 October 2026

The following records the defects found before the changes above. Unresolved
items remain review obligations.


### Device evidence

The current checkout built with `:app:assembleDebug` and installed successfully
as `com.cranamp.app.debug` on the connected Samsung SM-T970, Android 13. Android
reported a successful cold launch and the activity in the foreground. The device
was using DeX freeform windows, a 2800 by 1752 landscape display and a density
override of 260. The application window, rather than the physical display, must
determine its layout.

The screen mirror failed to produce a desktop window. No physical S Pen hover,
touch, keyboard or visual result has been verified during this device session.
The existing native screenshots were captured on macOS at specified window sizes.

### Why the earlier review was insufficient

The earlier review checked named controls, hit bounds and selected paths. It did
not establish that each property changes the selected tool's result. It did not
follow every task from choosing a destination through editing and saving. It
treated access counts as a substitute for clear context. Screenshots mostly
showed Pencil with an empty layer stack, which concealed problems in other tools
and populated panels.

The next implementation needs one task model shared by all layouts. Each control
needs a declared purpose, applicable tools, prerequisites, state change and home.
Layout changes may move that home. They must preserve the task and selection.

### Instrument inventory

These findings come from the current UI and model code. They are not device test
results. The required context column is the design requirement for the revision.

| Instrument | Required context | Current failure or review gap |
| --- | --- | --- |
| Pencil | Width, opacity, colour, destination, paint states | Full properties offers gradient, but `paint_line` does not supply a gradient to `paint_shape`. |
| Line | Width, opacity, endpoints, colour | The generic property page does not distinguish geometric stroke settings from colour effects. |
| Rectangle | Outline or fill, applicable width, opacity, colour | Width remains visible for filled shapes; opacity moves to a separate page. Result relevance needs verification. |
| Ellipse | Outline or fill, applicable width, opacity, colour | Same relevance and property-placement problem as Rectangle. |
| Lift pixels | Selection bounds, captured image, next action | Shows Size and Opacity although the selection branch uses neither. The name does not explain that capture leaves pixels intact. |
| Stamp | Captured image and dimensions, repeat, applicable states | Can be chosen without a capture. Shows Opacity, but `paint_cluster` writes through `paint_line_untracked` without opacity blending. Transform commands live in Pixel study. |
| Text | Text, font, size, spacing, colour, opacity | Font and spacing are separated from the visible text input. Generic gradient controls are shown although the text branch excludes gradients. Physical keyboard/IME review is missing. |
| Curve | Width, bend, opacity, colour | Opacity moves away from the two visible properties; the stroke needs a clear start/end preview. |
| Taper | Width, bend, opacity, colour, direction | Shares the Curve icon. The menu and properties do not clearly teach the difference in the result. |
| Glass | Bevel, refraction, opacity, destination | Uses an eye icon also associated with preview. Bevel zero invokes automatic bevel in the model, but the field presents zero as an ordinary numeric value. |
| Pan | Navigation mode and route back to the last drawing tool | `hand` is separate from `view.brush`; the properties heading still describes the drawing brush. |
| Colour | Current colour, exact entry, sampling, saved colours | Exact entry exists, but its relationship to gradients and the colour mask is fragmented. No direct canvas colour sampler is exposed in the tool list. |
| Saved colours | Select, add, replace, remove, selected swatch | Each action needs an explicit target and feedback; empty/full palettes and deletion of the selected swatch need workflow review. |
| Paint layers | Active destination, order, opacity, visibility, lock, rename, merge, delete | Wide layouts show CompactLayers and remove the Layers toolbar action. CompactLayers has no route to full layer properties. Sprites replaces that panel with target rows. |
| Sprite destination | Selected window, sprite, source and affected variants | Source bitmaps, parts and sprite targets use separate navigation paths; users must infer how they relate. |
| Canvas states | Viewed state separate from states changed by paint | The state panel mixes viewing controls with paint scope. Their different effects are not explained at the point of use. |
| State comparison | Selected sprite and direct return to that sprite | A top-level workspace duplicates part of the state workflow and changes the available navigation. |
| Guides and clip | Visible guide, active clip, clear action | Guide controls still use the older fixed-coordinate panel. A clip can affect painting without prominent canvas context. |
| Selection transforms | Captured image, flip/rotate, preview, apply | Flip and rotate are in Pixel study, away from selection and Stamp. |
| Pixel study | Chosen pixel/region, coordinates, source ownership | The adaptive wrapper supplies a new `None` pointer state rather than the canvas hover state, so its promised pointer-following context is not connected. |
| History | Meaningful operation, affected destination, undo/redo result | Basic undo exists. Recovery across selection, palette, layers, save and scope changes needs task-based review. |
| Skin options | Playlist/font colours, visualizer, format issues | The old panel includes fixed widths up to 355 pixels inside narrower inspectors. Copy includes implementation instructions. |
| EQ workbench | Selected shared slider artwork, frame, effect on every band | An older panel remains behind Project actions. Its relationship to the joined canvas and state inspection needs review. |
| Preview | Current skin, live behavior, return to the same editing context | A separate scope exists; selection and viewport continuity need review after each route into and out of Preview. |
| Files | Current document, dirty state, save project, export, open/replace | Save, export, apply and draft are presented as separate actions without a complete document lifecycle review. Native picker cancel/error and keyboard behavior need device checks. |

### Layout failures and obligations

| Width class | Current structural issue | Required arrangement to evaluate |
| --- | --- | --- |
| Small phone | Full-screen panels remove the artwork; repeated close/open cycles are needed to judge settings. Six fixed controls cannot serve every active task. | Canvas with a task-specific bottom strip; selection gets selection actions, text gets text entry, drawing gets relevant paint controls. Secondary screens preserve context and return to the same task. |
| Phone | The eight-button bar and two equal property slots impose the same structure on unrelated tools. Bottom sheets can cover the very controls whose effects need comparison. | More direct drawing tools when space permits, with property widths based on their content. Colour and text editing must support comparison with the artwork. |
| Tablet | A long icon rail includes layers and colour beside drawing tools. Scope icons use the same symbols as instruments. Much of the bottom property row is stretched empty space. | Stable task navigation separate from instruments; compact properties beside a useful canvas, with a visible active destination. |
| Wide tablet | Colour occupies most of a narrow inspector. Layers fall below it; full layer editing loses its route. | Resizable or bounded sections with independent access to current tool, colour and active layer. Every section needs an explicit priority and overflow rule. |
| Desktop | Window/source lists, workspace tabs and canvas states compete as navigation. The inspector repeats the wide-tablet stack. | A document navigator, a persistent tool/context strip, a working canvas and task-specific inspector sections. Source editing is a document view, not an unrelated mode. |

At 360 by 640 logical pixels, the current layout reserves only 288 pixels of
drawable canvas height after its scrollbar allowance. At 600 by 360, it leaves
100 pixels. These figures follow `Layout::new` and `EditorCanvas`; they are not
physical screen measurements. Short windows need a designed arrangement, not
only fewer toolbar buttons.

The revision must evaluate every width at its lower boundary, immediately below
the next boundary, and at short and tall heights. Android DeX freeform resizing,
split screen, portrait/landscape, safe areas and an open keyboard are separate
conditions. A wide display does not guarantee a wide application window.

### Complete task paths

Each path must be completed through visible controls at all five width classes.
Each result needs both a state/pixel assertion and a visual review.

1. Open a skin, choose a paint layer and state, draw, undo, redo, save a project,
   reopen it and verify the same result.
2. Select pixels, inspect the captured result, flip or rotate it, stamp it onto
   the chosen destination, change repeat and undo the whole operation.
3. Create text, change font/size/spacing with the keyboard open, place it, change
   colour and return to drawing without losing the text or viewport.
4. Create outlined and filled shapes, then curves, taper and glass. Change every
   offered property and verify that it affects the result as described.
5. Add and reorder several paint layers, rename one, adjust opacity, lock and
   hide it, attempt painting, recover, merge and undo.
6. Change the viewed state without changing paint scope; paint into one state
   and then all states; compare the correct sprite and return to the same area.
7. Select a source bitmap, edit a sprite, inspect its joined destination, preview
   the player and return with selection and zoom intact.
8. Choose an exact colour, save/replace/remove swatches, use a gradient or mask
   where supported, and confirm that unrelated tools do not show inert controls.
9. Pan and zoom during editing, introduce a second finger during a tentative
   stroke, use pen plus touch, resize the window with a panel open and resume.
10. Cancel file pickers, fail an import/export, switch away and return, and reopen
    a draft. Verify that recovery never loses the user's working document.

Interaction coverage must include each tool with source artwork and a paint
layer; current/all paint states; a full canvas and sprite crop; no selection and
an existing selection; open and closed secondary panels. Dependent cases include
Stamp without a capture, locked/hidden layers, active clips, full palettes and
keyboard occlusion. Unsupported combinations need a clear explanation or an
unavailable control. They must not silently accept ineffective settings.

Primary controls must remain visible for the active task. Secondary controls
have a one-tap budget. Other instruments have a two-tap budget including the
final selection. Count the actual task path, including scrolling and return
steps; opening a menu alone is not completion. Test both directions across every
width boundary with an active tool, selection and open panel.

### Revision order

First define the document/task state and property applicability for every tool.
Then define one home for each action and complete the paths above on paper.
Design each width around those paths, including short heights and keyboard use.
Only then produce screen mockups and implement the shared components. Review
populated screens and actual results, with a recorded failure list. Passing a
control-presence test cannot close a workflow or visual defect.


## Drawing controls and window sizing, October 25

The compact drawing layout now has a quick palette beside zoom. Its palette button opens exact colour and saved swatch editing. The tool settings button sits beside the current properties. Narrow fields stack their labels above the value. At 488 x 538 the canvas gains 44 logical pixels of height. Pan replaces paint controls with zoom and grid controls and returns another 68 pixels to the canvas.

The inspector follows the instrument. Lift and Stamp hide colour controls; Pan shows navigation. Every drawing instrument has its own SVG cursor and a visible drawing point. Brush outlines appear only for stroke tools. Canvas scope has its own icon.

Notifications use a dark glass material at the bottom right. Drawing and undo status stay out of notifications. Errors and save results can be dismissed. Hover help appears beside the pointer after a delay.

The player keeps the proportions of its skin stack. Studio has no aspect constraint. Cranpose restores Android's previous window layout when the last host size state leaves composition. The framework change is pinned at `6e0bebb1a5f8b166789855e1f3f964f7364bd6d5`. Player size ownership is at the app root, outside deferred layout.

Native GPU captures for this pass are in [the review gallery](native-review-v6.html).

Short windows open the canvas at a scale of at least 100 percent. The Fit command still fits the full skin when requested.

Validation for this pass: 418 unit tests, 34 drawing and handset interaction tests, strict native Clippy, and the web and iOS compile checks passed. The icon checks cover 43 distinct SVGs and 11 distinct instrument cursors. Two Android instrumentation tests passed on the Galaxy Tab S7: initial player sizing and repeated restoration of the system window size. The running player window reported 736 x 1319 physical pixels.

Tablet mirroring did not deliver video. Physical S Pen hover, two-finger gestures and manual DeX corner resizing remain unverified. The desktop captures and Android window tests do not replace those checks.
