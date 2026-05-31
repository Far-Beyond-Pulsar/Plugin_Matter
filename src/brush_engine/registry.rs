//! `BrushRegistry` — discovers, loads, and caches all `.pbrush` bundles.
//!
//! On startup the registry scans a `brushes/` directory for `.pbrush` folders
//! and falls back to the embedded built-in brushes when the directory is absent
//! or a bundle fails to parse.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{AnyElement, IntoElement, ParentElement, SharedString, Styled, div, px, rgba};
use ui::dropdown::DropdownItem;

use super::config::{BrushConfig, BrushShape};
use super::loader::load_pbrush;
use super::mask::BrushMask;
use crate::brush_engine::builtin::builtin_brushes;

/// A single loaded brush: its config, rasterised mask, and originating path.
#[derive(Clone)]
pub struct BrushEntry {
    /// Stable identifier derived from the `.pbrush` directory stem.
    pub id:     String,
    /// Parsed config from `config.json`.
    pub config: BrushConfig,
    /// Pre-rasterised mask — shared across clones.
    pub mask:   Arc<BrushMask>,
    /// Source directory on disk (`None` for built-in brushes).
    pub path:   Option<PathBuf>,
}

/// Holds all discovered brushes in display order.
pub struct BrushRegistry {
    pub brushes: Vec<BrushEntry>,
}

impl BrushRegistry {
    /// Scan `brushes_dir` for `.pbrush` sub-folders and merge with built-ins.
    ///
    /// Built-in brushes are always present and appear first.  User brushes from
    /// disk are appended in alphabetical order by display name.
    pub fn load_from_dir(brushes_dir: &Path) -> Self {
        let mut brushes = builtin_brushes();

        if let Ok(entries) = std::fs::read_dir(brushes_dir) {
            let mut disk_brushes: Vec<BrushEntry> = entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        == Some("pbrush")
                })
                .filter_map(|e| {
                    let path = e.path();
                    match load_pbrush(&path) {
                        Ok((config, mask)) => {
                            let id = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            // Skip if a built-in with the same id already exists.
                            if brushes.iter().any(|b| b.id == id) {
                                return None;
                            }
                            Some(BrushEntry {
                                id,
                                config,
                                mask: Arc::new(mask),
                                path: Some(path),
                            })
                        }
                        Err(e) => {
                            tracing::warn!("failed to load brush {:?}: {e}", path);
                            None
                        }
                    }
                })
                .collect();

            disk_brushes.sort_by(|a, b| a.config.name.cmp(&b.config.name));
            brushes.extend(disk_brushes);
        }

        Self { brushes }
    }

    /// Find a brush by its stable `id`.
    pub fn get(&self, id: &str) -> Option<&BrushEntry> {
        self.brushes.iter().find(|b| b.id == id)
    }

    /// Returns the first brush, which is the default selection.
    pub fn default_brush(&self) -> Option<&BrushEntry> {
        self.brushes.first()
    }

    /// Build the list of items consumed by the picker `Dropdown`.
    pub fn dropdown_items(&self) -> Vec<BrushDropdownItem> {
        self.brushes.iter().map(BrushDropdownItem::from_entry).collect()
    }
}

// ── Dropdown integration ──────────────────────────────────────────────────────

/// A `DropdownItem` that wraps a brush entry for the WGPUI `Dropdown` widget.
#[derive(Clone)]
pub struct BrushDropdownItem {
    /// Stable brush identifier (the `Dropdown`'s value type).
    pub id:    String,
    /// Display name.
    pub name:  String,
    /// Coarse shape for the preview thumbnail.
    pub shape: BrushShape,
}

impl BrushDropdownItem {
    pub fn from_entry(e: &BrushEntry) -> Self {
        Self {
            id:    e.id.clone(),
            name:  e.config.name.clone(),
            shape: e.config.shape.clone(),
        }
    }
}

impl DropdownItem for BrushDropdownItem {
    type Value = String;

    fn title(&self) -> SharedString {
        SharedString::from(self.name.clone())
    }

    /// Renders a small shape thumbnail beside the brush name.
    ///
    /// No GPUI context is available here, so we use hardcoded neutral colours.
    fn display_title(&self) -> Option<AnyElement> {
        let thumb = match self.shape {
            BrushShape::Circle => div()
                .w(px(18.0))
                .h(px(18.0))
                .rounded_full()
                .bg(rgba(0xd0d0d0ff)),
            BrushShape::Square => div()
                .w(px(18.0))
                .h(px(18.0))
                .rounded(px(2.0))
                .bg(rgba(0xd0d0d0ff)),
            BrushShape::Bristle => div()
                .w(px(18.0))
                .h(px(18.0))
                .flex()
                .gap(px(2.0))
                .child(div().w(px(3.0)).h_full().rounded(px(1.0)).bg(rgba(0xd0d0d0ff)))
                .child(div().w(px(3.0)).h_full().rounded(px(1.0)).bg(rgba(0xd0d0d0ff)))
                .child(div().w(px(3.0)).h_full().rounded(px(1.0)).bg(rgba(0xd0d0d0ff)))
                .child(div().w(px(3.0)).h_full().rounded(px(1.0)).bg(rgba(0xc0c0c0ff))),
            BrushShape::Custom => div()
                .w(px(18.0))
                .h(px(18.0))
                .rounded(px(4.0))
                .bg(rgba(0xb0b0b0ff)),
        };

        Some(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(thumb)
                .child(div().child(self.name.clone()))
                .into_any_element(),
        )
    }

    fn value(&self) -> &String {
        &self.id
    }

    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(&query.to_lowercase())
    }
}

// `Vec<T: DropdownItem>` already implements `DropdownDelegate` via the blanket
// impl in `ui::dropdown`.  No additional impl is needed here.
