use crate::prelude::*;
use ToolEvent::*;

pub struct TilemapTool {
    id: TheId,
}

impl Tool for TilemapTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Tilemap Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Tilemap Tool (T). Create new tiles from your tilemaps.")
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
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Tilemap Panel"),
                TheValue::Empty,
            ));

            return true;
        };

        false
    }
}
