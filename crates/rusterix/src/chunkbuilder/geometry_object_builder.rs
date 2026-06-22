use crate::collision_world::{
    ChunkCollision, DynamicOpening, OpeningType, StaticBarrier, WalkableFloor,
};
use crate::{Assets, Chunk, ChunkBuilder, Map, PixelSource};
use scenevm::{GeoId, SurfaceNoiseLayer};
use uuid::Uuid;
use vek::{Vec2, Vec3};

const DEFAULT_GEOMETRY_TILE_ID: &str = "27826750-a9e7-4346-994b-fb318b238452";
const TILED_FACE_RENDER_NUDGE: f32 = 0.0015;

#[derive(Clone)]
pub struct GeometryObjectBuilder;

impl GeometryObjectBuilder {
    fn default_tile_id() -> Uuid {
        Uuid::parse_str(DEFAULT_GEOMETRY_TILE_ID).unwrap_or_else(|_| Uuid::nil())
    }

    fn face_tile_id(
        tile: Option<&PixelSource>,
        material: Option<&crate::TileMaterialMeta>,
        assets: &Assets,
    ) -> Uuid {
        let base_id = tile
            .and_then(|source| source.render_tile_id(assets))
            .unwrap_or_else(Self::default_tile_id);

        material
            .and_then(|material| Assets::material_variant_tile_id(base_id, material))
            .filter(|variant_id| assets.tile_index(variant_id).is_some())
            .unwrap_or(base_id)
    }

    fn auto_face_uvs(points: &[Vec3<f32>]) -> Vec<[f32; 2]> {
        if points.len() < 3 {
            return vec![[0.0, 0.0]; points.len()];
        }

        let mut normal = Vec3::<f32>::zero();
        for index in 1..points.len() - 1 {
            normal += (points[index] - points[0]).cross(points[index + 1] - points[0]);
        }

        let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
        points
            .iter()
            .map(|point| {
                if abs.y >= abs.x && abs.y >= abs.z {
                    [point.x, point.z]
                } else if abs.x >= abs.z {
                    [point.z, point.y]
                } else {
                    [point.x, point.y]
                }
            })
            .collect()
    }

    fn barycentric_2d(
        p: Vec2<f32>,
        a: Vec2<f32>,
        b: Vec2<f32>,
        c: Vec2<f32>,
    ) -> Option<(f32, f32, f32)> {
        let v0 = b - a;
        let v1 = c - a;
        let v2 = p - a;
        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);
        let denom = d00 * d11 - d01 * d01;
        if denom.abs() <= 1e-6 {
            return None;
        }
        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;
        if u >= -1e-4 && v >= -1e-4 && w >= -1e-4 {
            Some((u, v, w))
        } else {
            None
        }
    }

    fn world_from_face_uv(
        uv: Vec2<f32>,
        face_uvs: &[Vec2<f32>],
        world_points: &[Vec3<f32>],
    ) -> Option<Vec3<f32>> {
        if face_uvs.len() < 3 || face_uvs.len() != world_points.len() {
            return None;
        }
        for index in 1..face_uvs.len() - 1 {
            let a = face_uvs[0];
            let b = face_uvs[index];
            let c = face_uvs[index + 1];
            let Some((u, v, w)) = Self::barycentric_2d(uv, a, b, c) else {
                continue;
            };
            return Some(
                world_points[0] * u + world_points[index] * v + world_points[index + 1] * w,
            );
        }
        None
    }

    fn face_render_nudge(world_points: &[Vec3<f32>], object_center: Vec3<f32>) -> Vec3<f32> {
        if world_points.len() < 3 {
            return Vec3::zero();
        }

        let mut normal = Vec3::<f32>::zero();
        let mut face_center = Vec3::<f32>::zero();
        for point in world_points {
            face_center += *point;
        }
        face_center /= world_points.len() as f32;

        for index in 1..world_points.len() - 1 {
            normal += (world_points[index] - world_points[0])
                .cross(world_points[index + 1] - world_points[0]);
        }
        let Some(mut normal) = normal.try_normalized() else {
            return Vec3::zero();
        };

        if normal.dot(face_center - object_center) < 0.0 {
            normal = -normal;
        }
        normal * TILED_FACE_RENDER_NUDGE
    }

    fn face_is_wall_like(world_points: &[Vec3<f32>]) -> bool {
        if world_points.len() < 3 {
            return false;
        }

        let mut normal = Vec3::<f32>::zero();
        for index in 1..world_points.len() - 1 {
            normal += (world_points[index] - world_points[0])
                .cross(world_points[index + 1] - world_points[0]);
        }
        let Some(normal) = normal.try_normalized() else {
            return false;
        };

        normal.y.abs() < 0.25
    }

    fn polygon_area_2d(points: &[Vec2<f32>]) -> f32 {
        if points.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        for index in 0..points.len() {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            area += a.x * b.y - b.x * a.y;
        }
        area.abs() * 0.5
    }

    fn walkable_floor_footprint_is_large_enough(points: &[Vec2<f32>]) -> bool {
        const MIN_WALKABLE_AREA: f32 = 0.05;
        const MIN_WALKABLE_NARROW_EXTENT: f32 = 0.12;

        if Self::polygon_area_2d(points) < MIN_WALKABLE_AREA {
            return false;
        }

        let mut min = Vec2::broadcast(f32::INFINITY);
        let mut max = Vec2::broadcast(f32::NEG_INFINITY);
        for point in points {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
        }

        (max.x - min.x).min(max.y - min.y) >= MIN_WALKABLE_NARROW_EXTENT
    }

    fn tiled_face_base_uvs(world_points: &[Vec3<f32>]) -> [[f32; 2]; 4] {
        if Self::face_is_wall_like(world_points) {
            [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
        } else {
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
        }
    }

    fn transformed_face_uvs(
        face: &crate::GeometryFace,
        uvs: &[[f32; 2]],
        _world_points: Option<&[Vec3<f32>]>,
    ) -> Vec<[f32; 2]> {
        if uvs.is_empty() {
            return Vec::new();
        }

        let min_uv = uvs.iter().fold(Vec2::broadcast(f32::INFINITY), |acc, uv| {
            Vec2::new(acc.x.min(uv[0]), acc.y.min(uv[1]))
        });
        let max_uv = uvs
            .iter()
            .fold(Vec2::broadcast(f32::NEG_INFINITY), |acc, uv| {
                Vec2::new(acc.x.max(uv[0]), acc.y.max(uv[1]))
            });
        if !min_uv.x.is_finite()
            || !min_uv.y.is_finite()
            || !max_uv.x.is_finite()
            || !max_uv.y.is_finite()
        {
            return uvs.to_vec();
        }

        let center = (min_uv + max_uv) * 0.5;
        let scale = Vec2::new(
            if face.texture_scale.x.abs() <= 1e-5 {
                1.0
            } else {
                face.texture_scale.x
            },
            if face.texture_scale.y.abs() <= 1e-5 {
                1.0
            } else {
                face.texture_scale.y
            },
        );
        let radians = face.texture_rotation.to_radians();
        let (sin, cos) = radians.sin_cos();

        uvs.iter()
            .enumerate()
            .map(|(_index, uv)| {
                let mut p = Vec2::new(uv[0], uv[1]) - center;
                p = Vec2::new(p.x / scale.x, p.y / scale.y);
                if radians.abs() > 1e-5 {
                    p = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
                }
                p += center + face.texture_offset;
                [p.x, p.y]
            })
            .collect()
    }

    fn surface_noise_layer(noise: &crate::GeometrySurfaceNoise) -> SurfaceNoiseLayer {
        SurfaceNoiseLayer {
            scale: noise.scale.max(0.0001),
            amount: noise.amount.clamp(0.0, 1.0),
            seed: noise.seed as f32,
        }
    }

    fn face_normal(world_points: &[Vec3<f32>], object_center: Vec3<f32>) -> Option<Vec3<f32>> {
        if world_points.len() < 3 {
            return None;
        }

        let mut normal = Vec3::<f32>::zero();
        let mut face_center = Vec3::<f32>::zero();
        for point in world_points {
            face_center += *point;
        }
        face_center /= world_points.len() as f32;

        for index in 1..world_points.len() - 1 {
            normal += (world_points[index] - world_points[0])
                .cross(world_points[index + 1] - world_points[0]);
        }
        let mut normal = normal.try_normalized()?;
        if normal.dot(face_center - object_center) < 0.0 {
            normal = -normal;
        }
        Some(normal)
    }

    fn raw_face_normal(world_points: &[Vec3<f32>]) -> Option<Vec3<f32>> {
        if world_points.len() < 3 {
            return None;
        }

        let mut normal = Vec3::<f32>::zero();
        for index in 1..world_points.len() - 1 {
            normal += (world_points[index] - world_points[0])
                .cross(world_points[index + 1] - world_points[0]);
        }
        normal.try_normalized()
    }

    fn object_center(object: &crate::GeometryObject, bbox: crate::BBox) -> Vec3<f32> {
        let center = bbox.center();
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for vertex in &object.vertices {
            let world = object.transform_point(*vertex);
            min_y = min_y.min(world.y);
            max_y = max_y.max(world.y);
        }
        let center_y = if min_y.is_finite() && max_y.is_finite() {
            (min_y + max_y) * 0.5
        } else {
            0.0
        };
        Vec3::new(center.x, center_y, center.y)
    }

    fn points_bbox(points: &[Vec3<f32>]) -> Option<crate::BBox> {
        let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut found = false;
        for point in points {
            if !point.x.is_finite() || !point.z.is_finite() {
                continue;
            }
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.z);
            found = true;
        }
        found.then(|| crate::BBox::new(min, max))
    }

    fn face_world_points(
        object: &crate::GeometryObject,
        face: &crate::GeometryFace,
    ) -> Option<Vec<Vec3<f32>>> {
        if face.indices.len() < 3 {
            return None;
        }
        let mut points = Vec::with_capacity(face.indices.len());
        for object_vertex_index in &face.indices {
            let vertex = object.vertices.get(*object_vertex_index)?;
            points.push(object.transform_point(*vertex));
        }
        Some(points)
    }

    fn add_face_barriers(
        collision: &mut ChunkCollision,
        object_id: Uuid,
        _face_index: usize,
        _normal: Vec3<f32>,
        _raw_normal: Vec3<f32>,
        world_points: &[Vec3<f32>],
    ) {
        let min_y = world_points
            .iter()
            .fold(f32::INFINITY, |acc, point| acc.min(point.y));
        let max_y = world_points
            .iter()
            .fold(f32::NEG_INFINITY, |acc, point| acc.max(point.y));
        if !min_y.is_finite() || !max_y.is_finite() || max_y - min_y <= 0.05 {
            return;
        }

        let mut y_levels = world_points.iter().map(|point| point.y).collect::<Vec<_>>();
        y_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        y_levels.dedup_by(|a, b| (*a - *b).abs() <= 1e-4);
        if y_levels.len() < 2 {
            return;
        }

        let mut segments: Vec<(Vec2<f32>, Vec2<f32>, f32, f32)> = Vec::new();
        for y_index in 0..y_levels.len() - 1 {
            let band_min_y = y_levels[y_index];
            let band_max_y = y_levels[y_index + 1];
            if band_max_y - band_min_y <= 0.02 {
                continue;
            }
            let sample_y = (band_min_y + band_max_y) * 0.5;
            let mut intersections: Vec<Vec2<f32>> = Vec::new();
            for index in 0..world_points.len() {
                let a = world_points[index];
                let b = world_points[(index + 1) % world_points.len()];
                let da = a.y - sample_y;
                let db = b.y - sample_y;
                if da.abs() <= 1e-5 && db.abs() <= 1e-5 {
                    let start = Vec2::new(a.x, a.z);
                    let end = Vec2::new(b.x, b.z);
                    if (end - start).magnitude_squared() > 1e-6 {
                        intersections.push(start);
                        intersections.push(end);
                    }
                    continue;
                }
                if (da > 0.0 && db > 0.0) || (da < 0.0 && db < 0.0) {
                    continue;
                }
                let denom = b.y - a.y;
                if denom.abs() <= 1e-6 {
                    continue;
                }
                let t = ((sample_y - a.y) / denom).clamp(0.0, 1.0);
                let point = a + (b - a) * t;
                intersections.push(Vec2::new(point.x, point.z));
            }

            let mut unique: Vec<Vec2<f32>> = Vec::new();
            for point in intersections {
                if !unique
                    .iter()
                    .any(|existing| (*existing - point).magnitude_squared() <= 1e-6)
                {
                    unique.push(point);
                }
            }
            if unique.len() < 2 {
                continue;
            }

            let (min_x, max_x, min_z, max_z) = unique.iter().fold(
                (
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                ),
                |(min_x, max_x, min_z, max_z), point| {
                    (
                        min_x.min(point.x),
                        max_x.max(point.x),
                        min_z.min(point.y),
                        max_z.max(point.y),
                    )
                },
            );
            let sort_by_x = max_x - min_x >= max_z - min_z;
            unique.sort_by(|a, b| {
                let av = if sort_by_x { a.x } else { a.y };
                let bv = if sort_by_x { b.x } else { b.y };
                av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal)
            });

            for pair in unique.chunks(2) {
                if pair.len() < 2 {
                    continue;
                }
                let start = pair[0];
                let end = pair[1];
                if (end - start).magnitude_squared() <= 1e-6 {
                    continue;
                }
                let duplicate = segments
                    .iter()
                    .any(|(existing_start, existing_end, y0, y1)| {
                        ((*y0 - band_min_y).abs() <= 1e-4 && (*y1 - band_max_y).abs() <= 1e-4)
                            && (((*existing_start - start).magnitude_squared() <= 1e-6
                                && (*existing_end - end).magnitude_squared() <= 1e-6)
                                || ((*existing_start - end).magnitude_squared() <= 1e-6
                                    && (*existing_end - start).magnitude_squared() <= 1e-6))
                    });
                if duplicate {
                    continue;
                }
                segments.push((start, end, band_min_y, band_max_y));
            }
        }

        for (start, end, barrier_min_y, barrier_max_y) in segments {
            let duplicate = collision.static_barriers.iter().any(|existing| {
                existing.geo_id == GeoId::GeometryObject(object_id)
                    && (existing.min_y - barrier_min_y).abs() <= 1e-4
                    && (existing.max_y - barrier_max_y).abs() <= 1e-4
                    && (((existing.start - start).magnitude_squared() <= 1e-6
                        && (existing.end - end).magnitude_squared() <= 1e-6)
                        || ((existing.start - end).magnitude_squared() <= 1e-6
                            && (existing.end - start).magnitude_squared() <= 1e-6))
            });
            if duplicate {
                continue;
            }
            collision.static_barriers.push(StaticBarrier {
                geo_id: GeoId::GeometryObject(object_id),
                start,
                end,
                min_y: barrier_min_y,
                max_y: barrier_max_y,
            });
        }
    }

    fn mesh_edge_adjacent_points(
        object: &crate::GeometryObject,
        a: Vec3<f32>,
        b: Vec3<f32>,
    ) -> Option<Vec<Vec3<f32>>> {
        const EPS_SQ: f32 = 0.0001;
        let mut points = Vec::new();
        for face in &object.faces {
            for index in 0..face.indices.len() {
                let Some(pa) = face
                    .indices
                    .get(index)
                    .and_then(|i| object.vertices.get(*i))
                else {
                    continue;
                };
                let Some(pb) = face
                    .indices
                    .get((index + 1) % face.indices.len())
                    .and_then(|i| object.vertices.get(*i))
                else {
                    continue;
                };
                let matches = ((*pa - a).magnitude_squared() <= EPS_SQ
                    && (*pb - b).magnitude_squared() <= EPS_SQ)
                    || ((*pa - b).magnitude_squared() <= EPS_SQ
                        && (*pb - a).magnitude_squared() <= EPS_SQ);
                if matches {
                    for vertex_index in &face.indices {
                        if let Some(point) = object.vertices.get(*vertex_index) {
                            points.push(*point);
                        }
                    }
                }
            }
        }

        (!points.is_empty()).then_some(points)
    }

    fn ordered_surface_loop_indices(face: &crate::GeometryFace) -> Vec<Vec<usize>> {
        let mut unused = (0..face.surface_segments.len()).collect::<Vec<_>>();
        let mut loops = Vec::new();

        while let Some(first_segment_index) = unused.pop() {
            let Some(first) = face.surface_segments.get(first_segment_index) else {
                continue;
            };
            let start = first.start;
            let mut current = first.end;
            let mut ordered = vec![start, current];

            while current != start {
                let Some((unused_pos, next_segment_index, next_point)) = unused
                    .iter()
                    .enumerate()
                    .find_map(|(unused_pos, segment_index)| {
                        let segment = face.surface_segments.get(*segment_index)?;
                        if segment.start == current {
                            Some((unused_pos, *segment_index, segment.end))
                        } else if segment.end == current {
                            Some((unused_pos, *segment_index, segment.start))
                        } else {
                            None
                        }
                    })
                else {
                    ordered.clear();
                    break;
                };
                unused.swap_remove(unused_pos);
                let _ = next_segment_index;
                current = next_point;
                if current != start {
                    ordered.push(current);
                }
                if ordered.len() > face.surface_points.len() + 1 {
                    ordered.clear();
                    break;
                }
            }

            if ordered.len() >= 3 {
                loops.push(ordered);
            }
        }

        loops
    }

    fn cutout_opening_boundary_2d(
        points: &[Vec3<f32>],
        normal: Vec3<f32>,
    ) -> Option<Vec<Vec2<f32>>> {
        if points.len() < 3 {
            return None;
        }
        let normal_2d = Vec2::new(normal.x, normal.z);
        let normal_len = normal_2d.magnitude();
        if normal_len <= 1e-6 {
            return None;
        }
        let normal_2d = normal_2d / normal_len;
        let tangent = Vec2::new(-normal_2d.y, normal_2d.x);

        let mut min_t = f32::INFINITY;
        let mut max_t = f32::NEG_INFINITY;
        let mut min_n = f32::INFINITY;
        let mut max_n = f32::NEG_INFINITY;
        for point in points {
            let p = Vec2::new(point.x, point.z);
            let t = p.dot(tangent);
            let n = p.dot(normal_2d);
            min_t = min_t.min(t);
            max_t = max_t.max(t);
            min_n = min_n.min(n);
            max_n = max_n.max(n);
        }
        if !min_t.is_finite() || !max_t.is_finite() || !min_n.is_finite() || !max_n.is_finite() {
            return None;
        }

        let half_depth = ((max_n - min_n) * 0.5).max(0.08);
        let center_n = (min_n + max_n) * 0.5;
        Some(vec![
            tangent * min_t + normal_2d * (center_n - half_depth),
            tangent * max_t + normal_2d * (center_n - half_depth),
            tangent * max_t + normal_2d * (center_n + half_depth),
            tangent * min_t + normal_2d * (center_n + half_depth),
        ])
    }

    fn add_cutout_openings_from_surface_guides(
        collision: &mut ChunkCollision,
        object: &crate::GeometryObject,
        face: &crate::GeometryFace,
        normal: Vec3<f32>,
    ) {
        if face.surface_points.len() < 3 || face.surface_segments.len() < 3 {
            return;
        }

        let loops = Self::ordered_surface_loop_indices(face);

        for loop_indices in loops {
            let Some(local_points) = loop_indices
                .iter()
                .map(|index| face.surface_points.get(*index).map(|point| point.position))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let mut opening_points = local_points.clone();
            let boundary_matches_mesh = (0..local_points.len()).all(|index| {
                let Some(adjacent_points) = Self::mesh_edge_adjacent_points(
                    object,
                    local_points[index],
                    local_points[(index + 1) % local_points.len()],
                ) else {
                    return false;
                };
                opening_points.extend(adjacent_points);
                true
            });
            if !boundary_matches_mesh {
                continue;
            }

            let world_points = opening_points
                .iter()
                .map(|point| object.transform_point(*point))
                .collect::<Vec<_>>();
            let Some(boundary_2d) = Self::cutout_opening_boundary_2d(&world_points, normal) else {
                continue;
            };
            let min_y = world_points
                .iter()
                .map(|point| point.y)
                .fold(f32::INFINITY, f32::min);
            let max_y = world_points
                .iter()
                .map(|point| point.y)
                .fold(f32::NEG_INFINITY, f32::max);
            if !min_y.is_finite() || !max_y.is_finite() || max_y - min_y <= 0.05 {
                continue;
            }

            collision.dynamic_openings.push(DynamicOpening {
                geo_id: GeoId::GeometryObject(object.id),
                item_blocking: Some(false),
                boundary_2d,
                floor_height: min_y - 0.05,
                ceiling_height: max_y + 0.05,
                opening_type: OpeningType::Passage,
            });
        }
    }

    fn add_tiled_face(
        face: &crate::GeometryFace,
        assets: &Assets,
        vmchunk: &mut scenevm::Chunk,
        object_id: Uuid,
        material: Option<&crate::TileMaterialMeta>,
        face_uvs: &[Vec2<f32>],
        world_points: &[Vec3<f32>],
        object_center: Vec3<f32>,
    ) -> bool {
        if face_uvs.len() < 3 || face_uvs.len() != world_points.len() {
            return false;
        }

        let min_uv = face_uvs
            .iter()
            .fold(Vec2::broadcast(f32::INFINITY), |acc, uv| {
                Vec2::new(acc.x.min(uv.x), acc.y.min(uv.y))
            });
        let max_uv = face_uvs
            .iter()
            .fold(Vec2::broadcast(f32::NEG_INFINITY), |acc, uv| {
                Vec2::new(acc.x.max(uv.x), acc.y.max(uv.y))
            });
        if !min_uv.x.is_finite()
            || !min_uv.y.is_finite()
            || !max_uv.x.is_finite()
            || !max_uv.y.is_finite()
        {
            return false;
        }

        let render_nudge = Self::face_render_nudge(world_points, object_center);
        let range = max_uv - min_uv;
        let cells_x = range.x.ceil().max(1.0) as i32;
        let cells_y = range.y.ceil().max(1.0) as i32;
        for ty in 0..cells_y {
            for tx in 0..cells_x {
                let x0 = min_uv.x + tx as f32;
                let x1 = (x0 + 1.0).min(max_uv.x);
                let y0 = min_uv.y + ty as f32;
                let y1 = (y0 + 1.0).min(max_uv.y);
                if x1 - x0 <= 1e-4 || y1 - y0 <= 1e-4 {
                    continue;
                }

                let corners_uv = [
                    Vec2::new(x0, y1),
                    Vec2::new(x0, y0),
                    Vec2::new(x1, y0),
                    Vec2::new(x1, y1),
                ];
                let mut vertices = Vec::with_capacity(4);
                let mut vertices_world = Vec::with_capacity(4);
                for uv in corners_uv {
                    let Some(world) = Self::world_from_face_uv(uv, face_uvs, world_points) else {
                        vertices.clear();
                        vertices_world.clear();
                        break;
                    };
                    let world = world + render_nudge;
                    vertices_world.push(world);
                    vertices.push([world.x, world.y, world.z, 1.0]);
                }
                if vertices.len() != 4 {
                    continue;
                }

                let source = face.tiles.get(&(tx, ty)).or(face.tile.as_ref());
                let tile_id = Self::face_tile_id(source, material, assets);
                let noise_tile_id = face
                    .surface_noise
                    .as_ref()
                    .and_then(|noise| noise.source.as_ref())
                    .map(|source| Self::face_tile_id(Some(source), None, assets));
                let tile_uvs = Self::tiled_face_base_uvs(&vertices_world);
                let uvs = Self::transformed_face_uvs(face, &tile_uvs, Some(&vertices_world));
                if let (Some(noise), Some(noise_tile_id)) =
                    (face.surface_noise.as_ref(), noise_tile_id)
                {
                    vmchunk.add_poly_3d_surface_noise(
                        GeoId::GeometryObject(object_id),
                        tile_id,
                        noise_tile_id,
                        vertices,
                        uvs,
                        vec![(0, 1, 2), (0, 2, 3)],
                        0,
                        true,
                        Self::surface_noise_layer(noise),
                    );
                } else {
                    vmchunk.add_poly_3d(
                        GeoId::GeometryObject(object_id),
                        tile_id,
                        vertices,
                        uvs,
                        vec![(0, 1, 2), (0, 2, 3)],
                        0,
                        true,
                    );
                }
            }
        }

        true
    }
}

impl ChunkBuilder for GeometryObjectBuilder {
    fn new() -> Self {
        Self
    }

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
        for object in &map.geometry_objects {
            // Build hidden objects too so scripts can later reveal 3D geometry
            // through the controlling area's `visible` attribute.
            let Some(bbox) = object.bbox() else {
                continue;
            };
            if !bbox.intersects(&chunk.bbox) || !chunk.bbox.contains(bbox.center()) {
                continue;
            }
            let object_center = Self::object_center(object, bbox);
            let object_material = Assets::object_material_meta(&object.properties);

            for face in &object.faces {
                if face.indices.len() < 3 {
                    continue;
                }

                let tile_id =
                    Self::face_tile_id(face.tile.as_ref(), object_material.as_ref(), assets);
                let mut vertices = Vec::with_capacity(face.indices.len());
                let mut local_points = Vec::with_capacity(face.indices.len());
                let mut world_points = Vec::with_capacity(face.indices.len());
                let mut uvs = Vec::with_capacity(face.indices.len());

                for (face_vertex_index, object_vertex_index) in face.indices.iter().enumerate() {
                    let Some(vertex) = object.vertices.get(*object_vertex_index) else {
                        vertices.clear();
                        break;
                    };
                    let world = object.transform_point(*vertex);
                    vertices.push([world.x, world.y, world.z, 1.0]);
                    local_points.push(*vertex);
                    world_points.push(world);

                    if !face.auto_uv {
                        let uv = face
                            .uvs
                            .get(face_vertex_index)
                            .copied()
                            .unwrap_or_else(Vec2::zero);
                        uvs.push([uv.x, uv.y]);
                    }
                }

                if vertices.len() < 3 {
                    continue;
                }
                if face.auto_uv {
                    uvs = Self::auto_face_uvs(&local_points);
                }
                if face.auto_uv && !face.tiles.is_empty() {
                    let face_uvs = uvs
                        .iter()
                        .map(|uv| Vec2::new(uv[0], uv[1]))
                        .collect::<Vec<_>>();
                    if Self::add_tiled_face(
                        face,
                        assets,
                        vmchunk,
                        object.id,
                        object_material.as_ref(),
                        &face_uvs,
                        &world_points,
                        object_center,
                    ) {
                        continue;
                    }
                }

                let mut indices = Vec::with_capacity(vertices.len().saturating_sub(2));
                for index in 1..vertices.len() - 1 {
                    indices.push((0, index, index + 1));
                }
                let uvs = Self::transformed_face_uvs(face, &uvs, Some(&world_points));

                let noise_tile_id = face
                    .surface_noise
                    .as_ref()
                    .and_then(|noise| noise.source.as_ref())
                    .map(|source| Self::face_tile_id(Some(source), None, assets));
                if let (Some(noise), Some(noise_tile_id)) =
                    (face.surface_noise.as_ref(), noise_tile_id)
                {
                    vmchunk.add_poly_3d_surface_noise(
                        GeoId::GeometryObject(object.id),
                        tile_id,
                        noise_tile_id,
                        vertices,
                        uvs,
                        indices,
                        0,
                        true,
                        Self::surface_noise_layer(noise),
                    );
                } else {
                    vmchunk.add_poly_3d(
                        GeoId::GeometryObject(object.id),
                        tile_id,
                        vertices,
                        uvs,
                        indices,
                        0,
                        true,
                    );
                }
            }
        }
    }

    fn build_collision(
        &mut self,
        map: &Map,
        _assets: &Assets,
        chunk_origin: Vec2<i32>,
        chunk_size: i32,
    ) -> ChunkCollision {
        let mut collision = ChunkCollision::new();
        let chunk_bbox = crate::BBox::from_pos_size(
            chunk_origin.map(|v| v as f32) * chunk_size as f32,
            Vec2::broadcast(chunk_size as f32),
        );

        for object in &map.geometry_objects {
            if !object.solid {
                continue;
            }
            let Some(object_bbox) = object.bbox() else {
                continue;
            };
            if !object_bbox.intersects(&chunk_bbox) {
                continue;
            }

            let object_center = Self::object_center(object, object_bbox);
            let object_min_y = object
                .vertices
                .iter()
                .map(|vertex| object.transform_point(*vertex).y)
                .fold(f32::INFINITY, f32::min);
            for (face_index, face) in object.faces.iter().enumerate() {
                let Some(world_points) = Self::face_world_points(object, face) else {
                    continue;
                };
                let Some(face_bbox) = Self::points_bbox(&world_points) else {
                    continue;
                };
                if !face_bbox.intersects(&chunk_bbox) {
                    continue;
                }
                let Some(normal) = Self::face_normal(&world_points, object_center) else {
                    continue;
                };
                let raw_normal = Self::raw_face_normal(&world_points).unwrap_or(normal);

                if normal.y >= 0.55 || raw_normal.y.abs() >= 0.55 {
                    let height = world_points.iter().map(|point| point.y).sum::<f32>()
                        / world_points.len() as f32;
                    if height <= object_min_y + 1e-3 {
                        continue;
                    }
                    let polygon_2d = world_points
                        .iter()
                        .map(|point| Vec2::new(point.x, point.z))
                        .collect::<Vec<_>>();
                    if !Self::walkable_floor_footprint_is_large_enough(&polygon_2d) {
                        continue;
                    }
                    let floor_normal = if normal.y >= 0.55 {
                        normal
                    } else if raw_normal.y >= 0.0 {
                        raw_normal
                    } else {
                        -raw_normal
                    };
                    collision.walkable_floors.push(WalkableFloor::planar(
                        GeoId::GeometryObject(object.id),
                        height,
                        polygon_2d,
                        floor_normal,
                        world_points[0],
                    ));
                } else if normal.y.abs() < 0.75 {
                    Self::add_cutout_openings_from_surface_guides(
                        &mut collision,
                        object,
                        face,
                        normal,
                    );
                    Self::add_face_barriers(
                        &mut collision,
                        object.id,
                        face_index,
                        normal,
                        raw_normal,
                        &world_points,
                    );
                }
            }
        }

        collision
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiled_wall_face_uvs_keep_tiles_upright() {
        let wall_cell = [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ];
        let floor_cell = [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ];

        assert!(GeometryObjectBuilder::face_is_wall_like(&wall_cell));
        assert_eq!(
            GeometryObjectBuilder::tiled_face_base_uvs(&wall_cell),
            [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]
        );
        assert!(!GeometryObjectBuilder::face_is_wall_like(&floor_cell));
        assert_eq!(
            GeometryObjectBuilder::tiled_face_base_uvs(&floor_cell),
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
        );
    }

    #[test]
    fn surface_noise_uses_shader_layer_without_topology_change() {
        let mut object = crate::GeometryObject::box_from_bounds(
            "Box",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 2.0),
        );
        let original_face_count = object.faces.len();
        object.faces[2].surface_noise = Some(crate::GeometrySurfaceNoise {
            scale: 1.0,
            amount: 0.35,
            seed: 0,
            source: Some(PixelSource::PaletteIndex(0)),
        });

        let mut map = Map::default();
        let object_id = object.id;
        map.geometry_objects.push(object);

        let assets = Assets::default();
        let mut chunk = Chunk::new(Vec2::zero(), 16);
        let mut vmchunk = scenevm::Chunk::new(Vec2::zero(), 16);
        let mut builder = GeometryObjectBuilder;
        builder.build(&map, &assets, &mut chunk, &mut vmchunk);

        let polys = vmchunk
            .polys3d_map
            .get(&GeoId::GeometryObject(object_id))
            .expect("geometry object should render");
        assert_eq!(polys.len(), original_face_count);
        assert!(polys.iter().any(|poly| poly.surface_noise.is_some()));
        assert_eq!(map.geometry_objects[0].faces.len(), original_face_count);
    }

    #[test]
    fn high_scale_surface_noise_does_not_add_render_tessellation() {
        let mut object = crate::GeometryObject::new("Many Triangles");
        for index in 0..64 {
            let x = (index % 8) as f32;
            let z = (index / 8) as f32;
            let base = object.vertices.len();
            object.vertices.extend([
                Vec3::new(x, 0.0, z),
                Vec3::new(x + 0.8, 0.0, z),
                Vec3::new(x, 0.0, z + 0.8),
            ]);
            object.faces.push(crate::GeometryFace {
                indices: vec![base, base + 1, base + 2],
                uvs: Vec::new(),
                auto_uv: true,
                texture_offset: Vec2::zero(),
                texture_scale: Vec2::broadcast(1.0),
                texture_rotation: 0.0,
                tile: None,
                tiles: Default::default(),
                surface_points: Vec::new(),
                surface_segments: Vec::new(),
                surface_noise: Some(crate::GeometrySurfaceNoise {
                    scale: 500.0,
                    amount: 0.35,
                    seed: 0,
                    source: Some(PixelSource::PaletteIndex(0)),
                }),
            });
        }
        let object_id = object.id;
        let face_count = object.faces.len();
        let mut map = Map::default();
        map.geometry_objects.push(object);

        let assets = Assets::default();
        let mut chunk = Chunk::new(Vec2::zero(), 16);
        let mut vmchunk = scenevm::Chunk::new(Vec2::zero(), 16);
        let mut builder = GeometryObjectBuilder;
        builder.build(&map, &assets, &mut chunk, &mut vmchunk);

        let polys = vmchunk
            .polys3d_map
            .get(&GeoId::GeometryObject(object_id))
            .expect("geometry object should render");
        assert_eq!(
            polys.len(),
            face_count,
            "shader-side surface noise should not add render polys"
        );
        assert!(polys.iter().all(|poly| poly.surface_noise.is_some()));
    }

    #[test]
    fn collision_keeps_inset_horizontal_geometry_faces_walkable() {
        let mut object = crate::GeometryObject::new("StairLike");
        let object_id = object.id;
        object.vertices = vec![
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(1.0, 0.5, 0.0),
            Vec3::new(1.0, 0.5, 1.0),
            Vec3::new(0.0, 0.5, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];
        object.faces = vec![crate::GeometryFace {
            // This winding produces a raw downward normal. For concave/stepped
            // objects, an outward-center flip would otherwise hide this tread
            // from collision because it sits below the object's vertical center.
            indices: vec![0, 1, 2, 3],
            uvs: Vec::new(),
            auto_uv: true,
            texture_offset: Vec2::zero(),
            texture_scale: Vec2::broadcast(1.0),
            texture_rotation: 0.0,
            tile: None,
            tiles: Default::default(),
            surface_points: Vec::new(),
            surface_segments: Vec::new(),
            surface_noise: None,
        }];

        let mut map = Map::default();
        map.geometry_objects.push(object);

        let assets = Assets::default();
        let mut builder = GeometryObjectBuilder;
        let collision = builder.build_collision(&map, &assets, Vec2::zero(), 16);

        assert!(
            collision
                .walkable_floors
                .iter()
                .any(|floor| floor.geo_id == GeoId::GeometryObject(object_id)
                    && (floor.height - 0.5).abs() < 1e-4)
        );
    }

    #[test]
    fn collision_ignores_tiny_decorative_ledge_faces_as_walkable() {
        let mut object = crate::GeometryObject::new("GrooveLike");
        let object_id = object.id;
        object.vertices = vec![
            Vec3::new(0.0, 0.25, 0.0),
            Vec3::new(1.0, 0.25, 0.0),
            Vec3::new(1.0, 0.25, 0.02),
            Vec3::new(0.0, 0.25, 0.02),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        object.faces = vec![crate::GeometryFace {
            indices: vec![0, 1, 2, 3],
            uvs: Vec::new(),
            auto_uv: true,
            texture_offset: Vec2::zero(),
            texture_scale: Vec2::broadcast(1.0),
            texture_rotation: 0.0,
            tile: None,
            tiles: Default::default(),
            surface_points: Vec::new(),
            surface_segments: Vec::new(),
            surface_noise: None,
        }];

        let mut map = Map::default();
        map.geometry_objects.push(object);

        let assets = Assets::default();
        let mut builder = GeometryObjectBuilder;
        let collision = builder.build_collision(&map, &assets, Vec2::zero(), 16);

        assert!(
            !collision
                .walkable_floors
                .iter()
                .any(|floor| floor.geo_id == GeoId::GeometryObject(object_id)),
            "thin decorative ledges should not become floor collision"
        );
    }

    #[test]
    fn collision_ignores_stale_cap_inside_real_cutout_boundary() {
        fn test_face(indices: Vec<usize>) -> crate::GeometryFace {
            crate::GeometryFace {
                indices,
                uvs: Vec::new(),
                auto_uv: true,
                texture_offset: Vec2::zero(),
                texture_scale: Vec2::broadcast(1.0),
                texture_rotation: 0.0,
                tile: None,
                tiles: Default::default(),
                surface_points: Vec::new(),
                surface_segments: Vec::new(),
                surface_noise: None,
            }
        }

        let mut object = crate::GeometryObject::new("WallWithCutoutAndStaleCap");
        object.vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(6.0, 0.0, 0.0),
            Vec3::new(6.0, 3.0, 0.0),
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(4.0, 2.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(2.0, 0.0, 0.5),
            Vec3::new(4.0, 0.0, 0.5),
            Vec3::new(4.0, 2.0, 0.5),
            Vec3::new(2.0, 2.0, 0.5),
        ];

        let mut stale_front_cap = test_face(vec![0, 1, 2, 3]);
        stale_front_cap.surface_points = vec![
            crate::GeometrySurfacePoint {
                position: object.vertices[4],
                mode: crate::GeometrySurfacePointMode::Corner,
            },
            crate::GeometrySurfacePoint {
                position: object.vertices[5],
                mode: crate::GeometrySurfacePointMode::Corner,
            },
            crate::GeometrySurfacePoint {
                position: object.vertices[6],
                mode: crate::GeometrySurfacePointMode::Corner,
            },
            crate::GeometrySurfacePoint {
                position: object.vertices[7],
                mode: crate::GeometrySurfacePointMode::Corner,
            },
        ];
        stale_front_cap.surface_segments = vec![
            crate::GeometrySurfaceSegment {
                start: 0,
                end: 1,
                mode: crate::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.0,
            },
            crate::GeometrySurfaceSegment {
                start: 1,
                end: 2,
                mode: crate::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.0,
            },
            crate::GeometrySurfaceSegment {
                start: 2,
                end: 3,
                mode: crate::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.0,
            },
            crate::GeometrySurfaceSegment {
                start: 3,
                end: 0,
                mode: crate::GeometrySurfaceSegmentMode::Line,
                curve_amount: 0.0,
            },
        ];

        object.faces = vec![
            stale_front_cap,
            test_face(vec![4, 5, 9, 8]),
            test_face(vec![5, 6, 10, 9]),
            test_face(vec![6, 7, 11, 10]),
            test_face(vec![7, 4, 8, 11]),
            test_face(vec![4, 7, 6, 5]),
            test_face(vec![9, 10, 11, 8]),
        ];

        let mut map = Map::default();
        map.geometry_objects.push(object);

        let assets = Assets::default();
        let mut builder = GeometryObjectBuilder;
        let mut collision = builder.build_collision(&map, &assets, Vec2::zero(), 16);
        collision.walkable_floors.push(WalkableFloor::flat(
            GeoId::GeometryObject(map.geometry_objects[0].id),
            0.0,
            vec![
                Vec2::new(0.0, -1.0),
                Vec2::new(6.0, -1.0),
                Vec2::new(6.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
        ));

        let mut world = crate::CollisionWorld::new(16);
        world.update_chunk(Vec2::zero(), collision);
        let (end, arrived) = world
            .move_towards_on_floors_direct(
                Vec2::new(3.0, -0.25),
                Vec2::new(3.0, 0.75),
                1.0,
                0.3,
                1.0,
                0.0,
            )
            .expect("cutout floor should be walkable");

        assert!(arrived, "movement should pass through the cutout");
        assert!(end.z > 0.7, "expected to move through opening, got {end:?}");
    }
}
