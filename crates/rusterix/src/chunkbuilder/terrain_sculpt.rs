//! Sparse, world-space sculpt offsets layered onto the shared terrain rules.
//! Stored as ordinary HeightPoints so old projects need no migration.
use super::terrain_generator::{TerrainConfig, TerrainGenerator};
use crate::{BBox, HeightControlPoint, Map, Value};
use rustc_hash::FxHashMap;
use vek::Vec2;

pub const KEY: &str = "terrain_sculpt";
pub const RESOLUTION: f32 = 4.;

#[derive(Clone, Default)]
pub struct TerrainSculpt {
    offsets: FxHashMap<(i32, i32), f32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SculptMode {
    Raise,
    Lower,
    Smooth,
}

impl TerrainSculpt {
    pub fn from_map(map: &Map) -> Self {
        let mut result = Self::default();
        if let Some(points) = map.properties.get_height_points(KEY) {
            for p in points {
                if p.height.is_finite() && p.position.iter().all(|v| v.is_finite()) {
                    result.offsets.insert(
                        (
                            (p.position[0] * RESOLUTION).round() as i32,
                            (p.position[1] * RESOLUTION).round() as i32,
                        ),
                        p.height,
                    );
                }
            }
        }
        result
    }

    pub fn sample(&self, p: Vec2<f32>) -> f32 {
        let p = p * RESOLUTION;
        let x = p.x.floor() as i32;
        let z = p.y.floor() as i32;
        let u = p.x - x as f32;
        let v = p.y - z as f32;
        let at = |x, z| self.offsets.get(&(x, z)).copied().unwrap_or(0.);
        (at(x, z) * (1. - u) + at(x + 1, z) * u) * (1. - v)
            + (at(x, z + 1) * (1. - u) + at(x + 1, z + 1) * u) * v
    }

    pub fn bounds(map: &Map) -> Option<BBox> {
        let points = map.properties.get_height_points(KEY)?;
        let mut min = Vec2::broadcast(f32::INFINITY);
        let mut max = Vec2::broadcast(f32::NEG_INFINITY);
        for p in points {
            if p.height.is_finite() && p.position.iter().all(|v| v.is_finite()) {
                let at = Vec2::from(p.position);
                min = min.map2(at, f32::min);
                max = max.map2(at, f32::max);
            }
        }
        min.x
            .is_finite()
            .then(|| BBox::new(min - 1. / RESOLUTION, max + 1. / RESOLUTION))
    }

    pub fn max_offset(&self) -> f32 {
        self.offsets.values().copied().fold(0., f32::max)
    }

    /// Amount is height per dab for raise/lower; smooth uses it as a blend rate.
    /// Read a frozen sampler for the entire dab, making smoothing order independent.
    pub fn dab(
        map: &mut Map,
        center: Vec2<f32>,
        radius: f32,
        amount: f32,
        mode: SculptMode,
    ) -> bool {
        if !center.x.is_finite()
            || !center.y.is_finite()
            || !radius.is_finite()
            || !amount.is_finite()
            || amount <= 0.
        {
            return false;
        }
        let radius = radius.clamp(0.5, 16.);
        let sampler = TerrainGenerator::new(TerrainConfig::for_map(map)).sampler(map);
        let mut sculpt = Self::from_map(map);
        let min = ((center - radius) * RESOLUTION).map(|v| v.floor() as i32);
        let max = ((center + radius) * RESOLUTION).map(|v| v.ceil() as i32);
        let mut changed = false;
        for z in min.y..=max.y {
            for x in min.x..=max.x {
                let p = Vec2::new(x as f32, z as f32) / RESOLUTION;
                let t = (p - center).magnitude() / radius;
                if t >= 1. {
                    continue;
                }
                let falloff = (1. - t * t).powi(2);
                let height = sampler.sample(p);
                let target = match mode {
                    SculptMode::Raise => (height + amount * falloff).min(128.),
                    SculptMode::Lower => (height - amount * falloff).max(0.),
                    SculptMode::Smooth => {
                        let mut average = 0.;
                        for dz in -1..=1 {
                            for dx in -1..=1 {
                                average += sampler
                                    .sample(p + Vec2::new(dx as f32, dz as f32) / RESOLUTION);
                            }
                        }
                        height + (average / 9. - height) * (amount * falloff).min(1.)
                    }
                };
                let delta = target - height;
                if delta.abs() < 0.000001 {
                    continue;
                }
                let offset = sculpt.offsets.get(&(x, z)).copied().unwrap_or(0.) + delta;
                if offset.abs() < 0.000001 {
                    sculpt.offsets.remove(&(x, z));
                } else {
                    sculpt.offsets.insert((x, z), offset);
                }
                changed = true;
            }
        }
        if changed {
            let mut points: Vec<_> = sculpt.offsets.into_iter().collect();
            points.sort_by_key(|(cell, _)| *cell);
            if points.is_empty() {
                map.properties.remove(KEY);
            } else {
                map.properties.set(
                    KEY,
                    Value::HeightPoints(
                        points
                            .into_iter()
                            .map(|((x, z), height)| HeightControlPoint {
                                position: [x as f32 / RESOLUTION, z as f32 / RESOLUTION],
                                height,
                            })
                            .collect(),
                    ),
                );
            }
            map.properties.set("terrain_enabled", Value::Bool(true));
            if !map.properties.contains("terrain_edge_falloff") {
                map.properties.set("terrain_edge_falloff", Value::Float(0.));
            }
            let n = map
                .properties
                .get_int_default("terrain_subdivisions", 1)
                .max(4);
            map.properties.set("terrain_subdivisions", Value::Int(n));
            map.changed = map.changed.wrapping_add(1);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::super::terrain_surface::TerrainSurface;
    use super::*;
    use crate::{Assets, Chunk, ChunkBuilder, MapCamera, PixelSource, Texture, Tile};

    #[test]
    fn brush_raises_lowers_and_smooths_existing_hills_without_changing_materials() {
        let mut map = super::super::terrain_surface::tests::hill();
        let center = Vec2::new(0.5, 0.5);
        let material = uuid::Uuid::new_v4();
        map.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(material)),
        );
        let original = map.vertices.clone();
        let sample =
            |map: &Map, p| TerrainGenerator::sample_height_at(map, p, &TerrainConfig::for_map(map));
        let before = sample(&map, center);
        assert!(TerrainSculpt::dab(
            &mut map,
            center,
            2.,
            0.4,
            SculptMode::Raise
        ));
        assert!((sample(&map, center) - before - 0.4).abs() < 0.0001);
        assert_eq!(sample(&map, center + Vec2::new(5., 0.)), 0.);
        assert!(TerrainSculpt::dab(
            &mut map,
            center,
            2.,
            0.2,
            SculptMode::Lower
        ));
        assert!((sample(&map, center) - before - 0.2).abs() < 0.0001);
        let peak = sample(&map, center);
        for _ in 0..30 {
            assert!(TerrainSculpt::dab(
                &mut map,
                center,
                2.,
                0.75,
                SculptMode::Smooth
            ));
        }
        assert!(sample(&map, center) < peak - 0.05);
        for _ in 0..20 {
            TerrainSculpt::dab(&mut map, center, 2., 0.5, SculptMode::Lower);
        }
        assert_eq!(sample(&map, center), 0.);
        assert_eq!(map.vertices, original);
        assert_eq!(
            map.properties.get_source("default_terrain_tile"),
            Some(&PixelSource::TileId(material))
        );
    }

    #[test]
    fn sculpt_roundtrips_and_matches_shared_generated_triangles_and_walking() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        let center = Vec2::new(-0.5, 0.5);
        for _ in 0..10 {
            TerrainSculpt::dab(&mut map, center, 3., 0.1, SculptMode::Raise);
        }
        let json = serde_json::to_string(&map).unwrap();
        let restored: Map = serde_json::from_str(&json).unwrap();
        let surface = TerrainSurface::from_map(&restored).unwrap();
        assert!((surface.height_at(center).unwrap() - 1.).abs() < 0.0001);
        let mut assets = Assets::default();
        let tile = Tile::from_texture(Texture::new(vec![80, 110, 70, 255], 1, 1));
        let id = tile.id;
        assets.set_tiles([(id, tile)].into_iter().collect());
        let generator = TerrainGenerator::new(TerrainConfig::for_map(&restored));
        for x in [-4, 0] {
            let meshes = generator
                .generate(
                    &restored,
                    &Chunk::new(Vec2::new(x, -4), 4),
                    &assets,
                    id,
                    None,
                )
                .unwrap();
            for (_, vertices, _, _) in meshes {
                for v in vertices {
                    assert!((surface.grid_height(Vec2::new(v.x, v.z)) - v.y).abs() < 0.0001);
                }
            }
        }
        let elevation = crate::isometric::elevation::ElevationMap::from_map(&restored);
        let (top, blocked) = elevation.move_distance(
            vek::Vec3::new(-4., 0., 0.5),
            Vec2::new(3.5, 0.),
            0.22,
            0.35,
            0.5,
        );
        assert!(!blocked && (top.y - 1.).abs() < 0.0001);
        let (back, blocked) = elevation.move_distance(top, Vec2::new(-3.5, 0.), 0.22, 0.35, 0.5);
        assert!(!blocked && back.y < 0.0001);
        assert!(restored.vertices.is_empty());
        assert!(restored.bbox().contains(center));
        let mut physical = restored.clone();
        physical.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(id)),
        );
        physical.camera = MapCamera::ThreeDFirstPerson;
        let collision =
            super::super::d3chunkbuilder::D3ChunkBuilder::build_collision_world(&physical, &assets);
        assert!((collision.get_floor_height(center).unwrap() - 1.).abs() < 0.0001);
        let mut builder = super::super::geometry_object_builder::GeometryObjectBuilder::new();
        let mut chunk = Chunk::new(Vec2::new(-4, 0), 4);
        let mut render = scenevm::Chunk::default();
        builder.build(&physical, &assets, &mut chunk, &mut render);
        assert!(
            !render.polys3d_map.is_empty(),
            "active geometry builder must render the shared terrain"
        );
        for poly in render.polys3d_map.values().flatten() {
            for v in &poly.vertices {
                assert!((surface.grid_height(Vec2::new(v[0], v[2])) - v[1]).abs() < 0.0001);
            }
        }
        let floor = builder.build_collision(&physical, &assets, Vec2::new(-1, 0), 4);
        assert!(
            !floor.walkable_floors.is_empty(),
            "active geometry editor collision must include terrain"
        );
    }
}
