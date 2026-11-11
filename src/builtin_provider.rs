//! The plugin's editor provider: its file types and editors, offered to
//! the editor while the plugin is enabled (`plugin_manager::PluginEditorProvider`,
//! registered by the root's `build`).

use gpui::AppContext;
use gpui::{App, Window};
use plugin_editor_api::*;
use plugin_manager::{BuiltinEditorProvider, EditorContext};
use std::path::PathBuf;
use std::sync::Arc;
use ui::dock::PanelView;

/// Opens `.pif` files in the Matter texture-painting editor.
pub struct MatterEditorBuiltinProvider;

impl BuiltinEditorProvider for MatterEditorBuiltinProvider {
    fn provider_id(&self) -> &str {
        "com.pulsar.matter-editor"
    }

    fn file_types(&self) -> Vec<FileTypeDefinition> {
        // PIF assets are directory-backed bundles named `*.pif`.
        // Register as FolderBased so `Path::is_dir()` resolution still maps them
        // to the Matter editor during open-path lookup.
        vec![FileTypeDefinition {
            id: FileTypeId::new("pif"),
            extension: "pif".to_string(),
            display_name: "Pulsar Image Format".to_string(),
            icon: ui::IconName::EditPencil,
            color: gpui::rgb(0xE91E63).into(),
            structure: FileStructure::FolderBased {
                marker_file: "manifest.json".to_string(),
                template_structure: vec![PathTemplate::Folder {
                    path: ".raster".into(),
                }],
            },
            default_content: serde_json::json!({
                "pif_version": "1.0",
                "canvas": {
                    "width": 1024,
                    "height": 1024,
                    "color_space": "sRGB"
                },
                "layers": [
                    {
                        "type": "raster",
                        "id": "background",
                        "name": "Background",
                        "visible": true,
                        "opacity": 1.0,
                        "blend_mode": "normal",
                        "tile_size": 256,
                        "tiles": {}
                    }
                ]
            }),
            creation_directory: None,

            categories: vec!["Textures".to_string()],
        }]
    }

    fn editors(&self) -> Vec<EditorMetadata> {
        vec![EditorMetadata {
            id: EditorId::new("matter-editor"),
            display_name: "Matter Editor".into(),
            supported_file_types: vec![FileTypeId::new("pif")],
        }]
    }

    fn can_handle(&self, editor_id: &EditorId) -> bool {
        editor_id.as_str() == "matter-editor"
    }

    fn create_editor(
        &self,
        file_path: PathBuf,
        editor_context: &EditorContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Arc<dyn PanelView>, PluginError> {
        use crate::brush_engine::{BrushDropdownItem, BrushRegistry};
        use crate::state::Document;
        use gpui::Rgba;
        use ui::{color_picker::ColorPickerState, dropdown::DropdownState, IndexPath};

        let document = if engine_fs::virtual_fs::exists(&file_path).unwrap_or(false) {
            Document::open(file_path.clone()).map_err(|e| PluginError::Other {
                message: format!("Failed to open PIF file {:?}: {}", file_path, e),
            })?
        } else {
            Document::new(1024, 1024).map_err(|e| PluginError::Other {
                message: format!("Failed to create new PIF document: {}", e),
            })?
        };

        let fg = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                Rgba {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }
                .into(),
            )
        });
        let bg = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                Rgba {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                }
                .into(),
            )
        });

        let brushes_dir = std::env::current_dir().unwrap_or_default().join("brushes");
        let brush_registry = Arc::new(BrushRegistry::load_from_dir(&brushes_dir));
        let items: Vec<BrushDropdownItem> = brush_registry.dropdown_items();

        let active_brush_id = document.tool_state.active_brush_id.clone();
        let initial = items
            .iter()
            .position(|item| item.id == active_brush_id)
            .map(|idx| IndexPath::default().row(idx))
            .or_else(|| (!items.is_empty()).then(|| IndexPath::default().row(0)));

        let brush_dropdown = cx.new(|cx| DropdownState::new(items, initial, window, cx));
        let panel = cx.new(|cx| {
            crate::MatterEditorPanel::new(
                document,
                fg,
                bg,
                brush_dropdown,
                brush_registry.clone(),
                cx,
            )
        });
        let undo = editor_context.services.undo.clone();
        panel.update(cx, |panel, _| panel.set_undo_service(undo));
        Ok(Arc::new(panel))
    }
}

/// Registers the editor provider.
pub(crate) fn build(p: &mut pulsar_plugins::PluginBuilder) {
    p.add(plugin_manager::PluginEditorProvider {
        origin: p.origin(),
        id: "com.pulsar.matter-editor",
        create: || Arc::new(MatterEditorBuiltinProvider),
    });
}
