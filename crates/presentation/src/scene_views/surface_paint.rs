//! Immutable game paint pages share one GPU source on this SceneViews device.
use super::*;
use renderkit::{AlphaMode, ColorSpace, GpuImage, ImageDescriptor, wgpu};
use rusterix::surface_paint::{PAGE_SIZE, SurfacePaint};
use scenevm::paint_pages::PaintPageBinding;

/// Logical current allocations and transfer counts, not peak/driver memory.
#[derive(Clone, Copy, Debug, Default)]
pub struct SurfacePaintStats {
    pub sources: usize,
    pub logical_cpu_page_bytes: u64,
    pub gpu_plane_bytes: u64,
    pub uploaded_pages: u64,
    pub reused_bindings: u64,
}
struct BoundSource {
    source: Arc<SurfacePaint>,
    binding: Arc<PaintPageBinding>,
}
#[derive(Default)]
pub(super) struct Sources {
    bindings: FxHashMap<usize, BoundSource>,
    uploaded_pages: u64,
    reused_bindings: u64,
}
impl Sources {
    fn binding(
        &mut self,
        context: &GpuContext,
        source: &Arc<SurfacePaint>,
    ) -> Result<Arc<PaintPageBinding>> {
        if context.is_lost() {
            return Err(RenderError("Surface paint device was lost".into()));
        }
        let identity = Arc::as_ptr(source) as usize;
        if let Some(old) = self.bindings.get(&identity) {
            self.reused_bindings += 1;
            return Ok(old.binding.clone());
        }
        source.validate().map_err(RenderError)?;
        let limits = context.device.limits();
        if source.width > limits.max_texture_dimension_2d
            || source.height > limits.max_texture_dimension_2d
            || (std::mem::size_of_val(source.entries.as_ref()) as u64 + 20)
                > u64::from(limits.max_storage_buffer_binding_size)
        {
            return Err(RenderError(
                "Surface paint source exceeds this device's texture/table limits".into(),
            ));
        }
        let descriptor = |format| ImageDescriptor {
            width: source.width,
            height: source.height,
            format,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        };
        let color = GpuImage::new(context, descriptor(wgpu::TextureFormat::Rgba8Unorm))?;
        let material = GpuImage::new(context, descriptor(wgpu::TextureFormat::Rgba8Uint))?;
        for page in &source.pages {
            let origin = wgpu::Origin3d {
                x: page.atlas_origin[0],
                y: page.atlas_origin[1],
                z: 0,
            };
            for (image, pixels) in [(&color, &page.color_rgba), (&material, &page.material_rgba)] {
                context.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &image.texture,
                        mip_level: 0,
                        origin,
                        aspect: wgpu::TextureAspect::All,
                    },
                    pixels,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(PAGE_SIZE * 4),
                        rows_per_image: Some(PAGE_SIZE),
                    },
                    wgpu::Extent3d {
                        width: PAGE_SIZE,
                        height: PAGE_SIZE,
                        depth_or_array_layers: 1,
                    },
                );
            }
            context.record_upload((page.color_rgba.len() + page.material_rgba.len()) as u64);
        }
        let binding = PaintPageBinding::new(
            context,
            color,
            material,
            source.entries.clone(),
            source.alpha_geo_ids.clone(),
        )?;
        self.uploaded_pages += source.pages.len() as u64;
        self.bindings.insert(
            identity,
            BoundSource {
                source: source.clone(),
                binding: binding.clone(),
            },
        );
        Ok(binding)
    }
    fn stats(&self) -> SurfacePaintStats {
        SurfacePaintStats {
            sources: self.bindings.len(),
            logical_cpu_page_bytes: self
                .bindings
                .values()
                .map(|entry| {
                    entry
                        .source
                        .pages
                        .iter()
                        .map(|page| (page.color_rgba.len() + page.material_rgba.len()) as u64)
                        .sum::<u64>()
                })
                .sum(),
            gpu_plane_bytes: self
                .bindings
                .values()
                .map(|entry| u64::from(entry.source.width) * u64::from(entry.source.height) * 8)
                .sum(),
            uploaded_pages: self.uploaded_pages,
            reused_bindings: self.reused_bindings,
        }
    }
}
impl SceneViews {
    pub fn surface_paint_stats(&self) -> SurfacePaintStats {
        self.surface_paint.stats()
    }

    pub(super) fn retire_unused_surface_paint(&mut self) {
        let live: std::collections::HashSet<_> = self
            .views
            .values()
            .filter_map(|view| {
                view.game_surface_paint
                    .as_ref()
                    .map(|source| Arc::as_ptr(source) as usize)
            })
            .collect();
        self.surface_paint
            .bindings
            .retain(|identity, _| live.contains(identity));
    }
    pub(crate) fn apply_game_surface_paint(
        &mut self,
        id: ViewId,
        source: Option<&Arc<SurfacePaint>>,
    ) -> Result<()> {
        let view = self
            .views
            .get(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let same = match (&view.game_surface_paint, source) {
            (None, None) => true,
            (Some(old), Some(next)) => Arc::ptr_eq(old, next),
            _ => false,
        };
        if same {
            return Ok(());
        }
        let binding = source
            .map(|source| self.surface_paint.binding(&self.context, source))
            .transpose()?;
        let view = self.views.get_mut(&id).unwrap();
        view.scene.vm.set_active_vm(0);
        if let Some(binding) = binding {
            view.scene
                .vm
                .set_paint_page_binding(&self.context, binding)?;
        } else {
            view.scene
                .vm
                .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
        }
        view.game_surface_paint = source.cloned();
        self.retire_unused_surface_paint();
        Ok(())
    }
}
