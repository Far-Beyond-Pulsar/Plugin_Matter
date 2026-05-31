//! Embedded built-in brush definitions.
//!
//! SVG data and config JSON for all built-in brushes are compiled into the
//! binary with `include_str!` / `include_bytes!`.  This ensures the brushes
//! are available even when the `brushes/` directory is absent or inaccessible.

use std::sync::Arc;

use super::config::{BrushConfig, BrushShape};
use super::loader::rasterise_svg_bytes;
use super::mask::BrushMask;
use super::registry::BrushEntry;

// ── Embedded assets ───────────────────────────────────────────────────────────

const ROUND_SOFT_SVG:  &[u8] = include_bytes!("../../brushes/round_soft.pbrush/mask.svg");
const ROUND_HARD_SVG:  &[u8] = include_bytes!("../../brushes/round_hard.pbrush/mask.svg");
const SQUARE_FLAT_SVG: &[u8] = include_bytes!("../../brushes/square_flat.pbrush/mask.svg");
const TEXTURE_SVG:     &[u8] = include_bytes!("../../brushes/texture_rough.pbrush/mask.svg");
const BRISTLE_SVG:     &[u8] = include_bytes!("../../brushes/bristle_soft.pbrush/mask.svg");

// ── Builder helpers ───────────────────────────────────────────────────────────

struct BuiltinDef {
    id:    &'static str,
    svg:   &'static [u8],
    config: BrushConfig,
}

fn make_config(
    name:        &str,
    description: &str,
    shape:       BrushShape,
    spacing:     f32,
    hardness:    f32,
    size:        f32,
    opacity:     f32,
) -> BrushConfig {
    BrushConfig {
        name:          name.to_string(),
        mask_file:     "mask.svg".to_string(),
        shape,
        description:   description.to_string(),
        spacing,
        hardness,
        default_size:    size,
        default_opacity: opacity,
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Return all built-in `BrushEntry` instances.
///
/// SVGs are rasterised once at call time.  Entries whose SVG fails to parse are
/// logged and replaced with a procedurally generated fallback.
pub fn builtin_brushes() -> Vec<BrushEntry> {
    let defs: &[BuiltinDef] = &[
        BuiltinDef {
            id:  "round_soft",
            svg: ROUND_SOFT_SVG,
            config: make_config(
                "Round Soft",
                "Soft airbrush-style round stamp with smooth radial falloff.",
                BrushShape::Circle,
                0.20, 0.4, 20.0, 1.0,
            ),
        },
        BuiltinDef {
            id:  "round_hard",
            svg: ROUND_HARD_SVG,
            config: make_config(
                "Round Hard",
                "Hard-edged round stamp — uniform coverage with a thin antialiased edge.",
                BrushShape::Circle,
                0.15, 1.0, 20.0, 1.0,
            ),
        },
        BuiltinDef {
            id:  "square_flat",
            svg: SQUARE_FLAT_SVG,
            config: make_config(
                "Square Flat",
                "Flat square stamp — full coverage across the entire brush footprint.",
                BrushShape::Square,
                0.15, 1.0, 20.0, 1.0,
            ),
        },
        BuiltinDef {
            id:  "texture_rough",
            svg: TEXTURE_SVG,
            config: make_config(
                "Texture Rough",
                "Rough, gritty stamp — overlapping circles emulate natural media texture.",
                BrushShape::Circle,
                0.30, 0.6, 30.0, 0.85,
            ),
        },
        BuiltinDef {
            id:  "bristle_soft",
            svg: BRISTLE_SVG,
            config: make_config(
                "Bristle Soft",
                "Six parallel bristle strands with soft tapered tips.",
                BrushShape::Bristle,
                0.10, 0.7, 24.0, 0.9,
            ),
        },
    ];

    defs.iter()
        .map(|def| {
            let mask = match rasterise_svg_bytes(def.svg) {
                Ok(m)  => Arc::new(m),
                Err(e) => {
                    tracing::warn!("builtin brush '{}' SVG parse failed: {e}", def.id);
                    Arc::new(BrushMask::default_round())
                }
            };
            BrushEntry {
                id:     def.id.to_string(),
                config: def.config.clone(),
                mask,
                path:   None,
            }
        })
        .collect()
}
