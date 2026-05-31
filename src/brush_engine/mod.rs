//! Brush engine — loads, caches, and applies multi-channel brush masks.
//!
//! # Bundle format (`*.pbrush` directory)
//!
//! ```text
//! brushes/
//!   round_soft.pbrush/
//!     config.json      ← BrushConfig (name, spacing, shape hint, …)
//!     mask.svg         ← brush mask image  (or mask.png)
//! ```
//!
//! # RGBA channel semantics
//!
//! Each pixel of the rasterised mask encodes four material properties:
//!
//! | Channel | Property        | Typical use |
//! |---------|-----------------|-------------|
//! | **R**   | Intensity       | Coverage multiplier during stamping |
//! | **G**   | Reflectiveness  | Per-stamp metallic / specular strength |
//! | **B**   | Smear amount    | Lateral colour diffusion for future smear mode |
//! | **A**   | Roughness       | Surface roughness for PBR material layers; also the mask boundary flag |
//!
//! Built-in SVG brushes are greyscale (R = G = B = value), so the intensity
//! channel naturally encodes the brush shape.  Pixels outside the SVG shape
//! have A = 0 and are skipped during rasterisation.
//!
//! For fully independent PBR channel control, design brush masks as PNG files
//! where each channel is set explicitly.
//!
//! # Built-in brushes
//!
//! Five brushes are always available, compiled into the binary:
//!
//! | ID             | Shape   | Character |
//! |----------------|---------|-----------|
//! | `round_soft`   | Circle  | Smooth Gaussian-like airbrush |
//! | `round_hard`   | Circle  | Hard-edged round stamp |
//! | `square_flat`  | Square  | Flat uniform square |
//! | `texture_rough`| Circle  | Rough gritty texture |
//! | `bristle_soft` | Bristle | Six-strand fan brush |

pub mod builtin;
pub mod config;
pub mod loader;
pub mod mask;
pub mod registry;
pub mod stamp;

pub use config::{BrushConfig, BrushShape};
pub use loader::load_pbrush;
pub use mask::{BrushChannels, BrushMask};
pub use registry::{BrushDropdownItem, BrushEntry, BrushRegistry};
pub use stamp::{composite_wet_erase, composite_wet_over_base, stamp_into_wet};
