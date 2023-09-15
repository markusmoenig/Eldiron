extern crate ref_thread_local;
use ref_thread_local::RefThreadLocal;

use crate::prelude::*;

pub struct RegionInstance {
    // Game data
    pub region_data: GameRegionData,
    pub region_behavior: Vec<GameBehaviorData>,

    pub behaviors: FxHashMap<Uuid, GameBehaviorData>,
    pub systems: FxHashMap<Uuid, GameBehaviorData>,
    pub items: FxHashMap<Uuid, GameBehaviorData>,
    pub spells: FxHashMap<Uuid, GameBehaviorData>,
    pub game_data: GameBehaviorData,
    pub scripts: FxHashMap<String, String>,

    // For faster lookup
    pub system_names: Vec<String>,
    pub system_ids: Vec<Uuid>,
    pub area_ids: Vec<Uuid>,

    // Identifie the currently executing loot item
    pub curr_loot_item: Option<(isize, isize, usize)>,

    // The currently executing behavior tree id
    pub curr_executing_tree: Uuid,

    // Region settings
    pub pixel_based_movement: bool,

    /// Tick count used for timing
    pub dealt_damage_success: bool,

    /// Respawns the given chararacter uuid at the given tick count
    pub respawn_instance: FxHashMap<Uuid, (usize, CharacterInstanceData)>,

    // Game settings
    screen_size: (i32, i32),
    def_square_tile_size: i32,

    pub gear_slots: Vec<String>,
    pub weapon_slots: Vec<String>,

    pub ticks_per_minute: usize,

    // Variable names
    pub primary_currency: String,
    pub hitpoints: String,
    pub max_hitpoints: String,
}

impl RegionInstance {
    pub fn new() -> Self {
        Self {
            region_data: GameRegionData::new(),
            region_behavior: vec![],

            behaviors: FxHashMap::default(),
            systems: FxHashMap::default(),
            items: FxHashMap::default(),
            spells: FxHashMap::default(),
            game_data: GameBehaviorData::new(),
            scripts: FxHashMap::default(),

            system_names: vec![],
            system_ids: vec![],
            area_ids: vec![],

            curr_loot_item: None,

            curr_executing_tree: Uuid::new_v4(),

            pixel_based_movement: true,

            dealt_damage_success: false,

            respawn_instance: FxHashMap::default(),

            screen_size: (1024, 608),
            def_square_tile_size: 32,

            weapon_slots: vec![],
            gear_slots: vec![],

            ticks_per_minute: 4,

            // Variable names
            primary_currency: "".to_string(),
            hitpoints: "".to_string(),
            max_hitpoints: "".to_string(),
        }
    }

    /// Game tick
    pub fn tick(&mut self) -> Vec<Message> {
        let mut messages = vec![];

        let character_instances_len;
        let mut to_respawn: Vec<(Uuid, CharacterInstanceData)> = vec![];

        {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.characters.clear();
            data.area_characters.clear();
            data.prev_area_characters.clear();
            data.lights.clear();

            character_instances_len = data.character_instances.len();

            // Check if we need to respawn something
            if data.respawn_instance.is_empty() == false {
                for (id, (tick, char_data)) in &data.respawn_instance.clone() {
                    if *tick <= *TICK_COUNT.borrow() as usize {
                        data.respawn_instance.remove(id);
                        to_respawn.push((*id, char_data.clone()));
                    }
                }
            }
        }

        // Respawn what we must
        for (id, data) in to_respawn {
            self.create_behavior_instance(id, false, Some(data));
        }

        // Execute behaviors
        for inst_index in 0..character_instances_len {
            let state;
            let instance_type;

            let sleeping;

            {
                let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                data.curr_index = inst_index;

                data.executed_connections = vec![];
                data.script_errors = vec![];

                data.character_instances[inst_index].audio = vec![];
                data.character_instances[inst_index].multi_choice_data = vec![];

                state = data.character_instances[inst_index].state;
                instance_type = data.character_instances[inst_index].instance_type;

                if self.pixel_based_movement == true {
                    if data.character_instances[data.curr_index]
                        .old_position
                        .is_some()
                    {
                        data.character_instances[data.curr_index].curr_transition_time += 1;

                        if data.character_instances[data.curr_index].curr_transition_time
                            > data.character_instances[data.curr_index].max_transition_time
                        {
                            data.character_instances[data.curr_index].old_position = None;
                            data.character_instances[data.curr_index].curr_transition_time = 0;
                        }
                    }
                }

                if data.character_instances[inst_index].sleep_cycles > 0 {
                    data.character_instances[inst_index].sleep_cycles -= 1;
                    sleeping = true;
                } else {
                    sleeping = false;
                }

                // Are we debugging this character ?
                data.is_debugging = Some(data.character_instances[inst_index].behavior_id)
                    == data.debug_behavior_id;
            }

            // Skip Sleep cycles
            if sleeping == false {
                // Purged: Skip
                if state == BehaviorInstanceState::Purged {
                    continue;
                }

                // Killed: NPC Skip
                if state == BehaviorInstanceState::Killed
                    && instance_type == BehaviorInstanceType::NonPlayerCharacter
                {
                    continue;
                }

                // NPC Tick
                if instance_type == BehaviorInstanceType::NonPlayerCharacter {
                    let mut execute_trees = true;

                    {
                        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

                        // Check if this NPC has active communication
                        if data.character_instances[inst_index]
                            .communication
                            .is_empty()
                            == false
                        {
                            let mut com_to_drop: Option<usize> = None;

                            for c_index in
                                0..data.character_instances[inst_index].communication.len()
                            {
                                if data.character_instances[inst_index].communication[c_index]
                                    .end_time
                                    < *DATE.borrow()
                                {
                                    // Drop this communication for the NPC
                                    com_to_drop = Some(c_index);

                                    // Remove the communication for the Player
                                    let player_index = data.character_instances[inst_index]
                                        .communication[c_index]
                                        .player_index;
                                    data.character_instances[player_index].communication = vec![];
                                    data.character_instances[player_index].multi_choice_data =
                                        vec![];

                                    break;
                                }
                            }

                            if let Some(index) = com_to_drop {
                                data.character_instances[inst_index]
                                    .communication
                                    .remove(index);
                            }

                            // Communication is ongoing, dont do anything
                            if data.character_instances[inst_index]
                                .communication
                                .is_empty()
                                == false
                            {
                                execute_trees = false;
                            }
                        }
                    }

                    if execute_trees {
                        // Execute trees of an NPC

                        let locked_tree;
                        let behavior_id;

                        {
                            let data = &REGION_DATA.borrow()[*CURR_INST.borrow()];
                            locked_tree = data.character_instances[data.curr_index].locked_tree;
                            behavior_id = data.character_instances[inst_index].behavior_id;
                        }

                        // Has a locked tree ?
                        if let Some(locked_tree) = locked_tree {
                            execute_node(behavior_id, locked_tree, &mut BEHAVIORS.borrow_mut());
                        } else {
                            // Unlocked, execute all valid trees
                            let trees;
                            {
                                let data = &REGION_DATA.borrow()[*CURR_INST.borrow()];
                                trees = data.character_instances[inst_index].tree_ids.clone();
                            }
                            for node_id in &trees {
                                // Only execute trees here with an "Always" execute setting (0)
                                if let Some(value) = get_node_value(
                                    (behavior_id, *node_id, "execute"),
                                    self,
                                    BehaviorType::Behaviors,
                                ) {
                                    if let Some(value) = value.to_integer() {
                                        if value != 0 {
                                            continue;
                                        }
                                    }
                                }
                                execute_node(behavior_id, *node_id, &mut BEHAVIORS.borrow_mut());
                            }

                            // Execute all previously computed Class and Race trees
                            let system_tree_tick_names;
                            {
                                let data = &REGION_DATA.borrow()[*CURR_INST.borrow()];
                                system_tree_tick_names = data.character_instances[inst_index]
                                    .system_tree_tick_names
                                    .clone();
                            }

                            for (system_name, tree_name) in system_tree_tick_names {
                                execute_system(system_name.as_str(), tree_name.as_str());
                            }
                        }
                    }
                } else
                // PC Tick
                if instance_type == BehaviorInstanceType::Player {
                    // Execute the tree which matches the current action
                    let action;
                    {
                        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                        action = data.character_instances[inst_index].action.clone();
                        // DEBUG INCOMING ACTION
                        //println!("{:?}", action);
                    }
                    if let Some(action) = &action {
                        if action.direction != PlayerDirection::None {
                            // A directed action ( Move / Look - North etc)

                            if action.action.to_lowercase() == "cast" && action.spell.is_some() {
                                // Cast spell
                                let mut to_cast: Option<(Uuid, Uuid)> = None;
                                if let Some(spell_name) = &action.spell {
                                    let name = spell_name.to_lowercase();
                                    for (behavior_id, spell) in &self.spells {
                                        if spell.name.to_lowercase() == name {
                                            for (node_id, node) in &spell.nodes {
                                                if node.behavior_type
                                                    == BehaviorNodeType::BehaviorTree
                                                    && node.name.to_lowercase() == "cast"
                                                {
                                                    to_cast = Some((*behavior_id, *node_id));
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(to_cast) = to_cast {
                                    execute_node(to_cast.0, to_cast.1, &mut SPELLS.borrow_mut());
                                }
                            } else {
                                if execute_behavior(inst_index, &action.action) == false {
                                    println!(
                                        "Cannot find valid tree for directed action {}",
                                        action.action
                                    );
                                }
                            }
                        } else if let Some(inventory_index) = &action.inventory_index {
                            // An action on an inventory item index

                            let index = *inventory_index as usize;

                            // Get the item and set the state if any
                            if let Some(item) = get_inventory_item_at(index, true) {
                                let mut to_execute = vec![];

                                // Get the behavior trees to execute
                                if let Some(item_behavior) =
                                    self.get_behavior(item.id, BehaviorType::Items)
                                {
                                    for (id, node) in &item_behavior.nodes {
                                        if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                            if node.name == action.action {
                                                to_execute.push((item_behavior.id, *id));
                                            }
                                        }
                                    }
                                }

                                // Execute them
                                if to_execute.is_empty() == false {
                                    for (behavior_id, node_id) in to_execute {
                                        execute_node(behavior_id, node_id, &mut ITEMS.borrow_mut());
                                    }
                                    set_inventory_item_state_at(index);
                                } else {
                                    // If we cannot find the tree on the item, look for it on the player
                                    {
                                        let data: &mut RegionData =
                                            &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                                        for id in &data.character_instances[inst_index].tree_ids {
                                            if let Some(behavior) = self.get_behavior(
                                                data.character_instances[inst_index].behavior_id,
                                                BehaviorType::Behaviors,
                                            ) {
                                                if let Some(node) = behavior.nodes.get(&id) {
                                                    if node.name == action.action {
                                                        to_execute.push((
                                                            data.character_instances[inst_index]
                                                                .behavior_id,
                                                            *id,
                                                        ));
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if to_execute.is_empty() == false {
                                        for (behavior_id, node_id) in to_execute {
                                            execute_node(
                                                behavior_id,
                                                node_id,
                                                &mut BEHAVIORS.borrow_mut(),
                                            );
                                        }
                                    } else {
                                        println!(
                                            "Cannot find valid tree for directed action {}",
                                            action.action
                                        );
                                    }
                                }
                            }
                        } else if let Some(uuid) = &action.multi_choice_uuid {
                            // Multi Choice Answer

                            let mut communication_id: Option<(Uuid, Uuid)> = None;
                            {
                                let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                                if data.character_instances[inst_index]
                                    .communication
                                    .is_empty()
                                    == false
                                {
                                    //let npc_index = data.character_instances[inst_index].communication[0].npc_index;
                                    communication_id = Some(
                                        data.character_instances[inst_index].communication[0]
                                            .npc_behavior_id,
                                    );
                                    data.character_instances[inst_index].multi_choice_answer =
                                        Some(*uuid);
                                }
                            }

                            if let Some(behavior_id) = communication_id {
                                execute_node(
                                    behavior_id.0,
                                    behavior_id.1,
                                    &mut BEHAVIORS.borrow_mut(),
                                );
                            }
                        }

                        // Clear the action for the instance
                        {
                            let data: &mut RegionData =
                                &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                            data.character_instances[inst_index].action = None;
                        }
                    }
                }

                // Execute the trees queued for execution by script "execute" cmds
                let old_index;
                let to_execute;
                {
                    let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                    to_execute = data.to_execute.clone();
                    old_index = data.curr_index;
                    data.to_execute = vec![];
                }
                for (index, tree_name) in to_execute {
                    {
                        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                        data.curr_index = index;
                    }
                    execute_behavior(index, tree_name.as_str());
                    {
                        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                        data.curr_index = old_index;
                    }
                }

                // Characters do not lock on target, clear the target index
                if instance_type == BehaviorInstanceType::Player {
                    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                    data.character_instances[inst_index].target_instance_index = None;
                }
            }

            // Add to the characters

            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

            // If we are debugging this instance, send the debug data
            if Some(data.character_instances[inst_index].behavior_id) == data.debug_behavior_id {
                let debug = BehaviorDebugData {
                    executed_connections: data.executed_connections.clone(),
                    script_errors: data.script_errors.clone(),
                };
                messages.push(Message::DebugData(debug));
            }

            if let Some(position) = &data.character_instances[data.curr_index].position {
                if let Some(tile) = data.character_instances[inst_index].tile.clone() {
                    let character = CharacterData {
                        position: position.clone(),
                        old_position: data.character_instances[data.curr_index]
                            .old_position
                            .clone(),
                        max_transition_time: data.character_instances[data.curr_index]
                            .max_transition_time,
                        curr_transition_time: data.character_instances[data.curr_index]
                            .curr_transition_time,
                        tile,
                        name: data.character_instances[data.curr_index].name.clone(),
                        id: data.character_instances[data.curr_index].id,
                        index: inst_index,
                        effects: data.character_instances[data.curr_index].effects.clone(),
                    };
                    if let Some(list) = data.characters.get_mut(&position.region) {
                        list.push(character);
                    } else {
                        data.characters.insert(position.region, vec![character]);
                    }
                }
                data.character_instances[inst_index].effects = vec![];
            }

            // Check the inventory for lights
            let lights = get_inventory_lights(data);

            for mut light in lights {
                if let Some(position) = &data.character_instances[inst_index].position {
                    light.position = (position.x, position.y);
                }
                data.lights.push(light);
            }
        }

        // Parse the loot and add the lights
        {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            for (position, loot) in &data.loot {
                for item in loot {
                    if let Some(state) = &item.state {
                        if let Some(light) = &state.light {
                            let mut light_clone = light.clone();
                            light_clone.position = *position;
                            data.lights.push(light_clone);
                        }
                    }
                }
            }
        }

        // Execute region area behaviors

        let mut to_execute: Vec<(Uuid, usize, Uuid)> = vec![];
        {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.displacements = FxHashMap::default();
            for area_index in 0..data.region_data.areas.len() {
                for (node_id, node) in &data.region_area_behavior[area_index].nodes {
                    if node.behavior_type == BehaviorNodeType::InsideArea
                        || node.behavior_type == BehaviorNodeType::EnterArea
                        || node.behavior_type == BehaviorNodeType::LeaveArea
                        || node.behavior_type == BehaviorNodeType::Always
                    {
                        to_execute.push((data.region_data.id, area_index, *node_id));
                    }
                }
            }
        }

        for tuple in to_execute {
            execute_area_node(tuple.0, tuple.1, tuple.2);
        }

        // Parse the player characters and generate updates

        for inst_index in 0..character_instances_len {
            let mut send_update = false;

            {
                let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

                // Purge invalid target indices
                if let Some(target_index) =
                    data.character_instances[inst_index].target_instance_index
                {
                    if data.character_instances[target_index].state.is_dead() {
                        data.character_instances[inst_index].target_instance_index = None;
                    }
                }

                // Send update if this is a player and no editor debugging
                if data.character_instances[inst_index].instance_type
                    == BehaviorInstanceType::Player
                    && data.debug_behavior_id.is_none()
                {
                    send_update = true;
                } else
                // Otherwise send this update if this is the current character being debugged in the editor
                if Some(data.character_instances[inst_index].behavior_id)
                    == data.debug_behavior_id
                {
                    send_update = true;
                }
            }

            if send_update {
                {
                    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                    if data.character_instances[inst_index].state == BehaviorInstanceState::Purged {
                        continue;
                    }
                }

                {
                    let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                    data.curr_player_inst_index = inst_index;
                }

                let mut screen_script_name: Option<String> = None;
                let mut screen_scripts: Option<FxHashMap<String, String>> = None;

                // Send screen scripts ?

                let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];

                if data.character_instances[inst_index].send_screen_scripts == false {
                    screen_scripts = Some(self.scripts.clone());
                    data.character_instances[inst_index].send_screen_scripts = true;
                }

                if let Some(new_script_name) =
                    &data.character_instances[inst_index].new_player_script
                {
                    if *new_script_name != data.character_instances[inst_index].curr_player_script {
                        screen_script_name = Some(new_script_name.clone());
                    }
                }

                if let Some(screen_script_name) = &screen_script_name {
                    data.character_instances[inst_index].curr_player_script =
                        screen_script_name.clone();
                    data.character_instances[inst_index].new_player_script = None;
                }

                let mut region: Option<GameRegionData> = None;
                let mut characters: Vec<CharacterData> = vec![];
                let mut displacements: FxHashMap<(isize, isize), TileData> = FxHashMap::default();

                let mut needs_transfer_to: Option<Uuid> = None;
                if let Some(position) = &data.character_instances[inst_index].position.clone() {
                    if position.region != data.region_data.id {
                        // We need to transfer the character to a new region
                        needs_transfer_to = Some(position.region);
                    } else
                    // Check if the character is in a region we did not send to the client yet OR if the editor is debugging
                    if data.character_instances[inst_index]
                        .regions_send
                        .contains(&position.region)
                        == false
                        || data.debug_behavior_id.is_some()
                    {
                        region = Some(data.region_data.clone());
                        data.character_instances[inst_index]
                            .regions_send
                            .insert(position.region);
                    }
                    // Copy the displacements
                    displacements = data.displacements.clone();

                    // Send the characters of the client region
                    if let Some(chars) = data.characters.get(&position.region) {
                        characters = chars.clone();
                    }
                }

                // Set the data sheet position to the current position
                if let Some(position) = &data.character_instances[inst_index].position {
                    data.sheets[inst_index].position = position.clone();
                }

                if let Some(tile) = &data.character_instances[inst_index].tile {
                    data.sheets[inst_index].tile = tile.clone();
                } else {
                    data.sheets[inst_index].tile = TileId::empty();
                }

                let update = GameUpdate {
                    id: data.character_instances[inst_index].id,
                    screen_size: self.screen_size,
                    def_square_tile_size: self.def_square_tile_size,
                    position: data.character_instances[inst_index].position.clone(),
                    old_position: data.character_instances[inst_index].old_position.clone(),
                    max_transition_time: data.character_instances[inst_index].max_transition_time,
                    curr_transition_time: data.character_instances[inst_index].curr_transition_time,
                    sheet: data.sheets[inst_index].clone(),
                    screen_script_name,
                    screen_scripts,
                    region,
                    lights: data.lights.clone(),
                    displacements,
                    characters,
                    loot: data.loot.clone(),
                    messages: data.character_instances[inst_index].messages.clone(),
                    audio: data.character_instances[inst_index].audio.clone(),
                    multi_choice_data: data.character_instances[inst_index]
                        .multi_choice_data
                        .clone(),
                    communication: data.character_instances[inst_index].communication.clone(),
                    date: DATE.borrow().clone(),
                };

                data.character_instances[inst_index].messages = vec![];

                if let Some(transfer_to) = needs_transfer_to {
                    // Serialize character
                    messages.push(Message::TransferCharacter(
                        transfer_to,
                        data.character_instances[inst_index].clone(),
                        data.sheets[inst_index].clone(),
                    ));
                    // Purge the character
                    data.character_instances[inst_index].state = BehaviorInstanceState::Purged;
                    data.player_uuid_indices
                        .remove(&data.character_instances[inst_index].id);
                }

                if data.character_instances[inst_index].save {
                    if let Some(user_name) = &data.character_instances[inst_index].user_name {
                        let mut sheet = data.sheets[inst_index].clone();
                        sheet.behavior_id =
                            Some(data.character_instances[inst_index].behavior_id.clone());
                        sheet.screen = Some(
                            data.character_instances[inst_index]
                                .curr_player_script
                                .clone(),
                        );
                        messages.push(Message::SaveCharacter(update.id, user_name.clone(), sheet));
                    }
                    data.character_instances[inst_index].save = false;
                }

                //
                messages.push(Message::PlayerUpdate(update.id, update));
            } else {
                let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                // This handles character region transfers for NPCs
                if let Some(position) = data.character_instances[inst_index].position.clone() {
                    let mut needs_transfer_to: Option<Uuid> = None;
                    if position.region != self.region_data.id {
                        // We need to transfer the character to a new region
                        needs_transfer_to = Some(position.region);
                    }

                    if let Some(transfer_to) = needs_transfer_to {
                        // Serialize character
                        messages.push(Message::TransferCharacter(
                            transfer_to,
                            data.character_instances[inst_index].clone(),
                            data.sheets[inst_index].clone(),
                        ));
                        // Purge the character
                        data.character_instances[inst_index].state = BehaviorInstanceState::Purged;
                        data.player_uuid_indices
                            .remove(&data.character_instances[inst_index].id);
                    }
                }
            }
        }

        messages
    }

    /// Setup the region instance data by decoding the JSON for all game elements and sets up the npc and game behavior instances.
    pub fn setup(
        &mut self,
        region: String,
        region_behavior: FxHashMap<Uuid, Vec<String>>,
        behaviors: Vec<String>,
        systems: Vec<String>,
        items: Vec<String>,
        spells: Vec<String>,
        game: String,
        scripts: FxHashMap<String, String>,
    ) {
        // Decode all JSON
        if let Some(region_data) = serde_json::from_str::<GameRegionData>(&region).ok() {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.region_data = region_data.clone();
            self.region_data = region_data;

            if let Some(property) = data.region_data.settings.get("movement") {
                if let Some(value) = property.as_string() {
                    if value.to_lowercase() == "tile" {
                        data.pixel_based_movement = false;
                    }
                }
            }

            if let Some(areas) = region_behavior.get(&data.region_data.id) {
                for a in areas {
                    if let Some(ab) = serde_json::from_str::<GameBehaviorData>(&a).ok() {
                        data.region_area_behavior.push(ab);
                    }
                }
            }
        }
        for b in behaviors {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&b).ok() {
                self.behaviors.insert(behavior_data.id, behavior_data);
            }
        }
        for s in systems {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&s).ok() {
                self.system_names.push(behavior_data.name.clone());
                self.system_ids.push(behavior_data.id.clone());
                self.systems.insert(behavior_data.id, behavior_data);
            }
        }
        for i in items {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&i).ok() {
                if let Some(instances) = &behavior_data.loot {
                    for instance in instances {
                        if instance.position.region != self.region_data.id {
                            continue;
                        }
                        let mut loot = Item::new(behavior_data.id, behavior_data.name.clone());
                        loot.item_type = "gear".to_string();
                        loot.amount = instance.amount;
                        loot.stackable = 1;

                        for (_index, node) in &behavior_data.nodes {
                            if node.behavior_type == BehaviorNodeType::BehaviorType {
                                if let Some(value) = node.values.get(&"tile".to_string()) {
                                    loot.tile = value.to_tile_data();
                                }
                                if let Some(value) = node.values.get(&"settings".to_string()) {
                                    if let Some(str) = value.to_string() {
                                        let mut s = PropertySink::new();
                                        s.load_from_string(str.clone());
                                        loot.read_from_sink(&s);
                                    }
                                }
                            }
                        }

                        loot.exectute_on_startup = instance.execute_on_startup.clone();

                        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                        if let Some(v) = data
                            .loot
                            .get_mut(&(instance.position.x, instance.position.y))
                        {
                            v.push(loot);
                        } else {
                            data.loot
                                .insert((instance.position.x, instance.position.y), vec![loot]);
                        }
                    }
                }
                self.items.insert(behavior_data.id, behavior_data);
            }
        }
        for i in spells {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&i).ok() {
                self.spells.insert(behavior_data.id, behavior_data);
            }
        }
        if let Some(game_data) = serde_json::from_str(&game).ok() {
            self.game_data = game_data;

            // Update the game settings, just in case they don't contain the latest
            if self.game_data.settings.is_some() {
                crate::gamedata::game::update_game_sink(
                    &mut self.game_data.settings.as_mut().unwrap(),
                );
            }

            // Read global game settings

            if let Some(settings) = &self.game_data.settings {
                if let Some(screen_size) = settings.get("screen_size") {
                    match screen_size.value {
                        PropertyValue::IntX(v) => {
                            self.screen_size = (v[0], v[1]);
                        }
                        _ => {}
                    }
                }
                if let Some(def_square_tile_size) = settings.get("def_square_tile_size") {
                    match def_square_tile_size.value {
                        PropertyValue::Int(v) => {
                            self.def_square_tile_size = v;
                        }
                        _ => {}
                    }
                }

                if let Some(property) = settings.get("gear_slots") {
                    if let Some(name) = property.as_string() {
                        let ar: Vec<&str> = name.split(",").collect();
                        for s in ar {
                            self.gear_slots.push(s.to_lowercase().trim().to_string());
                        }
                    }
                }

                if let Some(property) = settings.get("weapon_slots") {
                    if let Some(name) = property.as_string() {
                        let ar: Vec<&str> = name.split(",").collect();
                        for s in ar {
                            self.weapon_slots.push(s.to_lowercase().trim().to_string());
                        }
                    }
                }

                if let Some(property) = settings.get("ticks_per_minute") {
                    if let Some(ticks) = property.as_int() {
                        self.ticks_per_minute = ticks as usize;
                        *TICKS_PER_MINUTE.borrow_mut() = ticks as usize;
                    }
                }
            }
        }

        self.scripts = scripts;

        // Create all behavior instances of characters inside this region
        let ids: Vec<Uuid> = self.behaviors.keys().cloned().collect();
        for id in ids {
            self.create_behavior_instance(id, true, None);
        }

        // Generate Loot

        let mut loot_map;

        {
            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            loot_map = data.loot.clone();
        }

        // We iterate over all loot and initialize state if necessary

        for (pos, loot) in &mut loot_map {
            for index in 0..loot.len() {
                if let Some(mut state) = check_and_create_item_state(
                    loot[index].id,
                    loot[index].exectute_on_startup.clone(),
                ) {
                    if let Some(light) = &mut state.light {
                        light.position = pos.clone();
                    }
                    if state.tile.is_none() {
                        state.tile = loot[index].tile.clone();
                    }
                    loot[index].tile = None;
                    loot[index].state = Some(state);
                }
            }
        }

        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        data.loot = loot_map;
    }

    /// Creates a new user character
    pub fn create_character(
        &mut self,
        uuid: Uuid,
        user_name: Option<String>,
        char_data: CharacterInstanceData,
    ) {
        let mut player_id: Option<Uuid> = None;
        for b in &self.behaviors {
            if b.1.name == "Player" {
                player_id = Some(*b.0);
            }
        }
        if let Some(player_id) = player_id {
            let index = self.create_behavior_instance(player_id, false, Some(char_data.clone()));
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.character_instances[index].instance_type = BehaviorInstanceType::Player;
            data.character_instances[index].id = uuid;
            data.character_instances[index].user_name = user_name;
            data.character_instances[index].save = true; // Save the character after being created
            data.character_instances[index].position = Some(char_data.position);
            data.player_uuid_indices.insert(uuid, index);
            log::info!("Player {:?} created.", char_data.name);
        }
    }

    /// Login a user character
    pub fn login_character(&mut self, uuid: Uuid, user_name: String, mut sheet: Sheet) {
        let class_name: String = sheet.class_name.clone();
        let index;

        {
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            index = data.sheets.len();

            // Collect the ids of the behavior trees
            // We do not save them with the sheet in case the game gets updated

            let mut to_execute: Vec<Uuid> = vec![];
            if let Some(id) = sheet.behavior_id {
                if let Some(behavior) = &self.behaviors.get_mut(&id) {
                    for (id, node) in &behavior.nodes {
                        if node.behavior_type == BehaviorNodeType::BehaviorTree {
                            for (value_name, value) in &node.values {
                                if *value_name == "execute".to_string() {
                                    if let Some(v) = value.to_integer() {
                                        if v == 0 {
                                            // Always
                                            for c in &behavior.connections {
                                                if c.0 == *id {
                                                    to_execute.push(c.0);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let instance = BehaviorInstance {
                id: uuid,
                user_name: Some(user_name),
                save: false,
                logoff: false,
                state: BehaviorInstanceState::Normal,
                name: sheet.name.clone(),
                behavior_id: sheet.behavior_id.clone().unwrap(),
                tree_ids: to_execute,
                system_tree_tick_names: vec![],
                position: Some(sheet.position.clone()),
                tile: Some(sheet.tile.clone()),
                target_instance_index: None,
                locked_tree: None,
                party: vec![],
                node_values: FxHashMap::default(),
                sleep_cycles: 0,
                systems_id: Uuid::new_v4(),
                action: None,
                instance_type: BehaviorInstanceType::Player,
                update: None,
                regions_send: std::collections::HashSet::new(),
                curr_player_script: "".to_string(),
                game_locked_tree: None,
                new_player_script: sheet.screen.clone(),
                messages: vec![],
                audio: vec![],
                old_position: None,
                max_transition_time: 0,
                curr_transition_time: 0,
                multi_choice_data: vec![],
                communication: vec![],
                multi_choice_answer: None,
                effects: vec![],
                instance_creation_data: None,
                send_screen_scripts: false,
            };

            sheet.screen = None;
            sheet.behavior_id = None;

            data.sheets.push(sheet);
            data.character_instances.push(instance);
            data.player_uuid_indices.insert(uuid, index);
        }

        self.set_level_tree(index, class_name);
    }

    /// Creates a new player instance
    pub fn create_player_instance(&mut self, uuid: Uuid, position: Position) {
        let mut player_id: Option<Uuid> = None;
        for b in &self.behaviors {
            if b.1.name == "Player" {
                player_id = Some(*b.0);
            }
        }
        if let Some(player_id) = player_id {
            let index = self.create_behavior_instance(player_id, false, None);
            let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
            data.character_instances[index].instance_type = BehaviorInstanceType::Player;
            data.character_instances[index].id = uuid;
            data.character_instances[index].position = Some(position);
            data.player_uuid_indices.insert(uuid, index);
            log::info!("Player instance {} created.", uuid);
        }
    }

    /// Destroys a player instance
    pub fn destroy_player_instance(&mut self, uuid: Uuid) {
        let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        for inst_index in 0..data.character_instances.len() {
            if data.character_instances[inst_index].id == uuid {
                self.purge_instance(inst_index, data);
                break;
            }
        }
    }

    /// Creates an instance of a behavior (character)
    fn create_behavior_instance(
        &mut self,
        id: Uuid,
        npc_only: bool,
        data: Option<CharacterInstanceData>,
    ) -> usize {
        let mut index = 0;

        let mut startup_trees: Vec<Uuid> = vec![];
        let mut behavior_name = "".to_string();
        let mut behavior_id = Uuid::new_v4();
        let mut class_name: Option<String> = None;
        let mut race_name: Option<String> = None;

        let mut to_create: Vec<CharacterInstanceData> = vec![];

        // Collect all the default data for the behavior from the nodes: Position, tile, behavior Trees and variables.
        let mut to_execute: Vec<Uuid> = vec![];
        let mut default_position: Option<Position> = None;
        let mut default_tile: Option<TileId> = None;
        let mut default_alignment: i32 = 1;
        let mut settings_sink = PropertySink::new();

        // Instances to create for this behavior
        if let Some(behavior) = &self.behaviors.get_mut(&id) {
            behavior_name = behavior.name.clone();
            behavior_id = behavior.id.clone();

            if npc_only && behavior.name == "Player" {
                return index;
            }

            for (id, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree {
                    for (value_name, value) in &node.values {
                        if *value_name == "execute".to_string() {
                            if let Some(v) = value.to_integer() {
                                if v == 1 {
                                    // Startup only tree
                                    for c in &behavior.connections {
                                        if c.0 == *id {
                                            startup_trees.push(c.0);
                                        }
                                    }
                                    break;
                                } else if v == 0 {
                                    // Always
                                    for c in &behavior.connections {
                                        if c.0 == *id {
                                            to_execute.push(c.0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value) = node.values.get(&"position".to_string()) {
                        default_position = value.to_position();
                    }
                    if let Some(value) = node.values.get(&"tile".to_string()) {
                        default_tile = value.to_tile_id()
                    }
                    if let Some(value) = node.values.get(&"settings".to_string()) {
                        if let Some(settings) = value.to_string() {
                            settings_sink.load_from_string(settings);
                        }
                    }
                    if let Some(value) = node.values.get(&"alignment".to_string()) {
                        if let Some(alignment) = value.to_integer() {
                            default_alignment = 2 - alignment - 1;
                        }
                    }
                }
            }
            // Add main
            if default_position.is_some() && default_tile.is_some() && data.is_none() {
                let main = CharacterInstanceData {
                    position: default_position.unwrap().clone(),
                    tile: default_tile.clone(),
                    name: Some(behavior.name.clone()),
                    alignment: Some(default_alignment),
                    class: None,
                    race: None,
                    screen: None,
                };
                to_create.push(main)
            }
            if data.is_none() {
                // Add the instances of main
                if let Some(instances) = &behavior.instances {
                    for i in instances {
                        let mut inst = (*i).clone();
                        if inst.name.is_none() {
                            inst.name = Some(behavior.name.clone());
                        }
                        if inst.tile.is_none() {
                            inst.tile = default_tile.clone();
                        }
                        to_create.push(inst);
                    }
                }
            } else {
                // If we get the character instance data, only add this (new player, respawn)
                to_create.push(data.unwrap());
            }
        }

        // Now we have all instances of the behavior we need to create
        for inst in to_create {
            // Only create when instance is in this region
            if inst.position.region != self.region_data.id {
                continue;
            }

            let mut instance: BehaviorInstance = BehaviorInstance {
                id: uuid::Uuid::new_v4(),
                user_name: None,
                save: false,
                logoff: false,
                state: BehaviorInstanceState::Normal,
                name: behavior_name.clone(),
                behavior_id: behavior_id,
                tree_ids: to_execute.clone(),
                system_tree_tick_names: vec![],
                position: Some(inst.position.clone()),
                tile: inst.tile.clone(),
                target_instance_index: None,
                locked_tree: None,
                party: vec![],
                node_values: FxHashMap::default(),
                sleep_cycles: 0,
                systems_id: Uuid::new_v4(),
                action: None,
                instance_type: BehaviorInstanceType::NonPlayerCharacter,
                update: None,
                regions_send: std::collections::HashSet::new(),
                curr_player_script: String::new(),
                game_locked_tree: None,
                new_player_script: None,
                messages: vec![],
                audio: vec![],
                old_position: None,
                max_transition_time: 0,
                curr_transition_time: 0,
                multi_choice_data: vec![],
                communication: vec![],
                multi_choice_answer: None,
                effects: vec![],
                instance_creation_data: Some(inst.clone()),
                send_screen_scripts: false,
            };

            {
                let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                index = data.character_instances.len();
            }

            // Player got a screen name, assign it
            if let Some(screen) = inst.screen {
                if inst.tile.is_none() {
                    instance.tile = default_tile.clone();
                }

                instance.new_player_script = self.get_script_name_for_screen(screen);
            }

            // Create skills

            let mut skills = Skills::new();

            for (_id, behavior) in &self.systems {
                if behavior.name.to_lowercase() == "skills" {
                    for (_id, node) in &behavior.nodes {
                        if node.behavior_type == BehaviorNodeType::SkillTree {
                            skills.add_skill(node.name.clone());

                            // Add the skill to the skill_tree

                            let mut rc: Vec<(i32, String, String)> = vec![];
                            let mut parent_id = node.id;

                            loop {
                                let mut found = false;
                                for (id1, c1, id2, c2) in &behavior.connections {
                                    if *id1 == parent_id && *c1 == BehaviorNodeConnector::Bottom {
                                        for (uuid, node) in &behavior.nodes {
                                            if *uuid == *id2 {
                                                let mut start = 0;
                                                if let Some(value) =
                                                    node.values.get(&"start".to_string())
                                                {
                                                    if let Some(i) = value.to_integer() {
                                                        start = i;
                                                    }
                                                }
                                                let mut message = "".to_string();
                                                if let Some(value) =
                                                    node.values.get(&"message".to_string())
                                                {
                                                    if let Some(m) = value.to_string() {
                                                        message = m;
                                                    }
                                                }

                                                parent_id = node.id;
                                                found = true;

                                                rc.push((start, node.name.clone(), message));
                                            }
                                        }
                                    } else if *id2 == parent_id
                                        && *c2 == BehaviorNodeConnector::Bottom
                                    {
                                        for (uuid, node) in &behavior.nodes {
                                            if *uuid == *id1 {
                                                let mut start = 0;
                                                if let Some(value) =
                                                    node.values.get(&"start".to_string())
                                                {
                                                    if let Some(i) = value.to_integer() {
                                                        start = i;
                                                    }
                                                }
                                                let mut message = "".to_string();
                                                if let Some(value) =
                                                    node.values.get(&"message".to_string())
                                                {
                                                    if let Some(m) = value.to_string() {
                                                        message = m;
                                                    }
                                                }
                                                parent_id = node.id;
                                                found = true;

                                                rc.push((start, node.name.clone(), message));
                                            }
                                        }
                                    }
                                }
                                if found == false {
                                    break;
                                }
                            }

                            let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                            data.skill_trees.insert(node.name.clone(), rc);
                        }
                    }
                }
            }

            let mut system_startup_trees: Vec<String> = vec![];

            // Get the class and race names and add them to the startup trees

            if let Some(class) = settings_sink.get("class") {
                if let Some(cl) = class.as_string() {
                    class_name = Some(cl.clone());
                }
            }
            if let Some(race) = settings_sink.get("race") {
                if let Some(ra) = race.as_string() {
                    race_name = Some(ra.clone());
                }
            }

            if let Some(class) = inst.class {
                class_name = Some(class);
            }

            if let Some(race) = inst.race {
                race_name = Some(race);
            }

            if let Some(class_name) = &class_name {
                system_startup_trees.push(class_name.clone());
            }

            if let Some(race_name) = &race_name {
                system_startup_trees.push(race_name.clone());
            }

            let mut system_tree_tick_names: Vec<(String, String)> = vec![];
            let mut startup_system_trees: Vec<(Uuid, Uuid)> = vec![];

            // Execute the startup trees in the given systems for execution (for class and race) and store
            // the trees to execute every tick
            for system_name in system_startup_trees {
                if self.system_names.contains(&system_name) {
                    for (system_id, system) in &self.systems {
                        if system.name == system_name {
                            for (id, node) in &system.nodes {
                                if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                    for (value_name, value) in &node.values {
                                        if *value_name == "execute".to_string() {
                                            if let Some(v) = value.to_integer() {
                                                if v == 0 {
                                                    // Always, store it for execution during ticks
                                                    system_tree_tick_names.push((
                                                        system_name.clone(),
                                                        node.name.clone(),
                                                    ));
                                                } else if v == 1 {
                                                    // Startup only tree
                                                    for c in &system.connections {
                                                        if c.0 == *id {
                                                            startup_system_trees
                                                                .push((*system_id, c.0));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            instance.system_tree_tick_names = system_tree_tick_names;

            // Add Spells appropriate for this character

            let mut spells = Spells::new();

            for (id, behavior) in &self.spells {
                for (_id, node) in &behavior.nodes {
                    if node.behavior_type == BehaviorNodeType::BehaviorType {
                        if let Some(value) = node.values.get(&"settings".to_string()) {
                            let mut spell_tile: Option<TileData> = None;
                            let mut spells_sink = PropertySink::new();
                            let mut spell_distance = 3;

                            if let Some(value) = node.values.get(&"tile".to_string()) {
                                spell_tile = value.to_tile_data();
                            }
                            if let Some(settings) = value.to_string() {
                                spells_sink.load_from_string(settings);
                            }

                            let mut include_spell = false;

                            if let Some(c) = spells_sink.get_as_string_array("classes") {
                                if c[0].to_lowercase() == "all" {
                                    include_spell = true;
                                } else if let Some(class_name) = &class_name {
                                    for v in 0..c.len() {
                                        if c[v].to_lowercase() == class_name.to_lowercase() {
                                            include_spell = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            if let Some(distance) = spells_sink.get(&"spell_distance") {
                                if let Some(d) = distance.as_int() {
                                    spell_distance = d;
                                }
                            }

                            if include_spell {
                                let mut spell = Spell::new(*id, behavior.name.to_string());
                                spell.tile = spell_tile;
                                spell.distance = spell_distance;
                                spells.spells.push(spell);
                            }
                        }
                        break;
                    }
                }
            }

            // --- End Spells

            let instances_len;

            // Set the sheet
            {
                let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
                let mut sheet = Sheet::new();
                sheet.name = behavior_name.clone();

                if let Some(name) = inst.name {
                    sheet.name = name.clone();
                    instance.name = name;
                }

                if let Some(class_name) = class_name.clone() {
                    sheet.class_name = class_name;
                }
                if let Some(race_name) = race_name.clone() {
                    sheet.race_name = race_name;
                }
                sheet.position = inst.position.clone();
                sheet.home_location = inst.position;
                sheet.alignment = if inst.alignment.is_some() {
                    inst.alignment.unwrap()
                } else {
                    default_alignment
                };
                sheet.spells = spells;
                sheet.skills = skills;
                data.sheets.push(sheet);
                data.character_instances.push(instance);
                data.curr_index = index;

                instances_len = data.character_instances.len();
            }

            if index < instances_len {
                // Set the class based level tree
                if let Some(class_name) = class_name.clone() {
                    self.set_level_tree(index, class_name);
                }

                // Execute the system startup trees
                for (system_id, node_id) in &startup_system_trees {
                    execute_node(*system_id, *node_id, &mut SYSTEMS.borrow_mut());
                }

                // Execute the startup only trees
                for startup_id in &startup_trees {
                    {
                        REGION_DATA.borrow_mut()[*CURR_INST.borrow()].curr_index = index;
                    }
                    execute_node(behavior_id, startup_id.clone(), &mut BEHAVIORS.borrow_mut());
                }
            }
        }

        index
    }

    /// Returns the layered tiles at the given position and checks for displacements
    pub fn get_tile_without_displacements_at(&self, pos: (isize, isize)) -> Vec<TileData> {
        let mut rc = vec![];

        if let Some(t) = self.region_data.layer1.get(&pos) {
            rc.push(t.clone());
        }
        if let Some(t) = self.region_data.layer2.get(&pos) {
            rc.push(t.clone());
        }
        if let Some(t) = self.region_data.layer3.get(&pos) {
            rc.push(t.clone());
        }
        if let Some(t) = self.region_data.layer4.get(&pos) {
            rc.push(t.clone());
        }
        rc
    }

    /// Gets the behavior for the given id
    pub fn get_behavior(&self, id: Uuid, behavior_type: BehaviorType) -> Option<&GameBehaviorData> {
        if behavior_type == BehaviorType::Regions {
            for b in &self.region_behavior {
                if b.id == id {
                    return Some(&b);
                }
            }
        } else if behavior_type == BehaviorType::Behaviors {
            return self.behaviors.get(&id);
        } else if behavior_type == BehaviorType::Systems {
            return self.systems.get(&id);
        } else if behavior_type == BehaviorType::Items {
            return self.items.get(&id);
        } else if behavior_type == BehaviorType::GameLogic {
            return Some(&self.game_data);
        }
        None
    }

    /// Gets the mutable behavior for the given behavior type
    pub fn get_mut_behavior(
        &mut self,
        id: Uuid,
        behavior_type: BehaviorType,
    ) -> Option<&mut GameBehaviorData> {
        if behavior_type == BehaviorType::Regions {
            for b in &mut self.region_behavior {
                if b.id == id {
                    return Some(b);
                }
            }
        } else if behavior_type == BehaviorType::Behaviors {
            return self.behaviors.get_mut(&id);
        } else if behavior_type == BehaviorType::Systems {
            return self.systems.get_mut(&id);
        } else if behavior_type == BehaviorType::Items {
            return self.items.get_mut(&id);
        } else if behavior_type == BehaviorType::GameLogic {
            return Some(&mut self.game_data);
        }
        None
    }

    /// Purges this instance, voiding it.
    pub fn purge_instance(&mut self, inst_index: usize, data: &mut RegionData) {
        data.character_instances[inst_index].state = BehaviorInstanceState::Purged;
        data.player_uuid_indices
            .remove(&data.character_instances[inst_index].id);
    }

    /// Transfers a character instance into this region
    pub fn transfer_character_into(&mut self, instance: BehaviorInstance, sheet: Sheet) {
        // TODO, fill in purged
        let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        data.player_uuid_indices
            .insert(instance.id, data.character_instances.len());

        data.character_instances.push(instance);
        data.sheets.push(sheet);
    }

    /// Sets the debugging behavior id.
    pub fn set_debug_behavior_id(&mut self, behavior_id: Uuid) {
        let data: &mut RegionData = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        data.debug_behavior_id = Some(behavior_id);
    }

    /// Gets the current time in milliseconds
    pub fn get_time(&self) -> u128 {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window().unwrap().performance().unwrap().now() as u128
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let stop = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            stop.as_millis()
        }
    }

    /// Set class based level tree for a character
    pub fn set_level_tree(&mut self, instance_index: usize, system_name: String) {
        let tree_name = "Level Tree".to_string();

        let mut levels: Vec<(i32, String, Uuid)> = vec![];
        let mut level_behavior_id = Uuid::new_v4();
        let mut experience_msg: String = "You gained {} experience.".to_string();

        for (id, behavior) in &self.systems {
            if behavior.name == system_name {
                for (_id, node) in &behavior.nodes {
                    if node.name == tree_name {
                        if let Some(value) = node.values.get(&"message".to_string()) {
                            if let Some(m) = value.to_string() {
                                experience_msg = m;
                            }
                        }
                        // Store the levels

                        let mut rc: Vec<(i32, String, Uuid)> = vec![];
                        let mut parent_id = node.id;

                        level_behavior_id = *id;

                        loop {
                            let mut found = false;
                            for (id1, c1, id2, c2) in &behavior.connections {
                                if *id1 == parent_id && *c1 == BehaviorNodeConnector::Bottom {
                                    for (uuid, node) in &behavior.nodes {
                                        if *uuid == *id2 {
                                            let mut start = 0;
                                            if let Some(value) =
                                                node.values.get(&"start".to_string())
                                            {
                                                if let Some(i) = value.to_integer() {
                                                    start = i;
                                                }
                                            }
                                            let mut message = "".to_string();
                                            if let Some(value) =
                                                node.values.get(&"message".to_string())
                                            {
                                                if let Some(m) = value.to_string() {
                                                    message = m;
                                                }
                                            }

                                            parent_id = node.id;
                                            found = true;

                                            rc.push((start, message, parent_id));
                                        }
                                    }
                                } else if *id2 == parent_id && *c2 == BehaviorNodeConnector::Bottom
                                {
                                    for (uuid, node) in &behavior.nodes {
                                        if *uuid == *id1 {
                                            let mut start = 0;
                                            if let Some(value) =
                                                node.values.get(&"start".to_string())
                                            {
                                                if let Some(i) = value.to_integer() {
                                                    start = i;
                                                }
                                            }
                                            let mut message = "".to_string();
                                            if let Some(value) =
                                                node.values.get(&"message".to_string())
                                            {
                                                if let Some(m) = value.to_string() {
                                                    message = m;
                                                }
                                            }
                                            parent_id = node.id;
                                            found = true;

                                            rc.push((start, message, parent_id));
                                        }
                                    }
                                }
                            }
                            if found == false {
                                break;
                            }
                        }

                        levels = rc;
                    }
                }
            }
        }

        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        let sheet = &mut data.sheets[instance_index];
        sheet.experience.system_name = Some(system_name);
        sheet.experience.tree_name = Some(tree_name.to_string());
        sheet.experience.levels = levels;
        sheet.experience.experience_msg = experience_msg;
        sheet.experience.level_behavior_id = level_behavior_id;
    }

    /// Returns the script name for a given game behavior tree
    fn get_script_name_for_screen(&self, screen_name: String) -> Option<String> {
        let mut screen_node_id: Option<Uuid> = None;

        for (id, node) in &self.game_data.nodes {
            if node.behavior_type == BehaviorNodeType::BehaviorTree {
                if node.name == screen_name {
                    for c in &self.game_data.connections {
                        if c.0 == *id {
                            screen_node_id = Some(c.2);
                        }
                    }
                }
            }
        }

        if let Some(screen_node_id) = screen_node_id {
            if let Some(screen_node) = self.game_data.nodes.get(&screen_node_id) {
                if let Some(value) = screen_node.values.get("script_name") {
                    if let Some(script_name) = value.to_string() {
                        return Some(script_name);
                    }
                }
            }
        }
        None
    }
}
