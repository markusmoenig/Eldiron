use crate::prelude::*;
use ToolEvent::*;

pub struct CodeTool {
    id: TheId,
}

impl Tool for CodeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Code Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("I draw tiles")
    }
    fn icon_name(&self) -> String {
        str!("code")
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
                TheId::named("Set CodeGrid Panel"),
                TheValue::Empty,
            ));

            return true;
        };

        false
    }
}
