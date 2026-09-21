# Midnight Snack continuity correction

This earlier review verified stroke routing but missed the composition breaks later marked by the user. See [the annotated correction](annotated-abrupt-review.md) for the current artwork, findings and validation.

The interrupted strokes were an authoring mistake. The joined-canvas brushes already route across components; I instead generated separate textile rectangles with local Y origins and changed the repeat from 18 to 25 pixels. I also used a handwritten target list that omitted `equalizer.preamp.line`, and joined two footer crops at a straight splice. Source-write counts did not prove that the final visible ink matched.

The six-layer baseline is preserved in `target/continuity-before.cstudio`. Layer **07 · One cloth, one canvas** contains the correction. All artwork writes went through Skin Studio MCP. The [replay](../../tools/skin-studio/midnight_snack_joins.mjs) now uses one global textile phase, keys the shared EQ backdrops and the obsolete preamp stripe, blends the footer transition, and draws each outer cord once through the joined canvas. All three opaque drawing operations require the new continuity check to pass.

The [region manifest](../../tools/skin-studio/continuity_review.json) records these native rectangles. Before/after editor crops, four-pixel exterior halos, ownership and all four GPU states are in `target/continuity-review/index.html`. The capture tool leaves approval pending by design; the visual findings below are the completed review.

| Region | Native rectangle | Visual finding in all four states |
| --- | --- | --- |
| Cat/preamp | 80,130,126,24 | The dark one-pixel source stripe no longer cuts the orange paw. The white live EQ curve remains a runtime overlay when EQ is active. |
| EQ/playlist cloth | 16,200,246,52 | The weave keeps its phase and width across the lower EQ and playlist title. The kittens, fish, star and lettering remain intact. |
| Left rail | 0,238,20,120 | One outer cord continues through the header, repeated rail and footer, including the exterior halo. |
| Footer splice | 105,339,42,38 | The ruler-straight change at x127 is blended into the cloth; the lavender cat and gold glyphs stay intact. |
| Right rail | 251,238,24,139 | The outer cord continues through title, repeated rail and footer. The gold/pink fish remains a keyed silhouette. |

The recorded draw reports in `target/continuity-review/strokes.json` checked **86,048 composed ink/state pixels with zero mismatches**. Two deliberate magenta-only operations are correctly unverified by this opaque-ink check. Repeated-rail overpainting generated 58 historical source overwrites, but its final pixels agree; the guard checks the final result rather than rejecting harmless intermediate writes.

Validation completed:

- 318 Rust unit tests and 35 integration tests passed, including hidden preamp coverage, atomic rollback, consistent repeated-rail overpainting, conflicting aliases, clipped crossings, GUI warnings and magenta-only unverified results. Test bodies remain under `/test/`.
- The final real-writer probe checked **414,700 native pixels/states with zero mismatches**. Each standard view has 82,534 writable bitmap pixels and 21,141 palette-only playlist pixels; 1,216 writable pixels sit beneath the opaque runtime spectrum.
- Layered project and exported WSZ produce identical GPU PNGs. All 28 EQ track frames retain keyed backdrops. Every pixel of both outer cords is continuous at all 29 playlist tile offsets.
- Inspected all four before/after GPU states for each declared crop, plus slider endpoints/intermediate positions and playlist heights 146, 158, 174, 261 and 522. Export reports no flat drawable-area warnings and no opaque moving sprite cells.
- `cargo fmt --check` and working/index diff whitespace checks passed.

[Updated GPU preview](../../assets/skins/Catamp%20Midnight%20Snack.png) · [Layered project](../../assets/skins/Catamp%20Midnight%20Snack.cstudio) · [Studio continuity guard](continuity.md)
