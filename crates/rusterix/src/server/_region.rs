use crate::server::py_fn::*;
use crate::{
    Assets, Choice, Currency, Entity, EntityAction, Item, Map, MultipleChoice, PixelSource,
    PlayerCamera, RegionCtx, Value, ValueContainer,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use rand::*;

use rustpython::vm::*;
use std::sync::{Arc, Mutex};
use theframework::prelude::{FxHashMap, TheTime, Uuid};
use vek::num_traits::zero;

use std::sync::atomic::{AtomicU32, Ordering};
use vek::Vec2;

use std::sync::{LazyLock, RwLock};
use theframework::prelude::TheValue;

/// The global store of RegionCtx
static REGIONCTX: LazyLock<RwLock<FxHashMap<u32, Arc<Mutex<RegionCtx>>>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

/// Register a new RegionCtx
pub fn register_regionctx(id: u32, instance: Arc<Mutex<RegionCtx>>) {
    REGIONCTX.write().unwrap().insert(id, instance);
}

/// Clear the store.
pub fn clear_regionctx_store() {
    REGIONCTX.write().unwrap().clear();
}

/// Get a specific RegionCtx
pub fn get_regionctx(id: u32) -> Option<Arc<Mutex<RegionCtx>>> {
    REGIONCTX.read().unwrap().get(&id).cloned()
}

/// Get gelper
pub fn with_regionctx<F, R>(region_id: u32, f: F) -> Option<R>
where
    F: FnOnce(&mut RegionCtx) -> R,
{
    get_regionctx(region_id).map(|arc| {
        let mut ctx = arc.lock().unwrap(); // Consider proper error handling if needed
        f(&mut ctx)
    })
}

/// Get the region id embedded in the VM
pub fn get_region_id(vm: &VirtualMachine) -> Option<u32> {
    let module = vm.import("__region_meta", 0).ok()?;
    let obj = module.get_attr("__region_id", vm).ok()?;
    obj.try_to_value::<u32>(vm).ok()
}

// Global Id Generator over all threads and regions
static GLOBAL_ID_GEN: AtomicU32 = AtomicU32::new(0);

pub fn get_global_id() -> u32 {
    GLOBAL_ID_GEN.fetch_add(1, Ordering::Relaxed)
}

use EntityAction::*;

use super::RegionMessage;
use super::data::{apply_entity_data, apply_item_data};
use RegionMessage::*;

pub struct RegionInstance {
    pub id: u32,

    interp: Interpreter,
    scope: Arc<Mutex<rustpython_vm::scope::Scope>>,

    name: String,

    /// Send messages to this region
    pub to_sender: Sender<RegionMessage>,
    /// Local receiver
    to_receiver: Receiver<RegionMessage>,

    /// Send messages from this region
    from_sender: Sender<RegionMessage>,
    /// Local receiver
    pub from_receiver: Receiver<RegionMessage>,

    /// Entity block mode
    entity_block_mode: i32,
}

use rustpython_vm::{PyObjectRef, VirtualMachine, builtins::PyModule};

impl RegionInstance {
    pub fn new(region_id: u32) -> Self {
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        let scope = Arc::new(Mutex::new(interp.enter(|vm| vm.new_scope_with_builtins())));

        interp.enter(|vm| {
            let scope = scope.lock().unwrap();

            let module = PyModule::new().into_ref(&vm.ctx);
            module
                .as_object()
                .set_attr("__region_id", vm.new_pyobj(region_id), vm)
                .ok()
                .unwrap();

            let sys = vm.import("sys", 0).ok().unwrap();
            let modules = sys.get_attr("modules", vm).ok().unwrap();
            modules
                .set_item("__region_meta", module.into(), vm)
                .ok()
                .unwrap();

            // let _ = scope.globals.set_item(
            //     "register_player",
            //     vm.new_function("register_player", register_player).into(),
            //     vm,
            // );

            let _ = scope.globals.set_item(
                "action",
                vm.new_function("action", player_action).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "intent",
                vm.new_function("intent", player_intent).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_player_camera",
                vm.new_function("set_player_camera", set_player_camera)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_tile",
                vm.new_function("set_tile", set_tile).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_emit_light",
                vm.new_function("set_emit_light", set_emit_light).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_rig_sequence",
                vm.new_function("set_rig_sequence", set_rig_sequence).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("take", vm.new_function("take", take).into(), vm);

            let _ = scope
                .globals
                .set_item("equip", vm.new_function("equip", equip).into(), vm);

            let _ = scope.globals.set_item(
                "get_attr_of",
                vm.new_function("get_attr_of", get_attr_of).into(),
                vm,
            );

            // let _ = scope.globals.set_item(
            //     "get_entity_attr",
            //     vm.new_function("get_entity_attr", get_entity_attr).into(),
            //     vm,
            // );

            // let _ = scope.globals.set_item(
            //     "get_item_attr",
            //     vm.new_function("get_item_attr", get_item_attr).into(),
            //     vm,
            // );

            let _ = scope.globals.set_item(
                "get_attr",
                vm.new_function("get_attr", get_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_attr",
                vm.new_function("set_attr", set_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "toggle_attr",
                vm.new_function("toggle_attr", toggle_attr).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "random",
                vm.new_function("random", random_in_range).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "notify_in",
                vm.new_function("notify_in", notify_in).into(),
                vm,
            );

            // let _ = scope.globals.set_item(
            //     "get_sector_name",
            //     vm.new_function("get_sector_name", get_sector_name).into(),
            //     vm,
            // );

            // let _ = scope.globals.set_item(
            //     "face_random",
            //     vm.new_function("face_random", face_random).into(),
            //     vm,
            // );

            let _ = scope.globals.set_item(
                "random_walk",
                vm.new_function("random_walk", random_walk).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "random_walk_in_sector",
                vm.new_function("random_walk_in_sector", random_walk_in_sector)
                    .into(),
                vm,
            );

            let _ =
                scope
                    .globals
                    .set_item("message", vm.new_function("message", message).into(), vm);

            let _ = scope
                .globals
                .set_item("debug", vm.new_function("debug", debug).into(), vm);

            let _ = scope.globals.set_item(
                "inventory_items",
                vm.new_function("inventory_items", inventory_items).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "inventory_items_of",
                vm.new_function("inventory_items_of", inventory_items_of)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "entities_in_radius",
                vm.new_function("entities_in_radius", entities_in_radius)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "set_proximity_tracking",
                vm.new_function("set_proximity_tracking", set_proximity_tracking)
                    .into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "deal_damage",
                vm.new_function("deal_damage", deal_damage).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "took_damage",
                vm.new_function("took_damage", took_damage).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "block_events",
                vm.new_function("block_events", block_events).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "add_item",
                vm.new_function("add_item", add_item).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "drop_items",
                vm.new_function("drop_items", drop_items).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "offer_inventory",
                vm.new_function("offer_inventory", offer_inventory).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("drop", vm.new_function("drop", drop).into(), vm);

            let _ = scope.globals.set_item(
                "teleport",
                vm.new_function("teleport", teleport).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("goto", vm.new_function("goto", goto).into(), vm);

            let _ = scope.globals.set_item(
                "close_in",
                vm.new_function("close_in", close_in).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("id", vm.new_function("id", id).into(), vm);

            let _ = scope.globals.set_item(
                "set_debug_loc",
                vm.new_function("set_debug_loc", set_debug_loc).into(),
                vm,
            );
        });

        let (to_sender, to_receiver) = unbounded::<RegionMessage>();
        let (from_sender, from_receiver) = unbounded::<RegionMessage>();

        Self {
            id: region_id,

            interp,
            scope,

            name: String::new(),

            to_receiver,
            to_sender,
            from_receiver,
            from_sender,

            entity_block_mode: 0,
        }
    }

    /// Initializes the Python bases classes, sets the map and applies entities
    pub fn init(
        &mut self,
        name: String,
        map: Map,
        assets: &Assets,
        config_toml: String,
        debug_mode: bool,
    ) {
        self.name = name.clone();

        let mut ctx = RegionCtx::default();
        ctx.debug_mode = debug_mode;

        if let Ok(toml) = config_toml.parse::<toml::Table>() {
            ctx.config = toml;
        }

        ctx.map = map;
        ctx.blocking_tiles = assets.blocking_tiles();
        ctx.assets = assets.clone();

        // Installing currencies

        _ = ctx.currencies.add_currency(Currency {
            name: "Gold".into(),
            symbol: "G".into(),
            exchange_rate: 1.0,
            max_limit: None,
        });
        ctx.currencies.base_currency = "G".to_string();

        // Installing Entity Class Templates
        for (name, (entity_source, entity_data)) in &assets.entities {
            if let Err(err) = self.execute(entity_source) {
                ctx.startup_errors.push(format!(
                    "{}: Error Compiling {} Character Class: {}",
                    self.name, name, err,
                ));
            }
            if let Err(err) = self.execute(&format!("{} = {}()", name, name)) {
                ctx.startup_errors.push(format!(
                    "{}: Error Installing {} Character Class: {}",
                    self.name, name, err,
                ));
            }

            // Store entity classes which handle player
            match entity_data.parse::<toml::Table>() {
                Ok(data) => {
                    if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                        if let Some(value) = game.get("player") {
                            if let Some(v) = value.as_bool() {
                                if v {
                                    ctx.entity_player_classes.insert(name.clone());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    ctx.startup_errors.push(format!(
                        "{}: Error Parsing {} Entity Class: {}",
                        self.name, name, err,
                    ));
                }
            }

            ctx.entity_class_data
                .insert(name.clone(), entity_data.clone());
        }

        // Installing Item Class Templates
        for (name, (item_source, item_data)) in &assets.items {
            if let Err(err) = self.execute(item_source) {
                ctx.startup_errors.push(format!(
                    "{}: Error Compiling {} Item Class: {}",
                    self.name, name, err,
                ));
            }
            if let Err(err) = self.execute(&format!("{} = {}()", name, name)) {
                ctx.startup_errors.push(format!(
                    "{}: Error Installing {} Item Class: {}",
                    self.name, name, err,
                ));
            }
            ctx.item_class_data.insert(name.clone(), item_data.clone());
        }

        // Remove player based entities, these only get created on demand from a client
        let player_classes = ctx.entity_player_classes.clone();
        ctx.map
            .entities
            .retain(|entity| match entity.get_attr_string("class_name") {
                Some(class_name) => !player_classes.contains(&class_name),
                None => true,
            });

        // Set an entity id and mark all fields dirty for the first transmission to the server.
        for e in ctx.map.entities.iter_mut() {
            e.id = get_global_id();
            // By default we set the sequence to idle.
            e.set_attribute(
                "_source_seq",
                Value::Source(PixelSource::Sequence("idle".into())),
            );
            e.set_attribute("mode", Value::Str("active".into()));
            e.mark_all_dirty();
        }

        // Set an item id and mark all fields dirty for the first transmission to the server.
        for i in ctx.map.items.iter_mut() {
            i.id = get_global_id();
            // By default we set the sequence to idle.
            i.attributes.set(
                "_source_seq",
                Value::Source(PixelSource::Sequence("_".into())),
            );
            i.mark_all_dirty();
        }

        // --- Startup

        ctx.from_sender.set(self.from_sender.clone()).unwrap();
        ctx.to_receiver.set(self.to_receiver.clone()).unwrap();
        ctx.region_id = self.id;
        ctx.mapmini = ctx.map.as_mini(&ctx.blocking_tiles);

        // Build collision geometry for all chunks (new collision system)
        use crate::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};
        let mut chunk_builder = D3ChunkBuilder::new();
        let chunk_size = 10; // Match collision_world chunk size

        // Calculate chunk bounds from map
        let mut min_chunk = vek::Vec2::new(i32::MAX, i32::MAX);
        let mut max_chunk = vek::Vec2::new(i32::MIN, i32::MIN);

        for surface in ctx.map.surfaces.values() {
            if let Some(sector) = ctx.map.find_sector(surface.sector_id) {
                let bbox = sector.bounding_box(&ctx.map);
                let chunk_min = vek::Vec2::new(
                    (bbox.min.x / chunk_size as f32).floor() as i32,
                    (bbox.min.y / chunk_size as f32).floor() as i32,
                );
                let chunk_max = vek::Vec2::new(
                    (bbox.max.x / chunk_size as f32).floor() as i32,
                    (bbox.max.y / chunk_size as f32).floor() as i32,
                );
                min_chunk =
                    vek::Vec2::new(min_chunk.x.min(chunk_min.x), min_chunk.y.min(chunk_min.y));
                max_chunk =
                    vek::Vec2::new(max_chunk.x.max(chunk_max.x), max_chunk.y.max(chunk_max.y));
            }
        }

        // Build collision for each chunk
        for cy in min_chunk.y..=max_chunk.y {
            for cx in min_chunk.x..=max_chunk.x {
                let chunk_origin = vek::Vec2::new(cx, cy);
                let chunk_collision =
                    chunk_builder.build_collision(&ctx.map, chunk_origin, chunk_size);

                ctx.collision_world
                    .update_chunk(chunk_origin, chunk_collision);
            }
        }

        ctx.ticks = 0;

        ctx.ticks_per_minute = 4;
        ctx.ticks_per_minute = get_config_i32_default(&ctx, "game", "ticks_per_minute", 4) as u32;

        // let game_tick_ms = get_config_i32_default(&ctx, "game", "game_tick_ms", 250) as u64;
        let target_fps = get_config_i32_default(&ctx, "game", "target_fps", 30) as f32;

        ctx.delta_time = 1.0 / target_fps;
        ctx.health_attr = get_config_string_default(&ctx, "game", "health", "HP").to_string();

        self.entity_block_mode = {
            let mode = get_config_string_default(&ctx, "game", "entity_block_mode", "always");
            if mode == "always" { 1 } else { 0 }
        };

        // Send startup messages
        ctx.error_count = ctx.startup_errors.len() as u32;
        for l in &ctx.startup_errors {
            send_log_message(self.id, l.clone());
        }

        let entities: Vec<Entity> = ctx.map.entities.clone();
        let items = ctx.map.items.clone();

        // Setting the data for the entities.
        for entity in entities.iter() {
            if let Some(class_name) = entity.get_attr_string("class_name") {
                if let Some(data) = ctx.entity_class_data.get(&class_name) {
                    for e in ctx.map.entities.iter_mut() {
                        if e.id == entity.id {
                            apply_entity_data(e, data);

                            // Fill up the inventory slots
                            if let Some(Value::Int(inv_slots)) = e.attributes.get("inventory_slots")
                            {
                                e.inventory = vec![];
                                for _ in 0..*inv_slots {
                                    e.inventory.push(None);
                                }
                            }

                            // Set the wallet
                            if let Some(Value::Int(wealth)) = e.attributes.get("wealth") {
                                _ = e.add_base_currency(*wealth as i64, &ctx.currencies)
                            }
                        }
                    }
                }
            }
        }

        // Register the ctx, from here on we have to lock it
        register_regionctx(self.id, Arc::new(Mutex::new(ctx)));

        // Send "startup" event to all entities.
        for entity in entities.iter() {
            if let Some(class_name) = entity.get_attr_string("class_name") {
                let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                    ctx.entity_classes.insert(entity.id, class_name.clone());
                    ctx.curr_entity_id = entity.id;
                });
                if let Err(err) = self.execute(&cmd) {
                    send_log_message(
                        self.id,
                        format!(
                            "{}: Event Error ({}) for '{}': {}",
                            name,
                            "startup",
                            self.get_entity_name(entity.id),
                            err,
                        ),
                    );
                }

                // Determine, set and notify the entity about the sector it is in.
                let mut sector_name = String::new();
                with_regionctx(self.id, |ctx| {
                    if let Some(sector) = ctx.map.find_sector_at(entity.get_pos_xz()) {
                        sector_name = sector.name.clone();
                    }
                    {
                        for e in ctx.map.entities.iter_mut() {
                            if e.id == entity.id {
                                e.attributes.set("sector", Value::Str(sector_name.clone()));
                            }
                        }
                    }
                });
                if !sector_name.is_empty() {
                    let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector_name);
                    _ = self.execute(&cmd);
                }
            }
        }

        // Send "startup" event to all items.
        for item in items.iter() {
            if let Some(class_name) = item.get_attr_string("class_name") {
                let cmd = format!("{}.event(\"startup\", \"\")", class_name);
                with_regionctx(self.id, |ctx| {
                    ctx.item_classes.insert(item.id, class_name.clone());
                    ctx.curr_item_id = Some(item.id);
                });
                if let Err(err) = self.execute(&cmd) {
                    send_log_message(
                        self.id,
                        format!(
                            "{}: Item Event Error ({}) for '{}': {}",
                            name,
                            "startup",
                            self.get_entity_name(item.id),
                            err,
                        ),
                    );
                }
            }
        }
        with_regionctx(self.id, |ctx| {
            ctx.curr_item_id = None;
        });

        // Running the character setup scripts for the class instances
        for entity in entities.iter() {
            if let Some(setup) = entity.get_attr_string("setup") {
                if let Err(err) = self.execute(&setup) {
                    send_log_message(
                        self.id,
                        format!(
                            "{}: Setup '{}/{}': {}",
                            name,
                            entity.get_attr_string("name").unwrap_or("Unknown".into()),
                            entity
                                .get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ),
                    );
                    with_regionctx(self.id, |ctx| {
                        ctx.error_count += 1;
                    });
                }

                with_regionctx(self.id, |ctx| {
                    ctx.curr_entity_id = entity.id;
                });
                if let Err(err) = self.execute("setup()") {
                    send_log_message(
                        self.id,
                        format!(
                            "{}: Setup '{}/{}': {}",
                            name,
                            entity.get_attr_string("name").unwrap_or("Unknown".into()),
                            entity
                                .get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ),
                    );
                    with_regionctx(self.id, |ctx| {
                        ctx.error_count += 1;
                    });
                }

                /*
                // Setting the data for the entity.
                if let Some(class_name) = entity.get_attr_string("class_name") {
                    if let Some(data) = ENTITY_CLASS_DATA.borrow().get(&class_name) {
                        let mut map = MAP.borrow_mut();
                        for e in map.entities.iter_mut() {
                            if e.id == entity.id {
                                apply_entity_data(e, data);

                                if let Some(inv_slots) = e.attributes.get("inventory_slots") {
                                    println!("{} {}", class_name, inv_slots);
                                }
                            }
                        }
                    }
                }*/
            }
        }

        // Running the item setup scripts for the class instances
        let mut items = vec![];
        with_regionctx(self.id, |ctx| {
            items = ctx.map.items.clone();
        });
        for item in items.iter_mut() {
            if let Some(setup) = item.get_attr_string("setup") {
                if let Err(err) = self.execute(&setup) {
                    send_log_message(
                        self.id,
                        format!(
                            "{}: Item Setup '{}/{}': {}",
                            name,
                            item.get_attr_string("name").unwrap_or("Unknown".into()),
                            item.get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ),
                    );
                    with_regionctx(self.id, |ctx| {
                        ctx.error_count += 1;
                    });
                }

                with_regionctx(self.id, |ctx| {
                    ctx.curr_item_id = Some(item.id);
                });
                if let Err(err) = self.execute("setup()") {
                    send_log_message(
                        self.id,
                        format!(
                            "{}: Item Setup '{}/{}': {}",
                            name,
                            item.get_attr_string("name").unwrap_or("Unknown".into()),
                            item.get_attr_string("class_name")
                                .unwrap_or("Unknown".into()),
                            err,
                        ),
                    );
                    with_regionctx(self.id, |ctx| {
                        ctx.error_count += 1;
                    });
                }
            }
            // Setting the data for the item.
            if let Some(class_name) = item.get_attr_string("class_name") {
                let mut cmd = String::new();
                with_regionctx(self.id, |ctx| {
                    if let Some(data) = ctx.item_class_data.get(&class_name) {
                        for i in ctx.map.items.iter_mut() {
                            if i.id == item.id {
                                apply_item_data(i, data);
                                *item = i.clone();
                            }
                        }
                    }
                    // Send active state
                    cmd = format!(
                        "{}.event(\"active\", {})",
                        class_name,
                        if item.attributes.get_bool_default("active", false) {
                            "True"
                        } else {
                            "False"
                        }
                    );
                });
                _ = self.execute(&cmd);
            }
        }

        // Wrapping up ...
        let mut error_count = 0;
        with_regionctx(self.id, |ctx| {
            ctx.curr_item_id = None;
            error_count = ctx.error_count;
        });

        // Send startup log message
        send_log_message(
            self.id,
            format!("{}: Startup with {} errors.", name, error_count),
        );
    }

    /// System tick
    pub fn system_tick(&self) {
        let mut ticks = 0;

        with_regionctx(self.id, |ctx| {
            ctx.ticks += 1;
            ticks = ctx.ticks;

            let mins = ctx.time.total_minutes();
            ctx.time = TheTime::from_ticks(ticks, ctx.ticks_per_minute);

            if ctx.time.total_minutes() > mins {
                // If the time changed send to server
                self.from_sender
                    .send(RegionMessage::Time(self.id, ctx.time))
                    .unwrap();
            }
        });

        // Process notifications for entities.
        let to_process = {
            let mut notifications = vec![];
            with_regionctx(self.id, |ctx| {
                notifications = ctx.notifications_entities.clone();
            });

            notifications
                .iter()
                .filter(|(_, tick, _)| *tick <= ticks)
                .cloned() // Clone only the relevant items
                .collect::<Vec<_>>() // Store them in a new list
        };
        for (id, _tick, notification) in &to_process {
            if !is_entity_dead(self.id, *id) {
                let mut cmd = String::new();
                with_regionctx(self.id, |ctx| {
                    if let Some(class_name) = ctx.entity_classes.get(id) {
                        cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                        ctx.curr_entity_id = *id;
                        ctx.curr_item_id = None;
                    }
                });

                let _ = self.execute(&cmd);
            }
        }

        with_regionctx(self.id, |ctx| {
            ctx.notifications_entities.retain(|(id, tick, _)| {
                !to_process
                    .iter()
                    .any(|(pid, _, _)| pid == id && *tick <= ticks)
            });
        });

        // Process notifications for items.
        let to_process = {
            let mut notifications = vec![];
            with_regionctx(self.id, |ctx| {
                notifications = ctx.notifications_items.clone();
            });

            notifications
                .iter()
                .filter(|(_, tick, _)| *tick <= ticks)
                .cloned()
                .collect::<Vec<_>>()
        };
        for (id, _tick, notification) in &to_process {
            let mut cmd = String::new();
            with_regionctx(self.id, |ctx| {
                if let Some(class_name) = ctx.item_classes.get(id) {
                    cmd = format!("{}.event(\"{}\", \"\")", class_name, notification);
                    ctx.curr_item_id = Some(*id);
                }
            });
            let _ = self.execute(&cmd);
            with_regionctx(self.id, |ctx| {
                ctx.curr_item_id = None;
            });
        }

        with_regionctx(self.id, |ctx| {
            ctx.notifications_items.retain(|(id, tick, _)| {
                !to_process
                    .iter()
                    .any(|(pid, _, _)| pid == id && *tick <= ticks)
            });
        });

        // Check Proximity Alerts
        with_regionctx(self.id, |ctx| {
            for (id, radius) in ctx.entity_proximity_alerts.iter() {
                let entities = self.entities_in_radius(ctx, Some(*id), None, *radius);
                if !entities.is_empty() {
                    if let Some(class_name) = ctx.entity_classes.get(id) {
                        let cmd = format!(
                            "{}.event(\"{}\", [{}])",
                            class_name,
                            "proximity_warning",
                            entities
                                .iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        ctx.to_execute_entity
                            .push((*id, "proximity_warning".into(), cmd));
                    }
                }
            }

            if ctx.debug_mode {
                self.from_sender
                    .send(RegionMessage::DebugData(ctx.debug.clone()))
                    .unwrap();
            }
        });
    }

    /// Redraw tick
    pub fn redraw_tick(&mut self) {
        // Catch up with the server messages
        while let Ok(msg) = self.to_receiver.try_recv() {
            match msg {
                Event(entity_id, event, value) => {
                    let mut cmd = String::new();
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        if let Some(class_name) = ctx.entity_classes.get(&entity_id) {
                            cmd = format!("{}.event('{}', {})", class_name, event, value);
                            ctx.curr_entity_id = entity_id;
                            ctx.curr_item_id = None;
                        }
                    });

                    if let Err(err) = self.execute(&cmd) {
                        send_log_message(
                            self.id,
                            format!(
                                "{}: Event Error for '{}': {}",
                                self.name,
                                self.get_entity_name(entity_id),
                                err,
                            ),
                        );
                    }
                }
                UserEvent(entity_id, event, value) => {
                    let mut cmd = String::new();
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        if let Some(class_name) = ctx.entity_classes.get(&entity_id) {
                            cmd = format!("{}.user_event('{}', '{}')", class_name, event, value);
                            ctx.curr_entity_id = entity_id;
                            ctx.curr_item_id = None;
                        }
                    });
                    if let Err(err) = self.execute(&cmd) {
                        send_log_message(
                            self.id,
                            format!(
                                "{}: User Event Error for '{}': {}",
                                self.name,
                                self.get_entity_name(entity_id),
                                err,
                            ),
                        );
                    }
                }
                UserAction(entity_id, action) => match action {
                    Intent(intent) => {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            if let Some(entity) = ctx
                                .map
                                .entities
                                .iter_mut()
                                .find(|entity| entity.id == entity_id)
                            {
                                entity.set_attribute("intent", Value::Str(intent));
                            }
                        });
                    }
                    EntityClicked(clicked_entity_id, distance) => {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                                if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                                    // Send "intent" event for the entity
                                    let mut cont = ValueContainer::default();
                                    cont.set("distance", Value::Float(distance));
                                    cont.set("entity_id", Value::UInt(entity_id));
                                    cont.set("target_id", Value::UInt(clicked_entity_id));

                                    let intent =
                                        entity.attributes.get_str_default("intent", "".into());
                                    cont.set("intent", Value::Str(intent.clone()));

                                    let event_name = format!("intent: {}", intent);

                                    let cmd = format!(
                                        "{}.event('intent', {})",
                                        class_name,
                                        cont.to_python_dict_string()
                                    );
                                    ctx.to_execute_entity.push((
                                        entity.id,
                                        event_name.clone(),
                                        cmd.clone(),
                                    ));

                                    // Send for the target
                                    let mut cont = ValueContainer::default();
                                    cont.set("distance", Value::Float(distance));
                                    cont.set("entity_id", Value::UInt(entity_id));
                                    cont.set("intent", Value::Str(intent));
                                    if let Some(class_name) =
                                        ctx.entity_classes.get(&clicked_entity_id)
                                    {
                                        let cmd = format!(
                                            "{}.event('intent', {})",
                                            class_name,
                                            cont.to_python_dict_string()
                                        );
                                        ctx.to_execute_entity.push((
                                            clicked_entity_id,
                                            event_name,
                                            cmd,
                                        ));
                                    }

                                    entity.set_attribute("intent", Value::Str(String::new()));
                                }
                            }
                        });
                    }
                    ItemClicked(clicked_item_id, distance) => {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                                if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                                    // Send "intent" event for the entity
                                    let mut cont = ValueContainer::default();
                                    cont.set("distance", Value::Float(distance));
                                    cont.set("item_id", Value::UInt(clicked_item_id));
                                    cont.set("entity_id", Value::UInt(entity.id));

                                    let intent =
                                        entity.attributes.get_str_default("intent", "".into());

                                    let event_name = format!("intent: {}", intent);

                                    cont.set("intent", Value::Str(intent));
                                    let cmd = format!(
                                        "{}.event('intent', {})",
                                        class_name,
                                        cont.to_python_dict_string()
                                    );
                                    ctx.to_execute_entity.push((
                                        entity.id,
                                        event_name.clone(),
                                        cmd.clone(),
                                    ));

                                    if let Some(class_name) = ctx.item_classes.get(&clicked_item_id)
                                    {
                                        let cmd = format!(
                                            "{}.event('intent', {})",
                                            class_name,
                                            cont.to_python_dict_string()
                                        );
                                        ctx.to_execute_item.push((
                                            clicked_item_id,
                                            event_name,
                                            cmd,
                                        ));
                                    }

                                    entity.set_attribute("intent", Value::Str(String::new()));
                                }
                            }
                        });
                    }
                    Choice(choice) => match &choice {
                        Choice::ItemToSell(item_id, seller_id, buyer_id) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                let mut msg_to_buyer: Option<String> = None;
                                let mut msg_role = "system";

                                // Get the price of the item.
                                let mut price = 0;
                                let mut can_afford = false;
                                if let Some(entity) = get_entity_mut(&mut ctx.map, *seller_id) {
                                    if let Some(item) = entity.get_item(*item_id) {
                                        if let Some(w) = item.get_attribute("worth") {
                                            if let Some(worth) = w.to_i32() {
                                                price = worth as i64;
                                            }
                                        }
                                    }
                                }

                                // Check if the buyer can afford
                                if let Some(entity) = get_entity_mut(&mut ctx.map, *buyer_id) {
                                    can_afford = entity.wallet.can_afford(price, &ctx.currencies);
                                }

                                if can_afford {
                                    let mut item_to_sell: Option<Item> = None;
                                    if let Some(entity) = get_entity_mut(&mut ctx.map, *seller_id) {
                                        if let Some(item) = entity.remove_item(*item_id) {
                                            // println!("itemtosell {:?}", item);
                                            item_to_sell = Some(item);
                                            _ = entity.add_base_currency(price, &ctx.currencies);
                                        }
                                    }
                                    if let Some(item) = item_to_sell {
                                        if let Some(entity) =
                                            get_entity_mut(&mut ctx.map, *buyer_id)
                                        {
                                            msg_to_buyer = Some(format!(
                                                "{{you_bought}} {{I:{}.name, article=indef, case=lower}}",
                                                item.id
                                            ));
                                            _ = entity.add_item(item);
                                            _ = entity.spend_currency(price, &ctx.currencies);
                                        }
                                    }
                                } else {
                                    msg_to_buyer = Some("{cant_afford}".into());
                                    msg_role = "warning";
                                }

                                if let Some(msg_to_buyer) = msg_to_buyer {
                                    send_message(ctx, *buyer_id, msg_to_buyer, msg_role);
                                }
                            });
                        }
                        Choice::Cancel(from_id, to_id) => {
                            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                                if let Some(class_name) = ctx.entity_classes.get(from_id) {
                                    let cmd = format!("{}.event('goodbye', {})", class_name, to_id);
                                    ctx.to_execute_entity
                                        .push((*from_id, "goodbye".into(), cmd));
                                }
                            });
                        }
                    },
                    _ => {
                        with_regionctx(self.id, |ctx: &mut RegionCtx| {
                            if let Some(entity) = ctx
                                .map
                                .entities
                                .iter_mut()
                                .find(|entity| entity.id == entity_id)
                            {
                                entity.action = action;
                            }
                        });
                    }
                },
                CreateEntity(_id, entity) => self.create_entity_instance(entity),
                TransferEntity(_region_id, entity, _dest_region_name, dest_sector_name) => {
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        receive_entity(ctx, entity, dest_sector_name);
                    });
                }
                Time(_id, time) => {
                    // User manually set the server time
                    with_regionctx(self.id, |ctx: &mut RegionCtx| {
                        ctx.ticks = time.to_ticks(ctx.ticks_per_minute);
                        ctx.time = time;
                    });
                }
                Quit => {
                    println!("Shutting down '{}'. Goodbye.", self.name);
                }
                _ => {}
            }
        }

        // ---

        let mut updates: Vec<Vec<u8>> = vec![];
        let mut item_updates: Vec<Vec<u8>> = vec![];

        let mut entities = vec![];
        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            entities = ctx.map.entities.clone();
        });

        for entity in &mut entities {
            match &entity.action.clone() {
                EntityAction::Forward => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_north();
                                }
                                self.move_entity(entity, 1.0, self.entity_block_mode);
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_north();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, 1.0, self.entity_block_mode);
                    }
                }
                EntityAction::Left => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_west();
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_left(4.0);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_west();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.turn_left(4.0);
                    }
                }
                EntityAction::Right => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            // If no intent we walk
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_east();
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    entity.turn_right(4.0);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_east();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        entity.turn_right(4.0);
                    }
                }
                EntityAction::Backward => {
                    if entity.is_player() {
                        let intent = entity.attributes.get_str_default("intent", "".into());
                        if intent.is_empty() {
                            if let Some(Value::PlayerCamera(player_camera)) =
                                entity.attributes.get("player_camera")
                            {
                                if *player_camera != PlayerCamera::D3FirstP {
                                    entity.face_south();
                                    self.move_entity(entity, 1.0, self.entity_block_mode);
                                } else {
                                    self.move_entity(entity, -1.0, self.entity_block_mode);
                                }
                            }
                        } else {
                            // If intent is set we send "intent" events
                            entity.face_south();
                            let position = entity.get_forward_pos(1.0);
                            self.send_entity_intent_events(entity, position);
                            entity.action = EntityAction::Off;
                        }
                    } else {
                        self.move_entity(entity, -1.0, self.entity_block_mode);
                    }
                }
                EntityAction::CloseIn(target, target_radius, speed) => {
                    if is_entity_dead(self.id, *target) {
                        continue;
                    }

                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
                    let target_id = *target;

                    let mut coord: Option<vek::Vec2<f32>> = None;

                    with_regionctx(self.id, |ctx| {
                        let speed: f32 = 4.0 * speed * ctx.delta_time;

                        if let Some(entity) =
                            ctx.map.entities.iter().find(|entity| entity.id == *target)
                        {
                            coord = Some(entity.get_pos_xz());
                        }

                        if let Some(coord) = coord {
                            let (new_position, arrived) = ctx.mapmini.close_in(
                                position,
                                coord,
                                *target_radius,
                                speed,
                                radius,
                                1.0,
                            );

                            entity.set_pos_xz(new_position);
                            if arrived {
                                entity.action = EntityAction::Off;

                                // Send closed in event
                                if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                                    let cmd = format!(
                                        "{}.event(\"closed_in\", {})",
                                        class_name, target_id
                                    );
                                    ctx.to_execute_entity.push((
                                        entity.id,
                                        "closed_in".into(),
                                        cmd,
                                    ));
                                }
                            }

                            check_player_for_section_change(ctx, entity);
                        }
                    });
                }
                EntityAction::Goto(coord, speed) => {
                    let position = entity.get_pos_xz();
                    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;

                    with_regionctx(self.id, |ctx| {
                        let speed = 4.0 * speed * ctx.delta_time;

                        let (new_position, arrived) = ctx
                            .mapmini
                            .move_towards(position, *coord, speed, radius, 1.0);

                        entity.set_pos_xz(new_position);
                        if arrived {
                            entity.action = EntityAction::Off;

                            let mut sector_name: String = String::new();
                            {
                                if let Some(s) = ctx.map.find_sector_at(new_position) {
                                    sector_name = s.name.clone();
                                }
                            }

                            // Send arrived event
                            if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                                let cmd =
                                    format!("{}.event('arrived', \"{}\")", class_name, sector_name);
                                ctx.to_execute_entity.push((
                                    entity.id,
                                    "arrived".into(),
                                    cmd.clone(),
                                ));
                            }
                        };
                        check_player_for_section_change(ctx, entity);
                    });
                }
                EntityAction::RandomWalk(distance, speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let pos = find_random_position(entity.get_pos_xz(), *distance);
                        entity.action = RandomWalk(*distance, *speed, *max_sleep, 1, pos);
                        entity.face_at(pos);
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.random_range(*max_sleep / 2..=*max_sleep) as u32,
                                RandomWalk(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let t = RandomWalk(*distance, *speed, *max_sleep, 0, *target);
                            let max_sleep = *max_sleep;
                            let blocked = self.move_entity(entity, 1.0, self.entity_block_mode);
                            if blocked {
                                let mut rng = rand::rng();
                                entity.action = self.create_sleep_switch_action(
                                    rng.random_range(max_sleep / 2..=max_sleep) as u32,
                                    t,
                                );
                            }
                        }
                    }
                }
                EntityAction::RandomWalkInSector(distance, speed, max_sleep, state, target) => {
                    if *state == 0 {
                        // State 0: Uninitialized, find a target location.
                        let curr_pos = entity.get_pos_xz().clone();
                        with_regionctx(self.id, |ctx| {
                            if let Some(sector) = ctx.map.find_sector_at(curr_pos) {
                                let mut new_pos = find_random_position(curr_pos, *distance);
                                let mut found = false;

                                for _ in 0..10 {
                                    if sector.is_inside(&ctx.map, new_pos) {
                                        found = true;
                                        break;
                                    } else {
                                        new_pos = find_random_position(curr_pos, *distance);
                                    }
                                }

                                if found {
                                    entity.action = RandomWalkInSector(
                                        *distance, *speed, *max_sleep, 1, new_pos,
                                    );
                                    entity.face_at(new_pos);
                                } else {
                                    entity.action = RandomWalkInSector(
                                        *distance, *speed, *max_sleep, 0, curr_pos,
                                    );
                                }
                            }
                        });
                    } else if *state == 1 {
                        // State 1: Walk towards
                        if target.distance(entity.get_pos_xz()) < 0.1 {
                            // Arrived, Sleep
                            let mut rng = rand::rng();
                            entity.action = self.create_sleep_switch_action(
                                rng.random_range(*max_sleep / 2..=*max_sleep) as u32,
                                RandomWalkInSector(*distance, *speed, *max_sleep, 0, *target),
                            );
                        } else {
                            let t = RandomWalkInSector(*distance, *speed, *max_sleep, 0, *target);
                            let max_sleep = *max_sleep;
                            let blocked = self.move_entity(entity, 1.0, self.entity_block_mode);
                            if blocked {
                                let mut rng = rand::rng();
                                entity.action = self.create_sleep_switch_action(
                                    rng.random_range(max_sleep / 2..=max_sleep) as u32,
                                    t,
                                );
                            }
                        }
                    }
                }
                SleepAndSwitch(tick, action) => {
                    with_regionctx(self.id, |ctx| {
                        if *tick <= ctx.ticks {
                            entity.action = *action.clone();
                        }
                    });
                }
                _ => {}
            }
            if entity.is_dirty() {
                updates.push(entity.get_update().pack());
                entity.clear_dirty();
            }
        }

        with_regionctx(self.id, |ctx| {
            ctx.map.entities = entities;

            // Send the entity updates if non empty
            if !updates.is_empty() {
                self.from_sender
                    .send(RegionMessage::EntitiesUpdate(self.id, updates))
                    .unwrap();
            }

            // let mut items = MAP.borrow().items.clone();
            for item in &mut ctx.map.items {
                if item.is_dirty() {
                    item_updates.push(item.get_update().pack());
                    item.clear_dirty();
                }
            }

            // Send the item updates if non empty
            if !item_updates.is_empty() {
                self.from_sender
                    .send(RegionMessage::ItemsUpdate(self.id, item_updates))
                    .unwrap();
            }
        });

        // Execute delayed scripts for entities
        let mut to_execute_entity = vec![];
        with_regionctx(self.id, |ctx| {
            to_execute_entity = ctx.to_execute_entity.clone();
            ctx.to_execute_entity.clear();
        });
        for todo in to_execute_entity {
            let mut ticks = 0;
            let mut state_data = FxHashMap::default();

            with_regionctx(self.id, |ctx| {
                ctx.curr_entity_id = todo.0;
                ctx.curr_item_id = None;
                state_data = ctx.entity_state_data.clone();
                ticks = ctx.ticks;
            });

            if let Some(state_data) = state_data.get_mut(&todo.0) {
                // Check if we have already executed this script in this tick
                if let Some(Value::Int64(tick)) = state_data.get(&todo.1) {
                    if *tick >= ticks {
                        if todo.1.starts_with("intent") {
                            with_regionctx(self.id, |ctx| {
                                send_message(ctx, todo.0, "{cant_do_that_yet}".into(), "warning");
                            });
                        }
                        continue;
                    }
                }
                // Store the tick we executed this in
                state_data.set(&todo.1, Value::Int64(ticks));
            } else {
                let mut vc = ValueContainer::default();
                vc.set(&todo.1, Value::Int64(ticks));
                state_data.insert(todo.0, vc);
            }

            with_regionctx(self.id, |ctx| {
                ctx.entity_state_data = state_data;
            });

            if let Err(err) = self.execute(&todo.2) {
                send_log_message(
                    self.id,
                    format!(
                        "TO_EXECUTE_ENTITY: Error for '{}': {}: {}",
                        todo.0, todo.1, err,
                    ),
                );
            }
        }

        // Execute delayed scrips for items.
        // This is because we can only borrow REGION once.

        let mut to_execute_item = vec![];
        with_regionctx(self.id, |ctx| {
            to_execute_item = ctx.to_execute_item.clone();
            ctx.to_execute_item.clear();
        });

        for todo in to_execute_item {
            let mut ticks = 0;
            let mut state_data = FxHashMap::default();
            with_regionctx(self.id, |ctx| {
                ctx.curr_item_id = Some(todo.0);
                state_data = ctx.item_state_data.clone();
                ticks = ctx.ticks;
            });

            if let Some(state_data) = state_data.get_mut(&todo.0) {
                // Check if we have already executed this script in this tick
                if let Some(Value::Int64(tick)) = state_data.get(&todo.1) {
                    if *tick >= ticks {
                        continue;
                    }
                }
                // Store the tick we executed this in
                state_data.set(&todo.1, Value::Int64(ticks));
            } else {
                let mut vc = ValueContainer::default();
                vc.set(&todo.1, Value::Int64(ticks));
                state_data.insert(todo.0, vc);
            }

            with_regionctx(self.id, |ctx| {
                ctx.item_state_data = state_data;
            });

            if let Err(err) = self.execute(&todo.2) {
                send_log_message(
                    self.id,
                    format!(
                        "TO_EXECUTE_ITEM: Error for '{}': {}: {}",
                        todo.0, todo.1, err,
                    ),
                );
            }
        }
    }

    /// Execute a script.
    pub fn execute(&self, source: &str) -> Result<PyObjectRef, String> {
        let scope = self.scope.lock().unwrap();

        self.interp.enter(|vm| {
            let rc = vm.run_block_expr(scope.clone(), source);
            match rc {
                Ok(obj) => Ok(obj),
                Err(error) => {
                    let mut err_line: Option<u32> = None;

                    if let Some(tb) = error.__traceback__() {
                        // let file_name = tb.frame.code.source_path.as_str();
                        let instruction_index =
                            tb.frame.lasti.load(std::sync::atomic::Ordering::Relaxed);
                        err_line = Some(instruction_index / 2);
                        // let function_name = tb.frame.code.obj_name.as_str();
                    }

                    let mut err_string = String::new();
                    if let Some(err) = error.args().first() {
                        if let Ok(msg) = err.str(vm) {
                            err_string = msg.to_string();
                        }
                    }

                    if let Some(err_line) = err_line {
                        err_string = format!("{} at line {}.", err_string, err_line);
                    }
                    println!("err {}", err_string);
                    Err(err_string)
                }
            }
        })
    }

    /// Create a sleep action which switches back to the previous action.
    fn create_sleep_switch_action(&self, minutes: u32, switchback: EntityAction) -> EntityAction {
        with_regionctx(self.id, |ctx| {
            let tick = ctx.ticks + (minutes as i64 * ctx.ticks_per_minute as i64) as i64;
            SleepAndSwitch(tick, Box::new(switchback))
        })
        .unwrap()
    }

    /// Moves an entity forward or backward. Returns true if blocked.
    fn move_entity(&self, entity: &mut Entity, dir: f32, entity_block_mode: i32) -> bool {
        with_regionctx(self.id, |ctx| {
            let speed = 4.0 * ctx.delta_time;
            let move_vector = entity.orientation * speed * dir;
            let position = entity.get_pos_xz();
            let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;

            let mut new_position = position + move_vector;

            // We'll do up to N attempts to resolve collisions via sliding
            const MAX_ITERATIONS: usize = 5;

            for _attempt in 0..MAX_ITERATIONS {
                let mut pushed = false; // Track if we had to push/slide this iteration

                // 1) Check collisions with ENTITIES
                for other in ctx.map.entities.iter() {
                    if other.id == entity.id || other.get_mode() == "dead" {
                        continue;
                    }

                    let other_pos = other.get_pos_xz();
                    let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
                    let combined_radius = radius + other_radius;
                    let combined_radius_sq = combined_radius * combined_radius;

                    // Are we colliding now?
                    let dist_vec = new_position - other_pos;
                    let dist_sq = dist_vec.magnitude_squared();
                    if dist_sq < combined_radius_sq {
                        // Send events
                        if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                            let cmd = format!(
                                "{}.event('{}', {})",
                                class_name, "bumped_into_entity", other.id
                            );
                            ctx.to_execute_entity.push((
                                entity.id,
                                "bumped_into_entity".into(),
                                cmd,
                            ));
                        }
                        if let Some(class_name) = ctx.entity_classes.get(&other.id) {
                            let cmd = format!(
                                "{}.event('{}', {})",
                                class_name, "bumped_by_entity", entity.id
                            );
                            ctx.to_execute_entity
                                .push((other.id, "bumped_by_entity".into(), cmd));
                        }

                        // If blocking, we attempt to slide
                        if entity_block_mode > 0 {
                            // Normal from the obstacle center to the entity
                            let normal = dist_vec.normalized();

                            let total_move = new_position - position;
                            let slide = total_move - normal * total_move.dot(normal);

                            let slide_pos = position + slide;
                            let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                            if slide_dist_sq >= combined_radius_sq {
                                // We successfully slid away
                                new_position = slide_pos;
                            } else {
                                // If even after sliding we still collide, we push out just enough
                                // to stand exactly at the boundary
                                let actual_dist = (slide_pos - other_pos).magnitude();
                                if actual_dist < combined_radius {
                                    let push_amount = combined_radius - actual_dist;
                                    new_position = slide_pos + normal * push_amount;
                                    // Re-check again next iteration
                                }
                            }
                            pushed = true;
                        }
                    }
                }

                // 2) Check collisions with ITEMS
                for other in ctx.map.items.iter() {
                    if !other.attributes.get_bool_default("visible", false) {
                        continue;
                    }

                    let other_pos = other.get_pos_xz();
                    let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
                    let combined_radius = radius + other_radius;
                    let combined_radius_sq = combined_radius * combined_radius;

                    let dist_vec = new_position - other_pos;
                    let dist_sq = dist_vec.magnitude_squared();
                    if dist_sq < combined_radius_sq {
                        // Send events
                        if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                            let cmd = format!(
                                "{}.event('{}', {})",
                                class_name, "bumped_into_item", other.id
                            );
                            ctx.to_execute_entity
                                .push((entity.id, "bumped_into_item".into(), cmd));
                        }
                        if let Some(class_name) = ctx.item_classes.get(&other.id) {
                            let cmd = format!(
                                "{}.event('{}', {})",
                                class_name, "bumped_by_entity", entity.id
                            );
                            ctx.to_execute_item
                                .push((other.id, "bumped_by_entity".into(), cmd));
                        }

                        // If item is blocking, we attempt to slide
                        if other.attributes.get_bool_default("blocking", false) {
                            let normal = dist_vec.normalized();

                            let total_move = new_position - position;
                            let slide = total_move - normal * total_move.dot(normal);

                            let slide_pos = position + slide;
                            let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                            if slide_dist_sq >= combined_radius_sq {
                                // we successfully slid away
                                new_position = slide_pos;
                            } else {
                                // If still colliding, push to boundary
                                let actual_dist = (slide_pos - other_pos).magnitude();
                                if actual_dist < combined_radius {
                                    let push_amount = combined_radius - actual_dist;
                                    new_position = slide_pos + normal * push_amount;
                                    // We'll re-check next iteration
                                }
                            }
                            pushed = true;
                        }
                    }
                }

                // If we didn't have to push at all, we’re clear => break early
                if !pushed {
                    break;
                }
            }

            // Now we set the new position after we've done all the entity/item collision resolution
            entity.set_pos_xz(new_position);

            entity.position.y = ctx
                .map
                .terrain
                .sample_height_bilinear(entity.position.x, entity.position.z)
                + 1.5;

            // Finally, let the geometry/linedef collision do its thing (OLD SYSTEM)
            let (end_position, geometry_blocked) =
                ctx.mapmini
                    .move_distance(position, new_position - position, radius);

            // Move the entity after geometry
            entity.set_pos_xz(end_position);

            // NEW COLLISION SYSTEM
            let collision_blocked = {
                let move_vec = end_position - position;
                let start_pos = vek::Vec3::new(
                    position.x,
                    entity.position.y,
                    position.y, /* z component */
                );
                let move_vec_3d = vek::Vec3::new(move_vec.x, 0.0, move_vec.y);
                let (collision_pos, blocked) =
                    ctx.collision_world
                        .move_distance(start_pos, move_vec_3d, radius);

                entity.set_pos_xz(vek::Vec2::new(collision_pos.x, collision_pos.z));
                blocked
            };

            check_player_for_section_change(ctx, entity);
            geometry_blocked || collision_blocked
        })
        .unwrap()
    }

    /// Create a new entity instance.
    pub fn create_entity_instance(&self, mut entity: Entity) {
        entity.id = get_global_id();
        entity.set_attribute(
            "_source_seq",
            Value::Source(PixelSource::Sequence("idle".into())),
        );
        entity.set_attribute("mode", Value::Str("active".into()));
        entity.mark_all_dirty();

        if let Some(class_name) = entity.get_attr_string("class_name") {
            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                ctx.map.entities.push(entity.clone());

                // Setting the data for the entity
                if let Some(data) = ctx.entity_class_data.get(&class_name) {
                    for e in ctx.map.entities.iter_mut() {
                        if e.id == entity.id {
                            apply_entity_data(e, data);

                            // Fill up the inventory slots
                            if let Some(Value::Int(inv_slots)) = e.attributes.get("inventory_slots")
                            {
                                e.inventory = vec![];
                                for _ in 0..*inv_slots {
                                    e.inventory.push(None);
                                }
                            }

                            // Set the wallet
                            if let Some(Value::Int(wealth)) = e.attributes.get("wealth") {
                                _ = e.add_base_currency(*wealth as i64, &ctx.currencies)
                            }
                        }
                    }
                }

                ctx.curr_entity_id = entity.id;

                // Register player
                if ctx.entity_player_classes.contains(&class_name) {
                    if let Some(entity) = get_entity_mut(&mut ctx.map, ctx.curr_entity_id) {
                        entity
                            .set_attribute("player_camera", Value::PlayerCamera(PlayerCamera::D2));
                    }

                    self.from_sender
                        .send(RegisterPlayer(self.id, ctx.curr_entity_id))
                        .unwrap();
                }

                // Register the class for the entity
                ctx.entity_classes.insert(entity.id, class_name.clone());
            });

            // Send "startup" event
            let cmd = format!("{}.event(\"startup\", \"\")", class_name);
            if let Err(err) = self.execute(&cmd) {
                send_log_message(
                    0,
                    format!(
                        "{}: Event Error ({}) for '{}': {}",
                        self.name,
                        "startup",
                        self.get_entity_name(entity.id),
                        err,
                    ),
                );
            }

            // Determine, set and notify the entity about the sector it is in.
            let mut sector_name = String::new();

            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                if let Some(sector) = ctx.map.find_sector_at(entity.get_pos_xz()) {
                    sector_name = sector.name.clone();
                }
                {
                    for e in ctx.map.entities.iter_mut() {
                        if e.id == entity.id {
                            e.attributes.set("sector", Value::Str(sector_name.clone()));
                        }
                    }
                }
            });
            if !sector_name.is_empty() {
                let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector_name);
                _ = self.execute(&cmd);
            }
        }

        // Running the character setup script
        if let Some(setup) = entity.get_attr_string("setup") {
            if let Err(err) = self.execute(&setup) {
                send_log_message(
                    self.id,
                    format!(
                        "{}: Setup '{}/{}': {}",
                        self.name,
                        entity.get_attr_string("name").unwrap_or("Unknown".into()),
                        entity
                            .get_attr_string("class_name")
                            .unwrap_or("Unknown".into()),
                        err,
                    ),
                );
                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                    ctx.error_count += 1;
                });
            }

            with_regionctx(self.id, |ctx: &mut RegionCtx| {
                ctx.curr_entity_id = entity.id;
            });

            if let Err(err) = self.execute("setup()") {
                send_log_message(
                    self.id,
                    format!(
                        "{}: Setup '{}/{}': {}",
                        self.name,
                        entity.get_attr_string("name").unwrap_or("Unknown".into()),
                        entity
                            .get_attr_string("class_name")
                            .unwrap_or("Unknown".into()),
                        err,
                    ),
                );
                with_regionctx(self.id, |ctx: &mut RegionCtx| {
                    ctx.error_count += 1;
                });
            }
        }

        send_log_message(
            self.id,
            format!(
                "{}: Spawned `{}`",
                self.name,
                self.get_entity_name(entity.id),
            ),
        );
    }

    /// Get the name of the entity with the given id.
    fn get_entity_name(&self, id: u32) -> String {
        let mut name = "Unknown".to_string();
        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            for entity in ctx.map.entities.iter() {
                if entity.id == id {
                    if let Some(n) = entity.attributes.get_str("name") {
                        name = n.to_string();
                    }
                }
            }
        });
        name
    }

    /// Send "intent" events for the entity or item at the given position.
    fn send_entity_intent_events(&self, entity: &mut Entity, position: Vec2<f32>) {
        with_regionctx(self.id, |ctx: &mut RegionCtx| {
            if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                // Send "intent" event for the entity
                let mut cont = ValueContainer::default();
                cont.set("distance", Value::Float(1.0));

                let mut target_item_id = None;
                let mut target_entity_id = None;

                let mut found_target = false;
                if let Some(entity_id) = get_entity_at(ctx, position, entity.id) {
                    if entity_id != entity.id {
                        cont.set("entity_id", Value::UInt(entity.id));
                        cont.set("target_id", Value::UInt(entity_id));
                        if let Some(i_id) = get_item_at(ctx, position) {
                            cont.set("item_id", Value::UInt(i_id));
                        }
                        target_entity_id = Some(entity_id);
                        found_target = true;
                    }
                }
                if !found_target {
                    if let Some(i_id) = get_item_at(ctx, position) {
                        cont.set("entity_id", Value::UInt(entity.id));
                        cont.set("item_id", Value::UInt(i_id));
                        target_item_id = Some(i_id);
                        found_target = true;
                    }
                }

                let intent = entity.attributes.get_str_default("intent", "".into());

                if !found_target {
                    let message = format!("{{nothing_to_{}}}", intent);
                    entity.set_attribute("intent", Value::Str(String::new()));
                    send_message(ctx, entity.id, message, "system");
                    return;
                }

                let event_name = format!("intent: {}", intent);

                cont.set("intent", Value::Str(intent));
                let cmd = format!(
                    "{}.event('intent', {})",
                    class_name,
                    cont.to_python_dict_string()
                );
                ctx.to_execute_entity
                    .push((entity.id, event_name.clone(), cmd.clone()));

                if let Some(target_entity_id) = target_entity_id {
                    if let Some(class_name) = ctx.entity_classes.get(&target_entity_id) {
                        let cmd = format!(
                            "{}.event('intent', {})",
                            class_name,
                            cont.to_python_dict_string()
                        );
                        ctx.to_execute_entity
                            .push((target_entity_id, event_name, cmd));
                    }
                } else if let Some(item_id) = target_item_id {
                    if let Some(class_name) = ctx.item_classes.get(&item_id) {
                        let cmd = format!(
                            "{}.event('intent', {})",
                            class_name,
                            cont.to_python_dict_string()
                        );
                        ctx.to_execute_item.push((item_id, event_name, cmd));
                    }
                }

                entity.set_attribute("intent", Value::Str(String::new()));
            }
        });
    }

    /// Returns the entities in the radius of the character or item.
    fn entities_in_radius(
        &self,
        ctx: &RegionCtx,
        entity_id: Option<u32>,
        item_id: Option<u32>,
        radius: f32,
    ) -> Vec<u32> {
        let mut position = None;
        let mut is_entity = false;
        let mut id = 0;

        if let Some(item_id) = item_id {
            if let Some(item) = ctx.map.items.iter().find(|item| item.id == item_id) {
                id = item_id;
                position = Some(item.get_pos_xz());
            }
        } else if let Some(entity_id) = entity_id {
            is_entity = true;
            if let Some(entity) = ctx
                .map
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
            {
                id = entity.id;
                position = Some(entity.get_pos_xz());
            }
        }

        let mut entities = Vec::new();

        if let Some(position) = position {
            for other in ctx.map.entities.iter() {
                if is_entity && other.id == id {
                    continue;
                }
                let other_position = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5);

                let distance_squared = (position - other_position).magnitude_squared();
                let combined_radius = radius + other_radius;
                let combined_radius_squared = combined_radius * combined_radius;

                // Entity is inside the radius
                if distance_squared < combined_radius_squared {
                    entities.push(other.id);
                }
            }
        }

        entities
    }
}

/// Check if the player moved to a different sector and if yes send "enter" and "left" events
fn check_player_for_section_change(ctx: &mut RegionCtx, entity: &mut Entity) {
    // Determine, set and notify the entity about the sector it is in.
    if let Some(sector) = ctx.map.find_sector_at(entity.get_pos_xz()) {
        if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
            if sector.name != *old_sector_name {
                if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                    // Send entered event
                    if !sector.name.is_empty() {
                        let cmd = format!("{}.event(\"entered\", \"{}\")", class_name, sector.name);
                        // println!("{cmd}");
                        ctx.to_execute_entity
                            .push((entity.id, "bumped_into_item".into(), cmd));
                    }
                    // Send left event
                    if !old_sector_name.is_empty() {
                        let cmd =
                            format!("{}.event(\"left\", \"{}\")", class_name, old_sector_name);
                        // println!("{cmd}");
                        ctx.to_execute_entity
                            .push((entity.id, "bumped_into_item".into(), cmd));
                    }
                }

                entity
                    .attributes
                    .set("sector", Value::Str(sector.name.clone()));
            }
        }
    } else if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
        // Send left event
        if !old_sector_name.is_empty() {
            if let Some(class_name) = ctx.entity_classes.get(&entity.id) {
                let cmd = format!("{}.event(\"left\", \"{}\")", class_name, old_sector_name);
                // println!("{cmd}");
                ctx.to_execute_entity
                    .push((entity.id, "bumped_into_item".into(), cmd));
            }
        }
        entity.attributes.set("sector", Value::Str(String::new()));
    }
}

/// Set Player Camera
fn set_player_camera(camera: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let player_camera = match camera.as_str() {
            "iso" => PlayerCamera::D3Iso,
            "firstp" => PlayerCamera::D3FirstP,
            _ => PlayerCamera::D2,
        };

        if let Some(entity) = get_entity_mut(&mut ctx.map, ctx.curr_entity_id) {
            entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
        }
    });
}

/// Is the given entity dead.
pub fn is_entity_dead(region_id: u32, id: u32) -> bool {
    let mut v = false;
    with_regionctx(region_id, |ctx: &mut RegionCtx| {
        for entity in &ctx.map.entities {
            if entity.id == id {
                v = entity.attributes.get_str_default("mode", "active".into()) == "dead";
            }
        }
    });
    v
}

/// Is the given entity dead.
pub fn is_entity_dead_ctx(ctx: &RegionCtx, id: u32) -> bool {
    let mut v = false;
    for entity in &ctx.map.entities {
        if entity.id == id {
            v = entity.attributes.get_str_default("mode", "active".into()) == "dead";
        }
    }
    v
}

/// Search for a mutable reference to an item with the given ID. Checks the map and on each entity.
fn get_item_mut<'a>(map: &'a mut Map, item_id: u32) -> Option<&'a mut Item> {
    // Look in the top-level items
    if let Some(item) = map.items.iter_mut().find(|item| item.id == item_id) {
        return Some(item);
    }
    // Look in each entity’s inventory
    for entity in map.entities.iter_mut() {
        for item in entity.inventory.iter_mut() {
            if let Some(item) = item {
                if item.id == item_id {
                    return Some(item);
                }
            }
        }
    }
    None
}

/// Search for a mutable reference to an entity with the given ID.
fn get_entity_mut<'a>(map: &'a mut Map, entity_id: u32) -> Option<&'a mut Entity> {
    // Look in the top-level items
    if let Some(entity) = map
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        return Some(entity);
    }
    None
}

/// Sets light emission to on / off
fn set_emit_light(value: bool, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                if let Some(Value::Light(light)) = item.attributes.get_mut("light") {
                    light.active = value;
                    item.mark_dirty_attribute("light");
                }
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                if let Some(Value::Light(light)) = entity.attributes.get_mut("light") {
                    light.active = value;
                    entity.mark_dirty_attribute("light");
                }
            }
        }
    });
}

/// Set the tile_id of the current entity or item.
fn set_tile(id: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Ok(uuid) = Uuid::try_parse(&id) {
            if let Some(item_id) = ctx.curr_item_id {
                if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                    item.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                }
            } else {
                let entity_id = ctx.curr_entity_id;
                if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                    entity.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                }
            }
        }
    });
}

/// Set rigging sequence
pub fn set_rig_sequence(
    args: rustpython_vm::function::FuncArgs,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let mut sequence = vec![];

    for arg in args.args.iter() {
        if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
            sequence.push(v);
        }
    }

    Ok(())
}

/// Take the given item.
fn take(item_id: u32, vm: &VirtualMachine) -> bool {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        let mut rc = true;

        if let Some(pos) = ctx.map.items.iter().position(|item| {
            item.id == item_id && !item.attributes.get_bool_default("static", false)
        }) {
            let item = ctx.map.items.remove(pos);

            if let Some(entity) = ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
            {
                let mut item_name = "Unknown".to_string();
                if let Some(name) = item.attributes.get_str("name") {
                    item_name = name.to_string();
                }

                fn article_for(item_name: &str) -> (&'static str, String) {
                    let name = item_name.to_ascii_lowercase();

                    let pair_items = ["trousers", "pants", "gloves", "boots", "scissors"];
                    let mass_items = ["armor", "cloth", "water", "meat"];

                    if pair_items.contains(&name.as_str()) {
                        ("a pair of", item_name.to_string())
                    } else if mass_items.contains(&name.as_str()) {
                        ("some", item_name.to_string())
                    } else {
                        let first = name.chars().next().unwrap_or('x');
                        let article = match first {
                            'a' | 'e' | 'i' | 'o' | 'u' => "an",
                            _ => "a",
                        };
                        (article, item_name.to_string())
                    }
                }

                let mut message = format!(
                    "You take {} {}",
                    article_for(&item_name.to_lowercase()).0,
                    item_name.to_lowercase()
                );

                if item.attributes.get_bool_default("monetary", false) {
                    // This is not a standalone item but money
                    let amount = item.attributes.get_int_default("worth", 0);
                    if amount > 0 {
                        message = format!("You take {} gold.", amount);
                        _ = entity.add_base_currency(amount as i64, &ctx.currencies);
                    }
                } else if entity.add_item(item).is_err() {
                    // TODO: Send message.
                    println!("Take: Too many items");
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Inventory Full".into()), true);
                    }
                    rc = false;
                }

                if ctx.debug_mode && rc {
                    add_debug_value(ctx, TheValue::Text("Ok".into()), false);
                }

                ctx.from_sender
                    .get()
                    .unwrap()
                    .send(RegionMessage::RemoveItem(ctx.region_id, item_id))
                    .unwrap();

                let msg = RegionMessage::Message(
                    ctx.region_id,
                    Some(entity_id),
                    None,
                    entity_id,
                    message,
                    "system".into(),
                );
                ctx.from_sender.get().unwrap().send(msg).unwrap();
            }
        } else {
            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
            }
        }
        rc
    })
    .unwrap()
}

/// Block the events for the entity / item for the given amount of minutes.
pub fn block_events(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let mut minutes: f32 = 4.0;
        let mut events: Vec<String> = Vec::new();

        for (i, arg) in args.args.iter().enumerate() {
            if i == 0 {
                if let Some(v) = Value::from_pyobject(arg.clone(), vm).and_then(|v| v.to_f32()) {
                    minutes = v;
                }
            } else if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                events.push(v);
            }
        }

        let target_tick = Value::Int64(ctx.ticks + (ctx.ticks_per_minute as f32 * minutes) as i64);

        if let Some(item_id) = ctx.curr_item_id {
            let state_data = &mut ctx.item_state_data;
            if let Some(state_data) = state_data.get_mut(&item_id) {
                for event in events {
                    state_data.set(&event, target_tick.clone());
                }
            } else {
                let mut vc = ValueContainer::default();
                for event in events {
                    vc.set(&event, target_tick.clone());
                }
                state_data.insert(item_id, vc);
            }
        } else {
            let entity_id = ctx.curr_entity_id;

            let state_data = &mut ctx.entity_state_data;
            if let Some(state_data) = state_data.get_mut(&entity_id) {
                for event in events {
                    state_data.set(&event, target_tick.clone());
                }
            } else {
                let mut vc = ValueContainer::default();
                for event in events {
                    vc.set(&event, target_tick.clone());
                }
                state_data.insert(entity_id, vc);
            }
        }
    });
}

/// Deal damage to the given entity. Sends an "take_damage" event to the other entity.
fn deal_damage(id: u32, dict: PyObjectRef, vm: &VirtualMachine) {
    let dict = extract_dictionary(dict, vm);

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Ok(dict) = dict {
            if let Some(entity) = ctx.map.entities.iter().find(|entity| entity.id == id) {
                if let Some(class_name) = entity.attributes.get_str("class_name") {
                    let cmd = format!("{}.event('{}', {})", class_name, "take_damage", dict);
                    ctx.to_execute_entity.push((id, "take_damage".into(), cmd));
                }
            } else if let Some(item) = ctx.map.items.iter_mut().find(|item| item.id == id) {
                if let Some(class_name) = item.attributes.get_str("class_name") {
                    let cmd = format!("{}.event('{}', {})", class_name, "take_damage", dict);
                    ctx.to_execute_item.push((id, "take_damage".into(), cmd));
                }
            }
        }
    });
}

/// Send a message to the entity.
fn send_message(ctx: &RegionCtx, id: u32, message: String, role: &str) {
    let msg = RegionMessage::Message(ctx.region_id, Some(id), None, id, message, role.to_string());
    ctx.from_sender.get().unwrap().send(msg).unwrap();
}

/// An entity took damage. Send out messages and check for death.
fn took_damage(from: u32, mut amount: i32, vm: &VirtualMachine) {
    let mut kill = false;

    // Make sure we don't heal by accident
    amount = amount.max(0);
    if amount == 0 {
        return;
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let id = ctx.curr_entity_id;

        // Check for death
        if let Some(entity) = get_entity_mut(&mut ctx.map, id) {
            let health_attr = ctx.health_attr.clone();
            if let Some(mut health) = entity.attributes.get_int(&health_attr) {
                // Reduce the health of the target
                health -= amount;
                health = health.max(0);
                // Set the new health
                entity.set_attribute(&health_attr, Value::Int(health));

                let mode = entity.attributes.get_str_default("mode", "".into());
                if health <= 0 && mode != "dead" {
                    // Send "death" event
                    if let Some(class_name) = entity.attributes.get_str("class_name") {
                        let cmd = format!("{}.event(\"death\", \"\")", class_name);
                        ctx.to_execute_entity.push((entity.id, "death".into(), cmd));

                        entity.set_attribute("mode", Value::Str("dead".into()));
                        entity.action = EntityAction::Off;
                        ctx.entity_proximity_alerts.remove(&entity.id);

                        kill = true;
                    }
                }
            }
        }

        // if receiver got killed, send a "kill" event to the attacker
        if kill {
            if let Some(entity) = get_entity_mut(&mut ctx.map, from) {
                // Send "kill" event
                if let Some(class_name) = entity.attributes.get_str("class_name") {
                    let cmd = format!("{}.event(\"kill\", {})", class_name, id);
                    ctx.to_execute_entity.push((from, "kill".into(), cmd));
                }
            }
        }
    });
}

/// Get an attribute from the given entity.
fn get_attr_of(id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            if let Some(v) = entity.attributes.get(&key) {
                value = v.clone();
            }
        }

        if ctx.debug_mode {
            if value != Value::NoValue {
                add_debug_value(ctx, TheValue::Text(value.to_string()), false);
            }
        }
    });

    if value == Value::NoValue {
        let item_id = id;
        with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                if let Some(v) = item.get_attribute(&key) {
                    value = v.clone();
                }
            }

            if ctx.debug_mode {
                if value == Value::NoValue {
                    add_debug_value(ctx, TheValue::Text("Not Found".into()), true);
                } else {
                    add_debug_value(ctx, TheValue::Text(value.to_string()), false);
                }
            }
        });
    }

    Ok(value.to_pyobject(vm))
}

/*
/// Get an attribute from the given entity.
fn get_entity_attr(entity_id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            if let Some(v) = entity.attributes.get(&key) {
                value = v.clone();
            }
        }

        if ctx.debug_mode {
            if value == Value::NoValue {
                add_debug_value(ctx, Value::Str("Not Found".into()), true);
            }
        }
    });

    Ok(value.to_pyobject(vm))
}

/// Get an attribute from the given item.
fn get_item_attr(item_id: u32, key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
            if let Some(v) = item.get_attribute(&key) {
                value = v.clone();
            }
        }

        if ctx.debug_mode {
            if value == Value::NoValue {
                add_debug_value(ctx, Value::Str("Not Found".into()), true);
            }
        }
    });

    Ok(value.to_pyobject(vm))
}
*/

/// Get an attribute from the current item or entity.
fn get_attr(key: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut value = Value::NoValue;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                if let Some(v) = item.get_attribute(&key) {
                    value = v.clone();
                }
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                if let Some(v) = entity.attributes.get(&key) {
                    value = v.clone();
                }
            }
        }

        if ctx.debug_mode {
            if value == Value::NoValue {
                add_debug_value(ctx, TheValue::Text("Not Found".into()), true);
            } else {
                add_debug_value(ctx, TheValue::Text(value.to_string()), false);
            }
        }
    });

    Ok(value.to_pyobject(vm))
}

/// Toggles a boolean attribute of the current entity or item.
fn toggle_attr(key: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                item.attributes.toggle(&key);
                if key == "active" {
                    // Send active state
                    if let Some(class_name) = item.attributes.get_str("class_name") {
                        let cmd = format!(
                            "{}.event(\"active\", {})",
                            class_name,
                            if item.attributes.get_bool_default("active", false) {
                                "True"
                            } else {
                                "False"
                            }
                        );
                        ctx.to_execute_item.push((item.id, "active".into(), cmd));
                    }
                }
            } else {
                let entity_id = ctx.curr_entity_id;
                if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                    entity.attributes.toggle(&key);
                }
            }
        }
    });
}

/// Set the attribute of the current entity or item.
fn set_attr(key: PyObjectRef, value: PyObjectRef, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Ok(key) = String::try_from_object(vm, key) {
            if let Some(value) = Value::from_pyobject(value, vm) {
                if let Some(item_id) = ctx.curr_item_id {
                    if let Some(item) = get_item_mut(&mut ctx.map, item_id) {
                        item.set_attribute(&key, value);

                        if key == "active" {
                            // Send active state
                            if let Some(class_name) = item.attributes.get_str("class_name") {
                                let cmd = format!(
                                    "{}.event(\"active\", {})",
                                    class_name,
                                    if item.attributes.get_bool_default("active", false) {
                                        "True"
                                    } else {
                                        "False"
                                    }
                                );
                                ctx.to_execute_item.push((item.id, "active".into(), cmd));
                            }
                        }
                    }
                } else {
                    let entity_id = ctx.curr_entity_id;
                    if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                        entity.set_attribute(&key, value);
                    }
                }
            }
        }
    });
}

/// Returns a list of filtered inventory items.
fn inventory_items_of(
    entity_id: u32,
    filter: String,
    vm: &VirtualMachine,
) -> PyResult<PyObjectRef> {
    let mut items = Vec::new();

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(entity) = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
        {
            for (_, item) in entity.iter_inventory() {
                let name = item.attributes.get_str("name").unwrap_or_default();
                let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                    items.push(item.id);
                }
            }
        }
    });

    let py_list = vm.ctx.new_list(
        items
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Returns a list of filtered inventory items.
fn inventory_items(filter: String, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut items = Vec::new();

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;

        if let Some(entity) = ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
        {
            for (_, item) in entity.iter_inventory() {
                let name = item.attributes.get_str("name").unwrap_or_default();
                let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                    items.push(item.id);
                }
            }
        }
    });

    let py_list = vm.ctx.new_list(
        items
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Drop the item with the given id.
fn drop(item_id: u32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        let mut slot = None;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            // Collect matching slot indices
            for (index, item) in entity.inventory.iter().enumerate() {
                if let Some(item) = item {
                    if item.id == item_id {
                        slot = Some(index);
                        break;
                    }
                }
            }

            let mut removed_items = Vec::new();
            if let Some(slot) = slot {
                if let Some(mut item) = entity.remove_item_from_slot(slot) {
                    item.position = entity.position;
                    item.mark_all_dirty();
                    removed_items.push(item);
                }
            }

            for item in removed_items {
                ctx.map.items.push(item);
            }
        }
    });
}

/// Drop the given items.
fn drop_items(filter: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            // Collect matching slot indices
            let matching_slots: Vec<usize> = entity
                .iter_inventory()
                .filter_map(|(slot, item)| {
                    let name = item.attributes.get_str("name").unwrap_or_default();
                    let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                    if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                        Some(slot)
                    } else {
                        None
                    }
                })
                .collect();

            // Remove matching items from slots
            let mut removed_items = Vec::new();
            for slot in matching_slots {
                if let Some(mut item) = entity.remove_item_from_slot(slot) {
                    item.position = entity.position;
                    item.mark_all_dirty();
                    removed_items.push(item);
                }
            }

            for item in removed_items {
                ctx.map.items.push(item);
            }
        }
    });
}

/// Offer inventory.
fn offer_inventory(to: u32, filter: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            // Collect matching slot indices
            let matching_item_ids: Vec<u32> = entity
                .iter_inventory()
                .filter_map(|(_, item)| {
                    let name = item.attributes.get_str("name").unwrap_or_default();
                    let class_name = item.attributes.get_str("class_name").unwrap_or_default();

                    if filter.is_empty() || name.contains(&filter) || class_name.contains(&filter) {
                        Some(item.id)
                    } else {
                        None
                    }
                })
                .collect();

            let mut choices = MultipleChoice::new(ctx.region_id, entity_id, to);
            for item_id in matching_item_ids {
                let choice = Choice::ItemToSell(item_id, entity_id, to);
                choices.add(choice);
            }

            ctx.from_sender
                .get()
                .unwrap()
                .send(RegionMessage::MultipleChoice(choices))
                .unwrap();
        }
    });
}

/// Returns the entity at the given position (if any)
fn get_entity_at(ctx: &RegionCtx, position: Vec2<f32>, but_not: u32) -> Option<u32> {
    let mut entity = None;

    for other in ctx.map.entities.iter() {
        if other.id == but_not {
            continue;
        }
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            entity = Some(other.id);
            break; // We only need the first item found
        }
    }

    entity
}

/// Returns the item at the given position (if any)
fn get_item_at(ctx: &RegionCtx, position: Vec2<f32>) -> Option<u32> {
    let mut item = None;

    for other in ctx.map.items.iter() {
        let other_position = other.get_pos_xz();

        let distance = position.distance(other_position);

        // Item is inside the radius
        if distance < 1.0 {
            item = Some(other.id);
            break; // We only need the first item found
        }
    }

    item
}

/// Returns the entities in the radius of the character or item.
fn entities_in_radius(vm: &VirtualMachine) -> PyResult<PyObjectRef> {
    let mut radius = 0.5;
    let mut position = None;
    let mut is_entity = false;
    let mut id = 0;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if let Some(item) = ctx.map.items.iter().find(|item| item.id == item_id) {
                id = item_id;
                position = Some(item.get_pos_xz());
                radius = item.attributes.get_float_default("radius", 0.5);
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            is_entity = true;
            if let Some(entity) = ctx
                .map
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
            {
                id = entity.id;
                position = Some(entity.get_pos_xz());
                radius = entity.attributes.get_float_default("radius", 0.5);
            }
        }
    });

    let mut entities = Vec::new();

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(position) = position {
            for other in ctx.map.entities.iter() {
                if is_entity && other.id == id {
                    continue;
                }
                let other_position = other.get_pos_xz();
                let other_radius = other.attributes.get_float_default("radius", 0.5);

                let distance_squared = (position - other_position).magnitude_squared();
                let combined_radius = radius + other_radius;
                let combined_radius_squared = combined_radius * combined_radius;

                // Entity is inside the radius
                if distance_squared < combined_radius_squared {
                    entities.push(other.id);
                }
            }
        }
    });

    let py_list = vm.ctx.new_list(
        entities
            .iter()
            .map(|&id| vm.ctx.new_int(id).into())
            .collect::<Vec<PyObjectRef>>(),
    );

    Ok(py_list.into())
}

/// Add an item to the characters inventory
fn add_item(class_name: String, vm: &VirtualMachine) -> i32 {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item) = create_item(ctx, class_name.clone()) {
            let id = ctx.curr_entity_id;
            if let Some(entity) = ctx.map.entities.iter_mut().find(|entity| entity.id == id) {
                let item_id = item.id;
                if entity.add_item(item).is_ok() {
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Ok".into()), false);
                    }
                    item_id as i32
                } else {
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Inventory Full".into()), true);
                    }
                    println!("add_item ({}): Inventory is full", class_name);
                    -1
                }
            } else {
                -1
            }
        } else {
            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
            }
            -1
        }
    })
    .unwrap()
}

/// Add a debug value at the current debug position
#[inline(always)]
pub fn add_debug_value(ctx: &mut RegionCtx, value: TheValue, error: bool) {
    if let Some((event, x, y)) = &ctx.curr_debug_loc {
        if let Some(item_id) = ctx.curr_item_id {
            ctx.debug.add_value(item_id, event, *x, *y, value);
            if error {
                ctx.debug.add_error(item_id, event, *x, *y);
            } else {
                ctx.debug.remove_error(item_id, event, *x, *y);
            }
        } else {
            ctx.debug
                .add_value(ctx.curr_entity_id, event, *x, *y, value);
            if error {
                ctx.debug.add_error(ctx.curr_entity_id, event, *x, *y);
            } else {
                ctx.debug.remove_error(ctx.curr_entity_id, event, *x, *y);
            }
        }

        ctx.curr_debug_loc = None;
    }
}

/// Equip the item with the given item id.
fn equip(item_id: u32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let id = ctx.curr_entity_id;
        if let Some(entity) = ctx.map.entities.iter_mut().find(|entity| entity.id == id) {
            let mut slot: Option<String> = None;
            if let Some(item) = entity.get_item(item_id) {
                if let Some(sl) = item.attributes.get_str("slot") {
                    slot = Some(sl.to_string());
                }
            }

            if let Some(slot) = slot {
                if entity.equip_item(item_id, &slot).is_err() {
                    println!("Equipped failure");
                } else {
                    if ctx.debug_mode {
                        add_debug_value(ctx, TheValue::Text("Ok".into()), false);
                    }
                }
            } else {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
                }
            }
        }
    });
}

/// Notify the entity / item in the given amount of minutes.
fn notify_in(minutes: i32, notification: String, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let tick = ctx.ticks + (minutes as u32 * ctx.ticks_per_minute) as i64;
        if let Some(item_id) = ctx.curr_item_id {
            ctx.notifications_items.push((item_id, tick, notification));
        } else {
            if !is_entity_dead_ctx(ctx, ctx.curr_entity_id) {
                ctx.notifications_entities
                    .push((ctx.curr_entity_id, tick, notification));
            }
        }
    });
}

/*
/// Returns the name of the sector the entity or item is currently in.
fn get_sector_name() -> String {
    let map = MAP.borrow();

    if let Some(item_id) = *CURR_ITEMID.borrow() {
        for e in map.items.iter() {
            if e.id == item_id {
                let pos = e.get_pos_xz();
                if let Some(s) = map.find_sector_at(pos) {
                    if s.name.is_empty() {
                        return "Unnamed Sector".to_string();
                    } else {
                        return s.name.clone();
                    }
                }
            }
        }
    } else {
        for e in map.entities.iter() {
            if e.id == *CURR_ENTITYID.borrow() {
                let pos = e.get_pos_xz();
                if let Some(s) = map.find_sector_at(pos) {
                    if s.name.is_empty() {
                        return "Unnamed Sector".to_string();
                    } else {
                        return s.name.clone();
                    }
                }
            }
        }
    }

    "Not inside any sector".to_string()
}

/// Faces the entity at a random direction.
fn face_random() {
    let entity_id = *CURR_ENTITYID.borrow();
    if let Some(entity) = MAP
        .borrow_mut()
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    {
        entity.face_random();
    }
}*/

/// Goto a destination sector with the given speed.
fn goto(destination: String, speed: f32, vm: &VirtualMachine) {
    let mut coord: Option<vek::Vec2<f32>> = None;

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        for sector in &ctx.map.sectors {
            if sector.name == destination {
                coord = sector.center(&ctx.map);
            }
        }

        if let Some(coord) = coord {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
            {
                entity.action = Goto(coord, speed);
            }
        } else {
            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Unknown Sector".into()), true);
            }
        }
    });
}

/// CloseIn: Move within a radius of a target entity with a given speed
fn close_in(target: u32, target_radius: f32, speed: f32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.action = CloseIn(target, target_radius, speed);
        }
    });
}

/// Randomly walks
fn random_walk(
    distance: PyObjectRef,
    speed: PyObjectRef,
    max_sleep: PyObjectRef,
    vm: &VirtualMachine,
) {
    let distance: f32 = get_f32(distance, 1.0, vm);
    let speed: f32 = get_f32(speed, 1.0, vm);
    let max_sleep: i32 = get_i32(max_sleep, 0, vm);

    with_regionctx(get_region_id(vm).unwrap(), |ctx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.action = RandomWalk(distance, speed, max_sleep, 0, zero());
        }
    });
}

/// Randomly walks within the current sector.
fn random_walk_in_sector(
    distance: PyObjectRef,
    speed: PyObjectRef,
    max_sleep: PyObjectRef,
    vm: &VirtualMachine,
) {
    let distance: f32 = get_f32(distance, 1.0, vm); // Default distance: 1.0
    let speed: f32 = get_f32(speed, 1.0, vm); // Default speed: 1.0
    let max_sleep: i32 = get_i32(max_sleep, 0, vm); // Default max_sleep: 0

    with_regionctx(get_region_id(vm).unwrap(), |ctx| {
        let entity_id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
            entity.action = RandomWalkInSector(distance, speed, max_sleep, 0, zero());
        }
    });
}

/// Set Proximity Tracking
pub fn set_proximity_tracking(
    args: rustpython_vm::function::FuncArgs,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let mut turn_on = false;
    let mut distance = 5.0;

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Bool(v)) = Value::from_pyobject(arg.clone(), vm) {
                turn_on = v;
            }
        } else if i == 1 {
            if let Some(Value::Float(v)) = Value::from_pyobject(arg.clone(), vm) {
                distance = v;
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(item_id) = ctx.curr_item_id {
            if turn_on {
                ctx.item_proximity_alerts.insert(item_id, distance);
            } else {
                ctx.item_proximity_alerts.remove(&item_id);
            }
        } else {
            let entity_id = ctx.curr_entity_id;
            if turn_on {
                ctx.entity_proximity_alerts.insert(entity_id, distance);
            } else {
                ctx.entity_proximity_alerts.remove(&entity_id);
            }
        }
    });

    Ok(())
}

/// Teleport
pub fn teleport(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut sector_name = String::new();
    let mut region_name = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                sector_name = v.clone();
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                region_name = v.clone();
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if region_name.is_empty() {
            // Teleport entity in this region to the given sector.

            let mut new_pos: Option<vek::Vec2<f32>> = None;
            for sector in &ctx.map.sectors {
                if sector.name == sector_name {
                    new_pos = sector.center(&ctx.map);
                }
            }

            if let Some(new_pos) = new_pos {
                let entity_id = ctx.curr_entity_id;
                let mut entities = ctx.map.entities.clone();
                if let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) {
                    entity.set_pos_xz(new_pos);
                    check_player_for_section_change(ctx, entity);
                }
                ctx.map.entities = entities;
            } else {
                if ctx.debug_mode {
                    add_debug_value(ctx, TheValue::Text("Unknown Sector".into()), true);
                }
            }
        } else {
            // Remove the entity from this region and send it to the server to be moved
            // into a new region.

            let entity_id = ctx.curr_entity_id;
            if let Some(pos) = ctx.map.entities.iter().position(|e| e.id == entity_id) {
                let removed = ctx.map.entities.remove(pos);

                ctx.entity_classes.remove(&removed.id);

                let msg =
                    RegionMessage::TransferEntity(ctx.region_id, removed, region_name, sector_name);
                ctx.from_sender.get().unwrap().send(msg).unwrap();
            }
        }
    });

    Ok(())
}

/// Message
pub fn message(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut receiver = None;
    let mut message = None;
    let mut category = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        if i == 0 {
            if let Some(Value::UInt(v)) = Value::from_pyobject(arg.clone(), vm) {
                receiver = Some(v);
            } else if let Some(Value::Int(v)) = Value::from_pyobject(arg.clone(), vm) {
                receiver = Some(v as u32);
            }
        } else if i == 1 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                message = Some(v);
            }
        } else if i == 2 {
            if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                category = v.clone();
            }
        }
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if receiver.is_some() && message.is_some() {
            let mut entity_id = Some(ctx.curr_entity_id);
            let item_id = ctx.curr_item_id;
            if item_id.is_some() {
                entity_id = None;
            }

            let message = message.unwrap();
            let msg = RegionMessage::Message(
                ctx.region_id,
                entity_id,
                item_id,
                receiver.unwrap() as u32,
                message,
                category,
            );
            ctx.from_sender.get().unwrap().send(msg).unwrap();

            if ctx.debug_mode {
                add_debug_value(ctx, TheValue::Text("Ok".into()), false);
            }
        }
    });

    Ok(())
}

/// Debug
pub fn debug(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
    let mut output = String::new();

    for (i, arg) in args.args.iter().enumerate() {
        let arg_str = match vm.call_method(arg.as_object(), "__repr__", ()) {
            Ok(repr_obj) => match repr_obj.str(vm) {
                Ok(s) => s.as_str().to_owned(),
                Err(_) => "<error converting repr to str>".to_owned(),
            },
            Err(_) => "<error calling __repr__>".to_owned(),
        };

        if i > 0 {
            output.push(' ');
        }
        output.push_str(&arg_str);
    }

    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        if let Some(name) = get_attr_internal(ctx, "name") {
            output = format!("{}: {}", name, output);
        }
    });

    send_log_message(get_region_id(vm).unwrap(), output);
    Ok(())
}

/// Send a log message.
pub fn send_log_message(id: u32, message: String) {
    with_regionctx(id, |ctx| {
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::LogMessage(message))
            .unwrap();
    });
}

/// Get an i32 config value
fn get_config_i32_default(ctx: &RegionCtx, table: &str, key: &str, default: i32) -> i32 {
    let mut value = default;
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(val) = game.get(key) {
            if let Some(v) = val.as_integer() {
                value = v as i32;
            }
        }
    }
    value
}

/*
fn _get_config_f32_default(table: &str, key: &str, default: f32) -> f32 {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_float() {
                return v as f32;
            }
        }
    }
    default
}

fn _get_config_bool_default(table: &str, key: &str, default: bool) -> bool {
    let tab = CONFIG.borrow();
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(value) = game.get(key) {
            if let Some(v) = value.as_bool() {
                return v;
            }
        }
    }
    default
}
*/

fn get_config_string_default(ctx: &RegionCtx, table: &str, key: &str, default: &str) -> String {
    let mut value = default.to_string();
    let tab = &ctx.config;
    if let Some(game) = tab.get(table).and_then(toml::Value::as_table) {
        if let Some(val) = game.get(key) {
            if let Some(v) = val.as_str() {
                value = v.to_string();
            }
        }
    }
    value
}

/// Get an attribute value from the current item or entity.
fn get_attr_internal(ctx: &mut RegionCtx, key: &str) -> Option<Value> {
    if let Some(id) = ctx.curr_item_id {
        if let Some(item) = get_item_mut(&mut ctx.map, id) {
            return item.attributes.get(key).cloned();
        }
    } else {
        let id = ctx.curr_entity_id;
        if let Some(entity) = get_entity_mut(&mut ctx.map, id) {
            return entity.attributes.get(key).cloned();
        }
    };

    None
}

/// Create a new item with the given class name.
fn create_item(ctx: &mut RegionCtx, class_name: String) -> Option<Item> {
    if !ctx.assets.items.contains_key(&class_name) {
        return None;
    }

    let id = get_global_id();
    let mut item = Item {
        id,
        ..Default::default()
    };

    item.set_attribute("class_name", Value::Str(class_name.clone()));
    item.set_attribute("name", Value::Str(class_name.clone()));

    // Setting the data for the item.
    if let Some(data) = ctx.item_class_data.get(&class_name) {
        apply_item_data(&mut item, data);
    }

    if let Some(class_name) = item.get_attr_string("class_name") {
        let cmd = format!("{}.event(\"startup\", \"\")", class_name);
        ctx.item_classes.insert(item.id, class_name.clone());
        ctx.to_execute_item.push((item.id, "startup".into(), cmd));
    }

    item.mark_all_dirty();

    // Send active state
    let cmd = format!(
        "{}.event(\"active\", {})",
        class_name,
        if item.attributes.get_bool_default("active", false) {
            "True"
        } else {
            "False"
        }
    );
    ctx.to_execute_item.push((item.id, "active".into(), cmd));

    Some(item)
}

/// Received an entity from another region
pub fn receive_entity(ctx: &mut RegionCtx, mut entity: Entity, dest_sector_name: String) {
    entity.action = EntityAction::Off;

    let mut entities = ctx.map.entities.clone();

    let mut new_pos: Option<vek::Vec2<f32>> = None;
    for sector in &ctx.map.sectors {
        if sector.name == dest_sector_name {
            new_pos = sector.center(&ctx.map);
        }
    }

    if let Some(new_pos) = new_pos {
        entity.set_pos_xz(new_pos);
        check_player_for_section_change(ctx, &mut entity);
    }

    if let Some(class_name) = entity.get_attr_string("class_name") {
        ctx.entity_classes.insert(entity.id, class_name.clone());
    }

    entities.push(entity);
    ctx.map.entities = entities;
}

fn id(vm: &VirtualMachine) -> u32 {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        ctx.curr_entity_id
    })
    .unwrap()
}

/// Used only for local, Eldiron Creator emitted commands.
fn player_action(action: String, vm: &VirtualMachine) {
    if let Ok(parsed_action) = action.parse::<EntityAction>() {
        with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
            let entity_id = ctx.curr_entity_id;
            if let Some(entity) = get_entity_mut(&mut ctx.map, entity_id) {
                entity.action = parsed_action;
            }
        });
    }
}

/// Dummy. Only used on the client side.
fn player_intent(_intent: String, _vm: &VirtualMachine) {}

/// Set the current debug location in the grid.
fn set_debug_loc(event: String, x: u32, y: u32, vm: &VirtualMachine) {
    with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
        ctx.curr_debug_loc = Some((event, x, y));
    });
}
