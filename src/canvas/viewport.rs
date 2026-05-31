//! CanvasViewport — GPUI entity that owns the WgpuSurface and drives the
//! canvas renderer each frame. Follows the same pattern as HelioViewport in
//! the Pulsar level editor.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui::*;
use parking_lot::RwLock;
use ui::ActiveTheme;

use crate::brush_engine::{stamp_into_wet, BrushMask, BrushRegistry};
use crate::state::{ActiveTool, Document, commands::PaintStrokeCommand};
use super::renderer::{CanvasRenderer, CanvasRenderInput};

const SURFACE_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const ZOOM_MIN:    f32 = 0.05;
const ZOOM_MAX:    f32 = 32.0;
const TILE_SIZE:   u32 = 256;

// ── Active stroke state ───────────────────────────────────────────────────────

/// Number of recent positions kept for direction averaging.
/// A longer window gives more stable rotation at the cost of a small lag.
const DIR_WINDOW: usize = 12;

struct ActiveStroke {
    color:         [u8; 4],
    brush_size:    f32,
    brush_opacity: f32,
    is_eraser:     bool,
    layer_id:      String,
    /// Brush mask locked in at stroke start.
    brush_mask:    Arc<BrushMask>,
    /// Base spacing from brush config (`0..1` of brush size).
    brush_spacing: f32,

    // ── Direction tracking ────────────────────────────────────────────────────
    /// Smoothed stroke-direction unit vector (cos θ, sin θ).
    /// Stored as a vector — not an angle — to avoid wrap-around at ±π.
    dir_x:      f32,
    dir_y:      f32,
    /// Ring buffer of recent canvas-space positions used to compute a stable
    /// windowed direction rather than a noisy per-sample delta.
    pos_buffer: VecDeque<Point<f32>>,

    // ── Wet-buffer model ──────────────────────────────────────────────────────
    /// Snapshot of each tile *before* this stroke began.  Never modified.
    tiles_before:  HashMap<(String, u32, u32), Vec<u8>>,
    /// Accumulated stroke stamps for this stroke.
    wet_tiles:     HashMap<(String, u32, u32), Vec<u8>>,
    /// Composited result (tiles_before + wet).  This is
    /// what gets written to PIF and shown as live tile data during the stroke.
    tiles_current: HashMap<(String, u32, u32), Vec<u8>>,

    /// Last canvas-space point stamped (for interpolation).
    last_pos: Option<Point<f32>>,
    /// Smoothed speed in brush-diameters per pointer sample.
    speed_ema: f32,
    /// Distance since the last placed stamp (for arc-length resampling).
    stamp_carry: f32,
}

#[derive(Clone, Copy)]
struct StampSample {
    pos: Point<f32>,
    dir_x: f32,
    dir_y: f32,
    axis_scale: f32,
    flow_scale: f32,
}

// ── CanvasViewport entity ─────────────────────────────────────────────────────

pub struct CanvasViewport {
    focus_handle:    FocusHandle,
    document:        Arc<RwLock<Document>>,
    brush_registry:  Arc<BrushRegistry>,
    renderer:        Arc<Mutex<CanvasRenderer>>,
    surface:         Option<WgpuSurfaceHandle>,

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

    /// Last confirmed stroke direction (unit vector).
    /// Persists between strokes so the first stamp of a new stroke continues
    /// in the same orientation rather than defaulting to an arbitrary angle —
    /// matches the behaviour of Photoshop, Procreate, and Clip Studio Paint.
    last_dir_x: f32,
    last_dir_y: f32,

    // Raw window-space cursor position
    cursor_win: Option<Point<f32>>,

    // Element origin and size tracked by cursor overlay's paint callback.
    // Logical pixel values from GPUI's layout — used to convert mouse events
    // (also logical) into element-local / canvas-space coordinates.
    element_origin: Rc<RefCell<Point<f32>>>,
    element_size:   Rc<RefCell<[f32; 2]>>,
}

impl CanvasViewport {
    pub fn new(
        document:       Arc<RwLock<Document>>,
        brush_registry: Arc<BrushRegistry>,
        cx:             &mut Context<Self>,
    ) -> Self {
        Self {
            focus_handle:    cx.focus_handle(),
            document,
            brush_registry,
            renderer:        Arc::new(Mutex::new(CanvasRenderer::new())),
            surface:         None,
            pan:           Point::new(32.0, 32.0),
            zoom:          1.0,
            last_dir_x: 1.0,
            last_dir_y: 0.0,
            is_panning:            false,
            pan_win_start:         None,
            pan_offset_at_start:   Point::default(),
            is_mid_panning:            false,
            mid_win_start:             None,
            mid_offset_at_start:       Point::default(),
            active_stroke: None,
            cursor_win:    None,
            element_origin: Rc::new(RefCell::new(Point::default())),
            element_size:   Rc::new(RefCell::new([0.0, 0.0])),
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

    /// Ensure a tile is initialised in all stroke buffers.
    fn ensure_tile(
        layer_id:      &str,
        tx: u32, ty: u32,
        doc:           &Document,
        tiles_before:  &mut HashMap<(String, u32, u32), Vec<u8>>,
        wet_tiles:     &mut HashMap<(String, u32, u32), Vec<u8>>,
        tiles_current: &mut HashMap<(String, u32, u32), Vec<u8>>,
    ) {
        let key = (layer_id.to_string(), tx, ty);
        if tiles_before.contains_key(&key) { return; }

        let raw = doc.load_tile(layer_id, tx, ty).unwrap_or_default();
        let base = if raw.is_empty() {
            vec![0u8; (TILE_SIZE * TILE_SIZE * 4) as usize]
        } else {
            raw
        };
        wet_tiles.insert(key.clone(), vec![0u8; (TILE_SIZE * TILE_SIZE * 4) as usize]);
        tiles_current.insert(key.clone(), base.clone());
        tiles_before.insert(key, base);
    }

    fn apply_wet_over_current(cur: &mut [u8], wet: &[u8]) {
        for i in (0..cur.len()).step_by(4) {
            let sa = wet[i + 3] as f32 / 255.0;
            if sa <= 0.0 {
                continue;
            }
            let da = cur[i + 3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 {
                continue;
            }
            let sr = wet[i] as f32 / 255.0;
            let sg = wet[i + 1] as f32 / 255.0;
            let sb = wet[i + 2] as f32 / 255.0;
            let dr = cur[i] as f32 / 255.0;
            let dg = cur[i + 1] as f32 / 255.0;
            let db = cur[i + 2] as f32 / 255.0;
            let out_r = (sr * sa + dr * da * (1.0 - sa)) / out_a;
            let out_g = (sg * sa + dg * da * (1.0 - sa)) / out_a;
            let out_b = (sb * sa + db * da * (1.0 - sa)) / out_a;
            cur[i] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
            cur[i + 1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
            cur[i + 2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
            cur[i + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    fn apply_wet_erase_over_current(cur: &mut [u8], wet: &[u8]) {
        for i in (0..cur.len()).step_by(4) {
            let erase = wet[i + 3] as f32 / 255.0;
            if erase <= 0.0 {
                continue;
            }
            let ca = cur[i + 3] as f32 / 255.0;
            cur[i + 3] = (ca * (1.0 - erase) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    fn recomposite_tile(stroke: &mut ActiveStroke, key: &(String, u32, u32)) {
        let (Some(before), Some(cur)) = (
            stroke.tiles_before.get(key),
            stroke.tiles_current.get_mut(key),
        ) else {
            return;
        };
        cur.clone_from_slice(before);
        if stroke.is_eraser {
            if let Some(w) = stroke.wet_tiles.get(key) {
                Self::apply_wet_erase_over_current(cur, w);
            }
        } else {
            if let Some(w) = stroke.wet_tiles.get(key) {
                Self::apply_wet_over_current(cur, w);
            }
        }
    }

    fn stamp_sample(
        stroke:     &mut ActiveStroke,
        sample:     &StampSample,
        doc:        &Document,
    ) {
        let angle  = sample.dir_y.atan2(sample.dir_x);
        let radius = stroke.brush_size * 0.5;
        let min_x  = (sample.pos.x - radius).floor().max(0.0) as u32;
        let min_y  = (sample.pos.y - radius).floor().max(0.0) as u32;
        let max_x  = (sample.pos.x + radius).ceil() as u32;
        let max_y  = (sample.pos.y + radius).ceil() as u32;

        let min_tx = min_x / TILE_SIZE;
        let max_tx = max_x / TILE_SIZE;
        let min_ty = min_y / TILE_SIZE;
        let max_ty = max_y / TILE_SIZE;

        for ty in min_ty..=max_ty {
            for tx in min_tx..=max_tx {
                Self::ensure_tile(
                    &stroke.layer_id, tx, ty, doc,
                    &mut stroke.tiles_before,
                    &mut stroke.wet_tiles,
                    &mut stroke.tiles_current,
                );
                let key = (stroke.layer_id.clone(), tx, ty);

                let wet = stroke.wet_tiles.get_mut(&key);
                if let Some(wet) = wet {
                    stamp_into_wet(
                        wet,
                        &stroke.brush_mask,
                        sample.pos,
                        stroke.brush_size,
                        (stroke.brush_size * sample.axis_scale).max(1.0),
                        angle,
                        (stroke.brush_opacity * sample.flow_scale).clamp(0.01, 1.0),
                        stroke.brush_opacity,
                        stroke.color,
                        tx * TILE_SIZE,
                        ty * TILE_SIZE,
                    );
                }
                Self::recomposite_tile(stroke, &key);
            }
        }
    }

    /// Interpolate stamps from `from` to `to`, updating the windowed direction.
    ///
    /// Direction is derived from a **sliding window** of the last [`DIR_WINDOW`]
    /// positions (oldest→newest trend), then lightly smoothed with a low-α EMA.
    /// This is far more stable than per-sample deltas — it matches the approach
    /// used internally by Photoshop's brush engine.
    fn stamp_segment(
        stroke: &mut ActiveStroke,
        from:   Point<f32>,
        to:     Point<f32>,
        doc:    &Document,
    ) {
        let dx   = to.x - from.x;
        let dy   = to.y - from.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 0.01 {
            return;
        }
        let seg_nx = dx / dist;
        let seg_ny = dy / dist;

        // Add the new position to the ring buffer.
        if stroke.pos_buffer.len() >= DIR_WINDOW {
            stroke.pos_buffer.pop_front();
        }
        stroke.pos_buffer.push_back(to);

        // Compute direction from the oldest to the newest sample in the window.
        // Using the full window trend (not just the last delta) means a straight
        // line stays straight even if individual input samples are jittery.
        const MIN_MOVE: f32 = 1.0; // canvas pixels — below this, keep last dir
        if stroke.pos_buffer.len() >= 2 && dist >= MIN_MOVE {
            let oldest = *stroke.pos_buffer.front().unwrap();
            let newest = *stroke.pos_buffer.back().unwrap();
            let tdx = newest.x - oldest.x;
            let tdy = newest.y - oldest.y;
            let tlen = (tdx * tdx + tdy * tdy).sqrt();
            if tlen > 0.1 {
                let nx = tdx / tlen;
                let ny = tdy / tlen;
                // Light EMA on top of the windowed trend — keeps transitions
                // smooth without introducing perceptible lag.
                const ALPHA: f32 = 0.20;
                stroke.dir_x += (nx - stroke.dir_x) * ALPHA;
                stroke.dir_y += (ny - stroke.dir_y) * ALPHA;
            }
        }
        let dir_len = (stroke.dir_x * stroke.dir_x + stroke.dir_y * stroke.dir_y).sqrt();
        if dir_len > 0.0001 {
            stroke.dir_x /= dir_len;
            stroke.dir_y /= dir_len;
        } else {
            stroke.dir_x = seg_nx;
            stroke.dir_y = seg_ny;
        }

        // Speed in brush diameters per input sample.  We adapt spacing, flow,
        // and motion-axis footprint to keep fast strokes responsive and clean.
        let speed = if stroke.brush_size > 1.0 {
            dist / stroke.brush_size
        } else {
            dist
        };
        const SPEED_ALPHA: f32 = 0.35;
        stroke.speed_ema += (speed - stroke.speed_ema) * SPEED_ALPHA;
        let speed = stroke.speed_ema.max(0.0);

        let spacing_boost = (1.0 + speed * 1.10).clamp(1.0, 2.6);
        let raw_step = stroke.brush_size * stroke.brush_spacing * spacing_boost;
        let min_step = (stroke.brush_size * 0.15).max(0.25);
        let max_step = (stroke.brush_size * 0.70).max(min_step);
        let step = raw_step.clamp(min_step, max_step);

        let large_brush_factor = ((stroke.brush_size - 24.0) / 96.0).clamp(0.0, 1.0);
        let axis_scale = (1.0 - speed * large_brush_factor * 0.55).clamp(0.45, 1.0);
        let flow_scale = (1.0 / (1.0 + speed * 0.8)).clamp(0.35, 1.0);

        let carry = stroke.stamp_carry.min((step - 0.001).max(0.0));
        let mut offset = if carry <= 0.0 { step } else { step - carry };
        while offset <= dist {
            let pt = Point::new(from.x + seg_nx * offset, from.y + seg_ny * offset);
            let sample = StampSample {
                pos: pt,
                dir_x: stroke.dir_x,
                dir_y: stroke.dir_y,
                axis_scale,
                flow_scale,
            };
            Self::stamp_sample(stroke, &sample, doc);
            offset += step;
        }
        stroke.stamp_carry = if step > 0.0 { (carry + dist) % step } else { 0.0 };
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

        // Lock active brush params for the duration of this stroke.
        let (brush_mask, brush_spacing) = self
            .brush_registry
            .get(&doc.tool_state.active_brush_id)
            .map(|e| (e.mask.clone(), e.config.spacing.clamp(0.05, 0.75)))
            .unwrap_or_else(|| (Arc::new(BrushMask::default_round()), 0.20));

        let mut stroke = ActiveStroke {
            color,
            brush_size:    doc.tool_state.brush_size,
            brush_opacity: doc.tool_state.brush_opacity,
            is_eraser,
            layer_id,
            brush_mask,
            brush_spacing,
            // Inherit last stroke's direction — first stamp of a new stroke
            // stays oriented correctly rather than snapping to a fixed default.
            dir_x: self.last_dir_x,
            dir_y: self.last_dir_y,
            pos_buffer:    VecDeque::with_capacity(DIR_WINDOW),
            tiles_before:  HashMap::new(),
            wet_tiles:     HashMap::new(),
            tiles_current: HashMap::new(),
            last_pos:      Some(canvas_pos),
            speed_ema:     0.0,
            stamp_carry:   0.0,
        };
        let seed = StampSample {
            pos: canvas_pos,
            dir_x: stroke.dir_x,
            dir_y: stroke.dir_y,
            axis_scale: 1.0,
            flow_scale: 1.0,
        };
        Self::stamp_sample(&mut stroke, &seed, &doc);
        self.active_stroke = Some(stroke);
    }

    /// Continue an active stroke to `canvas_pos`.
    fn continue_stroke(&mut self, canvas_pos: Point<f32>) {
        let doc = self.document.read();
        if let Some(stroke) = &mut self.active_stroke {
            let from = stroke.last_pos.unwrap_or(canvas_pos);
            let smooth_alpha = (0.20 + (stroke.speed_ema * 0.25).clamp(0.0, 0.40)).clamp(0.20, 0.60);
            let smooth_pos = Point::new(
                from.x + (canvas_pos.x - from.x) * smooth_alpha,
                from.y + (canvas_pos.y - from.y) * smooth_alpha,
            );
            Self::stamp_segment(stroke, from, smooth_pos, &doc);
            stroke.last_pos = Some(smooth_pos);
        }
    }

    /// Finish the stroke: persist direction, commit to PIF, push undo entry.
    fn finish_stroke(&mut self) {
        let Some(stroke) = self.active_stroke.take() else { return };

        // Persist the stroke's final direction so the next stroke starts
        // oriented the same way (matches professional app behaviour).
        self.last_dir_x = stroke.dir_x;
        self.last_dir_y = stroke.dir_y;

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
        let size_rc      = self.element_size.clone();
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
                let sw = f32::from(bounds.size.width);
                let sh = f32::from(bounds.size.height);
                *origin_rc.borrow_mut() = Point::new(ox, oy);
                *size_rc.borrow_mut()   = [sw, sh];

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

                    // Logical display size from GPUI layout bounds.
                    // This is the same coordinate space as mouse events and pan
                    // offsets — critical for correct brush position calculation.
                    let viewport_size = *self.element_size.borrow();

                    let input = CanvasRenderInput {
                        pan_offset:        [self.pan.x, self.pan.y],
                        zoom:              self.zoom,
                        canvas_size:       [canvas_w as f32, canvas_h as f32],
                        viewport_size,
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
                        this.pan.y = this.pan_offset_at_start.y + (w.y - start.y);
                    }
                } else if this.is_mid_panning {
                    if let Some(start) = this.mid_win_start {
                        this.pan.x = this.mid_offset_at_start.x + (w.x - start.x);
                        this.pan.y = this.mid_offset_at_start.y + (w.y - start.y);
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
