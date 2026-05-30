//! Toolbar rendering

use gpui::*;
use ui::{button::{Button, ButtonVariants}, IconName, Theme};

use crate::state::{Document, ActiveTool};

pub fn render_toolbar(
    doc: &Document,
    theme: &Theme,
) -> impl IntoElement {
    let can_undo = !doc.history.is_empty_undo();
    let can_redo = !doc.history.is_empty_redo();
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
        .child(
            if can_undo {
                Button::new("undo")
                    .icon(IconName::Undo)
                    .tooltip("Undo (Cmd+Z)")
                    .on_click(|_, _, _| {})
            } else {
                Button::new("undo").icon(IconName::Undo)
            }
        )
        .child(
            if can_redo {
                Button::new("redo")
                    .icon(IconName::Redo)
                    .tooltip("Redo (Cmd+Shift+Z)")
                    .on_click(|_, _, _| {})
            } else {
                Button::new("redo").icon(IconName::Redo)
            }
        )
        .child(separator(theme))
        .child(tool_button(IconName::EditPencil, "Paint (B)", active_tool == ActiveTool::Paint))
        .child(tool_button(IconName::Erase, "Eraser (E)", active_tool == ActiveTool::Erase))
        .child(tool_button(IconName::Droplet, "Fill (G)", active_tool == ActiveTool::Fill))
        .child(tool_button(IconName::Eye, "Eyedropper (I)", active_tool == ActiveTool::Eyedropper))
        .child(tool_button(IconName::DragHandGesture, "Pan (H)", active_tool == ActiveTool::Pan))
}

fn tool_button(
    icon: IconName,
    tooltip_text: &str,
    is_active: bool,
) -> Button {
    let id = format!("tool_{:?}", icon);
    let mut btn = Button::new(id)
        .icon(icon)
        .tooltip(tooltip_text);
    
    if is_active {
        btn = btn.primary();
    }
    
    btn.on_click(move |_, _, _| {})
}

fn separator(theme: &Theme) -> impl IntoElement {
    div()
        .w(px(1.0))
        .h(px(24.0))
        .bg(theme.border.opacity(0.5))
        .mx_1()
}
