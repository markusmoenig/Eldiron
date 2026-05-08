use crate::collision_world::ChunkCollision;
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
        let cells_x = (max_uv.x - min_uv.x).ceil().max(1.0) as i32;
        let cells_y = (max_uv.y - min_uv.y).ceil().max(1.0) as i32;
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
                for uv in corners_uv {
                    let Some(world) = Self::world_from_face_uv(uv, face_uvs, world_points) else {
                        vertices.clear();
                        break;
                    };
                    let world = world + render_nudge;
                    vertices.push([world.x, world.y, world.z, 1.0]);
                }
                if vertices.len() != 4 {
                    continue;
                }

                let source = face.tiles.get(&(tx, ty)).or(face.tile.as_ref());
                let tile_id = Self::face_tile_id(source, assets);
                vmchunk.add_poly_3d(
                    GeoId::GeometryObject(object_id),
                    tile_id,
                    vertices,
                    vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                    vec![(0, 1, 2), (0, 2, 3)],
                    0,
                    true,
                );
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
            let Some(bbox) = object.bbox() else {
                continue;
            };
            if !bbox.intersects(&chunk.bbox) || !chunk.bbox.contains(bbox.center()) {
                continue;
            }
            let object_center = {
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
            };

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

    fn build_collision(
        &mut self,
        _map: &Map,
        _assets: &Assets,
        _chunk_origin: Vec2<i32>,
        _chunk_size: i32,
    ) -> ChunkCollision {
        ChunkCollision::new()
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }
}
