//! Shared stamp kernels can target an explicit reference image or a bounded
//! virtual raster. Virtual coordinates remain unchanged, including noise hashes.
use super::*;
mod cooperative;
mod procedural;
pub use cooperative::StampWork;
use cooperative::{Slice, finish};

pub(super) trait Target: Sized + Send {
    fn dim(&self) -> &TheDim;
    fn blend(&mut self, x: i32, y: i32, color: [u8; 4]);
    fn replace(&mut self, x: i32, y: i32, color: [u8; 4]);
    fn blank(&self) -> Self;
    fn material_mask(&mut self, source: Self, material: [u8; 3]);
}

impl IsoPaintRenderer {
    pub(in crate::iso_paint_render) fn draw_iso_paint_stamp_shape(
        buffer: &mut impl Target,
        stamp: &IsoPaintStamp,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        screen: [i32; 2],
        stamp_depth: Option<f32>,
        owner_geo_id: Option<scenevm::GeoId>,
        size: f32,
    ) {
        finish(Self::draw_iso_paint_stamp_shape_steps(
            buffer,
            stamp,
            surface_buffer,
            screen,
            stamp_depth,
            owner_geo_id,
            size,
            &mut Slice::unlimited(),
        ));
    }
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
    region_origin: [u32; 2],
    region_size: [u32; 2],
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
        if !(0..2).all(|axis| {
            sample[axis] >= u64::from(self.region_origin[axis])
                && sample[axis] < u64::from(self.region_origin[axis] + self.region_size[axis])
        }) {
            return None;
        }
        Some(
            ((sample[1] as usize - self.region_origin[1] as usize) * self.region_size[0] as usize
                + sample[0] as usize
                - self.region_origin[0] as usize)
                * 4,
        )
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
            region_origin: self.region_origin,
            region_size: self.region_size,
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

/// Alpha-only samples share the exact coordinate mapping and integer source-
/// over formula. They retain one cutoff bit per sprite texel, not RGBA pixels.
struct AlphaSamples {
    mapping: Samples,
    alpha: Vec<u8>,
}
impl Target for AlphaSamples {
    fn dim(&self) -> &TheDim {
        &self.mapping.dim
    }
    fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if let Some(offset) = self.mapping.index(x, y) {
            let alpha = &mut self.alpha[offset / 4];
            *alpha = (u32::from(color[3]) + u32::from(*alpha) * (255 - u32::from(color[3])) / 255)
                .min(255) as u8;
        }
    }
    fn replace(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if let Some(offset) = self.mapping.index(x, y) {
            self.alpha[offset / 4] = color[3];
        }
    }
    fn blank(&self) -> Self {
        Self {
            mapping: self.mapping.blank(),
            alpha: vec![0; self.alpha.len()],
        }
    }
    fn material_mask(&mut self, source: Self, _: [u8; 3]) {
        for (source, target) in source.alpha.into_iter().zip(&mut self.alpha) {
            if source > 0 {
                *target = source;
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
/// Retained current stamp input and exact original crop. No sprite pixels are
/// retained; a residency owner requests only a bounded output rectangle.
#[derive(Clone)]
pub struct StampRecipe {
    pub(super) stamp: std::sync::Arc<IsoPaintStamp>,
    dim: TheDim,
    bounds: Bounds,
    output: [u32; 2],
    anchor: [i32; 2],
    alpha_mask: std::sync::OnceLock<std::sync::Arc<[u32]>>,
}
impl StampRecipe {
    pub fn prepare(stamp: std::sync::Arc<IsoPaintStamp>) -> Result<Option<Self>, String> {
        finish(Self::prepare_steps(stamp, &mut Slice::unlimited()))
    }
    /// Freeze crop and cutoff coverage cooperatively for private paged sources.
    pub fn prepare_masked_work(
        stamp: std::sync::Arc<IsoPaintStamp>,
    ) -> StampWork<Option<std::sync::Arc<Self>>> {
        StampWork::new(async move {
            let mut work = Slice::cooperative();
            let Some(recipe) = Self::prepare_steps(stamp, &mut work).await? else {
                return Ok(None);
            };
            recipe.alpha_mask_steps(&mut work).await;
            Ok(Some(std::sync::Arc::new(recipe)))
        })
    }
    async fn prepare_steps(
        stamp: std::sync::Arc<IsoPaintStamp>,
        work: &mut Slice,
    ) -> Result<Option<Self>, String> {
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
        IsoPaintRenderer::draw_iso_paint_stamp_shape_steps(
            &mut coverage,
            &stamp,
            None,
            anchor,
            None,
            None,
            size,
            work,
        )
        .await;
        if !coverage.bounds.present() {
            return Ok(None);
        }
        let crop = coverage.bounds.size();
        let output = [crop[0].min(256), crop[1].min(256)];

        Ok(Some(Self {
            stamp,
            dim,
            bounds: coverage.bounds,
            output,
            anchor,
            alpha_mask: Default::default(),
        }))
    }
    pub fn size(&self) -> [u32; 2] {
        self.output
    }
    pub fn alpha_mask(&self) -> std::sync::Arc<[u32]> {
        finish(self.alpha_mask_steps(&mut Slice::unlimited()))
    }
    async fn alpha_mask_steps(&self, work: &mut Slice) -> std::sync::Arc<[u32]> {
        if let Some(mask) = self.alpha_mask.get() {
            return mask.clone();
        }
        let pixels = self.output[0] as usize * self.output[1] as usize;
        let mut target = AlphaSamples {
            mapping: Samples {
                dim: self.dim,
                bounds: self.bounds,
                size: self.output,
                region_origin: [0; 2],
                region_size: self.output,
                rgba: Vec::new(),
            },
            alpha: vec![0; pixels],
        };
        IsoPaintRenderer::draw_iso_paint_stamp_shape_steps(
            &mut target,
            &self.stamp,
            None,
            self.anchor,
            None,
            None,
            self.stamp.size.max(0.01),
            work,
        )
        .await;
        let mut mask = vec![0u32; pixels.div_ceil(32)];
        for (index, alpha) in target.alpha.into_iter().enumerate() {
            work.checkpoint().await;
            // Exactly BILLBOARD_ALPHA_CUTOFF=0.08 (20/255 < .08 < 21/255).
            if alpha >= 21 {
                mask[index / 32] |= 1 << (index % 32);
            }
        }
        let mask: std::sync::Arc<[u32]> = mask.into();
        let _ = self.alpha_mask.set(mask.clone());
        self.alpha_mask.get().unwrap().clone()
    }
    pub fn crop(&self) -> ([i32; 2], [i32; 2], [i32; 2]) {
        (self.bounds.min, self.bounds.max, self.anchor)
    }
    /// Original sprite coordinates, with transparent padding at partial edge
    /// pages. Allocation is bounded independently of original canvas size.
    pub fn raster(&self, origin: [u32; 2], size: [u32; 2]) -> Result<Vec<u8>, String> {
        finish(self.raster_steps(origin, size, &mut Slice::unlimited()))
    }
    pub fn raster_work(
        self: &std::sync::Arc<Self>,
        origin: [u32; 2],
        size: [u32; 2],
    ) -> StampWork<Vec<u8>> {
        let recipe = self.clone();
        StampWork::new(async move {
            recipe
                .raster_steps(origin, size, &mut Slice::cooperative())
                .await
        })
    }
    async fn raster_steps(
        &self,
        origin: [u32; 2],
        size: [u32; 2],
        work: &mut Slice,
    ) -> Result<Vec<u8>, String> {
        if !(0..2).all(|axis| {
            size[axis] > 0
                && size[axis] <= 256
                && origin[axis] < self.output[axis]
                && origin[axis].checked_add(size[axis]).is_some()
        }) {
            return Err("Invalid stamp source rectangle".into());
        }
        let mut samples = Samples {
            dim: self.dim,
            bounds: self.bounds,
            size: self.output,
            region_origin: origin,
            region_size: size,
            rgba: vec![0; size[0] as usize * size[1] as usize * 4],
        };
        IsoPaintRenderer::draw_iso_paint_stamp_shape_steps(
            &mut samples,
            &self.stamp,
            None,
            self.anchor,
            None,
            None,
            self.stamp.size.max(0.01),
            work,
        )
        .await;
        Ok(samples.rgba)
    }
}

/// Synchronous small-sprite convenience API. Paged producers retain the recipe
/// and generate only selected64px rectangles instead of this complete sprite.
pub(super) fn native_sprite(stamp: &IsoPaintStamp) -> Result<Option<NativeSprite>, String> {
    finish(native_sprite_steps(
        std::sync::Arc::new(stamp.clone()),
        &mut Slice::unlimited(),
    ))
}
pub(super) fn native_sprite_work(
    stamp: std::sync::Arc<IsoPaintStamp>,
) -> StampWork<Option<NativeSprite>> {
    StampWork::new(async move { native_sprite_steps(stamp, &mut Slice::cooperative()).await })
}
async fn native_sprite_steps(
    stamp: std::sync::Arc<IsoPaintStamp>,
    work: &mut Slice,
) -> Result<Option<NativeSprite>, String> {
    let Some(recipe) = StampRecipe::prepare_steps(stamp, work).await? else {
        return Ok(None);
    };
    Ok(Some(NativeSprite {
        sprite: scenevm::OrganicBillboardSprite {
            width: recipe.output[0],
            height: recipe.output[1],
            rgba: recipe
                .raster_steps([0; 2], recipe.output, work)
                .await?
                .into(),
        },
        min: recipe.bounds.min,
        max: recipe.bounds.max,
        anchor: recipe.anchor,
    }))
}
