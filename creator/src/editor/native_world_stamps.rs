//! CPU sprite ownership survives camera/device changes. Only current, complete
//! recipes publish; placement and VM access stay on the application thread.
use super::*;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{
    StampSpritePreparation, StampSpriteRequest, StampSpriteSet,
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
        if self
            .request
            .as_ref()
            .is_none_or(|request| !request.same_source(layer))
        {
            let request = StampSpriteRequest::capture(layer);
            let same_visuals = self.request.as_ref().is_some_and(|old| old.same(&request));
            if !same_visuals {
                self.queued = None;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.work = None;
                }
                self.failure = None;
                self.ready_matches = self.ready.as_ref().is_some_and(|set| set.matches(&request));
                if !self.ready_matches {
                    let mut prep = request.clone().prepare(self.ready.clone());
                    if request.is_empty() {
                        prep.advance(0)?;
                        self.ready = Some(Arc::new(prep.finish()?));
                        self.ready_matches = true;
                    } else {
                        self.queued = Some(prep);
                    }
                }
            }
            // Keep the latest exact backing even when only placement changed.
            // Existing work/ready artwork is already proven visually equivalent.
            self.request = Some(request);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self.ready.as_ref().filter(|_| self.ready_matches).cloned())
    }

    /// An active native worker will wake its owner. Queued admission and browser
    /// cooperative work still need an application deadline until they finish.
    pub fn waits_for_wake(&self) -> bool {
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
        self.failure.is_some()
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
                    Err(std::sync::mpsc::TryRecvError::Empty) => return false,
                }
            }
            if self.queued.is_some()
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
            false
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = wake;
            let Some(mut prep) = self.queued.take() else {
                return false;
            };
            match prep.advance(1) {
                Ok(true) => {
                    self.complete(prep.finish());
                    true
                }
                Ok(false) => {
                    self.queued = Some(prep);
                    false
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

pub(super) use shared::iso_paint_render::stamp_billboards::StampUploadKey as UploadKey;

pub(super) fn upload(
    set: &StampSpriteSet,
    layer: &IsoPaintLayer,
    vm: &mut scenevm::SceneVM,
    camera: scenevm::Camera3D,
    height: u32,
    uploaded: &mut Option<UploadKey>,
) -> Result<(), String> {
    shared::iso_paint_render::stamp_billboards::upload_sprite_set(
        set, layer, vm, camera, height, uploaded,
    )
    .map(|_| ())
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
