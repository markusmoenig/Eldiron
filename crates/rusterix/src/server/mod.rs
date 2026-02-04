pub mod assets;
pub mod currency;
pub mod data;
pub mod entity;
pub mod item;
pub mod message;
pub mod py_fn;
pub mod region;
pub mod region_host;
pub mod regionctx;

use crossbeam_channel::{Receiver, Sender};
use rayon::prelude::*;

use crate::Command;
use crate::EntityAction;
use crate::prelude::*;
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use theframework::prelude::*;

// Pipes to the regions
type RegionRegistry = Arc<RwLock<FxHashMap<u32, Sender<RegionMessage>>>>;
static REGIONPIPE: LazyLock<RegionRegistry> =
    LazyLock::new(|| Arc::new(RwLock::new(FxHashMap::default())));

// List of currently active local players
type Player = Arc<RwLock<Vec<(u32, u32)>>>;
static LOCAL_PLAYERS: LazyLock<Player> = LazyLock::new(|| Arc::new(RwLock::new(Vec::new())));

// SenderEntityId, SenderItemId, ReceiverId, Message
pub type Message = (Option<u32>, Option<u32>, u32, String, String);

#[derive(Clone, Copy, PartialEq)]
pub enum ServerState {
    Off,
    Running,
    Paused,
}

pub struct Server {
    pub id_gen: u32,

    /// In debug mode the serve sends grid based status updates for all entities / items
    pub debug_mode: bool,
    pub debug: DebugModule,

    /// Maps region uuids to the region id
    pub region_id_map: FxHashMap<Uuid, u32>,
    from_region: Vec<Receiver<RegionMessage>>,

    /// Maps region names to the region id
    pub region_name_id_map: FxHashMap<String, u32>,

    // By region
    pub entities: FxHashMap<u32, Vec<Entity>>,
    pub items: FxHashMap<u32, Vec<Item>>,
    pub messages: FxHashMap<u32, Vec<Message>>,
    pub multiple_choice: FxHashMap<u32, Vec<MultipleChoice>>,
    pub times: FxHashMap<u32, TheTime>,

    pub state: ServerState,

    pub log: String,
    pub log_changed: bool,

    pub instances: Vec<Arc<Mutex<RegionInstance>>>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            id_gen: 0,

            debug_mode: false,
            debug: DebugModule::default(),

            region_id_map: FxHashMap::default(),
            region_name_id_map: FxHashMap::default(),
            from_region: vec![],

            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            messages: FxHashMap::default(),
            multiple_choice: FxHashMap::default(),
            times: FxHashMap::default(),

            state: ServerState::Off,

            log: String::new(),
            log_changed: true,

            instances: vec![],
        }
    }

    /// Clear the log
    pub fn clear_log(&mut self) {
        self.log = String::new();
    }

    /// Retrieve the log
    pub fn get_log(&mut self) -> String {
        self.log_changed = false;
        self.log.clone()
    }

    /// Set the server state.
    pub fn set_state(&mut self, state: ServerState) {
        self.state = state;
    }

    /// Create the given region instance.
    pub fn create_region_instance(
        &mut self,
        name: String,
        map: Map,
        assets: &Assets,
        config_toml: String,
    ) {
        let mut region_instance = RegionInstance::new(self.instances.len() as u32);
        // region_instance.id = self.get_next_id();

        self.region_id_map.insert(map.id, region_instance.id);
        self.region_name_id_map
            .insert(name.clone(), region_instance.id);

        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.insert(region_instance.id, region_instance.to_sender.clone());
        }

        self.from_region.push(region_instance.from_receiver.clone());

        region_instance.init(name, map, assets, config_toml, self.debug_mode);
        self.instances.push(Arc::new(Mutex::new(region_instance)));
    }

    /// Send a system tick to all instances.
    pub fn system_tick(&self) {
        self.instances.par_iter().for_each(|instance| {
            instance.lock().unwrap().system_tick();
        });
    }

    /// Send a redraw tick to all instances.
    pub fn redraw_tick(&self) {
        self.instances.par_iter().for_each(|instance| {
            instance.lock().unwrap().redraw_tick();
        });
    }

    /// Process a set of commands from a client.
    pub fn process_client_commands(&mut self, commands: Vec<Command>) {
        for cmd in commands {
            match cmd {
                Command::CreateEntity(id, entity) => {
                    if let Some(region_id) = self.region_id_map.get(&id) {
                        if let Ok(pipe) = REGIONPIPE.read() {
                            if let Some(sender) = pipe.get(region_id) {
                                match sender.send(RegionMessage::CreateEntity(*region_id, entity)) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("{:?}", err.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get entities and items for a given region.
    pub fn get_entities_items(
        &self,
        region_id: &Uuid,
    ) -> (Option<&Vec<Entity>>, Option<&Vec<Item>>) {
        let mut rc: (Option<&Vec<Entity>>, Option<&Vec<Item>>) = (None, None);

        rc.0 = if let Some(region_id) = self.region_id_map.get(region_id) {
            self.entities.get(region_id)
        } else {
            None
        };

        rc.1 = if let Some(region_id) = self.region_id_map.get(region_id) {
            self.items.get(region_id)
        } else {
            None
        };

        rc
    }

    /// Apply entities and items for a given region.
    pub fn apply_entities_items(&self, map: &mut Map) {
        if let Some(region_id) = self.region_id_map.get(&map.id) {
            if let Some(entities) = self.entities.get(region_id) {
                map.entities = entities.clone();
            }

            if let Some(items) = self.items.get(region_id) {
                map.items = items.clone();
            }
        };
    }

    /// Get messages for a given region and clear them.
    pub fn get_messages(&mut self, region_id: &Uuid) -> Vec<Message> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let messages = self.messages.get(region_id).cloned();
            self.messages.remove(region_id);
            messages.unwrap_or(vec![])
        } else {
            vec![]
        }
    }

    /// Get multi-choice for a given region and clear them.
    pub fn get_choices(&mut self, region_id: &Uuid) -> Vec<MultipleChoice> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let choices = self.multiple_choice.get(region_id).cloned();
            self.multiple_choice.remove(region_id);
            choices.unwrap_or(vec![])
        } else {
            vec![]
        }
    }

    /// Get the current time for the given region.
    pub fn get_time(&self, region_id: &Uuid) -> Option<TheTime> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            if let Some(time) = self.times.get(region_id) {
                return Some(*time);
            }
        }
        None
    }

    /// Set the current time for the given region.
    pub fn set_time(&mut self, region_id: &Uuid, time: TheTime) -> TheTime {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            if let Ok(pipe) = REGIONPIPE.read() {
                if let Some(sender) = pipe.get(region_id) {
                    self.times.clear();
                    match sender.send(RegionMessage::Time(*region_id, time)) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("{:?}", err.to_string());
                        }
                    }
                }
            }
        }
        TheTime::default()
    }

    /// Retrieves all messages from the regions. Returns the name of the new region should the
    /// players region change.
    pub fn update(&mut self, assets: &mut Assets) -> Option<String> {
        let mut rc: Option<String> = None;

        for receiver in &self.from_region {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    RegionMessage::RegisterPlayer(region_id, entity_id) => {
                        if let Ok(mut players) = LOCAL_PLAYERS.write() {
                            println!("Registering player: {} {}", region_id, entity_id);
                            players.push((region_id, entity_id));
                        }
                    }
                    RegionMessage::EntitiesUpdate(id, serialized_updates) => {
                        let updates: Vec<EntityUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| EntityUpdate::unpack(&data))
                            .collect();

                        if let Some(entities) = self.entities.get_mut(&id) {
                            Self::process_entity_updates(entities, updates, assets);
                        } else {
                            let mut entities = vec![];
                            Self::process_entity_updates(&mut entities, updates, assets);
                            self.entities.insert(id, entities);
                        }
                    }
                    RegionMessage::ItemsUpdate(id, serialized_updates) => {
                        let updates: Vec<ItemUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| ItemUpdate::unpack(&data))
                            .collect();

                        if let Some(items) = self.items.get_mut(&id) {
                            Self::process_item_updates(items, updates);
                        } else {
                            let mut items = vec![];
                            Self::process_item_updates(&mut items, updates);
                            self.items.insert(id, items);
                        }
                    }
                    RegionMessage::RemoveItem(region_id, item_id) => {
                        if let Some(items) = self.items.get_mut(&region_id) {
                            items.retain(|item| item.id != item_id);
                        }
                    }
                    RegionMessage::LogMessage(message) => {
                        println!("{}", message);
                        if self.log.is_empty() {
                            self.log = message;
                        } else {
                            self.log += &format!("{}{}", "\n", message);
                        }
                        self.log_changed = true;
                    }
                    RegionMessage::Message(
                        id,
                        sender_entity,
                        sender_item,
                        receiver_id,
                        message,
                        category,
                    ) => {
                        // println!(
                        //     "({:?}, {:?}) -> {}: {}",
                        //     sender_entity, sender_item, receiver_id, message
                        // );
                        //

                        if let Some(messages) = self.messages.get_mut(&id) {
                            messages.push((
                                sender_entity,
                                sender_item,
                                receiver_id,
                                message,
                                category,
                            ));
                        } else {
                            let messages =
                                vec![(sender_entity, sender_item, receiver_id, message, category)];
                            self.messages.insert(id, messages);
                        }
                    }
                    RegionMessage::MultipleChoice(choices) => {
                        if let Some(multi_choice) = self.multiple_choice.get_mut(&choices.region) {
                            multi_choice.push(choices.clone());
                        } else {
                            let multi_choice = vec![choices.clone()];
                            self.multiple_choice.insert(choices.region, multi_choice);
                        }
                    }
                    RegionMessage::Time(id, time) => {
                        self.times.insert(id, time);
                    }
                    RegionMessage::TransferEntity(
                        from_region_id,
                        entity,
                        dest_region_name,
                        dest_sector_name,
                    ) => {
                        // If we cannot find the destination region, send the entity back from where it came
                        let mut dest_id = from_region_id;
                        if let Some(region_id) = self.region_name_id_map.get(&dest_region_name) {
                            dest_id = *region_id;
                        }

                        let mut removed_local: Option<Entity> = None;
                        // Remove entity from the old region
                        if let Some(entities) = self.entities.get_mut(&from_region_id) {
                            if let Some(pos) = entities.iter().position(|e| e.id == entity.id) {
                                removed_local = Some(entities.remove(pos));
                            }
                        }

                        // Add entity to the dest region
                        if let Some(removed_local) = removed_local {
                            if let Some(entities) = self.entities.get_mut(&dest_id) {
                                entities.push(removed_local);
                            } else {
                                self.entities.insert(dest_id, vec![removed_local]);
                            }
                            rc = Some(dest_region_name.clone());
                        }

                        // Change the local player reference to the new region
                        if let Ok(mut players) = LOCAL_PLAYERS.write() {
                            for item in &mut *players {
                                if item.1 == entity.id {
                                    item.0 = dest_id;
                                }
                            }
                        }

                        if let Ok(pipe) = REGIONPIPE.read() {
                            if let Some(sender) = pipe.get(&dest_id) {
                                match sender.send(RegionMessage::TransferEntity(
                                    dest_id,
                                    entity.clone(),
                                    dest_region_name.clone(),
                                    dest_sector_name.clone(),
                                )) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("{:?}", err.to_string());
                                    }
                                }
                            }
                        }
                    }
                    RegionMessage::DebugData(data) => {
                        self.debug.merge(&data);
                    }
                    _ => {}
                }
            }
        }

        rc
    }

    /// Update existing entities (or create new ones if they do not exist).
    pub fn process_entity_updates(
        entities: &mut Vec<Entity>,
        updates: Vec<EntityUpdate>,
        assets: &mut Assets,
    ) {
        // Create a mapping from entity ID to index for efficient lookup
        let mut entity_map: FxHashMap<u32, usize> = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = entity_map.get(&update.id) {
                // Entity exists, apply the update
                if entities[index].apply_update(update) {
                    assets.entity_tiles.remove(&entities[index].id);
                }
            } else {
                // Entity does not exist, create a new one
                let mut new_entity = Entity {
                    id: update.id,
                    ..Default::default()
                };
                new_entity.apply_update(update);

                // Add to the entity list
                let new_entity_id = new_entity.id;
                entities.push(new_entity);

                // Update the map with the new entitys ID
                entity_map.insert(new_entity_id, entities.len() - 1);
            }
        }
    }

    /// Update existing items (or create new ones if they do not exist).
    pub fn process_item_updates(items: &mut Vec<Item>, updates: Vec<ItemUpdate>) {
        // Create a mapping from entity ID to index for efficient lookup
        let mut item_map: FxHashMap<u32, usize> = items
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = item_map.get(&update.id) {
                // Entity exists, apply the update
                items[index].apply_update(update);
            } else {
                // Entity does not exist, create a new one
                let mut new_item = Item {
                    id: update.id,
                    ..Default::default()
                };
                new_item.apply_update(update);

                // Add to the item list
                let new_entity_id = new_item.id;
                items.push(new_item);

                // Update the map with the new items ID
                item_map.insert(new_entity_id, items.len() - 1);
            }
        }
    }

    /// Send a local player event to the registered players
    pub fn local_player_event(&mut self, event: String, value: Value) {
        if let Ok(local_players) = LOCAL_PLAYERS.read() {
            if let Ok(pipe) = REGIONPIPE.read() {
                for (region_id, entity_id) in local_players.iter() {
                    if let Some(sender) = pipe.get(region_id) {
                        match sender.send(RegionMessage::UserEvent(
                            *entity_id,
                            event.clone(),
                            value.clone(),
                        )) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("{:?}", err.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Send a local player action to the registered players
    pub fn local_player_action(&mut self, action: EntityAction) {
        if let Ok(local_players) = LOCAL_PLAYERS.read() {
            if let Ok(pipe) = REGIONPIPE.read() {
                for (region_id, entity_id) in local_players.iter() {
                    if let Some(sender) = pipe.get(region_id) {
                        match sender.send(RegionMessage::UserAction(*entity_id, action.clone())) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("{:?}", err.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Pause all region instances.
    pub fn pause(&mut self) {
        if let Ok(pipes) = REGIONPIPE.read() {
            for sender in pipes.values() {
                _ = sender.send(RegionMessage::Pause);
            }
        }
        self.state = ServerState::Paused;
    }

    /// Continue all region instances.
    pub fn continue_instances(&mut self) {
        if let Ok(pipes) = REGIONPIPE.read() {
            for sender in pipes.values() {
                _ = sender.send(RegionMessage::Continue);
            }
        }
        self.state = ServerState::Running;
    }

    /// Shuts down all region instances.
    pub fn stop(&mut self) {
        if let Ok(pipes) = REGIONPIPE.read() {
            for sender in pipes.values() {
                _ = sender.send(RegionMessage::Quit);
            }
        }
        self.clear();
    }

    /// Shuts down all region instances.
    pub fn clear(&mut self) {
        if let Ok(mut pipes) = REGIONPIPE.write() {
            pipes.clear();
        }
        if let Ok(mut players) = LOCAL_PLAYERS.write() {
            players.clear();
        }
        self.entities.clear();
        self.items.clear();
        self.messages.clear();
        self.id_gen = 0;
        self.region_id_map.clear();
        self.region_name_id_map.clear();
        self.state = ServerState::Off;
        self.from_region.clear();
        self.times.clear();
        self.clear_log();

        // Clear the store
        crate::server::region::clear_regionctx_store();

        self.instances.clear();
    }

    /// Create a id
    pub fn get_next_id(&mut self) -> u32 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }
}
