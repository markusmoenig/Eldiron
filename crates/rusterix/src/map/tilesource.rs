use serde::{Deserialize, Serialize};
use theframework_core::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileSource {
    SingleTile(Uuid),
}
