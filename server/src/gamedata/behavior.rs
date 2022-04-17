use serde::{Deserialize, Serialize};
use rand::prelude::*;

use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;
use itertools::Itertools;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum BehaviorType {
    Tiles,
    Areas,
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
    Sequence
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
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct BehaviorNode {
    pub behavior_type           : BehaviorNodeType,
    pub name                    : String,

    pub values                  : HashMap<String, (f64, f64, f64, f64, String)>,
    pub id                      : usize,

    pub position                : (isize, isize),
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct BehaviorInstance {

    // The instance id (unique)
    pub id                      : usize,

    // The behavior id for this instance
    pub behavior_id             : usize,

    // The current systems id
    pub systems_id              : usize,

    // The ids of the behavior tree nodes for this instance
    pub tree_ids                : Vec<usize>,

    // The name of the instance
    pub name                    : String,

    // Store number variables.
    // This is for serialization only / deserialization only, not used at runtime
    pub number_values           : HashMap<String, f64>,

    // An instance id of the entity we are currently interacting with
    pub target_instance_index   : Option<usize>,

    // The number of ticks this instance is skipping
    pub sleep_cycles            : usize,

    // Instance ids of the entities we are currently engaged in combat with
    pub engaged_with            : Vec<usize>,

    // Temporary values nodes can use to store instance data, these are NOT saved, i.e. emptied before saving.
    // The key is the behavior type and node id.
    pub node_values             : HashMap<(BehaviorType, usize), (f64, f64, f64, f64, String)>,

    // State values to optionally store game state related to this instance. This data is saved.
    pub state_values            : HashMap<String, (f64, f64, f64, f64, String)>,

    // For characters, the 2D position id and the currently displayed tile id.
    pub position                : Option<(usize, isize, isize)>,
    pub tile                    : Option<(usize, usize, usize)>,
}

#[derive(Serialize, Deserialize)]
pub struct GameBehaviorData {
    pub nodes                   : HashMap<usize, BehaviorNode>,
    pub connections             : Vec<(usize, BehaviorNodeConnector, usize, BehaviorNodeConnector)>,
    pub id                      : usize,

    pub name                    : String,
}

pub struct GameBehavior {
    pub name                    : String,
    pub path                    : PathBuf,
    pub data                    : GameBehaviorData,
}

impl GameBehavior {
    pub fn new(path: &PathBuf) -> Self {

        let name = path::Path::new(&path).file_stem().unwrap().to_str().unwrap();

        // Gets the content of the settings file
        let contents = fs::read_to_string( path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let data = serde_json::from_str(&contents)
            .unwrap_or(GameBehaviorData { nodes: HashMap::new(), connections: vec![], id: thread_rng().gen_range(1..=u32::MAX) as usize, name: "New Behavior".to_string() });

        Self {
            name        : name.to_string(),
            path        : path.clone(),
            data,
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
    pub fn rename(&mut self, name: String, path: String) {
        self.name = name.clone();
        let _ = std::fs::rename(self.path.clone(), path::Path::new("game").join(path).join(name + ".json"));
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