use crate::prelude::*;
use crate::vm::{Program, VMValue};
use crate::{CollisionWorld, Entity, MapMini, PlayerCamera};
use crossbeam_channel::{Receiver, Sender};
use eldiron_ruleset::{
    ResolvedAction, ResolvedActionCatalogue, ResolvedActionEffect, ResolvedCondition,
    ResolvedDerivedStat, ResolvedEquipmentPolicy, ResolvedInvocationScheme,
};
use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};
use theframework::prelude::*;
use toml::Table;
use uuid::Uuid;

#[derive(Default, Clone)]
pub struct LegacyDebugModule;

impl LegacyDebugModule {
    pub fn clear_execution(&mut self) {}
    pub fn add_value(&mut self, _id: u32, _event: &str, _x: u32, _y: u32, _value: TheValue) {}
    pub fn add_error(&mut self, _id: u32, _event: &str, _x: u32, _y: u32) {}
    pub fn remove_error(&mut self, _id: u32, _event: &str, _x: u32, _y: u32) {}
    pub fn mark_executed(&mut self, _id: u32, _event: &str, _x: u32, _y: u32) {}
    pub fn mark_header_executed(&mut self, _id: u32, _event: &str) {}
    pub fn mark_condition(
        &mut self,
        _id: u32,
        _event: &str,
        _x: u32,
        _y: u32,
        _taken: bool,
        _display: TheValue,
    ) {
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptScope {
    #[default]
    Entity,
    Item,
    Region,
    World,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationMode {
    #[default]
    Realtime,
    TurnBased,
    Hybrid,
}

impl SimulationMode {
    pub fn from_config_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "turn_based" | "turnbased" => Self::TurnBased,
            "hybrid" => Self::Hybrid,
            _ => Self::Realtime,
        }
    }
}

static WORLD_STATE: LazyLock<RwLock<ValueContainer>> =
    LazyLock::new(|| RwLock::new(ValueContainer::default()));

#[derive(Default)]
pub(crate) struct ResolvedRulesCache {
    initialized: bool,
    catalogue: ResolvedActionCatalogue,
    conditions: BTreeMap<String, ResolvedCondition>,
    derived_stats: BTreeMap<String, ResolvedDerivedStat>,
    equipment: ResolvedEquipmentPolicy,
    error: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct TaggedTileContact {
    tags: Vec<String>,
    cell: Vec2<i32>,
    layer: u8,
}

fn resolve_rules_state(
    rules: &Table,
) -> Result<
    (
        ResolvedActionCatalogue,
        BTreeMap<String, ResolvedCondition>,
        BTreeMap<String, ResolvedDerivedStat>,
        ResolvedEquipmentPolicy,
    ),
    String,
> {
    let catalogue = eldiron_ruleset::resolve_action_catalogue(rules)?;
    let conditions = eldiron_ruleset::resolve_conditions(rules)?;
    let derived_stats = eldiron_ruleset::resolve_derived_stats(rules)?;
    let equipment = eldiron_ruleset::resolve_equipment_policy(rules)?;
    for action in catalogue.actions.values() {
        for effect in &action.effects {
            let condition = match effect {
                ResolvedActionEffect::ApplyCondition { condition, .. }
                | ResolvedActionEffect::RemoveCondition { condition, .. } => condition,
                _ => continue,
            };
            if !conditions.contains_key(condition) {
                return Err(format!(
                    "actions.{}.result references unknown condition '{}'.",
                    action.id, condition
                ));
            }
        }
    }
    Ok((catalogue, conditions, derived_stats, equipment))
}

#[derive(Default)]
pub struct RegionCtx {
    pub map: Map,
    pub mapmini: MapMini,
    pub collision_world: CollisionWorld,

    pub paused: bool,

    pub blocking_tiles: FxHashSet<Uuid>,

    pub debug_mode: bool,
    pub debug: LegacyDebugModule,
    pub eldrin_debug: EldrinDebugModule,
    pub curr_debug_loc: Option<(String, u32, u32)>,
    pub current_debug_function: String,

    pub time: TheTime,
    pub region_id: u32,

    pub notifications_entities: Vec<(u32, i64, String)>,
    pub notifications_items: Vec<(u32, i64, String)>,
    pub active_choice_sessions: Vec<ChoiceSession>,
    /// The container panel most recently opened by each actor. A matching world
    /// container remains a valid loot source while its panel is open client-side.
    pub active_container_sessions: FxHashMap<u32, (u32, Option<u32>)>,

    pub ticks: i64,
    pub ticks_per_minute: u32,

    pub curr_entity_id: u32,
    pub curr_item_id: Option<u32>,
    pub current_script_scope: ScriptScope,

    pub entity_classes: FxHashMap<u32, String>,
    pub item_classes: FxHashMap<u32, String>,

    pub entity_player_classes: FxHashSet<String>,

    pub entity_class_data: FxHashMap<String, String>,
    pub item_class_data: FxHashMap<String, String>,
    pub entity_authoring_data: FxHashMap<String, String>,
    pub item_authoring_data: FxHashMap<String, String>,

    pub entity_proximity_alerts: FxHashMap<u32, f32>,
    pub item_proximity_alerts: FxHashMap<u32, f32>,

    pub entity_state_data: FxHashMap<u32, ValueContainer>,
    pub item_state_data: FxHashMap<u32, ValueContainer>,
    pub(crate) entity_tile_contacts: FxHashMap<u32, FxHashMap<(u32, Uuid), TaggedTileContact>>,
    pub entity_respawn_snapshots: FxHashMap<u32, Entity>,
    pub region_state: ValueContainer,
    pub procedural_spawn_guard: u8,

    pub to_execute_entity: Vec<(u32, String, VMValue)>,
    pub to_execute_item: Vec<(u32, String, VMValue)>,
    pub to_execute_world: Vec<(String, VMValue)>,
    pub pending_entity_transfers: Vec<(u32, String, String)>,

    pub entity_programs: FxHashMap<String, Arc<Program>>,
    pub item_programs: FxHashMap<String, Arc<Program>>,
    pub world_program: Option<Arc<Program>>,
    pub region_program: Option<Arc<Program>>,

    pub error_count: u32,
    pub startup_errors: Vec<String>,

    pub delta_time: f32,
    pub simulation_mode: SimulationMode,
    pub turn_timeout_ms: u32,
    pub config: Table,
    pub rules: Table,
    pub(crate) resolved_rules: RwLock<ResolvedRulesCache>,
    pub assets: Assets,

    pub to_receiver: OnceLock<Receiver<RegionMessage>>,
    pub from_sender: OnceLock<Sender<RegionMessage>>,

    pub health_attr: String,
    pub max_health_attr: String,
    pub level_attr: String,
    pub experience_attr: String,
    pub damage_committed: bool,
    pub current_damage_kind: Option<String>,
    pub current_damage_source_item: Option<u32>,

    pub currencies: Currencies,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceSession {
    pub from: u32,
    pub to: u32,
    pub expires_at_tick: i64,
    pub max_distance: f32,
}

impl RegionCtx {
    pub(crate) fn sync_attribute_roles(&mut self) {
        let roles = eldiron_ruleset::resolve_attribute_roles(&self.rules).unwrap_or_default();
        self.health_attr = roles.get("health").unwrap_or_default().to_string();
        self.max_health_attr = roles.get("max_health").unwrap_or_default().to_string();
        self.level_attr = roles.get("level").unwrap_or_default().to_string();
        self.experience_attr = roles.get("experience").unwrap_or_default().to_string();
    }

    fn initialize_resolved_rules(&self) -> Result<(), String> {
        {
            let cache = self
                .resolved_rules
                .read()
                .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
            if cache.initialized {
                return cache.error.clone().map_or(Ok(()), Err);
            }
        }

        let resolved = resolve_rules_state(&self.rules);
        let mut cache = self
            .resolved_rules
            .write()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        if !cache.initialized {
            cache.initialized = true;
            match resolved {
                Ok((catalogue, conditions, derived_stats, equipment)) => {
                    cache.catalogue = catalogue;
                    cache.conditions = conditions;
                    cache.derived_stats = derived_stats;
                    cache.equipment = equipment;
                }
                Err(err) => cache.error = Some(err),
            }
        }
        cache.error.clone().map_or(Ok(()), Err)
    }

    pub fn set_rules(&mut self, rules: Table) -> Result<(), String> {
        self.rules = rules;
        self.sync_attribute_roles();
        let resolved = resolve_rules_state(&self.rules);
        let cache = self
            .resolved_rules
            .get_mut()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        *cache = ResolvedRulesCache {
            initialized: true,
            catalogue: resolved
                .as_ref()
                .map(|(catalogue, _, _, _)| catalogue.clone())
                .unwrap_or_default(),
            conditions: resolved
                .as_ref()
                .map(|(_, conditions, _, _)| conditions.clone())
                .unwrap_or_default(),
            derived_stats: resolved
                .as_ref()
                .map(|(_, _, derived_stats, _)| derived_stats.clone())
                .unwrap_or_default(),
            equipment: resolved
                .as_ref()
                .map(|(_, _, _, equipment)| equipment.clone())
                .unwrap_or_default(),
            error: resolved.err(),
        };
        cache.error.clone().map_or(Ok(()), Err)
    }

    pub fn invalidate_resolved_rules(&mut self) {
        if let Ok(cache) = self.resolved_rules.get_mut() {
            *cache = ResolvedRulesCache::default();
        }
    }

    pub fn resolved_action(&self, action_id: &str) -> Result<Option<ResolvedAction>, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        Ok(cache.catalogue.action(action_id).cloned())
    }

    pub fn resolved_condition(
        &self,
        condition_id: &str,
    ) -> Result<Option<ResolvedCondition>, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        Ok(cache.conditions.get(condition_id.trim()).cloned())
    }

    pub(crate) fn with_resolved_conditions<T>(
        &self,
        read: impl FnOnce(&BTreeMap<String, ResolvedCondition>) -> T,
    ) -> Result<T, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        Ok(read(&cache.conditions))
    }

    pub(crate) fn with_resolved_derived_stats<T>(
        &self,
        read: impl FnOnce(&BTreeMap<String, ResolvedDerivedStat>) -> T,
    ) -> Result<T, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        Ok(read(&cache.derived_stats))
    }

    pub(crate) fn resolved_equipment_policy(&self) -> Result<ResolvedEquipmentPolicy, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved rules cache is unavailable.".to_string())?;
        Ok(cache.equipment.clone())
    }

    pub fn resolved_action_for_intent(
        &self,
        intent: &str,
    ) -> Result<Option<ResolvedAction>, String> {
        self.initialize_resolved_rules()?;
        let intent = intent.trim();
        if intent.is_empty() {
            return Ok(None);
        }
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        if let Some(action) = cache.catalogue.actions.get(intent) {
            return Ok(Some(action.clone()));
        }
        Ok(cache.catalogue.action_for_intent(intent).cloned())
    }

    pub fn resolved_action_for_spell(
        &self,
        spell_id: &str,
    ) -> Result<Option<ResolvedAction>, String> {
        self.initialize_resolved_rules()?;
        let spell_id = spell_id.trim();
        if spell_id.is_empty() {
            return Ok(None);
        }
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        if let Some(action) = cache.catalogue.actions.get(spell_id)
            && action.required_spell() == Some(spell_id)
        {
            return Ok(Some(action.clone()));
        }
        Ok(cache
            .catalogue
            .actions
            .values()
            .find(|action| action.required_spell() == Some(spell_id))
            .cloned())
    }

    pub fn resolved_action_for_invocation(
        &self,
        scheme_id: &str,
        phrase: &str,
    ) -> Result<Option<ResolvedAction>, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        Ok(cache
            .catalogue
            .action_for_invocation(scheme_id, phrase)
            .cloned())
    }

    pub fn resolved_invocation_bindings(
        &self,
    ) -> Result<Vec<(ResolvedInvocationScheme, String, ResolvedAction)>, String> {
        self.initialize_resolved_rules()?;
        let cache = self
            .resolved_rules
            .read()
            .map_err(|_| "Resolved action cache is unavailable.".to_string())?;
        let mut bindings = Vec::new();
        for action in cache.catalogue.actions.values() {
            for invocation in &action.invocations {
                let Some(scheme) = cache.catalogue.invocation_schemes.get(&invocation.scheme)
                else {
                    continue;
                };
                bindings.push((
                    scheme.clone(),
                    scheme.phrase_for_sequence(&invocation.sequence),
                    action.clone(),
                ));
            }
        }
        Ok(bindings)
    }

    pub fn clear_world_state() {
        if let Ok(mut state) = WORLD_STATE.write() {
            *state = ValueContainer::default();
        }
    }

    pub fn get_world_value(key: &str) -> Option<Value> {
        WORLD_STATE
            .read()
            .ok()
            .and_then(|state| state.get(key).cloned())
    }

    pub fn set_world_value(key: &str, value: Value) {
        if let Ok(mut state) = WORLD_STATE.write() {
            state.set(key, value);
        }
    }

    pub fn get_region_value(&self, key: &str) -> Option<Value> {
        self.region_state.get(key).cloned()
    }

    pub fn set_region_value(&mut self, key: &str, value: Value) {
        self.region_state.set(key, value);
    }

    pub fn resolve_sector_spawn_position(
        &self,
        sector_name: &str,
        radius: f32,
    ) -> Option<Vec2<f32>> {
        let mut marker_centers = Vec::new();
        for sector in &self.map.sectors {
            let Some(center) = sector.center(&self.map) else {
                continue;
            };
            let kind = sector.properties.get_str("procedural_kind").unwrap_or("");
            if sector.name == sector_name || kind == sector_name {
                marker_centers.push(center);
                continue;
            }
        }

        for center in &marker_centers {
            if self.mapmini.is_walkable_position(*center, radius) {
                return Some(*center);
            }
        }

        let marker = marker_centers.first().copied();
        if let Some(marker) = marker {
            let search_steps = [
                Vec2::zero(),
                Vec2::new(1.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(-1.0, -1.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(-2.0, 0.0),
                Vec2::new(0.0, 2.0),
                Vec2::new(0.0, -2.0),
            ];
            for offset in search_steps {
                let candidate = marker + offset;
                let candidate_sector = self.map.find_sector_at(candidate);
                let is_floorish = candidate_sector
                    .map(|sector| {
                        matches!(
                            sector.properties.get_str("procedural_kind").unwrap_or(""),
                            "floor" | "corridor" | "entrance" | "exit"
                        )
                    })
                    .unwrap_or(false);
                if is_floorish && self.mapmini.is_walkable_position(candidate, radius) {
                    return Some(candidate);
                }
            }
        }

        None
    }

    pub fn resolve_named_spawn_position_3d(
        &self,
        name: &str,
        radius: f32,
        preferred_y: Option<f32>,
    ) -> Option<Vec3<f32>> {
        if let Some(center) = self.map.named_area_center_3d(name) {
            return Some(center);
        }

        self.resolve_sector_spawn_position(name, radius)
            .map(|center| {
                let y = self
                    .map
                    .geometry_floor_height_at(center)
                    .or(preferred_y)
                    .unwrap_or(0.0);
                Vec3::new(center.x, y, center.y)
            })
    }

    fn nearest_sector_id_for_pos(&self, pos: Vec2<f32>, max_distance: f32) -> Option<u32> {
        let mut best: Option<(u32, f32)> = None;

        for sector in &self.map.sectors {
            if sector.layer.is_some() {
                continue;
            }
            if let Some(distance) = sector.signed_distance(&self.map, pos) {
                let distance = distance.abs();
                if distance <= max_distance {
                    match best {
                        Some((_, best_distance)) if distance >= best_distance => {}
                        _ => best = Some((sector.id, distance)),
                    }
                }
            }
        }

        best.map(|(sector_id, _)| sector_id)
    }

    fn stored_entity_sector_id(&self, entity: &Entity) -> Option<u32> {
        entity
            .attributes
            .get("sector_id")
            .and_then(|value| match value {
                Value::Int64(v) if *v >= 0 => Some(*v as u32),
                Value::Int(v) if *v >= 0 => Some(*v as u32),
                _ => None,
            })
    }

    fn entity_sector_id(&self, entity: &Entity) -> Option<u32> {
        self.stored_entity_sector_id(entity)
            .or_else(|| self.map.find_sector_at(entity.get_pos_xz()).map(|s| s.id))
            .or_else(|| self.nearest_sector_id_for_pos(entity.get_pos_xz(), 2.0))
    }

    fn authoring_table(&self) -> Option<Table> {
        self.assets.authoring_src.parse::<toml::Table>().ok()
    }

    fn sector_text_metadata(&self, sector: &Sector) -> (String, String) {
        let mut title = sector.name.clone();
        let mut description = String::new();

        if let Some(Value::Str(data)) = sector.properties.get("data")
            && let Ok(table) = data.parse::<toml::Table>()
        {
            if let Some(value) = table.get("title").and_then(toml::Value::as_str)
                && !value.trim().is_empty()
            {
                title = value.to_string();
            }
            if let Some(value) = table.get("description").and_then(toml::Value::as_str)
                && !value.trim().is_empty()
            {
                description = value.to_string();
            }

            for section in ["text_adventure", "text", "ui"] {
                if let Some(group) = table.get(section).and_then(toml::Value::as_table) {
                    if let Some(value) = group.get("title").and_then(toml::Value::as_str)
                        && !value.trim().is_empty()
                    {
                        title = value.to_string();
                    }
                    if let Some(value) = group.get("description").and_then(toml::Value::as_str)
                        && !value.trim().is_empty()
                    {
                        description = value.to_string();
                    }
                }
            }
        }

        (title, description)
    }

    fn sector_show_flags(&self, sector: &Sector) -> (bool, bool) {
        let mut show_in_2d = true;
        let mut show_in_3d = true;

        if let Some(Value::Str(data)) = sector.properties.get("data")
            && let Ok(table) = data.parse::<toml::Table>()
        {
            if let Some(value) = table.get("show_in_2d").and_then(toml::Value::as_bool) {
                show_in_2d = value;
            }
            if let Some(value) = table.get("show_in_3d").and_then(toml::Value::as_bool) {
                show_in_3d = value;
            }

            for section in ["text_adventure", "text", "ui"] {
                if let Some(group) = table.get(section).and_then(toml::Value::as_table) {
                    if let Some(value) = group.get("show_in_2d").and_then(toml::Value::as_bool) {
                        show_in_2d = value;
                    }
                    if let Some(value) = group.get("show_in_3d").and_then(toml::Value::as_bool) {
                        show_in_3d = value;
                    }
                }
            }
        }

        (show_in_2d, show_in_3d)
    }

    fn entity_display_name(&self, entity: &Entity) -> String {
        entity
            .attributes
            .get_str("name")
            .map(str::to_string)
            .or_else(|| entity.attributes.get_str("class_name").map(str::to_string))
            .unwrap_or_else(|| format!("Entity {}", entity.id))
    }

    fn player_ids_in_sector_id(&self, sector_id: u32) -> Vec<u32> {
        self.map
            .entities
            .iter()
            .filter(|entity| entity.is_player())
            .filter(|entity| self.entity_sector_id(entity) == Some(sector_id))
            .map(|entity| entity.id)
            .collect()
    }

    fn send_text_only_message_to_players(&self, player_ids: &[u32], message: String) {
        self.send_categorized_message_to_players(player_ids, message, "text_only");
    }

    fn send_categorized_message_to_players(
        &self,
        player_ids: &[u32],
        message: String,
        category: &str,
    ) {
        for player_id in player_ids {
            let msg = RegionMessage::Message(
                self.region_id,
                None,
                None,
                *player_id,
                message.clone(),
                category.to_string(),
            );
            self.from_sender.get().unwrap().send(msg).unwrap();
        }
    }

    pub(crate) fn send_item_drop_message_for_position(&self, pos: Vec2<f32>, count: usize) {
        let sector_id = self
            .map
            .find_sector_at(pos)
            .map(|sector| sector.id)
            .or_else(|| self.nearest_sector_id_for_pos(pos, 2.0));
        let Some(sector_id) = sector_id else {
            return;
        };
        if count == 0 {
            return;
        }

        let players = self.player_ids_in_sector_id(sector_id);
        if players.is_empty() {
            return;
        }

        let message = if count == 1 {
            "Something falls to the floor.".to_string()
        } else {
            "Several things fall to the floor.".to_string()
        };
        self.send_text_only_message_to_players(&players, message);
    }

    fn send_npc_sector_change_messages(
        &self,
        entity: &Entity,
        old_sector_id: Option<u32>,
        new_sector_id: Option<u32>,
    ) {
        if entity.is_player() {
            return;
        }

        let name = self.entity_display_name(entity);

        if let Some(old_sector_id) = old_sector_id {
            let players = self.player_ids_in_sector_id(old_sector_id);
            if !players.is_empty() {
                self.send_text_only_message_to_players(&players, format!("{} leaves.", name));
            }
        }

        if let Some(new_sector_id) = new_sector_id {
            let players = self.player_ids_in_sector_id(new_sector_id);
            if !players.is_empty() {
                self.send_text_only_message_to_players(&players, format!("{} enters.", name));
            }
        }
    }

    fn should_send_player_sector_description(
        &mut self,
        entity: &Entity,
        sector: &Sector,
        startup: bool,
    ) -> bool {
        let (_, description) = self.sector_text_metadata(sector);
        if description.trim().is_empty() {
            return false;
        }

        let is_2d = matches!(
            entity.attributes.get("player_camera"),
            Some(Value::PlayerCamera(PlayerCamera::D2 | PlayerCamera::D2Grid)) | None
        );
        let (show_in_2d, show_in_3d) = self.sector_show_flags(sector);
        if (is_2d && !show_in_2d) || (!is_2d && !show_in_3d) {
            return false;
        }

        let Some(authoring) = self.authoring_table() else {
            return true;
        };

        let sector_messages = authoring
            .get("sector_messages")
            .and_then(toml::Value::as_table);

        let show_on_startup = sector_messages
            .and_then(|t| t.get("show_on_startup"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);
        if startup && !show_on_startup {
            return false;
        }

        let mode_key = if is_2d { "mode_2d" } else { "mode_3d" };
        let cooldown_key = if is_2d {
            "cooldown_minutes_2d"
        } else {
            "cooldown_minutes_3d"
        };

        let mode = sector_messages
            .and_then(|t| t.get(mode_key))
            .and_then(toml::Value::as_str)
            .unwrap_or("always")
            .trim()
            .to_ascii_lowercase();

        if mode == "never" {
            return false;
        }

        let Some(state) = self.entity_state_data.get_mut(&entity.id) else {
            return true;
        };

        let seen_key = format!(
            "sector_desc_seen_{}_{}",
            if is_2d { "2d" } else { "3d" },
            sector.id
        );
        let tick_key = format!(
            "sector_desc_tick_{}_{}",
            if is_2d { "2d" } else { "3d" },
            sector.id
        );

        match mode.as_str() {
            "once" => !state.get_bool_default(&seen_key, false),
            "cooldown" => {
                let minutes = sector_messages
                    .and_then(|t| t.get(cooldown_key))
                    .and_then(toml::Value::as_float)
                    .unwrap_or(10.0)
                    .max(0.0);
                let cooldown_ticks = (minutes * self.ticks_per_minute as f64).round() as i64;
                if cooldown_ticks <= 0 {
                    true
                } else {
                    let last_tick = state.get(&tick_key).and_then(|v| match v {
                        Value::Int64(v) => Some(*v),
                        Value::Int(v) => Some(*v as i64),
                        _ => None,
                    });
                    last_tick
                        .map(|last_tick| self.ticks.saturating_sub(last_tick) >= cooldown_ticks)
                        .unwrap_or(true)
                }
            }
            _ => true,
        }
    }

    fn mark_player_sector_description_shown(
        &mut self,
        entity_id: u32,
        sector: &Sector,
        is_2d: bool,
    ) {
        let state = self.entity_state_data.entry(entity_id).or_default();
        let seen_key = format!(
            "sector_desc_seen_{}_{}",
            if is_2d { "2d" } else { "3d" },
            sector.id
        );
        let tick_key = format!(
            "sector_desc_tick_{}_{}",
            if is_2d { "2d" } else { "3d" },
            sector.id
        );
        state.set(&seen_key, Value::Bool(true));
        state.set(&tick_key, Value::Int64(self.ticks));
    }

    pub(crate) fn send_player_sector_description(
        &mut self,
        entity: &Entity,
        sector: &Sector,
        startup: bool,
    ) {
        let (_, description) = self.sector_text_metadata(sector);
        if !self.should_send_player_sector_description(entity, sector, startup) {
            return;
        }

        let is_2d = matches!(
            entity.attributes.get("player_camera"),
            Some(Value::PlayerCamera(PlayerCamera::D2 | PlayerCamera::D2Grid)) | None
        );

        let msg = RegionMessage::Message(
            self.region_id,
            Some(entity.id),
            None,
            entity.id,
            description,
            "system".to_string(),
        );
        self.from_sender.get().unwrap().send(msg).unwrap();
        self.mark_player_sector_description_shown(entity.id, sector, is_2d);
    }

    fn resolve_item_class_name(&self, requested: &str) -> Option<String> {
        let requested = requested.trim();
        if requested.is_empty() {
            return None;
        }

        if self.assets.items.contains_key(requested) {
            return Some(requested.to_string());
        }

        // Accept case-insensitive class/template ids.
        if let Some((class_name, _)) = self
            .assets
            .items
            .iter()
            .find(|(class_name, _)| class_name.eq_ignore_ascii_case(requested))
        {
            return Some(class_name.clone());
        }

        // Accept display names and stable ruleset ids from item data.
        for (class_name, (_, item_data)) in &self.assets.items {
            if let Ok(table) = item_data.parse::<toml::Table>() {
                let attr_name = table
                    .get("attributes")
                    .and_then(toml::Value::as_table)
                    .and_then(|attrs| attrs.get("name"))
                    .and_then(toml::Value::as_str);
                let ruleset_id = table
                    .get("attributes")
                    .and_then(toml::Value::as_table)
                    .and_then(|attrs| attrs.get("ruleset_id"))
                    .and_then(toml::Value::as_str);
                let source_id = table
                    .get("attributes")
                    .and_then(toml::Value::as_table)
                    .and_then(|attrs| attrs.get("source_id"))
                    .and_then(toml::Value::as_str);
                let ruleset_path_id = table
                    .get("attributes")
                    .and_then(toml::Value::as_table)
                    .and_then(|attrs| attrs.get("ruleset_path"))
                    .and_then(toml::Value::as_str)
                    .and_then(|path| path.rsplit('.').next());
                let top_name = table.get("name").and_then(toml::Value::as_str);
                if attr_name.is_some_and(|name| name.eq_ignore_ascii_case(requested))
                    || ruleset_id.is_some_and(|id| id.eq_ignore_ascii_case(requested))
                    || source_id.is_some_and(|id| id.eq_ignore_ascii_case(requested))
                    || ruleset_path_id.is_some_and(|id| id.eq_ignore_ascii_case(requested))
                    || top_name.is_some_and(|name| name.eq_ignore_ascii_case(requested))
                {
                    return Some(class_name.clone());
                }
            }
        }

        None
    }

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
        let class_name = self.resolve_item_class_name(&class_name)?;
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

    fn tagged_tile_contacts_at(
        &self,
        position: Vec2<f32>,
    ) -> FxHashMap<(u32, Uuid), TaggedTileContact> {
        let cell = position.map(|component| component.floor() as i32);
        self.map
            .sectors
            .iter()
            .filter_map(|sector| {
                let layer = sector.layer?;
                let tile_id = match sector.properties.get_default_source()? {
                    PixelSource::TileId(tile_id) => *tile_id,
                    _ => return None,
                };
                let tile = self.assets.tiles.get(&tile_id)?;
                if tile.gameplay_tags.is_empty() || !sector.is_inside(&self.map, position) {
                    return None;
                }
                let tags = tile.normalized_gameplay_tags();
                if tags.is_empty() {
                    return None;
                }
                Some((
                    (sector.id, tile_id),
                    TaggedTileContact { tags, cell, layer },
                ))
            })
            .collect()
    }

    fn push_tile_tag_event(
        &mut self,
        entity_id: u32,
        event: &str,
        contact: &TaggedTileContact,
        tag: &str,
    ) {
        self.to_execute_entity.push((
            entity_id,
            event.to_string(),
            VMValue::new_with_string(
                contact.cell.x as f32,
                contact.cell.y as f32,
                contact.layer as f32,
                tag,
            ),
        ));
    }

    /// Emit tag-based transitions for authored 2D tile placements underneath
    /// an entity. Logical named sectors are handled independently.
    fn check_entity_for_tile_change(&mut self, entity_id: u32, position: Vec2<f32>) {
        let new_contacts = self.tagged_tile_contacts_at(position);
        let old_contacts = self
            .entity_tile_contacts
            .remove(&entity_id)
            .unwrap_or_default();

        for (key, old_contact) in &old_contacts {
            let new_tags = new_contacts.get(key).map(|contact| &contact.tags);
            for tag in &old_contact.tags {
                if new_tags.is_none_or(|tags| !tags.contains(tag)) {
                    self.push_tile_tag_event(entity_id, "left_tile", old_contact, tag);
                }
            }
        }

        for (key, new_contact) in &new_contacts {
            let old_tags = old_contacts.get(key).map(|contact| &contact.tags);
            for tag in &new_contact.tags {
                if old_tags.is_none_or(|tags| !tags.contains(tag)) {
                    self.push_tile_tag_event(entity_id, "entered_tile", new_contact, tag);
                }
            }
        }

        if !new_contacts.is_empty() {
            self.entity_tile_contacts.insert(entity_id, new_contacts);
        }
    }

    /// Check if the player moved to a different sector and if yes send "enter" and "left" events
    pub fn check_player_for_section_change(&mut self, entity: &mut Entity) {
        let entity_id = entity.id;
        let entity_position = entity.get_pos_xz();
        // Determine, set and notify the entity about the sector it is in.
        if let Some(sector) = self.map.find_sector_at(entity_position).cloned() {
            let old_sector_name = entity
                .attributes
                .get_str("sector")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let old_sector_id = self.stored_entity_sector_id(entity);
            if old_sector_id != Some(sector.id) {
                self.send_npc_sector_change_messages(entity, old_sector_id, Some(sector.id));
                // Send entered event
                if !sector.name.is_empty() {
                    self.to_execute_entity.push((
                        entity.id,
                        "entered".into(),
                        VMValue::from(sector.name.clone()),
                    ));
                }
                if entity.is_player() {
                    let snapshot = entity.clone();
                    self.send_player_sector_description(&snapshot, &sector, false);
                }
                // Send left event
                if !old_sector_name.is_empty() {
                    self.to_execute_entity.push((
                        entity.id,
                        "left".into(),
                        VMValue::from(old_sector_name.clone()),
                    ));
                }

                entity.set_attribute("sector", Value::Str(sector.name.clone()));
                entity.set_attribute("sector_id", Value::Int64(sector.id as i64));
            }
        } else if let Some(Value::Str(old_sector_name)) = entity.attributes.get("sector") {
            // Send left event
            if !old_sector_name.is_empty() {
                if let Some(_class_name) = self.entity_classes.get(&entity.id) {
                    self.to_execute_entity.push((
                        entity.id,
                        "left".into(),
                        VMValue::from(old_sector_name.clone()),
                    ));
                }
            }
            entity.set_attribute("sector", Value::Str(String::new()));
            entity.set_attribute("sector_id", Value::Int64(-1));
        }
        self.check_entity_for_tile_change(entity_id, entity_position);
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
            let old_sector_id = self
                .map
                .entities
                .get(idx)
                .and_then(|e| self.stored_entity_sector_id(e));
            let sector_name = self.map.find_sector_at(pos).map(|s| s.name.clone());
            let sector_id = self.map.find_sector_at(pos).map(|s| s.id);
            let sector_snapshot = self.map.find_sector_at(pos).cloned();
            let mut entered_player_sector: Option<u32> = None;
            let entity_snapshot = self.map.entities.get(idx).cloned();
            let mut npc_transition: Option<(Entity, Option<u32>, Option<u32>)> = None;

            if let Some(entity) = self.map.entities.get_mut(idx) {
                if let (Some(sector_name), Some(sector_id)) = (sector_name, sector_id) {
                    if old_sector_id != Some(sector_id) {
                        if let Some(snapshot) = entity_snapshot.as_ref() {
                            npc_transition =
                                Some((snapshot.clone(), old_sector_id, Some(sector_id)));
                        }
                        let is_player = entity.is_player();
                        if !sector_name.is_empty() {
                            self.to_execute_entity.push((
                                entity.id,
                                "entered".into(),
                                VMValue::from(sector_name.clone()),
                            ));
                        }
                        if is_player {
                            entered_player_sector = Some(entity.id);
                        }
                        if !old_sector.is_empty() {
                            self.to_execute_entity.push((
                                entity.id,
                                "left".into(),
                                VMValue::from(old_sector.clone()),
                            ));
                        }
                        entity.set_attribute("sector", Value::Str(sector_name.clone()));
                        entity.set_attribute("sector_id", Value::Int64(sector_id as i64));
                    }
                } else {
                    if !old_sector.is_empty() {
                        self.to_execute_entity.push((
                            entity.id,
                            "left".into(),
                            VMValue::from(old_sector.clone()),
                        ));
                    }
                    entity.set_attribute("sector", Value::Str(String::new()));
                    entity.set_attribute("sector_id", Value::Int64(-1));
                }
            }

            if let Some((entity, old_sector_id, new_sector_id)) = npc_transition {
                self.send_npc_sector_change_messages(&entity, old_sector_id, new_sector_id);
            }

            if entered_player_sector.is_some()
                && let (Some(entity), Some(sector)) =
                    (entity_snapshot.as_ref(), sector_snapshot.as_ref())
            {
                self.send_player_sector_description(entity, sector, false);
            }

            self.check_entity_for_tile_change(id, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_tagged_tile_sector(ctx: &mut RegionCtx, tile_id: Uuid, tags: &[&str]) {
        let v0 = ctx.map.add_vertex_at(0.0, 0.0);
        let v1 = ctx.map.add_vertex_at(1.0, 0.0);
        let v2 = ctx.map.add_vertex_at(1.0, 1.0);
        let v3 = ctx.map.add_vertex_at(0.0, 1.0);
        ctx.map.create_linedef_manual(v0, v1);
        ctx.map.create_linedef_manual(v1, v2);
        ctx.map.create_linedef_manual(v2, v3);
        ctx.map.create_linedef_manual(v3, v0);
        let sector_id = ctx.map.close_polygon_manual().unwrap();
        let sector = ctx.map.find_sector_mut(sector_id).unwrap();
        sector.layer = Some(3);
        sector
            .properties
            .set("source", Value::Source(PixelSource::TileId(tile_id)));

        let mut tile = Tile::empty();
        tile.id = tile_id;
        tile.gameplay_tags = tags.iter().map(|tag| (*tag).to_string()).collect();
        ctx.assets.tiles.insert(tile_id, tile);
    }

    #[test]
    fn tagged_tile_emits_entered_and_left_events_with_cell_and_layer() {
        let mut ctx = RegionCtx::default();
        add_tagged_tile_sector(&mut ctx, Uuid::new_v4(), &["Chair", "seat"]);
        let mut entity = Entity::new();
        entity.id = 7;
        entity.set_pos_xz(Vec2::new(0.5, 0.5));

        ctx.check_player_for_section_change(&mut entity);

        let entered = ctx
            .to_execute_entity
            .iter()
            .filter(|(_, event, _)| event == "entered_tile")
            .collect::<Vec<_>>();
        assert_eq!(entered.len(), 2);
        assert_eq!(entered[0].0, 7);
        assert_eq!(entered[0].2.as_string(), Some("chair"));
        assert_eq!(entered[0].2.to_vec3(), Vec3::new(0.0, 0.0, 3.0));
        assert_eq!(entered[1].2.as_string(), Some("seat"));

        ctx.to_execute_entity.clear();
        entity.set_pos_xz(Vec2::new(1.5, 0.5));
        ctx.check_player_for_section_change(&mut entity);

        let left = ctx
            .to_execute_entity
            .iter()
            .filter(|(_, event, _)| event == "left_tile")
            .collect::<Vec<_>>();
        assert_eq!(left.len(), 2);
        assert_eq!(left[0].2.as_string(), Some("chair"));
        assert_eq!(left[0].2.to_vec3(), Vec3::new(0.0, 0.0, 3.0));
        assert_eq!(left[1].2.as_string(), Some("seat"));
    }

    #[test]
    fn untagged_tiles_do_not_emit_tile_events() {
        let mut ctx = RegionCtx::default();
        add_tagged_tile_sector(&mut ctx, Uuid::new_v4(), &[]);
        let mut entity = Entity::new();
        entity.id = 8;
        entity.set_pos_xz(Vec2::new(0.5, 0.5));

        ctx.check_player_for_section_change(&mut entity);

        assert!(
            ctx.to_execute_entity
                .iter()
                .all(|(_, event, _)| event != "entered_tile" && event != "left_tile")
        );
    }

    #[test]
    fn source_item_id_resolves_to_runtime_template() {
        let mut ctx = RegionCtx::default();
        ctx.assets.items.insert(
            "Bone Key".to_string(),
            (
                String::new(),
                r#"
[attributes]
source_id = "bone_key"
name = "Bone Key"
"#
                .to_string(),
            ),
        );

        assert_eq!(
            ctx.resolve_item_class_name("bone_key"),
            Some("Bone Key".to_string())
        );
    }
}
