//! `.pbrush` bundle loader.
//!
//! A bundle is a directory whose name ends in `.pbrush` and which contains:
//! - `config.json` — serialised [`BrushConfig`]
//! - A mask image referenced by `config.mask_file` (`.svg` or `.png`)
//!
//! The loader is intentionally synchronous; brushes are loaded once at startup
//! and cached in the [`BrushRegistry`](super::registry::BrushRegistry).

use anyhow::{Context, Result, bail};
use std::path::Path;

use super::config::BrushConfig;
use super::mask::BrushMask;

/// Load a `.pbrush` directory and return its parsed config plus rasterised mask.
pub fn load_pbrush(dir: &Path) -> Result<(BrushConfig, BrushMask)> {
    let config_path = dir.join("config.json");
    let json = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let config: BrushConfig = serde_json::from_str(&json)
        .with_context(|| format!("parsing {}", config_path.display()))?;

    let mask_path = dir.join(&config.mask_file);
    let ext = mask_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mask = match ext.as_str() {
        "png" => load_png(&mask_path)?,
        "svg" => load_svg(&mask_path)?,
        other => bail!("unsupported brush mask format '.{other}' in {}", mask_path.display()),
    };

    Ok((config, mask))
}

// ── PNG ───────────────────────────────────────────────────────────────────────

fn load_png(path: &Path) -> Result<BrushMask> {
    let img = image::open(path)
        .with_context(|| format!("decoding PNG {}", path.display()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    Ok(BrushMask::from_rgba(img.into_raw(), w, h))
}

// ── SVG ───────────────────────────────────────────────────────────────────────

/// Raster resolution for SVG brush masks.
const SVG_RASTER_SIZE: u32 = 64;

fn load_svg(path: &Path) -> Result<BrushMask> {
    rasterise_svg_bytes(
        &std::fs::read(path)
            .with_context(|| format!("reading SVG {}", path.display()))?,
    )
    .with_context(|| format!("rasterising SVG {}", path.display()))
}

/// Parse and rasterise SVG bytes to a [`BrushMask`] at [`SVG_RASTER_SIZE`].
///
/// `resvg` / `tiny-skia` use pre-multiplied alpha internally.  We un-multiply
/// before handing the buffer to `BrushMask` so per-channel reads are linear.
pub(crate) fn rasterise_svg_bytes(svg: &[u8]) -> Result<BrushMask> {
    const S: u32 = SVG_RASTER_SIZE;

    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg, &opt)
        .context("parsing SVG")?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(S, S)
        .context("allocating pixmap")?;

    let sx = S as f32 / tree.size().width();
    let sy = S as f32 / tree.size().height();
    let transform = resvg::tiny_skia::Transform::from_scale(sx, sy);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let pixels = unpremultiply(pixmap.take());
    Ok(BrushMask::from_rgba(pixels, S, S))
}

/// Convert pre-multiplied RGBA to straight (un-multiplied) RGBA in-place.
fn unpremultiply(mut data: Vec<u8>) -> Vec<u8> {
    for chunk in data.chunks_mut(4) {
        let a = chunk[3];
        if a > 0 && a < 255 {
            let af = a as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 / af).round().min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / af).round().min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / af).round().min(255.0) as u8;
        }
    }
    data
}
