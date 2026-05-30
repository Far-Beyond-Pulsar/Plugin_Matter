//! Color panel component

use gpui::*;

/// Render the color panel
pub fn render_color_panel(fg_color: Rgba, bg_color: Rgba) -> impl IntoElement {
    let fg_bytes = [
        (fg_color.r * 255.0) as u8,
        (fg_color.g * 255.0) as u8,
        (fg_color.b * 255.0) as u8,
    ];
    let bg_bytes = [
        (bg_color.r * 255.0) as u8,
        (bg_color.g * 255.0) as u8,
        (bg_color.b * 255.0) as u8,
    ];
    
    let fg_rgb = rgb(
        ((fg_bytes[0] as u32) << 16) | 
        ((fg_bytes[1] as u32) << 8) | 
        (fg_bytes[2] as u32)
    );
    let bg_rgb = rgb(
        ((bg_bytes[0] as u32) << 16) | 
        ((bg_bytes[1] as u32) << 8) | 
        (bg_bytes[2] as u32)
    );
    
    div()
        .size_full()
        .flex()
        .flex_col()
        .p_2()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xcccccc))
                .mb_2()
                .child("Colors")
        )
        .child(
            // Color swatches
            div()
                .flex()
                .gap_2()
                .mb_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x999999))
                                .child("Foreground")
                        )
                        .child(
                            div()
                                .w_16()
                                .h_16()
                                .rounded_md()
                                .bg(fg_rgb)
                                .border_2()
                                .border_color(rgb(0xcccccc))
                        )
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x999999))
                                .child("Background")
                        )
                        .child(
                            div()
                                .w_16()
                                .h_16()
                                .rounded_md()
                                .bg(bg_rgb)
                                .border_2()
                                .border_color(rgb(0x666666))
                        )
                )
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x999999))
                .child(format!(
                    "RGB: {}, {}, {}",
                    fg_bytes[0], fg_bytes[1], fg_bytes[2]
                ))
        )
}
