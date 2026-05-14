use crate::collision_world::{ChunkCollision, StaticBarrier, WalkableFloor};
use crate::{Assets, Chunk, ChunkBuilder, Map, PixelSource};
use scenevm::GeoId;
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

    fn face_tile_id(tile: Option<&PixelSource>, assets: &Assets) -> Uuid {
        tile.and_then(|source| source.tile_from_tile_list(assets))
            .map(|tile| tile.id)
            .unwrap_or_else(Self::default_tile_id)
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

    fn transformed_face_uvs(
        face: &crate::GeometryFace,
        uvs: &[[f32; 2]],
        world_points: Option<&[Vec3<f32>]>,
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
            .map(|(index, uv)| {
                let mut p = Vec2::new(uv[0], uv[1]) - center;
                p = Vec2::new(p.x / scale.x, p.y / scale.y);
                if radians.abs() > 1e-5 {
                    p = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
                }
                p += center + face.texture_offset;
                if let Some(noise) = face.surface_noise.as_ref()
                    && let Some(world_points) = world_points
                    && let Some(world) = world_points.get(index)
                {
                    p += Self::surface_noise_uv_offset(noise, *world);
                }
                [p.x, p.y]
            })
            .collect()
    }

    fn surface_noise_uv_offset(noise: &crate::GeometrySurfaceNoise, world: Vec3<f32>) -> Vec2<f32> {
        let scale = noise.scale.max(0.05);
        let amount = noise.amount.clamp(0.0, 1.0) * 0.18;
        if amount <= 1e-5 {
            return Vec2::zero();
        }
        let seed = noise.seed as f32 * 0.017;
        let p = world * scale;
        let nx = ((p.x * 12.9898 + p.y * 78.233 + p.z * 37.719 + seed).sin() * 43_758.547)
            .rem_euclid(1.0);
        let ny = ((p.x * 93.989 + p.y * 67.345 + p.z * 21.123 + seed + 19.19).sin() * 24_634.635)
            .rem_euclid(1.0);
        Vec2::new(nx - 0.5, ny - 0.5) * amount
    }

    fn surface_noise_weight(noise: &crate::GeometrySurfaceNoise, world: Vec3<f32>) -> f32 {
        let scale = noise.scale.max(0.05);
        let amount = noise.amount.clamp(0.0, 1.0);
        if amount <= 1e-5 {
            return 0.0;
        }
        let seed = noise.seed as f32 * 0.017 + 41.37;
        let p = world * scale;
        let n = ((p.x * 27.619 + p.y * 57.583 + p.z * 12.9898 + seed).sin() * 16_719.371)
            .rem_euclid(1.0);
        (n * amount).clamp(0.0, 1.0)
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

        let mut segments: Vec<(Vec2<f32>, Vec2<f32>)> = Vec::new();
        for index in 0..world_points.len() {
            let a = world_points[index];
            let b = world_points[(index + 1) % world_points.len()];
            let start = Vec2::new(a.x, a.z);
            let end = Vec2::new(b.x, b.z);
            if (end - start).magnitude_squared() <= 1e-6 {
                continue;
            }
            let duplicate = segments.iter().any(|(existing_start, existing_end)| {
                ((*existing_start - start).magnitude_squared() <= 1e-6
                    && (*existing_end - end).magnitude_squared() <= 1e-6)
                    || ((*existing_start - end).magnitude_squared() <= 1e-6
                        && (*existing_end - start).magnitude_squared() <= 1e-6)
            });
            if duplicate {
                continue;
            }
            segments.push((start, end));
        }

        for (start, end) in segments {
            collision.static_barriers.push(StaticBarrier {
                geo_id: GeoId::GeometryObject(object_id),
                start,
                end,
                min_y,
                max_y,
            });
        }
    }

    fn add_tiled_face(
        face: &crate::GeometryFace,
        assets: &Assets,
        vmchunk: &mut scenevm::Chunk,
        object_id: Uuid,
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
        let has_tile_overrides = !face.tiles.is_empty();
        let noise_cells = face
            .surface_noise
            .as_ref()
            .map(|noise| (noise.scale.max(1.0).sqrt() * 8.0).ceil().clamp(8.0, 128.0) as i32)
            .unwrap_or(1);
        let cells_x = if has_tile_overrides {
            range.x.ceil().max(1.0) as i32
        } else {
            range.x.ceil().max(noise_cells as f32).max(1.0) as i32
        };
        let cells_y = if has_tile_overrides {
            range.y.ceil().max(1.0) as i32
        } else {
            range.y.ceil().max(noise_cells as f32).max(1.0) as i32
        };
        for ty in 0..cells_y {
            for tx in 0..cells_x {
                let (x0, x1, y0, y1, tile_x, tile_y) = if has_tile_overrides {
                    let x0 = min_uv.x + tx as f32;
                    let x1 = (x0 + 1.0).min(max_uv.x);
                    let y0 = min_uv.y + ty as f32;
                    let y1 = (y0 + 1.0).min(max_uv.y);
                    (x0, x1, y0, y1, tx, ty)
                } else {
                    let x0 = min_uv.x + range.x * (tx as f32 / cells_x as f32);
                    let x1 = min_uv.x + range.x * ((tx + 1) as f32 / cells_x as f32);
                    let y0 = min_uv.y + range.y * (ty as f32 / cells_y as f32);
                    let y1 = min_uv.y + range.y * ((ty + 1) as f32 / cells_y as f32);
                    let tile_x = (x0 - min_uv.x).floor().max(0.0) as i32;
                    let tile_y = (y0 - min_uv.y).floor().max(0.0) as i32;
                    (x0, x1, y0, y1, tile_x, tile_y)
                };
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

                let source = face.tiles.get(&(tile_x, tile_y)).or(face.tile.as_ref());
                let tile_id = Self::face_tile_id(source, assets);
                let noise_tile_id = face
                    .surface_noise
                    .as_ref()
                    .and_then(|noise| noise.source.as_ref())
                    .map(|source| Self::face_tile_id(Some(source), assets));
                let uvs = Self::transformed_face_uvs(
                    face,
                    &[[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                    Some(&vertices_world),
                );
                if let (Some(noise), Some(noise_tile_id)) =
                    (face.surface_noise.as_ref(), noise_tile_id)
                {
                    let blend_weights = vertices_world
                        .iter()
                        .map(|world| Self::surface_noise_weight(noise, *world))
                        .collect();
                    vmchunk.add_poly_3d_blended(
                        GeoId::GeometryObject(object_id),
                        tile_id,
                        noise_tile_id,
                        vertices,
                        uvs,
                        blend_weights,
                        vec![(0, 1, 2), (0, 2, 3)],
                        0,
                        true,
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
            if !object.visible {
                continue;
            }
            let Some(bbox) = object.bbox() else {
                continue;
            };
            if !bbox.intersects(&chunk.bbox) || !chunk.bbox.contains(bbox.center()) {
                continue;
            }
            let object_center = Self::object_center(object, bbox);

            for face in &object.faces {
                if face.indices.len() < 3 {
                    continue;
                }

                let tile_id = Self::face_tile_id(face.tile.as_ref(), assets);
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
                if face.auto_uv && (!face.tiles.is_empty() || face.surface_noise.is_some()) {
                    let face_uvs = uvs
                        .iter()
                        .map(|uv| Vec2::new(uv[0], uv[1]))
                        .collect::<Vec<_>>();
                    if Self::add_tiled_face(
                        face,
                        assets,
                        vmchunk,
                        object.id,
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
                    .map(|source| Self::face_tile_id(Some(source), assets));
                if let (Some(noise), Some(noise_tile_id)) =
                    (face.surface_noise.as_ref(), noise_tile_id)
                {
                    let blend_weights = world_points
                        .iter()
                        .map(|world| Self::surface_noise_weight(noise, *world))
                        .collect();
                    vmchunk.add_poly_3d_blended(
                        GeoId::GeometryObject(object.id),
                        tile_id,
                        noise_tile_id,
                        vertices,
                        uvs,
                        blend_weights,
                        indices,
                        0,
                        true,
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
            for face in &object.faces {
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
                    Self::add_face_barriers(&mut collision, object.id, &world_points);
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
    fn surface_noise_tessellates_render_mesh_without_topology_change() {
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
        assert!(polys.len() > original_face_count);
        assert_eq!(map.geometry_objects[0].faces.len(), original_face_count);
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
}
