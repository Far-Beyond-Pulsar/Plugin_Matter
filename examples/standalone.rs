//! Standalone test of the Matter editor panel.

use std::sync::Arc;

use gpui::*;
use plugin_matter::{
    brush_engine::{BrushRegistry, BrushDropdownItem},
    MatterEditorPanel,
    state::Document,
};
use ui::{color_picker::ColorPickerState, dropdown::DropdownState, Assets, IndexPath, Root, Theme, ThemeMode};

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        // 1. Initialise component registry (buttons, inputs, colour pickers, …)
        ui::init(cx);
        // 2. Load themes and apply dark mode.
        ui::themes::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);

        let document = Document::new(2048, 2048).expect("failed to create document");

        // Discover brushes relative to the working directory (PROJ_ROOT/brushes/).
        let brushes_dir = std::env::current_dir()
            .unwrap_or_default()
            .join("brushes");
        let brush_registry = Arc::new(BrushRegistry::load_from_dir(&brushes_dir));

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
                // Entities that require a Window are constructed here.

                let fg = cx.new(|cx| {
                    ColorPickerState::new(window, cx)
                        .default_value(Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }.into())
                });
                let bg = cx.new(|cx| {
                    ColorPickerState::new(window, cx)
                        .default_value(Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }.into())
                });

                // Build the dropdown items and pre-select the first brush.
                let items: Vec<BrushDropdownItem> = brush_registry.dropdown_items();
                let initial = if items.is_empty() { None } else { Some(IndexPath::default().row(0)) };

                let brush_dropdown = cx.new(|cx| {
                    DropdownState::new(items, initial, window, cx)
                });

                let panel = cx.new(|cx| {
                    MatterEditorPanel::new(
                        document,
                        fg,
                        bg,
                        brush_dropdown,
                        brush_registry.clone(),
                        cx,
                    )
                });

                // ui::Root is required for popups/modals opened by ui:: components.
                cx.new(|cx| Root::new(panel.into(), window, cx))
            },
        )
        .expect("failed to open window");
    });
}
