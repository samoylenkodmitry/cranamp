# Catamp Midnight Snack

An orange tabby has eaten the controls. Kittens stalk fish faders and a lavender cat sleeps beneath the playlist. The main/equalizer join runs through the tabby; the attic, woven quilt and native controls occupy one illustrated canvas.

- [Classic WSZ](../../assets/skins/Catamp%20Midnight%20Snack.wsz)
- [Eight-layer Studio project](../../assets/skins/Catamp%20Midnight%20Snack.cstudio)
- [Actual GPU preview](../../assets/skins/Catamp%20Midnight%20Snack.png)
- [Replay recipe](../../tools/skin-studio/midnight_snack.mjs)
- [Continuous-art correction](../../tools/skin-studio/midnight_snack_continuous.mjs)
- [Native correction rectangles](../../tools/skin-studio/blank_audit_review.json)
- [Cause of the blanks and correction review](midnight-snack-review.md)
- [Stroke interruptions and completed continuity review](continuity-review.md)
- [User-marked composition breaks and correction](annotated-abrupt-review.md)
- [Pixel audit and flat-area diagnostics](pixel-audit.md)
- [Every native pixel, four-state probe JSON](midnight-snack-pixel-probe.json)
- [Continuous reference PNG](../../assets/skins/sources/Midnight%20Snack%20continuous.png)
- [Final image-edit prompt](../../assets/skins/sources/Midnight%20Snack%20continuous%20prompt.txt)

Both reference images were generated with the built-in image-generation tool. The repaired reference fills the earlier prompt's empty regions. The final skin is painted through Skin Studio MCP using native crops, stamps, paths and paint layers, with every control state and 18 paw cursors. The EQ continues the illustrated bedding; shared slider backdrops are transparent. The classic playlist interior is palette-only, with artwork painted into its adjacent shadow. The spectrum uses a keyed background so live bars float over the attic.

Open Studio, then run `node tools/skin-studio/midnight_snack.mjs` to rebuild from Moon Garden, including the blank-area, transparency, continuity and annotated composition corrections. The recipe refuses unsaved work. To apply only the latest correction to the saved seven-layer project, run `node tools/skin-studio/midnight_snack_scene.mjs`.

`node test/skin-studio/midnight_snack.mjs` checks exact GPU equality between the layered project and exported WSZ and captures slider extremes, all four focus/press states, and tall playlist variants under `target/midnight-snack-gpu/`.

`node tools/skin-studio/region_review.mjs tools/skin-studio/blank_audit_review.json after target/blank-audit-review` captures editor/native crops, four-pixel halos, source ownership and four GPU states. Inspect these visually; mapping counts do not approve artwork.

Studio's [explicit crop and native sizing](image-stamps.md) and [pixel audit](pixel-audit.md) support this workflow. Latest main already contained the SkinStudio extraction, external test modules and test-location guard; AGENTS.md explicitly forbids test bodies in implementation files, tools, scripts and generated code.
