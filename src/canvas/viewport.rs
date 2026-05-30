//! Canvas viewport rendering

use gpui::{Bounds, canvas, fill, IntoElement, Pixels, point, px, Rgba, size, Window};
use crate::state::{Document, ViewportState};

/// Canvas viewport component  
pub struct CanvasViewport;

impl CanvasViewport {
    pub fn new() -> Self {
        Self
    }
    
    /// Render a checkerboard background
    fn render_checkerboard(window: &mut Window, bounds: Bounds<Pixels>) {
        let checker_size = px(16.0);
        let light = Rgba { r: 0.9, g: 0.9, b: 0.9, a: 1.0 };
        let dark = Rgba { r: 0.7, g: 0.7, b: 0.7, a: 1.0 };
        
        let cols = (f32::from(bounds.size.width) / f32::from(checker_size)).ceil() as i32;
        let rows = (f32::from(bounds.size.height) / f32::from(checker_size)).ceil() as i32;
        
        for row in 0..rows {
            for col in 0..cols {
                let is_light = (row + col) % 2 == 0;
                let color = if is_light { light } else { dark };
                
                window.paint_quad(fill(
                    Bounds {
                        origin: point(
                            bounds.origin.x + checker_size * col as f32,
                            bounds.origin.y + checker_size * row as f32,
                        ),
                        size: size(checker_size, checker_size),
                    },
                    color,
                ));
            }
        }
    }
    
    pub fn render(
        self,
        _doc: &Document,
        _viewport: &ViewportState,
    ) -> impl IntoElement {
        canvas(
            move |_bounds, _window, _cx| {},
            move |bounds, _state, window, _cx| {
                // Render checkerboard background
                Self::render_checkerboard(window, bounds);
                
                // TODO: Render PIF layers
                // TODO: Render brush cursor
            },
        )
    }
}
