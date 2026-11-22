//! Solid, walkable 2D iso terrain. Heights belong to placements, not sprite pixels.
use crate::{BBox, Map};
use theframework_core::prelude::*;

pub const STEP_UP: f32 = 0.35;
pub const STEP_DOWN: f32 = 0.5;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Surface {
    bounds: BBox,
    triangles: Vec<[Vec2<f32>; 3]>,
    height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MapCamera, Value};

    fn top(map: &mut Map, x: f32, z: f32, w: f32, d: f32, height: f32) -> u32 {
        map.possible_polygon.clear();
        let v =
            [[x, z], [x + w, z], [x + w, z + d], [x, z + d]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let s = map.find_sector_mut(id).unwrap();
        s.properties.set("iso_walkable", Value::Bool(true));
        s.properties.set("floor_height", Value::Float(height));
        id
    }

    #[test]
    fn climbs_stairs_both_ways_but_blocks_cliffs() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        for x in 1..=4 {
            top(&mut map, x as f32, 0., 1., 2., x as f32 * 0.25);
        }
        top(&mut map, 5., 0., 2., 2., 1.);
        let h = ElevationMap::from_map(&map);
        let start = Vec3::new(0.5, 0., 0.5);
        let (up, blocked) = h.move_distance(start, Vec2::new(5., 0.), 0.22, STEP_UP, STEP_DOWN);
        assert!(!blocked);
        assert_eq!(up, Vec3::new(5.5, 1., 0.5));
        let (down, blocked) = h.move_distance(up, Vec2::new(-5., 0.), 0.22, STEP_UP, STEP_DOWN);
        assert!(!blocked);
        assert_eq!(down, start);
        let (edge, blocked) = h.move_distance(up, Vec2::new(0., 2.), 0.22, STEP_UP, STEP_DOWN);
        assert!(blocked);
        assert!(edge.z < 1.8);
        assert_eq!(edge.y, 1.);
        assert!(!h.can_reach(Vec2::new(5.5, 2.5), Vec2::new(5.5, 0.5), 0.22));
    }

    #[test]
    fn picks_raised_tops_and_preserves_flat_maps() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        top(&mut map, 2., 2., 3., 3., 0.75);
        let h = ElevationMap::from_map(&map);
        let projection = super::super::Projection {
            origin: Vec2::new(400., 300.),
            unit: 40.,
        };
        let p = Vec2::new(3., 3.);
        assert_eq!(h.height_at(p), 0.75);
        let picked = h.pick(projection, projection.screen(p, 0.75));
        assert!((picked - Vec3::new(p.x, 0.75, p.y)).magnitude() < 0.0001);
        map.camera = MapCamera::TwoD;
        assert!(ElevationMap::from_map(&map).is_empty());
    }

    #[test]
    fn traverses_both_routes_at_the_same_overpass_without_switching_layers() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        let deck = top(&mut map, 0., 0., 3., 1., 2.43);
        map.find_sector_mut(deck)
            .unwrap()
            .properties
            .set("iso_overpass", Value::Bool(true));
        top(&mut map, -2., 0., 2., 1., 2.43);
        top(&mut map, 3., 0., 2., 1., 2.43);
        let mini = map.as_mini(&FxHashSet::default());
        let worlds = crate::CollisionWorld::new(16);
        let assets = crate::Assets::default();
        let world = crate::movement::MovementWorld {
            map: &map,
            assets: &assets,
            mapmini: &mini,
            collision_world: &worlds,
            mode: crate::movement::CollisionMode::Tile,
            diagnostics: None,
        };
        for (start, end) in [
            (Vec3::new(-0.5, 2.43, 0.5), Vec3::new(3.5, 2.43, 0.5)),
            (Vec3::new(1.5, 0., -0.5), Vec3::new(1.5, 0., 1.5)),
        ] {
            let mut actor = crate::Entity::default();
            actor.position = start;
            actor.set_attribute("radius", Value::Float(0.22));
            let direct = crate::movement::move_by(
                &world,
                &mut actor,
                Vec2::new(end.x - start.x, end.z - start.z),
                0,
            );
            assert!(!direct.geometry_blocked);
            assert_eq!(actor.position, end);

            let mut position = start;
            for _ in 0..100 {
                let (next, arrived) = mini.move_towards_elevation(
                    position,
                    Vec2::new(end.x, end.z),
                    None,
                    0.12,
                    0.21,
                );
                position = next;
                assert_eq!(position.y, start.y);
                if arrived {
                    break;
                }
            }
            assert_eq!(position, end);
        }
        let (_, blocked) = mini.elevation.move_distance(
            Vec3::new(1.5, 2.43, 0.5),
            Vec2::new(0., 1.),
            0.21,
            STEP_UP,
            STEP_DOWN,
        );
        assert!(blocked);

        top(&mut map, -3.3, 0., 0.45, 1., 0.27);
        let mini = map.as_mini(&FxHashSet::default());
        let mut position = Vec3::new(-3.10, 0.27, 0.5);
        let goal = Vec2::new(-3.5, 0.5);
        for _ in 0..20 {
            let (next, arrived) = mini.move_towards_elevation(position, goal, None, 0.06, 0.21);
            position = next;
            if arrived {
                break;
            }
        }
        assert_eq!(position, Vec3::new(-3.5, 0., 0.5));
    }

    #[test]
    fn path_uses_stairs_instead_of_cliff_and_unreachable_search_terminates() {
        let mut map = Map::default();
        map.camera = MapCamera::TwoDIso;
        for x in 1..=4 {
            top(&mut map, x as f32, 0., 1., 2., x as f32 * 0.25);
        }
        top(&mut map, 5., 0., 2., 2., 1.);
        top(&mut map, 10., 10., 1., 1., 4.);
        let mini = map.as_mini(&FxHashSet::default());
        let mut p = Vec2::new(5.5, 3.5);
        let goal = Vec2::new(5.5, 0.5);
        let mut climbed = false;
        for _ in 0..800 {
            let next = mini.move_towards(p, goal, 0.05, 0.22, 1.).0;
            assert!(mini.elevation.can_reach(p, next, 0.22));
            climbed |= mini.elevation.height_at(next) > 0.;
            p = next;
            if (p - goal).magnitude() < 0.01 {
                break;
            }
        }
        assert!(climbed);
        assert!((p - goal).magnitude() < 0.01, "{p:?}");
        assert_eq!(
            mini.move_towards(p, Vec2::new(10.5, 10.5), 0.1, 0.22, 1.).0,
            p
        );
        assert_eq!(
            mini.close_in(p, Vec2::new(10.5, 10.5), 0.2, 0.1, 0.22, 1.)
                .0,
            p
        );
    }
}
impl Surface {
    fn contains(&self, p: Vec2<f32>) -> bool {
        if !self.bounds.contains(p) {
            return false;
        }
        self.triangles.iter().any(|t| {
            let edge =
                |a: Vec2<f32>, b: Vec2<f32>| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
            let e = [edge(t[0], t[1]), edge(t[1], t[2]), edge(t[2], t[0])];
            e.iter().all(|v| *v >= -0.00001) || e.iter().all(|v| *v <= 0.00001)
        })
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ElevationMap {
    surfaces: Vec<Surface>,
    #[serde(default)]
    overpasses: Vec<Surface>,
    pub terrain: Option<std::sync::Arc<crate::chunkbuilder::terrain_surface::TerrainSurface>>,
}
impl ElevationMap {
    pub fn from_map(map: &Map) -> Self {
        if map.camera != crate::MapCamera::TwoDIso {
            return Self::default();
        }
        let mut elevation = Self {
            terrain: crate::chunkbuilder::terrain_surface::TerrainSurface::from_map(map)
                .map(std::sync::Arc::new),
            ..Self::default()
        };
        for (surface, overpass) in map
            .sectors
            .iter()
            .filter(|s| s.properties.get_bool_default("iso_walkable", false))
            .filter_map(|s| {
                let (vertices, indices) = s.generate_geometry(map)?;
                let height = s.properties.get_float_default("floor_height", 0.);
                if !height.is_finite() {
                    return None;
                }
                let surface = Surface {
                    bounds: s.bounding_box(map),
                    height,
                    triangles: indices
                        .into_iter()
                        .map(|(a, b, c)| {
                            [
                                Vec2::from(vertices[a]),
                                Vec2::from(vertices[b]),
                                Vec2::from(vertices[c]),
                            ]
                        })
                        .collect(),
                };
                Some((
                    surface,
                    s.properties.get_bool_default("iso_overpass", false),
                ))
            })
        {
            if overpass {
                elevation.overpasses.push(surface);
            } else {
                elevation.surfaces.push(surface);
            }
        }
        elevation
    }
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty() && self.overpasses.is_empty() && self.terrain.is_none()
    }
    pub fn has_overpasses(&self) -> bool {
        !self.overpasses.is_empty()
    }

    /// The highest authored walkable surface, including elevated crossings.
    pub fn height_at(&self, p: Vec2<f32>) -> f32 {
        self.overpasses
            .iter()
            .filter(|surface| surface.contains(p))
            .fold(self.solid_height_at(p), |highest, surface| {
                highest.max(surface.height)
            })
    }

    /// Keep an actor on the surface nearest their current height where surfaces overlap.
    pub fn height_near(&self, p: Vec2<f32>, reference_y: f32) -> f32 {
        let ground = self.solid_height_at(p);
        self.overpasses
            .iter()
            .filter(|surface| surface.height > ground && surface.contains(p))
            .fold(ground, |nearest, surface| {
                if (surface.height - reference_y).abs() < (nearest - reference_y).abs() {
                    surface.height
                } else {
                    nearest
                }
            })
    }

    fn solid_height_at(&self, p: Vec2<f32>) -> f32 {
        self.surfaces
            .iter()
            .filter(|s| s.contains(p))
            .map(|s| s.height)
            .max_by(f32::total_cmp)
            .unwrap_or(0.)
            .max(
                self.terrain
                    .as_ref()
                    .and_then(|t| t.height_at(p))
                    .unwrap_or(0.),
            )
    }
    pub fn move_distance(
        &self,
        start: Vec3<f32>,
        delta: Vec2<f32>,
        radius: f32,
        up: f32,
        down: f32,
    ) -> (Vec3<f32>, bool) {
        let mut last = start;
        let steps = (delta.magnitude() / 0.05).ceil().max(1.) as usize;
        let probes = [
            Vec2::zero(),
            Vec2::new(radius, 0.),
            Vec2::new(-radius, 0.),
            Vec2::new(0., radius),
            Vec2::new(0., -radius),
        ];
        for i in 1..=steps {
            let p = Vec2::new(start.x, start.z) + delta * (i as f32 / steps as f32);
            if probes.iter().any(|probe| {
                let change = self.height_near(p + *probe, last.y) - last.y;
                change > up + 0.001 || change < -down - 0.001
            }) {
                return (last, true);
            }
            last = Vec3::new(p.x, self.height_near(p, last.y), p.y);
        }
        (last, false)
    }
    pub fn can_reach(&self, from: Vec2<f32>, to: Vec2<f32>, radius: f32) -> bool {
        if self.is_empty() {
            return true;
        }
        !self
            .move_distance(
                Vec3::new(from.x, self.height_at(from), from.y),
                to - from,
                radius,
                STEP_UP,
                STEP_DOWN,
            )
            .1
    }
    /// Pick the visible walkable top along the iso view ray.
    pub fn pick(&self, projection: super::Projection, pixel: Vec2<f32>) -> Vec3<f32> {
        let base = projection.ground(pixel);
        let ray = Vec2::new(-1.2247449, 1.2247449);
        let mut best = Vec3::new(base.x, 0., base.y);
        let mut depth = base.y - base.x;
        for (surfaces, overpass) in [(&self.surfaces, false), (&self.overpasses, true)] {
            for s in surfaces {
                let h = s.height;
                let p = base + ray * h;
                let d = p.y - p.x + h * 0.8164966;
                if d <= depth || !s.contains(p) {
                    continue;
                }
                let ground = self.solid_height_at(p);
                if (overpass && ground <= h) || (ground - h).abs() < 0.001 {
                    best = Vec3::new(p.x, h, p.y);
                    depth = d;
                }
            }
        }
        if let Some(terrain) = &self.terrain {
            use crate::chunkbuilder::terrain_surface::barycentric;
            let end = base + ray * terrain.max_height;
            let clip = BBox::new(
                base.map2(end, f32::min) - 0.01,
                base.map2(end, f32::max) + 0.01,
            );
            terrain.triangles(clip, |tri| {
                let projected = tri.map(|v| projection.screen(Vec2::new(v.x, v.z), v.y));
                if let Some(w) = barycentric(projected, pixel) {
                    let p = tri[0] * w[0] + tri[1] * w[1] + tri[2] * w[2];
                    let d = p.z - p.x + p.y * 0.8164966;
                    if d >= depth && (self.solid_height_at(Vec2::new(p.x, p.z)) - p.y).abs() < 0.002
                    {
                        best = p;
                        depth = d;
                    }
                }
            });
        }
        best
    }
}
