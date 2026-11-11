use crate::{RenderError, Result};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransferCounters {
    pub uploaded_bytes: u64,
    pub readback_bytes: u64,
    pub capture_requests: u64,
}
#[derive(Default)]
struct Counters {
    uploaded: AtomicU64,
    readback: AtomicU64,
    captures: AtomicU64,
}
static NEXT_DEVICE: AtomicU64 = AtomicU64::new(1);
/// A device domain. Creating a replacement context invalidates old GPU resources.
#[derive(Clone)]
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    generation: u64,
    counters: Arc<Counters>,
}
impl GpuContext {
    pub async fn request(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Arc<Self>> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RenderError(format!("request adapter: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("renderkit-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError(format!("request device: {e}")))?;
        Ok(Arc::new(Self {
            instance,
            adapter,
            device,
            queue,
            generation: NEXT_DEVICE.fetch_add(1, Ordering::Relaxed),
            counters: Arc::new(Counters::default()),
        }))
    }
    pub async fn headless() -> Result<Arc<Self>> {
        Self::request(
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env()),
            None,
        )
        .await
    }
    pub fn transfers(&self) -> TransferCounters {
        TransferCounters {
            uploaded_bytes: self.counters.uploaded.load(Ordering::Relaxed),
            readback_bytes: self.counters.readback.load(Ordering::Relaxed),
            capture_requests: self.counters.captures.load(Ordering::Relaxed),
        }
    }
    pub fn record_upload(&self, bytes: u64) {
        self.counters.uploaded.fetch_add(bytes, Ordering::Relaxed);
    }
    pub(crate) fn record_capture(&self, bytes: u64) {
        self.counters.readback.fetch_add(bytes, Ordering::Relaxed);
        self.counters.captures.fetch_add(1, Ordering::Relaxed);
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Drive mapping/completion callbacks without waiting for the GPU.
    pub fn poll(&self) -> Result<()> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map(|_| ())
            .map_err(|e| RenderError(format!("device poll: {e}")))
    }
}
