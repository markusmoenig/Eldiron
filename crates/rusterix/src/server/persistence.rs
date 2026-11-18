//! World storage records. These are not replication DTOs.
use super::{
    currency::Wallet,
    entity::{EntitySequence, EntitySequenceState},
    message::RuntimeRenderState,
    nodes::region::SavedBehaviors,
};
use crate::{BlockPropInstance, EntityAction, GeometryObject, Surface, prelude::*, vm::VMValue};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use theframework_core::prelude::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedItem {
    pub id: u32,
    pub creator_id: Uuid,
    pub position: Vec3<f32>,
    pub item_type: String,
    pub max_capacity: u32,
    pub attributes: ValueContainer,
    pub contents: Option<Vec<SavedItem>>,
}
impl SavedItem {
    pub fn capture(item: &Item) -> Self {
        Self {
            id: item.id,
            creator_id: item.creator_id,
            position: item.position,
            item_type: item.item_type.clone(),
            max_capacity: item.max_capacity,
            attributes: item.attributes.clone(),
            contents: item
                .container
                .as_ref()
                .map(|v| v.iter().map(Self::capture).collect()),
        }
    }
    pub fn materialize(&self) -> Item {
        let mut item = Item {
            id: self.id,
            creator_id: self.creator_id,
            position: self.position,
            item_type: self.item_type.clone(),
            max_capacity: self.max_capacity,
            attributes: self.attributes.clone(),
            container: self
                .contents
                .as_ref()
                .map(|v| v.iter().map(Self::materialize).collect()),
            ..Item::default()
        };
        item.mark_all_dirty();
        item
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedEntity {
    pub id: u32,
    pub creator_id: Uuid,
    pub position: Vec3<f32>,
    pub orientation: Vec2<f32>,
    pub tilt: f32,
    pub attributes: ValueContainer,
    pub inventory: Vec<Option<SavedItem>>,
    pub equipped: Vec<(String, SavedItem)>,
    pub wallet: Wallet,
    pub sequences: Vec<(String, EntitySequence)>,
    pub active_sequence: Option<EntitySequenceState>,
    pub paused_sequence: Option<EntitySequenceState>,
    pub action: EntityAction,
}
impl SavedEntity {
    pub fn capture(entity: &Entity) -> Self {
        let mut attributes = entity.attributes.clone();
        // Device/input state is never a durable command. Conditions, cooldowns,
        // lifecycle and behavior attributes are intentionally retained.
        for key in [
            "__frame_motion",
            "__frame_moved",
            "__grid_desired_action",
            "__grid_blocked_action",
            "__grid_goto_target",
        ] {
            attributes.remove(key);
        }
        Self {
            id: entity.id,
            creator_id: entity.creator_id,
            position: entity.position,
            orientation: entity.orientation,
            tilt: entity.tilt,
            attributes,
            inventory: entity
                .inventory
                .iter()
                .map(|i| i.as_ref().map(SavedItem::capture))
                .collect(),
            equipped: entity
                .equipped
                .iter()
                .map(|(k, v)| (k.clone(), SavedItem::capture(v)))
                .collect(),
            wallet: entity.wallet.clone(),
            sequences: entity
                .sequences
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            active_sequence: entity.active_sequence.clone(),
            paused_sequence: entity.paused_sequence.clone(),
            action: if entity.is_player() {
                EntityAction::Off
            } else {
                entity.action.clone()
            },
        }
    }
    pub fn materialize(&self) -> Entity {
        let mut entity = Entity {
            id: self.id,
            creator_id: self.creator_id,
            position: self.position,
            orientation: self.orientation,
            tilt: self.tilt,
            attributes: self.attributes.clone(),
            inventory: self
                .inventory
                .iter()
                .map(|i| i.as_ref().map(SavedItem::materialize))
                .collect(),
            equipped: self
                .equipped
                .iter()
                .map(|(k, v)| (k.clone(), v.materialize()))
                .collect(),
            wallet: self.wallet.clone(),
            sequences: self.sequences.iter().cloned().collect(),
            active_sequence: self.active_sequence.clone(),
            paused_sequence: self.paused_sequence.clone(),
            action: self.action.clone(),
            ..Entity::default()
        };
        entity.mark_all_dirty();
        entity
    }
}
/// Runtime geometry can differ from authored geometry (procedural maps, doors,
/// terrain). Explicitly omit editor selection, UI, occupants and baked images.
#[derive(Clone, Serialize, Deserialize)]
pub struct SavedGeometry {
    pub id: Uuid,
    pub name: String,
    pub changed: u32,
    pub offset: Vec2<f32>,
    pub grid_size: f32,
    pub subdivisions: f32,
    pub vertices: Vec<crate::map::vertex::Vertex>,
    pub linedefs: Vec<crate::map::linedef::Linedef>,
    pub sectors: Vec<crate::map::sector::Sector>,
    pub geometry_objects: Vec<GeometryObject>,
    pub props: Vec<BlockPropInstance>,
    pub placements: Vec<crate::map::block_prop::BlockPropSurfacePlacement>,
    pub lights: Vec<Light>,
    pub properties: ValueContainer,
    pub surfaces: Vec<(Uuid, Surface)>,
    pub profiles: Vec<(Uuid, SavedGeometry)>,
    pub face_emissions: Vec<(Uuid, crate::FaceEmission)>,
    pub face_particle_emissions: Vec<(Uuid, crate::FaceParticleEmission)>,
}
impl SavedGeometry {
    pub fn capture(map: &Map) -> Self {
        Self {
            id: map.id,
            name: map.name.clone(),
            changed: map.changed,
            offset: map.offset,
            grid_size: map.grid_size,
            subdivisions: map.subdivisions,
            vertices: map.vertices.clone(),
            linedefs: map.linedefs.clone(),
            sectors: map.sectors.clone(),
            geometry_objects: map.geometry_objects.clone(),
            props: map.block_prop_instances.clone(),
            placements: map.block_prop_surface_placements.clone(),
            lights: map.lights.clone(),
            properties: map.properties.clone(),
            surfaces: map.surfaces.iter().map(|(k, v)| (*k, v.clone())).collect(),
            profiles: map
                .profiles
                .iter()
                .map(|(k, v)| (*k, Self::capture(v)))
                .collect(),
            face_emissions: map
                .face_emissions
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            face_particle_emissions: map
                .face_particle_emissions
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        }
    }
    pub fn materialize(&self) -> Map {
        let mut map = Map::default();
        map.id = self.id;
        map.name = self.name.clone();
        map.changed = self.changed;
        map.offset = self.offset;
        map.grid_size = self.grid_size;
        map.subdivisions = self.subdivisions;
        map.vertices = self.vertices.clone();
        map.linedefs = self.linedefs.clone();
        map.sectors = self.sectors.clone();
        map.geometry_objects = self.geometry_objects.clone();
        map.block_prop_instances = self.props.clone();
        map.block_prop_surface_placements = self.placements.clone();
        map.lights = self.lights.clone();
        map.properties = self.properties.clone();
        map.surfaces = self.surfaces.iter().cloned().collect();
        map.profiles = self
            .profiles
            .iter()
            .map(|(k, v)| (*k, v.materialize()))
            .collect();
        map.face_emissions = self.face_emissions.iter().cloned().collect();
        map.face_particle_emissions = self.face_particle_emissions.iter().cloned().collect();
        map
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RegionRecord {
    pub geometry: SavedGeometry,
    pub entities: Vec<SavedEntity>,
    pub items: Vec<SavedItem>,
    pub stashed: Vec<SavedEntity>,
    pub respawns: Vec<(u32, SavedEntity)>,
    pub values: ValueContainer,
    pub ticks: i64,
    pub time: TheTime,
    pub entity_classes: Vec<(u32, String)>,
    pub item_classes: Vec<(u32, String)>,
    pub entity_state: Vec<(u32, ValueContainer)>,
    pub item_state: Vec<(u32, ValueContainer)>,
    pub entity_alerts: Vec<(u32, f32)>,
    pub item_alerts: Vec<(u32, f32)>,
    pub entity_notifications: Vec<(u32, i64, String)>,
    pub item_notifications: Vec<(u32, i64, String)>,
    pub talk_steps: Vec<(u32, String)>,
    pub talk_resumes: Vec<(u32, String)>,
    pub dismiss_dialogues: Vec<u32>,
    pub entity_events: Vec<(u32, String, VMValue)>,
    pub item_events: Vec<(u32, String, VMValue)>,
    pub world_events: Vec<(String, VMValue)>,
    pub behaviors: SavedBehaviors,
    pub globals: Vec<VMValue>,
    pub simulation_time: Duration,
    pub last_advance: Duration,
    pub last_request: Option<Duration>,
    pub pending_system: u32,
    pub pending_redraw: u32,
    pub step_pending: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct WorldRecord {
    pub next_id: u32,
    pub values: ValueContainer,
    pub regions: Vec<RegionRecord>,
    pub render: RuntimeRenderState,
    pub region_render: Vec<(Uuid, RuntimeRenderState)>,
}
impl WorldRecord {
    /// IDs are durable world-local serials, not connection handles. Preserve the
    /// allocator high-water mark, including IDs of already destroyed objects.
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::HashSet;
        let mut ids = HashSet::new();
        let mut maps = HashSet::new();
        fn item(
            i: &SavedItem,
            ids: &mut HashSet<u32>,
            next: u32,
            depth: usize,
        ) -> Result<(), String> {
            if depth > 32
                || i.id == 0
                || i.id >= next
                || !ids.insert(i.id)
                || !i.position.into_array().iter().all(|v| v.is_finite())
            {
                return Err("Invalid/duplicate saved item identity or position".into());
            }
            if let Some(items) = &i.contents {
                for i in items {
                    item(i, ids, next, depth + 1)?;
                }
            }
            Ok(())
        }
        for region in &self.regions {
            if !maps.insert(region.geometry.id) {
                return Err("Duplicate saved region".into());
            }
            for e in region.entities.iter().chain(&region.stashed) {
                if e.id == 0
                    || e.id >= self.next_id
                    || !ids.insert(e.id)
                    || !e
                        .position
                        .into_array()
                        .iter()
                        .chain(e.orientation.into_array().iter())
                        .all(|v| v.is_finite())
                    || !e.tilt.is_finite()
                {
                    return Err("Invalid/duplicate saved character identity or pose".into());
                }
                for i in e
                    .inventory
                    .iter()
                    .flatten()
                    .chain(e.equipped.iter().map(|(_, i)| i))
                {
                    item(i, &mut ids, self.next_id, 0)?;
                }
            }
            for i in &region.items {
                item(i, &mut ids, self.next_id, 0)?;
            }
        }
        if self.next_id == 0 {
            return Err("Invalid saved ID allocator".into());
        }
        Ok(())
    }
}

pub struct PreparedWorld {
    pub(crate) record: WorldRecord,
    pub(crate) regions: Vec<super::region::PreparedRegion>,
}
impl super::Server {
    pub fn persistence_ready(&self) -> bool {
        !self
            .connections
            .values()
            .any(|s| matches!(s, super::PlayerConnectionState::Spawning { .. }))
            && self.instances.iter().all(|i| {
                let i = i.lock().unwrap();
                i.durable_commands_settled()
                    && i.context()
                        .is_some_and(|c| c.lock().unwrap().pending_entity_transfers.is_empty())
            })
    }
    pub fn capture_durable_world(&self) -> Result<WorldRecord, String> {
        if !self.persistence_ready() {
            return Err("World has unsettled commands or transfers; poll before checkpoint".into());
        }
        let record = WorldRecord {
            next_id: self.world.high_water(),
            values: self.world.snapshot(),
            regions: self
                .instances
                .iter()
                .map(|i| i.lock().unwrap().capture_durable_region())
                .collect::<Result<_, _>>()?,
            render: self.world_render.clone(),
            region_render: self
                .region_id_map
                .iter()
                .filter_map(|(uuid, id)| self.region_render.get(id).map(|r| (*uuid, r.clone())))
                .collect(),
        };
        record.validate()?;
        Ok(record)
    }
    pub fn prepare_durable_world(&self, record: &WorldRecord) -> Result<PreparedWorld, String> {
        record.validate()?;
        if record.next_id < self.world.high_water() {
            return Err("Saved object allocator cannot move backwards".into());
        }
        if record.next_id < self.world.high_water() {
            return Err("Saved ID allocator cannot move backwards".into());
        }
        if record.regions.len() != self.instances.len() {
            return Err("Saved region set differs from content".into());
        }
        let mut regions = Vec::new();
        for instance in &self.instances {
            let instance = instance.lock().unwrap();
            let context = instance.context().ok_or("Region not initialized")?;
            let id = context.lock().unwrap().map.id;
            let region = record
                .regions
                .iter()
                .find(|r| r.geometry.id == id)
                .ok_or("Saved region missing from content")?;
            regions.push(instance.prepare_durable_region(region)?);
        }
        Ok(PreparedWorld {
            record: record.clone(),
            regions,
        })
    }
    /// All fallible reconstruction completes before store commit/publication.
    pub fn apply_durable_world(&mut self, prepared: PreparedWorld) {
        self.world
            .restore(prepared.record.next_id, prepared.record.values);
        for (instance, region) in self.instances.iter().zip(prepared.regions) {
            instance.lock().unwrap().apply_durable_region(region);
        }
        self.world_render = prepared.record.render;
        self.region_render = prepared
            .record
            .region_render
            .into_iter()
            .filter_map(|(id, r)| self.region_id_map.get(&id).map(|runtime| (*runtime, r)))
            .collect();
        self.entities.clear();
        self.items.clear();
        self.runtime_maps.clear();
        self.block_prop_instances.clear();
        for instance in &self.instances {
            let instance = instance.lock().unwrap();
            let context = instance.context().unwrap();
            let ctx = context.lock().unwrap();
            self.entities.insert(instance.id, ctx.map.entities.clone());
            self.items.insert(instance.id, ctx.map.items.clone());
            self.runtime_maps.insert(instance.id, ctx.map.clone());
            self.block_prop_instances
                .insert(instance.id, ctx.map.block_prop_instances.clone());
            self.times.insert(instance.id, ctx.time);
        }
        // An atomic region transfer changes one body location and all attached
        // connections together. A removed body loses control.
        for state in self.connections.values_mut() {
            if let super::PlayerConnectionState::Active(control) = state {
                if let Some((region_id, _)) = self
                    .entities
                    .iter()
                    .find(|(_, entities)| entities.iter().any(|e| e.id == control.entity_id))
                {
                    control.region_id = *region_id;
                } else if let Some(instance) = self.instances.iter().find(|i| {
                    i.lock()
                        .unwrap()
                        .context()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .stashed_entities
                        .contains_key(&control.entity_id)
                }) {
                    control.region_id = instance.lock().unwrap().id;
                } else {
                    *state = super::PlayerConnectionState::AwaitingCharacter;
                }
            }
        }
    }
    pub fn attach_saved_character(
        &mut self,
        connection: super::PlayerConnectionId,
        region: Uuid,
        entity: u32,
    ) -> Result<(), String> {
        if self.connection_state(connection)
            != Some(super::PlayerConnectionState::AwaitingCharacter)
        {
            return Err("Connection already controls a character".into());
        }
        if self
            .connections
            .values()
            .any(|s| s.control().is_some_and(|c| c.entity_id == entity))
        {
            return Err("Saved character is already controlled".into());
        }
        let id = *self
            .region_id_map
            .get(&region)
            .ok_or("Unknown saved character region")?;
        let instance = self
            .instances
            .iter()
            .find(|i| i.lock().unwrap().id == id)
            .ok_or("Region unavailable")?;
        let context = instance
            .lock()
            .unwrap()
            .context()
            .ok_or("Region unavailable")?;
        if !context
            .lock()
            .unwrap()
            .find_entity(entity)
            .is_some_and(|e| e.is_player())
        {
            return Err("Saved character missing".into());
        }
        self.connections.insert(
            connection,
            super::PlayerConnectionState::Active(super::PlayerControl {
                region_id: id,
                entity_id: entity,
            }),
        );
        Ok(())
    }
    pub fn set_restore_bootstrap(&mut self, restoring: bool) {
        self.restore_bootstrap = restoring;
    }
}
