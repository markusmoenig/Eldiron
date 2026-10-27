use serde::{Deserialize, Serialize};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileSource {
    SingleTile(Uuid),
}
