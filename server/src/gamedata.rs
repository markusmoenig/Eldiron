pub mod area;
pub mod behavior;
pub mod nodes;

use std::collections::HashMap;
use std::fs::metadata;

use crate::gamedata::area::GameArea;
use crate::gamedata::behavior::{ BehaviorNodeConnector, BehaviorInstance, GameBehavior };
use crate::asset::TileUsage;

use itertools::Itertools;

use std::path;
use std::fs;

use self::behavior::{BehaviorNodeType};

type NodeCall = fn(&mut BehaviorInstance, id: (usize, usize), data: &mut GameData) -> behavior::BehaviorNodeConnector;

pub struct GameData {
    pub areas                   : HashMap<usize, GameArea>,
    pub areas_names             : Vec<String>,
    pub areas_ids               : Vec<usize>,

    pub behaviors               : HashMap<usize, GameBehavior>,
    pub behaviors_names         : Vec<String>,
    pub behaviors_ids           : Vec<usize>,

    pub nodes                   : HashMap<BehaviorNodeType, NodeCall>,

    pub instances               : HashMap<usize, BehaviorInstance>
}

impl GameData {

    pub fn new() -> Self {

        // Create the tile areas
        let mut areas: HashMap<usize, GameArea> = HashMap::new();
        let mut areas_names = vec![];
        let mut areas_ids = vec![];

        let tilemaps_path = path::Path::new("game").join("areas");
        let paths = fs::read_dir(tilemaps_path).unwrap();

        for path in paths {
            let path = &path.unwrap().path();
            let md = metadata(path).unwrap();

            if md.is_dir() {
                let mut area = GameArea::new(path);

                areas_names.push(area.name.clone());

                // Make sure we create a unique id (check if the id already exists in the set)
                let mut has_id_already = true;
                while has_id_already {

                    has_id_already = false;
                    for (key, _value) in &areas {
                        if key == &area.data.id {
                            has_id_already = true;
                        }
                    }

                    if has_id_already {
                        area.data.id += 1;
                    }
                }

                area.calc_dimensions();

                areas_ids.push(area.data.id);
                areas.insert(area.data.id, area);
            }
        }

        let sorted_keys= areas.keys().sorted();
        for key in sorted_keys {
            let area = &areas[key];

            // If the area has no tiles we assume it's new and we save the data
            if area.data.tiles.len() == 0 {
                area.save_data();
            }
        }

        // Behaviors

        let behavior_path = path::Path::new("game").join("behavior");
        let paths = fs::read_dir(behavior_path).unwrap();

        let mut behaviors: HashMap<usize, GameBehavior> = HashMap::new();
        let mut behaviors_names = vec![];
        let mut behaviors_ids = vec![];

        for path in paths {
            let path = &path.unwrap().path();
            let md = metadata(path).unwrap();

            if md.is_dir() {
                let mut behavior = GameBehavior::new(path);

                behaviors_names.push(behavior.name.clone());

                // Make sure we create a unique id (check if the id already exists in the set)
                let mut has_id_already = true;
                while has_id_already {

                    has_id_already = false;
                    for (key, _value) in &behaviors {
                        if key == &behavior.data.id {
                            has_id_already = true;
                        }
                    }

                    if has_id_already {
                        behavior.data.id += 1;
                    }
                }

                if behavior.data.nodes.len() == 0 {
                    behavior.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
                    behavior.save_data();
                }
                behaviors_ids.push(behavior.data.id);
                behaviors.insert(behavior.data.id, behavior);
            }
        }

        // let sorted_keys= behaviors.keys().sorted();
        // for key in sorted_keys {
        //     let behavior = &behaviors[key];

        //     // If the behavior has no nodes we assume it's new and we save the data
        //     if behavior.data.nodes.len() == 0 {
        //         behavior.save_data();
        //     }
        // }

        let mut nodes : HashMap<BehaviorNodeType, NodeCall> = HashMap::new();
        nodes.insert(BehaviorNodeType::DiceCheck, nodes::dice_check);
        nodes.insert(BehaviorNodeType::Expression, nodes::expression);
        nodes.insert(BehaviorNodeType::Say, nodes::say);

        Self {
            areas,
            areas_names,
            areas_ids,

            behaviors,
            behaviors_names,
            behaviors_ids,

            nodes,

            instances               : HashMap::new(),
        }
    }

    /// Sets a value in the current area
    pub fn save_area(&self, id: usize) {
        let area = &mut self.areas.get(&id).unwrap();
        area.save_data();
    }

    /// Sets a value in the area
    pub fn set_area_value(&mut self, id: usize, pos: (isize, isize), value: (usize, usize, usize, TileUsage)) {
        let area = &mut self.areas.get_mut(&id).unwrap();
        area.set_value(pos, value);
    }

    /// Sets the value for the given behavior id
    pub fn set_behavior_id_value(&mut self, id: (usize, usize, String), value: (f64, f64, f64, f64, String)) {
        if let Some(behavior) = self.behaviors.get_mut(&id.0) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                node.values.insert(id.2.clone(), value);
                behavior.save_data();
            }
        }
    }

    /// Gets the value of the behavior id
    pub fn get_behavior_id_value(&self, id: (usize, usize, String), def: (f64, f64, f64, f64, String)) -> (f64, f64, f64, f64, String) {
        if let Some(behavior) = self.behaviors.get(&id.0) {
            if let Some(node) = behavior.data.nodes.get(&id.1) {
                if let Some(v) = node.values.get(&id.2) {
                    return v.clone();
                }
            }
        }
        def
    }

    /// Create save data and return it
    pub fn save(&self) -> String {
        let json = serde_json::to_string(&self.instances).unwrap();
        json
    }

    /// Create a new behavior of the given id and return it's id
    pub fn create_behavior(&mut self, id: usize, is_temporary: bool, execute: bool) -> usize {

        let mut to_execute : Vec<usize> = vec![];

        if let Some(behavior) = self.behaviors.get_mut(&id) {
            for (id, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree {

                    for c in &behavior.data.connections {
                        if c.0 == *id {
                            to_execute.push(c.2);
                        }
                    }
                }
            }

            let mut instance = BehaviorInstance {id: 0, behavior_id: id, tree_ids: to_execute.clone(), values: HashMap::new(), in_progress_id: None};

            if is_temporary == false {
                // Make sure id is unique
                let mut has_id_already = true;
                while has_id_already {

                    has_id_already = false;
                    for (key, _value) in &self.instances {
                        if key == &instance.id {
                            has_id_already = true;
                        }
                    }

                    if has_id_already {
                        instance.id += 1;
                    }
                }
            }

            //let node_id = instance.id.clone();

            if execute {
                for node_id in to_execute {
                    self.execute_node(&mut instance, (id.clone(), node_id));
                }
            }

            if is_temporary == false {
                self.instances.insert(instance.id.clone(), instance);
            }

            return id;
        }

        0
    }

    /// Executes the given node and follows the connection chain
    fn execute_node(&mut self, instance: &mut BehaviorInstance, id: (usize, usize)) {

        let mut connector : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(behavior) = self.behaviors.get_mut(&id.0) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                // println!("Executing:: {}", node.name);

                if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                    connector = Some(node_call(instance, id, self));
                }
            }
        }

        // Search the connections if we can find an ongoing node connection
        let mut connected_node_id : Option<usize> = None;
        if let Some(connector) = connector {
            if let Some(behavior) = self.behaviors.get_mut(&id.0) {

                for c in &behavior.data.connections {
                    if c.0 == id.1 && c.1 == connector {
                        connected_node_id = Some(c.2);
                    }
                }
            }
        }

        // And if yes execute it
        if let Some(connected_node_id) = connected_node_id {
            self.execute_node(instance, (id.0, connected_node_id));
        }
    }
}