use super::*;
use crate::{Project, RuntimeView};
use rustc_hash::FxHashMap;
use rusterix::{Assets, Map};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplicationError {
    WrongVersion,
    WrongConnection,
    NeedsBaseline,
    MissingContent(RegionContentRef),
    InvalidFrame(String),
    Backpressure,
    UnknownConnection,
}
impl std::fmt::Display for ReplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ReplicationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyResult {
    Applied,
    IgnoredStale,
}

/// Owned, recipient-filtered CPU state. No Server, channels, simulation clock,
/// or GPU resources are needed to apply a frame or present this world.
pub struct ClientWorld {
    world: Uuid,
    connection: Uuid,
    generation: u64,
    sequence: u64,
    tick: u64,
    movement_tick: u64,
    motion: Option<rusterix::movement::MotionSnapshot>,
    view_id: Uuid,
    controlled: Option<u32>,
    current: Option<Uuid>,
    state: ServerState,
    content: FxHashMap<RegionContentRef, Arc<Map>>,
    regions: FxHashMap<Uuid, (RegionContentRef, Map, TheTime, RuntimeRenderState)>,
    entity_indices: FxHashMap<Uuid, FxHashMap<u32, usize>>,
    item_indices: FxHashMap<Uuid, FxHashMap<u32, usize>>,
    events: FxHashMap<Uuid, RegionEvents>,
    events_overflowed: bool,
}
impl ClientWorld {
    pub(crate) fn belongs_to(&self, world: Uuid, connection: Uuid) -> bool {
        self.world == world && self.connection == connection
    }
    pub fn new(world: Uuid, connection: Uuid) -> Self {
        Self {
            world,
            connection,
            generation: 0,
            sequence: 0,
            tick: 0,
            movement_tick: 0,
            motion: None,
            view_id: Uuid::new_v4(),
            controlled: None,
            current: None,
            state: ServerState::Off,
            content: Default::default(),
            regions: Default::default(),
            entity_indices: Default::default(),
            item_indices: Default::default(),
            events: Default::default(),
            events_overflowed: false,
        }
    }
    /// Content is provisioned separately from gameplay packets. Dynamic actors,
    /// inventories, and prop runtime state are never installed. Authored prop
    /// materials/parameters remain part of the separately provisioned content.
    pub fn install_content(
        &mut self,
        reference: RegionContentRef,
        source: &Map,
    ) -> Result<(), ReplicationError> {
        if source.id != reference.map || source.changed != reference.map_revision {
            return Err(ReplicationError::InvalidFrame(
                "Content revision does not match map".into(),
            ));
        }
        if self.content.contains_key(&reference) {
            return Ok(());
        }
        let mut map = source.clone();
        clear_occupants(&mut map);
        self.content.insert(reference, Arc::new(map));
        Ok(())
    }
    pub fn map(&self, id: Uuid) -> Option<&Map> {
        self.regions.get(&id).map(|r| &r.1)
    }
    pub fn maps(&self) -> impl Iterator<Item = &Map> {
        self.regions.values().map(|region| &region.1)
    }
    pub fn current_map(&self) -> Option<&Map> {
        self.map(self.current?)
    }
    pub fn controlled_entity(&self) -> Option<u32> {
        self.controlled
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn movement_tick(&self) -> u64 {
        self.movement_tick
    }
    pub fn motion(&self) -> Option<&rusterix::movement::MotionSnapshot> {
        self.motion.as_ref()
    }
    pub fn current_region(&self) -> Option<Uuid> {
        self.current
    }
    pub fn state(&self) -> ServerState {
        self.state
    }
    pub fn content_reference(&self, id: Uuid) -> Option<RegionContentRef> {
        self.regions.get(&id).map(|r| r.0)
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn take_events(&mut self, region: Uuid) -> RegionEvents {
        self.events.remove(&region).unwrap_or_default()
    }
    /// Transient output is bounded independently of world state. A stalled
    /// consumer must report this flag; old transient events cannot be recovered
    /// by resubscribing to a state baseline.
    pub(crate) fn pending_event_count(&self) -> usize {
        self.events.values().map(RegionEvents::count).sum()
    }
    pub fn events_overflowed(&self) -> bool {
        self.events_overflowed
    }
    pub fn take_event_overflow(&mut self) -> bool {
        std::mem::take(&mut self.events_overflowed)
    }
    pub(crate) fn clear_events(&mut self) {
        self.events.clear();
        self.events_overflowed = false;
    }
    pub(crate) fn pending_events(&mut self, region: &Uuid) -> Option<&mut RegionEvents> {
        self.events.get_mut(region)
    }

    /// Validation and content readiness are checked before touching visible
    /// state. Duplicate/stale packets cannot replay transient UI/audio events.
    pub fn apply(&mut self, frame: &ClientFrame) -> Result<ApplyResult, ReplicationError> {
        if frame.version != REPLICATION_VERSION {
            return Err(ReplicationError::WrongVersion);
        }
        if frame.world != self.world || frame.connection != self.connection {
            return Err(ReplicationError::WrongConnection);
        }
        if frame.generation < self.generation
            || (frame.generation == self.generation && frame.sequence <= self.sequence)
        {
            return Ok(ApplyResult::IgnoredStale);
        }
        if frame.generation == 0 || frame.sequence == 0 {
            return Err(ReplicationError::NeedsBaseline);
        }
        if matches!(frame.update, WorldUpdate::Delta(_))
            && (frame.generation != self.generation
                || Some(frame.sequence) != self.sequence.checked_add(1)
                || self.generation == 0)
        {
            return Err(ReplicationError::NeedsBaseline);
        }
        self.validate(frame)?;
        match &frame.update {
            WorldUpdate::Baseline(regions) => {
                let mut replacement = FxHashMap::default();
                for region in regions {
                    let mut map = self.content[&region.content].as_ref().clone();
                    map.entities = region
                        .entities
                        .iter()
                        .map(EntitySnapshot::materialize)
                        .collect();
                    map.items = region.items.iter().map(ItemSnapshot::materialize).collect();
                    map.block_prop_instances = materialize_props(&map, &region.props);
                    rusterix::server::region::resources::apply_views(&mut map, &region.resource_views);
                    replacement.insert(
                        region.content.map,
                        (region.content, map, region.time, region.render.clone()),
                    );
                }
                self.entity_indices = replacement
                    .iter()
                    .map(|(id, (_, map, _, _))| {
                        (
                            *id,
                            map.entities
                                .iter()
                                .enumerate()
                                .map(|(index, e)| (e.id, index))
                                .collect(),
                        )
                    })
                    .collect();
                self.item_indices = replacement
                    .iter()
                    .map(|(id, (_, map, _, _))| {
                        (
                            *id,
                            map.items
                                .iter()
                                .enumerate()
                                .map(|(index, i)| (i.id, index))
                                .collect(),
                        )
                    })
                    .collect();
                self.regions = replacement;
                if self.controlled != frame.controlled_entity {
                    self.events.clear();
                    self.events_overflowed = false;
                } else {
                    // Keep unread transcript/audio through content resync or
                    // transfer; interactive UI state belongs to the old view.
                    for events in self.events.values_mut() {
                        events.choices.clear();
                        events.containers.clear();
                    }
                }
                self.view_id = Uuid::new_v4();
                self.content
                    .retain(|key, _| self.regions.values().any(|r| r.0 == *key));
            }
            WorldUpdate::Delta(regions) => {
                for region in regions {
                    let (_, map, time, render) = self.regions.get_mut(&region.content.map).unwrap();
                    if !region.removed_entities.is_empty() {
                        let removed: HashSet<_> = region.removed_entities.iter().copied().collect();
                        map.entities.retain(|entity| !removed.contains(&entity.id));
                        self.entity_indices.insert(
                            region.content.map,
                            map.entities
                                .iter()
                                .enumerate()
                                .map(|(i, e)| (e.id, i))
                                .collect(),
                        );
                    }
                    if !region.entities.is_empty() || !region.poses.is_empty() {
                        let indices = self.entity_indices.get_mut(&region.content.map).unwrap();
                        for entity in &region.entities {
                            if let Some(index) = indices.get(&entity.id) {
                                map.entities[*index] = entity.materialize();
                            } else {
                                indices.insert(entity.id, map.entities.len());
                                map.entities.push(entity.materialize());
                            }
                        }
                        for pose in &region.poses {
                            pose.apply(&mut map.entities[indices[&pose.id]]);
                        }
                    }
                    if !region.removed_items.is_empty() {
                        let removed: HashSet<_> = region.removed_items.iter().copied().collect();
                        map.items.retain(|item| !removed.contains(&item.id));
                        self.item_indices.insert(
                            region.content.map,
                            map.items
                                .iter()
                                .enumerate()
                                .map(|(i, e)| (e.id, i))
                                .collect(),
                        );
                    }
                    if !region.items.is_empty() {
                        let indices = self.item_indices.get_mut(&region.content.map).unwrap();
                        for item in &region.items {
                            if let Some(index) = indices.get(&item.id) {
                                map.items[*index] = item.materialize();
                            } else {
                                indices.insert(item.id, map.items.len());
                                map.items.push(item.materialize());
                            }
                        }
                    }
                    if let Some(props) = &region.props {
                        map.block_prop_instances =
                            materialize_props(&self.content[&region.content], props);
                    }
                    if let Some(views) = &region.resource_views { rusterix::server::region::resources::apply_views(map, views); }
                    *time = region.time;
                    *render = region.render.clone();
                }
            }
        }
        self.generation = frame.generation;
        self.sequence = frame.sequence;
        self.tick = frame.tick;
        self.movement_tick = frame.movement_tick;
        self.motion = frame.motion.clone();
        self.controlled = frame.controlled_entity;
        self.current = frame.current_region;
        self.state = frame.state;
        let pending: usize = self.events.values().map(RegionEvents::count).sum();
        let incoming: usize = frame.events.iter().map(|d| d.events.count()).sum();
        if pending + incoming > MAX_PENDING_EVENTS {
            self.events_overflowed = true;
        } else {
            for delivery in &frame.events {
                self.events
                    .entry(delivery.region)
                    .or_default()
                    .append(delivery.events.clone());
            }
        }
        Ok(ApplyResult::Applied)
    }

    fn validate(&self, frame: &ClientFrame) -> Result<(), ReplicationError> {
        let invalid = |text: &str| ReplicationError::InvalidFrame(text.into());
        if let Some(motion) = &frame.motion {
            if motion.generation != frame.generation
                || !motion.parameters.valid()
                || !motion.controller.valid()
                || frame.controlled_entity.is_none()
            {
                return Err(invalid("Invalid motion metadata"));
            }
            if frame.generation == self.generation
                && self
                    .motion
                    .as_ref()
                    .is_some_and(|old| old.processed_sequence > motion.processed_sequence)
            {
                return Err(invalid("Movement acknowledgment regressed"));
            }
        }
        if frame.movement_tick < self.movement_tick {
            return Err(invalid("Movement tick regressed"));
        }
        if frame.tick < self.tick {
            return Err(invalid("Tick regressed"));
        }
        if matches!(frame.update, WorldUpdate::Delta(_))
            && (frame.controlled_entity != self.controlled || frame.current_region != self.current)
        {
            return Err(ReplicationError::NeedsBaseline);
        }
        if frame.generation == self.generation
            && (frame.controlled_entity != self.controlled || frame.current_region != self.current)
        {
            return Err(ReplicationError::NeedsBaseline);
        }
        let mut region_ids = HashSet::new();
        let mut object_count = 0;
        let mut pose_count = 0;
        let mut check = |content: RegionContentRef,
                         entities: &[EntitySnapshot],
                         items: &[ItemSnapshot]|
         -> Result<(), ReplicationError> {
            if !region_ids.insert(content.map) || region_ids.len() > MAX_REGIONS {
                return Err(invalid("Duplicate/too many regions"));
            }
            let mut ids = HashSet::new();
            for entity in entities {
                if entity.id == 0
                    || !ids.insert(entity.id)
                    || !entity
                        .position
                        .iter()
                        .chain(entity.orientation.iter())
                        .all(|v| v.is_finite())
                    || !entity.tilt.is_finite()
                {
                    return Err(invalid("Invalid entity"));
                }
                object_count += 1;
                if entity.attributes.len() > 4096
                    || entity
                        .inventory
                        .as_ref()
                        .is_some_and(|items| items.len() > MAX_OBJECTS)
                {
                    return Err(invalid("Oversized entity"));
                }
                for item in entity
                    .equipped
                    .values()
                    .chain(entity.inventory.iter().flatten().flatten())
                {
                    validate_item(item, 0, &mut object_count, &mut HashSet::new())?;
                }
            }
            ids.clear();
            for item in items {
                if !ids.insert(item.id) {
                    return Err(invalid("Duplicate item"));
                }
                validate_item(item, 0, &mut object_count, &mut HashSet::new())?;
            }
            if object_count > MAX_OBJECTS {
                return Err(invalid("Too many objects"));
            }
            Ok(())
        };
        match &frame.update {
            WorldUpdate::Baseline(regions) => {
                for region in regions {
                    check(region.content, &region.entities, &region.items)?;
                    validate_props(&region.props)?;
                    validate_resources(&region.resource_views)?;
                    if !self.content.contains_key(&region.content) {
                        return Err(ReplicationError::MissingContent(region.content));
                    }
                }
            }
            WorldUpdate::Delta(regions) => {
                for region in regions {
                    check(region.content, &region.entities, &region.items)?;
                    if let Some(props) = &region.props {
                        validate_props(props)?;
                    }
                    if let Some(views) = &region.resource_views { validate_resources(views)?; }
                    if self.regions.get(&region.content.map).map(|r| r.0) != Some(region.content) {
                        return Err(ReplicationError::NeedsBaseline);
                    }
                    let known = &self.entity_indices[&region.content.map];
                    let mut poses = HashSet::new();
                    for pose in &region.poses {
                        if pose.id == 0
                            || !poses.insert(pose.id)
                            || !known.contains_key(&pose.id)
                            || !pose
                                .position
                                .iter()
                                .chain(pose.orientation.iter())
                                .all(|v| v.is_finite())
                            || !pose.tilt.is_finite()
                        {
                            return Err(invalid("Invalid/unknown entity pose"));
                        }
                    }
                    pose_count += region.poses.len();
                    if pose_count > MAX_OBJECTS {
                        return Err(invalid("Too many objects"));
                    }
                    let mut entity_ids: HashSet<_> = region.entities.iter().map(|e| e.id).collect();
                    if !poses.is_disjoint(&entity_ids) {
                        return Err(invalid("Conflicting entity pose/replacement"));
                    }
                    entity_ids.extend(poses);
                    let item_ids: HashSet<_> = region.items.iter().map(|i| i.id).collect();
                    for (removals, upserts) in [
                        (&region.removed_entities, entity_ids),
                        (&region.removed_items, item_ids),
                    ] {
                        let unique: HashSet<_> = removals.iter().copied().collect();
                        if unique.len() != removals.len()
                            || unique.contains(&0)
                            || !unique.is_disjoint(&upserts)
                        {
                            return Err(invalid("Conflicting or duplicate removal"));
                        }
                    }
                }
            }
        }
        if object_count + pose_count > MAX_OBJECTS {
            return Err(invalid("Too many objects"));
        }
        let known_region = |id: &Uuid| match &frame.update {
            WorldUpdate::Baseline(regions) => regions.iter().any(|r| &r.content.map == id),
            _ => self.regions.contains_key(id),
        };
        if frame.current_region.is_some_and(|id| !known_region(&id)) {
            return Err(invalid("Unknown selected region"));
        }
        let event_count: usize = frame.events.iter().map(|d| d.events.count()).sum();
        if event_count > 4096 {
            return Err(invalid("Too many events"));
        }
        let mut deliveries = HashSet::new();
        for delivery in &frame.events {
            if !known_region(&delivery.region) || !deliveries.insert(delivery.region) {
                return Err(invalid("Invalid event region"));
            }
            let events = &delivery.events;
            if events
                .messages
                .iter()
                .any(|m| Some(m.2) != frame.controlled_entity)
                || events
                    .world_text
                    .iter()
                    .any(|m| Some(m.0) != frame.controlled_entity)
                || events.choices.iter().any(|choice| {
                    Some(choice.to) != frame.controlled_entity
                        || choice
                            .choices
                            .iter()
                            .any(|c| Some(c.session_meta().1) != frame.controlled_entity)
                })
                || !events.container_recipients.is_empty()
            {
                return Err(invalid("Misaddressed private event"));
            }
        }
        Ok(())
    }

    /// Presentation sources are static/local content; gameplay state is borrowed
    /// exclusively from this replica, without a server or GameSession.
    pub fn view<'a>(
        &'a self,
        project: &'a Project,
        assets: &'a Assets,
        region: Option<Uuid>,
    ) -> Option<RuntimeView<'a>> {
        let id = region.or(self.current)?;
        let (_, map, time, render) = self.regions.get(&id)?;
        Some(RuntimeView::from_replica(
            self.view_id,
            self.controlled,
            project,
            assets,
            Some(map),
            *time,
            self.state,
            render.clone(),
        ))
    }
    pub(crate) fn empty_view<'a>(
        &self,
        project: &'a Project,
        assets: &'a Assets,
    ) -> RuntimeView<'a> {
        RuntimeView::from_replica(
            self.view_id,
            self.controlled,
            project,
            assets,
            None,
            project.time,
            self.state,
            RuntimeRenderState::default(),
        )
    }
}

fn clear_occupants(map: &mut Map) {
    map.entities.clear();
    map.items.clear();
    for prop in &mut map.block_prop_instances {
        prop.runtime_state = Default::default();
    }
    for profile in map.profiles.values_mut() {
        clear_occupants(profile);
    }
}
fn materialize_props(content: &Map, snapshots: &[BlockPropInstance]) -> Vec<BlockPropInstance> {
    snapshots
        .iter()
        .map(|snapshot| {
            let mut prop = snapshot.clone();
            if let Some(authored) = content
                .block_prop_instances
                .iter()
                .find(|p| p.id == prop.id && p.asset_id == prop.asset_id)
            {
                prop.overrides.clone_from(&authored.overrides);
                prop.parameter_overrides
                    .clone_from(&authored.parameter_overrides);
            }
            prop
        })
        .collect()
}
fn validate_item(
    item: &ItemSnapshot,
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<u32>,
) -> Result<(), ReplicationError> {
    *count += 1;
    if depth > 32
        || *count > MAX_OBJECTS
        || item.id == 0
        || !ids.insert(item.id)
        || item.attributes.len() > 4096
        || !item.position.iter().all(|v| v.is_finite())
    {
        return Err(ReplicationError::InvalidFrame("Invalid item tree".into()));
    }
    if let Some(contents) = &item.contents {
        if contents.len() > MAX_OBJECTS {
            return Err(ReplicationError::InvalidFrame(
                "Too many container items".into(),
            ));
        }
        for child in contents {
            validate_item(child, depth + 1, count, ids)?;
        }
    }
    Ok(())
}
fn validate_props(props: &[BlockPropInstance]) -> Result<(), ReplicationError> {
    let mut ids = HashSet::new();
    if props.len() > MAX_OBJECTS
        || props.iter().any(|p| {
            !ids.insert(p.id) || !p.world_transform.iter().flatten().all(|v| v.is_finite())
        })
    {
        return Err(ReplicationError::InvalidFrame("Invalid props".into()));
    }
    Ok(())
}

fn validate_resources(views: &[ResourceView]) -> Result<(), ReplicationError> {
    let mut ids=HashSet::new();
    if views.len()>MAX_OBJECTS || views.iter().any(|v|v.access.is_nil() || !ids.insert(v.access) || v.source.len()>256 || v.profile.len()>128 || v.phase.len()>128 || v.appearance.len()>128 || v.supplies.len()>128 || v.supplies.keys().any(|k|k.len()>128)) {
        return Err(ReplicationError::InvalidFrame("Invalid resource views".into()));
    }
    Ok(())
}
