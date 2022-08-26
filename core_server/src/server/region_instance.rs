use crate::prelude::*;
use rhai::{Engine, AST, Scope};

type NodeCall = fn(instance_index: usize, id: (usize, usize), data: &mut RegionInstance, behavior_type: BehaviorType) -> BehaviorNodeConnector;

pub struct RegionInstance<'a> {
    // Game data
    pub region_data                 : GameRegionData,
    pub region_behavior             : Vec<GameBehaviorData>,

    pub behaviors                   : HashMap<usize, GameBehaviorData>,
    pub systems                     : HashMap<usize, GameBehaviorData>,
    pub items                       : HashMap<usize, GameBehaviorData>,
    pub game_data                   : GameBehaviorData,

    // For faster lookup
    pub system_names                : Vec<String>,
    pub system_ids                  : Vec<usize>,
    pub area_ids                    : Vec<usize>,

    // All nodes
    nodes                           : HashMap<BehaviorNodeType, NodeCall>,

    /// The script engine
    pub engine                      : Engine,
    /// Script ast's, id is (BehaviorType, BehaviorId, BehaviorNodeID, AtomParameterID)
    pub ast                         : HashMap<(BehaviorType, usize, usize, String), AST>,

    // Character instances
    pub instances                   : Vec<BehaviorInstance>,
    pub scopes                      : Vec<rhai::Scope<'a>>,

    /// The current instance index of the current "Player" when executing the Game behavior per player
    pub curr_player_inst_index      : usize,

    /// Player game scopes
    pub game_player_scopes          : HashMap<usize, Scope<'a>>,

    /// The index of the game instance
    game_instance_index             : Option<usize>,

    /// Player uuid => player instance index
    pub player_uuid_indices         : HashMap<Uuid, usize>,

    /// The displacements for this region
    pub displacements               : HashMap<(isize, isize), TileData>,

    // Used by ticks for state memory

    /// Current characters per region
    pub characters                  : HashMap<usize, Vec<CharacterData>>,
    // Characters instance indices in a given area
    pub area_characters             : HashMap<usize, Vec<usize>>,
    // The character instances from the previous tick, used to figure out onEnter, onLeave etc events
    pub prev_area_characters        : HashMap<usize, Vec<usize>>,

    // Lights for this region
    pub lights                      : HashMap<usize, Vec<Light>>,

    // These are fields which provide debug feedback while running and are only used in the editors debug mode

    // The behavior id to debug, this is send from the server
    debug_behavior_id               : Option<usize>,

    // We are debugging the current tick characters
    is_debugging                    : bool,

    pub messages                    : Vec<(String, MessageType)>,
    pub executed_connections        : Vec<(BehaviorType, usize, BehaviorNodeConnector)>,
    pub changed_variables           : Vec<(usize, usize, usize, f64)>, // A variable has been changed: instance index, behavior id, node id, new value
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
                if let Some(n) = s.parse::<i64>().ok() {
                    let mut rng = thread_rng();
                    let random = rng.gen_range(1..=n) as f64;
                    //println!{"d{} {}",n, random};
                    return Ok(Some(random.into()));
                }
            }
            Ok(None)
        });

        // Display f64 as ints
        use pathfinding::num_traits::ToPrimitive;
        engine.register_fn("to_string", |x: f64| format!("{}", x.to_isize().unwrap()));

        let mut nodes : HashMap<BehaviorNodeType, NodeCall> = HashMap::new();

        nodes.insert(BehaviorNodeType::Expression, expression);
        nodes.insert(BehaviorNodeType::Script, script);
        nodes.insert(BehaviorNodeType::Message, message);
        nodes.insert(BehaviorNodeType::Pathfinder, pathfinder);
        nodes.insert(BehaviorNodeType::Lookout, lookout);
        nodes.insert(BehaviorNodeType::CloseIn, close_in);
        nodes.insert(BehaviorNodeType::CallSystem, call_system);
        nodes.insert(BehaviorNodeType::CallBehavior, call_behavior);
        nodes.insert(BehaviorNodeType::LockTree, lock_tree);
        nodes.insert(BehaviorNodeType::UnlockTree, unlock_tree);
        nodes.insert(BehaviorNodeType::SetState, set_state);

        nodes.insert(BehaviorNodeType::Always, always);
        nodes.insert(BehaviorNodeType::InsideArea, inside_area);
        nodes.insert(BehaviorNodeType::EnterArea, enter_area);
        nodes.insert(BehaviorNodeType::LeaveArea, leave_area);
        nodes.insert(BehaviorNodeType::DisplaceTiles, displace_tiles);
        nodes.insert(BehaviorNodeType::TeleportArea, teleport_area);
        nodes.insert(BehaviorNodeType::MessageArea, message_area);
        nodes.insert(BehaviorNodeType::AudioArea, audio_area);
        nodes.insert(BehaviorNodeType::LightArea, light_area);

        nodes.insert(BehaviorNodeType::Move, player_move);

        nodes.insert(BehaviorNodeType::Screen, screen);

        Self {
            region_data             : GameRegionData::new(),
            region_behavior         : vec![],

            behaviors               : HashMap::new(),
            systems                 : HashMap::new(),
            items                   : HashMap::new(),
            game_data               : GameBehaviorData::new(),

            system_names            : vec![],
            system_ids              : vec![],
            area_ids                : vec![],

            engine,
            ast                     : HashMap::new(),
            nodes,

            instances               : vec![],
            scopes                  : vec![],

            curr_player_inst_index  : 0,
            game_player_scopes      : HashMap::new(),

            game_instance_index     : None,

            player_uuid_indices     : HashMap::new(),

            displacements           : HashMap::new(),

            characters              : HashMap::new(),
            area_characters         : HashMap::new(),
            prev_area_characters    : HashMap::new(),
            lights                  : HashMap::new(),

            debug_behavior_id       : None,
            is_debugging            : false,

            messages                : vec![],
            executed_connections    : vec![],
            changed_variables       : vec![],
        }
    }

    /// Game tick
    pub fn tick(&mut self) -> Vec<Message> {
        self.changed_variables = vec![];
        self.messages = vec![];
        self.characters = HashMap::new();
        self.lights = HashMap::new();
        self.prev_area_characters = self.area_characters.clone();
        self.area_characters = HashMap::new();

        let mut messages = vec![];

        // Execute behaviors
        for inst_index in 0..self.instances.len() {

            self.messages = vec![];
            self.executed_connections = vec![];

            self.instances[inst_index].messages = vec![];
            self.instances[inst_index].audio = vec![];

            if  self.instances[inst_index].old_position.is_some() {
                self.instances[inst_index].curr_transition_time += 1;

                if self.instances[inst_index].curr_transition_time > self.instances[inst_index].max_transition_time {
                    self.instances[inst_index].old_position = None;
                    self.instances[inst_index].curr_transition_time = 0;
                }
            }

            // Skip Sleep cycles
            if self.instances[inst_index].sleep_cycles > 0 {
                self.instances[inst_index].sleep_cycles -= 1;
            } else {

                // Killed or Purged: Skip
                if self.instances[inst_index].state == BehaviorInstanceState::Purged || self.instances[inst_index].state == BehaviorInstanceState::Killed {
                    continue;
                }

                // Are we debugging this character ?
                self.is_debugging = Some(self.instances[inst_index].behavior_id) == self.debug_behavior_id;

                if self.instances[inst_index].instance_type == BehaviorInstanceType::NonPlayerCharacter {
                    // Execute trees of an NPC

                    // Has a locked tree ?
                    if let Some(locked_tree) = self.instances[inst_index].locked_tree {
                            self.execute_node(inst_index, locked_tree);
                    } else {
                        // Unlocked, execute all valid trees
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
                } else {
                    // Execute the tree which matches the current action, i.e. "onXXX", like "onMove"

                    let mut tree_id: Option<usize> = None;
                    if let Some(action) = &self.instances[inst_index].action {
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
                            self.execute_node(inst_index, tree_id);
                        } else {
                            println!("Cannot find valid tree for action {}", action.action);
                        }

                        self.instances[inst_index].action = None;
                    }
                }

                // If we are debugging this instance, send the debug data
                if Some(self.instances[inst_index].behavior_id) == self.debug_behavior_id {
                    let debug = BehaviorDebugData {
                        executed_connections    : self.executed_connections.clone()
                    };
                    messages.push(Message::DebugData(debug));
                }
            }

            // Add to the characters

            if let Some(position) = self.instances[inst_index].position {
                if let Some(tile) = self.instances[inst_index].tile {
                    let character = CharacterData {
                        position,
                        old_position            : self.instances[inst_index].old_position,
                        max_transition_time     : self.instances[inst_index].max_transition_time,
                        curr_transition_time    : self.instances[inst_index].curr_transition_time,
                        tile,
                        name                    : self.instances[inst_index].name.clone(),
                        id                      : self.instances[inst_index].id,
                        index                   : inst_index,
                     };
                     if let Some(list) = self.characters.get_mut(&position.0) {
                         list.push(character);
                     } else {
                         self.characters.insert(position.0, vec![character]);
                     }
                }
            }
        }

        // Execute region area behaviors
        let mut to_execute: Vec<(usize, usize)> = vec![];
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
                        }
                    } else {
                        screen = Some(self.instances[inst_index].curr_player_screen.clone());
                    }
                }

                let mut region        : Option<GameRegionData> = None;
                let mut characters    : Vec<CharacterData> = vec![];
                let mut displacements : HashMap<(isize, isize), TileData> = HashMap::new();
                let mut lights        : Vec<Light> = vec![];

                let mut needs_transfer_to: Option<usize> = None;
                if let Some(position) = self.instances[inst_index].position {

                    if position.0 != self.region_data.id {
                        // We need to transfer the character to a new region
                        needs_transfer_to = Some(position.0);
                    } else
                    // Check if the character is in a region we did not send to the client yet OR if the editor is debugging
                    if self.instances[inst_index].regions_send.contains(&position.0) == false || self.debug_behavior_id.is_some() {
                        region = Some(self.region_data.clone());
                        self.instances[inst_index].regions_send.insert(position.0);
                    }
                    // Copy the displacements
                    displacements = self.displacements.clone();

                    // Send the characters of the client region
                    if let Some(chars) = self.characters.get(&position.0) {
                        characters = chars.clone();
                    }

                    if self.lights.contains_key(&position.0) {
                        lights = self.lights[&position.0].clone();
                    }
                }

                let update = GameUpdate{
                    id                      : self.instances[inst_index].id,
                    position                : self.instances[inst_index].position,
                    old_position            : self.instances[inst_index].old_position,
                    max_transition_time     : self.instances[inst_index].max_transition_time,
                    curr_transition_time    : self.instances[inst_index].curr_transition_time,
                    tile                    : self.instances[inst_index].tile,
                    screen                  : screen,
                    region,
                    lights,
                    displacements,
                    characters,
                    messages                : self.instances[inst_index].messages.clone(),
                    audio                   : self.instances[inst_index].audio.clone(),
                 };

                //self.instances[inst_index].update = serde_json::to_string(&update).ok();
                if let Some(transfer_to) = needs_transfer_to {
                    self.instances[inst_index].scope_buffer = Some(ScopeBuffer::new(&self.scopes[inst_index]));
                    messages.push(Message::TransferCharacter(transfer_to, self.instances[inst_index].clone()));
                    self.purge_instance(inst_index);
                }
                messages.push(Message::PlayerUpdate(update.id, update));
            }
        }
        messages
    }

    /// Executes the given node and follows the connection chain
    pub fn execute_node(&mut self, instance_index: usize, node_id: usize) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<usize> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, usize, BehaviorNodeConnector)> = vec![];

        let mut is_sequence = false;
        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(behavior) = self.behaviors.get_mut(&self.instances[instance_index].behavior_id) {
            if let Some(node) = behavior.nodes.get_mut(&node_id) {

                // Handle special nodes
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
                        let connector = node_call(instance_index, (behavior_id, node_id), self, BehaviorType::Behaviors);
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

            if let Some(connector) = self.execute_node(instance_index, *connected_node_id) {
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
    pub fn execute_systems_node(&mut self, instance_index: usize, node_id: usize) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<usize> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, usize, BehaviorNodeConnector)> = vec![];

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
                    connectors.push(BehaviorNodeConnector::Bottom4);                    is_sequence = true;
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

    /// Executes the given node and follows the connection chain
    fn execute_area_node(&mut self, region_id: usize, area_index: usize, node_id: usize) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<usize> = vec![];

        let mut rc : Option<BehaviorNodeConnector> = None;

        // Call the node and get the resulting BehaviorNodeConnector
        if let Some(node) = self.region_behavior[area_index].nodes.get_mut(&node_id) {

            if let Some(node_call) = self.nodes.get_mut(&node.behavior_type) {
                let connector = node_call(region_id, (area_index, node_id), self, BehaviorType::Regions);
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
    fn execute_game_node(&mut self, instance_index: usize, node_id: usize) -> Option<BehaviorNodeConnector> {

        let mut connectors : Vec<BehaviorNodeConnector> = vec![];
        let mut connected_node_ids : Vec<usize> = vec![];
        let mut possibly_executed_connections : Vec<(BehaviorType, usize, BehaviorNodeConnector)> = vec![];

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
    pub fn setup(&mut self, region: String, region_behavior: HashMap<usize, Vec<String>>, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, game: String) {
        // Decode all JSON
        if let Some(region_data) = serde_json::from_str(&region).ok() {
            self.region_data = region_data;
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
                self.items.insert(behavior_data.id, behavior_data);
            }
        }
        if let Some(game_data) = serde_json::from_str(&game).ok() {
            self.game_data = game_data;
        }

        // Create all behavior instances of characters inside this region
        let ids : Vec<usize> = self.behaviors.keys().cloned().collect();
        for id in ids {
            self.create_behavior_instance(id, true);
        }

        // Create the game instance itself
        let mut to_execute : Vec<usize> = vec![];
        let mut startup_name : Option<String> = None;
        let mut locked_tree  : Option<usize> = None;
        let mut scope = rhai::Scope::new();
        let behavior = &mut self.game_data;

        // Collect name of the startup tree and the variables
        for (_id, node) in &behavior.nodes {
            if node.behavior_type == BehaviorNodeType::BehaviorType {
                if let Some(value )= node.values.get(&"startup".to_string()) {
                    startup_name = Some(value.4.clone());
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

        let instance = BehaviorInstance {id: Uuid::new_v4(), state: BehaviorInstanceState::Normal, name: behavior.name.clone(), behavior_id: behavior.id, tree_ids: to_execute.clone(), position: None, tile: None, target_instance_index: None, locked_tree, party: vec![], node_values: HashMap::new(), state_values: HashMap::new(), scope_buffer: None, sleep_cycles: 0, systems_id: 0, action: None, instance_type: BehaviorInstanceType::GameLogic, update: None, regions_send: std::collections::HashSet::new(), curr_player_screen_id: None, game_locked_tree: None, curr_player_screen: "".to_string(), messages: vec![], audio: vec![], old_position: None, max_transition_time: 0, curr_transition_time: 0, alignment: 1 };

        self.instances.push(instance);
        self.scopes.push(scope);

        for tree_id in &to_execute {
            // Execute this tree if it is a "Startup" Only tree
            if let Some(value)= self.get_game_node_value(*tree_id, "execute") {
                if value.0 == 1.0 {
                    //self.execute_game_node(index, tree_id.clone());
                }
            }
        }
        self.game_instance_index = Some(index);

    }

    /// Creates a new player instance
    pub fn create_player_instance(&mut self, uuid: Uuid, position: Position) {
        let mut player_id : Option<usize> = None;
        for b in &self.behaviors {
            if b.1.name == "Player" {
                player_id = Some(*b.0);
            }
        }
        if let Some(player_id) = player_id {
            let index = self.create_behavior_instance(player_id, false);
            self.instances[index].instance_type = BehaviorInstanceType::Player;
            self.instances[index].id = uuid;
            self.instances[index].position = Some(position);
            self.player_uuid_indices.insert(uuid, index);
            log::info!("Player instance {} created.", uuid);
        }
    }

    /// Creates an instance of a behavior (character)
    fn create_behavior_instance(&mut self, id: usize, npc_only: bool) -> usize {

        let mut index = 0;
        // Instances to create for this behavior
        if let Some(behavior) = self.behaviors.get_mut(&id) {

            if npc_only && behavior.name == "Player" {
                return index;
            }

            let mut to_create : Vec<CharacterInstanceData> = vec![];

            // Collect all the default data for the behavior from the nodes: Position, tile, behavior Trees and variables.
            let mut to_execute              : Vec<usize> = vec![];
            let mut default_position        : Option<(usize, isize, isize)> = None;
            let mut default_tile            : Option<(usize, usize, usize)> = None;
            let mut default_align           : i64 = 1;
            let mut default_scope    = rhai::Scope::new();

            for (id, node) in &behavior.nodes {
                if node.behavior_type == BehaviorNodeType::BehaviorTree {

                    for c in &behavior.connections {
                        if c.0 == *id {
                            to_execute.push(c.0);
                        }
                    }
                } else
                if node.behavior_type == BehaviorNodeType::BehaviorType {
                    if let Some(value )= node.values.get(&"position".to_string()) {
                        default_position = Some((value.0 as usize, value.1 as isize, value.2 as isize));
                    }
                    if let Some(value )= node.values.get(&"tile".to_string()) {
                        default_tile = Some((value.0 as usize, value.1 as usize, value.2 as usize));
                    }
                    if let Some(value )= node.values.get(&"type".to_string()) {
                        default_align = 2 - value.0 as i64 - 1;
                    }
                } else
                if node.behavior_type == BehaviorNodeType::VariableNumber {
                    if let Some(value )= node.values.get(&"value".to_string()) {
                        default_scope.push(node.name.clone(), value.0.clone());
                    } else {
                        default_scope.push(node.name.clone(), 0.0_f64);
                    }
                }
            }
            // Add main
            if default_position.is_some() && default_tile.is_some() {
                let main = CharacterInstanceData {
                    position    : default_position.unwrap().clone(),
                    tile        : default_tile.clone(),
                    name        : Some(behavior.name.clone()),
                    alignment   : default_align
                };
                to_create.push(main)
            }
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
                }
            }
            // Now we have all instances of the behavior we need to create
            for inst in to_create {

                // Only create when instance ins in this region
                if inst.position.0 != self.region_data.id {
                    continue;
                }

                //println!("Creating instance {}", inst.name.unwrap());
                let instance = BehaviorInstance {id: uuid::Uuid::new_v4(), state: BehaviorInstanceState::Normal, name: behavior.name.clone(), behavior_id: behavior.id, tree_ids: to_execute.clone(), position: Some(inst.position), tile: inst.tile, target_instance_index: None, locked_tree: None, party: vec![], node_values: HashMap::new(), state_values: HashMap::new(), scope_buffer: None, sleep_cycles: 0, systems_id: 0, action: None, instance_type: BehaviorInstanceType::NonPlayerCharacter, update: None, regions_send: std::collections::HashSet::new(), curr_player_screen_id: None, game_locked_tree: None, curr_player_screen: "".to_string(), messages: vec![], audio: vec![], old_position: None, max_transition_time: 0, curr_transition_time: 0, alignment: inst.alignment };

                index = self.instances.len();
                self.instances.push(instance);

                // Set the default values into the scope
                let mut scope = default_scope.clone();
                scope.set_value("name", behavior.name.clone());
                scope.set_value("alignment", inst.alignment as i64);
                self.scopes.push(scope);
            }
        }
        index
    }

    /// Returns a game node value
    fn get_game_node_value(&mut self, node_id: usize, node_property: &str) -> Option<(f64, f64, f64, f64, String)> {
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
    pub fn get_behavior(&self, id: usize, behavior_type: BehaviorType) -> Option<&GameBehaviorData> {
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
    pub fn get_mut_behavior(&mut self, id: usize, behavior_type: BehaviorType) -> Option<&mut GameBehaviorData> {
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
    pub fn transfer_character_into(&mut self, instance: BehaviorInstance) {
        // TODO, fill in purged
        self.player_uuid_indices.insert(instance.id, self.instances.len());
        let mut scope = rhai::Scope::new();
        if let Some(buffer) = &instance.scope_buffer {
            fill_scope_from_buffer(&mut scope, buffer);
        }
        self.instances.push(instance);
        self.scopes.push(scope);
    }

    /// Sets the debugging behavior id.
    pub fn set_debug_behavior_id(&mut self, behavior_id: usize) {
        self.debug_behavior_id = Some(behavior_id);
    }

}
