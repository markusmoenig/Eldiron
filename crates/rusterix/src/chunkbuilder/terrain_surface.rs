//! Camera-independent sampled surface of the shared terrain rules. Grid corners
//! and triangle interpolation are identical for rendering, walking and picking.
use super::terrain_generator::{TerrainConfig, TerrainGenerator, TerrainSampler};
use crate::{BBox, Map};
use std::sync::Arc;
use vek::{Vec2, Vec3};

#[derive(Clone)]
pub struct TerrainSurface {
    pub bounds: BBox,
    pub subdivisions: u32,
    pub max_height: f32,
    sampler: Option<Arc<TerrainSampler>>,
    sampled: Option<SampledTerrain>,
    excluded: Vec<(BBox, Vec<[Vec2<f32>; 3]>)>,
}
type Cached = (uuid::Uuid, u32, bool, Option<Arc<TerrainSurface>>);
thread_local! {static CACHE:std::cell::RefCell<Option<Cached>>=const {std::cell::RefCell::new(None)};}
// Wire content contains sampled geometry only, never an authored Map or its
// server properties/scripts. Sampling is exact at the grid corners used by
// collision, picking and rendered triangles.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SampledTerrain {
    origin: [i32; 2],
    width: usize,
    height: usize,
    values: Vec<f32>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct TerrainWire {
    bounds: BBox,
    subdivisions: u32,
    max_height: f32,
    excluded: Vec<(BBox, Vec<[Vec2<f32>; 3]>)>,
    samples: SampledTerrain,
}
impl serde::Serialize for TerrainSurface {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let scale = self.subdivisions as f32;
        let origin = [
            (self.bounds.min.x * scale).floor() as i32 - 1,
            (self.bounds.min.y * scale).floor() as i32 - 1,
        ];
        let width =
            ((self.bounds.max.x * scale).ceil() as i64 - origin[0] as i64 + 2).max(0) as usize;
        let height =
            ((self.bounds.max.y * scale).ceil() as i64 - origin[1] as i64 + 2).max(0) as usize;
        if width.checked_mul(height).is_none_or(|n| n > 1_000_000) {
            return Err(serde::ser::Error::custom(
                "Terrain content exceeds sample limit",
            ));
        }
        let mut values = Vec::with_capacity(width * height);
        for z in 0..height {
            for x in 0..width {
                values.push(self.sample(Vec2::new(
                    (origin[0] as f32 + x as f32) / scale,
                    (origin[1] as f32 + z as f32) / scale,
                )));
            }
        }
        TerrainWire {
            bounds: self.bounds,
            subdivisions: self.subdivisions,
            max_height: self.max_height,
            excluded: self.excluded.clone(),
            samples: SampledTerrain {
                origin,
                width,
                height,
                values,
            },
        }
        .serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for TerrainSurface {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = TerrainWire::deserialize(deserializer)?;
        let n = wire.samples.width.checked_mul(wire.samples.height);
        if !(1..=8).contains(&wire.subdivisions)
            || n.is_none_or(|n| n > 1_000_000 || n == 0 || n != wire.samples.values.len())
            || wire.samples.values.iter().any(|h| !h.is_finite())
        {
            return Err(serde::de::Error::custom("Invalid sampled terrain"));
        }
        Ok(Self {
            bounds: wire.bounds,
            subdivisions: wire.subdivisions,
            max_height: wire.max_height,
            excluded: wire.excluded,
            sampler: None,
            sampled: Some(wire.samples),
        })
    }
}
impl TerrainSurface {
    fn sample(&self, p: Vec2<f32>) -> f32 {
        if let Some(sampler) = &self.sampler {
            return sampler.sample(p);
        }
        let samples = self.sampled.as_ref().expect("terrain samples");
        let x = ((p.x * self.subdivisions as f32).round() as i64 - samples.origin[0] as i64)
            .clamp(0, samples.width as i64 - 1) as usize;
        let z = ((p.y * self.subdivisions as f32).round() as i64 - samples.origin[1] as i64)
            .clamp(0, samples.height as i64 - 1) as usize;
        samples.values[z * samples.width + x]
    }

    /// An immutable derived source can reuse authored IDs/revisions after undo
    /// or document replacement. Its CPU producer invalidates that map's local
    /// entry before rebuilding chunks; unrelated map entries are left alone.
    pub fn invalidate_cached(map: uuid::Uuid) {
        TerrainGenerator::invalidate_sample_cache(map);
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.as_ref().is_some_and(|entry| entry.0 == map) {
                *cache = None;
            }
        });
    }
    pub fn cached(map: &Map) -> Option<Arc<Self>> {
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let enabled = map.properties.get_bool_default("terrain_enabled", false);
            if cache
                .as_ref()
                .is_none_or(|c| (c.0, c.1, c.2) != (map.id, map.changed, enabled))
            {
                *cache = Some((
                    map.id,
                    map.changed,
                    enabled,
                    Self::from_map(map).map(Arc::new),
                ));
            }
            cache.as_ref().unwrap().3.clone()
        })
    }
    pub fn from_map(map: &Map) -> Option<Self> {
        if !map.properties.get_bool_default("terrain_enabled", false) {
            return None;
        }
        let config = TerrainConfig::for_map(map);
        let mut bounds = map.bbox();
        let mut max_height = 0_f32;
        for v in &map.vertices {
            if v.properties.get_bool_default("terrain_control", false) {
                max_height = max_height.max(v.z);
                if config.edge_falloff == 0. {
                    let r = v
                        .properties
                        .get_float_default("smoothness", config.smoothness)
                        .max(0.)
                        * 4.;
                    bounds.min = bounds.min.map2(v.as_vec2() - r, f32::min);
                    bounds.max = bounds.max.map2(v.as_vec2() + r, f32::max);
                }
            }
        }
        max_height += map
            .sectors
            .iter()
            .filter(|s| s.properties.get_int_default("terrain_mode", 0) == 2)
            .map(|s| s.properties.get_float_default("ridge_height", 1.).max(0.))
            .sum::<f32>();
        for l in &map.linedefs {
            if l.properties.get_bool_default("terrain_smooth", false) {
                for id in [l.start_vertex, l.end_vertex] {
                    if let Some(v) = map.find_vertex(id) {
                        max_height = max_height.max(v.z);
                    }
                }
            }
        }
        for sector in &map.sectors {
            if sector.properties.get_int_default("terrain_mode", 0) == 2 {
                let b = sector.bounding_box(map);
                let r = sector
                    .properties
                    .get_float_default("ridge_plateau_width", 0.)
                    .max(0.)
                    + sector
                        .properties
                        .get_float_default("ridge_falloff_distance", 5.)
                        .max(0.);
                bounds.min = bounds.min.map2(b.min - r, f32::min);
                bounds.max = bounds.max.map2(b.max + r, f32::max);
            }
        }
        for value in map.properties.values() {
            let cells: Vec<_> = match value {
                crate::Value::TileOverrides(tiles) => tiles.keys().copied().collect(),
                crate::Value::BlendOverrides(tiles) => tiles.keys().copied().collect(),
                _ => Vec::new(),
            };
            for (x, z) in cells {
                let p = Vec2::new(x as f32, z as f32);
                bounds.min = bounds.min.map2(p, f32::min);
                bounds.max = bounds.max.map2(p + 1., f32::max);
            }
        }
        super::terrain_shape::TerrainShapes::from_map(map)
            .expand_bounds(&mut bounds, &mut max_height);
        max_height += super::terrain_sculpt::TerrainSculpt::from_map(map).max_offset();
        // Complete cells keep rendering, ground queries and streaming consistent
        // when a guide's influence is narrower than one terrain grid cell.
        let grid = config.subdivisions as f32;
        bounds.min = (bounds.min * grid).map(f32::floor) / grid;
        bounds.max = (bounds.max * grid).map(f32::ceil) / grid;
        let excluded = map
            .sectors
            .iter()
            .filter(|s| s.properties.get_int_default("terrain_mode", 0) == 1)
            .filter_map(|s| {
                let (v, t) = s.generate_geometry(map)?;
                Some((
                    s.bounding_box(map),
                    t.into_iter()
                        .map(|(a, b, c)| [Vec2::from(v[a]), Vec2::from(v[b]), Vec2::from(v[c])])
                        .collect(),
                ))
            })
            .collect();
        Some(Self {
            bounds,
            subdivisions: config.subdivisions,
            max_height,
            sampler: Some(Arc::new(TerrainGenerator::new(config).sampler(map))),
            sampled: None,
            excluded,
        })
    }
    pub fn contains(&self, p: Vec2<f32>) -> bool {
        if !self.bounds.contains(p) {
            return false;
        }
        let n = self.subdivisions as f32;
        let a = (p * n).map(f32::floor) / n;
        let uv = (p - a) * n;
        let tri = if uv.x + uv.y <= 1. {
            [a, a + Vec2::new(0., 1. / n), a + Vec2::new(1. / n, 0.)]
        } else {
            [
                a + Vec2::new(1. / n, 0.),
                a + Vec2::new(0., 1. / n),
                a + 1. / n,
            ]
        };
        // Match the generator: exclude only facets fully inside a cutout.
        !self.excluded.iter().any(|(b, polygon)| {
            tri.iter()
                .all(|p| b.contains(*p) && polygon.iter().any(|t| barycentric(*t, *p).is_some()))
        })
    }

    pub fn grid_height(&self, p: Vec2<f32>) -> f32 {
        self.sample(p)
    }
    pub fn raycast(
        &self,
        origin: Vec3<f32>,
        direction: Vec3<f32>,
        distance: f32,
    ) -> Option<Vec3<f32>> {
        let end = origin + direction * distance;
        let a = Vec2::new(origin.x, origin.z);
        let b = Vec2::new(end.x, end.z);
        let clip = BBox::new(a.map2(b, f32::min) - 0.01, a.map2(b, f32::max) + 0.01);
        let mut best = None;
        let mut best_t = f32::NEG_INFINITY;
        self.triangles(clip, |tri| {
            let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
            let denominator = n.dot(direction);
            if denominator.abs() < 0.000001 {
                return;
            }
            let t = n.dot(tri[0] - origin) / denominator;
            if t < -0.00001 || t > distance + 0.00001 || t < best_t {
                return;
            }
            let p = origin + direction * t;
            if barycentric(tri.map(|v| Vec2::new(v.x, v.z)), Vec2::new(p.x, p.z)).is_some() {
                best = Some(p);
                best_t = t;
            }
        });
        best
    }
    pub fn height_at(&self, p: Vec2<f32>) -> Option<f32> {
        if !self.contains(p) {
            return None;
        }
        Some(TerrainSampler::height_on_grid_with(
            p,
            self.subdivisions,
            |corner| self.sample(corner),
        ))
    }

    /// Reuse exact corner evaluations during one caller's sampling pass. The
    /// borrow binds this bounded scratch cache to this immutable surface; no
    /// map revision, source pointer or cross-frame cache identity is needed.
    pub fn height_sampler(&self) -> impl FnMut(Vec2<f32>) -> Option<f32> + '_ {
        const MAX_CORNERS: usize = 32768;
        let mut corners = rustc_hash::FxHashMap::<[u32; 2], f32>::default();
        move |p| {
            if !self.contains(p) {
                return None;
            }
            Some(TerrainSampler::height_on_grid_with(
                p,
                self.subdivisions,
                |corner| {
                    let key = [corner.x.to_bits(), corner.y.to_bits()];
                    if let Some(&height) = corners.get(&key) {
                        return height;
                    }
                    let height = self.sample(corner);
                    if corners.len() < MAX_CORNERS {
                        corners.insert(key, height);
                    }
                    height
                },
            ))
        }
    }
    pub fn triangles(&self, clip: BBox, mut emit: impl FnMut([Vec3<f32>; 3])) {
        let n = self.subdivisions as f32;
        let min = self
            .bounds
            .min
            .map2(clip.min, f32::max)
            .map(|v| (v * n).floor() as i32);
        let max = self
            .bounds
            .max
            .map2(clip.max, f32::min)
            .map(|v| (v * n).ceil() as i32);
        if min.x >= max.x || min.y >= max.y {
            return;
        }
        // Sample each grid corner once, keeping two rows rather than a full
        // height map. This also speeds iso rasterization, shadows and ray picks.
        let row = |z: i32| {
            (min.x..=max.x)
                .map(|x| {
                    let p = Vec2::new(x as f32 / n, z as f32 / n);
                    Vec3::new(p.x, self.grid_height(p), p.y)
                })
                .collect::<Vec<_>>()
        };
        let mut previous = row(min.y);
        for z in min.y..max.y {
            let next = row(z + 1);
            for column in 0..previous.len() - 1 {
                for tri in [
                    [previous[column], next[column], previous[column + 1]],
                    [previous[column + 1], next[column], next[column + 1]],
                ] {
                    let center = (tri[0] + tri[1] + tri[2]) / 3.;
                    if self.contains(Vec2::new(center.x, center.z)) {
                        emit(tri);
                    }
                }
            }
            previous = next;
        }
    }
}

pub fn barycentric(t: [Vec2<f32>; 3], p: Vec2<f32>) -> Option<[f32; 3]> {
    let cross = |a: Vec2<f32>, b: Vec2<f32>| a.x * b.y - a.y * b.x;
    let area = cross(t[1] - t[0], t[2] - t[0]);
    if area.abs() < 0.0000001 {
        return None;
    }
    let a = cross(t[1] - p, t[2] - p) / area;
    let b = cross(t[2] - p, t[0] - p) / area;
    let c = 1. - a - b;
    (a >= -0.00001 && b >= -0.00001 && c >= -0.00001).then_some([a, b, c])
}

/// Display rasterization must give shared edges one owner. Geometric/ray
/// queries retain the inclusive `barycentric` contract above.
pub fn raster_barycentric(t: [Vec2<f32>; 3], p: Vec2<f32>) -> Option<[f32; 3]> {
    let weights = barycentric(t, p)?;
    let cross = |a: Vec2<f32>, b: Vec2<f32>| a.x * b.y - a.y * b.x;
    let positive = cross(t[1] - t[0], t[2] - t[0]) > 0.;
    for i in 0..3 {
        let (a, b) = if positive {
            (t[i], t[(i + 1) % 3])
        } else {
            (t[(i + 1) % 3], t[i])
        };
        let edge = cross(b - a, p - a);
        let owns = b.y < a.y || (b.y == a.y && b.x > a.x);
        if edge < -0.00001 || (edge.abs() <= 0.00001 && !owns) {
            return None;
        }
    }
    Some(weights)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{MapCamera, Value, Vertex};
    pub(crate) fn hill() -> Map {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        map.vertices = vec![Vertex::new(0, -8., -8.), Vertex::new(1, 8., 8.)];
        let mut peak = Vertex::new(2, 0.5, 0.5);
        peak.z = 1.2;
        peak.properties.set("terrain_control", Value::Bool(true));
        peak.properties.set("smoothness", Value::Float(1.));
        map.vertices.push(peak);
        map
    }
    #[test]
    fn public_terrain_roundtrip_preserves_collision_facets_without_authored_map() {
        let map = hill();
        let field = TerrainSurface::from_map(&map).unwrap();
        let bytes = serde_json::to_vec(&field).unwrap();
        let restored: TerrainSurface = serde_json::from_slice(&bytes).unwrap();
        assert!(restored.sampler.is_none());
        for x in -80..80 {
            for z in -80..80 {
                let p = Vec2::new(x as f32 / 10., z as f32 / 10.);
                assert_eq!(
                    field.height_at(p).map(f32::to_bits),
                    restored.height_at(p).map(f32::to_bits)
                );
            }
        }
    }
    #[test]
    fn local_corner_samples_preserve_exact_facets_and_source_independence() {
        let mut map = hill();
        map.vertices[0].x = -16.;
        map.vertices[0].y = -16.;
        map.vertices[1].x = 16.;
        map.vertices[1].y = 16.;
        for subdivisions in [1, 3, 4, 8] {
            map.properties
                .set("terrain_subdivisions", Value::Int(subdivisions));
            let field = TerrainSurface::from_map(&map).unwrap();
            let mut sample = field.height_sampler();
            let mut valid = 0;
            // The fine grid visits more corners than the bounded cache holds.
            // Repeat it to exercise hits and the uncached overflow path alike.
            for _ in 0..2 {
                for x in -160..=160 {
                    for y in -160..=160 {
                        let p = Vec2::new(x as f32 / 10., y as f32 / 10.);
                        let expected = field.height_at(p).map(f32::to_bits);
                        valid += usize::from(expected.is_some());
                        assert_eq!(sample(p).map(f32::to_bits), expected);
                    }
                }
            }
            assert!(valid > 10_000);
            for p in [
                Vec2::new(-0., 0.),
                Vec2::zero(),
                Vec2::new(100., 100.),
                Vec2::new(f32::NAN, 0.),
            ] {
                assert_eq!(
                    sample(p).map(f32::to_bits),
                    field.height_at(p).map(f32::to_bits)
                );
            }
        }
        let before = TerrainSurface::from_map(&map).unwrap();
        let mut old = before.height_sampler();
        let p = Vec2::new(0.5, 0.5);
        let original = old(p).unwrap();
        map.vertices[2].z = 2.4;
        let after = TerrainSurface::from_map(&map).unwrap();
        let mut new = after.height_sampler();
        assert_eq!(old(p).unwrap().to_bits(), original.to_bits());
        assert_eq!(
            new(p).map(f32::to_bits),
            after.height_at(p).map(f32::to_bits)
        );
        assert_ne!(new(p).unwrap().to_bits(), original.to_bits());
    }
    #[test]
    fn terrain_heights_and_triangles_are_camera_independent_and_match_shared_sampler() {
        let mut map = hill();
        let config = TerrainConfig::for_map(&map);
        for camera in [
            MapCamera::TwoD,
            MapCamera::TwoDIso,
            MapCamera::ThreeDIso,
            MapCamera::ThreeDFirstPerson,
        ] {
            map.camera = camera;
            let field = TerrainSurface::from_map(&map).unwrap();
            assert!((field.height_at(Vec2::new(0.5, 0.5)).unwrap() - 1.2).abs() < 0.0001);
            field.triangles(BBox::new(Vec2::new(-1., -1.), Vec2::new(2., 2.)), |tri| {
                for v in tri {
                    assert!(
                        (v.y - TerrainGenerator::sample_height_at(
                            &map,
                            Vec2::new(v.x, v.z),
                            &config
                        ))
                        .abs()
                            < 0.00001
                    );
                }
                let p = tri[0] * 0.21 + tri[1] * 0.32 + tri[2] * 0.47;
                assert!((field.height_at(Vec2::new(p.x, p.z)).unwrap() - p.y).abs() < 0.00001);
                assert!(
                    (TerrainGenerator::sample_height_at(&map, Vec2::new(p.x, p.z), &config) - p.y)
                        .abs()
                        < 0.00001
                );
            });
        }
        let before = TerrainSurface::cached(&map).unwrap();
        assert!(
            (TerrainGenerator::sample_height_at(&map, Vec2::new(0.5, 0.5), &config) - 1.2).abs()
                < 0.0001
        );
        TerrainSurface::invalidate_cached(uuid::Uuid::new_v4());
        assert!(Arc::ptr_eq(&before, &TerrainSurface::cached(&map).unwrap()));
        // Derived publications/undo can reuse the serialized revision. Explicit
        // source invalidation must replace terrain, not reuse the prior hill.
        map.vertices[2].z = 2.4;
        TerrainSurface::invalidate_cached(map.id);
        let after = TerrainSurface::cached(&map).unwrap();
        assert!(!Arc::ptr_eq(&before, &after));
        assert!((after.height_at(Vec2::new(0.5, 0.5)).unwrap() - 2.4).abs() < 0.0001);
        assert!(
            (TerrainGenerator::sample_height_at(&map, Vec2::new(0.5, 0.5), &config) - 2.4).abs()
                < 0.0001
        );
    }
    #[test]
    fn shared_smooth_hill_walks_both_ways_and_iso_picking_roundtrips() {
        let map = hill();
        let elevation = crate::isometric::elevation::ElevationMap::from_map(&map);
        let mut pos = Vec3::new(-4., 0., 0.5);
        let mut previous = pos.y;
        for _ in 0..90 {
            let (next, blocked) =
                elevation.move_distance(pos, Vec2::new(0.05, 0.), 0.22, 0.35, 0.5);
            assert!(!blocked);
            assert!(next.y + 0.0001 >= previous);
            assert!((next.y - previous).abs() < 0.03);
            pos = next;
            previous = pos.y;
        }
        assert!((pos.y - 1.2).abs() < 0.001);
        let projection = crate::isometric::Projection {
            origin: Vec2::new(400., 300.),
            unit: 40.,
        };
        for p in [
            Vec2::new(-1.2, 0.4),
            Vec2::new(0.5, 0.5),
            Vec2::new(1.3, 0.9),
        ] {
            let h = elevation.height_at(p);
            let hit = elevation.pick(projection, projection.screen(p, h));
            assert!(
                (hit - Vec3::new(p.x, h, p.y)).magnitude() < 0.001,
                "{p:?}: {hit:?}"
            );
        }
        let (back, blocked) = elevation.move_distance(pos, Vec2::new(-4.5, 0.), 0.22, 0.35, 0.5);
        assert!(!blocked && back.y < 0.0001);
    }
}
