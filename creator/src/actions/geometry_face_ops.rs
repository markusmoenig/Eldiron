use crate::prelude::*;
use rusterix::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn face_uvs_for_indices(
    object: &rusterix::GeometryObject,
    indices: &[usize],
) -> Vec<Vec2<f32>> {
    let points = indices
        .iter()
        .filter_map(|index| object.vertices.get(*index).copied())
        .collect::<Vec<_>>();
    if points.len() != indices.len() || points.len() < 3 {
        return indices.iter().map(|_| Vec2::zero()).collect();
    }

    let origin = points
        .iter()
        .copied()
        .fold(Vec3::zero(), |sum, point| sum + point)
        / points.len() as f32;
    let tangent = (points[1] - points[0])
        .try_normalized()
        .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
    let normal = {
        let mut normal = Vec3::zero();
        for index in 0..points.len() {
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            normal.x += (current.y - next.y) * (current.z + next.z);
            normal.y += (current.z - next.z) * (current.x + next.x);
            normal.z += (current.x - next.x) * (current.y + next.y);
        }
        normal.try_normalized().unwrap_or(Vec3::new(0.0, 1.0, 0.0))
    };
    let bitangent = normal
        .cross(tangent)
        .try_normalized()
        .unwrap_or(Vec3::new(0.0, 0.0, 1.0));

    let projected = points
        .iter()
        .map(|point| {
            let local = *point - origin;
            Vec2::new(local.dot(tangent), local.dot(bitangent))
        })
        .collect::<Vec<_>>();
    let (mut min, mut max) = (
        Vec2::new(f32::INFINITY, f32::INFINITY),
        Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
    );
    for uv in &projected {
        min.x = min.x.min(uv.x);
        min.y = min.y.min(uv.y);
        max.x = max.x.max(uv.x);
        max.y = max.y.max(uv.y);
    }
    let size = Vec2::new((max.x - min.x).max(1e-4), (max.y - min.y).max(1e-4));
    projected
        .iter()
        .map(|uv| Vec2::new((uv.x - min.x) / size.x, (uv.y - min.y) / size.y))
        .collect()
}

pub(crate) fn sanitize_geometry_selection(map: &mut Map) {
    let objects = map.geometry_objects.clone();
    map.selected_geometry_objects
        .retain(|object_id| objects.iter().any(|object| object.id == *object_id));

    map.selected_geometry_faces
        .retain(|(object_id, face_index)| {
            objects
                .iter()
                .find(|object| object.id == *object_id)
                .is_some_and(|object| *face_index < object.faces.len())
        });
    map.selected_geometry_vertices
        .retain(|(object_id, vertex_index)| {
            objects
                .iter()
                .find(|object| object.id == *object_id)
                .is_some_and(|object| *vertex_index < object.vertices.len())
        });

    for (object_id, _) in map.selected_geometry_faces.clone() {
        if !map.selected_geometry_objects.contains(&object_id) {
            map.selected_geometry_objects.push(object_id);
        }
    }
    for (object_id, _) in map.selected_geometry_vertices.clone() {
        if !map.selected_geometry_objects.contains(&object_id) {
            map.selected_geometry_objects.push(object_id);
        }
    }

    let mut seen_faces = BTreeSet::new();
    map.selected_geometry_faces
        .retain(|selection| seen_faces.insert(*selection));
    let mut seen_vertices = BTreeSet::new();
    map.selected_geometry_vertices
        .retain(|selection| seen_vertices.insert(*selection));
}

fn normalized_edge(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

fn ordered_boundary_vertices(edges: &[(usize, usize)]) -> Vec<usize> {
    if edges.is_empty() {
        return Vec::new();
    }

    let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (a, b) in edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }

    let start = adjacency
        .iter()
        .find_map(|(vertex, neighbors)| (neighbors.len() == 1).then_some(*vertex))
        .unwrap_or(edges[0].0);

    let mut ordered = Vec::new();
    let mut previous = None;
    let mut current = start;
    loop {
        ordered.push(current);
        let Some(neighbors) = adjacency.get(&current) else {
            break;
        };
        let next = neighbors
            .iter()
            .copied()
            .find(|neighbor| Some(*neighbor) != previous);
        let Some(next) = next else {
            break;
        };
        previous = Some(current);
        current = next;
        if current == start {
            break;
        }
        if ordered.len() > edges.len() + 1 {
            break;
        }
    }
    ordered
}

pub(crate) fn delete_selected_geometry_faces(map: &mut Map) -> bool {
    if map.selected_geometry_faces.is_empty() {
        return false;
    }

    let mut changed = false;
    let mut boundary_vertices = Vec::new();
    for object in &mut map.geometry_objects {
        let selected_faces = map
            .selected_geometry_faces
            .iter()
            .filter_map(|(object_id, face_index)| (*object_id == object.id).then_some(*face_index))
            .collect::<BTreeSet<_>>();
        if selected_faces.is_empty() {
            continue;
        }
        let mut edge_counts = BTreeMap::new();
        let mut directed_edges = Vec::new();
        for face_index in &selected_faces {
            let Some(face) = object.faces.get(*face_index) else {
                continue;
            };
            for index in 0..face.indices.len() {
                let a = face.indices[index];
                let b = face.indices[(index + 1) % face.indices.len()];
                *edge_counts.entry(normalized_edge(a, b)).or_insert(0usize) += 1;
                directed_edges.push((a, b));
            }
        }
        for vertex_index in ordered_boundary_vertices(
            &directed_edges
                .into_iter()
                .filter(|(a, b)| {
                    edge_counts
                        .get(&normalized_edge(*a, *b))
                        .is_some_and(|count| *count == 1)
                })
                .collect::<Vec<_>>(),
        ) {
            boundary_vertices.push((object.id, vertex_index));
        }

        let old_len = object.faces.len();
        let mut face_index = 0usize;
        object.faces.retain(|_| {
            let keep = !selected_faces.contains(&face_index);
            face_index += 1;
            keep
        });
        changed |= object.faces.len() != old_len;
    }
    if changed {
        map.selected_geometry_faces.clear();
        map.selected_geometry_vertices = boundary_vertices;
        sanitize_geometry_selection(map);
    }
    changed
}

pub(crate) fn fill_selected_geometry_vertices(map: &mut Map) -> bool {
    if map.selected_geometry_vertices.len() < 3 {
        return false;
    }

    let selected = map.selected_geometry_vertices.clone();
    let mut new_selected_faces = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let mut indices = selected
            .iter()
            .filter_map(|(object_id, vertex_index)| {
                (*object_id == object.id && *vertex_index < object.vertices.len())
                    .then_some(*vertex_index)
            })
            .collect::<Vec<_>>();
        indices.dedup();
        if indices.len() < 3 {
            continue;
        }

        let face_index = object.faces.len();
        let uvs = face_uvs_for_indices(object, &indices);
        object.faces.push(rusterix::GeometryFace {
            indices,
            uvs,
            auto_uv: true,
            tile: None,
            tiles: FxHashMap::default(),
        });
        new_selected_faces.push((object.id, face_index));
        changed = true;
    }

    if changed {
        map.selected_geometry_faces = new_selected_faces;
        map.selected_geometry_vertices.clear();
        sanitize_geometry_selection(map);
    }
    changed
}

pub(crate) fn merge_selected_geometry_faces(map: &mut Map) -> bool {
    if map.selected_geometry_faces.len() < 2 {
        return false;
    }

    let original = map.clone();
    if delete_selected_geometry_faces(map) && fill_selected_geometry_vertices(map) {
        true
    } else {
        *map = original;
        false
    }
}

fn face_normal(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    if face.indices.len() < 3 {
        return None;
    }
    let first = object.transform_point(*object.vertices.get(face.indices[0])?);
    let mut normal = Vec3::<f32>::zero();
    for index in 1..face.indices.len() - 1 {
        let a = object.transform_point(*object.vertices.get(face.indices[index])?) - first;
        let b = object.transform_point(*object.vertices.get(face.indices[index + 1])?) - first;
        normal += a.cross(b);
    }
    normal.try_normalized()
}

fn editing_face_normal(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    face_normal(object, face).map(|normal| -normal)
}

pub(crate) fn extrude_selected_geometry_faces(map: &mut Map, amount: f32) -> bool {
    if map.selected_geometry_faces.is_empty() || !amount.is_finite() || amount.abs() <= 1e-5 {
        return false;
    }

    let selections = map.selected_geometry_faces.clone();
    let mut new_selected_faces = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let face_indices = selections
            .iter()
            .filter_map(|(object_id, face_index)| (*object_id == object.id).then_some(*face_index))
            .collect::<Vec<_>>();
        if face_indices.is_empty() {
            continue;
        }

        let snapshot = object.clone();
        for face_index in face_indices {
            let Some(face) = snapshot.faces.get(face_index) else {
                continue;
            };
            let Some(normal) = editing_face_normal(&snapshot, face) else {
                continue;
            };
            let offset = normal * amount;
            let mut cap_indices = Vec::with_capacity(face.indices.len());
            for old_index in &face.indices {
                let Some(vertex) = snapshot.vertices.get(*old_index) else {
                    cap_indices.clear();
                    break;
                };
                let new_index = object.vertices.len();
                object.vertices.push(*vertex + offset);
                cap_indices.push(new_index);
            }
            if cap_indices.len() != face.indices.len() || cap_indices.len() < 3 {
                continue;
            }

            let cap_face_index = object.faces.len();
            let mut cap_face = face.clone();
            cap_face.indices = cap_indices.clone();
            cap_face.uvs = face_uvs_for_indices(object, &cap_face.indices);
            cap_face.auto_uv = true;
            object.faces.push(cap_face);
            new_selected_faces.push((object.id, cap_face_index));

            for index in 0..face.indices.len() {
                let next = (index + 1) % face.indices.len();
                let side_indices = vec![
                    face.indices[index],
                    face.indices[next],
                    cap_indices[next],
                    cap_indices[index],
                ];
                object.faces.push(rusterix::GeometryFace {
                    uvs: face_uvs_for_indices(object, &side_indices),
                    indices: side_indices,
                    auto_uv: true,
                    tile: face.tile.clone(),
                    tiles: FxHashMap::default(),
                });
            }
            changed = true;
        }
    }

    if changed {
        map.selected_geometry_faces = new_selected_faces;
        map.selected_geometry_vertices.clear();
        sanitize_geometry_selection(map);
    }
    changed
}

pub(crate) fn subdivide_selected_geometry_faces(map: &mut Map) -> bool {
    if map.selected_geometry_faces.is_empty() {
        return false;
    }

    let selections = map.selected_geometry_faces.clone();
    let mut new_selected_faces = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let face_indices = selections
            .iter()
            .filter_map(|(object_id, face_index)| (*object_id == object.id).then_some(*face_index))
            .collect::<Vec<_>>();
        if face_indices.is_empty() {
            continue;
        }

        for face_index in face_indices {
            let Some(face) = object.faces.get(face_index).cloned() else {
                continue;
            };
            if face.indices.len() != 4 {
                continue;
            }
            let [a, b, c, d] = [
                face.indices[0],
                face.indices[1],
                face.indices[2],
                face.indices[3],
            ];
            let (Some(pa), Some(pb), Some(pc), Some(pd)) = (
                object.vertices.get(a).copied(),
                object.vertices.get(b).copied(),
                object.vertices.get(c).copied(),
                object.vertices.get(d).copied(),
            ) else {
                continue;
            };

            let ab = object.vertices.len();
            object.vertices.push((pa + pb) * 0.5);
            let bc = object.vertices.len();
            object.vertices.push((pb + pc) * 0.5);
            let cd = object.vertices.len();
            object.vertices.push((pc + pd) * 0.5);
            let da = object.vertices.len();
            object.vertices.push((pd + pa) * 0.5);
            let center = object.vertices.len();
            object.vertices.push((pa + pb + pc + pd) * 0.25);

            let make_face = |indices: Vec<usize>| {
                let mut new_face = face.clone();
                new_face.indices = indices;
                new_face.uvs = face_uvs_for_indices(object, &new_face.indices);
                new_face.auto_uv = true;
                new_face
            };

            let first_face = make_face(vec![a, ab, center, da]);
            let second_face = make_face(vec![ab, b, bc, center]);
            let third_face = make_face(vec![center, bc, c, cd]);
            let fourth_face = make_face(vec![da, center, cd, d]);

            object.faces[face_index] = first_face;
            new_selected_faces.push((object.id, face_index));
            object.faces.push(second_face);
            object.faces.push(third_face);
            object.faces.push(fourth_face);

            changed = true;
        }
    }

    if changed {
        map.selected_geometry_faces = new_selected_faces;
        map.selected_geometry_vertices.clear();
        sanitize_geometry_selection(map);
    }
    changed
}

pub(crate) fn inset_selected_geometry_faces(map: &mut Map, amount: f32) -> bool {
    if map.selected_geometry_faces.is_empty() {
        return false;
    }

    let selections = map.selected_geometry_faces.clone();
    let mut new_selected_faces = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let face_indices = selections
            .iter()
            .filter_map(|(object_id, face_index)| (*object_id == object.id).then_some(*face_index))
            .collect::<Vec<_>>();
        if face_indices.is_empty() {
            continue;
        }

        for face_index in face_indices {
            let Some(face) = object.faces.get(face_index).cloned() else {
                continue;
            };
            if face.indices.len() < 3 {
                continue;
            }
            let points = face
                .indices
                .iter()
                .filter_map(|index| object.vertices.get(*index).copied())
                .collect::<Vec<_>>();
            if points.len() != face.indices.len() {
                continue;
            }

            let center = points
                .iter()
                .copied()
                .fold(Vec3::zero(), |sum, point| sum + point)
                / points.len() as f32;
            let min_radius = points
                .iter()
                .map(|point| (*point - center).magnitude())
                .fold(f32::INFINITY, f32::min);
            if !min_radius.is_finite() || min_radius <= 1e-4 {
                continue;
            }

            let inset_factor = (amount.abs() / min_radius).clamp(0.05, 0.45);
            let mut inner_indices = Vec::with_capacity(points.len());
            for point in &points {
                let inner_index = object.vertices.len();
                object
                    .vertices
                    .push(center + (*point - center) * (1.0 - inset_factor));
                inner_indices.push(inner_index);
            }

            let mut inner_face = face.clone();
            inner_face.indices = inner_indices.clone();
            inner_face.uvs = face_uvs_for_indices(object, &inner_face.indices);
            inner_face.auto_uv = true;
            object.faces[face_index] = inner_face;
            new_selected_faces.push((object.id, face_index));

            for index in 0..face.indices.len() {
                let next = (index + 1) % face.indices.len();
                let ring_indices = vec![
                    face.indices[index],
                    face.indices[next],
                    inner_indices[next],
                    inner_indices[index],
                ];
                object.faces.push(rusterix::GeometryFace {
                    uvs: face_uvs_for_indices(object, &ring_indices),
                    indices: ring_indices,
                    auto_uv: true,
                    tile: face.tile.clone(),
                    tiles: FxHashMap::default(),
                });
            }
            changed = true;
        }
    }

    if changed {
        map.selected_geometry_faces = new_selected_faces;
        map.selected_geometry_vertices.clear();
        sanitize_geometry_selection(map);
    }
    changed
}
