use crate::editor::MODELFXEDITOR;
use crate::prelude::*;

pub struct ModelNodeEditTool {
    id: TheId,
}

impl Tool for ModelNodeEditTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Edit Tool (E)."),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Edit Tool (E). Edit the nodes of the Geometry.")
    }
    fn icon_name(&self) -> String {
        str!("picker")
    }
    fn accel(&self) -> Option<char> {
        Some('e')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let ToolEvent::Activate = tool_event {
            MODELFXEDITOR.lock().unwrap().set_geometry_mode(false);

            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                layout.set_mode(TheSharedVLayoutMode::Shared);
                layout.set_shared_ratio(0.42);
            }

            ctx.ui
                .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 6));

            if let Some(layout) = ui.get_hlayout("Model Tool Params") {
                layout.clear();
            }
        } else if let ToolEvent::DeActivate = tool_event {
            if let Some(layout) = ui.get_hlayout("Material Tool Params") {
                layout.clear();
                layout.set_reverse_index(None);
            }
            if let Some(layout) = ui.get_sharedvlayout("Shared VLayout") {
                layout.set_mode(TheSharedVLayoutMode::Shared);
                layout.set_shared_ratio(crate::DEFAULT_VLAYOUT_RATIO);
            }
            MODELFXEDITOR.lock().unwrap().set_geometry_mode(true);
        }
        false
    }

    /*
    #[allow(clippy::too_many_arguments)]
    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server: &mut Server,
        _client: &mut Client,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        match event {
            // TheEvent::StateChanged(id, TheWidgetState::Selected) => {
            //     if id.name ==
            // }
            //
            _ => {}
        }

        redraw
    }*/
}
