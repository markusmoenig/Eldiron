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
    loop_indices: Vec<usize>,
    loop_points: Vec<Vec3<f32>>,
    loop_uvs: Vec<Vec2<f32>>,
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

fn append_selected_cutout_ring(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryFace,
    outer_points: [Vec3<f32>; 4],
    inner_points: &[Vec3<f32>],
    desired_normal: Vec3<f32>,
) -> Option<OpeningRing> {
    if inner_points.len() < 3 {
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
    let outer_world = outer_points.to_vec();

    let mut inner_uv = inner_points
        .iter()
        .map(|point| to_uv(*point))
        .collect::<Vec<_>>();
    let mut inner_world = inner_points.to_vec();
    if polygon_area_2d(&inner_uv) > 0.0 {
        inner_uv.reverse();
        inner_world.reverse();
    }

    let mut all_uv = outer_uv;
    all_uv.extend(inner_uv.iter().copied());
    let mut all_world = outer_world;
    all_world.extend(inner_world.iter().copied());

    let flat = all_uv
        .iter()
        .flat_map(|point| [point.x as f64, point.y as f64])
        .collect::<Vec<_>>();
    let triangles = earcut(&flat, &[4], 2).ok()?;
    if triangles.is_empty() {
        return None;
    }

    let base = object.vertices.len();
    object.vertices.extend(all_world);
    let vertex_indices = (0..all_uv.len())
        .map(|index| base + index)
        .collect::<Vec<_>>();
    for triangle in triangles.chunks_exact(3) {
        push_geometry_face_with_normal(
            object,
            source,
            vec![
                vertex_indices[triangle[0]],
                vertex_indices[triangle[1]],
                vertex_indices[triangle[2]],
            ],
            desired_normal,
        );
    }

    Some(OpeningRing {
        loop_indices: vertex_indices[4..].to_vec(),
        loop_points: inner_world,
        loop_uvs: inner_uv,
    })
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
    let mut edges: Vec<(usize, usize)> = Vec::new();
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
            edges.push((start_index, end_index));
        }
    }

    let mut adjacency: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (a, b) in &edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    if adjacency.len() < 3 || adjacency.values().any(|neighbors| neighbors.len() != 2) {
        return None;
    }

    let start = *adjacency.keys().next()?;
    let mut ordered = vec![start];
    let mut previous = start;
    let mut current = adjacency.get(&start)?.first().copied()?;
    while current != start {
        if ordered.contains(&current) {
            return None;
        }
        ordered.push(current);
        let neighbors = adjacency.get(&current)?;
        let next = neighbors
            .iter()
            .copied()
            .find(|neighbor| *neighbor != previous)?;
        previous = current;
        current = next;
        if ordered.len() > adjacency.len() {
            return None;
        }
    }
    if ordered.len() != adjacency.len() {
        return None;
    }

    let mut points = ordered
        .iter()
        .filter_map(|point_index| nodes.get(*point_index).copied())
        .collect::<Vec<_>>();
    points.dedup_by(|a, b| (*a - *b).magnitude_squared() <= 0.0001);
    if points.len() < 3 {
        return None;
    }
    Some(points)
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
            if face.indices.len() != 4 {
                continue;
            }
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
            let mut loop_segments = selected_segments
                .iter()
                .filter_map(|(selected_object_id, selected_face_index, segment_index)| {
                    (*selected_object_id == snapshot.id && *selected_face_index == face_index)
                        .then_some(*segment_index)
                })
                .collect::<BTreeSet<_>>();
            let mut inner_front = ordered_surface_loop_points(face, &loop_segments);
            if inner_front.is_none() {
                loop_segments = segment_indices.clone();
                inner_front = ordered_surface_loop_points(face, &loop_segments);
            }
            let face_points = [
                snapshot.vertices[face.indices[0]],
                snapshot.vertices[face.indices[1]],
                snapshot.vertices[face.indices[2]],
                snapshot.vertices[face.indices[3]],
            ];
            if inner_front.is_none() && loop_segments.is_empty() {
                let point_indices = selected_surface_point_indices_for_face(
                    &selected_points,
                    &selected_segments,
                    snapshot.id,
                    face_index,
                    face,
                );
                inner_front = ordered_surface_points_by_angle(face, &point_indices, face_points);
            }
            let Some(mut inner_front) = inner_front else {
                continue;
            };
            let Some(_) = snap_cutout_loop_to_boundary(&mut inner_front, face_points) else {
                continue;
            };

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

            let Some(front_ring) =
                append_selected_cutout_ring(object, face, face_points, &inner_front, normal)
            else {
                *object = snapshot.clone();
                continue;
            };
            let back_points = [
                face_points[0] + normal * depth,
                face_points[1] + normal * depth,
                face_points[2] + normal * depth,
                face_points[3] + normal * depth,
            ];
            let back_inner = front_ring
                .loop_points
                .iter()
                .map(|point| *point + normal * depth)
                .collect::<Vec<_>>();
            let Some(back_ring) = append_selected_cutout_ring(
                object,
                &opposite_face,
                back_points,
                &back_inner,
                -normal,
            ) else {
                *object = snapshot.clone();
                continue;
            };

            for index in 0..front_ring.loop_indices.len() {
                let next = (index + 1) % front_ring.loop_indices.len();
                if edge_on_same_outer_boundary(
                    front_ring.loop_uvs[index],
                    front_ring.loop_uvs[next],
                ) {
                    continue;
                }
                let reveal_face_index = push_geometry_face_with_normal(
                    object,
                    face,
                    vec![
                        front_ring.loop_indices[index],
                        front_ring.loop_indices[next],
                        back_ring.loop_indices[next],
                        back_ring.loop_indices[index],
                    ],
                    (front_ring.loop_points[next] - front_ring.loop_points[index])
                        .cross(normal * depth)
                        .try_normalized()
                        .unwrap_or(normal),
                );
                new_selected_faces.push((object.id, reveal_face_index));
            }
            changed = true;
            break;
        }
    }

    if changed {
        map.selected_geometry_faces = new_selected_faces;
        map.selected_geometry_vertices.clear();
        map.selected_geometry_surface_points.clear();
        map.selected_geometry_surface_segments.clear();
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
                object.faces.push(rusterix::GeometryFace {
                    uvs: face_uvs_for_indices(object, &side_indices),
                    indices: side_indices,
                    auto_uv: true,
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
