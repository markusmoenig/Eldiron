use super::shape_terrain::{self, Kind};
use super::terrain_material;
use crate::prelude::*;
use std::str::FromStr;

pub const EDIT_LINEDEF_ACTION_ID: &str = "284638fa-5769-442a-a55e-88121a37f193";

pub struct EditLinedef {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditLinedef {
    fn build_nodeui() -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::Text(
            "actionLinedefName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));

        shape_terrain::controls(&mut nodeui, Kind::Line);
        terrain_material::controls(&mut nodeui, Kind::Line);
        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        nodeui
    }
}

impl Action for EditLinedef {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_linedef"),
                Uuid::from_str(EDIT_LINEDEF_ACTION_ID).unwrap(),
            ),
            nodeui: Self::build_nodeui(),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_linedef_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        !map.selected_linedefs.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(linedef_id) = map.selected_linedefs.first()
            && let Some(linedef) = map.find_linedef(*linedef_id)
        {
            self.nodeui
                .set_text_value("actionLinedefName", linedef.name.clone());
            shape_terrain::load(&mut self.nodeui, &linedef.properties, Kind::Line);
            terrain_material::load(&mut self.nodeui, &linedef.properties, Kind::Line);
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

        let name = self
            .nodeui
            .get_text_value("actionLinedefName")
            .unwrap_or_default();

        for linedef_id in map.selected_linedefs.clone() {
            if let Some(linedef) = map.find_linedef_mut(linedef_id) {
                if name != linedef.name {
                    linedef.name = name.clone();
                    changed = true;
                }
                if server_ctx.pc.is_region() {
                    changed |=
                        shape_terrain::apply(&self.nodeui, &mut linedef.properties, Kind::Line);
                    changed |=
                        terrain_material::apply(&self.nodeui, &mut linedef.properties, Kind::Line);
                }
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

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server: &ServerContext,
    ) -> Option<Vec<super::ActionMaterialSlot>> {
        _server
            .pc
            .is_region()
            .then(|| terrain_material::slots(&self.nodeui, Kind::Line))
    }
    fn set_hud_material_source(
        &mut self,
        _map: &Map,
        _server: &ServerContext,
        slot: i32,
        source: rusterix::PixelSource,
    ) -> bool {
        terrain_material::set_slot(&mut self.nodeui, slot, Some(source), Kind::Line)
    }
    fn clear_hud_material_slot(&mut self, _map: &Map, _server: &ServerContext, slot: i32) -> bool {
        terrain_material::set_slot(&mut self.nodeui, slot, None, Kind::Line)
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
