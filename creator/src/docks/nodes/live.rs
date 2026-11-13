use super::*;
use rusterix::server::event_observation::{EventObservation, EventOwner};

#[derive(Clone, Default)]
pub(super) struct LiveEvents {
    pub active: bool,
    pub time: TheTime,
    pub combat_status: HashMap<Uuid, (String, Option<u32>)>,
    pub active_nodes: std::collections::HashSet<Uuid>,
    pub active_connections: std::collections::HashSet<Uuid>,
    highlight_revisions: HashMap<Uuid, (Uuid, web_time::Instant)>,
    session: Option<Uuid>,
    pub latest: HashMap<Uuid, EventObservation>,
    pub traces: Vec<rusterix::server::nodes::Trace>,
}
pub(super) fn matches_owner(
    event: &EventObservation,
    pc: ProjectContext,
    project: &Project,
    current_region: Uuid,
) -> bool {
    match pc {
        ProjectContext::WorldCode => event.owner == EventOwner::World,
        ProjectContext::RegionCode(region) => {
            event.owner == EventOwner::Region
                && project
                    .get_region(&region)
                    .is_some_and(|r| r.map.id == event.map)
        }
        ProjectContext::RegionCharacterInstance(region, id) => {
            event.owner == EventOwner::Entity(id)
                && project
                    .get_region(&region)
                    .is_some_and(|r| r.map.id == event.map)
        }
        ProjectContext::RegionItemInstance(region, id) => {
            event.owner == EventOwner::Item(id)
                && project
                    .get_region(&region)
                    .is_some_and(|r| r.map.id == event.map)
        }
        ProjectContext::RegionArea(region, area) => {
            event.owner == EventOwner::Area(area)
                && project
                    .get_region(&region)
                    .is_some_and(|r| r.map.id == event.map)
        }
        ProjectContext::CharacterCode(template) | ProjectContext::Character(template) => {
            project.get_region(&current_region).is_some_and(|r| {
                r.map.id == event.map
                    && r.characters.values().any(|c| {
                        c.character_id == template && event.owner == EventOwner::Entity(c.id)
                    })
            })
        }
        ProjectContext::ItemCode(template) | ProjectContext::Item(template) => {
            project.get_region(&current_region).is_some_and(|r| {
                r.map.id == event.map
                    && r.items
                        .values()
                        .any(|i| i.item_id == template && event.owner == EventOwner::Item(i.id))
            })
        }
        _ => false,
    }
}
impl NodesDock {
    pub(super) fn refresh_live(&mut self, project: &Project, server_ctx: &ServerContext) -> bool {
        if let Some(runtime) = server_ctx.reference.existing() {
            self.update_live(&runtime.server, project, server_ctx)
        } else {
            let changed = self.live.active
                || self.live.time != project.time
                || !self.live.latest.is_empty()
                || !self.live.traces.is_empty()
                || !self.live.active_nodes.is_empty()
                || !self.live.active_connections.is_empty()
                || !self.live.combat_status.is_empty();
            self.live = LiveEvents {
                time: project.time,
                ..Default::default()
            };
            changed
        }
    }
    pub(super) fn update_live(
        &mut self,
        server: &rusterix::server::Server,
        project: &Project,
        server_ctx: &ServerContext,
    ) -> bool {
        let active = server.state != rusterix::ServerState::Off;
        let time = project
            .get_region(&server_ctx.curr_region)
            .and_then(|region| server.get_time(&region.map.id))
            .filter(|_| active)
            .unwrap_or(project.time);
        let mut changed = self.live.active != active || self.live.time != time;
        self.live.time = time;
        if self.live.session != Some(server.event_session) {
            self.live.latest.clear();
            self.live.traces.clear();
            self.live.highlight_revisions.clear();
            self.live.session = Some(server.event_session);
            changed = true;
        }
        self.live.active = active;
        if !active {
            changed |= !self.live.latest.is_empty()
                || !self.live.active_nodes.is_empty()
                || !self.live.active_connections.is_empty();
            self.live.active_nodes.clear();
            self.live.active_connections.clear();
            self.live.highlight_revisions.clear();
            self.live.latest.clear();
            self.live.traces.clear();
            return changed;
        }
        let now = web_time::Instant::now();
        let mut nodes = std::collections::HashSet::new();
        let mut connections = std::collections::HashSet::new();
        let mut combat_status = HashMap::new();
        let mut visible_actors = std::collections::HashSet::new();
        for state in server.node_highlights.iter().filter(|state| {
            Self::owner(server_ctx.pc).as_deref() == Some(state.graph.as_str())
                && matches_owner(&state.event, server_ctx.pc, project, server_ctx.curr_region)
        }) {
            visible_actors.insert(state.actor.incarnation);
            let seen = self
                .live
                .highlight_revisions
                .entry(state.actor.incarnation)
                .or_insert((state.revision, now));
            if seen.0 != state.revision {
                *seen = (state.revision, now);
            }
            if let Some(status) = &state.status {
                for id in &state.active.nodes {
                    combat_status.insert(*id, (status.clone(), state.target));
                }
            }
            nodes.extend(state.active.nodes.iter().copied());
            connections.extend(state.active.connections.iter().copied());
            if now.duration_since(seen.1) < std::time::Duration::from_millis(900) {
                nodes.extend(state.recent.nodes.iter().copied());
                connections.extend(state.recent.connections.iter().copied());
            }
        }
        self.live
            .highlight_revisions
            .retain(|actor, _| visible_actors.contains(actor));
        changed |= combat_status != self.live.combat_status;
        self.live.combat_status = combat_status;
        changed |= nodes != self.live.active_nodes || connections != self.live.active_connections;
        self.live.active_nodes = nodes;
        self.live.active_connections = connections;
        let traces: Vec<_> = server
            .node_traces
            .iter()
            .filter(|trace| {
                matches_owner(&trace.event, server_ctx.pc, project, server_ctx.curr_region)
            })
            .cloned()
            .collect();
        if self.live.traces != traces {
            self.live.traces = traces;
            changed = true;
        }
        for node in &self.doc.nodes {
            let Some(name) = self.definitions.selected_event(node) else {
                continue;
            };
            if self.live.latest.get(&node.id).is_some_and(|e| {
                e.name != name || !matches_owner(e, server_ctx.pc, project, server_ctx.curr_region)
            }) {
                self.live.latest.remove(&node.id);
                changed = true;
            }
            if let Some(event) = server.event_observations.iter().rev().find(|e| {
                e.name == name && matches_owner(e, server_ctx.pc, project, server_ctx.curr_region)
            }) {
                if self.live.latest.get(&node.id) != Some(event) {
                    self.live.latest.insert(node.id, event.clone());
                    changed = true;
                }
            }
        }
        self.live
            .latest
            .retain(|id, _| self.doc.nodes.iter().any(|n| n.id == *id));
        changed
    }
}
pub(super) struct LiveContext<'a> {
    pub definitions: &'a GraphDefinitions,
    pub live: &'a LiveEvents,
    pub equipment_names: &'a HashMap<String, String>,
}
/// Immutable, source-specific presentation inputs retained by a native widget.
/// This owns no runtime or Project lock; the host publishes a replacement when
/// authoring definitions, live observations or asset labels change.
#[cfg(feature = "native-ui")]
#[derive(Clone)]
pub(super) struct NativeContext {
    pub definitions: std::sync::Arc<GraphDefinitions>,
    pub live: std::sync::Arc<LiveEvents>,
    pub equipment_names: HashMap<String, String>,
}
#[cfg(feature = "native-ui")]
impl NativeContext {
    fn borrowed(&self) -> LiveContext<'_> {
        LiveContext {
            definitions: &self.definitions,
            live: &self.live,
            equipment_names: &self.equipment_names,
        }
    }
}
#[cfg(feature = "native-ui")]
impl GraphContext for NativeContext {
    fn node_title(&self, node: &GraphNode) -> Option<String> {
        self.borrowed().node_title(node)
    }
    fn row_label(&self, node: &GraphNode, row: &GraphRow) -> Option<String> {
        self.borrowed().row_label(node, row)
    }
    fn column_label(
        &self,
        node: &GraphNode,
        row: &GraphRow,
        column: &GraphListColumn,
    ) -> Option<String> {
        self.borrowed().column_label(node, row, column)
    }
    fn cell_label(
        &self,
        node: &GraphNode,
        row: &GraphRow,
        cell: (usize, usize),
        value: &GraphControlValue,
    ) -> Option<String> {
        self.borrowed().cell_label(node, row, cell, value)
    }
    fn node_active(&self, node: Uuid) -> bool {
        self.borrowed().node_active(node)
    }
    fn connection_active(&self, connection: Uuid) -> bool {
        self.borrowed().connection_active(connection)
    }
    fn observe(&self, node: &GraphNode) -> GraphObservation {
        self.borrowed().observe(node)
    }
}
impl GraphContext for LiveContext<'_> {
    fn column_label(
        &self,
        node: &GraphNode,
        row: &GraphRow,
        _: &GraphListColumn,
    ) -> Option<String> {
        (node.definition.as_deref() == Some("entity_equipment")
            && row.key.as_deref() == Some("start_equipped_items"))
        .then(|| fl!("entity_equipment_item"))
    }
    fn cell_label(
        &self,
        node: &GraphNode,
        row: &GraphRow,
        cell: (usize, usize),
        value: &GraphControlValue,
    ) -> Option<String> {
        if node.definition.as_deref() == Some("entity_equipment")
            && row.key.as_deref() == Some("start_equipped_items")
            && cell.1 == 0
            && let GraphControlValue::Text(reference) = value
        {
            return self.equipment_names.get(reference).cloned();
        }
        None
    }

    fn node_title(&self, node: &GraphNode) -> Option<String> {
        AuthoringContext(self.definitions).node_title(node)
    }
    fn row_label(&self, node: &GraphNode, row: &GraphRow) -> Option<String> {
        if let Some(field) = row.key.as_deref().and_then(|k| k.strip_prefix("offers:")) {
            if let Some(event) = self.live.latest.get(&node.id) {
                return Some(
                    event
                        .fields
                        .get(field)
                        .map(|v| v.display())
                        .unwrap_or_else(|| fl!("node_value_unavailable")),
                );
            }
        }
        AuthoringContext(self.definitions).row_label(node, row)
    }
    fn node_active(&self, node: Uuid) -> bool {
        self.live.active && self.live.active_nodes.contains(&node)
    }
    fn connection_active(&self, connection: Uuid) -> bool {
        self.live.active && self.live.active_connections.contains(&connection)
    }
    fn observe(&self, node: &GraphNode) -> GraphObservation {
        if node
            .definition
            .as_deref()
            .is_some_and(|id| id.starts_with("entity"))
        {
            let condition = if node.definition.as_deref() == Some("entity") {
                node.rows
                    .iter()
                    .find(|r| r.key.as_deref() == Some("summary"))
                    .and_then(|r| {
                        if let GraphControlValue::Label(text) = &r.value {
                            Some(text == &fl!("entity_configuration_ready"))
                        } else {
                            None
                        }
                    })
            } else {
                None
            };
            return GraphObservation {
                text: if condition == Some(false) {
                    node.rows
                        .iter()
                        .find(|r| r.key.as_deref() == Some("summary"))
                        .and_then(|r| {
                            if let GraphControlValue::Label(text) = &r.value {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default()
                } else {
                    fl!("entity_configuration")
                },
                condition: match condition {
                    Some(true) => GraphCondition::True,
                    Some(false) => GraphCondition::False,
                    None => GraphCondition::Unknown,
                },
                ..Default::default()
            };
        }
        if node.definition.as_deref() == Some("engage") && self.node_active(node.id) {
            if let Some((status, target)) = self.live.combat_status.get(&node.id) {
                let status = match status.as_str() {
                    "closing_in" => fl!("node_closing_in"),
                    "attacking" => fl!("node_attacking"),
                    "cooldown" => fl!("node_cooldown"),
                    _ => fl!("node_running"),
                };
                return GraphObservation {
                    execution: GraphExecution::Running,
                    text: fl!(
                        "node_combat_status",
                        status = status,
                        target = target
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "—".into())
                    ),
                    ..Default::default()
                };
            }
        }

        if node.definition.as_deref() == Some("time_range") {
            let value = |key: &str| {
                node.rows
                    .iter()
                    .find(|row| row.key.as_deref() == Some(key))
                    .and_then(|row| match &row.value {
                        GraphControlValue::Text(value) => Some(value.as_str()),
                        _ => None,
                    })
                    .unwrap_or("")
            };
            let result = rusterix::server::nodes::time_range_contains(
                value("start"),
                value("end"),
                self.live.time,
            );
            return GraphObservation {
                condition: match &result {
                    Ok(true) => GraphCondition::True,
                    Ok(false) => GraphCondition::False,
                    _ => GraphCondition::Unknown,
                },
                text: match result {
                    Ok(true) => fl!("node_inside"),
                    Ok(false) => fl!("node_outside"),
                    Err(error) => error,
                },
                ..Default::default()
            };
        }
        if let Some(trace) = self
            .live
            .traces
            .iter()
            .rev()
            .find(|trace| trace.node == node.id)
        {
            if matches!(
                node.definition.as_deref(),
                Some("quest_guard" | "item_guard" | "player_attribute_guard")
            ) {
                return GraphObservation {
                    condition: match trace.condition {
                        Some(true) => GraphCondition::True,
                        Some(false) => GraphCondition::False,
                        None => GraphCondition::Unknown,
                    },
                    text: trace
                        .error
                        .clone()
                        .unwrap_or_else(|| match trace.condition {
                            Some(true) => fl!("node_guard_pass"),
                            Some(false) => fl!("node_guard_fail"),
                            None => String::new(),
                        }),
                    ..Default::default()
                };
            }
            return GraphObservation {
                execution: if trace.error.is_some() {
                    GraphExecution::Failed
                } else if self.node_active(node.id) {
                    GraphExecution::Running
                } else {
                    GraphExecution::Idle
                },
                text: trace.error.clone().unwrap_or_else(|| {
                    if trace.running {
                        fl!("node_running")
                    } else {
                        fl!("node_executed", tick = trace.event.tick.to_string())
                    }
                }),
                ..Default::default()
            };
        }
        if self.live.active && self.definitions.selected_event(node).is_some() {
            let text = if let Some(event) = self.live.latest.get(&node.id) {
                match event.owner {
                    EventOwner::Entity(id) | EventOwner::Item(id) => fl!(
                        "node_received_instance",
                        instance = id.to_string()[..8].to_string(),
                        tick = event.tick.to_string()
                    ),
                    _ => fl!("node_received", tick = event.tick.to_string()),
                }
            } else {
                fl!("node_live_waiting")
            };
            return GraphObservation {
                text,
                ..Default::default()
            };
        }
        if self.live.active {
            return GraphObservation {
                text: fl!("node_not_executed"),
                ..Default::default()
            };
        }
        AuthoringContext(self.definitions).observe(node)
    }
}

#[cfg(all(test, feature = "native-ui"))]
mod native_tests {
    use super::*;
    #[test]
    fn native_context_retains_source_labels_and_live_observations_without_a_runtime_owner() {
        let definitions = std::sync::Arc::new(catalog::definitions());
        let event = definitions.events().next().unwrap();
        let event_id = event.id.clone();
        let event_label = event.label.clone();
        let mut entry = definitions.instantiate("event", [20., 30.]).unwrap();
        entry
            .rows
            .iter_mut()
            .find(|row| row.key.as_deref() == Some("event"))
            .unwrap()
            .value = GraphControlValue::Custom {
            kind: "event".into(),
            data: event_id.into(),
        };
        let mut engage = GraphNode::new("Engage", [200., 30.], [40, 50, 60, 255]);
        engage.definition = Some("engage".into());
        let connection = Uuid::new_v4();
        let mut live = LiveEvents {
            active: true,
            ..Default::default()
        };
        live.active_nodes.insert(engage.id);
        live.active_connections.insert(connection);
        live.combat_status
            .insert(engage.id, ("attacking".into(), Some(42)));
        let mut names = HashMap::from([("item:source".into(), "Source blade".into())]);
        let context = NativeContext {
            definitions: definitions.clone(),
            live: std::sync::Arc::new(live.clone()),
            equipment_names: names.clone(),
        };
        live.active = false;
        live.active_nodes.clear();
        names.insert("item:source".into(), "Other blade".into());
        assert!(context.node_title(&entry).unwrap().contains(&event_label));
        assert!(context.node_active(engage.id));
        assert!(context.connection_active(connection));
        let observation = context.observe(&engage);
        assert_eq!(observation.execution, GraphExecution::Running);
        assert!(observation.text.contains("42"));
        let mut equipment = GraphNode::new("Equipment", [0.; 2], [1, 2, 3, 255]);
        equipment.definition = Some("entity_equipment".into());
        let mut row = GraphRow::new("Items", GraphControlValue::Text("item:source".into()));
        row.key = Some("start_equipped_items".into());
        assert_eq!(
            context
                .cell_label(&equipment, &row, (0, 0), &row.value)
                .as_deref(),
            Some("Source blade")
        );
        assert!(
            context
                .cell_label(&equipment, &row, (0, 1), &row.value)
                .is_none()
        );
        assert!(!live.active);
    }
}
