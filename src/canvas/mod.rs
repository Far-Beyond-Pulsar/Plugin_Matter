//! Canvas rendering and interaction

mod viewport;
pub mod stroke;

pub use viewport::render_canvas;

use gpui::*;

pub struct CanvasViewport;

impl CanvasViewport {
    pub fn render(_viewport: &crate::state::ViewportState, theme: &ui::Theme) -> impl IntoElement {
        // Simple checkerboard for now
        div()
            .size_full()
            .bg(theme.background)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child("Canvas viewport - TODO: implement rendering")
            )
    }
}
