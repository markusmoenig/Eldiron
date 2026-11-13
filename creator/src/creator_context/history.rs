//! Application-local active history. Documents keep inactive history snapshots;
//! controllers borrow this service explicitly, without a process-wide manager.
use crate::undo::UndoManager;
use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock};
use std::sync::Arc;

pub struct CreatorHistory {
    active: Arc<RwLock<UndoManager>>,
}
impl Default for CreatorHistory {
    fn default() -> Self {
        Self {
            active: Arc::new(RwLock::new(UndoManager::default())),
        }
    }
}
impl CreatorHistory {
    // Owned guards allow undo atoms to borrow disjoint authoring context while
    // the active stack is locked, matching existing reference-service borrows.
    pub fn read(&self) -> ArcRwLockReadGuard<RawRwLock, UndoManager> {
        self.active.read_arc()
    }
    pub fn write(&self) -> ArcRwLockWriteGuard<RawRwLock, UndoManager> {
        self.active.write_arc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{creator_context::CreatorContext, undo::project_atoms::ProjectUndoAtom};
    use shared::{asset::Asset, project::Project};
    use theframework::prelude::{TheContext, TheUI, Uuid};

    fn fixture() -> (Project, Uuid, TheUI, TheContext) {
        let mut project = Project::default();
        let mut asset = Asset::new();
        asset.name = "Original".into();
        let id = asset.id;
        project.assets.insert(id, asset);
        (project, id, TheUI::new(), TheContext::new(100, 100, 1.))
    }
    fn rename(
        project: &mut Project,
        id: Uuid,
        name: &str,
        history: &CreatorHistory,
        ctx: &mut TheContext,
    ) {
        let asset = project.assets.get_mut(&id).unwrap();
        let old = std::mem::replace(&mut asset.name, name.into());
        history
            .write()
            .add_undo(ProjectUndoAtom::RenameAsset(id, old, name.into()), ctx);
    }
    #[test]
    fn active_history_is_local_and_undo_redo_can_borrow_its_context() {
        let (mut project, id, mut ui, mut ctx) = fixture();
        let mut first = CreatorContext::new();
        let second = CreatorContext::new();
        let untouched = second.history.read().revision();
        rename(&mut project, id, "Edited", &first.history, &mut ctx);
        first.history.write().mark_saved();
        assert!(!first.history.read().has_unsaved());
        first
            .history
            .write()
            .undo(&mut first, &mut project, &mut ui, &mut ctx);
        assert_eq!(project.assets[&id].name, "Original");
        assert!(first.history.read().has_unsaved());
        assert!(!second.history.read().has_undo());
        assert_eq!(second.history.read().revision(), untouched);
        first
            .history
            .write()
            .redo(&mut first, &mut project, &mut ui, &mut ctx);
        assert_eq!(project.assets[&id].name, "Edited");
        assert!(!first.history.read().has_unsaved());
        let snapshot = first.history.read().clone();
        *first.history.write() = UndoManager::default();
        assert!(!first.history.read().has_undo());
        *first.history.write() = snapshot;
        assert!(first.history.read().has_undo());
        assert_eq!(second.history.read().revision(), untouched);
        assert!(!first.reference.initialized() && !second.reference.initialized());
        assert!(!first.jobs.scene_initialized() && !second.jobs.scene_initialized());
    }
    #[test]
    fn branching_after_a_saved_edit_stays_unsaved() {
        let (mut project, id, mut ui, mut ctx) = fixture();
        let mut owner = CreatorContext::new();
        rename(&mut project, id, "Saved", &owner.history, &mut ctx);
        owner.history.write().mark_saved();
        owner
            .history
            .write()
            .undo(&mut owner, &mut project, &mut ui, &mut ctx);
        rename(&mut project, id, "Branch", &owner.history, &mut ctx);
        assert!(
            owner.history.read().has_unsaved(),
            "a new edit cannot inherit a discarded redo branch's saved checkpoint"
        );
        assert!(!owner.history.read().has_redo());
        owner
            .history
            .write()
            .undo(&mut owner, &mut project, &mut ui, &mut ctx);
        assert_eq!(project.assets[&id].name, "Original");
        assert!(owner.history.read().has_unsaved());
        owner
            .history
            .write()
            .redo(&mut owner, &mut project, &mut ui, &mut ctx);
        assert_eq!(project.assets[&id].name, "Branch");
        assert!(owner.history.read().has_unsaved());
        owner.history.write().mark_saved();
        assert!(!owner.history.read().has_unsaved());
    }
    #[test]
    fn retention_cannot_mark_an_unreachable_saved_document_clean() {
        let (mut project, id, mut ui, mut ctx) = fixture();
        let mut owner = CreatorContext::new();
        owner.history.write().max_undo = 2;
        for name in ["A", "B", "C"] {
            rename(&mut project, id, name, &owner.history, &mut ctx);
        }
        for _ in 0..2 {
            owner
                .history
                .write()
                .undo(&mut owner, &mut project, &mut ui, &mut ctx);
        }
        assert_eq!(project.assets[&id].name, "A");
        assert!(!owner.history.read().has_undo());
        assert!(
            owner.history.read().has_unsaved(),
            "the retained origin is different from the evicted saved source"
        );
        owner.history.write().mark_saved();
        assert!(!owner.history.read().has_unsaved());
        owner.history.write().max_undo = 0;
        rename(&mut project, id, "D", &owner.history, &mut ctx);
        assert!(!owner.history.read().has_undo());
        assert!(
            owner.history.read().has_unsaved(),
            "zero retained actions cannot erase unsaved edits"
        );
        assert!(!owner.reference.initialized());
    }
}
