# Matter examples

Valid `.pif` (Pulsar Image Format) documents. A `.pif` is a folder bundle: a
`manifest.json` plus QOI tiles for painted pixels under `.raster/`. The
examples hold only manifests (nothing is painted yet), so they open in the
Matter editor as empty, correctly set up canvases. This folder is read-only:
copy one into your project to paint on it.

JSON has no comments, so each manifest has a `description` field.

| Bundle | Teaches |
|---|---|
| `BlankCanvas.pif` | The smallest document: one empty raster layer, 512x512 sRGB |
| `LayerStack.pif` | Several layers bottom to top: raster, vector shapes, a half-transparent layer, a hidden layer |
| `LinearMask.pif` | The `Linear` color space and a non-default `tile_size` for data textures |
| `VectorBadge.pif` | Vector-only documents: `rect`, `circle` and stroked `path` elements |

Layer fields: `id` (stable), `name`, `visible`, `opacity` (0 to 1), `blend_mode`
(`normal`). A raster layer maps tile coordinates `"x_y"` to files such as
`.raster/<name>.qoi` in `tiles`; a vector layer lists `elements`.
