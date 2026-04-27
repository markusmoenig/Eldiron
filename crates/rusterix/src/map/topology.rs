use crate::{BBox, Map};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::GeoId;
use uuid::Uuid;
use vek::Vec2;

/// Derived adjacency for editor/runtime dependency queries.
///
/// This is intentionally not serialized. The authored map remains the source of
/// truth; this cache answers "what depends on this edited element?" quickly.
#[derive(Debug, Default, Clone)]
pub struct MapTopology {
    pub vertex_to_linedefs: FxHashMap<u32, FxHashSet<u32>>,
    pub vertex_to_sectors: FxHashMap<u32, FxHashSet<u32>>,
    pub linedef_to_sectors: FxHashMap<u32, FxHashSet<u32>>,
    pub sector_to_surfaces: FxHashMap<u32, FxHashSet<Uuid>>,
    pub surface_to_sector: FxHashMap<Uuid, u32>,
}

impl MapTopology {
    const TERRAIN_VERTEX_PAD: f32 = 8.0;
    const TERRAIN_SECTOR_PAD: f32 = 8.0;

    pub fn build(map: &Map) -> Self {
        let mut topology = Self::default();

        for linedef in &map.linedefs {
            topology
                .vertex_to_linedefs
                .entry(linedef.start_vertex)
                .or_default()
                .insert(linedef.id);
            topology
                .vertex_to_linedefs
                .entry(linedef.end_vertex)
                .or_default()
                .insert(linedef.id);

            for sector_id in &linedef.sector_ids {
                topology
                    .linedef_to_sectors
                    .entry(linedef.id)
                    .or_default()
                    .insert(*sector_id);
                topology
                    .vertex_to_sectors
                    .entry(linedef.start_vertex)
                    .or_default()
                    .insert(*sector_id);
                topology
                    .vertex_to_sectors
                    .entry(linedef.end_vertex)
                    .or_default()
                    .insert(*sector_id);
            }
        }

        for sector in &map.sectors {
            for linedef_id in &sector.linedefs {
                topology
                    .linedef_to_sectors
                    .entry(*linedef_id)
                    .or_default()
                    .insert(sector.id);
                if let Some(linedef) = map.find_linedef(*linedef_id) {
                    topology
                        .vertex_to_sectors
                        .entry(linedef.start_vertex)
                        .or_default()
                        .insert(sector.id);
                    topology
                        .vertex_to_sectors
                        .entry(linedef.end_vertex)
                        .or_default()
                        .insert(sector.id);
                }
            }
        }

        for (surface_id, surface) in &map.surfaces {
            topology
                .sector_to_surfaces
                .entry(surface.sector_id)
                .or_default()
                .insert(*surface_id);
            topology
                .surface_to_sector
                .insert(*surface_id, surface.sector_id);
        }

        topology
    }

    pub fn sectors_for_vertices(
        &self,
        vertex_ids: impl IntoIterator<Item = u32>,
    ) -> FxHashSet<u32> {
        let mut sectors = FxHashSet::default();
        for vertex_id in vertex_ids {
            if let Some(found) = self.vertex_to_sectors.get(&vertex_id) {
                sectors.extend(found.iter().copied());
            }
        }
        sectors
    }

    pub fn sectors_for_linedefs(
        &self,
        linedef_ids: impl IntoIterator<Item = u32>,
    ) -> FxHashSet<u32> {
        let mut sectors = FxHashSet::default();
        for linedef_id in linedef_ids {
            if let Some(found) = self.linedef_to_sectors.get(&linedef_id) {
                sectors.extend(found.iter().copied());
            }
        }
        sectors
    }

    pub fn surfaces_for_sectors(
        &self,
        sector_ids: impl IntoIterator<Item = u32>,
    ) -> FxHashSet<Uuid> {
        let mut surfaces = FxHashSet::default();
        for sector_id in sector_ids {
            if let Some(found) = self.sector_to_surfaces.get(&sector_id) {
                surfaces.extend(found.iter().copied());
            }
        }
        surfaces
    }

    pub fn owners_for_vertices(
        &self,
        map: &Map,
        vertex_ids: impl IntoIterator<Item = u32>,
    ) -> FxHashSet<GeoId> {
        let mut owners = FxHashSet::default();
        let mut linedefs = FxHashSet::default();
        let mut sectors = FxHashSet::default();

        for vertex_id in vertex_ids {
            if let Some(found) = self.vertex_to_linedefs.get(&vertex_id) {
                linedefs.extend(found.iter().copied());
            }
            if let Some(found) = self.vertex_to_sectors.get(&vertex_id) {
                sectors.extend(found.iter().copied());
            }
            self.add_terrain_owners_for_vertex(map, vertex_id, &mut owners);
        }

        owners.extend(linedefs.iter().copied().map(GeoId::Linedef));
        owners.extend(sectors.iter().copied().map(GeoId::Sector));

        for linedef_id in linedefs {
            self.add_terrain_owners_for_linedef(map, linedef_id, &mut owners);
        }
        for sector_id in sectors {
            self.add_terrain_owners_for_sector(map, sector_id, &mut owners);
        }

        owners
    }

    pub fn owners_for_linedefs(
        &self,
        map: &Map,
        linedef_ids: impl IntoIterator<Item = u32>,
    ) -> FxHashSet<GeoId> {
        let mut owners = FxHashSet::default();
        let mut sectors = FxHashSet::default();

        for linedef_id in linedef_ids {
            owners.insert(GeoId::Linedef(linedef_id));
            if let Some(found) = self.linedef_to_sectors.get(&linedef_id) {
                sectors.extend(found.iter().copied());
            }
            self.add_terrain_owners_for_linedef(map, linedef_id, &mut owners);
        }

        owners.extend(sectors.iter().copied().map(GeoId::Sector));
        for sector_id in sectors {
            self.add_terrain_owners_for_sector(map, sector_id, &mut owners);
        }

        owners
    }

    pub fn owners_for_sectors(
        &self,
        map: &Map,
        sector_ids: impl IntoIterator<Item = u32>,
    ) -> FxHashSet<GeoId> {
        let mut owners = FxHashSet::default();
        for sector_id in sector_ids {
            owners.insert(GeoId::Sector(sector_id));
            self.add_terrain_owners_for_sector(map, sector_id, &mut owners);
        }
        owners
    }

    fn add_terrain_owners_for_vertex(
        &self,
        map: &Map,
        vertex_id: u32,
        owners: &mut FxHashSet<GeoId>,
    ) {
        if !map.properties.get_bool_default("terrain_enabled", false) {
            return;
        }
        let Some(vertex) = map.find_vertex(vertex_id) else {
            return;
        };
        if !vertex.properties.get_bool_default("terrain_control", false)
            && !vertex.properties.contains("terrain_source")
        {
            return;
        }
        let smoothness = vertex
            .properties
            .get_float_default("smoothness", 1.0)
            .max(0.0);
        let falloff = vertex
            .properties
            .get_float_default("terrain_tile_falloff", 1.0)
            .max(0.0);
        let radius = smoothness * 2.0 + falloff + Self::TERRAIN_VERTEX_PAD;
        let bbox = BBox::new(
            Vec2::new(vertex.x - radius, vertex.y - radius),
            Vec2::new(vertex.x + radius, vertex.y + radius),
        );
        Self::add_terrain_owners_for_bbox(bbox, owners);
    }

    fn add_terrain_owners_for_linedef(
        &self,
        map: &Map,
        linedef_id: u32,
        owners: &mut FxHashSet<GeoId>,
    ) {
        if !map.properties.get_bool_default("terrain_enabled", false) {
            return;
        }
        let Some(linedef) = map.find_linedef(linedef_id) else {
            return;
        };
        if !linedef.properties.contains("terrain_source") {
            return;
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
            (width + falloff + 4.0 + organic * 6.0) * 2.0,
        ));
        Self::add_terrain_owners_for_bbox(bbox, owners);
    }

    fn add_terrain_owners_for_sector(
        &self,
        map: &Map,
        sector_id: u32,
        owners: &mut FxHashSet<GeoId>,
    ) {
        if !map.properties.get_bool_default("terrain_enabled", false) {
            return;
        }
        let Some(sector) = map.find_sector(sector_id) else {
            return;
        };
        let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
        if terrain_mode == 0 && !sector.properties.contains("terrain_source") {
            return;
        }
        let mut bbox = sector.bounding_box(map);
        let influence = match terrain_mode {
            2 => {
                sector
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
                        .max(0.0)
            }
            1 => Self::TERRAIN_SECTOR_PAD,
            _ => sector
                .properties
                .get_float_default("terrain_tile_falloff", 1.0)
                .max(0.0),
        };
        bbox.expand(Vec2::broadcast(
            (influence + Self::TERRAIN_SECTOR_PAD) * 2.0,
        ));
        Self::add_terrain_owners_for_bbox(bbox, owners);
    }

    fn add_terrain_owners_for_bbox(bbox: BBox, owners: &mut FxHashSet<GeoId>) {
        if !bbox.min.x.is_finite()
            || !bbox.min.y.is_finite()
            || !bbox.max.x.is_finite()
            || !bbox.max.y.is_finite()
        {
            return;
        }
        let min_x = bbox.min.x.floor() as i32;
        let min_z = bbox.min.y.floor() as i32;
        let max_x = bbox.max.x.ceil() as i32;
        let max_z = bbox.max.y.ceil() as i32;
        for z in min_z..max_z.max(min_z + 1) {
            for x in min_x..max_x.max(min_x + 1) {
                owners.insert(GeoId::Terrain(x, z));
            }
        }
    }
}
