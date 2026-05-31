//! Matter Editor Panel — main docking panel for the texture/material editor.

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;
use ui::{
    color_picker::{ColorPickerEvent, ColorPickerState},
    dock::{Panel, PanelEvent},
    dropdown::{DropdownEvent, DropdownState},
    slider::{SliderEvent, SliderState},
    ActiveTheme, IconName,
};

use crate::brush_engine::{BrushDropdownItem, BrushRegistry};
use crate::canvas::CanvasViewport;
use crate::panels::{render_layers_panel, render_properties_panel};
use crate::state::Document;
use crate::tools::ToolRegistry;
use crate::ui::toolbar::render_toolbar;

pub struct MatterEditorPanel {
    focus_handle:    FocusHandle,
    pub document:    Arc<RwLock<Document>>,
    _tools:          Arc<RwLock<ToolRegistry>>,
    canvas:          Entity<CanvasViewport>,
    fg_picker:       Entity<ColorPickerState>,
    bg_picker:       Entity<ColorPickerState>,
    brush_dropdown:  Entity<DropdownState<Vec<BrushDropdownItem>>>,
    brush_size:      Entity<SliderState>,
    _brush_registry: Arc<BrushRegistry>,
    _subs:           Vec<Subscription>,
}

impl MatterEditorPanel {
    /// Construct the panel.  Call from inside a `cx.open_window` callback so
    /// that `window` is available for `ColorPickerState::new`.
    /// Construct the panel.
    ///
    /// `brush_dropdown` and `brush_registry` must be created in the enclosing
    /// `cx.open_window` callback where a `Window` reference is available.
    pub fn new(
        document:       Document,
        fg_picker:      Entity<ColorPickerState>,
        bg_picker:      Entity<ColorPickerState>,
        brush_dropdown: Entity<DropdownState<Vec<BrushDropdownItem>>>,
        brush_registry: Arc<BrushRegistry>,
        cx:             &mut Context<Self>,
    ) -> Self {
        let doc_arc = Arc::new(RwLock::new(document));

        let mut tools = ToolRegistry::new();
        tools.register(Box::new(crate::tools::PaintTool::new()));
        tools.register(Box::new(crate::tools::EraserTool::new()));
        tools.register(Box::new(crate::tools::FillTool::new()));
        tools.register(Box::new(crate::tools::EyedropperTool::new()));
        tools.register(Box::new(crate::tools::HandTool::new()));

        let canvas = cx.new(|cx| {
            CanvasViewport::new(doc_arc.clone(), brush_registry.clone(), cx)
        });

        let mut subs = Vec::new();

        // Foreground colour picker → tool state
        {
            let doc = doc_arc.clone();
            subs.push(cx.subscribe(&fg_picker, move |_, _, ev: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(hsla)) = ev {
                    doc.write().tool_state.foreground_color = (*hsla).into();
                    cx.notify();
                }
            }));
        }

        // Background colour picker → tool state
        {
            let doc = doc_arc.clone();
            subs.push(cx.subscribe(&bg_picker, move |_, _, ev: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(hsla)) = ev {
                    doc.write().tool_state.background_color = (*hsla).into();
                    cx.notify();
                }
            }));
        }

        // Brush dropdown selection → tool state
        {
            let doc = doc_arc.clone();
            subs.push(cx.subscribe(
                &brush_dropdown,
                move |_, _, ev: &DropdownEvent<Vec<BrushDropdownItem>>, cx| {
                    if let DropdownEvent::Confirm(Some(id)) = ev {
                        doc.write().tool_state.active_brush_id = id.clone();
                        cx.notify();
                    }
                },
            ));
        }

        // Brush size slider (1–300 px, step 1)
        let brush_size = cx.new(|_| {
            SliderState::new()
                .min(1.0)
                .max(300.0)
                .step(1.0)
                .default_value(doc_arc.read().tool_state.brush_size)
        });
        {
            let doc = doc_arc.clone();
            subs.push(cx.subscribe(&brush_size, move |_, _, ev: &SliderEvent, cx| {
                if let SliderEvent::Change(v) = ev {
                    doc.write().tool_state.brush_size = v.end();
                    cx.notify();
                }
            }));
        }

        Self {
            focus_handle:    cx.focus_handle(),
            document:        doc_arc,
            _tools:          Arc::new(RwLock::new(tools)),
            canvas,
            fg_picker,
            bg_picker,
            brush_dropdown,
            brush_size,
            _brush_registry: brush_registry,
            _subs:           subs,
        }
    }
}

impl Panel for MatterEditorPanel {
    fn panel_name(&self) -> &'static str { "matter_editor" }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Matter Editor".into_any_element()
    }

    fn tab_icon(&self, _cx: &App) -> Option<IconName> {
        Some(IconName::EditPencil)
    }
}

impl EventEmitter<PanelEvent> for MatterEditorPanel {}

impl Focusable for MatterEditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle { self.focus_handle.clone() }
}

impl Render for MatterEditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child({
                let doc = self.document.read();
                render_toolbar(&doc, self.document.clone(), theme)
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(render_layers_panel(self.document.clone(), theme))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .relative()
                            .child(self.canvas.clone())
                    )
                    .child(render_properties_panel(
                        &self.document.read(),
                        &self.fg_picker,
                        &self.bg_picker,
                        &self.brush_dropdown,
                        &self.brush_size,
                        theme,
                    ))
            )
    }
}
