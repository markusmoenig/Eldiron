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
        ("respawn", fl!("node_respawn"), vec![]),
        // Collision events. An item answers `bumped_by_entity`, the mover
        // answers the two mirror events.
        (
            "bumped_by_entity",
            fl!("node_bumped_by_entity"),
            vec![("entity", GraphValueType::Entity)],
        ),
        (
            "bumped_into_entity",
            fl!("node_bumped_into_entity"),
            vec![("entity", GraphValueType::Entity)],
        ),
        (
            "bumped_into_item",
            fl!("node_bumped_into_item"),
            vec![("item", GraphValueType::Text)],
        ),
        ("active", fl!("node_active"), vec![]),
        (
            "time",
            fl!("node_time"),
            vec![("hour", GraphValueType::Number)],
        ),
        (
            "entered",
            fl!("node_entered"),
            vec![("area", GraphValueType::Text)],
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
        ("death", fl!("node_death"), vec![]),
        (
            "kill",
            fl!("node_kill"),
            vec![
                ("killed", GraphValueType::Entity),
                ("name", GraphValueType::Text),
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
    let mut enter_area = GraphNode::new(&fl!("node_on_enter_area"), [0., 0.], [16, 112, 98, 255]);
    enter_area.width = 260.;
    row(
        &mut enter_area,
        "area",
        &fl!("node_area"),
        GraphControlValue::Text(String::new()),
    );
    port(
        &mut enter_area,
        "match",
        &fl!("node_match"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut enter_area,
        "no_match",
        &fl!("node_no_match"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "on_enter_area",
        &fl!("node_group_events"),
        &enter_area,
    ))
    .unwrap();
    let mut add_item = GraphNode::new(&fl!("node_add_item"), [0., 0.], [35, 87, 134, 255]);
    add_item.width = 260.;
    row(
        &mut add_item,
        "item",
        &fl!("node_item"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut add_item, "in", "", PortDirection::Input, 0.5);
    port(&mut add_item, "out", "Done", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "add_item",
        &fl!("node_group_actions"),
        &add_item,
    ))
    .unwrap();

    let mut drop_items = GraphNode::new(&fl!("node_drop_items"), [0., 0.], [35, 87, 134, 255]);
    drop_items.width = 260.;
    row(
        &mut drop_items,
        "filter",
        &fl!("node_filter"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut drop_items, "in", "", PortDirection::Input, 0.5);
    port(&mut drop_items, "out", "Done", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "drop_items",
        &fl!("node_group_actions"),
        &drop_items,
    ))
    .unwrap();

    let mut on_area = GraphNode::new(&fl!("node_on_area"), [0., 0.], [16, 112, 98, 255]);
    on_area.width = 240.;
    for (key, label, position) in [
        ("player_entered", fl!("node_port_player_entered"), 0.2),
        ("npc_entered", fl!("node_port_npc_entered"), 0.4),
        ("player_left", fl!("node_port_player_left"), 0.6),
        ("npc_left", fl!("node_port_npc_left"), 0.8),
    ] {
        port(&mut on_area, key, &label, PortDirection::Output, position);
    }
    let mut emit_light = GraphNode::new(&fl!("node_set_emit_light"), [0., 0.], [35, 87, 134, 255]);
    emit_light.width = 260.;
    row(
        &mut emit_light,
        "emit",
        &fl!("node_emit"),
        GraphControlValue::Choice {
            options: vec![
                "Follow Active".to_string(),
                "On".to_string(),
                "Off".to_string(),
            ],
            selected: 0,
        },
    );
    port(&mut emit_light, "in", "", PortDirection::Input, 0.5);
    port(&mut emit_light, "out", "Done", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "set_emit_light",
        &fl!("node_group_actions"),
        &emit_light,
    ))
    .unwrap();

    let mut on_event = GraphNode::new(&fl!("node_on_event_node"), [0., 0.], [16, 112, 98, 255]);
    on_event.width = 240.;
    row(
        &mut on_event,
        "event",
        &fl!("node_event_name"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut on_event, "out", "", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "on_event",
        &fl!("node_group_events"),
        &on_event,
    ))
    .unwrap();

    let mut notify_in = GraphNode::new(&fl!("node_notify_in"), [0., 0.], [35, 87, 134, 255]);
    notify_in.width = 260.;
    row(
        &mut notify_in,
        "minutes",
        &fl!("node_minutes"),
        GraphControlValue::Number {
            value: 1.,
            min: 0.,
            max: 600.,
            step: 0.5,
        },
    );
    row(
        &mut notify_in,
        "event",
        &fl!("node_event_name"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut notify_in, "in", "", PortDirection::Input, 0.5);
    port(&mut notify_in, "out", "Done", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "notify_in",
        &fl!("node_group_actions"),
        &notify_in,
    ))
    .unwrap();

    let mut radius = GraphNode::new(
        &fl!("node_entities_in_radius"),
        [0., 0.],
        [164, 98, 35, 255],
    );
    radius.width = 260.;
    port(&mut radius, "in", "", PortDirection::Input, 0.5);
    port(
        &mut radius,
        "empty",
        &fl!("node_port_empty"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut radius,
        "occupied",
        &fl!("node_port_occupied"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "entities_in_radius",
        &fl!("node_group_logic"),
        &radius,
    ))
    .unwrap();

    let mut dialog = GraphNode::new(&fl!("node_dialog"), [0., 0.], [35, 87, 134, 255]);
    dialog.width = 260.;
    row(
        &mut dialog,
        "node",
        &fl!("node_dialogue_node"),
        GraphControlValue::Text("greeting".into()),
    );
    port(&mut dialog, "in", "", PortDirection::Input, 0.5);
    port(
        &mut dialog,
        "out",
        &fl!("node_dialog_opened"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "dialog",
        &fl!("node_group_actions"),
        &dialog,
    ))
    .unwrap();

    let mut dialogue = GraphNode::new(&fl!("node_dialogue"), [0., 0.], [35, 110, 134, 255]);
    dialogue.width = 300.;
    row(
        &mut dialogue,
        "text",
        &fl!("node_dialogue_text"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut dialogue,
        "choices",
        &fl!("node_dialogue_choices"),
        GraphControlValue::List {
            columns: vec![
                GraphListColumn {
                    id: "label".into(),
                    label: fl!("node_dialogue_choice"),
                    control: GraphControlValue::Text(String::new()),
                },
                GraphListColumn {
                    id: "condition".into(),
                    label: fl!("node_dialogue_condition"),
                    control: GraphControlValue::Text(String::new()),
                },
            ],
            rows: vec![],
        },
    );
    port(&mut dialogue, "in", "", PortDirection::Input, 0.5);
    port(
        &mut dialogue,
        "done",
        &fl!("node_port_done"),
        PortDirection::Output,
        0.94,
    );
    for (index, position) in [0.12, 0.27, 0.42, 0.57, 0.72, 0.87].into_iter().enumerate() {
        port(
            &mut dialogue,
            &format!("choice:{index}"),
            &format!("{} {}", fl!("node_dialogue_choice"), index + 1),
            PortDirection::Output,
            position,
        );
    }
    defs.register_node(GraphNodeDefinition::from_template(
        "dialogue",
        &fl!("node_group_actions"),
        &dialogue,
    ))
    .unwrap();

    let mut prompt = dialogue.clone();
    prompt.title = fl!("node_prompt");
    prompt.width = 360.;
    if let Some(row) = prompt
        .rows
        .iter_mut()
        .find(|row| row.key.as_deref() == Some("choices"))
    {
        if let GraphControlValue::List { columns, .. } = &mut row.value {
            columns.retain(|column| column.id == "label");
        }
    }
    prompt.ports.retain(|port| {
        !port
            .key
            .as_deref()
            .is_some_and(|key| key.starts_with("choice:"))
    });
    defs.register_node(GraphNodeDefinition::from_template(
        "prompt",
        &fl!("node_group_actions"),
        &prompt,
    ))
    .unwrap();

    for (id, title, field, field_label, default) in [
        (
            "quest_guard",
            fl!("node_quest_guard"),
            "quest",
            fl!("node_quest_id"),
            String::new(),
        ),
        (
            "item_guard",
            fl!("node_item_guard"),
            "item",
            fl!("node_item"),
            String::new(),
        ),
        (
            "player_attribute_guard",
            fl!("node_player_attribute_guard"),
            "attribute",
            fl!("node_attribute"),
            String::new(),
        ),
    ] {
        let mut guard = GraphNode::new(&title, [0., 0.], [164, 98, 35, 255]);
        guard.width = 260.;
        row(
            &mut guard,
            field,
            &field_label,
            GraphControlValue::Text(default),
        );
        let (key, label, options) = match id {
            "quest_guard" => (
                "state",
                fl!("node_quest_state_label"),
                vec![
                    fl!("node_quest_not_started"),
                    fl!("node_quest_active"),
                    fl!("node_quest_completed"),
                    fl!("node_quest_not_completed"),
                ],
            ),
            "item_guard" => (
                "presence",
                fl!("node_guard_presence"),
                vec![fl!("node_guard_has"), fl!("node_guard_missing")],
            ),
            _ => (
                "expected",
                fl!("node_guard_expected"),
                vec![fl!("node_guard_true"), fl!("node_guard_false")],
            ),
        };
        row(
            &mut guard,
            key,
            &label,
            GraphControlValue::Choice {
                options,
                selected: 0,
            },
        );
        port(
            &mut guard,
            "condition",
            &fl!("node_guard_condition"),
            PortDirection::Output,
            0.5,
        );
        let mut definition =
            GraphNodeDefinition::from_template(id, &fl!("node_group_logic"), &guard);
        definition.starts_branch = false;
        defs.register_node(definition).unwrap();
    }

    // A whole conversation in one node. The steps are the lines and the choices
    // are the answers; a choice either moves inside the tree, ends it, or leaves
    // through one of the consequence ports, where ordinary nodes do the work.
    let mut talk = GraphNode::new(&fl!("node_talk"), [0., 0.], [35, 110, 134, 255]);
    talk.width = 520.;
    row(
        &mut talk,
        "conversation",
        &fl!("node_talk_conversation"),
        GraphControlValue::Custom {
            kind: "conversation".into(),
            data: serde_json::to_value(rusterix::server::nodes::Conversation::starter())
                .unwrap_or(serde_json::Value::Null),
        },
    );
    port(&mut talk, "in", "", PortDirection::Input, 0.5);
    port(
        &mut talk,
        "done",
        &fl!("node_port_done"),
        PortDirection::Output,
        0.93,
    );
    // Consequence ports are spread over the whole side, so six of them stay
    // readable on a node that only has one row.
    for index in 0..rusterix::server::nodes::CONSEQUENCE_SLOTS {
        port(
            &mut talk,
            &format!("out:{index}"),
            &format!("{} {}", fl!("node_talk_out"), index + 1),
            PortDirection::Output,
            0.10 + index as f32 * 0.14,
        );
    }
    defs.register_node(GraphNodeDefinition::from_template(
        "talk",
        &fl!("node_group_actions"),
        &talk,
    ))
    .unwrap();

    let mut quest_state = GraphNode::new(&fl!("node_quest_state"), [0., 0.], [164, 98, 35, 255]);
    row(
        &mut quest_state,
        "quest",
        &fl!("node_quest_id"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut quest_state, "in", "", PortDirection::Input, 0.5);
    for (key, label, position) in [
        ("not_started", fl!("node_quest_not_started"), 0.2),
        ("active", fl!("node_quest_active"), 0.5),
        ("completed", fl!("node_quest_completed"), 0.8),
    ] {
        port(
            &mut quest_state,
            key,
            &label,
            PortDirection::Output,
            position,
        );
    }
    defs.register_node(GraphNodeDefinition::from_template(
        "quest_state",
        &fl!("node_group_logic"),
        &quest_state,
    ))
    .unwrap();

    let mut set_quest = GraphNode::new(&fl!("node_set_quest"), [0., 0.], [35, 87, 134, 255]);
    row(
        &mut set_quest,
        "quest",
        &fl!("node_quest_id"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut set_quest,
        "state",
        &fl!("node_quest_state_label"),
        GraphControlValue::Choice {
            options: vec![
                fl!("node_quest_active"),
                fl!("node_quest_completed"),
                fl!("node_quest_not_started"),
            ],
            selected: 0,
        },
    );
    port(&mut set_quest, "in", "", PortDirection::Input, 0.5);
    for (key, label, position) in [
        ("done", fl!("node_done"), 0.25),
        ("unchanged", fl!("node_quest_unchanged"), 0.5),
        ("failed", fl!("node_action_failed"), 0.75),
    ] {
        port(&mut set_quest, key, &label, PortDirection::Output, position);
    }
    defs.register_node(GraphNodeDefinition::from_template(
        "set_quest",
        &fl!("node_group_actions"),
        &set_quest,
    ))
    .unwrap();

    let mut has_item = GraphNode::new(&fl!("node_inventory_has"), [0., 0.], [164, 98, 35, 255]);
    has_item.width = 260.;
    row(
        &mut has_item,
        "item",
        &fl!("node_item_name"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut has_item, "in", "", PortDirection::Input, 0.5);
    port(
        &mut has_item,
        "has",
        &fl!("node_port_has"),
        PortDirection::Output,
        0.35,
    );
    port(
        &mut has_item,
        "missing",
        &fl!("node_port_missing"),
        PortDirection::Output,
        0.8,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "inventory_has",
        &fl!("node_group_logic"),
        &has_item,
    ))
    .unwrap();

    let mut offer = GraphNode::new(&fl!("node_offer_inventory"), [0., 0.], [35, 87, 134, 255]);
    offer.width = 260.;
    row(
        &mut offer,
        "filter",
        &fl!("node_filter_name"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut offer, "in", "", PortDirection::Input, 0.5);
    port(&mut offer, "out", "Done", PortDirection::Output, 0.5);
    defs.register_node(GraphNodeDefinition::from_template(
        "offer_inventory",
        &fl!("node_group_actions"),
        &offer,
    ))
    .unwrap();

    defs.register_node(GraphNodeDefinition::from_template(
        "on_area",
        &fl!("node_group_events"),
        &on_area,
    ))
    .unwrap();
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
    let mut set_attribute =
        GraphNode::new(&fl!("node_set_attribute"), [0., 0.], [35, 87, 134, 255]);
    set_attribute.width = 300.;
    row(
        &mut set_attribute,
        "attribute",
        &fl!("node_attribute"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut set_attribute,
        "value_kind",
        &fl!("node_value_kind"),
        GraphControlValue::Choice {
            options: vec![
                fl!("node_value_text"),
                fl!("node_value_number"),
                fl!("node_value_bool"),
                "Toggle".to_string(),
            ],
            selected: 0,
        },
    );
    row(
        &mut set_attribute,
        "value",
        &fl!("node_value"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut set_attribute, "in", "", PortDirection::Input, 0.5);
    port(
        &mut set_attribute,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "set_attribute",
        &fl!("node_group_actions"),
        &set_attribute,
    ))
    .unwrap();
    let mut teleport = GraphNode::new(&fl!("node_teleport"), [0., 0.], [35, 87, 134, 255]);
    teleport.width = 300.;
    row(
        &mut teleport,
        "area",
        &fl!("node_area"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut teleport,
        "sector",
        &fl!("node_region"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut teleport, "in", "", PortDirection::Input, 0.5);
    port(
        &mut teleport,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "teleport",
        &fl!("node_group_actions"),
        &teleport,
    ))
    .unwrap();
    let mut message = GraphNode::new(&fl!("node_message"), [0., 0.], [35, 87, 134, 255]);
    message.width = 300.;
    row(
        &mut message,
        "text",
        &fl!("node_text"),
        GraphControlValue::Text(String::new()),
    );
    row(
        &mut message,
        "role",
        &fl!("node_role"),
        GraphControlValue::Text(String::new()),
    );
    port(&mut message, "in", "", PortDirection::Input, 0.5);
    port(
        &mut message,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "message",
        &fl!("node_group_actions"),
        &message,
    ))
    .unwrap();
    let mut state = GraphNode::new(&fl!("node_state"), [0., 0.], [35, 87, 134, 255]);
    row(
        &mut state,
        "state",
        &fl!("node_life_state"),
        GraphControlValue::Choice {
            options: vec![
                fl!("node_state_alive"),
                fl!("node_state_dead"),
                fl!("node_state_sleeping"),
                fl!("node_state_unconscious"),
            ],
            selected: 0,
        },
    );
    port(&mut state, "in", "", PortDirection::Input, 0.5);
    port(
        &mut state,
        "out",
        &fl!("node_done"),
        PortDirection::Output,
        0.5,
    );
    defs.register_node(GraphNodeDefinition::from_template(
        "state",
        &fl!("node_group_actions"),
        &state,
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
    let mut removed_prompt_ports = std::collections::HashSet::new();
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
                        let configuration = node
                            .definition
                            .as_deref()
                            .is_some_and(|id| id.starts_with("entity"));
                        *min = if configuration { lo.min(*value) } else { *lo };
                        *max = if configuration { hi.max(*value) } else { *hi };
                        *step = *increment;
                        if !configuration {
                            *value = value.clamp(*lo, *max);
                        }
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
                        let profile = (matches!(
                            row.key.as_deref(),
                            Some("profile" | "action" | "race" | "class" | "command")
                        ))
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
            if node.definition.as_deref() == Some("prompt") {
                let answer_labels: Vec<String> = node
                    .rows
                    .iter()
                    .find(|row| row.key.as_deref() == Some("choices"))
                    .and_then(|row| match &row.value {
                        GraphControlValue::List { rows, .. } => Some(rows),
                        _ => None,
                    })
                    .map(|rows| {
                        rows.iter()
                            .map(|row| match row.first() {
                                Some(GraphControlValue::Text(label)) => label.trim().to_string(),
                                _ => String::new(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                node.ports.retain(|port| {
                    let Some(index) = port
                        .key
                        .as_deref()
                        .and_then(|key| {
                            key.strip_prefix("choice:")
                                .or_else(|| key.strip_prefix("when:"))
                        })
                        .and_then(|index| index.parse::<usize>().ok())
                    else {
                        return true;
                    };
                    if index < 6
                        && answer_labels
                            .get(index)
                            .is_some_and(|label| !label.is_empty())
                    {
                        return true;
                    }
                    removed_prompt_ports.insert(port.id);
                    false
                });
                let count = answer_labels.len().min(6);
                for (index, label) in answer_labels.iter().take(6).enumerate() {
                    if label.is_empty() {
                        continue;
                    }
                    let key = format!("choice:{index}");
                    if !node
                        .ports
                        .iter()
                        .any(|port| port.key.as_deref() == Some(key.as_str()))
                    {
                        port(
                            node,
                            &key,
                            label,
                            PortDirection::Output,
                            (index + 1) as f32 / (count + 1) as f32,
                        );
                    }
                    let when = format!("when:{index}");
                    if !node
                        .ports
                        .iter()
                        .any(|port| port.key.as_deref() == Some(when.as_str()))
                    {
                        port(
                            node,
                            &when,
                            &format!("{} {}", fl!("node_prompt_when"), index + 1),
                            PortDirection::Input,
                            (index + 1) as f32 / (count + 1) as f32,
                        );
                    }
                }
                for port in &mut node.ports {
                    let Some(index) = port
                        .key
                        .as_deref()
                        .and_then(|key| {
                            key.strip_prefix("choice:")
                                .or_else(|| key.strip_prefix("when:"))
                        })
                        .and_then(|index| index.parse::<usize>().ok())
                    else {
                        continue;
                    };
                    if let Some(label) = answer_labels.get(index) {
                        port.label = if port
                            .key
                            .as_deref()
                            .is_some_and(|key| key.starts_with("when:"))
                        {
                            format!("{} {}", fl!("node_prompt_when"), index + 1)
                        } else {
                            label.clone()
                        };
                        port.position = (index + 1) as f32 / (count + 1) as f32;
                    }
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
    if !removed_prompt_ports.is_empty() {
        doc.connections.retain(|connection| {
            !removed_prompt_ports.contains(&connection.from)
                && !removed_prompt_ports.contains(&connection.to)
        });
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
