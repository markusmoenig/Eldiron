use crate::prelude::*;

use crossbeam_channel::{ Sender, Receiver };

pub struct RegionPool {

    sender                  : Sender<Message>,
    receiver                : Receiver<Message>,

    threaded                : bool,
    instances               : Vec<RegionInstance>,
}

impl RegionPool {

    pub fn new(threaded: bool, sender: Sender<Message>, receiver: Receiver<Message>) -> Self {
        Self {
            sender,
            receiver,

            threaded,
            instances       : vec![],
        }
    }

    pub fn add_regions(&mut self, regions: Vec<String>, behaviors: Vec<String>) {
        println!("Pool: Adding {}", regions.len());

        for region in regions {
            let mut instance = RegionInstance::new();
            instance.start(region, behaviors.clone());
            self.instances.push(instance);
        }

        if let Some(m) = self.receiver.recv().ok() {
            println!("{:?}", m);
        }
    }

    pub fn instances(&self) -> usize {
        self.instances.len()
    }
}