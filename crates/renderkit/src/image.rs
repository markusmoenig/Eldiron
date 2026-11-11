use crate::{AlphaMode, ColorSpace, GpuContext, ImageAsset, ImageHandle, RenderError, Result};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
static NEXT_REGISTRY: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    pub color_space: ColorSpace,
    pub alpha_mode: AlphaMode,
}
impl ImageDescriptor {
    pub fn rgba(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        }
    }
}
/// GPU-resident image. It allocates neither CPU pixels nor a readback buffer.
pub struct GpuImage {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub descriptor: ImageDescriptor,
    device_generation: u64,
}
impl GpuImage {
    pub fn new(context: &GpuContext, descriptor: ImageDescriptor) -> Result<Arc<Self>> {
        if descriptor.width == 0
            || descriptor.height == 0
            || descriptor.width > context.device.limits().max_texture_dimension_2d
            || descriptor.height > context.device.limits().max_texture_dimension_2d
        {
            return Err(RenderError("invalid GPU image dimensions".into()));
        }
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("renderkit-image"),
            size: wgpu::Extent3d {
                width: descriptor.width,
                height: descriptor.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: descriptor.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        Ok(Arc::new(Self::from_texture(context, texture, descriptor)?))
    }
    /// The supplied texture must have been created on this context's device.
    /// Ownership metadata prevents mixing registered images across device domains.
    pub fn from_texture(
        context: &GpuContext,
        texture: wgpu::Texture,
        descriptor: ImageDescriptor,
    ) -> Result<Self> {
        if texture.width() != descriptor.width
            || texture.height() != descriptor.height
            || texture.format() != descriptor.format
        {
            return Err(RenderError("GPU image descriptor mismatch".into()));
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            texture,
            view,
            descriptor,
            device_generation: context.generation(),
        })
    }
    pub fn device_generation(&self) -> u64 {
        self.device_generation
    }
}
struct Slot {
    generation: u32,
    image: Option<Arc<GpuImage>>,
}
/// Handles are registry-scoped and stale generations are rejected. Arc leases keep
/// removed resources alive while commands/frames still reference them.
pub struct ImageRegistry {
    context: Arc<GpuContext>,
    owner: u64,
    slots: Vec<Slot>,
    free: Vec<u32>,
}
impl ImageRegistry {
    pub fn new(context: Arc<GpuContext>) -> Self {
        Self {
            context,
            owner: NEXT_REGISTRY.fetch_add(1, Ordering::Relaxed),
            slots: vec![],
            free: vec![],
        }
    }
    pub fn context(&self) -> &Arc<GpuContext> {
        &self.context
    }
    pub fn insert(&mut self, image: Arc<GpuImage>) -> Result<ImageHandle> {
        if image.device_generation() != self.context.generation() {
            return Err(RenderError(
                "image belongs to a different GPU context".into(),
            ));
        }
        let slot = if let Some(index) = self.free.pop() {
            self.slots[index as usize].image = Some(image);
            index
        } else {
            let index = u32::try_from(self.slots.len())
                .map_err(|_| RenderError("image registry exhausted".into()))?;
            self.slots.push(Slot {
                generation: 1,
                image: Some(image),
            });
            index
        };
        Ok(ImageHandle {
            owner: self.owner,
            slot,
            generation: self.slots[slot as usize].generation,
        })
    }
    pub fn get(&self, handle: ImageHandle) -> Option<Arc<GpuImage>> {
        if handle.owner != self.owner {
            return None;
        }
        let slot = self.slots.get(handle.slot as usize)?;
        (slot.generation == handle.generation)
            .then(|| slot.image.clone())
            .flatten()
    }
    pub fn remove(&mut self, handle: ImageHandle) -> bool {
        if self.get(handle).is_none() {
            return false;
        }
        let slot = &mut self.slots[handle.slot as usize];
        slot.image = None;
        // Retire the slot on generation exhaustion instead of reviving an old handle.
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            self.free.push(handle.slot);
        }
        true
    }
    pub fn upload(&mut self, source: &ImageAsset) -> Result<ImageHandle> {
        let mut descriptor = ImageDescriptor::rgba(source.width(), source.height());
        descriptor.color_space = source.color_space;
        descriptor.alpha_mode = source.alpha_mode;
        if source.color_space == ColorSpace::Linear {
            descriptor.format = wgpu::TextureFormat::Rgba8Unorm;
        }
        let image = GpuImage::new(&self.context, descriptor)?;
        self.context.record_upload(source.pixels().len() as u64);
        self.context.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &image.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            source.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(source.width() * 4),
                rows_per_image: Some(source.height()),
            },
            image.texture.size(),
        );
        self.insert(image)
    }
}
