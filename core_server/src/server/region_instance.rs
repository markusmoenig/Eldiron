use crate::{prelude::*};
use rhai::{Engine, AST, Scope};

pub struct RegionInstance<'a> {
    // Game data
    pub region_data                 : GameRegionData,
    pub region_behavior             : Vec<GameBehaviorData>,

    pub behaviors                   : FxHashMap<Uuid, GameBehaviorData>,
    pub systems                     : FxHashMap<Uuid, GameBehaviorData>,
    pub items                       : FxHashMap<Uuid, GameBehaviorData>,
    pub game_data                   : GameBehaviorData,

    // For faster lookup
    pub system_names                : Vec<String>,
    pub system_ids                  : Vec<Uuid>,
    pub area_ids                    : Vec<Uuid>,

    // All nodes
    nodes                           : FxHashMap<BehaviorNodeType, NodeCall>,

    /// The script engine
    pub engine                      : Engine,
    /// Script ast's, id is (BehaviorType, BehaviorId, BehaviorNodeID, AtomParameterID)
    pub ast                         : FxHashMap<(BehaviorType, Uuid, Uuid, String), AST>,

    // Character instances
    pub instances                   : Vec<BehaviorInstance>,
    pub scopes                      : Vec<rhai::Scope<'a>>,

    /// The loot in the region
    pub loot                        : FxHashMap<(isize, isize), Vec<LootData>>,

    /// During action execution for regions this indicates the calling behavior index
    pub curr_action_inst_index      : Option<usize>,

    /// The current instance index of the current "Player" when executing the Game behavior per player
    pub curr_player_inst_index      : usize,

    /// If the execute_node call has indirection, this is set to the original index
    pub curr_redirected_inst_index  : Option<usize>,

    /// Player game scopes
    pub game_player_scopes          : FxHashMap<usize, Scope<'a>>,

    /// The index of the game instance
    game_instance_index             : Option<usize>,

    /// Player uuid => player instance index
    pub player_uuid_indices         : FxHashMap<Uuid, usize>,

    /// The displacements for this region
    pub displacements               : HashMap<(isize, isize), TileData>,

    // Used by ticks for state memory

    /// Current characters per region
    pub characters                  : FxHashMap<Uuid, Vec<CharacterData>>,
    // Characters instance indices in a given area
    pub area_characters             : FxHashMap<usize, Vec<usize>>,
    // The character instances from the previous tick, used to figure out onEnter, onLeave etc events
    pub prev_area_characters        : FxHashMap<usize, Vec<usize>>,

    // Lights for this region
    pub lights                      : Vec<LightData>,

    // The current move direction of the player
    pub action_direction_text       : String,

    // The current subject (inventory item etc.) of the player
    pub action_subject_text         : String,

    // Identifie the currently executing loot item
    pub curr_loot_item              : Option<(isize, isize, usize)>,

    // Identify the currently executing inventory item index
    pub curr_inventory_index        : Option<usize>,

    // The current player scope (if swapped out during item execution)
    pub curr_player_scope           : Scope<'a>,

    // The currently executing behavior tree id
    pub curr_executing_tree         : Uuid,

    // These are fields which provide debug feedback while running and are only used in the editors debug mode

    // The behavior id to debug, this is send from the server
    debug_behavior_id               : Option<Uuid>,

    // We are debugging the current tick characters
    is_debugging                    : bool,

    pub messages                    : Vec<(String, MessageType)>,
    pub executed_connections        : Vec<(BehaviorType, Uuid, BehaviorNodeConnector)>,
    pub script_errors               : Vec<((Uuid, Uuid, String), (String, Option<u32>))>,

    // Region settings

    pub pixel_based_movement        : bool,

    /// Tick count used for timing
    pub tick_count                  : usize,
    pub dealt_damage_success        : bool,

    /// Respawns the given chararacter uuid at the given tick count
    pub respawn_instance            : FxHashMap<Uuid, (usize, CharacterInstanceData)>,

    // Game settings

    screen_size                     : (i32, i32),
    def_square_tile_size            : i32,

    pub gear_slots                  : Vec<String>,
    pub weapon_slots                : Vec<String>,

    pub skill_trees                 : FxHashMap<String, Vec<(i32, String, String)>>,

    // Variable names

    pub primary_currency            : String,
    pub hitpoints                   : String,
    pub max_hitpoints               : String,
}

impl RegionInstance<'_> {

    pub fn new() -> Self {

        let mut engine = Engine::new();

        // Variable resolver for d??? -> random(???)
        #[allow(deprecated)]
        engine.on_var(|name, _index, _context| {

            if name.starts_with("d") {
                let mut s = name.to_string();
                s.remove(0);
                if let Some(n) = s.parse::<i32>().ok() {
                    let mut rng = thread_rng();
                    let random = rng.gen_range(1..=n) as f32;
                    return Ok(Some(random.into()));
                }
            }
            Ok(None)
        });

        script_register_message_api(&mut engine);
        script_register_inventory_api(&mut engine);
        script_register_gear_api(&mut engine);
        script_register_weapons_api(&mut engine);
        script_register_experience_api(&mut engine);

        // Display f64 as ints
        use pathfinding::num_traits::ToPrimitive;
        engine.register_fn("to_string", |x: f32| format!("{}", x.to_isize().unwrap()));

        let mut nodes : FxHashMap<BehaviorNodeType, NodeCall> = FxHashMap::default();

        nodes.insert(BehaviorNodeType::Expression, expression);
        nodes.insert(BehaviorNodeType::Script, script);
        nodes.insert(BehaviorNodeType::Pathfinder, pathfinder);
        nodes.insert(BehaviorNodeType::Lookout, lookout);
        nodes.insert(BehaviorNodeType::CloseIn, close_in);
        nodes.insert(BehaviorNodeType::CallSystem, call_system);
        nodes.insert(BehaviorNodeType::CallBehavior, call_behavior);
        nodes.insert(BehaviorNodeType::HasTarget, has_target);
        nodes.insert(BehaviorNodeType::Untarget, untarget);
        nodes.insert(BehaviorNodeType::DealDamage, deal_damage);
        nodes.insert(BehaviorNodeType::TakeDamage, take_damage);
        nodes.insert(BehaviorNodeType::DropInventory, drop_inventory);
        nodes.insert(BehaviorNodeType::Effect, effect);
        nodes.insert(BehaviorNodeType::Audio, audio);
        nodes.insert(BehaviorNodeType::Heal, heal);
        nodes.insert(BehaviorNodeType::TakeHeal, take_heal);
        nodes.insert(BehaviorNodeType::Respawn, respawn);
        nodes.insert(BehaviorNodeType::SetLevelTree, set_level_tree);

        nodes.insert(BehaviorNodeType::OverlayTiles, overlay_tiles);

        nodes.insert(BehaviorNodeType::Move, player_move);
        nodes.insert(BehaviorNodeType::Screen, screen);
        nodes.insert(BehaviorNodeType::Widget, widget);
        nodes.insert(BehaviorNodeType::Message, message);
        nodes.insert(BehaviorNodeType::Action, player_action);
        nodes.insert(BehaviorNodeType::Take, player_take);
        nodes.insert(BehaviorNodeType::Drop, player_drop);
        nodes.insert(BehaviorNodeType::Target, player_target);
        nodes.insert(BehaviorNodeType::LightItem, light_item);
        nodes.insert(BehaviorNodeType::SetItemTile, set_item_tile);
        nodes.insert(BehaviorNodeType::RandomWalk, random_walk);
        nodes.insert(BehaviorNodeType::MultiChoice, multi_choice);
        nodes.insert(BehaviorNodeType::Sell, sell);
        nodes.insert(BehaviorNodeType::LockTree, lock_tree);
        nodes.insert(BehaviorNodeType::UnlockTree, unlock_tree);
        nodes.insert(BehaviorNodeType::SetState, set_state);
        nodes.insert(BehaviorNodeType::Teleport, teleport);
        nodes.insert(BehaviorNodeType::Equip, player_equip);

        nodes.insert(BehaviorNodeType::Always, always);
        nodes.insert(BehaviorNodeType::InsideArea, inside_area);
        nodes.insert(BehaviorNodeType::EnterArea, enter_area);
        nodes.insert(BehaviorNodeType::LeaveArea, leave_area);
        nodes.insert(BehaviorNodeType::TeleportArea, teleport_area);
        nodes.insert(BehaviorNodeType::MessageArea, message_area);
        nodes.insert(BehaviorNodeType::AudioArea, audio_area);
        nodes.insert(BehaviorNodeType::LightArea, light_area);
        nodes.insert(BehaviorNodeType::ActionArea, action);

        nodes.insert(BehaviorNodeType::SkillTree, skill_tree);
        nodes.insert(BehaviorNodeType::SkillLevel, skill_level);

        Self {
            region_data                     : GameRegionData::new(),
            region_behavior                 : vec![],

            behaviors                       : FxHashMap::default(),
            systems                         : FxHashMap::default(),
            items                           : FxHashMap::default(),
            game_data                       : GameBehaviorData::new(),

            system_names                    : vec![],
            system_ids                      : vec![],
            area_ids                        : vec![],

            engine,
            ast                             : FxHashMap::default(),
            nodes,

            instances                       : vec![],
            scopes                          : vec![],

            loot                            : FxHashMap::default(),

            curr_action_inst_index          : None,

            curr_player_inst_index          : 0,

            curr_redirected_inst_index      : None,

            game_player_scopes              : FxHashMap::default(),

            game_instance_index             : None,

            player_uuid_indices             : FxHashMap::default(),

            displacements                   : HashMap::new(),

            characters                      : FxHashMap::default(),
            area_characters                 : FxHashMap::default(),
            prev_area_characters            : FxHashMap::default(),
            lights                          : vec![],

            action_direction_text           : "".to_string(),
            action_subject_text             : "".to_string(),

            curr_loot_item                  : None,
            curr_inventory_index            : None,
            curr_player_scope               : Scope::new(),

            curr_executing_tree             : Uuid::new_v4(),

            debug_behavior_id               : None,
            is_debugging                    : false,

            messages                        : vec![],
            executed_connections            : vec![],
            script_errors                   : vec![],

            pixel_based_movement            : true,

            tick_count                      : 0,
            dealt_damage_success            : false,

            respawn_instance                : FxHashMap::default(),

            screen_size                     : (1024, 608),
            def_square_tile_size            : 32,

            weapon_slots                    : vec![],
            gear_slots                      : vec![],

            skill_trees                     : FxHashMap::default(),

            // Variable names
            primary_currency                : "".to_string(),
            hitpoints                       : "".to_string(),
            max_hitpoints                   : "".to_string()
        }
    }

    /// Game tick
    pub fn tick(&mut self) -> Vec<Message> {

        self.messages = vec![];
        self.characters = FxHashMap::default();
        self.lights = vec![];
        self.prev_area_characters = self.area_characters.clone();
        self.area_characters = FxHashMap::default();

        let mut messages = vec![];

        let tick_time = self.get_time();

        // Check if we need to respawn something

        if self.respawn_instance.is_empty() == false {
            for (id, (tick, data)) in &self.respawn_instance.clone() {
                if *tick <= self.tick_count {
                    self.create_behavior_instance(*id, false, Some(data.clone()));
                    self.respawn_instance.remove(id);
                }
            }
        }

        // Execute behaviors
        for inst_index in 0..self.instances.len() {

            self.messages = vec![];
            self.executed_connections = vec![];
            self.script_errors = vec![];

            self.instances[inst_index].audio = vec![];
            self.instances[inst_index].multi_choice_data = vec![];

            if self.pixel_based_movement == true {
                if  self.instances[inst_index].old_position.is_some() {
                    self.instances[inst_index].curr_transition_time += 1;

                    if self.instances[inst_index].curr_transition_time > self.instances[inst_index].max_transition_time {
                        self.instances[inst_index].old_position = None;
                        self.instances[inst_index].curr_transition_time = 0;
                    }
                }
            }

            // Skip Sleep cycles
            if self.instances[inst_index].sleep_cycles > 0 {
                self.instances[inst_index].sleep_cycles -= 1;
            } else {

                // Purged: Skip
                if self.instances[inst_index].state == BehaviorInstanceState::Purged {
                    continue;
                }

                // Killed: NPC Skip
                if self.instances[inst_index].state == BehaviorInstanceState::Killed && self.instances[inst_index].instance_type == BehaviorInstanceType::NonPlayerCharacter {
                    continue;
                }

                // Are we debugging this character ?
                self.is_debugging = Some(self.instances[inst_index].behavior_id) == self.debug_behavior_id;

                if self.instances[inst_index].instance_type == BehaviorInstanceType::NonPlayerCharacter {

                    let mut execute_trees = true;

                    // Check if this NPC has active communication
                    if self.instances[inst_index].communication.is_empty() == false {
                        let mut com_to_drop : Option<usize> = None;

                        for c_index in 0..self.instances[inst_index].communication.len() {
                            if self.instances[inst_index].communication[c_index].end_time < tick_time {

                                // Drop this communication for the NPC
                                com_to_drop = Some(c_index);

                                // Remove the communication for the Player
                                let player_index = self.instances[inst_index].communication[c_index].player_index;
                                self.instances[player_index].communication = vec![];
                                self.instances[player_index].multi_choice_data = vec![];

                                break;
                            }
                        }

                        if let Some(index) = com_to_drop {
                            self.instances[inst_index].communication.remove(index);
                        }

                        if self.instances[inst_index].communication.is_empty() == false {
                            execute_trees = false;
                        }
                    }

                    if execute_trees {
                        // Execute trees of an NPC

                        // Has a locked tree ?
                        if let Some(locked_tree) = self.instances[inst_index].locked_tree {
                                self.execute_node(inst_index, locked_tree, None);
                        } else {
                            // Unlocked, execute all valid trees
                            let trees = self.instances[inst_index].tree_ids.clone();
                            for node_id in &trees {

                                // Only execute trees here with an "Always" execute setting (0)
                                if let Some(value)= get_node_value((self.instances[inst_index].behavior_id, *node_id, "execute"), self, BehaviorType::Behaviors) {
                                    if let Some(value) = value.to_integer() {
                                        if value != 0 {
                                            continue;
                                        }
                                    }
                                }
                                self.execute_node(inst_index, node_id.clone(), None);
                            }
                        }
                    }
                } else {
                    // Execute the tree which matches the current action

                    let mut tree_id: Option<Uuid> = None;
                    if let Some(action) = &self.instances[inst_index].action {
                        // DEBUG INCOMING ACTION
                        // println!("{:?}", self.instances[inst_index].action);
                        if action.direction != PlayerDirection::None {

                            // A directed action ( Move / Look - North etc)

                            for id in &self.instances[inst_index].tree_ids {
                                if let Some(behavior) = self.get_behavior(self.instances[inst_index].behavior_id, BehaviorType::Behaviors) {
                                    if let Some(node) = behavior.nodes.get(&id) {
                                        if node.name == action.action {
                                            tree_id = Some(*id);
                                            break;
                                        }
                                    }
                                }
                            }

                            if let Some(tree_id) = tree_id {
                                self.execute_node(inst_index, tree_id, None);
                            } else {
                                println!("Cannot find valid tree for directed action {}", action.action);
                            }
                        } else
                        if let Some(inventory_index) = &action.inventory_index {

                            // An action on an inventory item index

                            let index = *inventory_index as usize;
                            let mut item_id = None;
                            let mut scope_buffer : Option<ScopeBuffer> = None;
                            if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
                                if let Some(inv) = mess.read_lock::<Inventory>() {

                                    if index < inv.items.len() {
                                        item_id = Some(inv.items[index].id);
                                        if inv.items[index].state.is_some() {
                                            scope_buffer = inv.items[index].state.clone();
                                        }
                                    }
                                }
                            }

                            let mut to_execute = vec![];

                            if let Some(item_id) = item_id {
                                if let Some(item_behavior) = self.get_behavior(item_id, BehaviorType::Items) {
                                    for (id, node) in &item_behavior.nodes {
                                        if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                            if node.name == action.action {
                                                to_execute.push((item_behavior.id, *id));
                                            }
                                        }
                                    }
                                }
                            }

                            if to_execute.is_empty() == false {
                                // Execute the item actions
                                for (behavior_id, node_id) in to_execute {
                                    if let Some(scope_buffer) = &scope_buffer {
                                        // Move the item scope in / out
                                        self.curr_player_scope = self.scopes[inst_index].clone();
                                        let mut scope = Scope::new();
                                        scope_buffer.write_to_scope(&mut scope);
                                        self.scopes[inst_index] = scope;

                                        self.curr_inventory_index = Some(index);
                                        self.execute_item_node(inst_index, behavior_id, node_id);
                                        self.curr_inventory_index = None;

                                        let mut new_buffer = ScopeBuffer::new();
                                        new_buffer.read_from_scope(&self.scopes[inst_index]);

                                        self.scopes[inst_index] = self.curr_player_scope.clone();
                                        if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
                                            if let Some(mut inv) = mess.write_lock::<Inventory>() {
                                                inv.items[index].state = Some(new_buffer);
                                            }
                                        }
                                    } else {
                                        self.execute_item_node(inst_index, behavior_id, node_id);
                                    }
                                }
                            } else {
                                // If we cannot find the tree on the item, look for it on the player
                                for id in &self.instances[inst_index].tree_ids {
                                    if let Some(behavior) = self.get_behavior(self.instances[inst_index].behavior_id, BehaviorType::Behaviors) {
                                        if let Some(node) = behavior.nodes.get(&id) {
                                            if node.name == action.action {
                                                tree_id = Some(*id);
                                                break;
                                            }
                                        }
                                    }
                                }

                                if let Some(tree_id) = tree_id {
                                    self.execute_node(inst_index, tree_id, None);
                                } else {
                                    println!("Cannot find valid tree for directed action {}", action.action);
                                }
                            }
                        } else
                        if let Some(uuid) = &action.multi_choice_uuid {
                            // Multi Choice Answer
                            if self.instances[inst_index].communication.is_empty() == false {

                                let npc_index = self.instances[inst_index].communication[0].npc_index;
                                let behavior_id = self.instances[inst_index].communication[0].npc_behavior_tree;

                                self.instances[inst_index].multi_choice_answer = Some(*uuid);
                                self.curr_redirected_inst_index = Some(npc_index);
                                self.execute_node(npc_index, behavior_id, Some(inst_index));
                                self.instances[inst_index].multi_choice_answer = None;
                                self.curr_redirected_inst_index = None;

                                //self.instances[inst_index].communication = vec![];

                                // Drop comm for the NPC
                                /*
                                let mut com_to_drop : Option<usize> = None;
                                for c_index in 0..self.instances[npc_index].communication.len() {
                                    if self.instances[npc_index].communication[c_index].player_index == inst_index {
                                        // Drop this communication for the NPC
                                        com_to_drop = Some(c_index);
                                        break;
                                    }
                                }

                                if let Some(index) = com_to_drop {
                                    self.instances[npc_index].communication.remove(index);
                                }*/
                            }
                        }

                        self.instances[inst_index].action = None;
                    }
                    // Characters do not lock on targets
                    self.instances[inst_index].target_instance_index = None;
                }
            }

            // Extract the script messages for this instance
            if let Some(mess) = self.scopes[inst_index].get_mut("messages") {
                if let Some(mut message) = mess.write_lock::<ScriptMessageCmd>() {
                    if message.messages.is_empty() == false {
                        let my_name = self.instances[inst_index].name.clone();
                        for m in &message.messages {
                            match m {
                                ScriptMessage::Status(value) => {
                                    self.instances[inst_index].messages.push( MessageData {
                                        message_type        : MessageType::Status,
                                        message             : value.clone(),
                                        from                : my_name.clone(),
                                        right               : None,
                                        center              : None,
                                        buffer              : None,
                                    })
                                },
                                ScriptMessage::Debug(value) => {
                                    self.instances[inst_index].messages.push( MessageData {
                                        message_type        : MessageType::Debug,
                                        message             : value.clone(),
                                        from                : my_name.clone(),
                                        right               : None,
                                        center              : None,
                                        buffer              : None,
                                    })
                                },
                                ScriptMessage::Error(value) => {
                                    self.instances[inst_index].messages.push( MessageData {
                                        message_type        : MessageType::Error,
                                        message             : value.clone(),
                                        from                : my_name.clone(),
                                        right               : None,
                                        center              : None,
                                        buffer              : None,
                                    })
                                }
                            }
                        }
                    }
                    message.clear();
                }
            }

            // Inventory Actions

            let mut to_add = vec![];
            let mut to_equip = vec![];
            let mut to_equip_queued = vec![];

            // Check if we have to add items to the inventory and clone it for sending to the client
            if let Some(i) = self.scopes[inst_index].get_mut("inventory") {
                if let Some(mut inv) = i.write_lock::<Inventory>() {

                    // Add items
                    if inv.items_to_add.is_empty() == false {
                        let items_to_add = inv.items_to_add.clone();
                        for data in &items_to_add {
                            for (_id, behavior) in &mut self.items {

                                let mut added = false;

                                for item in &mut inv.items {
                                    if item.name == *data.0 {
                                        item.amount += data.1 as i32;
                                        added = true;
                                        break;
                                    }
                                }

                                if added == false {
                                    let mut tile_data : Option<TileData> = None;
                                    let mut sink : Option<PropertySink> = None;

                                    // Get the default tile for the item
                                    for (_index, node) in &behavior.nodes {
                                        if node.behavior_type == BehaviorNodeType::BehaviorType {
                                            if let Some(value) = node.values.get(&"tile".to_string()) {
                                                tile_data = value.to_tile_data();
                                            }
                                            if let Some(value) = node.values.get(&"settings".to_string()) {
                                                if let Some(str) = value.to_string() {
                                                    let mut s = PropertySink::new();
                                                    s.load_from_string(str.clone());
                                                    sink = Some(s);
                                                }
                                            }
                                        }
                                    }

                                    if behavior.name == *data.0 {
                                        let mut item = InventoryItem {
                                            id          : behavior.id,
                                            name        : behavior.name.clone(),
                                            item_type   : "gear".to_string(),
                                            tile        : tile_data,
                                            state       : None,
                                            light       : None,
                                            slot        : None,
                                            amount      : data.1 as i32,
                                            stackable   : 1,
                                            static_item : false,
                                            price       : 0.0,
                                            weight      : 0.0,
                                        };

                                        // Add state ?

                                        let mut states_to_execute = vec![];

                                        if let Some(sink) = sink {
                                            if let Some(state) = sink.get("state") {
                                                if let Some(value) = state.as_bool() {
                                                    if value == true {
                                                        item.state = Some(ScopeBuffer::new());
                                                        for (node_id, node) in &behavior.nodes {
                                                            if node.behavior_type == BehaviorNodeType::BehaviorTree {
                                                                for (value_name, value) in &node.values {
                                                                    if *value_name == "execute".to_string() {
                                                                        if let Some(v) = value.to_integer() {
                                                                            if v == 1 {
                                                                                // Startup only tree
                                                                                states_to_execute.push((behavior.id, *node_id));
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(static_item) = sink.get("static") {
                                                if let Some(st) = static_item.as_bool() {
                                                    item.static_item = st;
                                                }
                                            }
                                            if let Some(stackable_item) = sink.get("stackable") {
                                                if let Some(st) = stackable_item.as_int() {
                                                    if st >= 0 {
                                                        item.stackable = st;
                                                    }
                                                }
                                            }
                                            if let Some(price_item) = sink.get("price") {
                                                let price = price_item.to_float();
                                                if price >= 0.0 {
                                                    item.price = price;
                                                }
                                            }
                                            if let Some(weight_item) = sink.get("weight") {
                                                let weight = weight_item.to_float();
                                                if weight >= 0.0 {
                                                    item.weight = weight;
                                                }
                                            }
                                            if let Some(item_type) = sink.get("item_type") {
                                                if let Some(i_type) = item_type.as_string() {
                                                    item.item_type = i_type;
                                                }
                                            }
                                            if let Some(item_slot) = sink.get("slot") {
                                                if let Some(slot) = item_slot.as_string() {
                                                    item.slot = Some(slot);
                                                }
                                            }
                                        }

                                        to_add.push((item, states_to_execute));
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                        inv.items_to_add = vec![];
                    }

                    // Equip an item ?
                    if inv.items_to_equip.is_empty() == false {
                        for index in 0..inv.items_to_equip.len() {
                            let name = inv.items_to_equip[index].clone();
                            // Item is in the inventory ?
                            let removed_item = inv.remove_item_by_name(name.clone());
                            if let Some(item) = removed_item {
                                to_equip.push(item);
                            } else {
                                // Not in the inventory, has to be queued
                                to_equip_queued.push(name);
                            }
                        }
                        inv.items_to_equip = vec![];
                    }
                }
            }

            // Add new items
            for (mut item, states_to_execute) in to_add {
                for (item_id, node_id) in states_to_execute {
                    let curr_scope = self.scopes[inst_index].clone();
                    self.scopes[inst_index] = Scope::new();
                    self.execute_item_node(inst_index, item_id, node_id);
                    let scope = self.scopes[inst_index].clone();
                    self.scopes[inst_index] = curr_scope;
                    let mut buffer = ScopeBuffer::new();
                    buffer.read_from_scope(&scope);
                    item.state = Some(buffer);
                }
                if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
                    if let Some(mut inv) = mess.write_lock::<Inventory>() {
                        // Test if the item is queued to be equipped
                        if let Some(queued_index) = to_equip_queued.iter().position(|name| *name == item.name) {
                            to_equip_queued.remove(queued_index);
                            to_equip.push(item);
                        } else {
                            inv.add_item(item);
                        }
                    }
                }
            }

            // Equip items
            let mut to_add_back_to_inventory: Vec<InventoryItem> = vec![];
            for item in to_equip {
                let item_type = item.item_type.clone().to_lowercase();
                if let Some(slot) = item.slot.clone() {
                    if item_type == "weapon" {
                        if let Some(mess) = self.scopes[inst_index].get_mut("weapons") {
                            if let Some(mut weapons) = mess.write_lock::<Weapons>() {
                                // Remove existing item in the slot
                                if let Some(w) = weapons.slots.remove(&slot) {
                                    to_add_back_to_inventory.push(w);
                                }
                                // Insert the new weapon into the slot
                                weapons.slots.insert(slot, item);
                            }
                        }
                    } else
                    if item_type == "gear" {
                        if let Some(mess) = self.scopes[inst_index].get_mut("gear") {
                            if let Some(mut gear) = mess.write_lock::<Gear>() {
                                // Remove existing item in the slot
                                if let Some(g) = gear.slots.remove(&slot) {
                                    to_add_back_to_inventory.push(g);
                                }
                                // Insert the new gear into the slot
                                gear.slots.insert(slot, item);
                            }
                        }
                    }
                }
            }

            // Add removed items in the equipped slot(s) back into the inventory
            if to_add_back_to_inventory.is_empty() == false {
                if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
                    if let Some(mut inv) = mess.write_lock::<Inventory>() {
                        for item in to_add_back_to_inventory {
                            inv.items.push(item);
                        }
                    }
                }
            }

            // If we are debugging this instance, send the debug data
            if Some(self.instances[inst_index].behavior_id) == self.debug_behavior_id {
                let debug = BehaviorDebugData {
                    executed_connections    : self.executed_connections.clone(),
                    script_errors           : self.script_errors.clone(),
                };
                messages.push(Message::DebugData(debug));
            }

            // Add to the characters

            if let Some(position) = self.instances[inst_index].position.clone() {
                if let Some(tile) = self.instances[inst_index].tile.clone() {
                    let character = CharacterData {
                        position                : position.clone(),
                        old_position            : self.instances[inst_index].old_position.clone(),
                        max_transition_time     : self.instances[inst_index].max_transition_time,
                        curr_transition_time    : self.instances[inst_index].curr_transition_time,
                        tile,
                        name                    : self.instances[inst_index].name.clone(),
                        id                      : self.instances[inst_index].id,
                        index                   : inst_index,
                        effects                 : self.instances[inst_index].effects.clone(),
                     };
                     if let Some(list) = self.characters.get_mut(&position.region) {
                         list.push(character);
                     } else {
                         self.characters.insert(position.region, vec![character]);
                     }
                }
                self.instances[inst_index].effects = vec![];
            }

            // Check the inventory for lights
            if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
                if let Some(inv) = mess.read_lock::<Inventory>() {
                    for item in &inv.items {
                        if let Some(light) = &item.light {
                            let mut l = light.clone();
                            if let Some(position) = &self.instances[inst_index].position {
                                l.position = (position.x, position.y);
                            }
                            self.lights.push(l);
                        }
                    }
                }
            }
        }

        // Parse the loot and add the lights
        for (_position, loot) in &self.loot {
            for item in loot {
                if let Some(light) = &item.light {
                    self.lights.push(light.clone());
                }
            }
        }

        // Execute region area behaviors
        let mut to_execute: Vec<(usize, Uuid)> = vec![];
        self.displacements = HashMap::new();
        for area_index in 0..self.region_data.areas.len() {
            for (node_id, node) in &self.region_behavior[area_index].nodes {
                if node.behavior_type == BehaviorNodeType::InsideArea || node.behavior_type == BehaviorNodeType::EnterArea || node.behavior_type == BehaviorNodeType::LeaveArea || node.behavior_type == BehaviorNodeType::Always {
                    to_execute.push((area_index, *node_id));
                }
            }
        }

        for pairs in to_execute {
            self.execute_area_node(self.region_data.id, pairs.0, pairs.1);
        }

        // Parse the player characters and generate updates

        for inst_index in 0..self.instances.len() {

            let mut inventory = Inventory::new();
            let mut gear = Gear::new();
            let mut weapons = Weapons::new();
            let mut skills = Skills::new();
            let mut experience = Experience::new();

            // Clone the inventory for sending it to the client
            if let Some(i) = self.scopes[inst_index].get("inventory") {
                if let Some(inv) = i.read_lock::<Inventory>() {
                    inventory = inv.clone();
                }
            }

            // Clone the gear for sending it to the client
            if let Some(g) = self.scopes[inst_index].get("gear") {
                if let Some(ge) = g.read_lock::<Gear>() {
                    gear = ge.clone();
                }
            }

            // Clone the weapons for sending it to the client
            if let Some(w) = self.scopes[inst_index].get("weapons") {
                if let Some(weap) = w.read_lock::<Weapons>() {
                    weapons = weap.clone();
                }
            }

            // Clone the skills for sending it to the client
            if let Some(s) = self.scopes[inst_index].get("skills") {
                if let Some(sk) = s.read_lock::<Skills>() {
                    skills = sk.clone();
                }
            }

            // Clone the experience for sending it to the client
            if let Some(s) = self.scopes[inst_index].get("experience") {
                if let Some(exp) = s.read_lock::<Experience>() {
                    experience = exp.clone();
                }
            }

            // Purge invalid target indices
            if let Some(target_index) = self.instances[inst_index].target_instance_index {
                if self.instances[target_index].state != BehaviorInstanceState::Normal {
                    self.instances[inst_index].target_instance_index = None;
                }
            }

            let mut send_update = false;

            // Send update if this is a player and no editor debugging
            if self.instances[inst_index].instance_type == BehaviorInstanceType::Player && self.debug_behavior_id.is_none() {
                send_update = true;
            } else
            // Otherwise send this update if this is the current character being debugged in the editor
            if Some(self.instances[inst_index].behavior_id) == self.debug_behavior_id {
                send_update = true;
            }

            if send_update {

                if self.instances[inst_index].state == BehaviorInstanceState::Purged {
                    continue;
                }

                // Set the player index
                self.curr_player_inst_index = inst_index;
                let old_screen_id = self.instances[inst_index].curr_player_screen_id;

                let mut screen : Option<String> = None;
                let mut widgets : Vec<String> = vec![];

                // Execute the game behavior
                if let Some(game_inst_index) = self.game_instance_index {
                    if self.scopes.is_empty() == false {
                        if let Some(locked_tree) = self.instances[game_inst_index].locked_tree {
                            self.execute_game_node(game_inst_index, locked_tree);
                        }
                    }
                }

                // Check if we need to send a new screen

                if let Some(new_screen_id) = &self.instances[inst_index].curr_player_screen_id {
                    if let Some(old_screen_id) = &old_screen_id {
                        if new_screen_id != old_screen_id {
                            screen = Some(self.instances[inst_index].curr_player_screen.clone());
                            widgets = self.instances[inst_index].curr_player_widgets.clone();
                        }
                    } else {
                        screen = Some(self.instances[inst_index].curr_player_screen.clone());
                        widgets = self.instances[inst_index].curr_player_widgets.clone();
                    }
                }

                let mut region        : Option<GameRegionData> = None;
                let mut characters    : Vec<CharacterData> = vec![];
                let mut displacements : HashMap<(isize, isize), TileData> = HashMap::new();
                let mut scope_buffer = ScopeBuffer::new();

                let mut needs_transfer_to: Option<Uuid> = None;
                if let Some(position) = self.instances[inst_index].position.clone() {

                    if position.region != self.region_data.id {
                        // We need to transfer the character to a new region
                        needs_transfer_to = Some(position.region);
                    } else
                    // Check if the character is in a region we did not send to the client yet OR if the editor is debugging
                    if self.instances[inst_index].regions_send.contains(&position.region) == false || self.debug_behavior_id.is_some() {
                        region = Some(self.region_data.clone());
                        self.instances[inst_index].regions_send.insert(position.region);
                    }
                    // Copy the displacements
                    displacements = self.displacements.clone();

                    // Send the characters of the client region
                    if let Some(chars) = self.characters.get(&position.region) {
                        characters = chars.clone();
                    }

                    scope_buffer.read_from_scope(&self.scopes[inst_index]);
                }

                let update = GameUpdate{
                    id                      : self.instances[inst_index].id,
                    screen_size             : self.screen_size,
                    def_square_tile_size    : self.def_square_tile_size,
                    position                : self.instances[inst_index].position.clone(),
                    old_position            : self.instances[inst_index].old_position.clone(),
                    max_transition_time     : self.instances[inst_index].max_transition_time,
                    curr_transition_time    : self.instances[inst_index].curr_transition_time,
                    tile                    : self.instances[inst_index].tile.clone(),
                    screen,
                    widgets,
                    region,
                    lights                  : self.lights.clone(),
                    displacements,
                    characters,
                    loot                    : self.loot.clone(),
                    messages                : self.instances[inst_index].messages.clone(),
                    audio                   : self.instances[inst_index].audio.clone(),
                    scope_buffer            : scope_buffer,
                    inventory               : inventory.clone(),
                    gear                    : gear.clone(),
                    weapons                 : weapons.clone(),
                    skills                  : skills.clone(),
                    experience              : experience.clone(),
                    multi_choice_data       : self.instances[inst_index].multi_choice_data.clone(),
                    communication           : self.instances[inst_index].communication.clone(),
                 };

                self.instances[inst_index].messages = vec![];

                if let Some(transfer_to) = needs_transfer_to {
                    // Serialize character
                    self.serialize_character_instance(inst_index);
                    messages.push(Message::TransferCharacter(transfer_to, self.instances[inst_index].clone()));
                    self.purge_instance(inst_index);
                }
                messages.push(Message::PlayerUpdate(update.id, update));
            }
        }

        //println!("tick time {}", self.get_time() - tick_time);

        self.tick_count = self.tick_count.wrapping_add(1);

        messages
    }

    /// Executes the given node and follows the connection chain
    pub fn execute_node(&mut self, instance_index: usize, node_id: Uuid, redirection: Option<usize>) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<Uuid> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, Uuid, BehaviorNodeConnector)> = vec![];

        let mut is_sequence = false;
        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(behavior) = self.behaviors.get_mut(&self.instances[instance_index].behavior_id) {
            if let Some(node) = behavior.nodes.get_mut(&node_id) {

                // Handle special nodes
                if node.behavior_type == BehaviorNodeType::BehaviorTree || node.behavior_type == BehaviorNodeType::Linear {

                    if node.behavior_type == BehaviorNodeType::BehaviorTree {
                        self.curr_executing_tree = node.id;
                    }

                    connectors.push(BehaviorNodeConnector::Bottom1);
                    connectors.push(BehaviorNodeConnector::Bottom2);
                    connectors.push(BehaviorNodeConnector::Bottom);
                    connectors.push(BehaviorNodeConnector::Bottom3);
                    connectors.push(BehaviorNodeConnector::Bottom4);
                } else
                if node.behavior_type == BehaviorNodeType::Sequence {
                    connectors.push(BehaviorNodeConnector::Bottom1);
                    connectors.push(BehaviorNodeConnector::Bottom2);
                    connectors.push(BehaviorNodeConnector::Bottom);
                    connectors.push(BehaviorNodeConnector::Bottom3);
                    connectors.push(BehaviorNodeConnector::Bottom4);
                    is_sequence = true;
                } else {
                    if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                        let behavior_id = self.instances[instance_index].behavior_id.clone();
                        let idx = if redirection.is_some() { redirection.unwrap() } else { instance_index};
                        let connector = node_call(idx, (behavior_id, node_id), self, BehaviorType::Behaviors);
                        rc = Some(connector);
                        connectors.push(connector);
                    } else {
                        connectors.push(BehaviorNodeConnector::Bottom);
                    }
                }
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        for connector in connectors {
            if let Some(behavior) = self.behaviors.get_mut(&self.instances[instance_index].behavior_id) {

                for c in &behavior.connections {
                    if c.0 == node_id && c.1 == connector {
                        connected_node_ids.push(c.2);
                        if is_sequence == false {
                            self.executed_connections.push((BehaviorType::Behaviors, c.0, c.1));
                        } else {
                            possibly_executed_connections.push((BehaviorType::Behaviors, c.0, c.1));
                        }
                    }
                }
            }
        }

        // And if yes execute it
        for (index, connected_node_id) in connected_node_ids.iter().enumerate() {

            // If this is a sequence, mark this connection as executed
            if is_sequence {
                self.executed_connections.push(possibly_executed_connections[index]);
            }

            if let Some(connector) = self.execute_node(instance_index, *connected_node_id, redirection) {
                if is_sequence {
                    // Inside a sequence break out if the connector is not Success
                    if connector == BehaviorNodeConnector::Fail || connector == BehaviorNodeConnector::Right {
                        break;
                    }
                }
            }
        }
        rc
    }

    /// Executes the given systems node and follows the connection chain
    pub fn execute_systems_node(&mut self, instance_index: usize, node_id: Uuid) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<Uuid> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, Uuid, BehaviorNodeConnector)> = vec![];

        let mut is_sequence = false;
        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(system) = self.systems.get_mut(&self.instances[instance_index].systems_id) {
            if let Some(node) = system.nodes.get_mut(&node_id) {

                // Handle special nodes
                if node.behavior_type == BehaviorNodeType::BehaviorTree || node.behavior_type == BehaviorNodeType::Linear {
                    connectors.push(BehaviorNodeConnector::Bottom1);
                    connectors.push(BehaviorNodeConnector::Bottom2);
                    connectors.push(BehaviorNodeConnector::Bottom);
                } else
                if node.behavior_type == BehaviorNodeType::Sequence {
                    connectors.push(BehaviorNodeConnector::Bottom1);
                    connectors.push(BehaviorNodeConnector::Bottom2);
                    connectors.push(BehaviorNodeConnector::Bottom);
                    connectors.push(BehaviorNodeConnector::Bottom3);
                    connectors.push(BehaviorNodeConnector::Bottom4);
                    is_sequence = true;
                } else {
                    if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                        let systems_id = self.instances[instance_index].systems_id.clone();
                        let connector = node_call(instance_index, (systems_id, node_id), self, BehaviorType::Systems);
                        rc = Some(connector);
                        connectors.push(connector);
                    } else {
                        connectors.push(BehaviorNodeConnector::Bottom);
                    }
                }
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        for connector in connectors {
            if let Some(system) = self.systems.get_mut(&self.instances[instance_index].systems_id) {

                for c in &system.connections {
                    if c.0 == node_id && c.1 == connector {
                        connected_node_ids.push(c.2);
                        if is_sequence == false {
                            self.executed_connections.push((BehaviorType::Systems, c.0, c.1));
                        } else {
                            possibly_executed_connections.push((BehaviorType::Systems, c.0, c.1));
                        }
                    }
                }
            }
        }

        // And if yes execute it
        for (index, connected_node_id) in connected_node_ids.iter().enumerate() {

            // If this is a sequence, mark this connection as executed
            if is_sequence {
                self.executed_connections.push(possibly_executed_connections[index]);
            }

            if let Some(connector) = self.execute_systems_node(instance_index, *connected_node_id) {
                if is_sequence {
                    // Inside a sequence break out if the connector is not Success
                    if connector == BehaviorNodeConnector::Fail || connector == BehaviorNodeConnector::Right {
                        break;
                    }
                }
            }
        }
        rc
    }

    /// Executes the given item node and follows the connection chain
    pub fn execute_item_node(&mut self, instance_index: usize, item_id: Uuid, node_id: Uuid) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<Uuid> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, Uuid, BehaviorNodeConnector)> = vec![];

        let mut is_sequence = false;
        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(item) = self.items.get_mut(&item_id) {
            if let Some(node) = item.nodes.get_mut(&node_id) {

                // Handle special nodes
                if node.behavior_type == BehaviorNodeType::BehaviorTree || node.behavior_type == BehaviorNodeType::Linear {
                    connectors.push(BehaviorNodeConnector::Bottom1);
                    connectors.push(BehaviorNodeConnector::Bottom2);
                    connectors.push(BehaviorNodeConnector::Bottom);
                } else
                if node.behavior_type == BehaviorNodeType::Sequence {
                    connectors.push(BehaviorNodeConnector::Bottom1);
                    connectors.push(BehaviorNodeConnector::Bottom2);
                    connectors.push(BehaviorNodeConnector::Bottom);
                    connectors.push(BehaviorNodeConnector::Bottom3);
                    connectors.push(BehaviorNodeConnector::Bottom4);
                    is_sequence = true;
                } else {
                    if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                        let item_id = item_id;
                        let connector = node_call(instance_index, (item_id, node_id), self, BehaviorType::Items);
                        rc = Some(connector);
                        connectors.push(connector);
                    } else {
                        connectors.push(BehaviorNodeConnector::Bottom);
                    }
                }
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        for connector in connectors {
            if let Some(item) = self.items.get_mut(&item_id) {

                for c in &item.connections {
                    if c.0 == node_id && c.1 == connector {
                        connected_node_ids.push(c.2);
                        if is_sequence == false {
                            self.executed_connections.push((BehaviorType::Items, c.0, c.1));
                        } else {
                            possibly_executed_connections.push((BehaviorType::Items, c.0, c.1));
                        }
                    }
                }
            }
        }

        // And if yes execute it
        for (index, connected_node_id) in connected_node_ids.iter().enumerate() {

            // If this is a sequence, mark this connection as executed
            if is_sequence {
                self.executed_connections.push(possibly_executed_connections[index]);
            }

            if let Some(connector) = self.execute_item_node(instance_index, item_id, *connected_node_id) {
                if is_sequence {
                    // Inside a sequence break out if the connector is not Success
                    if connector == BehaviorNodeConnector::Fail || connector == BehaviorNodeConnector::Right {
                        break;
                    }

                }
            }
        }
        rc
    }

    /// Executes the given node and follows the connection chain
    pub fn execute_area_node(&mut self, region_id: Uuid, area_index: usize, node_id: Uuid) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<Uuid> = vec![];

        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(node) = self.region_behavior[area_index].nodes.get_mut(&node_id) {

            if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                let connector = node_call(area_index, (region_id, node_id), self, BehaviorType::Regions);
                rc = Some(connector);
                connectors.push(connector);
            } else {
                connectors.push(BehaviorNodeConnector::Bottom);
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        for connector in connectors {
            for c in &self.region_behavior[area_index].connections {
                if c.0 == node_id && c.1 == connector {
                    connected_node_ids.push(c.2);
                    self.executed_connections.push((BehaviorType::Regions, c.0, c.1));
                }
            }
        }

        // And if yes execute it
        for (_index, connected_node_id) in connected_node_ids.iter().enumerate() {
            self.execute_area_node(region_id, area_index, *connected_node_id);
        }
        rc
    }

    /// Executes the given node and follows the connection chain
    fn execute_game_node(&mut self, instance_index: usize, node_id: Uuid) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<Uuid> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, Uuid, BehaviorNodeConnector)> = vec![];

        let mut is_sequence = false;
        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        let behavior = &mut self.game_data;
        if let Some(node) = behavior.nodes.get_mut(&node_id) {
            // Handle special nodes
            if node.behavior_type == BehaviorNodeType::Screen{
                connectors.push(BehaviorNodeConnector::Bottom1);
                connectors.push(BehaviorNodeConnector::Bottom2);
                connectors.push(BehaviorNodeConnector::Bottom);
                connectors.push(BehaviorNodeConnector::Bottom3);
                connectors.push(BehaviorNodeConnector::Bottom4);

                if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                    let behavior_id = self.instances[instance_index].behavior_id.clone();
                    _ = node_call(instance_index, (behavior_id, node_id), self, BehaviorType::GameLogic);
                }
            } else
            if node.behavior_type == BehaviorNodeType::BehaviorTree || node.behavior_type == BehaviorNodeType::Linear {
                connectors.push(BehaviorNodeConnector::Bottom1);
                connectors.push(BehaviorNodeConnector::Bottom2);
                connectors.push(BehaviorNodeConnector::Bottom);
                connectors.push(BehaviorNodeConnector::Bottom3);
                connectors.push(BehaviorNodeConnector::Bottom4);
            } else
            if node.behavior_type == BehaviorNodeType::Sequence {
                connectors.push(BehaviorNodeConnector::Bottom1);
                connectors.push(BehaviorNodeConnector::Bottom2);
                connectors.push(BehaviorNodeConnector::Bottom);
                connectors.push(BehaviorNodeConnector::Bottom3);
                connectors.push(BehaviorNodeConnector::Bottom4);
                is_sequence = true;
            } else {
                if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                    let behavior_id = self.instances[instance_index].behavior_id.clone();
                    let connector = node_call(instance_index, (behavior_id, node_id), self, BehaviorType::GameLogic);
                    rc = Some(connector);
                    connectors.push(connector);
                } else {
                    connectors.push(BehaviorNodeConnector::Bottom);
                }
            }
        }

        // Search the connections to check if we can find an ongoing node connection
        for connector in connectors {
            let behavior = &mut self.game_data;
            for c in &behavior.connections {
                if c.0 == node_id && c.1 == connector {
                    connected_node_ids.push(c.2);
                    if is_sequence == false {
                        self.executed_connections.push((BehaviorType::GameLogic, c.0, c.1));
                    } else {
                        possibly_executed_connections.push((BehaviorType::GameLogic, c.0, c.1));
                    }
                }
            }
        }

        // And if yes execute it
        for (index, connected_node_id) in connected_node_ids.iter().enumerate() {

            // If this is a sequence, mark this connection as executed
            if is_sequence {
                self.executed_connections.push(possibly_executed_connections[index]);
            }

            if let Some(connector) = self.execute_game_node(instance_index, *connected_node_id) {
                if is_sequence {
                    // Inside a sequence break out if the connector is not Success
                    if connector == BehaviorNodeConnector::Fail || connector == BehaviorNodeConnector::Right {
                        break;
                    }
                }
            }
        }
        rc
    }

    /// Setup the region instance data by decoding the JSON for all game elements and sets up the npc and game behavior instances.
    pub fn setup(&mut self, region: String, region_behavior: HashMap<Uuid, Vec<String>>, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, game: String) {
        // Decode all JSON
        if let Some(region_data) = serde_json::from_str(&region).ok() {

            self.region_data = region_data;

            if let Some(property) = self.region_data.settings.get("movement") {
                if let Some(value) = property.as_string() {
                    if value.to_lowercase() == "tile" {
                        self.pixel_based_movement = false;
                    }
                }
            }

            if let Some(areas) = region_behavior.get(&self.region_data.id) {
                for a in areas {
                    if let Some(ab) = serde_json::from_str::<GameBehaviorData>(&a).ok() {
                        self.region_behavior.push(ab);
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
                        if instance.position.region != self.region_data.id { continue; }
                        let mut loot = LootData {
                            id          : behavior_data.id.clone(),
                            item_type   : "gear".to_string(),
                            name        : Some(behavior_data.name.clone()),
                            tile        : None,
                            state       : None,
                            light       : None,
                            slot        : None,
                            amount      : instance.amount,
                            stackable   : 1,
                            static_item : false,
                            price       : 0.0,
                            weight      : 0.0,
                        };

                        for (_index, node) in &behavior_data.nodes {
                            if node.behavior_type == BehaviorNodeType::BehaviorType {
                                if let Some(value) = node.values.get(&"tile".to_string()) {
                                    loot.tile = value.to_tile_data();
                                }
                                if let Some(value) = node.values.get(&"settings".to_string()) {
                                    if let Some(str) = value.to_string() {
                                        let mut s = PropertySink::new();
                                        s.load_from_string(str.clone());
                                        if let Some(static_item) = s.get("static") {
                                            if let Some(st) = static_item.as_bool() {
                                                loot.static_item = st;
                                            }
                                        }
                                        if let Some(stackable_item) = s.get("stackable") {
                                            if let Some(st) = stackable_item.as_int() {
                                                if st >= 0 {
                                                    loot.stackable = st;
                                                }
                                            }
                                        }
                                        if let Some(price_item) = s.get("price") {
                                            let price = price_item.to_float();
                                            if price >= 0.0 {
                                                loot.price = price;
                                            }
                                        }
                                        if let Some(weight_item) = s.get("weight") {
                                            let weight = weight_item.to_float();
                                            if weight >= 0.0 {
                                                loot.weight = weight;
                                            }
                                        }
                                        if let Some(item_type) = s.get("item_type") {
                                            if let Some(i_type) = item_type.as_string() {
                                                loot.item_type = i_type;
                                            }
                                        }
                                        if let Some(item_slot) = s.get("slot") {
                                            if let Some(slot) = item_slot.as_string() {
                                                loot.slot = Some(slot);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(v) = self.loot.get_mut(&(instance.position.x, instance.position.y)) {
                            v.push(loot);
                        } else {
                            self.loot.insert((instance.position.x, instance.position.y), vec![loot]);
                        }
                    }
                }
                self.items.insert(behavior_data.id, behavior_data);
            }
        }
        if let Some(game_data) = serde_json::from_str(&game).ok() {
            self.game_data = game_data;

            // Update the game settings, just in case they don't contain the latest
            if self.game_data.settings.is_some() {
                crate::gamedata::game::update_game_sink(&mut self.game_data.settings.as_mut().unwrap());
            }

            // Read global game settings

            if let Some(settings) = &self.game_data.settings {
                if let Some(screen_size) = settings.get("screen_size") {
                    match screen_size.value {
                        PropertyValue::IntX(v) => {
                            self.screen_size = (v[0], v[1]);
                        },
                        _ => {}
                    }
                }
                if let Some(def_square_tile_size) = settings.get("def_square_tile_size") {
                    match def_square_tile_size.value {
                        PropertyValue::Int(v) => {
                            self.def_square_tile_size = v;
                        },
                        _ => {}
                    }
                }

                if let Some(property) = settings.get("primary_currency") {
                    if let Some(name) = property.as_string() {
                        self.primary_currency = name;
                    }
                } else {
                    self.primary_currency = "gold".to_string();
                }

                if let Some(property) = settings.get("hitpoints") {
                    if let Some(name) = property.as_string() {
                        self.hitpoints = name;
                    }
                } else {
                    self.hitpoints = "HP".to_string();
                }

                if let Some(property) = settings.get("max_hitpoints") {
                    if let Some(name) = property.as_string() {
                        self.max_hitpoints = name;
                    }
                } else {
                    self.max_hitpoints = "MAX_HP".to_string();
                }

                if let Some(property) = settings.get("gear_slots") {
                    if let Some(name) = property.as_string() {
                        let ar : Vec<&str> = name.split(",").collect();
                        for s in ar {
                            self.gear_slots.push(s.to_lowercase().trim().to_string());
                        }
                    }
                }

                if let Some(property) = settings.get("weapon_slots") {
                    if let Some(name) = property.as_string() {
                        let ar : Vec<&str> = name.split(",").collect();
                        for s in ar {
                            self.weapon_slots.push(s.to_lowercase().trim().to_string());
                        }
                    }
                }
            }
        }

        // Create all behavior instances of characters inside this region
        let ids : Vec<Uuid> = self.behaviors.keys().cloned().collect();
        for id in ids {
            self.create_behavior_instance(id, true, None);
        }

        // Create the game instance itself
        let mut to_execute : Vec<Uuid> = vec![];
        let mut startup_name : Option<String> = None;
        let mut locked_tree  : Option<Uuid> = None;
        let scope = rhai::Scope::new();
        let behavior = &mut self.game_data;

        // Collect name of the startup tree and the variables
        for (_id, node) in &behavior.nodes {
            if node.behavior_type == BehaviorNodeType::BehaviorType {
                if let Some(value )= node.values.get(&"startup".to_string()) {
                    startup_name = Some(value.to_string_value());
                }
            }
        }

        // Second pass parse the trees and find the startup tree
        for (id, node) in &behavior.nodes {
            if node.behavior_type == BehaviorNodeType::BehaviorTree {

                for c in &behavior.connections {
                    if c.0 == *id {
                        to_execute.push(c.0);
                        if let Some(startup) = startup_name.clone() {
                            if node.name == startup {
                                locked_tree = Some(node.id);
                            }
                        }
                    }
                }
            }
        }

        let index = self.instances.len();

        let instance = BehaviorInstance {id: Uuid::new_v4(), state: BehaviorInstanceState::Normal, name: behavior.name.clone(), behavior_id: behavior.id, tree_ids: to_execute.clone(), position: None, tile: None, target_instance_index: None, locked_tree, party: vec![], node_values: FxHashMap::default(), scope_buffer: None, sleep_cycles: 0, systems_id: Uuid::new_v4(), action: None, instance_type: BehaviorInstanceType::GameLogic, update: None, regions_send: std::collections::HashSet::new(), curr_player_screen_id: None, game_locked_tree: None, curr_player_screen: "".to_string(), curr_player_widgets: vec![], messages: vec![], audio: vec![], old_position: None, max_transition_time: 0, curr_transition_time: 0, alignment: 1, multi_choice_data: vec![], communication: vec![], multi_choice_answer: None, damage_to_be_dealt: None, inventory_buffer: None, weapons_buffer: None, gear_buffer: None, skills_buffer: None, experience_buffer: None, effects: vec![], healing_to_be_dealt: None, instance_creation_data: None };

        self.instances.push(instance);
        self.scopes.push(scope);

        for tree_id in &to_execute {
            // Execute this tree if it is a "Startup" Only tree
            if let Some(value)= self.get_game_node_value(*tree_id, "execute") {
                if let Some(value) = value.to_integer() {
                    if value == 1 {
                        self.execute_game_node(index, tree_id.clone());
                    }
                }
            }
        }
        self.game_instance_index = Some(index);

        // We iterate over all loot and initialize state if necessary

        let mut loot_map = self.loot.clone();

        for (pos, loot) in &mut loot_map {
            for index in 0..loot.len() {
                let mut item_behavior_id : Option<Uuid> = None;
                if let Some(behavior) = self.get_behavior(loot[index].id, BehaviorType::Items) {
                    item_behavior_id = Some(behavior.id);
                }

                if let Some(item_behavior_id) = item_behavior_id {
                    self.curr_loot_item = Some((pos.0, pos.1, index));
                    loot[index].state = check_and_create_item_state(0, item_behavior_id, self);
                    if let Some(l) = self.loot.get(&pos) {
                        // Copy light state back
                        loot[index].light = l[index].light.clone();
                    }
                    self.curr_loot_item = None;
                }
            }
        }
        self.loot = loot_map;
    }

    /// Creates a new player instance
    pub fn create_player_instance(&mut self, uuid: Uuid, position: Position) {
        let mut player_id : Option<Uuid> = None;
        for b in &self.behaviors {
            if b.1.name == "Player" {
                player_id = Some(*b.0);
            }
        }
        if let Some(player_id) = player_id {
            let index = self.create_behavior_instance(player_id, false, None);
            self.instances[index].instance_type = BehaviorInstanceType::Player;
            self.instances[index].id = uuid;
            self.instances[index].position = Some(position);
            self.player_uuid_indices.insert(uuid, index);
            log::info!("Player instance {} created.", uuid);
        }
    }

    /// Destroyes a player instance
    pub fn destroy_player_instance(&mut self, uuid: Uuid) {
        for inst_index in 0..self.instances.len() {
            if self.instances[inst_index].id == uuid {
                self.purge_instance(inst_index);
                break;
            }
        }
    }


    /// Creates an instance of a behavior (character)
    fn create_behavior_instance(&mut self, id: Uuid, npc_only: bool, data: Option<CharacterInstanceData>) -> usize {

        let mut index = 0;

        let mut startup_trees               : Vec<Uuid> = vec![];

        // Instances to create for this behavior
        if let Some(behavior) = self.behaviors.get_mut(&id) {

            if npc_only && behavior.name == "Player" {
                return index;
            }

            let mut to_create : Vec<CharacterInstanceData> = vec![];

            // Collect all the default data for the behavior from the nodes: Position, tile, behavior Trees and variables.
            let mut to_execute              : Vec<Uuid> = vec![];
            let mut default_position        : Option<Position> = None;
            let mut default_tile            : Option<TileId> = None;
            let mut default_alignment       : i32 = 1;
            let default_scope        = rhai::Scope::new();

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
                                } else
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
                } else
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value )= node.values.get(&"position".to_string()) {
                        default_position = value.to_position();
                    }
                    if let Some(value )= node.values.get(&"tile".to_string()) {
                        default_tile = value.to_tile_id()
                    }
                    if let Some(value )= node.values.get(&"alignment".to_string()) {
                        if let Some(alignment) = value.to_integer() {
                            default_alignment = 2 - alignment- 1;
                        }
                    }
                }
            }
            // Add main
            if default_position.is_some() && default_tile.is_some() && data.is_none() {
                let main = CharacterInstanceData {
                    position    : default_position.unwrap().clone(),
                    tile        : default_tile.clone(),
                    name        : Some(behavior.name.clone()),
                    alignment   : default_alignment
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
                // If we get the character instance data, only add this (respawn)
                to_create.push(data.unwrap());
            }
            // Now we have all instances of the behavior we need to create
            for inst in to_create {

                // Only create when instance is in this region
                if inst.position.region != self.region_data.id {
                    continue;
                }

                //println!("Creating instance {}", inst.name.unwrap());

                let instance = BehaviorInstance {id: uuid::Uuid::new_v4(), state: BehaviorInstanceState::Normal, name: behavior.name.clone(), behavior_id: behavior.id, tree_ids: to_execute.clone(), position: Some(inst.position.clone()), tile: inst.tile.clone(), target_instance_index: None, locked_tree: None, party: vec![], node_values: FxHashMap::default(), scope_buffer: None, sleep_cycles: 0, systems_id: Uuid::new_v4(), action: None, instance_type: BehaviorInstanceType::NonPlayerCharacter, update: None, regions_send: std::collections::HashSet::new(), curr_player_screen_id: None, game_locked_tree: None, curr_player_screen: "".to_string(), curr_player_widgets: vec![], messages: vec![], audio: vec![], old_position: None, max_transition_time: 0, curr_transition_time: 0, alignment: inst.alignment, multi_choice_data: vec![], communication: vec![], multi_choice_answer: None, damage_to_be_dealt: None, inventory_buffer: None, weapons_buffer: None, gear_buffer: None, skills_buffer: None, experience_buffer: None, effects: vec![], healing_to_be_dealt: None, instance_creation_data: Some(inst.clone()) };

                index = self.instances.len();
                self.instances.push(instance);

                // Create skills

                let mut skills = Skills::new();

                for (_id, behavior) in &self.systems {
                    if behavior.name.to_lowercase() == "skills" {
                        for (_id, node) in &behavior.nodes {
                            if node.behavior_type == BehaviorNodeType::SkillTree {
                                skills.add_skill(node.name.clone());

                                // Add the skill to the skill_tree

                                let mut rc : Vec<(i32, String, String)> = vec![];
                                let mut parent_id = node.id;

                                loop {
                                    let mut found = false;
                                    for (id1, c1, id2, c2) in &behavior.connections {
                                        if *id1 == parent_id && *c1 == BehaviorNodeConnector::Bottom {
                                            for (uuid, node) in &behavior.nodes {
                                                if *uuid == *id2 {
                                                    let mut start = 0;
                                                    if let Some(value) = node.values.get(&"start".to_string()) {
                                                        if let Some(i) = value.to_integer() {
                                                            start = i;
                                                        }
                                                    }
                                                    let mut message = "".to_string();
                                                    if let Some(value) = node.values.get(&"message".to_string()) {
                                                        if let Some(m) = value.to_string() {
                                                            message = m;
                                                        }
                                                    }

                                                    parent_id = node.id;
                                                    found = true;

                                                    rc.push((start, node.name.clone(), message));
                                                }
                                            }
                                        } else
                                        if *id2 == parent_id && *c2 == BehaviorNodeConnector::Bottom {
                                            for (uuid, node) in &behavior.nodes {
                                                if *uuid == *id1 {
                                                    let mut start = 0;
                                                    if let Some(value) = node.values.get(&"start".to_string()) {
                                                        if let Some(i) = value.to_integer() {
                                                            start = i;
                                                        }
                                                    }
                                                    let mut message = "".to_string();
                                                    if let Some(value) = node.values.get(&"message".to_string()) {
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

                                self.skill_trees.insert(node.name.clone(), rc);
                            }
                        }
                    }
                }

                // println!("{:?}", self.skill_trees);

                // Set the default values into the scope
                let mut scope = default_scope.clone();
                scope.set_value("name", behavior.name.clone());
                scope.set_value("alignment", inst.alignment as i32);
                scope.set_value("messages", ScriptMessageCmd::new());
                scope.set_value("inventory", Inventory::new());
                scope.set_value("gear", Gear::new());
                scope.set_value("weapons", Weapons::new());
                scope.set_value("skills", skills);
                scope.set_value("experience", Experience::new());

                self.scopes.push(scope);
            }
        }

        if index < self.instances.len() {
            // Execute the startup only trees
            for startup_id in &startup_trees {
                self.execute_node(index, startup_id.clone(), None);
            }
        }

        index
    }

    /// Returns a game node value
    fn get_game_node_value(&mut self, node_id: Uuid, node_property: &str) -> Option<Value> {
        if let Some(node) = self.game_data.nodes.get(&node_id) {
            if let Some(value) = node.values.get(node_property) {
                return Some(value.clone());
            }
        }
        None
    }

    /// Returns the layered tiles at the given position and checks for displacements
    pub fn get_tile_at(&self, pos: (isize, isize)) -> Vec<TileData> {
        let mut rc = vec![];
        if let Some(t) = self.displacements.get(&pos) {
            rc.push(t.clone());
        } else {
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
        }
        rc
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
            return Some(&self.game_data);
        }
        None
    }

    /// Gets the mutable behavior for the given behavior type
    pub fn get_mut_behavior(&mut self, id: Uuid, behavior_type: BehaviorType) -> Option<&mut GameBehaviorData> {
        if behavior_type == BehaviorType::Regions {
            for b in &mut self.region_behavior {
                if b.id == id {
                    return Some(b);
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
            return Some(&mut self.game_data);
        }
        None
    }

    /// Purges this instance, voiding it.
    pub fn purge_instance(&mut self, inst_index: usize) {
        self.instances[inst_index].state = BehaviorInstanceState::Purged;
        self.player_uuid_indices.remove(&self.instances[inst_index].id);
    }

    /// Transfers a character instance into this region
    pub fn transfer_character_into(&mut self, mut instance: BehaviorInstance) {
        // TODO, fill in purged
        self.player_uuid_indices.insert(instance.id, self.instances.len());

        let mut scope = rhai::Scope::new();
        self.deserialize_character_instance(&mut instance, &mut scope);

        self.instances.push(instance);
        self.scopes.push(scope);
    }

    /// Sets the debugging behavior id.
    pub fn set_debug_behavior_id(&mut self, behavior_id: Uuid) {
        self.debug_behavior_id = Some(behavior_id);
    }

    /// Serializes the given instance index
    fn serialize_character_instance(&mut self, inst_index: usize) {
        // Serialize character
        let mut scope_buffer = ScopeBuffer::new();
        scope_buffer.read_from_scope(&self.scopes[inst_index]);

        if let Some(mess) = self.scopes[inst_index].get_mut("inventory") {
            if let Some(inv) = mess.write_lock::<Inventory>() {

                let i = inv.clone();
                if let Some(json) = serde_json::to_string(&i).ok() {
                        self.instances[inst_index].inventory_buffer = Some(json);
                }
            }
        }

        if let Some(mess) = self.scopes[inst_index].get_mut("weapons") {
            if let Some(weap) = mess.write_lock::<Weapons>() {

                let w = weap.clone();
                if let Some(json) = serde_json::to_string(&w).ok() {
                        self.instances[inst_index].weapons_buffer = Some(json);
                }
            }
        }

        if let Some(mess) = self.scopes[inst_index].get_mut("gear") {
            if let Some(ge) = mess.write_lock::<Gear>() {

                let g = ge.clone();
                if let Some(json) = serde_json::to_string(&g).ok() {
                        self.instances[inst_index].gear_buffer = Some(json);
                }
            }
        }


        if let Some(mess) = self.scopes[inst_index].get_mut("skills") {
            if let Some(sk) = mess.write_lock::<Skills>() {

                let s = sk.clone();
                if let Some(json) = serde_json::to_string(&s).ok() {
                        self.instances[inst_index].skills_buffer = Some(json);
                }
            }
        }

        if let Some(mess) = self.scopes[inst_index].get_mut("experience") {
            if let Some(ex) = mess.write_lock::<Experience>() {

                let e = ex.clone();
                if let Some(json) = serde_json::to_string(&e).ok() {
                        self.instances[inst_index].experience_buffer = Some(json);
                }
            }
        }

        self.instances[inst_index].scope_buffer = Some(scope_buffer);
    }

    /// Deserializes the given instance
    fn deserialize_character_instance(&self, instance: &mut BehaviorInstance, mut scope: &mut Scope) {
        if let Some(buffer) = &instance.scope_buffer {
            buffer.write_to_scope(&mut scope);
        }

        scope.set_value("messages", ScriptMessageCmd::new());

        if let Some(inventory_buffer) = &instance.inventory_buffer {
            let inventory : Inventory = serde_json::from_str(&inventory_buffer)
                .unwrap_or(Inventory::new());
            scope.set_value("inventory", inventory);
        } else {
            // Should not happen
            scope.set_value("inventory", Inventory::new());
        }

        if let Some(weapons_buffer) = &instance.weapons_buffer {
            let weapons : Weapons = serde_json::from_str(&weapons_buffer)
                .unwrap_or(Weapons::new());
            scope.set_value("weapons", weapons);
        } else {
            // Should not happen
            scope.set_value("weapons", Weapons::new());
        }

        if let Some(gear_buffer) = &instance.gear_buffer {
            let gear : Gear = serde_json::from_str(&gear_buffer)
                .unwrap_or(Gear::new());
            scope.set_value("gear", gear);
        } else {
            // Should not happen
            scope.set_value("gear", Gear::new());
        }

        if let Some(skills_buffer) = &instance.skills_buffer {
            let skills : Skills = serde_json::from_str(&skills_buffer)
                .unwrap_or(Skills::new());
            scope.set_value("skills", skills);
        } else {
            // Should not happen
            scope.set_value("skills", Skills::new());
        }

        if let Some(experience_buffer) = &instance.experience_buffer {
            let experience : Experience = serde_json::from_str(&experience_buffer)
                .unwrap_or(Experience::new());
            scope.set_value("experience", experience);
        } else {
            // Should not happen
            scope.set_value("experience", Experience::new());
        }

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
}
