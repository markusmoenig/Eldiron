extern crate ref_thread_local;
use ref_thread_local::{ref_thread_local, RefThreadLocal};

use crate::prelude::*;
use crossbeam_channel::{ Sender, Receiver, tick, select };

// Local thread globals which can be accessed from both Rust and Rhai
ref_thread_local! {
    pub static managed REGION_DATA      : Vec<RegionData> = vec![];

    pub static managed UTILITY          : RegionUtility = RegionUtility::new();

    pub static managed BEHAVIORS        : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
    pub static managed ITEMS            : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
    pub static managed SPELLS           : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
    pub static managed SYSTEMS          : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
    pub static managed GAME_BEHAVIOR    : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();

    pub static managed STATE            : State = State::new();

    pub static managed ENGINE           : rhai::Engine = rhai::Engine::new();
    pub static managed TICK_COUNT       : u128 = 5 * 60 * 4;
    pub static managed TICKS_PER_MINUTE : usize = 4;
    pub static managed DATE             : Date = Date::new();

    pub static managed CURR_INST        : usize = 0;
}

pub struct RegionPool {

    sender                  : Sender<Message>,
    receiver                : Receiver<Message>,

    threaded                : bool,
    instances               : Vec<RegionInstance>,
}

impl RegionPool {

    pub fn new(threaded: bool, sender: Sender<Message>, receiver: Receiver<Message>) -> Self {

        // Create the Engine for the pool

        let mut engine = rhai::Engine::new();

        // Variable resolver for d??? -> random(???)
        #[allow(deprecated)]
        engine.on_var(|name, _index, _context| {
            if name.starts_with("d") {
                let mut s = name.to_string();
                s.remove(0);
                if let Some(n) = s.parse::<i32>().ok() {
                    let mut util = UTILITY.borrow_mut();
                    let random = util.rng.gen_range(1..=n);
                    return Ok(Some(random.into()));
                }
            } else
            if name.starts_with("r") {
                let mut util = UTILITY.borrow_mut();
                if let Some(result) = util.roll(&name[1..name.len()]).ok() {
                    return Ok(Some(result.into()));
                }
            }
            Ok(None)
        });

        engine.register_fn("roll", |exp: &str| -> i32 {
            let mut util = UTILITY.borrow_mut();
            if let Some(rc) = util.roll(exp).ok() {
                rc
            } else {
                1
            }
        });

        engine.register_fn("get_sheet", || -> Sheet {
            let data = &REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.sheets[data.curr_index].clone()
        });

        engine.register_fn("get_target_sheet", || -> Sheet {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            if let Some(target_index) = data.character_instances[data.curr_index].target_instance_index {
                data.sheets[target_index].clone()
            } else {
                Sheet::new()
            }
        });

        engine.register_fn("set_sheet", |sheet: Sheet| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.sheets[data.curr_index] = sheet;
        });

        engine.register_fn("set_target_sheet", |sheet: Sheet| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            if let Some(target_index) = data.character_instances[data.curr_index].target_instance_index {
                data.sheets[target_index] = sheet;
            }
        });

        engine.register_fn("inventory_add", |mut sheet: Sheet, item_name: &str| -> Sheet {
            inventory_add(&mut sheet, item_name, 1, &mut ITEMS.borrow_mut());
            sheet
        });

        engine.register_fn("inventory_equip", |mut sheet: Sheet, item_name: &str| -> Sheet {
            inventory_equip(&mut sheet, item_name);
            sheet
        });

        engine.register_fn("inventory_add_gold", |mut sheet: Sheet, amount: i32| -> Sheet {
            sheet.wealth.add(Currency::new(amount, 0));
            sheet
        });

        engine.register_fn("inventory_add_silver", |mut sheet: Sheet, amount: i32| -> Sheet {
            sheet.wealth.add(Currency::new(0, amount));
            sheet
        });

        engine.register_fn("inventory_add_gold_silver", |mut sheet: Sheet, gold: i32, silver: i32| -> Sheet {
            sheet.wealth.add(Currency::new(gold, silver));
            sheet
        });

        engine.register_fn("get_state", || -> bool {
            STATE.borrow().state
        });

        engine.register_fn("set_state", |state: bool| {
            STATE.borrow_mut().state = state
        });

        engine.register_fn("toggle_state", || {
            let mut state = STATE.borrow_mut();
            state.state = !state.state;
        });

        engine.register_fn("execute", |tree: &str| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.to_execute.push((data.curr_index, tree.to_string()));
        });

        engine.register_fn("execute_on_target", |tree: &str| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            if let Some(target_index) = data.character_instances[data.curr_index].target_instance_index {
                data.to_execute.push((target_index, tree.to_string()));
            }
        });

        engine.register_fn("send_status_message", |message: &str| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            let name = data.character_instances[data.curr_index].name.clone();
            data.character_instances[data.curr_index].messages.push( MessageData {
                    message_type        : MessageType::Status,
                    message             : message.to_string(),
                    from                : name,
                    right               : None,
                    center              : None,
                    buffer              : None,
            });
        });

        engine.register_fn("send_status_message_target", |message: &str| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            if let Some(target) = data.character_instances[data.curr_index].target_instance_index {
                let name = data.character_instances[target].name.clone();
                data.character_instances[target].messages.push( MessageData {
                        message_type        : MessageType::Status,
                        message             : message.to_string(),
                        from                : name,
                        right               : None,
                        center              : None,
                        buffer              : None,
                });
            }
        });

        // Roll the damage for the main weapon
        engine.register_fn("roll_weapon_damage", |mut sheet: Sheet, slot_name: String| -> i32 {
            roll_weapon_damage(&mut sheet, slot_name)
        });

        engine.register_fn("increase_weapon_skill_by", |mut sheet: Sheet, slot_name: String, amount: i32| -> Sheet {
            inc_weapon_skill_by(&mut sheet, slot_name, amount);
            sheet
        });

        // Roll the damage for the main weapon
        engine.register_fn("execute_weapon_effects", || {
            let item_effects : Option<(Uuid, Uuid)>;
            {
                let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                item_effects = data.item_effects;
            }
            if let Some(item_effects) = item_effects {
                execute_node(item_effects.0, item_effects.1, &mut ITEMS.borrow_mut());
            }
        });

        // Get the skill name for the given item name
        engine.register_fn("get_item_skill", |item_name: String| -> String {
            if let Some(name) = get_item_skill_name(item_name) {
                name
            } else {
                "".to_string()
            }
        });

        // Increases the given skill by the given amount
        engine.register_fn("increase_skill_by", |mut sheet: Sheet, skill_name: String, amount: i32| -> Sheet {
            increase_skill_by(&mut sheet, skill_name, amount);
            sheet
        });

        // Increases the experience by the given amount
        engine.register_fn("increase_experience_by", |mut sheet: Sheet, amount: i32| -> Sheet {
            increase_experience_by(&mut sheet, amount);
            sheet
        });

        Sheet::register(&mut engine);
        Currency::register(&mut engine);

        script_register_message_api(&mut engine);
        script_register_inventory_api(&mut engine);
        script_register_spells_api(&mut engine);
        script_register_gear_api(&mut engine);
        script_register_weapons_api(&mut engine);
        script_register_experience_api(&mut engine);
        script_register_date_api(&mut engine);
        script_register_failure_enum_api(&mut engine);

        // Display f64 as ints
        use pathfinding::num_traits::ToPrimitive;
        engine.register_fn("to_string", |x: f32| format!("{}", x.to_isize().unwrap()));

        *ENGINE.borrow_mut() = engine;

        Self {
            sender,
            receiver,

            threaded,
            instances       : vec![],
        }
    }

    pub fn add_regions(&mut self, regions: Vec<String>, regions_behavior: FxHashMap<Uuid, Vec<String>>, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, spells: Vec<String>, game: String, scripts: FxHashMap<String, String>) {

        // --- Add the behaviors to the global pool
        let mut decoded_behaviors : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
        for i in &behaviors {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&i).ok() {
                decoded_behaviors.insert(behavior_data.id, behavior_data);
            }
        }
        {
            let mut static_behaviors = BEHAVIORS.borrow_mut();
            *static_behaviors = decoded_behaviors;
        }

        // --- Add the items to the global pool
        let mut decoded_items : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
        for i in &items {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&i).ok() {
                decoded_items.insert(behavior_data.id, behavior_data);
            }
        }
        {
            let mut static_items = ITEMS.borrow_mut();
            *static_items = decoded_items;
        }

        // --- Add the spells to the global pool
        let mut decoded_spells : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
        for i in &spells {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&i).ok() {
                decoded_spells.insert(behavior_data.id, behavior_data);
            }
        }
        {
            let mut static_spells = SPELLS.borrow_mut();
            *static_spells = decoded_spells;
        }

        // --- Add the systems to the global pool
        let mut decoded_systems : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
        for i in &systems {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&i).ok() {
                decoded_systems.insert(behavior_data.id, behavior_data);
            }
        }
        {
            let mut static_systems = SYSTEMS.borrow_mut();
            *static_systems = decoded_systems;
        }

        // --- Add the game behavior
        let mut decoded_game : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();
        if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&game).ok() {
            decoded_game.insert(behavior_data.id, behavior_data);
        }
        {
            let mut static_game = GAME_BEHAVIOR.borrow_mut();
            *static_game = decoded_game;
        }

        // --- Setup the regions

        for region in regions {
            let mut instance = RegionInstance::new();
            {
                let data = RegionData::new();
                REGION_DATA.borrow_mut().push(data);
            }
            instance.setup(region, regions_behavior.clone(), behaviors.clone(), systems.clone(), items.clone(), spells.clone(), game.clone(), scripts.clone());
            self.instances.push(instance);
            {
                let mut index = *CURR_INST.borrow();
                index += 1;
                *CURR_INST.borrow_mut() = index;
            }
        }

        if self.threaded {
            self.run();
        }
    }

    /// The game loop for these regions. Only called when mt is available. Otherwise server calls tick() directly.
    pub fn run(&mut self) {

        let ticker = tick(std::time::Duration::from_millis(250));

        loop {

            select! {
                recv(ticker) -> _ => {
                    _ = self.tick()
                },
                recv(self.receiver) -> mess => {
                    if let Some(message) = mess.ok() {
                        match message {
                            Message::Quit() => {
                                break;
                            },
                            Message::Status(status) => {
                                println!{"Pool received status {}", status};
                                log::error!("{:?}", status);
                            },
                            Message::CreatePlayerInstance(uuid, position) => {
                                self.create_player_instance(uuid, position);
                            },
                            Message::DestroyPlayerInstance(uuid) => {
                                self.destroy_player_instance(uuid);
                            },
                            Message::ExecutePlayerAction(uuid, region_id, player_action) => {
                                self.execute_player_action(uuid, region_id, player_action);
                            },
                            Message::SetDebugBehaviorId(id) => {
                                let mut index = 0;
                                for inst in &mut self.instances {
                                    {
                                        *CURR_INST.borrow_mut() = index;
                                    }
                                    inst.set_debug_behavior_id(id);
                                    index += 1;
                                }
                            }
                            _ => { log::error!("Unhandled message for region pool: {:?}", message); }
                        }
                    }
                }
            }
        }
    }

    /// Game tick, uses messages when running multi-threaded, otherwise returns the messages back to the server.
    pub fn tick(&mut self) -> Option<Vec<Message>> {

        let mut ret_messages : Vec<Message> = vec![];
        let mut characters_to_transfer : Vec<(Uuid, BehaviorInstance, Sheet)> = vec![];

        {
            *CURR_INST.borrow_mut() = 0;
        }

        DATE.borrow_mut().from_ticks(*TICK_COUNT.borrow(), *TICKS_PER_MINUTE.borrow());

        for instance in &mut self.instances {
            let messages = instance.tick();
            for m in messages {
                match m {
                    Message::TransferCharacter(region_id, instance, sheet) => {
                        characters_to_transfer.push((region_id, instance, sheet));
                    },
                    _ => {
                        if self.threaded {
                            self.sender.send(m).unwrap()
                        } else {
                            ret_messages.push(m);
                        }
                    }
                }
            }
            {
                let mut index = *CURR_INST.borrow();
                index += 1;
                *CURR_INST.borrow_mut() = index;
            }
        }

        for transfer in characters_to_transfer {
            {
                *CURR_INST.borrow_mut() = 0;
            }
            for i in &mut self.instances {
                if i.region_data.id == transfer.0 {
                    let uuid = transfer.1.id;
                    i.transfer_character_into(transfer.1, transfer.2);
                    let message = Message::CharacterHasBeenTransferredInsidePool(uuid, i.region_data.id);
                    if self.threaded {
                        self.sender.send(message).unwrap();
                    } else {
                        ret_messages.push(message);
                    }
                    break;
                }
                {
                    let mut index = *CURR_INST.borrow();
                    index += 1;
                    *CURR_INST.borrow_mut() = index;
                }
            }
        }

        let mut ticks = *TICK_COUNT.borrow();
        ticks = ticks.wrapping_add(1);
        *TICK_COUNT.borrow_mut() = ticks;

        // If running none
        if self.threaded == false {
            return Some(ret_messages);
        }

        None

    }

    /// Create a new player instance
    pub fn create_player_instance(&mut self, uuid: Uuid, position: Position) {
        {
            *CURR_INST.borrow_mut() = 0;
        }
        for inst in &mut self.instances {
            if inst.region_data.id == position.region {
                inst.create_player_instance(uuid, position.clone());
            }
            {
                let mut index = *CURR_INST.borrow();
                index += 1;
                *CURR_INST.borrow_mut() = index;
            }
        }
    }

    /// Destroy the given player instance
    pub fn destroy_player_instance(&mut self, uuid: Uuid) {
        {
            *CURR_INST.borrow_mut() = 0;
        }
        for inst in &mut self.instances {
            inst.destroy_player_instance(uuid);
            {
                let mut index = *CURR_INST.borrow();
                index += 1;
                *CURR_INST.borrow_mut() = index;
            }
        }
    }

    /// Executes the given player action
    pub fn execute_player_action(&mut self, uuid: Uuid, region_id: Uuid, player_action: PlayerAction) {
        {
            *CURR_INST.borrow_mut() = 0;
        }
        for inst in &mut self.instances {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            if inst.region_data.id == region_id {
                if let Some(inst_index) = data.player_uuid_indices.get(&uuid) {
                    data.character_instances[*inst_index].action = Some(player_action);
                    break;
                }
            }
            {
                let mut index = *CURR_INST.borrow();
                index += 1;
                *CURR_INST.borrow_mut() = index;
            }
        }
    }

    /// Number of region instances handled by this pool
    pub fn instances(&self) -> usize {
        self.instances.len()
    }

    /// Contains true if this pool contains the region with the given id
    pub fn contains_region(&self, region_id: Uuid) -> bool {
        for i in &self.instances {
            if i.region_data.id == region_id {
                return true;
            }
        }
        false
    }
}