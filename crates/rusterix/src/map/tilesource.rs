use serde::{Deserialize, Serialize};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct TileGroupMemberRef {
    pub tile_id: Uuid,
    pub x: u16,
    pub y: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
pub struct TileGroup {
    pub id: Uuid,
    #[serde(default)]
    pub name: String,
    pub width: u16,
    pub height: u16,
    #[serde(default)]
    pub members: Vec<TileGroupMemberRef>,
    #[serde(default)]
    pub tags: String,
}

impl TileGroup {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            width,
            height,
            members: Vec::new(),
            tags: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum TileSource {
    SingleTile(Uuid),
    TileGroup(Uuid),
    TileGroupMember { group_id: Uuid, member_index: u16 },
    Procedural(Uuid),
}
