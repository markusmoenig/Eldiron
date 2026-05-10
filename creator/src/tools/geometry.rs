use crate::editor::RUSTERIX;
use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use earcutr::earcut;
use rusterix::prelude::*;
use scenevm::GeoId;
use std::collections::{BTreeMap, BTreeSet};

const GIZMO_AXIS_X: u32 = 1;
const GIZMO_AXIS_Y: u32 = 2;
const GIZMO_AXIS_Z: u32 = 3;
const GIZMO_RESIZE_X_MIN: u32 = 101;
const GIZMO_RESIZE_X_MAX: u32 = 102;
const GIZMO_RESIZE_Y_MIN: u32 = 103;
const GIZMO_RESIZE_Y_MAX: u32 = 104;
const GIZMO_RESIZE_Z_MIN: u32 = 105;
const GIZMO_RESIZE_Z_MAX: u32 = 106;

pub struct GeometryTool {
    id: TheId,
    hud: Hud,
    undo_map: Option<Map>,
    drag: Option<GeometryDrag>,
}

struct GeometryDrag {
    object_id: Uuid,
    start_hit: Vec3<f32>,
    start_vertices: Vec<Vec3<f32>>,
    start_transform: [[f32; 4]; 4],
    vertex_indices: Option<Vec<usize>>,
    axis: Option<Vec3<f32>>,
    gizmo_kind: Option<GizmoDragKind>,
    start_plane_hit: Option<Vec3<f32>>,
    changed: bool,
}

#[derive(Clone, Copy, Debug)]
enum GizmoDragKind {
    Move,
    Resize { component: usize, sign: f32 },
}

#[derive(Clone, Copy, Debug)]
struct FaceHit {
    index: usize,
    distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GeometrySelectionMode {
    Object,
    Face,
    Vertex,
    Edge,
}

fn geometry_selection_mode(map: &Map) -> GeometrySelectionMode {
    match map.geometry_selection_mode {
        1 => GeometrySelectionMode::Face,
        2 => GeometrySelectionMode::Vertex,
        3 => GeometrySelectionMode::Edge,
        _ => GeometrySelectionMode::Object,
    }
}

fn gizmo_axis(axis_id: u32) -> Option<Vec3<f32>> {
    match axis_id {
        GIZMO_AXIS_X => Some(Vec3::new(1.0, 0.0, 0.0)),
        GIZMO_AXIS_Y => Some(Vec3::new(0.0, 1.0, 0.0)),
        GIZMO_AXIS_Z => Some(Vec3::new(0.0, 0.0, 1.0)),
        GIZMO_RESIZE_X_MIN => Some(Vec3::new(-1.0, 0.0, 0.0)),
        GIZMO_RESIZE_X_MAX => Some(Vec3::new(1.0, 0.0, 0.0)),
        GIZMO_RESIZE_Y_MIN => Some(Vec3::new(0.0, -1.0, 0.0)),
        GIZMO_RESIZE_Y_MAX => Some(Vec3::new(0.0, 1.0, 0.0)),
        GIZMO_RESIZE_Z_MIN => Some(Vec3::new(0.0, 0.0, -1.0)),
        GIZMO_RESIZE_Z_MAX => Some(Vec3::new(0.0, 0.0, 1.0)),
        _ => None,
    }
}

fn gizmo_kind(axis_id: u32) -> Option<GizmoDragKind> {
    match axis_id {
        GIZMO_AXIS_X | GIZMO_AXIS_Y | GIZMO_AXIS_Z => Some(GizmoDragKind::Move),
        GIZMO_RESIZE_X_MIN => Some(GizmoDragKind::Resize {
            component: 0,
            sign: -1.0,
        }),
        GIZMO_RESIZE_X_MAX => Some(GizmoDragKind::Resize {
            component: 0,
            sign: 1.0,
        }),
        GIZMO_RESIZE_Y_MIN => Some(GizmoDragKind::Resize {
            component: 1,
            sign: -1.0,
        }),
        GIZMO_RESIZE_Y_MAX => Some(GizmoDragKind::Resize {
            component: 1,
            sign: 1.0,
        }),
        GIZMO_RESIZE_Z_MIN => Some(GizmoDragKind::Resize {
            component: 2,
            sign: -1.0,
        }),
        GIZMO_RESIZE_Z_MAX => Some(GizmoDragKind::Resize {
            component: 2,
            sign: 1.0,
        }),
        _ => None,
    }
}

fn vec_component(value: Vec3<f32>, component: usize) -> f32 {
    match component {
        0 => value.x,
        1 => value.y,
        2 => value.z,
        _ => 0.0,
    }
}

fn bound_vertex_indices(
    vertices: &[Vec3<f32>],
    bounds: (Vec3<f32>, Vec3<f32>),
    component: usize,
    sign: f32,
    epsilon: f32,
) -> Vec<usize> {
    let target = if sign < 0.0 {
        vec_component(bounds.0, component)
    } else {
        vec_component(bounds.1, component)
    };
    vertices
        .iter()
        .enumerate()
        .filter_map(|(index, vertex)| {
            ((vec_component(*vertex, component) - target).abs() <= epsilon).then_some(index)
        })
        .collect()
}

fn closest_point_on_axis_to_ray(
    axis_origin: Vec3<f32>,
    axis: Vec3<f32>,
    ray_origin: Vec3<f32>,
    ray_dir: Vec3<f32>,
) -> Option<Vec3<f32>> {
    let ray_dir = ray_dir.try_normalized()?;
    let axis = axis.try_normalized()?;
    let w0 = ray_origin - axis_origin;
    let a = ray_dir.dot(ray_dir);
    let b = ray_dir.dot(axis);
    let c = axis.dot(axis);
    let d = ray_dir.dot(w0);
    let e = axis.dot(w0);
    let denom = a * c - b * b;
    if denom.abs() <= 1e-6 {
        return None;
    }
    let ray_t = (b * e - c * d) / denom;
    let axis_t = (a * e - b * d) / denom;
    (ray_t.is_finite() && ray_t >= 0.0 && axis_t.is_finite()).then_some(axis_origin + axis * axis_t)
}

fn snap_drag_step(amount: f32, step: f32) -> f32 {
    if step <= 0.0 || !amount.is_finite() {
        return 0.0;
    }
    if amount.abs() + 1e-4 < step {
        return 0.0;
    }
    amount.signum() * step
}

fn translated_transform(mut transform: [[f32; 4]; 4], delta: Vec3<f32>) -> [[f32; 4]; 4] {
    transform[3][0] += delta.x;
    transform[3][1] += delta.y;
    transform[3][2] += delta.z;
    transform
}

fn transform_point_with(transform: [[f32; 4]; 4], point: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(
        point.x * transform[0][0]
            + point.y * transform[1][0]
            + point.z * transform[2][0]
            + transform[3][0],
        point.x * transform[0][1]
            + point.y * transform[1][1]
            + point.z * transform[2][1]
            + transform[3][1],
        point.x * transform[0][2]
            + point.y * transform[1][2]
            + point.z * transform[2][2]
            + transform[3][2],
    )
}

fn bbox_for_vertices(vertices: &[Vec3<f32>], transform: [[f32; 4]; 4]) -> Option<rusterix::BBox> {
    let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut found = false;

    for vertex in vertices {
        let world = transform_point_with(transform, *vertex);
        if !world.x.is_finite() || !world.z.is_finite() {
            continue;
        }
        min.x = min.x.min(world.x);
        min.y = min.y.min(world.z);
        max.x = max.x.max(world.x);
        max.y = max.y.max(world.z);
        found = true;
    }

    found.then(|| rusterix::BBox::new(min, max))
}

fn add_bbox_dirty_chunks(bbox: rusterix::BBox, chunks: &mut FxHashSet<(i32, i32)>) {
    if !bbox.min.x.is_finite()
        || !bbox.min.y.is_finite()
        || !bbox.max.x.is_finite()
        || !bbox.max.y.is_finite()
    {
        return;
    }

    let chunk_size = 32;
    let min_cx = (bbox.min.x / chunk_size as f32).floor() as i32;
    let min_cy = (bbox.min.y / chunk_size as f32).floor() as i32;
    let max_cx = (bbox.max.x / chunk_size as f32).ceil() as i32;
    let max_cy = (bbox.max.y / chunk_size as f32).ceil() as i32;
    for cy in min_cy..max_cy.max(min_cy + 1) {
        for cx in min_cx..max_cx.max(min_cx + 1) {
            chunks.insert((cx * chunk_size, cy * chunk_size));
        }
    }
}

fn refresh_geometry_topology_edit(old_map: Option<&Map>, map: &Map, ctx: &mut TheContext) {
    let mut dirty_chunks = FxHashSet::default();
    if let Some(old_map) = old_map {
        for object in &old_map.geometry_objects {
            if let Some(bbox) = object.bbox() {
                add_bbox_dirty_chunks(bbox, &mut dirty_chunks);
            }
        }
    }
    for object in &map.geometry_objects {
        if let Some(bbox) = object.bbox() {
            add_bbox_dirty_chunks(bbox, &mut dirty_chunks);
        }
    }

    if !dirty_chunks.is_empty() {
        crate::utils::editor_scene_replace_incremental_map_update(
            map.clone(),
            dirty_chunks.into_iter().collect(),
        );
    } else {
        RUSTERIX.write().unwrap().set_dirty();
    }
    RUSTERIX.write().unwrap().set_overlay_dirty();
    ctx.ui.redraw_all = true;
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Update Geometry Overlay 3D"),
        TheValue::Empty,
    ));
    ctx.ui.send(TheEvent::Custom(
        TheId::named("Map Selection Changed"),
        TheValue::Empty,
    ));
}

fn geometry_bounds(vertices: &[Vec3<f32>]) -> Option<(Vec3<f32>, Vec3<f32>)> {
    let mut min = Vec3::broadcast(f32::INFINITY);
    let mut max = Vec3::broadcast(f32::NEG_INFINITY);
    let mut found = false;
    for vertex in vertices {
        if !vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite() {
            continue;
        }
        min.x = min.x.min(vertex.x);
        min.y = min.y.min(vertex.y);
        min.z = min.z.min(vertex.z);
        max.x = max.x.max(vertex.x);
        max.y = max.y.max(vertex.y);
        max.z = max.z.max(vertex.z);
        found = true;
    }
    found.then_some((min, max))
}

fn resize_selected_geometry(map: &mut Map, delta_size: Vec3<f32>) -> bool {
    if map.selected_geometry_objects.is_empty() {
        return false;
    }

    let selected = map.selected_geometry_objects.clone();
    let min_size = ServerContext::edit_grid_step(map.subdivisions).max(0.05);
    let snap = |value: f32| (value / min_size).round() * min_size;
    let mut changed = false;
    for object in &mut map.geometry_objects {
        if !selected.contains(&object.id) {
            continue;
        }
        let Some((min, max)) = geometry_bounds(&object.vertices) else {
            continue;
        };
        let size = max - min;
        let center = (min + max) * 0.5;
        let target_min = Vec3::new(
            snap(min.x - delta_size.x),
            snap(min.y - delta_size.y),
            snap(min.z - delta_size.z),
        );
        let target_max = Vec3::new(
            snap(max.x + delta_size.x),
            snap(max.y + delta_size.y),
            snap(max.z + delta_size.z),
        );
        let target_size = Vec3::new(
            (target_max.x - target_min.x).max(min_size),
            (target_max.y - target_min.y).max(min_size),
            (target_max.z - target_min.z).max(min_size),
        );
        let target_center = (target_min + target_max) * 0.5;
        if target_size.x <= min_size && delta_size.x < 0.0
            || target_size.y <= min_size && delta_size.y < 0.0
            || target_size.z <= min_size && delta_size.z < 0.0
        {
            continue;
        }
        let safe_size = Vec3::new(size.x.max(0.0001), size.y.max(0.0001), size.z.max(0.0001));
        for vertex in &mut object.vertices {
            let local = (*vertex - center) / safe_size;
            *vertex = target_center + local * target_size;
        }
        changed = true;
    }
    changed
}

fn rotate_selected_geometry_objects_y(map: &mut Map, quarter_turns: i32) -> bool {
    if map.selected_geometry_objects.is_empty() {
        return false;
    }

    let selected = map.selected_geometry_objects.clone();
    let angle = quarter_turns as f32 * std::f32::consts::FRAC_PI_2;
    let cos = angle.cos().round();
    let sin = angle.sin().round();
    let rotate = |point: Vec3<f32>, center: Vec3<f32>| {
        let local = point - center;
        center
            + Vec3::new(
                local.x * cos - local.z * sin,
                local.y,
                local.x * sin + local.z * cos,
            )
    };

    let mut changed = false;
    for object in &mut map.geometry_objects {
        if !selected.contains(&object.id) {
            continue;
        }
        let Some((min, max)) = geometry_bounds(&object.vertices) else {
            continue;
        };
        let center = (min + max) * 0.5;
        for vertex in &mut object.vertices {
            *vertex = rotate(*vertex, center);
        }
        for face in &mut object.faces {
            for point in &mut face.surface_points {
                point.position = rotate(point.position, center);
            }
        }
        changed = true;
    }
    changed
}

fn closest_geometry_vertex(object: &rusterix::GeometryObject, hit: Vec3<f32>) -> Option<usize> {
    object
        .vertices
        .iter()
        .enumerate()
        .filter_map(|(index, vertex)| {
            let world = object.transform_point(*vertex);
            (world.x.is_finite() && world.y.is_finite() && world.z.is_finite())
                .then_some((index, (world - hit).magnitude_squared()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
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

fn point_triangle_distance_squared(
    point: Vec3<f32>,
    a: Vec3<f32>,
    b: Vec3<f32>,
    c: Vec3<f32>,
) -> f32 {
    // Christer Ericson, Real-Time Collision Detection, closest point on triangle.
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (point - a).magnitude_squared();
    }

    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (point - b).magnitude_squared();
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (point - (a + ab * v)).magnitude_squared();
    }

    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (point - c).magnitude_squared();
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (point - (a + ac * w)).magnitude_squared();
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (point - (b + (c - b) * w)).magnitude_squared();
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (point - (a + ab * v + ac * w)).magnitude_squared()
}

fn ray_triangle_hit(
    ray_origin: Vec3<f32>,
    ray_dir: Vec3<f32>,
    a: Vec3<f32>,
    b: Vec3<f32>,
    c: Vec3<f32>,
) -> Option<f32> {
    let edge_1 = b - a;
    let edge_2 = c - a;
    let h = ray_dir.cross(edge_2);
    let det = edge_1.dot(h);
    if det.abs() <= 1e-6 {
        return None;
    }

    let inv_det = 1.0 / det;
    let s = ray_origin - a;
    let u = inv_det * s.dot(h);
    if !(-1e-4..=1.0001).contains(&u) {
        return None;
    }

    let q = s.cross(edge_1);
    let v = inv_det * ray_dir.dot(q);
    if v < -1e-4 || u + v > 1.0001 {
        return None;
    }

    let t = inv_det * edge_2.dot(q);
    (t.is_finite() && t >= 0.0).then_some(t)
}

fn closest_geometry_face_hit(object: &rusterix::GeometryObject, hit: Vec3<f32>) -> Option<FaceHit> {
    object
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            let normal = face_normal(object, face)?;
            let first = object.transform_point(*object.vertices.get(*face.indices.first()?)?);
            let plane_distance = (hit - first).dot(normal).abs();
            let mut surface_distance = f32::INFINITY;
            for index in 1..face.indices.len().saturating_sub(1) {
                let a = object.transform_point(*object.vertices.get(face.indices[0])?);
                let b = object.transform_point(*object.vertices.get(face.indices[index])?);
                let c = object.transform_point(*object.vertices.get(face.indices[index + 1])?);
                surface_distance =
                    surface_distance.min(point_triangle_distance_squared(hit, a, b, c));
            }
            Some(FaceHit {
                index: face_index,
                distance: plane_distance * plane_distance + surface_distance,
            })
        })
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

fn closest_geometry_face_from_ray(
    object: &rusterix::GeometryObject,
    ray_origin: Vec3<f32>,
    ray_dir: Vec3<f32>,
) -> Option<usize> {
    let ray_dir = ray_dir.try_normalized()?;
    object
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            let mut best_t = f32::INFINITY;
            for index in 1..face.indices.len().saturating_sub(1) {
                let a = object.transform_point(*object.vertices.get(face.indices[0])?);
                let b = object.transform_point(*object.vertices.get(face.indices[index])?);
                let c = object.transform_point(*object.vertices.get(face.indices[index + 1])?);
                if let Some(t) = ray_triangle_hit(ray_origin, ray_dir, a, b, c) {
                    best_t = best_t.min(t);
                }
            }
            best_t.is_finite().then_some((face_index, best_t))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(face_index, _)| face_index)
}

fn closest_geometry_face(object: &rusterix::GeometryObject, hit: Vec3<f32>) -> Option<usize> {
    closest_geometry_face_hit(object, hit).map(|hit| hit.index)
}

fn point_segment_distance(point: Vec3<f32>, a: Vec3<f32>, b: Vec3<f32>) -> f32 {
    let ab = b - a;
    let ab_len_sq = ab.dot(ab);
    if ab_len_sq <= 1e-6 {
        return (point - a).magnitude();
    }
    let t = ((point - a).dot(ab) / ab_len_sq).clamp(0.0, 1.0);
    (point - (a + ab * t)).magnitude()
}

fn closest_geometry_edge(
    object: &rusterix::GeometryObject,
    hit: Vec3<f32>,
) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), f32)> = None;
    for face in &object.faces {
        if face.indices.len() < 2 {
            continue;
        }
        for index in 0..face.indices.len() {
            let a_index = face.indices[index];
            let b_index = face.indices[(index + 1) % face.indices.len()];
            let (Some(a), Some(b)) = (object.vertices.get(a_index), object.vertices.get(b_index))
            else {
                continue;
            };
            let distance =
                point_segment_distance(hit, object.transform_point(*a), object.transform_point(*b));
            if best
                .as_ref()
                .map(|(_, best_distance)| distance < *best_distance)
                .unwrap_or(true)
            {
                best = Some(((a_index, b_index), distance));
            }
        }
    }
    best.map(|(edge, _)| edge)
}

fn selected_geometry_vertex_indices(map: &Map, object_id: Uuid) -> Vec<usize> {
    let mut indices = BTreeSet::new();
    for (_, vertex_index) in map
        .selected_geometry_vertices
        .iter()
        .filter(|(id, _)| *id == object_id)
    {
        indices.insert(*vertex_index);
    }
    if let Some(object) = map
        .geometry_objects
        .iter()
        .find(|object| object.id == object_id)
    {
        for (_, face_index) in map
            .selected_geometry_faces
            .iter()
            .filter(|(id, _)| *id == object_id)
        {
            if let Some(face) = object.faces.get(*face_index) {
                indices.extend(face.indices.iter().copied());
            }
        }
    }
    indices.into_iter().collect()
}

fn move_selected_geometry_vertices(map: &mut Map, delta: Vec3<f32>) -> bool {
    let selected_objects = map.selected_geometry_objects.clone();
    let mut changed = false;
    for object_id in selected_objects {
        let indices = selected_geometry_vertex_indices(map, object_id);
        if indices.is_empty() {
            continue;
        }
        let Some(object) = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == object_id)
        else {
            continue;
        };
        for index in indices {
            let Some(vertex) = object.vertices.get_mut(index) else {
                continue;
            };
            *vertex += delta;
            changed = true;
        }
    }
    changed
}

fn move_selected_geometry_faces_along_normals(map: &mut Map, amount: f32) -> bool {
    if map.selected_geometry_faces.is_empty() {
        return false;
    }

    let selected = map.selected_geometry_faces.clone();
    let mut changed = false;
    for object_id in map.selected_geometry_objects.clone() {
        let Some(object_snapshot) = map
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)
            .cloned()
        else {
            continue;
        };
        let mut moves = Vec::<(usize, Vec3<f32>)>::new();
        for (_, face_index) in selected.iter().filter(|(id, _)| *id == object_id) {
            let Some(face) = object_snapshot.faces.get(*face_index) else {
                continue;
            };
            let Some(normal) = editing_face_normal(&object_snapshot, face) else {
                continue;
            };
            for vertex_index in &face.indices {
                moves.push((*vertex_index, normal));
            }
        }
        let mut unique_moves = BTreeMap::<usize, (Vec3<f32>, usize)>::new();
        for (vertex_index, normal) in moves {
            let entry = unique_moves
                .entry(vertex_index)
                .or_insert((Vec3::zero(), 0));
            entry.0 += normal;
            entry.1 += 1;
        }
        let Some(object) = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == object_id)
        else {
            continue;
        };
        for (vertex_index, (normal_sum, count)) in unique_moves {
            if let Some(vertex) = object.vertices.get_mut(vertex_index) {
                let normal = (normal_sum / count.max(1) as f32)
                    .try_normalized()
                    .unwrap_or(normal_sum);
                *vertex += normal * amount;
                changed = true;
            }
        }
    }
    changed
}

fn apply_tile_to_selected_geometry_faces(map: &mut Map, source: PixelSource) -> bool {
    if map.selected_geometry_faces.is_empty() && map.selected_geometry_objects.is_empty() {
        return false;
    }

    let selected_objects = map.selected_geometry_objects.clone();
    let selected = map.selected_geometry_faces.clone();
    let geometry_source = crate::utils::SurfaceApplySource::Direct(source.clone());
    let mut changed = false;
    for object_id in selected_objects {
        changed |= crate::utils::apply_surface_source_to_geometry_object(
            map,
            object_id,
            &geometry_source,
            None,
        );
    }
    for object in &mut map.geometry_objects {
        for (_, face_index) in selected.iter().filter(|(id, _)| *id == object.id) {
            let Some(face) = object.faces.get_mut(*face_index) else {
                continue;
            };
            face.tile = Some(source.clone());
            face.tiles.clear();
            face.auto_uv = true;
            changed = true;
        }
    }
    changed
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
        if current == start || ordered.len() > edges.len() {
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

fn face_uvs_for_indices(object: &rusterix::GeometryObject, indices: &[usize]) -> Vec<Vec2<f32>> {
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

fn face_normal_for_points(points: &[Vec3<f32>]) -> Option<Vec3<f32>> {
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

fn face_projection_basis(points: &[Vec3<f32>], normal: Vec3<f32>) -> Option<(Vec3<f32>, Vec3<f32>)> {
    let origin = points.first().copied()?;
    let tangent = points
        .iter()
        .skip(1)
        .find_map(|point| (*point - origin).try_normalized())?;
    let bitangent = normal.cross(tangent).try_normalized()?;
    Some((tangent, bitangent))
}

fn project_face_points(points: &[Vec3<f32>], normal: Vec3<f32>) -> Option<Vec<Vec2<f32>>> {
    let origin = points.first().copied()?;
    let (tangent, bitangent) = face_projection_basis(points, normal)?;
    Some(
        points
            .iter()
            .map(|point| {
                let local = *point - origin;
                Vec2::new(local.dot(tangent), local.dot(bitangent))
            })
            .collect(),
    )
}

fn polygon_area_2d(points: &[Vec2<f32>]) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }
    area * 0.5
}

fn face_is_planar(points: &[Vec3<f32>], normal: Vec3<f32>, epsilon: f32) -> bool {
    let Some(origin) = points.first().copied() else {
        return false;
    };
    points
        .iter()
        .all(|point| (*point - origin).dot(normal).abs() <= epsilon)
}

fn polygon_is_convex_2d(points: &[Vec2<f32>]) -> bool {
    if points.len() <= 3 {
        return true;
    }
    let winding = polygon_area_2d(points).signum();
    if winding == 0.0 {
        return false;
    }
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        let c = points[(index + 2) % points.len()];
        let ab = b - a;
        let bc = c - b;
        let cross = ab.x * bc.y - ab.y * bc.x;
        if cross.abs() <= 1e-5 {
            continue;
        }
        if cross.signum() != winding {
            return false;
        }
    }
    true
}

fn face_needs_triangulation(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
    epsilon: f32,
) -> bool {
    if face.indices.len() <= 3 {
        return false;
    }
    let points = face
        .indices
        .iter()
        .filter_map(|index| object.vertices.get(*index).copied())
        .collect::<Vec<_>>();
    if points.len() != face.indices.len() {
        return false;
    }
    let Some(normal) = face_normal_for_points(&points) else {
        return false;
    };
    if !face_is_planar(&points, normal, epsilon) {
        return true;
    }
    let Some(projected) = project_face_points(&points, normal) else {
        return false;
    };
    !polygon_is_convex_2d(&projected)
}

fn triangulate_geometry_face(
    object: &rusterix::GeometryObject,
    face: &rusterix::GeometryFace,
) -> Option<Vec<rusterix::GeometryFace>> {
    let points = face
        .indices
        .iter()
        .filter_map(|index| object.vertices.get(*index).copied())
        .collect::<Vec<_>>();
    if points.len() != face.indices.len() || points.len() <= 3 {
        return None;
    }
    let normal = face_normal_for_points(&points)?;
    let projected = project_face_points(&points, normal)?;
    if projected.len() != points.len() || polygon_area_2d(&projected).abs() <= 1e-6 {
        return None;
    }

    let flat = projected
        .iter()
        .flat_map(|point| [point.x as f64, point.y as f64])
        .collect::<Vec<_>>();
    let triangles = earcut(&flat, &[], 2).ok()?;
    if triangles.is_empty() {
        return None;
    }

    let mut faces = Vec::with_capacity(triangles.len() / 3);
    for triangle in triangles.chunks_exact(3) {
        let mut indices = vec![
            face.indices[triangle[0]],
            face.indices[triangle[1]],
            face.indices[triangle[2]],
        ];
        let tri_points = indices
            .iter()
            .filter_map(|index| object.vertices.get(*index).copied())
            .collect::<Vec<_>>();
        if let Some(tri_normal) = face_normal_for_points(&tri_points)
            && tri_normal.dot(normal) < 0.0
        {
            indices.swap(1, 2);
        }
        let mut triangle_face = face.clone();
        triangle_face.indices = indices;
        triangle_face.uvs = face_uvs_for_indices(object, &triangle_face.indices);
        triangle_face.auto_uv = true;
        triangle_face.surface_points.clear();
        triangle_face.surface_segments.clear();
        faces.push(triangle_face);
    }
    Some(faces)
}

fn normalize_geometry_faces_for_object(map: &mut Map, object_id: Uuid) -> bool {
    let Some(object) = map
        .geometry_objects
        .iter_mut()
        .find(|object| object.id == object_id)
    else {
        return false;
    };

    let epsilon = ServerContext::edit_grid_step(map.subdivisions) * 0.001;
    let old_faces = object.faces.clone();
    let mut faces = Vec::with_capacity(old_faces.len());
    let mut changed = false;
    for face in old_faces {
        if face_needs_triangulation(object, &face, epsilon)
            && let Some(triangles) = triangulate_geometry_face(object, &face)
        {
            faces.extend(triangles);
            changed = true;
        } else {
            faces.push(face);
        }
    }

    if changed {
        object.faces = faces;
        map.selected_geometry_faces.clear();
    }
    changed
}

fn normalize_selected_geometry_object_faces(map: &mut Map) -> bool {
    let object_ids = map.selected_geometry_objects.clone();
    let mut changed = false;
    for object_id in object_ids {
        changed |= normalize_geometry_faces_for_object(map, object_id);
    }
    changed
}

fn delete_selected_geometry_faces(map: &mut Map) -> bool {
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
    }
    changed
}

fn fill_selected_geometry_vertices(map: &mut Map) -> bool {
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
    }
    changed
}

fn compact_merged_face_indices(indices: Vec<usize>) -> Option<Vec<usize>> {
    let mut compact = Vec::with_capacity(indices.len());
    for index in indices {
        if compact.last().copied() != Some(index) {
            compact.push(index);
        }
    }
    if compact.len() > 1 && compact.first() == compact.last() {
        compact.pop();
    }

    let mut unique = BTreeSet::new();
    if compact.len() < 3 || compact.iter().any(|index| !unique.insert(*index)) {
        return None;
    }
    Some(compact)
}

fn resolve_vertex_merge_target(targets: &BTreeMap<usize, usize>, index: usize) -> usize {
    let mut current = index;
    let mut seen = BTreeSet::new();
    while let Some(next) = targets.get(&current).copied() {
        if next == current || !seen.insert(current) {
            break;
        }
        current = next;
    }
    current
}

fn rebuild_geometry_object_after_vertex_merge(
    object: &mut rusterix::GeometryObject,
    targets: &BTreeMap<usize, usize>,
) -> Option<BTreeMap<usize, usize>> {
    if targets.is_empty() {
        return None;
    }

    let old_faces = object.faces.clone();
    let mut removed_vertices = BTreeSet::new();
    for old_index in targets.keys().copied() {
        let target = resolve_vertex_merge_target(targets, old_index);
        if target != old_index {
            removed_vertices.insert(old_index);
        }
    }
    if removed_vertices.is_empty() {
        return None;
    }

    let mut remap = vec![None; object.vertices.len()];
    let mut vertices = Vec::with_capacity(object.vertices.len() - removed_vertices.len());
    for (old_index, vertex) in object.vertices.iter().copied().enumerate() {
        if removed_vertices.contains(&old_index) {
            continue;
        }
        remap[old_index] = Some(vertices.len());
        vertices.push(vertex);
    }

    for old_index in &removed_vertices {
        let target = resolve_vertex_merge_target(targets, *old_index);
        let Some(Some(new_target)) = remap.get(target) else {
            continue;
        };
        remap[*old_index] = Some(*new_target);
    }

    object.vertices = vertices;
    let mut faces = Vec::with_capacity(old_faces.len());
    for face in old_faces {
        let mut indices = Vec::with_capacity(face.indices.len());
        let mut valid = true;
        let mut face_changed = false;
        for old_index in &face.indices {
            let Some(Some(new_index)) = remap.get(*old_index) else {
                valid = false;
                break;
            };
            face_changed |= *new_index != *old_index;
            indices.push(*new_index);
        }
        let Some(indices) = valid.then(|| compact_merged_face_indices(indices)).flatten() else {
            continue;
        };

        let mut face = face;
        if face_changed || face.indices.len() != indices.len() {
            face.indices = indices;
            face.uvs = face_uvs_for_indices(object, &face.indices);
            face.auto_uv = true;
            face.surface_points.clear();
            face.surface_segments.clear();
        }
        faces.push(face);
    }
    object.faces = faces;

    let mut old_to_new = BTreeMap::new();
    for (old_index, new_index) in remap.into_iter().enumerate() {
        if let Some(new_index) = new_index {
            old_to_new.insert(old_index, new_index);
        }
    }
    Some(old_to_new)
}

fn merge_selected_geometry_vertices(map: &mut Map) -> bool {
    if map.selected_geometry_vertices.len() < 2 {
        return false;
    }

    let selected = map.selected_geometry_vertices.clone();
    let mut new_selected_vertices = Vec::new();
    let mut changed = false;

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

        let Some(target_old_index) = selected_vertices.iter().next().copied() else {
            continue;
        };
        let center = selected_vertices
            .iter()
            .filter_map(|index| object.vertices.get(*index).copied())
            .fold(Vec3::zero(), |sum, vertex| sum + vertex)
            / selected_vertices.len() as f32;

        let old_faces = object.faces.clone();
        let mut remap = vec![None; object.vertices.len()];
        let mut vertices = Vec::with_capacity(object.vertices.len() - selected_vertices.len() + 1);
        let mut target_new_index = None;

        for (old_index, vertex) in object.vertices.iter().copied().enumerate() {
            if old_index == target_old_index {
                target_new_index = Some(vertices.len());
                remap[old_index] = target_new_index;
                vertices.push(center);
            } else if selected_vertices.contains(&old_index) {
                continue;
            } else {
                remap[old_index] = Some(vertices.len());
                vertices.push(vertex);
            }
        }

        let Some(target_new_index) = target_new_index else {
            continue;
        };
        for old_index in &selected_vertices {
            remap[*old_index] = Some(target_new_index);
        }

        object.vertices = vertices;
        let mut faces = Vec::with_capacity(old_faces.len());
        for face in old_faces {
            let mut indices = Vec::with_capacity(face.indices.len());
            let mut valid = true;
            for old_index in &face.indices {
                let Some(Some(new_index)) = remap.get(*old_index) else {
                    valid = false;
                    break;
                };
                indices.push(*new_index);
            }
            let Some(indices) = valid.then(|| compact_merged_face_indices(indices)).flatten()
            else {
                continue;
            };

            let mut face = face;
            face.indices = indices;
            face.uvs = face_uvs_for_indices(object, &face.indices);
            face.auto_uv = true;
            face.surface_points.clear();
            face.surface_segments.clear();
            faces.push(face);
        }
        object.faces = faces;
        new_selected_vertices.push((object.id, target_new_index));
        changed = true;
    }

    if changed {
        map.selected_geometry_faces.clear();
        map.selected_geometry_vertices = new_selected_vertices;
    }
    changed
}

fn auto_merge_overlapping_selected_geometry_vertices(map: &mut Map, epsilon: f32) -> bool {
    if map.selected_geometry_vertices.is_empty() {
        return false;
    }

    let epsilon_sq = epsilon * epsilon;
    let selected = map.selected_geometry_vertices.clone();
    let mut new_selected_vertices = Vec::new();
    let mut changed = false;

    for object in &mut map.geometry_objects {
        let selected_vertices = selected
            .iter()
            .filter_map(|(object_id, vertex_index)| {
                (*object_id == object.id && *vertex_index < object.vertices.len())
                    .then_some(*vertex_index)
            })
            .collect::<BTreeSet<_>>();
        if selected_vertices.is_empty() {
            continue;
        }

        let mut targets = BTreeMap::new();
        for selected_index in &selected_vertices {
            let Some(position) = object.vertices.get(*selected_index).copied() else {
                continue;
            };

            let mut best: Option<(usize, f32)> = None;
            for (candidate_index, candidate) in object.vertices.iter().copied().enumerate() {
                if candidate_index == *selected_index || selected_vertices.contains(&candidate_index)
                {
                    continue;
                }
                let distance_sq = (candidate - position).magnitude_squared();
                if distance_sq <= epsilon_sq
                    && best
                        .map(|(_, best_distance_sq)| distance_sq < best_distance_sq)
                        .unwrap_or(true)
                {
                    best = Some((candidate_index, distance_sq));
                }
            }

            if best.is_none() {
                for candidate_index in selected_vertices.iter().copied() {
                    if candidate_index >= *selected_index {
                        continue;
                    }
                    let Some(candidate) = object.vertices.get(candidate_index).copied() else {
                        continue;
                    };
                    let distance_sq = (candidate - position).magnitude_squared();
                    if distance_sq <= epsilon_sq
                        && best
                            .map(|(_, best_distance_sq)| distance_sq < best_distance_sq)
                            .unwrap_or(true)
                    {
                        best = Some((candidate_index, distance_sq));
                    }
                }
            }

            if let Some((target_index, _)) = best {
                targets.insert(*selected_index, target_index);
            }
        }

        let Some(old_to_new) = rebuild_geometry_object_after_vertex_merge(object, &targets) else {
            continue;
        };
        let mut selected_after_merge = BTreeSet::new();
        for old_index in selected_vertices {
            let target = resolve_vertex_merge_target(&targets, old_index);
            if let Some(new_index) = old_to_new.get(&target).copied() {
                selected_after_merge.insert((object.id, new_index));
            }
        }
        new_selected_vertices.extend(selected_after_merge.into_iter());
        changed = true;
    }

    if changed {
        map.selected_geometry_faces.clear();
        map.selected_geometry_vertices = new_selected_vertices;
    }
    changed
}

fn split_selected_geometry_edges(map: &mut Map) -> bool {
    if map.selected_geometry_vertices.len() < 2 {
        return false;
    }

    let selected = map.selected_geometry_vertices.clone();
    let mut split_faces = false;

    for object in &mut map.geometry_objects {
        let selected_vertices = selected
            .iter()
            .filter_map(|(object_id, vertex_index)| {
                (*object_id == object.id && *vertex_index < object.vertices.len())
                    .then_some(*vertex_index)
            })
            .collect::<Vec<_>>();
        if selected_vertices.len() != 2 {
            continue;
        }

        let a = selected_vertices[0];
        let b = selected_vertices[1];
        let original_face_count = object.faces.len();
        for face_index in 0..original_face_count {
            let old_face = object.faces[face_index].clone();
            let Some(pos_a) = old_face.indices.iter().position(|index| *index == a) else {
                continue;
            };
            let Some(pos_b) = old_face.indices.iter().position(|index| *index == b) else {
                continue;
            };
            let len = old_face.indices.len();
            if len < 4 {
                continue;
            }
            let delta = pos_a.abs_diff(pos_b);
            if delta == 1 || delta == len - 1 {
                continue;
            }

            let collect_loop = |start: usize, end: usize| {
                let mut out = Vec::new();
                let mut index = start;
                loop {
                    out.push(old_face.indices[index]);
                    if index == end {
                        break;
                    }
                    index = (index + 1) % len;
                }
                out
            };
            let first_indices = collect_loop(pos_a, pos_b);
            let second_indices = collect_loop(pos_b, pos_a);
            if first_indices.len() < 3 || second_indices.len() < 3 {
                continue;
            }

            let mut first_face = old_face.clone();
            first_face.indices = first_indices;
            first_face.uvs = face_uvs_for_indices(object, &first_face.indices);
            first_face.auto_uv = true;
            first_face.surface_points.clear();
            first_face.surface_segments.clear();

            let mut second_face = old_face;
            second_face.indices = second_indices;
            second_face.uvs = face_uvs_for_indices(object, &second_face.indices);
            second_face.auto_uv = true;
            second_face.surface_points.clear();
            second_face.surface_segments.clear();

            object.faces[face_index] = first_face;
            object.faces.push(second_face);
            split_faces = true;
        }
    }

    if split_faces {
        map.selected_geometry_faces.clear();
        map.selected_geometry_vertices = selected;
        return true;
    }

    let mut new_selected_vertices = Vec::new();
    let mut changed = false;

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

        let mut edge_midpoints = BTreeMap::new();
        for face_index in 0..object.faces.len() {
            let old_indices = object.faces[face_index].indices.clone();
            let mut indices = Vec::with_capacity(old_indices.len() + 1);
            let mut face_changed = false;
            for index in 0..old_indices.len() {
                let a = old_indices[index];
                let b = old_indices[(index + 1) % old_indices.len()];
                indices.push(a);
                if selected_vertices.contains(&a) && selected_vertices.contains(&b) {
                    let edge = normalized_edge(a, b);
                    let midpoint_index = if let Some(midpoint_index) = edge_midpoints.get(&edge) {
                        *midpoint_index
                    } else {
                        let Some(pa) = object.vertices.get(a).copied() else {
                            continue;
                        };
                        let Some(pb) = object.vertices.get(b).copied() else {
                            continue;
                        };
                        let midpoint_index = object.vertices.len();
                        object.vertices.push((pa + pb) * 0.5);
                        edge_midpoints.insert(edge, midpoint_index);
                        new_selected_vertices.push((object.id, midpoint_index));
                        midpoint_index
                    };
                    indices.push(midpoint_index);
                    face_changed = true;
                }
            }

            if face_changed {
                let uvs = face_uvs_for_indices(object, &indices);
                object.faces[face_index].indices = indices;
                object.faces[face_index].uvs = uvs;
                object.faces[face_index].auto_uv = true;
                object.faces[face_index].surface_points.clear();
                object.faces[face_index].surface_segments.clear();
                changed = true;
            }
        }
    }

    if changed {
        map.selected_geometry_faces.clear();
        map.selected_geometry_vertices = new_selected_vertices;
    }
    changed
}

fn delete_selected_geometry_vertices(map: &mut Map) -> bool {
    if map.selected_geometry_vertices.is_empty() {
        return false;
    }

    let mut changed = false;
    for object in &mut map.geometry_objects {
        let selected_vertices = map
            .selected_geometry_vertices
            .iter()
            .filter_map(|(object_id, vertex_index)| {
                (*object_id == object.id).then_some(*vertex_index)
            })
            .collect::<BTreeSet<_>>();
        if selected_vertices.is_empty() {
            continue;
        }

        let old_vertex_len = object.vertices.len();
        let old_face_len = object.faces.len();
        let mut remap = vec![None; object.vertices.len()];
        let mut vertices = Vec::with_capacity(object.vertices.len());
        for (old_index, vertex) in object.vertices.iter().copied().enumerate() {
            if selected_vertices.contains(&old_index) {
                continue;
            }
            remap[old_index] = Some(vertices.len());
            vertices.push(vertex);
        }

        let old_faces = object.faces.clone();
        object.vertices = vertices;

        let mut faces = Vec::with_capacity(old_faces.len());
        for face in old_faces {
            let mut indices = Vec::with_capacity(face.indices.len());
            let mut valid = true;
            for old_index in &face.indices {
                let Some(Some(new_index)) = remap.get(*old_index) else {
                    valid = false;
                    break;
                };
                indices.push(*new_index);
            }
            let Some(indices) = valid.then(|| compact_merged_face_indices(indices)).flatten()
            else {
                continue;
            };
            let mut face = face;
            face.indices = indices;
            face.uvs = face_uvs_for_indices(object, &face.indices);
            face.auto_uv = true;
            face.surface_points.clear();
            face.surface_segments.clear();
            faces.push(face);
        }

        let object_changed = object.vertices.len() != old_vertex_len || faces.len() != old_face_len;
        object.faces = faces;
        changed |= object_changed;
    }
    if changed {
        map.selected_geometry_vertices.clear();
        map.selected_geometry_faces.clear();
    }
    changed
}

fn sanitize_geometry_selection(map: &mut Map) {
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

fn ensure_geometry_object_selected(map: &mut Map, object_id: Uuid) {
    if !map.selected_geometry_objects.contains(&object_id) {
        map.selected_geometry_objects.push(object_id);
    }
}

fn remove_geometry_object_selection(map: &mut Map, object_id: Uuid) {
    map.selected_geometry_objects.retain(|id| *id != object_id);
    map.selected_geometry_faces
        .retain(|(selected_object_id, _)| *selected_object_id != object_id);
    map.selected_geometry_vertices
        .retain(|(selected_object_id, _)| *selected_object_id != object_id);
}

fn add_geometry_face_selection(map: &mut Map, object_id: Uuid, face_index: usize) {
    ensure_geometry_object_selected(map, object_id);
    let selection = (object_id, face_index);
    if !map.selected_geometry_faces.contains(&selection) {
        map.selected_geometry_faces.push(selection);
    }
}

fn remove_geometry_face_selection(map: &mut Map, object_id: Uuid, face_index: usize) {
    map.selected_geometry_faces
        .retain(|selected| *selected != (object_id, face_index));
}

fn add_geometry_vertex_selection(map: &mut Map, object_id: Uuid, vertex_index: usize) {
    ensure_geometry_object_selected(map, object_id);
    let selection = (object_id, vertex_index);
    if !map.selected_geometry_vertices.contains(&selection) {
        map.selected_geometry_vertices.push(selection);
    }
}

fn remove_geometry_vertex_selection(map: &mut Map, object_id: Uuid, vertex_index: usize) {
    map.selected_geometry_vertices
        .retain(|selected| *selected != (object_id, vertex_index));
}

fn add_geometry_edge_selection(map: &mut Map, object_id: Uuid, a_index: usize, b_index: usize) {
    ensure_geometry_object_selected(map, object_id);
    let a_selection = (object_id, a_index);
    let b_selection = (object_id, b_index);
    if !map.selected_geometry_vertices.contains(&a_selection) {
        map.selected_geometry_vertices.push(a_selection);
    }
    if !map.selected_geometry_vertices.contains(&b_selection) {
        map.selected_geometry_vertices.push(b_selection);
    }
}

fn remove_geometry_edge_selection(map: &mut Map, object_id: Uuid, a_index: usize, b_index: usize) {
    map.selected_geometry_vertices
        .retain(|selected| *selected != (object_id, a_index) && *selected != (object_id, b_index));
}

fn face_edge_position(face: &rusterix::GeometryFace, edge: (usize, usize)) -> Option<usize> {
    if face.indices.len() < 2 {
        return None;
    }
    let normalized = normalized_edge(edge.0, edge.1);
    (0..face.indices.len()).find(|index| {
        normalized_edge(
            face.indices[*index],
            face.indices[(*index + 1) % face.indices.len()],
        ) == normalized
    })
}

fn selected_geometry_edges_for_object(
    map: &Map,
    object: &rusterix::GeometryObject,
) -> Vec<(usize, usize)> {
    let selected = map
        .selected_geometry_vertices
        .iter()
        .filter_map(|(object_id, vertex_index)| (*object_id == object.id).then_some(*vertex_index))
        .collect::<BTreeSet<_>>();
    if selected.len() < 2 {
        return Vec::new();
    }

    let mut edges = BTreeSet::new();
    for face in &object.faces {
        if face.indices.len() < 2 {
            continue;
        }
        for index in 0..face.indices.len() {
            let a = face.indices[index];
            let b = face.indices[(index + 1) % face.indices.len()];
            if selected.contains(&a) && selected.contains(&b) {
                edges.insert(normalized_edge(a, b));
            }
        }
    }
    edges.into_iter().collect()
}

fn select_geometry_edge_loops(map: &mut Map) -> bool {
    if map.selected_geometry_vertices.is_empty() {
        return false;
    }

    let objects = map.geometry_objects.clone();
    let mut selected_vertices = BTreeSet::new();
    let mut changed = false;

    for object in &objects {
        let mut queue = selected_geometry_edges_for_object(map, object);
        if queue.is_empty() {
            continue;
        }

        let mut visited = BTreeSet::new();
        while let Some(edge) = queue.pop() {
            let edge = normalized_edge(edge.0, edge.1);
            if !visited.insert(edge) {
                continue;
            }
            selected_vertices.insert((object.id, edge.0));
            selected_vertices.insert((object.id, edge.1));

            for face in &object.faces {
                if face.indices.len() != 4 {
                    continue;
                }
                let Some(edge_position) = face_edge_position(face, edge) else {
                    continue;
                };
                let opposite = normalized_edge(
                    face.indices[(edge_position + 2) % 4],
                    face.indices[(edge_position + 3) % 4],
                );
                if !visited.contains(&opposite) {
                    queue.push(opposite);
                }
            }
        }
        changed = true;
    }

    if changed {
        map.selected_geometry_faces.clear();
        map.selected_geometry_vertices = selected_vertices.into_iter().collect();
        sanitize_geometry_selection(map);
    }
    changed
}

impl Tool for GeometryTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Object Tool"),
            hud: Hud::new(HudMode::Selection),
            undo_map: None,
            drag: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_object")
    }

    fn icon_name(&self) -> String {
        "cube".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('G')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                server_ctx.curr_map_tool_type = MapToolType::Selection;
                server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.geometry_selection_mode = 0;
                    map.selected_vertices.clear();
                    map.selected_linedefs.clear();
                    map.selected_sectors.clear();
                    map.selected_entity_item = None;
                    map.selected_geometry_vertices.clear();
                    map.selected_geometry_faces.clear();
                    map.selected_geometry_surface_points.clear();
                    map.selected_geometry_surface_segments.clear();
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));
                true
            }
            DeActivate => {
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                self.undo_map = None;
                self.drag = None;
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(coord) => {
                if self
                    .hud
                    .clicked(coord.x, coord.y, map, _ui, ctx, server_ctx)
                {
                    return None;
                }

                self.undo_map = None;
                self.drag = None;

                if let Some((axis, gizmo_kind)) = server_ctx.geo_hit.and_then(|hit| match hit {
                    GeoId::Gizmo(axis_id) => gizmo_axis(axis_id).zip(gizmo_kind(axis_id)),
                    _ => None,
                }) {
                    let Some(object_id) = map.selected_geometry_objects.first().copied() else {
                        return None;
                    };
                    let Some(object) = map
                        .geometry_objects
                        .iter()
                        .find(|object| object.id == object_id)
                    else {
                        return None;
                    };
                    let step = ServerContext::edit_grid_step(map.subdivisions);
                    let bounds = geometry_bounds(&object.vertices);
                    let selected_indices = match gizmo_kind {
                        GizmoDragKind::Move => {
                            let selection_mode = geometry_selection_mode(map);
                            match selection_mode {
                                GeometrySelectionMode::Object => Vec::new(),
                                GeometrySelectionMode::Face
                                | GeometrySelectionMode::Vertex
                                | GeometrySelectionMode::Edge => {
                                    selected_geometry_vertex_indices(map, object_id)
                                }
                            }
                        }
                        GizmoDragKind::Resize { component, sign } => bounds
                            .map(|bounds| {
                                bound_vertex_indices(
                                    &object.vertices,
                                    bounds,
                                    component,
                                    sign,
                                    step * 0.1,
                                )
                            })
                            .unwrap_or_default(),
                    };
                    let vertex_indices = (!selected_indices.is_empty()).then_some(selected_indices);
                    let start_hit = if matches!(gizmo_kind, GizmoDragKind::Resize { .. }) {
                        server_ctx.geo_hit_pos
                    } else if axis.y.abs() < 0.5 {
                        let resting_y = bounds
                            .map(|(min, _)| min.y)
                            .unwrap_or(server_ctx.geo_hit_pos.y);
                        Vec3::new(
                            server_ctx.geo_hit_pos.x,
                            resting_y,
                            server_ctx.geo_hit_pos.z,
                        )
                    } else {
                        server_ctx.geo_hit_pos
                    };
                    self.undo_map = Some(map.clone());
                    self.drag = Some(GeometryDrag {
                        object_id,
                        start_hit,
                        start_vertices: object.vertices.clone(),
                        start_transform: object.transform,
                        vertex_indices,
                        axis: Some(axis),
                        gizmo_kind: Some(gizmo_kind),
                        start_plane_hit: None,
                        changed: false,
                    });
                    return None;
                }

                let Some(GeoId::GeometryObject(object_id)) = server_ctx.geo_hit else {
                    if !_ui.shift && !_ui.alt {
                        map.selected_geometry_objects.clear();
                        map.selected_geometry_vertices.clear();
                        map.selected_geometry_faces.clear();
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                        RUSTERIX.write().unwrap().set_overlay_dirty();
                    }
                    return None;
                };

                let Some(object) = map
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == object_id)
                else {
                    return None;
                };

                let start_vertices = object.vertices.clone();
                let start_transform = object.transform;
                let selection_mode = geometry_selection_mode(map);
                let selected_face = match selection_mode {
                    GeometrySelectionMode::Face => server_ctx
                        .hover_ray_origin_3d
                        .zip(server_ctx.hover_ray_dir_3d)
                        .and_then(|(ray_origin, ray_dir)| {
                            closest_geometry_face_from_ray(object, ray_origin, ray_dir)
                        })
                        .or_else(|| closest_geometry_face(object, server_ctx.geo_hit_pos)),
                    _ => None,
                };
                let selected_vertex = match selection_mode {
                    GeometrySelectionMode::Vertex => {
                        closest_geometry_vertex(object, server_ctx.geo_hit_pos)
                    }
                    _ => None,
                };
                let selected_edge = match selection_mode {
                    GeometrySelectionMode::Edge => {
                        closest_geometry_edge(object, server_ctx.geo_hit_pos)
                    }
                    _ => None,
                };
                let vertex_indices = match selection_mode {
                    GeometrySelectionMode::Object => None,
                    GeometrySelectionMode::Face => selected_face.and_then(|face_index| {
                        object.faces.get(face_index).map(|face| {
                            face.indices
                                .iter()
                                .copied()
                                .filter(|index| *index < start_vertices.len())
                                .collect::<Vec<_>>()
                        })
                    }),
                    GeometrySelectionMode::Vertex => {
                        selected_vertex.map(|vertex_index| vec![vertex_index])
                    }
                    GeometrySelectionMode::Edge => {
                        selected_edge.map(|(a_index, b_index)| vec![a_index, b_index])
                    }
                };
                if _ui.shift || _ui.alt {
                    match selection_mode {
                        GeometrySelectionMode::Object => {
                            if _ui.shift {
                                ensure_geometry_object_selected(map, object_id);
                            } else {
                                remove_geometry_object_selection(map, object_id);
                            }
                        }
                        GeometrySelectionMode::Face => {
                            if let Some(face_index) = selected_face {
                                if _ui.shift {
                                    add_geometry_face_selection(map, object_id, face_index);
                                } else {
                                    remove_geometry_face_selection(map, object_id, face_index);
                                }
                            }
                        }
                        GeometrySelectionMode::Vertex => {
                            if let Some(vertex_index) = selected_vertex {
                                if _ui.shift {
                                    add_geometry_vertex_selection(map, object_id, vertex_index);
                                } else {
                                    remove_geometry_vertex_selection(map, object_id, vertex_index);
                                }
                            }
                        }
                        GeometrySelectionMode::Edge => {
                            if let Some((a_index, b_index)) = selected_edge {
                                if _ui.shift {
                                    add_geometry_edge_selection(map, object_id, a_index, b_index);
                                } else {
                                    remove_geometry_edge_selection(
                                        map, object_id, a_index, b_index,
                                    );
                                }
                            }
                        }
                    }
                    sanitize_geometry_selection(map);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    return None;
                } else if selection_mode == GeometrySelectionMode::Object
                    || vertex_indices.is_none()
                {
                    map.clear_selection();
                    map.selected_geometry_objects.push(object_id);
                } else {
                    map.clear_selection();
                    map.selected_geometry_objects.push(object_id);
                    if let Some(face_index) = selected_face {
                        map.selected_geometry_faces.push((object_id, face_index));
                    }
                    if let Some(vertex_index) = selected_vertex {
                        map.selected_geometry_vertices
                            .push((object_id, vertex_index));
                    }
                    if let Some((a_index, b_index)) = selected_edge {
                        map.selected_geometry_vertices.push((object_id, a_index));
                        map.selected_geometry_vertices.push((object_id, b_index));
                    }
                }
                if selection_mode != GeometrySelectionMode::Face {
                    self.undo_map = Some(map.clone());
                    self.drag = Some(GeometryDrag {
                        object_id,
                        start_hit: server_ctx.geo_hit_pos,
                        start_vertices,
                        start_transform,
                        vertex_indices: vertex_indices.clone(),
                        axis: None,
                        gizmo_kind: None,
                        start_plane_hit: Some(server_ctx.geo_hit_pos),
                        changed: false,
                    });
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Map Selection Changed"),
                    TheValue::Empty,
                ));
                RUSTERIX.write().unwrap().set_overlay_dirty();
                None
            }
            MapDragged(_coord) => {
                let Some(drag) = self.drag.as_mut() else {
                    return None;
                };
                let object_id = drag.object_id;
                let mut next_axis_anchor = None;

                let delta = if let Some(axis) = drag.axis {
                    let Some(ray_origin) = server_ctx.hover_ray_origin_3d else {
                        return None;
                    };
                    let Some(ray_dir) = server_ctx.hover_ray_dir_3d else {
                        return None;
                    };
                    let anchor = if let Some(anchor) = drag.start_plane_hit {
                        anchor
                    } else {
                        let Some(anchor) =
                            closest_point_on_axis_to_ray(drag.start_hit, axis, ray_origin, ray_dir)
                        else {
                            return None;
                        };
                        drag.start_plane_hit = Some(anchor);
                        return None;
                    };
                    let Some(hit) =
                        closest_point_on_axis_to_ray(drag.start_hit, axis, ray_origin, ray_dir)
                    else {
                        return None;
                    };
                    let raw_amount = (hit - anchor).dot(axis);
                    let step = ServerContext::edit_grid_step(map.subdivisions);
                    let amount = snap_drag_step(raw_amount, step);
                    if amount.abs() > 0.0 {
                        next_axis_anchor = Some(anchor + axis * amount);
                    }
                    axis * amount
                } else {
                    if server_ctx.geo_hit.is_none() {
                        return None;
                    }
                    server_ctx.geo_hit_pos - drag.start_hit
                };
                if delta.magnitude_squared() <= 0.0001 {
                    return None;
                }

                let old_bbox = bbox_for_vertices(&drag.start_vertices, drag.start_transform);
                let Some(object) = map
                    .geometry_objects
                    .iter_mut()
                    .find(|object| object.id == object_id)
                else {
                    return None;
                };

                let step = ServerContext::edit_grid_step(map.subdivisions);
                let snapped_delta = if drag.axis.is_some() {
                    delta
                } else {
                    Vec3::new(
                        (delta.x / step).round() * step,
                        0.0,
                        (delta.z / step).round() * step,
                    )
                };
                if let Some(GizmoDragKind::Resize { component, .. }) = drag.gizmo_kind {
                    let Some((min, max)) = geometry_bounds(&drag.start_vertices) else {
                        return None;
                    };
                    let size = vec_component(max - min, component);
                    let amount = vec_component(snapped_delta, component).abs();
                    let expands = vec_component(snapped_delta, component)
                        * drag
                            .axis
                            .map(|axis| vec_component(axis, component))
                            .unwrap_or(0.0)
                        > 0.0;
                    if !expands && size - amount < step.max(0.05) {
                        return None;
                    }
                }
                if let Some(indices) = &drag.vertex_indices {
                    for vertex_index in indices {
                        let Some(vertex) = object.vertices.get_mut(*vertex_index) else {
                            continue;
                        };
                        let Some(start) = drag.start_vertices.get(*vertex_index) else {
                            continue;
                        };
                        *vertex = *start + snapped_delta;
                    }
                } else {
                    object.transform = translated_transform(drag.start_transform, snapped_delta);
                }
                let new_bbox = object.bbox();
                if let Some(anchor) = next_axis_anchor {
                    drag.start_plane_hit = Some(anchor);
                    drag.start_vertices = object.vertices.clone();
                    drag.start_transform = object.transform;
                }
                drag.changed = true;
                let mut dirty_chunks = FxHashSet::default();
                if let Some(bbox) = old_bbox {
                    add_bbox_dirty_chunks(bbox, &mut dirty_chunks);
                }
                if let Some(bbox) = new_bbox {
                    add_bbox_dirty_chunks(bbox, &mut dirty_chunks);
                }
                if !dirty_chunks.is_empty() {
                    crate::utils::editor_scene_replace_incremental_map_update(
                        map.clone(),
                        dirty_chunks.into_iter().collect(),
                    );
                }
                RUSTERIX.write().unwrap().set_overlay_dirty();
                None
            }
            MapUp(_) => {
                let Some(drag) = self.drag.take() else {
                    self.undo_map = None;
                    return None;
                };
                let undo_map = self.undo_map.take();
                if drag.changed
                    && let Some(old_map) = undo_map
                {
                    if drag.vertex_indices.is_some()
                        && drag.axis.is_none()
                        && drag.gizmo_kind.is_none()
                    {
                        let step = ServerContext::edit_grid_step(map.subdivisions);
                        let mut topology_changed = auto_merge_overlapping_selected_geometry_vertices(
                            map,
                            (step * 0.01).max(0.0001),
                        );
                        topology_changed |= normalize_geometry_faces_for_object(map, drag.object_id);
                        if topology_changed {
                            sanitize_geometry_selection(map);
                            refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                        }
                    }
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }
                None
            }
            MapEscape => {
                if let Some(old_map) = self.undo_map.take() {
                    *map = old_map;
                }
                self.drag = None;
                None
            }
            MapDelete => {
                if map.selected_geometry_objects.is_empty() {
                    return None;
                }
                let old_map = map.clone();

                if delete_selected_geometry_faces(map) || delete_selected_geometry_vertices(map) {
                    sanitize_geometry_selection(map);
                    refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                let selected = map.selected_geometry_objects.clone();
                map.geometry_objects
                    .retain(|object| !selected.contains(&object.id));
                map.selected_geometry_objects.clear();
                map.selected_geometry_vertices.clear();
                map.selected_geometry_faces.clear();
                refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(old_map),
                    Box::new(map.clone()),
                ))
            }
            MapKey(key) => {
                let step = ServerContext::edit_grid_step(map.subdivisions);
                if matches!(key, 'r' | 'R')
                    && !map.selected_geometry_objects.is_empty()
                    && map.selected_geometry_faces.is_empty()
                    && map.selected_geometry_vertices.is_empty()
                    && map.selected_geometry_surface_points.is_empty()
                    && map.selected_geometry_surface_segments.is_empty()
                {
                    let old_map = map.clone();
                    let quarter_turns = if key == 'R' { -1 } else { 1 };
                    if !rotate_selected_geometry_objects_y(map, quarter_turns) {
                        return None;
                    }
                    refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                if matches!(key, 'x' | 'X') && !map.selected_geometry_vertices.is_empty() {
                    let old_map = map.clone();
                    if !split_selected_geometry_edges(map) {
                        return None;
                    }
                    sanitize_geometry_selection(map);
                    refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                if matches!(key, 'm' | 'M') && !map.selected_geometry_vertices.is_empty() {
                    let old_map = map.clone();
                    if !merge_selected_geometry_vertices(map) {
                        return None;
                    }
                    normalize_selected_geometry_object_faces(map);
                    sanitize_geometry_selection(map);
                    refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                if matches!(key, 'f' | 'F') && !map.selected_geometry_vertices.is_empty() {
                    let old_map = map.clone();
                    if !fill_selected_geometry_vertices(map) {
                        return None;
                    }
                    normalize_selected_geometry_object_faces(map);
                    sanitize_geometry_selection(map);
                    refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                if matches!(key, 'l' | 'L') && !map.selected_geometry_vertices.is_empty() {
                    let old_map = map.clone();
                    if !select_geometry_edge_loops(map) {
                        return None;
                    }
                    refresh_geometry_topology_edit(None, map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                if matches!(key, 't' | 'T')
                    && (!map.selected_geometry_faces.is_empty()
                        || !map.selected_geometry_objects.is_empty())
                {
                    let source = server_ctx
                        .curr_tile_id
                        .map(PixelSource::TileId)
                        .or_else(|| get_source(_ui, server_ctx))?;
                    let old_map = map.clone();
                    if !apply_tile_to_selected_geometry_faces(map, source) {
                        return None;
                    }
                    sanitize_geometry_selection(map);
                    RUSTERIX.write().unwrap().set_dirty();
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                let vertex_move = match key {
                    ']' | '}'
                        if !map.selected_geometry_vertices.is_empty()
                            || !map.selected_geometry_faces.is_empty() =>
                    {
                        Some(Vec3::new(0.0, step, 0.0))
                    }
                    '[' | '{'
                        if !map.selected_geometry_vertices.is_empty()
                            || !map.selected_geometry_faces.is_empty() =>
                    {
                        Some(Vec3::new(0.0, -step, 0.0))
                    }
                    _ => None,
                };
                if let Some(delta) = vertex_move {
                    let old_map = map.clone();
                    if !move_selected_geometry_vertices(map, delta) {
                        return None;
                    }
                    normalize_selected_geometry_object_faces(map);
                    sanitize_geometry_selection(map);
                    refresh_geometry_topology_edit(Some(&old_map), map, ctx);
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                let face_push = match key {
                    '+' | '=' if !map.selected_geometry_faces.is_empty() => Some(step),
                    '-' | '_' if !map.selected_geometry_faces.is_empty() => Some(-step),
                    _ => None,
                };
                if let Some(amount) = face_push {
                    let old_map = map.clone();
                    if !move_selected_geometry_faces_along_normals(map, amount) {
                        return None;
                    }
                    sanitize_geometry_selection(map);
                    RUSTERIX.write().unwrap().set_dirty();
                    RUSTERIX.write().unwrap().set_overlay_dirty();
                    return Some(ProjectUndoAtom::MapEdit(
                        server_ctx.pc,
                        Box::new(old_map),
                        Box::new(map.clone()),
                    ));
                }

                let delta_size = match key {
                    '+' | '=' => Some(Vec3::new(step, 0.0, step)),
                    '-' | '_' => Some(Vec3::new(-step, 0.0, -step)),
                    ']' | '}' => Some(Vec3::new(0.0, step, 0.0)),
                    '[' | '{' => Some(Vec3::new(0.0, -step, 0.0)),
                    _ => None,
                }?;

                let old_map = map.clone();
                if !resize_selected_geometry(map, delta_size) {
                    return None;
                }
                sanitize_geometry_selection(map);
                RUSTERIX.write().unwrap().set_overlay_dirty();
                Some(ProjectUndoAtom::MapEdit(
                    server_ctx.pc,
                    Box::new(old_map),
                    Box::new(map.clone()),
                ))
            }
            _ => None,
        }
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        assets: &Assets,
    ) {
        let id = map
            .selected_geometry_objects
            .first()
            .map(|id| id.as_u128() as u32);
        self.hud.draw(buffer, map, ctx, server_ctx, id, assets);
    }
}
