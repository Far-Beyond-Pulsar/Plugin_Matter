//! Matter Editor Panel

use gpui::*;
use parking_lot::RwLock;
use std::sync::Arc;
use ui::{dock::{Panel, PanelEvent}, IconName, ActiveTheme};

use crate::state::Document;
use crate::ui::toolbar::render_toolbar;
use crate::panels::{render_layers_panel, render_properties_panel};
use crate::canvas::render_canvas;
use crate::tools::ToolRegistry;

pub struct MatterEditorPanel {
    focus_handle: FocusHandle,
    pub document: Arc<RwLock<Document>>,
    tools: Arc<RwLock<ToolRegistry>>,
}

impl MatterEditorPanel {
    pub fn new(document: Document, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();
        
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(crate::tools::PaintTool::new()));
        tools.register(Box::new(crate::tools::EraserTool::new()));
        tools.register(Box::new(crate::tools::FillTool::new()));
        tools.register(Box::new(crate::tools::EyedropperTool::new()));
        tools.register(Box::new(crate::tools::HandTool::new()));
        
        Self {
            focus_handle,
            document: Arc::new(RwLock::new(document)),
            tools: Arc::new(RwLock::new(tools)),
        }
    }
}

impl Panel for MatterEditorPanel {
    fn panel_name(&self) -> &'static str {
        "matter_editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Matter Editor".into_any_element()
    }
    
    fn tab_icon(&self, _cx: &App) -> Option<IconName> {
        Some(IconName::EditPencil)
    }
}

impl EventEmitter<PanelEvent> for MatterEditorPanel {}

impl Focusable for MatterEditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MatterEditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let doc = self.document.read();
        let theme = cx.theme();
        
        div()
            .size_full()
            .bg(theme.background)
            .child(render_toolbar(&*doc, theme))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(render_layers_panel(self.document.clone(), theme))
                    .child(render_canvas(self.document.clone(), self.tools.clone(), theme))
                    .child(render_properties_panel(&*doc, theme))
            )
    }
}
