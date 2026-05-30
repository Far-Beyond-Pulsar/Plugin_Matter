//! Layers panel

use gpui::{prelude::FluentBuilder as _, *};
use parking_lot::RwLock;
use std::sync::Arc;
use ui::{button::Button, IconName, Theme};

use crate::state::{Document, commands::*};

pub fn render_layers_panel(
    document: Arc<RwLock<Document>>,
    theme: &Theme,
) -> impl IntoElement {
    let doc         = document.read();
    let active_id   = doc.active_layer().map(str::to_string);
    let pif         = doc.pif.lock();
    let manifest    = pif.manifest();
    let layer_count = manifest.layers.len();
    let layers_vec: Vec<_> = manifest.layers.iter().cloned().collect();
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
        // ── Header ────────────────────────────────────────────────────────
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
                            let layer_id   = format!("layer-{}", uuid::Uuid::new_v4());
                            let layer_name = format!("Layer {}", layer_count + 1);
                            let command    = CreateLayerCommand::new(
                                doc.pif.clone(), layer_id.clone(), layer_name,
                            );
                            if doc.history.execute(Box::new(command)).is_ok() {
                                doc.set_active_layer(layer_id);
                            }
                        })
                })
        )
        // ── Layer list ─────────────────────────────────────────────────────
        .child(
            div()
                .flex()
                .flex_col()
                .w_full()
                .flex_1()
                .overflow_hidden()
                .children(layers_vec.iter().enumerate().map(|(idx, layer)| {
                    let is_active = active_id.as_deref() == Some({
                        use pulsar_image_format::model::Layer;
                        match layer {
                            Layer::Raster { id, .. } | Layer::Vector { id, .. } => id.as_str(),
                        }
                    });
                    render_layer_item(document.clone(), layer, idx, is_active, theme)
                }))
        )
}

fn render_layer_item(
    document:  Arc<RwLock<Document>>,
    layer:     &pulsar_image_format::model::Layer,
    idx:       usize,
    is_active: bool,
    theme:     &Theme,
) -> impl IntoElement {
    use pulsar_image_format::model::Layer;

    let (id, name, visible) = match layer {
        Layer::Raster { id, name, visible, .. } => (id.clone(), name.clone(), *visible),
        Layer::Vector { id, name, visible, .. } => (id.clone(), name.clone(), *visible),
    };

    // Active layer: accent-coloured left border + slightly lighter background.
    let bg = if is_active {
        theme.accent.opacity(0.15)
    } else {
        theme.background
    };

    let row = div()
        .flex()
        .w_full()
        .h(px(36.0))
        .px_2()
        .gap_2()
        .items_center()
        .bg(bg)
        .border_b_1()
        .border_color(theme.border.opacity(0.3))
        .when(is_active, |s| {
            s.border_l_2().border_color(theme.accent)
        })
        .hover(|s| s.bg(theme.background.blend(theme.foreground.opacity(0.06))))
        // Click anywhere on the row to make this the active layer.
        .on_mouse_down(MouseButton::Left, {
            let doc_clone = document.clone();
            let layer_id  = id.clone();
            move |_ev, _win, _cx| {
                doc_clone.write().set_active_layer(layer_id.clone());
            }
        });

    row
        // Visibility toggle
        .child({
            let doc_clone = document.clone();
            let layer_id  = id.clone();
            Button::new(format!("layer-vis-{}", idx))
                .icon(if visible { IconName::Eye } else { IconName::EyeOff })
                .on_click(move |_ev, _win, _cx| {
                    let mut doc = doc_clone.write();
                    let cmd = ToggleLayerVisibilityCommand::new(doc.pif.clone(), layer_id.clone());
                    let _ = doc.history.execute(Box::new(cmd));
                })
        })
        // Layer name
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(if is_active { theme.accent } else { theme.foreground })
                .font_weight(if is_active { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .child(name)
        )
        // Delete button
        .child({
            let doc_clone = document.clone();
            let layer_id  = id.clone();
            Button::new(format!("layer-del-{}", idx))
                .icon(IconName::Trash)
                .on_click(move |_ev, _win, _cx| {
                    let mut doc = doc_clone.write();
                    let cmd = DeleteLayerCommand::new(doc.pif.clone(), layer_id.clone());
                    let _ = doc.history.execute(Box::new(cmd));
                })
        })
}
