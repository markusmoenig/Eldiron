use crate::prelude::*;

/// Prefab-only presentation tool. The Prefab editor owns the actual Tiles dock;
/// selecting this tool switches its lower panel without changing the current
/// geometry selection or mutating the isolated Prefab map.
pub struct TilePickerTool {
    id: TheId,
}

impl Tool for TilePickerTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Prefab Tile Picker Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_prefab_tile_picker")
    }

    fn icon_name(&self) -> String {
        "bricks".to_string()
    }
}
