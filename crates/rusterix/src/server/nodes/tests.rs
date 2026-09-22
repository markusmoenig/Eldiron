use super::*;
use serde_json::json;
#[derive(Default)]
struct Output(Vec<String>);
impl WorldServices for Output {
    fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
        Default::default()
    }
    fn say(&mut self, _: &Actor, text: String) {
        self.0.push(text);
    }
}
fn graph(event: &str, text: &str) -> Value {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let out = Uuid::new_v4();
    let input = Uuid::new_v4();
    json!({"version":1,"nodes":[
        {"id":a,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":event}}}],"ports":[{"id":out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":b,"definition":"say","rows":[{"key":"text","value":{"Text":text}}],"ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[{"id":Uuid::new_v4(),"from":out,"to":input}]})
}
#[test]
fn startup_spawn_and_trace_are_native_and_once_only() {
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let identity = Uuid::new_v4();
    runtime
        .spawn(
            identity,
            Uuid::new_v4(),
            1,
            Some(
                Registry::builtin()
                    .compile(&graph("startup", "Hello"))
                    .unwrap(),
            ),
        )
        .unwrap();
    runtime.update(&mut output);
    runtime.update(&mut output);
    assert_eq!(output.0, ["Hello"]);
    assert_eq!(runtime.traces.len(), 2);
    assert!(runtime.traces[1].connection.is_some());
    assert_eq!(runtime.traces[0].event.owner, EventOwner::Entity(identity));
}
#[test]
fn invisibility_and_sleep_do_not_remove_presence_or_events() {
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let id = Uuid::new_v4();
    runtime
        .spawn(
            id,
            Uuid::new_v4(),
            1,
            Some(
                Registry::builtin()
                    .compile(&graph("startup", "Invisible but alive"))
                    .unwrap(),
            ),
        )
        .unwrap();
    let actor = runtime.actors.get_mut(&id).unwrap();
    actor.visible = false;
    actor.activity = Activity::Sleeping;
    runtime.update(&mut output);
    assert_eq!(output.0.len(), 1);
    assert!(runtime.actors.contains_key(&id));
}
#[test]
fn set_attribute_node_keeps_health_integral() {
    use super::region::typed_attribute_value;

    // Health is integer gameplay state, whatever the previous value was.
    assert_eq!(
        typed_attribute_value("HP", EventAttribute::Number(5.0), None, "HP"),
        crate::Value::Int(5)
    );
    assert_eq!(
        typed_attribute_value(
            "HP",
            EventAttribute::Number(5.0),
            Some(&crate::Value::Float(1.0)),
            "HP"
        ),
        crate::Value::Int(5)
    );

    // Other numeric attributes follow the type the attribute already carries.
    assert_eq!(
        typed_attribute_value(
            "STAMINA",
            EventAttribute::Number(3.9),
            Some(&crate::Value::UInt(7)),
            "HP"
        ),
        crate::Value::UInt(3)
    );
    assert_eq!(
        typed_attribute_value(
            "focus",
            EventAttribute::Number(3.5),
            Some(&crate::Value::Float(0.0)),
            "HP"
        ),
        crate::Value::Float(3.5)
    );
    // An attribute with no prior type keeps the historical float behaviour.
    assert_eq!(
        typed_attribute_value("weight", EventAttribute::Number(2.0), None, "HP"),
        crate::Value::Float(2.0)
    );
}
#[test]
fn death_cancels_startup_and_stale_events_cannot_target_respawn() {
    let mut runtime = Runtime::default();
    let id = Uuid::new_v4();
    let map = Uuid::new_v4();
    let old = runtime
        .spawn(
            id,
            map,
            1,
            Some(
                Registry::builtin()
                    .compile(&graph("startup", "Must not run"))
                    .unwrap(),
            ),
        )
        .unwrap();
    assert!(runtime.despawn(old));
    let new = runtime.spawn(id, map, 2, None).unwrap();
    assert_ne!(old, new);
    assert!(!runtime.send(old, "arrived", BTreeMap::new(), 0));
    assert!(!runtime.despawn(old));
    let mut output = Output::default();
    runtime.update(&mut output);
    assert!(output.0.is_empty());
    assert!(runtime.is_current(new));
}
#[test]
fn custom_event_payload_reaches_say_and_missing_values_fail_visibly() {
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let actor = runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(
                Registry::builtin()
                    .compile(&graph("custom.greeting", "Welcome {event.destination}"))
                    .unwrap(),
            ),
        )
        .unwrap();
    runtime.send(
        actor,
        "custom.greeting",
        BTreeMap::from([("destination".into(), EventField::Text("garden".into()))]),
        1,
    );
    runtime.update(&mut output);
    assert_eq!(output.0, ["Welcome garden"]);
    runtime.send(actor, "custom.greeting", BTreeMap::new(), 2);
    runtime.update(&mut output);
    assert_eq!(output.0.len(), 1);
    assert!(runtime.traces.back().unwrap().error.is_some());
}
#[test]
fn malformed_graphs_are_rejected_without_fallback() {
    let registry = Registry::builtin();
    let mut doc = graph("startup", "Hello");
    doc["nodes"][1]["definition"] = json!("eldrin");
    assert!(registry.compile(&doc).is_err());
    let mut doc = graph("startup", "Hello");
    doc["connections"][0]["to"] = json!(Uuid::new_v4());
    assert!(registry.compile(&doc).is_err());
    let mut doc = graph("startup", "Hello");
    doc["version"] = json!(2);
    assert!(registry.compile(&doc).is_err());
}

#[test]
fn filter_routes_only_matching_branch_and_preserves_payload() {
    let mut doc = graph("arrived", "Hello {event.destination}");
    let filter = Uuid::new_v4();
    let input = Uuid::new_v4();
    let output = Uuid::new_v4();
    let destination = doc["connections"][0]["to"].clone();
    doc["connections"][0]["to"] = json!(input);
    doc["nodes"].as_array_mut().unwrap().push(json!({"id": filter, "definition":"filter", "rows":[
        {"key":"field","value":{"Text":"event.destination"}},
        {"key":"operator","value":{"Choice":{"selected":0}}},
        {"key":"expected","value":{"Text":"garden"}}
    ],"ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"},{"id":output,"key":"match","direction":"Output","kind":"flow"}]}));
    doc["connections"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":Uuid::new_v4(),"from":output,"to":destination}));
    let mut runtime = Runtime::default();
    let mut out = Output::default();
    let actor = runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    for destination in ["office", "garden"] {
        runtime.send(
            actor,
            "arrived",
            BTreeMap::from([("destination".into(), EventField::Text(destination.into()))]),
            1,
        );
    }
    runtime.update(&mut out);
    assert_eq!(out.0, ["Hello garden"]);
}
#[test]
fn cycles_are_bounded_and_reported() {
    let mut doc = graph("startup", "Loop");
    let port = Uuid::new_v4();
    doc["nodes"][1]["ports"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":port,"key":"out","direction":"Output","kind":"flow"}));
    let target = doc["connections"][0]["to"].clone();
    doc["connections"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":Uuid::new_v4(),"from":port,"to":target}));
    let mut runtime = Runtime::default();
    let mut out = Output::default();
    runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.update(&mut out);
    assert!(out.0.len() < 4096);
    assert!(runtime.traces.len() <= 512);
    assert!(
        runtime
            .traces
            .back()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("budget")
    );
}

#[test]
fn external_module_registers_without_scheduler_changes() {
    struct CustomModule;
    struct Custom;
    impl NodeModule for CustomModule {
        fn id(&self) -> &'static str {
            "custom"
        }
        fn inputs(&self) -> &'static [&'static str] {
            &["in"]
        }
        fn outputs(&self) -> &'static [&'static str] {
            &["out"]
        }
        fn compile(&self, _: &BTreeMap<String, Value>) -> Result<Box<dyn Operation>, String> {
            Ok(Box::new(Custom))
        }
    }
    impl Operation for Custom {
        fn execute(
            &self,
            ctx: &EventContext<'_>,
            world: &mut dyn WorldServices,
        ) -> Result<&'static str, String> {
            world.say(ctx.actor, format!("Hour {}", ctx.time.hours));
            Ok("out")
        }
    }
    let mut registry = Registry::builtin();
    registry.register(Box::new(CustomModule)).unwrap();
    assert!(registry.register(Box::new(CustomModule)).is_err());
    let mut doc = graph("startup", "unused");
    doc["nodes"][1]["definition"] = json!("custom");
    let mut runtime = Runtime::default();
    let mut out = Output::default();
    runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(registry.compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.update(&mut out);
    assert_eq!(out.0, ["Hour 0"]);
}

#[test]
fn engine_dispatch_waits_for_registered_player_and_replaces_legacy_behavior() {
    use crate::server::{
        message::RegionMessage, region_host::run_server_named_fn, regionctx::RegionCtx,
    };
    use crate::vm::{Execution, VM, VMValue};
    let mut doc = graph("startup", "unused");
    doc["nodes"][1]["definition"] = json!("player_camera");
    doc["nodes"][1]["rows"] = json!([{"key":"camera","value":{"Choice":{"selected":1}}}]);
    let template = Uuid::new_v4();
    let identity = Uuid::new_v4();
    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 17;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), doc)]),
        characters: HashMap::from([("Player".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();
    let program = VM::default()
        .prepare_str("fn event(event, value) { set_attr(\"legacy_ran\", true); }")
        .unwrap();
    let args = [VMValue::from_string("startup"), VMValue::zero()];
    // The dispatch hook must never create the Player itself.
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );
    assert!(ctx.map.entities.is_empty());
    assert!(ctx.node_behaviors.is_none());
    let mut entity = crate::Entity::new();
    entity.id = 17;
    entity.creator_id = identity;
    entity.set_attribute("class_name", crate::Value::Str("Player".into()));
    entity.set_attribute("visible", crate::Value::Bool(false));
    ctx.map.entities.push(entity);
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );
    let entity = &ctx.map.entities[0];
    assert_eq!(
        entity.attributes.get("player_camera"),
        Some(&crate::Value::PlayerCamera(crate::PlayerCamera::D2Grid))
    );
    assert!(entity.attributes.get("legacy_ran").is_none());
    assert!(!entity.attributes.get_bool_default("visible", true));
    assert_eq!(
        ctx.event_observations.back().unwrap().owner,
        EventOwner::Entity(identity)
    );
    let traces: Vec<_> = receiver
        .try_iter()
        .filter_map(|msg| {
            if let RegionMessage::NodeTraces(t) = msg {
                Some(t)
            } else {
                None
            }
        })
        .flatten()
        .collect();
    assert_eq!(traces.len(), 2);
    assert!(traces[1].connection.is_some());
}

#[test]
fn state_alive_puts_a_stashed_body_back_into_the_world() {
    use crate::server::{region_host::run_server_named_fn, regionctx::RegionCtx};
    use crate::vm::{Execution, VM, VMValue};
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let out = Uuid::new_v4();
    let input = Uuid::new_v4();
    // The shape the Hideout2D player uses: Event(death) -> State(Alive).
    let doc = json!({"version":1,"nodes":[
        {"id":a,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"death"}}}],"ports":[{"id":out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":b,"definition":"state","rows":[{"key":"state","value":{"Choice":{"options":["Alive","Dead","Sleeping","Unconscious"],"selected":0}}}],"ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[{"id":Uuid::new_v4(),"from":out,"to":input}]});
    let template = Uuid::new_v4();
    let identity = Uuid::new_v4();
    let mut ctx = RegionCtx::default();
    ctx.health_attr = "HP".into();
    ctx.curr_entity_id = 17;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), doc)]),
        characters: HashMap::from([("Player".into(), template)]),
        ..Default::default()
    });
    let (sender, _receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();

    let mut entity = crate::Entity::new();
    entity.id = 17;
    entity.creator_id = identity;
    entity.set_attribute("class_name", crate::Value::Str("Player".into()));
    entity.set_attribute("mode", crate::Value::Str("dead".into()));
    entity.set_attribute("HP", crate::Value::Int(0));
    // Invisible for its own reasons. Death must not have written this, and
    // rising must not clear it.
    entity.set_attribute("visible", crate::Value::Bool(false));
    ctx.map.entities.push(entity);
    assert!(
        ctx.stash_entity(17),
        "the body starts dead and out of the world"
    );
    assert!(!ctx.is_entity_present(17));

    let program = VM::default()
        .prepare_str("fn event(event, value) { }")
        .unwrap();
    let args = [VMValue::from_string("death"), VMValue::zero()];
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );

    assert!(
        ctx.is_entity_present(17),
        "State(Alive) has to put the body back into the world"
    );
    assert!(!ctx.is_entity_stashed(17));
    let body = ctx.find_entity(17).expect("body");
    assert_eq!(body.get_mode(), "active");
    assert_eq!(
        body.attributes.get_int("HP"),
        Some(1),
        "rising from zero health restores one hit point"
    );
    assert!(
        !body.attributes.get_bool_default("visible", true),
        "rising must not touch visibility"
    );
}

/// The real Hideout2D character graph, so the engine is checked against the
/// graph that actually ships rather than a hand-written imitation of it.
fn hideout2d_character_graph(name: &str) -> Option<(Value, Uuid)> {
    use std::io::Read;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/Hideout2D.eldiron");
    let bytes = std::fs::read(&path).ok()?;
    let json: Value = if bytes.starts_with(b"PK\x03\x04") {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
        let mut entry = archive.by_name("project.json").ok()?;
        let mut text = Vec::new();
        entry.read_to_end(&mut text).ok()?;
        serde_json::from_slice(&text).ok()?
    } else {
        serde_json::from_slice(&bytes).ok()?
    };
    let character = json["characters"]
        .as_object()?
        .values()
        .find(|character| character["name"] == name)?;
    let id = Uuid::parse_str(character["id"].as_str()?).ok()?;
    let graph = json["node_graphs"][format!("behavior/character/{id}")].clone();
    (!graph.is_null()).then_some((graph, id))
}

fn hideout2d_player_graph() -> Option<(Value, Uuid)> {
    hideout2d_character_graph("Player")
}

/// Both converted fixtures must ship no Eldrin at all: their graphs answer every
/// event, so a leftover script would only be dead, uneditable code.
#[test]
fn shipped_fixtures_carry_no_eldrin() {
    for (label, path) in [
        ("Hideout2D", "test_projects/Hideout2D.eldiron"),
        ("Gate", "test_projects/Gate.eldiron"),
    ] {
        let Some(json) = read_project_json(path) else {
            continue;
        };
        for key in ["world_source", "world_source_debug"] {
            assert!(
                json[key].as_str().unwrap_or("").trim().is_empty(),
                "{label} still ships {key}"
            );
        }
        for section in ["characters", "items", "regions"] {
            let entries: Vec<&Value> = match &json[section] {
                Value::Object(map) => map.values().collect(),
                Value::Array(list) => list.iter().collect(),
                _ => Vec::new(),
            };
            for entry in entries {
                for key in ["source", "source_debug"] {
                    assert!(
                        entry[key].as_str().unwrap_or("").trim().is_empty(),
                        "{label} {section} '{}' still ships {key}",
                        entry["name"].as_str().unwrap_or("?")
                    );
                }
            }
        }
    }
}

/// Every Gate character ships a behaviour graph that compiles, since the scripts
/// it replaced are gone.
#[test]
fn gate_characters_are_converted_and_their_graphs_compile() {
    let Some((characters, graphs)) = gate_character_graphs() else {
        return;
    };
    assert_eq!(characters.len(), 5, "Gate ships five characters");
    for (name, character) in &characters {
        let id = character["id"].as_str().unwrap_or_default();
        let document = graphs
            .get(&format!("behavior/character/{id}"))
            .unwrap_or_else(|| panic!("Gate character '{name}' has no graph"));
        Registry::shared()
            .compile(document)
            .unwrap_or_else(|error| panic!("Gate character '{name}': {error}"));
    }
}

/// Read a project fixture as JSON, unpacking the zip when needed.
fn read_project_json(path: &str) -> Option<Value> {
    use std::io::Read;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    let bytes = std::fs::read(&path).ok()?;
    if bytes.starts_with(b"PK\x03\x04") {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
        let mut entry = archive.by_name("project.json").ok()?;
        let mut text = Vec::new();
        entry.read_to_end(&mut text).ok()?;
        serde_json::from_slice(&text).ok()
    } else {
        serde_json::from_slice(&bytes).ok()
    }
}

/// Read the Gate fixture once: character name -> (character, graph) pairs.
fn gate_character_graphs() -> Option<(Vec<(String, Value)>, serde_json::Map<String, Value>)> {
    let json = read_project_json("test_projects/Gate.eldiron")?;
    let characters = json["characters"]
        .as_object()?
        .values()
        .map(|character| {
            (
                character["name"].as_str().unwrap_or_default().to_string(),
                character.clone(),
            )
        })
        .collect();
    Some((characters, json["node_graphs"].as_object().cloned()?))
}

/// The shipped graph of one named Gate character.
fn gate_character_graph(name: &str) -> Option<(Value, Uuid)> {
    let (characters, graphs) = gate_character_graphs()?;
    let character = characters
        .iter()
        .find(|(character, _)| character == name)
        .map(|(_, character)| character)?;
    let id = character["id"].as_str()?;
    let document = graphs.get(&format!("behavior/character/{id}"))?.clone();
    Some((document, Uuid::parse_str(id).ok()?))
}

/// The shipped Guard graph answers a `damaged` observation with its line, so the
/// migration's wiring runs through the real runtime, not just compilation.
#[test]
fn gate_guard_graph_answers_damage() {
    let Some((document, _)) = gate_character_graph("Guard") else {
        return;
    };
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let handle = runtime
        .attach(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::shared().compile(&document).unwrap()),
        )
        .unwrap();
    runtime.send(handle, "damaged", BTreeMap::new(), 0);
    runtime.update(&mut output);
    assert!(
        output.0.iter().any(|line| line == "How dare you!"),
        "the Guard has to answer damage, got {:?}",
        output.0
    );
    assert!(runtime.traces.iter().all(|trace| trace.error.is_none()));
    runtime.despawn(handle);
}

#[test]
fn hideout2d_player_death_chain_raises_the_stashed_body() {
    use crate::server::{region_host::run_server_named_fn, regionctx::RegionCtx};
    use crate::vm::{Execution, VM, VMValue};
    let Some((document, template)) = hideout2d_player_graph() else {
        // The project is an authoring fixture; a checkout without it still has
        // the behavioural coverage through the synthetic graph test above.
        return;
    };
    let identity = Uuid::new_v4();
    let mut ctx = RegionCtx::default();
    ctx.health_attr = "HP".into();
    ctx.max_health_attr = "MAX_HP".into();
    ctx.curr_entity_id = 3;
    ctx.map.name = "Harbor".into();
    // The death chain ends in Teleport(Start, Harbor), which needs a place to
    // land so the whole chain runs to the end.
    ctx.map
        .geometry_objects
        .push(crate::GeometryObject::box_from_bounds(
            "Start",
            vek::Vec3::new(0.0, 0.0, 0.0),
            vek::Vec3::new(4.0, 2.0, 4.0),
        ));
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), document)]),
        characters: HashMap::from([("Player".into(), template)]),
        ..Default::default()
    });
    let (sender, _receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();

    let mut entity = crate::Entity::new();
    entity.id = 3;
    entity.creator_id = identity;
    entity.set_attribute("class_name", crate::Value::Str("Player".into()));
    entity.set_attribute("mode", crate::Value::Str("dead".into()));
    entity.set_attribute("HP", crate::Value::Int(0));
    entity.set_attribute("MAX_HP", crate::Value::Int(12));
    ctx.map.entities.push(entity);
    assert!(ctx.stash_entity(3), "death took the body out of the world");

    let program = VM::default()
        .prepare_str("fn event(event, value) { }")
        .unwrap();
    let args = [VMValue::from_string("death"), VMValue::zero()];
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );

    assert!(
        ctx.is_entity_present(3),
        "the shipped player graph has to raise the body through State(Alive)"
    );
    assert!(!ctx.is_entity_stashed(3));
    let body = ctx.find_entity(3).expect("body");
    assert_eq!(body.get_mode(), "active");
    assert_eq!(
        body.attributes.get_int("HP"),
        Some(5),
        "the player's death chain has to set the Hideout2D respawn health"
    );
    assert!(
        body.attributes.get("visible").is_none()
            || body.attributes.get_bool_default("visible", true),
        "the death chain must not have written visibility"
    );
}

/// The Hideout2D shape: an NPC graph opens its own dialogue when the player
/// reaches it through `Event(intent) -> Filter(event.intent == "use") -> Dialog`.
#[test]
fn dialog_node_opens_the_actors_dialog_for_the_subject() {
    use crate::server::{
        message::RegionMessage, region_host::run_server_named_fn, regionctx::RegionCtx,
    };
    use crate::vm::{Execution, VM, VMValue};

    let event = Uuid::new_v4();
    let filter = Uuid::new_v4();
    let dialog = Uuid::new_v4();
    let event_out = Uuid::new_v4();
    let filter_in = Uuid::new_v4();
    let filter_match = Uuid::new_v4();
    let dialog_in = Uuid::new_v4();
    let document = json!({"version":1,"nodes":[
        {"id":event,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"intent"}}}],"ports":[{"id":event_out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":filter,"definition":"filter","rows":node_rows(vec![("field", json!({"Text":"event.intent"})),("operator", json!({"Choice":{"selected":0}})),("expected", json!({"Text":"use"}))]),"ports":[{"id":filter_in,"key":"in","direction":"Input","kind":"flow"},{"id":filter_match,"key":"match","direction":"Output","kind":"flow"}]},
        {"id":dialog,"definition":"dialog","rows":node_rows(vec![("node", json!({"Text":"greeting"}))]),"ports":[{"id":dialog_in,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[{"id":Uuid::new_v4(),"from":event_out,"to":filter_in},{"id":Uuid::new_v4(),"from":filter_match,"to":dialog_in}]});

    let template = Uuid::new_v4();
    let speaker_identity = Uuid::new_v4();
    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), document)]),
        characters: HashMap::from([("Warden Mara".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();

    let mut listener = crate::Entity::new();
    listener.id = 1;
    listener.creator_id = Uuid::new_v4();
    listener.set_attribute("class_name", crate::Value::Str("Player".into()));
    ctx.map.entities.push(listener);

    let mut speaker = crate::Entity::new();
    speaker.id = 2;
    speaker.creator_id = speaker_identity;
    speaker.set_attribute("class_name", crate::Value::Str("Warden Mara".into()));
    ctx.entity_classes.insert(2, "Warden Mara".into());
    ctx.entity_class_data.insert(
        "Warden Mara".into(),
        r#"
        [dialog]
        start = "greeting"

        [dialog.nodes.greeting]
        text = "Well met."
        choices = [{ label = "Bye.", end = true }]
        "#
        .into(),
    );
    ctx.map.entities.push(speaker);

    let program = VM::default()
        .prepare_str("fn event(event, value) { }")
        .unwrap();
    let args = [
        VMValue::from_string("intent"),
        VMValue::new_with_string(1.0, 1.0, 0.0, "use"),
    ];
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );

    let messages: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        messages.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text == "Well met." && role == "dialog"
        )),
        "the speaker's dialogue has to open for the listener, got {messages:?}"
    );
    assert!(
        ctx.active_choice_sessions
            .iter()
            .any(|session| session.from == 2 && session.to == 1),
        "the choice session has to run from the speaker to the listener"
    );
}

/// A Dialogue node parks the flow, then resumes down the branch the player
/// picked. This is the node-native replacement for the TOML dialog tree.
#[test]
fn dialogue_node_waits_and_resumes_on_the_chosen_branch() {
    use crate::server::{
        message::{Choice, RegionMessage},
        region_host::run_server_named_fn,
        regionctx::RegionCtx,
    };
    use crate::vm::{Execution, VM, VMValue};

    let event = Uuid::new_v4();
    let dialogue = Uuid::new_v4();
    let msg_a = Uuid::new_v4();
    let msg_b = Uuid::new_v4();
    let event_out = Uuid::new_v4();
    let dialogue_in = Uuid::new_v4();
    let choice_0 = Uuid::new_v4();
    let choice_1 = Uuid::new_v4();
    let a_in = Uuid::new_v4();
    let b_in = Uuid::new_v4();
    let choices = json!({"List":{"columns":[
        {"id":"label","label":"Choice","control":{"Text":""}},
        {"id":"condition","label":"Condition","control":{"Text":""}}
    ],"rows":[
        [{"Text":"Ask about the bell"},{"Text":""}],
        [{"Text":"Leave"},{"Text":""}]
    ]}});
    let document = json!({"version":1,"nodes":[
        {"id":event,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"intent"}}}],"ports":[{"id":event_out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":dialogue,"definition":"dialogue","rows":node_rows(vec![("text", json!({"Text":"Well met."})),("choices", choices)]),"ports":[
            {"id":dialogue_in,"key":"in","direction":"Input","kind":"flow"},
            {"id":choice_0,"key":"choice:0","direction":"Output","kind":"flow"},
            {"id":choice_1,"key":"choice:1","direction":"Output","kind":"flow"}
        ]},
        {"id":msg_a,"definition":"set_attribute","rows":node_rows(vec![("attribute", json!({"Text":"asked"})),("value_kind", json!({"Choice":{"options":["Text","Number","Boolean"],"selected":2}})),("value", json!({"Text":"true"}))]),"ports":[{"id":a_in,"key":"in","direction":"Input","kind":"flow"}]},
        {"id":msg_b,"definition":"set_attribute","rows":node_rows(vec![("attribute", json!({"Text":"left"})),("value_kind", json!({"Choice":{"options":["Text","Number","Boolean"],"selected":2}})),("value", json!({"Text":"true"}))]),"ports":[{"id":b_in,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[
        {"id":Uuid::new_v4(),"from":event_out,"to":dialogue_in},
        {"id":Uuid::new_v4(),"from":choice_0,"to":a_in},
        {"id":Uuid::new_v4(),"from":choice_1,"to":b_in}
    ]});

    let template = Uuid::new_v4();
    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), document)]),
        characters: HashMap::from([("Warden Mara".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();

    let mut listener = crate::Entity::new();
    listener.id = 1;
    listener.creator_id = Uuid::new_v4();
    listener.set_attribute("class_name", crate::Value::Str("Player".into()));
    ctx.map.entities.push(listener);

    let mut speaker = crate::Entity::new();
    speaker.id = 2;
    speaker.creator_id = Uuid::new_v4();
    speaker.set_attribute("class_name", crate::Value::Str("Warden Mara".into()));
    ctx.entity_classes.insert(2, "Warden Mara".into());
    ctx.map.entities.push(speaker);

    let program = VM::default()
        .prepare_str("fn event(event, value) { }")
        .unwrap();
    let args = [
        VMValue::from_string("intent"),
        VMValue::new_with_string(1.0, 1.0, 0.0, "use"),
    ];
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );

    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        offered.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text == "Well met." && role == "dialog"
        )),
        "the dialogue text has to reach the listener, got {offered:?}"
    );
    assert!(
        offered.iter().any(|message| matches!(
            message,
            RegionMessage::MultipleChoice(choice)
                if choice.choices.iter().filter(|c| matches!(c, Choice::NodeChoice(_))).count() == 2
        )),
        "both choices have to be offered, got {offered:?}"
    );

    // Nothing down either branch until the player answers.
    let parked: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(parked.is_empty(), "the flow has to wait, got {parked:?}");

    // The player picks the first choice; the flow resumes on choice:0.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);

    let speaker = ctx.find_entity(2).expect("speaker");
    assert!(
        speaker.attributes.get_bool_default("asked", false),
        "the chosen branch has to run"
    );
    assert!(
        !speaker.attributes.get_bool_default("left", false),
        "only the chosen branch may run"
    );
}

/// The migrated Hideout2D conversation, against the graph that actually ships:
/// Mara greets, hides the choices her conditions rule out, then continues into
/// the briefing the player picked. Her node carries the conversation document
/// the editor writes, rather than the legacy steps/choices tables.
#[test]
fn hideout2d_mara_dialogue_runs_from_the_shipped_graph() {
    use crate::server::{
        message::{Choice, RegionMessage},
        region_host::run_server_named_fn,
        regionctx::RegionCtx,
    };
    use crate::vm::{Execution, VM, VMValue};

    let Some((graph, template)) = hideout2d_character_graph("Warden Mara") else {
        return;
    };

    // The migration target itself, so an accidental revert to the tables shows.
    let talk = graph["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["definition"] == "talk"))
        .expect("Mara ships a Talk node");
    let row = talk["rows"]
        .as_array()
        .and_then(|rows| rows.iter().find(|row| row["key"] == "conversation"))
        .expect("Mara's Talk node carries the conversation document");
    let document = Conversation::from_control(&row["value"]).expect("the document parses");
    assert_eq!(document.entry, "greeting");
    assert_eq!(
        document.step_names(),
        vec!["greeting", "bell", "prepare", "contract", "settled"]
    );

    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), graph)]),
        characters: HashMap::from([("Warden Mara".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();

    let mut player = crate::Entity::new();
    player.id = 1;
    player.creator_id = Uuid::new_v4();
    player.set_attribute("class_name", crate::Value::Str("Player".into()));
    ctx.map.entities.push(player);

    let mut mara = crate::Entity::new();
    mara.id = 2;
    mara.creator_id = template;
    mara.set_attribute("class_name", crate::Value::Str("Warden Mara".into()));
    mara.set_attribute("barrow_active", crate::Value::Bool(false));
    mara.set_attribute("barrow_complete", crate::Value::Bool(false));
    ctx.entity_classes.insert(2, "Warden Mara".into());
    ctx.map.entities.push(mara);

    let program = VM::default()
        .prepare_str("fn event(event, value) { }")
        .unwrap();
    let args = [
        VMValue::from_string("intent"),
        VMValue::new_with_string(1.0, 1.0, 0.0, "use"),
    ];
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );

    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        offered.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text.starts_with("Warden Mara keeps watch") && role == "dialog"
        )),
        "Mara has to greet the player, got {offered:?}"
    );
    let offered_indices: Vec<u32> = offered
        .iter()
        .filter_map(|message| match message {
            RegionMessage::MultipleChoice(choice) => Some(
                choice
                    .choices
                    .iter()
                    .filter_map(|choice| match choice {
                        Choice::NodeChoice(choice) => Some(choice.index),
                        _ => None,
                    })
                    .collect::<Vec<u32>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        offered_indices,
        vec![0, 5],
        "only the choices whose conditions hold may be offered: the bell and Leave"
    );

    // The player asks about the bell; the conversation continues inside the
    // node, with no wires between the lines.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);

    let answered: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        answered.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text.contains("Three nights ago the dead") && role == "dialog"
        )),
        "the briefing has to follow the picked choice, got {answered:?}"
    );

    // Accepting leaves through the consequence port the graph wires to Set
    // Attribute and Message nodes.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);
    let mara = ctx.find_entity(2).expect("Warden Mara");
    assert!(
        mara.attributes.get_bool_default("barrow_active", false),
        "accepting the contract has to write the quest flag"
    );
    let finished: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        finished.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, _, text, role)
                if text.contains("Quest started") && role == "quest"
        )),
        "the quest message has to reach the player, got {finished:?}"
    );

    // Talking again offers the contract now that the quest is running.
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );
    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    let offered_indices: Vec<u32> = offered
        .iter()
        .filter_map(|message| match message {
            RegionMessage::MultipleChoice(choice) => Some(
                choice
                    .choices
                    .iter()
                    .filter_map(|choice| match choice {
                        Choice::NodeChoice(choice) => Some(choice.index),
                        _ => None,
                    })
                    .collect::<Vec<u32>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        offered_indices,
        vec![1, 5],
        "a running quest offers the contract instead of the introduction"
    );
}

#[test]
fn missing_graph_does_not_fall_back_and_item_events_keep_their_owner() {
    use crate::server::{
        message::RegionMessage, region_host::run_server_named_fn, regionctx::RegionCtx,
    };
    use crate::vm::{Execution, VM, VMValue};
    let mut ctx = RegionCtx::default();
    let template = Uuid::new_v4();
    let mut item = crate::Item::default();
    item.id = 9;
    item.creator_id = Uuid::new_v4();
    item.set_attribute("class_name", crate::Value::Str("Sign".into()));
    let owner = item.creator_id;
    ctx.map.items.push(item);
    ctx.curr_item_id = Some(9); // A deferred engine event may retain default Entity scope.
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(
            format!("behavior/item/{template}"),
            graph("startup", "Sign ready"),
        )]),
        items: HashMap::from([("Sign".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();
    let program = VM::default()
        .prepare_str("fn event(event, value) { say(\"legacy\"); }")
        .unwrap();
    let args = [VMValue::from_string("startup"), VMValue::zero()];
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );
    assert_eq!(
        ctx.event_observations.back().unwrap().owner,
        EventOwner::Item(owner)
    );
    let says: Vec<_> = receiver
        .try_iter()
        .filter_map(|msg| {
            if let RegionMessage::Say(_, e, i, text, _) = msg {
                Some((e, i, text))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(says, vec![(None, Some(9), "Sign ready".into())]);
    ctx.assets.node_behaviors.as_mut().unwrap().graphs.clear();
    ctx.node_behaviors = None;
    run_server_named_fn(
        &mut Execution::default(),
        "event",
        &args,
        &program,
        &mut ctx,
    );
    assert!(
        !receiver
            .try_iter()
            .any(|msg| matches!(msg, RegionMessage::Say(..)))
    );
}

#[test]
fn routine_entry_starts_random_walk_through_world_services() {
    struct Walking {
        started: bool,
    }
    impl WorldServices for Walking {
        fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
            Default::default()
        }
        fn say(&mut self, _: &Actor, _: String) {}
        fn random_walk(
            &mut self,
            _: &Actor,
            area: &str,
            distance: f32,
            speed: f32,
            pause: i32,
        ) -> Result<bool, String> {
            assert_eq!((area, distance, speed, pause), ("Office", 3., 1., 2));
            self.started = true;
            Ok(true)
        }
    }
    let mut doc = graph("startup", "unused");
    doc["nodes"][0]["definition"] = json!("routine");
    doc["nodes"][0]["rows"] = json!([]);
    doc["nodes"][1]["definition"] = json!("random_walk");
    doc["nodes"][1]["rows"] = json!([
        {"key":"area","value":{"Text":"Office"}},
        {"key":"distance","value":{"Number":{"value":3.}}},
        {"key":"speed","value":{"Number":{"value":1.}}},
        {"key":"pause","value":{"Number":{"value":2.}}}
    ]);
    let mut runtime = Runtime::default();
    let mut world = Walking { started: false };
    let handle = runtime
        .attach(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.send(handle, "routine", BTreeMap::new(), 0);
    runtime.update(&mut world);
    assert!(world.started);
    assert_eq!(runtime.traces.len(), 2);
    assert!(runtime.traces.iter().all(|t| t.error.is_none()));
    let snapshots = runtime.take_highlights();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].active.nodes.len(), 2);
    assert_eq!(snapshots[0].active.connections.len(), 1);
    // Clearing bounded history must not extinguish the running path.
    runtime.traces.clear();
    runtime.tick(handle, 1, &mut world);
    assert_eq!(runtime.highlights[&handle].active, snapshots[0].active);
    assert!(runtime.take_highlights().is_empty());
    // Removing the connection reevaluates Routine and drops the old branch.
    doc["connections"] = json!([]);
    runtime
        .replace_graph(
            handle,
            Arc::new(Registry::builtin().compile(&doc).unwrap()),
            &mut world,
        )
        .unwrap();
    runtime.update(&mut world);
    let switched = runtime.take_highlights();
    assert!(switched[0].active.nodes.is_empty());
    assert!(switched[0].recent.connections.is_empty());
    assert_eq!(switched[0].recent.nodes.len(), 1);
    runtime.despawn(handle);
    let removed = runtime.take_highlights();
    assert!(removed[0].active.nodes.is_empty());
    assert!(removed[0].recent.nodes.is_empty());
}

#[test]
fn live_graph_replacement_changes_next_event_without_replaying_effects() {
    let mut doc = graph("talk", "Hello");
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let actor = runtime
        .attach(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.send(actor, "talk", BTreeMap::new(), 0);
    runtime.update(&mut output);
    doc["nodes"][1]["rows"][0]["value"] = json!({"Text":"Updated"});
    runtime
        .replace_graph(
            actor,
            Arc::new(Registry::builtin().compile(&doc).unwrap()),
            &mut output,
        )
        .unwrap();
    assert_eq!(output.0, ["Hello"]);
    runtime.send(actor, "talk", BTreeMap::new(), 1);
    runtime.update(&mut output);
    assert_eq!(output.0, ["Hello", "Updated"]);
}
#[test]
fn moving_a_node_does_not_change_the_running_program() {
    let doc = graph("startup", "Hello");
    let mut moved = doc.clone();
    moved["nodes"][0]["position"] = json!([300., 400.]);
    moved["nodes"][0]["color"] = json!([1, 2, 3, 255]);
    assert!(Registry::same_program(&doc, &moved));
    moved["nodes"][1]["rows"][0]["value"] = json!({"Text":"Changed"});
    assert!(!Registry::same_program(&doc, &moved));
}

#[test]
fn connecting_a_running_routine_executes_the_new_branch() {
    let mut doc = graph("routine", "Walking");
    doc["nodes"][0]["definition"] = json!("routine");
    doc["nodes"][0]["rows"] = json!([]);
    let connections = doc["connections"].take();
    doc["connections"] = json!([]);
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let actor = runtime
        .attach(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.send(actor, "routine", BTreeMap::new(), 0);
    runtime.update(&mut output);
    assert!(output.0.is_empty());
    doc["connections"] = connections;
    runtime
        .replace_graph(
            actor,
            Arc::new(Registry::builtin().compile(&doc).unwrap()),
            &mut output,
        )
        .unwrap();
    runtime.update(&mut output);
    assert_eq!(output.0, ["Walking"]);
}

#[test]
fn correcting_a_failed_node_retries_it_without_replaying_startup() {
    let mut doc = graph("startup", "{event.missing}");
    let mut runtime = Runtime::default();
    let mut output = Output::default();
    let actor = runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.update(&mut output);
    assert!(runtime.traces.back().unwrap().error.is_some());
    runtime.traces.clear();
    doc["nodes"][1]["rows"][0]["value"] = json!({"Text":"Recovered"});
    runtime
        .replace_graph(
            actor,
            Arc::new(Registry::builtin().compile(&doc).unwrap()),
            &mut output,
        )
        .unwrap();
    runtime.update(&mut output);
    assert_eq!(output.0, ["Recovered"]);
    assert_eq!(runtime.traces.len(), 1);
    assert!(runtime.traces[0].error.is_none());
    assert!(runtime.failed.is_empty());
}

#[test]
fn time_ranges_cover_boundaries_midnight_and_invalid_input() {
    let time = |h, m| theframework::prelude::TheTime::new_time(h, m).unwrap();
    assert!(!time_range_contains("14:00", "16:00", time(13, 59)).unwrap());
    assert!(time_range_contains("14:00", "16:00", time(14, 0)).unwrap());
    assert!(!time_range_contains("14:00", "16:00", time(16, 0)).unwrap());
    assert!(time_range_contains("22:00", "06:00", time(0, 30)).unwrap());
    assert!(!time_range_contains("22:00", "06:00", time(6, 0)).unwrap());
    assert!(time_range_contains("00:00", "00:00", time(12, 0)).unwrap());
    assert!(time_range_contains("25:00", "06:00", time(12, 0)).is_err());
}

#[test]
fn routine_rechecks_only_when_time_condition_changes() {
    struct Clock {
        time: theframework::prelude::TheTime,
        output: Vec<String>,
        cancelled: usize,
    }
    impl WorldServices for Clock {
        fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
            self.time
        }
        fn say(&mut self, _: &Actor, text: String) {
            self.output.push(text);
        }
        fn cancel_activity(&mut self, _: &Actor) {
            self.cancelled += 1;
        }
    }
    let mut doc = graph("routine", "Working");
    doc["nodes"][0]["definition"] = json!("routine");
    doc["nodes"][0]["rows"] = json!([]);
    let input = Uuid::new_v4();
    let output = Uuid::new_v4();
    let range = json!({"id":Uuid::new_v4(),"definition":"time_range",
        "rows":[{"key":"start","value":{"Text":"14:00"}},{"key":"end","value":{"Text":"16:00"}}],
        "ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"},{"id":output,"key":"inside","direction":"Output","kind":"flow"}]});
    let target = doc["connections"][0]["to"].clone();
    doc["connections"][0]["to"] = json!(input);
    doc["connections"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":Uuid::new_v4(),"from":output,"to":target}));
    doc["nodes"].as_array_mut().unwrap().push(range);
    let mut world = Clock {
        time: theframework::prelude::TheTime::new_time(13, 59).unwrap(),
        output: vec![],
        cancelled: 0,
    };
    let mut runtime = Runtime::default();
    let actor = runtime
        .attach(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.send(actor, "routine", BTreeMap::new(), 0);
    runtime.update(&mut world);
    assert!(world.output.is_empty());
    world.time.hours = 14;
    world.time.minutes = 0;
    runtime.tick(actor, 1, &mut world);
    runtime.tick(actor, 2, &mut world);
    assert_eq!(world.output, ["Working"]);
    world.time.hours = 16;
    runtime.tick(actor, 3, &mut world);
    assert_eq!(world.cancelled, 3);
    assert_eq!(world.output, ["Working"]);
}

#[test]
fn go_to_waits_for_completion_and_routes_failures() {
    struct Navigation {
        result: Option<Result<(), String>>,
        output: Vec<String>,
    }
    impl WorldServices for Navigation {
        fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
            Default::default()
        }
        fn say(&mut self, _: &Actor, text: String) {
            self.output.push(text);
        }
        fn go_to(&mut self, _: &Actor, _: &str, _: f32) -> Result<(), String> {
            Ok(())
        }
        fn go_to_result(&mut self, _: &Actor) -> Option<Result<(), String>> {
            self.result.take()
        }
    }
    let mut doc = graph("startup", "Done");
    let input = Uuid::new_v4();
    let done = Uuid::new_v4();
    let error = Uuid::new_v4();
    let target = doc["connections"][0]["to"].clone();
    doc["connections"][0]["to"] = json!(input);
    doc["connections"].as_array_mut().unwrap().extend([
        json!({"id":Uuid::new_v4(),"from":done,"to":target}),
        json!({"id":Uuid::new_v4(),"from":error,"to":target}),
    ]);
    doc["nodes"].as_array_mut().unwrap().push(json!({"id":Uuid::new_v4(),"definition":"go_to",
        "rows":[{"key":"destination","value":{"Text":"Garden"}},{"key":"speed","value":{"Number":{"value":1.0}}}],
        "ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"},{"id":done,"key":"done","direction":"Output","kind":"flow"},{"id":error,"key":"error","direction":"Output","kind":"flow"}]}));
    let mut runtime = Runtime::default();
    let mut world = Navigation {
        result: None,
        output: vec![],
    };
    let actor = runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.update(&mut world);
    runtime.tick(actor, 1, &mut world);
    assert!(world.output.is_empty());
    assert!(runtime.traces.back().unwrap().running);
    world.result = Some(Ok(()));
    runtime.tick(actor, 2, &mut world);
    runtime.tick(actor, 3, &mut world);
    assert_eq!(world.output, ["Done"]);
    runtime.send(actor, "startup", BTreeMap::new(), 4);
    runtime.update(&mut world);
    world.result = Some(Err("Blocked".into()));
    runtime.tick(actor, 5, &mut world);
    assert_eq!(world.output, ["Done", "Done"]);
    assert!(
        runtime
            .traces
            .iter()
            .any(|trace| trace.error.as_deref() == Some("Blocked"))
    );
}

#[test]
fn use_action_routes_event_subject_and_refusal_to_failed() {
    #[derive(Default)]
    struct Actions(Vec<(String, Option<u32>)>);
    impl WorldServices for Actions {
        fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
            Default::default()
        }
        fn say(&mut self, _: &Actor, _: String) {}
        fn use_action(
            &mut self,
            _: &Actor,
            action: &str,
            subject: Option<u32>,
        ) -> Result<(), String> {
            self.0.push((action.into(), subject));
            Err("Out of range".into())
        }
    }
    let mut doc = graph("intent", "");
    doc["nodes"][1]["definition"] = json!("use_action");
    doc["nodes"][1]["rows"] =
        json!([{"key":"action","value":{"Choice":{"options":["basic_attack"],"selected":0}}}]);
    let mut runtime = Runtime::default();
    let actor = runtime
        .spawn(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    let mut actions = Actions::default();
    for tick in 1..=2 {
        runtime.send(
            actor,
            "intent",
            BTreeMap::from([("subject".into(), EventField::Entity(42))]),
            tick,
        );
        runtime.update(&mut actions);
    }
    assert_eq!(actions.0, vec![("basic_attack".into(), Some(42)); 2]);
    assert!(
        runtime
            .traces
            .iter()
            .any(|trace| trace.error.as_deref() == Some("Out of range"))
    );
}

#[derive(Default)]
struct Probe {
    said: Vec<String>,
    states: Vec<String>,
    attributes: Vec<(String, EventAttribute)>,
    messages: Vec<(String, String)>,
    teleports: Vec<(String, String)>,
    values: BTreeMap<String, String>,
}
impl WorldServices for Probe {
    fn time(&self, _: &Actor) -> theframework::prelude::TheTime {
        Default::default()
    }
    fn say(&mut self, _: &Actor, text: String) {
        self.said.push(text);
    }
    fn set_attribute(
        &mut self,
        _: &Actor,
        attribute: &str,
        value: EventAttribute,
    ) -> Result<(), String> {
        self.attributes.push((attribute.into(), value));
        Ok(())
    }
    fn attribute_display(&self, _: &Actor, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
    fn message(&mut self, _: &Actor, text: String, role: &str) -> Result<(), String> {
        self.messages.push((text, role.into()));
        Ok(())
    }
    fn set_state(&mut self, _: &Actor, state: &str) -> Result<(), String> {
        self.states.push(state.into());
        Ok(())
    }
    fn teleport(&mut self, _: &Actor, area: &str, sector: &str) -> Result<(), String> {
        self.teleports.push((area.into(), sector.into()));
        Ok(())
    }
}
fn node_rows(rows: Vec<(&str, Value)>) -> Value {
    Value::Array(
        rows.into_iter()
            .map(|(key, value)| json!({"key": key, "value": value}))
            .collect(),
    )
}
/// `event -> node`, used to exercise one action node against a Probe.
fn action_graph(event: &str, definition: &str, rows: Vec<(&str, Value)>) -> Value {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let out = Uuid::new_v4();
    let input = Uuid::new_v4();
    json!({"version":1,"nodes":[
        {"id":a,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":event}}}],"ports":[{"id":out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":b,"definition":definition,"rows":node_rows(rows),"ports":[{"id":input,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[{"id":Uuid::new_v4(),"from":out,"to":input}]})
}
fn run_action(
    definition: &str,
    rows: Vec<(&str, Value)>,
    probe: &mut Probe,
) -> Vec<EventObservation> {
    let mut runtime = Runtime::default();
    let id = Uuid::new_v4();
    runtime
        .spawn(
            id,
            Uuid::new_v4(),
            1,
            Some(
                Registry::builtin()
                    .compile(&action_graph("startup", definition, rows))
                    .unwrap(),
            ),
        )
        .unwrap();
    runtime.update(probe);
    runtime
        .traces
        .iter()
        .map(|trace| trace.event.clone())
        .collect()
}
#[test]
fn set_attribute_node_writes_typed_values() {
    let mut probe = Probe::default();
    run_action(
        "set_attribute",
        vec![
            ("attribute", json!({"Text": "mode"})),
            ("value_kind", json!({"Choice": {"selected": 0}})),
            ("value", json!({"Text": "active"})),
        ],
        &mut probe,
    );
    run_action(
        "set_attribute",
        vec![
            ("attribute", json!({"Text": "visible"})),
            ("value_kind", json!({"Choice": {"selected": 2}})),
            ("value", json!({"Text": "true"})),
        ],
        &mut probe,
    );
    assert_eq!(
        probe.attributes,
        [
            ("mode".into(), EventAttribute::Text("active".into())),
            ("visible".into(), EventAttribute::Bool(true)),
        ]
    );
}
#[test]
fn message_node_records_text_and_role() {
    let mut probe = Probe::default();
    run_action(
        "message",
        vec![
            ("text", json!({"Text": "You died."})),
            ("role", json!({"Text": "severe"})),
        ],
        &mut probe,
    );
    assert_eq!(probe.messages, [("You died.".into(), "severe".into())]);
}
#[test]
fn teleport_and_attribute_filter_nodes_use_services() {
    let mut probe = Probe::default();
    probe
        .values
        .insert("barrow_warden_slain".into(), "false".into());
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let out = Uuid::new_v4();
    let filter_in = Uuid::new_v4();
    let filter_match = Uuid::new_v4();
    let tp_in = Uuid::new_v4();
    let doc = json!({"version":1,"nodes":[
        {"id":a,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"startup"}}}],"ports":[{"id":out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":b,"definition":"filter","rows":node_rows(vec![("field", json!({"Text":"attr.barrow_warden_slain"})),("operator", json!({"Choice":{"selected":1}})),("expected", json!({"Text":"true"}))]),"ports":[{"id":filter_in,"key":"in","direction":"Input","kind":"flow"},{"id":filter_match,"key":"match","direction":"Output","kind":"flow"}]},
        {"id":c,"definition":"teleport","rows":node_rows(vec![("area", json!({"Text":"Start"})),("sector", json!({"Text":"Harbor"}))]),"ports":[{"id":tp_in,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[{"id":Uuid::new_v4(),"from":out,"to":filter_in},{"id":Uuid::new_v4(),"from":filter_match,"to":tp_in}]});
    let mut runtime = Runtime::default();
    let id = Uuid::new_v4();
    runtime
        .spawn(
            id,
            Uuid::new_v4(),
            1,
            Some(Registry::builtin().compile(&doc).unwrap()),
        )
        .unwrap();
    runtime.update(&mut probe);
    assert_eq!(probe.teleports, [("Start".into(), "Harbor".into())]);
}

#[test]
fn state_node_sets_a_life_state() {
    let mut probe = Probe::default();
    run_action(
        "state",
        vec![("state", json!({"Choice": {"selected": 3}}))],
        &mut probe,
    );
    assert_eq!(probe.states, ["Unconscious"]);
}

#[test]
fn on_enter_area_routes_match_and_no_match() {
    let entry = Uuid::new_v4();
    let arrived = Uuid::new_v4();
    let elsewhere = Uuid::new_v4();
    let matched = Uuid::new_v4();
    let unmatched = Uuid::new_v4();
    let arrived_in = Uuid::new_v4();
    let elsewhere_in = Uuid::new_v4();
    let doc = json!({"version":1,"nodes":[
        {"id":entry,"definition":"on_enter_area","rows":[{"key":"area","value":{"Text":"Garden"}}],"ports":[
            {"id":matched,"key":"match","direction":"Output","kind":"flow"},
            {"id":unmatched,"key":"no_match","direction":"Output","kind":"flow"}]},
        {"id":arrived,"definition":"say","rows":[{"key":"text","value":{"Text":"Arrived"}}],"ports":[
            {"id":arrived_in,"key":"in","direction":"Input","kind":"flow"}]},
        {"id":elsewhere,"definition":"say","rows":[{"key":"text","value":{"Text":"Elsewhere"}}],"ports":[
            {"id":elsewhere_in,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[
        {"id":Uuid::new_v4(),"from":matched,"to":arrived_in},
        {"id":Uuid::new_v4(),"from":unmatched,"to":elsewhere_in}
    ]});
    let plan = Registry::builtin().compile(&doc).expect("graph compiles");
    let mut runtime = Runtime::default();
    let mut probe = Probe::default();
    let id = Uuid::new_v4();
    let map = Uuid::new_v4();
    let handle = runtime.spawn(id, map, 1, Some(plan)).unwrap();
    let enter = |runtime: &mut Runtime, probe: &mut Probe, area: &str| {
        let mut fields = BTreeMap::new();
        fields.insert("area".to_string(), EventField::Text(area.into()));
        assert!(runtime.send_observation(
            handle,
            EventObservation {
                map,
                owner: EventOwner::Entity(id),
                name: "entered".into(),
                tick: 1,
                fields,
            },
        ));
        runtime.update(probe);
    };

    enter(&mut runtime, &mut probe, "Garden");
    assert_eq!(probe.said, ["Arrived"]);
    enter(&mut runtime, &mut probe, "Cellar");
    assert_eq!(probe.said, ["Arrived", "Elsewhere"]);
}

#[test]
fn on_area_routes_entered_and_left_for_the_place_that_owns_the_graph() {
    let on_area = Uuid::new_v4();
    let inside = Uuid::new_v4();
    let outside = Uuid::new_v4();
    let entered_port = Uuid::new_v4();
    let left_port = Uuid::new_v4();
    let inside_in = Uuid::new_v4();
    let outside_in = Uuid::new_v4();
    let doc = json!({"version":1,"nodes":[
        {"id":on_area,"definition":"on_area","rows":[],"ports":[
            {"id":entered_port,"key":"player_entered","direction":"Output","kind":"flow"},
            {"id":left_port,"key":"npc_left","direction":"Output","kind":"flow"}]},
        {"id":inside,"definition":"say","rows":[{"key":"text","value":{"Text":"Hello"}}],"ports":[
            {"id":inside_in,"key":"in","direction":"Input","kind":"flow"}]},
        {"id":outside,"definition":"say","rows":[{"key":"text","value":{"Text":"Farewell"}}],"ports":[
            {"id":outside_in,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[
        {"id":Uuid::new_v4(),"from":entered_port,"to":inside_in},
        {"id":Uuid::new_v4(),"from":left_port,"to":outside_in}
    ]});
    let plan = Registry::builtin().compile(&doc).expect("graph compiles");
    let mut runtime = Runtime::default();
    let mut probe = Probe::default();
    let area = Uuid::new_v4();
    let entrant = 7;
    let map = Uuid::new_v4();
    let handle = runtime.spawn(area, map, entrant, Some(plan)).unwrap();
    let transition = |runtime: &mut Runtime, probe: &mut Probe, name: &str| {
        let mut fields = BTreeMap::new();
        fields.insert("entity".to_string(), EventField::Entity(entrant));
        assert!(runtime.send_observation(
            handle,
            EventObservation {
                map,
                owner: EventOwner::Area(area),
                name: name.into(),
                tick: 1,
                fields,
            },
        ));
        runtime.update(probe);
    };

    probe.values.insert("player".into(), "true".into());
    transition(&mut runtime, &mut probe, "entered");
    assert_eq!(probe.said, ["Hello"]);
    probe.values.remove("player");
    transition(&mut runtime, &mut probe, "left");
    assert_eq!(probe.said, ["Hello", "Farewell"]);
}

#[test]
fn on_area_routes_who_and_when() {
    let on_area = Uuid::new_v4();
    let hello = Uuid::new_v4();
    let bye = Uuid::new_v4();
    let player_entered = Uuid::new_v4();
    let npc_entered = Uuid::new_v4();
    let player_left = Uuid::new_v4();
    let npc_left = Uuid::new_v4();
    let hello_in = Uuid::new_v4();
    let bye_in = Uuid::new_v4();
    let doc = json!({"version":1,"nodes":[
        {"id":on_area,"definition":"on_area","rows":[],"ports":[
            {"id":player_entered,"key":"player_entered","direction":"Output","kind":"flow"},
            {"id":npc_entered,"key":"npc_entered","direction":"Output","kind":"flow"},
            {"id":player_left,"key":"player_left","direction":"Output","kind":"flow"},
            {"id":npc_left,"key":"npc_left","direction":"Output","kind":"flow"}]},
        {"id":hello,"definition":"say","rows":[{"key":"text","value":{"Text":"Player here"}}],"ports":[
            {"id":hello_in,"key":"in","direction":"Input","kind":"flow"}]},
        {"id":bye,"definition":"say","rows":[{"key":"text","value":{"Text":"NPC left"}}],"ports":[
            {"id":bye_in,"key":"in","direction":"Input","kind":"flow"}]}
    ],"connections":[
        {"id":Uuid::new_v4(),"from":player_entered,"to":hello_in},
        {"id":Uuid::new_v4(),"from":npc_left,"to":bye_in}
    ]});
    let plan = Registry::builtin().compile(&doc).expect("graph compiles");
    let mut runtime = Runtime::default();
    let mut probe = Probe::default();
    let area = Uuid::new_v4();
    let map = Uuid::new_v4();
    let handle = runtime.spawn(area, map, 7, Some(plan)).unwrap();
    let transition = |runtime: &mut Runtime, probe: &mut Probe, name: &str| {
        let mut fields = BTreeMap::new();
        fields.insert("entity".to_string(), EventField::Entity(7));
        assert!(runtime.send_observation(
            handle,
            EventObservation {
                map,
                owner: EventOwner::Area(area),
                name: name.into(),
                tick: 1,
                fields,
            },
        ));
        runtime.update(probe);
    };

    // An NPC entering takes no wired terminal.
    transition(&mut runtime, &mut probe, "entered");
    assert!(probe.said.is_empty());

    probe.values.insert("player".into(), "true".into());
    transition(&mut runtime, &mut probe, "entered");
    assert_eq!(probe.said, ["Player here"]);

    probe.values.remove("player");
    transition(&mut runtime, &mut probe, "left");
    assert_eq!(probe.said, ["Player here", "NPC left"]);
}

#[test]
fn an_instance_graph_layers_over_its_template() {
    let event_row =
        |data: &str| json!([{"key":"event","value":{"Custom":{"kind":"event","data":data}}}]);
    let template = json!({"version":1,"nodes":[
        {"id":"routine-1","definition":"routine","rows":[],"ports":[]},
        {"id":"start-1","definition":"event","rows":event_row("startup"),"ports":[]},
        {"id":"death-1","definition":"event","rows":event_row("death"),"ports":[]}
    ],"connections":[{"id":"c1","from":"x","to":"y"}]});
    let instance = json!({"version":1,"nodes":[
        {"id":"start-2","definition":"event","rows":event_row("startup"),"ports":[]},
        {"id":"name-2","definition":"set_attribute","rows":[],"ports":[]}
    ],"connections":[{"id":"c2","from":"p","to":"q"}]});
    // The runtime drops the template entries for the events the instance answers.
    let replaced: std::collections::HashSet<String> = ["start-1".to_string()].into_iter().collect();
    let merged = super::region::compose_graphs(&template, &instance, &replaced);

    let ids: Vec<&str> = merged["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"routine-1"), "the template routine survives");
    assert!(
        ids.contains(&"death-1"),
        "the template death chain survives"
    );
    assert!(!ids.contains(&"start-1"), "the overridden entry is dropped");
    assert!(ids.contains(&"start-2") && ids.contains(&"name-2"));
    assert_eq!(merged["connections"].as_array().unwrap().len(), 2);
}

#[test]
fn composition_keeps_template_chains_and_overrides_the_instance_ones() {
    let routine_id = Uuid::new_v4();
    let template_start = Uuid::new_v4();
    let instance_start = Uuid::new_v4();
    let template = json!({"version":1,"nodes":[
        {"id":routine_id,"definition":"routine","rows":[],"ports":[
            {"id":Uuid::new_v4(),"key":"out","direction":"Output","kind":"flow"}]},
        {"id":template_start,"definition":"event","rows":[
            {"key":"event","value":{"Custom":{"kind":"event","data":"startup"}}}],"ports":[
            {"id":Uuid::new_v4(),"key":"out","direction":"Output","kind":"flow"}]}
    ],"connections":[]});
    let instance = json!({"version":1,"nodes":[
        {"id":instance_start,"definition":"event","rows":[
            {"key":"event","value":{"Custom":{"kind":"event","data":"startup"}}}],"ports":[
            {"id":Uuid::new_v4(),"key":"out","direction":"Output","kind":"flow"}]}
    ],"connections":[]});

    // The same resolution `owner_graph` performs at runtime.
    let instance_plan = Registry::builtin().compile(&instance).unwrap();
    let template_plan = Registry::builtin().compile(&template).unwrap();
    let owned: std::collections::HashSet<String> = instance_plan.events.keys().cloned().collect();
    let replaced: std::collections::HashSet<String> = template_plan
        .events
        .iter()
        .filter(|(event, _)| owned.contains(*event))
        .flat_map(|(_, nodes)| nodes.iter().map(|node| node.to_string()))
        .collect();
    let merged = super::region::compose_graphs(&template, &instance, &replaced);
    let plan = Registry::builtin().compile(&merged).unwrap();

    // The template chain the instance never mentions still runs.
    assert!(plan.events.contains_key("routine"), "template routine kept");
    // And the instance owns the event it does define.
    let startup = plan.events.get("startup").expect("startup entry");
    assert!(startup.contains(&instance_start));
    assert!(!startup.contains(&template_start));
}

/// A Talk graph: an intent trigger into a Talk node whose `out:0` port runs a Set
/// Attribute node and then a Message node. That chain is a choice's consequence
/// now, so the graph — not a script — does the work.
fn talk_graph(conversation: serde_json::Value) -> (serde_json::Value, Uuid) {
    let event = Uuid::new_v4();
    let talk = Uuid::new_v4();
    let set = Uuid::new_v4();
    let message = Uuid::new_v4();
    let event_out = Uuid::new_v4();
    let talk_in = Uuid::new_v4();
    let talk_out0 = Uuid::new_v4();
    let set_in = Uuid::new_v4();
    let set_out = Uuid::new_v4();
    let message_in = Uuid::new_v4();
    let message_out = Uuid::new_v4();

    let mut ports = vec![
        json!({"id":talk_in,"key":"in","direction":"Input","kind":"flow"}),
        json!({"id":Uuid::new_v4(),"key":"done","direction":"Output","kind":"flow"}),
    ];
    for slot in 0..CONSEQUENCE_SLOTS {
        let id = if slot == 0 { talk_out0 } else { Uuid::new_v4() };
        ports.push(json!({"id":id,"key":format!("out:{slot}"),"direction":"Output","kind":"flow"}));
    }

    let document = json!({"version":1,"nodes":[
        {"id":event,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"intent"}}}],"ports":[{"id":event_out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":talk,"definition":"talk","rows":[{"key":"conversation","value":{"Custom":{"kind":"conversation","data":conversation}}}],"ports":ports},
        {"id":set,"definition":"set_attribute","rows":node_rows(vec![("attribute", json!({"Text":"barrow_active"})),("value_kind", json!({"Choice":{"options":["Text","Number","Boolean"],"selected":2}})),("value", json!({"Text":"true"}))]),"ports":[{"id":set_in,"key":"in","direction":"Input","kind":"flow"},{"id":set_out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":message,"definition":"message","rows":node_rows(vec![("text", json!({"Text":"Quest started."})),("role", json!({"Text":"quest"}))]),"ports":[{"id":message_in,"key":"in","direction":"Input","kind":"flow"},{"id":message_out,"key":"out","direction":"Output","kind":"flow"}]}
    ],"connections":[
        {"id":Uuid::new_v4(),"from":event_out,"to":talk_in},
        {"id":Uuid::new_v4(),"from":talk_out0,"to":set_in},
        {"id":Uuid::new_v4(),"from":set_out,"to":message_in}
    ]});
    (document, talk)
}

/// A Talk graph whose `out:0` chain runs a Set Attribute node and then hands the
/// flow back to the Talk node, so an answer can do work and keep talking.
fn talk_resume_graph(conversation: serde_json::Value) -> serde_json::Value {
    let event = Uuid::new_v4();
    let talk = Uuid::new_v4();
    let set = Uuid::new_v4();
    let event_out = Uuid::new_v4();
    let talk_in = Uuid::new_v4();
    let talk_out0 = Uuid::new_v4();
    let set_in = Uuid::new_v4();
    let set_out = Uuid::new_v4();
    let mut ports = vec![
        json!({"id":talk_in,"key":"in","direction":"Input","kind":"flow"}),
        json!({"id":Uuid::new_v4(),"key":"done","direction":"Output","kind":"flow"}),
    ];
    for slot in 0..CONSEQUENCE_SLOTS {
        let id = if slot == 0 { talk_out0 } else { Uuid::new_v4() };
        ports.push(json!({"id":id,"key":format!("out:{slot}"),"direction":"Output","kind":"flow"}));
    }
    json!({"version":1,"nodes":[
        {"id":event,"definition":"event","rows":[{"key":"event","value":{"Custom":{"kind":"event","data":"intent"}}}],"ports":[{"id":event_out,"key":"out","direction":"Output","kind":"flow"}]},
        {"id":talk,"definition":"talk","rows":[{"key":"conversation","value":{"Custom":{"kind":"conversation","data":conversation}}}],"ports":ports},
        {"id":set,"definition":"set_attribute","rows":node_rows(vec![("attribute", json!({"Text":"handed_in"})),("value_kind", json!({"Choice":{"options":["Text","Number","Boolean"],"selected":2}})),("value", json!({"Text":"true"}))]),"ports":[{"id":set_in,"key":"in","direction":"Input","kind":"flow"},{"id":set_out,"key":"out","direction":"Output","kind":"flow"}]}
    ],"connections":[
        {"id":Uuid::new_v4(),"from":event_out,"to":talk_in},
        {"id":Uuid::new_v4(),"from":talk_out0,"to":set_in},
        {"id":Uuid::new_v4(),"from":set_out,"to":talk_in}
    ]})
}

/// Set up the speaker/listener pair the dialogue tests use, and run the intent.
fn run_intent(ctx: &mut crate::server::regionctx::RegionCtx) {
    let program = crate::vm::VM::default()
        .prepare_str("fn event(event, value) { }")
        .unwrap();
    let args = [
        crate::vm::VMValue::from_string("intent"),
        crate::vm::VMValue::new_with_string(1.0, 1.0, 0.0, "use"),
    ];
    crate::server::region_host::run_server_named_fn(
        &mut crate::vm::Execution::default(),
        "event",
        &args,
        &program,
        ctx,
    );
}

/// The speaker's class name is what selects its behavior graph, so a caller
/// testing a shipped character has to name that character here.
fn talk_speaker_and_listener(class_name: &str) -> (crate::Entity, crate::Entity) {
    let mut listener = crate::Entity::new();
    listener.id = 1;
    listener.creator_id = Uuid::new_v4();
    listener.set_attribute("class_name", crate::Value::Str("Player".into()));
    let mut speaker = crate::Entity::new();
    speaker.id = 2;
    speaker.creator_id = Uuid::new_v4();
    speaker.set_attribute("class_name", crate::Value::Str(class_name.into()));
    (listener, speaker)
}

/// A whole conversation runs from one node: the conditions filter the answers,
/// a jump continues in place and needs no wire, and the one answer that does
/// something leaves through its consequence port into real nodes.
#[test]
fn talk_node_runs_a_conversation_from_one_node() {
    use crate::server::{message::RegionMessage, regionctx::RegionCtx};

    let conversation = json!({
        "entry": "greeting",
        "steps": [
            {"name": "greeting", "text": "Well met.",
             "choices": [
                {"label": "Ask about the bell", "condition": "not self.barrow_active", "then": {"Go": {"step": "briefing"}}},
                {"label": "Leave"}
             ]},
            {"name": "briefing", "text": "Three nights ago the dead opened the barrow.",
             "choices": [
                {"label": "Accept", "then": {"Out": {"slot": 0}}}
             ]}
        ]
    });
    let (document, _talk) = talk_graph(conversation);
    let template = Uuid::new_v4();

    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), document)]),
        characters: HashMap::from([("Warden Mara".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();
    let (listener, speaker) = talk_speaker_and_listener("Warden Mara");
    ctx.map.entities.push(listener);
    ctx.entity_classes.insert(2, "Warden Mara".into());
    ctx.map.entities.push(speaker);

    run_intent(&mut ctx);

    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        offered.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text == "Well met." && role == "dialog"
        )),
        "the first step has to reach the player, got {offered:?}"
    );
    let offered_count = offered
        .iter()
        .filter_map(|message| match message {
            RegionMessage::MultipleChoice(choice) => Some(
                choice
                    .choices
                    .iter()
                    .filter(|entry| matches!(entry, crate::server::message::Choice::NodeChoice(_)))
                    .count(),
            ),
            _ => None,
        })
        .next()
        .unwrap_or(0);
    assert_eq!(offered_count, 2, "both answers are offered at first");

    // The jump stays inside the node: no wire, and no port fired.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);
    assert!(
        ctx.talk_steps.contains_key(&2),
        "the conversation parks on the step it jumped to"
    );
    let speaker = ctx.find_entity(2).expect("speaker");
    assert!(
        !speaker.attributes.get_bool_default("barrow_active", false),
        "a jump must not run the consequence chain"
    );
    let replayed: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        replayed.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text.starts_with("Three nights ago") && role == "dialog"
        )),
        "the jump has to show the next step, got {replayed:?}"
    );

    // Accepting leaves through out:0, so the wired nodes do the work.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);
    let speaker = ctx.find_entity(2).expect("speaker");
    assert!(
        speaker.attributes.get_bool_default("barrow_active", false),
        "the wired Set Attribute node has to run"
    );
    let finished: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        finished.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, _, text, role)
                if text.contains("Quest started") && role == "quest"
        )),
        "the wired Message node has to reach the player, got {finished:?}"
    );
    assert!(
        ctx.talk_steps.is_empty(),
        "the conversation clears its step when it ends"
    );

    // Talking again starts over, and the condition now hides the first answer.
    run_intent(&mut ctx);
    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    let choices = offered
        .iter()
        .filter_map(|message| match message {
            crate::server::message::RegionMessage::MultipleChoice(choice) => Some(
                choice
                    .choices
                    .iter()
                    .filter(|entry| matches!(entry, crate::server::message::Choice::NodeChoice(_)))
                    .count(),
            ),
            _ => None,
        })
        .last()
        .unwrap_or(0);
    assert_eq!(
        choices, 1,
        "only the choice whose condition holds may be offered, got {offered:?}"
    );
}

/// An answer can leave through a port, let the wired nodes run, and hand the
/// conversation back at another step — the `set ...; go ...` shape, as graph.
#[test]
fn a_consequence_can_hand_the_conversation_back_at_another_step() {
    use crate::server::{message::RegionMessage, regionctx::RegionCtx};

    let conversation = json!({
        "entry": "greeting",
        "steps": [
            {"name": "greeting", "text": "Well met.",
             "choices": [
                {"label": "Hand in the sigil", "then": {"Out": {"slot": 0, "resume": "thanks"}}},
                {"label": "Leave"}
             ]},
            {"name": "thanks", "text": "Mara nods. Keep it.",
             "choices": [{"label": "Leave"}]}
        ]
    });
    let document = talk_resume_graph(conversation);
    let template = Uuid::new_v4();

    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), document)]),
        characters: HashMap::from([("Warden Mara".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();
    let (listener, speaker) = talk_speaker_and_listener("Warden Mara");
    ctx.map.entities.push(listener);
    ctx.entity_classes.insert(2, "Warden Mara".into());
    ctx.map.entities.push(speaker);

    run_intent(&mut ctx);
    let _ = receiver.try_iter().count();

    // The answer leaves through out:0, the wired node runs, and the flow returns
    // to the Talk node, which continues at `thanks`.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);

    let speaker = ctx.find_entity(2).expect("speaker");
    assert!(
        speaker.attributes.get_bool_default("handed_in", false),
        "the wired consequence has to run"
    );
    assert_eq!(
        ctx.talk_steps.get(&2).map(String::as_str),
        Some("thanks"),
        "the conversation has to continue at the resume step"
    );
    let answered: Vec<RegionMessage> = receiver.try_iter().collect();
    assert!(
        answered.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, 1, text, role)
                if text == "Mara nods. Keep it." && role == "dialog"
        )),
        "the resume step has to be shown, got {answered:?}"
    );
}

/// Conditions reach the inventory, so a conversation can ask for the sigil
/// without an Inventory Has node.
#[test]
fn talk_conditions_read_the_inventory() {
    use crate::server::{message::RegionMessage, regionctx::RegionCtx};

    let conversation = json!({
        "entry": "hand_in",
        "steps": [
            {"name": "hand_in", "text": "Do you have it?",
             "choices": [
                {"label": "Here it is", "condition": "has(\"Grave Sigil\")", "then": {"Out": {"slot": 0}}},
                {"label": "Not yet", "condition": "not has(\"Grave Sigil\")"}
             ]}
        ]
    });
    let (document, _talk) = talk_graph(conversation);
    let template = Uuid::new_v4();

    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), document)]),
        characters: HashMap::from([("Warden Mara".into(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();
    let (mut listener, speaker) = talk_speaker_and_listener("Warden Mara");
    listener.inventory.resize(4, None);
    ctx.map.entities.push(listener);
    ctx.entity_classes.insert(2, "Warden Mara".into());
    ctx.map.entities.push(speaker);

    // Empty handed: only the "not yet" line is offered.
    run_intent(&mut ctx);
    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    let labels: Vec<String> = offered
        .iter()
        .filter_map(|message| match message {
            RegionMessage::MultipleChoice(choice) => Some(choice.choices.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|entry| match entry {
            crate::server::message::Choice::NodeChoice(node) => Some(node.label),
            _ => None,
        })
        .collect();
    assert_eq!(labels, vec!["Not yet".to_string()], "got {offered:?}");

    // With the sigil in hand the other line opens up.
    let mut sigil = crate::Item::new();
    sigil.set_attribute("name", crate::Value::Str("Grave Sigil".into()));
    ctx.map
        .entities
        .iter_mut()
        .find(|entity| entity.id == 1)
        .expect("listener")
        .add_item(sigil)
        .expect("free inventory slot");

    ctx.dialog_choices.insert(2, DialogChoiceMade::Dismissed);
    region::tick(&mut ctx);
    let _ = receiver.try_iter().count();

    run_intent(&mut ctx);
    let offered: Vec<RegionMessage> = receiver.try_iter().collect();
    let labels: Vec<String> = offered
        .iter()
        .filter_map(|message| match message {
            RegionMessage::MultipleChoice(choice) => Some(choice.choices.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|entry| match entry {
            crate::server::message::Choice::NodeChoice(node) => Some(node.label),
            _ => None,
        })
        .collect();
    assert!(
        labels.contains(&"Here it is".to_string()),
        "holding the sigil has to open the hand in choice, got {labels:?}"
    );
}

/// A conversation is checked when the node compiles, so a broken jump or a port
/// the node does not have shows as a diagnostic instead of a failed playtest.
#[test]
fn conversation_validation_reports_broken_trees() {
    use crate::server::nodes::conversation::{Choice, Conversation, Step, Then};

    let choice = |then: Then| Choice {
        label: "Answer".into(),
        condition: String::new(),
        then,
    };
    let conversation = |then: Then| Conversation {
        entry: "start".into(),
        steps: vec![Step {
            name: "start".into(),
            text: "Hello.".into(),
            choices: vec![choice(then)],
        }],
    };

    assert!(
        Conversation::default().validate().is_err(),
        "a conversation without steps is refused"
    );
    assert!(
        conversation(Then::Go {
            step: "ghost".into()
        })
        .validate()
        .is_err(),
        "a jump to an unknown step is refused"
    );
    assert!(
        conversation(Then::Out {
            slot: CONSEQUENCE_SLOTS,
            resume: String::new(),
        })
        .validate()
        .is_err(),
        "a port the node does not expose is refused"
    );
    assert!(
        conversation(Then::Out {
            slot: 0,
            resume: "ghost".into(),
        })
        .validate()
        .is_err(),
        "a resume step that does not exist is refused"
    );
    assert_eq!(
        conversation(Then::Out {
            slot: 0,
            resume: String::new(),
        })
        .validate(),
        Ok(()),
        "a consequence port inside the offered slots is fine"
    );
    assert_eq!(Conversation::starter().validate(), Ok(()));
}

/// Run one intent against a character's shipped graph, with a player nearby.
fn run_shipped_character(
    name: &str,
) -> Option<(
    crate::server::regionctx::RegionCtx,
    crossbeam_channel::Receiver<crate::server::message::RegionMessage>,
)> {
    use crate::server::regionctx::RegionCtx;

    let (graph, template) = hideout2d_character_graph(name)?;
    let mut ctx = RegionCtx::default();
    ctx.curr_entity_id = 2;
    ctx.assets.node_behaviors = Some(region::BehaviorAssets {
        graphs: HashMap::from([(format!("behavior/character/{template}"), graph)]),
        characters: HashMap::from([(name.to_string(), template)]),
        ..Default::default()
    });
    let (sender, receiver) = crossbeam_channel::unbounded();
    ctx.from_sender.set(sender).unwrap();
    let (listener, speaker) = talk_speaker_and_listener(name);
    ctx.map.entities.push(listener);
    ctx.entity_classes.insert(2, name.into());
    ctx.map.entities.push(speaker);
    run_intent(&mut ctx);
    Some((ctx, receiver))
}

/// Take everything the engine has sent since the last look. The channel drains,
/// so one snapshot has to answer every question about a step.
fn drain_messages(
    receiver: &crossbeam_channel::Receiver<crate::server::message::RegionMessage>,
) -> Vec<crate::server::message::RegionMessage> {
    receiver.try_iter().collect()
}

/// The choice indices offered in a snapshot, in order.
fn node_choice_indices(messages: &[crate::server::message::RegionMessage]) -> Vec<u32> {
    messages
        .iter()
        .filter_map(|message| match message {
            crate::server::message::RegionMessage::MultipleChoice(choice) => Some(
                choice
                    .choices
                    .iter()
                    .filter_map(|entry| match entry {
                        crate::server::message::Choice::NodeChoice(node) => Some(node.index),
                        _ => None,
                    })
                    .collect::<Vec<u32>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Dialogue lines sent to the player in a snapshot.
fn dialog_lines(messages: &[crate::server::message::RegionMessage]) -> Vec<String> {
    messages
        .iter()
        .filter_map(|message| match message {
            crate::server::message::RegionMessage::Message(_, Some(2), None, _, text, role)
                if role == "dialog" =>
            {
                Some(text.clone())
            }
            _ => None,
        })
        .collect()
}

/// Brother Corvin's shipped graph is one Talk node: a plain conversation tree
/// with no conditions and no state, walked by its choices.
#[test]
fn hideout2d_corvin_conversation_runs_from_the_shipped_graph() {
    let Some((mut ctx, receiver)) = run_shipped_character("Brother Corvin") else {
        return;
    };

    let offered = drain_messages(&receiver);
    assert_eq!(
        node_choice_indices(&offered),
        vec![0, 1, 2, 3],
        "the whole table at the table"
    );
    let greeting = dialog_lines(&offered);
    assert!(
        greeting.iter().any(|line| line.starts_with("Brother Corvin has covered")),
        "got {greeting:?}"
    );

    // "Ask about Words of Power" leads to the words, which leads to crafting.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(1));
    region::tick(&mut ctx);
    let offered = drain_messages(&receiver);
    let words = dialog_lines(&offered);
    assert!(
        words.iter().any(|line| line.starts_with("SAR IR")),
        "the words step has to follow, got {words:?}"
    );
    assert_eq!(node_choice_indices(&offered), vec![0, 1]);

    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);
    let crafting = dialog_lines(&drain_messages(&receiver));
    assert!(
        crafting.iter().any(|line| line.starts_with("Warding Salt")),
        "the crafting step has to follow, got {crafting:?}"
    );

    // "Back" returns to the greeting, which is the loop the tree had.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);
    let back = dialog_lines(&drain_messages(&receiver));
    assert!(
        back.iter().any(|line| line.starts_with("Brother Corvin has covered")),
        "back has to return to the greeting, got {back:?}"
    );
}

/// Quartermaster Nessa's shipped graph opens her stock from a choice that leaves
/// through its consequence port, where a Message node and an Offer Inventory
/// node do the work.
#[test]
fn hideout2d_nessa_opens_her_stock_from_the_shipped_graph() {
    use crate::server::message::RegionMessage;

    let Some((mut ctx, receiver)) = run_shipped_character("Quartermaster Nessa") else {
        return;
    };

    let offered = drain_messages(&receiver);
    assert_eq!(node_choice_indices(&offered), vec![0, 1, 2]);
    let greeting = dialog_lines(&offered);
    assert!(
        greeting.iter().any(|line| line.starts_with("Quartermaster Nessa checks")),
        "got {greeting:?}"
    );

    // "Browse expedition supplies" messages the player and opens the chest.
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(0));
    region::tick(&mut ctx);
    let messages = drain_messages(&receiver);
    assert!(
        messages.iter().any(|message| matches!(
            message,
            RegionMessage::Message(_, Some(2), None, _, text, role)
                if text == "Nessa opens the expedition chest." && role == "dialog"
        )),
        "the chest message has to reach the player, got {messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, RegionMessage::MultipleChoice(_))),
        "the stock has to open as a choice list, got {messages:?}"
    );

    // The second branch is the advice, still inside the same node.
    run_intent(&mut ctx);
    let _ = drain_messages(&receiver);
    ctx.dialog_choices.insert(2, DialogChoiceMade::Index(1));
    region::tick(&mut ctx);
    let offered = drain_messages(&receiver);
    let advice = dialog_lines(&offered);
    assert!(
        advice.iter().any(|line| line.starts_with("A torch, arrows")),
        "the advice step has to follow, got {advice:?}"
    );
    assert_eq!(node_choice_indices(&offered), vec![0, 1]);
}
