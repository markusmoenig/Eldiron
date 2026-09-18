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
            Registry::builtin().compile(&doc).unwrap(),
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
            Registry::builtin().compile(&doc).unwrap(),
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
            Registry::builtin().compile(&doc).unwrap(),
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
            Registry::builtin().compile(&doc).unwrap(),
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
