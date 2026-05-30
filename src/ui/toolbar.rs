//! Toolbar rendering

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;
use ui::{button::{Button, ButtonVariants}, IconName, Theme};

use crate::state::{Document, ActiveTool};

pub fn render_toolbar(
    doc: &Document,
    document: Arc<RwLock<Document>>,
    theme: &Theme,
) -> impl IntoElement {
    let can_undo    = !doc.history.is_empty_undo();
    let can_redo    = !doc.history.is_empty_redo();
    let active_tool = doc.tool_state.active_tool;

    div()
        .flex()
        .w_full()
        .h(px(48.0))
        .bg(theme.sidebar.opacity(0.98))
        .border_b_1()
        .border_color(theme.border.opacity(0.8))
        .items_center()
        .px_2()
        .gap_1()
        // ── History ────────────────────────────────────────────────────────
        .child({
            let doc_undo = document.clone();
            let mut btn = Button::new("undo").icon(IconName::Undo).tooltip("Undo (Cmd+Z)");
            if can_undo {
                btn = btn.on_click(move |_, _, _| {
                    let _ = doc_undo.write().history.undo();
                });
            }
            btn
        })
        .child({
            let doc_redo = document.clone();
            let mut btn = Button::new("redo").icon(IconName::Redo).tooltip("Redo (Cmd+Shift+Z)");
            if can_redo {
                btn = btn.on_click(move |_, _, _| {
                    let _ = doc_redo.write().history.redo();
                });
            }
            btn
        })
        .child(separator(theme))
        // ── Tools ──────────────────────────────────────────────────────────
        .child(tool_button(document.clone(), IconName::EditPencil, "Paint (B)",       ActiveTool::Paint,      active_tool))
        .child(tool_button(document.clone(), IconName::Erase,      "Eraser (E)",      ActiveTool::Erase,      active_tool))
        .child(tool_button(document.clone(), IconName::Droplet,    "Fill (G)",        ActiveTool::Fill,       active_tool))
        .child(tool_button(document.clone(), IconName::Eye,        "Eyedropper (I)",  ActiveTool::Eyedropper, active_tool))
        .child(tool_button(document.clone(), IconName::DragHandGesture, "Pan (H)",    ActiveTool::Pan,        active_tool))
        .child(separator(theme))
        // ── Zoom label ─────────────────────────────────────────────────────
        .child(
            div()
                .text_xs()
                .text_color(theme.foreground.opacity(0.6))
                .px_2()
                .child("100%")   // TODO: bind to viewport zoom
        )
}

fn tool_button(
    document:    Arc<RwLock<Document>>,
    icon:        IconName,
    tooltip_text: &str,
    tool:        ActiveTool,
    active_tool: ActiveTool,
) -> Button {
    let id = format!("tool_{:?}", icon);
    let mut btn = Button::new(id).icon(icon).tooltip(tooltip_text);

    if active_tool == tool {
        btn = btn.primary();
    }

    btn.on_click(move |_, _, _| {
        document.write().tool_state.active_tool = tool;
    })
}

fn separator(theme: &Theme) -> impl IntoElement {
    div()
        .w(px(1.0))
        .h(px(24.0))
        .bg(theme.border.opacity(0.5))
        .mx_1()
}
