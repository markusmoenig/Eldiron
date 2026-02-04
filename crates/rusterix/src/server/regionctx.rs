use crate::prelude::*;
use crate::vm::{Program, VMValue};
use crate::{CollisionWorld, MapMini};
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, OnceLock};
use theframework::prelude::*;
use toml::Table;
use uuid::Uuid;

#[derive(Default)]
pub struct RegionCtx {
    pub map: Map,
    pub mapmini: MapMini,
    pub collision_world: CollisionWorld,

    pub paused: bool,

    pub blocking_tiles: FxHashSet<Uuid>,

    pub debug_mode: bool,
    pub debug: DebugModule,
    pub curr_debug_loc: Option<(String, u32, u32)>,

    pub time: TheTime,
    pub region_id: u32,

    pub notifications_entities: Vec<(u32, i64, String)>,
    pub notifications_items: Vec<(u32, i64, String)>,

    pub ticks: i64,
    pub ticks_per_minute: u32,

    pub curr_entity_id: u32,
    pub curr_item_id: Option<u32>,

    pub entity_classes: FxHashMap<u32, String>,
    pub item_classes: FxHashMap<u32, String>,

    pub entity_player_classes: FxHashSet<String>,

    pub entity_class_data: FxHashMap<String, String>,
    pub item_class_data: FxHashMap<String, String>,

    pub entity_proximity_alerts: FxHashMap<u32, f32>,
    pub item_proximity_alerts: FxHashMap<u32, f32>,

    pub entity_state_data: FxHashMap<u32, ValueContainer>,
    pub item_state_data: FxHashMap<u32, ValueContainer>,

    pub to_execute_entity: Vec<(u32, String, VMValue)>,
    pub to_execute_item: Vec<(u32, String, VMValue)>,

    pub entity_programs: FxHashMap<String, Arc<Program>>,
    pub item_programs: FxHashMap<String, Arc<Program>>,

    pub error_count: u32,
    pub startup_errors: Vec<String>,

    pub delta_time: f32,
    pub config: Table,
    pub assets: Assets,

    pub to_receiver: OnceLock<Receiver<RegionMessage>>,
    pub from_sender: OnceLock<Sender<RegionMessage>>,

    pub health_attr: String,

    pub currencies: Currencies,
}

impl RegionCtx {
    /// Search for a mutable reference to an entity with the given ID.
    pub fn get_entity_mut(&mut self, entity_id: u32) -> Option<&mut Entity> {
        self.map
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
    }

    /// Search for a mutable reference to the current entity.
    pub fn get_current_entity_mut(&mut self) -> Option<&mut Entity> {
        self.map
            .entities
            .iter_mut()
            .find(|entity| entity.id == self.curr_entity_id)
    }

    /// Search for a mutable reference to an item with the given ID. Checks the map and the inventory of each entity.
    pub fn get_item_mut(&mut self, item_id: u32) -> Option<&mut Item> {
        if let Some(item) = self.map.items.iter_mut().find(|item| item.id == item_id) {
            return Some(item);
        }

        // Look in each entity’s inventory
        for entity in self.map.entities.iter_mut() {
            for item in entity.inventory.iter_mut() {
                if let Some(item) = item {
                    if item.id == item_id {
                        return Some(item);
                    }
                }
            }
        }
        None
    }

    /// Search for a mutable reference to the current item.
    pub fn get_current_item_mut(&mut self) -> Option<&mut Item> {
        self.curr_item_id.and_then(|id| self.get_item_mut(id))
    }

    /// Create a new item with the given class name.
    pub fn create_item(&mut self, class_name: String) -> Option<Item> {
        if !self.assets.items.contains_key(&class_name) {
            return None;
        }

        let id = crate::server::region::get_global_id();
        let mut item = Item {
            id,
            ..Default::default()
        };

        item.set_attribute("class_name", Value::Str(class_name.clone()));
        item.set_attribute("name", Value::Str(class_name.clone()));

        // Setting the data for the item.
        if let Some(data) = self.item_class_data.get(&class_name) {
            super::data::apply_item_data(&mut item, data);
        }

        if let Some(class_name) = item.get_attr_string("class_name") {
            // let cmd = format!("{}.event(\"startup\", \"\")", class_name);
            self.item_classes.insert(item.id, class_name.clone());
            self.to_execute_item
                .push((item.id, "startup".into(), VMValue::zero()));
        }

        item.mark_all_dirty();

        let value = if item.attributes.get_bool_default("active", false) {
            VMValue::from_bool(true)
        } else {
            VMValue::from_bool(false)
        };

        self.to_execute_item.push((item.id, "active".into(), value));

        Some(item)
    }

    /// Is the given entity dead.
    pub fn is_entity_dead_ctx(&self, id: u32) -> bool {
        let mut v = false;
        for entity in &self.map.entities {
            if entity.id == id {
                v = entity.attributes.get_str_default("mode", "active".into()) == "dead";
            }
        }
        v
    }

    /// Send a log message.
    pub fn send_log_message(&mut self, message: String) {
        self.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::LogMessage(message))
            .unwrap();
    }

    /// Get the name of the entity with the given id.
    pub fn get_entity_name(&self, id: u32) -> String {
        let mut name = "Unknown".to_string();
        for entity in self.map.entities.iter() {
            if entity.id == id {
                if let Some(n) = entity.attributes.get_str("name") {
                    name = n.to_string();
                }
            }
        }
        name
    }

    /// Check if the player moved to a different sector and if yes send "enter" and "left" events
    pub fn check_player_for_section_change(&mut self, entity: &mut Entity) {
        // Determine, set and notify the entity about the sector it is in.
        if let Some(sector) = self.map.find_sector_at(entity.get_pos_xz()) {
            if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
                if sector.name != *old_sector_name {
                    // Send entered event
                    if !sector.name.is_empty() {
                        self.to_execute_entity.push((
                            entity.id,
                            "entered".into(),
                            VMValue::from(sector.name.clone()),
                        ));
                    }
                    // Send left event
                    if !old_sector_name.is_empty() {
                        self.to_execute_entity.push((
                            entity.id,
                            "left".into(),
                            VMValue::from(old_sector_name.clone()),
                        ));
                    }

                    entity
                        .attributes
                        .set("sector", Value::Str(sector.name.clone()));
                }
            }
        } else if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
            // Send left event
            if !old_sector_name.is_empty() {
                if let Some(_class_name) = self.entity_classes.get(&entity.id) {
                    // let cmd = format!("{}.event(\"left\", \"{}\")", class_name, old_sector_name);
                    // println!("{cmd}");
                    self.to_execute_entity.push((
                        entity.id,
                        "bumped_into_item".into(),
                        VMValue::from(old_sector_name.clone()),
                    ));
                }
            }
            entity.attributes.set("sector", Value::Str(String::new()));
        }
    }

    pub fn check_player_for_section_change_id(&mut self, id: u32) {
        if let Some(idx) = self.map.entities.iter().position(|e| e.id == id) {
            // Read-only data first to avoid overlapping mutable borrows
            let pos = self.map.entities[idx].get_pos_xz();
            let old_sector = self
                .map
                .entities
                .get(idx)
                .and_then(|e| e.attributes.get_str("sector"))
                .map(|s| s.to_string())
                .unwrap_or_default();
            let sector_name = self.map.find_sector_at(pos).map(|s| s.name.clone());

            if let Some(entity) = self.map.entities.get_mut(idx) {
                if let Some(sector_name) = sector_name {
                    if sector_name != old_sector {
                        if !sector_name.is_empty() {
                            self.to_execute_entity.push((
                                entity.id,
                                "entered".into(),
                                VMValue::from(sector_name.clone()),
                            ));
                        }
                        if !old_sector.is_empty() {
                            self.to_execute_entity.push((
                                entity.id,
                                "left".into(),
                                VMValue::from(old_sector.clone()),
                            ));
                        }
                        entity
                            .attributes
                            .set("sector", Value::Str(sector_name.clone()));
                    }
                } else {
                    if !old_sector.is_empty() {
                        self.to_execute_entity.push((
                            entity.id,
                            "left".into(),
                            VMValue::from(old_sector.clone()),
                        ));
                    }
                    entity.attributes.set("sector", Value::Str(String::new()));
                }
            }
        }
    }
}
