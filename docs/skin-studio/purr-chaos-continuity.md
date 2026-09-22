# Purr Chaos: annotated continuity repair

The 2026-09-22 13:52 annotation is preserved in `target/catamp-continuous-controls/user-markup.png` and `annotated-before.cstudio`. The eight human markup strokes were removed from the working copy only after saving that baseline. The corrected deliverables are `assets/skins/Catamp Purr Chaos Font Fixed.{wsz,cstudio,png}`. All artwork edits used Skin Studio drawing operations; no raster artwork was generated outside Studio.

## Reviewed regions

| Native rectangle | Correction |
| --- | --- |
| `[234,0,30,16]` | Continue title paper through minimize/shade cells and their rim while retaining ivy and icons. |
| `[103,55,137,19]` | Match both paw-track mattes to the surrounding paper in all frames and both moving variants. |
| `[14,70,251,20]` | Complete the golden fish across the seek track. Replace the moving handle with a drawn sardine tin that carries its own complete opaque illustration. |
| `[253,336,20,26]` | Continue the rail highlight into a curved vine and leaf above the footer kitten. |

The seek handle initially retained its old paper-colored rectangle, which visibly cut the fixed fish when moved. That version was rejected. The final tin intentionally occludes the fixed artwork as a foreground object, without copying a stationary backdrop into the moving cell. The paw backdrop recolor reported foreground occlusion of the preserved paw silhouettes; native samples and GPU travel captures were inspected. Other crossing strokes passed strict continuity checks. Requests and reports are in `target/catamp-continuous-controls/strokes.json`.

## Verification

- Whole-canvas writer probe: 414,700 checks, zero mismatches.
- Native/editor crops, four-pixel halos, and all four focus/press GPU states for every manifest region.
- Released/pressed seek and paw controls at endpoints and intermediate positions, including overlap of the tin and golden fish.
- All 29 playlist resize phases, with scrollbar positions cycled through start, middle and end.
- Classic export audit: exportable, no divergences, no alpha or missing moving-cell pixels.
- Reloaded the exported WSZ from a separate path without a companion project: the 550×754 GPU image matched the layered project exactly, with zero changed pixels. This is a Cranamp round-trip check, not a new external-player certification.

The before/after gallery is `target/catamp-continuous-controls/index.html`; state, travel, resize, validation and export evidence sit beside it. `test/skin-studio/purr-chaos-boundaries.json` preserves the region manifest for future reviews.

Flat-area warnings were visually reviewed, including every exported largest flat square: `[29,225,27,27]`, `[199,26,18,18]`, `[170,0,14,14]`, `[91,59,10,10]`, `[82,82,13,13]`, `[205,346,9,9]`, `[201,361,10,10]`, `[74,217,8,8]`, `[238,103,10,10]`, `[26,99,8,8]`, `[40,156,13,13]`, `[198,92,12,12]`, `[218,73,8,8]`, `[218,217,8,8]`, `[63,148,11,11]`, and `[65,195,13,13]`. These are continuous paper between illustrated subjects, intentional sign/readout fields, or the shaded fish body. Regional checks also identified title paper `[230,0,13,13]` and paper behind the EQ control `[215,55,9,9]`. None was treated as inaccessible bitmap space. The footer junction has no flat-area warning.

Do not rerun the old classic-migration art recipe over this skin: it predates these hand-drawn continuations. Preserve the current project and repeat the region/travel review when changing shared source cells.
