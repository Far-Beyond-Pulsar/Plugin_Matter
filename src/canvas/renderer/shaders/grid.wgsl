// Grid background shader — renders the orientation grid + canvas area.
// Draws dot-based minor/major grids outside and inside the canvas, with a
// canvas drop-shadow and border for visual anchoring.

struct GridUniforms {
    pan_offset:    vec2<f32>,   // Screen-space pan (pixels)
    zoom:          f32,         // Scale factor
    _pad0:         f32,
    viewport_size: vec2<f32>,   // Viewport width/height in pixels
    canvas_size:   vec2<f32>,   // Canvas width/height in pixels
}

@group(0) @binding(0) var<uniform> u: GridUniforms;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
}

// Full-screen quad — two triangles covering NDC [-1,1]
var<private> VERTS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0), vec2<f32>(-1.0,  1.0),
    vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0), vec2<f32>(-1.0,  1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VOut {
    let p = VERTS[vi];
    var o: VOut;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.uv  = p * 0.5 + vec2<f32>(0.5);
    return o;
}

// ── helpers ─────────────────────────────────────────────────────────────────

// Distance to nearest grid intersection point (both axes must be near 0).
fn dot_dist(screen_pos: vec2<f32>, step: f32, origin: vec2<f32>) -> f32 {
    let rel = screen_pos - origin;
    let mx  = rel.x % step;
    let my  = rel.y % step;
    let dx  = min(abs(mx), abs(step - abs(mx)));
    let dy  = min(abs(my), abs(step - abs(my)));
    return sqrt(dx * dx + dy * dy);
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    // uv.y=0 is at the NDC bottom (screen bottom), but pan/mouse coords use
    // y=0 at the TOP — flip Y so sp matches the same space as pan_offset.
    let sp = vec2<f32>(
        in.uv.x * u.viewport_size.x,
        (1.0 - in.uv.y) * u.viewport_size.y,
    );

    // ── Canvas bounds in screen space ─────────────────────────────────────
    let cx0 = u.pan_offset.x;
    let cy0 = u.pan_offset.y;
    let cx1 = cx0 + u.canvas_size.x * u.zoom;
    let cy1 = cy0 + u.canvas_size.y * u.zoom;

    let in_canvas = sp.x >= cx0 && sp.x < cx1 && sp.y >= cy0 && sp.y < cy1;

    // ── Grid step sizes (screen pixels) ───────────────────────────────────
    let minor_step = clamp(10.0 * u.zoom, 4.0, 120.0);
    let major_step = clamp(100.0 * u.zoom, 8.0, 800.0);
    let origin     = u.pan_offset;   // grid anchors to canvas origin

    // ── Base background ───────────────────────────────────────────────────
    var color: vec4<f32>;
    if in_canvas {
        color = vec4<f32>(0.975, 0.975, 0.980, 1.0);  // near-white canvas
    } else {
        color = vec4<f32>(0.118, 0.118, 0.137, 1.0);  // dark viewport bg
    }

    // ── Dot grid (viewport bg) ─────────────────────────────────────────────
    if !in_canvas {
        if minor_step >= 6.0 {
            let d = dot_dist(sp, minor_step, origin);
            if d < 0.9 {
                let a = 1.0 - d / 0.9;
                color = mix(color, vec4<f32>(0.24, 0.24, 0.30, 1.0), a * 0.8);
            }
        }
        if major_step >= 8.0 {
            let d = dot_dist(sp, major_step, origin);
            if d < 1.4 {
                let a = 1.0 - d / 1.4;
                color = mix(color, vec4<f32>(0.36, 0.36, 0.46, 1.0), a);
            }
        }
    }

    // ── Drop shadow behind canvas ──────────────────────────────────────────
    let sh_off = 5.0;
    let sh_blur = 10.0;
    if !in_canvas {
        let sx = sp.x - sh_off;
        let sy = sp.y - sh_off;
        if sx >= cx0 && sx < cx1 && sy >= cy0 && sy < cy1 {
            let edge_dist = min(
                min(sx - cx0, cx1 - sx),
                min(sy - cy0, cy1 - sy),
            );
            let shadow_a = clamp(edge_dist / sh_blur, 0.0, 1.0) * 0.35;
            color = mix(color, vec4<f32>(0.0, 0.0, 0.0, 1.0), shadow_a);
        }
    }

    // ── Canvas grid overlay ────────────────────────────────────────────────
    if in_canvas {
        // Minor dots on canvas
        if minor_step >= 8.0 {
            let d = dot_dist(sp, minor_step, origin);
            if d < 0.75 {
                let a = (1.0 - d / 0.75) * 0.4;
                color = mix(color, vec4<f32>(0.80, 0.80, 0.85, 1.0), a);
            }
        }
        // Major dots on canvas (more prominent)
        if major_step >= 8.0 {
            let d = dot_dist(sp, major_step, origin);
            if d < 1.2 {
                let a = (1.0 - d / 1.2) * 0.65;
                color = mix(color, vec4<f32>(0.70, 0.70, 0.78, 1.0), a);
            }
        }
    }

    // ── Canvas border (1px outline) ────────────────────────────────────────
    let bw = 1.0;
    let border_color = vec4<f32>(0.45, 0.45, 0.58, 1.0);
    let on_h = (abs(sp.y - cy0) < bw || abs(sp.y - cy1) < bw)
             && sp.x >= cx0 - bw && sp.x <= cx1 + bw;
    let on_v = (abs(sp.x - cx0) < bw || abs(sp.x - cx1) < bw)
             && sp.y >= cy0 - bw && sp.y <= cy1 + bw;
    if on_h || on_v {
        color = border_color;
    }

    return color;
}
