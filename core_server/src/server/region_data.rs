use crate::prelude::*;

pub struct RegionData {
    /// The character sheets in the region
    pub sheets                      : Vec<Sheet>,

    /// Behavior Instance Data
    pub character_instances         : Vec<BehaviorInstance>,

    /// Holds the tile data and areas for the region
    pub region_data                 : GameRegionData,

    /// The behavior graphs for the regions area
    pub region_area_behavior        : Vec<GameBehaviorData>,

    /// The displacements for this region
    pub displacements               : FxHashMap<(isize, isize), TileData>,

    /// Do characters move per tile or per pixel ?
    pub pixel_based_movement        : bool,

    /// The loot in the region
    pub loot                        : FxHashMap<(isize, isize), Vec<Item>>,

    /// The node functions
    pub nodes                       : FxHashMap<BehaviorNodeType, NodeDataCall>,

    /// The text of the current movement (North, South)
    pub action_direction_text       : String,

    /// The text for the current subject / context
    pub action_subject_text         : String,

    /// During region area execution this points to the calling behavior index (for sending messages etc)
    pub curr_action_character_index  : Option<usize>,

    /// The current instance index of the current "Player" when executing the Game behavior per player
    pub curr_player_inst_index      : usize,

    /// Player uuid => player instance index
    pub player_uuid_indices         : FxHashMap<Uuid, usize>,

    /// Current characters per region
    pub characters                  : FxHashMap<Uuid, Vec<CharacterData>>,

    // Characters instance indices in a given area
    pub area_characters             : FxHashMap<usize, Vec<usize>>,

    // The character instances from the previous tick, used to figure out onEnter, onLeave etc events
    pub prev_area_characters        : FxHashMap<usize, Vec<usize>>,

    // Lights for this region
    pub lights                      : Vec<LightData>,

    /// How many ticks for one minute (gets read from the game settings)
    pub ticks_per_minute            : usize,

    /// Node trees which have been marked for execution inside scripts
    pub to_execute                  : Vec<(usize, String)>,

    /// Respawns the given chararacter uuid at the given tick count
    pub respawn_instance            : FxHashMap<Uuid, (usize, CharacterInstanceData)>,

    pub skill_trees                 : FxHashMap<String, Vec<(i32, String, String)>>,

    // Debug Data which gets displayer in the editor

    /// The behavior id of the character to debug, this is send from the server
    pub debug_behavior_id           : Option<Uuid>,
    // We are debugging the current tick characters
    pub is_debugging                : bool,
    /// Executed connections for the last tick for the debug character
    pub executed_connections        : Vec<(Uuid, BehaviorNodeConnector)>,
    /// Script errors for the debug character
    pub script_errors               : Vec<((Uuid, Uuid, String), (String, Option<u32>))>,

    /// The item effects to the right of the rolled weapon level
    pub item_effects                : Option<(Uuid, Uuid)>,

    /// The current character sheet index
    pub curr_index                  : usize,

    /// The current area behavior index sheet index
    pub curr_area_index             : usize,
}

impl RegionData {
    pub fn new() -> Self {

        let mut nodes : FxHashMap<BehaviorNodeType, NodeDataCall> = FxHashMap::default();

        // BEHAVIOR
        nodes.insert(BehaviorNodeType::Script, node_script);
        nodes.insert(BehaviorNodeType::Expression, node_expression);
        nodes.insert(BehaviorNodeType::Message, node_message);
        nodes.insert(BehaviorNodeType::Audio, node_audio);
        nodes.insert(BehaviorNodeType::HasTarget, node_has_target);
        nodes.insert(BehaviorNodeType::RandomWalk, node_random_walk);
        nodes.insert(BehaviorNodeType::Lookout, node_lookout);
        nodes.insert(BehaviorNodeType::CloseIn, node_close_in);
        nodes.insert(BehaviorNodeType::Pathfinder, node_pathfinder);
        nodes.insert(BehaviorNodeType::Untarget, node_untarget);
        nodes.insert(BehaviorNodeType::LockTree, node_lock_tree);
        nodes.insert(BehaviorNodeType::UnlockTree, node_unlock_tree);
        nodes.insert(BehaviorNodeType::QueryState, node_query_state);
        nodes.insert(BehaviorNodeType::SetState, node_set_state);
        nodes.insert(BehaviorNodeType::Take, node_player_take);
        nodes.insert(BehaviorNodeType::Drop, node_player_drop);
        nodes.insert(BehaviorNodeType::Target, node_player_target);
        nodes.insert(BehaviorNodeType::Equip, node_player_equip);
        nodes.insert(BehaviorNodeType::MultiChoice, node_multi_choice);
        nodes.insert(BehaviorNodeType::Sell, node_sell);
        nodes.insert(BehaviorNodeType::CallBehavior, node_call_behavior);
        nodes.insert(BehaviorNodeType::CallSystem, node_call_system);
        nodes.insert(BehaviorNodeType::DropInventory, node_drop_inventory);
        nodes.insert(BehaviorNodeType::Effect, node_effect);
        nodes.insert(BehaviorNodeType::Respawn, node_respawn);
        nodes.insert(BehaviorNodeType::Schedule, node_schedule);
        nodes.insert(BehaviorNodeType::SetLevelTree, node_set_level_tree);
        nodes.insert(BehaviorNodeType::OverlayTiles, node_overlay_tiles);
        nodes.insert(BehaviorNodeType::SetItemTile, node_set_item_tile);
        nodes.insert(BehaviorNodeType::Teleport, node_teleport);

        nodes.insert(BehaviorNodeType::MagicTarget, node_magic_target);

        nodes.insert(BehaviorNodeType::Screen, node_screen);
        //nodes.insert(BehaviorNodeType::Widget, node_widget);

        // PLAYER
        nodes.insert(BehaviorNodeType::Action, node_player_action);
        nodes.insert(BehaviorNodeType::Move, node_player_move);

        // REGION
        nodes.insert(BehaviorNodeType::Always, node_always_area);
        nodes.insert(BehaviorNodeType::ActionArea, node_action_area);
        nodes.insert(BehaviorNodeType::EnterArea, node_enter_area);
        nodes.insert(BehaviorNodeType::InsideArea, node_inside_area);
        nodes.insert(BehaviorNodeType::LeaveArea, node_leave_area);
        nodes.insert(BehaviorNodeType::TeleportArea, node_teleport_area);
        nodes.insert(BehaviorNodeType::MessageArea, node_message_area);
        nodes.insert(BehaviorNodeType::AudioArea, node_audio_area);
        nodes.insert(BehaviorNodeType::LightArea, node_light_area);

        // ITEM
        nodes.insert(BehaviorNodeType::LightItem, node_light_item);

        Self {
            sheets                          : vec![],
            character_instances             : vec![],
            region_data                     : GameRegionData::new(),
            region_area_behavior            : vec![],
            displacements                   : FxHashMap::default(),
            pixel_based_movement            : true,
            loot                            : FxHashMap::default(),

            nodes,

            action_direction_text           : "".to_string(),
            action_subject_text             : "".to_string(),

            curr_action_character_index     : None,
            curr_player_inst_index          : 0,
            player_uuid_indices             : FxHashMap::default(),

            characters                      : FxHashMap::default(),
            area_characters                 : FxHashMap::default(),
            prev_area_characters            : FxHashMap::default(),
            lights                          : vec![],

            ticks_per_minute                : 4,

            to_execute                      : vec![],

            respawn_instance                : FxHashMap::default(),

            skill_trees                     : FxHashMap::default(),

            // Debug
            debug_behavior_id               : None,
            is_debugging                    : false,
            executed_connections            : vec![],
            script_errors                   : vec![],

            item_effects                    : None,

            curr_index                      : 0,
            curr_area_index                 : 0,
        }
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

    /// Returns the current position of the character index, takes into account an ongoing animation
    pub fn get_instance_position(&self, inst_index: usize) -> Option<Position> {
        if let Some(old_position) = &self.character_instances[inst_index].old_position {
            return Some(old_position.clone());
        }
        self.character_instances[inst_index].position.clone()
    }
}