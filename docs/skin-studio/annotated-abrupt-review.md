# Correcting the user's marked visual breaks

Historical review of the retired Cranamp-only transparency convention. Its magenta spectrum/sprite claims do not describe current rendering or exports. Use the [classic compatibility contract](classic-compatibility.md).

The earlier continuity review missed the composition. It verified that new strokes survived source mapping, while a rectangular procedural quilt still replaced the illustrated bed, separate kitten crops remained visible, and the GPU spectrum covered the attic with a solid rectangle. Passing pixel-write tests could not justify those edges.

The supplied annotation is preserved locally as `target/annotated-abrupt-review/user-markup.jpg`; its seven-layer project is `target/annotated-abrupt-before.cstudio`. [The native manifest](../../tools/skin-studio/annotated_abrupt_review.json) maps every marked boundary. Before/after editor boards, four-pixel halos and all four GPU states are in `target/annotated-abrupt-review/index.html`. All those crops were visually inspected.

| Mark | Native review rectangle | Correction |
| --- | --- | --- |
| Spectrum box | 22,41,83,21 | VISCOLOR slot 0 uses the exact magenta key. The player now reveals the existing attic beneath stopped/unlit spectrum pixels; lit bars retain their colors. |
| Top of pasted quilt | 16,143,243,33 | Continue the original bedding illustration immediately after its previous source boundary. Remove the rectangular fabric replacement. |
| Left quilt/kitten cutouts | 0,151,85,79 | Restore the whole white kitten and connected pillows from the same source composition; remove the two independent kitten stickers. |
| Right quilt edge | 238,148,37,82 | Continue the illustrated cloth and hanging ornaments across the entire EQ background, instead of ending them against a narrow replacement strip. |
| Playlist/footer cut | 0,326,275,39 | Match every footer-edge pixel to the playlist/rail above it, then bring the sleeping cat and artwork out of a short uneven shadow. |

Layer **08 · The quilt was never a rectangle** was painted through Studio MCP. The old duplicated fish title strip is removed. The illustration settles into a shared dark field that reaches the exact playlist fill color. The fish handles remain separate keyed sprites, never baked into the background. [The replay](../../tools/skin-studio/midnight_snack_scene.mjs) is part of the full build recipe.

Studio now exposes the keyed spectrum setting in its palette UI/MCP output. Coverage and the exhaustive real-writer probe honor that palette instead of always classifying the spectrum as opaque. `AGENTS.md` explicitly requires review of complete illustrated subjects, marked boundaries and runtime fill transitions, beyond stroke mechanics.

Validation: **320 unit tests and 35 integration tests passed**. The live GPU test verifies project/export equality and exact native equality between the stopped spectrum and the editor artwork. All 28 EQ frames keep transparent track backdrops; both border strokes and the full footer shadow boundary pass all 29 playlist tile offsets. Four focus/press states, slider positions and five playlist sizes were captured. The full native probe completed **414,700 checks with zero mismatches** and reports zero opaque spectrum pixels for this skin. Formatting and diff whitespace checks passed.

The bedding continuity report deliberately records 1,364 covered ink/state pixels: the eleven 31-pixel fish silhouettes across four states. Those are intentional foreground sprites. Title/footer shadow draws pass exact visible-ink checks. The live EQ curve is also intentional runtime content. Neither assertion is a substitute for the crop review above.

[Current GPU preview](../../assets/skins/Catamp%20Midnight%20Snack.png) · [Studio project](../../assets/skins/Catamp%20Midnight%20Snack.cstudio) · [WSZ](../../assets/skins/Catamp%20Midnight%20Snack.wsz)

Exact-magenta spectrum transparency requires the updated Cranamp renderer; other players may interpret the palette differently.
