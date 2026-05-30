//! Matter Editor Panel — main docking panel for the texture/material editor.

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;
use ui::{
    color_picker::{ColorPickerEvent, ColorPickerState},
    dock::{Panel, PanelEvent},
    ActiveTheme, IconName,
};

use crate::canvas::CanvasViewport;
use crate::panels::{render_layers_panel, render_properties_panel};
use crate::state::Document;
use crate::tools::ToolRegistry;
use crate::ui::toolbar::render_toolbar;

pub struct MatterEditorPanel {
    focus_handle: FocusHandle,
    pub document: Arc<RwLock<Document>>,
    _tools:       Arc<RwLock<ToolRegistry>>,
    canvas:       Entity<CanvasViewport>,
    fg_picker:    Entity<ColorPickerState>,
    bg_picker:    Entity<ColorPickerState>,
    _subs:        Vec<Subscription>,
}

impl MatterEditorPanel {
    /// Construct the panel.  Call from inside a `cx.open_window` callback so
    /// that `window` is available for `ColorPickerState::new`.
    pub fn new(
        document:  Document,
        fg_picker: Entity<ColorPickerState>,
        bg_picker: Entity<ColorPickerState>,
        cx:        &mut Context<Self>,
    ) -> Self {
        let doc_arc = Arc::new(RwLock::new(document));

        let mut tools = ToolRegistry::new();
        tools.register(Box::new(crate::tools::PaintTool::new()));
        tools.register(Box::new(crate::tools::EraserTool::new()));
        tools.register(Box::new(crate::tools::FillTool::new()));
        tools.register(Box::new(crate::tools::EyedropperTool::new()));
        tools.register(Box::new(crate::tools::HandTool::new()));

        let canvas = cx.new(|cx| CanvasViewport::new(doc_arc.clone(), cx));

        let mut subs = Vec::new();
        {
            let doc = doc_arc.clone();
            subs.push(cx.subscribe(&fg_picker, move |_this, _picker, ev: &ColorPickerEvent, _cx| {
                if let ColorPickerEvent::Change(Some(hsla)) = ev {
                    let rgba: Rgba = (*hsla).into();
                    doc.write().tool_state.foreground_color = rgba;
                }
            }));
        }
        {
            let doc = doc_arc.clone();
            subs.push(cx.subscribe(&bg_picker, move |_this, _picker, ev: &ColorPickerEvent, _cx| {
                if let ColorPickerEvent::Change(Some(hsla)) = ev {
                    let rgba: Rgba = (*hsla).into();
                    doc.write().tool_state.background_color = rgba;
                }
            }));
        }

        Self {
            focus_handle: cx.focus_handle(),
            document:     doc_arc,
            _tools:       Arc::new(RwLock::new(tools)),
            canvas,
            fg_picker,
            bg_picker,
            _subs: subs,
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
                        theme,
                    ))
            )
    }
}
