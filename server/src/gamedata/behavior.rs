use serde::{Deserialize, Serialize};

use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone)]
pub enum BehaviorNodeType {
    BehaviorType,
    BehaviorTree,
    Expression,
    VariableNumber,
    VariablePosition,
    Pathfinder,
    Say,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum BehaviorNodeConnector {
    Top,
    Right,
    Fail,
    Success,
    Bottom,
    Left,
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
    pub id                      : usize,
    pub behavior_id             : usize,
    pub tree_ids                : Vec<usize>,
    pub values                  : HashMap<String, (f64, f64, f64, f64)>,

    // For character
    pub position                : Option<(usize, isize, isize)>,
    pub tile                    : Option<(usize, usize, usize)>,

    pub in_progress_id          : Option<usize>
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
        let json_path = path.join( format!("{}{}", name, ".json"));
        let contents = fs::read_to_string( json_path )
            .unwrap_or("".to_string());

        // Construct the json settings
        let data = serde_json::from_str(&contents)
            .unwrap_or(GameBehaviorData { nodes: HashMap::new(), connections: vec![], id: 0, name: "New Behavior".to_string() });

        Self {
            name        : name.to_string(),
            path        : path.clone(),
            data,
        }
    }

    /// Save the GameBehaviorData to file
    pub fn save_data(&self) {
        let json_path = self.path.join( format!("{}{}", self.name, ".json"));
        let json = serde_json::to_string(&self.data).unwrap();
        fs::write(json_path, json)
            .expect("Unable to write area file");
    }

    /// Add a new node of the given type and name
    pub fn add_node(&mut self, behavior_type: BehaviorNodeType, name: String) -> usize {

        let mut node = BehaviorNode {
            behavior_type: behavior_type.clone(),
            name,
            values: HashMap::new(),
            id: 0,
            position: (100, 100),
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
}