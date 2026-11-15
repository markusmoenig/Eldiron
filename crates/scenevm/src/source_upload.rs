//! Application-frame budget for live scene buffer transfers. Explicit offscreen
//! reference/bake callers keep their own policy; no partial buffer is displayed.
use crate::SceneVM;
use renderkit::{RenderError, Result};

pub struct SourceUploadBudget {
    bytes: usize,
    gpu_copied_bytes: u64,
    preparation_cpu_ns: u64,
    preparation_calls: u64,
    started: Option<instant::Instant>,
    cpu_words: usize,
    cpu_steps: usize,
    cpu_started: Option<instant::Instant>,
    unlimited_cpu: bool,
    turn: usize,
}
impl Default for SourceUploadBudget {
    fn default() -> Self {
        Self {
            bytes: 0,
            gpu_copied_bytes: 0,
            preparation_cpu_ns: 0,
            preparation_calls: 0,
            started: None,
            cpu_words: 0,
            cpu_steps: 0,
            cpu_started: None,
            unlimited_cpu: false,
            turn: 0,
        }
    }
}
impl SourceUploadBudget {
    /// Share the application-frame CPU deadline with metadata/placement owners
    /// outside SceneVM. One admitted unit must be a bounded recipe/triangle step.
    pub fn try_prepare_cpu_step(&mut self) -> bool {
        if self.available_cpu_words() == 0 {
            return false;
        }
        self.record_cpu_step();
        true
    }
    pub fn prepare_cpu<R>(&mut self, prepare: impl FnOnce(&mut Self) -> R) -> R {
        let started = instant::Instant::now();
        let result = prepare(self);
        self.record_preparation(started.elapsed().as_nanos().min(u64::MAX as u128) as u64);
        result
    }
    /// CPU wall time inside cooperative preparation, including driver queueing;
    /// this is not GPU execution time or total application-frame CPU time.
    pub fn preparation_cpu_ns(&self) -> u64 {
        self.preparation_cpu_ns
    }
    pub fn preparation_calls(&self) -> u64 {
        self.preparation_calls
    }
    fn record_preparation(&mut self, elapsed: u64) {
        self.preparation_cpu_ns = self.preparation_cpu_ns.saturating_add(elapsed);
        self.preparation_calls = self.preparation_calls.saturating_add(1);
    }
    pub fn packed_cpu_bytes(&self) -> usize {
        self.cpu_words * 4
    }
    pub(crate) fn unlimited_cpu() -> Self {
        Self {
            unlimited_cpu: true,
            ..Default::default()
        }
    }
    pub(crate) fn available_cpu_words(&mut self) -> usize {
        if self.unlimited_cpu {
            return usize::MAX;
        }
        let started = self.cpu_started.get_or_insert_with(instant::Instant::now);
        if started.elapsed().as_millis() >= 2 {
            return 0;
        }
        (128 * 1024usize).saturating_sub(self.cpu_words.saturating_add(self.cpu_steps))
    }
    pub(crate) fn record_cpu_words(&mut self, words: usize) {
        self.cpu_words += words;
    }
    pub(crate) fn record_cpu_step(&mut self) {
        self.cpu_steps += 1;
    }
    /// Device-local retained-source copies, separate from CPU transfer quota.
    pub fn gpu_copied_bytes(&self) -> u64 {
        self.gpu_copied_bytes
    }
    pub(crate) fn record_gpu_copy(&mut self, bytes: u64) {
        self.gpu_copied_bytes = self.gpu_copied_bytes.saturating_add(bytes);
    }
    pub fn uploaded_bytes(&self) -> usize {
        self.bytes
    }
    pub fn reset(&mut self) {
        *self = Self {
            turn: self.turn.wrapping_add(1),
            ..Self::default()
        };
    }
    pub(crate) fn available(&mut self) -> usize {
        let started = self.started.get_or_insert_with(instant::Instant::now);
        if started.elapsed().as_millis() >= 2 {
            return 0;
        }
        (512 * 1024usize).saturating_sub(self.bytes) & !3
    }
    pub(crate) fn record(&mut self, bytes: usize) {
        self.bytes += bytes;
    }
}
impl SceneVM {
    /// Queue bounded writes before rendering a live view. All layers share the
    /// caller's frame budget. Completed old output textures remain untouched.
    pub fn prepare_source_uploads(&mut self, budget: &mut SourceUploadBudget) -> Result<bool> {
        let context = self
            .vm
            .target_context
            .as_ref()
            .ok_or_else(|| {
                RenderError("live source uploads require an injected GPU context".into())
            })?
            .clone();
        if context.is_lost() || self.device_generation() != Some(context.generation()) {
            return Err(RenderError(
                "scene source device was lost or replaced".into(),
            ));
        }
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError("scene GPU is not initialized".into()))?;
        let started = instant::Instant::now();
        let before = budget.uploaded_bytes();
        let result = (|| {
            let mut ready = true;
            let count = self.overlay_vms.len() + 1;
            for offset in 0..count {
                let index = (budget.turn % count + offset) % count;
                let vm = if index == 0 {
                    &mut self.vm
                } else {
                    &mut self.overlay_vms[index - 1]
                };
                ready &= vm
                    .prepare_organic_uploads(&gpu.device, &gpu.queue, budget)
                    .map_err(|error| RenderError(format!("scene source upload: {error:?}")))?;
            }
            Ok(ready)
        })();
        context.record_upload((budget.uploaded_bytes() - before) as u64);
        budget.record_preparation(started.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64);
        result
    }
    pub fn cancel_source_uploads(&mut self) {
        for vm in std::iter::once(&mut self.vm).chain(self.overlay_vms.iter_mut()) {
            vm.cancel_organic_preparation();
            if let Some(gpu) = &mut vm.gpu {
                gpu.pending_source_upload = None;
            }
        }
    }
}
