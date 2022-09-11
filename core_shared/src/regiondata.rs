use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionArea {
    pub name            : String,
    pub id              : Uuid,
    pub area            : Vec<(isize, isize)>,
    pub behavior        : Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameRegionData {
    #[serde(with = "vectorize")]
    pub layer1          : FxHashMap<(isize, isize), TileData>,
    #[serde(with = "vectorize")]
    pub layer2          : FxHashMap<(isize, isize), TileData>,
    #[serde(with = "vectorize")]
    pub layer3          : FxHashMap<(isize, isize), TileData>,
    #[serde(with = "vectorize")]
    pub layer4          : FxHashMap<(isize, isize), TileData>,
    pub id              : Uuid,
    pub curr_pos        : (isize, isize),
    pub min_pos         : (isize, isize),
    pub max_pos         : (isize, isize),
    pub areas           : Vec<RegionArea>,

    pub settings        : PropertySink,
}

impl GameRegionData {
    pub fn new() -> Self {
        Self {
            layer1      : FxHashMap::default(),
            layer2      : FxHashMap::default(),
            layer3      : FxHashMap::default(),
            layer4      : FxHashMap::default(),
            id          : Uuid::new_v4(),
            curr_pos    : (0,0),
            min_pos     : (10000,10000),
            max_pos     : (-10000, -10000),
            areas       : vec![],
            settings    : PropertySink::new(),
        }
    }
}