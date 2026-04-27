use crate::chunkbuilder::d3chunkbuilder::D3ChunkBuilder;
use crate::collision_world::ChunkCollision;
use crate::{Assets, BBox, Chunk, ChunkBuilder, Map, MapTopology};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::GeoId;
use uuid::Uuid;
use vek::Vec2;

/// Topology-backed 3D chunk builder entry point.
///
/// This is intentionally separate from `D3ChunkBuilder`. The old builder stays
/// in the tree as the known-good reference while sector/surface/terrain
/// generation is moved to a retained topology scene incrementally.
#[derive(Clone)]
pub struct TopologyBuilder {
    reference_builder: D3ChunkBuilder,
    cached_scene: Option<(Uuid, u32, TopologyScene)>,
}

#[derive(Clone, Debug)]
pub struct TopologySector {
    pub id: u32,
    pub bbox: BBox,
    pub surfaces: FxHashSet<Uuid>,
}

#[derive(Clone, Debug)]
pub struct TopologyScene {
    pub topology: MapTopology,
    pub sectors: FxHashMap<u32, TopologySector>,
}

impl TopologyScene {
    const TERRAIN_CHUNK_PAD: f32 = 12.0;

    pub fn build(map: &Map) -> Self {
        let topology = MapTopology::build(map);
        let mut sectors = FxHashMap::default();

        for sector in &map.sectors {
            let bbox = sector.bounding_box(map);
            if !bbox.min.x.is_finite()
                || !bbox.min.y.is_finite()
                || !bbox.max.x.is_finite()
                || !bbox.max.y.is_finite()
            {
                continue;
            }

            sectors.insert(
                sector.id,
                TopologySector {
                    id: sector.id,
                    bbox,
                    surfaces: topology
                        .sector_to_surfaces
                        .get(&sector.id)
                        .cloned()
                        .unwrap_or_default(),
                },
            );
        }

        Self { topology, sectors }
    }

    pub fn sector_owners_for_chunk(&self, chunk_bbox: &BBox) -> FxHashSet<GeoId> {
        self.sectors
            .values()
            .filter(|sector| {
                sector.bbox.intersects(chunk_bbox) && chunk_bbox.contains(sector.bbox.center())
            })
            .map(|sector| GeoId::Sector(sector.id))
            .collect()
    }

    pub fn owners_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<GeoId> {
        let mut owners = self.sector_owners_for_chunk(chunk_bbox);
        owners.extend(
            self.feature_linedef_ids_for_chunk(map, chunk_bbox)
                .into_iter()
                .map(GeoId::Linedef),
        );

        if map.properties.get_bool_default("terrain_enabled", false) {
            owners.extend(Self::terrain_tile_owners_for_chunk(chunk_bbox));
        }

        owners
    }

    pub fn filtered_map_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> Map {
        let mut sector_ids = self
            .sector_owners_for_chunk(chunk_bbox)
            .into_iter()
            .filter_map(|owner| match owner {
                GeoId::Sector(id) => Some(id),
                _ => None,
            })
            .collect::<FxHashSet<_>>();
        let terrain_enabled = map.properties.get_bool_default("terrain_enabled", false);
        if terrain_enabled {
            sector_ids.extend(self.terrain_sector_ids_for_chunk(map, chunk_bbox));
        }

        let mut surface_ids = FxHashSet::default();
        for sector_id in &sector_ids {
            if let Some(sector) = self.sectors.get(sector_id) {
                surface_ids.extend(sector.surfaces.iter().copied());
            }
        }

        let mut linedef_ids = FxHashSet::default();
        for sector in &map.sectors {
            if sector_ids.contains(&sector.id) {
                linedef_ids.extend(sector.linedefs.iter().copied());
            }
        }
        if terrain_enabled {
            linedef_ids.extend(self.terrain_linedef_ids_for_chunk(map, chunk_bbox));
        }
        linedef_ids.extend(self.feature_linedef_ids_for_chunk(map, chunk_bbox));

        let mut vertex_ids = FxHashSet::default();
        for linedef in &map.linedefs {
            if linedef_ids.contains(&linedef.id) {
                vertex_ids.insert(linedef.start_vertex);
                vertex_ids.insert(linedef.end_vertex);
            }
        }
        if terrain_enabled {
            vertex_ids.extend(self.terrain_vertex_ids_for_chunk(map, chunk_bbox));
            vertex_ids.extend(map.vertices.iter().map(|vertex| vertex.id));
        }
        vertex_ids.extend(self.feature_vertex_ids_for_chunk(map, chunk_bbox));

        let mut filtered = map.clone();
        filtered
            .sectors
            .retain(|sector| sector_ids.contains(&sector.id));
        filtered
            .linedefs
            .retain(|linedef| linedef_ids.contains(&linedef.id));
        filtered
            .vertices
            .retain(|vertex| vertex_ids.contains(&vertex.id));
        filtered.surfaces.retain(|surface_id, surface| {
            surface_ids.contains(surface_id) || sector_ids.contains(&surface.sector_id)
        });
        filtered
    }

    fn expanded_chunk_bbox(chunk_bbox: &BBox) -> BBox {
        let mut bbox = *chunk_bbox;
        bbox.expand(Vec2::broadcast(Self::TERRAIN_CHUNK_PAD * 2.0));
        bbox
    }

    fn terrain_tile_owners_for_chunk(chunk_bbox: &BBox) -> FxHashSet<GeoId> {
        let min_x = chunk_bbox.min.x.floor() as i32;
        let min_z = chunk_bbox.min.y.floor() as i32;
        let max_x = chunk_bbox.max.x.ceil() as i32;
        let max_z = chunk_bbox.max.y.ceil() as i32;
        let mut owners = FxHashSet::default();
        for z in min_z..max_z.max(min_z + 1) {
            for x in min_x..max_x.max(min_x + 1) {
                owners.insert(GeoId::Terrain(x, z));
            }
        }
        owners
    }

    fn terrain_sector_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let query_bbox = Self::expanded_chunk_bbox(chunk_bbox);
        let mut sector_ids = FxHashSet::default();

        for sector in &map.sectors {
            let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
            let builder_replace_surface = sector
                .properties
                .get_str_default("builder_surface_mode", String::new())
                == "replace"
                && !sector
                    .properties
                    .get_str_default("builder_graph_data", String::new())
                    .trim()
                    .is_empty()
                && sector
                    .properties
                    .get_str_default("builder_graph_target", "sector".to_string())
                    == "sector";

            if terrain_mode == 0 && !builder_replace_surface {
                continue;
            }

            let mut bbox = sector.bounding_box(map);
            if terrain_mode == 2 {
                let influence = sector
                    .properties
                    .get_float_default("ridge_plateau_width", 0.0)
                    .max(0.0)
                    + sector
                        .properties
                        .get_float_default("ridge_falloff_distance", 0.0)
                        .max(0.0)
                    + sector
                        .properties
                        .get_float_default("terrain_tile_falloff", 1.0)
                        .max(0.0);
                bbox.expand(Vec2::broadcast(influence * 2.0));
            }

            if bbox.intersects(&query_bbox) {
                sector_ids.insert(sector.id);
            }
        }

        sector_ids
    }

    fn terrain_linedef_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut query_bbox = Self::expanded_chunk_bbox(chunk_bbox);
        query_bbox.expand(Vec2::broadcast(100.0));
        let mut linedef_ids = FxHashSet::default();

        for linedef in &map.linedefs {
            if !linedef.properties.contains("terrain_source")
                && !linedef.properties.get_bool_default("terrain_smooth", false)
            {
                continue;
            }
            let width = linedef
                .properties
                .get_float_default("terrain_width", 2.0)
                .max(0.0);
            let falloff = linedef
                .properties
                .get_float_default("terrain_tile_falloff", 1.0)
                .max(0.0);
            let organic = linedef
                .properties
                .get_float_default("terrain_road_organic", 0.0)
                .clamp(0.0, 1.0);
            let mut bbox = linedef.bounding_box(map);
            bbox.expand(Vec2::broadcast(
                (width + falloff + organic * 6.0 + 4.0) * 2.0,
            ));
            if bbox.intersects(&query_bbox) {
                linedef_ids.insert(linedef.id);
            }
        }

        linedef_ids
    }

    fn terrain_vertex_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut query_bbox = Self::expanded_chunk_bbox(chunk_bbox);
        query_bbox.expand(Vec2::broadcast(100.0));
        let mut vertex_ids = FxHashSet::default();

        for vertex in &map.vertices {
            if !vertex.properties.get_bool_default("terrain_control", false)
                && !vertex.properties.contains("terrain_source")
            {
                continue;
            }
            let smoothness = vertex
                .properties
                .get_float_default("smoothness", 1.0)
                .max(0.0);
            let falloff = vertex
                .properties
                .get_float_default("terrain_tile_falloff", 1.0)
                .max(0.0);
            let radius = smoothness * 2.0 + falloff + Self::TERRAIN_CHUNK_PAD;
            let bbox = BBox::new(
                Vec2::new(vertex.x - radius, vertex.y - radius),
                Vec2::new(vertex.x + radius, vertex.y + radius),
            );
            if bbox.intersects(&query_bbox) {
                vertex_ids.insert(vertex.id);
            }
        }

        vertex_ids
    }

    fn feature_linedef_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut linedef_ids = FxHashSet::default();

        for linedef in &map.linedefs {
            let feature = linedef
                .properties
                .get_str_default("linedef_feature", "None".to_string());
            let has_linedef_feature = feature == "Palisade" || feature == "Fence";
            let has_builder_feature = !linedef
                .properties
                .get_str_default("builder_graph_data", String::new())
                .trim()
                .is_empty();
            if !has_linedef_feature && !has_builder_feature {
                continue;
            }

            let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
                continue;
            };
            let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
                continue;
            };
            let line_mid = Vec2::new((v0.x + v1.x) * 0.5, (v0.z + v1.z) * 0.5);
            if chunk_bbox.contains(line_mid) || linedef.bounding_box(map).intersects(chunk_bbox) {
                linedef_ids.insert(linedef.id);
            }
        }

        linedef_ids
    }

    fn feature_vertex_ids_for_chunk(&self, map: &Map, chunk_bbox: &BBox) -> FxHashSet<u32> {
        let mut vertex_ids = FxHashSet::default();

        for vertex in &map.vertices {
            if vertex
                .properties
                .get_str_default("builder_graph_data", String::new())
                .trim()
                .is_empty()
            {
                continue;
            }
            let pos = Vec2::new(vertex.x, vertex.y);
            if chunk_bbox.contains(pos) {
                vertex_ids.insert(vertex.id);
            }
        }

        vertex_ids
    }
}

impl TopologyBuilder {
    pub fn build_topology(map: &Map) -> MapTopology {
        MapTopology::build(map)
    }

    fn scene_for_map(&mut self, map: &Map) -> &TopologyScene {
        let needs_rebuild = self
            .cached_scene
            .as_ref()
            .is_none_or(|(map_id, changed, _)| *map_id != map.id || *changed != map.changed);

        if needs_rebuild {
            self.cached_scene = Some((map.id, map.changed, TopologyScene::build(map)));
        }

        &self.cached_scene.as_ref().expect("topology scene cache").2
    }
}

impl ChunkBuilder for TopologyBuilder {
    fn new() -> Self {
        Self {
            reference_builder: D3ChunkBuilder::new(),
            cached_scene: None,
        }
    }

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
        let filtered = {
            let scene = self.scene_for_map(map);
            scene.filtered_map_for_chunk(map, &chunk.bbox)
        };
        self.reference_builder
            .build(&filtered, assets, chunk, vmchunk);
    }

    fn build_collision(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk_origin: Vec2<i32>,
        chunk_size: i32,
    ) -> ChunkCollision {
        let _ = self.scene_for_map(map);
        self.reference_builder
            .build_collision(map, assets, chunk_origin, chunk_size)
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }
}
