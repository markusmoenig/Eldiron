//! Application-frame budget for live scene buffer transfers. Explicit offscreen
//! reference/bake callers keep their own policy; no partial buffer is displayed.
use crate::SceneVM;
use renderkit::{RenderError, Result};

pub struct SourceUploadBudget {
    bytes: usize,
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
