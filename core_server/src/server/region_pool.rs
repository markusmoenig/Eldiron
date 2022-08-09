use crate::prelude::*;

use crossbeam_channel::{ Sender, Receiver, tick, select };

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

    pub fn add_regions(&mut self, regions: Vec<String>, regions_behavior: HashMap<usize, Vec<String>>, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, game: String) {
        for region in regions {
            let mut instance = RegionInstance::new();
            instance.setup(region, regions_behavior.clone(), behaviors.clone(), systems.clone(), items.clone(), game.clone());
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
                recv(ticker) -> _ => self.tick(),
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
                                for inst in &mut self.instances {
                                    if inst.region_data.id == position.0 {
                                        inst.create_player_instance(uuid, position);
                                        log::error!("create player");
                                    }
                                }
                            },
                            Message::ExecutePlayerAction(uuid, region_id, player_action) => {
                                for inst in &mut self.instances {
                                    if inst.region_data.id == region_id {
                                        if let Some(inst_index) = inst.player_uuid_indices.get(&uuid) {
                                            inst.instances[*inst_index].action = Some(player_action);
                                            break;
                                        }
                                    }
                                }
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
        println!("Exiting");
    }

    /// Game tick
    pub fn tick(&mut self) {

        let mut characters_to_transfer : Vec<(usize, BehaviorInstance)> = vec![];

        for instance in &mut self.instances {
            let messages = instance.tick();
            for m in messages {
                match m {
                    Message::TransferCharacter(region_id, instance) => {
                        characters_to_transfer.push((region_id, instance));
                    },
                    _ => self.sender.send(m).unwrap(),
                }
            }
        }

        for transfer in characters_to_transfer {
            for i in &mut self.instances {
                if i.region_data.id == transfer.0 {
                    let uuid = transfer.1.id;
                    i.transfer_character_into(transfer.1);
                    self.sender.send(Message::CharacterHasBeenTransferredInsidePool(uuid, i.region_data.id)).unwrap();
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
    pub fn contains_region(&self, region_id: usize) -> bool {
        for i in &self.instances {
            if i.region_data.id == region_id {
                return true;
            }
        }
        false
    }
}