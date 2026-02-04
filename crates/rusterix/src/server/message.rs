use crate::{Entity, Value};
use codegridfx::DebugModule;
use theframework::prelude::*;

/// Messages to / from the Region to the server or client
#[derive(Debug)]
// #[allow(clippy::large_enum_variant)]
pub enum RegionMessage {
    /// Register a local player (which receives user based events).
    /// RegionInstanceId, PlayerId
    RegisterPlayer(u32, u32),
    /// An event
    Event(u32, String, Value),
    /// A user event
    UserEvent(u32, String, Value),
    /// Create the entity in the region.
    CreateEntity(u32, Entity),
    /// A user action
    UserAction(u32, EntityAction),
    /// Entity updates for a given region instance
    EntitiesUpdate(u32, Vec<Vec<u8>>),
    /// Item updates for a given region instance
    ItemsUpdate(u32, Vec<Vec<u8>>),
    /// Remove the given item from the Region
    RemoveItem(u32, u32),
    /// Log Message
    LogMessage(String),
    /// Time event of a Region
    Time(u32, TheTime),
    /// Tell: RegionId, SenderId_entity, SenderId_item, ReceiverId, Message
    Message(u32, Option<u32>, Option<u32>, u32, String, String),
    /// TransferEntity: Move the Entity from the region to a new region (name) in sector (name)
    TransferEntity(u32, Entity, String, String),
    /// Send a multiple choice
    MultipleChoice(MultipleChoice),
    /// Send the debug id of a character or item
    DebugData(DebugModule),
    /// Pause the server.
    Pause,
    /// Continue after pause
    Continue,
    /// Stop processing and quit
    Quit,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum EntityAction {
    #[default]
    Off,
    Left,
    Forward,
    Right,
    Backward,
    // Item clicked, item id, click distance and optional explicit intent
    ItemClicked(u32, f32, Option<String>),
    // Entity clicked, entity id and click distance
    EntityClicked(u32, f32),
    // Terrain clicked
    TerrainClicked(Vec2<f32>),
    /// Sleep until the given tick and switch back to the given action
    SleepAndSwitch(i64, Box<EntityAction>),
    /// User: Distance, Speed, Max Min Sleep. System: State, Target
    RandomWalk(f32, f32, i32, i32, Vec2<f32>),
    /// User: Distance, Speed, Max Min Sleep. System: State, Target
    RandomWalkInSector(f32, f32, i32, i32, Vec2<f32>),
    /// Intent: A string that represents an intent, e.g. "attack", "talk", etc.
    Intent(String),
    /// Goto: Move to a specific position with a given speed
    Goto(Vec2<f32>, f32),
    /// CloseIn: Move within a radius of a target entity with a given speed
    CloseIn(u32, f32, f32),
    /// A multiple choice item was selected by the user
    Choice(Choice),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum PlayerCamera {
    #[default]
    D2,
    D3Iso,
    D3FirstP,
}

use std::fmt;
use std::str::FromStr;
impl FromStr for EntityAction {
    type Err = ();

    /// Converts a `&str` to an `EntityAction`.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(EntityAction::Off),
            "left" => Ok(EntityAction::Left),
            "forward" => Ok(EntityAction::Forward),
            "right" => Ok(EntityAction::Right),
            "backward" => Ok(EntityAction::Backward),
            _ => Err(()), // Return an error for invalid values
        }
    }
}

impl fmt::Display for EntityAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EntityAction::Off => "none",
            EntityAction::Left => "left",
            EntityAction::Forward => "forward",
            EntityAction::Right => "right",
            EntityAction::Backward => "backward",
            _ => "none",
        };
        write!(f, "{}", s)
    }
}

use std::convert::TryFrom;
impl TryFrom<i32> for EntityAction {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(EntityAction::Left),
            1 => Ok(EntityAction::Forward),
            2 => Ok(EntityAction::Right),
            3 => Ok(EntityAction::Backward),
            _ => Err("Invalid value for EntityAction"),
        }
    }
}

/// A players choice.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Choice {
    // Cancels a multiple choice. from, to
    Cancel(u32, u32),
    /// An item to sell, item_id, seller_id, buyer_id
    ItemToSell(u32, u32, u32),
}

/// Multiple choices for the player
#[derive(Debug, Clone)]
pub struct MultipleChoice {
    pub region: u32,
    pub from: u32,
    pub to: u32,

    pub choices: Vec<Choice>,
}

impl MultipleChoice {
    pub fn new(region: u32, from: u32, to: u32) -> Self {
        Self {
            region,
            from,
            to,
            choices: vec![],
        }
    }

    /// Add a choice
    pub fn add(&mut self, choice: Choice) {
        self.choices.push(choice);
    }
}
