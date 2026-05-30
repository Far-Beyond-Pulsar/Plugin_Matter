//! Eyedropper tool implementation

use anyhow::Result;
use gpui::*;
use parking_lot::Mutex;
use std::sync::Arc;
use pulsar_image_format::PifAssetManager;
use ui::IconName;

use crate::tools::Tool;

pub struct EyedropperTool;

impl EyedropperTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for EyedropperTool {
    fn name(&self) -> &str {
        "Eyedropper"
    }
    
    fn icon(&self) -> IconName {
        IconName::Eye
    }
    
    fn hotkey(&self) -> &str {
        "I"
    }
    
    fn cursor(&self) -> CursorStyle {
        CursorStyle::PointingHand
    }
    
    fn on_mouse_down(
        &mut self,
        _pos: Point<Pixels>,
        _pif: Arc<Mutex<PifAssetManager>>,
        _brush_size: f32,
        _brush_opacity: f32,
        _color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        // TODO: Sample color from canvas
        Ok(None)
    }
    
    fn on_mouse_move(
        &mut self,
        _pos: Point<Pixels>,
        _brush_size: f32,
        _brush_opacity: f32,
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        Ok(None)
    }
    
    fn on_mouse_up(
        &mut self,
        _pif: Arc<Mutex<PifAssetManager>>,
        _layer_id: String,
        _color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        Ok(None)
    }
}
