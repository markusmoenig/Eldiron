//! Main-thread cooperative CPU preparation. Ownership is application-local;
//! data-only authoring can query existing work without initializing a presenter.
use rusterix::scenemanager::{SceneManager, SceneManagerResult};
use std::cell::{OnceCell, Ref, RefCell, RefMut};

#[derive(Default)]
pub struct CreatorJobs {
    scene: OnceCell<RefCell<SceneManager>>,
}
impl CreatorJobs {
    pub fn receive_scene(&self) -> Option<SceneManagerResult> {
        self.existing_scene_mut()
            .and_then(|mut scene| scene.receive())
    }
    pub fn scene_initialized(&self) -> bool {
        self.scene.get().is_some()
    }
    pub fn scene(&self) -> Ref<'_, SceneManager> {
        self.scene
            .get_or_init(|| RefCell::new(SceneManager::new()))
            .borrow()
    }
    pub fn scene_mut(&self) -> RefMut<'_, SceneManager> {
        self.scene
            .get_or_init(|| RefCell::new(SceneManager::new()))
            .borrow_mut()
    }
    pub fn existing_scene_mut(&self) -> Option<RefMut<'_, SceneManager>> {
        self.scene.get().map(|scene| scene.borrow_mut())
    }
}
