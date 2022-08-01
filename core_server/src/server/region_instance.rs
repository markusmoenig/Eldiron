use crate::prelude::*;

pub struct RegionInstance {

    region_data                     : GameRegionData,
}

impl RegionInstance {

    pub fn new() -> Self {
        Self {
            region_data             : GameRegionData::new(),
        }
    }

    pub fn start(&mut self, region: String, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, game: String) {
        if let Some(region_data) = serde_json::from_str(&region).ok() {
            self.region_data = region_data;
        }
    }

    pub fn tick(&mut self) {

    }
}