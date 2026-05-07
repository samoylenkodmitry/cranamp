# Cranamp Audacious Reference Plan

Reference inputs:
- Audacious reference: user-provided screenshot plus captured windows in `screenshots/audacious-*.png`.
- Cranamp baseline: `screenshots/cranamp-main-current.png`, `screenshots/cranamp-eq-current.png`, `screenshots/cranamp-playlist-current.png`.

## Differences Found

- [x] Capture the current Cranamp desktop windows and Audacious reference windows for direct comparison.
- [x] The main window renders the skin frame and controls, but its LCD text color and metadata do not match the Audacious reference closely enough.
- [x] The main window always shows both mono/stereo indicators as off; Audacious lights `STEREO` when a track is loaded/playing.
- [x] The equalizer graph only draws the static graph background and center line; Audacious draws a blue EQ response curve.
- [x] The playlist default window is shorter than the reference Audacious playlist.
- [x] Playlist rows are too sparse for the taller reference playlist and use green LCD text instead of the skin playlist colors from `PLEDIT.TXT`.
- [x] Playlist entries do not show right-aligned durations.
- [x] Playlist current-row styling does not match the reference selected/current row treatment.
- [x] Playlist footer LCD strips are blank; Audacious shows duration/total and elapsed readouts.

## Implementation Plan

- [x] Add duration metadata for bundled demo tracks so playlist rows and footer readouts can render stable times.
- [x] Increase the default playlist height to match the captured Audacious reference and adjust default stacking.
- [x] Rework playlist row rendering with denser rows, skin playlist colors, current row background, and right-aligned duration text.
- [x] Add playlist footer readouts for current duration/total duration and elapsed time.
- [x] Switch main LCD bitmap text to the skin's blue display color, show `320kbps 44khz` metadata when a track is selected, and light the stereo sprite when audio is present.
- [x] Draw an equalizer response curve over the EQ graph based on the current EQ band values.
- [x] Rebuild, test, and recapture Cranamp after implementation.

## Functional Follow-Up

- [x] Wire the playlist footer's tiny transport icons to the same previous/play/pause/stop/next/eject actions as the main window.
- [x] Make the playlist `REM`, `SEL`, `MISC`, and `LIST` footer buttons perform useful playlist actions instead of acting as decoration.
- [x] Add mouse-wheel scrolling over the playlist list area.
- [x] Add tests for the new playlist mutation behavior.
- [x] Rebuild, test, and recapture Cranamp after the functional pass.

## Rendering Polish

- [x] Compare stopped and playing Cranamp captures against the Audacious reference.
- [x] Fix the playlist selected-row background to render the skin's `#42351e` sRGB color instead of a too-light linear interpretation.
- [x] Split the main bitrate and sample-rate display into two independently positioned LCD readouts.
- [x] Tighten the main bitrate/sample-rate text spacing so the readouts match the Audacious bitmap density.
- [x] Align the playlist footer elapsed timer baseline with the Audacious LCD strip.
- [x] Rebuild, test, and recapture the corrected playing-state UI.

## Playlist Menu Parity

- [x] Add a persistent playlist multi-selection model with selected rows, an anchor row, and state cleanup after replace, append, remove, sort, and restore.
- [x] Render multiple selected playlist rows with the Winamp selected-row treatment.
- [x] Add `REM` menu actions: remove all, remove duplicate tracks, remove selected tracks, and remove unselected tracks.
- [x] Add `SEL` menu actions: select none, select all, search/select matching tracks, and invert selection.
- [x] Add `MISC` menu actions: sort by available track fields and randomize playlist order.
- [x] Add `LIST` menu actions: new playlist, import `.m3u/.m3u8`, and export `.m3u`.
- [x] Parse and write M3U playlists, including the exported `~/Desktop/test.m3u` path-per-line format.
- [x] Add focused tests for multi-selection, removal modes, sorting/randomizing, and M3U import/export behavior.
- [x] Rebuild, test, run the app, and recapture the playlist menu behavior.

## Playlist Interaction Parity

- [x] Make a single playlist row click select only, without changing the currently playing track.
- [x] Make a second plain click on the same row within the double-click window start playback.
- [x] Support Shift-click range selection from the previous anchor row.
- [x] Support Ctrl-click toggling for the clicked row while keeping the rest of the selection.
- [x] Change `SEL` -> `SEARCH` into an editable playlist search/filter overlay that live-selects matches.
- [x] Use the full playlist row width for title clipping and keep durations right-aligned.
- [x] Render the currently playing playlist row as a scrolling marquee string.
- [x] Add tests for playlist click selection and search filtering.
- [x] Rebuild, run Cranamp, and capture the updated windows for a rendering sanity check.
