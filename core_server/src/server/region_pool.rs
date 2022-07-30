use crate::prelude::*;

pub struct RegionPool {

    threaded                : bool,
    instances               : Vec<RegionInstance>,
}

impl RegionPool {

    pub fn new(threaded: bool) -> Self {
        Self {

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
    }

    pub fn instances(&self) -> usize {
        self.instances.len()
    }
}