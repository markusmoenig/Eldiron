
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RegionSettings {
    pub dynamic_lighting    : bool,
}

impl RegionSettings {

    pub fn new() -> Self {

        Self {
            dynamic_lighting    : false,
        }
    }
}
