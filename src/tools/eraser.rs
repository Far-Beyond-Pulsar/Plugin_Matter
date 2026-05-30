//! Eraser tool implementation

use anyhow::Result;
use gpui::*;
use parking_lot::Mutex;
use std::sync::Arc;
use pulsar_image_format::PifAssetManager;
use ui::IconName;

use crate::canvas::stroke::Stroke;
use crate::state::commands::PaintStrokeCommand;
use crate::tools::Tool;

pub struct EraserTool {
    stroke: Option<Stroke>,
}

impl EraserTool {
    pub fn new() -> Self {
        Self { stroke: None }
    }
}

impl Tool for EraserTool {
    fn name(&self) -> &str {
        "Eraser"
    }
    
    fn icon(&self) -> IconName {
        IconName::Erase
    }
    
    fn hotkey(&self) -> &str {
        "E"
    }
    
    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }
    
    fn on_mouse_down(
        &mut self,
        pos: Point<Pixels>,
        _pif: Arc<Mutex<PifAssetManager>>,
        brush_size: f32,
        brush_opacity: f32,
        _color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        let x: f32 = pos.x.into();
        let y: f32 = pos.y.into();
        let point = Point { x, y };
        let transparent = [0, 0, 0, 0];
        let mut stroke = Stroke::new(transparent);
        stroke.add_point(point, brush_size, brush_opacity);
        self.stroke = Some(stroke);
        Ok(None)
    }
    
    fn on_mouse_move(
        &mut self,
        pos: Point<Pixels>,
        brush_size: f32,
        brush_opacity: f32,
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        if let Some(stroke) = &mut self.stroke {
            let x: f32 = pos.x.into();
            let y: f32 = pos.y.into();
            let point = Point { x, y };
            stroke.add_point(point, brush_size, brush_opacity);
        }
        Ok(None)
    }
    
    fn on_mouse_up(
        &mut self,
        pif: Arc<Mutex<PifAssetManager>>,
        layer_id: String,
        _color: [u8; 4],
    ) -> Result<Option<Box<dyn crate::state::history::Command>>> {
        let transparent = [0, 0, 0, 0];
        if let Some(stroke) = self.stroke.take() {
            Ok(Some(Box::new(PaintStrokeCommand::new(pif, layer_id, stroke, transparent))))
        } else {
            Ok(None)
        }
    }
}
