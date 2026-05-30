//! Standalone test of Matter editor panel

use gpui::*;
use plugin_matter::{MatterEditorPanel, state::Document};

fn main() {
    Application::new().run(|cx: &mut App| {
        // Initialize UI theme system
        ui::init(cx);
        
        let document = Document::new(1024, 768).expect("Failed to create document");
        
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point { x: px(100.0), y: px(100.0) },
                    size: Size { width: px(1200.0), height: px(800.0) },
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("Matter Editor - Standalone".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| MatterEditorPanel::new(document, cx)),
        )
        .expect("Failed to open window");
    });
}
