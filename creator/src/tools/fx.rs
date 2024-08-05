use crate::prelude::*;
use ToolEvent::*;

pub struct FXTool {
    id: TheId,
}

impl Tool for FXTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("FX Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("FX Tool (X). Apply effects to tiles.")
    }
    fn icon_name(&self) -> String {
        str!("magicwand")
    }
    fn accel(&self) -> Option<char> {
        Some('x')
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
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            ctx.ui
                .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 3));
            server_ctx.show_fx_marker = true;

            return true;
        } else if let DeActivate = tool_event {
            server_ctx.show_fx_marker = false;
            return true;
        };

        false
    }
}
