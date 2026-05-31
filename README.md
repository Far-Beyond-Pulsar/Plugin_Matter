# Plugin Matter

A professional texture painting and material authoring plugin for Pulsar Engine. Plugin Matter provides a tile-based raster painting system built on WGPU, with full undo/redo history, a multi-layer compositor, a multi-channel brush engine, and deep integration with the PIF (Pulsar Image Format) asset pipeline.

---

## Overview

Plugin Matter embeds a self-contained texture editor into the Pulsar Engine panel system. The editor renders painting layers through a custom WGSL tile shader, composites them per-frame directly on the GPU, and records every stroke as a command-pattern operation so the full session history is undoable. The canvas uses a WgpuSurface that operates independently of the main GPUI render pass, allowing smooth HiDPI rendering at full paint resolution regardless of window scale factor.

The editor is laid out as three panels: a layer list on the left, the interactive canvas in the centre, and a properties and colour inspector on the right. A toolbar at the top carries the tool palette and undo/redo controls.

---

## Architecture

```
Plugin_Matter/
├── brushes/                            Built-in and user brush presets
│   ├── round_soft.pbrush/
│   │   ├── config.json                 Brush metadata (name, spacing, shape, …)
│   │   └── mask.svg                    64×64 RGBA brush mask (SVG or PNG)
│   ├── round_hard.pbrush/
│   ├── square_flat.pbrush/
│   ├── texture_rough.pbrush/
│   └── bristle_soft.pbrush/
├── src/
│   ├── lib.rs
│   ├── plugin.rs
│   ├── panel.rs                        Main docking panel, event wiring
│   ├── brush_engine/                   ← New brush system
│   │   ├── mod.rs                      Public API + channel-semantics docs
│   │   ├── config.rs                   BrushConfig / BrushShape (serde)
│   │   ├── mask.rs                     BrushMask: RGBA8 buffer, sample(), thumbnail()
│   │   ├── loader.rs                   .pbrush loader (SVG via resvg, PNG via image)
│   │   ├── builtin.rs                  Embedded SVGs compiled in with include_bytes!
│   │   ├── registry.rs                 BrushRegistry + BrushDropdownItem (UI adapter)
│   │   └── stamp.rs                    Mask-aware stamp rasteriser (replaces hard circle)
│   ├── state/
│   │   ├── document.rs                 PIF asset manager wrapper, layer CRUD
│   │   ├── history.rs                  Undo/redo stack, Command trait
│   │   ├── commands.rs                 PaintStroke, CreateLayer, DeleteLayer, …
│   │   ├── tool_state.rs               Active tool, active_brush_id, size, opacity, colours
│   │   └── viewport.rs                 Viewport pan/zoom state
│   ├── canvas/
│   │   ├── viewport.rs                 CanvasViewport: input, stroke rasterisation
│   │   ├── stroke.rs                   Legacy stroke types (kept for reference)
│   │   └── renderer/
│   │       ├── canvas_renderer.rs      WGPU grid pass and tile composite pass
│   │       ├── types.rs                CanvasRenderInput
│   │       └── shaders/
│   │           ├── grid.wgsl           Dot-grid background, canvas shadow, border
│   │           └── tile.wgsl           Canvas-space quad, per-layer texture sample
│   ├── tools/
│   │   ├── paint.rs                    Paint tool
│   │   ├── eraser.rs                   Eraser tool
│   │   ├── fill.rs                     Fill bucket tool
│   │   ├── eyedropper.rs               Colour sampler tool
│   │   └── hand.rs                     Pan tool
│   ├── panels/
│   │   ├── layers.rs                   Layer list with visibility, rename, reorder
│   │   └── properties.rs               Brush picker dropdown, colour pickers, settings
│   └── ui/
│       ├── toolbar.rs                  Tool buttons, undo/redo, zoom label
│       ├── layer_panel.rs              Layer row rendering
│       └── color_panel.rs              Foreground and background colour pickers
└── examples/
    └── standalone.rs                   Runs the editor without the engine
```

---

## Brush System

### `.pbrush` Bundle Format

Every brush is a directory whose name ends in `.pbrush`. It contains exactly two files:

```
my_brush.pbrush/
    config.json     ← metadata and default parameters
    mask.svg        ← brush shape mask (SVG or PNG)
```

**`config.json` fields:**

| Field | Type | Description |
|---|---|---|
| `name` | string | Display name shown in the picker |
| `mask_file` | string | Filename of the mask image (`mask.svg` or `mask.png`) |
| `shape` | `"circle"` \| `"square"` \| `"bristle"` \| `"custom"` | Shape hint for the picker thumbnail |
| `description` | string | Tooltip / subtitle |
| `spacing` | float | Stamp spacing as fraction of brush size (0.1 = dense, 0.5 = airy) |
| `hardness` | float | Informational edge-hardness hint (0.0–1.0) |
| `default_size` | float | Suggested brush size in canvas pixels |
| `default_opacity` | float | Suggested opacity (0.0–1.0) |

### RGBA Channel Semantics

Each pixel of the rasterised mask encodes four independent material properties:

| Channel | Property | Description |
|---|---|---|
| **R** | **Intensity** | Coverage strength: 0 = no paint, 255 = full paint |
| **G** | **Reflectiveness** | Metallic / specular contribution for PBR material layers |
| **B** | **Smear amount** | Lateral colour diffusion (smear brush mode, future) |
| **A** | **Roughness** | Surface roughness for PBR; also the mask boundary flag (0 = outside brush) |

**For SVG brush masks** the natural approach is a greyscale fill (R = G = B), which automatically encodes intensity in the R channel. The shape boundary is inferred from the SVG's alpha. Pixels with A = 0 are skipped during stamping.

**For PNG brush masks** all four channels can be set independently, enabling full PBR material-layer brushes — e.g., a brush that paints high metallic values in bright regions while leaving roughness untouched in dark regions.

### Built-in Brushes

Five brushes are compiled into the binary via `include_bytes!` and are always available:

| ID | Name | Shape | Character |
|---|---|---|---|
| `round_soft` | Round Soft | Circle | Smooth Gaussian-like airbrush falloff |
| `round_hard` | Round Hard | Circle | Uniform disc with a thin antialiased edge |
| `square_flat` | Square Flat | Square | Flat full-coverage square stamp |
| `texture_rough` | Texture Rough | Circle | Gritty overlapping-circle texture |
| `bristle_soft` | Bristle Soft | Bristle | Six parallel strands with tapered tips |

### Adding Custom Brushes

1. Create a directory named `<your_brush>.pbrush` inside `brushes/`.
2. Add a `config.json` with at least `name` and `mask_file`.
3. Create a `mask.svg` (or `mask.png`) at 64 × 64 pixels.
   - For a simple paint brush: draw a white or greyscale shape — brightness becomes intensity.
   - For a PBR material brush: use a PNG where R/G/B/A each encode a specific channel.
4. Restart the editor. The brush appears in the picker automatically.

### Brush Picker UI

The Properties panel contains a **Dropdown** (from the WGPUI component library) that lists all discovered brushes. Each item displays a small shape thumbnail (circle, square, or bristle strands) alongside the brush name. Selecting a brush immediately switches the active stamp; the selection is stored in `ToolState::active_brush_id` and the brush mask is locked in at the start of each stroke.

---

## Painting Model

The canvas is divided into 256 × 256 pixel tiles stored as raw RGBA8 buffers inside PIF. When a brush stroke begins, the viewport:

1. Looks up the selected brush in `BrushRegistry` and locks its `BrushMask` for the duration of the stroke.
2. Snapshots every tile the stroke will touch.
3. On each mouse sample, stamps the mask at canvas-space UV coordinates, reading the R channel as coverage and applying Porter-Duff src-over compositing into the live tile buffers.
4. On mouse release, commits modified tiles to PIF and pushes a `PaintStrokeCommand` (before/after tile snapshots) onto the undo stack.

The WGPU renderer composites all visible layers each frame. Live tile data from an in-progress stroke overrides committed PIF data for that frame, giving immediate visual feedback without write overhead during the stroke.

---

## Running Standalone

```bash
cd Plugin_Matter
cargo run --example standalone
```

The standalone runner loads brushes from `./brushes/` (the project directory). Built-in brushes are always available even if that directory is absent.

---

## Controls

| Action | Input |
|---|---|
| Paint | Left-click drag |
| Erase | Select Eraser tool, then left-click drag |
| Pan | Right-click drag, middle-click drag, or Hand tool + left-click drag |
| Zoom | Scroll wheel |
| Pick colour | Eyedropper tool, left-click |
| Change brush | Brush dropdown in Properties panel |
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
- [x] Tile-based raster layer storage (256 × 256 tiles)
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

- [x] Mask-aware brush stamp rasterisation (RGBA multi-channel)
- [x] `.pbrush` bundle format (SVG + PNG masks, JSON config)
- [x] Five built-in brushes embedded at compile time
- [x] Brush picker dropdown with shape thumbnails
- [x] SVG brush mask loading and rasterisation (via `resvg`)
- [x] PNG brush mask loading (via `image`)
- [x] Stroke interpolation between mouse samples
- [x] Live tile buffer overrides during active stroke
- [x] Commit to PIF on stroke end
- [x] Eraser mode (alpha-zero stamps)
- [x] Foreground and background colour selection
- [x] Brush size control
- [x] Brush opacity control
- [ ] Custom brush import via UI (drag-and-drop `.pbrush`)
- [ ] Pressure-sensitive dynamics (tablet input)
- [ ] Brush hardness and flow controls
- [ ] Smear and blur brushes (B-channel driven)
- [ ] PBR material-layer painting (G/B/A channels in properties panel)

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
- [x] Brush picker dropdown with shape thumbnails
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
- [ ] Filter layers (non-destructive adjustments)
- [ ] Auto-save and crash recovery
- [ ] Canvas resize and crop

---

## Dependencies

- **GPUI** — UI framework and rendering host (Far-Beyond-Pulsar fork)
- **WGPUI Component** — UI component library (buttons, dropdowns, colour pickers)
- **PIF-rs** — Pulsar Image Format: tile-based, Git-friendly raster storage
- **wgpu** — GPU pipeline for grid background and layer compositing
- **resvg** — Pure-Rust SVG rasteriser used to load SVG brush masks
- **image** — PNG decoding for PNG-format brush masks
- **parking_lot** — RwLock for shared document state across panel and canvas
- **uuid** — Layer identifiers
- **serde / serde_json** — Document and brush config serialisation
- **tracing** — Diagnostic logging

---

## License

See the main Pulsar Engine license.
