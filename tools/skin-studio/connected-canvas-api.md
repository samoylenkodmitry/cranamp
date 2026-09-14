# Connected canvas API

```python
import sys
sys.path.insert(0, 'tools/skin-studio')
from connected_canvas import CanvasPen, apply
p = CanvasPen()
p.curve([3,108], [8.5,118.5], [5,132], 2, '#d5f7ff')
plan = p.plan([0,106,12,28])
print(plan.skipped, plan.aliases)       # inspect omissions/shared sources
patch = p.prepare('connected glass edge', [0,106,12,28]) # read + validate
apply(patch)                          # one painting plane + one undo
```

| Contract | Value |
|---|---|
| Coordinates | Native assembled pixels; main y0, EQ y116, PL y232 |
| Canvas | 275×377; `playlist_height=145` (integer145..522) |
| Ownership | `rect=[x,y,width,height]`; drawing clipped to mapped source rectangles |
| Static surfaces | Main/EQ backgrounds; main/EQ/PL title variants; PL corners/footer |
| Dynamic controls | Never implicitly painted; `control_parts`/`control_prepare` uses named, explicit states |
| Titles | `titles='both'` default; `'active'`, `'inactive'`, `'none'` |
| Runtime | `preserve_runtime=True`: excludes timer cells, spectrum, song/bitrate/rate, EQ curve, PL text/times |
| Mask | `exclude=[[x,y,w,h], ...]`; subtracts exact native rectangles |
| Shared PL tiles/rails | `repeats='error'` default: plan lists omissions; parts/prepare reject partial patches. `repeats='skip'` explicitly permits omission |
| Shared-source edit | `repeats='shared'`: select ONE copy; repeats outside ownership rect too; listed in `plan.aliases` |
| Ambiguous repetition | Multiple copies with overlapping atlas ownership raise `ValueError`; narrow `rect`/mask |
| Dock row | Canvas y115 aliases main y114. Drawing y114 controls both; a y115-only crop maps to114 |
| Classic PL list area | No static bitmap; reported in `plan.skipped`; PLEDIT.TXT controls fill/text |
| Rasterization | Studio native geometry only. No scaling, bitmap generation, antialias filtering, or blur |

Runtime protection masks the *static paint beneath* live readouts. When rebuilding
an entire background illustration, use `preserve_runtime=False` to replace old
background fragments there too. This does not edit the separate glyph/control
atlases or disable live text/visualizers. Inspect the actual player afterward;
retaining an old rectangular background mask can visibly cut a new curved shape.

## Inspect the repair and its surrounding pixels

```python
from connected_canvas import context_rect, study_canvas
rect = [199,132,4,20]
print(context_rect(rect))                 # [195,128,12,28], pure bounds
result = study_canvas(rect, 'target/join-before.png') # native + integer detail
# Repeat after applying; view image files by vision. Do not JSON-decode result.
```

| Helper | Contract |
|---|---|
| `context_rect(rect,padding=4,playlist_height=145)` | Expand on all sides; clamp to native canvas; original ownership must fit; returns fresh `[x,y,w,h]` |
| `study_canvas(rect,path,zoom=4,padding=4,playlist_height=145)` | Validate locally; read status; call `studio_study`; return its raw image/caption result |
| Halo | Default 4 native pixels; nonnegative integer `padding`; use 2–4 for boundary defects |
| Current view | Must already be `panel='canvas'` with matching `preview_playlist_height`; mismatch rejects without changing view |
| Output | Absolute resolved `path`; writes only requested PNG; no artwork, layer, selection or view mutations |
| Detail | Integer `zoom` 1..8; full composite, original colors, no grid/geometry overlay; editor study, not actual GPU screenshot |

Inspect exterior halo for leftover bevel rows, clipped reflections and unmatched material at ownership edges. Study both sides of a join; compare actual player screenshot separately.

For the actual GPU presentation after any window resize:

```python
from connected_canvas import capture_canvas, presentation_bounds
capture_canvas('target/review.png')  # raw image/caption, no value(); requires presentation=true
print(presentation_bounds([1157,871],zoom=2,playlist_height=145)) # [303,58,550,754]
```

`capture_canvas` reads status and full screenshot PNG header dimensions, then requests a second `studio_screenshot` with the measured crop. Studio performs the crop; no pixel decoding, image editing, resampling or GUI changes happen in the helper. Returns raw MCP image content and writes the requested PNG. Keep the window size fixed during the pair. `presentation_bounds` is pure: it mirrors the presentation zoom cap and floors the centered origin, leaving an odd spare pixel on the right/bottom. A scene smaller than the presentation is rejected. Requires the current pixel-snapped presentation layout; older app builds centered on half-pixels at odd scene sizes. Cropping cannot repair those rendering artifacts.

```python
# Pure geometry methods; append operations, never touch live state.
p.rect(x,y,w,h,color, ramp=['#204455','#77bbdd','#e5fbff'], axis=[x,y,x,y+h])
p.round(x,y,w,h,radius,color, ramp=colors, axis=[x0,y0,x1,y1])
p.round_outline(x,y,w,h,radius,color,width=1) # large rim: native stroke, no interior fill
p.ellipse(x,y,w,h,color, fill=True, width=1)
p.path([[x,y], [x,y], [cx,cy,x,y], [c1x,c1y,c2x,c2y,x,y]], color,
       fill=True, width=1, ramp=colors, axis=[x0,y0,x1,y1])
p.line([[x,y],[x,y]], color, width=1)
p.curve([x,y], [cx,cy], [x,y], width, color, taper=False, clean_corners=True)
p.taper([x,y], [cx,cy], [x,y], width, color)
p.stamp(x,y, ['.xx.', 'xxxx'], {'.':'#204455','x':'#d5f7ff'}) # space skips
p.pixel(x,y,color)
```

`round_outline`: same cubic coordinates as `round`; centered integer stroke width 1..32. Complete path rasterizes before ownership/mask clipping. Use outlines for large rims to avoid repeated full-canvas polygon fills. Does not clear the interior.

## Explicit palette bands

```python
from connected_canvas import CanvasPen
p = CanvasPen()
p.bands(8,20,32,12, ['#476976','#87aeb8','#d9eeef'], [0,7,11,12])
# Horizontal rows: 7px shadow, 4px body, 1px highlight; no interpolated shades.
p.bands(48,20,8,12, ['#476976','#d9eeef'], [0,6,8], axis='x')
```

`bands(x,y,w,h,colors,edges,*,axis='y')` appends one solid native rectangle per color and returns the pen. Integer offsets run top to bottom for `y`, left to right for `x`; color `i` owns `[edges[i],edges[i+1])`. Supply a nonempty list/tuple of hex colors and exactly one more edge than colors. Edges must strictly increase from zero through the axis extent. Origins and positive dimensions are integers; local origins may be negative for clipped motifs. All inputs validate before appending; no live calls. Groups, exact recoloring, control mapping and ownership masks work normally.

Bands support deliberate color clusters with authored widths. Shape contours and highlight placement still need drawing decisions and native-size review. Use solid stamps/paths for irregular clusters. `ramp` interpolates shades and is appropriate for smooth lighting; a short ramp is not an exact limited palette. Opaque hex colors on a full-opacity plane retain the chosen palette; alpha or layer opacity blends it with the underlying art.

## Local groups and reusable native motifs

```python
# Every brush inside the group uses local coordinates, including ramp axes.
with p.group(12,110) as glint:
    glint.curve([0,0], [9.5,2.5], [16,12], 1, '#eefaf8', clean_corners=True)
    glint.stamp(3,2, [' x ', 'xxx', ' x '], {'x':'#eefaf8'})

# Copy the same authored native geometry; recolor exact values for a variant.
p.place(glint, 34,110, colors={'#eefaf8':'#96c6d4'})
parts = p.parts([12,110,40,16], exclude=[[22,114,2,2]])
```

| Method | Contract |
|---|---|
| `p.group(x=0,y=0,*,colors=None)` | Context manager yields independent `CanvasPen`; successful exit appends once; failed block appends nothing |
| `p.place(motif,x=0,y=0,*,colors=None)` | Copy a `Pen` or iterable of native operations; returns `p`; validates entire copy before append |
| Local coordinates | All brushes use one integer translation; fractional curve controls/path commands/ramp axes preserved |
| Nested groups | Integer offsets accumulate; original geometry and order preserved |
| `colors` | Exact, case-sensitive `{'#rrggbb':'#rrggbbaa', ...}` replacements in solid colors, ramp stops and stamp palettes |
| Independence | Copies and source share no mutable geometry, palette or stamp data; source remains reusable |
| Scope | Groups add geometry only; canvas coordinates still define `parts` ownership/masks; no implicit layer, clipping, scale, mirror or live call |

- Curves/path controls and palette axes may be fractional; pixel/rect origins stay integer.
- Palette ramps remain anchored to the entire canvas through every atlas translation.
- Masked parts are clipped inside Studio; strokes remain whole until native rasterization.
- Supported bridge ops: pixel, line, rect, ellipse, path, curve, tuft, stamp. Unknown operations/fields fail.
- Material/cluster/mirror/blur operations are not supported by this helper.
- `plan=p.plan(rect, **options)` returns `targets`, `skipped`, `aliases`, `readonly`.
- `parts=p.parts(rect, **options)` is pure and suitable for parallel artist `build()`.
- `patch=p.prepare(name,rect,baseline=None,**options)` reads source tokens if needed and validates; does not apply.
- `baseline` is the `parts` list from a matching `studio_patch inspect`, not a whole response.
- `apply(patch)` submits existing prepared geometry; stale source tokens fail. Do not recapture stale tokens blindly.
- In-place revision: `regional_patch.prepare_revision(name,p.parts(rect,...),review_baseline)`.
- Independent agents produce pure recipes; one coordinator applies sequentially. No live drawing during frame sweeps.
- Verify actual Cranamp render; checking geometry cannot establish visual quality.

## Named control cells: local coordinates, explicit states

```python
from connected_canvas import CanvasPen, control_cell, apply
p = CanvasPen()                         # a fresh local drawing, origin [0,0]
p.curve([4,4], [10.5,1.5], [20,5], 1, '#dcefed', clean_corners=True)
print(control_cell('seek', state='held')) # pure source metadata; rect[2:] is native size
parts = p.control_parts('seek', state='held') # pure build(); no clear or live call
patch = p.control_prepare('Fish dorsal reflection', 'seek', state='held')
apply(patch)                            # explicit mutation, one plane/Undo
```

| Names | Cell size | Allowed `state` |
|---|---|---|
| `prev`, `play`, `pause`, `stop` | 23×18 | `released`, `held` |
| `next` / `eject` | 22×18 / 22×16 | `released`, `held` |
| `seek` | 29×10 | `released`, `held` |
| `volume`, `balance` | 14×11 | `released`, `held` |
| `eq` | 11×11 | `released`, `held` |
| `scroll` | 8×18 | `released`, `held` |
| `shuffle` / `repeat` | 47×15 / 28×15 | `off`, `off-held`, `on`, `on-held` |
| `eq_on` / `eq_auto` | 26×12 / 32×12 | `off`, `off-held`, `on`, `on-held` |
| `eq_presets` | 44×12 | `released`, `held` |
| `eq_graph` / `eq_preamp` | 113×19 / 113×1 | `normal` |
| `volume_track` / `balance_track` / `eq_track` | 68×13 / 38×13 / 14×63 | `frame`, plus `frame=0..27` |

- `state` is required; no automatic propagation to another state.
- Geometry, optional `rect=[x,y,w,h]`, `exclude=[...]` use **cell-local pixels**. No atlas arithmetic.
- `rect` defaults to the full cell; crops/masks outside its bounds reject. Complete masking rejects.
- Complete strokes/ramps translate once, rasterize natively, then clip to the exact ownership rect. No implicit clearing; pixels outside ownership cannot change.
- `control_prepare(...,baseline=parts)` reads/validates like `prepare`; stale tokens reject. It never applies.
- Moving thumb art repeats at every slider position. Track names edit exactly one specified frame; no inference from current GUI value.
- `frame` accepts integer 0..27 only (booleans rejected); it is required for track names and rejected for other controls.

```python
p = CanvasPen()
p.rect(0,0,14,63,'#ff00ff')             # explicitly clear one native EQ groove cell
p.rect(6,4,2,54,'#32627b')
parts = p.control_parts('eq_track',state='frame',frame=27)
# Same local drawing can be mapped into explicit frame variants:
parts = [q for f in range(28) for q in p.control_parts('eq_track',state='frame',frame=f)]
# eq_graph has one source state; source dimensions and stride stay inside the helper.
print(control_cell('eq_graph',state='normal'))
```
- Several explicit variants in one plane: concatenate `p.control_parts(...)` lists; use `regional_patch.prepare(name,parts)`, `validate(patch)`, `apply(patch)`.
- For explicit replacement only, draw the full local background/key color yourself before details.

```sh
python3 tools/skin-studio/test_connected_canvas.py
```
