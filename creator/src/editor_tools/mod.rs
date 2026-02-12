pub mod tile_draw;
pub mod tile_select;
// pub mod tile_fill;
// pub mod tile_picker;

pub use tile_draw::*;
pub use tile_select::*;
// pub use tile_fill::*;
// pub use tile_picker::*;

use shared::prelude::*;
use theframework::prelude::*;

/// Tool trait for dock editors (like tile editor, tilemap editor, etc.)
#[allow(unused)]
pub trait EditorTool: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn id(&self) -> TheId;
    fn info(&self) -> String;
    fn icon_name(&self) -> String;
    fn rgba_view_mode(&self) -> Option<TheRGBAViewMode> {
        None
    }

    fn accel(&self) -> Option<char> {
        None
    }

    fn help_url(&self) -> Option<String> {
        None
    }

    fn activate(&mut self) {}
    fn deactivate(&mut self) {}

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        false
    }

    /// Get the current undo atom if the tool has pending changes
    /// This is called when a tool operation completes (e.g., on mouse up)
    fn get_undo_atom(&mut self, project: &Project) -> Option<Box<dyn std::any::Any>> {
        None
    }
}
