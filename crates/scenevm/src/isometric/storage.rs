//! Allocation ownership shared by injected lighting/shadow passes. Reference
//! adapters keep dedicated storage; canonical owners retain device-pool leases.
use crate::{SceneVMError, SceneVMResult};
use renderkit::{GpuContext, WorkingBufferDescriptor, WorkingBufferLease};
use std::ops::Deref;

pub(super) struct Buffer {
    pub buffer: wgpu::Buffer,
    capacity: usize,
    usage: wgpu::BufferUsages,
    _lease: Option<WorkingBufferLease>,
}
impl Buffer {
    pub fn new(
        device: &wgpu::Device,
        context: Option<&GpuContext>,
        bytes: usize,
        usage: wgpu::BufferUsages,
    ) -> SceneVMResult<Self> {
        let (buffer, lease) = if let Some(context) = context {
            let lease = context
                .acquire_working_buffer(WorkingBufferDescriptor {
                    bytes: bytes as u64,
                    usage,
                })
                .map_err(|error| SceneVMError::BufferAllocationFailed(error.to_string()))?;
            // The reference allocation has fresh-zero tails and sentinels. Keep
            // that policy without constructing a CPU-sized zero upload.
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("isometric metadata initialization"),
            });
            encoder.clear_buffer(lease.buffer(), 0, None);
            context.queue.submit([encoder.finish()]);
            (lease.buffer().clone(), Some(lease))
        } else {
            (
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("reference-isometric-metadata"),
                    size: bytes as u64,
                    usage,
                    mapped_at_creation: false,
                }),
                None,
            )
        };
        Ok(Self {
            buffer,
            capacity: bytes,
            usage,
            _lease: lease,
        })
    }
    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        context: Option<&GpuContext>,
    ) -> SceneVMResult<()> {
        if data.len() > self.capacity {
            let bytes = data.len().checked_next_power_of_two().ok_or_else(|| {
                SceneVMError::BufferAllocationFailed("isometric metadata size overflow".into())
            })?;
            *self = Self::new(device, context, bytes, self.usage)?;
        }
        if !data.is_empty() {
            queue.write_buffer(&self.buffer, 0, data);
            if let Some(context) = context {
                context.record_upload(data.len() as u64);
            }
        }
        Ok(())
    }
}
impl Deref for Buffer {
    type Target = wgpu::Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
