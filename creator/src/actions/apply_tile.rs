use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use rusterix::PixelSource;

pub struct ApplyTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ApplyTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Selector(
            "actionApplyTileMode".into(),
            "".into(),
            "".into(),
            vec!["repeat".into(), "scale".into()],
            0,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_apply_tile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_apply_tile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::ALT, 'a'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        (!map.selected_sectors.is_empty()
            || !map.selected_geometry_faces.is_empty()
            || !map.selected_geometry_objects.is_empty())
            && DOCKMANAGER.read().unwrap().dock == "Tiles"
            && (server_ctx.curr_tile_source.is_some() || server_ctx.curr_tile_id.is_some())
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

        let mut mode = self
            .nodeui
            .get_i32_value("actionApplyTileMode")
            .unwrap_or(0);

        mode = match mode {
            1 => 0,
            _ => 1,
        };

        if let Some(source_value) = crate::utils::get_source(_ui, server_ctx) {
            for sector_id in &map.selected_sectors.clone() {
                if let Some(sector) = map.find_sector_mut(*sector_id) {
                    let mut source = "source";

                    if server_ctx.pc.is_screen() {
                        if server_ctx.selected_hud_icon_index == 1 {
                            source = "ceiling_source";
                        }
                    }

                    sector
                        .properties
                        .set(source, Value::Source(source_value.clone()));
                    sector.properties.set("tile_mode", Value::Int(mode));
                    changed = true;
                }
            }

            let selected_geometry_faces = map.selected_geometry_faces.clone();
            let geometry_source = server_ctx
                .curr_tile_id
                .map(PixelSource::TileId)
                .unwrap_or_else(|| source_value.clone());
            let geometry_source = crate::utils::SurfaceApplySource::Direct(geometry_source);
            if selected_geometry_faces.is_empty() {
                for object_id in map.selected_geometry_objects.clone() {
                    changed |= crate::utils::apply_surface_source_to_geometry_object(
                        map,
                        object_id,
                        &geometry_source,
                        Some(mode),
                    );
                }
            } else {
                for (object_id, face_index) in selected_geometry_faces {
                    changed |= crate::utils::apply_surface_source_to_geometry_face(
                        map,
                        object_id,
                        face_index,
                        &geometry_source,
                        Some(mode),
                    );
                }
            }
        }

        if changed {
            map.update_surfaces();
            RUSTERIX.write().unwrap().set_dirty();
            RUSTERIX.write().unwrap().set_overlay_dirty();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_tile_prefers_explicit_geometry_face_selection_over_object_selection() {
        let mut map = Map::default();
        let object = rusterix::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_objects.push(object_id);
        map.selected_geometry_faces.push((object_id, 2));

        let action = ApplyTile::new();
        let mut ui = TheUI::default();
        let mut ctx = TheContext::new(64, 64, 1.0);
        let mut server_ctx = ServerContext::default();
        server_ctx.pc = ProjectContext::Region(Uuid::new_v4());
        server_ctx.curr_tile_id = Some(Uuid::new_v4());

        let Some(ProjectUndoAtom::MapEdit(_, old_map, new_map)) =
            action.apply(&mut map, &mut ui, &mut ctx, &mut server_ctx)
        else {
            panic!("apply tile should return a MapEdit undo atom");
        };

        assert!(
            old_map.geometry_objects[0]
                .faces
                .iter()
                .all(|face| face.tile.is_none())
        );
        for (index, face) in new_map.geometry_objects[0].faces.iter().enumerate() {
            if index == 2 {
                assert_eq!(face.tile, server_ctx.curr_tile_id.map(PixelSource::TileId));
            } else {
                assert_eq!(face.tile, None);
            }
        }
    }
}
