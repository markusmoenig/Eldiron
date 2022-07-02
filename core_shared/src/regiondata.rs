
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{asset::TileUsage, settings_region::RegionSettings};

#[derive(Serialize, Deserialize, Clone)]
pub struct RegionArea {
    pub name            : String,
    pub id              : usize,
    pub area            : Vec<(isize, isize)>,
    pub behavior        : usize,

}

#[derive(Serialize, Deserialize, Clone)]
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

    pub settings        : RegionSettings,
}