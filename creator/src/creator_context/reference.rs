//! Application-local compatibility facade for CPU assets/settings and remaining
//! reference renderers. Canonical native views own their explicit GPU context.
//! Lazy construction preserves data-only tools. Owned scoped guards let a caller
//! update disjoint CPU authoring state without borrowing the complete context.
use super::reference_minimap::ReferenceMinimap;
use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock};
use rusterix::Rusterix;
use std::{
    cell::{OnceCell, Ref, RefCell, RefMut},
    sync::Arc,
};
use theframework::prelude::ThePalette;

#[derive(Default)]
pub struct CreatorReference {
    source: OnceCell<Arc<RwLock<Rusterix>>>,
    minimap: RefCell<ReferenceMinimap>,
    authoring_palette: RefCell<ThePalette>,
    #[cfg(feature = "native-render")]
    context: std::cell::RefCell<Option<Arc<renderkit::GpuContext>>>,
}
impl CreatorReference {
    /// Compatibility selection snapshot for map-only Tool/Action APIs.
    /// Project palette data remains authored; reading this never creates a GPU.
    pub(crate) fn authoring_palette(&self) -> Ref<'_, ThePalette> {
        self.authoring_palette.borrow()
    }
    pub(crate) fn publish_palette(&self, palette: &ThePalette) {
        self.authoring_palette.borrow_mut().clone_from(palette);
    }

    /// These CPU cache borrows never initialize the compatibility engine/GPU.
    pub(crate) fn minimap(&self) -> Ref<'_, ReferenceMinimap> {
        self.minimap.borrow()
    }
    pub(crate) fn minimap_mut(&self) -> RefMut<'_, ReferenceMinimap> {
        self.minimap.borrow_mut()
    }
    pub(crate) fn clear_minimap(&self) {
        *self.minimap.borrow_mut() = ReferenceMinimap::default();
    }

    fn source(&self) -> Arc<RwLock<Rusterix>> {
        self.source
            .get_or_init(|| {
                #[cfg(feature = "native-render")]
                if let Some(context) = self.context.borrow().as_ref() {
                    let vm = scenevm::SceneVM::new_with_context(context.clone(), 100, 100);
                    return Arc::new(RwLock::new(Rusterix::with_scene_handler(
                        rusterix::SceneHandler::with_vm(vm),
                    )));
                }
                Arc::new(RwLock::new(Rusterix::new_without_audio()))
            })
            .clone()
    }
    /// Called before UI bootstrap and again after device replacement. Binding
    /// a host never initializes an otherwise unused reference facade.
    #[cfg(feature = "native-render")]
    pub fn bind_context(&self, context: Arc<renderkit::GpuContext>) -> bool {
        if self
            .context
            .borrow()
            .as_ref()
            .is_some_and(|old| old.generation() == context.generation())
        {
            return false;
        }
        if let Some(mut source) = self.existing_mut() {
            source.scene_handler.vm.rebind_context(context.clone());
        }
        *self.context.borrow_mut() = Some(context);
        true
    }
    pub fn initialized(&self) -> bool {
        self.source.get().is_some()
    }
    pub fn read(&self) -> ArcRwLockReadGuard<RawRwLock, Rusterix> {
        self.source().read_arc()
    }
    pub fn write(&self) -> ArcRwLockWriteGuard<RawRwLock, Rusterix> {
        self.source().write_arc()
    }
    pub fn existing(&self) -> Option<ArcRwLockReadGuard<RawRwLock, Rusterix>> {
        self.source.get().map(|source| source.read_arc())
    }
    pub fn existing_mut(&self) -> Option<ArcRwLockWriteGuard<RawRwLock, Rusterix>> {
        self.source.get().map(|source| source.write_arc())
    }
}
