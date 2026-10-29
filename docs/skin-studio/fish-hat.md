# Catamp Fish Hat

A Silverplay variation with orange tabbies, fish hats and a snack queue. The shared Studio pen performs every palette change and drawing operation.

- [WSZ](../../assets/skins/Catamp%20Fish%20Hat.wsz)
- [Editable project](../../assets/skins/Catamp%20Fish%20Hat.cstudio)
- [GPU preview](../../assets/skins/Catamp%20Fish%20Hat.png)
- [Drawing recipe](../../test/skin-studio/fish_hat.mjs)
- [GPU review captures](../../test/skin-studio/fish_hat_review.mjs)

The Silverplay baseline is preserved. The recipe refuses unsaved work. Run it with Skin Studio's MCP server enabled, then inspect the captures before exporting.

Reviewed Cranamp's four active/pressed states, slider positions 0 and 27, mixed EQ values, and playlist heights 145, 174 and 203. The fish and paw stay clear of shared EQ handles. Native continuity reports zero mismatches in all four states. The inherited flat regions form the fish body, open mouth, framed control grounds and caption spacing; they are intentional space within the illustration. Source validation reports no divergent cells or incomplete moving sprites.

The exported WSZ loaded in Webamp 2.3.1. Its artwork and captions were visually checked at the default slider positions. Fonts and runtime content differ between players; other players and shade modes were not tested. Cranamp was restarted after its window collapsed during the live skin change. After restart, the selected skin persisted and the mounted MP3 played with its timer and spectrum active.
