# Keep brush strokes continuous across components

A joined canvas is a coordinate system, not a guarantee that separately authored fragments match. Midnight Snack exposed three authoring mistakes:

- A handwritten sprite list omitted the preamp line. Its old dark pixels remained above the repainted background and graph.
- Fabric was generated per rectangle, restarting its vertical phase and changing its horizontal repeat from 18 to 25 pixels at a join.
- Separate footer crops met at a straight vertical splice.

Draw crossings once in native canvas coordinates. Use Auto and all states for a visible stroke. Preserve one texture phase and scale across rectangles. When a shared control backdrop cannot store different artwork at each location, use a keyed backdrop and put the artwork on the unique background beneath it. A repeating playlist tile must still obey its physical source period.

Every canvas `studio_draw` now returns `continuity`. It compares final opaque requested ink with the actual composed editor pixels. With `states:"all"`, it checks all four active/pressed combinations. Samples name the native coordinate, wanted/visible colors, and covering source sprites. Clipping, unmapped pixels and final shared-cell conflicts make the report fail. `shared_source_overwrites` is only a history count: consistent later overpainting of a repeated rail is valid. GUI stroke completion shows the same interruption warning.

`require_continuity:true` rejects and rolls back a failed draw before history is recorded. A dry run (`preview:true`) returns the report and restores the document. Tests prove that a hidden preamp stripe is caught, Auto reaches every overlay, failures preserve artwork/history, and selected-part clipping/shared aliases cannot claim success.

This check is deliberately concrete: it checks written opaque ink, not pre-existing artistic seams, runtime GPU overlays, erased/keyed pixels or pixels omitted by masks. Intentional occlusion can warn. Non-opaque, hidden or clipped paint planes, and draws with no opaque ink to check, return `continuous:null` instead of a false pass; strict mode rejects those too. Whole-component crops still require native boundary/halo review and GPU inspection. Never equate zero mapping errors with visual approval.

The repaired artwork and native crop findings are recorded in [the continuity review](continuity-review.md).
