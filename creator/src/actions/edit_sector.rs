use crate::prelude::*;
use std::str::FromStr;

pub const EDIT_SECTOR_ACTION_ID: &str = "1a1dea50-0181-46d9-acd6-913755c915e0";

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
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Text(
            "actionSectorItem".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionSectorVisible".into(),
            "".into(),
            "".into(),
            true,
        ));

        nodeui.add_item(TheNodeUIItem::OpenTree("terrain".into()));

        let item = TheNodeUIItem::Selector(
            "actionSectorTerrain".into(),
            "".into(),
            "".into(),
            vec!["None".into(), "Exclude".into(), "Ridge".into()],
            0,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSectorTerrainRidgeHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSectorTerrainRidgePlateau".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionSectorTerrainRidgeFalloff".into(),
            "".into(),
            "".into(),
            5.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::CloseTree);

        // let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_edit_sector_desc"));
        // nodeui.add_item(item);

        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_sector"),
                Uuid::from_str(EDIT_SECTOR_ACTION_ID).unwrap(),
            ),
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

                let item_name = sector.properties.get_str_default("item", "".into());
                self.nodeui.set_text_value("actionSectorItem", item_name);

                let visible = sector.properties.get_bool_default("visible", true);
                self.nodeui.set_bool_value("actionSectorVisible", visible);

                let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
                self.nodeui
                    .set_i32_value("actionSectorTerrain", terrain_mode);

                let ridge_height = sector.properties.get_float_default("ridge_height", 1.0);
                self.nodeui
                    .set_f32_value("actionSectorTerrainRidgeHeight", ridge_height);

                let ridge_plateau = sector
                    .properties
                    .get_float_default("ridge_plateau_width", 0.0);
                self.nodeui
                    .set_f32_value("actionSectorTerrainRidgePlateau", ridge_plateau);

                let ridge_falloff = sector
                    .properties
                    .get_float_default("ridge_falloff_distance", 5.0);
                self.nodeui
                    .set_f32_value("actionSectorTerrainRidgeFalloff", ridge_falloff);
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

        let item = self
            .nodeui
            .get_text_value("actionSectorItem")
            .unwrap_or(String::new());

        let visible = self
            .nodeui
            .get_bool_value("actionSectorVisible")
            .unwrap_or(true);

        let terrain_role = self
            .nodeui
            .get_i32_value("actionSectorTerrain")
            .unwrap_or(0);

        let ridge_height = self
            .nodeui
            .get_f32_value("actionSectorTerrainRidgeHeight")
            .unwrap_or(1.0);

        let ridge_plateau = self
            .nodeui
            .get_f32_value("actionSectorTerrainRidgePlateau")
            .unwrap_or(0.0);

        let ridge_falloff = self
            .nodeui
            .get_f32_value("actionSectorTerrainRidgeFalloff")
            .unwrap_or(5.0);

        if let Some(sector_id) = map.selected_sectors.first() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                if name != sector.name {
                    sector.name = name;
                    changed = true;
                }

                let it = sector.properties.get_str_default("item", "".into());
                if item != it {
                    sector.properties.set("item", Value::Str(item));
                    changed = true;
                }

                let vis = sector.properties.get_bool_default("visible", true);
                if vis != visible {
                    sector.properties.set("visible", Value::Bool(visible));
                    changed = true;
                }

                let terr = sector.properties.get_int_default("terrain_mode", 0);
                if terrain_role != terr {
                    sector
                        .properties
                        .set("terrain_mode", Value::Int(terrain_role));
                    changed = true;
                }

                let r_height = sector.properties.get_float_default("ridge_height", 1.0);
                if ridge_height != r_height {
                    sector
                        .properties
                        .set("ridge_height", Value::Float(ridge_height));
                    changed = true;
                }

                let r_plateau = sector
                    .properties
                    .get_float_default("ridge_plateau_width", 0.0);
                if ridge_plateau != r_plateau {
                    sector
                        .properties
                        .set("ridge_plateau_width", Value::Float(ridge_plateau));
                    changed = true;
                }

                let r_falloff = sector
                    .properties
                    .get_float_default("ridge_falloff_distance", 5.0);
                if ridge_falloff != r_falloff {
                    sector
                        .properties
                        .set("ridge_falloff_distance", Value::Float(ridge_falloff));
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
