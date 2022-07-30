use crate::prelude::*;

pub mod region_instance;
pub mod region_pool;

pub struct Server {

    pub regions             : Vec<String>,
    pub behaviors           : Vec<String>,

    /// If we don't use threads (for example for the web), all regions are in here.
    pub pool                : RegionPool
}

impl Server {

    pub fn new() -> Self {
        Self {
            regions         : vec![],
            behaviors       : vec![],
            pool            : RegionPool::new(false)
        }
    }

    /// Collects all data (assets, regions, behaviors etc.) and store them as JSON so that we can distribute them to threads as needed.
    pub fn collect_data(&mut self, data: &GameData) {

        for (_id, region) in &data.regions {
            if let Some(json) = serde_json::to_string(&region.data).ok() {
                self.regions.push(json);
            }
        }

        for (_id, behavior) in &data.behaviors {
            if let Some(json) = serde_json::to_string(&behavior.data).ok() {
                self.behaviors.push(json);
            }
        }
    }

    /// Starts the server and distributes regions over threads. max_num_threads limits the max number of threads or does not use threads at all if None
    pub fn start(&mut self, max_num_threads: Option<i32>) -> Result<(), String> {

        if let Some(max_num_threads) = max_num_threads {
            let max_regions_per_pool = 100;
            let mut regions = vec![];

            let start_thread = |regions: Vec<String>| {
                let behaviors = self.behaviors.clone();

                std::thread::spawn( move || {
                    let mut pool = RegionPool::new(true);
                    pool.add_regions(regions, behaviors);
                });
            };

            for json in &self.regions {

                if regions.len() < max_regions_per_pool {
                    regions.push(json.clone());
                } else {
                    start_thread(regions.clone());
                    regions = vec![];
                }
            }

            if regions.is_empty() == false {
                start_thread(regions.clone());
            }
        } else {
            self.pool.add_regions(self.regions.clone(), self.behaviors.clone());
        }

        Ok(())
    }


}