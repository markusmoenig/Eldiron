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
        println!("Pool: Adding {}", regions.len());

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
                                println!{"Received status {}", status};
                                //log::error!("{:?}", status);
                            },
                            Message::CreatePlayerInstance(uuid, position) => {
                                for inst in &mut self.instances {
                                    if inst.region_data.id == position.0 {
                                        inst.create_player_instance(uuid, position);
                                    }
                                }
                            },
                            Message::ExecutePlayerAction(uuid, region_id, player_action) => {
                                for inst in &mut self.instances {
                                    if inst.region_data.id == region_id {
                                        //inst.create_player_instance(uuid, position);
                                        if let Some(inst_index) = inst.player_uuid_indices.get(&uuid) {
                                            inst.instances[*inst_index].action = Some(player_action);
                                            println!("Applied player action");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        println!("Exiting");
    }

    /// Game tick
    pub fn tick(&mut self) {
        for instance in &mut self.instances {
            instance.tick();
        }
    }

    /// Number of region instances handled by this pool
    pub fn instances(&self) -> usize {
        self.instances.len()
    }
}