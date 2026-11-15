//! CPU sprite ownership survives camera/device changes. Only current, complete
//! recipes publish; placement and VM access stay on the application thread.
use super::*;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{
    StampRequestCapture, StampSpritePreparation, StampSpriteRequest, StampSpriteSet,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(target_arch = "wasm32"))]
use super::cpu_jobs::Slot;

type Scope = (Uuid, ProjectContext, Uuid, Option<Uuid>);
#[derive(Default)]
pub(super) struct Owner {
    scope: Option<Scope>,
    request: Option<StampSpriteRequest>,
    capture: Option<StampRequestCapture>,
    ready: Option<Arc<StampSpriteSet>>,
    ready_matches: bool,
    queued: Option<StampSpritePreparation>,
    #[cfg(not(target_arch = "wasm32"))]
    work: Option<Work>,
    failure: Option<String>,
}
impl Owner {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn scope(&mut self, scope: Scope) {
        if self.scope.as_ref() != Some(&scope) {
            self.clear();
            self.scope = Some(scope);
        }
    }

    pub fn request(
        &mut self,
        layer: &IsoPaintLayer,
    ) -> Result<Option<Arc<StampSpriteSet>>, String> {
        let same_source = self.capture.as_ref().map_or_else(
            || {
                self.request
                    .as_ref()
                    .is_some_and(|request| request.same_source(layer))
            },
            |capture| capture.same_source(layer),
        );
        if !same_source {
            self.capture = Some(StampSpriteRequest::begin_capture_with_previous(
                layer,
                self.request.as_ref(),
            ));
        }
        self.advance_capture(4096)?;
        // ready_matches still describes the last fully checked request. An
        // incomplete replacement cannot publish that artwork into its frame.
        if self.capture.is_some() {
            return Ok(None);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.ready.as_ref().filter(|_| self.ready_matches).cloned())
    }

    fn advance_capture(&mut self, items: usize) -> Result<bool, String> {
        let Some(capture) = &mut self.capture else {
            return Ok(false);
        };
        if !capture.advance(items) {
            return Ok(false);
        }
        let (request, same_visuals) = self.capture.take().unwrap().finish_with_previous()?;
        if !same_visuals {
            self.queued = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.work = None;
            }
            self.failure = None;
            self.ready_matches =
                !self.ready_matches && self.ready.as_ref().is_some_and(|set| set.matches(&request));
        }
        let busy = self.queued.is_some();
        #[cfg(not(target_arch = "wasm32"))]
        let busy = busy || self.work.is_some();
        if !self.ready_matches && !busy && self.failure.is_none() {
            let mut prep = request.clone().prepare(self.ready.clone());
            if request.is_empty() {
                prep.advance(0)?;
                self.ready = Some(Arc::new(prep.finish()?));
                self.ready_matches = true;
            } else {
                self.queued = Some(prep);
            }
        }
        self.request = Some(request);
        Ok(true)
    }

    /// An active native worker will wake its owner. Queued admission and browser
    /// cooperative work still need an application deadline until they finish.
    pub fn waits_for_wake(&self) -> bool {
        if self.capture.is_some() {
            return false;
        }
        if self.failure.is_some() {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.work.as_ref().is_some_and(|work| work.notifies)
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    pub fn failed(&self) -> bool {
        self.capture.is_none() && self.failure.is_some()
    }

    fn complete(&mut self, result: Result<StampSpriteSet, String>) {
        match result {
            Ok(set) => {
                self.ready = Some(Arc::new(set));
                self.ready_matches = true;
                self.failure = None;
            }
            Err(error) => self.failure = Some(error),
        }
    }

    pub fn advance(&mut self, wake: Option<theframework::platform::ApplicationWake>) -> bool {
        let captured = match self.advance_capture(4096) {
            Ok(changed) => changed,
            Err(error) => {
                self.failure = Some(error);
                return true;
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(work) = &self.work {
                match work.result.try_recv() {
                    Ok(result) => {
                        self.work = None;
                        self.complete(result);
                        return true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.work = None;
                        self.failure = Some("Live stamp worker stopped".into());
                        return true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => return captured,
                }
            }
            if self.capture.is_none()
                && self.queued.is_some()
                && let Some(slot) = Slot::acquire()
            {
                let prep = self.queued.take().unwrap();
                match Work::start(prep, slot, wake) {
                    Ok(work) => self.work = Some(work),
                    Err(error) => {
                        self.failure = Some(error);
                        return true;
                    }
                }
            }
            captured
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = wake;
            if self.capture.is_some() {
                return captured;
            }
            let Some(mut prep) = self.queued.take() else {
                return captured;
            };
            match prep.advance(1) {
                Ok(true) => {
                    self.complete(prep.finish());
                    true
                }
                Ok(false) => {
                    self.queued = Some(prep);
                    captured
                }
                Err(error) => {
                    self.failure = Some(error);
                    true
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct Work {
    result: std::sync::mpsc::Receiver<Result<StampSpriteSet, String>>,
    stop: Arc<AtomicBool>,
    notifies: bool,
}
#[cfg(not(target_arch = "wasm32"))]
impl Work {
    fn start(
        mut prep: StampSpritePreparation,
        slot: Slot,
        wake: Option<theframework::platform::ApplicationWake>,
    ) -> Result<Self, String> {
        let (send, result) = std::sync::mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let canceled = stop.clone();
        let notifies = wake.is_some();
        std::thread::Builder::new()
            .name("live-stamp-sources".into())
            .spawn(move || {
                let _slot = slot;
                // Even a failed kernel must publish an outcome before waking
                // the otherwise-idle application. No live state is borrowed.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    loop {
                        if canceled.load(Ordering::Acquire) {
                            return None;
                        }
                        match prep.advance(1) {
                            Ok(true) => return Some(prep.finish()),
                            Ok(false) => (),
                            Err(error) => return Some(Err(error)),
                        }
                    }
                }))
                .unwrap_or_else(|_| Some(Err("Live stamp preparation panicked".into())));
                drop(_slot);
                if !canceled.load(Ordering::Acquire)
                    && let Some(result) = result
                    && send.send(result).is_ok()
                    && let Some(wake) = wake
                {
                    wake.request_update();
                }
            })
            .map_err(|e| format!("Cannot start live stamp worker: {e}"))?;
        Ok(Self {
            result,
            stop,
            notifies,
        })
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for Work {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_pose_capture_withholds_ready_artwork_and_preserves_its_owner() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".into();
        layer.active_size = 1.;
        layer.add_stamp(shared::iso_paint::IsoPaintPoint::new(
            [8, 8],
            Some(vek::Vec3::zero()),
            None,
        ));
        let request = StampSpriteRequest::capture(&layer);
        let mut preparation = request.clone().prepare(None);
        while !preparation.advance(1).unwrap() {}
        let ready = Arc::new(preparation.finish().unwrap());
        let mut owner = Owner {
            request: Some(request),
            ready: Some(ready.clone()),
            ready_matches: true,
            ..Default::default()
        };
        let chunk = layer.chunks.values_mut().next().unwrap();
        let mut stamp = chunk.stamps[0].clone();
        stamp.world = Some([1., 2., 3.]);
        chunk.stamps = vec![stamp; 8193].into();
        assert!(owner.request(&layer).unwrap().is_none());
        assert!(!owner.waits_for_wake());
        assert!(!owner.failed());
        while owner.capture.is_some() {
            owner.advance_capture(4096).unwrap();
        }
        assert!(Arc::ptr_eq(
            &owner.request(&layer).unwrap().unwrap(),
            &ready
        ));
        assert!(owner.queued.is_none());
        layer.chunks.values_mut().next().unwrap().stamps[0].kind = "changed".into();
        assert!(owner.request(&layer).unwrap().is_none());
        layer.visible = false;
        let hidden = owner.request(&layer).unwrap().unwrap();
        assert_ne!(hidden.id(), ready.id());
        assert!(hidden.sprites().is_empty());
        assert!(owner.capture.is_none());
    }
}

impl Editor {
    pub(crate) fn advance_native_world_stamps(&mut self) -> bool {
        if !self.native_world_owns_viewport()
            || self.server_ctx.shared.editor_view_mode == EditorViewMode::D2
        {
            self.native_viewport.world.stamps.clear();
            return false;
        }
        if self
            .native_viewport
            .world
            .stamps
            .advance(self.application_wake.clone())
        {
            if let Some(gpu) = &mut self.native_viewport.world.gpu {
                gpu.scene_key = None;
            }
            true
        } else {
            false
        }
    }
}
