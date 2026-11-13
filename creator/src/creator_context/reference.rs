//! Application-local compatibility facade for CPU assets/settings and remaining
//! reference renderers. Canonical native views own their explicit GPU context.
//! Lazy construction preserves data-only tools. Owned scoped guards let a caller
//! update disjoint CPU authoring state without borrowing the complete context.
use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock};
use rusterix::Rusterix;
use std::{cell::OnceCell, sync::Arc};

#[derive(Default)]
pub struct CreatorReference {
    source: OnceCell<Arc<RwLock<Rusterix>>>,
    #[cfg(feature = "native-render")]
    context: std::cell::RefCell<Option<Arc<renderkit::GpuContext>>>,
}
impl CreatorReference {
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
