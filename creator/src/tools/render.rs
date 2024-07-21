use crate::prelude::*;
use ToolEvent::*;

pub struct RenderTool {
    id: TheId,
}

impl Tool for RenderTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Render Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Render Setings.")
    }
    fn icon_name(&self) -> String {
        str!("faders")
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Region Render"),
                TheValue::Empty,
            ));

            return true;
        };

        false
    }
}
