//! Application-local controller instances. Mutable tool/action parameters,
//! editor-display drafts and dock state never cross application lifetimes.
use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock};
use std::{cell::OnceCell, sync::Arc};

pub type CreatorActions = CreatorController<crate::actionlist::ActionList>;
pub type CreatorTools = CreatorController<crate::toollist::ToolList>;
pub type CreatorDocks = CreatorController<crate::dockmanager::DockManager>;

pub struct CreatorController<T> {
    active: OnceCell<Arc<RwLock<T>>>,
}
impl<T> Default for CreatorController<T> {
    fn default() -> Self {
        Self {
            active: OnceCell::new(),
        }
    }
}
impl<T: Default> CreatorController<T> {
    fn active(&self) -> Arc<RwLock<T>> {
        self.active
            .get_or_init(|| Arc::new(RwLock::new(T::default())))
            .clone()
    }
    pub fn initialized(&self) -> bool {
        self.active.get().is_some()
    }
    // Owned guards let callbacks borrow disjoint authoring state. The UI thread
    // owns these services; no application-global instance is constructed.
    pub fn read(&self) -> ArcRwLockReadGuard<RawRwLock, T> {
        self.active().read_arc()
    }
    pub fn write(&self) -> ArcRwLockWriteGuard<RawRwLock, T> {
        self.active().write_arc()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        creator_context::CreatorContext, dockmanager::DockManagerState,
        editor_display::EditorDisplay,
    };

    #[test]
    fn controller_parameters_and_selection_belong_to_one_application() {
        let first = CreatorContext::new();
        let second = CreatorContext::new();
        for owner in [&first, &second] {
            assert!(!owner.actions.initialized());
            assert!(!owner.tools.initialized());
            assert!(!owner.docks.initialized());
        }
        let original = second
            .actions
            .read()
            .get_action_by_command_id("geometry.create_revolve")
            .unwrap()
            .editor_display()
            .unwrap();
        let EditorDisplay::Profile2D(mut changed) = original.clone();
        changed.points[1].x += 0.25;
        let changed = EditorDisplay::Profile2D(changed);
        assert!(
            first
                .actions
                .write()
                .get_action_by_command_id_mut("geometry.create_revolve")
                .unwrap()
                .update_editor_display(&changed)
        );
        assert_eq!(
            first
                .actions
                .read()
                .get_action_by_command_id("geometry.create_revolve")
                .unwrap()
                .editor_display(),
            Some(changed)
        );
        assert_eq!(
            second
                .actions
                .read()
                .get_action_by_command_id("geometry.create_revolve")
                .unwrap()
                .editor_display(),
            Some(original)
        );
        // Stable automation command IDs are identical even when mutable action
        // instances and their generated widget IDs belong to separate owners.
        let catalog = |owner: &CreatorContext| {
            let actions = owner.actions.read();
            actions
                .actions
                .iter()
                .map(|a| actions.descriptor_by_id(a.id().uuid).unwrap().clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(catalog(&first), catalog(&second));
        {
            let mut tools = first.tools.write();
            tools.curr_game_tool = tools
                .game_tools
                .iter()
                .position(|tool| {
                    Some(tool.id().uuid) == tools.get_game_tool_uuid_by_command_id("tool.rect")
                })
                .unwrap();
        }
        assert!(first.tools.read().current_game_tool_is("tool.rect"));
        assert!(!second.tools.read().current_game_tool_is("tool.rect"));
        {
            let mut docks = first.docks.write();
            docks.dock = "Palette".into();
            docks.state = DockManagerState::Editor;
        }
        assert!(second.docks.read().state != DockManagerState::Editor);
        assert_ne!(second.docks.read().dock, "Palette");
        for owner in [&first, &second] {
            assert!(!owner.reference.initialized());
            assert!(!owner.jobs.scene_initialized());
            assert!(!owner.history.read().has_unsaved());
        }
        drop(first);
        assert!(!second.tools.read().current_game_tool_is("tool.rect"));
        assert_ne!(second.docks.read().dock, "Palette");
    }
}
