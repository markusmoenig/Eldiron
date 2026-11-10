//! Live geometry modifiers, compiled once for every shared terrain revision.
use crate::{BBox, Map};
use vek::Vec2;

#[derive(Clone)]
struct Corridor {
    a: Vec2<f32>,
    b: Vec2<f32>,
    heights: [f32; 2],
    half_width: f32,
    blend: f32,
}

#[derive(Clone)]
struct Region {
    edges: Vec<(Vec2<f32>, Vec2<f32>)>,
    bounds: BBox,
    height: f32,
    blend: f32,
    hill: bool,
}

#[derive(Clone, Default)]
pub struct TerrainShapes {
    lines: Vec<Corridor>,
    regions: Vec<Region>,
}

fn smooth_weight(distance: f32, blend: f32) -> f32 {
    let t = (1. - distance / blend.max(0.01)).clamp(0., 1.);
    t * t * t * (t * (t * 6. - 15.) + 10.)
}

fn segment(p: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> (f32, f32) {
    let delta = b - a;
    let t = ((p - a).dot(delta) / delta.magnitude_squared().max(1e-12)).clamp(0., 1.);
    ((p - (a + t * delta)).magnitude(), t)
}

impl TerrainShapes {
    pub fn from_map(map: &Map) -> Self {
        let lines = map
            .linedefs
            .iter()
            .filter(|l| l.properties.get_bool_default("terrain_shape", false))
            .filter_map(|l| {
                let a = map.find_vertex(l.start_vertex)?;
                let b = map.find_vertex(l.end_vertex)?;
                Some(Corridor {
                    a: a.as_vec2(),
                    b: b.as_vec2(),
                    heights: [
                        l.properties
                            .get_float_default("terrain_start_height", 0.)
                            .clamp(0., 32.),
                        l.properties
                            .get_float_default("terrain_end_height", 0.)
                            .clamp(0., 32.),
                    ],
                    half_width: l
                        .properties
                        .get_float_default("terrain_shape_width", 2.)
                        .clamp(0., 64.)
                        * 0.5,
                    blend: l
                        .properties
                        .get_float_default("terrain_shape_blend", 3.)
                        .clamp(0.01, 64.),
                })
            })
            .collect();
        let regions = map
            .sectors
            .iter()
            .filter(|s| s.properties.get_bool_default("terrain_shape", false))
            .filter_map(|s| {
                let edges: Vec<_> = s
                    .linedefs
                    .iter()
                    .filter_map(|id| {
                        let l = map.find_linedef(*id)?;
                        Some((
                            map.get_vertex(l.start_vertex)?,
                            map.get_vertex(l.end_vertex)?,
                        ))
                    })
                    .collect();
                if edges.len() < 3 {
                    return None;
                }
                Some(Region {
                    edges,
                    bounds: s.bounding_box(map),
                    height: s
                        .properties
                        .get_float_default("terrain_shape_height", 1.)
                        .clamp(0., 32.),
                    blend: s
                        .properties
                        .get_float_default("terrain_shape_blend", 3.)
                        .clamp(0.01, 64.),
                    hill: s.properties.get_int_default("terrain_shape_mode", 0) == 1,
                })
            })
            .collect();
        Self { lines, regions }
    }

    /// Independent of storage order: overlaps blend targets, rather than stack heights.
    pub fn apply(&self, p: Vec2<f32>, base: f32) -> f32 {
        let mut sum = 0.;
        let mut weights = 0.;
        let mut strength = 0_f32;
        for region in &self.regions {
            let margin = if region.hill { 0. } else { region.blend };
            if !BBox::new(region.bounds.min - margin, region.bounds.max + margin).contains(p) {
                continue;
            }
            let mut inside = false;
            let mut distance = f32::INFINITY;
            for &(a, b) in &region.edges {
                distance = distance.min(segment(p, a, b).0);
                if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y)
                {
                    inside = !inside;
                }
            }
            let weight = if region.hill {
                if inside {
                    1. - smooth_weight(distance, region.blend)
                } else {
                    0.
                }
            } else if inside {
                1.
            } else {
                smooth_weight(distance, region.blend)
            };
            sum += weight * region.height;
            weights += weight;
            strength = strength.max(weight);
        }
        let ground = if weights > 0. {
            base + (sum / weights - base) * strength
        } else {
            base
        };
        sum = 0.;
        weights = 0.;
        strength = 0.;
        for line in &self.lines {
            let (distance, t) = segment(p, line.a, line.b);
            let weight = smooth_weight((distance - line.half_width).max(0.), line.blend);
            sum += weight * (line.heights[0] + t * (line.heights[1] - line.heights[0]));
            weights += weight;
            strength = strength.max(weight);
        }
        if weights > 0. {
            ground + (sum / weights - ground) * strength
        } else {
            ground
        }
    }

    pub fn expand_bounds(&self, bounds: &mut BBox, max_height: &mut f32) {
        for line in &self.lines {
            let radius = line.half_width + line.blend;
            bounds.min = bounds
                .min
                .map2(line.a.map2(line.b, f32::min) - radius, f32::min);
            bounds.max = bounds
                .max
                .map2(line.a.map2(line.b, f32::max) + radius, f32::max);
            *max_height = max_height.max(line.heights[0]).max(line.heights[1]);
        }
        for region in &self.regions {
            let radius = if region.hill { 0. } else { region.blend };
            bounds.min = bounds.min.map2(region.bounds.min - radius, f32::min);
            bounds.max = bounds.max.map2(region.bounds.max + radius, f32::max);
            *max_height = max_height.max(region.height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunkbuilder::{
        terrain_generator::{TerrainConfig, TerrainGenerator},
        terrain_surface::TerrainSurface,
    };
    use crate::{Assets, Chunk, MapCamera, PixelSource, Texture, Tile, Value};

    fn map() -> Map {
        let mut map = Map::default();
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        map.add_vertex_at(-12., -12.);
        map.add_vertex_at(12., 12.);
        map
    }
    fn line(map: &mut Map, a: Vec2<f32>, b: Vec2<f32>, heights: [f32; 2]) -> u32 {
        let a = map.add_vertex_at(a.x, a.y);
        let b = map.add_vertex_at(b.x, b.y);
        let id = map.create_linedef_manual(a, b);
        map.clear_temp();
        let props = &mut map.find_linedef_mut(id).unwrap().properties;
        props.set("terrain_shape", Value::Bool(true));
        props.set("terrain_start_height", Value::Float(heights[0]));
        props.set("terrain_end_height", Value::Float(heights[1]));
        props.set("terrain_shape_width", Value::Float(2.));
        props.set("terrain_shape_blend", Value::Float(3.));
        map.changed += 1;
        id
    }
    fn region(map: &mut Map, hill: bool) -> u32 {
        let v = [[-4., -4.], [4., -4.], [4., 4.], [-4., 4.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let props = &mut map.find_sector_mut(id).unwrap().properties;
        props.set("terrain_shape", Value::Bool(true));
        props.set("terrain_shape_mode", Value::Int(i32::from(hill)));
        props.set("terrain_shape_height", Value::Float(1.2));
        props.set("terrain_shape_blend", Value::Float(2.));
        map.changed += 1;
        id
    }
    fn height(map: &Map, p: Vec2<f32>) -> f32 {
        TerrainGenerator::sample_height_at(map, p, &TerrainConfig::for_map(map))
    }
    #[test]
    fn corridor_interpolates_heights_and_blends_smoothly_at_both_edges() {
        let mut map = map();
        line(&mut map, Vec2::new(-3., 0.), Vec2::new(3., 0.), [0., 1.2]);
        assert!((height(&map, Vec2::zero()) - 0.6).abs() < 1e-5);
        assert!((height(&map, Vec2::new(3., 0.)) - 1.2).abs() < 1e-5);
        assert!((height(&map, Vec2::new(0., 2.5)) - 0.3).abs() < 1e-5);
        assert_eq!(height(&map, Vec2::new(0., 5.)), 0.);
        let sampler = TerrainGenerator::new(TerrainConfig::for_map(&map)).sampler(&map);
        assert!((sampler.sample(Vec2::new(0., 1.001)) - 0.6).abs() < 1e-5);
    }
    #[test]
    fn sector_plateau_is_filled_and_hill_rises_inside_its_footprint() {
        let mut map = map();
        let id = region(&mut map, false);
        assert!((height(&map, Vec2::zero()) - 1.2).abs() < 1e-5);
        assert!((height(&map, Vec2::new(5., 0.)) - 0.6).abs() < 1e-5);
        assert_eq!(height(&map, Vec2::new(6., 0.)), 0.);
        map.find_sector_mut(id)
            .unwrap()
            .properties
            .set("terrain_shape_mode", Value::Int(1));
        map.changed += 1;
        assert!((height(&map, Vec2::zero()) - 1.2).abs() < 1e-5);
        assert!((height(&map, Vec2::new(3., 0.)) - 0.6).abs() < 1e-5);
        assert_eq!(height(&map, Vec2::new(4., 0.)), 0.);
        assert_eq!(height(&map, Vec2::new(5., 0.)), 0.);
    }
    #[test]
    fn overlaps_are_order_independent_and_geometry_edits_invalidate_cached_queries() {
        let mut map = map();
        let id = line(&mut map, Vec2::new(-3., 0.), Vec2::new(3., 0.), [1., 1.]);
        line(&mut map, Vec2::new(0., -3.), Vec2::new(0., 3.), [2., 2.]);
        let before = height(&map, Vec2::zero());
        map.linedefs.reverse();
        map.changed += 1;
        assert!((height(&map, Vec2::zero()) - before).abs() < 1e-5);
        let edge = map.find_linedef(id).unwrap().clone();
        for v in [edge.start_vertex, edge.end_vertex] {
            map.find_vertex_mut(v).unwrap().y += 10.;
        }
        map.changed += 1;
        assert!((height(&map, Vec2::zero()) - 2.).abs() < 1e-5);
        map.find_linedef_mut(id)
            .unwrap()
            .properties
            .set("terrain_shape", Value::Bool(false));
        map.changed += 1;
        assert_eq!(height(&map, Vec2::new(-3., 10.)), 0.);
    }
    #[test]
    fn a_corridor_in_an_empty_map_streams_terrain_on_both_sides_of_the_line() {
        let mut map = Map::default();
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        line(&mut map, Vec2::new(-3., 0.), Vec2::new(3., 0.), [1., 1.]);
        let bounds = map.bbox();
        assert_eq!(bounds.min, Vec2::new(-7., -4.));
        assert_eq!(bounds.max, Vec2::new(7., 4.));
        let field = TerrainSurface::from_map(&map).unwrap();
        assert_eq!(field.height_at(Vec2::new(0., -1.)), Some(1.));
        assert_eq!(field.height_at(Vec2::new(0., 1.)), Some(1.));
    }

    #[test]
    fn narrow_ridge_keeps_rendered_facets_under_its_walkable_surface() {
        let mut map = Map::default();
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_subdivisions", Value::Int(4));
        let id = line(&mut map, Vec2::new(-3., 0.), Vec2::new(3., 0.), [1., 1.]);
        let props = &mut map.find_linedef_mut(id).unwrap().properties;
        props.set("terrain_shape_width", Value::Float(0.));
        props.set("terrain_shape_blend", Value::Float(0.01));
        map.changed += 1;
        let surface = TerrainSurface::from_map(&map).unwrap();
        let mut count = 0;
        surface.triangles(surface.bounds, |_| count += 1);
        assert!(count > 0);
        assert_eq!(surface.height_at(Vec2::zero()), Some(1.));
    }

    #[test]
    fn actual_3d_batches_match_iso_and_keep_one_polygon_per_ground_cell() {
        let mut map = map();
        region(&mut map, true);
        line(&mut map, Vec2::new(-3., 0.), Vec2::new(3., 0.), [0.1, 1.]);
        let tile = Tile::from_texture(Texture::new([70, 90, 50, 255].repeat(16), 4, 4));
        map.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(tile.id)),
        );
        let mut assets = Assets::default();
        assets.set_tiles([(tile.id, tile)].into_iter().collect());
        let field = TerrainSurface::from_map(&map).unwrap();
        let mut chunk = Chunk::new(Vec2::new(-4, -4), 8);
        let mut vm = scenevm::Chunk::new(Vec2::new(-4, -4), 8);
        super::super::d3chunkbuilder::generate_terrain(&map, &assets, &mut chunk, &mut vm);
        assert_eq!(vm.polys3d_map.len(), 64);
        for polygons in vm.polys3d_map.values() {
            assert_eq!(polygons.len(), 1, "one batch per cell, not per triangle");
            assert_eq!(polygons[0].indices.len(), 32);
            assert_eq!(polygons[0].vertices.len(), 25);
            assert_eq!(polygons[0].normals.len(), 25);
            assert!(polygons[0].normals.iter().all(|n| n[1] > 0.));
            for v in &polygons[0].vertices {
                assert!((v[1] - field.height_at(Vec2::new(v[0], v[2])).unwrap()).abs() < 1e-5);
            }
        }
        let original = height(&map, Vec2::zero());
        for camera in [
            MapCamera::TwoD,
            MapCamera::TwoDIso,
            MapCamera::ThreeDIso,
            MapCamera::ThreeDFirstPerson,
        ] {
            map.camera = camera;
            assert!((height(&map, Vec2::zero()) - original).abs() < 1e-5);
        }
    }
}
