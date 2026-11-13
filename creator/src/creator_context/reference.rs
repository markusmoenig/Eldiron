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
}
impl CreatorReference {
    fn source(&self) -> Arc<RwLock<Rusterix>> {
        self.source
            .get_or_init(|| Arc::new(RwLock::new(Rusterix::new_without_audio())))
            .clone()
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
