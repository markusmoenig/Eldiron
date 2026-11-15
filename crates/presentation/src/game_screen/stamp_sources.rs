//! CPU-only stamp preparation belongs to the retained screen/session, never to
//! a GPU domain. A source change retires work without publishing old pixels.
use super::*;
use rusterix::stamp_paint::StampPaint;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{
    StampSpritePreparation, StampSpriteRequest, StampSpriteSet,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use theframework::platform::ApplicationWake;

pub(super) struct Publication {
    pub layer: Arc<IsoPaintLayer>,
    pub pixels: Arc<StampSpriteSet>,
}
#[derive(Default)]
pub(super) struct Sources {
    scope: Option<(Uuid, Uuid)>,
    source: Option<Arc<StampPaint>>,
    layer: Option<Arc<IsoPaintLayer>>,
    request: Option<StampSpriteRequest>,
    ready: Option<Arc<StampSpriteSet>>,
    queued: Option<StampSpritePreparation>,
    #[cfg(not(target_arch = "wasm32"))]
    work: Option<Work>,
    failure: Option<String>,
    pub active: bool,
    pub changed: bool,
}
impl Sources {
    pub fn scope(&mut self, scope: Option<(Uuid, Uuid)>) {
        if self.scope != scope {
            *self = Self {
                scope,
                changed: true,
                ..Default::default()
            };
        }
    }
    pub fn cancel_pending(&mut self) {
        self.queued = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.work = None;
        }
    }
    fn busy(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if self.work.is_some() {
            return true;
        }
        self.queued.is_some()
    }
    pub fn pending(&self) -> bool {
        if self.failure.is_some() {
            return false;
        }
        if self.changed || self.queued.is_some() {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        return self.work.as_ref().is_some_and(|work| !work.notifies);
        #[cfg(target_arch = "wasm32")]
        false
    }
    pub fn request(
        &mut self,
        source: Option<&Arc<StampPaint>>,
    ) -> Result<Option<Publication>, String> {
        self.active = true;
        let same = match (&self.source, source) {
            (None, None) => true,
            (Some(old), Some(next)) => Arc::ptr_eq(old, next),
            _ => false,
        };
        if !same {
            self.cancel_pending();
            self.failure = None;
            self.source = source.cloned();
            self.layer =
                source.map(|source| Arc::new(shared::stamp_paint::presentation_layer(source)));
            self.request = self
                .layer
                .as_ref()
                .map(|layer| StampSpriteRequest::capture(layer));
            if source.is_none() {
                self.ready = None;
            }
            self.changed = true;
        }
        let Some(request) = &self.request else {
            return Ok(None);
        };
        let ready = self
            .ready
            .as_ref()
            .is_some_and(|ready| ready.matches(request));
        if !ready && !self.busy() && self.failure.is_none() {
            let mut prep = request.clone().prepare(self.ready.clone());
            if request.is_empty() {
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
            .filter(|ready| ready.matches(request))
            .map(|pixels| Publication {
                layer: self.layer.as_ref().unwrap().clone(),
                pixels: pixels.clone(),
            }))
    }
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }
    fn complete(&mut self, result: Result<StampSpriteSet, String>) -> Result<(), String> {
        match result {
            Ok(ready) => {
                self.ready = Some(Arc::new(ready));
                self.changed = true;
                Ok(())
            }
            Err(error) => {
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn advance(&mut self, wake: Option<ApplicationWake>) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(work) = &self.work {
                let result = match work.result.try_recv() {
                    Ok(result) => Some(result),
                    Err(std::sync::mpsc::TryRecvError::Empty) => None,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        Some(Err("Player stamp worker stopped".into()))
                    }
                };
                if let Some(result) = result {
                    self.work = None;
                    self.complete(result)?;
                }
            }
            if self.work.is_none()
                && self.queued.is_some()
                && let Some(slot) = Slot::acquire()
            {
                match Work::start(self.queued.take().unwrap(), slot, wake) {
                    Ok(work) => self.work = Some(work),
                    Err(error) => self.complete(Err(error))?,
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = wake;
            if let Some(mut prep) = self.queued.take() {
                match prep.advance(1) {
                    Ok(true) => self.complete(prep.finish())?,
                    Ok(false) => self.queued = Some(prep),
                    Err(error) => self.complete(Err(error))?,
                }
            }
        }
        Ok(())
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
            .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
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
        wake: Option<ApplicationWake>,
    ) -> Result<Self, String> {
        let (send, result) = std::sync::mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let canceled = stop.clone();
        let notifies = wake.is_some();
        std::thread::Builder::new()
            .name("player-stamp-sources".into())
            .spawn(move || {
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
                .unwrap_or_else(|_| Some(Err("Player stamp worker panicked".into())));
                drop(slot);
                if !canceled.load(Ordering::Acquire)
                    && let Some(result) = result
                    && send.send(result).is_ok()
                    && let Some(wake) = wake
                {
                    wake.request_update();
                }
            })
            .map_err(|error| format!("Cannot start player stamp worker: {error}"))?;
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
