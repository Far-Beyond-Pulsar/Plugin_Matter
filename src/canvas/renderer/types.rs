//! Types shared between the canvas renderer and GPUI viewport.

/// Per-frame input fed to the canvas renderer.
#[derive(Clone, Debug, Default)]
pub struct CanvasRenderInput {
    /// Pan offset: screen-space origin of canvas top-left corner (pixels).
    pub pan_offset: [f32; 2],
    /// Zoom multiplier (1.0 = 100%).
    pub zoom: f32,
    /// Canvas document dimensions in pixels.
    pub canvas_size: [f32; 2],
    /// Cursor position in screen pixels (None if outside viewport).
    pub cursor_screen_pos: Option<[f32; 2]>,
    /// Brush radius in screen pixels.
    pub brush_radius: f32,
    /// Cursor color (RGBA, each component 0..1).
    pub brush_color: [f32; 4],
}
