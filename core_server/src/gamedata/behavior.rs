use crate::prelude::*;

use serde::{Deserialize, Serialize};
use rand::prelude::*;
use uuid::Uuid;

use std::collections::HashSet;
use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;
use itertools::Itertools;

pub type Position = (usize, isize, isize);
pub type Tile = (usize, usize, usize);

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
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum BehaviorNodeType {
    BehaviorType,
    BehaviorTree,
    Expression,
    VariableNumber,
    VariablePosition,
    Script,
    Pathfinder,
    Message,
    Lookout,
    CloseIn,
    CallSystem,
    CallBehavior,
    Sequence,
    LockTree,
    UnlockTree,
    SetState,
    Linear,
    InsideArea,
    EnterArea,
    LeaveArea,
    Spawn,
    DisplaceTiles,
    Move,
    Screen,
    Widget,
    Settings,
    TeleportArea,                                           // Teleport Characters inside an area
    MessageArea,
    AudioArea,
    LightArea,
    Always
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BehaviorNode {
    pub behavior_type           : BehaviorNodeType,
    pub name                    : String,

    pub values                  : HashMap<String, (f64, f64, f64, f64, String)>,
    pub id                      : usize,

    pub position                : (isize, isize),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum BehaviorInstanceState {
    Normal,
    Hidden,
    Killed,
    Purged,
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
    pub alignment               : i64,

    // The behavior id for this instance
    pub behavior_id             : usize,

    // The current systems id
    pub systems_id              : usize,

    // The ids of the behavior tree nodes for this instance
    pub tree_ids                : Vec<usize>,

    // The name of the instance
    pub name                    : String,

    // Store all variables in the instances rhai::Scope
    // This is for serialization only / deserialization only, not used at runtime
    pub scope_buffer            : Option<ScopeBuffer>,

    // An instance id of the entity we are currently interacting with
    pub target_instance_index   : Option<usize>,

    // The number of ticks this instance is skipping
    pub sleep_cycles            : usize,

    // The locked tree, only this tree will be executed.
    pub locked_tree             : Option<usize>,

    // Instance ids of the entities in our party (including self)
    pub party                   : Vec<usize>,

    // Temporary values nodes can use to store instance data, these are NOT saved, i.e. emptied before saving.
    // The key is the behavior type and node id.
    pub node_values             : HashMap<(BehaviorType, usize), (f64, f64, f64, f64, String)>,

    // State values to optionally store game state related to this instance. This data is saved.
    pub state_values            : HashMap<String, (f64, f64, f64, f64, String)>,

    // For characters, the 2D position id and the currently displayed tile id.
    pub position                : Option<(usize, isize, isize)>,
    pub old_position            : Option<(usize, isize, isize)>,
    pub max_transition_time     : usize,
    pub curr_transition_time    : usize,

    pub tile                    : Option<(usize, usize, usize)>,

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
    pub regions_send            : HashSet<usize>,

    /// Current screen id
    pub curr_player_screen_id   : Option<usize>,

    /// Current screen content
    pub curr_player_screen      : String,

    /// The locked tree for the game behavior for this player
    pub game_locked_tree        : Option<usize>,
}

// An instance of a game behavior data
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterInstanceData {
    pub position                : Position,
    pub name                    : Option<String>,
    pub tile                    : Option<Tile>,
    pub alignment               : i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameBehaviorData {
    pub nodes                   : HashMap<usize, BehaviorNode>,
    pub connections             : Vec<(usize, BehaviorNodeConnector, usize, BehaviorNodeConnector)>,
    pub id                      : usize,

    pub name                    : String,

    pub curr_node_id            : Option<usize>,

    pub instances               : Option<Vec<CharacterInstanceData>>,

    pub settings                : Option<PropertySink>
}

impl GameBehaviorData {
    pub fn new() -> Self {
        Self {
            nodes                   : HashMap::new(),
            connections             : vec![],
            id                      : 0,
            name                    : "".to_string(),
            curr_node_id            : None,
            instances               : Some(vec![]),
            settings                : None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct BehaviorDebugData {
    //pub messages                    : Vec<(String, MessageType)>,
    pub executed_connections        : Vec<(BehaviorType, usize, BehaviorNodeConnector)>,
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
            .unwrap_or(GameBehaviorData { nodes: HashMap::new(), connections: vec![], id: thread_rng().gen_range(1..=u32::MAX) as usize, name: "New Behavior".to_string(), curr_node_id: None, instances: Some(vec![]), settings: None });

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
        let mut data = GameBehaviorData { nodes: HashMap::new(), connections: vec![], id: thread_rng().gen_range(1..=u32::MAX) as usize, name: "New Behavior".to_string(), curr_node_id: None, instances: Some(vec![]), settings: None };

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
            data            : GameBehaviorData { nodes: HashMap::new(), connections: vec![], id: thread_rng().gen_range(1..=u32::MAX) as usize, name            : "New Behavior".to_string(), curr_node_id: None, instances: Some(vec![]), settings: None }
        }
    }

    /// Save the GameBehaviorData to file
    pub fn save_data(&self) {
        let json = serde_json::to_string(&self.data).unwrap();
        fs::write(self.path.clone(), json)
            .expect("Unable to write area file");
    }

    /// Add a new node of the given type and name
    pub fn add_node(&mut self, behavior_type: BehaviorNodeType, name: String) -> usize {

        let mut node = BehaviorNode {
            behavior_type: behavior_type.clone(),
            name,
            values: HashMap::new(),
            id: 0,
            position: (250, 50),
        };

        if behavior_type == BehaviorNodeType::BehaviorType {
            node.position = (0, 0);
        }

        let mut has_id_already = true;
        while has_id_already {

            has_id_already = false;
            for (key, _value) in &self.data.nodes {
                if key == &node.id {
                    has_id_already = true;
                }
            }

            if has_id_already {
                node.id += 1;
            }
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