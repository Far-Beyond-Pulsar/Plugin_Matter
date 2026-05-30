//! Document state wrapping PIF asset manager

use pulsar_image_format::{PifAssetManager, SaveMode};
use pulsar_image_format::model::Layer;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
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

/// Main document state
pub struct Document {
    /// Path to the PIF file
    path: Option<PathBuf>,
    
    /// PIF asset manager (wrapped for thread-safe command access)
    pub pif: Arc<Mutex<PifAssetManager>>,
    
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
            pif: Arc::new(Mutex::new(pif)),
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
            pif: Arc::new(Mutex::new(pif)),
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
    pub fn pif_handle(&self) -> Arc<Mutex<PifAssetManager>> {
        Arc::clone(&self.pif)
    }
    
    /// Get canvas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        let pif = self.pif.lock();
        let canvas = &pif.manifest().canvas;
        (canvas.width, canvas.height)
    }
    
    /// Get the list of layers (cloned for rendering)
    pub fn layers(&self) -> Vec<Layer> {
        let pif = self.pif.lock();
        pif.manifest().layers.clone()
    }
    
    /// Get active layer ID
    pub fn active_layer(&self) -> Option<&str> {
        self.active_layer.as_deref()
    }
    
    /// Set active layer
    pub fn set_active_layer(&mut self, layer_id: String) {
        self.active_layer = Some(layer_id);
    }
    
    /// Add a new raster layer (direct, without command)
    /// Use CreateLayerCommand for undoable version
    pub fn add_raster_layer(&mut self, name: String) -> String {
        let id = format!("layer_{}", uuid::Uuid::new_v4());
        
        let mut pif = self.pif.lock();
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
        let pif = self.pif.lock();
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
