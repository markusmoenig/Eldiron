//! Server-side projection and typed local delivery. A wire subscriber has its
//! own bounded queue; reading it never drains another player's events.
use super::spatial::{Bounds, Object, SpatialIndex, SpatialSubscriptions, actor_bounds};
use super::*;
use crate::{EventInterests, PlayerConnectionId, PlayerConnectionState, Project};
use rustc_hash::FxHashMap;
use rusterix::{Choice, Entity, Server, ValueContainer};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

#[derive(Clone)]
struct ProjectedEntity {
    data: Arc<EntitySnapshot>,
    appearance: Option<u64>,
}
#[derive(Clone)]
struct ProjectedRegion {
    content: RegionContentRef,
    entities: Vec<ProjectedEntity>,
    items: Vec<Arc<ItemSnapshot>>,
    props: Vec<BlockPropInstance>,
    resource_views: Vec<ResourceView>,
    time: TheTime,
    render: RuntimeRenderState,
}
impl ProjectedRegion {
    fn baseline(&self) -> RegionSnapshot {
        RegionSnapshot {
            content: self.content,
            entities: self.entities.iter().map(|e| (*e.data).clone()).collect(),
            items: self.items.iter().map(|i| (**i).clone()).collect(),
            props: self.props.clone(),
            resource_views: self.resource_views.clone(),
            time: self.time,
            render: self.render.clone(),
        }
    }
}
#[derive(Clone)]
struct State {
    regions: Vec<ProjectedRegion>,
    current: Option<Uuid>,
    controlled: Option<u32>,
    state: ServerState,
    motion: Option<rusterix::movement::MotionSnapshot>,
}
struct Stream {
    previous: State,
    generation: u64,
    sequence: u64,
    client: ClientWorld,
    outgoing: Option<VecDeque<ClientFrame>>,
    overflowed: bool,
    offers: Vec<Choice>,
    force_baseline: bool,
}
pub(crate) struct ReplicationHub {
    pub policy: ReplicationPolicy,
    public_entities: FxHashMap<u32, ProjectedEntity>,
    public_items: FxHashMap<u32, Arc<ItemSnapshot>>,
    appearance_serial: u64,
    peak_stream_frames: usize,
    streams: FxHashMap<PlayerConnectionId, Stream>,
    spatial: FxHashMap<PlayerConnectionId, SpatialSubscriptions>,
    prop_bounds: FxHashMap<Uuid, (u32, u64, rusterix::BlockPropInstance, Bounds)>,
}
impl Default for ReplicationHub {
    fn default() -> Self {
        Self {
            policy: Default::default(),
            public_entities: Default::default(),
            public_items: Default::default(),
            appearance_serial: 0,
            peak_stream_frames: 0,
            streams: Default::default(),
            spatial: Default::default(),
            prop_bounds: Default::default(),
        }
    }
}
impl ReplicationHub {
    pub fn diagnostics(&self) -> crate::diagnostics::ReplicationDiagnostics {
        let mut d = crate::diagnostics::ReplicationDiagnostics {
            streams: self.streams.len(),
            peak_stream_frames: self.peak_stream_frames,
            ..Default::default()
        };
        for stream in self.streams.values() {
            if let Some(queue) = &stream.outgoing {
                d.subscribed_streams += 1;
                d.queued_frames += queue.len();
            }
            d.overflowed_streams += usize::from(stream.overflowed);
            d.pending_events += stream.client.pending_event_count();
            for region in &stream.previous.regions {
                d.projected_entities += region.entities.len();
                d.projected_items += region.items.len();
            }
        }
        d
    }
    pub fn set_policy(&mut self, policy: ReplicationPolicy) {
        self.policy = policy;
        self.public_entities.clear();
        self.public_items.clear();
        for stream in self.streams.values_mut() {
            stream.force_baseline = true;
        }
    }
    pub fn set_spatial(
        &mut self,
        id: PlayerConnectionId,
        subscriptions: Option<SpatialSubscriptions>,
    ) -> Result<(), ReplicationError> {
        if let Some(config) = &subscriptions {
            config.validate()?;
        }
        if let Some(config) = subscriptions {
            self.spatial.insert(id, config);
        } else {
            self.spatial.remove(&id);
        }
        if let Some(stream) = self.streams.get_mut(&id) {
            stream.force_baseline = true;
        }
        Ok(())
    }
    pub fn prefetch(&self, id: PlayerConnectionId) -> impl Iterator<Item = Uuid> + '_ {
        self.spatial
            .get(&id)
            .into_iter()
            .flat_map(|s| s.prefetch.iter().copied())
    }
    pub fn client(&self, id: PlayerConnectionId) -> Option<&ClientWorld> {
        self.streams.get(&id).map(|s| &s.client)
    }
    pub fn client_mut(&mut self, id: PlayerConnectionId) -> Option<&mut ClientWorld> {
        self.streams.get_mut(&id).map(|s| &mut s.client)
    }
    pub fn clear(&mut self) {
        self.public_entities.clear();
        self.public_items.clear();
        self.streams.clear();
        self.spatial.clear();
        self.prop_bounds.clear();
    }
    pub fn remove(&mut self, id: PlayerConnectionId) {
        self.streams.remove(&id);
        self.spatial.remove(&id);
    }
    pub fn clear_events(&mut self) {
        for stream in self.streams.values_mut() {
            stream.client.clear_events();
        }
    }

    pub fn subscribe(
        &mut self,
        id: PlayerConnectionId,
        world: Uuid,
        tick: u64,
        movement_tick: u64,
    ) -> Result<(), ReplicationError> {
        let stream = self
            .streams
            .get_mut(&id)
            .ok_or(ReplicationError::UnknownConnection)?;
        let baseline = frame(
            world,
            id,
            stream.generation,
            stream.sequence,
            tick,
            movement_tick,
            &stream.previous,
            WorldUpdate::Baseline(
                stream
                    .previous
                    .regions
                    .iter()
                    .map(ProjectedRegion::baseline)
                    .collect(),
            ),
            vec![],
        );
        stream.outgoing = Some(VecDeque::from([baseline]));
        stream.overflowed = false;
        Ok(())
    }
    pub fn take_frames(
        &mut self,
        id: PlayerConnectionId,
    ) -> Result<Vec<ClientFrame>, ReplicationError> {
        let stream = self
            .streams
            .get_mut(&id)
            .ok_or(ReplicationError::UnknownConnection)?;
        if stream.overflowed {
            return Err(ReplicationError::Backpressure);
        }
        Ok(stream
            .outgoing
            .as_mut()
            .ok_or(ReplicationError::NeedsBaseline)?
            .drain(..)
            .collect())
    }
    pub fn resync(&mut self, id: PlayerConnectionId) -> Result<(), ReplicationError> {
        let stream = self
            .streams
            .get_mut(&id)
            .ok_or(ReplicationError::UnknownConnection)?;
        // The next publish replaces the baseline, keeping the last accepted
        // client state until the entire replacement and content are ready.
        stream.force_baseline = true;
        stream.outgoing.as_mut().map(VecDeque::clear);
        stream.overflowed = false;
        Ok(())
    }
    pub fn publish(
        &mut self,
        world: Uuid,
        tick: u64,
        movement_tick: u64,
        project: &Project,
        server: &Server,
        current_map: &str,
        raw: &FxHashMap<Uuid, RegionEvents>,
        local_interests: EventInterests,
    ) -> Result<(), ReplicationError> {
        self.appearance_serial = self.appearance_serial.checked_add(1).ok_or_else(|| {
            ReplicationError::InvalidFrame("Projection appearance counter exhausted".into())
        })?;
        self.streams
            .retain(|id, _| server.connection_state(*id).is_some());
        self.spatial
            .retain(|id, _| server.connection_state(*id).is_some());
        let indexed = !self.spatial.is_empty();
        let sources: Vec<_> = project
            .regions
            .iter()
            .map(|region| {
                let runtime_id = server.region_name_id_map.get(&region.name).copied();
                let occupants = runtime_id
                    .and_then(|id| {
                        server.instances.iter().find_map(|instance| {
                            let instance = instance.lock().ok()?;
                            if instance.id != id {
                                return None;
                            }
                            let context = instance.context()?;
                            let context = context.lock().ok()?;
                            let prop_bounds = if indexed {
                                context
                                    .map
                                    .block_prop_instances
                                    .iter()
                                    .map(|prop| {
                                        let revision = server.runtime_map_version(id);
                                        let cached = self
                                            .prop_bounds
                                            .get(&prop.id)
                                            .filter(|(map_revision, runtime_revision, old, _)| {
                                                *map_revision == region.map.changed
                                                    && *runtime_revision == revision
                                                    && old == prop
                                            })
                                            .map(|v| v.3);
                                        let bounds = cached.unwrap_or_else(|| {
                                            let resolved = rusterix::resolve_block_prop_geometry(
                                                std::slice::from_ref(prop),
                                                &context.assets.block_props,
                                            );
                                            let bounds = resolved
                                                .geometry_objects
                                                .iter()
                                                .filter_map(|object| object.bbox())
                                                .map(|b| Bounds {
                                                    min: [b.min.x, b.min.y],
                                                    max: [b.max.x, b.max.y],
                                                })
                                                .reduce(Bounds::union)
                                                .unwrap_or_else(|| {
                                                    Bounds::point([
                                                        prop.world_transform[3][0],
                                                        prop.world_transform[3][2],
                                                    ])
                                                });
                                            self.prop_bounds.insert(
                                                prop.id,
                                                (
                                                    region.map.changed,
                                                    revision,
                                                    prop.clone(),
                                                    bounds,
                                                ),
                                            );
                                            bounds
                                        });
                                        (prop.id, bounds)
                                    })
                                    .collect()
                            } else {
                                vec![]
                            };
                            Some((
                                (context.map.entities.clone(), context.map.items.clone()),
                                prop_bounds,
                            ))
                        })
                    })
                    .unwrap_or_else(|| {
                        (
                            (region.map.entities.clone(), region.map.items.clone()),
                            vec![],
                        )
                    });
                let ((entities, items), props) = occupants;
                let index = indexed.then(|| {
                    let mut index = SpatialIndex::new(super::spatial::CELL_SIZE);
                    for e in &entities {
                        index.insert(
                            Object::Entity(e.id),
                            actor_bounds([e.position.x, e.position.z], &e.attributes),
                        );
                    }
                    for i in &items {
                        index.insert(
                            Object::Item(i.id),
                            actor_bounds([i.position.x, i.position.z], &i.attributes),
                        );
                    }
                    for (id, bounds) in props {
                        index.insert(Object::Prop(id), bounds);
                    }
                    index
                });
                let public_entities: Vec<_> = entities
                    .iter()
                    .map(|entity| {
                        let snapshot = EntitySnapshot::capture(entity, None, &self.policy);
                        let old = self.public_entities.get(&entity.id);
                        let appearance = if let Some(old) =
                            old.filter(|old| old.data.same_appearance(&snapshot))
                        {
                            old.appearance
                        } else {
                            Some(self.appearance_serial)
                        };
                        let data = old
                            .filter(|old| *old.data == snapshot)
                            .map(|old| old.data.clone())
                            .unwrap_or_else(|| Arc::new(snapshot));
                        let projected = ProjectedEntity { data, appearance };
                        self.public_entities.insert(entity.id, projected.clone());
                        projected
                    })
                    .collect();
                let public_items: Vec<_> = items
                    .iter()
                    .map(|item| {
                        let snapshot = ItemSnapshot::capture(item, false, &self.policy);
                        let data = self
                            .public_items
                            .get(&item.id)
                            .filter(|old| ***old == snapshot)
                            .cloned()
                            .unwrap_or_else(|| Arc::new(snapshot));
                        self.public_items.insert(item.id, data.clone());
                        data
                    })
                    .collect();
                (
                    region,
                    runtime_id,
                    (entities, items, public_entities, public_items),
                    index,
                )
            })
            .collect();
        let live_entities: HashSet<_> = sources
            .iter()
            .flat_map(|(_, _, (entities, _, _, _), _)| entities.iter().map(|e| e.id))
            .collect();
        let live_items: HashSet<_> = sources
            .iter()
            .flat_map(|(_, _, (_, items, _, _), _)| items.iter().map(|i| i.id))
            .collect();
        self.public_entities
            .retain(|id, _| live_entities.contains(id));
        self.public_items.retain(|id, _| live_items.contains(id));
        let live_props: HashSet<_> = sources
            .iter()
            .flat_map(|(r, _, _, _)| r.map.block_prop_instances.iter().map(|p| p.id))
            .collect();
        self.prop_bounds.retain(|id, _| live_props.contains(id));
        // Resolve each region once for every recipient in this publication.
        let mut resource_views_by_region = FxHashMap::default();
        for instance in &server.instances {
            let context = instance.lock().unwrap().context();
            if let Some(context) = context {
                let context = context.lock().unwrap();
                let views = rusterix::server::region::resources::views(&context)
                    .map_err(ReplicationError::InvalidFrame)?;
                resource_views_by_region.insert(context.map.id, views);
            }
        }
        for (connection, registration) in server.player_connections() {
            let owner = registration.control().map(|control| control.entity_id);
            let selected = match registration {
                PlayerConnectionState::Active(control) => Some(control.region_id),
                PlayerConnectionState::Spawning { region_id } => Some(region_id),
                _ => None,
            };
            let current = sources.iter().find_map(|(region, id, _, _)| {
                (selected.is_some() && *id == selected
                    || (connection == server.local_connection()
                        && selected.is_none()
                        && region.map.name == current_map))
                    .then_some(region.map.id)
            });
            let access = server.player_container_access(connection);
            let mut offers = self
                .streams
                .get(&connection)
                .filter(|stream| {
                    stream.previous.controlled == owner && stream.previous.current == current
                })
                .map(|stream| stream.offers.clone())
                .unwrap_or_default();
            for events in raw.values() {
                for choices in &events.choices {
                    if Some(choices.to) != owner {
                        continue;
                    }
                    offers.retain(|choice| choice.session_meta().0 != choices.from);
                    offers.extend(
                        choices
                            .choices
                            .iter()
                            .filter(|choice| Some(choice.session_meta().1) == owner)
                            .cloned(),
                    );
                }
            }
            offers.retain(|choice| server.player_choice_is_active(connection, choice));
            let subscriptions = self.spatial.get(&connection);
            let center = sources
                .iter()
                .flat_map(|(_, _, (entities, _, _, _), _)| entities)
                .find(|e| Some(e.id) == owner)
                .map(|e| [e.position.x, e.position.z]);
            let previous = self.streams.get(&connection).map(|s| &s.previous.regions);
            let mut visibility = FxHashMap::default();
            let mut regions = Vec::new();
            for (region, runtime_id, (entities, items, public_entities, public_items), index) in
                &sources
            {
                let mut pins = HashSet::new();
                for entity in entities {
                    if Some(entity.id) == owner
                        || owner.is_some_and(|id| {
                            entity.attributes.get_uint("party_leader_id") == Some(id)
                        })
                        || subscriptions.is_some_and(|s| s.entities.contains(&entity.id))
                    {
                        pins.insert(Object::Entity(entity.id));
                    }
                }
                if *runtime_id == selected {
                    if let Some((item, entity)) = access {
                        pins.insert(entity.map(Object::Entity).unwrap_or(Object::Item(item)));
                    }
                    for choice in &offers {
                        pins.insert(Object::Entity(choice.session_meta().0));
                    }
                    if let Some(events) = raw.get(&region.map.id) {
                        for group in &events.choices {
                            if Some(group.to) == owner
                                && group
                                    .choices
                                    .iter()
                                    .any(|c| server.player_choice_is_active(connection, c))
                            {
                                pins.insert(Object::Entity(group.from));
                            }
                        }
                        for (to, text) in &events.world_text {
                            if Some(*to) == owner {
                                use rusterix::world_text::TextAnchor;
                                match text.anchor {
                                    TextAnchor::Entity(id) => {
                                        pins.insert(Object::Entity(id));
                                    }
                                    TextAnchor::Item(id) => {
                                        pins.insert(Object::Item(id));
                                    }
                                    TextAnchor::Prefab(id) => {
                                        pins.insert(Object::Prop(id));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                let whole = subscriptions.is_none()
                    || subscriptions.is_some_and(|s| s.regions.contains(&region.map.id));
                if !whole && Some(region.map.id) != current && pins.is_empty() {
                    continue;
                }
                let visible = if whole {
                    None
                } else {
                    let config = subscriptions.unwrap();
                    let mut visible = pins;
                    if Some(region.map.id) == current
                        && let (Some(center), Some(index)) = (center, index)
                    {
                        visible.extend(index.query(Bounds::around(center, config.policy.radius)));
                        let outer =
                            Bounds::around(center, config.policy.radius + config.policy.margin);
                        if let Some(old) = previous
                            .and_then(|rs| rs.iter().find(|r| r.content.map == region.map.id))
                        {
                            for id in old
                                .entities
                                .iter()
                                .map(|e| Object::Entity(e.data.id))
                                .chain(old.items.iter().map(|i| Object::Item(i.id)))
                                .chain(old.props.iter().map(|p| Object::Prop(p.id)))
                            {
                                if index.intersects(id, outer) {
                                    visible.insert(id);
                                }
                            }
                        }
                    }
                    Some(visible)
                };
                let includes = |id: Object| visible.as_ref().is_none_or(|set| set.contains(&id));
                let content = RegionContentRef {
                    map: region.map.id,
                    map_revision: region.map.changed,
                    runtime_revision: runtime_id
                        .map(|id| server.runtime_map_version(id))
                        .unwrap_or(0),
                };
                let selected_access = if *runtime_id == selected {
                    access
                } else {
                    None
                };
                let entities=entities.iter().zip(public_entities).filter(|(entity,_)|includes(Object::Entity(entity.id))).map(|(entity,public)|{
                    let private=owner.is_some_and(|owner|entity.id==owner || entity.attributes.get_uint("party_leader_id")==Some(owner));
                    let container=selected_access.is_some_and(|(_,owner)|owner==Some(entity.id));
                    let trade=offers.iter().any(|choice|matches!(choice,Choice::ItemToSell(_,seller,..) if *seller==entity.id));
                    if private || container || trade {ProjectedEntity{data:Arc::new(project_entity(entity,owner,selected_access,&offers,&self.policy)),appearance:None}}
                    else {public.clone()}
                }).collect();
                let items = items
                    .iter()
                    .zip(public_items)
                    .filter(|(item, _)| includes(Object::Item(item.id)))
                    .map(|(item, public)| {
                        if selected_access == Some((item.id, None)) {
                            Arc::new(ItemSnapshot::capture(item, true, &self.policy))
                        } else {
                            public.clone()
                        }
                    })
                    .collect();
                let props = region
                    .map
                    .block_prop_instances
                    .iter()
                    .filter(|p| includes(Object::Prop(p.id)))
                    .map(|prop| public_prop(prop, &self.policy))
                    .collect();
                visibility.insert(region.map.id, visible);
                regions.push(ProjectedRegion {
                    content,
                    entities,
                    items,
                    props,
                    resource_views: resource_views_by_region.get(&region.map.id).cloned().unwrap_or_default(),
                    time: server.get_time(&region.map.id).unwrap_or(project.time),
                    render: server.get_render_state(&region.map.id),
                });
            }
            let state = State {
                regions,
                current,
                controlled: owner,
                state: server.state,
                motion: server.player_motion_snapshot(connection),
            };
            let interests = if connection == server.local_connection() {
                local_interests
            } else {
                EventInterests::default()
            };
            let mut batches: FxHashMap<Uuid, RegionEvents> = FxHashMap::default();
            for (region, events) in raw {
                let mut events = events.for_player(owner, interests);
                if let Some(visible) = visibility.get(region) {
                    if let Some(visible) = visible {
                        // A remote party/target pin is not a subscription to
                        // that region's unpositioned music/ambient bus.
                        if Some(*region) != current {
                            events.audio.clear();
                        }
                        events.says.retain(|say| {
                            say.0.is_none_or(|id| visible.contains(&Object::Entity(id)))
                                && say.1.is_none_or(|id| visible.contains(&Object::Item(id)))
                        });
                    }
                    batches.entry(*region).or_default().append(events);
                } else if let Some(current) = current {
                    // Private addressed output survives a transfer; distant
                    // broadcasts and stale interactive permissions do not.
                    let batch = batches.entry(current).or_default();
                    batch.messages.append(&mut events.messages);
                    batch.world_text.append(&mut events.world_text);
                }
            }
            let deliveries = batches
                .into_iter()
                .filter_map(|(region, events)| {
                    (events.count() > 0).then_some(RegionDelivery { region, events })
                })
                .collect();
            if !self.streams.contains_key(&connection) {
                let client = ClientWorld::new(world, connection.uuid());
                self.streams.insert(
                    connection,
                    Stream {
                        previous: State {
                            regions: vec![],
                            current: None,
                            controlled: None,
                            state: ServerState::Off,
                            motion: None,
                        },
                        generation: 0,
                        sequence: 0,
                        client,
                        outgoing: None,
                        overflowed: false,
                        offers: vec![],
                        force_baseline: false,
                    },
                );
            }
            let stream = self.streams.get_mut(&connection).unwrap();
            let baseline = (state.motion.is_some() && state.state != stream.previous.state)
                || state
                    .motion
                    .as_ref()
                    .map(|m| (&m.parameters, m.predictable, m.revision))
                    != stream
                        .previous
                        .motion
                        .as_ref()
                        .map(|m| (&m.parameters, m.predictable, m.revision))
                || stream.force_baseline
                || stream.generation == 0
                || state.controlled != stream.previous.controlled
                || state.current != stream.previous.current
                || state.regions.iter().map(|r| r.content).collect::<Vec<_>>()
                    != stream
                        .previous
                        .regions
                        .iter()
                        .map(|r| r.content)
                        .collect::<Vec<_>>();
            let generation = if baseline {
                stream
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| ReplicationError::InvalidFrame("Generation exhausted".into()))?
            } else {
                stream.generation
            };
            let sequence = if baseline {
                1
            } else {
                stream
                    .sequence
                    .checked_add(1)
                    .ok_or_else(|| ReplicationError::InvalidFrame("Sequence exhausted".into()))?
            };
            let update = if baseline {
                WorldUpdate::Baseline(
                    state
                        .regions
                        .iter()
                        .map(ProjectedRegion::baseline)
                        .collect(),
                )
            } else {
                WorldUpdate::Delta(
                    state
                        .regions
                        .iter()
                        .zip(&stream.previous.regions)
                        .map(|(now, before)| diff(now, before))
                        .collect(),
                )
            };
            let packet = frame(
                world,
                connection,
                generation,
                sequence,
                tick,
                movement_tick,
                &state,
                update,
                deliveries,
            );
            for region in &state.regions {
                let source = sources
                    .iter()
                    .find(|(source, _, _, _)| source.map.id == region.content.map)
                    .unwrap();
                stream
                    .client
                    .install_content(region.content, &source.0.map)?;
            }
            stream.client.apply(&packet)?;
            stream.generation = generation;
            stream.sequence = sequence;
            stream.force_baseline = false;
            stream.offers = offers;
            if let Some(queue) = &mut stream.outgoing {
                if queue.len() >= MAX_QUEUED_FRAMES {
                    queue.clear();
                    stream.overflowed = true;
                } else if !stream.overflowed {
                    queue.push_back(packet);
                    self.peak_stream_frames = self.peak_stream_frames.max(queue.len());
                }
            }
            stream.previous = state;
        }
        Ok(())
    }
}

fn frame(
    world: Uuid,
    connection: PlayerConnectionId,
    generation: u64,
    sequence: u64,
    tick: u64,
    movement_tick: u64,
    state: &State,
    update: WorldUpdate,
    events: Vec<RegionDelivery>,
) -> ClientFrame {
    let mut motion = state.motion.clone();
    if let Some(motion) = &mut motion {
        if motion.generation != generation {
            motion.generation = generation;
            motion.processed_sequence = 0;
            motion.controller = Default::default();
        }
    }
    ClientFrame {
        version: REPLICATION_VERSION,
        world,
        connection: connection.uuid(),
        generation,
        sequence,
        tick,
        movement_tick,
        motion,
        controlled_entity: state.controlled,
        current_region: state.current,
        state: state.state,
        update,
        events,
    }
}
fn diff(now: &ProjectedRegion, before: &ProjectedRegion) -> RegionDelta {
    let old_entities: FxHashMap<_, _> = before.entities.iter().map(|e| (e.data.id, e)).collect();
    let old_items: FxHashMap<_, _> = before.items.iter().map(|i| (i.id, i)).collect();
    let new_entities: HashSet<_> = now.entities.iter().map(|e| e.data.id).collect();
    let new_items: HashSet<_> = now.items.iter().map(|i| i.id).collect();
    let mut entities = vec![];
    let mut poses = vec![];
    for entity in &now.entities {
        if let Some(old) = old_entities.get(&entity.data.id) {
            if Arc::ptr_eq(&old.data, &entity.data) {
                continue;
            }
            let same = match (old.appearance, entity.appearance) {
                (Some(a), Some(b)) => a == b,
                _ => old.data.same_appearance(&entity.data),
            };
            if same {
                if *old.data != *entity.data {
                    poses.push(EntityPose::capture(&entity.data));
                }
                continue;
            }
        }
        entities.push((*entity.data).clone());
    }
    let items = now
        .items
        .iter()
        .filter(|item| {
            old_items
                .get(&item.id)
                .is_none_or(|old| !Arc::ptr_eq(old, item) && ***old != ***item)
        })
        .map(|i| (**i).clone())
        .collect();
    RegionDelta {
        content: now.content,
        entities,
        poses,
        items,
        removed_entities: before
            .entities
            .iter()
            .filter(|e| !new_entities.contains(&e.data.id))
            .map(|e| e.data.id)
            .collect(),
        removed_items: before
            .items
            .iter()
            .filter(|i| !new_items.contains(&i.id))
            .map(|i| i.id)
            .collect(),
        props: (now.props != before.props).then(|| now.props.clone()),
        resource_views: (now.resource_views != before.resource_views).then(||now.resource_views.clone()),
        time: now.time,
        render: now.render.clone(),
    }
}

fn project_entity(
    entity: &Entity,
    owner: Option<u32>,
    access: Option<(u32, Option<u32>)>,
    offers: &[Choice],
    policy: &ReplicationPolicy,
) -> EntitySnapshot {
    let mut snapshot = EntitySnapshot::capture(entity, owner, policy);
    if snapshot.inventory.is_none() {
        if let Some((item, Some(entity_owner))) = access
            && entity_owner == entity.id
        {
            let root = entity
                .inventory
                .iter()
                .flatten()
                .chain(entity.equipped.values())
                .find(|i| i.id == item);
            if let Some(root) = root {
                snapshot.inventory = Some(vec![Some(ItemSnapshot::capture(root, true, policy))]);
            }
        }
    }
    if snapshot.wallet.is_none() {
        // A trade session reveals only the explicitly offered catalogue, never
        // the seller's other items, custom attributes, wallet or bag contents.
        let mut catalogue: Vec<_> = entity.inventory.iter().flatten().filter(|item| offers.iter().any(|choice| {
            matches!(choice, Choice::ItemToSell(id, seller, ..) if *id == item.id && *seller == entity.id)
        })).map(|item| {
            let mut projected = ItemSnapshot::capture(item, false, policy);
            if let Some(worth) = item.attributes.get("worth") { projected.attributes.insert("worth".into(), worth.clone()); }
            Some(projected)
        }).collect();
        if !catalogue.is_empty() {
            snapshot
                .inventory
                .get_or_insert_with(Vec::new)
                .append(&mut catalogue);
        }
    }
    snapshot
}
fn public_prop(source: &BlockPropInstance, policy: &ReplicationPolicy) -> BlockPropInstance {
    let mut prop = source.clone();
    prop.overrides = ValueContainer::default();
    prop.parameter_overrides = ValueContainer::default();
    prop.runtime_state = ValueContainer::default();
    for (key, value) in source.runtime_state.iter() {
        let door = key
            .strip_prefix("door_")
            .and_then(|s| s.strip_suffix("_open"))
            .is_some_and(|id| Uuid::parse_str(id).is_ok());
        if door || policy.prop_state.contains(key) {
            prop.runtime_state.set(key, value.clone());
        }
    }
    prop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prop_packets_copy_only_declared_public_runtime_state() {
        let mut source = BlockPropInstance::new(Uuid::new_v4());
        source
            .overrides
            .set("authoring", rusterix::Value::Str("SERVER_SCRIPT".into()));
        source
            .parameter_overrides
            .set("hidden", rusterix::Value::Str("PRIVATE_PARAMETER".into()));
        source
            .runtime_state
            .set("lock_code", rusterix::Value::Str("PRIVATE_STATE".into()));
        source
            .runtime_state
            .set("open", rusterix::Value::Bool(true));
        let door = format!("door_{}_open", Uuid::new_v4());
        source.runtime_state.set(&door, rusterix::Value::Bool(true));
        let packet = public_prop(&source, &ReplicationPolicy::default());
        let json = serde_json::to_string(&packet).unwrap();
        for private in ["SERVER_SCRIPT", "PRIVATE_PARAMETER", "PRIVATE_STATE"] {
            assert!(!json.contains(private));
        }
        assert_eq!(packet.runtime_state.get_bool("open"), Some(true));
        assert_eq!(packet.runtime_state.get_bool(&door), Some(true));
    }
}
