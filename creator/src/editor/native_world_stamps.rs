//! CPU sprite ownership survives camera/device changes. Only current, complete
//! recipes publish; placement and VM access stay on the application thread.
use super::*;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{
    StampSpritePreparation, StampSpriteRequest, StampSpriteSet,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

type Scope = (Uuid, ProjectContext, Uuid, Option<Uuid>);
#[derive(Default)]
pub(super) struct Owner {
    scope: Option<Scope>,
    request: Option<StampSpriteRequest>,
    ready: Option<Arc<StampSpriteSet>>,
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
        let request = StampSpriteRequest::capture(layer);
        if self.request.as_ref().is_none_or(|old| !old.same(&request)) {
            self.queued = None;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.work = None;
            }
            self.failure = None;
            if self.ready.as_ref().is_some_and(|set| set.matches(&request)) {
                self.request = Some(request);
                return Ok(self.ready.clone());
            }
            let empty = request.is_empty();
            let mut prep = request.clone().prepare(self.ready.clone());
            self.request = Some(request);
            if empty {
                prep.advance(0)?;
                self.ready = Some(Arc::new(prep.finish()?));
            } else {
                self.queued = Some(prep);
            }
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self
            .ready
            .as_ref()
            .filter(|set| set.matches(self.request.as_ref().unwrap()))
            .cloned())
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
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
#[cfg(not(target_arch = "wasm32"))]
struct Slot;
#[cfg(not(target_arch = "wasm32"))]
impl Slot {
    fn acquire() -> Option<Self> {
        ACTIVE
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 2).then_some(n + 1)
            })
            .ok()
            .map(|_| Self)
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Release);
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

#[derive(PartialEq, Eq)]
pub(super) struct UploadKey {
    source: Uuid,
    instances: Vec<[u32; 7]>,
}

pub(super) fn upload(
    set: &StampSpriteSet,
    layer: &IsoPaintLayer,
    vm: &mut scenevm::SceneVM,
    camera: scenevm::Camera3D,
    height: u32,
    uploaded: &mut Option<UploadKey>,
) -> Result<(), String> {
    let instances = set.instances(layer, vm, camera, height)?;
    let key = UploadKey {
        source: set.id(),
        instances: instances
            .iter()
            .map(|i| {
                [
                    i.center[0].to_bits(),
                    i.center[1].to_bits(),
                    i.center[2].to_bits(),
                    i.width.to_bits(),
                    i.height.to_bits(),
                    i.sprite_index,
                    i.flags,
                ]
            })
            .collect(),
    };
    if uploaded.as_ref() == Some(&key) {
        return Ok(());
    }
    if instances.is_empty() {
        vm.execute(scenevm::Atom::ClearPaintBillboards);
    } else if uploaded
        .as_ref()
        .is_some_and(|old| old.source == key.source && !old.instances.is_empty())
    {
        vm.execute(scenevm::Atom::SetPaintBillboardInstances { instances });
    } else {
        vm.execute(scenevm::Atom::SetPaintBillboards {
            sprites: set.sprites(),
            instances,
        });
    }
    *uploaded = Some(key);
    Ok(())
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
