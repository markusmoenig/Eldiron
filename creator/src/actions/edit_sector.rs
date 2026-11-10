use super::shape_terrain::{self, Kind};
use super::terrain_material;
use crate::prelude::*;
use rusterix::Value;
use std::str::FromStr;

pub const EDIT_SECTOR_ACTION_ID: &str = "1a1dea50-0181-46d9-acd6-913755c915e0";

pub struct EditSector {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditSector {
    fn build_nodeui(iso: bool) -> TheNodeUI {
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
        if iso {
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "actionSectorFloorHeight".into(),
                fl!("sector_floor_height"),
                fl!("sector_floor_height_desc"),
                0.,
                0.0..=32.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::Checkbox(
                "actionSectorIsoWalkable".into(),
                "Walkable top".into(),
                "Use this tile top for iso navigation".into(),
                false,
            ));
        }

        shape_terrain::controls(&mut nodeui, Kind::Sector);
        terrain_material::controls(&mut nodeui, Kind::Sector);
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
            nodeui: Self::build_nodeui(false),
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
        self.nodeui = Self::build_nodeui(map.camera == rusterix::MapCamera::TwoDIso);
        if let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
        {
            self.nodeui
                .set_text_value("actionSectorName", sector.name.clone());
            shape_terrain::load(&mut self.nodeui, &sector.properties, Kind::Sector);
            terrain_material::load(&mut self.nodeui, &sector.properties, Kind::Sector);
            self.nodeui.set_text_value(
                "actionSectorItem",
                sector.properties.get_str_default("item", "".into()),
            );
            self.nodeui.set_bool_value(
                "actionSectorVisible",
                sector.properties.get_bool_default("visible", true),
            );
            self.nodeui.set_f32_value(
                "actionSectorFloorHeight",
                sector.properties.get_float_default("floor_height", 0.),
            );
            self.nodeui.set_bool_value(
                "actionSectorIsoWalkable",
                sector.properties.get_bool_default("iso_walkable", false),
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
        if let Err(error) = terrain_material::validate(&self.nodeui) {
            _ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            return None;
        }
        let mut changed = false;
        let prev = map.clone();
        let iso = map.camera == rusterix::MapCamera::TwoDIso;

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
            if server_ctx.pc.is_region() {
                changed |= shape_terrain::apply(&self.nodeui, &mut sector.properties, Kind::Sector);
                changed |=
                    terrain_material::apply(&self.nodeui, &mut sector.properties, Kind::Sector);
            }
            if iso {
                let h = self
                    .nodeui
                    .get_f32_value("actionSectorFloorHeight")
                    .unwrap_or(0.)
                    .clamp(0., 32.);
                let walk = self
                    .nodeui
                    .get_bool_value("actionSectorIsoWalkable")
                    .unwrap_or(false);
                if sector.properties.get_float_default("floor_height", 0.) != h
                    || sector.properties.get_bool_default("iso_walkable", false) != walk
                {
                    sector.properties.set("floor_height", Value::Float(h));
                    sector.properties.set("iso_walkable", Value::Bool(walk));
                    changed = true;
                }
            }
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
            if server_ctx.pc.is_region()
                && (self
                    .nodeui
                    .get_bool_value("terrainEnabled")
                    .unwrap_or(false)
                    || terrain_material::enabled(&self.nodeui))
            {
                shape_terrain::enable(map, server_ctx);
            }
            map.changed = map.changed.wrapping_add(1);
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

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server: &ServerContext,
    ) -> Option<Vec<super::ActionMaterialSlot>> {
        _server
            .pc
            .is_region()
            .then(|| terrain_material::slots(&self.nodeui, Kind::Sector))
    }
    fn set_hud_material_source(
        &mut self,
        _map: &Map,
        _server: &ServerContext,
        slot: i32,
        source: rusterix::PixelSource,
    ) -> bool {
        terrain_material::set_slot(&mut self.nodeui, slot, Some(source), Kind::Sector)
    }
    fn clear_hud_material_slot(&mut self, _map: &Map, _server: &ServerContext, slot: i32) -> bool {
        terrain_material::set_slot(&mut self.nodeui, slot, None, Kind::Sector)
    }
    fn preserves_hud_material_slots(&self) -> bool {
        true
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn iso_sector_inspector_edits_height_and_walkability_with_map_undo() {
        let mut map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        map.properties.set("iso_edit_height", Value::Float(0.25));
        let v = [
            Vec2::zero(),
            Vec2::new(1., 0.),
            Vec2::one(),
            Vec2::new(0., 1.),
        ];
        crate::iso_editor::paint_cell(
            &mut map,
            v,
            Some(rusterix::PixelSource::TileId(Uuid::new_v4())),
            false,
            0,
        );
        let id = map.selected_sectors[0];
        let mut action = <EditSector as Action>::new();
        action.load_params(&map);
        action.nodeui.set_f32_value("actionSectorFloorHeight", 0.75);
        action
            .nodeui
            .set_bool_value("actionSectorIsoWalkable", true);
        let other_id = map.sectors.len() as u32 + 10;
        map.sectors
            .push(rusterix::Sector::new(other_id, Vec::new()));
        let other = map.find_sector(other_id).unwrap().clone();
        let undo = action
            .apply(
                &mut map,
                &mut TheUI::new(),
                &mut TheContext::new(400, 300, 1.),
                &mut ServerContext::default(),
            )
            .unwrap();
        assert_eq!(
            map.find_sector(id)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.),
            0.75
        );
        assert_eq!(
            map.as_mini(&Default::default())
                .elevation
                .height_at(Vec2::broadcast(0.5)),
            0.75
        );
        assert_eq!(
            map.properties.get_float_default("iso_edit_height", 0.),
            0.25
        );
        assert_eq!(
            serde_json::to_value(map.find_sector(other_id).unwrap()).unwrap(),
            serde_json::to_value(&other).unwrap()
        );
        let ProjectUndoAtom::MapEdit(_, before, after) = undo else {
            panic!("Expected normal map undo");
        };
        assert_eq!(
            before
                .find_sector(id)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.),
            0.25
        );
        assert_eq!(
            after
                .find_sector(id)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.),
            0.75
        );
        map.camera = rusterix::MapCamera::TwoD;
        action.load_params(&map);
        assert!(
            action
                .nodeui
                .get_f32_value("actionSectorFloorHeight")
                .is_none()
        );
    }
}
