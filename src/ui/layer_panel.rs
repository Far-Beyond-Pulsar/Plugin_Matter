//! Layer panel component

use gpui::*;
use pulsar_image_format::model::Layer;

/// Render the layer panel
pub fn render_layer_panel<V>(
    layers: Vec<Layer>,
    _on_new_layer: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> impl IntoElement 
where
    V: 'static,
{
    let layer_list: Vec<(String, bool)> = layers
        .iter()
        .map(|layer| {
            let name = match layer {
                Layer::Raster { name, .. } => name.clone(),
                Layer::Vector { name, .. } => name.clone(),
            };
            let is_visible = match layer {
                Layer::Raster { visible, .. } => *visible,
                Layer::Vector { visible, .. } => *visible,
            };
            (name, is_visible)
        })
        .collect();
    
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
                .child("Layers")
        )
        .child(
            // New layer button
            div()
                .flex()
                .items_center()
                .justify_center()
                .h_8()
                .mb_2()
                .rounded_md()
                .bg(rgb(0x0078d4))
                .text_color(rgb(0xffffff))
                .text_sm()
                .hover(|this| this.bg(rgb(0x0066b8)))
                .child("+ New Layer")
        )
        .child(
            // Layer list
            div()
                .flex()
                .flex_col()
                .gap_1()
                .children(
                    layer_list.into_iter().map(|(name, is_visible)| {
                        div()
                            .flex()
                            .items_center()
                            .h_8()
                            .px_2()
                            .rounded_md()
                            .bg(rgb(0x2b2b2b))
                            .hover(|this| this.bg(rgb(0x3c3c3c)))
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(if is_visible { rgb(0xcccccc) } else { rgb(0x666666) })
                                    .child(name)
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .child(if is_visible { "👁" } else { "🚫" })
                            )
                    })
                )
        )
}
