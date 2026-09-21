# Sprite holes, erasure and skipped ink

Opaque `#ff00ff` marks a sprite hole in Cranamp. The editor composes paint layers first, then skips keyed pixels while placing each sprite over the joined canvas. The player converts the same exact key to GPU alpha. Atlas views and selected-sprite clipboard pixels retain the opaque marker; BMP export preserves it. Cursors retain their own alpha and do not use this bitmap key.

`transparent` or alpha zero erases the active paint layer and reveals older sprite paint. A missing stamp palette character skips the pixel entirely. Neither operation replaces old opaque sprite backgrounds with holes. Unpainted base pixels export as black because RGB BMP cannot retain alpha.

For a moving fish, cat paw, or other silhouette:

1. Target the exact sprite with `origin`.
2. Fill its complete width and height with opaque `#ff00ff`.
3. Stamp the subject at full opacity. Spaces may skip because the whole cell is already keyed; alternatively map them explicitly to the key.
4. Repeat for every released/pressed variant. Shared EQ handles need one source repair.
5. Inspect native crops plus a four-pixel halo in all four states and GPU previews at both travel endpoints and intermediate positions.

Do not paste a fabric patch into a handle. It moves independently of its backdrop and creates a mismatched rectangle.

Draw, study, validate and export return `transparency`: exact key count, each distinct moving source cell, per-cell key/alpha/opaque/missing counts, and `opaque_moving_sprites`. The last list catches even tiny textured rectangles. It is a review warning, since a rectangular control can be intentional. `studio_pixel` reports raw `rgba`, `sprite_key` on each source hit, and the visible composed `visible_rgba`.

Cranamp supports this key. Identical transparency in other players is not verified, so `studio_validate.plays_the_same_elsewhere` remains false for keyed skins. Its format `divergences` may still be empty. Portability repair preserves deliberate keys instead of baking in a fixed background.

The Midnight Snack bug combined an artwork error (a fabric rectangle under every fish) with contradictory tools: the GUI called magenta transparent and MCP called it erasure, while the editor/player rendered purple. Regression tests now exercise exact key decoding, neighboring purple ink, layer erasure versus skip, export/reopen, both source states, and every volume position.
