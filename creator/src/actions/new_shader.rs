use crate::editor::{CODEEDITOR, SHADEGRIDFX};
use crate::prelude::*;
use codegridfx::{Module, ModuleType};
pub struct NewShader {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for NewShader {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Clears the shader editor and creates a new, empty shader.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("New Shader"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Creates a new shader in the shader editor."
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
        *SHADEGRIDFX.write().unwrap() = Module::as_type(codegridfx::ModuleType::Shader);
        CODEEDITOR.write().unwrap().shader_content = ContentContext::Unknown;

        SHADEGRIDFX
            .write()
            .unwrap()
            .set_module_type(ModuleType::Shader);
        SHADEGRIDFX.write().unwrap().redraw(ui, ctx);

        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
