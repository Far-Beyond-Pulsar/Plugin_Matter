//! Tool trait and tool system

pub mod paint;
pub mod eraser;
pub mod fill;
pub mod eyedropper;
pub mod hand;

use gpui::*;
use ui::IconName;

pub use paint::PaintTool;
pub use eraser::EraserTool;
pub use fill::FillTool;
pub use eyedropper::EyedropperTool;
pub use hand::HandTool;

use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;
use pulsar_image_format::PifAssetManager;

/// Trait for all painting/editing tools
pub trait Tool: Send + Sync {
    /// Tool name for display
    fn name(&self) -> &str;
    
    /// Tool icon
    fn icon(&self) -> IconName;
    
    /// Keyboard shortcut (single letter)
    fn hotkey(&self) -> &str;
    
    /// Tool cursor
    fn cursor(&self) -> CursorStyle;
    
    /// Handle mouse down event
    fn on_mouse_down(
        &mut self,
        pos: Point<Pixels>,
        pif: Arc<Mutex<PifAssetManager>>,
        brush_size: f32,
        brush_opacity: f32,
        color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>>;
    
    /// Handle mouse move event
    fn on_mouse_move(
        &mut self,
        pos: Point<Pixels>,
        brush_size: f32,
        brush_opacity: f32,
    ) -> Result<Option<Box<dyn crate::state::history::Command>>>;
    
    /// Handle mouse up event
    fn on_mouse_up(
        &mut self,
        pif: Arc<Mutex<PifAssetManager>>,
        layer_id: String,
        color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>>;
}

/// Registry for managing tools
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }
    
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
    
    pub fn get(&self, index: usize) -> Option<&dyn Tool> {
        self.tools.get(index).map(|b| &**b)
    }
    
    pub fn find_by_hotkey(&self, key: &str) -> Option<usize> {
        self.tools.iter().position(|t| t.hotkey() == key)
    }
    
    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.tools.iter().position(|t| t.name() == name)
    }
}
