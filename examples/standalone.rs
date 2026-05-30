//! Standalone test of Matter editor panel

use gpui::{App, AppContext, Application, Bounds, size, px, WindowBounds, WindowOptions};
use plugin_matter::{MatterEditorPanel, Document};

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1400.), px(900.)), cx);
        
        // Create a new 1024x768 document
        let document = Document::new(1024, 768).expect("Failed to create document");
        
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Matter - Texture Editor".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| MatterEditorPanel::new(document)),
        )
        .expect("Failed to open window");
    });
}
