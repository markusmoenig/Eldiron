use renderkit::{
    GpuContext, ImageDescriptor, ImageHandle, ImageRegistry, ReadbackTicket, RenderError, Result,
};
use rustc_hash::FxHashMap;
use rusterix::SceneHandler;
use scenevm::{SceneVM, SharedAtlas, render_output::SceneRenderTarget};
use std::sync::Arc;
use theframework_core::Uuid;

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
    scene: SceneHandler,
    size: (u32, u32),
    target: Option<SceneRenderTarget>,
    output: Option<ViewImage>,
    dirty: bool,
    revision: u64,
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
}
impl SceneViews {
    pub fn new(images: &ImageRegistry, atlas: SharedAtlas) -> Self {
        Self {
            context: images.context().clone(),
            registry_owner: images.owner_id(),
            atlas,
            views: FxHashMap::default(),
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
                scene: SceneHandler::with_vm(vm),
                size,
                target,
                output: None,
                dirty: true,
                revision: 0,
            },
        );
        Ok(())
    }
    pub fn scene(&self, id: ViewId) -> Result<&SceneHandler> {
        self.views
            .get(&id)
            .map(|v| &v.scene)
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
        if size.0 > 0 && size.1 > 0 && view.target.as_ref().is_none_or(|t| t.size() != size) {
            let target = SceneRenderTarget::new(self.context.clone(), size.0, size.1)?;
            if let Some(output) = view.output.take() {
                images.remove(output.image);
            }
            view.target = Some(target);
        }
        // Zero-sized/suspended views retain their last target but produce no
        // paint output or render work. Resuming at the same size reuses it.
        view.size = size;
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
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        if view.size.0 == 0 || view.size.1 == 0 {
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
        let target = view
            .target
            .as_mut()
            .ok_or_else(|| RenderError("scene view target missing".into()))?;
        let rendered = view.scene.vm.render_gpu(target, revision)?;
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
            content_revision,
        };
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
        let target = view
            .target
            .as_ref()
            .ok_or_else(|| RenderError("scene view target missing".into()))?;
        Ok(ViewCapture {
            view: id,
            revision: output.revision,
            content_revision: output.content_revision,
            ticket: ReadbackTicket::capture(&self.context, target.image())?,
        })
    }
    pub fn remove(&mut self, id: ViewId, images: &mut ImageRegistry) -> Result<bool> {
        self.check_registry(images)?;
        if let Some(view) = self.views.remove(&id) {
            if let Some(output) = view.output {
                images.remove(output.image);
            }
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
        Ok(())
    }
    pub fn has_cpu_target_storage(&self, id: ViewId) -> Result<bool> {
        let view = self
            .views
            .get(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        Ok(view.scene.vm.render_target_cpu_bytes() > 0
            || view
                .target
                .as_ref()
                .is_some_and(|t| t.has_cpu_storage() || t.has_readback_storage()))
    }
}
