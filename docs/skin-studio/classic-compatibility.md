# Classic Winamp export contract

Skin Studio authors genuine classic WSZ archives. It no longer previews a private magenta-transparency dialect. A valid archive is a prerequisite for comparison, not proof that every target player renders every runtime feature identically.

## What Studio guarantees

The editor, GPU preview and export use opaque BMP pixels, including magenta. Export writes RGB Windows BMPs and explicitly carries PLEDIT.TXT and VISCOLOR.TXT. All publishing paths validate the document before replacing a WSZ. Incomplete source cells, invalid sheet dimensions, ambiguous filenames and unsupported window polygons are actionable errors. Validation never silently deletes sheets or fills artwork. Save an unfinished layered project instead of forcing an export.

studio_validate.exportable is the format result. The old plays_the_same_elsewhere field is null; external screenshot verification is separate. Magenta counts diagnose old projects without banning legitimate pink artwork. transparency.incomplete_moving_sprites identifies alpha or missing source pixels. Opaque moving sprites are normal.

TEXT.BMP is the actual font: classic cells are 5×6, including their spacing and opaque backdrop. studio_font {ink,background} and **Build classic text font** paint a correctly indexed atlas through the shared native pen. It is undoable and affects no illustrated captions. NUMS_EX.BMP takes precedence over NUMBERS.BMP; its extra blank/minus cells need a width of at least 108 pixels. EQ graph colors come from EQMAIN.BMP's gradient column, not the text font.

[REGION.TXT](transparency.md) supplies window cutouts. Studio generates disjoint rectangles for Audacious as well as Winamp and Webamp. The file is named REGION.TXT, not config.txt. Configuration also includes the two palette files; there is no universal config.txt that changes sprite drawing rules.

Project format 2 records this profile and preserves base/paint-layer RGBA separately from the opaque WSZ preview. Version 1 opens with a legacy warning; review every magenta placeholder before export.

The [EQ workbench](equalizer-workbench.md) exposes the 28 shared track frames and both handle states. Its explicit artwork-only mode preserves source pixels in the project but exports the canonical 275×163 EQMAIN.BMP omission used by the Björk reference. This specific omission receives a compatibility warning; other partial cells remain errors. Visible paw controls use full atlases with backdrops matching the surrounding paper.

## Review workflow

1. Author continuous artwork in native joined coordinates. Use coverage and all source variants when a stroke crosses cells.
2. Inspect editor studies with four pixels of context, all active/pressed states, and actual GPU output. Shared source cells cannot hold different pictures at different destinations. Use intentional track designs that remain correct at every position.
3. Run studio_validate, export, reopen the WSZ and repeat the GPU check. Format validation is not visual approval.
4. Compare in actual target players with the same scale, window focus, playback state, slider positions, playlist size, font settings and media. Cranamp rolls up the main window; verify the equalizer's and playlist's shades in external players, which Cranamp does not draw.
5. Keep evidence under a review directory. Record player/version and which states were checked. Do not report 'identical everywhere' based only on a source atlas or a structural test.

A local Webamp harness is in test/skin-studio/classic-preview.html. Serve the repository on localhost and open that page; it loads Webamp 2.3.1 from the npm CDN and the local bundled WSZ files. Silent test media is generated locally. No artwork is uploaded. The region fixture button expects target/classic-migration/region-fixture.wsz from a Studio test export.

## Limits of visual identity

The playlist takes Winamp's sizes, 275×116 and whole steps of 25 by 29, and
tiles PLEDIT.BMP's footer cell at `179,0` between the corners, with the
visualizer panel at `205,0` from 350 wide. A skin that leaves those cells
blank shows blank footer bars in every wide playlist, in Winamp as in
Cranamp. Playlist rows use system fonts and can differ between operating systems.

The visualizer is Winamp's. The field is VISCOLOR colour 0, dotted with colour 1 on
every other pixel of every other row. The analyzer's 19 bars are three pixels wide,
each row in its own colour from 2 at the top to 17 at the bottom, with peaks in 23.
The oscilloscope uses 18 to 22 by distance from the middle row. The main window
draws it at `24,43`. With the main window closed (Alt+W), the playlist draws it in
the visualizer panel at `2,12`, 72 columns wide. Rolled up, the main window
draws a 38x5 version at `79,5` in the strip: ten bars in colours 4, 8, 11, 14 and 17
from the top, with no dots and no peaks, or the oscilloscope in colour 18. As in
Winamp, it covers whatever the strip has there. It shows nothing while stopped, so
the skin's own art is visible then. Interpolation of the EQ curve, media metadata,
focus and playback animations are runtime behavior rather than pixels encoded by the skin. Cranamp draws the main window's shade, not the equalizer's or the playlist's. Native click-through and external Winamp/Audacious screenshots still require platform testing. Portable assets cannot eliminate these player differences by adding a configuration file.

## References

- [Configuration guide supplied by the user](https://winampskins.neocities.org/config)
- [Winamp region parsing](https://github.com/alexfreud/winamp/blob/community/Src/Winamp/Skins.cpp) and [window winding regions](https://github.com/alexfreud/winamp/blob/community/Src/Winamp/Set.cpp)
- [Audacious region rectangle interpretation](https://github.com/audacious-media-player/audacious-plugins/blob/master/src/skins-qt/skin-ini.cc)
- [Webamp region parser](https://github.com/captbaritone/webamp/blob/master/packages/webamp/js/regionParser.ts)
