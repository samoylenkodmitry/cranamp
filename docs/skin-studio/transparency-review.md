# Midnight Snack transparency correction

> Historical review of a retired Cranamp-only rendering mode. Its key-transparency claims and recipe are obsolete. Use [classic compatibility](classic-compatibility.md) and [REGION.TXT window cutouts](transparency.md). Current bundled skins have been migrated to opaque classic BMP cells.

The original repaint put fabric into each moving fish's entire rectangle. That fabric travelled with the control instead of revealing the artwork at its current position. Studio also contradicted itself: the color picker called opaque magenta transparent, MCP called it erasure, and the renderer treated it as purple. Layer erasure and sprite holes were conflated.

The repaired project keeps the five earlier layers and adds **06 · Fish without lunchboxes** through Studio MCP. All ten distinct released/pressed source cells now contain silhouette ink and an opaque magenta exterior: **1,162 keyed pixels**, no fully opaque moving cells, no missing or alpha-only pixels in those cells. The eight-pixel scroll fish is no longer clipped from a nine-pixel stamp. The replay script produces the same keyed handles on a fresh build.

| Region | Native rectangle | Visual result |
| --- | --- | --- |
| Seek, volume and balance fish | 16, 54, 248, 29 | Whiskers, cat fur, shelves and curtain remain visible around each fish. No teal rectangles. |
| All eleven EQ fish | 19, 154, 239, 64 | Shared source states reveal the local weave and rail through every exterior pixel. |
| Scroll fish | 256, 252, 11, 87 | Complete silhouette over the scrollbar rail; no moving fabric patch. |

Before/after captures include a four-pixel exterior halo and all four active/pressed states. The original five-layer project is preserved at `target/transparency-before.cstudio`; the visual comparison is `target/transparency-review/index.html`. Every corrected editor board and GPU crop was inspected, together with the twelve full-player captures for frames 0, 13 and 27 in all four states.

The live integration check proves layered-project/export GPU equality, checks all ten moving source cells, and captures five playlist heights. The whole-canvas real-writer probe ran again after repair: **414,700 native pixel/state checks, zero mismatches**. Each view has 82,534 writable pixels, including 1,216 hidden beneath the opaque spectrum, and 21,141 palette-only playlist pixels.

Rust regressions cover exact-key GPU decoding, neighboring purple ink, ignoring the key when selecting font ink, layer erasure versus skipped stamp characters, raw atlas visibility, export/reopen, every volume position in all four states, opaque textured-cell detection and preservation during portability repair.

Cranamp renders the key. Other players' key behavior remains unverified and is reported as such in Studio, rather than silently approving cross-player equality.
