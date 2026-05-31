# Plugin Matter

A professional texture painting and material authoring plugin for Pulsar Engine. Plugin Matter provides a tile-based raster painting system built on WGPU, with full undo/redo history, a multi-layer compositor, and deep integration with the PIF (Pulsar Image Format) asset pipeline.

---

## Overview

Plugin Matter embeds a self-contained texture editor into the Pulsar Engine panel system. The editor renders painting layers through a custom WGSL tile shader, composites them per-frame directly on the GPU, and records every stroke as a command-pattern operation so the full session history is undoable. The canvas uses a WgpuSurface that operates independently of the main GPUI render pass, allowing smooth HiDPI rendering at full paint resolution regardless of window scale factor.

The editor is laid out as three panels: a layer list on the left, the interactive canvas in the centre, and a properties and colour inspector on the right. A toolbar at the top carries the tool palette and undo/redo controls.

---

## Architecture

```
Plugin_Matter/
├── src/
│   ├── lib.rs
│   ├── plugin.rs
│   ├── panel.rs                    Main docking panel, event wiring
│   ├── state/
│   │   ├── document.rs             PIF asset manager wrapper, layer CRUD
│   │   ├── history.rs              Undo/redo stack, Command trait
│   │   ├── commands.rs             PaintStroke, CreateLayer, DeleteLayer,
│   │   │                           ToggleVisibility, SetOpacity, ReorderLayers
│   │   ├── tool_state.rs           Active tool, brush size, brush opacity, colours
│   │   └── viewport.rs             Viewport pan/zoom state
│   ├── canvas/
│   │   ├── viewport.rs             CanvasViewport entity: input, stroke rasterisation
│   │   ├── stroke.rs               CPU-side brush stamp renderer
│   │   └── renderer/
│   │       ├── canvas_renderer.rs  WGPU grid pass and tile composite pass
│   │       ├── types.rs            CanvasRenderInput
│   │       └── shaders/
│   │           ├── grid.wgsl       Dot-grid background, canvas shadow, border
│   │           └── tile.wgsl       Canvas-space quad, per-layer texture sample
│   ├── tools/
│   │   ├── paint.rs                Paint tool
│   │   ├── eraser.rs               Eraser tool
│   │   ├── fill.rs                 Fill bucket tool
│   │   ├── eyedropper.rs           Colour sampler tool
│   │   └── hand.rs                 Pan tool
│   ├── panels/
│   │   ├── layers.rs               Layer list with visibility, rename, reorder
│   │   └── properties.rs           Brush settings, colour pickers
│   └── ui/
│       ├── toolbar.rs              Tool buttons, undo/redo, zoom label
│       ├── layer_panel.rs          Layer row rendering
│       └── color_panel.rs          Foreground and background colour pickers
└── examples/
    └── standalone.rs               Runs the editor without the engine
```

---

## Painting Model

The canvas is divided into 256 x 256 pixel tiles. Each tile is stored as a raw RGBA8 buffer inside PIF. When a brush stroke begins, the viewport takes a snapshot of every tile the stroke will touch. As the mouse moves, brush stamps are rasterised directly into the live tile buffers at the sub-pixel level. On mouse release, the modified tiles are committed to PIF via `commit_changes` and a `PaintStrokeCommand` carrying the before and after tile data is pushed onto the undo stack.

The WGPU renderer composites all visible layers each frame. It reads tile data from PIF and uploads them as a single composite texture per layer, then draws a full-canvas textured quad for each visible layer with alpha blending. Live tile data from an in-progress stroke overrides the committed PIF data for that frame, giving immediate visual feedback without write overhead during the stroke.

---

## Running Standalone

```bash
cd Plugin_Matter
cargo run --example standalone
```

---

## Controls

| Action | Input |
|---|---|
| Paint | Left-click drag |
| Erase | Select eraser, then left-click drag |
| Pan | Right-click drag, or middle-click drag, or Hand tool + left-click drag |
| Zoom | Scroll wheel |
| Pick colour | Eyedropper tool, left-click |
| Undo | Undo button in toolbar |
| Redo | Redo button in toolbar |

---

## Progress

### Foundation

- [x] Three-panel UI layout: layers, canvas, properties
- [x] WGPU-backed canvas with dot-grid and canvas shadow shaders
- [x] WgpuSurface with HiDPI-correct logical-pixel coordinate handling
- [x] Pan with right-click drag and middle-click drag
- [x] Zoom to cursor with scroll wheel
- [x] Brush cursor ring overlay (GPUI canvas overlay)

### Document and Layer Model

- [x] PIF (Pulsar Image Format) integration via asset manager
- [x] Document open and create-new flows
- [x] Tile-based raster layer storage (256 x 256 tiles)
- [x] Default background layer on new document
- [x] Add layer command (undoable)
- [x] Delete layer command (undoable)
- [x] Toggle layer visibility command (undoable)
- [x] Set layer opacity command (undoable)
- [x] Reorder layers command (undoable)
- [ ] Layer blend modes beyond Normal
- [ ] Layer groups and clipping masks
- [ ] Merge layers command

### Painting

- [x] Brush stamp rasterisation on CPU (circular soft brush)
- [x] Stroke interpolation between mouse samples
- [x] Live tile buffer overrides during active stroke
- [x] Commit to PIF on stroke end
- [x] Eraser mode (alpha-zero stamps)
- [x] Foreground and background colour selection
- [x] Brush size control
- [x] Brush opacity control
- [ ] Pressure-sensitive dynamics (tablet input)
- [ ] Multiple brush tip shapes
- [ ] Brush hardness and flow controls
- [ ] Smear and blur brushes

### Tools

- [x] Paint brush
- [x] Eraser
- [x] Hand (pan)
- [x] Eyedropper (colour sampling stub)
- [x] Fill bucket (stub)
- [ ] Selection tools (rectangular, lasso, magic wand)
- [ ] Transform tool (move, scale, rotate layer)
- [ ] Gradient tool

### History

- [x] Command pattern with execute/undo/redo
- [x] Paint stroke command (full before/after tile snapshots)
- [x] Layer management commands
- [x] History size limit (100 steps)
- [ ] Stroke batching to reduce memory use on large canvases
- [ ] Persistent history across sessions

### Properties Panel

- [x] Foreground colour picker
- [x] Background colour picker
- [x] Brush size display
- [x] Brush opacity display
- [ ] Interactive brush size slider
- [ ] Interactive opacity slider
- [ ] Colour swatch history

### Procedural and Material Pipeline

- [ ] Node-based material graph editor
- [ ] Noise generators (Perlin, Simplex, Voronoi, Cellular)
- [ ] Pattern and tile generators
- [ ] Blending and filter nodes
- [ ] PBR texture set authoring (Albedo, Normal, Roughness, Metallic, AO)
- [ ] Channel packing and extraction
- [ ] 3D material preview sphere
- [ ] Material export to engine-native formats

### Polish

- [ ] Symmetry painting (horizontal, vertical, radial)
- [ ] Custom brush import
- [ ] Filter layers (non-destructive adjustments)
- [ ] Auto-save and crash recovery
- [ ] Canvas resize and crop

---

## Dependencies

- **GPUI** — UI framework and rendering host (Far-Beyond-Pulsar fork)
- **PIF-rs** — Pulsar Image Format: tile-based, Git-friendly raster storage
- **wgpu** — GPU pipeline for grid background and layer compositing
- **parking_lot** — RwLock for shared document state across panel and canvas
- **uuid** — Layer identifiers
- **serde / serde_json** — Document serialization
- **tracing** — Diagnostic logging

---

## License

See the main Pulsar Engine license.
