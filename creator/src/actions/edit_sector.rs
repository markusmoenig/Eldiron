use crate::prelude::*;

pub struct EditSector {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditSector {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "actionSectorName".into(),
            fl!("action_edit_sector_name"),
            fl!("status_action_edit_sector_name"),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Text(
            "actionSectorTags".into(),
            fl!("action_edit_sector_tags"),
            fl!("status_action_edit_sector_tags"),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionSectorVisible".into(),
            fl!("action_edit_sector_visible"),
            fl!("status_action_edit_sector_visible"),
            true,
        ));

        let item = TheNodeUIItem::Selector(
            "actionSectorTerrain".into(),
            fl!("action_edit_sector_terrain"),
            fl!("status_action_edit_sector_terrain"),
            vec![
                fl!("action_edit_sector_terrain_none"),
                fl!("action_edit_sector_terrain_exclude"),
            ],
            0,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_edit_sector_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_edit_sector")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_sector_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_sectors.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector(*sector_id) {
                self.nodeui
                    .set_text_value("actionSectorName", sector.name.clone());

                let tags = sector.properties.get_str_default("tags", "".into());
                self.nodeui.set_text_value("actionSectorTags", tags);

                let visible = sector.properties.get_bool_default("visible", true);
                self.nodeui.set_bool_value("actionSectorVisible", visible);

                let terrain_mode = sector.properties.get_str_default("terrain_mode", "".into());
                let mut terrain_val = 0;
                if terrain_mode == "exclude" {
                    terrain_val = 1;
                }
                self.nodeui
                    .set_i32_value("actionSectorTerrain", terrain_val);
            }
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let name = self
            .nodeui
            .get_text_value("actionSectorName")
            .unwrap_or(String::new());

        let tags = self
            .nodeui
            .get_text_value("actionSectorTags")
            .unwrap_or(String::new());

        let visible = self
            .nodeui
            .get_bool_value("actionSectorVisible")
            .unwrap_or(true);

        let terrain_role = self
            .nodeui
            .get_i32_value("actionSectorTerrain")
            .unwrap_or(0);

        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                if name != sector.name {
                    sector.name = name;
                    changed = true;
                }

                let ta = sector.properties.get_str_default("tags", "".into());
                if tags != ta {
                    sector.properties.set("tags", Value::Str(tags));
                    changed = true;
                }

                let vis = sector.properties.get_bool_default("visible", true);
                if vis != visible {
                    sector.properties.set("visible", Value::Bool(visible));
                    changed = true;
                }

                // let terr = sector.properties.get_str_default("terrain_mode", "".into());
                if terrain_role == 1 {
                    sector
                        .properties
                        .set("terrain_mode", Value::Str("exclude".to_string()));
                    changed = true;
                }
            }
        }

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

    fn apply_project(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // If the update sector is a screen widget, refresh the screen list to show potential new names
        if let ProjectContext::ScreenWidget(id, widget_id) = server_ctx.pc {
            if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                if let Some(node) = tree_layout.get_node_by_id_mut(&id) {
                    if let Some(screen) = project.screens.get(&id) {
                        gen_screen_tree_items(node, screen);
                    }
                    node.new_item_selected(&TheId::named_with_id_and_reference(
                        "Screen Content List Item",
                        widget_id,
                        id,
                    ));
                }
            }
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
