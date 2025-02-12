use crate::prelude::*;
use ToolEvent::*;

pub struct TilesetTool {
    id: TheId,
}

impl Tool for TilesetTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Tileset Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Tileset Tool (T). Manage the tiles in your tilesets.")
    }
    fn icon_name(&self) -> String {
        str!("bricks")
    }
    fn accel(&self) -> Option<char> {
        Some('t')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                layout.set_mode(TheSharedVLayoutMode::Bottom);
            }
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Tilemap Panel"),
                TheValue::Empty,
            ));

            return true;
        } else if let DeActivate = tool_event {
            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                layout.set_mode(TheSharedVLayoutMode::Shared);
            }
        }
        false
    }
}
