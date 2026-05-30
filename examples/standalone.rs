//! Standalone test of Matter editor panel

use gpui::*;
use plugin_matter::{MatterEditorPanel, state::Document};
use ui::{color_picker::ColorPickerState, Theme, ThemeMode};

fn main() {
    Application::new().run(|cx: &mut App| {
        // 1. Initialise component registry (buttons, inputs, color pickers…)
        ui::init(cx);
        // 2. Load themes from disk + apply the saved/default "Default Dark" theme.
        //    This is the same two-step sequence the engine uses and ensures icon
        //    fonts and palette resources are fully registered before any window opens.
        ui::themes::init(cx);
        // 3. Force dark mode regardless of system appearance.
        Theme::change(ThemeMode::Dark, None, cx);

        let document = Document::new(1024, 768).expect("Failed to create document");

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point { x: px(100.0), y: px(100.0) },
                    size:   Size  { width: px(1400.0), height: px(900.0) },
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("Matter Editor".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                // ColorPickerState::new requires a Window — construct here
                // inside the open_window callback where Window is available.
                let fg = cx.new(|cx| {
                    ColorPickerState::new(window, cx)
                        .default_value(Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }.into())
                });
                let bg = cx.new(|cx| {
                    ColorPickerState::new(window, cx)
                        .default_value(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }.into())
                });
                cx.new(|cx| MatterEditorPanel::new(document, fg, bg, cx))
            },
        )
        .expect("Failed to open window");
    });
}
