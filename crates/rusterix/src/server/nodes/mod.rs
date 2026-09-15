//! Node-only behavior runtime. No VM, renderer, or authoring widget dependencies.
//! Modules compile persisted parameters once; the scheduler only invokes operations.
use super::event_observation::{EventField, EventObservation, EventOwner};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use theframework::prelude::Uuid;

mod builtins;
pub use builtins::time_range_contains;
pub mod region;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorHandle {
    pub identity: Uuid,
    pub incarnation: Uuid,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Awake,
    Sleeping,
}
pub struct Actor {
    pub handle: ActorHandle,
    pub map: Uuid,
    pub render_id: u32,
    pub visible: bool,
    pub activity: Activity,
    graph: Option<Plan>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    pub event: EventObservation,
    pub node: Uuid,
    pub connection: Option<Uuid>,
    pub error: Option<String>,
    pub running: bool,
}
/// Execution snapshots are separate from the bounded diagnostic trace history.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExecutionPath {
    pub nodes: Vec<Uuid>,
    pub connections: Vec<Uuid>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Highlights {
    pub actor: ActorHandle,
    pub event: EventObservation,
    pub revision: Uuid,
    pub active: ExecutionPath,
    pub recent: ExecutionPath,
}
pub struct EventContext<'a> {
    pub actor: &'a Actor,
    pub event: &'a EventObservation,
    pub time: theframework::prelude::TheTime,
}
/// Modules access world functionality through services, never the legacy RegionCtx.
pub trait WorldServices {
    fn random_walk_active(&self, _actor: &Actor) -> bool {
        true
    }
    fn go_to(&mut self, _actor: &Actor, _destination: &str, _speed: f32) -> Result<(), String> {
        Err("Go To requires a character".into())
    }
    fn go_to_result(&mut self, _actor: &Actor) -> Option<Result<(), String>> {
        None
    }

    fn cancel_activity(&mut self, _actor: &Actor) {}
    fn refresh_random_walk(
        &mut self,
        _actor: &Actor,
        _area: &str,
        _distance: f32,
        _speed: f32,
        _pause: i32,
    ) -> Result<(), String> {
        Ok(())
    }

    fn time(&self, actor: &Actor) -> theframework::prelude::TheTime;
    fn say(&mut self, actor: &Actor, text: String);
    fn random_walk(
        &mut self,
        _actor: &Actor,
        _area: &str,
        _distance: f32,
        _speed: f32,
        _pause: i32,
    ) -> Result<bool, String> {
        Err("Random Walk is unavailable in this context".into())
    }
    fn resume_routine(&mut self, _actor: &Actor) -> Result<(), String> {
        Err("Routine is unavailable in this context".into())
    }
    fn player_camera(&mut self, _actor: &Actor, _camera: &str) -> Result<(), String> {
        Err("Player camera service is unavailable in this context".into())
    }
}
pub trait Operation: Send + Sync {
    fn condition(&self, _time: theframework::prelude::TheTime) -> Option<bool> {
        None
    }
    fn error_output(&self) -> Option<&'static str> {
        None
    }
    fn poll(
        &self,
        _ctx: &EventContext<'_>,
        _world: &mut dyn WorldServices,
    ) -> Option<Result<&'static str, String>> {
        None
    }

    fn retains_activity(&self, _output: &str) -> bool {
        false
    }
    fn refresh(
        &self,
        _context: &EventContext<'_>,
        _world: &mut dyn WorldServices,
    ) -> Result<(), String> {
        Ok(())
    }

    fn event(&self) -> Option<&str> {
        None
    }
    fn execute(
        &self,
        context: &EventContext<'_>,
        world: &mut dyn WorldServices,
    ) -> Result<&'static str, String>;
}
pub trait NodeModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn inputs(&self) -> &'static [&'static str];
    fn outputs(&self) -> &'static [&'static str];
    fn compile(&self, parameters: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String>;
}
#[derive(Default)]
pub struct Registry {
    modules: HashMap<String, Box<dyn NodeModule>>,
}
impl Registry {
    pub fn same_program(a: &Value, b: &Value) -> bool {
        let read = |value: &Value| {
            serde_json::from_value::<Document>(value.clone())
                .ok()
                .and_then(|doc| serde_json::to_value(doc).ok())
        };
        match (read(a), read(b)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
    pub fn builtin() -> Self {
        let mut r = Self::default();
        builtins::register(&mut r);
        r
    }
    pub fn register(&mut self, module: Box<dyn NodeModule>) -> Result<(), String> {
        if self.modules.contains_key(module.id()) {
            return Err(format!("Duplicate node module: {}", module.id()));
        }
        self.modules.insert(module.id().into(), module);
        Ok(())
    }
    /// Read only the semantic subset of the versioned graph document.
    pub fn compile(&self, value: &Value) -> Result<Plan, String> {
        let doc: Document = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if doc.version != 1 {
            return Err(format!("Unsupported graph version {}", doc.version));
        }
        if doc.nodes.len() > 4096 || doc.connections.len() > 16384 {
            return Err("Graph exceeds runtime size limits".into());
        }
        let mut nodes = HashMap::new();
        let mut ports = HashMap::new();
        let mut ids = HashSet::new();
        let mut events: HashMap<String, Vec<Uuid>> = HashMap::new();
        for node in doc.nodes {
            if !ids.insert(node.id) {
                return Err("Duplicate node ID".into());
            }
            let module = self
                .modules
                .get(&node.definition)
                .ok_or_else(|| format!("Unsupported node: {}", node.definition))?;
            let mut params = BTreeMap::new();
            for row in node.rows {
                if let Some(key) = row.key {
                    if params.insert(key, row.value).is_some() {
                        return Err("Duplicate parameter".into());
                    }
                }
            }
            let operation = module
                .compile(&params)
                .map_err(|e| format!("{}: {e}", node.definition))?;
            if let Some(event) = operation.event() {
                events.entry(event.into()).or_default().push(node.id);
            }
            let mut keys = HashSet::new();
            for port in node.ports {
                let expected = if port.direction == "Input" {
                    module.inputs()
                } else if port.direction == "Output" {
                    module.outputs()
                } else {
                    return Err("Invalid port direction".into());
                };
                if port.kind != "flow"
                    || !expected.contains(&port.key.as_str())
                    || !keys.insert(port.key.clone())
                {
                    return Err("Invalid module port".into());
                }
                if ports
                    .insert(port.id, (node.id, port.key, port.direction))
                    .is_some()
                {
                    return Err("Duplicate port ID".into());
                }
            }
            nodes.insert(node.id, operation);
        }
        let mut edges: HashMap<(Uuid, String), Vec<(Uuid, Uuid)>> = HashMap::new();
        let mut edge_ids = HashSet::new();
        for c in doc.connections {
            if !edge_ids.insert(c.id) {
                return Err("Duplicate connection ID".into());
            }
            let a = ports.get(&c.from).ok_or("Missing source terminal")?;
            let b = ports.get(&c.to).ok_or("Missing destination terminal")?;
            if a.2 != "Output" || b.2 != "Input" {
                return Err("Connection must run from output to input".into());
            }
            edges
                .entry((a.0, a.1.clone()))
                .or_default()
                .push((c.id, b.0));
        }
        Ok(Plan {
            nodes,
            events,
            edges,
        })
    }
}
#[derive(Deserialize, Serialize)]
struct Document {
    version: u32,
    nodes: Vec<Node>,
    connections: Vec<Connection>,
}
#[derive(Deserialize, Serialize)]
struct Node {
    id: Uuid,
    definition: String,
    rows: Vec<Row>,
    ports: Vec<Port>,
}
#[derive(Deserialize, Serialize)]
struct Row {
    key: Option<String>,
    value: Value,
}
#[derive(Deserialize, Serialize)]
struct Port {
    id: Uuid,
    key: String,
    direction: String,
    kind: String,
}
#[derive(Deserialize, Serialize)]
struct Connection {
    id: Uuid,
    from: Uuid,
    to: Uuid,
}
pub struct Plan {
    nodes: HashMap<Uuid, Box<dyn Operation>>,
    events: HashMap<String, Vec<Uuid>>,
    edges: HashMap<(Uuid, String), Vec<(Uuid, Uuid)>>,
}
#[derive(Default)]
pub struct Runtime {
    pub actors: HashMap<Uuid, Actor>,
    conditions: HashMap<ActorHandle, BTreeMap<Uuid, bool>>,
    highlights: HashMap<ActorHandle, Highlights>,
    highlight_dirty: HashSet<ActorHandle>,
    live_activity: HashMap<ActorHandle, (Uuid, EventObservation)>,
    pending: VecDeque<(ActorHandle, EventObservation, Option<Uuid>)>,
    failed: HashMap<ActorHandle, (Uuid, EventObservation)>,
    pub traces: VecDeque<Trace>,
    pub observations: VecDeque<EventObservation>,
}
impl Runtime {
    /// Presence in this collection means alive and spawned. Death removes presence;
    /// stable identity may later be spawned again with a different incarnation.
    pub fn spawn(
        &mut self,
        identity: Uuid,
        map: Uuid,
        render_id: u32,
        graph: Option<Plan>,
    ) -> Result<ActorHandle, String> {
        if self.pending.len() >= 4096 {
            return Err("Startup queue is full".into());
        }
        let handle = self.attach(identity, map, render_id, graph)?;
        self.send(handle, "startup", BTreeMap::new(), 0);
        Ok(handle)
    }
    /// Attach to an actor already created by the engine. Never creates map content
    /// or synthesizes Startup; the existing lifecycle supplies that event.
    pub fn attach(
        &mut self,
        identity: Uuid,
        map: Uuid,
        render_id: u32,
        graph: Option<Plan>,
    ) -> Result<ActorHandle, String> {
        if self.actors.contains_key(&identity) {
            return Err("Actor already spawned".into());
        }
        let handle = ActorHandle {
            identity,
            incarnation: Uuid::new_v4(),
        };
        self.actors.insert(
            identity,
            Actor {
                handle,
                map,
                render_id,
                visible: true,
                activity: Activity::Awake,
                graph,
            },
        );
        Ok(handle)
    }
    pub fn send_observation(&mut self, handle: ActorHandle, event: EventObservation) -> bool {
        if !self.is_current(handle)
            || self.pending.len() >= 4096
            || self.actors[&handle.identity].map != event.map
        {
            return false;
        }
        self.pending.push_back((handle, event, None));
        true
    }
    pub fn replace_graph(
        &mut self,
        handle: ActorHandle,
        plan: Plan,
        world: &mut dyn WorldServices,
    ) -> Result<(), String> {
        if !self.is_current(handle) {
            return Ok(());
        }
        let actor = &self.actors[&handle.identity];
        let changed_structure = actor.graph.as_ref().is_none_or(|old| {
            old.edges != plan.edges
                || old.events != plan.events
                || old.nodes.len() != plan.nodes.len()
                || old.nodes.keys().any(|id| !plan.nodes.contains_key(id))
        });
        let current_conditions: BTreeMap<_, _> = plan
            .nodes
            .iter()
            .filter_map(|(id, op)| op.condition(world.time(actor)).map(|v| (*id, v)))
            .collect();
        let changed_conditions = self
            .conditions
            .get(&handle)
            .is_some_and(|old| *old != current_conditions);
        self.conditions.insert(handle, current_conditions);
        let reevaluate =
            (changed_structure || changed_conditions) && plan.events.contains_key("routine");
        self.actors.get_mut(&handle.identity).unwrap().graph = Some(plan);
        let actor = &self.actors[&handle.identity];
        if reevaluate {
            world.cancel_activity(actor);
            self.live_activity.remove(&handle);
            if let Some(state) = self.highlights.get_mut(&handle) {
                state.active = ExecutionPath::default();
                state.recent = ExecutionPath::default();
                state.revision = Uuid::new_v4();
                self.highlight_dirty.insert(handle);
            }
            self.failed.remove(&handle);
            self.send(handle, "routine", BTreeMap::new(), 0);
        } else if let Some((id, event)) = self.failed.remove(&handle) {
            if actor.graph.as_ref().unwrap().nodes.contains_key(&id) {
                // Retry only the failed operation, never replay startup side effects.
                self.pending.push_back((handle, event, Some(id)));
            }
        } else if let Some((id, event)) = self.live_activity.get(&handle).cloned() {
            if let Some(operation) = actor
                .graph
                .as_ref()
                .and_then(|graph| graph.nodes.get(&id))
                .filter(|op| op.retains_activity("started") || op.retains_activity("running"))
            {
                if let Err(error) = operation.refresh(
                    &EventContext {
                        actor,
                        event: &event,
                        time: world.time(actor),
                    },
                    world,
                ) {
                    self.failed.insert(handle, (id, event.clone()));
                    if let Some(state) = self.highlights.get_mut(&handle) {
                        state.active = ExecutionPath::default();
                        state.revision = Uuid::new_v4();
                        self.highlight_dirty.insert(handle);
                    }
                    if self.traces.len() >= 512 {
                        self.traces.pop_front();
                    }
                    self.traces.push_back(Trace {
                        event,
                        node: id,
                        connection: None,
                        error: Some(error.clone()),
                        running: false,
                    });
                    return Err(error);
                }
            } else {
                world.cancel_activity(actor);
                self.live_activity.remove(&handle);
                if let Some(state) = self.highlights.get_mut(&handle) {
                    state.active = ExecutionPath::default();
                    state.recent = ExecutionPath::default();
                    state.revision = Uuid::new_v4();
                    self.highlight_dirty.insert(handle);
                }
            }
        }
        Ok(())
    }
    pub fn despawn(&mut self, handle: ActorHandle) -> bool {
        if !self.is_current(handle) {
            return false;
        }
        self.actors.remove(&handle.identity);
        self.live_activity.remove(&handle);
        if let Some(state) = self.highlights.get_mut(&handle) {
            state.active = ExecutionPath::default();
            state.recent = ExecutionPath::default();
            state.revision = Uuid::new_v4();
            self.highlight_dirty.insert(handle);
        }
        self.failed.remove(&handle);
        self.conditions.remove(&handle);
        self.pending.retain(|(actor, _, _)| *actor != handle);
        true
    }
    pub fn is_current(&self, handle: ActorHandle) -> bool {
        self.actors
            .get(&handle.identity)
            .is_some_and(|a| a.handle == handle)
    }
    pub fn send(
        &mut self,
        handle: ActorHandle,
        name: &str,
        fields: BTreeMap<String, EventField>,
        tick: i64,
    ) -> bool {
        if !self.is_current(handle) || self.pending.len() >= 4096 {
            return false;
        }
        let actor = &self.actors[&handle.identity];
        self.pending.push_back((
            handle,
            EventObservation {
                map: actor.map,
                owner: EventOwner::Entity(handle.identity),
                name: name.into(),
                fields,
                tick,
            },
            None,
        ));
        true
    }
    pub fn take_highlights(&mut self) -> Vec<Highlights> {
        let dirty: Vec<_> = self.highlight_dirty.drain().collect();
        let mut updates = Vec::new();
        for handle in dirty {
            if let Some(state) = self.highlights.get(&handle) {
                updates.push(state.clone());
            }
            if !self.is_current(handle) {
                self.highlights.remove(&handle);
            }
        }
        updates
    }
    /// Stop reporting a retained activity after lifecycle cancellation.
    pub fn suspend_activity(&mut self, handle: ActorHandle) {
        self.live_activity.remove(&handle);
        if let Some(state) = self.highlights.get_mut(&handle) {
            if !state.active.nodes.is_empty() || !state.recent.nodes.is_empty() {
                state.active = ExecutionPath::default();
                state.recent = ExecutionPath::default();
                state.revision = Uuid::new_v4();
                self.highlight_dirty.insert(handle);
            }
        }
    }
    /// Context-driven scheduling and asynchronous completion, independent of node types.
    pub fn tick(&mut self, handle: ActorHandle, tick: i64, world: &mut dyn WorldServices) {
        let Some(actor) = self
            .actors
            .get(&handle.identity)
            .filter(|a| a.handle == handle)
        else {
            return;
        };
        let Some(graph) = &actor.graph else {
            return;
        };
        let conditions: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .filter_map(|(id, op)| op.condition(world.time(actor)).map(|v| (*id, v)))
            .collect();
        let changed = self
            .conditions
            .insert(handle, conditions.clone())
            .is_some_and(|old| old != conditions);
        let routine_owns_activity = self
            .live_activity
            .get(&handle)
            .is_none_or(|(_, event)| event.name == "routine");
        if changed && routine_owns_activity && graph.events.contains_key("routine") {
            world.cancel_activity(actor);
            self.live_activity.remove(&handle);
            if let Some(state) = self.highlights.get_mut(&handle) {
                state.active = ExecutionPath::default();
                state.recent = ExecutionPath::default();
                state.revision = Uuid::new_v4();
                self.highlight_dirty.insert(handle);
            }
            self.failed.remove(&handle);
            self.send(handle, "routine", BTreeMap::new(), tick);
            self.update(world);
            return;
        }
        let Some((id, mut event)) = self.live_activity.get(&handle).cloned() else {
            return;
        };
        if self
            .failed
            .get(&handle)
            .is_some_and(|(failed, _)| *failed == id)
        {
            return;
        }
        let Some(op) = graph.nodes.get(&id) else {
            return;
        };
        let Some(result) = op.poll(
            &EventContext {
                actor,
                event: &event,
                time: world.time(actor),
            },
            world,
        ) else {
            return;
        };
        event.tick = tick;
        let completed_path = self
            .highlights
            .get(&handle)
            .map(|state| state.active.clone())
            .unwrap_or_default();
        self.live_activity.remove(&handle);
        if let Some(state) = self.highlights.get_mut(&handle) {
            state.active = ExecutionPath::default();
            state.recent = ExecutionPath::default();
            state.revision = Uuid::new_v4();
            self.highlight_dirty.insert(handle);
        }
        if let Some(state) = self.highlights.get_mut(&handle) {
            state.recent = completed_path;
            state.event = event.clone();
        }
        if result.is_err() {
            self.failed.insert(handle, (id, event.clone()));
        }
        if self.traces.len() >= 512 {
            self.traces.pop_front();
        }
        self.traces.push_back(Trace {
            event: event.clone(),
            node: id,
            connection: None,
            error: result.as_ref().err().cloned(),
            running: false,
        });
        let completion_error = result.as_ref().err().cloned();
        let port = result.ok().or_else(|| op.error_output());
        if let Some(edges) = port.and_then(|port| graph.edges.get(&(id, port.into()))) {
            for (connection, target) in edges {
                if self.pending.len() >= 4096 {
                    break;
                }
                if self.traces.len() >= 512 {
                    self.traces.pop_front();
                }
                self.traces.push_back(Trace {
                    event: event.clone(),
                    node: id,
                    connection: Some(*connection),
                    error: completion_error.clone(),
                    running: false,
                });
                if let Some(state) = self.highlights.get_mut(&handle) {
                    if !state.recent.connections.contains(connection) {
                        state.recent.connections.push(*connection);
                    }
                }
                self.pending
                    .push_back((handle, event.clone(), Some(*target)));
            }
        }
        self.update(world);
    }
    pub fn update(&mut self, world: &mut dyn WorldServices) {
        // A fixed budget bounds cycles and fan-out without recursively calling nodes.
        let mut budget = 4096;
        while budget > 0 {
            let Some((handle, event, start)) = self.pending.pop_front() else {
                break;
            };
            if !self.is_current(handle) {
                continue;
            }
            budget -= 1;
            super::event_observation::retain_observation(&mut self.observations, event.clone());
            let actor = &self.actors[&handle.identity];
            let Some(graph) = &actor.graph else {
                continue;
            };
            if event.name == "routine" || !self.conditions.contains_key(&handle) {
                self.conditions.insert(
                    handle,
                    graph
                        .nodes
                        .iter()
                        .filter_map(|(id, op)| op.condition(world.time(actor)).map(|v| (*id, v)))
                        .collect(),
                );
            }
            let mut work: VecDeque<(Uuid, Option<Uuid>)> = graph
                .events
                .get(&event.name)
                .into_iter()
                .flatten()
                .map(|id| (*id, None))
                .collect();
            if let Some(id) = start {
                work.clear();
                if graph.nodes.contains_key(&id) {
                    work.push_back((id, None));
                }
            }
            if work.is_empty() {
                continue;
            }
            let state = self.highlights.entry(handle).or_insert_with(|| Highlights {
                actor: handle,
                event: event.clone(),
                revision: Uuid::new_v4(),
                active: ExecutionPath::default(),
                recent: ExecutionPath::default(),
            });
            if start.is_none() || state.event != event {
                state.recent = ExecutionPath::default();
            }
            state.event = event.clone();
            state.revision = Uuid::new_v4();
            self.highlight_dirty.insert(handle);
            let mut started_activity = false;
            while let Some((id, incoming)) = work.pop_front() {
                let result = if budget <= 1 {
                    budget = 0;
                    Err("Graph execution budget exceeded (possible cycle)".into())
                } else {
                    budget -= 1;
                    graph.nodes[&id].execute(
                        &EventContext {
                            actor,
                            event: &event,
                            time: world.time(actor),
                        },
                        world,
                    )
                };
                let state = self.highlights.get_mut(&handle).unwrap();
                if !state.recent.nodes.contains(&id) {
                    state.recent.nodes.push(id);
                }
                if let Some(connection) = incoming {
                    if !state.recent.connections.contains(&connection) {
                        state.recent.connections.push(connection);
                    }
                }
                if self.traces.len() >= 512 {
                    self.traces.pop_front();
                }
                self.traces.push_back(Trace {
                    event: event.clone(),
                    node: id,
                    connection: incoming,
                    error: result.as_ref().err().cloned(),
                    running: result.as_ref().is_ok_and(|port| *port == "running"),
                });
                if result.is_err() {
                    self.failed.insert(handle, (id, event.clone()));
                } else if self
                    .failed
                    .get(&handle)
                    .is_some_and(|(failed, _)| *failed == id)
                {
                    self.failed.remove(&handle);
                }
                if budget == 0 {
                    break;
                }
                let port = result
                    .as_ref()
                    .ok()
                    .copied()
                    .or_else(|| graph.nodes[&id].error_output());
                if let Some(port) = port {
                    if graph.nodes[&id].retains_activity(port) {
                        self.live_activity.insert(handle, (id, event.clone()));
                        started_activity = true;
                    }
                    if let Some(edges) = graph.edges.get(&(id, port.into())) {
                        // Bound queued fan-out as well as executed work.
                        if work.len() + edges.len() > 4096 {
                            self.traces.back_mut().unwrap().error =
                                Some("Graph fan-out budget exceeded".into());
                            break;
                        }
                        work.extend(edges.iter().map(|(edge, node)| (*node, Some(*edge))));
                    }
                }
            }
            if started_activity {
                let state = self.highlights.get_mut(&handle).unwrap();
                state.active = state.recent.clone();
            }
        }
    }
}
