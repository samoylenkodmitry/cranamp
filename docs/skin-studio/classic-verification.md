# Classic compatibility verification — 2026-09-22

All 18 WSZ files under assets/skins were migrated with Skin Studio's native pen. Their editable projects and GPU previews were refreshed. Original archives/projects were preserved under target/classic-migration/originals. The migration recipe is tools/skin-studio/classic_migration.mjs; it deliberately reopens those preserved originals, so do not rerun it over newer artistic revisions without review.

## Implemented

- Ordinary opaque BMP semantics, exact classic bitmap-font cell indexing, correct EQ controls and playlist title states, and atlas-derived EQ graph colors.
- No invented bitrate/sample-rate readouts. Stopped playback reveals underlying time/spectrum artwork; Studio's playback selector reaches the GPU preview.
- REGION.TXT parsing, whole-window clipping and mask generation as merged rectangular polygons. Both WSZ and layered-project persistence preserve regions. Generation is undoable; invalid masks refuse without modifying the document.
- A shared classic-winamp-v1 export gate for GUI, MCP and Apply to player. Incomplete alpha, ambiguous entries, malformed palettes and nonportable polygons refuse before replacing the destination. Project format 2 preserves unfinished base alpha independently.
- Updated authoring documentation, AGENTS.md, the installed Skin Studio skill and live MCP guidance.

## Evidence

- cargo test --lib --test bundled_skins_are_portable --test studio_drawing_e2e --no-fail-fast: **341 library tests, 1 bundled-archive test covering all 18 skins, and 18 GUI tests passed**.
- cargo build --bin cranamp succeeded. git diff --check passed.
- Actual GPU captures for every skin at all four active/pressed combinations and slider positions 0, 9, 18, 27 are under target/classic-migration/final. Stopped previews are beside the bundled WSZ files. The repeatable capture helper is test/skin-studio/capture-classic.mjs.
- Real Webamp 2.3.1 was loaded in a browser with the local harness. Purr Chaos Font Fixed, Midnight Snack and Silverplay were visually inspected, plus a generated mask cutting a corner and a rectangle over the time display. The cutouts revealed the browser background. This is representative external review, not a claim of exhaustive cross-player screenshot equality.
- The native GPU region fixture also checks clipping over live content. Primary Winamp, Webamp and Audacious sources were inspected for polygon interpretation. Audacious treats polygons as bounding rectangles, so portable output uses rectangular pieces.

## Remaining platform limits

The exact flat-area warnings for all 18 skins were recorded beside their captures as flat-review.json and reviewed against the full compositions. Retained negative space includes Catnip/Moonlit's continuous cream paper, the Purr Chaos/Purrmission green field connecting EQ and playlist, Silverplay's blue control rails and glass face, dark cat silhouettes in Cat Scan/Feral Night/Seance, and established instrument/readout surfaces in Cardboard/Freefall/Salvage/Moon Garden. These are editable design areas, not unavailable pixels. Midnight Snack and Sampler reported no flat-area warnings in the playing view. This review preserves the existing art direction; it is not an AAA quality certification.

No native Winamp or Audacious executable was available for screenshot verification. Native desktop click-through has not been certified. Cranamp still has no functional shade UI, although shade masks are imported/exported. Playlist system fonts, spectrum algorithms, EQ interpolation and player metadata can differ. These limits are also stated in the export report and [classic compatibility contract](classic-compatibility.md).

A passing archive audit establishes format compatibility. It does not establish visual parity for every runtime state or approve the composition.
