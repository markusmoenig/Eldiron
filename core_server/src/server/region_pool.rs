extern crate ref_thread_local;
use ref_thread_local::{ref_thread_local, RefThreadLocal};

use crate::prelude::*;
use crossbeam_channel::{ Sender, Receiver, tick, select };

// Local thread globals which can be accessed from both Rust and Rhai
ref_thread_local! {
    pub static managed REGION_DATA      : Vec<RegionData> = vec![];

    pub static managed UTILITY          : RegionUtility = RegionUtility::new();
    pub static managed ITEMS            : FxHashMap<Uuid, GameBehaviorData> = FxHashMap::default();

    pub static managed ENGINE           : rhai::Engine = rhai::Engine::new();

    pub static managed CURR_INST        : usize = 0;
}

pub struct RegionPool<'a> {

    sender                  : Sender<Message>,
    receiver                : Receiver<Message>,

    threaded                : bool,
    instances               : Vec<RegionInstance<'a>>,
}

impl RegionPool<'_> {

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

        engine.register_fn("set_sheet", |sheet: Sheet| {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.sheets[data.curr_index] = sheet;
        });

        engine.register_fn("inventory_add", |mut sheet: Sheet, item_name: &str| -> Sheet {
            inventory_add(&mut sheet, item_name, 1, &mut ITEMS.borrow_mut());
            sheet
        });

        Sheet::register(&mut engine);

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
                                for inst in &mut self.instances {
                                    inst.set_debug_behavior_id(id);
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
        let mut characters_to_transfer : Vec<(Uuid, BehaviorInstance)> = vec![];

        {
            *CURR_INST.borrow_mut() = 0;
        }

        for instance in &mut self.instances {
            let messages = instance.tick();
            for m in messages {
                match m {
                    Message::TransferCharacter(region_id, instance) => {
                        characters_to_transfer.push((region_id, instance));
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
            for i in &mut self.instances {
                if i.region_data.id == transfer.0 {
                    let uuid = transfer.1.id;
                    i.transfer_character_into(transfer.1);
                    let message = Message::CharacterHasBeenTransferredInsidePool(uuid, i.region_data.id);
                    if self.threaded {
                        self.sender.send(message).unwrap();
                    } else {
                        ret_messages.push(message);
                    }
                    break;
                }
            }
        }

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

    /// Create a new player instance
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
            if inst.region_data.id == region_id {
                if let Some(inst_index) = inst.player_uuid_indices.get(&uuid) {
                    inst.instances[*inst_index].action = Some(player_action);
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