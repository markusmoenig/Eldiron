pub mod area;
pub mod behavior;
pub mod nodes;
pub mod nodes_utility;

use std::collections::HashMap;
use std::fs::metadata;

use crate::gamedata::area::GameArea;
use crate::gamedata::behavior::{ BehaviorNodeConnector, BehaviorInstance, GameBehavior };
use crate::asset::TileUsage;

use itertools::Itertools;

use std::path;
use std::fs;

use self::behavior::{BehaviorNodeType};

type NodeCall = fn(instance_index: usize, id: (usize, usize), data: &mut GameData) -> behavior::BehaviorNodeConnector;

pub struct GameData {
    pub areas                   : HashMap<usize, GameArea>,
    pub areas_names             : Vec<String>,
    pub areas_ids               : Vec<usize>,

    pub behaviors               : HashMap<usize, GameBehavior>,
    pub behaviors_names         : Vec<String>,
    pub behaviors_ids           : Vec<usize>,

    pub nodes                   : HashMap<BehaviorNodeType, NodeCall>,

    pub instances               : Vec<BehaviorInstance>,

    pub say                     : Vec<String>,
    pub executed_connections    : Vec<(usize, BehaviorNodeConnector)>
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

            if md.is_file() {
                if let Some(name) = path::Path::new(&path).extension() {
                    if name == "json" || name == "JSON" {
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
                            behavior.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
                            behavior.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
                            behavior.save_data();
                        }
                        behaviors_ids.push(behavior.data.id);
                        behaviors.insert(behavior.data.id, behavior);
                    }
                }
            }
        }

        let mut nodes : HashMap<BehaviorNodeType, NodeCall> = HashMap::new();
        nodes.insert(BehaviorNodeType::Expression, nodes::expression);
        nodes.insert(BehaviorNodeType::Say, nodes::say);
        nodes.insert(BehaviorNodeType::Pathfinder, nodes::pathfinder);

        Self {
            areas,
            areas_names,
            areas_ids,

            behaviors,
            behaviors_names,
            behaviors_ids,

            nodes,

            instances               : vec![],

            say                     : vec![],
            executed_connections    : vec![],
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

    /// Sets the value for the given behavior id
    pub fn set_behavior_node_name(&mut self, id: (usize, usize), value: String) {
        if let Some(behavior) = self.behaviors.get_mut(&id.0) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                node.name = value;
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

    /// Gets the position for the given behavior
    pub fn get_behavior_default_position(&self, id: usize) -> Option<(usize, isize, isize)> {
        if let Some(behavior) = self.behaviors.get(&id) {
            for (_index, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(position) = node.values.get(&"position".to_string()) {
                        return Some((position.0 as usize, position.1 as isize, position.2 as isize));
                    }
                }
            }
        }
        None
    }

    /// Gets the position for the given behavior
    pub fn get_behavior_default_tile(&self, id: usize) -> Option<(usize, usize, usize)> {
        if let Some(behavior) = self.behaviors.get(&id) {
            for (_index, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(tile) = node.values.get(&"tile".to_string()) {
                        return Some((tile.0 as usize, tile.1 as usize, tile.2 as usize));
                    }
                }
            }
        }
        None
    }

    /// Create save data and return it
    pub fn save(&self) -> String {
        let json = serde_json::to_string(&self.instances).unwrap();
        json
    }

    /// Create a new behavior of the given id and return it's id
    pub fn create_behavior(&mut self, id: usize) -> usize {

        let mut to_execute : Vec<usize> = vec![];

        let mut position : Option<(usize, isize, isize)> = None;
        let mut tile     : Option<(usize, usize, usize)> = None;

        if let Some(behavior) = self.behaviors.get_mut(&id) {
            for (id, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree {

                    for c in &behavior.data.connections {
                        if c.0 == *id {
                            to_execute.push(c.0);
                        }
                    }
                } else
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value )= node.values.get(&"position".to_string()) {
                        position = Some((value.0 as usize, value.1 as isize, value.2 as isize));
                    }
                    if let Some(value )= node.values.get(&"tile".to_string()) {
                        tile = Some((value.0 as usize, value.1 as usize, value.2 as usize));
                    }
                }
            }

            let mut instance = BehaviorInstance {id: 0, name: behavior.name.clone(), behavior_id: id, tree_ids: to_execute.clone(), values: HashMap::new(), in_progress_id: None, position, tile};

            // Make sure id is unique
            let mut has_id_already = true;
            while has_id_already {

                has_id_already = false;
                for index in 0..self.instances.len() {
                    if self.instances[index].id == instance.id {
                        has_id_already = true;
                    }
                }

                if has_id_already {
                    instance.id += 1;
                }
            }

            let instance_id = instance.id.clone();
            self.instances.insert(instance_id, instance);

            return id;
        }

        0
    }

    /// Returns the tile at the given position
    pub fn get_tile_at(&self, pos: (usize, isize, isize)) -> Option<(usize, usize, usize, TileUsage)> {
        if let Some(area) = self.areas.get(&pos.0) {
            if let Some(value) = area.get_value((pos.1, pos.2)) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Game tick
    pub fn tick(&mut self) {
        self.say = vec![];
        self.executed_connections = vec![];
        for index in 0..self.instances.len() {
            let trees = self.instances[index].tree_ids.clone();
            for node_id in &trees {
                self.execute_node(index, node_id.clone());
            }
        }
    }

    /// Clear the game instances
    pub fn clear_instances(&mut self) {
        self.instances = vec![];
        self.say = vec![];
        self.executed_connections = vec![];
    }

    /// Executes the given node and follows the connection chain
    fn execute_node(&mut self, instance_index: usize, node_id: usize) {

        let mut connector : Option<BehaviorNodeConnector> = None;
        let mut connected_node_id : Option<usize> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(behavior) = self.behaviors.get_mut(&self.instances[instance_index].behavior_id) {
            if let Some(node) = behavior.data.nodes.get_mut(&node_id) {
                // println!("Executing:: {}", node.name);

                if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                    let behavior_id = self.instances[instance_index].behavior_id.clone();
                    connector = Some(node_call(instance_index, (behavior_id, node_id), self));
                } else {
                    connector = Some(BehaviorNodeConnector::Bottom);
                }
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        if let Some(connector) = connector {
            if let Some(behavior) = self.behaviors.get_mut(&self.instances[instance_index].behavior_id) {

                for c in &behavior.data.connections {
                    if c.0 == node_id && c.1 == connector {
                        connected_node_id = Some(c.2);
                        self.executed_connections.push((c.0, c.1));
                    }
                }
            }
        }

        // And if yes execute it
        if let Some(connected_node_id) = connected_node_id {
            self.execute_node(instance_index, connected_node_id);
        }
    }
}