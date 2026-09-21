# Native image stamps

`studio_draw` image operations accept optional `source_rect: [x, y, width, height]` in source PNG pixels. The crop must be nonempty and inside the source. Set both `width` and `height` to explicitly fit it to native skin pixels; each dimension must be 1–2048. Cropping happens before nearest-neighbour resizing. Omit the dimensions to retain one source pixel per skin pixel.

```json
{"op":"image","x":0,"y":0,"data":"BASE64_PNG","source_rect":[0,0,1100,1508],"width":275,"height":377}
```

Transparent pixels preserve the artwork underneath; partial alpha blends after resizing. The normal sprite selection, state propagation, preview and single-step undo apply. Invalid geometry rejects the complete transaction without leaving earlier operations behind.

Use `studio_study` to inspect joins and shared cells, then `studio_screenshot` to review the actual GPU player.
