//! Properties panel — brush picker, colour pickers, and brush settings.

use gpui::*;
use ui::{
    color_picker::ColorPicker,
    dropdown::{Dropdown, DropdownState},
    slider::{Slider, SliderState},
    Theme,
};

use crate::brush_engine::BrushDropdownItem;
use crate::state::Document;

pub fn render_properties_panel(
    doc:            &Document,
    fg_picker:      &Entity<ui::color_picker::ColorPickerState>,
    bg_picker:      &Entity<ui::color_picker::ColorPickerState>,
    brush_dropdown: &Entity<DropdownState<Vec<BrushDropdownItem>>>,
    brush_size:     &Entity<SliderState>,
    theme:          &Theme,
) -> impl IntoElement {
    let size_val  = doc.tool_state.brush_size;
    let brush_opacity = doc.tool_state.brush_opacity;

    div()
        .flex()
        .flex_col()
        .w(px(250.0))
        .h_full()
        .bg(theme.sidebar)
        .border_l_1()
        .border_color(theme.border)
        // ── Header ────────────────────────────────────────────────────────
        .child(
            div()
                .flex()
                .w_full()
                .h(px(40.0))
                .px_2()
                .items_center()
                .border_b_1()
                .border_color(theme.border.opacity(0.5))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child("Properties")
                )
        )
        // ── Colour section ─────────────────────────────────────────────────
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .p_3()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground.opacity(0.7))
                        .child("COLOR")
                )
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .items_start()
                        // Foreground picker
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.foreground.opacity(0.6))
                                        .child("Foreground")
                                )
                                .child(
                                    ColorPicker::new(fg_picker)
                                        .label("Foreground")
                                )
                        )
                        // Background picker
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.foreground.opacity(0.6))
                                        .child("Background")
                                )
                                .child(
                                    ColorPicker::new(bg_picker)
                                        .label("Background")
                                )
                        )
                )
        )
        // ── Brush picker ───────────────────────────────────────────────────
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .p_3()
                .gap_2()
                .border_t_1()
                .border_color(theme.border.opacity(0.5))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground.opacity(0.7))
                        .child("BRUSH")
                )
                // Dropdown — lists all loaded brushes with shape thumbnails.
                .child(
                    Dropdown::new(brush_dropdown)
                        .placeholder("Select brush…")
                        .menu_width(px(220.0))
                )
        )
        // ── Brush size slider ──────────────────────────────────────────────
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .px_3()
                .pb_3()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.foreground.opacity(0.6))
                                .child("Size")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.foreground.opacity(0.5))
                                .child(format!("{:.0}px", size_val))
                        )
                )
                .child(Slider::new(brush_size).horizontal())
                .child(
                    div().text_xs().text_color(theme.foreground.opacity(0.6))
                         .child(format!("Opacity: {:.0}%", brush_opacity * 100.0))
                )
        )
}
