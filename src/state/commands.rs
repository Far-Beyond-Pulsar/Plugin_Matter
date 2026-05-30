//! Concrete command implementations for all document operations

use super::history::{Command, CommandError, CommandResult};
use pulsar_image_format::model::Layer;
use pulsar_image_format::PifAssetManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fmt;

/// Command to paint a brush stroke on a raster layer
pub struct PaintStrokeCommand {
    /// Layer ID to paint on
    layer_id: String,
    
    /// Dirty tiles with before/after state: (tile_x, tile_y) -> (before, after)
    tiles: HashMap<(u32, u32), (Vec<u8>, Vec<u8>)>,
    
    /// Reference to the PIF asset manager
    pif: Arc<Mutex<PifAssetManager>>,
    
    /// Description of the stroke
    description: String,
}

impl fmt::Debug for PaintStrokeCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PaintStrokeCommand")
            .field("layer_id", &self.layer_id)
            .field("tiles", &self.tiles.len())
            .field("description", &self.description)
            .finish()
    }
}

impl PaintStrokeCommand {
    pub fn new(
        layer_id: String,
        tiles: HashMap<(u32, u32), (Vec<u8>, Vec<u8>)>,
        pif: Arc<Mutex<PifAssetManager>>,
    ) -> Self {
        let tile_count = tiles.len();
        Self {
            layer_id,
            tiles,
            pif,
            description: format!("Paint Stroke ({} tiles)", tile_count),
        }
    }
}

impl Command for PaintStrokeCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let mut dirty_tiles = HashMap::new();
        
        for ((tile_x, tile_y), (_before, after)) in &self.tiles {
            dirty_tiles.insert(
                (self.layer_id.clone(), *tile_x, *tile_y),
                after.clone(),
            );
        }
        
        pif.commit_changes(dirty_tiles)
            .map_err(|e| CommandError::ExecutionFailed(e.to_string()))
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let mut dirty_tiles = HashMap::new();
        
        for ((tile_x, tile_y), (before, _after)) in &self.tiles {
            dirty_tiles.insert(
                (self.layer_id.clone(), *tile_x, *tile_y),
                before.clone(),
            );
        }
        
        pif.commit_changes(dirty_tiles)
            .map_err(|e| CommandError::UndoFailed(e.to_string()))
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

/// Command to create a new layer
pub struct CreateLayerCommand {
    /// The layer to create
    layer: Option<Layer>,
    
    /// Position to insert at
    position: usize,
    
    /// Reference to the PIF asset manager
    pif: Arc<Mutex<PifAssetManager>>,
}

impl fmt::Debug for CreateLayerCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateLayerCommand")
            .field("position", &self.position)
            .finish()
    }
}

impl CreateLayerCommand {
    pub fn new(layer: Layer, position: usize, pif: Arc<Mutex<PifAssetManager>>) -> Self {
        Self {
            layer: Some(layer),
            position,
            pif,
        }
    }
}

impl Command for CreateLayerCommand {
    fn execute(&mut self) -> CommandResult<()> {
        if let Some(layer) = self.layer.take() {
            let mut pif = self.pif.lock().unwrap();
            let manifest = pif.manifest_mut();
            
            if self.position <= manifest.layers.len() {
                manifest.layers.insert(self.position, layer);
                Ok(())
            } else {
                Err(CommandError::ExecutionFailed("Invalid layer position".to_string()))
            }
        } else {
            Err(CommandError::ExecutionFailed("Layer already executed".to_string()))
        }
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let manifest = pif.manifest_mut();
        
        if self.position < manifest.layers.len() {
            let layer = manifest.layers.remove(self.position);
            self.layer = Some(layer);
            Ok(())
        } else {
            Err(CommandError::UndoFailed("Layer not found".to_string()))
        }
    }
    
    fn description(&self) -> &str {
        "Create Layer"
    }
}

/// Command to delete a layer
pub struct DeleteLayerCommand {
    /// The deleted layer (stored for undo)
    layer: Option<Layer>,
    
    /// Position the layer was at
    position: usize,
    
    /// Reference to the PIF asset manager
    pif: Arc<Mutex<PifAssetManager>>,
}

impl fmt::Debug for DeleteLayerCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeleteLayerCommand")
            .field("position", &self.position)
            .finish()
    }
}

impl DeleteLayerCommand {
    pub fn new(position: usize, pif: Arc<Mutex<PifAssetManager>>) -> Self {
        Self {
            layer: None,
            position,
            pif,
        }
    }
}

impl Command for DeleteLayerCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let manifest = pif.manifest_mut();
        
        if self.position < manifest.layers.len() {
            let layer = manifest.layers.remove(self.position);
            self.layer = Some(layer);
            Ok(())
        } else {
            Err(CommandError::ExecutionFailed("Layer not found".to_string()))
        }
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        if let Some(layer) = self.layer.take() {
            let mut pif = self.pif.lock().unwrap();
            let manifest = pif.manifest_mut();
            
            if self.position <= manifest.layers.len() {
                manifest.layers.insert(self.position, layer);
                Ok(())
            } else {
                Err(CommandError::UndoFailed("Invalid layer position".to_string()))
            }
        } else {
            Err(CommandError::UndoFailed("No layer to restore".to_string()))
        }
    }
    
    fn description(&self) -> &str {
        "Delete Layer"
    }
}

/// Command to toggle layer visibility
pub struct ToggleLayerVisibilityCommand {
    /// Layer ID to toggle
    layer_id: String,
    
    /// Reference to the PIF asset manager
    pif: Arc<Mutex<PifAssetManager>>,
}

impl fmt::Debug for ToggleLayerVisibilityCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToggleLayerVisibilityCommand")
            .field("layer_id", &self.layer_id)
            .finish()
    }
}

impl ToggleLayerVisibilityCommand {
    pub fn new(layer_id: String, pif: Arc<Mutex<PifAssetManager>>) -> Self {
        Self { layer_id, pif }
    }
}

impl Command for ToggleLayerVisibilityCommand {
    fn execute(&mut self) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let manifest = pif.manifest_mut();
        
        for layer in &mut manifest.layers {
            let id = match layer {
                Layer::Raster { id, .. } => id,
                Layer::Vector { id, .. } => id,
            };
            
            if id == &self.layer_id {
                match layer {
                    Layer::Raster { visible, .. } => *visible = !*visible,
                    Layer::Vector { visible, .. } => *visible = !*visible,
                }
                return Ok(());
            }
        }
        
        Err(CommandError::ExecutionFailed(format!("Layer {} not found", self.layer_id)))
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        // Toggling is symmetric, so undo is the same as execute
        self.execute()
    }
    
    fn description(&self) -> &str {
        "Toggle Layer Visibility"
    }
}

/// Command to set layer opacity
pub struct SetLayerOpacityCommand {
    /// Layer ID to modify
    layer_id: String,
    
    /// Old opacity (for undo)
    old_opacity: f32,
    
    /// New opacity
    new_opacity: f32,
    
    /// Reference to the PIF asset manager
    pif: Arc<Mutex<PifAssetManager>>,
}

impl fmt::Debug for SetLayerOpacityCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetLayerOpacityCommand")
            .field("layer_id", &self.layer_id)
            .field("old_opacity", &self.old_opacity)
            .field("new_opacity", &self.new_opacity)
            .finish()
    }
}

impl SetLayerOpacityCommand {
    pub fn new(layer_id: String, old_opacity: f32, new_opacity: f32, pif: Arc<Mutex<PifAssetManager>>) -> Self {
        Self {
            layer_id,
            old_opacity,
            new_opacity,
            pif,
        }
    }
    
    fn set_opacity(&self, opacity: f32) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let manifest = pif.manifest_mut();
        
        for layer in &mut manifest.layers {
            let id = match layer {
                Layer::Raster { id, .. } => id,
                Layer::Vector { id, .. } => id,
            };
            
            if id == &self.layer_id {
                match layer {
                    Layer::Raster { opacity: o, .. } => *o = opacity,
                    Layer::Vector { opacity: o, .. } => *o = opacity,
                }
                return Ok(());
            }
        }
        
        Err(CommandError::ExecutionFailed(format!("Layer {} not found", self.layer_id)))
    }
}

impl Command for SetLayerOpacityCommand {
    fn execute(&mut self) -> CommandResult<()> {
        self.set_opacity(self.new_opacity)
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        self.set_opacity(self.old_opacity)
    }
    
    fn description(&self) -> &str {
        "Set Layer Opacity"
    }
}

/// Command to reorder layers
pub struct ReorderLayersCommand {
    /// Old position
    from: usize,
    
    /// New position
    to: usize,
    
    /// Reference to the PIF asset manager
    pif: Arc<Mutex<PifAssetManager>>,
}

impl fmt::Debug for ReorderLayersCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReorderLayersCommand")
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

impl ReorderLayersCommand {
    pub fn new(from: usize, to: usize, pif: Arc<Mutex<PifAssetManager>>) -> Self {
        Self { from, to, pif }
    }
    
    fn move_layer(&self, from: usize, to: usize) -> CommandResult<()> {
        let mut pif = self.pif.lock().unwrap();
        let manifest = pif.manifest_mut();
        
        if from >= manifest.layers.len() || to >= manifest.layers.len() {
            return Err(CommandError::ExecutionFailed("Invalid layer positions".to_string()));
        }
        
        let layer = manifest.layers.remove(from);
        manifest.layers.insert(to, layer);
        Ok(())
    }
}

impl Command for ReorderLayersCommand {
    fn execute(&mut self) -> CommandResult<()> {
        self.move_layer(self.from, self.to)
    }
    
    fn undo(&mut self) -> CommandResult<()> {
        self.move_layer(self.to, self.from)
    }
    
    fn description(&self) -> &str {
        "Reorder Layers"
    }
}
