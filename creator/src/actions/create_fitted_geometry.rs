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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FittedConstruction {
    Solid,
    Barred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FittedLeaves {
    Single,
    Split,
}

#[derive(Clone, Copy, Debug)]
struct FittedOptions {
    construction: FittedConstruction,
    leaves: FittedLeaves,
    depth: f32,
    bar_width: f32,
    bar_spacing: f32,
    rail_width: f32,
}

impl Default for FittedOptions {
    fn default() -> Self {
        Self {
            construction: FittedConstruction::Solid,
            leaves: FittedLeaves::Single,
            depth: 1.0,
            bar_width: 0.08,
            bar_spacing: 0.35,
            rail_width: 0.1,
        }
    }
}

#[derive(Clone)]
struct FittedFrame {
    center: Vec3<f32>,
    depth_axis: Vec3<f32>,
    horizontal_axis: Vec3<f32>,
    vertical_axis: Vec3<f32>,
    polygon: Vec<Vec2<f32>>,
    original_depth: f32,
}

struct FittedSelection {
    object_index: usize,
    band_faces: BTreeSet<usize>,
    loops: [Vec<usize>; 2],
}

struct FittedInput {
    source: rusterix::GeometryObject,
    source_face: Option<rusterix::GeometryFace>,
    frame: FittedFrame,
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

fn selected_plane_normal(vertices: &[Vec3<f32>], selected: &BTreeSet<usize>) -> Option<Vec3<f32>> {
    if selected.len() < 3 {
        return None;
    }
    let indices = selected.iter().copied().collect::<Vec<_>>();
    let mut widest = None;
    for a in 0..indices.len() {
        for b in a + 1..indices.len() {
            let delta = *vertices.get(indices[b])? - *vertices.get(indices[a])?;
            let distance = delta.magnitude_squared();
            if widest.is_none_or(|(_, _, best)| distance > best) {
                widest = Some((indices[a], indices[b], distance));
            }
        }
    }
    let (a_index, b_index, _) = widest?;
    let a = *vertices.get(a_index)?;
    let direction = *vertices.get(b_index)? - a;
    indices
        .into_iter()
        .filter(|index| *index != a_index && *index != b_index)
        .filter_map(|index| {
            let cross = direction.cross(*vertices.get(index)? - a);
            Some((cross.magnitude_squared(), cross.try_normalized()?))
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, normal)| normal)
}

fn ordered_edge_chains(edges: &BTreeSet<Edge>) -> Option<Vec<Vec<usize>>> {
    let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
    for (a, b) in edges {
        adjacency.entry(*a).or_default().push(*b);
        adjacency.entry(*b).or_default().push(*a);
    }
    if adjacency.is_empty() || adjacency.values().any(|neighbors| neighbors.len() > 2) {
        return None;
    }

    let mut remaining = edges.clone();
    let mut chains = Vec::new();
    while let Some(first_edge) = remaining.first().copied() {
        let start = adjacency
            .iter()
            .find_map(|(vertex, neighbors)| {
                (neighbors.len() == 1
                    && remaining.contains(&normalized_edge(*vertex, neighbors[0])))
                .then_some(*vertex)
            })
            .unwrap_or(first_edge.0);
        let mut chain = vec![start];
        let mut current = start;
        let mut previous = None;
        loop {
            let next = adjacency.get(&current)?.iter().copied().find(|neighbor| {
                Some(*neighbor) != previous
                    && remaining.contains(&normalized_edge(current, *neighbor))
            });
            let Some(next) = next else {
                break;
            };
            remaining.remove(&normalized_edge(current, next));
            previous = Some(current);
            current = next;
            if current == start {
                break;
            }
            chain.push(current);
        }
        if chain.len() < 2 {
            return None;
        }
        chains.push(chain);
    }
    Some(chains)
}

fn fitted_selection_from_band(
    object_index: usize,
    object: &rusterix::GeometryObject,
    band_faces: BTreeSet<usize>,
) -> Result<FittedSelection, FittedGeometryError> {
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

fn infer_reveal_band_from_rim(
    object_index: usize,
    object: &rusterix::GeometryObject,
    selected: &BTreeSet<usize>,
) -> Result<FittedSelection, FittedGeometryError> {
    let selected_edges = object
        .faces
        .iter()
        .flat_map(face_edges)
        .filter(|(a, b)| selected.contains(a) && selected.contains(b))
        .collect::<BTreeSet<_>>();
    let rim_normal =
        selected_plane_normal(&object.vertices, selected).ok_or(FittedGeometryError::Contours)?;

    // A ground-level doorway is an open profile embedded in the larger closed
    // boundary of the wall face. Collect every side/reveal face adjacent to the
    // selected boundary, separate those faces into strips, then choose the
    // shortest valid strip. This selects the doorway reveal rather than the
    // building's outer shell.
    let mut candidate_faces = BTreeSet::new();
    for rim_edge in &selected_edges {
        if let Some((face_index, _)) = object
            .faces
            .iter()
            .enumerate()
            .filter(|(_, face)| face_edges(face).any(|edge| edge == *rim_edge))
            .filter_map(|(face_index, face)| {
                let alignment = local_face_normal(&object.vertices, face)?
                    .dot(rim_normal)
                    .abs();
                (alignment < 0.8).then_some((face_index, alignment))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            candidate_faces.insert(face_index);
        }
    }

    let face_edge_sets = candidate_faces
        .iter()
        .map(|face_index| {
            (
                *face_index,
                face_edges(&object.faces[*face_index]).collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut remaining_faces = candidate_faces;
    let mut valid_bands = Vec::<(f32, FittedSelection)>::new();
    while let Some(seed) = remaining_faces.first().copied() {
        let mut band_faces = BTreeSet::new();
        let mut queue = VecDeque::from([seed]);
        while let Some(face_index) = queue.pop_front() {
            if !remaining_faces.remove(&face_index) {
                continue;
            }
            band_faces.insert(face_index);
            let edges = &face_edge_sets[&face_index];
            let neighbors = remaining_faces
                .iter()
                .copied()
                .filter(|neighbor| {
                    face_edge_sets[neighbor]
                        .iter()
                        .any(|edge| edges.contains(edge))
                })
                .collect::<Vec<_>>();
            queue.extend(neighbors);
        }

        let near_edges = selected_edges
            .iter()
            .copied()
            .filter(|edge| {
                band_faces
                    .iter()
                    .any(|face_index| face_edge_sets[face_index].contains(edge))
            })
            .collect::<BTreeSet<_>>();
        let Some(mut near_chains) = ordered_edge_chains(&near_edges) else {
            continue;
        };
        if near_chains.len() != 1 || near_chains[0].len() < 3 {
            continue;
        }
        let near = near_chains.pop().unwrap();

        let mut edge_counts = BTreeMap::<Edge, usize>::new();
        for face_index in &band_faces {
            for edge in &face_edge_sets[face_index] {
                *edge_counts.entry(*edge).or_default() += 1;
            }
        }
        let far_edges = edge_counts
            .into_iter()
            .filter_map(|(edge, count)| {
                (count == 1 && !selected.contains(&edge.0) && !selected.contains(&edge.1))
                    .then_some(edge)
            })
            .collect::<BTreeSet<_>>();
        let Some(mut far_chains) = ordered_edge_chains(&far_edges) else {
            continue;
        };
        if far_chains.len() != 1 || far_chains[0].len() < 3 {
            continue;
        }
        let far = far_chains.pop().unwrap();
        let length = near_edges
            .iter()
            .map(|(a, b)| (object.vertices[*a] - object.vertices[*b]).magnitude())
            .sum::<f32>();
        valid_bands.push((
            length,
            FittedSelection {
                object_index,
                band_faces,
                loops: [near, far],
            },
        ));
    }

    valid_bands
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, selection)| selection)
        .ok_or(FittedGeometryError::Contours)
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
    if selected.len() < 3 {
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

    // Preserve the old C-then-L workflow when the whole reveal band is selected.
    // A single C-selected rim falls through to automatic reveal inference.
    let band_faces = object
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            (face.indices.len() >= 3 && face.indices.iter().all(|index| selected.contains(index)))
                .then_some(face_index)
        })
        .collect::<BTreeSet<_>>();
    if !band_faces.is_empty()
        && let Ok(selection) = fitted_selection_from_band(object_index, object, band_faces)
    {
        return Ok(selection);
    }

    infer_reveal_band_from_rim(object_index, object, &selected)
}

fn loop_center(vertices: &[Vec3<f32>], indices: &[usize]) -> Option<Vec3<f32>> {
    let mut center = Vec3::zero();
    for index in indices {
        center += *vertices.get(*index)?;
    }
    Some(center / indices.len() as f32)
}

fn polygon_area(polygon: &[Vec2<f32>]) -> f32 {
    polygon
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = polygon[(index + 1) % polygon.len()];
            point.x * next.y - next.x * point.y
        })
        .sum::<f32>()
        * 0.5
}

fn fitted_frame(
    source: &rusterix::GeometryObject,
    loops: &[Vec<usize>; 2],
) -> Result<FittedFrame, FittedGeometryError> {
    let centers = [
        loop_center(&source.vertices, &loops[0]).ok_or(FittedGeometryError::Contours)?,
        loop_center(&source.vertices, &loops[1]).ok_or(FittedGeometryError::Contours)?,
    ];
    let delta = centers[1] - centers[0];
    let original_depth = delta.magnitude();
    let depth_axis = delta
        .try_normalized()
        .ok_or(FittedGeometryError::Contours)?;
    let mut vertical_axis = Vec3::unit_y() - depth_axis * depth_axis.dot(Vec3::unit_y());
    if vertical_axis.magnitude_squared() <= 1e-6 {
        vertical_axis = Vec3::unit_z() - depth_axis * depth_axis.dot(Vec3::unit_z());
    }
    let vertical_axis = vertical_axis
        .try_normalized()
        .ok_or(FittedGeometryError::Contours)?;
    let horizontal_axis = vertical_axis
        .cross(depth_axis)
        .try_normalized()
        .ok_or(FittedGeometryError::Contours)?;
    let center = (centers[0] + centers[1]) * 0.5;
    let mut polygon = loops[0]
        .iter()
        .filter_map(|index| source.vertices.get(*index).copied())
        .map(|point| {
            let local = point - center;
            Vec2::new(local.dot(horizontal_axis), local.dot(vertical_axis))
        })
        .collect::<Vec<_>>();
    if polygon.len() != loops[0].len() || polygon.len() < 3 {
        return Err(FittedGeometryError::Contours);
    }
    if polygon_area(&polygon) < 0.0 {
        polygon.reverse();
    }
    Ok(FittedFrame {
        center,
        depth_axis,
        horizontal_axis,
        vertical_axis,
        polygon,
        original_depth,
    })
}

fn geometry_fitted_input(map: &Map) -> Result<FittedInput, FittedGeometryError> {
    let selection = fitted_selection(map)?;
    let source = map.geometry_objects[selection.object_index].clone();
    let frame = fitted_frame(&source, &selection.loops)?;
    let source_face = selection
        .band_faces
        .first()
        .and_then(|face_index| source.faces.get(*face_index))
        .cloned();
    Ok(FittedInput {
        source,
        source_face,
        frame,
    })
}

fn wall_opening_fitted_input(map: &Map) -> Result<FittedInput, FittedGeometryError> {
    let assembly_id = map
        .selected_wall_assembly
        .ok_or(FittedGeometryError::Selection)?;
    let span_id = map
        .selected_wall_spans
        .first()
        .copied()
        .ok_or(FittedGeometryError::Selection)?;
    let opening_id = map
        .selected_wall_opening
        .ok_or(FittedGeometryError::Selection)?;
    let assembly = map
        .wall_assembly(assembly_id)
        .ok_or(FittedGeometryError::Selection)?;
    let span = assembly
        .span(span_id)
        .ok_or(FittedGeometryError::Selection)?;
    let opening = assembly
        .opening(span_id, opening_id)
        .ok_or(FittedGeometryError::Selection)?;
    let style = span.style_override.as_ref().unwrap_or(&assembly.style);
    let length = assembly
        .span_length(span_id)
        .ok_or(FittedGeometryError::Contours)?;
    let along = opening.center.clamp(0.0, length);
    let middle_height = opening.bottom + opening.height * 0.5;
    let center = assembly
        .span_point(span_id, Vec2::new(along, middle_height))
        .ok_or(FittedGeometryError::Contours)?;
    let epsilon = (length * 0.01).clamp(0.005, 0.05);
    let before = assembly
        .span_point(
            span_id,
            Vec2::new((along - epsilon).max(0.0), middle_height),
        )
        .ok_or(FittedGeometryError::Contours)?;
    let after = assembly
        .span_point(
            span_id,
            Vec2::new((along + epsilon).min(length), middle_height),
        )
        .ok_or(FittedGeometryError::Contours)?;
    let horizontal_axis = Vec3::new(after.x - before.x, 0.0, after.z - before.z)
        .try_normalized()
        .ok_or(FittedGeometryError::Contours)?;
    let vertical_axis = Vec3::unit_y();
    let depth_axis = horizontal_axis
        .cross(vertical_axis)
        .try_normalized()
        .ok_or(FittedGeometryError::Contours)?;

    let half_width = opening.width * 0.5;
    let half_height = opening.height * 0.5;
    let mut polygon = match opening.shape {
        rusterix::WallOpeningShape::Rectangular => vec![
            Vec2::new(-half_width, -half_height),
            Vec2::new(half_width, -half_height),
            Vec2::new(half_width, half_height),
            Vec2::new(-half_width, half_height),
        ],
        rusterix::WallOpeningShape::Arch => {
            let radius_y = opening.effective_arch_radius();
            let spring = half_height - radius_y;
            let mut points = vec![
                Vec2::new(-half_width, -half_height),
                Vec2::new(half_width, -half_height),
            ];
            points.extend((0..=32).map(|segment| {
                let angle = segment as f32 / 32.0 * std::f32::consts::PI;
                Vec2::new(angle.cos() * half_width, spring + angle.sin() * radius_y)
            }));
            points
        }
    };
    polygon.dedup_by(|left, right| (*left - *right).magnitude_squared() <= 1e-10);
    if polygon.len() >= 2 && (polygon[0] - polygon[polygon.len() - 1]).magnitude_squared() <= 1e-10
    {
        polygon.pop();
    }
    if polygon.len() < 3 {
        return Err(FittedGeometryError::Contours);
    }
    if polygon_area(&polygon) < 0.0 {
        polygon.reverse();
    }

    let mut source_face = new_face(Vec::new(), None);
    source_face.tile = Some(opening.frame.pixel_source(style));
    Ok(FittedInput {
        source: rusterix::GeometryObject::new("Wall Opening"),
        source_face: Some(source_face),
        frame: FittedFrame {
            center,
            depth_axis,
            horizontal_axis,
            vertical_axis,
            polygon,
            original_depth: style.thickness.max(0.01),
        },
    })
}

fn fitted_input(map: &Map) -> Result<FittedInput, FittedGeometryError> {
    if map.selected_wall_opening.is_some() {
        wall_opening_fitted_input(map)
    } else {
        geometry_fitted_input(map)
    }
}

fn fitted_input_is_available(map: &Map) -> bool {
    if map.selected_wall_opening.is_some() {
        let Some(assembly_id) = map.selected_wall_assembly else {
            return false;
        };
        let Some(span_id) = map.selected_wall_spans.first().copied() else {
            return false;
        };
        let Some(opening_id) = map.selected_wall_opening else {
            return false;
        };
        let Some(assembly) = map.wall_assembly(assembly_id) else {
            return false;
        };
        assembly
            .span_length(span_id)
            .is_some_and(|length| length > 1e-5)
            && assembly
                .opening(span_id, opening_id)
                .is_some_and(|opening| opening.width > 1e-5 && opening.height > 1e-5)
    } else {
        fitted_selection(map).is_ok()
    }
}

fn clip_polygon_half_plane(
    polygon: &[Vec2<f32>],
    inside: impl Fn(Vec2<f32>) -> bool,
    intersection: impl Fn(Vec2<f32>, Vec2<f32>) -> Vec2<f32>,
) -> Vec<Vec2<f32>> {
    let mut output = Vec::new();
    let Some(mut previous) = polygon.last().copied() else {
        return output;
    };
    let mut previous_inside = inside(previous);
    for current in polygon.iter().copied() {
        let current_inside = inside(current);
        if current_inside != previous_inside {
            output.push(intersection(previous, current));
        }
        if current_inside {
            output.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }
    output
}

fn clip_polygon_x_min(polygon: &[Vec2<f32>], min_x: f32) -> Vec<Vec2<f32>> {
    clip_polygon_half_plane(
        polygon,
        |point| point.x >= min_x - 1e-5,
        |a, b| {
            let t = (min_x - a.x) / (b.x - a.x);
            a + (b - a) * t
        },
    )
}

fn clip_polygon_x_max(polygon: &[Vec2<f32>], max_x: f32) -> Vec<Vec2<f32>> {
    clip_polygon_half_plane(
        polygon,
        |point| point.x <= max_x + 1e-5,
        |a, b| {
            let t = (max_x - a.x) / (b.x - a.x);
            a + (b - a) * t
        },
    )
}

fn clip_polygon_y_min(polygon: &[Vec2<f32>], min_y: f32) -> Vec<Vec2<f32>> {
    clip_polygon_half_plane(
        polygon,
        |point| point.y >= min_y - 1e-5,
        |a, b| {
            let t = (min_y - a.y) / (b.y - a.y);
            a + (b - a) * t
        },
    )
}

fn clip_polygon_y_max(polygon: &[Vec2<f32>], max_y: f32) -> Vec<Vec2<f32>> {
    clip_polygon_half_plane(
        polygon,
        |point| point.y <= max_y + 1e-5,
        |a, b| {
            let t = (max_y - a.y) / (b.y - a.y);
            a + (b - a) * t
        },
    )
}

fn clip_polygon_rect(polygon: &[Vec2<f32>], min: Vec2<f32>, max: Vec2<f32>) -> Vec<Vec2<f32>> {
    let polygon = clip_polygon_x_min(polygon, min.x);
    let polygon = clip_polygon_x_max(&polygon, max.x);
    let polygon = clip_polygon_y_min(&polygon, min.y);
    clip_polygon_y_max(&polygon, max.y)
}

fn append_extruded_polygon(
    output: &mut rusterix::GeometryObject,
    polygon: &[Vec2<f32>],
    frame: &FittedFrame,
    depth: f32,
    source_face: Option<&rusterix::GeometryFace>,
) -> bool {
    if polygon.len() < 3 || depth <= 1e-5 {
        return false;
    }
    let flat = polygon
        .iter()
        .flat_map(|point| [point.x as f64, point.y as f64])
        .collect::<Vec<_>>();
    let Ok(triangles) = earcut(&flat, &[], 2) else {
        return false;
    };
    if triangles.is_empty() {
        return false;
    }

    let base = output.vertices.len();
    for depth_offset in [-depth * 0.5, depth * 0.5] {
        output.vertices.extend(polygon.iter().map(|point| {
            frame.center
                + frame.horizontal_axis * point.x
                + frame.vertical_axis * point.y
                + frame.depth_axis * depth_offset
        }));
    }
    let count = polygon.len();
    for index in 0..count {
        let next = (index + 1) % count;
        output.faces.push(new_face(
            vec![
                base + index,
                base + next,
                base + count + next,
                base + count + index,
            ],
            source_face,
        ));
    }
    for triangle in triangles.chunks_exact(3) {
        output.faces.push(new_face(
            vec![base + triangle[2], base + triangle[1], base + triangle[0]],
            source_face,
        ));
        output.faces.push(new_face(
            vec![
                base + count + triangle[0],
                base + count + triangle[1],
                base + count + triangle[2],
            ],
            source_face,
        ));
    }
    true
}

fn transformed_vector(object: &rusterix::GeometryObject, vector: Vec3<f32>) -> Vec3<f32> {
    (object.transform_point(vector) - object.transform_point(Vec3::zero()))
        .try_normalized()
        .unwrap_or(vector)
}

fn add_fitted_metadata(
    object: &mut rusterix::GeometryObject,
    source: &rusterix::GeometryObject,
    frame: &FittedFrame,
    leaf: &str,
    hinge_x: f32,
    slide_distance: f32,
    construction: FittedConstruction,
) {
    let motion_axis = transformed_vector(source, frame.horizontal_axis);
    let hinge = source.transform_point(frame.center + frame.horizontal_axis * hinge_x);
    object
        .properties
        .set("fitted_leaf", Value::Str(leaf.to_string()));
    object.properties.set(
        "fitted_construction",
        Value::Str(
            match construction {
                FittedConstruction::Solid => "Solid",
                FittedConstruction::Barred => "Barred",
            }
            .to_string(),
        ),
    );
    object.properties.set(
        "fitted_motion_axis",
        Value::Vec3([motion_axis.x, motion_axis.y, motion_axis.z]),
    );
    object.properties.set(
        "fitted_hinge_pivot",
        Value::Vec3([hinge.x, hinge.y, hinge.z]),
    );
    object
        .properties
        .set("fitted_slide_distance", Value::Float(slide_distance));
}

fn polygon_bounds(polygon: &[Vec2<f32>]) -> Option<(Vec2<f32>, Vec2<f32>)> {
    let mut min = Vec2::broadcast(f32::INFINITY);
    let mut max = Vec2::broadcast(f32::NEG_INFINITY);
    for point in polygon {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    (min.x.is_finite() && min.y.is_finite() && max.x.is_finite() && max.y.is_finite())
        .then_some((min, max))
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
        smoothing_group: source.map_or(0, |face| face.smoothing_group),
    }
}

fn build_fitted_leaf(
    source: &rusterix::GeometryObject,
    source_face: Option<&rusterix::GeometryFace>,
    frame: &FittedFrame,
    polygon: Vec<Vec2<f32>>,
    full_bounds: (Vec2<f32>, Vec2<f32>),
    options: FittedOptions,
    name: &str,
    leaf: &str,
    hinge_x: f32,
) -> Result<rusterix::GeometryObject, FittedGeometryError> {
    if polygon.len() < 3 {
        return Err(FittedGeometryError::Contours);
    }
    let mut fitted = rusterix::GeometryObject::new(name);
    fitted.kind = rusterix::GeometryObjectKind::Prop;
    fitted.transform = source.transform;

    let changed = match options.construction {
        FittedConstruction::Solid => {
            append_extruded_polygon(&mut fitted, &polygon, frame, options.depth, source_face)
        }
        FittedConstruction::Barred => {
            let (full_min, full_max) = full_bounds;
            let width = options.bar_width.max(0.01);
            let spacing = options.bar_spacing.max(width);
            let mut changed = false;

            let mut bar_centers = vec![full_min.x + width * 0.5, full_max.x - width * 0.5];
            let mut x = full_min.x + spacing;
            while x < full_max.x - spacing * 0.25 {
                bar_centers.push(x);
                x += spacing;
            }
            bar_centers.sort_by(|a, b| a.total_cmp(b));
            bar_centers.dedup_by(|a, b| (*a - *b).abs() < width * 0.25);
            for x in bar_centers {
                let bar = clip_polygon_rect(
                    &polygon,
                    Vec2::new(x - width * 0.5, full_min.y),
                    Vec2::new(x + width * 0.5, full_max.y),
                );
                changed |=
                    append_extruded_polygon(&mut fitted, &bar, frame, options.depth, source_face);
            }

            let rail_width = options.rail_width.max(0.01);
            let height = full_max.y - full_min.y;
            for y in [full_min.y + height * 0.22, full_min.y + height * 0.78] {
                let rail = clip_polygon_rect(
                    &polygon,
                    Vec2::new(full_min.x, y - rail_width * 0.5),
                    Vec2::new(full_max.x, y + rail_width * 0.5),
                );
                changed |=
                    append_extruded_polygon(&mut fitted, &rail, frame, options.depth, source_face);
            }
            changed
        }
    };
    if !changed {
        return Err(FittedGeometryError::Contours);
    }

    for face_index in 0..fitted.faces.len() {
        let indices = fitted.faces[face_index].indices.clone();
        fitted.faces[face_index].uvs = face_uvs_for_indices(&fitted, &indices);
    }
    fitted.ensure_face_paint_data();
    add_fitted_metadata(
        &mut fitted,
        source,
        frame,
        leaf,
        hinge_x,
        if leaf == "Single" {
            full_bounds.1.x - full_bounds.0.x
        } else {
            (full_bounds.1.x - full_bounds.0.x) * 0.5
        },
        options.construction,
    );
    Ok(fitted)
}

fn create_fitted_geometry(
    map: &mut Map,
    mut options: FittedOptions,
) -> Result<Vec<Uuid>, FittedGeometryError> {
    let FittedInput {
        source,
        source_face,
        frame,
    } = fitted_input(map)?;
    if !options.depth.is_finite() || options.depth <= 0.0 {
        options.depth = frame.original_depth.max(0.01);
    }
    let full_bounds = polygon_bounds(&frame.polygon).ok_or(FittedGeometryError::Contours)?;
    let split_x = (full_bounds.0.x + full_bounds.1.x) * 0.5;
    let source_face = source_face.as_ref();
    let leaves = match options.leaves {
        FittedLeaves::Single => vec![build_fitted_leaf(
            &source,
            source_face,
            &frame,
            frame.polygon.clone(),
            full_bounds,
            options,
            if options.construction == FittedConstruction::Barred {
                "Fitted Barred Gate"
            } else {
                "Fitted Geometry"
            },
            "Single",
            full_bounds.0.x,
        )?],
        FittedLeaves::Split => vec![
            build_fitted_leaf(
                &source,
                source_face,
                &frame,
                clip_polygon_x_max(&frame.polygon, split_x),
                full_bounds,
                options,
                "Fitted Left Leaf",
                "Left",
                full_bounds.0.x,
            )?,
            build_fitted_leaf(
                &source,
                source_face,
                &frame,
                clip_polygon_x_min(&frame.polygon, split_x),
                full_bounds,
                options,
                "Fitted Right Leaf",
                "Right",
                full_bounds.1.x,
            )?,
        ],
    };

    let fitted_ids = leaves.iter().map(|leaf| leaf.id).collect::<Vec<_>>();
    let fitted_faces = leaves
        .iter()
        .flat_map(|leaf| (0..leaf.faces.len()).map(move |face_index| (leaf.id, face_index)))
        .collect::<Vec<_>>();
    map.geometry_objects.extend(leaves);
    map.clear_selection();
    map.selected_geometry_objects = fitted_ids.clone();
    map.selected_geometry_faces = fitted_faces;
    map.changed = map.changed.wrapping_add(1);
    Ok(fitted_ids)
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
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionFittedConstruction".into(),
            "Construction".into(),
            "Create a solid panel or an iron gate made from bars and rails.".into(),
            vec!["Solid".into(), "Barred".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionFittedLeaves".into(),
            "Leaves".into(),
            "Create one fitted object or two independently moving center-split leaves.".into(),
            vec!["Single".into(), "Center Split".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFittedDepth".into(),
            "Depth".into(),
            "The generated depth, centered between the opening's front and back contours.".into(),
            1.0,
            0.01..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFittedBarWidth".into(),
            "Bar Width".into(),
            "Width of vertical bars when Construction is Barred.".into(),
            0.08,
            0.01..=16.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFittedBarSpacing".into(),
            "Bar Spacing".into(),
            "Center-to-center spacing of vertical bars.".into(),
            0.35,
            0.02..=64.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFittedRailWidth".into(),
            "Rail Width".into(),
            "Width of the two horizontal gate rails.".into(),
            0.1,
            0.01..=16.0,
            false,
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
            && fitted_input_is_available(map)
    }

    fn load_params(&mut self, map: &Map) {
        if let Ok(input) = fitted_input(map) {
            self.nodeui
                .set_f32_value("actionFittedDepth", input.frame.original_depth.max(0.01));
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let previous = map.clone();
        let options = FittedOptions {
            construction: if self
                .nodeui
                .get_i32_value("actionFittedConstruction")
                .unwrap_or(0)
                == 1
            {
                FittedConstruction::Barred
            } else {
                FittedConstruction::Solid
            },
            leaves: if self.nodeui.get_i32_value("actionFittedLeaves").unwrap_or(0) == 1 {
                FittedLeaves::Split
            } else {
                FittedLeaves::Single
            },
            depth: self
                .nodeui
                .get_f32_value("actionFittedDepth")
                .unwrap_or(1.0),
            bar_width: self
                .nodeui
                .get_f32_value("actionFittedBarWidth")
                .unwrap_or(0.08),
            bar_spacing: self
                .nodeui
                .get_f32_value("actionFittedBarSpacing")
                .unwrap_or(0.35),
            rail_width: self
                .nodeui
                .get_f32_value("actionFittedRailWidth")
                .unwrap_or(0.1),
        };
        if create_fitted_geometry(map, options).is_err() {
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

    fn selected_wall_opening_map(shape: rusterix::WallOpeningShape) -> Map {
        let mut map = Map::new();
        let mut assembly = rusterix::WallAssembly::new("Wall");
        assembly.style.thickness = 0.4;
        let start = assembly.add_node(Vec3::new(0.0, 0.0, 0.0));
        let end = assembly.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span_id = assembly.add_span(start, end).unwrap();
        let opening_id = assembly
            .add_opening(span_id, Vec2::new(1.0, 0.0), Vec2::new(3.0, 2.5), shape)
            .unwrap();
        let assembly_id = assembly.id;
        map.wall_assemblies.push(assembly);
        map.selected_wall_assembly = Some(assembly_id);
        map.selected_wall_spans.push(span_id);
        map.selected_wall_opening = Some(opening_id);
        map
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
    fn infers_reveal_band_from_one_selected_rim() {
        let (mut map, object_id) = opening_band_map();
        let object = &mut map.geometry_objects[0];
        object.vertices.extend([
            Vec3::new(-2.0, -2.0, 0.0),
            Vec3::new(2.0, -2.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(-2.0, 2.0, 0.0),
        ]);
        object.faces.extend([
            face(vec![8, 9, 1, 0]),
            face(vec![9, 10, 2, 1]),
            face(vec![10, 11, 3, 2]),
            face(vec![11, 8, 0, 3]),
        ]);
        map.selected_geometry_vertices = (0..4).map(|index| (object_id, index)).collect();

        let selection = fitted_selection(&map).expect("one closed rim should be sufficient");

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
    fn isolates_ground_level_opening_from_larger_wall_contour() {
        let mut map = Map::new();
        let mut object = rusterix::GeometryObject::new("Wall with ground-level opening");
        object.vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
            Vec3::new(0.0, 4.0, 6.0),
            Vec3::new(0.0, 0.0, 6.0),
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.0, 3.0, 2.0),
            Vec3::new(0.0, 3.0, 4.0),
            Vec3::new(0.0, 0.0, 4.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 4.0, 0.0),
            Vec3::new(1.0, 4.0, 6.0),
            Vec3::new(1.0, 0.0, 6.0),
            Vec3::new(1.0, 0.0, 2.0),
            Vec3::new(1.0, 3.0, 2.0),
            Vec3::new(1.0, 3.0, 4.0),
            Vec3::new(1.0, 0.0, 4.0),
        ];
        object.faces = vec![
            face(vec![0, 1, 5, 4]),
            face(vec![1, 2, 6, 5]),
            face(vec![2, 3, 7, 6]),
            face(vec![8, 12, 13, 9]),
            face(vec![9, 13, 14, 10]),
            face(vec![10, 14, 15, 11]),
            face(vec![0, 8, 9, 1]),
            face(vec![1, 9, 10, 2]),
            face(vec![2, 10, 11, 3]),
            face(vec![4, 5, 13, 12]),
            face(vec![5, 6, 14, 13]),
            face(vec![6, 7, 15, 14]),
        ];
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_vertices = (0..8).map(|index| (object_id, index)).collect();

        let selection = fitted_selection(&map).expect("the doorway reveal should be inferred");

        assert_eq!(selection.band_faces, BTreeSet::from([9, 10, 11]));
        assert_eq!(
            selection.loops[0].iter().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([4, 5, 6, 7])
        );
        assert_eq!(
            selection.loops[1].iter().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([12, 13, 14, 15])
        );
    }

    #[test]
    fn creates_independent_capped_solid_without_changing_source() {
        let (mut map, source_id) = opening_band_map();
        let source = map.geometry_objects[0].clone();

        let fitted_ids = create_fitted_geometry(
            &mut map,
            FittedOptions {
                depth: 1.0,
                ..Default::default()
            },
        )
        .expect("fitted solid should be created");
        let fitted_id = fitted_ids[0];

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

    #[test]
    fn creates_fitted_geometry_directly_from_selected_wall_opening() {
        let mut map = selected_wall_opening_map(rusterix::WallOpeningShape::Rectangular);
        let input = fitted_input(&map).expect("selected wall opening should be a fitted source");
        assert!((input.frame.original_depth - 0.4).abs() < 0.001);
        assert_eq!(
            polygon_bounds(&input.frame.polygon).unwrap().0,
            Vec2::new(-1.0, -1.25)
        );
        assert_eq!(
            polygon_bounds(&input.frame.polygon).unwrap().1,
            Vec2::new(1.0, 1.25)
        );

        let fitted_ids = create_fitted_geometry(
            &mut map,
            FittedOptions {
                depth: 0.4,
                ..Default::default()
            },
        )
        .expect("wall opening should create fitted geometry");
        let fitted = map
            .geometry_objects
            .iter()
            .find(|object| object.id == fitted_ids[0])
            .unwrap();
        let min_z = fitted
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::INFINITY, f32::min);
        let max_z = fitted
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_z + 0.2).abs() < 0.001);
        assert!((max_z - 0.2).abs() < 0.001);
    }

    #[test]
    fn selected_arch_wall_opening_supplies_curved_fitted_profile() {
        let map = selected_wall_opening_map(rusterix::WallOpeningShape::Arch);
        let input = fitted_input(&map).expect("selected arch should be a fitted source");
        let (min, max) = polygon_bounds(&input.frame.polygon).unwrap();
        assert!((min.x + 1.0).abs() < 0.001);
        assert!((max.x - 1.0).abs() < 0.001);
        assert!((min.y + 1.25).abs() < 0.001);
        assert!((max.y - 1.25).abs() < 0.001);
        assert!(input.frame.polygon.len() > 20);
    }

    #[test]
    fn custom_depth_stays_centered_in_the_opening() {
        let (mut map, _) = opening_band_map();
        let fitted_ids = create_fitted_geometry(
            &mut map,
            FittedOptions {
                depth: 0.4,
                ..Default::default()
            },
        )
        .unwrap();
        let fitted = map
            .geometry_objects
            .iter()
            .find(|object| object.id == fitted_ids[0])
            .unwrap();
        let min_z = fitted
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::INFINITY, f32::min);
        let max_z = fitted
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_z - 0.3).abs() < 0.001);
        assert!((max_z - 0.7).abs() < 0.001);
    }

    #[test]
    fn split_barred_output_creates_two_independent_watertight_leaves() {
        let (mut map, _) = opening_band_map();
        let fitted_ids = create_fitted_geometry(
            &mut map,
            FittedOptions {
                construction: FittedConstruction::Barred,
                leaves: FittedLeaves::Split,
                depth: 0.2,
                bar_width: 0.08,
                bar_spacing: 0.4,
                rail_width: 0.1,
            },
        )
        .unwrap();

        assert_eq!(fitted_ids.len(), 2);
        assert_eq!(map.selected_geometry_objects, fitted_ids);
        for (index, fitted_id) in fitted_ids.iter().enumerate() {
            let fitted = map
                .geometry_objects
                .iter()
                .find(|object| object.id == *fitted_id)
                .unwrap();
            assert!(!fitted.vertices.is_empty());
            assert!(!fitted.faces.is_empty());
            assert_eq!(
                fitted.properties.get_str("fitted_leaf"),
                Some(if index == 0 { "Left" } else { "Right" })
            );
            let mut edge_counts = BTreeMap::<Edge, usize>::new();
            for face in &fitted.faces {
                for edge in face_edges(face) {
                    *edge_counts.entry(edge).or_default() += 1;
                }
            }
            assert!(edge_counts.values().all(|count| *count == 2));
        }
    }
}
