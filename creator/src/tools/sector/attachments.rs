use crate::prelude::*;
use rusterix::Value;
use std::collections::{HashMap, HashSet};

pub(crate) const OWNER: &str = "iso_attached_to";

pub(super) fn root(map: &Map, id: u32) -> Option<u32> {
    let mut sector = map.find_sector(id)?;
    let mut seen = HashSet::new();
    while let Some(owner) = sector.properties.get_id(OWNER) {
        if !seen.insert(sector.creator_id) {
            return None;
        }
        sector = map.sectors.iter().find(|s| s.creator_id == owner)?;
    }
    (sector.id != id).then_some(sector.id)
}

/// Move authored placements as well as their live Creator preview counterparts.
pub(super) struct SectorDrag {
    pub(super) before: Box<Region>,
    pub(super) offset: Vec2<f32>,
    sectors: HashSet<u32>,
    items: HashSet<Uuid>,
    characters: HashSet<Uuid>,
    lights: Vec<usize>,
    vertices: HashMap<u32, Vec2<f32>>,
    anchors: Vec<(u32, Vec2<f32>)>,
}

fn instance_owner(data: &str) -> Option<Uuid> {
    if !data.contains(OWNER) {
        return None;
    }
    let table = data.parse::<toml::Table>().ok()?;
    table.get("placement")?.get(OWNER)?.as_str()?.parse().ok()
}

impl SectorDrag {
    pub(super) fn new(region: &Region) -> Option<Self> {
        let map = &region.map;
        let mut sectors: HashSet<_> = map.selected_sectors.iter().copied().collect();
        let mut owners: HashSet<_> = map
            .sectors
            .iter()
            .filter(|sector| sectors.contains(&sector.id))
            .map(|sector| sector.creator_id)
            .collect();
        // A selected footprint also carries sub-sectors that are themselves parents.
        while owners.len() < map.sectors.len() {
            let count = owners.len();
            for sector in &map.sectors {
                if sector
                    .properties
                    .get_id(OWNER)
                    .is_some_and(|owner| owners.contains(&owner))
                {
                    sectors.insert(sector.id);
                    owners.insert(sector.creator_id);
                }
            }
            if owners.len() == count {
                break;
            }
        }
        let items: HashSet<_> = region
            .items
            .iter()
            .filter(|(_, item)| {
                instance_owner(&item.data).is_some_and(|owner| owners.contains(&owner))
            })
            .map(|(id, _)| *id)
            .collect();
        let characters: HashSet<_> = region
            .characters
            .iter()
            .filter(|(_, character)| {
                instance_owner(&character.data).is_some_and(|owner| owners.contains(&owner))
            })
            .map(|(id, _)| *id)
            .collect();
        let lights: Vec<_> = map
            .lights
            .iter()
            .enumerate()
            .filter(|(_, light)| {
                light
                    .properties
                    .get_id(OWNER)
                    .is_some_and(|owner| owners.contains(&owner))
            })
            .map(|(i, _)| i)
            .collect();
        if sectors.len() == map.selected_sectors.len()
            && items.is_empty()
            && characters.is_empty()
            && lights.is_empty()
        {
            return None;
        }
        Some(Self {
            before: Box::new(region.clone()),
            offset: Vec2::zero(),
            sectors,
            items,
            characters,
            lights,
            vertices: HashMap::new(),
            anchors: Vec::new(),
        })
    }

    /// Split only the boundary with stationary geometry. Preserve the topology
    /// shared between moving sectors and all of their stable sector identities.
    fn prepare_geometry(&mut self, map: &mut Map) {
        let moving_lines: HashSet<_> = map
            .sectors
            .iter()
            .filter(|sector| self.sectors.contains(&sector.id))
            .flat_map(|sector| sector.linedefs.iter().copied())
            .collect();
        let fixed_lines: HashSet<_> = map
            .sectors
            .iter()
            .filter(|sector| !self.sectors.contains(&sector.id))
            .flat_map(|sector| sector.linedefs.iter().copied())
            .collect();
        let fixed_vertices: HashSet<_> = map
            .linedefs
            .iter()
            .filter(|line| !moving_lines.contains(&line.id) || fixed_lines.contains(&line.id))
            .flat_map(|line| [line.start_vertex, line.end_vertex])
            .collect();
        let moving_vertices: HashSet<_> = map
            .linedefs
            .iter()
            .filter(|line| moving_lines.contains(&line.id))
            .flat_map(|line| [line.start_vertex, line.end_vertex])
            .collect();
        let mut split_vertices = HashMap::new();
        for vertex in map.vertices.clone() {
            if moving_vertices.contains(&vertex.id) {
                let id = if fixed_vertices.contains(&vertex.id) {
                    let id = map
                        .duplicate_vertex(vertex.id)
                        .expect("available map vertex ID");
                    split_vertices.insert(vertex.id, id);
                    id
                } else {
                    vertex.id
                };
                self.vertices.insert(id, vertex.as_vec2());
            }
        }

        let mut split_lines = HashMap::new();
        for original in map
            .linedefs
            .clone()
            .into_iter()
            .filter(|line| moving_lines.contains(&line.id))
        {
            let line = if fixed_lines.contains(&original.id) {
                let mut moving = original.clone();
                moving.id = map
                    .find_free_linedef_id()
                    .expect("available map linedef ID");
                moving.creator_id = Uuid::new_v4();
                moving.sector_ids.retain(|id| self.sectors.contains(id));
                split_lines.insert(original.id, moving.id);
                map.find_linedef_mut(original.id)
                    .unwrap()
                    .sector_ids
                    .retain(|id| !self.sectors.contains(id));
                map.linedefs.push(moving);
                map.linedefs.last_mut().unwrap()
            } else {
                map.find_linedef_mut(original.id).unwrap()
            };
            for vertex in [&mut line.start_vertex, &mut line.end_vertex] {
                if let Some(id) = split_vertices.get(vertex) {
                    *vertex = *id;
                }
            }
        }
        for sector in &mut map.sectors {
            if self.sectors.contains(&sector.id) {
                for line in &mut sector.linedefs {
                    if let Some(id) = split_lines.get(line) {
                        *line = *id;
                    }
                }
                if let Some(anchor) = sector.properties.get_vec2("iso_anchor") {
                    self.anchors.push((sector.id, Vec2::from(anchor)));
                }
            }
        }
    }

    pub(super) fn translate(&mut self, region: &mut Region, offset: Vec2<f32>) {
        if offset == self.offset {
            return;
        }
        if offset == Vec2::zero() {
            // Returning the pointer to its starting point also rejoins split boundaries.
            region.map.clone_from(&self.before.map);
            self.vertices.clear();
            self.anchors.clear();
        } else {
            if self.offset == Vec2::zero() {
                self.prepare_geometry(&mut region.map);
            }
            for vertex in &mut region.map.vertices {
                if let Some(start) = self.vertices.get(&vertex.id) {
                    vertex.x = start.x + offset.x;
                    vertex.y = start.y + offset.y;
                }
            }
            for (id, start) in &self.anchors {
                if let Some(sector) = region.map.find_sector_mut(*id) {
                    sector
                        .properties
                        .set("iso_anchor", Value::Vec2((*start + offset).into_array()));
                }
            }
        }
        let delta = Vec3::new(offset.x, 0., offset.y);
        for id in &self.items {
            region.items[id].position = self.before.items[id].position + delta;
        }
        for id in &self.characters {
            region.characters[id].position = self.before.characters[id].position + delta;
        }
        for item in &mut region.map.items {
            if self.items.contains(&item.creator_id) {
                item.position = region.items[&item.creator_id].position;
            }
        }
        for entity in &mut region.map.entities {
            if self.characters.contains(&entity.creator_id) {
                entity.position = region.characters[&entity.creator_id].position;
            }
        }
        for i in &self.lights {
            region.map.lights[*i].set_position(self.before.map.lights[*i].position() + delta);
        }
        self.offset = offset;
    }
}
