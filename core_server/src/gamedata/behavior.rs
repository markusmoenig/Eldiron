use crate::prelude::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::collections::HashSet;
use std::fs;
use std::path;
use std::path::PathBuf;

use itertools::Itertools;

//pub type Position = (usize, isize, isize);
//pub type Tile = (Uuid, usize, usize);

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum BehaviorType {
    Tiles,
    Regions,
    Behaviors,
    Systems,
    Items,
    GameLogic,
    Spells,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum BehaviorNodeType {
    BehaviorType,
    BehaviorTree,
    Sequence,
    Linear,
    Expression,
    Script,
    Move,
    Screen,
    InsideArea,
    EnterArea,
    LeaveArea,
    Always,
    TeleportArea,                                           // Teleport characters when inside an area
    MessageArea,                                            // Send a message to a character when in an area
    AudioArea,                                              // Play audio when in an area
    LightArea,                                              // Light when in an area
    Message,
    Action,                                                 // Player Action
    ActionArea,                                             // Define Action for an Area
    Take,                                                   // Take loot
    Drop,                                                   // Drop gear or item
    LightItem,                                              // Creates a light source for an inventory item
    SetItemTile,                                            // Sets the tile of the item
    RandomWalk,                                             // Random walk for NPCs
    Pathfinder,                                             // Go somewhere
    CloseIn,                                                // Close In on another character
    Lookout,                                                // Look out for another character
    MultiChoice,                                            // Multi Choice
    LockTree,
    UnlockTree,
    SetState,
    Sell,
    CallSystem,
    CallBehavior,
    HasTarget,
    Untarget,
    DealDamage,
    TakeDamage,
    DropInventory,
    Target,
    Teleport,
    Effect,
    Audio,
    Heal,
    TakeHeal,
    Respawn,
    Widget,
    Equip,
    SkillTree,
    SkillLevel,
    SkillLevelItem,
    LevelTree,
    Level,
    SetLevelTree,
    OverlayTiles,
    Schedule,
    HasState,
    MagicTarget,
    MagicDamage,
    Cellular,
    DrunkardsWalk,
    StartArea,
    TeleportToArea,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum BehaviorNodeConnector {
    Top,
    Right,
    Fail,
    Success,
    Bottom,
    Left,
    Bottom1,
    Bottom2,
    Bottom3,
    Bottom4,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BehaviorNode {
    pub behavior_type           : BehaviorNodeType,
    pub name                    : String,

    #[serde(skip)]
    pub asts                    : FxHashMap<String, rhai::AST>,

    pub values                  : FxHashMap<String, Value>,
    pub id                      : Uuid,

    pub position                : (isize, isize),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum BehaviorInstanceState {
    Normal,
    Killed,
    Purged,
    Sleeping,
    Intoxicated
}

impl BehaviorInstanceState {
    // Character is dead
    pub fn is_dead(&self) -> bool {
        match self {
            Self::Killed => true,
            Self::Purged => true,
            _ => false
        }
    }

    // Character is alive
    pub fn is_alive(&self) -> bool {
        match self {
            Self::Killed => false,
            Self::Purged => false,
            _ => true
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum BehaviorInstanceType {
    NonPlayerCharacter,
    Player,
    GameLogic
}

// Server instance of a behavior
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BehaviorInstance {

    // The instance id (unique)
    pub id                      : Uuid,

    // The instance state
    pub instance_type           : BehaviorInstanceType,

    // The instance state
    pub state                   : BehaviorInstanceState,

    // Alignment
    pub alignment               : i32,

    // The behavior id for this instance
    pub behavior_id             : Uuid,

    // The current systems id
    pub systems_id              : Uuid,

    // The ids of the behavior tree nodes for this instance
    pub tree_ids                : Vec<Uuid>,

    // The name of the instance
    pub name                    : String,

    // Store all variables in the instances rhai::Scope
    // This is for serialization only / deserialization only, not used at runtime
    pub scope_buffer            : Option<ScopeBuffer>,

    // Store the inventory / weapons / gear / skills / experience
    // This is for serialization / deserialization only, not used at runtime
    pub inventory_buffer        : Option<String>,
    pub weapons_buffer          : Option<String>,
    pub gear_buffer             : Option<String>,
    pub skills_buffer           : Option<String>,
    pub experience_buffer       : Option<String>,

    // An instance index of the entity we are currently interacting with
    pub target_instance_index   : Option<usize>,

    // The number of ticks this instance is skipping
    pub sleep_cycles            : usize,

    // The locked tree, only this tree will be executed.
    pub locked_tree             : Option<Uuid>,

    // Instance ids of the entities in our party (including self)
    pub party                   : Vec<Uuid>,

    // The key is the behavior id and node id.
    pub node_values             : FxHashMap<(Uuid, Uuid), Value>,

    // For characters, the 2D position id and the currently displayed tile id.
    pub position                : Option<Position>,
    pub old_position            : Option<Position>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : Option<TileId>,

    // Messages for this player in the current tick
    pub messages                : Vec<MessageData>,

    // Audio files to play for this player in the current tick
    pub audio                   : Vec<String>,

    /// The current player action
    pub action                  : Option<PlayerAction>,

    // Server side handling of the "Player" character

    /// The current player update
    pub update                  : Option<String>,

    /// The regions we send to the player client already
    pub regions_send            : HashSet<Uuid>,

    /// Current screen id
    pub curr_player_screen_id   : Option<Uuid>,

    /// Current screen content
    pub curr_player_screen      : String,
    pub curr_player_widgets     : Vec<String>,

    /// Did we send the screen scripts to the client already ?
    pub send_screen_scripts     : bool,

    /// The locked tree for the game behavior for this player
    pub game_locked_tree        : Option<Uuid>,

    /// Multi choice data for the player character
    pub multi_choice_data       : Vec<MultiChoiceData>,

    /// Answer
    pub multi_choice_answer     : Option<Uuid>,

    /// This character has an ongoing communication
    pub communication           : Vec<PlayerCommunication>,

    /// Damage to be dealt to this character
    pub damage_to_be_dealt      : Option<i32>,

    /// Healing to be dealt to this character
    pub healing_to_be_dealt     : Option<i32>,

    /// Effects to be played in this tick
    pub effects                 : Vec<TileId>,

    /// The instance data at creation time, needed for the respawn node
    pub instance_creation_data  : Option<CharacterInstanceData>,
}

/// Represents a character behavior instance
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CharacterInstanceData {
    pub position                : Position,
    pub name                    : Option<String>,
    pub tile                    : Option<TileId>,
    pub alignment               : i32,
}

/// Represents loot instance
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LootInstanceData {
    pub position                : Position,
    pub name                    : Option<String>,
    pub tile                    : Option<TileId>,
    pub amount                  : i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameBehaviorData {
    pub nodes                   : FxHashMap<Uuid, BehaviorNode>,
    pub connections             : Vec<(Uuid, BehaviorNodeConnector, Uuid, BehaviorNodeConnector)>,
    pub id                      : Uuid,

    pub name                    : String,

    pub curr_node_id            : Option<Uuid>,

    pub instances               : Option<Vec<CharacterInstanceData>>,
    pub loot                    : Option<Vec<LootInstanceData>>,

    pub settings                : Option<PropertySink>
}

impl GameBehaviorData {
    pub fn new() -> Self {
        Self {
            nodes                   : FxHashMap::default(),
            connections             : vec![],
            id                      : Uuid::new_v4(),
            name                    : "".to_string(),
            curr_node_id            : None,
            instances               : Some(vec![]),
            loot                    : Some(vec![]),
            settings                : None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BehaviorDebugData {
    pub executed_connections        : Vec<(BehaviorType, Uuid, BehaviorNodeConnector)>,
    pub script_errors               : Vec<((Uuid, Uuid, String), (String, Option<u32>))>
}

pub struct GameBehavior {
    pub name                    : String,
    pub path                    : PathBuf,
    pub behavior_path           : PathBuf,
    pub data                    : GameBehaviorData,
}

impl GameBehavior {
    pub fn load_from_path(path: &PathBuf, behavior_path: &PathBuf) -> Self {

        let name = path::Path::new(&path).file_stem().unwrap().to_str().unwrap();

        // Gets the content of the settings file
        let contents = fs::read_to_string( path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let mut data = serde_json::from_str(&contents)
            .unwrap_or(GameBehaviorData { nodes: FxHashMap::default(), connections: vec![], id: Uuid::new_v4(), name: "New Behavior".to_string(), curr_node_id: None, instances: Some(vec![]), loot: Some(vec![]), settings: None });

        data.name = name.to_owned();

        Self {
            name            : name.to_string(),
            path            : path.clone(),
            behavior_path   : behavior_path.clone(),
            data,
        }
    }

    #[cfg(feature = "embed_binaries")]
    pub fn load_from_embedded(file_name: &str) -> Self {

        let name = path::Path::new(&file_name).file_stem().unwrap().to_str().unwrap();

        // Construct the json settings
        let mut data = GameBehaviorData { nodes: FxHashMap::default(), connections: vec![], id: Uuid::new_v4(), name: "New Behavior".to_string(), curr_node_id: None, instances: Some(vec![]), loot: Some(vec![]), settings: None };

        if let Some(bytes) = Embedded::get(file_name) {
            if let Some(string) = std::str::from_utf8(bytes.data.as_ref()).ok() {
                data = serde_json::from_str(&string).unwrap();
            }
        }

        Self {
            name            : name.to_string(),
            path            : PathBuf::new(),
            behavior_path   : PathBuf::new(),
            data,
        }
    }

    pub fn new() -> Self {

        Self {
            name            : "name".to_string(),
            path            : std::path::Path::new("").to_path_buf(),
            behavior_path   : std::path::Path::new("").to_path_buf(),
            data            : GameBehaviorData { nodes: FxHashMap::default(), connections: vec![], id: Uuid::new_v4(), name            : "New Behavior".to_string(), curr_node_id: None, instances: Some(vec![]), loot: Some(vec![]), settings: None }
        }
    }

    /// Save the GameBehaviorData to file
    pub fn save_data(&self) {
        let json = serde_json::to_string(&self.data).unwrap();
        fs::write(self.path.clone(), json)
            .expect("Unable to write behavior file");
    }

    /// Add a new node of the given type and name
    pub fn add_node(&mut self, behavior_type: BehaviorNodeType, name: String) -> Uuid {

        let mut node = BehaviorNode {
            behavior_type: behavior_type.clone(),
            name,
            values      : FxHashMap::default(),
            id          : Uuid::new_v4(),
            position    : (250, 50),
            asts        : FxHashMap::default(),
        };

        if behavior_type == BehaviorNodeType::BehaviorType {
            node.position = (0, 0);
        }

        // Insert the node
        let id = node.id.clone();
        self.data.nodes.insert(node.id, node);
        id
    }

    /// Rename the behavior
    pub fn rename(&mut self, name: String) {
        self.name = name.clone();
        if std::fs::rename(self.path.clone(), self.behavior_path.join(name.clone() + ".json")).is_ok() {
            _ = std::fs::remove_file(self.path.clone());
            self.path = self.behavior_path.join(name + ".json");
        }
    }

    /// Get the names of the behavior tree nodes.
    pub fn get_behavior_tree_names(&self) -> Vec<String> {
        let mut names : Vec<String> = vec![];

        let sorted_keys = self.data.nodes.keys().sorted();

        for i in sorted_keys {
            if self.data.nodes[i].behavior_type == BehaviorNodeType:: BehaviorTree {
                names.push( self.data.nodes[i].name.clone() );
            }

        }
        names
    }

}

// Settings

pub fn update_behavior_sink(sink: &mut PropertySink) {

    if sink.contains("race") == false {
        sink.properties.push(Property::new_string("race".to_string(), "human".to_string()));
    }

    if sink.contains("class") == false {
        sink.properties.push(Property::new_string("class".to_string(), "paladin".to_string()));
    }

}

pub fn generate_behavior_sink_descriptions() -> FxHashMap<String, Vec<String>> {
    let mut map : FxHashMap<String, Vec<String>> = FxHashMap::default();

    map.insert("race".to_string(), vec!["The race of the character.".to_string()]);
    map.insert("class".to_string(), vec!["The class of the character.".to_string()]);

    map
}