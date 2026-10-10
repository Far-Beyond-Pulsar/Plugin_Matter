//! Document state wrapping PIF asset manager

use pulsar_image_format::{PifAssetManager, SaveMode};
use pulsar_image_format::model::Layer;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::{Mutex, MutexGuard};
use thiserror::Error;

use super::history::History;
use super::tool_state::ToolState;
use super::viewport::ViewportState;

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("PIF error: {0}")]
    Pif(#[from] pulsar_image_format::PifError),
    #[error("Layer not found: {0}")]
    LayerNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DocumentError>;

// ── Shared PIF handle ─────────────────────────────────────────────────────────

/// PIF asset manager shared by the document and its undo commands, paired
/// with the document revision.
///
/// [`read`](Self::read) gives shared access only; the sole way to mutate is
/// [`write`](Self::write), whose guard bumps the revision when it drops. Any
/// edit, direct or through a history command, therefore moves the revision.
#[derive(Clone)]
pub struct SharedPif(Arc<PifCell>);

struct PifCell {
    manager:  Mutex<PifAssetManager>,
    revision: AtomicU64,
}

impl SharedPif {
    pub fn new(manager: PifAssetManager) -> Self {
        Self(Arc::new(PifCell {
            manager:  Mutex::new(manager),
            revision: AtomicU64::new(0),
        }))
    }

    /// Lock for reading. Does not move the revision.
    pub fn read(&self) -> PifRead<'_> {
        PifRead(self.0.manager.lock())
    }

    /// Lock for writing. The revision is bumped when the guard drops.
    pub fn write(&self) -> PifWrite<'_> {
        PifWrite { guard: self.0.manager.lock(), revision: &self.0.revision }
    }

    /// Monotonic count of writes (and other document edits) so far.
    pub fn revision(&self) -> u64 {
        self.0.revision.load(Ordering::Acquire)
    }

    /// Record an edit to document state held outside the PIF manager.
    fn bump(&self) {
        self.0.revision.fetch_add(1, Ordering::AcqRel);
    }
}

/// Shared access to the PIF manager.
pub struct PifRead<'a>(MutexGuard<'a, PifAssetManager>);

impl Deref for PifRead<'_> {
    type Target = PifAssetManager;
    fn deref(&self) -> &PifAssetManager { &self.0 }
}

/// Exclusive access to the PIF manager; bumps the revision on drop.
pub struct PifWrite<'a> {
    guard:    MutexGuard<'a, PifAssetManager>,
    revision: &'a AtomicU64,
}

impl Deref for PifWrite<'_> {
    type Target = PifAssetManager;
    fn deref(&self) -> &PifAssetManager { &self.guard }
}

impl DerefMut for PifWrite<'_> {
    fn deref_mut(&mut self) -> &mut PifAssetManager { &mut self.guard }
}

impl Drop for PifWrite<'_> {
    fn drop(&mut self) {
        // Runs before `guard` unlocks, so a reader that sees the new revision
        // also sees the new content.
        self.revision.fetch_add(1, Ordering::AcqRel);
    }
}

/// Main document state
pub struct Document {
    /// Path to the PIF file
    path: Option<PathBuf>,
    
    /// PIF asset manager, shared with commands; carries the document revision
    pub pif: SharedPif,
    
    /// Active layer ID
    active_layer: Option<String>,
    
    /// Document has unsaved changes
    is_dirty: bool,
    
    /// Undo/redo history
    pub history: History,
    
    /// Tool state
    pub tool_state: ToolState,
    
    /// Viewport state
    pub viewport: ViewportState,
}

impl Document {
    /// Create a new empty document
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let temp_path = std::env::temp_dir().join(format!("matter_temp_{}.pif", uuid::Uuid::new_v4()));
        let pif = PifAssetManager::create_new(&temp_path, width, height, SaveMode::Directory)?;
        
        let (_width, _height) = (width, height);
        
        let mut doc = Self {
            path: None,
            pif: SharedPif::new(pif),
            active_layer: None,
            is_dirty: false,
            history: History::default(),
            tool_state: ToolState::default(),
            viewport: ViewportState::new(),
        };
        
        // Add a default background layer
        let layer_id = doc.add_raster_layer("Background".to_string());
        doc.active_layer = Some(layer_id);
        
        Ok(doc)
    }
    
    /// Open an existing document from a file
    pub fn open(path: PathBuf) -> Result<Self> {
        let pif = PifAssetManager::open(&path)?;
        let active_layer = pif.manifest().layers.first().map(|l| l.id().to_string());
        let (_width, _height) = {
            let canvas = &pif.manifest().canvas;
            (canvas.width, canvas.height)
        };
        
        Ok(Self {
            path: Some(path),
            pif: SharedPif::new(pif),
            active_layer,
            is_dirty: false,
            history: History::default(),
            tool_state: ToolState::default(),
            viewport: ViewportState::new(),
        })
    }
    
    /// Save the document
    pub fn save(&mut self) -> Result<()> {
        if let Some(_path) = &self.path {
            // PIF auto-saves on commit_changes, we just mark as clean
            self.is_dirty = false;
            Ok(())
        } else {
            Err(DocumentError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No file path set, use save_as instead",
            )))
        }
    }
    
    /// Save the document to a new path
    pub fn save_as(&mut self, path: PathBuf) -> Result<()> {
        self.path = Some(path);
        self.save()
    }
    
    /// Get a clone of the PIF handle for commands
    pub fn pif_handle(&self) -> SharedPif {
        self.pif.clone()
    }

    /// Content revision: bumped by every change to pixels, layers or the
    /// active layer, whether made here or by a command through the PIF
    /// handle. Save bookkeeping (path, dirty flag) and editor state
    /// (`tool_state`, `viewport`) are not content and leave it alone.
    pub fn revision(&self) -> u64 {
        self.pif.revision()
    }
    
    /// Get canvas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        let pif = self.pif.read();
        let canvas = &pif.manifest().canvas;
        (canvas.width, canvas.height)
    }
    
    /// Get the list of layers (cloned for rendering)
    pub fn layers(&self) -> Vec<Layer> {
        let pif = self.pif.read();
        pif.manifest().layers.clone()
    }
    
    /// Get active layer ID
    pub fn active_layer(&self) -> Option<&str> {
        self.active_layer.as_deref()
    }
    
    /// Set active layer
    pub fn set_active_layer(&mut self, layer_id: String) {
        if self.active_layer.as_deref() != Some(layer_id.as_str()) {
            self.active_layer = Some(layer_id);
            self.pif.bump();
        }
    }
    
    /// Add a new raster layer (direct, without command)
    /// Use CreateLayerCommand for undoable version
    pub fn add_raster_layer(&mut self, name: String) -> String {
        let id = format!("layer_{}", uuid::Uuid::new_v4());
        
        let mut pif = self.pif.write();
        pif.manifest_mut().layers.push(Layer::Raster {
            id: id.clone(),
            name,
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        });
        
        self.is_dirty = true;
        id
    }
    
    /// Load a tile from a layer
    pub fn load_tile(&self, layer_id: &str, tile_x: u32, tile_y: u32) -> Result<Vec<u8>> {
        let pif = self.pif.read();
        Ok(pif.load_raster_tile(layer_id, tile_x, tile_y)?)
    }
    
    /// Check if document has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
    
    /// Mark document as dirty
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }
    
    /// Get file path
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    
    /// Get filename for display
    pub fn filename(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::commands::*;
    use crate::state::history::Command;

    /// Document backed by a PIF bundle in a scratch directory removed on drop.
    struct TestDoc {
        doc: Document,
        dir: PathBuf,
    }

    impl TestDoc {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("matter_rev_test_{}", uuid::Uuid::new_v4()));
            let pif = PifAssetManager::create_new(dir.join("doc.pif"), 512, 512, SaveMode::Directory).unwrap();
            let mut doc = Document {
                path: None,
                pif: SharedPif::new(pif),
                active_layer: None,
                is_dirty: false,
                history: History::default(),
                tool_state: ToolState::default(),
                viewport: ViewportState::new(),
            };
            let id = doc.add_raster_layer("Background".to_string());
            doc.set_active_layer(id);
            Self { doc, dir }
        }
    }

    impl Drop for TestDoc {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Asserts `f` moves the revision forward.
    fn bumps(doc: &mut Document, f: impl FnOnce(&mut Document)) {
        let before = doc.revision();
        f(doc);
        assert!(doc.revision() > before, "revision did not move");
    }

    /// Asserts `f` leaves the revision alone.
    fn keeps(doc: &mut Document, f: impl FnOnce(&mut Document)) {
        let before = doc.revision();
        f(doc);
        assert_eq!(doc.revision(), before, "revision moved");
    }

    #[test]
    fn test_pif_write_guard_bumps_on_drop() {
        let t = TestDoc::new();
        let pif = t.doc.pif_handle();
        let before = pif.revision();

        drop(pif.read());
        assert_eq!(pif.revision(), before);

        let guard = pif.write();
        drop(guard);
        assert_eq!(pif.revision(), before + 1);
        // Clones share one counter.
        assert_eq!(t.doc.revision(), before + 1);
    }

    #[test]
    fn test_document_mutations_bump_revision() {
        let mut t = TestDoc::new();
        let doc = &mut t.doc;

        let mut second = String::new();
        bumps(doc, |d| second = d.add_raster_layer("Second".to_string()));
        bumps(doc, |d| d.set_active_layer(second.clone()));
        // Re-selecting the active layer is not a change.
        keeps(doc, |d| d.set_active_layer(second.clone()));
        bumps(doc, |d| {
            let mut tiles = HashMap::new();
            tiles.insert((second.clone(), 0, 0), vec![255u8; 256 * 256 * 4]);
            d.pif.write().commit_changes(tiles).unwrap();
        });
    }

    #[test]
    fn test_reads_and_bookkeeping_keep_revision() {
        let mut t = TestDoc::new();
        let doc = &mut t.doc;
        let id = doc.active_layer().unwrap().to_string();

        keeps(doc, |d| {
            let _ = d.dimensions();
            let _ = d.layers();
            let _ = d.active_layer();
            let _ = d.load_tile(&id, 0, 0);
            let _ = d.pif.read().manifest().layers.len();
            let _ = d.is_dirty();
            let _ = d.path();
            let _ = d.filename();
            let _ = d.history.can_undo();
        });
        // Save bookkeeping and editor state are not document content.
        keeps(doc, |d| {
            d.mark_dirty();
            d.tool_state.brush_size = 42.0;
            d.viewport.zoom = 2.0;
        });
    }

    #[test]
    fn test_commands_bump_revision_on_execute_undo_redo() {
        let mut t = TestDoc::new();
        let doc = &mut t.doc;
        let base = doc.active_layer().unwrap().to_string();
        let pif = doc.pif_handle();

        let tile = |v: u8| {
            let mut m = HashMap::new();
            m.insert((base.clone(), 0, 0), vec![v; 256 * 256 * 4]);
            m
        };
        let commands: Vec<Box<dyn Command>> = vec![
            Box::new(CreateLayerCommand::new(pif.clone(), "extra".into(), "Extra".into())),
            Box::new(ToggleLayerVisibilityCommand::new(pif.clone(), base.clone())),
            Box::new(SetLayerOpacityCommand::new(pif.clone(), base.clone(), 0.5)),
            Box::new(ReorderLayersCommand::new(pif.clone(), 0, 1)),
            Box::new(PaintStrokeCommand::from_tiles(pif.clone(), tile(0), tile(255))),
            Box::new(DeleteLayerCommand::new(pif.clone(), "extra".into())),
        ];
        for cmd in commands {
            bumps(doc, |d| d.history.execute(cmd).unwrap());
            bumps(doc, |d| d.history.undo().unwrap());
            bumps(doc, |d| d.history.redo().unwrap());
        }
    }
}
