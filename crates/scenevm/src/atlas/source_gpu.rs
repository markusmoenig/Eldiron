//! Immutable GPU source publications. Every consumer pins its own version;
//! changing pixels/layout never overwrites a retained private frame's textures.
use super::*;
#[cfg(test)]
mod tests;

pub(crate) struct AtlasGpuSnapshot {
    color: wgpu::Texture,
    material: wgpu::Texture,
    _leases: [Option<renderkit::TargetLease>; 2],
}
impl AtlasGpuSnapshot {
    pub(crate) fn same_storage(&self, other: &Self) -> bool {
        self.color == other.color && self.material == other.material
    }
    pub(crate) fn views(&self) -> (wgpu::TextureView, wgpu::TextureView) {
        // Full chains preserve the existing trilinear/material sampling policy.
        (
            self.color
                .create_view(&wgpu::TextureViewDescriptor::default()),
            self.material
                .create_view(&wgpu::TextureViewDescriptor::default()),
        )
    }
}
impl SharedAtlas {
    pub fn upload_to_gpu_with_context(
        &self,
        context: &renderkit::GpuContext,
    ) -> renderkit::Result<()> {
        self.ensure_built();
        let mut inner = self.inner.lock().unwrap();
        let current = |texture: &Texture| {
            texture.gpu.as_ref().is_some_and(|gpu| {
                gpu.context_generation == Some(context.generation())
                    && gpu.device == context.device
                    && gpu.texture.width() == texture.width
                    && gpu.texture.height() == texture.height
                    && gpu.target_lease.is_some()
            })
        };
        if !inner.atlas_dirty && current(&inner.atlas) && current(&inner.atlas_material) {
            return Ok(());
        }
        // Acquire both before replacing either published plane. A failed
        // allocation leaves the previous complete source/storage untouched.
        let color = inner.atlas.prepare_gpu_storage(context)?;
        let material = inner.atlas_material.prepare_gpu_storage(context)?;
        inner.atlas.gpu = Some(color);
        inner.atlas_material.gpu = Some(material);
        // These write mip0 and every generated lower mip before any snapshot
        // can escape the lock. Recycled contents are never sampled or exported.
        inner
            .atlas
            .upload_to_gpu_with(&context.device, &context.queue);
        context.record_upload(inner.atlas.data.len() as u64);
        inner
            .atlas_material
            .upload_to_gpu_with(&context.device, &context.queue);
        context.record_upload(inner.atlas_material.data.len() as u64);
        inner.atlas_dirty = false;
        Ok(())
    }
    pub(crate) fn retained_texture_views(&self) -> Option<AtlasGpuSnapshot> {
        self.with_views(|color, material| AtlasGpuSnapshot {
            color: color.texture.clone(),
            material: material.texture.clone(),
            _leases: [color.target_lease.clone(), material.target_lease.clone()],
        })
    }
}
