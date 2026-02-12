use crate::prelude::*;

pub struct TileSelectTool {
    id: TheId,
}

impl EditorTool for TileSelectTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id("Tile Select Tool", Uuid::new_v4()),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Select Tool (S). Drag to replace selection, Shift-drag to add, Alt-drag to subtract; supports cut/copy/paste on selected pixels.".to_string()
    }

    fn icon_name(&self) -> String {
        "selection".to_string()
    }

    fn rgba_view_mode(&self) -> Option<TheRGBAViewMode> {
        Some(TheRGBAViewMode::TileSelection)
    }

    fn accel(&self) -> Option<char> {
        Some('S')
    }
}
