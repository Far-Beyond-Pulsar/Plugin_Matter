//! Brush configuration — the `config.json` inside every `.pbrush` bundle.

use serde::{Deserialize, Serialize};

/// Coarse shape category used to render a preview thumbnail in the picker UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrushShape {
    /// Circular footprint (round soft, round hard, etc.)
    Circle,
    /// Square / rectangular footprint.
    Square,
    /// Multiple thin parallel strands (fan brush, bristle, etc.)
    Bristle,
    /// Anything that does not fit the above categories.
    Custom,
}

impl Default for BrushShape {
    fn default() -> Self { Self::Circle }
}

/// All parameters stored in a brush bundle's `config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrushConfig {
    /// Display name shown in the brush picker.
    pub name: String,

    /// File name of the mask image relative to the `.pbrush` directory.
    pub mask_file: String,

    /// Coarse shape hint for the picker preview thumbnail.
    #[serde(default)]
    pub shape: BrushShape,

    /// Short human-readable description (shown as tooltip or subtitle).
    #[serde(default)]
    pub description: String,

    /// Fractional stamp spacing (0–1 as a multiple of brush size).
    /// 0.15 = place a new stamp every 15 % of the brush diameter.
    #[serde(default = "default_spacing")]
    pub spacing: f32,

    /// Edge softness hint (0.0 = feathered, 1.0 = hard edge).
    /// Currently informational; the actual falloff is encoded in the mask.
    #[serde(default = "default_hardness")]
    pub hardness: f32,

    /// Suggested default brush size in canvas pixels.
    #[serde(default = "default_size")]
    pub default_size: f32,

    /// Suggested default opacity (0.0–1.0).
    #[serde(default = "default_opacity")]
    pub default_opacity: f32,
}

fn default_spacing()  -> f32 { 0.20 }
fn default_hardness() -> f32 { 0.50 }
fn default_size()     -> f32 { 20.0 }
fn default_opacity()  -> f32 { 1.0  }
