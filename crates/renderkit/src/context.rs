use crate::{RenderError, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransferCounters {
    /// Instrumented source/image transfers. Immutable draw inputs have separate
    /// actual-upload counters in BufferCacheUsage.
    pub uploaded_bytes: u64,
    pub readback_bytes: u64,
    pub capture_requests: u64,
    /// Four-byte producer status metadata; excluded from color capture bytes.
    pub status_readback_bytes: u64,
    /// Explicit editable/exportable asset production, also included in readback_bytes.
    pub asset_bake_readback_bytes: u64,
    pub asset_bake_readback_requests: u64,
}
#[derive(Default)]
struct Counters {
    uploaded: AtomicU64,
    readback: AtomicU64,
    captures: AtomicU64,
    status: AtomicU64,
    asset_bytes: AtomicU64,
    asset_requests: AtomicU64,
    image_allocations: AtomicU64,
}
/// Unpooled images allocated by GpuImage::new, retained through their last Arc
/// owner. Bytes estimate texel storage, excluding driver overhead, imported raw
/// textures, pooled targets and raw handles retained only by queued commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageAllocationUsage {
    pub live_bytes: u64,
    pub live_images: u64,
    pub peak_live_bytes: u64,
    pub allocations: u64,
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
    loss_reason: Arc<std::sync::OnceLock<String>>,
    faults: Arc<crate::fault::Notifications>,
    counters: Arc<Counters>,
    capture_queue: Arc<crate::capture_queue::CaptureQueue>,
    source_pool: Arc<crate::source_pool::SourcePool>,
    target_pool: Arc<crate::target_pool::TargetPool>,
    buffer_cache: Arc<crate::buffer_cache::BufferCache>,
    working_buffers: Arc<crate::working_buffers::WorkingBuffers>,
    image_allocations: Arc<crate::allocation_usage::AllocationUsage>,
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
                // The shared pipelines fit within 15 inter-stage variables.
                // Metal on iOS/simulator exposes 15 even when all of our other
                // required limits are supported; WebGPU's default requests 16.
                required_limits: wgpu::Limits {
                    max_inter_stage_shader_variables: 15,
                    ..wgpu::Limits::default()
                },
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError(format!("request device: {e}")))?;
        // Browser validation is asynchronous: successful queue submission alone
        // cannot establish that a shader/pipeline was accepted. Preserve the
        // first uncaptured error for the next host poll. Native wgpu retains its
        // default synchronous validation/panic behavior.
        let faults = Arc::new(crate::fault::Notifications::default());
        #[cfg(target_arch = "wasm32")]
        let validation_error = {
            let error = Arc::new(std::sync::Mutex::new(None));
            let capture = error.clone();
            let notify = faults.clone();
            device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
                capture
                    .lock()
                    .unwrap()
                    .get_or_insert_with(|| error.to_string());
                notify.signal();
            }));
            error
        };
        let lost = Arc::new(AtomicBool::new(false));
        let loss_reason = Arc::new(std::sync::OnceLock::new());
        let report = loss_reason.clone();
        let flag = lost.clone();
        let notify = faults.clone();
        device.set_device_lost_callback(move |reason, message| {
            let _ = report.set(format!("{reason:?}: {message}"));
            flag.store(true, Ordering::Release);
            notify.signal();
        });
        Ok(Arc::new(Self {
            lost,
            loss_reason,
            faults,
            instance,
            adapter,
            device,
            queue,
            generation: NEXT_DEVICE.fetch_add(1, Ordering::Relaxed),
            counters: Arc::new(Counters::default()),
            capture_queue: Arc::new(crate::capture_queue::CaptureQueue::default()),
            source_pool: Arc::new(crate::source_pool::SourcePool::default()),
            target_pool: Arc::new(crate::target_pool::TargetPool::default()),
            buffer_cache: Arc::new(crate::buffer_cache::BufferCache::default()),
            working_buffers: Arc::new(crate::working_buffers::WorkingBuffers::default()),
            image_allocations: Arc::new(crate::allocation_usage::AllocationUsage::default()),
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
        // Sample the asset partition once so the total and its labeled subset
        // remain consistent while a native worker submits another bake.
        let asset_bytes = self.counters.asset_bytes.load(Ordering::Relaxed);
        let asset_requests = self.counters.asset_requests.load(Ordering::Relaxed);
        TransferCounters {
            uploaded_bytes: self.counters.uploaded.load(Ordering::Relaxed),
            readback_bytes: self.counters.readback.load(Ordering::Relaxed) + asset_bytes,
            capture_requests: self.counters.captures.load(Ordering::Relaxed) + asset_requests,
            status_readback_bytes: self.counters.status.load(Ordering::Relaxed),
            asset_bake_readback_bytes: asset_bytes,
            asset_bake_readback_requests: asset_requests,
        }
    }
    /// Outstanding explicit color/depth staging, shared by cloned contexts.
    /// Four-byte producer metadata uses its separate bounded status protocol.
    pub fn capture_queue_usage(&self) -> crate::CaptureQueueUsage {
        self.capture_queue.usage()
    }
    /// Reserve actual staging bytes before allocation. A submitted producer must
    /// retain the lease until both GPU completion and consumer retirement.
    pub fn reserve_capture(&self, bytes: u64) -> Result<crate::CaptureLease> {
        if self.is_lost() {
            return Err(RenderError("capture device is lost".into()));
        }
        self.capture_queue.reserve(bytes)
    }
    /// Retained targets shared by all renderers in this device domain.
    pub fn target_pool_usage(&self) -> crate::TargetPoolUsage {
        self.target_pool.usage()
    }
    pub fn image_allocation_usage(&self) -> ImageAllocationUsage {
        let (live_bytes, live_images, peak_live_bytes) = self.image_allocations.snapshot();
        ImageAllocationUsage {
            live_bytes,
            live_images,
            peak_live_bytes,
            allocations: self.counters.image_allocations.load(Ordering::Relaxed),
        }
    }
    pub(crate) fn charge_image(&self, bytes: u64) -> crate::allocation_usage::AllocationCharge {
        self.counters
            .image_allocations
            .fetch_add(1, Ordering::Relaxed);
        self.image_allocations.charge(bytes)
    }
    pub fn set_target_cache_limits(&self, bytes: u64, textures: usize) {
        self.target_pool.set_limits(bytes, textures);
    }
    /// Reused contents are unspecified. Hold the lease until all uses have been
    /// submitted on this queue, and clear/overwrite before publishing the image.
    pub fn acquire_target(&self, descriptor: crate::TargetDescriptor) -> Result<crate::TargetLease> {
        self.target_pool.acquire(self, descriptor)
    }
    pub fn working_buffer_pool_usage(&self) -> crate::WorkingBufferPoolUsage {
        self.working_buffers.usage()
    }
    pub fn set_working_buffer_cache_limits(&self, bytes: u64, buffers: usize) {
        self.working_buffers.set_limits(bytes, buffers);
    }
    /// Reused storage is uninitialized. Retain leases through queue submission
    /// and fully initialize before any shader/consumer reads its contents.
    pub fn acquire_working_buffer(
        &self,
        descriptor: crate::WorkingBufferDescriptor,
    ) -> Result<crate::WorkingBufferLease> {
        self.working_buffers.acquire(self, descriptor)
    }
    /// Shared immutable paint inputs, with separate geometry/uniform upload counters.
    pub fn immutable_buffer_cache_usage(&self) -> crate::BufferCacheUsage {
        self.buffer_cache.usage()
    }
    pub fn set_immutable_buffer_cache_limits(&self, bytes: u64, entries: usize) {
        self.buffer_cache.set_limits(bytes, entries);
    }
    /// Exact immutable contents can be shared before prior draws submit. The
    /// cache bounds retained GPU plus CPU content bytes, not total driver memory.
    pub fn upload_immutable_buffer(&self, bytes: &[u8], usage: wgpu::BufferUsages) -> Result<Arc<crate::GpuBuffer>> {
        self.buffer_cache.upload(self, bytes, usage)
    }
    pub fn source_pool_usage(&self) -> crate::SourcePoolUsage {
        self.source_pool.usage()
    }
    pub fn try_source_buffer(
        &self,
        bytes: u64,
        data: Option<&[u8]>,
    ) -> Option<Arc<crate::SourceBuffer>> {
        if self.is_lost()
            || bytes % 4 != 0
            || bytes > u64::from(self.device.limits().max_storage_buffer_binding_size)
            || data.is_some_and(|data| data.len() as u64 > bytes)
        {
            return None;
        }
        self.source_pool.buffer(&self.device, bytes, data)
    }
    pub fn touch_source_buffer(&self, buffer: &Arc<crate::SourceBuffer>) {
        self.source_pool.touch(buffer);
    }
    pub fn try_source_submission(&self, upload_bytes: u64) -> Option<crate::SourceSubmission> {
        if self.is_lost() {
            return None;
        }
        self.source_pool.submission(upload_bytes)
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
    /// Label a CPU asset bake independently from viewport/export captures.
    pub fn record_asset_bake(&self, bytes: u64) {
        self.counters
            .asset_bytes
            .fetch_add(bytes, Ordering::Relaxed);
        self.counters.asset_requests.fetch_add(1, Ordering::Relaxed);
    }
    /// Wake a host even when no redraw or simulation update is scheduled. This
    /// registers no thread or event loop and retains the listener only weakly.
    pub fn observe_faults(
        &self,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> crate::GpuFaultListener {
        self.faults.observe(wake)
    }
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
    /// The driver's reason for losing this device domain, when reported.
    pub fn device_loss_reason(&self) -> Option<&str> {
        self.loss_reason.get().map(String::as_str)
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
