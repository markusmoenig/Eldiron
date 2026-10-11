use crate::{BBox, Map};
use std::hash::Hasher;
use vek::Vec2;

type Triangles = (Vec<[f32; 2]>, Vec<(usize, usize, usize)>);

pub(super) struct SectorGeometry {
    pub geometry: Option<Triangles>,
    pub bounds: BBox,
    pub span: Option<(f32, f32)>,
    area: f32,
}

pub(super) struct LineGeometry {
    pub endpoints: Option<(Vec2<f32>, Vec2<f32>)>,
    pub bounds: BBox,
    pub span: Option<(f32, f32)>,
}

#[derive(Default)]
pub(super) struct PreviewGeometry {
    fingerprint: Option<u64>,
    outline_fingerprint: Option<u64>,
    pub hidden_rect_edges: rustc_hash::FxHashSet<u32>,
    pub sectors: Vec<SectorGeometry>,
    pub lines: Vec<LineGeometry>,
    pub sorted_sectors: Vec<usize>,
}

/// Includes coordinates and topology, rather than just vector lengths: dragging,
/// undo, replacement and switching maps must all invalidate the preview cache.
pub(crate) fn geometry_fingerprint(map: &Map) -> u64 {
    let mut hash = rustc_hash::FxHasher::default();
    hash.write(map.id.as_bytes());
    hash.write_usize(map.vertices.len());
    for v in &map.vertices {
        hash.write_u32(v.id);
        hash.write_u32(v.x.to_bits());
        hash.write_u32(v.y.to_bits());
        hash.write_u32(v.z.to_bits());
    }
    hash.write_usize(map.linedefs.len());
    for l in &map.linedefs {
        hash.write_u32(l.id);
        hash.write_u32(l.start_vertex);
        hash.write_u32(l.end_vertex);
    }
    hash.write_usize(map.sectors.len());
    for s in &map.sectors {
        hash.write_u32(s.id);
        hash.write_usize(s.linedefs.len());
        for id in &s.linedefs {
            hash.write_u32(*id);
        }
    }
    hash.finish()
}

impl PreviewGeometry {
    pub fn update(&mut self, map: &Map) -> bool {
        let fingerprint = geometry_fingerprint(map);
        let mut outlines = rustc_hash::FxHasher::default();
        outlines.write_u64(fingerprint);
        for sector in &map.sectors {
            outlines.write_u8(u8::from(
                sector.name.is_empty()
                    && (sector.properties.contains("rect")
                        || sector.properties.contains("rect_rendering")),
            ));
        }
        let outline_fingerprint = outlines.finish();
        if self.outline_fingerprint != Some(outline_fingerprint) {
            let mut rect_edges = rustc_hash::FxHashSet::default();
            let mut other_edges = rustc_hash::FxHashSet::default();
            for sector in &map.sectors {
                let unnamed_rect = sector.name.is_empty()
                    && (sector.properties.contains("rect")
                        || sector.properties.contains("rect_rendering"));
                let edges = if unnamed_rect {
                    &mut rect_edges
                } else {
                    &mut other_edges
                };
                edges.extend(sector.linedefs.iter().copied());
            }
            rect_edges.retain(|id| !other_edges.contains(id));
            self.hidden_rect_edges = rect_edges;
            self.outline_fingerprint = Some(outline_fingerprint);
        }
        if self.fingerprint == Some(fingerprint) {
            return false;
        }
        self.sectors = map
            .sectors
            .iter()
            .map(|s| {
                let geometry = s.generate_geometry(map);
                let area = geometry.as_ref().map_or(0.0, |(v, triangles)| {
                    triangles
                        .iter()
                        .map(|&(a, b, c)| {
                            let (a, b, c) = (v[a], v[b], v[c]);
                            0.5 * ((a[0] * b[1] + b[0] * c[1] + c[0] * a[1])
                                - (a[1] * b[0] + b[1] * c[0] + c[1] * a[0]))
                                .abs()
                        })
                        .sum()
                });
                SectorGeometry {
                    geometry,
                    area,
                    bounds: s.bounding_box(map),
                    span: s.y_span(map),
                }
            })
            .collect();
        self.lines = map
            .linedefs
            .iter()
            .map(|l| LineGeometry {
                endpoints: map
                    .get_vertex(l.start_vertex)
                    .zip(map.get_vertex(l.end_vertex)),
                bounds: l.bounding_box(map),
                span: l.y_span_world(map),
            })
            .collect();
        self.sorted_sectors = (0..self.sectors.len()).collect();
        self.sorted_sectors.sort_by(|a, b| {
            self.sectors[*b]
                .area
                .partial_cmp(&self.sectors[*a].area)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.fingerprint = Some(fingerprint);
        true
    }
}

pub(super) fn intersects_slice(span: Option<(f32, f32)>, center: f32, thickness: f32) -> bool {
    thickness > 0.0
        && span.is_some_and(|(min, max)| {
            max >= center - thickness * 0.5 && min <= center + thickness * 0.5
        })
}

pub(super) fn viewport(map: &Map, screen: Vec2<f32>, margin_pixels: f32) -> Option<BBox> {
    if !map.grid_size.is_finite() || map.grid_size <= 0.0 {
        return None;
    }
    let center = Vec2::new(map.offset.x, -map.offset.y) + screen * 0.5;
    let margin = Vec2::broadcast(margin_pixels);
    Some(BBox::new(
        (-center - margin) / map.grid_size,
        (screen - center + margin) / map.grid_size,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Linedef, Sector, Vertex};

    fn polygon() -> Map {
        let mut map = Map::new();
        // Concave polygon and sparse IDs exercise triangulation and indexed lookup.
        map.vertices = [(0., 0.), (3., 0.), (3., 3.), (1., 1.), (0., 3.)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, y))| Vertex::new(100 + i as u32, x, y))
            .collect();
        map.linedefs = (0..5)
            .map(|i| Linedef::new(200 + i, 100 + i, 100 + (i + 1) % 5))
            .collect();
        map.sectors.push(Sector::new(300, (200..205).collect()));
        map
    }

    #[test]
    fn cached_geometry_matches_reference_and_tracks_edits() {
        let mut map = polygon();
        let original = map.clone();
        let mut cache = PreviewGeometry::default();
        assert!(cache.update(&map));
        assert_eq!(
            cache.sectors[0].geometry,
            map.sectors[0].generate_geometry(&map)
        );
        assert_eq!(cache.sectors[0].bounds, map.sectors[0].bounding_box(&map));
        assert_eq!(cache.sectors[0].area, map.sectors[0].area(&map));
        assert!(!cache.update(&map));
        map.offset.x += 40.;
        map.grid_size *= 2.;
        assert!(!cache.update(&map), "panning and zooming preserve geometry");
        map.vertices[3].x = 2.;
        map.vertices[3].z = 7.;
        assert!(cache.update(&map));
        assert_eq!(cache.sectors[0].span, map.sectors[0].y_span(&map));
        assert_eq!(
            cache.sectors[0].geometry,
            map.sectors[0].generate_geometry(&map)
        );
        map = original;
        assert!(cache.update(&map), "undo restores the original geometry");
        map.linedefs[0].start_vertex = 104;
        assert!(cache.update(&map), "same-size topology edits invalidate");
        map.sectors[0].linedefs.reverse();
        assert!(cache.update(&map));
        map.vertices.pop();
        assert!(cache.update(&map));
        assert!(cache.sectors[0].geometry.is_none());
        map = polygon();
        assert!(cache.update(&map), "map switching invalidates");
    }

    #[test]
    fn outline_cache_tracks_names_and_shared_edges_without_retriangulation() {
        let mut map = polygon();
        map.sectors[0]
            .properties
            .set("rect", crate::Value::Bool(true));
        let mut cache = PreviewGeometry::default();
        assert!(cache.update(&map));
        assert!(cache.hidden_rect_edges.contains(&200));
        map.sectors[0].name = "Visible outline".into();
        assert!(!cache.update(&map));
        assert!(cache.hidden_rect_edges.is_empty());
        map.sectors[0].name.clear();
        cache.update(&map);
        assert!(cache.hidden_rect_edges.contains(&200));
        let mut second = map.sectors[0].clone();
        second.id += 1;
        second.properties.remove("rect");
        map.sectors.push(second);
        cache.update(&map);
        assert!(
            cache.hidden_rect_edges.is_empty(),
            "shared edges of non-rect sectors stay visible"
        );
    }

    #[test]
    fn viewport_and_slice_keep_boundary_geometry() {
        let mut map = polygon();
        map.grid_size = 10.;
        map.offset = Vec2::new(-50., 50.);
        let view = viewport(&map, Vec2::new(100., 100.), 8.).unwrap();
        assert!(view.contains(Vec2::new(-0.8, -0.8)));
        assert!(view.intersects(&BBox::new(Vec2::new(-1., 0.), Vec2::new(0., 1.))));
        assert!(!view.intersects(&BBox::new(Vec2::new(12., 12.), Vec2::new(13., 13.))));
        assert!(intersects_slice(Some((1., 3.)), 0., 2.));
        assert!(!intersects_slice(Some((1.01, 3.)), 0., 2.));
        assert!(!intersects_slice(None, 0., 2.));
        assert!(!intersects_slice(Some((0., 0.)), 0., 0.));
    }

    #[test]
    fn lookup_survives_sparse_ids_reordering_removal_and_serialization() {
        let mut map = polygon();
        assert_eq!(map.get_vertex(103), Some(Vec2::new(1., 1.)));
        map.vertices.swap(0, 3);
        map.find_vertex_mut(103).unwrap().x = 2.;
        assert_eq!(map.get_vertex(103), Some(Vec2::new(2., 1.)));
        map.find_vertex_mut(103).unwrap().x = 1.;
        assert_eq!(map.get_vertex(103), Some(Vec2::new(1., 1.)));
        map.vertices.retain(|v| v.id != 103);
        assert!(map.get_vertex(103).is_none());
        map.vertices.push(Vertex::new(103, 9., 8.));
        assert_eq!(map.get_vertex(103), Some(Vec2::new(9., 8.)));
        map.linedefs.reverse();
        assert_eq!(map.find_linedef(200).unwrap().start_vertex, 100);
        let serialized = serde_json::to_string(&map).unwrap();
        assert!(!serialized.contains("geometry_lookup"));
        let loaded: Map = serde_json::from_str(&serialized).unwrap();
        assert_eq!(loaded.get_vertex(103), map.clone().get_vertex(103));
        assert_eq!(loaded.find_sector(300).unwrap().id, 300);
    }

    #[test]
    #[ignore = "set ELDIRON_BENCH_PROJECT_JSON to an extracted project.json"]
    fn benchmark_large_map_preview_geometry() {
        let file = std::env::var("ELDIRON_BENCH_PROJECT_JSON").unwrap();
        let project: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let map: Map = serde_json::from_value(project["regions"][0]["map"].clone()).unwrap();
        let mut cache = PreviewGeometry::default();
        let start = std::time::Instant::now();
        cache.update(&map);
        println!("cold geometry: {:?}", start.elapsed());
        let start = std::time::Instant::now();
        for _ in 0..100 {
            assert!(!cache.update(&map));
        }
        println!("cached geometry per redraw: {:?}", start.elapsed() / 100);
        let view = viewport(&map, Vec2::new(1280., 720.), 8.).unwrap();
        println!(
            "visible sectors: {} / {}",
            cache
                .sectors
                .iter()
                .filter(|s| view.intersects(&s.bounds))
                .count(),
            map.sectors.len()
        );
        for (sector, cached) in map.sectors.iter().zip(&cache.sectors) {
            assert_eq!(sector.generate_geometry(&map), cached.geometry);
            assert_eq!(sector.bounding_box(&map), cached.bounds);
            assert_eq!(sector.y_span(&map), cached.span);
        }
        let reference: Vec<_> = map.sorted_sectors_by_area().iter().map(|s| s.id).collect();
        assert_eq!(
            reference,
            cache
                .sorted_sectors
                .iter()
                .map(|i| map.sectors[*i].id)
                .collect::<Vec<_>>()
        );
    }
}
