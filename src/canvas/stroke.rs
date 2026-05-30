//! Brush stroke recording and rendering

use gpui::Point;
use std::collections::HashMap;

/// A recorded brush stroke
#[derive(Debug, Clone)]
pub struct Stroke {
    /// Points along the stroke path (canvas coordinates)
    pub points: Vec<Point<f32>>,
    
    /// Brush size at each point
    pub sizes: Vec<f32>,
    
    /// Brush opacity at each point (0.0-1.0)
    pub opacities: Vec<f32>,
    
    /// Brush color (RGBA)
    pub color: [u8; 4],
}

impl Stroke {
    /// Create a new empty stroke
    pub fn new(color: [u8; 4]) -> Self {
        Self {
            points: Vec::new(),
            sizes: Vec::new(),
            opacities: Vec::new(),
            color,
        }
    }
    
    /// Add a point to the stroke
    pub fn add_point(&mut self, point: Point<f32>, size: f32, opacity: f32) {
        self.points.push(point);
        self.sizes.push(size);
        self.opacities.push(opacity);
    }
    
    /// Check if stroke is empty
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
    
    /// Rasterize the stroke to dirty tiles
    /// Returns a map of (tile_x, tile_y) -> before/after tile data
    pub fn rasterize(&self, 
        layer_id: &str, 
        tile_loader: &dyn Fn(&str, u32, u32) -> Vec<u8>
    ) -> HashMap<(u32, u32), (Vec<u8>, Vec<u8>)> {
        let mut dirty_tiles: HashMap<(u32, u32), Vec<u8>> = HashMap::new();
        
        const TILE_SIZE: u32 = 256;
        
        // Rasterize each line segment in the stroke
        for i in 0..self.points.len() {
            let point = self.points[i];
            let size = self.sizes[i];
            let opacity = self.opacities[i];
            
            // Calculate brush stamp bounds
            let radius = size / 2.0;
            let min_x = (point.x - radius).max(0.0) as u32;
            let max_x = (point.x + radius) as u32;
            let min_y = (point.y - radius).max(0.0) as u32;
            let max_y = (point.y + radius) as u32;
            
            // Determine which tiles are affected
            let min_tile_x = min_x / TILE_SIZE;
            let max_tile_x = max_x / TILE_SIZE;
            let min_tile_y = min_y / TILE_SIZE;
            let max_tile_y = max_y / TILE_SIZE;
            
            // Render to each affected tile
            for tile_y in min_tile_y..=max_tile_y {
                for tile_x in min_tile_x..=max_tile_x {
                    // Get or load the tile
                    let tile_data = dirty_tiles.entry((tile_x, tile_y))
                        .or_insert_with(|| tile_loader(layer_id, tile_x, tile_y));
                    
                    // Render circular brush stamp to this tile
                    render_brush_to_tile(
                        tile_data,
                        point,
                        size,
                        opacity,
                        self.color,
                        tile_x * TILE_SIZE,
                        tile_y * TILE_SIZE,
                    );
                }
            }
        }
        
        // Convert to before/after format
        dirty_tiles.into_iter().map(|(coords, after)| {
            let before = tile_loader(layer_id, coords.0, coords.1);
            (coords, (before, after))
        }).collect()
    }
}

/// Render a circular brush stamp to a tile
fn render_brush_to_tile(
    tile_data: &mut [u8],
    center: Point<f32>,
    size: f32,
    opacity: f32,
    color: [u8; 4],
    tile_offset_x: u32,
    tile_offset_y: u32,
) {
    const TILE_SIZE: u32 = 256;
    let radius = size / 2.0;
    let radius_sq = radius * radius;
    
    // Calculate bounds within the tile
    let tile_min_x = (center.x - radius).max(tile_offset_x as f32) as u32;
    let tile_max_x = (center.x + radius).min((tile_offset_x + TILE_SIZE) as f32) as u32;
    let tile_min_y = (center.y - radius).max(tile_offset_y as f32) as u32;
    let tile_max_y = (center.y + radius).min((tile_offset_y + TILE_SIZE) as f32) as u32;
    
    for py in tile_min_y..tile_max_y {
        for px in tile_min_x..tile_max_x {
            // Distance from brush center
            let dx = px as f32 - center.x;
            let dy = py as f32 - center.y;
            let dist_sq = dx * dx + dy * dy;
            
            if dist_sq <= radius_sq {
                // Soft falloff
                let dist = dist_sq.sqrt();
                let falloff = 1.0 - (dist / radius);
                let alpha = (falloff * opacity * 255.0) as u8;
                
                // Convert canvas coords to tile-local coords
                let local_x = (px - tile_offset_x) as usize;
                let local_y = (py - tile_offset_y) as usize;
                let idx = (local_y * TILE_SIZE as usize + local_x) * 4;
                
                if idx + 3 < tile_data.len() {
                    // Alpha blend the brush color
                    let src_alpha = alpha as f32 / 255.0;
                    let dst_alpha = tile_data[idx + 3] as f32 / 255.0;
                    let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);
                    
                    if out_alpha > 0.0 {
                        for i in 0..3 {
                            let src = color[i] as f32;
                            let dst = tile_data[idx + i] as f32;
                            let blended = (src * src_alpha + dst * dst_alpha * (1.0 - src_alpha)) / out_alpha;
                            tile_data[idx + i] = blended.min(255.0) as u8;
                        }
                        tile_data[idx + 3] = (out_alpha * 255.0).min(255.0) as u8;
                    }
                }
            }
        }
    }
}
