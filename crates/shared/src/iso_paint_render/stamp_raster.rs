//! Shared stamp kernels can target an explicit reference image or a bounded
//! virtual raster. Virtual coordinates remain unchanged, including noise hashes.
use super::*;
mod procedural;

pub(super) trait Target: Sized {
    fn dim(&self) -> &TheDim;
    fn blend(&mut self, x: i32, y: i32, color: [u8; 4]);
    fn replace(&mut self, x: i32, y: i32, color: [u8; 4]);
    fn blank(&self) -> Self;
    fn material_mask(&mut self, source: Self, material: [u8; 3]);
}
impl Target for TheRGBABuffer {
    fn dim(&self) -> &TheDim {
        TheRGBABuffer::dim(self)
    }
    fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        let dim = *self.dim();
        IsoPaintRenderer::iso_paint_blend_pixel(
            self.pixels_mut(),
            dim.width.max(0) as usize,
            dim.height.max(0) as usize,
            x,
            y,
            color,
        );
    }
    fn replace(&mut self, x: i32, y: i32, color: [u8; 4]) {
        let dim = *self.dim();
        if x >= 0 && y >= 0 && x < dim.width && y < dim.height {
            let index = (y as usize * dim.width as usize + x as usize) * 4;
            self.pixels_mut()[index..index + 4].copy_from_slice(&color);
        }
    }
    fn blank(&self) -> Self {
        Self::new(*self.dim())
    }
    fn material_mask(&mut self, source: Self, material: [u8; 3]) {
        for (color, target) in source
            .pixels()
            .chunks_exact(4)
            .zip(self.pixels_mut().chunks_exact_mut(4))
        {
            if color[3] > 0 {
                target.copy_from_slice(&[material[0], material[1], material[2], color[3]]);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    min: [i32; 2],
    max: [i32; 2],
}
impl Bounds {
    fn empty() -> Self {
        Self {
            min: [i32::MAX; 2],
            max: [-1; 2],
        }
    }
    fn include(&mut self, x: i32, y: i32) {
        self.min = [self.min[0].min(x), self.min[1].min(y)];
        self.max = [self.max[0].max(x), self.max[1].max(y)];
    }
    fn present(self) -> bool {
        self.max[0] >= self.min[0] && self.max[1] >= self.min[1]
    }
    fn size(self) -> [u32; 2] {
        [
            (self.max[0] - self.min[0] + 1) as u32,
            (self.max[1] - self.min[1] + 1) as u32,
        ]
    }
}
struct Coverage {
    dim: TheDim,
    bounds: Bounds,
}
impl Target for Coverage {
    fn dim(&self) -> &TheDim {
        &self.dim
    }
    fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        // Integer source-over never removes positive coverage. Recording all
        // nonzero contributors therefore yields the exact final alpha bounds.
        if color[3] > 0 && x >= 0 && y >= 0 && x < self.dim.width && y < self.dim.height {
            self.bounds.include(x, y);
        }
    }
    fn replace(&mut self, x: i32, y: i32, color: [u8; 4]) {
        self.blend(x, y, color);
    }
    fn blank(&self) -> Self {
        Self {
            dim: self.dim,
            bounds: Bounds::empty(),
        }
    }
    fn material_mask(&mut self, source: Self, _: [u8; 3]) {
        if source.bounds.present() {
            self.bounds
                .include(source.bounds.min[0], source.bounds.min[1]);
            self.bounds
                .include(source.bounds.max[0], source.bounds.max[1]);
        }
    }
}

struct Samples {
    dim: TheDim,
    bounds: Bounds,
    size: [u32; 2],
    rgba: Vec<u8>,
}
impl Samples {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < self.bounds.min[0]
            || y < self.bounds.min[1]
            || x > self.bounds.max[0]
            || y > self.bounds.max[1]
        {
            return None;
        }
        let crop = self.bounds.size();
        let source = [
            (x - self.bounds.min[0]) as u64,
            (y - self.bounds.min[1]) as u64,
        ];
        // Invert floor(sample * crop / output) exactly, using integers. Only
        // source pixels actually sampled by the established <=256px sprite
        // receive storage; all their blend operations retain their old order.
        let sample = [
            (source[0] * u64::from(self.size[0])).div_ceil(u64::from(crop[0])),
            (source[1] * u64::from(self.size[1])).div_ceil(u64::from(crop[1])),
        ];
        for axis in 0..2 {
            if sample[axis] >= u64::from(self.size[axis])
                || sample[axis] * u64::from(crop[axis]) / u64::from(self.size[axis]) != source[axis]
            {
                return None;
            }
        }
        Some((sample[1] as usize * self.size[0] as usize + sample[0] as usize) * 4)
    }
}
impl Target for Samples {
    fn dim(&self) -> &TheDim {
        &self.dim
    }
    fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if let Some(index) = self.index(x, y) {
            IsoPaintRenderer::iso_paint_blend_pixel_at(&mut self.rgba, index, color);
        }
    }
    fn replace(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if let Some(index) = self.index(x, y) {
            self.rgba[index..index + 4].copy_from_slice(&color);
        }
    }
    fn blank(&self) -> Self {
        Self {
            dim: self.dim,
            bounds: self.bounds,
            size: self.size,
            rgba: vec![0; self.rgba.len()],
        }
    }
    fn material_mask(&mut self, source: Self, material: [u8; 3]) {
        for (color, target) in source
            .rgba
            .chunks_exact(4)
            .zip(self.rgba.chunks_exact_mut(4))
        {
            if color[3] > 0 {
                target.copy_from_slice(&[material[0], material[1], material[2], color[3]]);
            }
        }
    }
}

pub(super) struct NativeSprite {
    pub sprite: scenevm::OrganicBillboardSprite,
    pub min: [i32; 2],
    pub max: [i32; 2],
    pub anchor: [i32; 2],
}
/// Two deterministic passes use constant coverage metadata and at most256x256
/// RGBA samples (plus one equally bounded material scratch image). No temporary
/// allocation depends on the original canvas area.
pub(super) fn native_sprite(stamp: &IsoPaintStamp) -> Result<Option<NativeSprite>, String> {
    if !stamp.size.is_finite() {
        return Err("Stamp size must be finite".into());
    }
    let size = stamp.size.max(0.01);
    let side = (size * 24.).ceil().max(16.) as f64 + 4.;
    let above = (size * 34.).ceil().max(24.) as f64 + 4.;
    let below = (size * 14.).ceil().max(12.) as f64 + 4.;
    // Leave signed-coordinate headroom for every shipped procedural kernel's
    // offsets/products. Invalid coordinates are errors, not a legacy handoff.
    let limit = (i32::MAX / 64) as f64;
    if ![side, above, below]
        .into_iter()
        .all(|v| v.is_finite() && v <= limit)
    {
        return Err("Stamp dimensions exceed the procedural coordinate range".into());
    }
    let anchor = [side as i32, above as i32];
    let dim = TheDim::sized(side as i32 * 2 + 1, above as i32 + below as i32 + 1);
    let mut coverage = Coverage {
        dim,
        bounds: Bounds::empty(),
    };
    IsoPaintRenderer::draw_iso_paint_stamp_shape(
        &mut coverage,
        stamp,
        None,
        anchor,
        None,
        None,
        size,
    );
    if !coverage.bounds.present() {
        return Ok(None);
    }
    let crop = coverage.bounds.size();
    let output = [crop[0].min(256), crop[1].min(256)];
    let mut samples = Samples {
        dim,
        bounds: coverage.bounds,
        size: output,
        rgba: vec![0; output[0] as usize * output[1] as usize * 4],
    };
    IsoPaintRenderer::draw_iso_paint_stamp_shape(
        &mut samples,
        stamp,
        None,
        anchor,
        None,
        None,
        size,
    );
    Ok(Some(NativeSprite {
        sprite: scenevm::OrganicBillboardSprite {
            width: output[0],
            height: output[1],
            rgba: samples.rgba,
        },
        min: coverage.bounds.min,
        max: coverage.bounds.max,
        anchor,
    }))
}
