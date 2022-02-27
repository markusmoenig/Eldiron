use serde::{Deserialize, Serialize};

use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq)]
pub enum BehaviorNodeType {
    BehaviorTree,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum BehaviorNodeState {
    Idle,
    Running,
}

#[derive(Serialize, Deserialize)]
pub struct BehaviorNode {
    pub behavior_type           : BehaviorNodeType,
    pub behavior_state          : BehaviorNodeState,
    pub name                    : String,

    pub values                  : HashMap<String, (f64, f64, f64, f64)>,
    pub id                      : usize,

    pub position                : (isize, isize),

    pub ok_connection           : Option<usize>,
    pub fail_connection         : Option<usize>,
    pub right_connection        : Option<usize>,

    pub connected               : bool,
}

#[derive(Serialize, Deserialize)]
pub struct GameBehaviorData {
    pub nodes           : Vec<BehaviorNode>,
    pub id              : usize,

    pub name            : String,
}

pub struct GameBehavior {
    pub name            : String,
    pub path            : PathBuf,
    pub data            : GameBehaviorData,
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
            .unwrap_or(GameBehaviorData { nodes: vec![], id: 0, name: "New Behavior".to_string() });

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
    pub fn add_node(&mut self, behavior_type: BehaviorNodeType, name: String) {
        let mut node = BehaviorNode { behavior_type: behavior_type, behavior_state: BehaviorNodeState::Idle, name, values: HashMap::new(),id: 0, position: (100, 100), ok_connection: None, fail_connection: None, right_connection: None, connected: false };


        let mut has_id_already = true;
        while has_id_already {

            has_id_already = false;
            for n in &self.data.nodes {
                if n.id == node.id {
                    has_id_already = true;
                }
            }

            if has_id_already {
                node.id += 1;
            }
        }

        self.data.nodes.push(node);
    }
}