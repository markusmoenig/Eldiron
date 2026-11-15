//! Canonical geometry allocations retain leases alongside their existing VM
//! slots. A semantic role replaces its previous lease even across render modes.
use renderkit::{GpuContext, WorkingBufferDescriptor, WorkingBufferLease};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Vertices2D,
    Indices2D,
    Vertices3D,
    Indices3D,
    Visible,
    Opaque,
    PaintAlpha,
    Transparent,
    Particles,
    Lines,
    TileBins,
    TileTriangles,
    GridHeader,
    GridData,
}
#[derive(Default)]
pub(super) struct GeometryStorage {
    leases: [Option<WorkingBufferLease>; 14],
}
impl GeometryStorage {
    pub fn create(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        context: Option<&GpuContext>,
        kind: Kind,
        descriptor: &wgpu::BufferDescriptor<'_>,
    ) -> crate::SceneVMResult<wgpu::Buffer> {
        if let Some(context) = context {
            let lease = context
                .acquire_working_buffer(WorkingBufferDescriptor {
                    bytes: descriptor.size.max(4),
                    usage: descriptor.usage,
                })
                .map_err(|error| crate::SceneVMError::BufferAllocationFailed(error.to_string()))?;
            // Partial geometry uploads depend on fresh-zero padding/sentinels.
            // Restore that contract on reused storage without a CPU zero image.
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("geometry allocation initialization"),
            });
            encoder.clear_buffer(lease.buffer(), 0, None);
            queue.submit([encoder.finish()]);
            let buffer = lease.buffer().clone();
            self.leases[kind as usize] = Some(lease);
            Ok(buffer)
        } else {
            self.leases[kind as usize] = None;
            Ok(device.create_buffer(descriptor))
        }
    }
    pub fn create_init(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        context: Option<&GpuContext>,
        kind: Kind,
        descriptor: &wgpu::util::BufferInitDescriptor<'_>,
    ) -> crate::SceneVMResult<wgpu::Buffer> {
        if context.is_none() {
            self.leases[kind as usize] = None;
            return Ok(device.create_buffer_init(descriptor));
        }
        if let Some(context) = context.filter(|_| !descriptor.contents.is_empty()) {
            // Full replacement initializes every allocated byte. Avoid an extra
            // clear submission for frequently rebuilt compute geometry/tables.
            let lease = context
                .acquire_working_buffer(WorkingBufferDescriptor {
                    bytes: descriptor.contents.len() as u64,
                    usage: descriptor.usage,
                })
                .map_err(|error| crate::SceneVMError::BufferAllocationFailed(error.to_string()))?;
            queue.write_buffer(lease.buffer(), 0, descriptor.contents);
            let buffer = lease.buffer().clone();
            self.leases[kind as usize] = Some(lease);
            return Ok(buffer);
        }
        // Empty sources retain the same initialized four-byte dummy storage.
        self.create(
            device,
            queue,
            context,
            kind,
            &wgpu::BufferDescriptor {
                label: descriptor.label,
                size: descriptor.contents.len().max(4) as u64,
                usage: descriptor.usage,
                mapped_at_creation: false,
            },
        )
    }
}
