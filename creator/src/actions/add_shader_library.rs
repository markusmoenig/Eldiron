use crate::prelude::*;
pub struct AddShaderLibrary {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for AddShaderLibrary {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Adds the current shader to the Shader Library.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Add Shader to Library"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Add the current shader to the library."
    }

    fn role(&self) -> ActionRole {
        ActionRole::UI
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.curr_map_tool_helper == MapToolHelper::ShaderEditor
    }

    fn apply(
        &self,
        _map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        open_text_dialog(
            "Add Shader To Library",
            "Shader Name",
            "Shader",
            Uuid::new_v4(),
            ui,
            ctx,
        );

        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
