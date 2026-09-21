# Pixel audit and flat-area review

Run `studio_probe {"path":"target/pixel-probe.json"}` before reserving any blank space. It exercises every native pixel through the real paint writer in four active/pressed views, with Auto targets and all source variants, on a disposable document. Two different inks prevent an already-painted or aliased source from being mistaken for an unwritable pixel. The saved JSON contains a coverage character and a verified-write character for every pixel in every view. `rows:true` includes those maps in the tool response; omit it for a compact response. A region can be supplied with `rect:[x,y,w,h]`.

The probe checks intrinsic source capability. Clip, color masks, alpha lock, sprite selection and locked paint layers are deliberately disabled in the disposable copy; the live document, view, clipboard and history are untouched. It does not claim that a painted source remains visible through runtime content, or that arbitrary artwork can differ between positions sharing the same source cell.

Coverage letters:

| Letter | Meaning |
| --- | --- |
| D | Writable bitmap |
| S | Writable shared or stateful sprite |
| R | Writable bitmap under a runtime footprint; not an exclusion |
| O | Writable bitmap hidden by the opaque spectrum background |
| P | Playlist palette fill; no bitmap source |
| X | No source mapped on this surface |

Spectrum coverage follows the skin palette: VISCOLOR background slot 0 set to
opaque `#ff00ff` reveals the skin and changes those pixels from O to R. Lit
analyzer bars remain runtime content. The disposable probe retains the live
palette, so its classification agrees with coverage and the GPU.

Large draw transactions, Pixel Study and export now report `flat_drawable_areas`. Export checks all four active/pressed compositions, even when an atlas is selected. The GUI Pixel Study shows the warning count. Each reported region includes its color, connected pixel count, bounds, an entirely flat `largest_flat_square`, and overlapping source targets at that square.

This is a review aid, not an artistic verdict: it detects exact-color connected areas of at least 256 visible drawable pixels containing an 8×8 square. It includes text backdrops and excludes palette-only fill and the opaque spectrum. It does not detect every sparse or low-contrast composition. Intentional fabric, sky or negative space may trigger it; visually inspect the reported square, neighboring sprites, alternate states and GPU result.

Never use successful export, zero mapping mismatches or passing tests as visual approval. A flat clear is still a clear even if it makes text easier to read.
