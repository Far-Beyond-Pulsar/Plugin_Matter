//! Hand (pan) tool implementation

use anyhow::Result;
use gpui::*;
use parking_lot::Mutex;
use std::sync::Arc;
use pulsar_image_format::PifAssetManager;
use ui::IconName;

use crate::tools::Tool;

pub struct HandTool {
    last_pos: Option<Point<f32>>,
}

impl HandTool {
    pub fn new() -> Self {
        Self { last_pos: None }
    }
}

impl Tool for HandTool {
    fn name(&self) -> &str {
        "Hand"
    }
    
    fn icon(&self) -> IconName {
        IconName::DragHandGesture
    }
    
    fn hotkey(&self) -> &str {
        "H"
    }
    
    fn cursor(&self) -> CursorStyle {
        CursorStyle::PointingHand
    }
    
    fn on_mouse_down(
        &mut self,
        pos: Point<Pixels>,
        _pif: Arc<Mutex<PifAssetManager>>,
        _brush_size: f32,
        _brush_opacity: f32,
        _color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        let x: f32 = pos.x.into();
        let y: f32 = pos.y.into();
        self.last_pos = Some(Point { x, y });
        Ok(None)
    }
    
    fn on_mouse_move(
        &mut self,
        pos: Point<Pixels>,
        _brush_size: f32,
        _brush_opacity: f32,
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        if let Some(last) = self.last_pos {
            let x: f32 = pos.x.into();
            let y: f32 = pos.y.into();
            let curr = Point { x, y };
            let _delta = Point { x: curr.x - last.x, y: curr.y - last.y };
            
            // TODO: Update viewport pan
            
            self.last_pos = Some(curr);
        }
        Ok(None)
    }
    
    fn on_mouse_up(
        &mut self,
        _pif: Arc<Mutex<PifAssetManager>>,
        _layer_id: String,
        _color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        self.last_pos = None;
        Ok(None)
    }
}
