//! Layers panel

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;
use ui::{button::Button, IconName, Theme};

use crate::state::{Document, commands::*};

pub fn render_layers_panel(
    document: Arc<RwLock<Document>>,
    theme: &Theme,
) -> impl IntoElement {
    let doc = document.read();
    let pif = doc.pif.lock();
    let manifest = pif.manifest();
    let layers = &manifest.layers;
    let layer_count = layers.len();
    
    let layers_vec: Vec<_> = layers.iter().cloned().collect();
    drop(pif);
    drop(doc);

    div()
        .flex()
        .flex_col()
        .w(px(250.0))
        .h_full()
        .bg(theme.sidebar)
        .border_r_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .w_full()
                .h(px(40.0))
                .px_2()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(theme.border.opacity(0.5))
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child("Layers")
                )
                .child({
                    let doc_clone = document.clone();
                    Button::new("add-layer")
                        .icon(IconName::Plus)
                        .tooltip("Add Layer")
                        .on_click(move |_event, _window, _cx| {
                            let mut doc = doc_clone.write();
                            let layer_id = format!("layer-{}", uuid::Uuid::new_v4());
                            let layer_name = format!("Layer {}", layer_count + 1);
                            
                            let command = CreateLayerCommand::new(
                                doc.pif.clone(),
                                layer_id,
                                layer_name,
                            );
                            
                            let _ = doc.history.execute(Box::new(command));
                        })
                })
        )
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .max_h_full()
                .children(layers_vec.iter().enumerate().map(|(idx, layer)| {
                    render_layer_item(document.clone(), layer, idx, theme)
                }))
        )
}

fn render_layer_item(
    document: Arc<RwLock<Document>>,
    layer: &pulsar_image_format::model::Layer,
    idx: usize,
    theme: &Theme,
) -> impl IntoElement {
    use pulsar_image_format::model::Layer;
    
    let (id, name, visible) = match layer {
        Layer::Raster { id, name, visible, .. } => (id.clone(), name.clone(), *visible),
        Layer::Vector { id, name, visible, .. } => (id.clone(), name.clone(), *visible),
    };

    div()
        .flex()
        .w_full()
        .h(px(36.0))
        .px_2()
        .gap_2()
        .items_center()
        .bg(theme.background)
        .border_b_1()
        .border_color(theme.border.opacity(0.3))
        .hover(|style| style.bg(theme.background.blend(theme.foreground.opacity(0.05))))
        .child({
            let doc_clone = document.clone();
            let layer_id = id.clone();
            Button::new(format!("layer-vis-{}", idx))
                .icon(if visible { IconName::Eye } else { IconName::EyeOff })
                .on_click(move |_event, _window, _cx| {
                    let mut doc = doc_clone.write();
                    let command = ToggleLayerVisibilityCommand::new(
                        doc.pif.clone(),
                        layer_id.clone(),
                    );
                    let _ = doc.history.execute(Box::new(command));
                })
        })
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme.foreground)
                .child(name)
        )
        .child({
            let doc_clone = document.clone();
            let layer_id = id.clone();
            Button::new(format!("layer-del-{}", idx))
                .icon(IconName::Trash)
                .on_click(move |_event, _window, _cx| {
                    let mut doc = doc_clone.write();
                    let command = DeleteLayerCommand::new(
                        doc.pif.clone(),
                        layer_id.clone(),
                    );
                    let _ = doc.history.execute(Box::new(command));
                })
        })
}
