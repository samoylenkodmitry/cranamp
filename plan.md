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

## Android Support

- [x] Enable the native audio backend for Android builds so Rodio/CPAL playback is compiled into the APK.
- [x] Add an Android-specific single-surface Winamp app where the main window, equalizer, and playlist are always stacked vertically.
- [x] Make the Android content height collapse when the equalizer or playlist panels are hidden.
- [x] Add an Android `NativeActivity` subclass that opens SAF file, folder, playlist import, and playlist export flows.
- [x] Copy selected Android audio documents into app-private files so the existing path-based playback backend can play them.
- [x] Wire Android picker results into Cranamp replace/append/import/export playlist behavior.
- [x] Keep Android manifest freeform metadata aligned with the full stacked Winamp height.
- [x] Track Cranpose blockers for true always-on-top overlay hosting and app-driven Android native window resizing, which are not exposed by Cranpose 0.0.61 (`samoylenkodmitry/Cranpose#232`, `samoylenkodmitry/Cranpose#238`).
- [x] Scale the Android stacked composition to the live freeform/native-surface width.
- [x] Stretch the Android playlist panel height from the remaining native-surface space so the stacked Winamp surface fills resizable freeform windows.
- [x] Keep Android pointer hit testing in scaled skin coordinates while using the responsive freeform layout.
- [x] Patch Cranpose Android pointer conversion to add the native-surface/content-rect inset, keeping finger hits aligned when Samsung inflates the surface for freeform shadows.
- [x] Add Cranamp-side Android drag hit testing for non-interactive Winamp skin areas.
- [ ] Blocked: app-driven Android `NativeActivity` freeform task moving is not exposed reliably. `View.startMovingTask`, `IWindowSession.startMovingTask`, hidden API exemptions, and UI-thread `Window.setAttributes()` were tested on Samsung DeX/freeform; task bounds did not move, and `Window.setAttributes()` caused native-surface resize churn.
- [x] Add the freeform move attempt as a best-effort Android bridge and fail it once per gesture when the platform blocks movement.
- [x] Subtract Android surface insets when starting a freeform move so the raw drag anchor stays under the finger.
- [x] File the Cranpose input-coordinate blocker for Android freeform `surfaceInsets` (`samoylenkodmitry/Cranpose#240`) and add the freeform movement findings to the existing overlay blocker (`samoylenkodmitry/Cranpose#232`).

## Floating Surface Strategy

- [x] Reclassify Android freeform support as an optional desktop/tablet UX, not the core always-on-top floating player.
- [x] Identify the true Android floating-player path: `SYSTEM_ALERT_WINDOW` permission, `Settings.canDrawOverlays`, `Settings.ACTION_MANAGE_OVERLAY_PERMISSION`, an explicit overlay lifecycle, and `WindowManager.addView()` with `TYPE_APPLICATION_OVERLAY`, `FLAG_NOT_FOCUSABLE`, and `PixelFormat.TRANSLUCENT`.
- [x] Confirm that Cranpose's current Android runtime only renders into the launcher `NativeActivity` `ANativeWindow`, so a skinned always-on-top overlay cannot be implemented correctly in Cranamp alone.
- [x] Keep the Cranpose blocker tracked in `samoylenkodmitry/Cranpose#232`: Android overlay support needs a service-owned `Surface`/`SurfaceView`/`TextureView` or equivalent `ANativeWindow` host API, pointer translation, and lifecycle handling outside the Activity window.
- [ ] After Cranpose exposes an overlay-capable Android surface host, add a Cranamp `OverlayService` that owns permission checks, overlay creation/destruction, drag/dismiss gestures, and a compact stacked Winamp surface.
- [ ] Add an Android in-app "floating mini-player" action that requests overlay permission when needed, starts the overlay service when allowed, and falls back to the normal Activity/freeform path when denied or revoked.
- [ ] Decouple Android playback/controller state enough that the overlay and full Activity can issue transport, seek, volume, load, import, and export commands without depending on the Activity window being foregrounded.
- [ ] Treat Android freeform launch metadata as a debug/desktop fallback only; do not rely on it for always-on-top behavior.
- [x] Identify the browser/WASM experimental path: Chromium Document Picture-in-Picture can host the existing Cranamp canvas after a user gesture, but it is browser-controlled and not equivalent to a native transparent overlay.
- [x] Add a web-only Document Picture-in-Picture path that reparents the live Cranamp canvas into the PiP document and restores it to the main page when PiP closes.
- [x] Keep the PiP path feature-detected, with a disabled launcher and normal embedded widget fallback when unsupported.
- [x] Resize the web Cranamp host/PiP window from the stacked Cranamp surface size.
- [x] Let the web PiP canvas fill a resized PiP viewport, then scale Winamp to the canvas width and stretch the playlist to the remaining height.

## Skin Loading

- [x] Replace the bundled Winamp skin with `~/Downloads/cranampskin.wsz`.
- [x] Make the main-window logo/menu button open an external `.wsz/.zip` skin picker on desktop and web.
- [x] Add Android SAF skin import support so selected `.wsz/.zip` archives are copied into app-private storage and decoded by Cranamp.
