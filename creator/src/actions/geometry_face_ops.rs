use crate::prelude::*;
use earcutr::earcut;
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

fn ordered_fill_vertices(object: &rusterix::GeometryObject, indices: &[usize]) -> Vec<usize> {
    if indices.len() < 3 {
        return indices.to_vec();
    }

    let points = indices
        .iter()
        .filter_map(|index| object.vertices.get(*index).copied())
        .collect::<Vec<_>>();
    if points.len() != indices.len() {
        return indices.to_vec();
    }

    let center = points
        .iter()
        .copied()
        .fold(Vec3::zero(), |sum, point| sum + point)
        / points.len() as f32;

    let mut normal = Vec3::zero();
    'find_normal: for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                let candidate = (points[j] - points[i]).cross(points[k] - points[i]);
                if candidate.magnitude_squared() > 1e-8 {
                    normal = candidate.normalized();
                    break 'find_normal;
                }
            }
        }
    }
    if normal.magnitude_squared() <= 1e-8 {
        return indices.to_vec();
    }

    let tangent = points
        .iter()
        .map(|point| *point - center)
        .max_by(|a, b| {
            a.magnitude_squared()
                .partial_cmp(&b.magnitude_squared())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .and_then(|axis| axis.try_normalized())
        .unwrap_or(Vec3::unit_x());
    let bitangent = normal
        .cross(tangent)
        .try_normalized()
        .unwrap_or(Vec3::unit_z());

    let mut ordered = indices.to_vec();
    ordered.sort_by(|a, b| {
        let pa = object.vertices[*a] - center;
        let pb = object.vertices[*b] - center;
        let aa = pa.dot(bitangent).atan2(pa.dot(tangent));
        let ab = pb.dot(bitangent).atan2(pb.dot(tangent));
        aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
    });

    let sorted_points = ordered
        .iter()
        .filter_map(|index| object.vertices.get(*index).copied())
        .collect::<Vec<_>>();
    let mut sorted_normal = Vec3::zero();
    for index in 0..sorted_points.len() {
        let current = sorted_points[index];
        let next = sorted_points[(index + 1) % sorted_points.len()];
        sorted_normal.x += (current.y - next.y) * (current.z + next.z);
        sorted_normal.y += (current.z - next.z) * (current.x + next.x);
        sorted_normal.z += (current.x - next.x) * (current.y + next.y);
    }
    if sorted_normal.dot(normal) < 0.0 {
        ordered.reverse();
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

        let indices = ordered_fill_vertices(object, &indices);
        let face_index = object.faces.len();
        let uvs = face_uvs_for_indices(object, &indices);
        object.faces.push(rusterix::GeometryFace {
            indices,
            uvs,
            auto_uv: true,
            texture_offset: Vec2::zero(),
            texture_scale: Vec2::broadcast(1.0),
            texture_rotation: 0.0,
            tile: None,
            tiles: FxHashMap::default(),
            surface_points: Vec::new(),
            surface_segments: Vec::new(),
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

fn local_face_normal(
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
    normal.try_normalized()
}

fn face_normal_from_vertices(
    vertices: &[Vec3<f32>],
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    if face.indices.len() < 3 {
        return None;
    }
    let first = *vertices.get(face.indices[0])?;
    let mut normal = Vec3::<f32>::zero();
    for index in 1..face.indices.len() - 1 {
        let a = *vertices.get(face.indices[index])? - first;
        let b = *vertices.get(face.indices[index + 1])? - first;
        normal += a.cross(b);
    }
    normal.try_normalized()
}

fn face_center(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec3<f32>> {
    if face.indices.is_empty() {
        return None;
    }
    let mut center = Vec3::zero();
    for index in &face.indices {
        center += *object.vertices.get(*index)?;
    }
    Some(center / face.indices.len() as f32)
}

fn quad_grid_point(points: &[Vec3<f32>; 4], x: f32, y: f32) -> Vec3<f32> {
    let bottom = points[0] + (points[1] - points[0]) * x;
    let top = points[3] + (points[2] - points[3]) * x;
    bottom + (top - bottom) * y
}

fn push_geometry_face(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    indices: Vec<usize>,
) -> usize {
    let face_index = object.faces.len();
    object.faces.push(rusterix::GeometryFace {
        uvs: face_uvs_for_indices(object, &indices),
        indices,
        auto_uv: true,
        texture_offset: source.texture_offset,
        texture_scale: source.texture_scale,
        texture_rotation: source.texture_rotation,
        tile: source.tile.clone(),
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
    });
    face_index
}

fn append_opening_ring(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    points: [Vec3<f32>; 4],
    width: f32,
    height: f32,
    reverse_faces: bool,
) -> Option<[usize; 4]> {
    let u_len = (points[1] - points[0]).magnitude();
    let v_len = (points[3] - points[0]).magnitude();
    if u_len <= 1e-4 || v_len <= 1e-4 {
        return None;
    }

    let width_ratio = if width.is_finite() && width > 1e-4 {
        (width / u_len).clamp(0.05, 0.9)
    } else {
        0.5
    };
    let height_ratio = if height.is_finite() && height > 1e-4 {
        (height / v_len).clamp(0.05, 0.9)
    } else {
        0.5
    };
    let x0 = 0.5 - width_ratio * 0.5;
    let x1 = 0.5 + width_ratio * 0.5;
    let y0 = 0.5 - height_ratio * 0.5;
    let y1 = 0.5 + height_ratio * 0.5;
    let xs = [0.0, x0, x1, 1.0];
    let ys = [0.0, y0, y1, 1.0];

    let mut grid = [[0usize; 4]; 4];
    for (y_index, y) in ys.iter().enumerate() {
        for (x_index, x) in xs.iter().enumerate() {
            grid[y_index][x_index] = object.vertices.len();
            object.vertices.push(quad_grid_point(&points, *x, *y));
        }
    }

    for y in 0..3 {
        for x in 0..3 {
            if x == 1 && y == 1 {
                continue;
            }
            let indices = if reverse_faces {
                vec![
                    grid[y][x],
                    grid[y + 1][x],
                    grid[y + 1][x + 1],
                    grid[y][x + 1],
                ]
            } else {
                vec![
                    grid[y][x],
                    grid[y][x + 1],
                    grid[y + 1][x + 1],
                    grid[y + 1][x],
                ]
            };
            push_geometry_face(object, source, indices);
        }
    }

    Some([grid[1][1], grid[1][2], grid[2][2], grid[2][1]])
}

fn push_geometry_face_with_normal(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    mut indices: Vec<usize>,
    desired_normal: Vec3<f32>,
) -> usize {
    if indices.len() >= 3 {
        let first = object.vertices[indices[0]];
        let mut normal = Vec3::<f32>::zero();
        for index in 1..indices.len() - 1 {
            normal += (object.vertices[indices[index]] - first)
                .cross(object.vertices[indices[index + 1]] - first);
        }
        if normal.dot(desired_normal) < 0.0 {
            indices.reverse();
        }
    }
    push_geometry_face(object, source, indices)
}

fn face_on_cut_plane(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
    plane_center: Vec3<f32>,
    plane_normal: Vec3<f32>,
) -> bool {
    const PLANE_EPS: f32 = 0.0001;
    let Some(face_center) = face_center(object, face) else {
        return false;
    };
    if (face_center - plane_center).dot(plane_normal).abs() > PLANE_EPS {
        return false;
    }
    let Some(face_normal) = local_face_normal(object, face) else {
        return false;
    };
    face_normal.dot(plane_normal).abs() > 0.95
}

struct OpeningRing {
    holes: Vec<OpeningHole>,
    face_indices: Vec<usize>,
}

struct OpeningHole {
    loop_indices: Vec<usize>,
    loop_points: Vec<Vec3<f32>>,
    loop_uvs: Vec<Vec2<f32>>,
}

fn aligned_back_hole_indices(
    front_hole: &OpeningHole,
    back_hole: &OpeningHole,
    normal: Vec3<f32>,
    depth: f32,
) -> Option<Vec<usize>> {
    if front_hole.loop_points.len() != back_hole.loop_points.len()
        || back_hole.loop_points.len() != back_hole.loop_indices.len()
    {
        return None;
    }

    let mut used = vec![false; back_hole.loop_points.len()];
    let mut aligned = Vec::with_capacity(front_hole.loop_points.len());
    for front_point in &front_hole.loop_points {
        let target = *front_point + normal * depth;
        let mut best: Option<(usize, f32)> = None;
        for (index, back_point) in back_hole.loop_points.iter().enumerate() {
            if used[index] {
                continue;
            }
            let distance = (*back_point - target).magnitude_squared();
            if best.is_none_or(|(_, best_distance)| distance < best_distance) {
                best = Some((index, distance));
            }
        }
        let (index, distance) = best?;
        if distance > 0.0001 {
            return None;
        }
        used[index] = true;
        aligned.push(back_hole.loop_indices[index]);
    }

    Some(aligned)
}

fn polygon_area_2d(points: &[Vec2<f32>]) -> f32 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

pub(crate) fn surface_segment_points(
    face: &rusterix::GeometryFace,
    segment: &rusterix::GeometrySurfaceSegment,
    normal: Vec3<f32>,
    resolution: usize,
) -> Option<Vec<Vec3<f32>>> {
    let a = face.surface_points.get(segment.start)?.position;
    let b = face.surface_points.get(segment.end)?.position;
    if segment.mode == rusterix::GeometrySurfaceSegmentMode::Line {
        return Some(vec![a, b]);
    }

    let direction = b - a;
    let length = direction.magnitude();
    if length <= 1e-5 {
        return Some(vec![a, b]);
    }
    let dir = direction / length;
    let side = surface_segment_arc_side(face, segment, normal, a, b)
        .or_else(|| normal.cross(dir).try_normalized())
        .unwrap_or(Vec3::zero());
    let control = (a + b) * 0.5 + side * length * segment.curve_amount;
    let steps = resolution.max(4);
    Some(
        (0..=steps)
            .map(|index| {
                let t = index as f32 / steps as f32;
                let omt = 1.0 - t;
                a * (omt * omt) + control * (2.0 * omt * t) + b * (t * t)
            })
            .collect(),
    )
}

fn surface_segment_arc_side(
    face: &rusterix::GeometryFace,
    segment: &rusterix::GeometrySurfaceSegment,
    normal: Vec3<f32>,
    a: Vec3<f32>,
    b: Vec3<f32>,
) -> Option<Vec3<f32>> {
    let mut connected_points = BTreeSet::new();
    let mut pending = vec![segment.start, segment.end];
    while let Some(point_index) = pending.pop() {
        if !connected_points.insert(point_index) {
            continue;
        }
        for other in &face.surface_segments {
            if other.start == point_index && !connected_points.contains(&other.end) {
                pending.push(other.end);
            }
            if other.end == point_index && !connected_points.contains(&other.start) {
                pending.push(other.start);
            }
        }
    }

    if connected_points.len() < 3 {
        return None;
    }

    let mut centroid = Vec3::zero();
    let mut count = 0;
    for point_index in connected_points {
        let point = face.surface_points.get(point_index)?.position;
        centroid += point;
        count += 1;
    }
    if count < 3 {
        return None;
    }

    centroid /= count as f32;
    let mid = (a + b) * 0.5;
    let outward = mid - centroid;
    let planar = outward - normal * outward.dot(normal);
    planar.try_normalized()
}

pub(crate) fn selected_surface_curve_segment_ids(map: &Map) -> BTreeSet<(Uuid, usize, usize)> {
    let mut selected_segments = map
        .selected_geometry_surface_segments
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    let mut selected_points_by_face: BTreeMap<(Uuid, usize), BTreeSet<usize>> = BTreeMap::new();
    for (object_id, face_index, point_index) in &map.selected_geometry_surface_points {
        selected_points_by_face
            .entry((*object_id, *face_index))
            .or_default()
            .insert(*point_index);
    }

    for ((object_id, face_index), selected_points) in selected_points_by_face {
        if selected_points.len() < 2 {
            continue;
        }
        let Some(object) = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)
        else {
            continue;
        };
        let Some(face) = object.faces.get(face_index) else {
            continue;
        };
        let segment_count_before = selected_segments.len();
        for (segment_index, segment) in face.surface_segments.iter().enumerate() {
            if selected_points.contains(&segment.start) && selected_points.contains(&segment.end) {
                selected_segments.insert((object_id, face_index, segment_index));
            }
        }
        if selected_points.len() == 2 && selected_segments.len() == segment_count_before {
            let point_pair = selected_points.iter().copied().collect::<Vec<_>>();
            if let Some(path) = shortest_surface_segment_path(face, point_pair[0], point_pair[1]) {
                for segment_index in path {
                    selected_segments.insert((object_id, face_index, segment_index));
                }
            }
        }
    }

    selected_segments
}

fn shortest_surface_segment_path(
    face: &rusterix::GeometryFace,
    start: usize,
    end: usize,
) -> Option<Vec<usize>> {
    let mut distances: BTreeMap<usize, f32> = BTreeMap::new();
    let mut previous: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut visited = BTreeSet::new();
    distances.insert(start, 0.0);

    loop {
        let Some((&current, &current_distance)) = distances
            .iter()
            .filter(|(point, _)| !visited.contains(*point))
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
        else {
            break;
        };
        if current == end {
            break;
        }
        visited.insert(current);

        for (segment_index, segment) in face.surface_segments.iter().enumerate() {
            let neighbor = if segment.start == current {
                segment.end
            } else if segment.end == current {
                segment.start
            } else {
                continue;
            };
            if visited.contains(&neighbor) {
                continue;
            }
            let a = face.surface_points.get(current)?.position;
            let b = face.surface_points.get(neighbor)?.position;
            let distance = current_distance + (b - a).magnitude();
            if distances
                .get(&neighbor)
                .is_none_or(|existing| distance < *existing)
            {
                distances.insert(neighbor, distance);
                previous.insert(neighbor, (current, segment_index));
            }
        }
    }

    if !distances.contains_key(&end) {
        return None;
    }

    let mut current = end;
    let mut path = Vec::new();
    while current != start {
        let (prev, segment_index) = *previous.get(&current)?;
        path.push(segment_index);
        current = prev;
    }
    path.reverse();
    (!path.is_empty()).then_some(path)
}

pub(crate) fn set_selected_surface_detail_curve(
    map: &mut Map,
    mode: rusterix::GeometrySurfaceSegmentMode,
    curve_amount: f32,
) -> bool {
    let selected_segments = selected_surface_curve_segment_ids(map);
    if selected_segments.is_empty() {
        return false;
    }

    let mut changed = false;
    for (object_id, face_index, segment_index) in selected_segments {
        let Some(object) = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == object_id)
        else {
            continue;
        };
        let Some(face) = object.faces.get_mut(face_index) else {
            continue;
        };
        let Some(segment) = face.surface_segments.get_mut(segment_index) else {
            continue;
        };
        segment.mode = mode;
        segment.curve_amount = curve_amount;
        changed = true;
    }

    if changed {
        map.changed = map.changed.wrapping_add(1);
    }
    changed
}

fn append_selected_cutout_ring(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    outer_points: Vec<Vec3<f32>>,
    outer_indices: Vec<usize>,
    holes: &[Vec<Vec3<f32>>],
    desired_normal: Vec3<f32>,
) -> Option<OpeningRing> {
    if outer_points.len() != 4
        || outer_indices.len() != 4
        || holes.is_empty()
        || holes.iter().any(|hole| hole.len() < 3)
    {
        return None;
    }

    let u = outer_points[1] - outer_points[0];
    let v = outer_points[3] - outer_points[0];
    let u_len_sq = u.magnitude_squared();
    let v_len_sq = v.magnitude_squared();
    if u_len_sq <= 1e-6 || v_len_sq <= 1e-6 {
        return None;
    }
    let to_uv = |point: Vec3<f32>| -> Vec2<f32> {
        let local = point - outer_points[0];
        Vec2::new(local.dot(u) / u_len_sq, local.dot(v) / v_len_sq)
    };

    let outer_uv = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];
    let mut all_uv = outer_uv;
    let mut hole_offsets = Vec::new();
    let mut processed_holes = Vec::new();
    for hole in holes {
        let mut inner_uv = hole.iter().map(|point| to_uv(*point)).collect::<Vec<_>>();
        let mut inner_world = hole.to_vec();
        if polygon_area_2d(&inner_uv) > 0.0 {
            inner_uv.reverse();
            inner_world.reverse();
        }
        hole_offsets.push(all_uv.len());
        all_uv.extend(inner_uv.iter().copied());
        processed_holes.push((inner_uv, inner_world));
    }

    let flat = all_uv
        .iter()
        .flat_map(|point| [point.x as f64, point.y as f64])
        .collect::<Vec<_>>();
    let triangles = earcut(&flat, &hole_offsets, 2).ok()?;
    if triangles.is_empty() {
        return None;
    }

    let mut vertex_indices = outer_indices;
    let mut opening_holes = Vec::new();
    for (inner_uv, inner_world) in processed_holes {
        let mut loop_indices = Vec::with_capacity(inner_world.len());
        for point in &inner_world {
            let vertex_index = object.vertices.len();
            object.vertices.push(*point);
            vertex_indices.push(vertex_index);
            loop_indices.push(vertex_index);
        }
        opening_holes.push(OpeningHole {
            loop_indices,
            loop_points: inner_world,
            loop_uvs: inner_uv,
        });
    }
    let mut face_indices = Vec::new();
    for triangle in triangles.chunks_exact(3) {
        let face_index = push_geometry_face_with_normal(
            object,
            source,
            vec![
                vertex_indices[triangle[0]],
                vertex_indices[triangle[1]],
                vertex_indices[triangle[2]],
            ],
            desired_normal,
        );
        face_indices.push(face_index);
    }

    Some(OpeningRing {
        holes: opening_holes,
        face_indices,
    })
}

fn aligned_face_indices_for_points(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
    points: [Vec3<f32>; 4],
) -> Option<[usize; 4]> {
    let mut used = BTreeSet::new();
    let mut aligned = Vec::with_capacity(4);
    for point in points {
        let mut best: Option<(usize, f32)> = None;
        for vertex_index in &face.indices {
            if used.contains(vertex_index) {
                continue;
            }
            let Some(vertex) = object.vertices.get(*vertex_index) else {
                continue;
            };
            let distance = (*vertex - point).magnitude_squared();
            if best.is_none_or(|(_, best_distance)| distance < best_distance) {
                best = Some((*vertex_index, distance));
            }
        }
        let (vertex_index, distance) = best?;
        if distance > 0.0001 {
            return None;
        }
        used.insert(vertex_index);
        aligned.push(vertex_index);
    }

    Some([aligned[0], aligned[1], aligned[2], aligned[3]])
}

fn aligned_indices_from_candidates_for_points(
    object: &rusterix::GeometryObject,
    candidates: &[usize],
    points: &[Vec3<f32>],
) -> Option<Vec<usize>> {
    if candidates.len() != points.len() {
        return None;
    }

    let mut used = BTreeSet::new();
    let mut aligned = Vec::with_capacity(points.len());
    for point in points {
        let mut best: Option<(usize, f32)> = None;
        for vertex_index in candidates {
            if used.contains(vertex_index) {
                continue;
            }
            let Some(vertex) = object.vertices.get(*vertex_index) else {
                continue;
            };
            let distance = (*vertex - *point).magnitude_squared();
            if best.is_none_or(|(_, best_distance)| distance < best_distance) {
                best = Some((*vertex_index, distance));
            }
        }
        let (vertex_index, distance) = best?;
        if distance > 0.0001 {
            return None;
        }
        used.insert(vertex_index);
        aligned.push(vertex_index);
    }

    Some(aligned)
}

fn coplanar_face_group_any_orientation(
    object: &rusterix::GeometryObject,
    seed_face_index: usize,
    normal: Vec3<f32>,
    plane_center: Vec3<f32>,
) -> BTreeSet<usize> {
    const PLANE_EPS: f32 = 0.0001;
    object
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            let face_normal = local_face_normal(object, face)?;
            if face_normal.dot(normal).abs() < 0.95 {
                return None;
            }
            let face_center = face_center(object, face)?;
            ((face_center - plane_center).dot(normal).abs() <= PLANE_EPS).then_some(face_index)
        })
        .chain(std::iter::once(seed_face_index))
        .collect()
}

fn ordered_boundary_loops_for_faces(
    object: &rusterix::GeometryObject,
    face_indices: &BTreeSet<usize>,
) -> Vec<Vec<usize>> {
    let mut edge_counts = BTreeMap::<(usize, usize), usize>::new();
    for face_index in face_indices {
        let Some(face) = object.faces.get(*face_index) else {
            continue;
        };
        for index in 0..face.indices.len() {
            let edge = normalized_edge(
                face.indices[index],
                face.indices[(index + 1) % face.indices.len()],
            );
            *edge_counts.entry(edge).or_insert(0) += 1;
        }
    }

    let mut pending = edge_counts
        .iter()
        .filter_map(|(edge, count)| (*count == 1).then_some(*edge))
        .collect::<BTreeSet<_>>();
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for (a, b) in &pending {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }

    let mut loops = Vec::new();
    while let Some((start, mut next)) = pending.iter().next().copied() {
        let mut current = start;
        let mut ordered = vec![start];
        loop {
            pending.remove(&normalized_edge(current, next));
            current = next;
            if current == start {
                break;
            }
            ordered.push(current);
            let Some(neighbors) = adjacency.get(&current) else {
                break;
            };
            let Some(next_neighbor) = neighbors
                .iter()
                .copied()
                .find(|neighbor| pending.contains(&normalized_edge(current, *neighbor)))
            else {
                break;
            };
            next = next_neighbor;
            if ordered.len() > adjacency.len() {
                break;
            }
        }
        if ordered.len() >= 3 {
            loops.push(ordered);
        }
    }

    loops
}

fn plane_basis_for_loop(
    object: &rusterix::GeometryObject,
    loop_indices: &[usize],
    normal: Vec3<f32>,
) -> Option<(Vec3<f32>, Vec3<f32>, Vec3<f32>)> {
    let origin = *object.vertices.get(*loop_indices.first()?)?;
    let normal = normal.try_normalized()?;
    let mut u = Vec3::zero();
    for index in 0..loop_indices.len() {
        let a = *object.vertices.get(loop_indices[index])?;
        let b = *object
            .vertices
            .get(loop_indices[(index + 1) % loop_indices.len()])?;
        let edge = b - a;
        if edge.magnitude_squared() > 1e-6 {
            u = edge.try_normalized()?;
            break;
        }
    }
    if u.magnitude_squared() <= 1e-6 {
        return None;
    }
    let v = normal.cross(u).try_normalized()?;
    Some((origin, u, v))
}

fn loop_area_for_indices(
    object: &rusterix::GeometryObject,
    loop_indices: &[usize],
    origin: Vec3<f32>,
    u: Vec3<f32>,
    v: Vec3<f32>,
) -> Option<f32> {
    let points = loop_indices
        .iter()
        .map(|index| {
            let local = *object.vertices.get(*index)? - origin;
            Some(Vec2::new(local.dot(u), local.dot(v)))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(polygon_area_2d(&points))
}

fn simplify_collinear_loop_indices(
    object: &rusterix::GeometryObject,
    loop_indices: Vec<usize>,
) -> Vec<usize> {
    if loop_indices.len() <= 4 {
        return loop_indices;
    }

    let mut simplified = Vec::with_capacity(loop_indices.len());
    for index in 0..loop_indices.len() {
        let prev_index = loop_indices[(index + loop_indices.len() - 1) % loop_indices.len()];
        let curr_index = loop_indices[index];
        let next_index = loop_indices[(index + 1) % loop_indices.len()];
        let (Some(prev), Some(curr), Some(next)) = (
            object.vertices.get(prev_index).copied(),
            object.vertices.get(curr_index).copied(),
            object.vertices.get(next_index).copied(),
        ) else {
            simplified.push(curr_index);
            continue;
        };
        let a = curr - prev;
        let b = next - curr;
        let collinear = a.cross(b).magnitude_squared() <= 1e-8 && a.dot(b) >= 0.0;
        if !collinear {
            simplified.push(curr_index);
        }
    }

    if simplified.len() >= 3 {
        simplified
    } else {
        loop_indices
    }
}

fn split_rect_outer_and_hole_loops(
    object: &rusterix::GeometryObject,
    face_indices: &BTreeSet<usize>,
    normal: Vec3<f32>,
) -> Option<([usize; 4], Vec<Vec<Vec3<f32>>>)> {
    let loops = ordered_boundary_loops_for_faces(object, face_indices);
    if loops.is_empty() {
        return None;
    }
    let (origin, u, v) = plane_basis_for_loop(object, &loops[0], normal)?;
    let mut ranked = loops
        .into_iter()
        .filter_map(|loop_indices| {
            let area = loop_area_for_indices(object, &loop_indices, origin, u, v)?;
            Some((area.abs(), loop_indices))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let outer = simplify_collinear_loop_indices(object, ranked.first()?.1.clone());
    if outer.len() != 4 {
        return None;
    }
    let holes = ranked
        .iter()
        .skip(1)
        .filter_map(|(_, loop_indices)| {
            let points = loop_indices
                .iter()
                .map(|index| object.vertices.get(*index).copied())
                .collect::<Option<Vec<_>>>()?;
            (points.len() >= 3).then_some(points)
        })
        .collect::<Vec<_>>();
    Some(([outer[0], outer[1], outer[2], outer[3]], holes))
}

fn project_loops_to_plane(
    loops: impl IntoIterator<Item = Vec<Vec3<f32>>>,
    plane_center: Vec3<f32>,
    plane_normal: Vec3<f32>,
) -> Vec<Vec<Vec3<f32>>> {
    let Some(normal) = plane_normal.try_normalized() else {
        return Vec::new();
    };
    loops
        .into_iter()
        .filter_map(|loop_points| {
            let projected = loop_points
                .into_iter()
                .map(|point| point - normal * (point - plane_center).dot(normal))
                .collect::<Vec<_>>();
            (projected.len() >= 3).then_some(projected)
        })
        .collect()
}

fn loop_fits_within_quad(loop_points: &[Vec3<f32>], face_points: [Vec3<f32>; 4]) -> bool {
    if loop_points.is_empty() {
        return false;
    }
    let u = face_points[1] - face_points[0];
    let v = face_points[3] - face_points[0];
    let u_len_sq = u.magnitude_squared();
    let v_len_sq = v.magnitude_squared();
    if u_len_sq <= 1e-6 || v_len_sq <= 1e-6 {
        return false;
    }
    const EPS: f32 = 0.001;
    loop_points.iter().all(|point| {
        let local = *point - face_points[0];
        let uv = Vec2::new(local.dot(u) / u_len_sq, local.dot(v) / v_len_sq);
        uv.x >= -EPS && uv.x <= 1.0 + EPS && uv.y >= -EPS && uv.y <= 1.0 + EPS
    })
}

fn any_loop_extends_beyond_quad(loops: &[Vec<Vec3<f32>>], face_points: [Vec3<f32>; 4]) -> bool {
    loops
        .iter()
        .any(|loop_points| !loop_fits_within_quad(loop_points, face_points))
}

fn hole_boundary_edges_for_faces(
    object: &rusterix::GeometryObject,
    face_indices: &BTreeSet<usize>,
    normal: Vec3<f32>,
) -> BTreeSet<(usize, usize)> {
    let loops = ordered_boundary_loops_for_faces(object, face_indices);
    if loops.len() <= 1 {
        return BTreeSet::new();
    }
    let Some((origin, u, v)) = plane_basis_for_loop(object, &loops[0], normal) else {
        return BTreeSet::new();
    };
    let mut ranked = loops
        .into_iter()
        .filter_map(|loop_indices| {
            let area = loop_area_for_indices(object, &loop_indices, origin, u, v)?;
            Some((area.abs(), loop_indices))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut edges = BTreeSet::new();
    for (_, loop_indices) in ranked.iter().skip(1) {
        for index in 0..loop_indices.len() {
            edges.insert(normalized_edge(
                loop_indices[index],
                loop_indices[(index + 1) % loop_indices.len()],
            ));
        }
    }
    edges
}

fn face_contains_any_edge(face: &rusterix::GeometryFace, edges: &BTreeSet<(usize, usize)>) -> bool {
    if edges.is_empty() {
        return false;
    }
    for index in 0..face.indices.len() {
        if edges.contains(&normalized_edge(
            face.indices[index],
            face.indices[(index + 1) % face.indices.len()],
        )) {
            return true;
        }
    }
    false
}

fn append_cutout_reveal_faces(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    front_ring: &OpeningRing,
    aligned_back_holes: &[Vec<usize>],
    normal: Vec3<f32>,
    depth: f32,
    new_selected_faces: &mut Vec<(Uuid, usize)>,
) {
    for (front_hole, back_indices) in front_ring.holes.iter().zip(aligned_back_holes.iter()) {
        for index in 0..front_hole.loop_indices.len() {
            let next = (index + 1) % front_hole.loop_indices.len();
            if edge_on_same_outer_boundary(front_hole.loop_uvs[index], front_hole.loop_uvs[next]) {
                continue;
            }
            let reveal_face_index = push_geometry_face_with_normal(
                object,
                source,
                vec![
                    front_hole.loop_indices[index],
                    front_hole.loop_indices[next],
                    back_indices[next],
                    back_indices[index],
                ],
                (front_hole.loop_points[next] - front_hole.loop_points[index])
                    .cross(normal * depth)
                    .try_normalized()
                    .unwrap_or(normal),
            );
            new_selected_faces.push((object.id, reveal_face_index));
        }
    }
}

fn point_near_segment_2d(point: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> bool {
    const EPS_SQ: f32 = 0.0004;
    let ab = b - a;
    let len_sq = ab.magnitude_squared();
    if len_sq <= 1e-8 {
        return (point - a).magnitude_squared() <= EPS_SQ;
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (point - closest).magnitude_squared() <= EPS_SQ
}

fn point_strictly_in_polygon_2d(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    for index in 0..polygon.len() {
        if point_near_segment_2d(point, polygon[index], polygon[(index + 1) % polygon.len()]) {
            return false;
        }
    }

    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let a = polygon[current];
        let b = polygon[previous];
        if (a.y > point.y) != (b.y > point.y) {
            let denom = b.y - a.y;
            if denom.abs() <= 1e-8 {
                previous = current;
                continue;
            }
            let x = (b.x - a.x) * (point.y - a.y) / denom + a.x;
            if point.x < x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn segment_intersects_segment_2d(
    a0: Vec2<f32>,
    a1: Vec2<f32>,
    b0: Vec2<f32>,
    b1: Vec2<f32>,
) -> bool {
    const EPS: f32 = 1e-5;
    let cross = |a: Vec2<f32>, b: Vec2<f32>| a.x * b.y - a.y * b.x;
    let orient = |a: Vec2<f32>, b: Vec2<f32>, c: Vec2<f32>| cross(b - a, c - a);

    let o1 = orient(a0, a1, b0);
    let o2 = orient(a0, a1, b1);
    let o3 = orient(b0, b1, a0);
    let o4 = orient(b0, b1, a1);

    (o1 > EPS && o2 < -EPS || o1 < -EPS && o2 > EPS)
        && (o3 > EPS && o4 < -EPS || o3 < -EPS && o4 > EPS)
}

fn polygons_overlap_2d(face: &[Vec2<f32>], hole: &[Vec2<f32>]) -> bool {
    if face.len() < 3 || hole.len() < 3 {
        return false;
    }
    let face_center = face
        .iter()
        .copied()
        .fold(Vec2::zero(), |sum, point| sum + point)
        / face.len() as f32;
    if point_strictly_in_polygon_2d(face_center, hole) {
        return true;
    }
    if face
        .iter()
        .any(|point| point_strictly_in_polygon_2d(*point, hole))
    {
        return true;
    }
    if hole
        .iter()
        .any(|point| point_strictly_in_polygon_2d(*point, face))
    {
        return true;
    }
    for face_index in 0..face.len() {
        let face_a = face[face_index];
        let face_b = face[(face_index + 1) % face.len()];
        for hole_index in 0..hole.len() {
            if segment_intersects_segment_2d(
                face_a,
                face_b,
                hole[hole_index],
                hole[(hole_index + 1) % hole.len()],
            ) {
                return true;
            }
        }
    }
    false
}

fn prune_cutout_internal_caps(
    object: &mut rusterix::GeometryObject,
    outer_points: &[Vec3<f32>],
    front_ring: &OpeningRing,
) {
    if outer_points.len() != 4 || front_ring.holes.is_empty() {
        return;
    }
    let u = outer_points[1] - outer_points[0];
    let v = outer_points[3] - outer_points[0];
    let u_len_sq = u.magnitude_squared();
    let v_len_sq = v.magnitude_squared();
    if u_len_sq <= 1e-6 || v_len_sq <= 1e-6 {
        return;
    }
    let cut_normal = u.cross(v).try_normalized().unwrap_or(Vec3::unit_y());
    let to_uv = |point: Vec3<f32>| -> Vec2<f32> {
        let local = point - outer_points[0];
        Vec2::new(local.dot(u) / u_len_sq, local.dot(v) / v_len_sq)
    };

    let hole_uvs = front_ring
        .holes
        .iter()
        .map(|hole| {
            hole.loop_points
                .iter()
                .map(|point| to_uv(*point))
                .collect::<Vec<_>>()
        })
        .filter(|uvs| uvs.len() >= 3 && polygon_area_2d(uvs).abs() > 1e-5)
        .collect::<Vec<_>>();
    if hole_uvs.is_empty() {
        return;
    }

    let vertices = object.vertices.clone();
    object.faces.retain(|face| {
        if face.indices.is_empty() {
            return true;
        }
        let Some(face_normal) = face_normal_from_vertices(&vertices, face) else {
            return true;
        };
        if face_normal.dot(cut_normal).abs() < 0.95 {
            return true;
        }
        let mut center = Vec3::zero();
        let mut face_uvs = Vec::with_capacity(face.indices.len());
        for index in &face.indices {
            let Some(vertex) = vertices.get(*index) else {
                return true;
            };
            center += *vertex;
            face_uvs.push(to_uv(*vertex));
        }
        let center = center / face.indices.len() as f32;
        if !center.x.is_finite() || !center.y.is_finite() || !center.z.is_finite() {
            return true;
        }
        let uv = to_uv(center);
        !hole_uvs.iter().any(|hole| {
            point_strictly_in_polygon_2d(uv, hole) || polygons_overlap_2d(&face_uvs, hole)
        })
    });
}

fn reattach_cutout_surface_guides(
    object: &mut rusterix::GeometryObject,
    front_ring: &OpeningRing,
) -> Option<(
    Vec<(Uuid, usize)>,
    Vec<(Uuid, usize, usize)>,
    Vec<(Uuid, usize, usize)>,
)> {
    let Some(guide_face_index) = front_ring.face_indices.first().copied() else {
        return None;
    };

    let mut retained_points = Vec::new();
    let mut retained_segments = Vec::new();
    for hole in &front_ring.holes {
        if let Some((selected_points, selected_segments)) =
            reattach_surface_loop_to_face(object, guide_face_index, &hole.loop_points, true)
        {
            retained_points.extend(selected_points);
            retained_segments.extend(selected_segments);
        }
    }
    (!retained_points.is_empty() || !retained_segments.is_empty()).then_some((
        vec![(object.id, guide_face_index)],
        retained_points,
        retained_segments,
    ))
}

fn cutout_existing_surface_ring(
    object: &mut rusterix::GeometryObject,
    snapshot: &rusterix::GeometryObject,
    face_index: usize,
    face: &rusterix::GeometryFace,
    inner_loops: Vec<Vec<Vec3<f32>>>,
    new_selected_faces: &mut Vec<(Uuid, usize)>,
) -> Option<(
    Vec<(Uuid, usize)>,
    Vec<(Uuid, usize, usize)>,
    Vec<(Uuid, usize, usize)>,
)> {
    let Some(normal) = local_face_normal(snapshot, face) else {
        return None;
    };
    let Some(center) = face_center(snapshot, face) else {
        return None;
    };

    let source_group = coplanar_face_group_any_orientation(snapshot, face_index, normal, center);
    let Some((outer_indices, mut existing_holes)) =
        split_rect_outer_and_hole_loops(snapshot, &source_group, normal)
    else {
        return None;
    };
    let Some(outer_points) = outer_indices
        .iter()
        .map(|index| snapshot.vertices.get(*index).copied())
        .collect::<Option<Vec<_>>>()
    else {
        return None;
    };
    existing_holes.extend(inner_loops);
    existing_holes = project_loops_to_plane(existing_holes, center, normal);
    if existing_holes.is_empty() {
        return None;
    }

    let mut opposite: Option<(usize, f32)> = None;
    for (candidate_index, candidate) in snapshot.faces.iter().enumerate() {
        if source_group.contains(&candidate_index) {
            continue;
        }
        let Some(candidate_normal) = local_face_normal(snapshot, candidate) else {
            continue;
        };
        if normal.dot(candidate_normal) > -0.95 {
            continue;
        }
        let Some(candidate_center) = face_center(snapshot, candidate) else {
            continue;
        };
        let distance = (candidate_center - center).dot(normal).abs();
        if distance <= 1e-4 {
            continue;
        }
        if opposite
            .as_ref()
            .is_none_or(|(_, best_distance)| distance > *best_distance)
        {
            opposite = Some((candidate_index, distance));
        }
    }
    if opposite.is_none() {
        for (candidate_index, candidate) in snapshot.faces.iter().enumerate() {
            if source_group.contains(&candidate_index) {
                continue;
            }
            let Some(candidate_normal) = local_face_normal(snapshot, candidate) else {
                continue;
            };
            if normal.dot(candidate_normal).abs() < 0.95 {
                continue;
            }
            let Some(candidate_center) = face_center(snapshot, candidate) else {
                continue;
            };
            let distance = (candidate_center - center).dot(normal).abs();
            if distance <= 1e-4 {
                continue;
            }
            if opposite
                .as_ref()
                .is_none_or(|(_, best_distance)| distance > *best_distance)
            {
                opposite = Some((candidate_index, distance));
            }
        }
    }
    let Some((opposite_index, _)) = opposite else {
        return None;
    };
    let Some(opposite_face) = snapshot.faces.get(opposite_index).cloned() else {
        return None;
    };
    let Some(opposite_center) = face_center(snapshot, &opposite_face) else {
        return None;
    };
    let depth = (opposite_center - center).dot(normal);
    if depth.abs() <= 1e-4 {
        return None;
    }

    let opposite_group =
        coplanar_face_group_any_orientation(snapshot, opposite_index, normal, opposite_center);
    let Some((back_outer_raw, _)) =
        split_rect_outer_and_hole_loops(snapshot, &opposite_group, -normal)
    else {
        return None;
    };
    let back_points = outer_points
        .iter()
        .map(|point| *point + normal * depth)
        .collect::<Vec<_>>();
    let Some(back_outer_indices) =
        aligned_indices_from_candidates_for_points(snapshot, &back_outer_raw, &back_points)
    else {
        return None;
    };

    let existing_hole_edges = hole_boundary_edges_for_faces(snapshot, &source_group, normal);
    let mut retained_faces = Vec::with_capacity(object.faces.len());
    for (index, existing) in object.faces.iter().cloned().enumerate() {
        let remove = source_group.contains(&index)
            || opposite_group.contains(&index)
            || face_contains_any_edge(&existing, &existing_hole_edges);
        if !remove {
            retained_faces.push(existing);
        }
    }
    object.faces = retained_faces;

    let front_outer_points = outer_points.clone();
    let Some(front_ring) = append_selected_cutout_ring(
        object,
        face,
        outer_points,
        outer_indices.to_vec(),
        &existing_holes,
        normal,
    ) else {
        *object = snapshot.clone();
        return None;
    };
    let back_inner = front_ring
        .holes
        .iter()
        .map(|hole| {
            hole.loop_points
                .iter()
                .map(|point| *point + normal * depth)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let Some(back_ring) = append_selected_cutout_ring(
        object,
        &opposite_face,
        back_points,
        back_outer_indices,
        &back_inner,
        -normal,
    ) else {
        *object = snapshot.clone();
        return None;
    };

    let mut aligned_back_holes = Vec::with_capacity(front_ring.holes.len());
    for (front_hole, back_hole) in front_ring.holes.iter().zip(back_ring.holes.iter()) {
        let Some(back_indices) = aligned_back_hole_indices(front_hole, back_hole, normal, depth)
        else {
            *object = snapshot.clone();
            return None;
        };
        aligned_back_holes.push(back_indices);
    }

    append_cutout_reveal_faces(
        object,
        face,
        &front_ring,
        &aligned_back_holes,
        normal,
        depth,
        new_selected_faces,
    );
    prune_cutout_internal_caps(object, &front_outer_points, &front_ring);
    reattach_cutout_surface_guides(object, &front_ring)
}

fn reattach_surface_loop_to_face(
    object: &mut rusterix::GeometryObject,
    face_index: usize,
    loop_points: &[Vec3<f32>],
    loop_closed: bool,
) -> Option<(Vec<(Uuid, usize, usize)>, Vec<(Uuid, usize, usize)>)> {
    let object_id = object.id;
    let face = object.faces.get_mut(face_index)?;
    let point_start = face.surface_points.len();
    for point in loop_points {
        face.surface_points.push(rusterix::GeometrySurfacePoint {
            position: *point,
            mode: rusterix::GeometrySurfacePointMode::Corner,
        });
    }
    let point_count = loop_points.len();
    if point_count == 0 {
        return None;
    }

    let mut selected_points = Vec::new();
    for point_index in point_start..point_start + point_count {
        selected_points.push((object_id, face_index, point_index));
    }

    let mut selected_segments = Vec::new();
    let segment_count = if loop_closed {
        point_count
    } else {
        point_count.saturating_sub(1)
    };
    for offset in 0..segment_count {
        let start = point_start + offset;
        let end = point_start + ((offset + 1) % point_count);
        face.surface_segments
            .push(rusterix::GeometrySurfaceSegment {
                start,
                end,
                mode: rusterix::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.35,
            });
        selected_segments.push((
            object_id,
            face_index,
            face.surface_segments.len().saturating_sub(1),
        ));
    }

    Some((selected_points, selected_segments))
}

pub(crate) fn duplicate_selected_surface_detail(
    map: &mut Map,
    offset_u: f32,
    offset_v: f32,
) -> bool {
    if map.selected_geometry_surface_segments.is_empty()
        && map.selected_geometry_surface_points.is_empty()
    {
        return false;
    }

    let selected_segments = map.selected_geometry_surface_segments.clone();
    let selected_points = map.selected_geometry_surface_points.clone();
    let mut new_selected_points = Vec::new();
    let mut new_selected_segments = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let snapshot = object.clone();
        for (face_index, face) in snapshot.faces.iter().enumerate() {
            let segment_indices = selected_surface_segments_for_face(
                &selected_segments,
                &selected_points,
                snapshot.id,
                face_index,
                face,
            );
            let point_indices = selected_surface_point_indices_for_face(
                &selected_points,
                &selected_segments,
                snapshot.id,
                face_index,
                face,
            );
            if point_indices.is_empty() {
                continue;
            }
            let Some(normal) = local_face_normal(&snapshot, face) else {
                continue;
            };
            let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
            let (u, v) = if abs.y >= abs.x && abs.y >= abs.z {
                (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0))
            } else if abs.x >= abs.z {
                (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0))
            } else {
                (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
            };
            let offset = u * offset_u + v * offset_v;

            let Some(target_face) = object.faces.get_mut(face_index) else {
                continue;
            };
            let mut remap = BTreeMap::new();
            for point_index in point_indices {
                let Some(point) = face.surface_points.get(point_index) else {
                    continue;
                };
                let new_index = target_face.surface_points.len();
                target_face
                    .surface_points
                    .push(rusterix::GeometrySurfacePoint {
                        position: point.position + offset,
                        mode: point.mode,
                    });
                remap.insert(point_index, new_index);
                new_selected_points.push((object.id, face_index, new_index));
                changed = true;
            }

            for segment_index in segment_indices {
                let Some(segment) = face.surface_segments.get(segment_index) else {
                    continue;
                };
                let (Some(start), Some(end)) = (remap.get(&segment.start), remap.get(&segment.end))
                else {
                    continue;
                };
                target_face
                    .surface_segments
                    .push(rusterix::GeometrySurfaceSegment {
                        start: *start,
                        end: *end,
                        mode: segment.mode,
                        curve_amount: segment.curve_amount,
                    });
                new_selected_segments.push((
                    object.id,
                    face_index,
                    target_face.surface_segments.len().saturating_sub(1),
                ));
            }
        }
    }

    if changed {
        map.selected_geometry_surface_points = new_selected_points;
        map.selected_geometry_surface_segments = new_selected_segments;
        map.changed = map.changed.wrapping_add(1);
        sanitize_geometry_selection(map);
    }
    changed
}

fn edge_on_same_outer_boundary(a: Vec2<f32>, b: Vec2<f32>) -> bool {
    const EPS: f32 = 0.08;
    (a.x <= EPS && b.x <= EPS)
        || (a.x >= 1.0 - EPS && b.x >= 1.0 - EPS)
        || (a.y <= EPS && b.y <= EPS)
        || (a.y >= 1.0 - EPS && b.y >= 1.0 - EPS)
}

fn snap_cutout_loop_to_boundary(
    points: &mut [Vec3<f32>],
    face_points: [Vec3<f32>; 4],
) -> Option<Vec<Vec2<f32>>> {
    const SNAP_EPS: f32 = 0.08;
    let face_u = face_points[1] - face_points[0];
    let face_v = face_points[3] - face_points[0];
    let face_u_len_sq = face_u.magnitude_squared();
    let face_v_len_sq = face_v.magnitude_squared();
    if face_u_len_sq <= 1e-6 || face_v_len_sq <= 1e-6 {
        return None;
    }

    let to_uv = |point: Vec3<f32>| -> Vec2<f32> {
        let local = point - face_points[0];
        Vec2::new(
            local.dot(face_u) / face_u_len_sq,
            local.dot(face_v) / face_v_len_sq,
        )
    };
    let from_uv = |uv: Vec2<f32>| -> Vec3<f32> { face_points[0] + face_u * uv.x + face_v * uv.y };

    let mut uvs = points.iter().map(|point| to_uv(*point)).collect::<Vec<_>>();
    let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for uv in &uvs {
        min.x = min.x.min(uv.x);
        min.y = min.y.min(uv.y);
        max.x = max.x.max(uv.x);
        max.y = max.y.max(uv.y);
    }

    let snap_min_x = min.x.abs() <= SNAP_EPS;
    let snap_max_x = (1.0 - max.x).abs() <= SNAP_EPS;
    let snap_min_y = min.y.abs() <= SNAP_EPS;
    let snap_max_y = (1.0 - max.y).abs() <= SNAP_EPS;

    for (index, uv) in uvs.iter_mut().enumerate() {
        if snap_min_x && uv.x <= SNAP_EPS {
            uv.x = 0.0;
        }
        if snap_max_x && uv.x >= 1.0 - SNAP_EPS {
            uv.x = 1.0;
        }
        if snap_min_y && uv.y <= SNAP_EPS {
            uv.y = 0.0;
        }
        if snap_max_y && uv.y >= 1.0 - SNAP_EPS {
            uv.y = 1.0;
        }
        uv.x = uv.x.clamp(0.0, 1.0);
        uv.y = uv.y.clamp(0.0, 1.0);
        points[index] = from_uv(*uv);
    }

    Some(uvs)
}

fn selected_surface_segments_for_face(
    selected_segments: &[(Uuid, usize, usize)],
    selected_points: &[(Uuid, usize, usize)],
    object_id: Uuid,
    face_index: usize,
    face: &rusterix::GeometryFace,
) -> BTreeSet<usize> {
    let mut segments = selected_segments
        .iter()
        .filter_map(|(selected_object_id, selected_face_index, segment_index)| {
            (*selected_object_id == object_id && *selected_face_index == face_index)
                .then_some(*segment_index)
        })
        .collect::<BTreeSet<_>>();

    let point_selection = selected_points
        .iter()
        .filter_map(|(selected_object_id, selected_face_index, point_index)| {
            (*selected_object_id == object_id && *selected_face_index == face_index)
                .then_some(*point_index)
        })
        .collect::<BTreeSet<_>>();
    if !point_selection.is_empty() {
        for (segment_index, segment) in face.surface_segments.iter().enumerate() {
            if point_selection.contains(&segment.start) || point_selection.contains(&segment.end) {
                segments.insert(segment_index);
            }
        }
    }

    segments
}

fn selected_surface_point_indices_for_face(
    selected_points: &[(Uuid, usize, usize)],
    selected_segments: &[(Uuid, usize, usize)],
    object_id: Uuid,
    face_index: usize,
    face: &rusterix::GeometryFace,
) -> BTreeSet<usize> {
    let mut points = selected_points
        .iter()
        .filter_map(|(selected_object_id, selected_face_index, point_index)| {
            (*selected_object_id == object_id && *selected_face_index == face_index)
                .then_some(*point_index)
        })
        .collect::<BTreeSet<_>>();
    for (selected_object_id, selected_face_index, segment_index) in selected_segments {
        if *selected_object_id != object_id || *selected_face_index != face_index {
            continue;
        }
        if let Some(segment) = face.surface_segments.get(*segment_index) {
            points.insert(segment.start);
            points.insert(segment.end);
        }
    }
    points
}

fn ordered_surface_loop_points(
    face: &rusterix::GeometryFace,
    segment_indices: &BTreeSet<usize>,
) -> Option<Vec<Vec3<f32>>> {
    if segment_indices.len() < 3 {
        return None;
    }

    let mut nodes: Vec<Vec3<f32>> = Vec::new();
    let mut edges: Vec<(usize, usize, usize)> = Vec::new();
    let node_for = |nodes: &mut Vec<Vec3<f32>>, point: Vec3<f32>| -> usize {
        const EPS_SQ: f32 = 0.0001;
        if let Some(index) = nodes
            .iter()
            .position(|existing| (*existing - point).magnitude_squared() <= EPS_SQ)
        {
            index
        } else {
            nodes.push(point);
            nodes.len() - 1
        }
    };

    for segment_index in segment_indices {
        let segment = face.surface_segments.get(*segment_index)?;
        if segment.start == segment.end {
            continue;
        }
        let start = face.surface_points.get(segment.start)?.position;
        let end = face.surface_points.get(segment.end)?.position;
        let start_index = node_for(&mut nodes, start);
        let end_index = node_for(&mut nodes, end);
        if start_index != end_index {
            edges.push((start_index, end_index, *segment_index));
        }
    }

    let mut adjacency: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for (a, b, segment_index) in &edges {
        adjacency.entry(*a).or_default().push((*b, *segment_index));
        adjacency.entry(*b).or_default().push((*a, *segment_index));
    }
    if adjacency.len() < 3 || adjacency.values().any(|neighbors| neighbors.len() != 2) {
        return None;
    }

    let start = *adjacency.keys().next()?;
    let mut ordered_nodes = vec![start];
    let mut ordered_segments = Vec::new();
    let mut previous = start;
    let (mut current, first_segment) = adjacency.get(&start)?.first().copied()?;
    ordered_segments.push(first_segment);
    while current != start {
        if ordered_nodes.contains(&current) {
            return None;
        }
        ordered_nodes.push(current);
        let neighbors = adjacency.get(&current)?;
        let (next, segment_index) = neighbors
            .iter()
            .copied()
            .find(|(neighbor, _)| *neighbor != previous)?;
        ordered_segments.push(segment_index);
        previous = current;
        current = next;
        if ordered_nodes.len() > adjacency.len() {
            return None;
        }
    }
    if ordered_nodes.len() != adjacency.len() || ordered_segments.len() != ordered_nodes.len() {
        return None;
    }

    let mut normal = Vec3::<f32>::zero();
    for index in 0..ordered_nodes.len() {
        let a = *nodes.get(ordered_nodes[index])?;
        let b = *nodes.get(ordered_nodes[(index + 1) % ordered_nodes.len()])?;
        normal += a.cross(b);
    }
    let normal = normal.try_normalized()?;

    let mut points = Vec::new();
    for (edge_index, segment_index) in ordered_segments.iter().enumerate() {
        let a_node = ordered_nodes[edge_index];
        let b_node = ordered_nodes[(edge_index + 1) % ordered_nodes.len()];
        let segment = face.surface_segments.get(*segment_index)?;
        let mut segment_points = surface_segment_points(face, segment, normal, 8)?;
        if face
            .surface_points
            .get(segment.start)
            .is_some_and(|start| (start.position - nodes[a_node]).magnitude_squared() > 0.0001)
        {
            segment_points.reverse();
        }
        if !points.is_empty() {
            segment_points.remove(0);
        }
        if nodes.get(b_node).is_some_and(|node| {
            (segment_points.last().copied().unwrap_or(*node) - *node).magnitude_squared() > 0.01
        }) {
            return None;
        }
        points.extend(segment_points);
    }
    points.dedup_by(|a, b| (*a - *b).magnitude_squared() <= 0.0001);
    if points.len() < 3 {
        return None;
    }
    Some(points)
}

fn ordered_surface_loop_components(
    face: &rusterix::GeometryFace,
    segment_indices: &BTreeSet<usize>,
) -> Vec<Vec<Vec3<f32>>> {
    let mut pending = segment_indices.clone();
    let mut loops = Vec::new();

    while let Some(start_segment) = pending.iter().next().copied() {
        let mut component = BTreeSet::new();
        let mut queue = vec![start_segment];
        pending.remove(&start_segment);

        while let Some(segment_index) = queue.pop() {
            if !component.insert(segment_index) {
                continue;
            }
            let Some(segment) = face.surface_segments.get(segment_index) else {
                continue;
            };
            let endpoints = [segment.start, segment.end];
            let connected = pending
                .iter()
                .copied()
                .filter(|candidate_index| {
                    face.surface_segments
                        .get(*candidate_index)
                        .is_some_and(|candidate| {
                            endpoints.contains(&candidate.start)
                                || endpoints.contains(&candidate.end)
                        })
                })
                .collect::<Vec<_>>();
            for connected_index in connected {
                pending.remove(&connected_index);
                queue.push(connected_index);
            }
        }

        if let Some(points) = ordered_surface_loop_points(face, &component) {
            loops.push(points);
        }
    }

    loops
}

pub(crate) enum CutoutLoopValidation {
    Valid { loops: usize },
    Empty,
    MultipleFaces,
    OpenLoop,
}

fn selected_surface_segment_components(
    face: &rusterix::GeometryFace,
    segment_indices: &BTreeSet<usize>,
) -> Vec<BTreeSet<usize>> {
    let mut pending = segment_indices.clone();
    let mut components = Vec::new();

    while let Some(start_segment) = pending.iter().next().copied() {
        let mut component = BTreeSet::new();
        let mut queue = vec![start_segment];
        pending.remove(&start_segment);

        while let Some(segment_index) = queue.pop() {
            if !component.insert(segment_index) {
                continue;
            }
            let Some(segment) = face.surface_segments.get(segment_index) else {
                continue;
            };
            let endpoints = [segment.start, segment.end];
            let connected = pending
                .iter()
                .copied()
                .filter(|candidate_index| {
                    face.surface_segments
                        .get(*candidate_index)
                        .is_some_and(|candidate| {
                            endpoints.contains(&candidate.start)
                                || endpoints.contains(&candidate.end)
                        })
                })
                .collect::<Vec<_>>();
            for connected_index in connected {
                pending.remove(&connected_index);
                queue.push(connected_index);
            }
        }

        if !component.is_empty() {
            components.push(component);
        }
    }

    components
}

pub(crate) fn validate_selected_cutout_loops(map: &Map) -> CutoutLoopValidation {
    if map.selected_geometry_surface_segments.is_empty()
        && map.selected_geometry_surface_points.is_empty()
    {
        return CutoutLoopValidation::Empty;
    }

    let mut hosts = BTreeSet::new();
    let mut loops = 0usize;
    for object in &map.geometry_objects {
        for (face_index, face) in object.faces.iter().enumerate() {
            let segment_indices = selected_surface_segments_for_face(
                &map.selected_geometry_surface_segments,
                &map.selected_geometry_surface_points,
                object.id,
                face_index,
                face,
            );
            if segment_indices.is_empty() {
                continue;
            }
            hosts.insert((object.id, face_index));
            if hosts.len() > 1 {
                return CutoutLoopValidation::MultipleFaces;
            }

            for component in selected_surface_segment_components(face, &segment_indices) {
                if ordered_surface_loop_points(face, &component).is_some() {
                    loops += 1;
                } else {
                    return CutoutLoopValidation::OpenLoop;
                }
            }
        }
    }

    if loops == 0 {
        CutoutLoopValidation::Empty
    } else {
        CutoutLoopValidation::Valid { loops }
    }
}

fn ordered_surface_points_by_angle(
    face: &rusterix::GeometryFace,
    point_indices: &BTreeSet<usize>,
    face_points: [Vec3<f32>; 4],
) -> Option<Vec<Vec3<f32>>> {
    if point_indices.len() < 3 {
        return None;
    }
    let u = (face_points[1] - face_points[0]).try_normalized()?;
    let normal = (face_points[1] - face_points[0])
        .cross(face_points[3] - face_points[0])
        .try_normalized()?;
    let v = normal.cross(u).try_normalized()?;

    let mut projected = Vec::new();
    let mut center = Vec2::zero();
    for point_index in point_indices {
        let point = face.surface_points.get(*point_index)?.position;
        let local = point - face_points[0];
        let uv = Vec2::new(local.dot(u), local.dot(v));
        center += uv;
        projected.push((*point_index, point, uv));
    }
    center /= projected.len() as f32;
    projected.sort_by(|a, b| {
        let aa = (a.2.y - center.y).atan2(a.2.x - center.x);
        let bb = (b.2.y - center.y).atan2(b.2.x - center.x);
        aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let points = projected
        .iter()
        .map(|(_, point, _)| *point)
        .collect::<Vec<_>>();
    (points.len() >= 3).then_some(points)
}

pub(crate) fn cutout_selected_surface_loop(map: &mut Map) -> bool {
    if map.selected_geometry_surface_segments.is_empty()
        && map.selected_geometry_surface_points.is_empty()
    {
        return false;
    }

    let selected_segments = map.selected_geometry_surface_segments.clone();
    let selected_points = map.selected_geometry_surface_points.clone();
    let mut new_selected_faces = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let snapshot = object.clone();
        for (face_index, face) in snapshot.faces.iter().enumerate() {
            let segment_indices = selected_surface_segments_for_face(
                &selected_segments,
                &selected_points,
                snapshot.id,
                face_index,
                face,
            );
            if segment_indices.is_empty() {
                continue;
            }

            let loop_segments = selected_segments
                .iter()
                .filter_map(|(selected_object_id, selected_face_index, segment_index)| {
                    (*selected_object_id == snapshot.id && *selected_face_index == face_index)
                        .then_some(*segment_index)
                })
                .collect::<BTreeSet<_>>();
            let mut inner_loops = ordered_surface_loop_components(face, &loop_segments);
            if inner_loops.is_empty() {
                inner_loops = ordered_surface_loop_components(face, &segment_indices);
            }
            if face.indices.len() != 4 {
                if inner_loops.is_empty() {
                    continue;
                }
                if let Some((guide_faces, guide_points, guide_segments)) =
                    cutout_existing_surface_ring(
                        object,
                        &snapshot,
                        face_index,
                        face,
                        inner_loops,
                        &mut new_selected_faces,
                    )
                {
                    map.selected_geometry_faces = guide_faces;
                    map.selected_geometry_surface_points = guide_points;
                    map.selected_geometry_surface_segments = guide_segments;
                    changed = true;
                    break;
                }
                continue;
            }

            let face_points = [
                snapshot.vertices[face.indices[0]],
                snapshot.vertices[face.indices[1]],
                snapshot.vertices[face.indices[2]],
                snapshot.vertices[face.indices[3]],
            ];
            if inner_loops.is_empty() && loop_segments.is_empty() {
                let point_indices = selected_surface_point_indices_for_face(
                    &selected_points,
                    &selected_segments,
                    snapshot.id,
                    face_index,
                    face,
                );
                if let Some(points) =
                    ordered_surface_points_by_angle(face, &point_indices, face_points)
                {
                    inner_loops.push(points);
                }
            }
            if inner_loops.is_empty() {
                continue;
            }
            if any_loop_extends_beyond_quad(&inner_loops, face_points) {
                if let Some((guide_faces, guide_points, guide_segments)) =
                    cutout_existing_surface_ring(
                        object,
                        &snapshot,
                        face_index,
                        face,
                        inner_loops,
                        &mut new_selected_faces,
                    )
                {
                    map.selected_geometry_faces = guide_faces;
                    map.selected_geometry_surface_points = guide_points;
                    map.selected_geometry_surface_segments = guide_segments;
                    changed = true;
                    break;
                }
                continue;
            }
            let mut snapped_loops = Vec::new();
            for mut inner_front in inner_loops {
                if snap_cutout_loop_to_boundary(&mut inner_front, face_points).is_some() {
                    snapped_loops.push(inner_front);
                }
            }
            if snapped_loops.is_empty() {
                continue;
            }

            let Some(normal) = local_face_normal(&snapshot, face) else {
                continue;
            };
            let Some(center) = face_center(&snapshot, face) else {
                continue;
            };

            let mut opposite: Option<(usize, f32)> = None;
            for (candidate_index, candidate) in snapshot.faces.iter().enumerate() {
                if candidate_index == face_index || candidate.indices.len() != 4 {
                    continue;
                }
                let Some(candidate_normal) = local_face_normal(&snapshot, candidate) else {
                    continue;
                };
                if normal.dot(candidate_normal) > -0.95 {
                    continue;
                }
                let Some(candidate_center) = face_center(&snapshot, candidate) else {
                    continue;
                };
                let distance = (candidate_center - center).dot(normal).abs();
                if distance <= 1e-4 {
                    continue;
                }
                if opposite
                    .as_ref()
                    .is_none_or(|(_, best_distance)| distance > *best_distance)
                {
                    opposite = Some((candidate_index, distance));
                }
            }
            let Some((opposite_index, _)) = opposite else {
                continue;
            };
            let Some(opposite_face) = snapshot.faces.get(opposite_index).cloned() else {
                continue;
            };
            let Some(opposite_center) = face_center(&snapshot, &opposite_face) else {
                continue;
            };
            let depth = (opposite_center - center).dot(normal);
            if depth.abs() <= 1e-4 {
                continue;
            }

            let mut retained_faces = Vec::with_capacity(object.faces.len().saturating_sub(2));
            for (index, existing) in object.faces.iter().cloned().enumerate() {
                let remove = index == face_index
                    || index == opposite_index
                    || face_on_cut_plane(&snapshot, &existing, center, normal)
                    || face_on_cut_plane(&snapshot, &existing, opposite_center, normal);
                if !remove {
                    retained_faces.push(existing);
                }
            }
            object.faces = retained_faces;

            let front_outer_points = face_points.to_vec();
            let Some(front_ring) = append_selected_cutout_ring(
                object,
                face,
                front_outer_points.clone(),
                face.indices.clone(),
                &snapped_loops,
                normal,
            ) else {
                *object = snapshot.clone();
                continue;
            };
            let back_points = [
                face_points[0] + normal * depth,
                face_points[1] + normal * depth,
                face_points[2] + normal * depth,
                face_points[3] + normal * depth,
            ];
            let Some(back_indices) =
                aligned_face_indices_for_points(&snapshot, &opposite_face, back_points)
            else {
                *object = snapshot.clone();
                continue;
            };
            let back_inner = front_ring
                .holes
                .iter()
                .map(|hole| {
                    hole.loop_points
                        .iter()
                        .map(|point| *point + normal * depth)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let Some(back_ring) = append_selected_cutout_ring(
                object,
                &opposite_face,
                back_points.to_vec(),
                back_indices.to_vec(),
                &back_inner,
                -normal,
            ) else {
                *object = snapshot.clone();
                continue;
            };

            let mut aligned_back_holes = Vec::with_capacity(front_ring.holes.len());
            for (front_hole, back_hole) in front_ring.holes.iter().zip(back_ring.holes.iter()) {
                let Some(back_indices) =
                    aligned_back_hole_indices(front_hole, back_hole, normal, depth)
                else {
                    *object = snapshot.clone();
                    aligned_back_holes.clear();
                    break;
                };
                aligned_back_holes.push(back_indices);
            }
            if aligned_back_holes.len() != front_ring.holes.len() {
                continue;
            }

            append_cutout_reveal_faces(
                object,
                face,
                &front_ring,
                &aligned_back_holes,
                normal,
                depth,
                &mut new_selected_faces,
            );
            prune_cutout_internal_caps(object, &front_outer_points, &front_ring);
            if let Some((guide_faces, guide_points, guide_segments)) =
                reattach_cutout_surface_guides(object, &front_ring)
            {
                map.selected_geometry_faces = guide_faces;
                map.selected_geometry_surface_points = guide_points;
                map.selected_geometry_surface_segments = guide_segments;
            }
            changed = true;
            break;
        }
    }

    if changed {
        if map.selected_geometry_surface_segments.is_empty()
            && map.selected_geometry_surface_points.is_empty()
        {
            map.selected_geometry_faces = new_selected_faces;
        }
        map.selected_geometry_vertices.clear();
        sanitize_geometry_selection(map);
    }
    changed
}

pub(crate) fn cut_opening_selected_geometry_faces(map: &mut Map, width: f32, height: f32) -> bool {
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
            let snapshot = object.clone();
            let Some(face) = snapshot.faces.get(face_index).cloned() else {
                continue;
            };
            if face.indices.len() != 4 {
                continue;
            }
            let Some(normal) = local_face_normal(&snapshot, &face) else {
                continue;
            };
            let Some(center) = face_center(&snapshot, &face) else {
                continue;
            };

            let mut opposite: Option<(usize, f32)> = None;
            for (candidate_index, candidate) in snapshot.faces.iter().enumerate() {
                if candidate_index == face_index || candidate.indices.len() != 4 {
                    continue;
                }
                let Some(candidate_normal) = local_face_normal(&snapshot, candidate) else {
                    continue;
                };
                if normal.dot(candidate_normal) > -0.95 {
                    continue;
                }
                let Some(candidate_center) = face_center(&snapshot, candidate) else {
                    continue;
                };
                let distance = (candidate_center - center).dot(normal).abs();
                if distance <= 1e-4 {
                    continue;
                }
                if opposite
                    .as_ref()
                    .is_none_or(|(_, best_distance)| distance > *best_distance)
                {
                    opposite = Some((candidate_index, distance));
                }
            }
            let Some((opposite_index, _)) = opposite else {
                continue;
            };
            let Some(opposite_face) = snapshot.faces.get(opposite_index).cloned() else {
                continue;
            };
            let Some(opposite_center) = face_center(&snapshot, &opposite_face) else {
                continue;
            };
            let depth = (opposite_center - center).dot(normal);
            if depth.abs() <= 1e-4 {
                continue;
            }

            let face_points = [
                snapshot.vertices[face.indices[0]],
                snapshot.vertices[face.indices[1]],
                snapshot.vertices[face.indices[2]],
                snapshot.vertices[face.indices[3]],
            ];
            let opposite_points = [
                face_points[0] + normal * depth,
                face_points[1] + normal * depth,
                face_points[2] + normal * depth,
                face_points[3] + normal * depth,
            ];

            let mut retained_faces = Vec::with_capacity(object.faces.len().saturating_sub(2));
            for (index, existing) in object.faces.iter().cloned().enumerate() {
                if index != face_index && index != opposite_index {
                    retained_faces.push(existing);
                }
            }
            object.faces = retained_faces;

            let Some(front_loop) =
                append_opening_ring(object, &face, face_points, width, height, false)
            else {
                *object = snapshot;
                continue;
            };
            let Some(back_loop) =
                append_opening_ring(object, &opposite_face, opposite_points, width, height, true)
            else {
                *object = snapshot;
                continue;
            };

            for index in 0..4 {
                let next = (index + 1) % 4;
                let reveal_face_index = push_geometry_face(
                    object,
                    &face,
                    vec![
                        front_loop[index],
                        front_loop[next],
                        back_loop[next],
                        back_loop[index],
                    ],
                );
                new_selected_faces.push((object.id, reveal_face_index));
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
        let mut processed_faces = BTreeSet::new();
        let mut added_faces = Vec::new();
        let mut cap_added_indices = Vec::new();
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

            let mut cap_face = face.clone();
            cap_face.indices = cap_indices.clone();
            cap_face.uvs = face_uvs_for_indices(object, &cap_face.indices);
            cap_face.auto_uv = true;
            cap_added_indices.push(added_faces.len());
            added_faces.push(cap_face);

            for index in 0..face.indices.len() {
                let next = (index + 1) % face.indices.len();
                let side_indices = vec![
                    face.indices[index],
                    face.indices[next],
                    cap_indices[next],
                    cap_indices[index],
                ];
                added_faces.push(rusterix::GeometryFace {
                    uvs: face_uvs_for_indices(object, &side_indices),
                    indices: side_indices,
                    auto_uv: true,
                    texture_offset: face.texture_offset,
                    texture_scale: face.texture_scale,
                    texture_rotation: face.texture_rotation,
                    tile: face.tile.clone(),
                    tiles: FxHashMap::default(),
                    surface_points: Vec::new(),
                    surface_segments: Vec::new(),
                });
            }
            processed_faces.insert(face_index);
            changed = true;
        }

        if !added_faces.is_empty() {
            let mut retained_faces = snapshot
                .faces
                .iter()
                .cloned()
                .enumerate()
                .filter_map(|(index, face)| (!processed_faces.contains(&index)).then_some(face))
                .collect::<Vec<_>>();
            let retained_count = retained_faces.len();
            new_selected_faces.extend(
                cap_added_indices
                    .iter()
                    .map(|added_index| (object.id, retained_count + *added_index)),
            );
            retained_faces.extend(added_faces);
            object.faces = retained_faces;
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

        let selected_faces = face_indices.into_iter().collect::<BTreeSet<_>>();
        let snapshot = object.clone();
        let mut edge_midpoints = BTreeMap::new();
        let mut replacements: BTreeMap<usize, Vec<rusterix::GeometryFace>> = BTreeMap::new();

        for face_index in selected_faces.iter().copied() {
            let Some(face) = snapshot.faces.get(face_index).cloned() else {
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
                snapshot.vertices.get(a).copied(),
                snapshot.vertices.get(b).copied(),
                snapshot.vertices.get(c).copied(),
                snapshot.vertices.get(d).copied(),
            ) else {
                continue;
            };

            let mut midpoint = |edge: (usize, usize), pos: Vec3<f32>| {
                if let Some(index) = edge_midpoints.get(&normalized_edge(edge.0, edge.1)) {
                    *index
                } else {
                    let index = object.vertices.len();
                    object.vertices.push(pos);
                    edge_midpoints.insert(normalized_edge(edge.0, edge.1), index);
                    index
                }
            };

            let ab = midpoint((a, b), (pa + pb) * 0.5);
            let bc = midpoint((b, c), (pb + pc) * 0.5);
            let cd = midpoint((c, d), (pc + pd) * 0.5);
            let da = midpoint((d, a), (pd + pa) * 0.5);
            let center = object.vertices.len();
            object.vertices.push((pa + pb + pc + pd) * 0.25);

            let make_face = |indices: Vec<usize>| {
                let mut new_face = face.clone();
                new_face.indices = indices;
                new_face.uvs = face_uvs_for_indices(object, &new_face.indices);
                new_face.auto_uv = true;
                new_face.surface_points.clear();
                new_face.surface_segments.clear();
                new_face
            };

            replacements.insert(
                face_index,
                vec![
                    make_face(vec![a, ab, center, da]),
                    make_face(vec![ab, b, bc, center]),
                    make_face(vec![center, bc, c, cd]),
                    make_face(vec![da, center, cd, d]),
                ],
            );

            changed = true;
        }

        if !replacements.is_empty() {
            let mut rebuilt_faces =
                Vec::with_capacity(snapshot.faces.len() + replacements.len() * 3);
            for (face_index, face) in snapshot.faces.iter().cloned().enumerate() {
                if let Some(replacement_faces) = replacements.get(&face_index) {
                    for face in replacement_faces {
                        new_selected_faces.push((object.id, rebuilt_faces.len()));
                        rebuilt_faces.push(face.clone());
                    }
                    continue;
                }

                let mut indices = Vec::with_capacity(face.indices.len() + face.indices.len());
                let mut inserted = false;
                for index in 0..face.indices.len() {
                    let a = face.indices[index];
                    let b = face.indices[(index + 1) % face.indices.len()];
                    indices.push(a);
                    if let Some(midpoint) = edge_midpoints.get(&normalized_edge(a, b)).copied() {
                        if !selected_faces.contains(&face_index) {
                            indices.push(midpoint);
                            inserted = true;
                        }
                    }
                }

                if inserted {
                    let mut face = face;
                    face.indices = indices;
                    face.uvs = face_uvs_for_indices(object, &face.indices);
                    face.auto_uv = true;
                    rebuilt_faces.push(face);
                } else {
                    rebuilt_faces.push(face);
                }
            }
            object.faces = rebuilt_faces;
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
                    texture_offset: face.texture_offset,
                    texture_scale: face.texture_scale,
                    texture_rotation: face.texture_rotation,
                    tile: face.tile.clone(),
                    tiles: FxHashMap::default(),
                    surface_points: Vec::new(),
                    surface_segments: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn add_surface_loop(
        map: &mut Map,
        object_index: usize,
        face_index: usize,
        points: &[Vec3<f32>],
    ) {
        let object_id = map.geometry_objects[object_index].id;
        let face = &mut map.geometry_objects[object_index].faces[face_index];
        let start = face.surface_points.len();
        for point in points {
            face.surface_points.push(rusterix::GeometrySurfacePoint {
                position: *point,
                mode: rusterix::GeometrySurfacePointMode::Corner,
            });
        }

        map.selected_geometry_surface_points.clear();
        map.selected_geometry_surface_segments.clear();
        for offset in 0..points.len() {
            let segment_index = face.surface_segments.len();
            face.surface_segments
                .push(rusterix::GeometrySurfaceSegment {
                    start: start + offset,
                    end: start + ((offset + 1) % points.len()),
                    mode: rusterix::GeometrySurfaceSegmentMode::Line,
                    curve_amount: 0.35,
                });
            map.selected_geometry_surface_segments
                .push((object_id, face_index, segment_index));
        }
    }

    fn selected_face_index(map: &Map, object_id: Uuid) -> usize {
        map.selected_geometry_faces
            .iter()
            .find_map(|(selected_object_id, face_index)| {
                (*selected_object_id == object_id).then_some(*face_index)
            })
            .expect("cutout should leave a guide face selected")
    }

    fn bottom_cap_overlaps(object: &rusterix::GeometryObject, hole: &[Vec3<f32>]) -> bool {
        cap_overlaps_at_y(object, hole, 0.0)
    }

    fn cap_overlaps_at_y(object: &rusterix::GeometryObject, hole: &[Vec3<f32>], y: f32) -> bool {
        let hole_xz = hole
            .iter()
            .map(|point| Vec2::new(point.x, point.z))
            .collect::<Vec<_>>();

        object.faces.iter().any(|face| {
            let Some(normal) = local_face_normal(object, face) else {
                return false;
            };
            if normal.y.abs() < 0.95 {
                return false;
            }
            let points = face
                .indices
                .iter()
                .filter_map(|index| object.vertices.get(*index).copied())
                .collect::<Vec<_>>();
            if points.len() != face.indices.len()
                || points.iter().any(|point| (point.y - y).abs() > 0.0001)
            {
                return false;
            }
            let face_xz = points
                .iter()
                .map(|point| Vec2::new(point.x, point.z))
                .collect::<Vec<_>>();
            polygons_overlap_2d(&face_xz, &hole_xz)
        })
    }

    #[test]
    fn subdivide_keeps_all_child_faces_selected() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "floor",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.5, 4.0),
        );
        let object_id = object.id;
        let top_face_index = object
            .faces
            .iter()
            .position(|face| {
                local_face_normal(&object, face)
                    .map(|normal| normal.y > 0.9)
                    .unwrap_or(false)
            })
            .expect("box should have a top face");
        map.geometry_objects.push(object);
        map.selected_geometry_faces
            .push((object_id, top_face_index));

        assert!(subdivide_selected_geometry_faces(&mut map));

        let selected = map
            .selected_geometry_faces
            .iter()
            .filter(|(selected_object_id, _)| *selected_object_id == object_id)
            .count();
        assert_eq!(selected, 4);
    }

    #[test]
    fn subdivide_splits_neighbor_faces_on_shared_edges() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "floor",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.5, 4.0),
        );
        let object_id = object.id;
        let top_face_index = object
            .faces
            .iter()
            .position(|face| {
                local_face_normal(&object, face)
                    .map(|normal| normal.y > 0.9)
                    .unwrap_or(false)
            })
            .expect("box should have a top face");
        let top_edges = object.faces[top_face_index]
            .indices
            .iter()
            .enumerate()
            .map(|(index, a)| {
                normalized_edge(
                    *a,
                    object.faces[top_face_index].indices
                        [(index + 1) % object.faces[top_face_index].indices.len()],
                )
            })
            .collect::<BTreeSet<_>>();
        map.geometry_objects.push(object);
        map.selected_geometry_faces
            .push((object_id, top_face_index));

        assert!(subdivide_selected_geometry_faces(&mut map));

        let object = &map.geometry_objects[0];
        for edge in top_edges {
            let midpoint = object
                .vertices
                .iter()
                .enumerate()
                .find_map(|(index, point)| {
                    let expected = (object.vertices[edge.0] + object.vertices[edge.1]) * 0.5;
                    ((*point - expected).magnitude_squared() <= 0.000001).then_some(index)
                })
                .expect("subdivide should create a midpoint for each selected boundary edge");
            let containing_faces = object
                .faces
                .iter()
                .filter(|face| face.indices.contains(&midpoint))
                .count();
            assert!(
                containing_faces >= 3,
                "boundary midpoint should be shared by child faces and the neighboring side face"
            );
        }
    }

    #[test]
    fn cutout_rebuilds_bottom_without_caps_after_repeated_surface_loops() {
        let mut map = Map::new();
        let object = rusterix::GeometryObject::box_from_bounds(
            "floor",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(8.0, 0.5, 6.0),
        );
        let object_id = object.id;
        map.geometry_objects.push(object);

        let first_hole = vec![
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::new(2.5, 0.5, 1.0),
            Vec3::new(2.5, 0.5, 2.5),
            Vec3::new(1.0, 0.5, 2.5),
        ];
        add_surface_loop(&mut map, 0, 4, &first_hole);
        assert!(cutout_selected_surface_loop(&mut map));
        assert!(!bottom_cap_overlaps(&map.geometry_objects[0], &first_hole));

        let second_face = selected_face_index(&map, object_id);
        let second_hole = vec![
            Vec3::new(4.0, 0.5, 1.0),
            Vec3::new(5.5, 0.5, 1.2),
            Vec3::new(5.2, 0.5, 2.6),
            Vec3::new(3.8, 0.5, 2.4),
        ];
        add_surface_loop(&mut map, 0, second_face, &second_hole);
        assert!(cutout_selected_surface_loop(&mut map));
        assert!(!bottom_cap_overlaps(&map.geometry_objects[0], &first_hole));
        assert!(!bottom_cap_overlaps(&map.geometry_objects[0], &second_hole));
    }

    #[test]
    fn cutout_uses_coplanar_surface_when_guide_spans_split_face() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_projects/CutoutSeveralFaces1.eldiron");
        let contents = std::fs::read_to_string(path)
            .expect("CutoutSeveralFaces1 fixture should be available for tests");
        let mut project: Project =
            serde_json::from_str(&contents).expect("CutoutSeveralFaces1 fixture deserializes");
        let map = &mut project.regions[0].map;
        let (object_id, face_index, segment_count, guide_loop) = map
            .geometry_objects
            .iter()
            .find_map(|object| {
                object
                    .faces
                    .iter()
                    .enumerate()
                    .find_map(|(face_index, face)| {
                        (!face.surface_segments.is_empty()).then(|| {
                            let guide_loop = face
                                .surface_points
                                .iter()
                                .map(|point| point.position)
                                .collect::<Vec<_>>();
                            (
                                object.id,
                                face_index,
                                face.surface_segments.len(),
                                guide_loop,
                            )
                        })
                    })
            })
            .expect("fixture should contain surface detail");
        map.selected_geometry_surface_segments = (0..segment_count)
            .map(|segment_index| (object_id, face_index, segment_index))
            .collect();

        assert!(cutout_selected_surface_loop(map));

        let object = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)
            .expect("cutout should keep the source object");
        assert!(!cap_overlaps_at_y(object, &guide_loop, 0.5));
        assert!(!cap_overlaps_at_y(object, &guide_loop, 0.0));
        assert!(
            object
                .faces
                .iter()
                .all(|face| face.indices.len() >= 3 && face.uvs.len() == face.indices.len())
        );
    }
}
