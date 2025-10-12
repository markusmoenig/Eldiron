use crate::editor::{CODEEDITOR, CUSTOMCAMERA, NODEEDITOR, RUSTERIX, SHAPEPICKER, UNDOMANAGER};
use crate::prelude::*;
use crate::tools::apply_tile::ApplyTile;

pub struct ActionList {
    pub actions: Vec<Box<dyn Action>>,
}

impl Default for ActionList {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionList {
    pub fn new() -> Self {
        let actions: Vec<Box<dyn Action>> = vec![Box::new(ApplyTile::new())];
        Self { actions }
    }

    /// Returns an action by the given id.
    pub fn get_action_by_id(&self, id: Uuid) -> Option<&Box<dyn Action>> {
        for action in &self.actions {
            if action.id().uuid == id {
                return Some(action);
            }
        }
        None
    }
}
