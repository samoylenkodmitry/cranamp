# Catamp Moon Garden

Moon Garden adapts a generated concept into a 275×377 native Winamp skin. Its continuous scene is painted through Cranamp Skin Studio MCP with individual one-pixel pencil operations. The colors come from the generated reference; this is a reference-guided pixel adaptation, not an illustration whose every pixel was invented by hand.

- [Generated reference](../../assets/skins/concepts/Moon%20Garden.png)
- [Actual player preview](../../assets/skins/Catamp%20Moon%20Garden.png)
- [Portable WSZ](../../assets/skins/Catamp%20Moon%20Garden.wsz)
- [Seven-layer Studio project](../../assets/skins/Catamp%20Moon%20Garden.cstudio)
- [Paw cursor set](../../assets/skins/Catamp%20Moon%20Garden%20cursors.png)
- [Native pixel recipe](../../tools/skin-studio/moon_garden_pixels.mjs)
- [Authored pixel refinements](../../tools/skin-studio/moon_garden_refine.mjs)
- [Boundary and silhouette repairs](../../tools/skin-studio/moon_garden_continuity.mjs)
- [Marked-region review manifest](../../tools/skin-studio/moon_garden_review.json)
- [Reference palette and pixel grid](../../assets/skins/sources/Moon%20Garden%20reference%20pixels.json)

The source image was area-sampled as a whole at 275×377 and reduced to an 80-color palette with weighted median-cut clustering. The saved grid makes the drawing reproducible. The recipe projects that continuous grid into the joined canvas and issues 82,534 one-pixel marks, plus 500 marks for the shared playlist title tile, through `studio_draw`. It does not assemble image cutouts. Native controls, text, handles and their states are drawn separately; one-pixel corrections restore the seek stem and readable glyphs.

The upper cat crosses the main/EQ join with its original proportions; its loose tail curves into the margin to clear the shared preamp track. The lantern moves upward without horizontal squeezing, and the sleeping cat uses one uniform scale. The footer books are drawn at native size with leather spines and paper shading. A fifth layer adds authored one-pixel clusters: bellflowers, fuller fader leaves, moths, fireflies and constellation details. An irregular arch edge accommodates the fixed spectrum footprint, and the garden wall continues through repeated playlist tiles. Smaller transport signs and distinct ON/AUTO states remain readable on the illustration.

Flower buds and stems form the live EQ faders. Repeating garden margins support playlist resizing; the text interior uses the single flat background color required by the classic skin format. Seven editable layers preserve the control frames, continuous reference scene, functional details, control repairs, authored pixel refinements, reclaimed readout/footer margins and repaired silhouettes.

The seventh pass addresses the annotated hard edges: it restores the crescent and village under the song and MONO/STEREO readouts, paints control backgrounds to match the scene, rejoins the seek strip, tapers the tail before the shared preamp cell, finishes the right lantern and fern bank, and replaces the squeezed left rail with native-width ivy continuing through its cap and footer. The exact 76×16 spectrum remains opaque in the GPU player; the classic playlist interior is palette-only. All 18 cursor roles retain the native `paw` style, with directional hints and visible toe pads.

Catnip is preserved independently as `assets/skins/Catamp Catnip.wsz` and `.cstudio`.

## Rebuild

Open Studio with `cargo run -- --skin-studio 'assets/skins/Catamp Moon Garden.cstudio'`. In another terminal, run `node tools/skin-studio/moon_garden_pixels.mjs`. The recipe refuses unsaved work, opens the bundled Silverplay base, paints seven native layers, draws paw cursors, checks portability and readability, and saves the WSZ, layered project and actual GPU player screenshot. Replay uses the checked-in palette grid; the generation tool is not needed.

For annotated changes, capture the saved baseline and repaired version with `node tools/skin-studio/region_review.mjs tools/skin-studio/moon_garden_review.json before target/moon-garden-region-review` and the same command with `after`. The generated `index.html` pairs native and magnified editor crops, four GPU states and source-ownership reports for every listed correction, including a four-pixel exterior halo. Capture does not certify visual quality; the report deliberately starts with review pending.

## Verification

The seven-layer replay produced identical RGBA player pixels to the reviewed drawing. All eight marked regions were reviewed with exterior halos in editor and GPU views, including every distinct alternate-state crop. The new rail/footer connection was checked at playlist heights 145, 146, 158, 173 and 522.

Reviewed actual GPU previews for all four active/inactive and released/pressed combinations, slider extremes, and a 522-pixel-tall playlist. Reloading the exported WSZ produced identical RGBA player pixels at 275×377, identical pixels for all 18 cursor images, and identical hotspots. Studio reported no portability divergences or hard-to-read text warnings. The Rust suite passed, including coverage and cursor regressions; Clippy and the formatting check passed.

## Original generation prompt

Mode: built-in imagegen, new generation, no reference image.

```text
Use case: ui-mockup / stylized-concept.
Create one beautiful, finished pixel-art concept for a CAT-THEMED CLASSIC WINAMP SKIN. The design is a single immersive joined canvas, like an enchanted moonlit garden inhabited by cats, with functional music UI woven into the illustration. This is a reference that a pixel artist will redraw as an actual 275x377 pixel skin.
Composition: portrait aspect 275:377, straight-on, the skin fills the image. The top 116 units are the player, the middle 116 are the equalizer, the bottom 145 are playlist. There are NO horizontal panel borders; illustrations flow across those two joins. No outer mockup, no surrounding desk, no perspective.
Art direction: exquisite deliberate pixel clusters, stepped contours, a restrained midnight-indigo, blue-violet, muted teal, warm ivory and pale gold palette. Luminous flowers and leafy stems, a crescent and sparse tiny stars. Two expressive ivory cats with dusky blue shadows: one large cat sprawls from the left side of the player into the EQ area, another naps along the lower playlist edge. Give the scene graceful curves, asymmetry, clear silhouettes and atmospheric beauty. Flowers and cats feel organically part of the interface, not pasted badges.
Practical layout to respect: timer at upper left around x48..98 y27..39; song text at upper right x112..259 y27..35; volume near x107..174 y57..70; transport around x16..152 y88..106. EQ: tiny switches around y134, a small graph x86..199 y134..153, eleven vertical faders between y155..217. The faders can be delicate stems with flower buds as handles, transport icons can be tiny stars or petals, no raised button bezels. Keep the lower playlist text area x14..255 y253..334 clean dark indigo and readable with just a few quiet cream text lines. Cat/flower artwork stays along its borders and footer.
Text: small title "MOON GARDEN", timer "00:00"; very little other text. Music controls still recognizable but visually unboxed.
Avoid: panels, metallic hardware, shiny glass, bevels, generic rectangular buttons, photorealism, soft airbrush, busy noise, oversized typography, watermarks.
Render as crisp, convincing crafted pixel art, enlarged with hard edges.
```
