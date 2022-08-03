use crate::prelude::*;

pub mod region_instance;
pub mod region_pool;
pub mod message;
pub mod utilities;

use crossbeam_channel::{ Sender, Receiver, unbounded };

pub struct RegionPoolMeta {
    sender                  : Sender<Message>,
    receiver                : Receiver<Message>,

    region_ids              : Vec<usize>,
}

pub struct Server<'a> {

    pub regions             : HashMap<usize, String>,
    pub behaviors           : Vec<String>,
    pub systems             : Vec<String>,
    pub items               : Vec<String>,
    pub game                : String,

    /// If we don't use threads (for example for the web), all regions are in here.
    pub pool                : Option<RegionPool<'a>>,

    /// The meta data for all pools
    metas                   : Vec<RegionPoolMeta>,
}

impl Server<'_> {

    pub fn new() -> Self {
        Self {
            regions         : HashMap::new(),
            behaviors       : vec![],
            systems         : vec![],
            items           : vec![],
            game            : "".to_string(),
            pool            : None,
            metas           : vec![],
        }
    }

    /// Collects all data (assets, regions, behaviors etc.) and store them as JSON so that we can distribute them to threads as needed.
    pub fn collect_data(&mut self, data: &GameData) {

        for (id, region) in &data.regions {
            if let Some(json) = serde_json::to_string(&region.data).ok() {
                self.regions.insert(*id, json);
            }
        }

        for (_id, behavior) in &data.behaviors {
            if let Some(json) = serde_json::to_string(&behavior.data).ok() {
                self.behaviors.push(json);
            }
        }

        for (_id, system) in &data.systems {
            if let Some(json) = serde_json::to_string(&system.data).ok() {
                self.systems.push(json);
            }
        }

        for (_id, item) in &data.items {
            if let Some(json) = serde_json::to_string(&item.data).ok() {
                self.items.push(json);
            }
        }

        if let Some(json) = serde_json::to_string(&data.game.behavior.data).ok() {
            self.game = json;
        }
    }

    /// Starts the server and distributes regions over threads. max_num_threads limits the max number of threads or does not use threads at all if None.
    pub fn start(&mut self, max_num_threads: Option<i32>) -> Result<(), String> {

        if  let Some(max_num_threads) = max_num_threads {
            let max_regions_per_pool = 100;

            let mut regions = vec![];
            let mut region_ids = vec![];

            let mut start_thread = |region_ids: Vec<usize>, regions: Vec<String>| {

                let (sender, receiver) = unbounded();

                let s = sender.clone();
                let r = receiver.clone();

                let meta = RegionPoolMeta {
                    sender,
                    receiver,
                    region_ids,
                };

                let behaviors = self.behaviors.clone();
                let systems = self.systems.clone();
                let items = self.items.clone();
                let game = self.game.clone();

                let _handle = std::thread::spawn( move || {
                    let mut pool = RegionPool::new(true, s, r);
                    pool.add_regions(regions, behaviors, systems, items, game);
                });

                meta.sender.send(Message::Status("Startup".to_string())).unwrap();
                self.metas.push(meta);
            };

            for (id, json) in &self.regions {

                if regions.len() < max_regions_per_pool {
                    regions.push(json.clone());
                    region_ids.push(*id);
                } else {
                    start_thread(region_ids, regions.clone());
                    regions = vec![];
                    region_ids = vec![];
                }
            }

            if regions.is_empty() == false {
                start_thread(region_ids, regions.clone());
            }
        } else {
            let (sender, receiver) = unbounded::<Message>();

            sender.send(Message::Status("Startup".to_string())).unwrap();

            let mut pool = RegionPool::new(false, sender, receiver);
            pool.add_regions(self.regions.values().cloned().collect(), self.behaviors.clone(), self.systems.clone(), self.items.clone(), self.game.clone());
            self.pool = Some(pool);
        }

        Ok(())
    }

    /// Shutdown the system
    pub fn shutdown(&mut self) -> Result<(), String> {

        for meta in &self.metas {
            meta.sender.send(Message::Quit()).unwrap();
        }

        Ok(())
    }

    /// Only called when running on a non threaded system (web)
    pub fn tick(&mut self) {
        if let Some(pool) = &mut self.pool {
            pool.tick();
        }
    }

}