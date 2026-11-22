use super::*;
use shared::prelude::Region;
use std::collections::{HashMap, HashSet};
use theframework::prelude::TheTime;
use theframework_core::graph::{GraphControlValue, GraphDocument, GraphNode};

pub(super) struct Section {
    pub(super) label: String,
    pub(super) steps: Vec<String>,
    routine: bool,
}

pub(super) fn summarize(project: &Project, region: &Region, subject: &Subject) -> Vec<Section> {
    let (kind, template) = match subject.kind {
        Kind::Character => (
            "character",
            region
                .characters
                .get(&subject.id)
                .map(|placed| placed.character_id)
                .or_else(|| {
                    project
                        .characters
                        .values()
                        .find(|t| subject.template_name.as_deref() == Some(&t.name))
                        .map(|t| t.id)
                }),
        ),
        Kind::Item => (
            "item",
            region
                .items
                .get(&subject.id)
                .map(|placed| placed.item_id)
                .or_else(|| {
                    project
                        .items
                        .values()
                        .find(|t| subject.template_name.as_deref() == Some(&t.name))
                        .map(|t| t.id)
                }),
        ),
    };
    let instance = format!("behavior/region/{}/{kind}/{}", region.id, subject.id);
    let keys = std::iter::once(instance).chain(template.map(|id| format!("behavior/{kind}/{id}")));
    let mut sections = Vec::new();
    for graph in keys.filter_map(|key| project.node_graphs.get(&key)) {
        let mut pending = vec![graph];
        while let Some(branch) = pending.pop() {
            if let Some(branches) = branch.get("branches").and_then(serde_json::Value::as_array) {
                pending.extend(branches.iter().rev());
            }
            if let Ok(doc) = serde_json::from_value::<GraphDocument>(branch.clone())
                && doc.version == 1
            {
                summarize_branch(&doc, region, &mut sections);
            }
        }
    }
    sections.sort_by_key(|section| !section.routine);
    sections
}

fn field<'a>(node: &'a GraphNode, key: &str) -> Option<&'a str> {
    match &node
        .rows
        .iter()
        .find(|r| r.key.as_deref() == Some(key))?
        .value
    {
        GraphControlValue::Text(value) | GraphControlValue::Label(value) => Some(value.as_str()),
        GraphControlValue::Choice { options, selected } => {
            options.get(*selected).map(String::as_str)
        }
        GraphControlValue::Custom { data, .. } => data.as_str(),
        _ => None,
    }
    .map(str::trim)
    .filter(|value| !value.is_empty())
}

fn definition(node: &GraphNode) -> &str {
    node.definition.as_deref().unwrap_or_default()
}

fn title(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    first
        .to_uppercase()
        .chain(chars)
        .map(|c| if c == '_' { ' ' } else { c })
        .collect()
}

fn step(node: &GraphNode, region: &Region) -> Option<String> {
    let label = match definition(node) {
        "go_to" => fl!(
            "behavior_overview_go_to",
            target = field(node, "destination")?
        ),
        "perform_intent" => {
            let intent = title(field(node, "intent")?);
            let target = field(node, "item")
                .and_then(|value| Uuid::parse_str(value).ok())
                .and_then(|id| region.items.get(&id))
                .map(|item| item.name.clone())
                .unwrap_or_else(|| fl!("behavior_overview_placed_item"));
            format!("{intent} {target}")
        }
        "offer_inventory" => field(node, "filter").map_or_else(
            || fl!("behavior_overview_offer_inventory"),
            |filter| fl!("behavior_overview_offer_stock", filter = filter),
        ),
        "say" => format!("“{}”", field(node, "text")?),
        "state" => fl!("behavior_overview_state", state = field(node, "state")?),
        "set_attribute" if field(node, "attribute") == Some("active") => {
            match (field(node, "value_kind"), field(node, "value")) {
                (Some("Toggle"), _) => fl!("behavior_overview_toggle"),
                (_, Some("true")) => fl!("behavior_overview_turn_on"),
                (_, Some("false")) => fl!("behavior_overview_turn_off"),
                _ => node.title.clone(),
            }
        }
        "set_emit_light" if field(node, "emit") == Some("Follow Active") => return None,
        "set_attribute" | "filter" | "time_range" | "random_chance" | "resume_routine"
        | "notify_in" | "routine" | "event" | "on_event" => return None,
        "" => return None,
        _ => node.title.clone(),
    };
    (!label.trim().is_empty()).then_some(label)
}

fn minute(value: &str) -> Option<u16> {
    let (hours, minutes) = value.split_once(':')?;
    let hours: u16 = hours.parse().ok()?;
    let minutes: u16 = minutes.parse().ok()?;
    (hours < 24 && minutes < 60).then(|| hours * 60 + minutes)
}

fn window(start: u16, end: u16) -> String {
    let clock = |minute: u16| format!("{:02}:{:02}", minute % 1440 / 60, minute % 60);
    format!("{}–{}", clock(start), clock(end))
}

type Routes = Vec<(String, Vec<String>)>;

fn summarize_branch(doc: &GraphDocument, region: &Region, sections: &mut Vec<Section>) {
    let disabled: HashSet<_> = doc
        .nodes
        .iter()
        .filter(|n| n.disabled)
        .map(|n| n.id)
        .collect();
    let nodes: HashMap<_, _> = doc
        .nodes
        .iter()
        .filter(|n| !n.disabled && !n.branch.is_some_and(|root| disabled.contains(&root)))
        .map(|n| (n.id, n))
        .collect();
    let ports: HashMap<_, _> = nodes
        .values()
        .flat_map(|node| {
            node.ports
                .iter()
                .map(|port| (port.id, (node.id, port.key.as_deref().unwrap_or_default())))
        })
        .collect();
    let mut edges: HashMap<Uuid, Vec<(&str, Uuid)>> = HashMap::new();
    for connection in &doc.connections {
        if let (Some(&(from, port)), Some(&(to, _))) =
            (ports.get(&connection.from), ports.get(&connection.to))
        {
            edges.entry(from).or_default().push((port, to));
        }
    }
    let mut times = vec![0, 1440];
    for node in nodes.values().filter(|n| definition(n) == "time_range") {
        times.extend(
            ["start", "end"]
                .into_iter()
                .filter_map(|key| minute(field(node, key)?)),
        );
    }
    times.sort_unstable();
    times.dedup();

    for root in doc.nodes.iter().filter(|n| nodes.contains_key(&n.id)) {
        let routine = definition(root) == "routine";
        let event = match definition(root) {
            "routine" => fl!("behavior_overview_routine"),
            "event" | "on_event" => {
                let name = field(root, "event")
                    .map(title)
                    .unwrap_or_else(|| root.title.clone());
                fl!("behavior_overview_on_event", event = name)
            }
            _ => continue,
        };
        let mut spans: Vec<(u16, u16, Routes)> = Vec::new();
        for pair in times.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            let now = TheTime::new_time((start / 60) as u8, (start % 60) as u8).unwrap();
            let mut visitor = vec![(root.id, event.clone())];
            let mut seen = HashSet::new();
            let mut routes = Routes::new();
            while let Some((id, label)) = visitor.pop() {
                if !seen.insert((id, label.clone())) {
                    continue;
                }
                let node = nodes[&id];
                if let Some(action) = step(node, region) {
                    if let Some((_, steps)) = routes.iter_mut().find(|(name, _)| name == &label) {
                        if !steps.contains(&action) {
                            steps.push(action);
                        }
                    } else {
                        routes.push((label.clone(), vec![action]));
                    }
                }
                let condition = if definition(node) == "time_range" {
                    field(node, "start")
                        .zip(field(node, "end"))
                        .and_then(|(start, end)| {
                            rusterix::server::nodes::time_range_contains(start, end, now).ok()
                        })
                        .map(|inside| if inside { "inside" } else { "outside" })
                } else {
                    None
                };
                for &(port, to) in edges.get(&id).into_iter().flatten().rev() {
                    if condition.is_some_and(|port_to_use| port != port_to_use)
                        || matches!(port, "error" | "failed" | "cannot_engage")
                    {
                        continue;
                    }
                    let label = if definition(node) == "filter"
                        && port == "match"
                        && field(node, "field") == Some("event.intent")
                        && matches!(field(node, "operator"), Some("=" | "Equals"))
                        && let Some(intent) = field(node, "expected")
                    {
                        fl!("behavior_overview_on_event", event = title(intent))
                    } else {
                        label.clone()
                    };
                    visitor.push((to, label));
                }
            }
            if let Some((_, previous_end, previous)) = spans.last_mut()
                && *previous == routes
            {
                *previous_end = end;
            } else {
                spans.push((start, end, routes));
            }
        }
        if spans.len() > 1 && spans.first().map(|span| &span.2) == spans.last().map(|span| &span.2)
        {
            let end = spans.remove(0).1;
            spans.last_mut().unwrap().1 = end;
        }
        for (start, end, routes) in spans {
            for (label, steps) in routes {
                let label = if start == 0 && end == 1440 {
                    label
                } else {
                    format!("{label} · {}", window(start, end))
                };
                if !sections
                    .iter()
                    .any(|entry| entry.label == label && entry.steps == steps)
                {
                    sections.push(Section {
                        label,
                        steps,
                        routine,
                    });
                }
            }
        }
    }
}
