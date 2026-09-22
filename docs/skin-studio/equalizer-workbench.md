# EQ workbench

Open **EQ workbench** in the drawing or live-preview toolbar. The GUI and `studio_eq` use the same document operations and undo history.

## Visible controls

Classic EQMAIN.BMP has 28 level frames of 14×63 pixels. All eleven sliders, including preamp, share that bank; the two 11×11 handle images are shared too. A different level is not a different band. Painting each frame cannot store a different slice of an illustration under every slider.

1. Choose **Visible controls**, then a level from 0–27.
2. Choose **Paint shared track** with **This frame** or **All 28**. The selected cell appears on the joined canvas.
3. Match its opaque backdrop exactly to the adjacent background. Match the released and pressed handle backdrops too; a handle carries its entire rectangle. Use a guide line only if it joins correctly through the handle.
4. **Copy this frame to all 28** duplicates the completed track using one undoable native-pen transaction. It leaves handles and surrounding art untouched.
5. Use **Same level**, **Rising levels**, **High/low**, and **Compare all 28 frames**. Check released/pressed GPU previews at both endpoints and intermediate levels.

Keep unique cat contours and other stationary details outside shared track cells, or design genuinely repeating detail. Do not copy a fixed background picture into a moving handle. A continuity report cannot detect every unintended repeated mark elsewhere in the shared bank; inspect all eleven destinations.

The Catamp Purr Chaos repair keeps visible paw handles. Tracks, handles and surrounding EQ paper now share RGB `#bed1aa`; subtle guides use `#9eb495`. The uniform paper is an intentional continuous ground between the illustrated cats, not an inaccessible rectangle.

## Artwork-only EQ

The user’s [Björk reference](https://skins.webamp.org/skin/0ac4669a78b4ddec5a2343ef097d865c/_B_J_O_R_K_.wsz/) contains a 275×163 EQMAIN.BMP. The file ends before slider graphics start at y164. This omits track, handle, graph and preset graphics; it does not make BMP magenta transparent. Invisible control hit targets remain.

**Artwork only** exports that exact cropped convention and removes those graphics from Studio’s composition. The layered project retains the complete source atlas, so **Visible controls** restores it. Importing a genuinely cropped WSZ cannot restore pixels that were never included. Validation recognizes this specific omission with a warning; arbitrary truncated cells still fail. Check the exported file in each target player before claiming parity.

## MCP

`studio_eq {}` reports the shared bank, native sources, destinations and current mode. Actions: `edit_frame`, `edit_handle`, `edit_background`, `copy_frame_to_all`, `preview`, and `set_mode`. Frames are 0–27; editing scope is `current` or `all`; preview patterns are `flat`, `ramp`, `alternating`; modes are `controls`, `artwork`. Use current tools/list schemas for exact parameters. All artwork still goes through `studio_draw` and the shared GUI pen.

Save the `.cstudio` project, validate, export, reopen the WSZ without its companion project, and compare actual GPU captures. Format validity is separate from visual approval and cross-player certification.
