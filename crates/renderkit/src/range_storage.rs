//! Immutable published ranges in append-only GPU allocations. A live range pins
//! its whole allocation; unused tails may be filled, published bytes never change.
use crate::{GpuContext, Result, wgpu};
use std::{
    collections::HashMap,
    num::NonZeroU64,
    sync::{Arc, Weak},
};

const PAGE_BYTES: u64 = 64 * 1024;
struct Page {
    buffer: wgpu::Buffer,
    generation: u64,
    _allocation: crate::allocation_usage::AllocationCharge,
}
/// Immutable shader/geometry input. STORAGE consumers must bind read-only;
/// published bytes belong to the content cache and must never be modified.
/// Retain the range through submission; eviction only releases cache ownership.
pub struct GpuBufferRange {
    page: Arc<Page>,
    offset: u64,
    size: u64,
}
impl GpuBufferRange {
    /// Raw allocation for binding identity/accounting. Consumers must not write
    /// any part of this allocation; use the range's slice or sized binding.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.page.buffer
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn device_generation(&self) -> u64 {
        self.page.generation
    }
    /// Stable only while a range owner is retained (as in a binding cache).
    pub fn allocation_identity(&self) -> usize {
        Arc::as_ptr(&self.page) as usize
    }
    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.page.buffer.slice(self.offset..self.offset + self.size)
    }
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.page.buffer,
            offset: self.offset,
            size: NonZeroU64::new(self.size),
        })
    }
    /// Bind at zero and pass offset() as the corresponding dynamic offset.
    pub fn dynamic_binding(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.page.buffer,
            offset: 0,
            size: NonZeroU64::new(self.size),
        })
    }
}
#[derive(Default)]
pub(crate) struct RangeStorage {
    heads: HashMap<u32, (Weak<Page>, u64)>,
}
impl RangeStorage {
    pub fn upload(
        &mut self,
        context: &GpuContext,
        bytes: &[u8],
        usage: wgpu::BufferUsages,
        accounting: &Arc<crate::allocation_usage::AllocationUsage>,
    ) -> Result<(Arc<GpuBufferRange>, bool)> {
        let size = (bytes.len() as u64).max(1).div_ceil(4) * 4;
        let limits = context.device.limits();
        let mut alignment = 4;
        if usage.contains(wgpu::BufferUsages::UNIFORM) {
            alignment = alignment.max(u64::from(limits.min_uniform_buffer_offset_alignment));
        }
        if usage.contains(wgpu::BufferUsages::STORAGE) {
            alignment = alignment.max(u64::from(limits.min_storage_buffer_offset_alignment));
        }
        let packed = size <= PAGE_BYTES / 4;
        let existing = self
            .heads
            .get(&usage.bits())
            .and_then(|(page, next)| {
                let offset = next.div_ceil(alignment) * alignment;
                page.upgrade()
                    .filter(|page| offset + size <= page.buffer.size())
                    .map(|page| (page, offset))
            })
            .filter(|_| packed);
        let created = existing.is_none();
        let (page, offset) = existing.unwrap_or_else(|| {
            let capacity = if packed {
                PAGE_BYTES.min(limits.max_buffer_size).max(size)
            } else {
                size
            };
            let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("renderkit immutable ranges"),
                size: capacity,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            (
                Arc::new(Page {
                    buffer,
                    generation: context.generation(),
                    _allocation: accounting.charge(capacity),
                }),
                0,
            )
        });
        if packed {
            self.heads
                .insert(usage.bits(), (Arc::downgrade(&page), offset + size));
        }
        if bytes.len() as u64 == size {
            context.queue.write_buffer(&page.buffer, offset, bytes);
        } else {
            let mut padded = vec![0; size as usize];
            padded[..bytes.len()].copy_from_slice(bytes);
            context.queue.write_buffer(&page.buffer, offset, &padded);
        }
        Ok((Arc::new(GpuBufferRange { page, offset, size }), created))
    }
}
