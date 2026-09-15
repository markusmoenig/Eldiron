use theframework::thegraph::*;

/// Host-owned node/event catalogue; the framework contains no Eldiron event names.
pub fn definitions() -> GraphDefinitions {
    let mut routine = GraphNode::new("Routine", [50., 140.], [16, 112, 98, 255]);
    routine.rows.push(GraphRow::new(
        "Normal activity",
        GraphControlValue::Choice {
            options: vec!["Daily life".into(), "Guard duty".into()],
            selected: 0,
        },
    ));
    routine.ports.push(GraphPort::new(
        "",
        PortDirection::Output,
        PortSide::Right,
        0.55,
    ));
    let mut time = GraphNode::new("Time Range", [400., 140.], [16, 112, 98, 255]);
    time.rows.push(GraphRow::new(
        "From",
        GraphControlValue::Custom {
            kind: "time".into(),
            data: 840.into(),
        },
    ));
    time.rows.push(GraphRow::new(
        "Until",
        GraphControlValue::Custom {
            kind: "time".into(),
            data: 960.into(),
        },
    ));
    time.ports.push(GraphPort::new(
        "",
        PortDirection::Input,
        PortSide::Left,
        0.4,
    ));
    time.ports.push(GraphPort::new(
        "Inside",
        PortDirection::Output,
        PortSide::Right,
        0.4,
    ));
    time.ports.push(GraphPort::new(
        "Outside",
        PortDirection::Output,
        PortSide::Bottom,
        0.5,
    ));
    let mut walk = GraphNode::new("Random Walk", [790., 140.], [35, 87, 134, 255]);
    walk.rows.push(GraphRow::new(
        "Destination",
        GraphControlValue::Choice {
            options: vec!["Office".into(), "Garden".into(), "Workshop".into()],
            selected: 0,
        },
    ));
    walk.rows.push(GraphRow::new(
        "Speed",
        GraphControlValue::Number {
            value: 1.,
            min: 0.1,
            max: 3.,
            step: 0.1,
        },
    ));
    walk.rows.push(GraphRow::new(
        "Pathfinding",
        GraphControlValue::Choice {
            options: vec!["Navigation".into(), "Direct".into()],
            selected: 0,
        },
    ));
    walk.rows.push(GraphRow::new(
        "Area preview",
        GraphControlValue::Preview {
            asset: "office".into(),
            caption: String::new(),
        },
    ));
    walk.ports.push(GraphPort::new(
        "",
        PortDirection::Input,
        PortSide::Left,
        0.3,
    ));
    walk.ports.push(GraphPort::new(
        "Cancel",
        PortDirection::Input,
        PortSide::Top,
        0.7,
    ));
    walk.ports.push(GraphPort::new(
        "Done",
        PortDirection::Output,
        PortSide::Right,
        0.8,
    ));
    let mut dialogue = GraphNode::new("Dialogue", [400., 460.], [35, 87, 134, 255]);
    dialogue.width = 280.;
    dialogue.rows.push(GraphRow::new(
        "Speaker",
        GraphControlValue::Choice {
            options: vec!["This character".into(), "Event > Speaker".into()],
            selected: 0,
        },
    ));
    dialogue.rows.push(GraphRow::new(
        "Line · preset",
        GraphControlValue::Choice {
            options: vec![
                "Coming along?".into(),
                "Good to see you.".into(),
                "I have work to do.".into(),
            ],
            selected: 0,
        },
    ));
    dialogue.rows.push(GraphRow::new(
        "Choice",
        GraphControlValue::Text("Yes, let's go.".into()),
    ));
    dialogue.rows.push(GraphRow::new(
        "Choice",
        GraphControlValue::Text("Not now.".into()),
    ));
    dialogue.ports.push(GraphPort::new(
        "Talk",
        PortDirection::Input,
        PortSide::Left,
        0.2,
    ));
    for i in 2..4 {
        let mut p = GraphPort::new("", PortDirection::Output, PortSide::Right, 0.);
        p.row = Some(dialogue.rows[i].id);
        dialogue.ports.push(p);
    }
    let mut arrived = GraphNode::new("Event", [50., 600.], [16, 112, 98, 255]);
    arrived.rows.push(GraphRow::new(
        "Event",
        GraphControlValue::Custom {
            kind: "event".into(),
            data: "arrived".into(),
        },
    ));
    arrived.ports.push(GraphPort::new(
        "",
        PortDirection::Output,
        PortSide::Right,
        0.5,
    ));
    let mut equals = GraphNode::new("Text Equals", [400., 600.], [16, 112, 98, 255]);
    equals.rows.push(GraphRow::new(
        "Equals · exact match",
        GraphControlValue::Text("garden".into()),
    ));
    equals.ports.push(GraphPort::new(
        "",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    equals.ports.push(GraphPort::new(
        "Match",
        PortDirection::Output,
        PortSide::Right,
        0.65,
    ));
    equals.rows.push(GraphRow::new(
        "Value",
        GraphControlValue::Text(String::new()),
    ));
    equals.rows[1].value_type = Some(GraphValueType::Text);
    let mut say = GraphNode::new("Say", [790., 650.], [35, 87, 134, 255]);
    say.rows.push(GraphRow::new(
        "Text",
        GraphControlValue::Text("Welcome to the garden.".into()),
    ));
    say.rows[0].value_type = Some(GraphValueType::Text);
    say.ports.push(GraphPort::new(
        "",
        PortDirection::Input,
        PortSide::Left,
        0.5,
    ));
    let mut definitions = GraphDefinitions::default();
    for (id, label, fields) in [
        (
            "arrived",
            "Arrived",
            vec![("destination", "Destination", GraphValueType::Text)],
        ),
        (
            "damaged",
            "Damaged",
            vec![
                ("attacker", "Attacker", GraphValueType::Entity),
                ("amount", "Amount", GraphValueType::Number),
                ("kind", "Damage Kind", GraphValueType::Text),
            ],
        ),
        (
            "delivery",
            "Delivery Arrived (custom)",
            vec![
                ("location", "Delivery Location", GraphValueType::Text),
                ("quantity", "Quantity", GraphValueType::Number),
            ],
        ),
    ] {
        definitions
            .register_event(GraphEventDefinition {
                id: id.into(),
                label: label.into(),
                fields: fields
                    .into_iter()
                    .map(|(id, label, value_type)| GraphEventField {
                        id: id.into(),
                        label: label.into(),
                        value_type,
                    })
                    .collect(),
            })
            .unwrap();
    }
    for (id, category, template) in [
        ("routine", "Behavior", &routine),
        ("time_range", "Conditions", &time),
        ("random_walk", "Movement", &walk),
        ("dialogue", "Interaction", &dialogue),
        ("event", "Events", &arrived),
        ("text_equals", "Conditions", &equals),
        ("say", "Interaction", &say),
    ] {
        let mut definition = GraphNodeDefinition::from_template(id, category, template);
        if id == "event" {
            definition.parameters[0].id = "event".into();
            definition.event_parameter = Some("event".into());
        }
        if id == "text_equals" {
            definition.parameters[0].id = "expected".into();
            definition.parameters[1].id = "value".into();
        }
        definitions.register_node(definition).unwrap();
    }
    definitions
}
