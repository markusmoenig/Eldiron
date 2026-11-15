//! Explicit GPU ownership and resources shared by UI and scene renderers.
//! No application event loop, widget tree, or gameplay state lives here.
mod buffer_cache;
pub use buffer_cache::{BufferCacheUsage, GpuBuffer};
mod allocation_usage;
mod working_buffers;
pub use working_buffers::{WorkingBufferDescriptor, WorkingBufferLease, WorkingBufferPoolUsage};
mod capture_queue;
mod source_pool;
mod target_pool;
pub use target_pool::{TargetDescriptor, TargetLease, TargetPoolUsage};
pub use source_pool::{SourceBuffer, SourceBufferPin, SourcePoolUsage, SourceSubmission};
mod context;
mod ordered_compute;
pub use ordered_compute::OrderedCompute;
pub use capture_queue::{CaptureLease, CaptureQueueUsage};
mod fault;
pub use fault::GpuFaultListener;
mod image;
mod readback;
mod status_readback;
mod surface;
pub use context::*;
pub use image::*;
pub use readback::*;
pub use status_readback::*;
pub use surface::*;
pub use theframework_core::{AlphaMode, ColorSpace, ImageAsset, ImageHandle};
pub use wgpu;

#[derive(Debug, Clone)]
pub struct RenderError(pub String);
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RenderError {}
pub type Result<T> = std::result::Result<T, RenderError>;
