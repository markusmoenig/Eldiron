extern crate ref_thread_local;
use ref_thread_local::{ref_thread_local};

use crate::prelude::*;

use crossbeam_channel::{ Sender, Receiver, tick, select };

ref_thread_local! {
    pub static managed UTILITY          : RegionUtility = RegionUtility::new();
    pub static managed SHEETS           : Vec<Sheet> = vec![];

    pub static managed CURR_SHEET       : usize = 0;
}

pub struct RegionPool<'a> {

    sender                  : Sender<Message>,
    receiver                : Receiver<Message>,

    threaded                : bool,
    instances               : Vec<RegionInstance<'a>>,
}

impl RegionPool<'_> {

    pub fn new(threaded: bool, sender: Sender<Message>, receiver: Receiver<Message>) -> Self {
        Self {
            sender,
            receiver,

            threaded,
            instances       : vec![],
        }
    }

    pub fn add_regions(&mut self, regions: Vec<String>, regions_behavior: FxHashMap<Uuid, Vec<String>>, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, spells: Vec<String>, game: String, scripts: FxHashMap<String, String>) {
        for region in regions {
            let mut instance = RegionInstance::new();
            instance.setup(region, regions_behavior.clone(), behaviors.clone(), systems.clone(), items.clone(), spells.clone(), game.clone(), scripts.clone());
            self.instances.push(instance);
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
        for inst in &mut self.instances {
            if inst.region_data.id == position.region {
                inst.create_player_instance(uuid, position.clone());
            }
        }
    }

    /// Create a new player instance
    pub fn destroy_player_instance(&mut self, uuid: Uuid) {
        for inst in &mut self.instances {
            inst.destroy_player_instance(uuid);
        }
    }

    /// Executes the given player action
    pub fn execute_player_action(&mut self, uuid: Uuid, region_id: Uuid, player_action: PlayerAction) {
        for inst in &mut self.instances {
            if inst.region_data.id == region_id {
                if let Some(inst_index) = inst.player_uuid_indices.get(&uuid) {
                    inst.instances[*inst_index].action = Some(player_action);
                    break;
                }
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