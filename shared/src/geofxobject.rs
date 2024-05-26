use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,

    pub geos: Vec<GeoFXNode>,
}

impl Default for GeoFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl GeoFXObject {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            geos: Vec::new(),
        }
    }
}
