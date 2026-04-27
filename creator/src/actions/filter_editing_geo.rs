use crate::prelude::*;

pub struct FilterEditingGeo {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for FilterEditingGeo {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionEditingGeoFilterMode".into(),
            "Mode".into(),
            "Choose which editor geometry remains visible while editing.".into(),
            vec!["All".into(), "Dungeon".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionDungeonNoCeiling".into(),
            "Dungeon No Ceiling".into(),
            "Hide dungeon ceiling geometry while the Dungeon filter is active.".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_filter_edit_geo_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_filter_edit_geo")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_filter_edit_geo_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(
        &self,
        _map: &Map,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) -> bool {
        true
    }

    fn load_params_project(&mut self, _project: &Project, server_ctx: &mut ServerContext) {
        self.nodeui.set_i32_value(
            "actionEditingGeoFilterMode",
            server_ctx.editing_geo_filter.to_index(),
        );
        self.nodeui
            .set_bool_value("actionDungeonNoCeiling", server_ctx.dungeon_no_ceiling);
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let mode = self
            .nodeui
            .get_i32_value("actionEditingGeoFilterMode")
            .unwrap_or(0);
        server_ctx.editing_geo_filter = EditingGeoFilter::from_index(mode);
        server_ctx.dungeon_no_ceiling = self
            .nodeui
            .get_bool_value("actionDungeonNoCeiling")
            .unwrap_or(false);

        crate::utils::editor_scene_full_rebuild(project, server_ctx);
        crate::editor::TOOLLIST
            .write()
            .unwrap()
            .update_geometry_overlay_3d(project, server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Client Properties"),
            TheValue::Empty,
        ));
        ctx.ui.redraw_all = true;
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
