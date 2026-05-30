# Plugin_Matter

A professional texture painting and procedural material generation plugin for Pulsar Engine.

## Features

### Phase 1 - Foundation (MVP) ✅ COMPLETE
- ✅ Full UI layout with 3-panel design (toolbar, layers, canvas, properties)
- ✅ Pan/zoom viewport with mouse controls
- ✅ Checkerboard canvas background
- ✅ Layer management UI
- ✅ Tool selection toolbar
- ✅ Color picker panel
- ✅ PIF file format integration
- ✅ Document state management

### Phase 2 - Advanced Painting (In Progress)
- Interactive brush painting
- Multiple brush shapes and dynamics
- Eraser, fill bucket, eyedropper tools
- Undo/redo system
- Layer blend modes
- Transform tools

### Phase 3 - Procedural Generation (Planned)
- Node-based material editor
- Noise generators (Perlin, Simplex, Voronoi)
- Pattern generators
- Filter nodes
- Node graph execution engine

### Phase 4 - Material Authoring (Planned)
- PBR texture sets (Albedo, Normal, Roughness, Metallic, AO)
- 3D material preview
- Channel packing tools
- Material export

### Phase 5 - Polish & Advanced Features (Planned)
- Symmetry painting
- Custom brushes
- Filter layers
- Auto-save
- Performance optimization

## Architecture

```
Plugin_Matter/
├── src/
│   ├── lib.rs              # Main module
│   ├── plugin.rs           # Plugin registration
│   ├── panel.rs            # Main editor panel (complete)
│   ├── state/              # State management
│   │   ├── document.rs     # PIF wrapper (complete)
│   │   ├── viewport.rs     # Pan/zoom state (complete)
│   │   ├── tool_state.rs   # Tool settings (complete)
│   │   └── history.rs      # Undo/redo
│   ├── canvas/             # Canvas rendering
│   │   ├── viewport.rs     # Checkerboard + rendering (complete)
│   │   └── tools/          # Painting tools
│   └── ui/                 # UI components
│       ├── toolbar.rs      # Tool selection (complete)
│       ├── layer_panel.rs  # Layer list (complete)
│       └── color_panel.rs  # Color picker (complete)
└── examples/
    └── standalone.rs       # Test application
```

## Building

```bash
# Build the plugin
cargo build

# Run the standalone test
cargo run --example standalone
```

## Dependencies

- **GPUI** - UI framework
- **PIF-rs** - Pulsar Image Format for Git-friendly texture storage
- **wgpu** - GPU-accelerated rendering
- **uuid** - Unique IDs for layers
- **parking_lot** - Fast synchronization
- **serde** - Serialization

## Current Status

**Phase 1 MVP**: ✅ COMPLETE and functional!

The plugin now has:
- Full 3-panel professional UI layout
- Working pan/zoom viewport
- Layer list display
- Tool selection toolbar
- Color picker UI
- PIF document integration with default layer
- All code compiling successfully

## Next Steps

1. Implement basic brush painting
2. Add PIF layer rendering to canvas
3. Implement stroke recording and commit
4. Add interactive color picker
5. Create layer creation/deletion
6. Implement undo/redo

## License

See main Pulsar Engine license.
