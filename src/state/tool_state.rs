//! Tool state management

use gpui::Rgba;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Paint,
    Erase,
    Eyedropper,
    Fill,
    Pan,
}

/// State for the active tool
#[derive(Clone, Debug)]
pub struct ToolState {
    pub active_tool: ActiveTool,
    pub brush_size: f32,
    pub brush_opacity: f32,
    pub foreground_color: Rgba,
    pub background_color: Rgba,
}

impl ToolState {
    /// Convert foreground color to RGBA8888 bytes
    pub fn foreground_bytes(&self) -> [u8; 4] {
        [
            (self.foreground_color.r * 255.0) as u8,
            (self.foreground_color.g * 255.0) as u8,
            (self.foreground_color.b * 255.0) as u8,
            (self.foreground_color.a * 255.0) as u8,
        ]
    }
    
    /// Convert background color to RGBA8888 bytes
    pub fn background_bytes(&self) -> [u8; 4] {
        [
            (self.background_color.r * 255.0) as u8,
            (self.background_color.g * 255.0) as u8,
            (self.background_color.b * 255.0) as u8,
            (self.background_color.a * 255.0) as u8,
        ]
    }
}

impl Default for ToolState {
    fn default() -> Self {
        Self {
            active_tool: ActiveTool::Paint,
            brush_size: 20.0,
            brush_opacity: 1.0,
            foreground_color: Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            background_color: Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
        }
    }
}
