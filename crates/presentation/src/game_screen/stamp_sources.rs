//! CPU-only stamp preparation belongs to the retained screen/session, never to
//! a GPU domain. A source change retires work without publishing old pixels.
use super::*;
use rusterix::stamp_paint::StampPaint;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::stamp_billboards::{
    StampRequestCapture, StampSpritePreparation, StampSpriteRequest, StampSpriteSet,
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
    layer_preparation: Option<shared::stamp_paint::StampLayerPreparation>,
    capture: Option<StampRequestCapture>,
    request: Option<StampSpriteRequest>,
    ready: Option<Arc<StampSpriteSet>>,
    ready_matches: bool,
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
        if self.layer_preparation.is_some() || self.capture.is_some() {
            // An inactive screen may return with the same published Arc. Force
            // adaptation to restart rather than matching an abandoned capture.
            self.source = None;
            self.layer = None;
            self.request = None;
            self.ready_matches = false;
        }
        self.layer_preparation = None;
        self.capture = None;
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
        if self.layer_preparation.is_some() || self.capture.is_some() {
            return true;
        }
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
            self.layer = None;
            if let Some(source) = source {
                self.layer_preparation = Some(shared::stamp_paint::StampLayerPreparation::new(
                    source.clone(),
                ));
            } else {
                self.request = None;
                self.ready = None;
                self.ready_matches = false;
            }
            self.changed = true;
        }
        self.advance_capture(4096)?;
        if self.layer_preparation.is_some() || self.capture.is_some() {
            return Ok(None);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(self
            .ready
            .as_ref()
            .filter(|_| self.ready_matches)
            .map(|pixels| Publication {
                layer: self.layer.as_ref().unwrap().clone(),
                pixels: pixels.clone(),
            }))
    }

    fn advance_capture(&mut self, items: usize) -> Result<(), String> {
        if let Some(preparation) = &mut self.layer_preparation {
            if !preparation.advance(items) {
                return Ok(());
            }
            let layer = Arc::new(self.layer_preparation.take().unwrap().finish()?);
            self.capture = Some(StampSpriteRequest::begin_capture_with_previous(
                &layer,
                self.request.as_ref(),
            ));
            self.layer = Some(layer);
            // Adaptation and recipe validation each get a separate turn.
            self.changed = true;
            return Ok(());
        }
        if let Some(capture) = &mut self.capture {
            if !capture.advance(items) {
                return Ok(());
            }
            let (request, same_visuals) = self.capture.take().unwrap().finish_with_previous()?;
            if !same_visuals {
                self.ready_matches = !self.ready_matches
                    && self.ready.as_ref().is_some_and(|set| set.matches(&request));
            }
            self.request = Some(request);
            self.changed = true;
        }
        let Some(request) = &self.request else {
            return Ok(());
        };
        if !self.ready_matches && !self.busy() && self.failure.is_none() {
            let mut prep = request.clone().prepare(self.ready.clone());
            if request.is_empty() {
                prep.advance(0)?;
                self.ready = Some(Arc::new(prep.finish()?));
                self.ready_matches = true;
            } else {
                self.queued = Some(prep);
            }
        }
        Ok(())
    }
    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }
    fn complete(&mut self, result: Result<StampSpriteSet, String>) -> Result<(), String> {
        match result {
            Ok(ready) => {
                self.ready = Some(Arc::new(ready));
                self.ready_matches = true;
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
        self.advance_capture(4096)?;
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
                && self.layer_preparation.is_none()
                && self.capture.is_none()
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
            if self.layer_preparation.is_none()
                && self.capture.is_none()
                && let Some(mut prep) = self.queued.take()
            {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abandoned_layer_restarts_and_pose_only_changes_reuse_artwork() {
        let recipe = rusterix::stamp_paint::StampRecipe {
            id: Uuid::new_v4(),
            kind: "grass".into(),
            variant: "default".into(),
            world: Some([0.; 3]),
            surface_uv: None,
            paint_geo: None,
            size: 1.,
            camera_scale: None,
            viewport_size: None,
            rotation: 0.,
            variation: 0,
            material_id: 0,
            material_override: false,
            color: [255; 4],
            palette_colors: Vec::new(),
            opacity: 1.,
            source_image: None,
            source_material_pass: false,
        };
        let initial = Arc::new(StampPaint {
            stamps: vec![recipe.clone()],
        });
        let layer = Arc::new(shared::stamp_paint::presentation_layer(&initial));
        let request = StampSpriteRequest::capture(&layer);
        let mut preparation = request.clone().prepare(None);
        while !preparation.advance(1).unwrap() {}
        let ready = Arc::new(preparation.finish().unwrap());
        let mut owner = Sources {
            source: Some(initial),
            layer: Some(layer),
            request: Some(request),
            ready: Some(ready.clone()),
            ready_matches: true,
            ..Default::default()
        };
        let mut moved = recipe;
        moved.world = Some([1., 2., 3.]);
        let replacement = Arc::new(StampPaint {
            stamps: vec![moved; 8193],
        });
        assert!(owner.request(Some(&replacement)).unwrap().is_none());
        assert!(owner.pending());
        owner.cancel_pending();
        assert!(!owner.has_source());
        assert!(owner.request(Some(&replacement)).unwrap().is_none());
        for _ in 0..10000 {
            if owner.layer_preparation.is_none() && owner.capture.is_none() {
                break;
            }
            owner.advance_capture(4096).unwrap();
        }
        let publication = owner.request(Some(&replacement)).unwrap().unwrap();
        assert!(Arc::ptr_eq(&publication.pixels, &ready));
        assert_eq!(
            publication
                .layer
                .chunks
                .values()
                .next()
                .unwrap()
                .stamps
                .len(),
            8193
        );
        assert!(!owner.busy());
        assert!(owner.request(None).unwrap().is_none());
        assert!(owner.ready.is_none());
        assert!(owner.request(Some(&replacement)).unwrap().is_none());
        assert!(owner.request(None).unwrap().is_none());
        assert!(owner.capture.is_none() && owner.layer_preparation.is_none());
    }
}
