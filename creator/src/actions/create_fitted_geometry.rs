use crate::actions::geometry_face_ops::face_uvs_for_indices;
use crate::editor::RUSTERIX;
use crate::prelude::*;
use earcutr::earcut;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Edge = (usize, usize);

#[derive(Debug, PartialEq, Eq)]
enum FittedGeometryError {
    Selection,
    Contours,
}

struct FittedSelection {
    object_index: usize,
    band_faces: BTreeSet<usize>,
    loops: [Vec<usize>; 2],
}

fn normalized_edge(a: usize, b: usize) -> Edge {
    if a < b { (a, b) } else { (b, a) }
}

fn face_edges(face: &rusterix::GeometryFace) -> impl Iterator<Item = Edge> + '_ {
    (0..face.indices.len()).map(|index| {
        normalized_edge(
            face.indices[index],
            face.indices[(index + 1) % face.indices.len()],
        )
    })
}

fn closed_boundary_loops(edges: &BTreeSet<Edge>) -> Option<Vec<Vec<usize>>> {
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for (a, b) in edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    if adjacency.is_empty() || adjacency.values().any(|neighbors| neighbors.len() != 2) {
        return None;
    }

    let mut remaining = adjacency.keys().copied().collect::<BTreeSet<_>>();
    let mut loops = Vec::new();
    while let Some(start) = remaining.first().copied() {
        let mut ordered = Vec::new();
        let mut previous = None;
        let mut current = start;
        loop {
            if ordered.contains(&current) {
                return None;
            }
            ordered.push(current);
            remaining.remove(&current);
            let neighbors = adjacency.get(&current)?;
            let next = neighbors
                .iter()
                .copied()
                .find(|neighbor| Some(*neighbor) != previous)?;
            previous = Some(current);
            current = next;
            if current == start {
                break;
            }
            if ordered.len() > adjacency.len() {
                return None;
            }
        }
        if ordered.len() < 3 {
            return None;
        }
        loops.push(ordered);
    }
    Some(loops)
}

fn fitted_selection(map: &Map) -> Result<FittedSelection, FittedGeometryError> {
    let selected_by_object = map.selected_geometry_vertices.iter().fold(
        BTreeMap::<Uuid, BTreeSet<usize>>::new(),
        |mut out, selection| {
            out.entry(selection.0).or_default().insert(selection.1);
            out
        },
    );
    if selected_by_object.len() != 1 {
        return Err(FittedGeometryError::Selection);
    }
    let (object_id, selected) = selected_by_object.into_iter().next().unwrap();
    if selected.len() < 6 {
        return Err(FittedGeometryError::Selection);
    }
    let Some(object_index) = map
        .geometry_objects
        .iter()
        .position(|object| object.id == object_id)
    else {
        return Err(FittedGeometryError::Selection);
    };
    let object = &map.geometry_objects[object_index];
    if selected.iter().any(|index| *index >= object.vertices.len()) {
        return Err(FittedGeometryError::Selection);
    }

    // After C followed by L, the reveal faces are the faces for which every vertex is selected.
    // Their boundary is exactly the pair of contours on either side of the opening.
    let band_faces = object
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            (face.indices.len() >= 3 && face.indices.iter().all(|index| selected.contains(index)))
                .then_some(face_index)
        })
        .collect::<BTreeSet<_>>();
    if band_faces.is_empty() {
        return Err(FittedGeometryError::Contours);
    }

    let mut edge_counts = BTreeMap::<Edge, usize>::new();
    for face_index in &band_faces {
        for edge in face_edges(&object.faces[*face_index]) {
            *edge_counts.entry(edge).or_default() += 1;
        }
    }
    if edge_counts.values().any(|count| *count > 2) {
        return Err(FittedGeometryError::Contours);
    }
    let boundary = edge_counts
        .into_iter()
        .filter_map(|(edge, count)| (count == 1).then_some(edge))
        .collect::<BTreeSet<_>>();
    let loops = closed_boundary_loops(&boundary).ok_or(FittedGeometryError::Contours)?;
    let Ok(loops) = <Vec<Vec<usize>> as TryInto<[Vec<usize>; 2]>>::try_into(loops) else {
        return Err(FittedGeometryError::Contours);
    };

    // Reject disconnected collections of selected faces that merely happen to have two loops.
    let mut connected = BTreeSet::new();
    let mut queue = VecDeque::from([*band_faces.first().unwrap()]);
    while let Some(face_index) = queue.pop_front() {
        if !connected.insert(face_index) {
            continue;
        }
        let edges = face_edges(&object.faces[face_index]).collect::<BTreeSet<_>>();
        for neighbor in &band_faces {
            if !connected.contains(neighbor)
                && face_edges(&object.faces[*neighbor]).any(|edge| edges.contains(&edge))
            {
                queue.push_back(*neighbor);
            }
        }
    }
    if connected != band_faces {
        return Err(FittedGeometryError::Contours);
    }

    Ok(FittedSelection {
        object_index,
        band_faces,
        loops,
    })
}

fn local_face_normal(vertices: &[Vec3<f32>], face: &rusterix::GeometryFace) -> Option<Vec3<f32>> {
    let first = *vertices.get(*face.indices.first()?)?;
    let mut normal = Vec3::zero();
    for index in 1..face.indices.len().saturating_sub(1) {
        let a = *vertices.get(face.indices[index])? - first;
        let b = *vertices.get(face.indices[index + 1])? - first;
        normal += a.cross(b);
    }
    normal.try_normalized()
}

fn polygon_normal(points: &[Vec3<f32>]) -> Option<Vec3<f32>> {
    if points.len() < 3 {
        return None;
    }
    let mut normal = Vec3::zero();
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal.try_normalized()
}

fn loop_center(vertices: &[Vec3<f32>], indices: &[usize]) -> Option<Vec3<f32>> {
    let mut center = Vec3::zero();
    for index in indices {
        center += *vertices.get(*index)?;
    }
    Some(center / indices.len() as f32)
}

fn source_face_for_loop<'a>(
    object: &'a rusterix::GeometryObject,
    band_faces: &BTreeSet<usize>,
    loop_indices: &[usize],
) -> Option<&'a rusterix::GeometryFace> {
    for index in 0..loop_indices.len() {
        let edge = normalized_edge(
            loop_indices[index],
            loop_indices[(index + 1) % loop_indices.len()],
        );
        if let Some(face) = object
            .faces
            .iter()
            .enumerate()
            .find_map(|(face_index, face)| {
                (!band_faces.contains(&face_index)
                    && face_edges(face).any(|candidate| candidate == edge))
                .then_some(face)
            })
        {
            return Some(face);
        }
    }
    None
}

fn new_face(
    indices: Vec<usize>,
    source: Option<&rusterix::GeometryFace>,
) -> rusterix::GeometryFace {
    rusterix::GeometryFace {
        id: Uuid::new_v4(),
        paint_surface_id: None,
        indices,
        uvs: Vec::new(),
        paint_uvs: Vec::new(),
        auto_uv: true,
        texture_offset: source.map_or_else(Vec2::zero, |face| face.texture_offset),
        texture_scale: source.map_or_else(Vec2::one, |face| face.texture_scale),
        texture_rotation: source.map_or(0.0, |face| face.texture_rotation),
        tile: source.and_then(|face| face.tile.clone()),
        tiles: source.map_or_else(FxHashMap::default, |face| face.tiles.clone()),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
    }
}

fn append_cap(
    output: &mut rusterix::GeometryObject,
    source_object: &rusterix::GeometryObject,
    old_loop: &[usize],
    remap: &BTreeMap<usize, usize>,
    desired_normal: Vec3<f32>,
    source_face: Option<&rusterix::GeometryFace>,
) -> bool {
    let points = old_loop
        .iter()
        .filter_map(|index| source_object.vertices.get(*index).copied())
        .collect::<Vec<_>>();
    if points.len() != old_loop.len() {
        return false;
    }
    let Some(normal) = polygon_normal(&points) else {
        return false;
    };
    let tangent = (points[1] - points[0])
        .try_normalized()
        .or_else(|| (points[2] - points[0]).try_normalized());
    let Some(tangent) = tangent else {
        return false;
    };
    let Some(bitangent) = normal.cross(tangent).try_normalized() else {
        return false;
    };
    let origin = points[0];
    let flat = points
        .iter()
        .flat_map(|point| {
            let local = *point - origin;
            [local.dot(tangent) as f64, local.dot(bitangent) as f64]
        })
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
            .filter_map(|index| remap.get(&old_loop[*index]).copied())
            .collect::<Vec<_>>();
        if indices.len() != 3 {
            return false;
        }
        let a = output.vertices[indices[0]];
        let b = output.vertices[indices[1]];
        let c = output.vertices[indices[2]];
        if (b - a).cross(c - a).dot(desired_normal) < 0.0 {
            indices.reverse();
        }
        output.faces.push(new_face(indices, source_face));
    }
    true
}

fn create_fitted_geometry(map: &mut Map) -> Result<Uuid, FittedGeometryError> {
    let selection = fitted_selection(map)?;
    let source = map.geometry_objects[selection.object_index].clone();
    let selected_vertices = selection
        .band_faces
        .iter()
        .flat_map(|face_index| source.faces[*face_index].indices.iter().copied())
        .collect::<BTreeSet<_>>();

    let mut fitted = rusterix::GeometryObject::new("Fitted Geometry");
    fitted.kind = rusterix::GeometryObjectKind::Prop;
    fitted.transform = source.transform;
    let mut remap = BTreeMap::new();
    for old_index in selected_vertices {
        let Some(vertex) = source.vertices.get(old_index).copied() else {
            return Err(FittedGeometryError::Selection);
        };
        remap.insert(old_index, fitted.vertices.len());
        fitted.vertices.push(vertex);
    }

    // The wall reveal points into the wall. Reversing it gives the fitted solid the matching
    // inward-facing side winding while retaining the reveal's material properties.
    for face_index in &selection.band_faces {
        let source_face = &source.faces[*face_index];
        let mut indices = source_face
            .indices
            .iter()
            .filter_map(|index| remap.get(index).copied())
            .collect::<Vec<_>>();
        if indices.len() != source_face.indices.len() {
            return Err(FittedGeometryError::Selection);
        }
        indices.reverse();
        fitted.faces.push(new_face(indices, Some(source_face)));
    }

    let centers = [
        loop_center(&source.vertices, &selection.loops[0]).ok_or(FittedGeometryError::Contours)?,
        loop_center(&source.vertices, &selection.loops[1]).ok_or(FittedGeometryError::Contours)?,
    ];
    for loop_index in 0..2 {
        let source_face =
            source_face_for_loop(&source, &selection.band_faces, &selection.loops[loop_index]);
        let fallback = (centers[1 - loop_index] - centers[loop_index])
            .try_normalized()
            .ok_or(FittedGeometryError::Contours)?;
        let desired_normal = source_face
            .and_then(|face| local_face_normal(&source.vertices, face))
            .unwrap_or(fallback);
        if !append_cap(
            &mut fitted,
            &source,
            &selection.loops[loop_index],
            &remap,
            desired_normal,
            source_face,
        ) {
            return Err(FittedGeometryError::Contours);
        }
    }

    for face_index in 0..fitted.faces.len() {
        let indices = fitted.faces[face_index].indices.clone();
        fitted.faces[face_index].uvs = face_uvs_for_indices(&fitted, &indices);
    }
    fitted.ensure_face_paint_data();

    let fitted_id = fitted.id;
    let face_count = fitted.faces.len();
    map.geometry_objects.push(fitted);
    map.clear_selection();
    map.selected_geometry_objects.push(fitted_id);
    map.selected_geometry_faces = (0..face_count)
        .map(|face_index| (fitted_id, face_index))
        .collect();
    map.changed = map.changed.wrapping_add(1);
    Ok(fitted_id)
}

pub struct CreateFittedGeometry {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for CreateFittedGeometry {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_create_fitted_geometry_desc"),
        ));
        Self {
            id: TheId::named(&fl!("action_create_fitted_geometry")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_fitted_geometry_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && fitted_selection(map).is_ok()
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let previous = map.clone();
        if create_fitted_geometry(map).is_err() {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("status_create_fitted_geometry_failed"),
            ));
            return None;
        }

        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.set_dirty();
            rusterix.set_overlay_dirty();
        }
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_create_fitted_geometry_created"),
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(previous),
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

    fn face(indices: Vec<usize>) -> rusterix::GeometryFace {
        new_face(indices, None)
    }

    fn opening_band_map() -> (Map, Uuid) {
        let mut map = Map::new();
        let mut object = rusterix::GeometryObject::new("Opening");
        object.vertices = vec![
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        object.faces = vec![
            face(vec![0, 1, 5, 4]),
            face(vec![1, 2, 6, 5]),
            face(vec![2, 3, 7, 6]),
            face(vec![3, 0, 4, 7]),
        ];
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_objects.push(object_id);
        map.selected_geometry_vertices = (0..8).map(|index| (object_id, index)).collect();
        (map, object_id)
    }

    #[test]
    fn finds_two_contours_around_selected_reveal_band() {
        let (map, _) = opening_band_map();
        let selection = fitted_selection(&map).expect("opening band should be valid");

        assert_eq!(selection.band_faces.len(), 4);
        assert_eq!(selection.loops.len(), 2);
        assert!(
            selection
                .loops
                .iter()
                .all(|loop_indices| loop_indices.len() == 4)
        );
    }

    #[test]
    fn creates_independent_capped_solid_without_changing_source() {
        let (mut map, source_id) = opening_band_map();
        let source = map.geometry_objects[0].clone();

        let fitted_id = create_fitted_geometry(&mut map).expect("fitted solid should be created");

        assert_eq!(map.geometry_objects.len(), 2);
        assert_eq!(map.geometry_objects[0], source);
        let fitted = map
            .geometry_objects
            .iter()
            .find(|object| object.id == fitted_id)
            .unwrap();
        assert_eq!(fitted.vertices.len(), 8);
        assert_eq!(fitted.faces.len(), 8);
        assert_ne!(fitted.id, source_id);
        let mut edge_counts = BTreeMap::<Edge, usize>::new();
        for fitted_face in &fitted.faces {
            for edge in face_edges(fitted_face) {
                *edge_counts.entry(edge).or_default() += 1;
            }
        }
        assert!(edge_counts.values().all(|count| *count == 2));
        assert_eq!(map.selected_geometry_objects, vec![fitted_id]);
        assert_eq!(map.selected_geometry_faces.len(), fitted.faces.len());
    }
}
