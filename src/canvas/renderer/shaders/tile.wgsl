// Tile/canvas texture shader — renders a textured quad in canvas space.
// Used for painting layer data and canvas composites.

struct TileUniforms {
    pan_offset:    vec2<f32>,   // Screen-space pan (pixels)
    zoom:          f32,
    _pad:          f32,
    viewport_size: vec2<f32>,
    _pad2:         vec2<f32>,
}

@group(0) @binding(0) var<uniform>   u:             TileUniforms;
@group(0) @binding(1) var           tile_tex:       texture_2d<f32>;
@group(0) @binding(2) var           tile_samp:      sampler;

struct VIn {
    @location(0) canvas_pos: vec2<f32>,   // Position in canvas space (pixels)
    @location(1) uv:         vec2<f32>,
}

struct VOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
}

@vertex
fn vs_main(v: VIn) -> VOut {
    // canvas → screen → NDC
    let screen = v.canvas_pos * u.zoom + u.pan_offset;
    let ndc_x  =  (screen.x / u.viewport_size.x) * 2.0 - 1.0;
    let ndc_y  = -((screen.y / u.viewport_size.y) * 2.0 - 1.0);

    var o: VOut;
    o.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    o.uv       = v.uv;
    return o;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    return textureSample(tile_tex, tile_samp, in.uv);
}
