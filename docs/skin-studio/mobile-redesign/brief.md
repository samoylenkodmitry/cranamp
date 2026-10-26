This is the earlier design brief. See [current workspaces](README.md) for the implemented layouts.

# Skin Studio mobile redesign

Status: product design specification, October 2026. The native implementation
and validation limits are tracked in [the workspace guide](README.md).

## Product task

A person opens a skin, chooses a visible part, paints it, checks it in the player,
and saves a reusable project or a portable skin. The document and undo history
survive screen changes. File import is an entry action, not an editor setting.

## Screens

1. Projects. Visual project covers, a recent working copy, and a floating plus icon.
   The plus opens an action sheet: import a skin/project, start a blank project,
   or duplicate a bundled skin. Failed imports stay here with a useful error.
2. Parts. An illustrated map of Player, Equalizer, Playlist, Cursors and Windowshade.
   Selecting a part enters its canvas. The joined canvas is an explicit advanced
   scope. Selection changes the visible area; it does not silently change which
   source sprites receive paint.
3. Canvas. Artwork occupies the screen between two fixed toolbars. Top: back,
   project/part title, undo, redo and overflow. Bottom: brush, colour swatch,
   selection, layers and preview icons. The current tool is visible. Pan and zoom
   use two fingers; stylus or one finger paints. Hand mode remains available.
4. Tool settings. A sheet for the selected tool only: stroke sample, size slider,
   opacity, and that tool's own settings. The tool chooser is a grid of illustrated
   tool icons. Text fields, switches and sliders replace rows of word buttons.
5. Layers. Thumbnail rows with visibility, lock and drag handles. The active layer
   is marked. A row overflow holds duplicate/delete. Painting layers and classic
   sprite destinations are separate. Advanced sprite/state scope is explicit.
6. Preview. A working player, a play/pause control, and a part/state selector.
   Editing tools are absent. Back preserves canvas position. States show visual
   examples of pressed and inactive controls. Export errors link to the affected
   part and preserve the project.
7. Save and share. Project name, saved status and a format row. Save project retains
   layers. Export WSZ creates a player skin. Apply to player returns to playback.
   Destructive replacement and failed saves never silently discard current edits.

## Size rules

| Available logical width | Part selection | Painting | Secondary controls |
| --- | --- | --- | --- |
| 320–379, small phone | Full-screen illustrated list; one part per canvas | Fixed bottom icon toolbar; edge-to-edge canvas | Full-screen editor for layers/colour; short tool settings sheet |
| 380–599, large phone | Expandable thumbnail sheet | Canvas remains visible behind tool sheets | Bottom sheet up to 60% height, expandable for lists; only one sheet at a time |
| 600–899, compact tablet | Persistent part rail | Canvas plus rail | Inspector overlays only when requested; portrait uses a sheet if canvas space falls below 360 points |
| 900–1279, wide tablet | Persistent part rail | Canvas in the centre | Docked inspector |
| 1280+, desktop | Contextual parts or source bitmaps | Compose, Sprites, States and Preview workspaces | Working preview and compact properties in Compose; sprite destinations in Sprites/States; visual states and export report in Preview |

Width alone selects the layout class on every platform. Short windows use a
scrolling side toolbar and full-screen secondary panels. Resizing retains the
document and canvas anchor. Desktop scope entry fits the artwork to the available
space. Tool buttons remain grouped at the top of the rail.

## Interaction contract

All interactive targets are at least 48 logical points. Icons have accessible
names, selected state and tooltips for pointer users. A short caption belongs under
an unfamiliar icon in a chooser; text is not banned from the interface. Primary
navigation does not use rectangular word buttons. Colour is not the sole marker
of selection. Destructive actions require an explicit confirmation.

Canvas gestures never scroll the whole app. Fixed controls respect safe areas.
There is no permanent second preview on phones. Errors appear beside their action
or in a message sheet, not below a long canvas. Loading, cancellation and retry
have visible states. Import, export and audio preparation expose progress and a
cancel action. Project autosave is distinct from exporting a WSZ.

The mockups specify the experience and visual hierarchy. They do not certify
implemented gestures, storage, accessibility or platform file-picker behavior.

## Acceptance for the native editor

- Open a WSZ, ZIP or CSTUDIO from the system picker, select a part, make a stroke,
  undo it, preview and export without finding a control below the canvas.
- Cancel an import or fail validation without replacing the current document.
  Recover the working copy after leaving Studio and after process termination.
- Retain the selected part, tool, undo history and canvas anchor when rotating,
  resizing, opening a panel or returning from Preview.
- At 320 points, every toolbar action has a 48-point target and remains reachable.
  Long names truncate safely. Keyboard and screen-reader focus follow the active
  screen. A modal blocks interaction with the screen beneath it.
- On a large phone, opening a sheet reframes the part in the remaining visible
  canvas area. Closing the sheet restores the canvas anchor. On a compact tablet,
  use a bottom sheet when a side inspector would leave less than 360 points for
  the canvas. A wide tablet exposes colour and preview beside the tool properties.
- Test the actual GPU canvas with paint, pan and pinch gestures. Test file pickers
  on physical iOS and Android devices. Inspect safe areas and keyboard occlusion.
  HTML prototype checks do not satisfy these native acceptance checks.
