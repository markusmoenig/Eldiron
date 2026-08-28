use crate::editor::RUSTERIX;
use crate::prelude::*;

fn surface_detail_target_faces(map: &Map, tool_type: MapToolType) -> Vec<(Uuid, usize)> {
    let mut targets = Vec::new();
    if tool_type == MapToolType::Selection {
        for object_id in &map.selected_geometry_objects {
            if let Some(object) = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
            {
                targets.extend((0..object.faces.len()).map(|face_index| (*object_id, face_index)));
            }
        }
    } else {
        targets.extend(map.selected_geometry_faces.iter().copied());
        targets.extend(
            map.selected_geometry_surface_points
                .iter()
                .map(|(object_id, face_index, _)| (*object_id, *face_index)),
        );
        targets.extend(
            map.selected_geometry_surface_segments
                .iter()
                .map(|(object_id, face_index, _)| (*object_id, *face_index)),
        );
    }
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn surface_detail_targets_have_detail(map: &Map, tool_type: MapToolType) -> bool {
    surface_detail_target_faces(map, tool_type)
        .iter()
        .any(|(object_id, face_index)| {
            map.geometry_objects
                .iter()
                .find(|object| object.id == *object_id)
                .and_then(|object| object.faces.get(*face_index))
                .is_some_and(|face| {
                    !face.surface_points.is_empty() || !face.surface_segments.is_empty()
                })
        })
}

fn clear_selected_surface_detail(map: &mut Map, tool_type: MapToolType) -> bool {
    let selections = surface_detail_target_faces(map, tool_type);

    let mut changed = false;
    for (object_id, face_index) in &selections {
        let Some(face) = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == *object_id)
            .and_then(|object| object.faces.get_mut(*face_index))
        else {
            continue;
        };
        if !face.surface_points.is_empty() || !face.surface_segments.is_empty() {
            face.surface_points.clear();
            face.surface_segments.clear();
            changed = true;
        }
    }

    if changed {
        map.selected_geometry_surface_points
            .retain(|(object_id, face_index, _)| !selections.contains(&(*object_id, *face_index)));
        map.selected_geometry_surface_segments
            .retain(|(object_id, face_index, _)| !selections.contains(&(*object_id, *face_index)));
        map.changed = map.changed.wrapping_add(1);
    }
    changed
}

pub struct ClearSurfaceDetail {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ClearSurfaceDetail {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_clear_surface_detail_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_clear_surface_detail")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_clear_surface_detail_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && surface_detail_targets_have_detail(map, server_ctx.curr_map_tool_type)
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        if !clear_selected_surface_detail(map, server_ctx.curr_map_tool_type) {
            return None;
        }

        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
        ctx.ui.redraw_all = true;
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
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

    fn face_with_surface_detail() -> rusterix::GeometryFace {
        rusterix::GeometryFace {
            id: Uuid::new_v4(),
            paint_surface_id: Some(Uuid::new_v4()),
            indices: vec![0, 1, 2],
            uvs: Vec::new(),
            paint_uvs: Vec::new(),
            auto_uv: true,
            texture_offset: Vec2::zero(),
            texture_scale: Vec2::one(),
            texture_rotation: 0.0,
            tile: None,
            tiles: FxHashMap::default(),
            surface_points: vec![
                rusterix::GeometrySurfacePoint {
                    position: Vec3::zero(),
                    mode: rusterix::GeometrySurfacePointMode::Corner,
                },
                rusterix::GeometrySurfacePoint {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    mode: rusterix::GeometrySurfacePointMode::Corner,
                },
            ],
            surface_segments: vec![rusterix::GeometrySurfaceSegment {
                start: 0,
                end: 1,
                mode: rusterix::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.35,
            }],
            smoothing_group: 0,
        }
    }

    #[test]
    fn clears_only_guides_on_selected_faces() {
        let mut map = Map::default();
        let mut object = rusterix::GeometryObject::new("Detailed");
        object.faces = vec![face_with_surface_detail(), face_with_surface_detail()];
        let object_id = object.id;
        let paint_surface_id = object.faces[0].paint_surface_id;
        map.geometry_objects.push(object);
        map.selected_geometry_faces.push((object_id, 0));
        map.selected_geometry_surface_points.push((object_id, 0, 0));
        map.selected_geometry_surface_segments
            .push((object_id, 0, 0));

        assert!(clear_selected_surface_detail(&mut map, MapToolType::Sector));
        assert!(map.geometry_objects[0].faces[0].surface_points.is_empty());
        assert!(map.geometry_objects[0].faces[0].surface_segments.is_empty());
        assert_eq!(
            map.geometry_objects[0].faces[0].paint_surface_id,
            paint_surface_id
        );
        assert!(!map.geometry_objects[0].faces[1].surface_points.is_empty());
        assert_eq!(map.selected_geometry_faces, vec![(object_id, 0)]);
        assert!(map.selected_geometry_surface_points.is_empty());
        assert!(map.selected_geometry_surface_segments.is_empty());
    }

    #[test]
    fn selected_edge_mode_guide_clears_its_whole_host_face() {
        let mut map = Map::default();
        let mut object = rusterix::GeometryObject::new("Detailed");
        object.faces = vec![face_with_surface_detail(), face_with_surface_detail()];
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_surface_segments
            .push((object_id, 1, 0));

        assert_eq!(
            surface_detail_target_faces(&map, MapToolType::Linedef),
            vec![(object_id, 1)]
        );
        assert!(clear_selected_surface_detail(
            &mut map,
            MapToolType::Linedef
        ));
        assert!(!map.geometry_objects[0].faces[0].surface_points.is_empty());
        assert!(map.geometry_objects[0].faces[1].surface_points.is_empty());
        assert!(map.geometry_objects[0].faces[1].surface_segments.is_empty());
    }

    #[test]
    fn selected_object_clears_surface_detail_from_all_of_its_faces() {
        let mut map = Map::default();
        let mut selected = rusterix::GeometryObject::new("Selected");
        selected.faces = vec![face_with_surface_detail(), face_with_surface_detail()];
        let selected_id = selected.id;
        let selected_paint_ids = selected
            .faces
            .iter()
            .map(|face| face.paint_surface_id)
            .collect::<Vec<_>>();
        let mut other = rusterix::GeometryObject::new("Other");
        other.faces = vec![face_with_surface_detail()];
        map.geometry_objects.extend([selected, other]);
        map.selected_geometry_objects.push(selected_id);

        assert_eq!(
            surface_detail_target_faces(&map, MapToolType::Selection),
            vec![(selected_id, 0), (selected_id, 1)]
        );
        assert!(clear_selected_surface_detail(
            &mut map,
            MapToolType::Selection
        ));
        assert!(
            map.geometry_objects[0]
                .faces
                .iter()
                .all(|face| face.surface_points.is_empty() && face.surface_segments.is_empty())
        );
        assert_eq!(
            map.geometry_objects[0]
                .faces
                .iter()
                .map(|face| face.paint_surface_id)
                .collect::<Vec<_>>(),
            selected_paint_ids
        );
        assert!(!map.geometry_objects[1].faces[0].surface_points.is_empty());
        assert_eq!(map.selected_geometry_objects, vec![selected_id]);
    }
}
