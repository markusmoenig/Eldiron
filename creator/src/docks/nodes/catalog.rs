//! Behavior definitions belong to Creator, never to the generic graph renderer.
use theframework::thegraph::*;

pub fn definitions() -> GraphDefinitions {
    definitions_with_rules(shared::rulesets::latest_official_ruleset())
}
pub fn definitions_for_project(project: &shared::project::Project) -> GraphDefinitions {
    let rules = shared::rulesets::resolve_project_rules(&project.config, &project.rules)
        .unwrap_or_default();
    definitions_with_rules(&rules)
}
fn definitions_with_rules(rules: &str) -> GraphDefinitions {
    let mut defs = GraphDefinitions::default();
    for (id, label, fields) in [
        ("startup", fl!("node_startup"), vec![]),
        (
            "time",
            fl!("node_time"),
            vec![("hour", GraphValueType::Number)],
        ),
        (
            "entered",
            fl!("node_entered"),
            vec![("sector", GraphValueType::Text)],
        ),
        (
            "damaged",
            fl!("node_damaged"),
            vec![
                ("attacker", GraphValueType::Entity),
                ("amount", GraphValueType::Number),
                ("kind", GraphValueType::Text),
            ],
        ),
        (
            "intent",
            fl!("node_intent"),
            vec![
                ("intent", GraphValueType::Text),
                ("subject", GraphValueType::Entity),
                ("distance", GraphValueType::Number),
                ("count", GraphValueType::Number),
            ],
        ),
    ] {
        defs.register_event(GraphEventDefinition {
            id: id.into(),
            label: label.into(),
            fields: fields
                .into_iter()
                .map(|(id, value_type)| GraphEventField {
                    id: id.into(),
                    label: id.into(),
                    value_type,
                })
                .collect(),
        })
        .unwrap();
    }
    let mut event = GraphNode::new(&fl!("node_event"), [0., 0.], [16, 112, 98, 255]);
    row(
        &mut event,
        "event",
        &fl!("node_event"),
        GraphControlValue::Custom {
            kind: "event".into(),
            data: "entered".into(),
        },
    );
    port(&mut event, "out", "", PortDirection::Output, 0.5);
    let mut def = GraphNodeDefinition::from_template("event", &fl!("node_group_events"), &event);
    def.event_parameter = Some("event".into());
    defs.register_node(def).unwrap();
    let mut filter = GraphNode::new(&fl!("node_filter"), [0., 0.], [164, 98, 35, 255]);
    row(
        &mut filter,
        "field",
        &fl!("node_field"),
        GraphControlValue::Text("event.destination".into()),
    );
    row(
        &mut filter,
        "operator",
        &fl!("node_compare"),
        GraphControlValue::Choice {
            options: vec![
                fl!("node_equals"),
                fl!("node_not_equal"),
                fl!("node_contains"),
                fl!("node_greater"),
                fl!("node_less"),
            ],
            selected: 0,
        },
    );
    row(
        &mut filter,
        "expected",
        &fl!("node_against"),
        GraphControlValue::Text(fl!("node_sample_garden")),
    );
    port(&mut filter, "in", "", PortDirection::Input, 0.5);
    port(
        &mut filter,
        "match",
        &fl!("node_match"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut filter,
        "no_match",
        &fl!("node_no_match"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "filter",
        &fl!("node_group_logic"),
        &filter,
    ))
    .unwrap();
    let mut say = GraphNode::new(&fl!("node_say"), [0., 0.], [35, 87, 134, 255]);
    say.width = 300.;
    row(
        &mut say,
        "text",
        &fl!("node_text"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut say, "in", "", PortDirection::Input, 0.5);
    port(
        &mut say,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "say",
        &fl!("node_group_actions"),
        &say,
    ))
    .unwrap();
    let mut camera = GraphNode::new(&fl!("node_player_camera"), [0., 0.], [35, 87, 134, 255]);
    row(
        &mut camera,
        "camera",
        &fl!("node_camera"),
        GraphControlValue::Choice {
            options: vec![
                fl!("node_camera_2d"),
                fl!("node_camera_2d_grid"),
                fl!("node_camera_iso"),
                fl!("node_camera_firstp"),
                fl!("node_camera_firstp_grid"),
            ],
            selected: 1,
        },
    );
    port(&mut camera, "in", "", PortDirection::Input, 0.5);
    port(
        &mut camera,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "player_camera",
        &fl!("node_group_actions"),
        &camera,
    ))
    .unwrap();
    let mut routine = GraphNode::new(&fl!("node_routine"), [0., 0.], [16, 112, 98, 255]);
    port(&mut routine, "out", "", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "routine",
        &fl!("node_group_events"),
        &routine,
    ))
    .unwrap();
    let mut range = GraphNode::new(&fl!("node_time_range"), [0., 0.], [164, 98, 35, 255]);
    for (key, label, value) in [
        ("start", fl!("node_time_start"), "14:00"),
        ("end", fl!("node_time_end"), "16:00"),
    ] {
        row(
            &mut range,
            key,
            &label,
            GraphControlValue::Text(value.into()),
        );
    }
    port(&mut range, "in", "", PortDirection::Input, 0.5);
    port(
        &mut range,
        "inside",
        &fl!("node_inside"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut range,
        "outside",
        &fl!("node_outside"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "time_range",
        &fl!("node_group_logic"),
        &range,
    ))
    .unwrap();
    let mut goto = GraphNode::new(&fl!("node_go_to"), [0., 0.], [35, 87, 134, 255]);
    row(
        &mut goto,
        "destination",
        &fl!("node_destination"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut goto,
        "speed",
        &fl!("node_walk_speed"),
        GraphControlValue::Number {
            value: 1.,
            min: 0.1,
            max: 10.,
            step: 0.1,
        },
    );
    port(&mut goto, "in", "", PortDirection::Input, 0.5);
    port(
        &mut goto,
        "done",
        &fl!("node_done"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut goto,
        "error",
        &fl!("node_error"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "go_to",
        &fl!("node_group_actions"),
        &goto,
    ))
    .unwrap();
    let mut action = GraphNode::new(&fl!("node_use_action"), [0., 0.], [35, 87, 134, 255]);
    let options: Vec<String> = rules
        .parse::<toml::Table>()
        .ok()
        .and_then(|rules| {
            rules
                .get("actions")
                .and_then(toml::Value::as_table)
                .map(|actions| {
                    actions
                        .iter()
                        .filter(|(_, action)| {
                            action.get("kind").and_then(toml::Value::as_str) == Some("attack")
                                && matches!(
                                    action.get("target").and_then(toml::Value::as_str),
                                    Some(
                                        "hostile_entity"
                                            | "hostile_or_neutral_entity"
                                            | "friendly_entity"
                                            | "friendly_or_self"
                                            | "any_entity"
                                    )
                                )
                        })
                        .map(|(id, _)| id.clone())
                        .collect()
                })
        })
        .unwrap_or_default();
    let selected = options
        .iter()
        .position(|id| id == "basic_attack")
        .unwrap_or(0);
    row(
        &mut action,
        "action",
        &fl!("node_ruleset_action"),
        GraphControlValue::Choice { options, selected },
    );
    port(&mut action, "in", "", PortDirection::Input, 0.5);
    port(
        &mut action,
        "done",
        &fl!("node_done"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut action,
        "failed",
        &fl!("node_action_failed"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "use_action",
        &fl!("node_group_actions"),
        &action,
    ))
    .unwrap();
    for (id, title, default_profile, outputs) in [
        (
            "lookout",
            fl!("node_lookout"),
            "hostile",
            vec![
                ("found", fl!("node_target_found")),
                ("watching", fl!("node_watching")),
            ],
        ),
        (
            "engage",
            fl!("node_engage"),
            "default",
            vec![
                ("defeated", fl!("node_target_defeated")),
                ("lost", fl!("node_target_lost")),
                ("cannot_engage", fl!("node_cannot_engage")),
            ],
        ),
    ] {
        let mut node = GraphNode::new(&title, [0., 0.], [35, 87, 134, 255]);
        let options =
            shared::rulesets::behavior::policy_ids_from_source(rules, id).unwrap_or_default();
        let selected = options
            .iter()
            .position(|id| id == default_profile)
            .unwrap_or(0);
        row(
            &mut node,
            "profile",
            &fl!("node_ruleset_profile"),
            GraphControlValue::Choice { options, selected },
        );
        if id == "lookout" {
            add_lookout_distances(&mut node, rules, default_profile);
        }
        port(&mut node, "in", "", PortDirection::Input, 0.5);
        let count = outputs.len();
        for (i, (key, label)) in outputs.into_iter().enumerate() {
            port(
                &mut node,
                key,
                &label,
                PortDirection::Output,
                (i + 1) as f32 / (count + 1) as f32,
            );
        }
        defs.register_node(GraphNodeDefinition::from_template(
            id,
            &fl!("node_group_actions"),
            &node,
        ))
        .unwrap();
    }
    let mut walk = GraphNode::new(&fl!("node_random_walk"), [0., 0.], [35, 87, 134, 255]);
    walk.width = 260.;
    row(
        &mut walk,
        "area",
        &fl!("node_walk_area"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut walk,
        "distance",
        &fl!("node_walk_distance"),
        GraphControlValue::Number {
            value: 1.,
            min: 0.1,
            max: 10.,
            step: 0.5,
        },
    );
    row(
        &mut walk,
        "speed",
        &fl!("node_walk_speed"),
        GraphControlValue::Number {
            value: 1.,
            min: 0.1,
            max: 10.,
            step: 0.1,
        },
    );
    row(
        &mut walk,
        "pause",
        &fl!("node_walk_pause"),
        GraphControlValue::Number {
            value: 2.,
            min: 0.,
            max: 60.,
            step: 1.,
        },
    );
    port(&mut walk, "in", "", PortDirection::Input, 0.5);
    port(
        &mut walk,
        "started",
        &fl!("node_started"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut walk,
        "outside_area",
        &fl!("node_outside_area"),
        PortDirection::Output,
        0.7,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "random_walk",
        &fl!("node_group_actions"),
        &walk,
    ))
    .unwrap();
    let mut resume = GraphNode::new(&fl!("node_resume_routine"), [0., 0.], [35, 87, 134, 255]);
    port(&mut resume, "in", "", PortDirection::Input, 0.5);
    port(
        &mut resume,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "resume_routine",
        &fl!("node_group_actions"),
        &resume,
    ))
    .unwrap();
    defs
}
fn add_lookout_distances(node: &mut GraphNode, source: &str, profile: &str) {
    let Ok((reaction, escape)) =
        shared::rulesets::behavior::lookout_distances_from_source(source, profile)
    else {
        return;
    };
    for (key, label, value) in [
        ("reaction_distance", fl!("node_reaction_distance"), reaction),
        ("escape_distance", fl!("node_escape_distance"), escape),
    ] {
        if !node.rows.iter().any(|row| row.key.as_deref() == Some(key)) {
            row(
                node,
                key,
                &label,
                GraphControlValue::Number {
                    value: value.round().clamp(1., 20.),
                    min: 1.,
                    max: 20.,
                    step: 1.,
                },
            );
        }
    }
}
/// Upgrade old Lookout nodes once, preserving all authored distances thereafter.
pub fn hydrate_lookout_distances(doc: &mut GraphDocument, project: &shared::project::Project) {
    let Ok(source) = shared::rulesets::resolve_project_rules(&project.config, &project.rules)
    else {
        return;
    };
    for node in &mut doc.nodes {
        if node.definition.as_deref() != Some("lookout") {
            continue;
        }
        let profile = node
            .rows
            .iter()
            .find(|row| row.key.as_deref() == Some("profile"))
            .and_then(|row| match &row.value {
                GraphControlValue::Choice { options, selected } => options.get(*selected).cloned(),
                _ => None,
            });
        if let Some(profile) = profile {
            add_lookout_distances(node, &source, &profile);
        }
    }
}
fn row(node: &mut GraphNode, key: &str, label: &str, value: GraphControlValue) {
    let mut r = GraphRow::new(label, value);
    r.key = Some(key.into());
    node.rows.push(r);
}
fn port(node: &mut GraphNode, key: &str, label: &str, direction: PortDirection, position: f32) {
    let side = if direction == PortDirection::Input {
        PortSide::Left
    } else {
        PortSide::Right
    };
    let mut p = GraphPort::new(label, direction, side, position);
    p.key = Some(key.into());
    node.ports.push(p);
}
/// Read-only schema rows are derived from the event definition. They are not inputs.
/// Preserve their IDs when the schema stays the same so redraws never alter documents.
pub fn sync_fields(doc: &mut GraphDocument, defs: &GraphDefinitions) {
    for node in &mut doc.nodes {
        // Labels are presentation metadata; authored parameter values keep their meaning.
        if let Some(def) = node.definition.as_deref().and_then(|id| defs.node(id)) {
            node.title = def.title.clone();
            node.color = def.color;
            for row in &mut node.rows {
                if let Some(param) = def
                    .parameters
                    .iter()
                    .find(|p| Some(&p.id) == row.key.as_ref())
                {
                    row.label = param.label.clone();
                    if let (
                        GraphControlValue::Number {
                            value,
                            min,
                            max,
                            step,
                        },
                        GraphControlValue::Number {
                            min: lo,
                            max: hi,
                            step: increment,
                            ..
                        },
                    ) = (&mut row.value, &param.default)
                    {
                        *min = *lo;
                        *max = *hi;
                        *step = *increment;
                        *value = value.clamp(*lo, *max);
                        if node.definition.as_deref() == Some("lookout") {
                            *value = value.round();
                        }
                    }
                    if let (
                        GraphControlValue::Choice { options, selected },
                        GraphControlValue::Choice {
                            options: translated,
                            ..
                        },
                    ) = (&mut row.value, &param.default)
                    {
                        let profile = (matches!(row.key.as_deref(), Some("profile" | "action")))
                            .then(|| options.get(*selected).cloned())
                            .flatten();
                        *options = translated.clone();
                        if let Some(profile) = profile {
                            *selected = options
                                .iter()
                                .position(|id| id == &profile)
                                .unwrap_or_else(|| {
                                    options.push(profile);
                                    options.len() - 1
                                });
                        }
                    }
                }
            }
            for port in &mut node.ports {
                if let Some(def_port) = def.ports.iter().find(|p| Some(&p.id) == port.key.as_ref())
                {
                    port.label = def_port.label.clone();
                }
            }
        }
        if node.definition.as_deref() != Some("event") {
            continue;
        }
        let fields = defs
            .selected_event(node)
            .and_then(|id| defs.event(id))
            .map(|e| e.fields.clone())
            .unwrap_or_default();
        let desired: Vec<_> = fields
            .iter()
            .map(|f| {
                (
                    format!("offers:{}", f.id),
                    format!("event.{}", f.id),
                    fl!("node_awaiting", value_type = value_type_label(f.value_type)),
                )
            })
            .collect();
        node.rows.retain(|r| {
            !r.key.as_deref().is_some_and(|k| k.starts_with("offers:"))
                || desired.iter().any(|(k, _, _)| Some(k) == r.key.as_ref())
        });
        for (key, label, value) in desired {
            if let Some(r) = node.rows.iter_mut().find(|r| r.key.as_ref() == Some(&key)) {
                r.label = label;
                r.value = GraphControlValue::Label(value);
            } else {
                row(node, &key, &label, GraphControlValue::Label(value));
            }
        }
    }
}

fn value_type_label(value: GraphValueType) -> String {
    match value {
        GraphValueType::Text => fl!("node_type_text"),
        GraphValueType::Number => fl!("node_type_number"),
        GraphValueType::Boolean => fl!("node_type_boolean"),
        GraphValueType::Entity => fl!("node_type_entity"),
    }
}
