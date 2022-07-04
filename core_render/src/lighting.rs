use std::collections::HashMap;
use core_shared::{regiondata::GameRegionData, light::Light};

/*
#[derive(PartialEq, Clone, Debug)]
pub struct TileLighting {
    pub fixed                   : f32,
    pub dynamic                 : f32,
}*/

pub fn compute_lighting(region: &GameRegionData, lights: &Vec<Light>) -> HashMap<(isize, isize), f64> {
    let mut map : HashMap<(isize, isize), f64> = HashMap::new();

    //for (index, area) in region.areas.iter().enumerate() {
        //for n in region.
    //}

    for l in lights {
        map.insert(l.position.clone(), 1.0);
    }

    map
}