pub mod region;
pub mod behavior;
pub mod game;
pub mod item;

use core_shared::prelude::*;

use std::fs::metadata;

pub mod prelude {
    pub use crate::gamedata::behavior::*;
    pub use crate::gamedata::game::*;
    pub use crate::gamedata::region::*;
    pub use uuid::Uuid;
}

use crate::gamedata::prelude::*;

#[cfg(not(feature = "embed_binaries"))]
use itertools::Itertools;

use std::path::{self, PathBuf};
use std::fs;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

pub struct GameData {

    pub asset                   : Option<Asset>,

    pub path                    : PathBuf,

    pub regions                 : FxHashMap<Uuid, GameRegion>,
    pub regions_names           : Vec<String>,
    pub regions_ids             : Vec<Uuid>,

    pub behaviors               : FxHashMap<Uuid, GameBehavior>,
    pub behaviors_names         : Vec<String>,
    pub behaviors_ids           : Vec<Uuid>,

    pub systems                 : FxHashMap<Uuid, GameBehavior>,
    pub systems_names           : Vec<String>,
    pub systems_ids             : Vec<Uuid>,

    pub items                   : FxHashMap<Uuid, GameBehavior>,
    pub items_names             : Vec<String>,
    pub items_ids               : Vec<Uuid>,

    pub game                    : Game,
}

impl GameData {

    // Load the game data from the given path
    pub fn load_from_path(path: path::PathBuf) -> Self {

        // Create the tile regions
        let mut regions: FxHashMap<Uuid, GameRegion> = FxHashMap::default();
        let mut regions_names = vec![];
        let mut regions_ids = vec![];

        #[cfg(not(feature = "embed_binaries"))]
        {
            let region_path = path.join("game").join("regions");
            let mut paths: Vec<_> = fs::read_dir(region_path.clone()).unwrap()
                                                    .map(|r| r.unwrap())
                                                    .collect();
            paths.sort_by_key(|dir| dir.path());

            for path in paths {
                let path = &path.path();
                let md = metadata(path).unwrap();

                if md.is_dir() {
                    let mut region = GameRegion::new(path, &region_path);
                    regions_names.push(region.name.clone());

                    region.calc_dimensions();

                    regions_ids.push(region.data.id);
                    regions.insert(region.data.id, region);
                }
            }

            let sorted_keys= regions.keys().sorted();
            for key in sorted_keys {
                let region = &regions[key];

                // If the region has no tiles we assume it's new and we save the data
                if region.data.layer1.len() == 0 {
                    region.save_data();
                }
            }
        }

        #[cfg(feature = "embed_binaries")]
        {
            for file in Embedded::iter() {
                let name = file.as_ref();

                if name.starts_with("game/regions/") && name.ends_with("level1.json") {
                    let mut region = GameRegion::new_from_embedded(name);
                    regions_names.push(region.name.clone());
                    region.calc_dimensions();
                    regions_ids.push(region.data.id);
                    regions.insert(region.data.id, region);
                }
            }
        }

        // Behaviors

        let mut behaviors: FxHashMap<Uuid, GameBehavior> = FxHashMap::default();
        let mut behaviors_names = vec![];
        let mut behaviors_ids = vec![];

        #[cfg(not(feature = "embed_binaries"))]
        {
            let behavior_path = path.join("game").join("characters");
            if let Some(paths) = fs::read_dir(behavior_path.clone()).ok() {

                for path in paths {
                    let path = &path.unwrap().path();
                    let md = metadata(path).unwrap();

                    if md.is_file() {
                        if let Some(name) = path::Path::new(&path).extension() {
                            if name == "json" || name == "JSON" {
                                let mut behavior = GameBehavior::load_from_path(path, &behavior_path);
                                behaviors_names.push(behavior.name.clone());

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
            }
        }

        #[cfg(feature = "embed_binaries")]
        {
            for file in Embedded::iter() {
                let name = file.as_ref();

                if name.starts_with("game/characters/") {
                    let behavior = GameBehavior::load_from_embedded(name);
                    behaviors_names.push(behavior.name.clone());
                    behaviors_ids.push(behavior.data.id);
                    behaviors.insert(behavior.data.id, behavior);
                }
            }
        }

        // Make sure the Player character is always first in the list
        let mut player_index : Option<usize> = None;
        for (index, b) in behaviors_names.iter().enumerate() {
            if b == "Player" {
                player_index = Some(index);
            }
        }

        if let Some(player_index) = player_index {
            let name = behaviors_names.remove(player_index);
            let id = behaviors_ids.remove(player_index);
            behaviors_names.insert(0, name);
            behaviors_ids.insert(0, id);
        }

        // Systems

        let mut systems: FxHashMap<Uuid, GameBehavior> = FxHashMap::default();
        let mut systems_names = vec![];
        let mut systems_ids = vec![];

        #[cfg(not(feature = "embed_binaries"))]
        {
            let systems_path = path.join("game").join("systems");
            if let Some(paths) = fs::read_dir(systems_path.clone()).ok() {

                for path in paths {
                    let path = &path.unwrap().path();
                    let md = metadata(path).unwrap();

                    if md.is_file() {
                        if let Some(name) = path::Path::new(&path).extension() {
                            if name == "json" || name == "JSON" {
                                let system = GameBehavior::load_from_path(path, &systems_path);
                                systems_names.push(system.name.clone());

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
            }
        }

        #[cfg(feature = "embed_binaries")]
        {
            for file in Embedded::iter() {
                let name = file.as_ref();

                if name.starts_with("game/systems/") {
                    let system = GameBehavior::load_from_embedded(name);
                    systems_names.push(system.name.clone());
                    systems_ids.push(system.data.id);
                    systems.insert(system.data.id, system);
                }
            }
        }

        // Items

        let mut items: FxHashMap<Uuid, GameBehavior> = FxHashMap::default();
        let mut items_names = vec![];
        let mut items_ids = vec![];

        let item_path = path.join("game").join("items");
        if let Some(paths) = fs::read_dir(item_path.clone()).ok() {

            for path in paths {
                let path = &path.unwrap().path();
                let md = metadata(path).unwrap();

                if md.is_file() {
                    if let Some(name) = path::Path::new(&path).extension() {
                        if name == "json" || name == "JSON" {
                            let mut item = GameBehavior::load_from_path(path, &item_path);
                            items_names.push(item.name.clone());

                            if item.data.nodes.len() == 0 {
                                item.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
                                item.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
                                item.save_data();
                            }
                            items_ids.push(item.data.id);
                            items.insert(item.data.id, item);
                        }
                    }
                }
            }
        }

        // Game

        #[allow(unused_mut)]
        let mut game;
        #[cfg(not(feature = "embed_binaries"))]
        {
            game = Game::load_from_path(&path.clone());
            if game.behavior.data.nodes.is_empty() {

                game.behavior.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
                game.behavior.add_node(BehaviorNodeType::BehaviorTree, "Game".to_string());

                game.save_data();
            }
        }

        #[cfg(feature = "embed_binaries")]
        {
            game = Game::load_from_embedded("game/game.json");
        }

        Self {

            path                    : path.clone(),
            asset                   : None,

            regions,
            regions_names,
            regions_ids,

            behaviors,
            behaviors_names,
            behaviors_ids,

            systems,
            systems_names,
            systems_ids,

            items,
            items_names,
            items_ids,

            game,
        }
    }

    // Create an empty structure
    pub fn new() -> Self {

        let regions: FxHashMap<Uuid, GameRegion> = FxHashMap::default();
        let regions_names = vec![];
        let regions_ids = vec![];

        // Behaviors

        let behaviors: FxHashMap<Uuid, GameBehavior> = FxHashMap::default();
        let behaviors_names = vec![];
        let behaviors_ids = vec![];

        // Systems

        let systems: FxHashMap<Uuid, GameBehavior> = FxHashMap::default();
        let systems_names = vec![];
        let systems_ids = vec![];

        // Items

        let items: FxHashMap<Uuid, GameBehavior> = FxHashMap::default();
        let items_names = vec![];
        let items_ids = vec![];

        // Game

        let game = Game::new();

        Self {

            path                    : PathBuf::new(),
            asset                   : None,

            regions,
            regions_names,
            regions_ids,

            behaviors,
            behaviors_names,
            behaviors_ids,

            systems,
            systems_names,
            systems_ids,

            items,
            items_names,
            items_ids,

            game,
        }
    }

    #[cfg(feature = "data_editing")]
    /// Saves the region to disk
    pub fn save_region(&self, id: Uuid) {
        if let Some(region) = &mut self.regions.get(&id) {
            region.save_data();
        }
    }

    #[cfg(feature = "data_editing")]
    /// Sets a value in the region
    pub fn set_region_value(&mut self, layer: usize, id: Uuid, pos: (isize, isize), value: TileData) {
        let region = &mut self.regions.get_mut(&id).unwrap();
        region.set_value(layer, pos, value);
    }

    #[cfg(feature = "data_editing")]
    /// Create a new behavior
    pub fn create_behavior(&mut self, name: String, _behavior_type: usize) {

        let path = self.path.join("game").join("behavior").join(name.clone() + ".json");

        let mut behavior = GameBehavior::load_from_path(&path, &self.path.join("game").join("behavior"));
        behavior.data.name = name.clone();

        self.behaviors_names.push(behavior.name.clone());
        self.behaviors_ids.push(behavior.data.id);

        behavior.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
        behavior.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
        behavior.save_data();

        self.behaviors.insert(behavior.data.id, behavior);
    }

    #[cfg(feature = "data_editing")]
    /// Create a new region
    pub fn create_region(&mut self, name: String) -> bool {
        let path = self.path.join("game").join("regions").join(name.clone());

        if fs::create_dir(path.clone()).ok().is_some() {
            let region = GameRegion::new(&path, &self.path.join("game").join("regions"));

            self.regions_names.push(region.name.clone());
            self.regions_ids.push(region.data.id);

            region.save_data();

            self.regions.insert(region.data.id, region);

            return true;
        }

        false
    }

    #[cfg(feature = "data_editing")]
    /// Create a new system
    pub fn create_system(&mut self, name: String, _behavior_type: usize) {

        let path = self.path.join("game").join("systems").join(name.clone() + ".json");

        let mut system = GameBehavior::load_from_path(&path, &self.path.join("game").join("systems"));
        system.data.name = name.clone();

        self.systems_names.push(system.name.clone());
        self.systems_ids.push(system.data.id);

        system.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
        system.save_data();

        self.systems.insert(system.data.id, system);
    }

    #[cfg(feature = "data_editing")]
    /// Create a new system
    pub fn create_item(&mut self, name: String, _behavior_type: usize) {

        let path = self.path.join("game").join("items").join(name.clone() + ".json");

        let mut item = GameBehavior::load_from_path(&path, &self.path.join("game").join("items"));
        item.data.name = name.clone();

        self.items_names.push(item.name.clone());
        self.items_ids.push(item.data.id);

        item.add_node(BehaviorNodeType::BehaviorType, "Behavior Type".to_string());
        item.add_node(BehaviorNodeType::BehaviorTree, "Behavior Tree".to_string());
        item.save_data();

        self.items.insert(item.data.id, item);
    }

    #[cfg(feature = "data_editing")]
    /// Sets the value for the given behavior id
    pub fn set_behavior_id_value(&mut self, id: (Uuid, Uuid, String), value: Value, behavior_type: BehaviorType) {
        if let Some(behavior) = self.get_mut_behavior(id.0, behavior_type) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                node.values.insert(id.2.clone(), value);
                behavior.save_data();
            }
        }
    }

    #[cfg(feature = "data_editing")]
    /// Sets the name for the given node
    pub fn set_behavior_node_name(&mut self, id: (Uuid, Uuid), value: String, behavior_type: BehaviorType) {
        if let Some(behavior) = self.get_mut_behavior(id.0, behavior_type) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                node.name = value;
                behavior.save_data();
            }
        }
    }

    #[cfg(feature = "data_editing")]
    /// Gets the value of the behavior id
    pub fn get_behavior_id_value(&mut self, id: (Uuid, Uuid, String), def: Value, behavior_type: BehaviorType) -> Value {

        if let Some(behavior) = self.get_mut_behavior(id.0, behavior_type) {
            if let Some(node) = behavior.data.nodes.get_mut(&id.1) {
                if let Some(v) = node.values.get(&id.2) {
                    return v.clone();
                } else {
                    node.values.insert(id.2.clone(), def.clone());
                    behavior.save_data();
                }
            }
        }
        def
    }

    /// Gets the position for the given behavior
    pub fn get_behavior_default_position(&self, id: Uuid) -> Option<Position> {
        if let Some(behavior) = self.behaviors.get(&id) {
            for (_index, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"position".to_string()) {
                        return value.to_position();
                    }
                }
            }
        }
        None
    }

    /// Gets the default tile for the given behavior
    pub fn get_behavior_default_tile(&self, id: Uuid) -> Option<TileData> {
        if let Some(behavior) = self.behaviors.get(&id) {
            for (_index, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"tile".to_string()) {
                        return value.to_tile_data();
                    }
                }
            }
        }
        None
    }

    /// Gets the alignment variable
    pub fn get_behavior_default_alignment(&self, id: Uuid) -> i32 {
        if let Some(behavior) = self.behaviors.get(&id) {
            for (_index, node) in &behavior.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"alignment".to_string()) {
                        match value {
                            Value::Integer(v) => return 2 - *v as i32 - 1,
                            _ => {},
                        }
                    }
                }
            }
        }
        0
    }

    /// Gets the default tile for the given item
    pub fn get_item_default_tile(&self, id: Uuid) -> Option<TileData> {
        if let Some(item) = self.items.get(&id) {
            for (_index, node) in &item.data.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"tile".to_string()) {
                        return value.to_tile_data();
                    }
                }
            }
        }
        None
    }

    /// Save data and return it
    // pub fn save(&self) -> String {
    //     let json = serde_json::to_string(&self.instances).unwrap();
    //     json
    // }

    /// Returns the layered tiles at the given position
    pub fn get_tile_at(&self, pos: (Uuid, isize, isize)) -> Vec<TileData> {
        if let Some(region) = self.regions.get(&pos.0) {
            return region.get_value((pos.1, pos.2));
        }
        vec![]
    }

    /// Delete the region of the given id
    pub fn delete_region(&mut self, index: &usize) {
        let id = self.regions_ids[*index].clone();

        if let Some(region) = self.regions.get(&id) {
            let _ = std::fs::remove_dir_all(region.path.clone());
        }

        self.regions_names.remove(*index);
        self.regions_ids.remove(*index);
        self.regions.remove(&id);
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

    /// Delete the item of the given id
    pub fn delete_item(&mut self, index: &usize) {
        let id = self.items_ids[*index].clone();

        if let Some(item) = self.items.get(&id) {
            let _ = std::fs::remove_file(item.path.clone());
        }

        self.items_names.remove(*index);
        self.items_ids.remove(*index);
        self.items.remove(&id);
    }

    /// Gets the behavior for the given behavior type
    pub fn get_behavior(&self, id: Uuid, behavior_type: BehaviorType) -> Option<&GameBehavior> {
        if behavior_type == BehaviorType::Regions {
            for (_index, region) in &self.regions {
                for index in 0..region.behaviors.len() {
                    if region.behaviors[index].data.id == id {
                        return Some(&region.behaviors[index]);
                    }
                }
            }
        } else
        if behavior_type == BehaviorType::Behaviors {
            return self.behaviors.get(&id);
        } else
        if behavior_type == BehaviorType::Systems {
            return self.systems.get(&id);
        } else
        if behavior_type == BehaviorType::Items {
            return self.items.get(&id);
        } else
        if behavior_type == BehaviorType::GameLogic {
            return Some(&self.game.behavior);
        }
        None
    }

    /// Gets the mutable behavior for the given behavior type
    pub fn get_mut_behavior(&mut self, id: Uuid, behavior_type: BehaviorType) -> Option<&mut GameBehavior> {
        if behavior_type == BehaviorType::Regions {
            for (_index, region) in &mut self.regions {
                for index in 0..region.behaviors.len() {
                    if region.behaviors[index].data.id == id {
                        return Some(&mut region.behaviors[index]);
                    }
                }
            }
        } else
        if behavior_type == BehaviorType::Behaviors {
            return self.behaviors.get_mut(&id);
        } else
        if behavior_type == BehaviorType::Systems {
            return self.systems.get_mut(&id);
        } else
        if behavior_type == BehaviorType::Items {
            return self.items.get_mut(&id);
        } else
        if behavior_type == BehaviorType::GameLogic {
            return Some(&mut self.game.behavior);
        }
        None
    }

    /// Returns a mutable reference to the game settings
    pub fn get_game_settings(&mut self) -> &mut PropertySink {
        if self.game.behavior.data.settings.is_none() {
            let mut settings = PropertySink::new();
            update_game_sink(&mut settings);
            self.game.behavior.data.settings = Some(settings);
        }

        self.game.behavior.data.settings.as_mut().unwrap()
    }

    /// Checks all behaviors if they contain all character attributes defined in the game settings
    pub fn check_all_behaviors_for_attributes(&mut self) {
        let ids = self.behaviors_ids.clone();
        for id in ids {
            self.check_behavior_for_attributes(id);
        }
    }

    /// Check the given behavior contains all character attributes defined in the game settings
    pub fn check_behavior_for_attributes(&mut self, _behavior_id: Uuid) {
        // Check to see if we added all variables from the game settings.
        let settings = self.get_game_settings();
        if let Some(attr) = settings.get("character_attributes") {
            if let Some(attr_string) = attr.as_string() {
                let mut attributes : Vec<&str> = attr_string.split(',').collect();

                for a in attributes.iter_mut() {
                    *a = a.trim();
                }

                /*
                if let Some(behavior) = self.behaviors.get_mut(&behavior_id) {
                    for (_id, node) in &behavior.data.nodes {
                        if node.behavior_type == BehaviorNodeType::VariableNumber {

                            if let Some(index) = attributes.iter().position(|&a| a == node.name) {
                                attributes.remove(index);
                            }
                        }
                    }

                    for a in attributes {
                        let id = behavior.add_node(BehaviorNodeType::VariableNumber, a.clone().to_string());
                        if let Some(node) = behavior.data.nodes.get_mut(&id) {
                            node.values.insert("value".to_string(), Value::Float(10.0));
                        }
                    }
                }*/
            }
        }
    }

}