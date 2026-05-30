//! Properties panel

use gpui::*;
use ui::Theme;

use crate::state::Document;

pub fn render_properties_panel(
    doc: &Document,
    theme: &Theme,
) -> impl IntoElement {
    let tool_state = &doc.tool_state;
    let fg = tool_state.foreground_color;
    let bg = tool_state.background_color;

    div()
        .flex()
        .flex_col()
        .w(px(250.0))
        .h_full()
        .bg(theme.sidebar)
        .border_l_1()
        .border_color(theme.border)
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
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(theme.foreground.opacity(0.6)).child("Foreground"))
                                .child(
                                    div()
                                        .w(px(60.0))
                                        .h(px(60.0))
                                        .rounded_md()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(rgb(
                                            ((fg.r * 255.0) as u32) << 16 |
                                            ((fg.g * 255.0) as u32) << 8 |
                                            (fg.b * 255.0) as u32
                                        ))
                                )
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(theme.foreground.opacity(0.6)).child("Background"))
                                .child(
                                    div()
                                        .w(px(60.0))
                                        .h(px(60.0))
                                        .rounded_md()
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(rgb(
                                            ((bg.r * 255.0) as u32) << 16 |
                                            ((bg.g * 255.0) as u32) << 8 |
                                            (bg.b * 255.0) as u32
                                        ))
                                )
                        )
                )
        )
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .p_3()
                .gap_2()
                .border_t_1()
                .border_color(theme.border.opacity(0.5))
                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(theme.foreground.opacity(0.7)).child("BRUSH"))
                .child(div().text_xs().text_color(theme.foreground.opacity(0.6)).child(format!("Size: {:.0}px", tool_state.brush_size)))
                .child(div().text_xs().text_color(theme.foreground.opacity(0.6)).child(format!("Opacity: {:.0}%", tool_state.brush_opacity * 100.0)))
        )
}
