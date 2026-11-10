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
    sampler: Arc<TerrainSampler>,
    excluded: Vec<(BBox, Vec<[Vec2<f32>; 3]>)>,
}
impl TerrainSurface {
    pub fn cached(map: &Map) -> Option<Arc<Self>> {
        type Cached = (uuid::Uuid, u32, bool, Option<Arc<TerrainSurface>>);
        thread_local! {static CACHE:std::cell::RefCell<Option<Cached>>=const {std::cell::RefCell::new(None)};}
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
            sampler: Arc::new(TerrainGenerator::new(config).sampler(map)),
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
        self.sampler.sample(p)
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
        Some(self.sampler.height_on_grid(p, self.subdivisions))
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
