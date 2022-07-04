use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum LightType {
    PointLight,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Light {
    pub light_type              : LightType,
    pub position                : (isize, isize),
    pub intensity               : u8,
}

impl Light {

    pub fn new(light_type: LightType, position: (isize, isize), intensity: u8) -> Self {

        Self {
            light_type,
            position ,
            intensity,
        }
    }
}