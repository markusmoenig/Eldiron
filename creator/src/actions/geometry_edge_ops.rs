use crate::actions::geometry_face_ops::face_uvs_for_indices;
use crate::prelude::*;
use rusterix::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

type Edge = (usize, usize);

fn normalized_edge(a: usize, b: usize) -> Edge {
    if a < b { (a, b) } else { (b, a) }
}

fn face_normal(object: &rusterix::GeometryObject, face_index: usize) -> Option<Vec3<f32>> {
    let face = object.faces.get(face_index)?;
    let first = *object.vertices.get(*face.indices.first()?)?;
    let mut normal = Vec3::zero();
    for index in 1..face.indices.len().saturating_sub(1) {
        normal += (*object.vertices.get(face.indices[index])? - first)
            .cross(*object.vertices.get(face.indices[index + 1])? - first);
    }
    normal.try_normalized()
}

fn face_center(object: &rusterix::GeometryObject, face_index: usize) -> Option<Vec3<f32>> {
    let face = object.faces.get(face_index)?;
    if face.indices.is_empty() {
        return None;
    }
    let mut center = Vec3::zero();
    for vertex_index in &face.indices {
        center += *object.vertices.get(*vertex_index)?;
    }
    Some(center / face.indices.len() as f32)
}

fn selected_edges(
    object: &rusterix::GeometryObject,
    selected_vertices: &BTreeSet<usize>,
) -> BTreeSet<Edge> {
    let mut edges = BTreeSet::new();
    for face in &object.faces {
        for index in 0..face.indices.len() {
            let a = face.indices[index];
            let b = face.indices[(index + 1) % face.indices.len()];
            if selected_vertices.contains(&a) && selected_vertices.contains(&b) {
                edges.insert(normalized_edge(a, b));
            }
        }
    }
    edges
}

fn inward_for_edge(
    object: &rusterix::GeometryObject,
    face_index: usize,
    edge: Edge,
) -> Option<Vec3<f32>> {
    let normal = face_normal(object, face_index)?;
    let a = *object.vertices.get(edge.0)?;
    let b = *object.vertices.get(edge.1)?;
    let direction = (b - a).try_normalized()?;
    let mut inward = normal.cross(direction).try_normalized()?;
    let center = face_center(object, face_index)?;
    if inward.dot(center - (a + b) * 0.5) < 0.0 {
        inward = -inward;
    }
    Some(inward)
}

fn offset_face_vertex(
    object: &rusterix::GeometryObject,
    face_index: usize,
    vertex_index: usize,
    valid_edges: &BTreeSet<Edge>,
    width: f32,
) -> Vec3<f32> {
    let source = object.vertices[vertex_index];
    let Some(face) = object.faces.get(face_index) else {
        return source;
    };
    let mut inwards = Vec::new();
    let mut effective_width = width;
    for index in 0..face.indices.len() {
        let a = face.indices[index];
        let b = face.indices[(index + 1) % face.indices.len()];
        let edge = normalized_edge(a, b);
        if !valid_edges.contains(&edge) || (a != vertex_index && b != vertex_index) {
            continue;
        }
        effective_width =
            effective_width.min((object.vertices[a] - object.vertices[b]).magnitude() * 0.45);
        if let Some(inward) = inward_for_edge(object, face_index, edge) {
            inwards.push(inward);
        }
    }

    match inwards.as_slice() {
        [] => source,
        [inward] => source + *inward * effective_width,
        [first, second, ..] => {
            let denominator = (1.0 + first.dot(*second)).max(0.1);
            let mut delta = (*first + *second) * (effective_width / denominator);
            let max_miter = effective_width * 4.0;
            if delta.magnitude_squared() > max_miter * max_miter {
                delta = delta.try_normalized().unwrap_or(*first) * max_miter;
            }
            source + delta
        }
    }
}

fn oriented_indices(
    object: &rusterix::GeometryObject,
    mut indices: Vec<usize>,
    desired_normal: Vec3<f32>,
) -> Vec<usize> {
    if indices.len() >= 3 {
        let first = object.vertices[indices[0]];
        let mut normal = Vec3::zero();
        for index in 1..indices.len() - 1 {
            normal += (object.vertices[indices[index]] - first)
                .cross(object.vertices[indices[index + 1]] - first);
        }
        if normal.dot(desired_normal) < 0.0 {
            indices.reverse();
        }
    }
    indices
}

fn append_face(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    indices: Vec<usize>,
    desired_normal: Vec3<f32>,
    smoothing_group: u32,
) -> usize {
    let indices = oriented_indices(object, indices, desired_normal);
    let mut face = source.clone();
    face.id = Uuid::new_v4();
    face.paint_surface_id = None;
    face.indices = indices;
    face.uvs = face_uvs_for_indices(object, &face.indices);
    face.paint_uvs = rusterix::geometry_face_paint_uvs(
        &face
            .indices
            .iter()
            .map(|index| object.vertices[*index])
            .collect::<Vec<_>>(),
    );
    face.auto_uv = true;
    face.tiles.clear();
    face.surface_points.clear();
    face.surface_segments.clear();
    face.smoothing_group = smoothing_group;
    let face_index = object.faces.len();
    object.faces.push(face);
    face_index
}

fn sorted_cap_indices(
    object: &rusterix::GeometryObject,
    source_vertex: Vec3<f32>,
    indices: &BTreeSet<usize>,
    desired_normal: Vec3<f32>,
) -> Vec<usize> {
    let normal = desired_normal.try_normalized().unwrap_or_else(Vec3::unit_y);
    let tangent = indices
        .iter()
        .map(|index| object.vertices[*index] - source_vertex)
        .find(|offset| offset.magnitude_squared() > 1e-10)
        .and_then(|offset| offset.try_normalized())
        .unwrap_or_else(Vec3::unit_x);
    let bitangent = normal
        .cross(tangent)
        .try_normalized()
        .unwrap_or_else(Vec3::unit_z);
    let mut sorted = indices.iter().copied().collect::<Vec<_>>();
    sorted.sort_by(|a, b| {
        let pa = object.vertices[*a] - source_vertex;
        let pb = object.vertices[*b] - source_vertex;
        let aa = pa.dot(bitangent).atan2(pa.dot(tangent));
        let ab = pb.dot(bitangent).atan2(pb.dot(tangent));
        aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
    });
    oriented_indices(object, sorted, normal)
}

fn bevel_object_edges(
    object: &mut rusterix::GeometryObject,
    selected_vertices: &BTreeSet<usize>,
    width: f32,
    segments: usize,
    profile: f32,
) -> Vec<usize> {
    let original = object.clone();
    let candidates = selected_edges(&original, selected_vertices);
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut edge_faces = BTreeMap::<Edge, Vec<usize>>::new();
    let mut incident_faces = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (face_index, face) in original.faces.iter().enumerate() {
        for vertex_index in &face.indices {
            incident_faces
                .entry(*vertex_index)
                .or_default()
                .insert(face_index);
        }
        for index in 0..face.indices.len() {
            edge_faces
                .entry(normalized_edge(
                    face.indices[index],
                    face.indices[(index + 1) % face.indices.len()],
                ))
                .or_default()
                .push(face_index);
        }
    }

    let valid_edges = candidates
        .into_iter()
        .filter(|edge| {
            let Some(faces) = edge_faces.get(edge) else {
                return false;
            };
            if faces.len() != 2 {
                return false;
            }
            let Some(first) = face_normal(&original, faces[0]) else {
                return false;
            };
            let Some(second) = face_normal(&original, faces[1]) else {
                return false;
            };
            first.dot(second) < 0.9995
                && (original.vertices[edge.0] - original.vertices[edge.1]).magnitude_squared()
                    > 1e-10
        })
        .collect::<BTreeSet<_>>();
    if valid_edges.is_empty() {
        return Vec::new();
    }

    let touched_vertices = valid_edges
        .iter()
        .flat_map(|edge| [edge.0, edge.1])
        .collect::<BTreeSet<_>>();
    let mut face_vertex = BTreeMap::<(usize, usize), usize>::new();
    for vertex_index in &touched_vertices {
        let Some(faces) = incident_faces.get(vertex_index) else {
            continue;
        };
        for face_index in faces {
            let point =
                offset_face_vertex(&original, *face_index, *vertex_index, &valid_edges, width);
            let new_index = object.vertices.len();
            object.vertices.push(point);
            face_vertex.insert((*face_index, *vertex_index), new_index);
        }
    }

    for (face_index, face) in object.faces.iter_mut().enumerate() {
        for vertex_index in &mut face.indices {
            if let Some(replacement) = face_vertex.get(&(face_index, *vertex_index)) {
                *vertex_index = *replacement;
            }
        }
    }

    let smoothing_group = original
        .faces
        .iter()
        .map(|face| face.smoothing_group)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    let segments = segments.clamp(1, 16);
    let profile = profile.clamp(0.0, 1.0);
    let mut cap_vertices = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut bevel_faces = Vec::new();

    for edge in &valid_edges {
        let faces = &edge_faces[edge];
        let first_normal = face_normal(&original, faces[0]).unwrap_or_else(Vec3::unit_y);
        let second_normal = face_normal(&original, faces[1]).unwrap_or_else(Vec3::unit_y);
        let desired_normal = (first_normal + second_normal)
            .try_normalized()
            .unwrap_or(first_normal);
        let mut rails = [
            Vec::with_capacity(segments + 1),
            Vec::with_capacity(segments + 1),
        ];

        for (endpoint, vertex_index) in [edge.0, edge.1].into_iter().enumerate() {
            let first_index = face_vertex[&(faces[0], vertex_index)];
            let second_index = face_vertex[&(faces[1], vertex_index)];
            let first = object.vertices[first_index];
            let second = object.vertices[second_index];
            let source = original.vertices[vertex_index];
            for step in 0..=segments {
                let index = if step == 0 {
                    first_index
                } else if step == segments {
                    second_index
                } else {
                    let t = step as f32 / segments as f32;
                    let midpoint = (first + second) * 0.5;
                    let control = midpoint * (1.0 - profile) + source * profile;
                    let one_minus_t = 1.0 - t;
                    let point = first * (one_minus_t * one_minus_t)
                        + control * (2.0 * one_minus_t * t)
                        + second * (t * t);
                    let index = object.vertices.len();
                    object.vertices.push(point);
                    index
                };
                rails[endpoint].push(index);
                cap_vertices.entry(vertex_index).or_default().insert(index);
            }
        }

        let source_face = original.faces[faces[0]].clone();
        for step in 0..segments {
            let face_index = append_face(
                object,
                &source_face,
                vec![
                    rails[0][step],
                    rails[1][step],
                    rails[1][step + 1],
                    rails[0][step + 1],
                ],
                desired_normal,
                smoothing_group,
            );
            bevel_faces.push(face_index);
        }
    }

    for vertex_index in &touched_vertices {
        let Some(faces) = incident_faces.get(vertex_index) else {
            continue;
        };
        let cap = cap_vertices.entry(*vertex_index).or_default();
        let mut desired_normal = Vec3::zero();
        let mut source_face_index = None;
        for face_index in faces {
            if let Some(mapped) = face_vertex.get(&(*face_index, *vertex_index)) {
                cap.insert(*mapped);
            }
            if let Some(normal) = face_normal(&original, *face_index) {
                desired_normal += normal;
            }
            source_face_index.get_or_insert(*face_index);
        }
        if cap.len() < 3 {
            continue;
        }
        let Some(source_face_index) = source_face_index else {
            continue;
        };
        let desired_normal = desired_normal.try_normalized().unwrap_or_else(Vec3::unit_y);
        let indices = sorted_cap_indices(
            object,
            original.vertices[*vertex_index],
            cap,
            desired_normal,
        );
        let source_face = original.faces[source_face_index].clone();
        let face_index = append_face(
            object,
            &source_face,
            indices,
            desired_normal,
            smoothing_group,
        );
        bevel_faces.push(face_index);
    }

    bevel_faces
}

pub(crate) fn bevel_selected_geometry_edges(
    map: &mut Map,
    width: f32,
    segments: usize,
    profile: f32,
) -> bool {
    if map.geometry_selection_mode != 3 || width <= 0.0 {
        return false;
    }
    let selected = map.selected_geometry_vertices.clone();
    let mut selected_faces = Vec::new();
    for object in &mut map.geometry_objects {
        let selected_vertices = selected
            .iter()
            .filter_map(|(object_id, vertex_index)| {
                (*object_id == object.id && *vertex_index < object.vertices.len())
                    .then_some(*vertex_index)
            })
            .collect::<BTreeSet<_>>();
        if selected_vertices.len() < 2 {
            continue;
        }
        selected_faces.extend(
            bevel_object_edges(object, &selected_vertices, width, segments, profile)
                .into_iter()
                .map(|face_index| (object.id, face_index)),
        );
    }
    if selected_faces.is_empty() {
        return false;
    }

    map.selected_geometry_vertices.clear();
    map.selected_geometry_faces = selected_faces;
    map.geometry_selection_mode = 1;
    true
}
