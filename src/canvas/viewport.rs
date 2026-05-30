//! Canvas viewport component

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;
use ui::Theme;

use crate::state::Document;
use crate::tools::ToolRegistry;

pub fn render_canvas(
    document: Arc<RwLock<Document>>,
    _tools: Arc<RwLock<ToolRegistry>>,
    theme: &Theme,
) -> impl IntoElement {
    let doc = document.read();
    
    div()
        .flex_1()
        .bg(theme.background)
        .child(super::CanvasViewport::render(&doc.viewport, theme))
}
