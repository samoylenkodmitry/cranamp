# Purr Chaos EQ boundary review

The 2026-09-22 14:39 markup and layered baseline are preserved in `target/catamp-eq-immersion/annotated-before.cstudio` and `user-markup.png`. The five annotation strokes were removed only in the working copy. The user chose visible paw handles, then explicitly requested matching background color.

## Corrections

All art was drawn through Skin Studio. No external raster generator or image-editing script painted the assets.

- All 28 shared EQ track frames, both handle backgrounds and adjacent paper use `#bed1aa`. Guides use `#9eb495`. Preserved paw silhouettes stay visible. This removes the panel-shaped contrast at every marked slider boundary.
- The resting paw has a complete angled contour in the space beside the first shared track. Old clipped paw remnants were removed and the headphone cable ends in that gap.
- The kitten leg and book were reviewed with matching paper around the preamp track; the contour remains intact.
- The footer plaque now ends in a rounded dark-green bevel, with the abrupt cream strip removed.

The original four marked rectangles are in `test/skin-studio/purr-chaos-eq-boundaries.json`; the review tool captured before/after native editor images, a four-pixel exterior halo, coverage and all four focus/press GPU states. Full cat subjects were also inspected. Final captures supersede the exploratory artwork-only study, which was rejected for this skin when the user chose visible paws.

## Evidence

Local evidence is under `target/catamp-eq-immersion/`:

- `index.html` and `review.json`: annotated boundary gallery.
- `visible-strokes.json`: native pen requests and continuity reports.
- `travel-board.png`: actual GPU at levels 0,7,14,21,27, released/pressed, plus ramp and alternating levels. Runtime EQ graph lines intentionally cross the illustration.
- `paper-checks.json`: all 24,696 shared-track source pixels match the intended paper/guide pattern; source RGBA probes confirm matching released/pressed handle and background paper.
- `validation.json` and `export.json`: classic profile passes, no alpha, missing moving pixels or divergences.
- `final-layered-crop.png`, `final-roundtrip-crop.png`, `roundtrip-report.json`: matching-state 550×754 GPU images have zero changed pixels after reopening the WSZ without a companion project.
- `webamp-mixed.png`: actual Webamp 2.3.1 loaded the exported bundled skin; two paw handles were dragged to opposite endpoints and the paper remained continuous. Native Winamp and Audacious were not newly exercised in this review.

All exported flat-area warnings were reviewed: `[63,223,29,29]`, `[199,26,18,18]`, `[170,0,14,14]`, `[91,59,10,10]`, `[82,82,13,13]`, `[205,346,9,9]`, `[201,361,10,10]`, `[238,103,10,10]`, `[26,99,8,8]`, `[198,92,12,12]`, `[218,73,8,8]`. They are deliberate continuous paper, signs/readouts or the shaded fish body. None was considered inaccessible. The paw drawing also flagged paper `[85,148,13,13]`, reviewed in the full composition and mixed states.

The code adds a shared GUI/MCP [EQ workbench](equalizer-workbench.md). Its five new tests cover mode preservation/undo, project round trips, exact cropped exports, shared-frame selection and copying all frames. The Studio unit suite passed 185 tests. The classic skin archive remains full height with visible controls. Deliverables are `assets/skins/Catamp Purr Chaos Font Fixed.{wsz,cstudio,png}`.
