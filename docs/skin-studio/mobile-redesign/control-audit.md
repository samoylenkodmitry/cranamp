# Studio control audit

This audit compares the pre-adaptive Studio UI with the current UI. It concerns access to controls, not approval of the visual design. The old `skin_studio.rs` and the old chooser implementations in `mod.rs` were compared with every adaptive module.

## Regressions found

| Control | Finding | Current route |
| --- | --- | --- |
| Playlist colours | Data and editing methods survived. The UI was buried in Project actions, labelled Export options, and used a fixed three-column layout. Clicking a slot overwrote it with the brush colour. | Skin, Colours, Playlist on phones; Skin, Colours on wide screens. A selected slot has HEX and RGB fields and a colour picker. Edits leave the brush colour unchanged. |
| Visualizer colours | Same hidden panel and fixed-width overflow. Slot colours had no direct numeric editor. | Skin, Colours, Visualizer on phones; Skin, Colours on wide screens. All 24 slots remain available. |
| Bitmap font | The generator survived in the hidden options panel. | Skin, Text font. Separate ink and background fields, current bitmap preview, generation and pixel editing. |
| Window cutouts | Both full-height window generators and reset actions survived in the hidden panel. | Skin, Window shape. Explicit cutout colour, player and equalizer previews, cut and reset actions. |
| Missing cursor generation | No route remained in the adaptive UI. | Skin, Cursors, Draw missing cursors. Existing cursor files are preserved. |
| Cursor hotspot | No route remained in the adaptive UI. | Skin, Cursors. Exact X and Y fields replace the old one-pixel direction buttons. Cursor canvas settings also expose the selected cursor's click point. |
| Pointer coordinates | The old pointer readout was absent. | Hovering the canvas shows native pixel coordinates. |

No source palette, font, region or cursor data was removed to implement the adaptive layouts. Existing model and archive methods remain in use. The palette regression test reads PLEDIT.TXT from the actual archive after a UI edit, checks that the brush colour is unchanged, and checks Undo. The cursor regression test creates missing cursor files through the UI and checks the encoded hotspot after editing both fields.

## Other old controls checked

| Capability | Current route or replacement |
| --- | --- |
| Source bitmaps and sprite destinations | Sprites, source list, sprite targets |
| Joined canvas and individual windows | Canvas, part selector |
| Active and pressed artwork | Canvas state; visible state controls on desktop |
| State variants and paint scope | States; Canvas state, Paint into |
| Volume, balance, position, scroll, digit and playback frames | Canvas state |
| All ten drawing tools | Visible tool rail and Drawing tools |
| Brush settings, text, gradients and masks | Context properties and Tool settings |
| Copy, stamp, flips, rotation, repeats and state sweep | Lift and Stamp properties |
| Saved brush swatches | Colour editor, add, replace and remove |
| Paint layer ordering, opacity, visibility, lock, merge and delete | Painting layers |
| Undo, redo and history restore | Header and Edit history. On small phones Redo is in Project actions. |
| Grid, fit, zoom and scroll | Canvas controls; Pan properties |
| Pixel inspection and colour sampling | Pixel study and canvas sampling. The old `inspect` call was a diagnostic during sampling, not a separate editor. |
| Sprite bounds and paint clipping | Project actions, Sprite rectangles |
| Equalizer frame workbench and artwork-only export mode | Project actions, EQ workbench |
| Live player and preview resize controls | Preview |
| Agent connection | Project actions on desktop platforms |
| Save project and export skin | Save and share through platform document services. The old direct filesystem methods are no longer GUI entry points. |
| Classic format validation | Skin, Format check; export still validates the archive |

The old `canvas_size` and `state_sheet_layer` calls supplied dimensions and the selected sprite. The viewport and selected sprite model now supply those values. They were not independent user actions.

## Visual review status

The current [v9 gallery](../../../target/ux-audit/v9/index.html) contains 2,470 native GPU captures: 130 named states at 19 window sizes. All 133 overview sheets were inspected, with full-size inspection of representative phone, short-window and desktop captures. The catalogue includes the restored settings, missing and populated cursor files, and the original drawing-tool cases. It is a finite catalogue of layouts and control states.

The quick-colour control now uses the shared icon face and a colour strip. The thick white frame is gone. Labels use regular weight below 14 logical pixels and medium weight above it. Layout gutters use 8 pixels. Short windows move workspace selection into a menu to retain canvas height. Font and format pages put the skin preview beside the controls when space permits and below them in narrower windows.

Cursor rows show rendered cursor pixels and control names. The archive filename remains secondary text. Click-point fields are bounded to the width of their values, with the selected cursor beside them when space permits.

The palette and cursor UI regression tests pass after these changes. Automated assertions do not approve composition or pen behaviour. Physical S Pen hover, pinch gestures and manual DeX resizing remain unverified.
