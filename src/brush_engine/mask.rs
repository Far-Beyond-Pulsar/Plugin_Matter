//! `BrushMask` — a rasterised brush stamp image with four material channels.
//!
//! # RGBA channel semantics
//!
//! | Channel | Material property    | Typical range |
//! |---------|----------------------|---------------|
//! | **R**   | Intensity (coverage) | 0 = no paint → 255 = full paint |
//! | **G**   | Reflectiveness       | 0 = matte → 255 = fully metallic |
//! | **B**   | Smear amount         | 0 = none → 255 = full lateral smear |
//! | **A**   | Roughness            | 0 = smooth → 255 = fully rough; also the alpha-mask validity flag |
//!
//! For basic colour-painting brushes (all built-ins) only the **R** channel
//! is consumed by the stamp rasteriser; G and B are available for future
//! material-layer painting modes.
//!
//! ## SVG brush masks
//! SVG masks use a greyscale fill (R = G = B), so the intensity stored in R
//! naturally encodes the brush shape's coverage.  A pixel fully outside the
//! SVG shape has A = 0 and is skipped during stamping.

/// Per-pixel values sampled from a `BrushMask`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BrushChannels {
    /// Coverage strength: 0.0 = no paint, 1.0 = full paint.  Driven by the R channel.
    pub intensity: f32,
    /// Metallic / specular contribution (G channel).
    pub reflectiveness: f32,
    /// Lateral smear force (B channel).
    pub smear: f32,
    /// Surface roughness (A channel); also used as the mask boundary flag.
    pub roughness: f32,
}

/// A pre-rasterised brush mask stored as a flat row-major RGBA8 buffer.
#[derive(Debug, Clone)]
pub struct BrushMask {
    /// Raw RGBA8 pixel data — length == `width * height * 4`.
    pub pixels: Vec<u8>,
    pub width:  u32,
    pub height: u32,
}

impl BrushMask {
    /// Construct a mask from an external RGBA8 buffer.
    ///
    /// # Panics
    /// Panics if `pixels.len() != width * height * 4`.
    pub fn from_rgba(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        assert_eq!(
            pixels.len() as u64,
            width as u64 * height as u64 * 4,
            "pixel buffer length mismatch"
        );
        Self { pixels, width, height }
    }

    /// Sample a single raw pixel.
    #[inline(always)]
    fn fetch(&self, x: usize, y: usize) -> BrushChannels {
        let i = (y * self.width as usize + x) * 4;
        BrushChannels {
            intensity:      self.pixels[i]     as f32 / 255.0,
            reflectiveness: self.pixels[i + 1] as f32 / 255.0,
            smear:          self.pixels[i + 2] as f32 / 255.0,
            roughness:      self.pixels[i + 3] as f32 / 255.0,
        }
    }

    /// Sample with **bilinear interpolation** at normalised UV (0.0..=1.0).
    ///
    /// Bilinear sampling is critical for rotated masks — nearest-neighbour
    /// produces severe staircase aliasing on any non-axis-aligned brush shape.
    pub fn sample(&self, u: f32, v: f32) -> BrushChannels {
        let w = (self.width  - 1) as f32;
        let h = (self.height - 1) as f32;

        let xf = u.clamp(0.0, 1.0) * w;
        let yf = v.clamp(0.0, 1.0) * h;

        let x0 = (xf.floor() as usize).min(self.width  as usize - 1);
        let y0 = (yf.floor() as usize).min(self.height as usize - 1);
        let x1 = (x0 + 1).min(self.width  as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);

        let tx = xf - xf.floor();
        let ty = yf - yf.floor();

        let s00 = self.fetch(x0, y0);
        let s10 = self.fetch(x1, y0);
        let s01 = self.fetch(x0, y1);
        let s11 = self.fetch(x1, y1);

        // Bilerp each channel independently.
        let blerp = |a: f32, b: f32, c: f32, d: f32| {
            let top    = a + (b - a) * tx;
            let bottom = c + (d - c) * tx;
            top + (bottom - top) * ty
        };

        BrushChannels {
            intensity:      blerp(s00.intensity,      s10.intensity,      s01.intensity,      s11.intensity),
            reflectiveness: blerp(s00.reflectiveness, s10.reflectiveness, s01.reflectiveness, s11.reflectiveness),
            smear:          blerp(s00.smear,          s10.smear,          s01.smear,          s11.smear),
            roughness:      blerp(s00.roughness,      s10.roughness,      s01.roughness,      s11.roughness),
        }
    }

    /// Build a 32 × 32 greyscale-intensity thumbnail (RGBA8, opaque).
    ///
    /// The thumbnail shows only the R (intensity) channel as a greyscale image,
    /// which is what the user perceives as the brush "shape".
    pub fn thumbnail(&self) -> Vec<u8> {
        const T: u32 = 32;
        let mut out = vec![0u8; (T * T * 4) as usize];
        for ty in 0..T {
            for tx in 0..T {
                let u = tx as f32 / (T - 1) as f32;
                let v = ty as f32 / (T - 1) as f32;
                let ch = self.sample(u, v);
                let v8 = (ch.intensity * 255.0) as u8;
                let i = ((ty * T + tx) * 4) as usize;
                out[i]     = v8;
                out[i + 1] = v8;
                out[i + 2] = v8;
                out[i + 3] = 255;
            }
        }
        out
    }

    /// Procedurally generate a soft round falloff mask.
    ///
    /// Used as the fallback when no `.pbrush` directories are found on disk.
    pub fn default_round() -> Self {
        const SIZE: u32 = 64;
        let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
        let half = SIZE as f32 / 2.0;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let dx = x as f32 + 0.5 - half;
                let dy = y as f32 + 0.5 - half;
                let d  = (dx * dx + dy * dy).sqrt();
                if d < half {
                    // Quadratic falloff: full coverage at centre, zero at edge.
                    let t = 1.0 - (d / half);
                    let v = (t * t * 255.0) as u8;
                    let i = ((y * SIZE + x) * 4) as usize;
                    pixels[i]     = v; // R = intensity
                    pixels[i + 1] = 0; // G = reflectiveness
                    pixels[i + 2] = 0; // B = smear
                    pixels[i + 3] = v; // A = roughness (mirrors intensity)
                }
            }
        }
        Self { pixels, width: SIZE, height: SIZE }
    }
}
