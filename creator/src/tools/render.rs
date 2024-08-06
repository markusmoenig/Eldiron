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
        str!("Render Setings (R).")
    }
    fn icon_name(&self) -> String {
        str!("faders")
    }
    fn accel(&self) -> Option<char> {
        Some('r')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if let Activate = tool_event {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Set Region Render"),
                TheValue::Empty,
            ));

            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                let node_canvas = region.render_settings.to_canvas();
                ui.set_node_canvas("RegionFX NodeCanvas", node_canvas);
            }

            return true;
        };

        false
    }
}
