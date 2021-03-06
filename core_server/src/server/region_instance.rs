use crate::prelude::*;

pub struct RegionInstance<'a> {
    // Game data
    region_data                     : GameRegionData,
    behaviors                       : HashMap<usize, GameBehaviorData>,
    systems                         : HashMap<usize, GameBehaviorData>,
    items                           : HashMap<usize, GameBehaviorData>,
    game_data                       : GameBehaviorData,

    // Character instances
    instances                       : Vec<BehaviorInstance>,
    scopes                          : Vec<rhai::Scope<'a>>,
}

impl RegionInstance<'_> {

    pub fn new() -> Self {
        Self {
            region_data             : GameRegionData::new(),
            behaviors               : HashMap::new(),
            systems                 : HashMap::new(),
            items                   : HashMap::new(),
            game_data               : GameBehaviorData::new(),

            instances               : vec![],
            scopes                  : vec![],
        }
    }

    /// Setup the instance data by decoding the JSON for all game elements.
    pub fn setup(&mut self, region: String, behaviors: Vec<String>, systems: Vec<String>, items: Vec<String>, game: String) {
        // Decode all JSON
        if let Some(region_data) = serde_json::from_str(&region).ok() {
            self.region_data = region_data;
        }
        for b in behaviors {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&b).ok() {
                self.behaviors.insert(behavior_data.id, behavior_data);
            }
        }
        for s in systems {
            if let Some(behavior_data) = serde_json::from_str::<GameBehaviorData>(&s).ok() {
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
        for (_index, behavior) in &self.behaviors {

            // Instances to create for this behavior
            let mut to_create : Vec<CharacterInstanceData> = vec![];

            // Collect all the default data for the behavior from the nodes: Position, tile, behavior Trees and variables.
            let mut to_execute : Vec<usize> = vec![];
            let mut default_position : Option<(usize, isize, isize)> = None;
            let mut default_tile     : Option<(usize, usize, usize)> = None;
            let mut default_scope = rhai::Scope::new();

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

                println!("Creating instance {}", inst.name.unwrap());
                let instance = BehaviorInstance {id: uuid::Uuid::new_v4(), state: BehaviorInstanceState::Normal, name: behavior.name.clone(), behavior_id: behavior.id, tree_ids: to_execute.clone(), position: Some(inst.position), tile: inst.tile, target_instance_index: None, locked_tree: None, party: vec![], node_values: HashMap::new(), state_values: HashMap::new(), number_values: HashMap::new(), sleep_cycles: 0, systems_id: 0, action: None, instance_type: BehaviorInstanceType::NonPlayerCharacter, update: None, regions_send: std::collections::HashSet::new(), curr_player_screen_id: None, game_locked_tree: None, curr_player_screen: "".to_string(), messages: vec![], audio: vec![], old_position: None, max_transition_time: 0, curr_transition_time: 0 };

                self.instances.push(instance);
                self.scopes.push(default_scope.clone());
            }
        }
    }

    pub fn tick(&mut self) {

    }
}