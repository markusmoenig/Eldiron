//! Optional host resource ownership. The standalone device/queue runtime keeps
//! its original allocation policy; shared hosts use bounded context pools.
use super::*;
use wgpu::util::DeviceExt;
#[derive(Default)]
pub(super) struct EncodedResources {
    #[cfg(feature = "shared-gpu")]
    inputs: Vec<std::sync::Arc<renderkit::GpuBuffer>>,
    #[cfg(feature = "shared-gpu")]
    working: Vec<renderkit::WorkingBufferLease>,
    #[cfg(feature = "shared-gpu")]
    outputs: Vec<renderkit::TargetLease>,
}
impl EncodedResources {
    pub fn retain_maps<const N: usize>(&mut self, _maps: &[SurfaceMap; N]) {
        #[cfg(feature = "shared-gpu")]
        self.outputs
            .extend(_maps.iter().filter_map(|map| map._allocation.clone()));
    }
}
fn allocation_error(error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        "render",
        Location::asset("MaterialZ GPU resources"),
        error.to_string(),
    )
}
impl MaterialZ {
    /// Surface contents become valid after their first successful encode.
    pub(super) fn create_maps<const N: usize>(
        &self,
        request: &RenderRequest,
        layouts: [OutputLayout; N],
    ) -> Result<[SurfaceMap; N]> {
        let mut maps = Vec::with_capacity(N);
        for layout in layouts {
            maps.push(self.create_map(request, layout)?);
        }
        maps.try_into()
            .map_err(|_| allocation_error("Invalid output map count"))
    }
    fn create_map(&self, request: &RenderRequest, layout: OutputLayout) -> Result<SurfaceMap> {
        let usage = wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        #[cfg(feature = "shared-gpu")]
        if let Some(context) = &self.context {
            let mut descriptor = renderkit::TargetDescriptor::color(
                request.resolution[0],
                request.resolution[1],
                layout.format,
            );
            descriptor.usage = usage;
            let lease = context
                .acquire_target(descriptor)
                .map_err(allocation_error)?;
            return Ok(SurfaceMap {
                texture: lease.texture().clone(),
                view: lease.view().clone(),
                layout,
                _allocation: Some(lease),
            });
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(layout.channel.name()),
            size: wgpu::Extent3d {
                width: request.resolution[0],
                height: request.resolution[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: layout.format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Ok(SurfaceMap {
            texture,
            view,
            layout,
            #[cfg(feature = "shared-gpu")]
            _allocation: None,
        })
    }
    pub(super) fn buffer(
        &self,
        label: &str,
        content: &[u8],
        _resources: &mut EncodedResources,
    ) -> Result<wgpu::Buffer> {
        #[cfg(feature = "shared-gpu")]
        if let Some(context) = &self.context {
            let owner = context
                .upload_immutable_buffer(content, wgpu::BufferUsages::STORAGE)
                .map_err(allocation_error)?;
            let buffer = owner.buffer().clone();
            _resources.inputs.push(owner);
            return Ok(buffer);
        }
        Ok(self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: content,
                usage: wgpu::BufferUsages::STORAGE,
            }))
    }
    pub(super) fn working_buffer(
        &self,
        _encoder: &mut wgpu::CommandEncoder,
        label: &str,
        bytes: u64,
        usage: wgpu::BufferUsages,
        _resources: &mut EncodedResources,
    ) -> Result<wgpu::Buffer> {
        #[cfg(feature = "shared-gpu")]
        if let Some(context) = &self.context {
            let owner = context
                .acquire_working_buffer(renderkit::WorkingBufferDescriptor {
                    bytes,
                    usage: usage | wgpu::BufferUsages::COPY_DST,
                })
                .map_err(allocation_error)?;
            let buffer = owner.buffer().clone();
            // Preserve newly allocated storage's zero-before-use semantics,
            // including partial intermediate fields and diagnostic atomic ORs.
            _encoder.clear_buffer(&buffer, 0, None);
            _resources.working.push(owner);
            return Ok(buffer);
        }
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes,
            usage,
            mapped_at_creation: false,
        }))
    }
}
