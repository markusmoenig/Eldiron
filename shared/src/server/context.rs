use theframework::prelude::*;

/// This gives context to the server of the editing state for live highlighting.
pub struct ServerContext {

    /// The currently selected region in the editor.
    pub curr_region: Uuid,

    /// The currently selected character in the editor.
    pub curr_character: Option<Uuid>,

    /// The currently selected character instance in the editor.
    pub curr_character_instance: Option<Uuid>,
}

impl Default for ServerContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerContext {
    pub fn new() -> Self {
        Self {
            curr_region: Uuid::nil(),
            curr_character: None,
            curr_character_instance: None,
        }
    }
}