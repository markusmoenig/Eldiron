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

    fn prepare_overlay(&mut self, input: ToolOverlayInputs<'_>) -> Option<ToolOverlayFrame> {
        ToolOverlayFrame::passive(
            self,
            input,
            crate::editor_overlay::EditorOverlay::new(input.target.size),
        )
    }
    fn overlay_input_current(
        &mut self,
        _frame: &ToolOverlayFrame,
        _target: ToolOverlayTarget,
        _map: &Map,
        _server: &ServerContext,
    ) -> bool {
        true
    }
    fn overlay_command_source_current(
        &mut self,
        _frame: &ToolOverlayFrame,
        _target: ToolOverlayTarget,
        _map: &Map,
        _server: &ServerContext,
    ) -> bool {
        true
    }
    #[cfg(feature = "native-ui")]
    fn presented_overlay(
        &mut self,
        _frame: &ToolOverlayFrame,
        _receipts: &[theframework::paint::FrameReceipt],
        _target: ToolOverlayTarget,
        _map: &Map,
        _server: &mut ServerContext,
    ) -> bool {
        true
    }
    fn info(&self) -> String {
        fl!("tool_prefab_tile_picker")
    }

    fn icon_name(&self) -> String {
        "bricks".to_string()
    }
}
