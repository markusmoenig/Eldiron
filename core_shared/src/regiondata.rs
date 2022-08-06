
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionArea {
    pub name            : String,
    pub id              : usize,
    pub area            : Vec<(isize, isize)>,
    pub behavior        : usize,

}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameRegionData {
    #[serde(with = "vectorize")]
    pub layer1          : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
    #[serde(with = "vectorize")]
    pub layer2          : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
    #[serde(with = "vectorize")]
    pub layer3          : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
    #[serde(with = "vectorize")]
    pub layer4          : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
    pub id              : usize,
    pub curr_pos        : (isize, isize),
    pub min_pos         : (isize, isize),
    pub max_pos         : (isize, isize),
    pub areas           : Vec<RegionArea>,

    pub settings        : PropertySink,
}

impl GameRegionData {
    pub fn new() -> Self {
        Self {
            layer1      : HashMap::new(),
            layer2      : HashMap::new(),
            layer3      : HashMap::new(),
            layer4      : HashMap::new(),
            id          : 0,
            curr_pos    : (0,0),
            min_pos     : (10000,10000),
            max_pos     : (-10000, -10000),
            areas       : vec![],
            settings    : PropertySink::new(),
        }
    }
}