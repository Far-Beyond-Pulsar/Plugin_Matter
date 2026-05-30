//! Viewport state (pan/zoom/camera)

use gpui::{Point, Pixels, px};

/// Viewport camera state
#[derive(Clone, Debug)]
pub struct ViewportState {
    /// Camera position (canvas coordinates)
    pub offset: Point<Pixels>,
    
    /// Zoom level (1.0 = 100%, 2.0 = 200%, 0.5 = 50%)
    pub zoom: f32,
    
    /// Whether user is currently panning
    pub is_panning: bool,
    
    /// Last mouse position for panning
    pub last_pan_pos: Option<Point<Pixels>>,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Point { x: px(0.0), y: px(0.0) },
            zoom: 1.0,
            is_panning: false,
            last_pan_pos: None,
        }
    }
}

impl ViewportState {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Pan by a delta
    pub fn pan(&mut self, delta: Point<Pixels>) {
        self.offset.x = self.offset.x + delta.x;
        self.offset.y = self.offset.y + delta.y;
    }
    
    /// Convert screen coordinates to canvas coordinates
    pub fn screen_to_canvas(&self, screen_pos: Point<Pixels>) -> Point<Pixels> {
        let sx: f32 = screen_pos.x.into();
        let sy: f32 = screen_pos.y.into();
        let ox: f32 = self.offset.x.into();
        let oy: f32 = self.offset.y.into();
        
        Point {
            x: px((sx - ox) / self.zoom),
            y: px((sy - oy) / self.zoom),
        }
    }
    
    /// Convert canvas coordinates to screen coordinates
    pub fn canvas_to_screen(&self, canvas_pos: Point<Pixels>) -> Point<Pixels> {
        let cx: f32 = canvas_pos.x.into();
        let cy: f32 = canvas_pos.y.into();
        let ox: f32 = self.offset.x.into();
        let oy: f32 = self.offset.y.into();
        
        Point {
            x: px(cx * self.zoom + ox),
            y: px(cy * self.zoom + oy),
        }
    }
    
    /// Start panning
    pub fn start_pan(&mut self, pos: Point<Pixels>) {
        self.is_panning = true;
        self.last_pan_pos = Some(pos);
    }
    
    /// Update pan
    pub fn update_pan(&mut self, pos: Point<Pixels>) {
        if let Some(last_pos) = self.last_pan_pos {
            self.offset.x = self.offset.x + (pos.x - last_pos.x);
            self.offset.y = self.offset.y + (pos.y - last_pos.y);
            self.last_pan_pos = Some(pos);
        }
    }
    
    /// Stop panning
    pub fn stop_pan(&mut self) {
        self.is_panning = false;
        self.last_pan_pos = None;
    }
    
    /// Zoom in/out around a point
    pub fn zoom_at(&mut self, screen_pos: Point<Pixels>, factor: f32) {
        let old_canvas_pos = self.screen_to_canvas(screen_pos);
        
        // Update zoom (clamp to reasonable range)
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
        
        // Adjust offset to keep point under cursor
        let new_canvas_pos = self.screen_to_canvas(screen_pos);
        let delta_x: f32 = new_canvas_pos.x.into();
        let old_x: f32 = old_canvas_pos.x.into();
        let delta_y: f32 = new_canvas_pos.y.into();
        let old_y: f32 = old_canvas_pos.y.into();
        
        self.offset.x = self.offset.x + px((delta_x - old_x) * self.zoom);
        self.offset.y = self.offset.y + px((delta_y - old_y) * self.zoom);
    }
}
