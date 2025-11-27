use crate::prelude::*;

pub struct Terrain {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for Terrain {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        // Surface extrusion settings
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionTerrainEnable".into(),
            fl!("action_terrain_enable"),
            fl!("status_action_terrain_enable"),
            true,
        ));

        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_terrain_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named("Terrain"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_terrain_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode != EditorViewMode::D2 || server_ctx.editing_surface.is_none()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let changed = true;
        let prev = map.clone();

        // Enable terrain on map
        map.properties.set("terrain_enabled", Value::Bool(true));

        if changed {
            Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
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
        self.nodeui.handle_event(event)
    }
}
