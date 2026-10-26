# Image generation prompts

Generated with the built-in image generation tool. These are UX mockups, not exported skins.

## phone-flow

Use case: ui-mockup. Create a high-fidelity product UX design board for a completely redesigned Cranamp Skin Studio mobile app. This is a real pixel-skin editor, not a dashboard. Landscape design board with FIVE clear, large, flat phone screens side by side, minimal neutral white board background, precise UI typography, no perspective, no decorative phone hardware. Each screen has a small caption below outside the app: "Projects", "Choose a part", "Paint", "Layers", "Preview". The phones represent a SMALL 360px-wide phone and have the same dimensions.
Design language: matte near-black navy app surfaces, warm-white type, restrained cyan active indication, crisp thin-line vector-style icons, 48px minimum touch areas, excellent spacing. The skin artwork is a tiny illustrated classic Winamp player with a silver cat and muted blue pixel art; keep the example consistent across screens. Do not use rows of rectangular text buttons, desktop toolbars, or giant dashboards.
Screen 1 Projects: app title "Skin Studio" at top, small settings icon, one prominent illustrated recent project card "Silverplay" with a subtle saved indicator, two smaller project thumbnails below, floating circular plus icon at bottom right, simple bottom navigation icons for projects and player. File import starts at plus, not in an editing toolbar.
Screen 2 Choose a part: back chevron, title "Choose a part", a vertical set of large illustrated preview rows: Player shows the main 275x116 cat player; Equalizer shows the sliders; Playlist shows list with decorative cat footer; Cursors shows a few tiny pointer arrows. Selection is a preview image with a thin cyan border, not a pill button. Small captions identify parts. Joined canvas is one quiet row at the bottom.
Screen 3 Paint: header back chevron left, centre "Silverplay" and smaller "Player", undo and redo icon buttons at right. Canvas occupies about 75 percent of the screen, showing ONLY the selected main player part at editable zoom with a subtle pixel grid and a circular brush cursor. At bottom a fixed five-item icon-only toolbar: pencil, circular colour swatch, selection rectangle, stacked layers, play-in-circle. Pencil selected with a cyan indicator. Small unobtrusive zoom readout and thumbnail navigator inside canvas. NO property panels, NO lists, NO second preview, NO words on buttons.
Screen 4 Layers: full screen editor, back chevron with title "Layers", plus icon, four clean layer rows with square art thumbnails, short layer names "Highlights", "Cat", "Controls", "Background"; eye icon, lock icon and drag handle on each. Highlights selected with cyan edge. Below a slider labelled "Opacity" value "85%". No toolbar grids. Paint is reached by back.
Screen 5 Preview: back chevron and title "Preview", working full Winamp skin composed of Player, Equalizer and Playlist centred and large, a single circular play/pause icon, and a simple state chooser showing two small visual thumbnails with captions "Normal" and "Pressed". Top right a share icon. No editing controls.
Keep all controls inside safe areas and screen bounds. Show believable original application UI, detailed pixel artwork, clear screen-to-screen continuity. This board is for implementation review, not a marketing poster. No slogans.

## adaptive-layouts

Use case: ui-mockup. Generate the second product design board for Cranamp Skin Studio. Use the preceding design board as a visual-language reference: dark navy surfaces, warm white type, restrained cyan, silver-and-blue cat pixel skin, crisp thin line icons. Design TWO LARGE flat app mockups on a pale neutral board, no perspective or device hardware, with clear captions underneath. Landscape composition about 1800x1100.
Left mockup about 35% of board width: LARGE PHONE portrait, caption "Large phone — part sheet". It shows a painting canvas behind a bottom sheet that occupies 45% of the phone. Header back chevron, "Silverplay" with "Equalizer" sublabel, undo and redo icons. Selected equalizer artwork on canvas. Bottom sheet has a drag handle, heading "Parts", and a 2-by-2 grid of ACTUAL ARTWORK THUMBNAILS with small captions Player, Equalizer, Playlist, Cursors. Equalizer has a cyan outline and check mark. At the bottom a quieter full-width thumbnail captioned "Joined canvas". These are visual choices, not pill buttons. A close icon dismisses sheet and preserves zoom. No screen-wide tab bar with words.
Right mockup about 65% board width: TABLET landscape, caption "Tablet — canvas with part rail and inspector". Show header "Silverplay" and a subtle "Saved" status, back/projects icon left, undo and redo, preview play-circle, share and overflow icons right. Persistent narrow left rail contains four vertically stacked small artwork thumbnails with understated captions Player, EQ, Playlist, Cursors. Player is selected in cyan. A narrow secondary vertical icon toolbar beside the rail has brush, eraser, selection, hand and eyedropper icons. A generous central canvas shows the selected MAIN PLAYER cropped/zoomed pixel cat artwork, not all skin panels; zoom readout at bottom, unobtrusive small navigator. Right inspector about 24% of app width is open to the selected pencil tool: title "Pencil", a stroke preview, a circular ink swatch, size slider with "3 px", opacity slider with "100%", a switch row "Pixel grid", and underneath a section "Layers" with three thumbnail rows Cat, Controls, Background and eye icons. Use ordinary form rows, sliders and icon controls; NO grid of word buttons, NO duplicate second live-player panel.
Both mockups must use the same document and icon system as the reference, realistic safe-area spacing, at least 48 logical px touch targets, strong typography and no crowding. Do not show numerical canvas dimensions. Do not add marketing text, gradients, extraneous decoration, or unsupported promises. The layout changes the working scope and tool placement with screen size; it is not a stretched phone screen.

## tool-colour-save

Use case: ui-mockup. Create a third high-fidelity Cranamp Skin Studio mobile product design board showing THREE flat large-phone app screens, equal size, side by side, on a neutral pale board. Follow the previous boards' matte near-black navy, warm-white type, thin line icon system and restrained cyan. No perspective, no phone hardware, no marketing text, no rows of rectangular word buttons. Screen captions outside the apps: "Brush settings", "Colour", "Save and share".
First phone: a selected cat pixel-art main player canvas remains visible in the upper 55% of the screen. Header shows back chevron, "Silverplay" and small "Player", undo/redo icons. A bottom sheet at 45% height has a drag handle and close icon, title "Pencil", a visually accurate stepped pixel stroke preview, circular colour swatch, a Size slider reading "3 px", an Opacity slider reading "100%", and a switch row "Pixel grid". It contains only pencil settings. Five icon toolbar is partly replaced by sheet, never hidden underneath actionable content. All controls have generous touch area.
Second phone: full-screen colour editor for a small phone. Back chevron and title "Colour", a large saturation/value square, vertical hue strip alongside, current and previous circular colour swatches, hex text field reading "#87ABC9", a 2-row grid of eight large colour swatches under heading "From this skin", and a circular eyedropper icon. The apply action is a check icon at upper right. No text-pill toolbar and no irrelevant advanced controls.
Third phone: full-screen Save and share. Back chevron, title "Save and share". A modest thumbnail of the whole current skin with project title "Silverplay". A name field under label "Name". Two large native list rows, each with an icon, readable title, one explanatory sentence and trailing action arrow: "Save project", subtitle "Keep layers and editing history"; "Export skin", subtitle "Use in a classic Winamp player". A third row with a player icon, "Use in Cranamp", subtitle "Apply this skin to your player". Saved-status text with a checkmark below "Project saved on this device". At bottom a quiet informational note "Your original skin stays unchanged." Keep the page sparse and focused. Do not represent these choices as coloured word buttons.
Design these as implementable native mobile screens: clear hierarchy, consistent icon geometry, 48px touch targets, content respecting safe areas, all text within bounds. Skin artwork is illustrative; do not show dimensions or fake exported files.


## Desktop scope revision

Built-in image generation, October 24, 2026. Output: desktop-workspaces.png.

Create a dark navy and cyan desktop editor board with four distinct workspaces:
Compose, Sprites, States and Preview. Compose uses an enlarged pixel canvas,
compact tool rail, contextual part navigation, working player preview and compact
brush/layer controls. Sprites shows an enlarged individual sprite, source bitmaps
and destination controls. States shows a numbered variant sheet. Preview shows
the working player beside state controls and an export report. Avoid large empty
areas and stretched phone controls. Generated skin artwork is illustrative.

## Width and tool access revision

Generate five layouts at 320, 430, 768, 1100 and 1440 logical pixels. Small phones
use a compact drawing bar and live settings; larger phones gain direct shapes.
Tablets use a side tool rail and bottom settings. Wide tablets dock the inspector.
Desktop adds source navigation and a working preview. Use quiet surfaces, short
labels and small custom icons. Keep primary controls visible. See
[width-workspaces-v2.png](width-workspaces-v2.png). Icons are authored SVG assets,
not generated bitmap artwork.
