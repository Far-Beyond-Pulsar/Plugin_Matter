//! CanvasViewport — GPUI entity that owns the WgpuSurface and drives the
//! canvas renderer each frame. Follows the same pattern as HelioViewport in
//! the Pulsar level editor.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui::*;
use parking_lot::RwLock;
use ui::ActiveTheme;

use crate::state::{ActiveTool, Document, commands::PaintStrokeCommand};
use crate::canvas::stroke::render_brush_stamp;
use super::renderer::{CanvasRenderer, CanvasRenderInput};

const SURFACE_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ZOOM_MIN:    f32 = 0.05;
const ZOOM_MAX:    f32 = 32.0;
const TILE_SIZE:   u32 = 256;

// ── Active stroke state ───────────────────────────────────────────────────────

struct ActiveStroke {
    color:        [u8; 4],
    brush_size:   f32,
    brush_opacity: f32,
    is_eraser:    bool,
    layer_id:     String,
    /// Tile snapshots taken *before* this stroke touched each tile.
    tiles_before: HashMap<(String, u32, u32), Vec<u8>>,
    /// Current state of every tile that has been modified.
    tiles_current: HashMap<(String, u32, u32), Vec<u8>>,
    /// Last canvas-space point stamped (for interpolation).
    last_pos:     Option<Point<f32>>,
}

// ── CanvasViewport entity ─────────────────────────────────────────────────────

pub struct CanvasViewport {
    focus_handle:  FocusHandle,
    document:      Arc<RwLock<Document>>,
    renderer:      Arc<Mutex<CanvasRenderer>>,
    surface:       Option<WgpuSurfaceHandle>,

    /// Canvas origin in element-local pixels (pan offset).
    pan:  Point<f32>,
    zoom: f32,

    // Pan via right-click or middle-click drag
    is_panning:          bool,
    pan_win_start:       Option<Point<f32>>,
    pan_offset_at_start: Point<f32>,

    is_mid_panning:          bool,
    mid_win_start:           Option<Point<f32>>,
    mid_offset_at_start:     Point<f32>,

    // Active stroke (Paint/Erase)
    active_stroke: Option<ActiveStroke>,

    // Raw window-space cursor position
    cursor_win: Option<Point<f32>>,

    // Element origin tracked by cursor overlay's paint callback
    element_origin: Rc<RefCell<Point<f32>>>,
}

impl CanvasViewport {
    pub fn new(document: Arc<RwLock<Document>>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle:  cx.focus_handle(),
            document,
            renderer:      Arc::new(Mutex::new(CanvasRenderer::new())),
            surface:       None,
            pan:           Point::new(32.0, 32.0),
            zoom:          1.0,
            is_panning:            false,
            pan_win_start:         None,
            pan_offset_at_start:   Point::default(),
            is_mid_panning:            false,
            mid_win_start:             None,
            mid_offset_at_start:       Point::default(),
            active_stroke: None,
            cursor_win:    None,
            element_origin: Rc::new(RefCell::new(Point::default())),
        }
    }

    // ── Coordinate helpers ────────────────────────────────────────────────────

    fn to_local(&self, win: Point<f32>) -> Point<f32> {
        let o = *self.element_origin.borrow();
        Point::new(win.x - o.x, win.y - o.y)
    }

    fn canvas_of_local(&self, local: Point<f32>) -> Point<f32> {
        Point::new(
            (local.x - self.pan.x) / self.zoom,
            (local.y - self.pan.y) / self.zoom,
        )
    }

    fn win_f32(p: Point<Pixels>) -> Point<f32> {
        Point::new(f32::from(p.x), f32::from(p.y))
    }

    // ── Drawing helpers ───────────────────────────────────────────────────────

    /// Ensure a tile is loaded into `tiles_current` and snapshot it into
    /// `tiles_before` the first time it is touched.
    fn ensure_tile(
        layer_id: &str,
        tx: u32, ty: u32,
        doc: &Document,
        tiles_before: &mut HashMap<(String, u32, u32), Vec<u8>>,
        tiles_current: &mut HashMap<(String, u32, u32), Vec<u8>>,
    ) {
        let key = (layer_id.to_string(), tx, ty);
        if tiles_current.contains_key(&key) { return; }

        let data = doc.load_tile(layer_id, tx, ty).unwrap_or_default();
        let data = if data.is_empty() {
            vec![0u8; (TILE_SIZE * TILE_SIZE * 4) as usize]
        } else {
            data
        };
        tiles_before.insert(key.clone(), data.clone());
        tiles_current.insert(key, data);
    }

    /// Stamp one brush circle at `canvas_pos` into live tile buffers.
    fn stamp_at(
        stroke: &mut ActiveStroke,
        canvas_pos: Point<f32>,
        doc: &Document,
    ) {
        let radius = stroke.brush_size * 0.5;
        let min_x = (canvas_pos.x - radius).floor().max(0.0) as u32;
        let min_y = (canvas_pos.y - radius).floor().max(0.0) as u32;
        let max_x = (canvas_pos.x + radius).ceil() as u32;
        let max_y = (canvas_pos.y + radius).ceil() as u32;

        let min_tx = min_x / TILE_SIZE;
        let max_tx = max_x / TILE_SIZE;
        let min_ty = min_y / TILE_SIZE;
        let max_ty = max_y / TILE_SIZE;

        for ty in min_ty..=max_ty {
            for tx in min_tx..=max_tx {
                Self::ensure_tile(
                    &stroke.layer_id, tx, ty, doc,
                    &mut stroke.tiles_before,
                    &mut stroke.tiles_current,
                );
                let key = (stroke.layer_id.clone(), tx, ty);
                if let Some(tile) = stroke.tiles_current.get_mut(&key) {
                    render_brush_stamp(
                        tile,
                        canvas_pos,
                        stroke.brush_size,
                        stroke.brush_opacity,
                        stroke.color,
                        tx * TILE_SIZE,
                        ty * TILE_SIZE,
                    );
                }
            }
        }
    }

    /// Interpolate between `from` and `to`, stamping every `step` pixels.
    fn stamp_segment(
        stroke: &mut ActiveStroke,
        from:   Point<f32>,
        to:     Point<f32>,
        doc:    &Document,
    ) {
        let dx   = to.x - from.x;
        let dy   = to.y - from.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (stroke.brush_size * 0.2).max(1.0);
        let steps = (dist / step).ceil() as u32;

        for i in 1..=steps.max(1) {
            let t = i as f32 / steps.max(1) as f32;
            let pt = Point::new(from.x + dx * t, from.y + dy * t);
            Self::stamp_at(stroke, pt, doc);
        }
    }

    /// Begin a paint stroke from a canvas-space position.
    fn start_stroke(&mut self, canvas_pos: Point<f32>) {
        let doc = self.document.read();
        let layer_id = match doc.active_layer() {
            Some(id) => id.to_string(),
            None => return,
        };
        let color = doc.tool_state.foreground_bytes();
        let (color, is_eraser) = match doc.tool_state.active_tool {
            ActiveTool::Erase => ([0u8, 0, 0, 0], true),
            _ => (color, false),
        };

        let mut stroke = ActiveStroke {
            color,
            brush_size:    doc.tool_state.brush_size,
            brush_opacity: doc.tool_state.brush_opacity,
            is_eraser,
            layer_id,
            tiles_before:  HashMap::new(),
            tiles_current: HashMap::new(),
            last_pos:      Some(canvas_pos),
        };
        Self::stamp_at(&mut stroke, canvas_pos, &doc);
        self.active_stroke = Some(stroke);
    }

    /// Continue an active stroke to `canvas_pos`.
    fn continue_stroke(&mut self, canvas_pos: Point<f32>) {
        let doc = self.document.read();
        if let Some(stroke) = &mut self.active_stroke {
            let from = stroke.last_pos.unwrap_or(canvas_pos);
            Self::stamp_segment(stroke, from, canvas_pos, &doc);
            stroke.last_pos = Some(canvas_pos);
        }
    }

    /// Finish the stroke: commit to PIF and push undo entry.
    fn finish_stroke(&mut self) {
        let Some(stroke) = self.active_stroke.take() else { return };
        if stroke.tiles_current.is_empty() { return; }

        let mut doc = self.document.write();
        // Write live tiles to PIF.
        let _ = doc.pif.lock().commit_changes(stroke.tiles_current.clone());
        doc.mark_dirty();

        // Register for undo — the changes are already applied so we use push_executed.
        let cmd = PaintStrokeCommand::from_tiles(
            doc.pif.clone(),
            stroke.tiles_before,
            stroke.tiles_current,
        );
        doc.history.push_executed(Box::new(cmd));
    }

    // ── Cursor overlay ────────────────────────────────────────────────────────

    fn render_cursor_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let cursor_win   = self.cursor_win;
        let origin_rc    = self.element_origin.clone();
        let brush_radius = {
            let doc = self.document.read();
            (doc.tool_state.brush_size * 0.5 * self.zoom).max(2.0)
        };
        let painting = self.active_stroke.is_some();
        let theme    = cx.theme().clone();

        gpui::canvas(
            |_bounds, _win, _cx| {},
            move |bounds, _pre, window, _cx| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);
                *origin_rc.borrow_mut() = Point::new(ox, oy);

                let Some(cw) = cursor_win else { return };
                let wx = cw.x;
                let wy = cw.y;

                let ring = if painting {
                    Hsla { h: 0.58, s: 0.8, l: 0.7, a: 0.9 }
                } else {
                    theme.foreground.opacity(0.85)
                };

                let segs = 32usize;
                let mut b = gpui::PathBuilder::stroke(px(1.5));
                let _ = b.move_to(point(px(wx + brush_radius), px(wy)));
                for i in 1..=segs {
                    let t = std::f32::consts::TAU * i as f32 / segs as f32;
                    let _ = b.line_to(point(
                        px(wx + brush_radius * t.cos()),
                        px(wy + brush_radius * t.sin()),
                    ));
                }
                let _ = b.close();
                if let Ok(p) = b.build() { window.paint_path(p, ring); }

                let ch = 4.0f32;
                for (ax, ay, bx2, by2) in [
                    (wx - ch, wy, wx + ch, wy),
                    (wx, wy - ch, wx, wy + ch),
                ] {
                    let mut line = gpui::PathBuilder::stroke(px(1.0));
                    let _ = line.move_to(point(px(ax), px(ay)));
                    let _ = line.line_to(point(px(bx2), px(by2)));
                    if let Ok(p) = line.build() { window.paint_path(p, ring); }
                }
            },
        )
        .absolute()
        .inset_0()
    }
}

impl Focusable for CanvasViewport {
    fn focus_handle(&self, _cx: &App) -> FocusHandle { self.focus_handle.clone() }
}

impl EventEmitter<()> for CanvasViewport {}

impl Render for CanvasViewport {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.request_animation_frame();

        // ── Lazy WgpuSurface creation ──────────────────────────────────────
        if self.surface.is_none() {
            match window.create_wgpu_surface(1600, 900, SURFACE_FMT) {
                Some(s) => {
                    tracing::info!("[CanvasViewport] WgpuSurface created");
                    self.surface = Some(s);
                }
                None => tracing::warn!("[CanvasViewport] create_wgpu_surface returned None"),
            }
        }

        // ── Render frame into back buffer ──────────────────────────────────
        if let Some(ref surface) = self.surface {
            if !surface.is_resize_pending() {
                if let Some((view, (w, h))) = surface.back_view_with_size() {
                    let (canvas_w, canvas_h) = self.document.read().dimensions();
                    let doc = self.document.read();

                    let cursor_local = self.cursor_win.map(|cw| {
                        let local = self.to_local(cw);
                        [local.x, local.y]
                    });

                    let input = CanvasRenderInput {
                        pan_offset:        [self.pan.x, self.pan.y],
                        zoom:              self.zoom,
                        canvas_size:       [canvas_w as f32, canvas_h as f32],
                        cursor_screen_pos: cursor_local,
                        brush_radius:      doc.tool_state.brush_size * 0.5 * self.zoom,
                        brush_color:       {
                            let c = doc.tool_state.foreground_color;
                            [c.r, c.g, c.b, c.a]
                        },
                    };

                    let live = self.active_stroke.as_ref().map(|s| &s.tiles_current);

                    if let Ok(mut renderer) = self.renderer.try_lock() {
                        renderer.render_frame(
                            surface.device(), surface.queue(),
                            &view, w, h, surface.format(),
                            &input, &doc, live,
                        );
                    }
                    drop(view);
                    surface.swap_buffers();
                }
            }
        }

        // ── Build elements ─────────────────────────────────────────────────
        let wgpu_elem: AnyElement = if let Some(ref surface) = self.surface {
            wgpu_surface(surface.clone())
                .defer_resize_until_mouse_up(true)
                .absolute()
                .inset_0()
                .into_any_element()
        } else {
            div()
                .absolute()
                .inset_0()
                .bg(rgba(0x1e1e23ff))
                .flex()
                .items_center()
                .justify_center()
                .child(div().text_color(rgba(0x888899ff)).text_sm()
                       .child("Initialising canvas…"))
                .into_any_element()
        };

        let cursor_overlay = self.render_cursor_overlay(cx);

        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .key_context("CanvasViewport")
            // Right-click drag → pan
            .on_mouse_down(MouseButton::Right, cx.listener(|this, ev: &MouseDownEvent, _win, _cx| {
                let w = Self::win_f32(ev.position);
                this.is_panning          = true;
                this.pan_win_start       = Some(w);
                this.pan_offset_at_start = this.pan;
            }))
            .on_mouse_up(MouseButton::Right, cx.listener(|this, _ev: &MouseUpEvent, _win, _cx| {
                this.is_panning    = false;
                this.pan_win_start = None;
            }))
            // Middle-click drag → pan
            .on_mouse_down(MouseButton::Middle, cx.listener(|this, ev: &MouseDownEvent, _win, _cx| {
                let w = Self::win_f32(ev.position);
                this.is_mid_panning         = true;
                this.mid_win_start          = Some(w);
                this.mid_offset_at_start    = this.pan;
            }))
            .on_mouse_up(MouseButton::Middle, cx.listener(|this, _ev: &MouseUpEvent, _win, _cx| {
                this.is_mid_panning = false;
                this.mid_win_start  = None;
            }))
            // Left-click → paint / erase / pan (Hand tool)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, ev: &MouseDownEvent, _win, _cx| {
                let w    = Self::win_f32(ev.position);
                let tool = this.document.read().tool_state.active_tool;
                match tool {
                    ActiveTool::Pan => {
                        this.is_panning          = true;
                        this.pan_win_start       = Some(w);
                        this.pan_offset_at_start = this.pan;
                    }
                    ActiveTool::Paint | ActiveTool::Erase => {
                        let local      = this.to_local(w);
                        let canvas_pos = this.canvas_of_local(local);
                        this.start_stroke(canvas_pos);
                    }
                    _ => {}
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _ev: &MouseUpEvent, _win, _cx| {
                if this.active_stroke.is_some() {
                    this.finish_stroke();
                }
                this.is_panning    = false;
                this.pan_win_start = None;
            }))
            // Mouse move: cursor tracking + panning + stroke continuation
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _win, cx| {
                let w = Self::win_f32(ev.position);
                this.cursor_win = Some(w);

                if this.is_panning {
                    if let Some(start) = this.pan_win_start {
                        this.pan.x = this.pan_offset_at_start.x + (w.x - start.x);
                        this.pan.y = this.pan_offset_at_start.y - (w.y - start.y);
                    }
                } else if this.is_mid_panning {
                    if let Some(start) = this.mid_win_start {
                        this.pan.x = this.mid_offset_at_start.x + (w.x - start.x);
                        this.pan.y = this.mid_offset_at_start.y - (w.y - start.y);
                    }
                } else if this.active_stroke.is_some() {
                    let local      = this.to_local(w);
                    let canvas_pos = this.canvas_of_local(local);
                    this.continue_stroke(canvas_pos);
                }

                cx.notify();
            }))
            // Scroll → zoom around cursor
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, _win, cx| {
                let w     = Self::win_f32(ev.position);
                let local = this.to_local(w);
                let focus = this.canvas_of_local(local);

                let raw: f32 = match ev.delta {
                    ScrollDelta::Pixels(p) => p.y.into(),
                    ScrollDelta::Lines(l)  => l.y * 32.0,
                };
                let factor = if raw < 0.0 { 1.1f32 } else { 0.9f32 };
                let new_zoom = (this.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);

                this.pan.x = local.x - focus.x * new_zoom;
                this.pan.y = local.y - focus.y * new_zoom;
                this.zoom  = new_zoom;
                cx.notify();
            }))
            .child(wgpu_elem)
            .child(cursor_overlay)
    }
}
