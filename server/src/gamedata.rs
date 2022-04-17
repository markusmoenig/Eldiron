pub mod area;
pub mod behavior;
pub mod nodes;
pub mod nodes_utility;
pub mod script;

use rhai::{ Engine, Scope };

use std::collections::HashMap;
use std::fs::metadata;

use crate::gamedata::area::GameArea;
use crate::gamedata::behavior::{ BehaviorNodeConnector, BehaviorInstance, GameBehavior, BehaviorNodeType, BehaviorType };
use crate::asset::TileUsage;

use itertools::Itertools;

use std::path;
use std::fs;

use rand::prelude::*;

use self::nodes_utility::get_node_value;

type NodeCall = fn(instance_index: usize, id: (usize, usize), data: &mut GameData, behavior_type: BehaviorType) -> behavior::BehaviorNodeConnector;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum MessageType {
    Say,
    Yell,
    Debug,
    Error,
}

pub struct GameData<'a> {
    pub areas                   : HashMap<usize, GameArea>,
    pub areas_names             : Vec<String>,
    pub areas_ids               : Vec<usize>,

    pub behaviors               : HashMap<usize, GameBehavior>,
    pub behaviors_names         : Vec<String>,
    pub behaviors_ids           : Vec<usize>,

    pub systems                 : HashMap<usize, GameBehavior>,
    pub systems_names           : Vec<String>,
    pub systems_ids             : Vec<usize>,

    pub items                   : HashMap<usize, GameBehavior>,
    pub items_names             : Vec<String>,
    pub items_ids               : Vec<usize>,

    pub nodes                   : HashMap<BehaviorNodeType, NodeCall>,

    pub engine                  : Engine,

    // All instances
    pub instances               : Vec<BehaviorInstance>,
    // Currently active instances
    pub active_instance_indices : Vec<usize>,

    pub scopes                  : Vec<Scope<'a>>,

    pub runs_in_editor          : bool,

    // These are fields which provide feedback to the editor / game while running
    pub messages                : Vec<(String, MessageType)>,
    pub executed_connections    : Vec<(usize, BehaviorNodeConnector)>,
    pub changed_variables       : Vec<(usize, usize, usize, f64)>, // A variable has been changed: instance index, behavior id, node id, new value
}

impl GameData<'_> {

    pub fn new() -> Self {

        // Create the tile areas
        let mut areas: HashMap<usize, GameArea> = HashMap::new();
        let mut areas_names = vec![];
        let mut areas_ids = vec![];

        let area_path = path::Path::new("game").join("areas");
        let paths = fs::read_dir(area_path).unwrap();

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

        // Systems

        let systems_path = path::Path::new("game").join("systems");
        let paths = fs::read_dir(systems_path).unwrap();

        let mut systems: HashMap<usize, GameBehavior> = HashMap::new();
        let mut systems_names = vec![];
        let mut systems_ids = vec![];

        for path in paths {
            let path = &path.unwrap().path();
            let md = metadata(path).unwrap();

            if md.is_file() {
                if let Some(name) = path::Path::new(&path).extension() {
                    if name == "json" || name == "JSON" {
                        let mut system = GameBehavior::new(path);
                        systems_names.push(system.name.clone());

                        // Make sure we create a unique id (check if the id already exists in the set)
                        let mut has_id_already = true;
                        while has_id_already {

                            has_id_already = false;
                            for (key, _value) in &systems {
                                if key == &system.data.id {
                                    has_id_already = true;
                                }
                            }

                            if has_id_already {
                                system.data.id += 1;
                            }
                        }

                        if system.data.nodes.len() == 0 {
                            // behavior.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
                            // behavior.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
                            // behavior.save_data();
                        }
                        systems_ids.push(system.data.id);
                        systems.insert(system.data.id, system);
                    }
                }
            }
        }

        // Items

        let item_path = path::Path::new("game").join("items");
        let paths = fs::read_dir(item_path).unwrap();

        let mut items: HashMap<usize, GameBehavior> = HashMap::new();
        let mut items_names = vec![];
        let mut items_ids = vec![];

        for path in paths {
            let path = &path.unwrap().path();
            let md = metadata(path).unwrap();

            if md.is_file() {
                if let Some(name) = path::Path::new(&path).extension() {
                    if name == "json" || name == "JSON" {
                        let mut item = GameBehavior::new(path);
                        items_names.push(item.name.clone());

                        // Make sure we create a unique id (check if the id already exists in the set)
                        let mut has_id_already = true;
                        while has_id_already {

                            has_id_already = false;
                            for (key, _value) in &behaviors {
                                if key == &item.data.id {
                                    has_id_already = true;
                                }
                            }

                            if has_id_already {
                                item.data.id += 1;
                            }
                        }

                        if item.data.nodes.len() == 0 {
                            // behavior.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
                            // behavior.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
                            // behavior.save_data();
                        }
                        items_ids.push(item.data.id);
                        items.insert(item.data.id, item);
                    }
                }
            }
        }

        let mut nodes : HashMap<BehaviorNodeType, NodeCall> = HashMap::new();
        nodes.insert(BehaviorNodeType::Expression, nodes::expression);
        nodes.insert(BehaviorNodeType::Script, nodes::script);
        nodes.insert(BehaviorNodeType::Say, nodes::say);
        nodes.insert(BehaviorNodeType::Pathfinder, nodes::pathfinder);
        nodes.insert(BehaviorNodeType::Lookout, nodes::lookout);
        nodes.insert(BehaviorNodeType::CloseIn, nodes::close_in);
        nodes.insert(BehaviorNodeType::CallSystem, nodes::call_system);
        nodes.insert(BehaviorNodeType::CallBehavior, nodes::call_behavior);

        Self {
            areas,
            areas_names,
            areas_ids,

            behaviors,
            behaviors_names,
            behaviors_ids,

            systems,
            systems_names,
            systems_ids,

            items,
            items_names,
            items_ids,

            nodes,

            engine                  : Engine::new(),

            instances               : vec![],
            active_instance_indices : vec![],

            scopes                  : vec![],

            runs_in_editor          : true,

            messages                : vec![],
            executed_connections    : vec![],
            changed_variables       : vec![],
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

    /// Create a new behavior
    pub fn create_behavior(&mut self, name: String, _behavior_type: usize) {

        let path = path::Path::new("game").join("behavior").join(name.clone() + ".json");

        let mut behavior = GameBehavior::new(&path);
        behavior.data.name = name.clone();

        self.behaviors_names.push(behavior.name.clone());
        self.behaviors_ids.push(behavior.data.id);

        behavior.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
        behavior.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
        behavior.save_data();

        self.behaviors.insert(behavior.data.id, behavior);
    }

    /// Create a new system
    pub fn create_system(&mut self, name: String, _behavior_type: usize) {

        let path = path::Path::new("game").join("systems").join(name.clone() + ".json");

        let mut system = GameBehavior::new(&path);
        system.data.name = name.clone();

        self.systems_names.push(system.name.clone());
        self.systems_ids.push(system.data.id);

        system.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
        system.save_data();

        self.systems.insert(system.data.id, system);
    }

    /// Sets the value for the given behavior id
    pub fn set_behavior_id_value(&mut self, id: (usize, usize, String), value: (f64, f64, f64, f64, String), behavior_type: BehaviorType) {
        if let Some(behavior) = self.get_mut_behavior(id.0, behavior_type) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                node.values.insert(id.2.clone(), value);
                behavior.save_data();
            }
        }
    }

    /// Sets the name for the given node
    pub fn set_behavior_node_name(&mut self, id: (usize, usize), value: String, behavior_type: BehaviorType) {
        if let Some(behavior) = self.get_mut_behavior(id.0, behavior_type) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                node.name = value;
                behavior.save_data();
            }
        }
    }

    /// Gets the value of the behavior id
    pub fn get_behavior_id_value(&self, id: (usize, usize, String), def: (f64, f64, f64, f64, String), behavior_type: BehaviorType) -> (f64, f64, f64, f64, String) {
        if let Some(behavior) = self.get_behavior(id.0, behavior_type) {
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

    /// Save data and return it
    pub fn save(&self) -> String {
        let json = serde_json::to_string(&self.instances).unwrap();
        json
    }

    /// Creates all behavior instances
    pub fn create_behavior_instances(&mut self) {
        self.active_instance_indices = vec![];
        for index in 0..self.behaviors_ids.len() {
            self.create_behavior_instance(self.behaviors_ids[index]);
        }
    }

    /// Activate the instances for the given area
    pub fn activate_area_instances(&mut self, area_id: usize) {
        self.active_instance_indices = vec![];
        for index in 0..self.instances.len() {
            if let Some(position) = self.instances[index].position {
                if position.0 == area_id {
                    self.active_instance_indices.push(index);
                }
            }
        }
    }

    /// Create a new behavior instance for the given id and return it's id
    pub fn create_behavior_instance(&mut self, id: usize) -> usize {

        let mut to_execute : Vec<usize> = vec![];

        let mut position : Option<(usize, isize, isize)> = None;
        let mut tile     : Option<(usize, usize, usize)> = None;

        let mut scope = Scope::new();

        // Insert Dices
        for d in (2..=20).step_by(2) {
            scope.push( format!("d{}", d), 0.0 as f64);
        }
        scope.push( "d100", 0.0 as f64);

        // Default values
        scope.push("_value1", 0.0_f64);
        scope.push("_value2", 0.0_f64);
        scope.push("_value3", 0.0_f64);

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
                } else
                if node.behavior_type == BehaviorNodeType::VariableNumber {
                    if let Some(value )= node.values.get(&"value".to_string()) {
                        scope.push(node.name.clone(), value.0.clone());
                    } else {
                        scope.push(node.name.clone(), 0.0_f64);
                    }
                }
            }

            let index = self.instances.len();

            let mut instance = BehaviorInstance {id: thread_rng().gen_range(1..=u32::MAX) as usize, name: behavior.name.clone(), behavior_id: id, tree_ids: to_execute.clone(), position, tile, target_instance_index: None, engaged_with: vec![], node_values: HashMap::new(), state_values: HashMap::new(), number_values: HashMap::new(), sleep_cycles: 0, systems_id: 0};

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

            self.instances.push(instance);
            self.scopes.push(scope);

            return index;
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
        self.executed_connections = vec![];
        self.changed_variables = vec![];
        for index in 0..self.active_instance_indices.len() {
            let inst_index = self.active_instance_indices[index];

            if self.instances[inst_index].sleep_cycles > 0 {
                self.instances[inst_index].sleep_cycles -= 1;
                continue;
            }

            let trees = self.instances[inst_index].tree_ids.clone();
            for node_id in &trees {

                // Only execute trees here with an "Always" execute setting (0)
                if let Some(value)= get_node_value((self.instances[inst_index].behavior_id, *node_id, "execute"), self, BehaviorType::Behaviors) {
                    if value.0 != 0.0 {
                        continue;
                    }
                }

                self.execute_node(inst_index, node_id.clone());
            }
        }
    }

    /// Clear the game instances
    pub fn clear_instances(&mut self) {
        self.instances = vec![];
        self.scopes = vec![];
        self.executed_connections = vec![];
        self.changed_variables = vec![];
        self.active_instance_indices = vec![];
    }

    /// Delete the behavior of the given id
    pub fn delete_behavior(&mut self, index: &usize) {
        let id = self.behaviors_ids[*index].clone();

        if let Some(behavior) = self.behaviors.get(&id) {
            let _ = std::fs::remove_file(behavior.path.clone());
        }

        self.behaviors_names.remove(*index);
        self.behaviors_ids.remove(*index);
        self.behaviors.remove(&id);
    }

    /// Delete the system of the given id
    pub fn delete_system(&mut self, index: &usize) {
        let id = self.systems_ids[*index].clone();

        if let Some(system) = self.systems.get(&id) {
            let _ = std::fs::remove_file(system.path.clone());
        }

        self.systems_names.remove(*index);
        self.systems_ids.remove(*index);
        self.systems.remove(&id);
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
                    connector = Some(node_call(instance_index, (behavior_id, node_id), self, BehaviorType::Behaviors));
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

        /// Executes the given systems node and follows the connection chain
    fn execute_systems_node(&mut self, instance_index: usize, node_id: usize) {

        let mut connector : Option<BehaviorNodeConnector> = None;
        let mut connected_node_id : Option<usize> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(system) = self.systems.get_mut(&self.instances[instance_index].systems_id) {
            if let Some(node) = system.data.nodes.get_mut(&node_id) {
                //println!("Executing:: {}", node.name);

                if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                    let behavior_id = self.instances[instance_index].systems_id.clone();
                    connector = Some(node_call(instance_index, (behavior_id, node_id), self, BehaviorType::Systems));
                } else {
                    connector = Some(BehaviorNodeConnector::Bottom);
                }
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        if let Some(connector) = connector {
            if let Some(system) = self.systems.get_mut(&self.instances[instance_index].systems_id) {

                for c in &system.data.connections {
                    if c.0 == node_id && c.1 == connector {
                        connected_node_id = Some(c.2);
                        self.executed_connections.push((c.0, c.1));
                    }
                }
            }
        }

        // And if yes execute it
        if let Some(connected_node_id) = connected_node_id {
            self.execute_systems_node(instance_index, connected_node_id);
        }
    }

    /// Gets the behavior for the given behaviortype
    pub fn get_behavior(&self, id: usize, behavior_type: BehaviorType) -> Option<&GameBehavior> {
        if behavior_type == BehaviorType::Behaviors {
            return self.behaviors.get(&id);
        } else
        if behavior_type == BehaviorType::Systems {
            return self.systems.get(&id);
        } else
        if behavior_type == BehaviorType::Items {
            return self.items.get(&id);
        }
        None
    }

    /// Gets the mutable behavior for the given behaviortype
    pub fn get_mut_behavior(&mut self, id: usize, behavior_type: BehaviorType) -> Option<&mut GameBehavior> {
        if behavior_type == BehaviorType::Behaviors {
            return self.behaviors.get_mut(&id);
        } else
        if behavior_type == BehaviorType::Systems {
            return self.systems.get_mut(&id);
        } else
        if behavior_type == BehaviorType::Items {
            return self.items.get_mut(&id);
        }
        None
    }
}