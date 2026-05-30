//! Command implementations for undo/redo

use super::history::{Command, CommandResult, CommandError};
use pulsar_image_format::PifAssetManager;
use pulsar_image_format::model::Layer;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;

/// Paint stroke command — stores full before/after tile snapshots so the
/// stroke can be undone/redone without re-rasterizing.
pub struct PaintStrokeCommand {
    pif: Arc<Mutex<PifAssetManager>>,
    /// (layer_id, tile_x, tile_y) → pixel data
    tiles_before: HashMap<(String, u32, u32), Vec<u8>>,
    tiles_after:  HashMap<(String, u32, u32), Vec<u8>>,
}

impl PaintStrokeCommand {
    /// Construct from already-applied tile snapshots.
    /// The caller is responsible for having already written `tiles_after`
    /// to PIF via `commit_changes`; this command only handles undo/redo.
    pub fn from_tiles(
        pif: Arc<Mutex<PifAssetManager>>,
        tiles_before: HashMap<(String, u32, u32), Vec<u8>>,
        tiles_after:  HashMap<(String, u32, u32), Vec<u8>>,
    ) -> Self {
        Self { pif, tiles_before, tiles_after }
    }
}

impl std::fmt::Debug for PaintStrokeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintStrokeCommand")
            .field("tile_count", &self.tiles_after.len())
            .finish()
    }
}

impl Command for PaintStrokeCommand {
    fn execute(&mut self) -> CommandResult<()> {
        self.pif.lock()
            .commit_changes(self.tiles_after.clone())
            .map_err(|e| CommandError::ExecutionFailed(e.to_string()))
    }

    fn undo(&mut self) -> CommandResult<()> {
        self.pif.lock()
            .commit_changes(self.tiles_before.clone())
            .map_err(|e| CommandError::UndoFailed(e.to_string()))
    }

    fn description(&self) -> &str { "Paint Stroke" }
}

/// Create layer command
pub struct CreateLayerCommand {
    pif: Arc<Mutex<PifAssetManager>>,
    layer_id: String,
    layer_name: String,
}

impl CreateLayerCommand {
    pub fn new(
        pif: Arc<Mutex<PifAssetManager>>,
        layer_id: String,
        layer_name: String,
    ) -> Self {
        Self {
            pif,
            layer_id,
            layer_name,
        }
    }
}

impl std::fmt::Debug for CreateLayerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateLayerCommand")
            .field("layer_id", &self.layer_id)
            .field("layer_name", &self.layer_name)
            .finish()
    }
}

impl Command for CreateLayerCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock();
        
        let new_layer = Layer::Raster {
            id: self.layer_id.clone(),
            name: self.layer_name.clone(),
            visible: true,
            opacity: 1.0,
            blend_mode: "normal".to_string(),
            tile_size: 256,
            tiles: HashMap::new(),
        };
        
        pif.manifest_mut().layers.push(new_layer);
        Ok(())
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock();
        let manifest = pif.manifest_mut();
        
        if let Some(pos) = manifest.layers.iter().position(|l| {
            match l {
                Layer::Raster { id, .. } | Layer::Vector { id, .. } => id == &self.layer_id
            }
        }) {
            manifest.layers.remove(pos);
            Ok(())
        } else {
            Err(CommandError::UndoFailed("Layer not found".to_string()))
        }
    }
    
    fn description(&self) -> &str {
        "Create Layer"
    }
}

/// Delete layer command
pub struct DeleteLayerCommand {
    pif: Arc<Mutex<PifAssetManager>>,
    layer_id: String,
    layer_backup: Option<Layer>,
    layer_index: Option<usize>,
}

impl DeleteLayerCommand {
    pub fn new(pif: Arc<Mutex<PifAssetManager>>, layer_id: String) -> Self {
        Self {
            pif,
            layer_id,
            layer_backup: None,
            layer_index: None,
        }
    }
}

impl std::fmt::Debug for DeleteLayerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeleteLayerCommand")
            .field("layer_id", &self.layer_id)
            .finish()
    }
}

impl Command for DeleteLayerCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock();
        let manifest = pif.manifest_mut();
        
        if let Some(pos) = manifest.layers.iter().position(|l| {
            match l {
                Layer::Raster { id, .. } | Layer::Vector { id, .. } => id == &self.layer_id
            }
        }) {
            self.layer_backup = Some(manifest.layers.remove(pos));
            self.layer_index = Some(pos);
            Ok(())
        } else {
            Err(CommandError::ExecutionFailed("Layer not found".to_string()))
        }
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        if let (Some(layer), Some(index)) = (self.layer_backup.take(), self.layer_index) {
            let mut pif = self.pif.lock();
            pif.manifest_mut().layers.insert(index, layer);
            Ok(())
        } else {
            Err(CommandError::UndoFailed("No backup layer".to_string()))
        }
    }
    
    fn description(&self) -> &str {
        "Delete Layer"
    }
}

/// Toggle layer visibility command
pub struct ToggleLayerVisibilityCommand {
    pif: Arc<Mutex<PifAssetManager>>,
    layer_id: String,
}

impl ToggleLayerVisibilityCommand {
    pub fn new(pif: Arc<Mutex<PifAssetManager>>, layer_id: String) -> Self {
        Self { pif, layer_id }
    }
}

impl std::fmt::Debug for ToggleLayerVisibilityCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToggleLayerVisibilityCommand")
            .field("layer_id", &self.layer_id)
            .finish()
    }
}

impl Command for ToggleLayerVisibilityCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock();
        let manifest = pif.manifest_mut();
        
        for layer in &mut manifest.layers {
            match layer {
                Layer::Raster { id, visible, .. } | Layer::Vector { id, visible, .. } => {
                    if id == &self.layer_id {
                        *visible = !*visible;
                        return Ok(());
                    }
                }
            }
        }
        
        Err(CommandError::ExecutionFailed("Layer not found".to_string()))
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        // Toggle is its own inverse
        self.execute()
    }
    
    fn description(&self) -> &str {
        "Toggle Layer Visibility"
    }
}

/// Set layer opacity command
pub struct SetLayerOpacityCommand {
    pif: Arc<Mutex<PifAssetManager>>,
    layer_id: String,
    new_opacity: f32,
    old_opacity: Option<f32>,
}

impl SetLayerOpacityCommand {
    pub fn new(pif: Arc<Mutex<PifAssetManager>>, layer_id: String, opacity: f32) -> Self {
        Self {
            pif,
            layer_id,
            new_opacity: opacity,
            old_opacity: None,
        }
    }
}

impl std::fmt::Debug for SetLayerOpacityCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetLayerOpacityCommand")
            .field("layer_id", &self.layer_id)
            .field("opacity", &self.new_opacity)
            .finish()
    }
}

impl Command for SetLayerOpacityCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock();
        let manifest = pif.manifest_mut();
        
        for layer in &mut manifest.layers {
            match layer {
                Layer::Raster { id, opacity, .. } | Layer::Vector { id, opacity, .. } => {
                    if id == &self.layer_id {
                        self.old_opacity = Some(*opacity);
                        *opacity = self.new_opacity;
                        return Ok(());
                    }
                }
            }
        }
        
        Err(CommandError::ExecutionFailed("Layer not found".to_string()))
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        if let Some(old) = self.old_opacity {
            let mut pif = self.pif.lock();
            let manifest = pif.manifest_mut();
            
            for layer in &mut manifest.layers {
                match layer {
                    Layer::Raster { id, opacity, .. } | Layer::Vector { id, opacity, .. } => {
                        if id == &self.layer_id {
                            *opacity = old;
                            return Ok(());
                        }
                    }
                }
            }
        }
        
        Err(CommandError::UndoFailed("No old opacity stored".to_string()))
    }
    
    fn description(&self) -> &str {
        "Set Layer Opacity"
    }
}

/// Reorder layers command
pub struct ReorderLayersCommand {
    pif: Arc<Mutex<PifAssetManager>>,
    from_index: usize,
    to_index: usize,
}

impl ReorderLayersCommand {
    pub fn new(pif: Arc<Mutex<PifAssetManager>>, from: usize, to: usize) -> Self {
        Self {
            pif,
            from_index: from,
            to_index: to,
        }
    }
}

impl std::fmt::Debug for ReorderLayersCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReorderLayersCommand")
            .field("from", &self.from_index)
            .field("to", &self.to_index)
            .finish()
    }
}

impl Command for ReorderLayersCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock();
        let layers = &mut pif.manifest_mut().layers;
        
        if self.from_index >= layers.len() || self.to_index >= layers.len() {
            return Err(CommandError::ExecutionFailed("Invalid layer index".to_string()));
        }
        
        let layer = layers.remove(self.from_index);
        layers.insert(self.to_index, layer);
        Ok(())
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        // Swap indices and execute
        std::mem::swap(&mut self.from_index, &mut self.to_index);
        let result = self.execute();
        std::mem::swap(&mut self.from_index, &mut self.to_index);
        result
    }
    
    fn description(&self) -> &str {
        "Reorder Layers"
    }
}
