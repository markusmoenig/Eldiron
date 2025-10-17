use crate::editor::SHADEGRIDFX;
use crate::prelude::*;
use codegridfx::ModuleType;
pub struct LoadShader {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for LoadShader {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Load the shader from the selected sector into the shader editor.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("Load Shader"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Loads a shader from a sector into the shader editor."
    }

    fn role(&self) -> ActionRole {
        ActionRole::UI
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.curr_map_tool_helper == MapToolHelper::ShaderEditor
            && !map.selected_sectors.is_empty()
    }

    fn apply(
        &self,
        map: &mut Map,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut shader_id = None;

        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector(*sector_id) {
                shader_id = sector.shader;
            }
        }

        if let Some(shader_id) = shader_id {
            if let Some(shader) = map.shaders.get(&shader_id) {
                *SHADEGRIDFX.write().unwrap() = shader.clone();

                SHADEGRIDFX
                    .write()
                    .unwrap()
                    .set_module_type(ModuleType::Shader);
                SHADEGRIDFX.write().unwrap().redraw(ui, ctx);
            }
        }

        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
