use crate::prelude::*;
use rusterix::Value;
use std::str::FromStr;

pub const EDIT_SECTOR_ACTION_ID: &str = "1a1dea50-0181-46d9-acd6-913755c915e0";

pub struct EditSector {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditSector {
    fn build_nodeui() -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::Text(
            "actionSectorName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::Text(
            "actionSectorItem".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionSectorVisible".into(),
            "".into(),
            "".into(),
            true,
        ));

        nodeui
    }
}

impl Action for EditSector {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_sector"),
                Uuid::from_str(EDIT_SECTOR_ACTION_ID).unwrap(),
            ),
            nodeui: Self::build_nodeui(),
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

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode == EditorViewMode::D2 && map.selected_sectors.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
        {
            self.nodeui
                .set_text_value("actionSectorName", sector.name.clone());
            self.nodeui.set_text_value(
                "actionSectorItem",
                sector.properties.get_str_default("item", "".into()),
            );
            self.nodeui.set_bool_value(
                "actionSectorVisible",
                sector.properties.get_bool_default("visible", true),
            );
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
            .unwrap_or_default();
        let item = self
            .nodeui
            .get_text_value("actionSectorItem")
            .unwrap_or_default();
        let visible = self
            .nodeui
            .get_bool_value("actionSectorVisible")
            .unwrap_or(true);

        if let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector_mut(*sector_id)
        {
            if name != sector.name {
                sector.name = name;
                changed = true;
            }

            let existing_item = sector.properties.get_str_default("item", "".into());
            if item != existing_item {
                sector.properties.set("item", Value::Str(item));
                changed = true;
            }

            let existing_visible = sector.properties.get_bool_default("visible", true);
            if visible != existing_visible {
                sector.properties.set("visible", Value::Bool(visible));
                changed = true;
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
        // If the updated sector is a screen widget, refresh the screen list to show potential new names.
        if let ProjectContext::ScreenWidget(id, widget_id) = server_ctx.pc
            && let Some(tree_layout) = ui.get_tree_layout("Project Tree")
            && let Some(node) = tree_layout.get_node_by_id_mut(&id)
        {
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

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn hud_material_slots(
        &self,
        map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        crate::actions::builder_hud_material_slots_for_selected_sector(map)
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
