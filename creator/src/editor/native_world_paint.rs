//! Current UV/selection pages survive camera/device changes. CPU jobs never
//! borrow a document, VM, widget or renderer, and publish only complete sources.
#[cfg(not(target_arch = "wasm32"))]
use super::cpu_jobs::Slot;
use super::*;
use shared::iso_paint::IsoPaintLayer;
use shared::iso_paint_render::surface_source::{
    SurfacePaintPreparation, SurfacePaintPublication, SurfacePaintRequest,
};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "native_world_paint/capture.rs"]
mod capture;
pub(super) use capture::Capture;

/// The down-frame's immutable paint capture remains the authoring coordinate
/// system until release. Repainting those surfaces does not require another
/// displayed-output query while their replacement GPU pages are pending.
pub(super) fn captured_query(
    camera: scenevm::Camera3D,
    size: (usize, usize),
    at: Vec2<i32>,
    paint: crate::tools::ToolPaintInput,
) -> (EditorPick, crate::tools::ToolViewInput) {
    let ray = Some(scenevm::vm::cpu_ray_from_uv(&camera, size.0 as u32, size.1 as u32,
        [at.x as f32 / size.0 as f32, at.y as f32 / size.1 as f32]));
    let pick = EditorPick {
        camera, ray,
        base: paint.surface.map(|surface| (surface.geo_id, Vec3::from(surface.world),
            surface.depth, Vec3::from(surface.normal))),
        paint_geo: paint.surface.map(|surface| surface.paint_geo),
        dynamic: None, guide: None, gizmo: None,
    };
    let input = crate::tools::ToolViewInput {
        camera: Some(camera), ray, isometric_item: None,
        base: pick.base.map(|(geo, at, depth, _)| (geo, at, depth)),
        dynamic: None, paint_geo: pick.paint_geo, paint: Some(paint), rectangle: None,
    };
    (pick, input)
}

type Scope = (Uuid, ProjectContext, Uuid, Option<Uuid>);
#[derive(Default)]
pub(super) struct Owner {
    scope: Option<Scope>,
    request: Option<SurfacePaintRequest>,
    table_limit: Option<u64>,
    ready: Option<Arc<SurfacePaintPublication>>,
    queued: Option<SurfacePaintPreparation>,
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
        table_limit: u64,
    ) -> Result<Option<Arc<SurfacePaintPublication>>, String> {
        let request = SurfacePaintRequest::capture(layer);
        let ready = self
            .ready
            .as_ref()
            .is_some_and(|source| source.matches(&request));
        if self.request.as_ref().is_none_or(|old| old != &request)
            || self.table_limit != Some(table_limit) && !ready
        {
            // A device with a different metadata limit can retry a failed or
            // pending preparation. Complete CPU publications remain reusable.
            self.table_limit = Some(table_limit);
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
            let mut prep = request.clone().prepare(table_limit);
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

    fn complete(&mut self, result: Result<SurfacePaintPublication, String>) {
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
                        self.failure = Some("Live UV paint worker stopped".into());
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
            match prep.advance(64) {
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
    result: std::sync::mpsc::Receiver<Result<SurfacePaintPublication, String>>,
    stop: Arc<AtomicBool>,
    notifies: bool,
}
#[cfg(not(target_arch = "wasm32"))]
impl Work {
    fn start(
        mut prep: SurfacePaintPreparation,
        slot: Slot,
        wake: Option<theframework::platform::ApplicationWake>,
    ) -> Result<Self, String> {
        let (send, result) = std::sync::mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let canceled = stop.clone();
        let notifies = wake.is_some();
        std::thread::Builder::new()
            .name("live-uv-paint".into())
            .spawn(move || {
                let _slot = slot;
                // Even a failed kernel must publish an outcome before waking
                // the otherwise-idle application. No live state is borrowed.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    loop {
                        if canceled.load(Ordering::Acquire) {
                            return None;
                        }
                        match prep.advance(64) {
                            Ok(true) => return Some(prep.finish()),
                            Ok(false) => (),
                            Err(error) => return Some(Err(error)),
                        }
                    }
                }))
                .unwrap_or_else(|_| Some(Err("Live UV paint preparation panicked".into())));
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

impl Editor {
    pub(crate) fn advance_native_world_paint(&mut self) -> bool {
        if !self.native_world_owns_viewport()
            || self.server_ctx.shared.editor_view_mode == EditorViewMode::D2
        {
            self.native_viewport.world.uv.clear();
            self.native_viewport.world.paint_capture.clear();
            return false;
        }
        if self
            .native_viewport
            .world
            .uv
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
