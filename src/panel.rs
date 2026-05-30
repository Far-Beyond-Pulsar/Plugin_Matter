//! Main editor panel with painting and undo/redo support

use gpui::*;

use crate::canvas::{CanvasViewport, Stroke};
use crate::state::{Document, ActiveTool, CreateLayerCommand, PaintStrokeCommand};
use crate::ui::{render_toolbar, render_layer_panel, render_color_panel};

/// Main Matter editor panel
pub struct MatterEditorPanel {
    /// Document state
    document: Document,
    
    /// Current stroke being recorded (if painting)
    current_stroke: Option<Stroke>,
    
    /// Last mouse position for stroke smoothing
    last_mouse_pos: Option<Point<Pixels>>,
}

impl MatterEditorPanel {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            current_stroke: None,
            last_mouse_pos: None,
        }
    }
    
    /// Handle mouse down - start stroke
    fn on_mouse_down(&mut self, event: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let viewport = &mut self.document.viewport;
        let tool_state = &self.document.tool_state;
        
        match tool_state.active_tool {
            ActiveTool::Pan => {
                self.last_mouse_pos = Some(event.position);
            }
            ActiveTool::Paint => {
                // Start a new stroke
                let color = tool_state.foreground_bytes();
                let mut stroke = Stroke::new(color);
                
                // Convert screen to canvas coordinates
                let canvas_pos = viewport.screen_to_canvas(event.position);
                let cx_f32: f32 = canvas_pos.x.into();
                let cy_f32: f32 = canvas_pos.y.into();
                
                stroke.add_point(
                    Point { x: cx_f32, y: cy_f32 },
                    tool_state.brush_size,
                    tool_state.brush_opacity,
                );
                
                self.current_stroke = Some(stroke);
                self.last_mouse_pos = Some(event.position);
                cx.notify();
            }
            _ => {}
        }
    }
    
    /// Handle mouse move - continue stroke or pan
    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let viewport = &mut self.document.viewport;
        let tool_state = &self.document.tool_state;
        
        if let Some(last_pos) = self.last_mouse_pos {
            match tool_state.active_tool {
                ActiveTool::Pan if event.modifiers.secondary() => {
                    // Pan the viewport
                    let delta = event.position - last_pos;
                    viewport.pan(delta);
                    self.last_mouse_pos = Some(event.position);
                    cx.notify();
                }
                ActiveTool::Paint => {
                    // Continue stroke
                    if let Some(stroke) = &mut self.current_stroke {
                        let canvas_pos = viewport.screen_to_canvas(event.position);
                        let cx_f32: f32 = canvas_pos.x.into();
                        let cy_f32: f32 = canvas_pos.y.into();
                        
                        stroke.add_point(
                            Point { x: cx_f32, y: cy_f32 },
                            tool_state.brush_size,
                            tool_state.brush_opacity,
                        );
                        self.last_mouse_pos = Some(event.position);
                        cx.notify();
                    }
                }
                _ => {}
            }
        }
    }
    
    /// Handle mouse up - finalize stroke
    fn on_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.last_mouse_pos = None;
        
        // Finalize stroke if painting
        if let Some(stroke) = self.current_stroke.take() {
            if !stroke.is_empty() {
                self.commit_stroke(stroke, cx);
            }
        }
        
        cx.notify();
    }
    
    /// Commit a stroke to the document with undo support
    fn commit_stroke(&mut self, stroke: Stroke, cx: &mut Context<Self>) {
        let layer_id = match self.document.active_layer() {
            Some(id) => id.to_string(),
            None => return, // No active layer
        };
        
        // Rasterize stroke to tiles
        let doc_clone = &self.document;
        let tiles = stroke.rasterize(&layer_id, &|lid, tx, ty| {
            doc_clone.load_tile(lid, tx, ty).unwrap_or_else(|_| vec![0; 256 * 256 * 4])
        });
        
        // Create and execute paint command
        let pif_handle = self.document.pif_handle();
        let command = Box::new(PaintStrokeCommand::new(layer_id, tiles, pif_handle));
        
        if let Err(e) = self.document.history.execute(command) {
            eprintln!("Failed to execute paint stroke: {}", e);
        } else {
            self.document.mark_dirty();
            cx.notify();
        }
    }
    
    /// Handle scroll for zoom
    fn on_scroll(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let delta_y = match event.delta {
            ScrollDelta::Pixels(p) => p.y,
            ScrollDelta::Lines(l) => (l.y * 20.0).into(),
        };
        
        let delta_f32: f32 = delta_y.into();
        let zoom_factor = if delta_f32 > 0.0 { 0.9 } else { 1.1 };
        self.document.viewport.zoom_at(event.position, zoom_factor);
        cx.notify();
    }
    
    /// Handle keyboard shortcuts
    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        
        // Undo: Ctrl+Z (or Cmd+Z on Mac)
        if modifiers.platform && !modifiers.shift && event.keystroke.key == "z" {
            if let Err(e) = self.document.history.undo() {
                eprintln!("Undo failed: {}", e);
            } else {
                cx.notify();
            }
        }
        
        // Redo: Ctrl+Shift+Z or Ctrl+Y
        if (modifiers.platform && modifiers.shift && event.keystroke.key == "z") ||
           (modifiers.platform && event.keystroke.key == "y") {
            if let Err(e) = self.document.history.redo() {
                eprintln!("Redo failed: {}", e);
            } else {
                cx.notify();
            }
        }
        
        // Tool shortcuts
        if !modifiers.platform && !modifiers.control {
            match event.keystroke.key.as_str() {
                "b" => { self.document.tool_state.active_tool = ActiveTool::Paint; cx.notify(); }
                "e" => { self.document.tool_state.active_tool = ActiveTool::Erase; cx.notify(); }
                "g" => { self.document.tool_state.active_tool = ActiveTool::Fill; cx.notify(); }
                "i" => { self.document.tool_state.active_tool = ActiveTool::Eyedropper; cx.notify(); }
                "h" => { self.document.tool_state.active_tool = ActiveTool::Pan; cx.notify(); }
                _ => {}
            }
        }
    }
    
    /// Create a new layer (with undo support)
    fn create_new_layer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        use pulsar_image_format::model::Layer;
        use std::collections::HashMap;
        
        let layer_id = format!("layer_{}", uuid::Uuid::new_v4());
        let layer = Layer::Raster {
            id: layer_id.clone(),
            name: format!("Layer {}", self.document.layers().len() + 1),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        };
        
        let pif_handle = self.document.pif_handle();
        let position = self.document.layers().len();
        let command = Box::new(CreateLayerCommand::new(layer, position, pif_handle));
        
        if let Err(e) = self.document.history.execute(command) {
            eprintln!("Failed to create layer: {}", e);
        } else {
            self.document.set_active_layer(layer_id);
            self.document.mark_dirty();
            cx.notify();
        }
    }
}

impl Render for MatterEditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let (width, height) = self.document.dimensions();
        let active_tool = self.document.tool_state.active_tool;
        let brush_size = self.document.tool_state.brush_size;
        let brush_opacity = self.document.tool_state.brush_opacity;
        let layers = self.document.layers();
        let fg_color = self.document.tool_state.foreground_color;
        let bg_color = self.document.tool_state.background_color;
        let zoom = self.document.viewport.zoom;
        let can_undo = self.document.history.can_undo();
        let can_redo = self.document.history.can_redo();
        
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .on_key_down(cx.listener(Self::on_key_down))
            .child(
                // Toolbar
                div()
                    .flex()
                    .items_center()
                    .h_12()
                    .px_4()
                    .gap_4()
                    .border_b_1()
                    .border_color(rgb(0x3c3c3c))
                    .child(render_toolbar(active_tool, brush_size, brush_opacity, can_undo, can_redo))
            )
            .child(
                // Main content area
                div()
                    .flex()
                    .flex_1()
                    .child(
                        // Left panel - Layers
                        div()
                            .w_64()
                            .border_r_1()
                            .border_color(rgb(0x3c3c3c))
                            .child(render_layer_panel::<Self>(layers, Self::create_new_layer))
                    )
                    .child(
                        // Center - Canvas
                        div()
                            .flex_1()
                            .bg(rgb(0x2b2b2b))
                            .child(CanvasViewport::new().render(&self.document, &self.document.viewport))
                    )
                    .child(
                        // Right panel - Properties
                        div()
                            .w_64()
                            .border_l_1()
                            .border_color(rgb(0x3c3c3c))
                            .child(render_color_panel(fg_color, bg_color))
                    )
            )
            .child(
                // Status bar
                div()
                    .flex()
                    .items_center()
                    .h_8()
                    .px_4()
                    .gap_4()
                    .border_t_1()
                    .border_color(rgb(0x3c3c3c))
                    .text_sm()
                    .text_color(rgb(0x999999))
                    .child(format!("Canvas: {}×{}", width, height))
                    .child("|")
                    .child(format!("Zoom: {:.0}%", zoom * 100.0))
                    .child("|")
                    .child(format!("Layer: {}", self.document.active_layer().unwrap_or("None")))
            )
    }
}
