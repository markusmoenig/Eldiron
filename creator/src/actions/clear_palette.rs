use crate::{
    editor::{DOCKMANAGER, UNDOMANAGER},
    prelude::*,
};

pub struct ClearPalette {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ClearPalette {
    fn new() -> Self
    where
        Self: Sized,
    {
        let nodeui: TheNodeUI = TheNodeUI::default();

        Self {
            id: TheId::named(&fl!("action_clear_palette")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_clear_palette_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Palette" && server_ctx.palette_tool_active
    }

    fn apply_project(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let prev = project.palette.clone();
        let prev_materials = project.palette_materials.clone();

        project.palette.clear();
        project.reset_all_palette_materials();
        apply_palette(ui, ctx, server_ctx, project);
        crate::undo::project_helper::refresh_palette_runtime(project);

        let undo_atom = ProjectUndoAtom::PaletteEdit(
            prev,
            prev_materials,
            project.palette.clone(),
            project.palette_materials.clone(),
        );
        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            _ => {}
        }
        false
    }
}
