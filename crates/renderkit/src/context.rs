use crate::{RenderError, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransferCounters {
    pub uploaded_bytes: u64,
    pub readback_bytes: u64,
    pub capture_requests: u64,
    /// Four-byte producer status metadata; excluded from color capture bytes.
    pub status_readback_bytes: u64,
}
#[derive(Default)]
struct Counters {
    uploaded: AtomicU64,
    readback: AtomicU64,
    captures: AtomicU64,
    status: AtomicU64,
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
    lost: Arc<AtomicBool>,
    counters: Arc<Counters>,
    #[cfg(target_arch = "wasm32")]
    validation_error: Arc<std::sync::Mutex<Option<String>>>,
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
        // Browser validation is asynchronous: successful queue submission alone
        // cannot establish that a shader/pipeline was accepted. Preserve the
        // first uncaptured error for the next host poll. Native wgpu retains its
        // default synchronous validation/panic behavior.
        #[cfg(target_arch = "wasm32")]
        let validation_error = {
            let error = Arc::new(std::sync::Mutex::new(None));
            let capture = error.clone();
            device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
                capture
                    .lock()
                    .unwrap()
                    .get_or_insert_with(|| error.to_string());
            }));
            error
        };
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        device.set_device_lost_callback(move |_, _| {
            flag.store(true, Ordering::Release);
        });
        Ok(Arc::new(Self {
            lost,
            instance,
            adapter,
            device,
            queue,
            generation: NEXT_DEVICE.fetch_add(1, Ordering::Relaxed),
            counters: Arc::new(Counters::default()),
            #[cfg(target_arch = "wasm32")]
            validation_error,
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
            status_readback_bytes: self.counters.status.load(Ordering::Relaxed),
        }
    }
    pub(crate) fn record_status_readback(&self) {
        self.counters.status.fetch_add(4, Ordering::Relaxed);
    }
    pub fn record_upload(&self, bytes: u64) {
        self.counters.uploaded.fetch_add(bytes, Ordering::Relaxed);
    }
    /// Record an explicit capture request submitted by an injected renderer.
    /// `bytes` is its logical GPU readback payload, excluding row padding.
    pub fn record_capture(&self, bytes: u64) {
        self.counters.readback.fetch_add(bytes, Ordering::Relaxed);
        self.counters.captures.fetch_add(1, Ordering::Relaxed);
    }
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Drive mapping/completion callbacks without waiting for the GPU.
    pub fn poll(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        if let Some(error) = self.validation_error.lock().unwrap().as_ref() {
            return Err(RenderError(format!("WebGPU validation: {error}")));
        }
        self.device
            .poll(wgpu::PollType::Poll)
            .map(|_| ())
            .map_err(|e| RenderError(format!("device poll: {e}")))
    }
}
