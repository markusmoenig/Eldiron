use crate::editor::RUSTERIX;
use crate::prelude::*;
use earcutr::earcut;
use theframework::prelude::FxHashMap;

pub struct CutStairs {
    id: TheId,
    nodeui: TheNodeUI,
}

#[derive(Clone, Copy)]
struct StairPoint {
    run: f32,
    drop: f32,
}

fn selected_stair_faces(map: &Map) -> Option<(usize, usize, usize)> {
    if map.selected_geometry_faces.len() != 2 {
        return None;
    }

    let (object_id, first_face) = map.selected_geometry_faces[0];
    let (second_object_id, second_face) = map.selected_geometry_faces[1];
    if object_id != second_object_id {
        return None;
    }

    let object_index = map
        .geometry_objects
        .iter()
        .position(|object| object.id == object_id)?;
    Some((object_index, first_face, second_face))
}

fn local_face_edit_normal(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    if face.indices.len() < 3 {
        return None;
    }
    let first = *object.vertices.get(face.indices[0])?;
    let mut normal = Vec3::<f32>::zero();
    for index in 1..face.indices.len() - 1 {
        let a = *object.vertices.get(face.indices[index])? - first;
        let b = *object.vertices.get(face.indices[index + 1])? - first;
        normal += a.cross(b);
    }
    normal.try_normalized().map(|normal| -normal)
}

fn shared_edge(
    first: &rusterix::GeometryFace,
    second: &rusterix::GeometryFace,
) -> Option<(usize, usize)> {
    let shared = first
        .indices
        .iter()
        .copied()
        .filter(|index| second.indices.contains(index))
        .collect::<Vec<_>>();
    (shared.len() == 2).then_some((shared[0], shared[1]))
}

fn geometry_face(
    indices: Vec<usize>,
    source_face: Option<&rusterix::GeometryFace>,
) -> rusterix::GeometryFace {
    rusterix::GeometryFace {
        indices,
        uvs: Vec::new(),
        auto_uv: true,
        texture_offset: source_face
            .map(|face| face.texture_offset)
            .unwrap_or_else(Vec2::zero),
        texture_scale: source_face
            .map(|face| face.texture_scale)
            .unwrap_or_else(|| Vec2::broadcast(1.0)),
        texture_rotation: source_face.map(|face| face.texture_rotation).unwrap_or(0.0),
        tile: source_face.and_then(|face| face.tile.clone()),
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        surface_noise: None,
    }
}

fn add_triangulated_cap_faces(
    faces: &mut Vec<rusterix::GeometryFace>,
    profile: &[StairPoint],
    vertex_offset: usize,
    reverse: bool,
    source_face: &rusterix::GeometryFace,
) -> bool {
    let flat = profile
        .iter()
        .flat_map(|point| [point.run as f64, (-point.drop) as f64])
        .collect::<Vec<_>>();
    let Ok(triangles) = earcut(&flat, &[], 2) else {
        return false;
    };
    if triangles.is_empty() {
        return false;
    }

    for triangle in triangles.chunks_exact(3) {
        let mut indices = triangle
            .iter()
            .map(|index| vertex_offset + *index)
            .collect::<Vec<_>>();
        if reverse {
            indices.reverse();
        }
        faces.push(geometry_face(indices, Some(source_face)));
    }
    true
}

fn best_bottom_face<'a>(
    object: &'a rusterix::GeometryObject,
    top_normal: Vec3<f32>,
) -> Option<&'a rusterix::GeometryFace> {
    object
        .faces
        .iter()
        .filter_map(|face| {
            local_face_edit_normal(object, face).map(|normal| (face, normal.dot(top_normal)))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .and_then(|(face, dot)| (dot < -0.75).then_some(face))
}

fn cut_stairs_into_object(
    object: &mut rusterix::GeometryObject,
    first_face_index: usize,
    second_face_index: usize,
    target_step_height: f32,
    landing: f32,
) -> bool {
    let Some(first_face) = object.faces.get(first_face_index).cloned() else {
        return false;
    };
    let Some(second_face) = object.faces.get(second_face_index).cloned() else {
        return false;
    };
    let Some(first_normal) = local_face_edit_normal(object, &first_face) else {
        return false;
    };
    let Some(second_normal) = local_face_edit_normal(object, &second_face) else {
        return false;
    };

    let (top_face, side_face, top_normal, side_normal) =
        if first_normal.y.abs() >= 0.75 && second_normal.y.abs() <= 0.25 {
            (first_face, second_face, first_normal, second_normal)
        } else if second_normal.y.abs() >= 0.75 && first_normal.y.abs() <= 0.25 {
            (second_face, first_face, second_normal, first_normal)
        } else {
            return false;
        };
    if top_normal.y <= 0.0 || top_normal.dot(side_normal).abs() > 0.25 {
        return false;
    }

    let Some((edge_a, edge_b)) = shared_edge(&top_face, &side_face) else {
        return false;
    };
    let Some(edge_start) = object.vertices.get(edge_a).copied() else {
        return false;
    };
    let Some(edge_end) = object.vertices.get(edge_b).copied() else {
        return false;
    };
    let Some(width_axis) = (edge_end - edge_start).try_normalized() else {
        return false;
    };
    let mut run_axis = (-side_normal - top_normal * (-side_normal).dot(top_normal))
        .try_normalized()
        .unwrap_or_else(Vec3::unit_z);

    let edge_center = (edge_start + edge_end) * 0.5;
    let max_top_run = top_face
        .indices
        .iter()
        .filter_map(|index| object.vertices.get(*index))
        .map(|point| (*point - edge_center).dot(run_axis))
        .fold(f32::NEG_INFINITY, f32::max);
    let min_top_run = top_face
        .indices
        .iter()
        .filter_map(|index| object.vertices.get(*index))
        .map(|point| (*point - edge_center).dot(run_axis))
        .fold(f32::INFINITY, f32::min);
    if max_top_run.abs() < min_top_run.abs() {
        run_axis = -run_axis;
    }

    let run_values = top_face
        .indices
        .iter()
        .filter_map(|index| object.vertices.get(*index))
        .map(|point| (*point - edge_center).dot(run_axis))
        .collect::<Vec<_>>();
    let total_run = run_values
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max)
        .max(0.0);
    if total_run <= 0.01 {
        return false;
    }

    let top_height = edge_center.dot(top_normal);
    let bottom_height = side_face
        .indices
        .iter()
        .filter_map(|index| object.vertices.get(*index))
        .map(|point| point.dot(top_normal))
        .fold(f32::INFINITY, f32::min);
    let total_height = top_height - bottom_height;
    if total_height <= 0.01 {
        return false;
    }

    let landing = landing.clamp(0.0, (total_run - 0.01).max(0.0));
    let stair_run = total_run - landing;
    if stair_run <= 0.01 {
        return false;
    }

    let target_step_height = target_step_height.max(0.01);
    let steps = (total_height / target_step_height).round().max(1.0) as usize;
    let step_height = total_height / steps as f32;
    let step_depth = stair_run / steps as f32;

    let mut profile = vec![
        StairPoint {
            run: 0.0,
            drop: total_height,
        },
        StairPoint {
            run: total_run,
            drop: total_height,
        },
        StairPoint {
            run: total_run,
            drop: 0.0,
        },
        StairPoint {
            run: stair_run,
            drop: 0.0,
        },
    ];
    for step in (0..steps).rev() {
        let run = step as f32 * step_depth;
        let upper_drop = total_height - (step + 1) as f32 * step_height;
        let lower_drop = total_height - step as f32 * step_height;
        profile.push(StairPoint {
            run,
            drop: upper_drop,
        });
        profile.push(StairPoint {
            run,
            drop: lower_drop,
        });
    }
    if profile
        .last()
        .is_some_and(|point| point.run.abs() <= 1e-4 && (point.drop - total_height).abs() <= 1e-4)
    {
        profile.pop();
    }

    let Some(bottom_face) = best_bottom_face(object, top_normal).cloned() else {
        return false;
    };
    let side_cap_face = side_face.clone();
    let top_source = top_face.clone();
    let side_source = side_face.clone();

    let width = (edge_end - edge_start).magnitude();
    if width <= 0.01 {
        return false;
    }

    let mut vertices = Vec::with_capacity(profile.len() * 2);
    for width_pos in [0.0, width] {
        for point in &profile {
            vertices.push(
                edge_start + width_axis * width_pos + run_axis * point.run
                    - top_normal * point.drop,
            );
        }
    }

    let count = profile.len();
    let mut faces = Vec::new();
    if !add_triangulated_cap_faces(&mut faces, &profile, 0, true, &side_cap_face) {
        return false;
    }
    if !add_triangulated_cap_faces(&mut faces, &profile, count, false, &side_cap_face) {
        return false;
    }

    for index in 0..count {
        let next = (index + 1) % count;
        let a = index;
        let b = next;
        let c = next + count;
        let d = index + count;
        let segment = profile[next].run - profile[index].run;
        let rise = profile[next].drop - profile[index].drop;
        let source = if rise.abs() <= 1e-4 && profile[index].drop <= 1e-4 {
            Some(&top_source)
        } else if rise.abs() <= 1e-4 && profile[index].drop >= total_height - 1e-4 {
            Some(&bottom_face)
        } else if rise.abs() <= 1e-4 && segment.abs() > 1e-4 {
            Some(&top_source)
        } else {
            Some(&side_source)
        };
        faces.push(geometry_face(vec![a, b, c, d], source));
    }

    object.vertices = vertices;
    object.faces = faces;
    true
}

pub fn cut_stairs_into_selected_faces(
    map: &mut Map,
    target_step_height: f32,
    landing: f32,
) -> bool {
    let Some((object_index, first_face, second_face)) = selected_stair_faces(map) else {
        return false;
    };
    let Some(object) = map.geometry_objects.get_mut(object_index) else {
        return false;
    };
    if !cut_stairs_into_object(object, first_face, second_face, target_step_height, landing) {
        return false;
    }

    map.selected_geometry_faces = vec![(object.id, 0)];
    map.selected_geometry_vertices.clear();
    map.selected_geometry_surface_points.clear();
    map.selected_geometry_surface_segments.clear();
    true
}

impl Action for CutStairs {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_cut_stairs_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCutStairsStepHeight".into(),
            "Step Height".into(),
            "".into(),
            0.25,
            0.01..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCutStairsLanding".into(),
            "Landing".into(),
            "".into(),
            0.0,
            0.0..=256.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_cut_stairs")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_cut_stairs_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && map.selected_geometry_faces.len() == 2
    }

    fn load_params(&mut self, map: &Map) {
        let step = ServerContext::edit_grid_step(map.subdivisions);
        self.nodeui
            .set_f32_value("actionCutStairsStepHeight", step.max(0.01));
        self.nodeui.set_f32_value("actionCutStairsLanding", 0.0);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let step_height = self
            .nodeui
            .get_f32_value("actionCutStairsStepHeight")
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions))
            .max(0.01);
        let landing = self
            .nodeui
            .get_f32_value("actionCutStairsLanding")
            .unwrap_or(0.0)
            .max(0.0);

        let prev = map.clone();
        if !cut_stairs_into_selected_faces(map, step_height, landing) {
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

    #[test]
    fn cut_stairs_rebuilds_one_box_object() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 1.0, 2.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_faces = vec![(object_id, 4), (object_id, 3)];

        assert!(cut_stairs_into_selected_faces(&mut map, 0.25, 1.0));
        assert_eq!(map.geometry_objects.len(), 1);
        let object = &map.geometry_objects[0];
        assert!(object.faces.len() > 6);
        assert_eq!(object.vertices.len(), 22);
        assert_eq!(map.selected_geometry_faces.len(), 1);
    }

    #[test]
    fn cut_stairs_requires_two_faces_on_same_object() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "box",
            Vec3::zero(),
            Vec3::new(1.0, 1.0, 1.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_faces = vec![(object_id, 4)];

        assert!(!cut_stairs_into_selected_faces(&mut map, 0.25, 0.0));
    }
}
