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
use instant::Instant;
use rayon::prelude::*;
use toml::Table;

use crate::Command;
use crate::EntityAction;
use crate::prelude::*;
use crate::server::message::{AudioCommand, PaletteRemap2DState, RuntimeRenderState};
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
// SenderEntityId, SenderItemId, Message, Category
pub type Say = (Option<u32>, Option<u32>, String, String);

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
    pub says: FxHashMap<u32, Vec<Say>>,
    pub multiple_choice: FxHashMap<u32, Vec<MultipleChoice>>,
    pub audio_commands: FxHashMap<u32, Vec<AudioCommand>>,
    pub world_render: RuntimeRenderState,
    pub region_render: FxHashMap<u32, RuntimeRenderState>,
    pub times: FxHashMap<u32, TheTime>,
    pub runtime_maps: FxHashMap<u32, Map>,
    pub runtime_map_position_guards: FxHashMap<u32, u8>,

    pub state: ServerState,

    pub log: String,
    pub log_changed: bool,
    pub print_log_messages: bool,

    pub instances: Vec<Arc<Mutex<RegionInstance>>>,
    last_visual_update_at: Instant,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            // 0 is reserved as NO_ID / None sentinel.
            id_gen: 1,

            debug_mode: false,
            debug: DebugModule::default(),

            region_id_map: FxHashMap::default(),
            region_name_id_map: FxHashMap::default(),
            from_region: vec![],

            entities: FxHashMap::default(),
            items: FxHashMap::default(),
            messages: FxHashMap::default(),
            says: FxHashMap::default(),
            multiple_choice: FxHashMap::default(),
            audio_commands: FxHashMap::default(),
            world_render: RuntimeRenderState::default(),
            region_render: FxHashMap::default(),
            times: FxHashMap::default(),
            runtime_maps: FxHashMap::default(),
            runtime_map_position_guards: FxHashMap::default(),

            state: ServerState::Off,

            log: String::new(),
            log_changed: true,
            print_log_messages: true,

            instances: vec![],
            last_visual_update_at: Instant::now(),
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
            if let Some(runtime_map) = self.runtime_maps.get(region_id) {
                *map = runtime_map.clone();
            }

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

    /// Get says for a given region and clear them.
    pub fn get_says(&mut self, region_id: &Uuid) -> Vec<Say> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let says = self.says.get(region_id).cloned();
            self.says.remove(region_id);
            says.unwrap_or(vec![])
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

    /// Get queued audio commands for a given region and clear them.
    pub fn get_audio_commands(&mut self, region_id: &Uuid) -> Vec<AudioCommand> {
        if let Some(region_id) = self.region_id_map.get(region_id) {
            let commands = self.audio_commands.get(region_id).cloned();
            self.audio_commands.remove(region_id);
            commands.unwrap_or(vec![])
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

    /// Get the effective runtime render state for the given region.
    pub fn get_render_state(&self, region_id: &Uuid) -> RuntimeRenderState {
        self.region_id_map
            .get(region_id)
            .map(|region_id| {
                self.world_render
                    .clone()
                    .merged(self.region_render.get(region_id).cloned())
            })
            .unwrap_or_else(|| self.world_render.clone())
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
        self.debug.clear_execution();
        let now = Instant::now();
        let visual_dt = now
            .saturating_duration_since(self.last_visual_update_at)
            .as_secs_f32()
            .clamp(0.0, 0.1);
        self.last_visual_update_at = now;

        for receiver in &self.from_region {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    RegionMessage::RegisterPlayer(region_id, entity_id) => {
                        if let Ok(mut players) = LOCAL_PLAYERS.write() {
                            players.push((region_id, entity_id));
                        }
                        if let Ok(pipe) = REGIONPIPE.read()
                            && let Some(sender) = pipe.get(&region_id)
                        {
                            let _ =
                                sender.send(RegionMessage::ShowStartupSectorDescription(entity_id));
                        }
                    }
                    RegionMessage::EntitiesUpdate(id, serialized_updates) => {
                        let updates: Vec<EntityUpdate> = serialized_updates
                            .into_iter()
                            .map(|data| EntityUpdate::unpack(&data))
                            .collect();
                        let runtime_map = self.runtime_maps.get(&id).cloned();
                        let guard_runtime_positions = self
                            .runtime_map_position_guards
                            .get(&id)
                            .copied()
                            .unwrap_or(0)
                            > 0;

                        if let Some(entities) = self.entities.get_mut(&id) {
                            Self::process_entity_updates(
                                entities,
                                updates,
                                assets,
                                runtime_map.as_ref(),
                                guard_runtime_positions,
                            );
                        } else {
                            let mut entities = vec![];
                            Self::process_entity_updates(
                                &mut entities,
                                updates,
                                assets,
                                runtime_map.as_ref(),
                                guard_runtime_positions,
                            );
                            self.entities.insert(id, entities);
                        }

                        if let Some(guard) = self.runtime_map_position_guards.get_mut(&id) {
                            *guard = guard.saturating_sub(1);
                            if *guard == 0 {
                                self.runtime_map_position_guards.remove(&id);
                            }
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
                        if self.print_log_messages {
                            println!("{}", message);
                        }
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
                    RegionMessage::Say(id, sender_entity, sender_item, message, category) => {
                        if let Some(says) = self.says.get_mut(&id) {
                            says.push((sender_entity, sender_item, message, category));
                        } else {
                            self.says
                                .insert(id, vec![(sender_entity, sender_item, message, category)]);
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
                    RegionMessage::AudioCmd(region_id, cmd) => {
                        if let Some(commands) = self.audio_commands.get_mut(&region_id) {
                            commands.push(cmd);
                        } else {
                            self.audio_commands.insert(region_id, vec![cmd]);
                        }
                    }
                    RegionMessage::SetPaletteRemap2D(region_id, start_index, end_index, mode) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        let palette = state
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.start_index = start_index.min(255);
                        palette.end_index = end_index.min(255);
                        palette.mode = mode;
                    }
                    RegionMessage::SetPaletteRemap2DBlend(region_id, blend) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        let palette = state
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.blend = blend.clamp(0.0, 1.0);
                    }
                    RegionMessage::SetWorldPaletteRemap2D(start_index, end_index, mode) => {
                        let palette = self
                            .world_render
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.start_index = start_index.min(255);
                        palette.end_index = end_index.min(255);
                        palette.mode = mode;
                    }
                    RegionMessage::SetWorldPaletteRemap2DBlend(blend) => {
                        let palette = self
                            .world_render
                            .palette_remap
                            .get_or_insert_with(PaletteRemap2DState::default);
                        palette.blend = blend.clamp(0.0, 1.0);
                    }
                    RegionMessage::SetRenderValue(region_id, name, value) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        state.render.set(&name, value);
                    }
                    RegionMessage::SetWorldRenderValue(name, value) => {
                        self.world_render.render.set(&name, value);
                    }
                    RegionMessage::SetPostValue(region_id, name, value) => {
                        let state = self
                            .region_render
                            .entry(region_id)
                            .or_insert_with(RuntimeRenderState::default);
                        state.post.set(&name, value);
                    }
                    RegionMessage::SetWorldPostValue(name, value) => {
                        self.world_render.post.set(&name, value);
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

                        // Messages emitted during the same script event before the deferred
                        // transfer is processed are initially bucketed under the source region.
                        // Move player-addressed messages with the entity so UI widgets in the
                        // destination region still receive them.
                        if from_region_id != dest_id {
                            let mut moved_messages = Vec::new();
                            if let Some(messages) = self.messages.get_mut(&from_region_id) {
                                let mut kept = Vec::with_capacity(messages.len());
                                for message in messages.drain(..) {
                                    if message.2 == entity.id {
                                        moved_messages.push(message);
                                    } else {
                                        kept.push(message);
                                    }
                                }
                                *messages = kept;
                            }
                            if !moved_messages.is_empty() {
                                self.messages
                                    .entry(dest_id)
                                    .or_default()
                                    .extend(moved_messages);
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
                    RegionMessage::MapUpdate(_id, map) => {
                        if let Some(region_id) = self.region_id_map.get(&map.id).copied() {
                            self.entities.insert(region_id, map.entities.clone());
                            self.items.insert(region_id, map.items.clone());
                            self.runtime_maps.insert(region_id, map.clone());
                            self.runtime_map_position_guards.insert(region_id, 4);
                        }
                        assets.maps.insert(map.name.clone(), map.clone());
                        rc = Some(map.name.clone());
                    }
                    RegionMessage::DebugData(data) => {
                        self.debug.merge(&data);
                    }
                    _ => {}
                }
            }
        }

        for entities in self.entities.values_mut() {
            for entity in entities.iter_mut() {
                entity.advance_position_interpolation(visual_dt);
            }
        }

        rc
    }

    fn client_entity_interp_duration(assets: &Assets) -> f32 {
        let Ok(config) = assets.config.parse::<Table>() else {
            return 0.0;
        };
        let game = config.get("game").and_then(|v| v.as_table());
        let simulation_mode = game
            .and_then(|table| table.get("simulation_mode"))
            .and_then(|value| value.as_str())
            .unwrap_or("realtime");
        if simulation_mode.eq_ignore_ascii_case("realtime") {
            return 0.0;
        }
        game.and_then(|table| table.get("game_tick_ms"))
            .and_then(|value| value.as_integer())
            .map(|ms| (ms.max(1) as f32) / 1000.0)
            .unwrap_or(0.25)
    }

    /// Update existing entities (or create new ones if they do not exist).
    pub fn process_entity_updates(
        entities: &mut Vec<Entity>,
        updates: Vec<EntityUpdate>,
        assets: &mut Assets,
        runtime_map: Option<&Map>,
        guard_runtime_positions: bool,
    ) {
        let interp_duration = Self::client_entity_interp_duration(assets);
        // Create a mapping from entity ID to index for efficient lookup
        let mut entity_map: FxHashMap<u32, usize> = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.id, index))
            .collect();

        for update in updates {
            if let Some(&index) = entity_map.get(&update.id) {
                if guard_runtime_positions
                    && entities[index].is_player()
                    && let Some(position) = update.position
                {
                    let pos_2d = Vec2::new(position.x, position.z);
                    let outside_runtime_map = runtime_map
                        .map(|map| map.find_sector_at(pos_2d).is_none())
                        .unwrap_or(false);
                    let moved_far =
                        (position - entities[index].position).magnitude_squared() > 0.25;
                    if outside_runtime_map && moved_far {
                        continue;
                    }
                }

                let previous_position = entities[index].position;
                let new_position = update.position;
                let should_interpolate =
                    interp_duration > 0.0 && new_position.is_some() && !entities[index].is_player();
                // Entity exists, apply the update
                if entities[index].apply_update(update) {
                    assets.entity_tiles.remove(&entities[index].id);
                }
                if should_interpolate && let Some(target_position) = new_position {
                    entities[index].begin_position_interpolation(
                        previous_position,
                        target_position,
                        interp_duration,
                    );
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

    /// Instantly move all registered local players to a sector, optionally in another region.
    pub fn local_player_teleport(&mut self, sector_name: String, region_name: String) {
        if let Ok(local_players) = LOCAL_PLAYERS.read() {
            if let Ok(pipe) = REGIONPIPE.read() {
                for (region_id, entity_id) in local_players.iter() {
                    if let Some(sender) = pipe.get(region_id) {
                        match sender.send(RegionMessage::TeleportEntity(
                            *entity_id,
                            sector_name.clone(),
                            region_name.clone(),
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

    /// Instantly move all registered local players to a position in their current region.
    pub fn local_player_teleport_pos(&mut self, position: Vec2<f32>) {
        if let Ok(local_players) = LOCAL_PLAYERS.read() {
            if let Ok(pipe) = REGIONPIPE.read() {
                for (region_id, entity_id) in local_players.iter() {
                    if let Some(sender) = pipe.get(region_id) {
                        match sender.send(RegionMessage::TeleportEntityPos(*entity_id, position)) {
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
        self.says.clear();
        self.audio_commands.clear();
        self.world_render = RuntimeRenderState::default();
        self.region_render.clear();
        self.id_gen = 1;
        self.region_id_map.clear();
        self.region_name_id_map.clear();
        self.state = ServerState::Off;
        self.from_region.clear();
        self.times.clear();
        self.clear_log();

        // Clear the store
        crate::server::region::clear_regionctx_store();
        crate::server::region::reset_global_id_gen();

        self.instances.clear();
    }

    /// Create a id
    pub fn get_next_id(&mut self) -> u32 {
        let id = self.id_gen;
        self.id_gen += 1;
        id
    }
}
