//! Application-frame budget for live scene buffer transfers. Explicit offscreen
//! reference/bake callers keep their own policy; no partial buffer is displayed.
use crate::SceneVM;
use renderkit::{RenderError, Result};

pub struct SourceUploadBudget {
    bytes: usize,
    started: Option<instant::Instant>,
}
impl Default for SourceUploadBudget {
    fn default() -> Self {
        Self {
            bytes: 0,
            started: None,
        }
    }
}
impl SourceUploadBudget {
    pub fn uploaded_bytes(&self) -> usize {
        self.bytes
    }
    pub fn reset(&mut self) {
        *self = Self::default();
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
            for vm in std::iter::once(&mut self.vm).chain(self.overlay_vms.iter_mut()) {
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
            if let Some(gpu) = &mut vm.gpu {
                gpu.pending_source_upload = None;
            }
        }
    }
}
