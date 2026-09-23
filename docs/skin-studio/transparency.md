# Classic window transparency

Classic Winamp skins support hard window cutouts through **REGION.TXT**. This is independent of BMP pixels: magenta (#ff00ff) is opaque artwork, and VISCOLOR slot 0 is an opaque RGB background. Classic controls copy complete rectangular source cells. There is no portable BMP sprite color key or partial opacity.

In Skin options, pick the exterior color with the brush and choose **Cut Normal exterior** or **Cut Equalizer exterior**. Studio flood-fills that exact color from the edges of MAIN.BMP or EQMAIN.BMP's 275×116 background and generates the region without changing any bitmap. **Reset** restores a rectangular window; Undo restores the prior configuration.

For automation use studio_regions:

- action:"generate", section:"Normal", transparent_color:"#ff00ff" cuts only that explicitly selected exterior color. exterior_only:false also cuts matching interior pixels.
- Supply rows instead for a precise mask: 116 strings of 275 characters, # visible and . cut out. WindowShade and EqualizerWS take 14 rows.
- action:"normalize" converts an imported polygon region to its native-pixel mask and regenerates portable rectangles.
- action:"list" reads masks; action:"remove" removes the selected section.

Studio serializes each mask as merged, disjoint four-point rectangles. This matters because Winamp uses nonzero-winding polygons, Webamp uses SVG polygons, and Audacious uses each polygon's bounding rectangle. Rectangles preserve the same pixel outline in all three. The generator rejects wholly invisible windows and masks exceeding Winamp's INI value buffer rather than writing truncated geometry. Playlist regions are not in this profile.

A window hole removes **all** content at that point, including text, controls and the background itself. It cannot make a fish slider reveal the illustration behind it. Design opaque moving cells against a compatible track cross-section; inspect released/pressed states and both endpoints. Do not copy unique fixed artwork into a moving handle.

Editing-layer alpha still works. Erasing reveals lower paint; a missing stamp symbol skips paint. Layered projects retain source alpha. WSZ export requires complete opaque sampled cells and emits RGB BMPs. CUR cursor transparency is separate.

The joined editor and full-height GPU player apply REGION.TXT. Native player surfaces allow alpha; the Studio presentation backdrop is black, so cutouts appear black there. Cranamp rolls up the main window without applying the WindowShade region, and does not draw the equalizer's or the playlist's shade: those regions are authored and preserved for external players. Invisible controls are shielded from activation; native OS click-through behavior is not certified.

See [classic compatibility](classic-compatibility.md) for export checks and cross-player verification limits.
