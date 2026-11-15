//! At most one requested CPU stamp page per frozen catalogue. Native generation
//! is off-thread with global two-worker admission; results never publish after
//! retirement. Browser work owns the same shared recipe without payload transfer.
use super::*;
use crate::iso_paint_render::StampRecipe;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::AtomicBool;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
struct Source {
    recipe: Arc<StampRecipe>,
    origin: [u32; 2],
}
impl Source {
    fn generate(&self) -> Result<PaintPagePixels, String> {
        Ok(PaintPagePixels {
            color: self.recipe.raster(self.origin, [SIZE; 2])?.into(),
            material: vec![0; PLANE_BYTES].into(),
        })
    }
}
#[derive(Default)]
pub(super) struct GeneratedPages {
    sources: std::collections::HashMap<usize, Source>,
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    pending: Option<Work>,
    ready: Option<(usize, PaintPagePixels)>,
}
impl GeneratedPages {
    pub fn insert(&mut self, page: usize, recipe: Arc<StampRecipe>, origin: [u32; 2]) {
        self.sources.insert(page, Source { recipe, origin });
    }
    pub fn poll(&self, page: usize) -> Result<Option<PaintPagePixels>, String> {
        let source = self
            .sources
            .get(&page)
            .ok_or("Missing generated stamp source")?;
        let mut state = self.state.lock().map_err(|_| "Stamp page state poisoned")?;
        if let Some((ready, pixels)) = &state.ready
            && *ready == page
        {
            return Ok(Some(pixels.clone()));
        }
        if let Some(mut work) = state.pending.take() {
            match work.poll()? {
                Some(pixels) => {
                    let completed = work.page;
                    state.ready = Some((completed, pixels.clone()));
                    if completed == page {
                        return Ok(Some(pixels));
                    }
                }
                None => {
                    state.pending = Some(work);
                    return Ok(None);
                }
            }
        }
        if let Some(work) = Work::start(page, source.clone())? {
            state.pending = Some(work);
        }
        Ok(None)
    }
    pub fn contains(&self, page: usize) -> bool {
        self.sources.contains_key(&page)
    }
}
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Slot;
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
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct Work {
    page: usize,
    result: std::sync::mpsc::Receiver<Result<PaintPagePixels, String>>,
    stop: Arc<AtomicBool>,
}
#[cfg(not(target_arch = "wasm32"))]
impl Work {
    fn start(page: usize, source: Source) -> Result<Option<Self>, String> {
        let Some(slot) = Slot::acquire() else {
            return Ok(None);
        };
        let (sender, result) = std::sync::mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        std::thread::Builder::new()
            .name("stamp-source-page".into())
            .spawn(move || {
                let _slot = slot;
                if cancelled.load(Ordering::Acquire) {
                    return;
                }
                let pixels = source.generate();
                if !cancelled.load(Ordering::Acquire) {
                    let _ = sender.send(pixels);
                }
            })
            .map_err(|error| format!("Cannot start stamp page worker: {error}"))?;
        Ok(Some(Self { page, result, stop }))
    }
    fn poll(&mut self) -> Result<Option<PaintPagePixels>, String> {
        use std::sync::mpsc::TryRecvError;
        match self.result.try_recv() {
            Ok(pixels) => pixels.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("Stamp page worker stopped".into()),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Drop for Work {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
#[cfg(target_arch = "wasm32")]
struct Work {
    page: usize,
    source: Source,
    _slot: Slot,
}
#[cfg(target_arch = "wasm32")]
impl Work {
    fn start(page: usize, source: Source) -> Result<Option<Self>, String> {
        Ok(Slot::acquire().map(|slot| Self {
            page,
            source,
            _slot: slot,
        }))
    }
    fn poll(&mut self) -> Result<Option<PaintPagePixels>, String> {
        self.source.generate().map(Some)
    }
}
