//! Brush stamp rasteriser — wet-buffer model with bilinear mask sampling.
//!
//! # How professional apps paint
//!
//! Naïvely compositing each stamp directly onto the destination tile causes
//! overlapping stamps to accumulate opacity beyond the user's chosen setting —
//! the stroke gets darker wherever it loops back on itself.
//!
//! The solution used by Photoshop, Krita, Procreate, and Clip Studio is a
//! **per-stroke wet buffer**:
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │  tiles_before  (snapshot at stroke start)    │
//! │       +                                      │
//! │  wet_tile      (accumulated stamps, capped)  │
//! │       ↓ composite each frame                 │
//! │  tiles_current (what's displayed and saved)  │
//! └──────────────────────────────────────────────┘
//! ```
//!
//! Each stamp writes into `wet_tile` (not directly to the destination).
//! A pixel in `wet_tile` can never exceed the stroke's `opacity` ceiling.
//! `tiles_current` is then rebuilt from `tiles_before + wet_tile` on every
//! stamp, so the displayed result is always correct regardless of overlap.
//!
//! # Rotation convention
//! The mask's V = 0 edge (top of the SVG/PNG) is the brush's *tip* direction.
//! We rotate by `angle + π/2` in UV space so the tip faces the stroke direction.
//!
//! # Sampling
//! Bilinear interpolation is used when reading the mask to avoid staircase
//! aliasing on rotated, detailed, or asymmetric brush shapes.

use std::f32::consts::FRAC_PI_2;

use gpui::Point;

use super::mask::BrushMask;

const TILE: u32 = 256;
const TILE_SZ: usize = TILE as usize;

/// Paint one stamp into a `wet_tile` buffer.
///
/// The wet tile accumulates paint from every stamp in the stroke.  Its alpha
/// is capped at `opacity` so overlapping stamps never darken beyond that limit.
///
/// # Parameters
/// - `wet_tile`      — 256×256 RGBA8 wet-buffer; starts fully transparent.
/// - `mask`          — brush mask, sampled with bilinear interpolation.
/// - `center`        — stamp centre in canvas-space pixels.
/// - `size`          — stamp diameter in canvas pixels.
/// - `angle`         — stroke direction in radians (0 = right, π/2 = down).
/// - `flow`          — per-stamp paint deposition (0–1).  Lower values mean
///                     each individual stamp is lighter; opacity is the ceiling.
/// - `opacity`       — maximum alpha any wet-buffer pixel may reach (0–1).
/// - `color`         — RGBA8 paint colour.
/// - `tile_offset_*` — canvas-space top-left corner of this tile.
pub fn stamp_into_wet(
    wet_tile:      &mut [u8],
    mask:          &BrushMask,
    center:        Point<f32>,
    size:          f32,
    angle:         f32,
    flow:          f32,
    opacity:       f32,
    color:         [u8; 4],
    tile_offset_x: u32,
    tile_offset_y: u32,
) {
    let radius = size / 2.0;

    // Rotation: tip (V=0) aligns with stroke direction.
    let a     = angle + FRAC_PI_2;
    let cos_a = a.cos();
    let sin_a = a.sin();

    let px_min = (center.x - radius).floor().max(tile_offset_x as f32) as u32;
    let px_max = ((center.x + radius).ceil() as u32).min(tile_offset_x + TILE);
    let py_min = (center.y - radius).floor().max(tile_offset_y as f32) as u32;
    let py_max = ((center.y + radius).ceil() as u32).min(tile_offset_y + TILE);

    for py in py_min..py_max {
        for px in px_min..px_max {
            let dx = px as f32 + 0.5 - center.x;
            let dy = py as f32 + 0.5 - center.y;

            // Rotate into brush-local UV space (bilinear sample).
            let dx_l =  dx * cos_a + dy * sin_a;
            let dy_l = -dx * sin_a + dy * cos_a;
            let u = (dx_l / size + 0.5).clamp(0.0, 1.0);
            let v = (dy_l / size + 0.5).clamp(0.0, 1.0);

            let ch = mask.sample(u, v);
            // Skip fully outside the mask.
            if ch.roughness < 1.0 / 255.0 && ch.intensity < 1.0 / 255.0 {
                continue;
            }

            let src_a = (ch.intensity * flow).clamp(0.0, 1.0);
            if src_a < 1.0 / 255.0 { continue; }

            let lx  = (px - tile_offset_x) as usize;
            let ly  = (py - tile_offset_y) as usize;
            let idx = (ly * TILE_SZ + lx) * 4;
            if idx + 3 >= wet_tile.len() { continue; }

            let dst_a = wet_tile[idx + 3] as f32 / 255.0;
            let raw_a = src_a + dst_a * (1.0 - src_a);

            // ── Opacity cap: the accumulated wet alpha never exceeds the
            //   stroke's opacity setting, no matter how many stamps overlap.
            let out_a = raw_a.min(opacity);

            if out_a > dst_a + 1.0 / 255.0 {
                // Blend colour channels proportionally to the new alpha.
                let new_contrib = out_a - dst_a; // alpha this stamp actually adds
                for i in 0..3 {
                    let src_c = color[i] as f32 / 255.0;
                    let dst_c = wet_tile[idx + i] as f32 / 255.0;
                    // Weighted average: new colour contribution at the margin.
                    let out_c = if out_a > 0.0 {
                        (dst_c * dst_a + src_c * new_contrib) / out_a
                    } else {
                        dst_c
                    };
                    wet_tile[idx + i] = (out_c * 255.0).min(255.0).round() as u8;
                }
                wet_tile[idx + 3] = (out_a * 255.0).min(255.0).round() as u8;
            }
        }
    }
}

/// Composite a wet buffer over a base tile (Porter-Duff src-over).
///
/// Called after every stamp to rebuild `out_tile` from the unmodified
/// `base_tile` snapshot and the accumulated `wet_tile`.  This guarantees
/// the displayed result reflects the correct opacity ceiling even as stamps
/// accumulate.
pub fn composite_wet_over_base(
    out_tile:  &mut [u8],
    base_tile: &[u8],
    wet_tile:  &[u8],
) {
    let n = out_tile.len().min(base_tile.len()).min(wet_tile.len());
    let pixels = n / 4;
    for i in 0..pixels {
        let b = i * 4;
        let wet_a = wet_tile[b + 3] as f32 / 255.0;
        if wet_a < 1.0 / 255.0 {
            // No paint here — pass the base through unchanged.
            out_tile[b]     = base_tile[b];
            out_tile[b + 1] = base_tile[b + 1];
            out_tile[b + 2] = base_tile[b + 2];
            out_tile[b + 3] = base_tile[b + 3];
            continue;
        }
        let base_a = base_tile[b + 3] as f32 / 255.0;
        let out_a  = wet_a + base_a * (1.0 - wet_a);
        if out_a > 0.0 {
            for c in 0..3 {
                let wc = wet_tile[b + c]  as f32 / 255.0;
                let bc = base_tile[b + c] as f32 / 255.0;
                let oc = (wc * wet_a + bc * base_a * (1.0 - wet_a)) / out_a;
                out_tile[b + c] = (oc * 255.0).min(255.0).round() as u8;
            }
        }
        out_tile[b + 3] = (out_a * 255.0).min(255.0).round() as u8;
    }
}

/// Apply an erase wet buffer to a base tile.
///
/// `wet_tile`'s alpha encodes how much to erase; base colour is preserved
/// (matching Photoshop's eraser behaviour on non-background layers).
pub fn composite_wet_erase(
    out_tile:  &mut [u8],
    base_tile: &[u8],
    wet_tile:  &[u8],
) {
    let n = out_tile.len().min(base_tile.len()).min(wet_tile.len());
    let pixels = n / 4;
    for i in 0..pixels {
        let b = i * 4;
        let erase = wet_tile[b + 3] as f32 / 255.0;
        let base_a = base_tile[b + 3] as f32 / 255.0;
        let new_a  = (base_a * (1.0 - erase)).max(0.0);
        out_tile[b]     = base_tile[b];
        out_tile[b + 1] = base_tile[b + 1];
        out_tile[b + 2] = base_tile[b + 2];
        out_tile[b + 3] = (new_a * 255.0).round() as u8;
        if new_a == 0.0 {
            out_tile[b] = 0;
            out_tile[b + 1] = 0;
            out_tile[b + 2] = 0;
        }
    }
}
