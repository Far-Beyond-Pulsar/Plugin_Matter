//! Toolbar component

use gpui::*;
use crate::state::ActiveTool;

/// Render the toolbar
pub fn render_toolbar(
    active_tool: ActiveTool,
    brush_size: f32,
    brush_opacity: f32,
    can_undo: bool,
    can_redo: bool,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(undo_button(can_undo))
        .child(redo_button(can_redo))
        .child(div().w_px().h_6().bg(rgb(0x3c3c3c)))
        .child(tool_button(active_tool == ActiveTool::Paint, "🖌️".to_string()))
        .child(tool_button(active_tool == ActiveTool::Erase, "⭕".to_string()))
        .child(tool_button(active_tool == ActiveTool::Fill, "🪣".to_string()))
        .child(tool_button(active_tool == ActiveTool::Eyedropper, "💧".to_string()))
        .child(tool_button(active_tool == ActiveTool::Pan, "↔️".to_string()))
        .child(div().w_px().h_6().bg(rgb(0x3c3c3c)))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x999999))
                        .child(format!("Size: {:.0}px", brush_size))
                )
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x999999))
                        .child(format!("Opacity: {:.0}%", brush_opacity * 100.0))
                )
        )
}

fn undo_button(can_undo: bool) -> impl IntoElement {
    if can_undo {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_8()
            .h_8()
            .rounded_md()
            .hover(|s| s.bg(rgb(0x3c3c3c)))
            .child("↶")
    } else {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_8()
            .h_8()
            .rounded_md()
            .opacity(0.3)
            .child("↶")
    }
}

fn redo_button(can_redo: bool) -> impl IntoElement {
    if can_redo {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_8()
            .h_8()
            .rounded_md()
            .hover(|s| s.bg(rgb(0x3c3c3c)))
            .child("↷")
    } else {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_8()
            .h_8()
            .rounded_md()
            .opacity(0.3)
            .child("↷")
    }
}

fn tool_button(is_active: bool, icon: String) -> impl IntoElement {
    if is_active {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_10()
            .h_10()
            .rounded_md()
            .bg(rgb(0x0078d4))
            .text_color(rgb(0xffffff))
            .child(icon)
    } else {
        div()
            .flex()
            .items_center()
            .justify_center()
            .w_10()
            .h_10()
            .rounded_md()
            .hover(|s| s.bg(rgb(0x3c3c3c)))
            .text_color(rgb(0xcccccc))
            .child(icon)
    }
}
