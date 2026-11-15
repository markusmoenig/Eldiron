use renderkit::{
    GpuContext, ImageDescriptor, ImageHandle, ImageRegistry, ReadbackTicket, RenderError, Result,
};
use rustc_hash::FxHashMap;
use rusterix::SceneHandler;
use scenevm::{SceneVM, SharedAtlas, render_output::SceneRenderTarget};
use std::sync::Arc;
use theframework_core::Uuid;
mod editor;
mod owner;
use owner::SceneOwner;
pub(crate) use owner::GamePaintScope;
mod isometric;
mod picking;
mod surface_paint;
mod stamp_paint;
pub use surface_paint::SurfacePaintStats;
mod surface_output;
pub use picking::{EditorPaintCapture, EditorPaintPick, EditorPick, EditorSurfacePick, ViewPick, ViewPickHit};

/// Stable application/controller identity, independent of window/device lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(pub Uuid);
impl ViewId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
impl Default for ViewId {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewImage {
    pub view: ViewId,
    pub image: ImageHandle,
    pub descriptor: ImageDescriptor,
    /// Successful output revision, independent of the document's revision.
    pub revision: u64,
    pub content_revision: u64,
}
/// Only an explicit capture allocates staging/CPU output. The stamp identifies
/// the exact scene result copied, even if the view resizes or is removed later.
pub struct ViewCapture {
    pub view: ViewId,
    pub revision: u64,
    pub content_revision: u64,
    pub ticket: ReadbackTicket,
}
struct SceneView {
    scene: SceneOwner,
    editor_source: Option<Arc<rusterix::scene_build_index::SceneBuildSnapshot>>,
    surface_paint_source: Option<Arc<rusterix::surface_paint::SurfacePaint>>,
    installed_surface_paint_source: Option<Arc<rusterix::surface_paint::SurfacePaint>>,
    surface_paint_ready: bool,
    stamp_paint_ready: bool,
    scene_sources_ready: bool,
    stamp_uploaded: Option<shared::iso_paint_render::stamp_billboards::StampUploadKey>,
    isometric: bool,
    isometric_renderer: Option<crate::isometric::IsometricSceneRenderer>,
    isometric_frame: Option<crate::isometric::IsometricPreparedFrame>,
    last_image: Option<Arc<renderkit::GpuImage>>,
    size: (u32, u32),
    output: Option<ViewImage>,
    dirty: bool,
    revision: u64,
    pending_content_revision: Option<u64>,
    game_scope: Option<GamePaintScope>,
}
/// Each view retains its own VM/camera/history and target. Immutable atlas source
/// data and its GPU textures are shared on the host device. UI only sees handles.
/// The application constructs this owner from the presenter's registry domain;
/// no view can replace a window runner's VM or initialize a second device.
pub struct SceneViews {
    context: Arc<GpuContext>,
    registry_owner: u64,
    atlas: SharedAtlas,
    views: FxHashMap<ViewId, SceneView>,
    surface_paint: surface_paint::Sources,
    source_upload_budget: scenevm::source_upload::SourceUploadBudget,
}
impl SceneViews {
    pub fn new(images: &ImageRegistry, atlas: SharedAtlas) -> Self {
        Self {
            context: images.context().clone(),
            registry_owner: images.owner_id(),
            atlas,
            views: FxHashMap::default(),
            surface_paint: Default::default(),
            source_upload_budget: Default::default(),
        }
    }
    pub fn context(&self) -> &Arc<GpuContext> {
        &self.context
    }
    pub fn atlas(&self) -> &SharedAtlas {
        &self.atlas
    }
    fn check_registry(&self, images: &ImageRegistry) -> Result<()> {
        if images.owner_id() != self.registry_owner
            || images.context().generation() != self.context.generation()
        {
            return Err(RenderError(
                "scene views belong to a different image/device domain".into(),
            ));
        }
        Ok(())
    }
    fn check_size(&self, size: (u32, u32)) -> Result<()> {
        let limit = self.context.device.limits().max_texture_dimension_2d;
        if size.0 > limit || size.1 > limit {
            return Err(RenderError("scene view exceeds target limits".into()));
        }
        Ok(())
    }
    pub fn create(&mut self, id: ViewId, size: (u32, u32)) -> Result<()> {
        self.check_size(size)?;
        if self.views.contains_key(&id) {
            return Err(RenderError("scene view already exists".into()));
        }
        let vm =
            SceneVM::new_with_context_and_atlas(self.context.clone(), 1, 1, self.atlas.clone());
        let target = if size.0 > 0 && size.1 > 0 {
            Some(SceneRenderTarget::new(
                self.context.clone(),
                size.0,
                size.1,
            )?)
        } else {
            None
        };
        self.views.insert(
            id,
            SceneView {
                scene: SceneOwner::new(SceneHandler::with_vm(vm), target),
                editor_source: None,
                surface_paint_source: None,
                installed_surface_paint_source: None,
                surface_paint_ready: true,
                stamp_paint_ready: true,
                scene_sources_ready: true,
                stamp_uploaded: None,
                isometric: false,
                isometric_renderer: None,
                isometric_frame: None,
                last_image: None,
                size,
                output: None,
                dirty: true,
                revision: 0,
                pending_content_revision: None,
                game_scope: None,
            },
        );
        Ok(())
    }
    pub fn scene(&self, id: ViewId) -> Result<&SceneHandler> {
        self.views
            .get(&id)
            .map(|v| &*v.scene)
            .ok_or_else(|| RenderError("unknown scene view".into()))
    }
    /// Preparing/mutating a view invalidates only that view. Document, session,
    /// camera controllers and job revisions remain owned by the application.
    pub fn scene_mut(&mut self, id: ViewId) -> Result<&mut SceneHandler> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        view.dirty = true;
        view.game_scope = None;
        view.scene_sources_ready = true;
        view.pending_content_revision = None;
        view.isometric_frame = None;
        Ok(&mut view.scene)
    }
    pub fn invalidate(&mut self, id: ViewId) -> Result<()> {
        self.views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?
            .dirty = true;
        Ok(())
    }
    pub fn resize(
        &mut self,
        id: ViewId,
        size: (u32, u32),
        images: &mut ImageRegistry,
    ) -> Result<()> {
        self.check_registry(images)?;
        self.check_size(size)?;
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view.size == size {
            return Ok(());
        }
        view.scene.cancel();
        view.game_scope = None;
        view.scene_sources_ready = true;
        view.pending_content_revision = None;
        view.scene.vm.cancel_source_uploads();
        let replace_target = if view.isometric {
            view.last_image
                .as_ref()
                .is_some_and(|image| (image.descriptor.width, image.descriptor.height) != size)
        } else {
            view.scene.target
                .as_ref()
                .is_none_or(|target| target.size() != size)
        };
        if size.0 > 0 && size.1 > 0 && replace_target {
            let target = if view.isometric {
                None
            } else {
                Some(SceneRenderTarget::new(
                    self.context.clone(),
                    size.0,
                    size.1,
                )?)
            };
            if let Some(output) = view.output.take() {
                images.remove(output.image);
            }
            view.last_image = None;
            view.scene.target = target;
        }
        // Zero-sized/suspended views retain their last target but produce no
        // paint output or render work. Resuming at the same size reuses it.
        view.size = size;
        if size.0 == 0 || size.1 == 0 {
            view.scene.retire_paint();
            view.scene.vm.cancel_source_uploads();
            view.scene_sources_ready = true;
        }
        view.dirty = true;
        Ok(())
    }
    pub fn output(&self, id: ViewId) -> Option<ViewImage> {
        self.views
            .get(&id)
            .filter(|v| v.size.0 > 0 && v.size.1 > 0)
            .and_then(|v| v.output)
    }
    pub fn render(
        &mut self,
        id: ViewId,
        content_revision: u64,
        images: &mut ImageRegistry,
    ) -> Result<Option<ViewImage>> {
        self.check_registry(images)?;
        if self.context.is_lost() {
            return Err(RenderError(
                "scene device lost; rebuild presentation from application state".into(),
            ));
        }
        self.advance_surface_paint_uploads()?;
        self.prepare_scene_source_uploads(id)?;
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view.size.0 == 0 || view.size.1 == 0 {
            return Ok(None);
        }
        if !view.surface_paint_ready || !view.stamp_paint_ready || !view.scene_sources_ready {
            view.pending_content_revision.get_or_insert(content_revision);
            return Ok(None);
        }
        if !view.dirty
            && view
                .output
                .is_some_and(|o| o.content_revision == content_revision)
        {
            return Ok(view.output);
        }
        let revision = view
            .revision
            .checked_add(1)
            .ok_or_else(|| RenderError("scene output revision exhausted".into()))?;
        let mut completed_content_revision = view.pending_content_revision.unwrap_or(content_revision);
        let rendered = if view.isometric {
            let frame = view
                .isometric_frame
                .as_ref()
                .ok_or_else(|| RenderError("isometric frame missing".into()))?;
            if frame.size() != view.size {
                return Err(RenderError("isometric frame size is stale".into()));
            }
            view.isometric_renderer
                .as_mut()
                .ok_or_else(|| RenderError("isometric producer missing".into()))?
                .render_prepared(frame, revision)?
        } else if view.scene.pending() || view.scene.pages.is_some() {
            if !view.scene.pending() {
                view.pending_content_revision.get_or_insert(content_revision);
                view.scene.start(self.context.clone(), revision)?;
            }
            let before = view.scene.residency_usage().unwrap();
            let result = view.scene.advance(self.surface_paint.available_frame_bytes(), &mut self.source_upload_budget);
            let after = view.scene.residency_usage().unwrap();
            self.surface_paint.record_residency_uploads(before, after);
            let Some(output) = result? else { return Ok(None); };
            completed_content_revision = view.pending_content_revision.take().unwrap_or(content_revision);
            output
        } else {
            let (scene, target) = view.scene.parts_mut();
            scene.vm.render_gpu(target.ok_or_else(|| RenderError("scene view target missing".into()))?, revision)?
        };
        let image = if let Some(output) = view.output {
            if images.get(output.image).is_none() {
                return Err(RenderError(
                    "scene view image was removed outside its owner".into(),
                ));
            }
            output.image
        } else {
            images.insert(rendered.image.clone())?
        };
        let output = ViewImage {
            view: id,
            image,
            descriptor: rendered.image.descriptor,
            revision,
            content_revision: completed_content_revision,
        };
        view.pending_content_revision = None;
        view.last_image = Some(rendered.image);
        view.revision = revision;
        view.output = Some(output);
        view.dirty = false;
        Ok(Some(output))
    }
    pub fn capture(&self, id: ViewId) -> Result<ViewCapture> {
        let view = self
            .views
            .get(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let output = self
            .output(id)
            .ok_or_else(|| RenderError("scene view has no visible rendered output".into()))?;
        let image = view
            .last_image
            .as_ref()
            .ok_or_else(|| RenderError("scene image missing".into()))?;
        Ok(ViewCapture {
            view: id,
            revision: output.revision,
            content_revision: output.content_revision,
            ticket: ReadbackTicket::capture(&self.context, image.clone())?,
        })
    }
    pub fn remove(&mut self, id: ViewId, images: &mut ImageRegistry) -> Result<bool> {
        self.check_registry(images)?;
        if let Some(view) = self.views.remove(&id) {
            if let Some(output) = view.output {
                images.remove(output.image);
            }
            self.retire_unused_surface_paint();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// Release registrations before switching projects while retaining the host.
    pub fn clear(&mut self, images: &mut ImageRegistry) -> Result<()> {
        self.check_registry(images)?;
        for view in self.views.drain().map(|(_, v)| v) {
            if let Some(output) = view.output {
                images.remove(output.image);
            }
        }
        self.retire_unused_surface_paint();
        Ok(())
    }
    pub fn has_cpu_target_storage(&self, id: ViewId) -> Result<bool> {
        let view = self
            .views
            .get(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        Ok(view
            .isometric_renderer
            .as_ref()
            .is_some_and(|r| r.has_cpu_storage() || r.has_readback_storage())
            || view.scene.has_cpu_target_storage())
    }
}
