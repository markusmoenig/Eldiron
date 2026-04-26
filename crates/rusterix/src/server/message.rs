use crate::{Entity, Value, ValueContainer};
use codegridfx::DebugModule;
use scenevm::PaletteRemap2DMode;
use theframework::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AudioCommand {
    /// Play an audio asset on a bus/layer.
    Play {
        name: String,
        bus: String,
        gain: f32,
        looping: bool,
    },
    /// Clear one bus/layer.
    ClearBus { bus: String },
    /// Clear all currently playing audio voices on all buses.
    ClearAll,
    /// Set volume for one bus/layer.
    SetBusVolume { bus: String, volume: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteRemap2DState {
    pub start_index: u32,
    pub end_index: u32,
    pub mode: PaletteRemap2DMode,
    pub blend: f32,
}

impl Default for PaletteRemap2DState {
    fn default() -> Self {
        Self {
            start_index: 0,
            end_index: 0,
            mode: PaletteRemap2DMode::Disabled,
            blend: 0.0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuntimeRenderState {
    pub palette_remap: Option<PaletteRemap2DState>,
    pub render: ValueContainer,
    pub post: ValueContainer,
}

impl RuntimeRenderState {
    pub fn merged(self, override_state: Option<Self>) -> Self {
        if let Some(override_state) = override_state {
            let mut render = self.render.clone();
            for key in override_state.render.keys().cloned().collect::<Vec<_>>() {
                if let Some(value) = override_state.render.get(&key).cloned() {
                    render.set(&key, value);
                }
            }
            let mut post = self.post.clone();
            for key in override_state.post.keys().cloned().collect::<Vec<_>>() {
                if let Some(value) = override_state.post.get(&key).cloned() {
                    post.set(&key, value);
                }
            }
            Self {
                palette_remap: override_state.palette_remap.or(self.palette_remap),
                render,
                post,
            }
        } else {
            self
        }
    }
}

/// Messages to / from the Region to the server or client
#[derive(Debug)]
// #[allow(clippy::large_enum_variant)]
pub enum RegionMessage {
    /// Register a local player (which receives user based events).
    /// RegionInstanceId, PlayerId
    RegisterPlayer(u32, u32),
    /// Request the current sector description for a player after registration/startup.
    ShowStartupSectorDescription(u32),
    /// An event
    Event(u32, String, Value),
    /// A user event
    UserEvent(u32, String, Value),
    /// Create the entity in the region.
    CreateEntity(u32, Entity),
    /// A user action
    UserAction(u32, EntityAction),
    /// Instantly move an entity to a sector, optionally in another region.
    TeleportEntity(u32, String, String),
    /// Instantly move an entity to a position in the current region.
    TeleportEntityPos(u32, Vec2<f32>),
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
    /// Say: RegionId, SenderId_entity, SenderId_item, Message, Category
    Say(u32, Option<u32>, Option<u32>, String, String),
    /// TransferEntity: Move the Entity from the region to a new region (name) in sector (name)
    TransferEntity(u32, Entity, String, String),
    /// Send a multiple choice
    MultipleChoice(MultipleChoice),
    /// Send an audio command to the client
    AudioCmd(u32, AudioCommand),
    /// Configure 2D palette remap setup for a region.
    SetPaletteRemap2D(u32, u32, u32, PaletteRemap2DMode),
    /// Update the active 2D palette remap blend for a region.
    SetPaletteRemap2DBlend(u32, f32),
    /// Configure the global/world 2D palette remap setup.
    SetWorldPaletteRemap2D(u32, u32, PaletteRemap2DMode),
    /// Update the active global/world 2D palette remap blend.
    SetWorldPaletteRemap2DBlend(f32),
    /// Override a runtime render setting for a region.
    SetRenderValue(u32, String, Value),
    /// Override a runtime render setting globally/world-wide.
    SetWorldRenderValue(String, Value),
    /// Override a runtime post setting for a region.
    SetPostValue(u32, String, Value),
    /// Override a runtime post setting globally/world-wide.
    SetWorldPostValue(String, Value),
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
    StrafeLeft,
    StrafeRight,
    ForwardLeft,
    ForwardRight,
    BackwardLeft,
    BackwardRight,
    // Item clicked, item id, click distance, optional explicit intent and optional owner entity
    ItemClicked(u32, f32, Option<String>, Option<u32>),
    // Entity clicked, entity id, click distance and optional explicit intent
    EntityClicked(u32, f32, Option<String>),
    // Terrain clicked
    TerrainClicked(Vec2<f32>),
    /// Sleep until the given tick and switch back to the given action
    SleepAndSwitch(i64, Box<EntityAction>),
    /// User: Distance, Speed, Max Min Sleep. System: State, Target
    RandomWalk(f32, f32, i32, i32, Vec2<f32>),
    /// User: Distance, Speed, Max Min Sleep. System: State, Target
    RandomWalkInSector(f32, f32, i32, i32, Vec2<f32>),
    /// Patrol along resolved route points.
    Patrol {
        points: Vec<Vec2<f32>>,
        route_wait: f32,
        route_speed: f32,
        route_mode: String,
        point_index: usize,
        forward: bool,
        wait_until_tick: i64,
    },
    /// Intent: A string that represents an intent, e.g. "attack", "talk", etc.
    Intent(String),
    /// Goto: Move to a specific position with a given speed
    Goto(Vec2<f32>, f32),
    /// Grid-aware click-to-walk target for 2D grid movement.
    GotoGrid(Vec2<f32>, f32),
    /// Smoothly move to a specific position while keeping an explicit facing.
    StepTo(Vec2<f32>, f32, Vec2<f32>, Vec2<f32>, Vec2<f32>),
    /// Smoothly rotate to a specific facing.
    RotateTo(Vec2<f32>),
    /// CloseIn: Move within a radius of a target entity with a given speed
    CloseIn(u32, f32, f32),
    /// Follow and attack a target using engine-owned melee rules.
    FollowAttack(u32, f32, i64),
    /// Set how player input is mapped to movement
    SetPlayerCamera(PlayerCamera),
    /// Move an item (inventory/equipped drag & drop).
    MoveItem {
        item_id: u32,
        owner_entity_id: Option<u32>,
        target_entity_id: Option<u32>,
        to_inventory_index: Option<usize>,
        to_equipped_slot: Option<String>,
    },
    /// Drop an owned item onto terrain/world at a specific position.
    DropItemAt {
        item_id: u32,
        owner_entity_id: Option<u32>,
        position: Vec2<f32>,
    },
    /// A multiple choice item was selected by the user
    Choice(Choice),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum PlayerCamera {
    #[default]
    D2,
    D2Grid,
    D3Iso,
    D3FirstP,
    D3FirstPGrid,
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
            "strafe_left" => Ok(EntityAction::StrafeLeft),
            "strafe_right" => Ok(EntityAction::StrafeRight),
            "forward_left" => Ok(EntityAction::ForwardLeft),
            "forward_right" => Ok(EntityAction::ForwardRight),
            "backward_left" => Ok(EntityAction::BackwardLeft),
            "backward_right" => Ok(EntityAction::BackwardRight),
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
            EntityAction::StrafeLeft => "strafe_left",
            EntityAction::StrafeRight => "strafe_right",
            EntityAction::ForwardLeft => "forward_left",
            EntityAction::ForwardRight => "forward_right",
            EntityAction::BackwardLeft => "backward_left",
            EntityAction::BackwardRight => "backward_right",
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
    // Cancels a multiple choice. from, to, expires_at_tick, max_distance
    Cancel(u32, u32, i64, f32),
    /// An item to sell. item_id, seller_id, buyer_id, expires_at_tick, max_distance
    ItemToSell(u32, u32, u32, i64, f32),
    /// A script-defined choice. label, choice_attr, from, to, index, expires_at_tick, max_distance
    ScriptChoice(String, String, u32, u32, u32, i64, f32),
    /// A TOML-authored dialog choice.
    DialogChoice(DialogChoice),
}

impl Choice {
    pub fn session_meta(&self) -> (u32, u32, i64, f32) {
        match self {
            Choice::Cancel(from, to, expires_at_tick, max_distance) => {
                (*from, *to, *expires_at_tick, *max_distance)
            }
            Choice::ItemToSell(_, seller_id, buyer_id, expires_at_tick, max_distance) => {
                (*seller_id, *buyer_id, *expires_at_tick, *max_distance)
            }
            Choice::ScriptChoice(_, _, from, to, _, expires_at_tick, max_distance) => {
                (*from, *to, *expires_at_tick, *max_distance)
            }
            Choice::DialogChoice(choice) => (
                choice.from,
                choice.to,
                choice.expires_at_tick,
                choice.max_distance,
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DialogChoice {
    pub label: String,
    pub dialog: String,
    pub from: u32,
    pub to: u32,
    pub index: u32,
    pub next: Option<String>,
    pub event: Option<String>,
    pub end: bool,
    pub expires_at_tick: i64,
    pub max_distance: f32,
}

/// Multiple choices for the player
#[derive(Debug, Clone)]
pub struct MultipleChoice {
    pub region: u32,
    pub from: u32,
    pub to: u32,
    pub expires_at_tick: i64,
    pub max_distance: f32,

    pub choices: Vec<Choice>,
}

impl MultipleChoice {
    pub fn new(region: u32, from: u32, to: u32, expires_at_tick: i64, max_distance: f32) -> Self {
        Self {
            region,
            from,
            to,
            expires_at_tick,
            max_distance,
            choices: vec![],
        }
    }

    /// Add a choice
    pub fn add(&mut self, choice: Choice) {
        self.choices.push(choice);
    }
}
