use crate::server::message::{AudioCommand, RegionMessage};
use crate::server::region::{
    RegionInstance, add_debug_value, apply_damage_direct, apply_damage_rules,
    apply_spell_default_attrs, consume_attack_ammunition_for_source, craft_ruleset_recipe,
    current_attack_base_damage_for_entity, current_attack_cooldown_for_entity,
    drop_items_into_ruleset_loot_container, entity_disposition_by_id, entity_is_hostile_by_id,
    entity_item_by_id, execute_ruleset_action, grant_experience, has_attack_ammunition_or_message,
    is_spell_on_cooldown, open_dialog_node, set_entity_cooldown_attrs, set_spell_cooldown,
};
use crate::server::regionctx::{ChoiceSession, ScriptScope};
use crate::vm::*;
use crate::{
    Choice, Entity, EntityAction, Item, Map, MultipleChoice, PixelSource, PlayerCamera, RegionCtx,
    Value, ValueContainer,
};
use rand::Rng;
use scenevm::{GeoId, PaletteRemap2DMode};
use theframework::prelude::TheValue;
use vek::{Vec2, Vec3};

struct RegionHost<'a> {
    ctx: &'a mut RegionCtx,
}

enum SpellTargetArg {
    Entity(u32),
    Position(Vec3<f32>),
}

fn opening_geo_for_item(item: &Item) -> Option<GeoId> {
    if let Some(object_id) = item.attributes.get_id("geometry_object_id") {
        return Some(GeoId::GeometryObject(object_id));
    }

    if let Some(sector_id) = match item.attributes.get("sector_id") {
        Some(Value::UInt(v)) => Some(*v),
        Some(Value::Int(v)) if *v >= 0 => Some(*v as u32),
        Some(Value::Int64(v)) if *v >= 0 => Some(*v as u32),
        _ => None,
    } {
        return Some(GeoId::Sector(sector_id));
    }

    let host_id = match item.attributes.get("profile_host_sector_id") {
        Some(Value::UInt(v)) => Some(*v),
        _ => None,
    }?;

    let profile_id = match item.attributes.get("profile_sector_id") {
        Some(Value::UInt(v)) => Some(*v),
        _ => None,
    }?;

    Some(GeoId::Hole(host_id, profile_id))
}

fn apply_geometry_object_item_attr(
    ctx: &mut RegionCtx,
    object_id: uuid::Uuid,
    key: &str,
    value: bool,
) {
    let Some(object) = ctx
        .map
        .geometry_objects
        .iter_mut()
        .find(|object| object.id == object_id)
    else {
        return;
    };

    match key {
        "visible" => object.visible = value,
        "blocking" => object.solid = value,
        _ => {}
    }
}

fn rebuild_runtime_navigation(ctx: &mut RegionCtx) {
    ctx.mapmini = ctx.map.as_mini(&ctx.blocking_tiles);
    ctx.collision_world = crate::CollisionWorld::default();

    use crate::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};
    let mut chunk_builder = D3ChunkBuilder::new();
    let chunk_size = 10;

    if ctx.map.vertices.is_empty() && ctx.map.geometry_objects.is_empty() {
        return;
    }

    let bbox = ctx.map.bbox();
    let min_chunk = Vec2::new(
        (bbox.min.x / chunk_size as f32).floor() as i32,
        (bbox.min.y / chunk_size as f32).floor() as i32,
    );
    let max_chunk = Vec2::new(
        (bbox.max.x / chunk_size as f32).floor() as i32,
        (bbox.max.y / chunk_size as f32).floor() as i32,
    );

    for cy in min_chunk.y..=max_chunk.y {
        for cx in min_chunk.x..=max_chunk.x {
            let chunk_origin = Vec2::new(cx, cy);
            let chunk_collision =
                chunk_builder.build_collision(&ctx.map, &ctx.assets, chunk_origin, chunk_size);
            ctx.collision_world
                .update_chunk(chunk_origin, chunk_collision);
        }
    }
}

fn rebuild_procedural_region(ctx: &mut RegionCtx, seed_arg: i64) -> bool {
    let debug_context_value = |ctx: &RegionCtx, key: &str| {
        ctx.get_region_value(key)
            .map(|value| value.to_string())
            .or_else(|| {
                RegionHost::get_config_value(&ctx.config, key).map(|value| value.to_string())
            })
            .unwrap_or_else(|| "<missing>".into())
    };

    let Some(mut cfg) = crate::procedural::parse_procedural_config_table(&ctx.config) else {
        ctx.send_log_message(format!(
            "[Procedural] {}: no [procedural] settings found",
            ctx.map.name
        ));
        return false;
    };
    cfg.apply_runtime_overrides(|key| ctx.get_region_value(key));
    if !cfg.enabled
        || !cfg.generator.eq_ignore_ascii_case("connected_rooms")
        || !cfg.mode.eq_ignore_ascii_case("2d")
    {
        ctx.send_log_message(format!(
            "[Procedural] {}: skipped generator='{}' mode='{}' enabled={}",
            ctx.map.name, cfg.generator, cfg.mode, cfg.enabled
        ));
        return false;
    }

    let run = ctx
        .get_region_value("procedural.run")
        .and_then(|value| match value {
            Value::Int(v) => Some(v.max(0) as u64),
            Value::UInt(v) => Some(v as u64),
            Value::Float(v) => Some(v.max(0.0) as u64),
            Value::Int64(v) => Some(v.max(0) as u64),
            _ => None,
        })
        .unwrap_or(0);
    if seed_arg > 0 {
        cfg.seed = seed_arg as u64;
    } else {
        let next_run = run.saturating_add(1);
        cfg.seed = cfg.seed.wrapping_add(next_run);
        ctx.set_region_value(
            "procedural.run",
            Value::Int(next_run.min(i32::MAX as u64) as i32),
        );
    }
    ctx.send_log_message(format!(
        "[Procedural] {}: rebuilding connected_rooms seed={} run={} depth={} rooms={} skeleton_pct={} size={}x{} room_size={}..{}",
        ctx.map.name,
        cfg.seed,
        run,
        debug_context_value(ctx, "dungeon.depth"),
        debug_context_value(ctx, "procedural.room_count"),
        debug_context_value(ctx, "procedural.characters.skeleton.percentage"),
        cfg.width,
        cfg.height,
        cfg.room_min_size,
        cfg.room_max_size
    ));

    let removed_item_ids = ctx.map.items.iter().map(|item| item.id).collect::<Vec<_>>();
    ctx.map.items.clear();
    for id in removed_item_ids {
        ctx.item_classes.remove(&id);
        ctx.item_state_data.remove(&id);
        ctx.item_proximity_alerts.remove(&id);
        ctx.notifications_items
            .retain(|(item_id, _, _)| *item_id != id);
        ctx.to_execute_item.retain(|(item_id, _, _)| *item_id != id);
    }

    let removed_entity_ids = ctx
        .map
        .entities
        .iter()
        .filter_map(|entity| {
            let is_player = entity
                .get_attr_string("class_name")
                .map(|class_name| ctx.entity_player_classes.contains(&class_name))
                .unwrap_or(false);
            if is_player {
                return None;
            }
            Some(entity.id)
        })
        .collect::<Vec<_>>();
    ctx.map
        .entities
        .retain(|entity| !removed_entity_ids.contains(&entity.id));
    for id in &removed_entity_ids {
        ctx.entity_classes.remove(id);
        ctx.entity_state_data.remove(id);
        ctx.entity_proximity_alerts.remove(id);
        ctx.notifications_entities
            .retain(|(entity_id, _, _)| entity_id != id);
        ctx.to_execute_entity
            .retain(|(entity_id, _, _)| entity_id != id);
        ctx.pending_entity_transfers
            .retain(|(entity_id, _, _)| entity_id != id);
    }
    ctx.active_choice_sessions.retain(|session| {
        !removed_entity_ids.contains(&session.from) && !removed_entity_ids.contains(&session.to)
    });

    let output = crate::procedural::bake_connected_rooms(&mut ctx.map, &ctx.assets.tiles, &cfg);
    let item_spawn_count = output.item_spawns.len();
    let character_spawn_count = output.character_spawns.len();

    for spawn in output.item_spawns {
        let Some(mut item) = ctx.create_item(spawn.name.clone()) else {
            continue;
        };
        item.set_position(spawn.position);
        item.set_attribute("procedural_generated", Value::Bool(true));
        item.set_attribute("procedural_kind", Value::Str(spawn.kind));
        item.mark_all_dirty();
        ctx.map.items.push(item);
    }

    for spawn in output.character_spawns {
        if !ctx.assets.entities.contains_key(&spawn.name) {
            continue;
        }
        let id = crate::server::region::get_global_id();
        let mut entity = Entity {
            id,
            position: spawn.position,
            ..Default::default()
        };
        entity.set_attribute("class_name", Value::Str(spawn.name.clone()));
        entity.set_attribute("name", Value::Str(spawn.name.clone()));
        entity.set_attribute("mode", Value::Str("active".into()));
        entity.set_attribute(
            "_source_seq",
            Value::Source(PixelSource::Sequence("idle".into())),
        );
        entity.set_attribute("procedural_generated", Value::Bool(true));
        entity.set_attribute("procedural_kind", Value::Str(spawn.kind));
        if let Some(data) = ctx.entity_class_data.get(&spawn.name) {
            crate::server::data::apply_entity_data(&mut entity, data);
            crate::server::region::apply_ruleset_character_defaults(&ctx.rules, &mut entity);
        }
        if let Some(Value::Int(inv_slots)) = entity.attributes.get("inventory_slots") {
            entity.inventory = vec![None; *inv_slots as usize];
        }
        if let Some(Value::Int(wealth)) = entity.attributes.get("wealth") {
            let _ = entity.add_base_currency(*wealth as i64, &ctx.currencies);
        }
        entity.mark_all_dirty();
        ctx.entity_classes.insert(entity.id, spawn.name);
        ctx.to_execute_entity
            .push((entity.id, "startup".into(), VMValue::zero()));
        ctx.map.entities.push(entity);
    }

    rebuild_runtime_navigation(ctx);
    let player_entity_ids = ctx
        .map
        .entities
        .iter()
        .filter_map(|entity| {
            let is_player = entity
                .get_attr_string("class_name")
                .map(|class_name| ctx.entity_player_classes.contains(&class_name))
                .unwrap_or(false);
            is_player.then_some((
                entity.id,
                entity.attributes.get_float_default("radius", 0.5).max(0.0) - 0.01,
            ))
        })
        .collect::<Vec<_>>();
    for (entity_id, radius) in player_entity_ids {
        if let Some(entrance_pos) = ctx.resolve_sector_spawn_position("entrance", radius) {
            if let Some(entity) = ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
            {
                entity.set_attribute("sector", Value::Str(String::new()));
                entity.set_attribute("sector_id", Value::Int64(-1));
                entity.set_pos_xz(entrance_pos);
                entity.mark_all_dirty();
            }
            ctx.check_player_for_section_change_id(entity_id);
        } else {
            ctx.send_log_message(format!(
                "[Procedural] {}: no walkable entrance position found for retained player id={}",
                ctx.map.name, entity_id
            ));
        }
    }
    ctx.procedural_spawn_guard = 8;
    ctx.send_log_message(format!(
        "[Procedural] {}: rebuilt vertices={} linedefs={} sectors={} surfaces={} items={} entities={} spawned_items={} spawned_characters={}",
        ctx.map.name,
        ctx.map.vertices.len(),
        ctx.map.linedefs.len(),
        ctx.map.sectors.len(),
        ctx.map.surfaces.len(),
        ctx.map.items.len(),
        ctx.map.entities.len(),
        item_spawn_count,
        character_spawn_count
    ));
    if let Some(sender) = ctx.from_sender.get() {
        let _ = sender.send(RegionMessage::MapUpdate(ctx.region_id, ctx.map.clone()));
    }
    true
}

fn convert_attr_value(key: &str, val: &VMValue, hint: Option<&Value>, health_attr: &str) -> Value {
    // Health is treated as integer gameplay state.
    if key == health_attr {
        return Value::Int(val.x as i32);
    }

    // Target IDs are used as numeric entity IDs in combat logic.
    // Preserve explicit empty-string clears used by scripts.
    if matches!(key, "target" | "attack_target") {
        if let Some(s) = val.as_string() {
            if s.is_empty() {
                return Value::Str(String::new());
            }
            if let Ok(id) = s.parse::<u32>() {
                return Value::UInt(id);
            }
            return Value::Str(s.to_string());
        }
        return Value::UInt(val.x.max(0.0) as u32);
    }
    val.to_value_with_hint(hint)
}

fn restore_entity_health_if_revived(entity: &mut Entity, health_attr: &str) {
    if entity.attributes.get_float_default(health_attr, 1.0) > 0.0 {
        return;
    }

    entity.set_attribute(health_attr, Value::Int(1));
}

impl<'a> RegionHost<'a> {
    fn eldrin_debug_target(&self) -> EldrinDebugTarget {
        eldrin_debug_target_for_ctx(self.ctx)
    }

    fn eldrin_debug_function(&self) -> &str {
        if self.ctx.current_debug_function.is_empty() {
            "event"
        } else {
            &self.ctx.current_debug_function
        }
    }

    fn push_debug_vm_value(&mut self, event: &str, x: u32, y: u32, value: &VMValue, error: bool) {
        let display = TheValue::Text(value.to_string());
        self.push_debug_text(event, x, y, display, error);
    }

    fn push_debug_text(&mut self, event: &str, x: u32, y: u32, display: TheValue, error: bool) {
        if let Some(item_id) = self.ctx.curr_item_id {
            self.ctx.debug.add_value(item_id, event, x, y, display);
            if error {
                self.ctx.debug.add_error(item_id, event, x, y);
            } else {
                self.ctx.debug.remove_error(item_id, event, x, y);
            }
        } else {
            self.ctx
                .debug
                .add_value(self.ctx.curr_entity_id, event, x, y, display);
            if error {
                self.ctx
                    .debug
                    .add_error(self.ctx.curr_entity_id, event, x, y);
            } else {
                self.ctx
                    .debug
                    .remove_error(self.ctx.curr_entity_id, event, x, y);
            }
        }
    }

    fn debug_return(&mut self, value: VMValue) -> Option<VMValue> {
        if self.ctx.debug_mode
            && let Some((event, x, y)) = self.ctx.curr_debug_loc.clone()
        {
            self.push_debug_vm_value(&event, x, y, &value, false);
            self.ctx.curr_debug_loc = None;
        }
        Some(value)
    }

    fn debug_return_bool(&mut self, value: bool) -> Option<VMValue> {
        let vm = VMValue::from_bool(value);
        if self.ctx.debug_mode
            && let Some((event, x, y)) = self.ctx.curr_debug_loc.clone()
        {
            self.push_debug_text(
                &event,
                x,
                y,
                TheValue::Text(if value { "True" } else { "False" }.into()),
                false,
            );
            self.ctx.curr_debug_loc = None;
        }
        Some(vm)
    }

    fn parse_route_names(attrs: &ValueContainer) -> Vec<String> {
        if let Some(Value::StrArray(values)) = attrs.get("route") {
            return values
                .iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        if let Some(value) = attrs.get_str("route") {
            let value = value.trim();
            if !value.is_empty() {
                return vec![value.to_string()];
            }
        }
        Vec::new()
    }

    fn resolve_route_points(
        map: &Map,
        route_names: &[String],
        start_pos: Vec2<f32>,
    ) -> Vec<Vec2<f32>> {
        #[derive(Clone)]
        struct Segment {
            start_id: u32,
            end_id: u32,
            start: Vec2<f32>,
            end: Vec2<f32>,
        }

        let mut points: Vec<Vec2<f32>> = Vec::new();
        let mut anchor = start_pos;

        for route_name in route_names {
            let name = route_name.trim();
            if name.is_empty() {
                continue;
            }

            let mut segments: Vec<Segment> = map
                .linedefs
                .iter()
                .filter(|ld| ld.name.eq_ignore_ascii_case(name))
                .filter_map(|ld| {
                    let a = map.get_vertex(ld.start_vertex)?;
                    let b = map.get_vertex(ld.end_vertex)?;
                    Some(Segment {
                        start_id: ld.start_vertex,
                        end_id: ld.end_vertex,
                        start: a,
                        end: b,
                    })
                })
                .collect();

            while !segments.is_empty() {
                let mut best_idx = 0usize;
                let mut best_dist = f32::MAX;
                let mut best_from_start = true;

                for (idx, seg) in segments.iter().enumerate() {
                    let ds = seg.start.distance_squared(anchor);
                    if ds < best_dist {
                        best_dist = ds;
                        best_idx = idx;
                        best_from_start = true;
                    }
                    let de = seg.end.distance_squared(anchor);
                    if de < best_dist {
                        best_dist = de;
                        best_idx = idx;
                        best_from_start = false;
                    }
                }

                let seed = segments.swap_remove(best_idx);
                let (mut current_vid, seed_start, seed_end) = if best_from_start {
                    (seed.end_id, seed.start, seed.end)
                } else {
                    (seed.start_id, seed.end, seed.start)
                };

                if points
                    .last()
                    .is_none_or(|last| last.distance_squared(seed_start) > 1e-8)
                {
                    points.push(seed_start);
                }
                points.push(seed_end);
                anchor = seed_end;

                loop {
                    let Some(next_idx) = segments
                        .iter()
                        .position(|seg| seg.start_id == current_vid || seg.end_id == current_vid)
                    else {
                        break;
                    };

                    let seg = segments.swap_remove(next_idx);
                    let next_point;
                    if seg.start_id == current_vid {
                        current_vid = seg.end_id;
                        next_point = seg.end;
                    } else {
                        current_vid = seg.start_id;
                        next_point = seg.start;
                    }
                    if points
                        .last()
                        .is_none_or(|last| last.distance_squared(next_point) > 1e-8)
                    {
                        points.push(next_point);
                    }
                    anchor = next_point;
                }
            }
        }

        points
    }

    fn nearest_point_index(from: Vec2<f32>, points: &[Vec2<f32>]) -> usize {
        let mut best_idx = 0usize;
        let mut best_dist = f32::MAX;
        for (idx, point) in points.iter().enumerate() {
            let d = from.distance_squared(*point);
            if d < best_dist {
                best_dist = d;
                best_idx = idx;
            }
        }
        best_idx
    }

    fn parse_target_arg_id(arg: &VMValue) -> Option<u32> {
        if let Some(s) = arg.as_string() {
            if let Ok(id) = s.parse::<u32>() {
                return Some(id);
            }
            return None;
        }
        Some(arg.x.max(0.0) as u32)
    }

    fn parse_palette_remap_2d_mode(arg: &VMValue) -> PaletteRemap2DMode {
        if let Some(s) = arg.as_string() {
            match s.trim().to_ascii_lowercase().as_str() {
                "off" | "disable" | "disabled" | "none" => PaletteRemap2DMode::Disabled,
                "luma" | "ramp" | "luma_ramp" => PaletteRemap2DMode::LumaRamp,
                "nearest" | "closest" => PaletteRemap2DMode::Nearest,
                "dither" | "dithered" | "dithered_ramp" | "bayer" => {
                    PaletteRemap2DMode::DitheredRamp
                }
                _ => PaletteRemap2DMode::Disabled,
            }
        } else {
            match arg.x.round() as i32 {
                1 => PaletteRemap2DMode::LumaRamp,
                2 => PaletteRemap2DMode::Nearest,
                3 => PaletteRemap2DMode::DitheredRamp,
                _ => PaletteRemap2DMode::Disabled,
            }
        }
    }

    fn split_context_path(path: &str) -> Option<(&str, &str)> {
        let (root, key) = path.split_once('.')?;
        if matches!(root, "world" | "region") && !key.is_empty() {
            Some((root, Self::normalize_context_key(key)))
        } else {
            None
        }
    }

    fn normalize_context_key(key: &str) -> &str {
        match key {
            "render.pal.start" => "render.palette_remap.start",
            "render.pal.end" => "render.palette_remap.end",
            "render.pal.mode" => "render.palette_remap.mode",
            "render.pal.blend" => "render.palette_remap.blend",
            _ => key,
        }
    }

    fn toml_to_runtime_value(value: &toml::Value) -> Option<Value> {
        match value {
            toml::Value::Boolean(value) => Some(Value::Bool(*value)),
            toml::Value::Integer(value) => i32::try_from(*value)
                .map(Value::Int)
                .ok()
                .or_else(|| Some(Value::Int64(*value))),
            toml::Value::Float(value) => Some(Value::Float(*value as f32)),
            toml::Value::String(value) => Some(Value::Str(value.clone())),
            toml::Value::Array(values) => values
                .iter()
                .map(|value| value.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
                .map(Value::StrArray),
            _ => None,
        }
    }

    fn get_config_value(config: &toml::Table, key: &str) -> Option<Value> {
        let mut parts = key.split('.').filter(|part| !part.is_empty());
        let first = parts.next()?;
        let mut value = config.get(first)?;
        for part in parts {
            value = value.as_table()?.get(part)?;
        }
        Self::toml_to_runtime_value(value)
    }

    fn get_context_value(&self, path: &str) -> Option<Value> {
        let (root, key) = Self::split_context_path(path)?;
        match root {
            "world" => RegionCtx::get_world_value(key),
            "region" => self
                .ctx
                .get_region_value(key)
                .or_else(|| Self::get_config_value(&self.ctx.config, key)),
            _ => None,
        }
    }

    fn set_context_value(&mut self, path: &str, value: Value) {
        let Some((root, key)) = Self::split_context_path(path) else {
            return;
        };

        match root {
            "world" => RegionCtx::set_world_value(key, value.clone()),
            "region" => {
                self.ctx.set_region_value(key, value.clone());
            }
            _ => return,
        }

        self.apply_render_context_value(root, key, &value);
    }

    fn apply_render_context_value(&mut self, root: &str, key: &str, value: &Value) {
        let Some(sender) = self.ctx.from_sender.get() else {
            return;
        };

        match key {
            "render.palette_remap.start"
            | "render.palette_remap.start_index"
            | "render.palette_remap.end"
            | "render.palette_remap.end_index"
            | "render.palette_remap.mode"
            | "render.palette_remap.blend" => {
                let start_index = self
                    .get_context_value(&format!("{root}.render.palette_remap.start"))
                    .or_else(|| {
                        self.get_context_value(&format!("{root}.render.palette_remap.start_index"))
                    })
                    .and_then(|v| Self::value_to_u32(&v))
                    .unwrap_or(0)
                    .min(255);
                let end_index = self
                    .get_context_value(&format!("{root}.render.palette_remap.end"))
                    .or_else(|| {
                        self.get_context_value(&format!("{root}.render.palette_remap.end_index"))
                    })
                    .and_then(|v| Self::value_to_u32(&v))
                    .unwrap_or(0)
                    .min(255);
                let mode = self
                    .get_context_value(&format!("{root}.render.palette_remap.mode"))
                    .map(|v| Self::parse_palette_remap_2d_mode(&VMValue::from_value(&v)))
                    .unwrap_or(PaletteRemap2DMode::Disabled);
                let blend = self
                    .get_context_value(&format!("{root}.render.palette_remap.blend"))
                    .and_then(|v| Self::value_to_f32(&v))
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);

                let _ = match root {
                    "world" => sender.send(RegionMessage::SetWorldPaletteRemap2D(
                        start_index,
                        end_index,
                        mode,
                    )),
                    "region" => sender.send(RegionMessage::SetPaletteRemap2D(
                        self.ctx.region_id,
                        start_index,
                        end_index,
                        mode,
                    )),
                    _ => Ok(()),
                };
                let _ = match root {
                    "world" => sender.send(RegionMessage::SetWorldPaletteRemap2DBlend(blend)),
                    "region" => sender.send(RegionMessage::SetPaletteRemap2DBlend(
                        self.ctx.region_id,
                        blend,
                    )),
                    _ => Ok(()),
                };
            }
            _ if key.starts_with("render.") => {
                let name = key.trim_start_matches("render.").to_string();
                let _ = match root {
                    "world" => sender.send(RegionMessage::SetWorldRenderValue(name, value.clone())),
                    "region" => sender.send(RegionMessage::SetRenderValue(
                        self.ctx.region_id,
                        name,
                        value.clone(),
                    )),
                    _ => Ok(()),
                };
            }
            _ if key.starts_with("post.") => {
                let name = key.trim_start_matches("post.").to_string();
                let _ = match root {
                    "world" => sender.send(RegionMessage::SetWorldPostValue(name, value.clone())),
                    "region" => sender.send(RegionMessage::SetPostValue(
                        self.ctx.region_id,
                        name,
                        value.clone(),
                    )),
                    _ => Ok(()),
                };
            }
            _ => {}
        }
    }

    fn value_to_f32(value: &Value) -> Option<f32> {
        match value {
            Value::Float(v) => Some(*v),
            Value::Int(v) => Some(*v as f32),
            Value::UInt(v) => Some(*v as f32),
            Value::Int64(v) => Some(*v as f32),
            Value::Str(s) => s.parse::<f32>().ok(),
            _ => None,
        }
    }

    fn value_to_u32(value: &Value) -> Option<u32> {
        match value {
            Value::UInt(v) => Some(*v),
            Value::Int(v) if *v >= 0 => Some(*v as u32),
            Value::Int64(v) if *v >= 0 => Some(*v as u32),
            Value::Float(v) if *v >= 0.0 => Some(v.round() as u32),
            Value::Str(s) => s.parse::<u32>().ok(),
            _ => None,
        }
    }

    fn get_current_target_id(&mut self) -> Option<u32> {
        // Current item target first
        if let Some(item_id) = self.ctx.curr_item_id
            && let Some(item) = self.ctx.get_item_mut(item_id)
        {
            if let Some(id) = item.attributes.get_uint("target") {
                return Some(id);
            }
            if let Some(id) = item.attributes.get_uint("attack_target") {
                return Some(id);
            }
            if let Some(s) = item.attributes.get_str("target")
                && let Ok(id) = s.parse::<u32>()
            {
                return Some(id);
            }
            if let Some(s) = item.attributes.get_str("attack_target")
                && let Ok(id) = s.parse::<u32>()
            {
                return Some(id);
            }
        }

        // Otherwise current entity target
        if let Some(entity) = self.ctx.get_current_entity_mut() {
            if let Some(id) = entity.attributes.get_uint("target") {
                return Some(id);
            }
            if let Some(id) = entity.attributes.get_uint("attack_target") {
                return Some(id);
            }
            if let Some(s) = entity.attributes.get_str("target")
                && let Ok(id) = s.parse::<u32>()
            {
                return Some(id);
            }
            if let Some(s) = entity.attributes.get_str("attack_target")
                && let Ok(id) = s.parse::<u32>()
            {
                return Some(id);
            }
        }

        None
    }

    fn configured_weapon_slots(&self) -> Vec<String> {
        self.ctx
            .config
            .get("game")
            .and_then(toml::Value::as_table)
            .and_then(|game| game.get("weapon_slots"))
            .and_then(toml::Value::as_array)
            .map(|slots| {
                slots
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(|slot| slot.trim().to_ascii_lowercase())
                    .filter(|slot| !slot.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| vec!["main_hand".into(), "off_hand".into()])
    }

    fn current_attack_source_item_id(&self) -> Option<u32> {
        if let Some(item_id) = self.ctx.curr_item_id {
            return Some(item_id);
        }

        let entity = self
            .ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == self.ctx.curr_entity_id)?;

        let configured_slots = self.configured_weapon_slots();
        for slot in &configured_slots {
            if let Some((_, item)) = entity
                .equipped
                .iter()
                .find(|(equipped_slot, _)| equipped_slot.trim().eq_ignore_ascii_case(slot))
            {
                return Some(item.id);
            }
        }

        entity
            .equipped
            .iter()
            .find(|(slot, _)| {
                matches!(
                    slot.trim().to_ascii_lowercase().as_str(),
                    "main_hand"
                        | "mainhand"
                        | "weapon"
                        | "hand_main"
                        | "off_hand"
                        | "offhand"
                        | "hand_off"
                )
            })
            .map(|(_, item)| item.id)
    }

    fn current_attack_base_damage(&self) -> i32 {
        current_attack_base_damage_for_entity(self.ctx, self.ctx.curr_entity_id)
    }

    fn current_attack_kind(&self, source_item_id: Option<u32>) -> String {
        let attacker = self
            .ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == self.ctx.curr_entity_id);

        if let Some(kind) = source_item_id
            .and_then(|item_id| attacker.and_then(|entity| entity_item_by_id(entity, item_id)))
            .and_then(|item| item.attributes.get_str("damage_kind"))
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
        {
            return kind.to_string();
        }

        "physical".to_string()
    }

    fn queue_damage(
        &mut self,
        target_id: Option<u32>,
        base_dmg: i32,
        kind: &str,
        source_item_id: Option<u32>,
    ) {
        if let Some(id) = target_id {
            if self
                .ctx
                .map
                .entities
                .iter()
                .any(|entity| entity.id == id && entity.get_mode() == "dead")
            {
                return;
            }

            let attacker_id = self.ctx.curr_entity_id;
            if !consume_attack_ammunition_for_source(self.ctx, attacker_id, source_item_id) {
                return;
            }
            let source_item_id = source_item_id.unwrap_or(0);
            let dmg = apply_damage_rules(self.ctx, id, attacker_id, base_dmg, kind, source_item_id);
            if self.ctx.curr_item_id.is_none() && dmg > 0 {
                if let Some(attacker) = self.ctx.get_current_entity_mut() {
                    let attack_time = attacker
                        .attributes
                        .get_float_default("avatar_attack_time", 0.35)
                        .max(0.05);
                    attacker.set_attribute("avatar_attack_left", Value::Float(attack_time));
                }
            }
            let autodamage = self
                .ctx
                .map
                .entities
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.attributes.get_bool_default("autodamage", false))
                .unwrap_or(false);

            if autodamage {
                _ = apply_damage_direct(
                    self.ctx,
                    id,
                    attacker_id,
                    dmg,
                    kind,
                    if source_item_id > 0 {
                        Some(source_item_id)
                    } else {
                        None
                    },
                );
            } else {
                let source_item_id = source_item_id as f32;
                self.ctx.to_execute_entity.push((
                    id,
                    "damaged".into(),
                    VMValue::new_with_string(attacker_id as f32, dmg as f32, source_item_id, kind),
                ));
            }
            if self.ctx.debug_mode {
                add_debug_value(
                    &mut self.ctx,
                    TheValue::Text(format!("{} dmg", dmg.max(0))),
                    false,
                );
            }
        } else if self.ctx.debug_mode {
            add_debug_value(&mut self.ctx, TheValue::Text("No Target".into()), true);
        }
    }

    fn set_current_target_id(&mut self, target_id: Option<u32>) {
        if let Some(item_id) = self.ctx.curr_item_id {
            if let Some(item) = self.ctx.get_item_mut(item_id) {
                if let Some(id) = target_id {
                    item.set_attribute("target", Value::UInt(id));
                    item.set_attribute("attack_target", Value::UInt(id));
                } else {
                    item.set_attribute("target", Value::Str(String::new()));
                    item.set_attribute("attack_target", Value::Str(String::new()));
                }
            }
            return;
        }

        if let Some(entity) = self.ctx.get_current_entity_mut() {
            if let Some(id) = target_id {
                entity.set_attribute("target", Value::UInt(id));
                entity.set_attribute("attack_target", Value::UInt(id));
            } else {
                entity.set_attribute("target", Value::Str(String::new()));
                entity.set_attribute("attack_target", Value::Str(String::new()));
            }
        }
    }

    fn attack_cooldown_seconds(&self) -> f32 {
        self.ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == self.ctx.curr_entity_id)
            .map(|entity| current_attack_cooldown_for_entity(self.ctx, entity))
            .unwrap_or(1.0)
    }

    fn try_start_attack_cooldown(&mut self) -> bool {
        if self.ctx.curr_item_id.is_some() {
            return true;
        }

        let entity_id = self.ctx.curr_entity_id;
        let key = "intent: attack";
        if let Some(state) = self.ctx.entity_state_data.get(&entity_id)
            && let Some(Value::Int64(tick)) = state.get(key)
            && *tick > self.ctx.ticks
        {
            return false;
        }

        let cooldown_seconds = self.attack_cooldown_seconds();
        let cooldown_ticks = RegionInstance::realtime_seconds_to_ticks(self.ctx, cooldown_seconds);
        if cooldown_ticks <= 0 {
            return true;
        }

        let state = self.ctx.entity_state_data.entry(entity_id).or_default();
        state.set(key, Value::Int64(self.ctx.ticks + cooldown_ticks));
        state.set(
            &format!("__cooldown_total:{}", key),
            Value::Float(cooldown_seconds),
        );
        set_entity_cooldown_attrs(self.ctx, entity_id, key, cooldown_seconds, cooldown_seconds);
        true
    }

    fn has_valid_target(&mut self) -> bool {
        let Some(target_id) = self.get_current_target_id() else {
            return false;
        };
        self.ctx.map.entities.iter().any(|e| {
            e.id == target_id && e.attributes.get_str_default("mode", "active".into()) != "dead"
        })
    }

    fn parse_spell_target_arg(arg: &VMValue) -> Option<SpellTargetArg> {
        if let Some(s) = arg.as_string() {
            if let Ok(id) = s.parse::<u32>() {
                return Some(SpellTargetArg::Entity(id));
            }
            return None;
        }

        // Existing VM style encodes vectors in x/y/z.
        // If y/z are non-zero, treat it as a world position.
        if arg.y != 0.0 || arg.z != 0.0 {
            return Some(SpellTargetArg::Position(Vec3::new(arg.x, arg.y, arg.z)));
        }

        Some(SpellTargetArg::Entity(arg.x.max(0.0) as u32))
    }
}

fn eldrin_debug_target_for_ctx(ctx: &RegionCtx) -> EldrinDebugTarget {
    match ctx.current_script_scope {
        ScriptScope::World => EldrinDebugTarget::World,
        ScriptScope::Region => EldrinDebugTarget::Region(ctx.region_id),
        ScriptScope::Item => ctx
            .curr_item_id
            .map(EldrinDebugTarget::Item)
            .unwrap_or(EldrinDebugTarget::Region(ctx.region_id)),
        ScriptScope::Entity => EldrinDebugTarget::Entity(ctx.curr_entity_id),
    }
}

impl<'a> HostHandler for RegionHost<'a> {
    fn on_debug_line(&mut self, line: usize) {
        if !self.ctx.debug_mode {
            return;
        }
        let target = self.eldrin_debug_target();
        let function = self.eldrin_debug_function().to_string();
        self.ctx.eldrin_debug.mark_executed(target, &function, line);
    }

    fn on_debug_value(&mut self, line: usize, name: &str, value: &VMValue) {
        if !self.ctx.debug_mode {
            return;
        }
        let target = self.eldrin_debug_target();
        let function = self.eldrin_debug_function().to_string();
        self.ctx
            .eldrin_debug
            .add_value(target, &function, line, name, value.clone());
    }

    fn on_debug_branch(&mut self, line: usize, taken: bool) {
        if !self.ctx.debug_mode {
            return;
        }
        let target = self.eldrin_debug_target();
        let function = self.eldrin_debug_function().to_string();
        self.ctx
            .eldrin_debug
            .mark_branch(target, &function, line, taken);
    }

    fn on_host_call(&mut self, name: &str, args: &[VMValue]) -> Option<VMValue> {
        match name {
            "action" => {
                if let Some(s) = args.get(0).and_then(|v| v.as_string()) {
                    if let Ok(action) = s.parse::<EntityAction>() {
                        if let Some(ent) = self
                            .ctx
                            .map
                            .entities
                            .iter_mut()
                            .find(|e| e.id == self.ctx.curr_entity_id)
                        {
                            ent.action = action;
                        }
                    }
                }
            }
            "intent" => {
                if let Some(s) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(ent) = self
                        .ctx
                        .map
                        .entities
                        .iter_mut()
                        .find(|e| e.id == self.ctx.curr_entity_id)
                    {
                        ent.set_attribute("intent", Value::Str(s.to_string()));
                    }
                }
            }
            "world_event" => {
                if let (Some(event), Some(value)) =
                    (args.first().and_then(|v| v.as_string()), args.get(1))
                {
                    self.ctx
                        .to_execute_world
                        .push((event.to_string(), value.clone()));

                    if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Text("Ok".into()), false);
                    }
                }
            }
            "teleport_entity" => {
                if let (Some(entity_id), Some(dest)) = (
                    args.first().map(|v| v.x as u32),
                    args.get(1).and_then(|v| v.as_string()),
                ) {
                    let region_name = args.get(2).and_then(|v| v.as_string()).unwrap_or("");

                    if region_name.trim().is_empty()
                        || region_name.trim().eq_ignore_ascii_case(&self.ctx.map.name)
                    {
                        let radius = self
                            .ctx
                            .map
                            .entities
                            .iter()
                            .find(|e| e.id == entity_id)
                            .map(|entity| {
                                entity.attributes.get_float_default("radius", 0.5).max(0.0) - 0.01
                            })
                            .unwrap_or(0.49);
                        if let Some(center) = self.ctx.resolve_sector_spawn_position(&dest, radius)
                        {
                            if let Some(entity) =
                                self.ctx.map.entities.iter_mut().find(|e| e.id == entity_id)
                            {
                                entity.set_pos_xz(center);
                                entity.mark_all_dirty();
                                self.ctx.check_player_for_section_change_id(entity_id);
                                if let Some(sender) = self.ctx.from_sender.get() {
                                    let _ = sender.send(RegionMessage::MapUpdate(
                                        self.ctx.region_id,
                                        self.ctx.map.clone(),
                                    ));
                                }
                            }
                        } else if self.ctx.debug_mode {
                            add_debug_value(
                                &mut self.ctx,
                                TheValue::Text("Unknown Sector".into()),
                                true,
                            );
                        }
                    } else {
                        self.ctx.pending_entity_transfers.push((
                            entity_id,
                            region_name.to_string(),
                            dest.to_string(),
                        ));
                    }
                }
            }
            "build_procedural" => {
                let seed = args.first().map(|v| v.x as i64).unwrap_or(0);
                let ok = rebuild_procedural_region(self.ctx, seed);
                return self.debug_return_bool(ok);
            }
            "message" => {
                if let (Some(receiver), Some(msg)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let category = args
                        .get(2)
                        .and_then(|v| v.as_string())
                        .unwrap_or("")
                        .to_string();

                    let mut entity_id = Some(self.ctx.curr_entity_id);
                    let item_id = self.ctx.curr_item_id;
                    if item_id.is_some() {
                        entity_id = None;
                    }

                    let msg = RegionMessage::Message(
                        self.ctx.region_id,
                        entity_id,
                        item_id,
                        receiver.x as u32,
                        msg.to_string(),
                        category,
                    );
                    if let Some(sender) = self.ctx.from_sender.get() {
                        let _ = sender.send(msg);
                    }

                    if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Text("Ok".into()), false);
                    }
                }
            }
            "say" => {
                if let Some(msg) = args.get(0).and_then(|v| v.as_string()) {
                    let category = args
                        .get(1)
                        .and_then(|v| v.as_string())
                        .unwrap_or("")
                        .to_string();
                    let mut entity_id = Some(self.ctx.curr_entity_id);
                    let item_id = self.ctx.curr_item_id;
                    if item_id.is_some() {
                        entity_id = None;
                    }

                    let msg = RegionMessage::Say(
                        self.ctx.region_id,
                        entity_id,
                        item_id,
                        msg.to_string(),
                        category,
                    );
                    if let Some(sender) = self.ctx.from_sender.get() {
                        let _ = sender.send(msg);
                    }

                    if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Text("Ok".into()), false);
                    }
                }
            }
            "set_target" => {
                let target_id = args
                    .first()
                    .and_then(Self::parse_target_arg_id)
                    .filter(|id| self.ctx.map.entities.iter().any(|e| e.id == *id));
                if let Some(target_id) = target_id {
                    self.set_current_target_id(Some(target_id));
                    return self.debug_return_bool(true);
                }
                return self.debug_return_bool(false);
            }
            "clear_target" => {
                self.set_current_target_id(None);
                return self.debug_return_bool(true);
            }
            "target" => {
                if let Some(target_id) = self.get_current_target_id() {
                    return self.debug_return(VMValue::from_u32(target_id));
                }
                return self.debug_return(VMValue::zero());
            }
            "has_target" => {
                let has_target = self.has_valid_target();
                return self.debug_return_bool(has_target);
            }
            "play_audio" => {
                if let Some(name) = args.first().and_then(|v| v.as_string()) {
                    let bus = args
                        .get(1)
                        .and_then(|v| v.as_string())
                        .unwrap_or("sfx")
                        .to_string();
                    let gain = args.get(2).map(|v| v.x).unwrap_or(1.0).clamp(0.0, 4.0);
                    let looping = args.get(3).map(|v| v.to_bool()).unwrap_or(false);

                    let msg = RegionMessage::AudioCmd(
                        self.ctx.region_id,
                        AudioCommand::Play {
                            name: name.to_string(),
                            bus,
                            gain,
                            looping,
                        },
                    );

                    if let Some(sender) = self.ctx.from_sender.get() {
                        let _ = sender.send(msg);
                    }
                }
            }
            "clear_audio" => {
                let cmd = if let Some(bus) = args.first().and_then(|v| v.as_string()) {
                    if bus.is_empty() {
                        AudioCommand::ClearAll
                    } else {
                        AudioCommand::ClearBus {
                            bus: bus.to_string(),
                        }
                    }
                } else {
                    AudioCommand::ClearAll
                };

                if let Some(sender) = self.ctx.from_sender.get() {
                    let _ = sender.send(RegionMessage::AudioCmd(self.ctx.region_id, cmd));
                }
            }
            "set_audio_bus_volume" => {
                if let (Some(bus), Some(volume)) =
                    (args.first().and_then(|v| v.as_string()), args.get(1))
                {
                    let cmd = AudioCommand::SetBusVolume {
                        bus: bus.to_string(),
                        volume: volume.x.clamp(0.0, 4.0),
                    };
                    if let Some(sender) = self.ctx.from_sender.get() {
                        let _ = sender.send(RegionMessage::AudioCmd(self.ctx.region_id, cmd));
                    }
                }
            }
            "cast_spell" => {
                if let (Some(template), Some(target_arg)) =
                    (args.first().and_then(|v| v.as_string()), args.get(1))
                {
                    let caster_id = self.ctx.curr_entity_id;
                    if is_spell_on_cooldown(self.ctx, caster_id, template) {
                        return Some(VMValue::from_i32(-1));
                    }

                    let success_pct = args.get(2).map(|v| v.x).unwrap_or(100.0).clamp(0.0, 100.0);
                    let mut rng = rand::rng();
                    let roll = rng.random_range(0.0..100.0);
                    if roll >= success_pct {
                        // Optional event for scripts reacting to failed casts.
                        if let Some(item_id) = self.ctx.curr_item_id {
                            self.ctx.to_execute_item.push((
                                item_id,
                                "cast_failed".into(),
                                VMValue::zero(),
                            ));
                        } else {
                            self.ctx.to_execute_entity.push((
                                self.ctx.curr_entity_id,
                                "cast_failed".into(),
                                VMValue::zero(),
                            ));
                        }
                        return Some(VMValue::from_i32(-1));
                    }

                    let Some(mut spell_item) = self.ctx.create_item(template.to_string()) else {
                        return Some(VMValue::from_i32(-1));
                    };
                    let had_cast_height = spell_item.attributes.contains("spell_cast_height");
                    spell_item.set_attribute("is_spell", Value::Bool(true));
                    if spell_item.attributes.get("visible").is_none() {
                        spell_item.set_attribute("visible", Value::Bool(true));
                    }
                    apply_spell_default_attrs(&mut spell_item);

                    spell_item.set_attribute("spell_caster_id", Value::UInt(caster_id));

                    let mut spawn_pos = Vec3::new(0.0, 0.0, 0.0);
                    let mut caster_dir = Vec2::new(1.0, 0.0);
                    let mut is_firstp = false;
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            spawn_pos = item.position;
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        spawn_pos = entity.position;
                        caster_dir = entity.orientation;
                        is_firstp = matches!(
                            entity.attributes.get("player_camera"),
                            Some(Value::PlayerCamera(
                                PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid
                            ))
                        );
                    }
                    let flight_height = spell_item
                        .attributes
                        .get_float_default("spell_flight_height", 0.5);
                    spawn_pos.y = flight_height;
                    let cast_time = spell_item
                        .attributes
                        .get_float_default("spell_cast_time", 0.0)
                        .max(0.0);
                    let cast_offset = spell_item
                        .attributes
                        .get_float_default("spell_cast_offset", 0.6)
                        .max(0.0);
                    let mut cast_height = spell_item
                        .attributes
                        .get_float_default("spell_cast_height", flight_height);
                    if is_firstp {
                        if !had_cast_height {
                            cast_height = cast_height.max(1.4);
                        }
                    }

                    let Some(target_arg) = Self::parse_spell_target_arg(target_arg) else {
                        return Some(VMValue::from_i32(-1));
                    };

                    let target_pos = match target_arg {
                        SpellTargetArg::Entity(target_id) => {
                            if let Some(target) =
                                self.ctx.map.entities.iter().find(|e| e.id == target_id)
                            {
                                spell_item.set_attribute("spell_target_id", Value::UInt(target_id));
                                target.position
                            } else {
                                return Some(VMValue::from_i32(-1));
                            }
                        }
                        SpellTargetArg::Position(pos) => {
                            spell_item.set_attribute("spell_target_x", Value::Float(pos.x));
                            spell_item.set_attribute("spell_target_y", Value::Float(flight_height));
                            spell_item.set_attribute("spell_target_z", Value::Float(pos.z));
                            pos
                        }
                    };

                    let mut dir = Vec2::new(target_pos.x - spawn_pos.x, target_pos.z - spawn_pos.z);
                    if dir.magnitude_squared() <= 1e-6 {
                        dir = caster_dir;
                    }
                    if dir.magnitude_squared() <= 1e-6 {
                        dir = Vec2::new(1.0, 0.0);
                    }
                    dir = dir.normalized();
                    if self.ctx.curr_item_id.is_none()
                        && let Some(entity) = self.ctx.get_current_entity_mut()
                    {
                        entity.set_orientation(dir);
                    }

                    spell_item.set_attribute("spell_dir_x", Value::Float(dir.x));
                    spell_item.set_attribute("spell_dir_z", Value::Float(dir.y));
                    spell_item.set_attribute("spell_travel", Value::Float(0.0));

                    let lifetime = spell_item
                        .attributes
                        .get_float_default("spell_lifetime", 3.0);
                    spell_item.set_attribute("spell_lifetime_left", Value::Float(lifetime));
                    if cast_time > 0.0 {
                        let hold_pos = Vec3::new(
                            spawn_pos.x + dir.x * cast_offset,
                            cast_height,
                            spawn_pos.z + dir.y * cast_offset,
                        );
                        spell_item.set_attribute("spell_casting", Value::Bool(true));
                        spell_item.set_attribute("spell_cast_left", Value::Float(cast_time));
                        spell_item.set_attribute("spell_cast_height", Value::Float(cast_height));
                        spell_item.set_attribute("spell_cast_offset", Value::Float(cast_offset));
                        spell_item.set_position(hold_pos);
                        if let Some(caster_mut) =
                            self.ctx.map.entities.iter_mut().find(|e| e.id == caster_id)
                        {
                            caster_mut.set_attribute("spell_casting", Value::Bool(true));
                        }
                    } else {
                        spell_item.set_position(spawn_pos);
                    }
                    spell_item.mark_all_dirty();
                    let spell_id = spell_item.id;
                    let cooldown = spell_item
                        .attributes
                        .get_float_default("spell_cooldown", 0.0)
                        .max(0.0);
                    let on_cast_message = spell_item
                        .attributes
                        .get_str("on_cast")
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());
                    self.ctx.map.items.push(spell_item);
                    if let Some(message) = on_cast_message
                        && let Some(sender) = self.ctx.from_sender.get()
                    {
                        let _ = sender.send(RegionMessage::Message(
                            self.ctx.region_id,
                            Some(caster_id),
                            None,
                            caster_id,
                            message,
                            "system".into(),
                        ));
                    }
                    set_spell_cooldown(self.ctx, caster_id, template, cooldown);
                    return Some(VMValue::from_i32(spell_id as i32));
                }
                return Some(VMValue::from_i32(-1));
            }
            "set_player_camera" => {
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if let Some(camera) = args.get(0).and_then(|v| v.as_string()) {
                        let player_camera = match camera {
                            "2d_grid" => PlayerCamera::D2Grid,
                            "iso" => PlayerCamera::D3Iso,
                            "iso_grid" => PlayerCamera::D2Grid,
                            "firstp" => PlayerCamera::D3FirstP,
                            "firstp_grid" => PlayerCamera::D3FirstPGrid,
                            _ => PlayerCamera::D2,
                        };
                        entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
                    }
                }
            }
            "get_context_var" => {
                if let Some(path) = args.first().and_then(|v| v.as_string()) {
                    let value = self
                        .get_context_value(path)
                        .map(|v| VMValue::from_value(&v))
                        .unwrap_or_else(VMValue::zero);
                    return self.debug_return(value);
                }
                return self.debug_return(VMValue::zero());
            }
            "set_context_var" => {
                if let (Some(path), Some(value)) =
                    (args.first().and_then(|v| v.as_string()), args.get(1))
                {
                    let existing = self.get_context_value(path);
                    self.set_context_value(path, value.to_value_with_hint(existing.as_ref()));
                }
            }
            "set_debug_loc" => {
                if let (Some(event), Some(x), Some(y)) = (
                    args.get(0).and_then(|v| v.as_string()),
                    args.get(1),
                    args.get(2),
                ) {
                    let x = x.x as u32;
                    let y = y.x as u32;
                    if let Some(item_id) = self.ctx.curr_item_id {
                        self.ctx.debug.mark_executed(item_id, event, x, y);
                    } else {
                        self.ctx
                            .debug
                            .mark_executed(self.ctx.curr_entity_id, event, x, y);
                    }
                    self.ctx.curr_debug_loc = Some((event.to_string(), x, y));
                }
            }
            "set_debug_value" => {
                if let (Some(event), Some(x), Some(y), Some(value)) = (
                    args.get(0).and_then(|v| v.as_string()),
                    args.get(1),
                    args.get(2),
                    args.get(3),
                ) {
                    self.push_debug_vm_value(event, x.x as u32, y.x as u32, value, false);
                }
            }
            "set_debug_condition" => {
                if let (Some(event), Some(x), Some(y), Some(value)) = (
                    args.get(0).and_then(|v| v.as_string()),
                    args.get(1),
                    args.get(2),
                    args.get(3),
                ) {
                    let taken = value.to_bool();
                    let display = TheValue::Text(if taken { "True" } else { "False" }.into());
                    if let Some(item_id) = self.ctx.curr_item_id {
                        self.ctx
                            .debug
                            .mark_condition(item_id, event, x.x as u32, y.x as u32, taken, display);
                    } else {
                        self.ctx.debug.mark_condition(
                            self.ctx.curr_entity_id,
                            event,
                            x.x as u32,
                            y.x as u32,
                            taken,
                            display,
                        );
                    }
                }
            }
            "mark_debug_header" => {
                if let Some(event) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        self.ctx.debug.mark_header_executed(item_id, event);
                    } else {
                        self.ctx
                            .debug
                            .mark_header_executed(self.ctx.curr_entity_id, event);
                    }
                }
            }
            "set_tile" => {
                if let Some(mode) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(source) = crate::server::data::parse_tile_source_from_str(mode) {
                        if let Some(item_id) = self.ctx.curr_item_id {
                            if let Some(item) = self.ctx.get_item_mut(item_id) {
                                item.set_attribute("source", Value::Source(source.clone()));
                            }
                        } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                            entity.set_attribute("source", Value::Source(source));
                        }
                    }
                }
            }
            "set_emit_light" => {
                let active = args.get(0).map(|v| v.to_bool()).unwrap_or(false);
                if let Some(item_id) = self.ctx.curr_item_id {
                    if let Some(item) = self.ctx.get_item_mut(item_id) {
                        if let Some(Value::Light(light)) = item.attributes.get_mut("light") {
                            light.active = active;
                            item.mark_dirty_attribute("light");
                        }
                    }
                } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if let Some(Value::Light(light)) = entity.attributes.get_mut("light") {
                        light.active = active;
                        entity.mark_dirty_attribute("light");
                    }
                }
            }
            "set_attr" => {
                if let (Some(key), Some(val)) =
                    (args.get(0).and_then(|v| v.as_string()), args.get(1))
                {
                    let health_attr = self.ctx.health_attr.clone();
                    if let Some(item_id) = self.ctx.curr_item_id {
                        let mut geometry_object_attr = None;
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            // Single conversion path with optional type hints (string tag or attr type).
                            let converted = convert_attr_value(
                                key,
                                val,
                                item.attributes.get(key),
                                &health_attr,
                            );
                            item.set_attribute(key, converted);
                            if matches!(key, "visible" | "blocking")
                                && let Some(object_id) =
                                    item.attributes.get_id("geometry_object_id")
                            {
                                geometry_object_attr = Some((
                                    object_id,
                                    item.attributes.get_bool_default(key, key == "visible"),
                                ));
                            }

                            let (queue_active, queued_id, active_val) = if key == "active" {
                                let active = item.attributes.get_bool_default("active", false);
                                (
                                    true,
                                    item.id,
                                    if active {
                                        VMValue::from_bool(true)
                                    } else {
                                        VMValue::from_bool(false)
                                    },
                                )
                            } else {
                                (false, 0, VMValue::zero())
                            };

                            if key == "blocking" {
                                let blocking = item.attributes.get_bool_default("blocking", false);
                                if let Some(group_id) = item.attributes.get_id("door_group_id") {
                                    for sector in &self.ctx.map.sectors {
                                        if sector.properties.get_id("door_group_id")
                                            == Some(group_id)
                                            && sector
                                                .properties
                                                .get_str_default("dungeon_part", String::new())
                                                == "door_panel"
                                        {
                                            self.ctx.collision_world.set_opening_state(
                                                GeoId::Sector(sector.id),
                                                !blocking,
                                            );
                                        }
                                    }
                                } else if let Some(geo_id) = opening_geo_for_item(item) {
                                    // True blocking => not passable
                                    self.ctx
                                        .collision_world
                                        .set_opening_state(geo_id, !blocking);
                                }
                            }

                            if queue_active {
                                self.ctx.to_execute_item.push((
                                    queued_id,
                                    "active".into(),
                                    active_val,
                                ));
                            }
                        }
                        if let Some((object_id, value)) = geometry_object_attr {
                            apply_geometry_object_item_attr(self.ctx, object_id, key, value);
                            if key == "blocking" {
                                rebuild_runtime_navigation(self.ctx);
                            }
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        let converted =
                            convert_attr_value(key, val, entity.attributes.get(key), &health_attr);
                        entity.set_attribute(key, converted);
                        if key == "mode" {
                            let mode = entity
                                .attributes
                                .get_str_default("mode", String::new())
                                .to_ascii_lowercase();
                            if mode == "dead" {
                                entity.set_attribute("visible", Value::Bool(false));
                            } else if mode == "active" {
                                entity.set_attribute("visible", Value::Bool(true));
                                restore_entity_health_if_revived(entity, &health_attr);
                            }
                        }
                    }
                }
            }
            "toggle_attr" => {
                if let Some(key) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        let mut push_active: Option<(u32, String, VMValue)> = None;
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            let next_value = !item.attributes.get_bool_default(key, false);
                            item.set_attribute(key, Value::Bool(next_value));
                            if key == "active" {
                                if let Some(class_name) = item.attributes.get_str("class_name") {
                                    let value = VMValue::from_bool(next_value);
                                    push_active = Some((item.id, class_name.to_string(), value));
                                }
                            }
                        }
                        if let Some((id, _class_name, value)) = push_active {
                            self.ctx.to_execute_item.push((id, "active".into(), value));
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        let next_value = !entity.attributes.get_bool_default(key, false);
                        entity.set_attribute(key, Value::Bool(next_value));
                    }
                }
            }
            "id" => {
                return self.debug_return(VMValue::broadcast(self.ctx.curr_entity_id as f32));
            }
            "get_attr_of" => {
                if let (Some(id_val), Some(key)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let id = id_val.x as u32;
                    if let Some(entity) = self.ctx.get_entity_mut(id) {
                        if let Some(v) = entity.attributes.get(key).cloned() {
                            return self.debug_return(VMValue::from_value(&v));
                        }
                    } else if let Some(item) = self.ctx.get_item_mut(id) {
                        if let Some(v) = item.attributes.get(key).cloned() {
                            return self.debug_return(VMValue::from_value(&v));
                        }
                    }
                }
                return self.debug_return(VMValue::zero());
            }
            "get_attr" => {
                if let Some(key) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            if let Some(v) = item.attributes.get(key).cloned() {
                                return self.debug_return(VMValue::from_value(&v));
                            }
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        if let Some(v) = entity.attributes.get(key).cloned() {
                            return self.debug_return(VMValue::from_value(&v));
                        }
                    }
                }
                return self.debug_return(VMValue::zero());
            }
            "disposition_of" => {
                if let Some(target) = args.first() {
                    let target_id = target.x.max(0.0) as u32;
                    if let Some(disposition) =
                        entity_disposition_by_id(self.ctx, self.ctx.curr_entity_id, target_id)
                    {
                        return self.debug_return(VMValue::from_string(disposition));
                    }
                }
                return self.debug_return(VMValue::from_string("neutral"));
            }
            "is_hostile" => {
                if let Some(target) = args.first() {
                    let target_id = target.x.max(0.0) as u32;
                    let hostile =
                        entity_is_hostile_by_id(self.ctx, self.ctx.curr_entity_id, target_id);
                    return self.debug_return(VMValue::from_bool(hostile));
                }
                return self.debug_return(VMValue::from_bool(false));
            }
            "random" => {
                // random(min, max) inclusive; fallback to 0..1 if missing args
                if let (Some(a), Some(b)) = (args.get(0), args.get(1)) {
                    let mut lo = a.x as i32;
                    let mut hi = b.x as i32;
                    if lo > hi {
                        std::mem::swap(&mut lo, &mut hi);
                    }
                    let mut rng = rand::rng();
                    let r: i32 = rng.random_range(lo..=hi);
                    if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Int(r), false);
                    }
                    return self.debug_return(VMValue::broadcast(r as f32));
                } else {
                    let r: f32 = rand::random();
                    if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Float(r), false);
                    }
                    return self.debug_return(VMValue::broadcast(r));
                }
            }
            "notify_in" => {
                if let (Some(mins), Some(notification)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let minutes = mins.x as i32;
                    let target_tick = self.ctx.ticks
                        + RegionInstance::game_minutes_to_ticks(&self.ctx, minutes as f32);
                    if let Some(item_id) = self.ctx.curr_item_id {
                        self.ctx.notifications_items.push((
                            item_id,
                            target_tick,
                            notification.to_string(),
                        ));
                    } else {
                        self.ctx.notifications_entities.push((
                            self.ctx.curr_entity_id,
                            target_tick,
                            notification.to_string(),
                        ));
                    }
                }
            }
            "random_walk" => {
                // distance, speed, max_sleep
                let distance = args.get(0).map(|v| v.x).unwrap_or(1.0);
                let speed = args.get(1).map(|v| v.x).unwrap_or(1.0);
                let max_sleep = args.get(2).map(|v| v.x as i32).unwrap_or(0);
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    entity.action =
                        EntityAction::RandomWalk(distance, speed, max_sleep, 0, Vec2::zero());
                }
            }
            "random_walk_in_sector" => {
                let distance = args.get(0).map(|v| v.x).unwrap_or(1.0);
                let speed = args.get(1).map(|v| v.x).unwrap_or(1.0);
                let max_sleep = args.get(2).map(|v| v.x as i32).unwrap_or(0);
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    entity.action = EntityAction::RandomWalkInSector(
                        distance,
                        speed,
                        max_sleep,
                        0,
                        Vec2::zero(),
                    );
                }
            }
            "patrol" => {
                let route_wait = args.first().map(|v| v.x).unwrap_or(1.0).max(0.0);
                let route_speed = args.get(1).map(|v| v.x).unwrap_or(1.0).max(0.0);
                let entity_id = self.ctx.curr_entity_id;
                let (route_mode, route_names, current_pos) = self
                    .ctx
                    .map
                    .entities
                    .iter()
                    .find(|e| e.id == entity_id)
                    .map(|entity| {
                        (
                            entity
                                .attributes
                                .get_str_default("route_mode", "loop".to_string())
                                .to_ascii_lowercase(),
                            Self::parse_route_names(&entity.attributes),
                            entity.get_pos_xz(),
                        )
                    })
                    .unwrap_or_else(|| ("loop".to_string(), Vec::new(), Vec2::zero()));
                let points = Self::resolve_route_points(&self.ctx.map, &route_names, current_pos);
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if points.is_empty() {
                        entity.action = EntityAction::Off;
                    } else {
                        let point_index = Self::nearest_point_index(current_pos, &points);
                        entity.action = EntityAction::Patrol {
                            points,
                            route_wait,
                            route_speed,
                            route_mode,
                            point_index,
                            forward: true,
                            wait_until_tick: 0,
                        };
                    }
                }
            }
            "set_proximity_tracking" => {
                let turn_on = args.get(0).map(|v| v.to_bool()).unwrap_or(false);
                let distance = args.get(1).map(|v| v.x).unwrap_or(5.0);
                if let Some(item_id) = self.ctx.curr_item_id {
                    if turn_on {
                        self.ctx.item_proximity_alerts.insert(item_id, distance);
                    } else {
                        self.ctx.item_proximity_alerts.remove(&item_id);
                    }
                } else {
                    let entity_id = self.ctx.curr_entity_id;
                    if turn_on {
                        self.ctx.entity_proximity_alerts.insert(entity_id, distance);
                    } else {
                        self.ctx.entity_proximity_alerts.remove(&entity_id);
                    }
                }
            }
            "set_rig_sequence" => {
                // Not yet modeled; ignore.
            }
            "take" => {
                if let Some(item_id) = args.get(0).map(|v| v.x as u32) {
                    let mut removed: Option<Item> = None;
                    if let Some(pos) = self.ctx.map.items.iter().position(|item| {
                        item.id == item_id && !item.attributes.get_bool_default("static", false)
                    }) {
                        removed = Some(self.ctx.map.items.remove(pos));
                    }

                    if let Some(item) = removed {
                        let entity_id = self.ctx.curr_entity_id;
                        let mut rc = true;

                        if let Some(entity) = self
                            .ctx
                            .map
                            .entities
                            .iter_mut()
                            .find(|entity| entity.id == entity_id)
                        {
                            let item_name = item
                                .attributes
                                .get_str("name")
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                            fn article_for(item_name: &str) -> (&'static str, String) {
                                let name = item_name.to_ascii_lowercase();

                                let pair_items =
                                    ["trousers", "pants", "gloves", "boots", "scissors"];
                                let mass_items = ["armor", "cloth", "water", "meat"];

                                if pair_items.contains(&name.as_str()) {
                                    ("a pair of", item_name.to_string())
                                } else if mass_items.contains(&name.as_str()) {
                                    ("some", item_name.to_string())
                                } else {
                                    let first = name.chars().next().unwrap_or('x');
                                    let article = match first {
                                        'a' | 'e' | 'i' | 'o' | 'u' => "an",
                                        _ => "a",
                                    };
                                    (article, item_name.to_string())
                                }
                            }

                            let mut message = format!(
                                "You take {} {}",
                                article_for(&item_name.to_lowercase()).0,
                                item_name.to_lowercase()
                            );

                            if item.attributes.get_bool_default("monetary", false) {
                                let amount = item.attributes.get_int_default("worth", 0);
                                if amount > 0 {
                                    message = format!("You take {} gold.", amount);
                                    let _ = entity
                                        .add_base_currency(amount as i64, &self.ctx.currencies);
                                }
                            } else if entity.add_item(item).is_err() {
                                // TODO: Send message.
                                println!("Take: Too many items");
                                if self.ctx.debug_mode {
                                    add_debug_value(
                                        &mut self.ctx,
                                        TheValue::Text("Inventory Full".into()),
                                        true,
                                    );
                                }
                                rc = false;
                            }

                            if self.ctx.debug_mode && rc {
                                add_debug_value(&mut self.ctx, TheValue::Text("Ok".into()), false);
                            }

                            if let Some(sender) = self.ctx.from_sender.get() {
                                let _ = sender
                                    .send(RegionMessage::RemoveItem(self.ctx.region_id, item_id));

                                let msg = RegionMessage::Message(
                                    self.ctx.region_id,
                                    Some(entity_id),
                                    None,
                                    entity_id,
                                    message,
                                    "system".into(),
                                );
                                let _ = sender.send(msg);
                            }
                        }
                    } else if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Text("Unknown Item".into()), true);
                    }
                }
            }
            /*fn take(item_id: u32, vm: &VirtualMachine) -> bool {
                with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                    let entity_id = ctx.curr_entity_id;
                    let mut rc = true;

                    if let Some(pos) = ctx.map.items.iter().position(|item| {
                        item.id == item_id && !item.attributes.get_bool_default("static", false)
                    }) {
                        let item = ctx.map.items.remove(pos);

                        if let Some(entity) = ctx
                            .map
                            .entities
                            .iter_mut()
                            .find(|entity| entity.id == entity_id)
                        {
                            let mut item_name = "Unknown".to_string();
                            if let Some(name) = item.attributes.get_str("name") {
                                item_name = name.to_string();
                            }

                            fn article_for(item_name: &str) -> (&'static str, String) {
                                let name = item_name.to_ascii_lowercase();

                                let pair_items = ["trousers", "pants", "gloves", "boots", "scissors"];
                                let mass_items = ["armor", "cloth", "water", "meat"];

                                if pair_items.contains(&name.as_str()) {
                                    ("a pair of", item_name.to_string())
                                } else if mass_items.contains(&name.as_str()) {
                                    ("some", item_name.to_string())
                                } else {
                                    let first = name.chars().next().unwrap_or('x');
                                    let article = match first {
                                        'a' | 'e' | 'i' | 'o' | 'u' => "an",
                                        _ => "a",
                                    };
                                    (article, item_name.to_string())
                                }
                            }

                            let mut message = format!(
                                "You take {} {}",
                                article_for(&item_name.to_lowercase()).0,
                                item_name.to_lowercase()
                            );

                            if item.attributes.get_bool_default("monetary", false) {
                                // This is not a standalone item but money
                                let amount = item.attributes.get_int_default("worth", 0);
                                if amount > 0 {
                                    message = format!("You take {} gold.", amount);
                                    _ = entity.add_base_currency(amount as i64, &ctx.currencies);
                                }
                            } else if entity.add_item(item).is_err() {
                                // TODO: Send message.
                                println!("Take: Too many items");
                                if ctx.debug_mode {
                                    add_debug_value(ctx, TheValue::Text("Inventory Full".into()), true);
                                }
                                rc = false;
                            }

                            if ctx.debug_mode && rc {
                                add_debug_value(ctx, TheValue::Text("Ok".into()), false);
                            }

                            ctx.from_sender
                                .get()
                                .unwrap()
                                .send(RegionMessage::RemoveItem(ctx.region_id, item_id))
                                .unwrap();

                            let msg = RegionMessage::Message(
                                ctx.region_id,
                                Some(entity_id),
                                None,
                                entity_id,
                                message,
                                "system".into(),
                            );
                            ctx.from_sender.get().unwrap().send(msg).unwrap();
                        }
                    } else {
                        if ctx.debug_mode {
                            add_debug_value(ctx, TheValue::Text("Unknown Item".into()), true);
                        }
                    }
                    rc
                })
                .unwrap()
            } */
            "equip" => {
                if let Some(item_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(slot) = self
                        .ctx
                        .get_current_entity_mut()
                        .and_then(|e| e.get_item(item_id))
                        .and_then(|it| it.attributes.get_str("slot").map(|s| s.to_string()))
                    {
                        if let Some(entity) = self.ctx.get_current_entity_mut() {
                            let _ = entity.equip_item(item_id, &slot);
                        }
                    }
                }
            }
            "inventory_items" => {
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    let filter = args
                        .get(0)
                        .and_then(|v| v.as_string())
                        .unwrap_or("")
                        .to_string();
                    let ids: Vec<u32> = entity
                        .iter_inventory()
                        .filter(|(_, it)| {
                            filter.is_empty()
                                || it
                                    .attributes
                                    .get_str("name")
                                    .map(|n| n.contains(&filter))
                                    .unwrap_or(false)
                                || it
                                    .attributes
                                    .get_str("class_name")
                                    .map(|c| c.contains(&filter))
                                    .unwrap_or(false)
                        })
                        .map(|(_, i)| i.id)
                        .collect();
                    let ids_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                    let mut v = VMValue::zero();
                    if let Some(id0) = ids.get(0) {
                        v.x = *id0 as f32;
                    }
                    if let Some(id1) = ids.get(1) {
                        v.y = *id1 as f32;
                    }
                    v.z = ids.len() as f32;
                    v.string = Some(ids_str.join(","));
                    return self.debug_return(v);
                }
            }
            "inventory_items_of" => {
                if let Some(entity_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(entity) = self.ctx.get_entity_mut(entity_id) {
                        let filter = args
                            .get(1)
                            .and_then(|v| v.as_string())
                            .unwrap_or("")
                            .to_string();
                        let ids: Vec<u32> = entity
                            .iter_inventory()
                            .filter(|(_, it)| {
                                filter.is_empty()
                                    || it
                                        .attributes
                                        .get_str("name")
                                        .map(|n| n.contains(&filter))
                                        .unwrap_or(false)
                                    || it
                                        .attributes
                                        .get_str("class_name")
                                        .map(|c| c.contains(&filter))
                                        .unwrap_or(false)
                            })
                            .map(|(_, i)| i.id)
                            .collect();
                        let ids_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                        let mut v = VMValue::zero();
                        if let Some(id0) = ids.get(0) {
                            v.x = *id0 as f32;
                        }
                        if let Some(id1) = ids.get(1) {
                            v.y = *id1 as f32;
                        }
                        v.z = ids.len() as f32;
                        v.string = Some(ids_str.join(","));
                        return self.debug_return(v);
                    }
                }
            }
            "entities_in_radius" => {
                // args: [radius], operates on current entity or item
                let mut radius = args.get(0).map(|v| v.x.max(0.0)).unwrap_or(0.5);

                // Determine source position and id
                let (source_pos, source_entity_id, _source_item_id) = if let Some(item_id) =
                    self.ctx.curr_item_id
                {
                    if let Some(item) = self.ctx.get_item_mut(item_id) {
                        radius = radius.max(item.attributes.get_float_default("radius", radius));
                    }
                    (
                        self.ctx.get_item_mut(item_id).map(|i| i.get_pos_xz()),
                        None,
                        Some(item_id),
                    )
                } else {
                    let mut pos = None;
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        radius = radius.max(entity.attributes.get_float_default("radius", radius));
                        pos = Some(entity.get_pos_xz());
                    }
                    (pos, Some(self.ctx.curr_entity_id), None)
                };

                let mut ids: Vec<u32> = Vec::new();
                if let Some(pos) = source_pos {
                    for other in &self.ctx.map.entities {
                        // Skip self if we're an entity
                        if source_entity_id == Some(other.id) {
                            continue;
                        }
                        let other_pos = other.get_pos_xz();
                        let other_radius = other.attributes.get_float_default("radius", 0.5);
                        let combined = radius + other_radius;
                        if (pos - other_pos).magnitude_squared() < combined * combined {
                            ids.push(other.id);
                        }
                    }
                }

                // Pack result: x/y first two ids, z = count, string = comma list
                let ids_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                let mut v = VMValue::zero();
                if let Some(id0) = ids.get(0) {
                    v.x = *id0 as f32;
                }
                if let Some(id1) = ids.get(1) {
                    v.y = *id1 as f32;
                }
                v.z = ids.len() as f32;
                v.string = Some(ids_str.join(","));
                return self.debug_return(v);
            }
            "list_get" => {
                // list is arg0 (comma-separated string), index is arg1
                let idx = args.get(1).map(|v| v.x as i32).unwrap_or(0);
                if let Some(list_str) = args.get(0).and_then(|v| v.as_string()) {
                    let parts: Vec<&str> = list_str.split(',').filter(|s| !s.is_empty()).collect();
                    if parts.is_empty() {
                        return self.debug_return(VMValue::zero());
                    }
                    let clamped = if idx < 0 {
                        0
                    } else if (idx as usize) >= parts.len() {
                        parts.len() - 1
                    } else {
                        idx as usize
                    };
                    if let Ok(val) = parts[clamped].parse::<f32>() {
                        return self.debug_return(VMValue::broadcast(val));
                    }
                    return self.debug_return(VMValue::zero());
                }
            }
            "is_item" => {
                if let Some(id) = args.get(0) {
                    let item_id = id.x as u32;
                    let exists = self.ctx.map.items.iter().any(|i| i.id == item_id)
                        || self
                            .ctx
                            .map
                            .entities
                            .iter()
                            .flat_map(|e| e.iter_inventory().map(|(_, it)| it.id))
                            .any(|i| i == item_id);
                    return self.debug_return_bool(exists);
                }
            }
            "is_entity" => {
                if let Some(id) = args.get(0) {
                    let entity_id = id.x as u32;
                    let exists = self.ctx.map.entities.iter().any(|e| e.id == entity_id);
                    return self.debug_return_bool(exists);
                }
            }
            "distance_to" => {
                if let Some(id) = args.get(0) {
                    let target = id.x as u32;
                    let mut target_pos: Option<Vec2<f32>> = None;
                    if let Some(e) = self.ctx.map.entities.iter().find(|e| e.id == target) {
                        target_pos = Some(e.get_pos_xz());
                    } else if let Some(i) = self.ctx.map.items.iter().find(|i| i.id == target) {
                        target_pos = Some(i.get_pos_xz());
                    }
                    if let Some(target_pos) = target_pos {
                        let pos = if let Some(item_id) = self.ctx.curr_item_id {
                            self.ctx.get_item_mut(item_id).map(|i| i.get_pos_xz())
                        } else {
                            self.ctx.get_current_entity_mut().map(|e| e.get_pos_xz())
                        };
                        if let Some(pos) = pos {
                            let dist = pos.distance(target_pos);
                            return self.debug_return(VMValue::broadcast(dist));
                        }
                    }
                    return self.debug_return(VMValue::zero());
                }
            }
            "deal_damage" => {
                // deal_damage() uses the normal weapon / unarmed rules against the current target.
                let mut ruleset_damage = || {
                    let source_item_id = self
                        .ctx
                        .curr_item_id
                        .or_else(|| self.current_attack_source_item_id());
                    (
                        self.get_current_target_id(),
                        self.current_attack_base_damage(),
                        self.current_attack_kind(source_item_id),
                        source_item_id,
                    )
                };
                let (target_id, base_dmg, kind, source_item_id) = match args {
                    [] => ruleset_damage(),
                    [kind] if kind.as_string().is_some() => {
                        let (target_id, base_dmg, _, source_item_id) = ruleset_damage();
                        (
                            target_id,
                            base_dmg,
                            kind.as_string().unwrap_or("physical").to_string(),
                            source_item_id,
                        )
                    }
                    [_amount] => ruleset_damage(),
                    [_amount, kind] if kind.as_string().is_some() => (
                        self.get_current_target_id(),
                        self.current_attack_base_damage(),
                        kind.as_string().unwrap_or("physical").to_string(),
                        self.ctx
                            .curr_item_id
                            .or_else(|| self.current_attack_source_item_id()),
                    ),
                    [target, _amount] => {
                        let (_, base_dmg, kind, source_item_id) = ruleset_damage();
                        (
                            Self::parse_target_arg_id(target)
                                .or_else(|| self.get_current_target_id()),
                            base_dmg,
                            kind,
                            source_item_id,
                        )
                    }
                    [target, _amount, kind] => (
                        Self::parse_target_arg_id(target).or_else(|| self.get_current_target_id()),
                        self.current_attack_base_damage(),
                        kind.as_string().unwrap_or("physical").to_string(),
                        self.ctx
                            .curr_item_id
                            .or_else(|| self.current_attack_source_item_id()),
                    ),
                    _ => ruleset_damage(),
                };
                if target_id.is_some()
                    && !has_attack_ammunition_or_message(
                        self.ctx,
                        self.ctx.curr_entity_id,
                        source_item_id,
                        "attack",
                    )
                {
                    return self.debug_return(VMValue::zero());
                }
                if target_id.is_some() && !self.try_start_attack_cooldown() {
                    return self.debug_return(VMValue::zero());
                }
                self.queue_damage(target_id, base_dmg, &kind, source_item_id);
            }
            "attack" => {
                let target_id = self.get_current_target_id();
                let source_item_id = self.current_attack_source_item_id();
                if target_id.is_some()
                    && !has_attack_ammunition_or_message(
                        self.ctx,
                        self.ctx.curr_entity_id,
                        source_item_id,
                        "attack",
                    )
                {
                    return self.debug_return(VMValue::zero());
                }
                if target_id.is_some() && !self.try_start_attack_cooldown() {
                    return self.debug_return(VMValue::zero());
                }
                let kind = self.current_attack_kind(source_item_id);
                let base_dmg = self.current_attack_base_damage();
                self.queue_damage(target_id, base_dmg, &kind, source_item_id);
            }
            "use_action" => {
                if let Some(action_id) = args.first().and_then(VMValue::as_string) {
                    let target_id = args
                        .get(1)
                        .and_then(Self::parse_target_arg_id)
                        .or_else(|| self.get_current_target_id());
                    let ok = execute_ruleset_action(
                        self.ctx,
                        self.ctx.curr_entity_id,
                        action_id,
                        target_id,
                    );
                    return self.debug_return_bool(ok);
                }
                return self.debug_return_bool(false);
            }
            "craft" => {
                if let Some(recipe_id) = args.first().and_then(VMValue::as_string) {
                    let ok = craft_ruleset_recipe(self.ctx, self.ctx.curr_entity_id, recipe_id);
                    return self.debug_return_bool(ok);
                }
                return self.debug_return_bool(false);
            }
            "took_damage" => {
                if let (Some(from), Some(amount_val)) = (args.get(0), args.get(1)) {
                    let from = from.x as u32;
                    // Make sure we don't heal by accident
                    let amount = amount_val.x.max(0.0) as i32;

                    if amount == 0 {
                        return None;
                    }

                    let id = self.ctx.curr_entity_id;
                    let kind = self
                        .ctx
                        .current_damage_kind
                        .as_deref()
                        .unwrap_or("physical")
                        .to_string();
                    let _ = apply_damage_direct(
                        self.ctx,
                        id,
                        from,
                        amount,
                        &kind,
                        self.ctx.current_damage_source_item,
                    );
                    self.ctx.damage_committed = true;
                }
            }
            "block_events" => {
                if let (Some(minutes), Some(event)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let target_tick =
                        self.ctx.ticks + RegionInstance::game_minutes_to_ticks(self.ctx, minutes.x);
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(state) = self.ctx.item_state_data.get_mut(&item_id) {
                            state.set(event, Value::Int64(target_tick));
                        }
                    } else {
                        let eid = self.ctx.curr_entity_id;
                        if let Some(state) = self.ctx.entity_state_data.get_mut(&eid) {
                            state.set(event, Value::Int64(target_tick));
                        }
                    }
                }
            }
            "add_item" => {
                if let Some(class_name) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item) = self.ctx.create_item(class_name.to_string()) {
                        let id = self.ctx.curr_entity_id;
                        if let Some(entity) = self.ctx.get_entity_mut(id) {
                            let item_id = item.id;
                            if entity.add_item(item).is_ok() {
                                if self.ctx.debug_mode {
                                    add_debug_value(self.ctx, TheValue::Text("Ok".into()), false);
                                }
                                return Some(VMValue::from_i32(item_id as i32));
                            } else {
                                if self.ctx.debug_mode {
                                    add_debug_value(
                                        self.ctx,
                                        TheValue::Text("Inventory Full".into()),
                                        true,
                                    );
                                }
                                println!("add_item ({}): Inventory is full", class_name);
                                return Some(VMValue::from_i32(-1));
                            }
                        } else {
                            return Some(VMValue::from_i32(-1));
                        }
                    } else {
                        if self.ctx.debug_mode {
                            add_debug_value(self.ctx, TheValue::Text("Unknown Item".into()), true);
                        }
                        self.ctx.send_log_message(format!(
                            "[warn] {} ({}) => add_item: '{}' is not a valid item template.",
                            self.ctx.get_entity_name(self.ctx.curr_entity_id),
                            self.ctx.curr_entity_id,
                            class_name
                        ));
                        return Some(VMValue::from_i32(-1));
                    }
                }
            }
            "offer_inventory" => {
                if let (Some(to), Some(filter)) = (
                    args.get(0).map(|v| v.x as u32),
                    args.get(1).and_then(|v| v.as_string()),
                ) {
                    let region_id = self.ctx.region_id;
                    let now_ticks = self.ctx.ticks;
                    let ticks_per_minute = self.ctx.ticks_per_minute;
                    let Some((entity_id, matching_item_ids, expires_at_tick, max_distance)) =
                        self.ctx.get_current_entity_mut().map(|entity| {
                            let matching_item_ids: Vec<u32> = entity
                                .iter_inventory()
                                .filter_map(|(_, item)| {
                                    let name = item.attributes.get_str("name").unwrap_or_default();
                                    let class_name =
                                        item.attributes.get_str("class_name").unwrap_or_default();

                                    if filter.is_empty()
                                        || name.contains(filter)
                                        || class_name.contains(filter)
                                    {
                                        Some(item.id)
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            let timeout_minutes = entity
                                .attributes
                                .get_float_default("timeout", 10.0)
                                .max(0.0);
                            let expires_at_tick =
                                now_ticks + (ticks_per_minute as f32 * timeout_minutes) as i64;
                            let max_distance = 2.0;
                            (entity.id, matching_item_ids, expires_at_tick, max_distance)
                        })
                    else {
                        return None;
                    };

                    self.ctx
                        .active_choice_sessions
                        .retain(|session| !(session.from == entity_id && session.to == to));
                    self.ctx.active_choice_sessions.push(ChoiceSession {
                        from: entity_id,
                        to,
                        expires_at_tick,
                        max_distance,
                    });
                    let mut choices = MultipleChoice::new(
                        region_id,
                        entity_id,
                        to,
                        expires_at_tick,
                        max_distance,
                    );
                    for item_id in matching_item_ids {
                        let choice = Choice::ItemToSell(
                            item_id,
                            entity_id,
                            to,
                            expires_at_tick,
                            max_distance,
                        );
                        choices.add(choice);
                    }

                    if let Some(sender) = self.ctx.from_sender.get() {
                        let _ = sender.send(RegionMessage::MultipleChoice(choices));
                    }
                }
            }
            "multiple_choice" => {
                if let (Some(to), Some(prompt), Some(choice_attr)) = (
                    args.first().map(|v| v.x as u32),
                    args.get(1).and_then(|v| v.as_string()),
                    args.get(2).and_then(|v| v.as_string()),
                ) {
                    let region_id = self.ctx.region_id;
                    let now_ticks = self.ctx.ticks;
                    let ticks_per_minute = self.ctx.ticks_per_minute;
                    let Some((entity_id, choice_specs, expires_at_tick, max_distance)) =
                        self.ctx.get_current_entity_mut().map(|entity| {
                            let choice_specs = match entity.attributes.get(choice_attr) {
                                Some(Value::StrArray(values)) => values.clone(),
                                Some(Value::Str(value)) => value
                                    .lines()
                                    .flat_map(|line| line.split(','))
                                    .map(str::trim)
                                    .filter(|line| !line.is_empty())
                                    .map(str::to_string)
                                    .collect(),
                                _ => Vec::new(),
                            };
                            let timeout_minutes = entity
                                .attributes
                                .get_float_default("timeout", 10.0)
                                .max(0.0);
                            let expires_at_tick =
                                now_ticks + (ticks_per_minute as f32 * timeout_minutes) as i64;
                            (entity.id, choice_specs, expires_at_tick, 2.0)
                        })
                    else {
                        return None;
                    };

                    if let Some(sender) = self.ctx.from_sender.get() {
                        let mut choices = MultipleChoice::new(
                            region_id,
                            entity_id,
                            to,
                            expires_at_tick,
                            max_distance,
                        );
                        for (index, raw) in choice_specs.iter().enumerate() {
                            let label = raw.trim();
                            if label.is_empty() {
                                continue;
                            }
                            choices.add(Choice::ScriptChoice(
                                label.to_string(),
                                choice_attr.to_string(),
                                entity_id,
                                to,
                                index as u32,
                                expires_at_tick,
                                max_distance,
                            ));
                        }

                        if !choices.choices.is_empty() {
                            if !prompt.is_empty() {
                                let _ = sender.send(RegionMessage::Message(
                                    region_id,
                                    Some(entity_id),
                                    None,
                                    to,
                                    prompt.to_string(),
                                    "multiple_choice".into(),
                                ));
                            }
                            self.ctx
                                .active_choice_sessions
                                .retain(|session| !(session.from == entity_id && session.to == to));
                            self.ctx.active_choice_sessions.push(ChoiceSession {
                                from: entity_id,
                                to,
                                expires_at_tick,
                                max_distance,
                            });
                            let _ = sender.send(RegionMessage::MultipleChoice(choices));
                        }
                    }
                }
            }
            "dialog" => {
                if let (Some(to), Some(node)) = (
                    args.first().map(|v| v.x as u32),
                    args.get(1).and_then(|v| v.as_string()),
                ) {
                    let from = self.ctx.curr_entity_id;
                    open_dialog_node(self.ctx, from, to, node);
                }
            }
            "gain_xp" => {
                let gained = args.first().map(|v| v.x.max(0.0)).unwrap_or(0.0);
                if gained > 0.0 {
                    let level_ups =
                        grant_experience(self.ctx, self.ctx.curr_entity_id, gained.round() as i32);
                    if self.ctx.debug_mode {
                        add_debug_value(
                            &mut self.ctx,
                            TheValue::Text(if let Some(level) = level_ups.last() {
                                format!("XP +{} -> Lv {}", gained.round() as i32, level)
                            } else {
                                format!("XP +{}", gained.round() as i32)
                            }),
                            false,
                        );
                    }
                }
            }
            "drop_items" => {
                if let Some(filter) = args.get(0).and_then(|v| v.as_string()) {
                    let entity_id = self.ctx.curr_entity_id;
                    if drop_items_into_ruleset_loot_container(self.ctx, entity_id, filter) {
                        return None;
                    }
                    let mut removed_items = Vec::new();
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        let matching_slots: Vec<usize> = entity
                            .iter_inventory()
                            .filter_map(|(slot, it)| {
                                let name = it.attributes.get_str("name").unwrap_or_default();
                                let class_name =
                                    it.attributes.get_str("class_name").unwrap_or_default();
                                if filter.is_empty()
                                    || name.contains(filter)
                                    || class_name.contains(filter)
                                {
                                    Some(slot)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        for slot in matching_slots {
                            if let Some(mut item) = entity.remove_item_from_slot(slot) {
                                // Drop at the entity position and mark dirty so the server transmits
                                item.position = entity.position;
                                item.mark_all_dirty();
                                removed_items.push(item);
                            }
                        }
                    }
                    self.ctx.map.items.extend(removed_items);
                }
            }
            "drop" => {
                if let Some(item_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        if let Some(pos) = entity
                            .inventory
                            .iter()
                            .position(|opt| opt.as_ref().map(|i| i.id) == Some(item_id))
                        {
                            if let Some(mut item) = entity.remove_item_from_slot(pos) {
                                item.position = entity.position;
                                item.mark_all_dirty();
                                self.ctx.map.items.push(item);
                            }
                        }
                    }
                }
            }
            "teleport" => {
                if let Some(dest) = args.get(0).and_then(|v| v.as_string()) {
                    let region_name = args.get(1).and_then(|v| v.as_string()).unwrap_or("");

                    if region_name.trim().is_empty()
                        || region_name.trim().eq_ignore_ascii_case(&self.ctx.map.name)
                    {
                        let radius = self
                            .ctx
                            .map
                            .entities
                            .iter()
                            .find(|entity| entity.id == self.ctx.curr_entity_id)
                            .map(|entity| {
                                entity.attributes.get_float_default("radius", 0.5).max(0.0) - 0.01
                            })
                            .unwrap_or(0.49);
                        let center = self.ctx.resolve_sector_spawn_position(&dest, radius);
                        if let Some(center) = center {
                            // First move the entity
                            if let Some(entity) = self.ctx.get_current_entity_mut() {
                                let id = entity.id;
                                entity.set_pos_xz(center);
                                entity.mark_all_dirty();
                                // Then run section change checks using a fresh borrow
                                self.ctx.check_player_for_section_change_id(id);
                                if let Some(sender) = self.ctx.from_sender.get() {
                                    let _ = sender.send(RegionMessage::MapUpdate(
                                        self.ctx.region_id,
                                        self.ctx.map.clone(),
                                    ));
                                }
                            }
                        } else if self.ctx.debug_mode {
                            add_debug_value(
                                &mut self.ctx,
                                TheValue::Text("Unknown Sector".into()),
                                true,
                            );
                        }
                    } else {
                        // Defer cross-region transfers until the current script event has
                        // finished. Scripts often restore HP/mode or send messages after
                        // teleport(); removing the entity immediately makes those writes
                        // order-dependent and can strand players in dead state.
                        self.ctx.pending_entity_transfers.push((
                            self.ctx.curr_entity_id,
                            region_name.to_string(),
                            dest.to_string(),
                        ));
                    }
                }
            }
            /*pub fn teleport(args: rustpython_vm::function::FuncArgs, vm: &VirtualMachine) -> PyResult<()> {
                let mut sector_name = String::new();
                let mut region_name = String::new();

                for (i, arg) in args.args.iter().enumerate() {
                    if i == 0 {
                        if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                            sector_name = v.clone();
                        }
                    } else if i == 1 {
                        if let Some(Value::Str(v)) = Value::from_pyobject(arg.clone(), vm) {
                            region_name = v.clone();
                        }
                    }
                }

                with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                    if region_name.is_empty() {
                        // Teleport entity in this region to the given sector.

                        let mut new_pos: Option<vek::Vec2<f32>> = None;
                        for sector in &ctx.map.sectors {
                            if sector.name == sector_name {
                                new_pos = sector.center(&ctx.map);
                            }
                        }

                        if let Some(new_pos) = new_pos {
                            let entity_id = ctx.curr_entity_id;
                            let mut entities = ctx.map.entities.clone();
                            if let Some(entity) = entities.iter_mut().find(|entity| entity.id == entity_id) {
                                entity.set_pos_xz(new_pos);
                                check_player_for_section_change(ctx, entity);
                            }
                            ctx.map.entities = entities;
                        } else {
                            if ctx.debug_mode {
                                add_debug_value(ctx, TheValue::Text("Unknown Sector".into()), true);
                            }
                        }
                    } else {
                        // Remove the entity from this region and send it to the server to be moved
                        // into a new region.

                        let entity_id = ctx.curr_entity_id;
                        if let Some(pos) = ctx.map.entities.iter().position(|e| e.id == entity_id) {
                            let removed = ctx.map.entities.remove(pos);

                            ctx.entity_classes.remove(&removed.id);

                            let msg =
                                RegionMessage::TransferEntity(ctx.region_id, removed, region_name, sector_name);
                            ctx.from_sender.get().unwrap().send(msg).unwrap();
                        }
                    }
                });

                Ok(())
            } */
            "goto" => {
                if let Some(dest) = args.get(0).and_then(|v| v.as_string()) {
                    let speed = args.get(1).map(|v| v.x).unwrap_or(1.0);
                    let coord = self.ctx.map.named_area_center(&dest);

                    if let Some(coord) = coord {
                        if let Some(entity) = self.ctx.get_current_entity_mut() {
                            let position = entity.get_pos_xz();
                            let start_center =
                                crate::server::region::RegionInstance::snapped_grid_center(
                                    position,
                                );
                            let target_center =
                                crate::server::region::RegionInstance::snapped_grid_center(coord);
                            let grid_aligned = (position - start_center).magnitude_squared()
                                <= 0.001
                                && (coord - target_center).magnitude_squared() <= 0.001;
                            if grid_aligned {
                                entity.action = EntityAction::GotoGrid(coord, speed);
                            } else {
                                entity.action = EntityAction::Goto(coord, speed);
                            }
                        }
                    } else if self.ctx.debug_mode {
                        add_debug_value(
                            &mut self.ctx,
                            TheValue::Text("Unknown Sector".into()),
                            true,
                        );
                    }
                }
            }
            "run_sequence" => {
                if let Some(name) = args.get(0).and_then(|v| v.as_string())
                    && let Some(entity) = self.ctx.get_current_entity_mut()
                {
                    let sequence_name = name.trim();
                    if entity.sequences.contains_key(sequence_name) {
                        entity.active_sequence = Some(crate::server::entity::EntitySequenceState {
                            name: sequence_name.to_string(),
                            step_index: 0,
                            wait_until_tick: None,
                        });
                        entity.paused_sequence = None;
                        entity.action = EntityAction::Off;
                    } else if self.ctx.debug_mode {
                        add_debug_value(
                            &mut self.ctx,
                            TheValue::Text("Unknown Sequence".into()),
                            true,
                        );
                    }
                }
            }
            "pause_sequence" => {
                if let Some(entity) = self.ctx.get_current_entity_mut()
                    && let Some(active) = entity.active_sequence.take()
                {
                    entity.paused_sequence = Some(active);
                    entity.action = EntityAction::Off;
                }
            }
            "resume_sequence" => {
                if let Some(entity) = self.ctx.get_current_entity_mut()
                    && entity.active_sequence.is_none()
                    && let Some(paused) = entity.paused_sequence.take()
                {
                    entity.active_sequence = Some(paused);
                    entity.action = EntityAction::Off;
                }
            }
            "cancel_sequence" => {
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    entity.active_sequence = None;
                    entity.paused_sequence = None;
                    entity.action = EntityAction::Off;
                }
            }
            /*fn goto(destination: String, speed: f32, vm: &VirtualMachine) {
                let mut coord: Option<vek::Vec2<f32>> = None;

                with_regionctx(get_region_id(vm).unwrap(), |ctx: &mut RegionCtx| {
                    for sector in &ctx.map.sectors {
                        if sector.name == destination {
                            coord = sector.center(&ctx.map);
                        }
                    }

                    if let Some(coord) = coord {
                        let entity_id = ctx.curr_entity_id;
                        if let Some(entity) = ctx
                            .map
                            .entities
                            .iter_mut()
                            .find(|entity| entity.id == entity_id)
                        {
                            entity.action = Goto(coord, speed);
                        }
                    } else {
                        if ctx.debug_mode {
                            add_debug_value(ctx, TheValue::Text("Unknown Sector".into()), true);
                        }
                    }
                });
            } */
            "close_in" => {
                if let (Some(target), Some(radius), Some(speed)) =
                    (args.get(0), args.get(1), args.get(2))
                {
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        entity.action = EntityAction::CloseIn(target.x as u32, radius.x, speed.x);
                    }
                }
            }
            "follow_attack" => {
                if let (Some(target), Some(speed)) = (args.get(0), args.get(1))
                    && let Some(entity) = self.ctx.get_current_entity_mut()
                {
                    let target_id = target.x.max(0.0) as u32;
                    let next_attack_tick = match entity.action {
                        EntityAction::FollowAttack(existing_target, _, next_tick)
                            if existing_target == target_id =>
                        {
                            next_tick
                        }
                        _ => 0,
                    };
                    entity.set_attribute("target", Value::UInt(target_id));
                    entity.set_attribute("attack_target", Value::UInt(target_id));
                    entity.action =
                        EntityAction::FollowAttack(target_id, speed.x, next_attack_tick);
                }
            }
            "debug" => {
                let mut output = String::new();

                for (i, arg) in args.iter().enumerate() {
                    let arg_str = if let Some(s) = arg.as_string() {
                        s.to_string()
                    } else {
                        format!("{}", arg.x)
                    };

                    if i > 0 {
                        output.push(' ');
                    }
                    output.push_str(&arg_str);
                }

                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if let Some(name) = entity.attributes.get_str("name") {
                        output = format!("{}: {}", name, output);
                    }
                }

                if let Some(sender) = self.ctx.from_sender.get() {
                    let _ = sender.send(RegionMessage::LogMessage(output));
                }
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::region::{
        apply_ruleset_character_defaults, drop_items_into_ruleset_loot_container,
        update_entity_respawns,
    };
    use crate::vm::{Execution, Program, VM, VMValue};
    use std::sync::Arc;

    fn official_rules_source() -> String {
        [
            include_str!("../../../../rulesets/eldiron/v1/ruleset.toml"),
            include_str!("../../../../rulesets/eldiron/v1/identity.toml"),
            include_str!("../../../../rulesets/eldiron/v1/attributes.toml"),
            include_str!("../../../../rulesets/eldiron/v1/progression.toml"),
            include_str!("../../../../rulesets/eldiron/v1/combat.toml"),
            include_str!("../../../../rulesets/eldiron/v1/messages.toml"),
            include_str!("../../../../rulesets/eldiron/v1/equipment.toml"),
            include_str!("../../../../rulesets/eldiron/v1/fx.toml"),
            include_str!("../../../../rulesets/eldiron/v1/actions.toml"),
            include_str!("../../../../rulesets/eldiron/v1/recipes.toml"),
            include_str!("../../../../rulesets/eldiron/v1/abilities_spells.toml"),
            include_str!("../../../../rulesets/eldiron/v1/races_classes.toml"),
        ]
        .join("\n\n")
    }

    fn official_locales_source() -> &'static str {
        include_str!("../../../../rulesets/eldiron/v1/locales.toml")
    }

    fn toml_value_to_attr(value: &toml::Value) -> Option<Value> {
        match value {
            toml::Value::String(value) => Some(Value::Str(value.clone())),
            toml::Value::Integer(value) if *value >= 0 => Some(Value::UInt(*value as u32)),
            toml::Value::Integer(value) => Some(Value::Int(*value as i32)),
            toml::Value::Float(value) => Some(Value::Float(*value as f32)),
            toml::Value::Boolean(value) => Some(Value::Bool(*value)),
            toml::Value::Array(values) => {
                let strings = values
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                (!strings.is_empty()).then_some(Value::StrArray(strings))
            }
            _ => None,
        }
    }

    fn table_string(table: &toml::value::Table, key: &str) -> Option<String> {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn copy_official_item_damage_attrs(item: &mut Item, item_table: &toml::value::Table) {
        let Some(damage) = item_table.get("damage").and_then(toml::Value::as_table) else {
            return;
        };

        if let Some(roll) = table_string(damage, "roll") {
            item.set_attribute("damage_roll", Value::Str(roll));
        }
        if let Some(value) = damage.get("bonus").and_then(toml_value_to_attr) {
            item.set_attribute("damage_bonus", value);
        }
        if let Some(attribute) = table_string(damage, "bonus_attribute") {
            item.set_attribute("damage_bonus_attribute", Value::Str(attribute));
        }
        if let Some(value) = damage.get("bonus_every").and_then(toml_value_to_attr) {
            item.set_attribute("damage_bonus_every", value);
        }
        if let Some(kind) = table_string(damage, "damage_kind") {
            item.set_attribute("damage_kind", Value::Str(kind));
        }
    }

    fn official_item_from_rules(
        rules: &toml::Table,
        item_id: u32,
        group: &str,
        id: &str,
    ) -> (String, String, Item, String) {
        let item_table = rules
            .get("items")
            .and_then(toml::Value::as_table)
            .and_then(|items| items.get(group))
            .and_then(toml::Value::as_table)
            .and_then(|items| items.get(id))
            .and_then(toml::Value::as_table)
            .expect("official item");
        let name = table_string(item_table, "name").unwrap_or_else(|| id.to_string());
        let kind = group.strip_suffix('s').unwrap_or(group).to_string();
        let slot = table_string(item_table, "slot").unwrap_or_else(|| "main_hand".into());
        let ruleset_path = format!("items.{}.{}", group, id);

        let mut item = Item::new();
        item.id = item_id;
        item.item_type = name.clone();
        item.set_attribute("class_name", Value::Str(name.clone()));
        item.set_attribute("name", Value::Str(name.clone()));
        item.set_attribute("ruleset_id", Value::Str(id.into()));
        item.set_attribute("ruleset_kind", Value::Str(kind));
        item.set_attribute("ruleset_path", Value::Str(ruleset_path));
        item.set_attribute("slot", Value::Str(slot.clone()));
        item.set_attribute("quality", Value::Int(100));
        item.set_attribute("condition", Value::Int(100));

        for key in [
            "category",
            "description",
            "rarity",
            "icon",
            "container_template",
            "visual_template",
            "rig_layer",
        ] {
            if let Some(value) = table_string(item_table, key) {
                item.set_attribute(key, Value::Str(value));
            }
        }
        for key in [
            "rig_scale",
            "rig_pivot",
            "color",
            "icon_color",
            "quality",
            "condition",
            "max_stack",
            "ammunition_quantity",
            "blade_color_index",
            "grip_color_index",
            "accent_color_index",
            "highlight_color_index",
        ] {
            if let Some(value) = item_table.get(key).and_then(toml_value_to_attr) {
                let attr = if key == "color" { "color_index" } else { key };
                item.set_attribute(attr, value);
            }
        }
        if let Some(max_stack) = item_table
            .get("max_stack")
            .and_then(toml::Value::as_integer)
        {
            item.set_max_capacity(max_stack.max(1) as u32);
        }
        if let Some(attributes) = item_table.get("attributes").and_then(toml::Value::as_table) {
            for (key, value) in attributes {
                if let Some(value) = toml_value_to_attr(value) {
                    item.set_attribute(key, value);
                }
            }
        }
        copy_official_item_damage_attrs(&mut item, item_table);
        item.apply_container_attributes();

        let mut data = toml::value::Table::new();
        if let Some(damage) = item_table.get("damage").and_then(toml::Value::as_table) {
            let mut ruleset = toml::value::Table::new();
            ruleset.insert("damage".into(), toml::Value::Table(damage.clone()));
            data.insert("ruleset".into(), toml::Value::Table(ruleset));
        }

        (
            slot,
            name,
            item,
            toml::to_string(&data).expect("official item data"),
        )
    }

    fn attack_host_context() -> RegionCtx {
        let mut ctx = RegionCtx {
            curr_entity_id: 1,
            ticks: 10,
            ticks_per_minute: 4,
            ..Default::default()
        };
        let mut entity = Entity::new();
        entity.id = 1;
        ctx.map.entities.push(entity);
        ctx
    }

    #[test]
    fn pending_intent_cooldown_does_not_block_script_attack() {
        let mut ctx = attack_host_context();
        ctx.entity_state_data
            .entry(1)
            .or_default()
            .set("__pending_intent_cooldown:attack", Value::Int64(30));

        let mut host = RegionHost { ctx: &mut ctx };

        assert!(host.try_start_attack_cooldown());
        assert!(
            host.ctx
                .entity_state_data
                .get(&1)
                .and_then(|state| state.get("intent: attack"))
                .is_some()
        );
    }

    #[test]
    fn active_intent_cooldown_blocks_script_attack() {
        let mut ctx = attack_host_context();
        ctx.entity_state_data
            .entry(1)
            .or_default()
            .set("intent: attack", Value::Int64(30));

        let mut host = RegionHost { ctx: &mut ctx };

        assert!(!host.try_start_attack_cooldown());
    }

    struct HeadlessRulesArena {
        ctx: RegionCtx,
        exec: Execution,
        _messages: crossbeam_channel::Receiver<RegionMessage>,
    }

    impl HeadlessRulesArena {
        fn new() -> Self {
            let ctx = RegionCtx {
                health_attr: "HP".into(),
                level_attr: "LEVEL".into(),
                ticks_per_minute: 10,
                ..Default::default()
            };
            let (sender, receiver) = crossbeam_channel::unbounded();
            let _ = ctx.from_sender.set(sender);
            Self {
                ctx,
                exec: Execution::default(),
                _messages: receiver,
            }
        }

        fn with_rules(rules: &str) -> Self {
            let mut arena = Self::new();
            arena.ctx.rules = rules
                .parse::<toml::Table>()
                .expect("valid arena rules TOML");
            arena
        }

        fn with_official_rules() -> Self {
            let mut arena = Self::with_rules(&official_rules_source());
            arena.ctx.config = r#"
                [game]
                weapon_slots = ["main_hand", "off_hand"]
            "#
            .parse::<toml::Table>()
            .expect("valid arena config TOML");
            arena
        }

        fn load_official_locales(&mut self) {
            self.ctx.assets.locales_src = official_locales_source().to_string();
            self.ctx.assets.read_locales();
        }

        fn add_official_entity(
            &mut self,
            id: u32,
            class: &str,
            race: &str,
            level: u32,
            target: Option<u32>,
        ) {
            let mut entity = Entity::new();
            entity.id = id;
            entity.position = Vec3::new(id as f32, 1.0, 0.0);
            entity.set_attribute("class_name", Value::Str(class.into()));
            entity.set_attribute("name", Value::Str(class.into()));
            entity.set_attribute("mode", Value::Str("active".into()));
            entity.set_attribute("visible", Value::Bool(true));
            entity.set_attribute("class", Value::Str(class.into()));
            entity.set_attribute("race", Value::Str(race.into()));
            entity.set_attribute("LEVEL", Value::UInt(level));
            if let Some(target) = target {
                entity.set_attribute("target", Value::UInt(target));
                entity.set_attribute("attack_target", Value::UInt(target));
            }

            apply_ruleset_character_defaults(&self.ctx.rules, &mut entity);
            if let Some(Value::Int(inv_slots)) = entity.attributes.get("inventory_slots") {
                entity.inventory = vec![None; (*inv_slots).max(0) as usize];
            }
            self.ctx.entity_classes.insert(id, class.into());
            self.ctx.map.entities.push(entity);
        }

        fn compile_program(source: &str) -> Arc<Program> {
            let mut vm = VM::default();
            Arc::new(vm.prepare_str(source).expect("valid arena script"))
        }

        fn add_script_class(&mut self, class: &str, source: &str) {
            self.ctx
                .entity_programs
                .insert(class.into(), Self::compile_program(source));
        }

        fn add_entity(&mut self, id: u32, class: &str, hp: i32, dmg: i32, target: Option<u32>) {
            let mut entity = Entity::new();
            entity.id = id;
            entity.position = Vec3::new(id as f32, 1.0, 0.0);
            entity.set_attribute("class_name", Value::Str(class.into()));
            entity.set_attribute("name", Value::Str(class.into()));
            entity.set_attribute("mode", Value::Str("active".into()));
            entity.set_attribute("visible", Value::Bool(true));
            entity.set_attribute("HP", Value::Int(hp));
            entity.set_attribute("MAX_HP", Value::Int(hp));
            entity.set_attribute("DMG", Value::Int(dmg));
            if let Some(target) = target {
                entity.set_attribute("target", Value::UInt(target));
                entity.set_attribute("attack_target", Value::UInt(target));
            }

            self.ctx.entity_classes.insert(id, class.into());
            self.ctx.map.entities.push(entity);
        }

        fn add_inventory_item(&mut self, entity_id: u32, item_id: u32, class: &str) {
            let mut item = Item::new();
            item.id = item_id;
            item.item_type = class.into();
            item.set_attribute("class_name", Value::Str(class.into()));
            item.set_attribute("name", Value::Str(class.into()));
            item.set_attribute("ruleset_id", Value::Str(class.to_ascii_lowercase()));
            item.set_attribute("visual_template", Value::Str("coin".into()));

            let entity = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
                .expect("inventory owner");
            entity.inventory.resize(4, None);
            entity.add_item(item).expect("free inventory slot");
        }

        fn add_official_inventory_item(
            &mut self,
            entity_id: u32,
            item_id: u32,
            group: &str,
            id: &str,
        ) {
            let (_, class_name, item, data) =
                official_item_from_rules(&self.ctx.rules, item_id, group, id);
            self.ctx.item_class_data.insert(class_name, data);
            let entity = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
                .expect("inventory owner");
            entity.inventory.resize(8, None);
            entity.add_item(item).expect("free inventory slot");
        }

        fn add_official_world_item(&mut self, item_id: u32, group: &str, id: &str, x: f32, z: f32) {
            let (_, class_name, mut item, data) =
                official_item_from_rules(&self.ctx.rules, item_id, group, id);
            item.position = Vec3::new(x, 1.0, z);
            self.ctx.item_class_data.insert(class_name, data);
            self.ctx.map.items.push(item);
        }

        fn equip_official_item(&mut self, entity_id: u32, item_id: u32, group: &str, id: &str) {
            let (slot, class_name, item, data) =
                official_item_from_rules(&self.ctx.rules, item_id, group, id);
            self.ctx.item_class_data.insert(class_name, data);
            let entity = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
                .expect("equipment owner");
            entity.equipped.insert(slot, item);
        }

        fn has_str_array_attr(&self, entity_id: u32, key: &str, expected: &str) -> bool {
            match self.entity(entity_id).attributes.get(key) {
                Some(Value::StrArray(values)) => values.iter().any(|value| value == expected),
                Some(Value::Str(value)) => value.split(',').map(str::trim).any(|v| v == expected),
                _ => false,
            }
        }

        fn set_entity_attr(&mut self, entity_id: u32, key: &str, value: Value) {
            let entity = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
                .expect("arena entity");
            entity.set_attribute(key, value);
        }

        fn clear_inventory(&mut self, entity_id: u32) {
            let entity = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|entity| entity.id == entity_id)
                .expect("arena entity");
            for slot in &mut entity.inventory {
                *slot = None;
            }
        }

        fn entity(&self, id: u32) -> &Entity {
            self.ctx
                .map
                .entities
                .iter()
                .find(|entity| entity.id == id)
                .expect("arena entity")
        }

        fn hp(&self, id: u32) -> i32 {
            self.entity(id).attributes.get_int("HP").expect("HP")
        }

        fn mp(&self, id: u32) -> i32 {
            self.entity(id).attributes.get_int_default("MP", 0)
        }

        fn mode(&self, id: u32) -> String {
            self.entity(id).get_mode()
        }

        fn attr_f32(&self, id: u32, key: &str) -> f32 {
            self.entity(id).attributes.get_float_default(key, -1.0)
        }

        fn attr_str(&self, id: u32, key: &str) -> String {
            self.entity(id)
                .attributes
                .get_str_default(key, String::new())
        }

        fn inventory_item_quantity(&self, entity_id: u32, ruleset_id: &str) -> i32 {
            self.entity(entity_id)
                .iter_inventory()
                .find_map(|(_, item)| {
                    item.attributes
                        .get_str("ruleset_id")
                        .filter(|id| id.trim() == ruleset_id)
                        .map(|_| item.attributes.get_int_default("quantity", 1))
                })
                .unwrap_or(0)
        }

        fn target(&self, id: u32) -> Option<u32> {
            self.entity(id).attributes.get_uint("target")
        }

        fn map_item(&self, id: u32) -> &Item {
            self.ctx
                .map
                .items
                .iter()
                .find(|item| item.id == id)
                .expect("map item")
        }

        fn message_texts(&self) -> Vec<(String, String)> {
            self._messages
                .try_iter()
                .filter_map(|message| match message {
                    RegionMessage::Message(_, _, _, _, text, role) => Some((text, role)),
                    _ => None,
                })
                .collect()
        }

        fn run_entity_event(&mut self, entity_id: u32, event: &str, payload: VMValue) {
            let program = self
                .ctx
                .entity_classes
                .get(&entity_id)
                .and_then(|class| self.ctx.entity_programs.get(class))
                .cloned();

            self.ctx.curr_entity_id = entity_id;
            self.ctx.curr_item_id = None;
            self.ctx.damage_committed = false;

            let is_damage_event = matches!(event, "damaged" | "take_damage");
            self.ctx.current_damage_kind = if is_damage_event {
                payload.as_string().map(str::to_string)
            } else {
                None
            };
            self.ctx.current_damage_source_item = if is_damage_event {
                let source_item_id = payload.z.max(0.0) as u32;
                (source_item_id > 0).then_some(source_item_id)
            } else {
                None
            };

            if let Some(program) = program {
                let args = [VMValue::from_string(event.to_string()), payload.clone()];
                run_server_fn(&mut self.exec, &args, &program, &mut self.ctx);
            }

            if is_damage_event && !self.ctx.damage_committed {
                let attacker_id = payload.x.max(0.0) as u32;
                let amount = payload.y.max(0.0) as i32;
                if amount > 0 {
                    let kind = self
                        .ctx
                        .current_damage_kind
                        .as_deref()
                        .unwrap_or("physical")
                        .to_string();
                    let source_item_id = self.ctx.current_damage_source_item;
                    let _ = apply_damage_direct(
                        &mut self.ctx,
                        entity_id,
                        attacker_id,
                        amount,
                        &kind,
                        source_item_id,
                    );
                }
            }

            self.ctx.current_damage_kind = None;
            self.ctx.current_damage_source_item = None;
            self.ctx.damage_committed = false;
        }

        fn drain_entity_events(&mut self) {
            while !self.ctx.to_execute_entity.is_empty() {
                let queued = std::mem::take(&mut self.ctx.to_execute_entity);
                for (entity_id, event, payload) in queued {
                    self.run_entity_event(entity_id, &event, payload);
                }
            }
        }
    }

    #[test]
    fn character_attack_triggers_damage_event_and_retaliation() {
        let mut arena = HeadlessRulesArena::new();
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
                if event == "damaged" {
                    set_attr("damaged_by", value.attacker_id);
                    set_attr("damage_taken", value.amount);
                }
            }
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("last_attacker", value.attacker_id);
                    set_attr("last_damage", value.amount);
                    set_attr("last_kind", value.kind);
                    set_attr("last_source_item", value.source_item_id);
                    set_target(value.attacker_id);
                    attack();
                }
            }
            "#,
        );
        arena.add_entity(1, "Warrior", 20, 3, Some(2));
        arena.add_entity(2, "Orc", 20, 2, None);

        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 17);
        assert_eq!(arena.attr_f32(2, "last_attacker") as u32, 1);
        assert_eq!(arena.attr_f32(2, "last_damage") as i32, 3);
        assert_eq!(arena.attr_str(2, "last_kind"), "physical");
        assert_eq!(arena.attr_f32(2, "last_source_item") as u32, 0);
        assert_eq!(arena.target(2), Some(1));
        assert_eq!(arena.hp(1), 18);
        assert_eq!(arena.attr_f32(1, "damaged_by") as u32, 2);
        assert_eq!(arena.attr_f32(1, "damage_taken") as i32, 2);
    }

    #[test]
    fn character_attack_cooldown_blocks_repeated_interaction_damage() {
        let mut arena = HeadlessRulesArena::new();
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
            }
            "#,
        );
        arena.add_script_class(
            "Target",
            r#"
            fn event(event, value) {
            }
            "#,
        );
        arena.add_entity(1, "Warrior", 20, 4, Some(2));
        arena.add_entity(2, "Target", 15, 0, None);

        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();
        assert_eq!(arena.hp(2), 11);

        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();
        assert_eq!(arena.hp(2), 11);

        arena.ctx.ticks += 10;
        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();
        assert_eq!(arena.hp(2), 7);
    }

    #[test]
    fn lethal_attack_fires_death_kill_once_and_drops_loot() {
        let mut arena = HeadlessRulesArena::new();
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
                if event == "kill" {
                    set_attr("kill_count", get_attr("kill_count") + 1);
                    set_attr("last_kill", value);
                }
                if event == "damaged" {
                    set_attr("damage_taken", get_attr("damage_taken") + value.amount);
                }
            }
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_target(value.attacker_id);
                    attack();
                }
                if event == "death" {
                    set_attr("death_count", get_attr("death_count") + 1);
                    drop_items("");
                }
            }
            "#,
        );
        arena.add_entity(1, "Warrior", 20, 8, Some(2));
        arena.add_entity(2, "Orc", 5, 2, None);
        arena.add_inventory_item(2, 10, "Golden Key");

        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 0);
        assert_eq!(arena.mode(2), "dead");
        assert_eq!(arena.attr_f32(2, "death_count") as i32, 1);
        assert_eq!(arena.attr_f32(1, "kill_count") as i32, 1);
        assert_eq!(arena.attr_f32(1, "last_kill") as u32, 2);
        assert_eq!(arena.entity(2).iter_inventory().count(), 0);
        assert_eq!(arena.ctx.map.items.len(), 1);

        let dropped = arena.map_item(10);
        assert_eq!(dropped.attributes.get_str("class_name"), Some("Golden Key"));
        assert_eq!(dropped.attributes.get_str("visual_template"), Some("coin"));
        assert_eq!(dropped.position, arena.entity(2).position);

        let warrior_hp_after_first_attack = arena.hp(1);
        arena.ctx.ticks += 10;
        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 0);
        assert_eq!(arena.attr_f32(2, "death_count") as i32, 1);
        assert_eq!(arena.attr_f32(1, "kill_count") as i32, 1);
        assert_eq!(arena.ctx.map.items.len(), 1);
        assert_eq!(arena.hp(1), warrior_hp_after_first_attack);
    }

    #[test]
    fn dead_drop_items_uses_ruleset_loot_container() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [loot.corpse]
            enabled = true
            item = "loot_corpse"
            include_equipped = true
            create_empty = false
            despawn_seconds = 600
            despawn_before_respawn_seconds = 1
            name = "{name}'s Remains"
            description = "Search {name} for carried loot."

            [items.containers.loot_corpse]
            name = "Loot Corpse"
            description = "A lootable body containing carried items."
            category = "corpse"
            slot = "container"
            rarity = "common"
            container_template = "default"
            visual_template = "bag_pouch"

            [items.containers.loot_corpse.attributes]
            container = true
            container_slots = 8
            static = true
            takeable = false
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "death" {
                    drop_items("");
                }
            }
            "#,
        );
        arena.add_entity(2, "Orc", 0, 0, None);
        arena.set_entity_attr(2, "mode", Value::Str("dead".into()));
        arena.add_inventory_item(2, 10, "Golden Key");

        arena.run_entity_event(2, "death", VMValue::zero());

        assert_eq!(arena.entity(2).iter_inventory().count(), 0);
        assert_eq!(arena.ctx.map.items.len(), 1);
        let corpse = &arena.ctx.map.items[0];
        assert_eq!(corpse.attributes.get_str("name"), Some("Orc's Remains"));
        assert_eq!(corpse.attributes.get_bool_default("static", false), true);
        assert_eq!(corpse.attributes.get_bool_default("takeable", true), false);
        let contents = corpse.container.as_ref().expect("corpse contents");
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].id, 10);
        assert_eq!(
            contents[0].attributes.get_str("class_name"),
            Some("Golden Key")
        );
    }

    #[test]
    fn dead_npc_respawns_at_full_health_and_removes_corpse() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [loot.corpse]
            enabled = true
            item = "loot_corpse"
            include_equipped = true
            create_empty = false
            despawn_seconds = 600
            despawn_before_respawn_seconds = 1
            name = "{name}'s Remains"
            description = "Search {name} for carried loot."

            [respawn.npc]
            enabled = true
            delay_seconds = 2
            health = "full"
            clear_corpse_on_respawn = true

            [items.containers.loot_corpse]
            name = "Loot Corpse"
            description = "A lootable body containing carried items."
            category = "corpse"
            slot = "container"
            rarity = "common"
            container_template = "default"
            visual_template = "bag_pouch"

            [items.containers.loot_corpse.attributes]
            container = true
            container_slots = 8
            static = true
            takeable = false
            "#,
        );
        arena.add_entity(2, "Orc", 10, 2, None);
        arena.add_inventory_item(2, 10, "Golden Key");
        arena.ctx.map.entities[0].action =
            EntityAction::RandomWalkInSector(1.0, 1.0, 4, 0, Vec2::zero());
        arena.ctx.entity_proximity_alerts.insert(2, 4.0);
        update_entity_respawns(&mut arena.ctx);

        arena.set_entity_attr(2, "HP", Value::Int(0));
        arena.set_entity_attr(2, "mode", Value::Str("dead".into()));
        arena.set_entity_attr(2, "visible", Value::Bool(false));
        arena.ctx.map.entities[0].action = EntityAction::Off;
        arena.ctx.entity_proximity_alerts.remove(&2);
        arena.ctx.map.entities[0].set_position(Vec3::new(7.5, 1.0, 3.5));

        assert!(drop_items_into_ruleset_loot_container(
            &mut arena.ctx,
            2,
            ""
        ));
        assert_eq!(arena.ctx.map.items.len(), 1);
        assert_eq!(
            arena.ctx.map.items[0]
                .attributes
                .get("despawn_at_tick")
                .and_then(|value| match value {
                    Value::Int64(value) => Some(*value),
                    Value::Int(value) => Some(*value as i64),
                    _ => None,
                }),
            Some(4)
        );
        assert_eq!(arena.entity(2).iter_inventory().count(), 0);

        update_entity_respawns(&mut arena.ctx);
        assert_eq!(arena.mode(2), "dead");
        assert_eq!(arena.ctx.map.items.len(), 1);

        arena.ctx.ticks = 4;
        update_entity_respawns(&mut arena.ctx);
        assert_eq!(arena.mode(2), "dead");
        assert!(arena.ctx.map.items.is_empty());

        arena.ctx.ticks = 8;
        arena.ctx.map.entities[0].set_position(Vec3::new(1.5, 1.0, 1.5));
        update_entity_respawns(&mut arena.ctx);

        assert_eq!(arena.mode(2), "active");
        assert_eq!(arena.hp(2), 10);
        assert!(arena.entity(2).get_update().snap_position);
        assert!(
            arena
                .entity(2)
                .attributes
                .get_bool_default("visible", false)
        );
        assert_eq!(arena.entity(2).position, Vec3::new(2.0, 1.0, 0.0));
        assert!(arena.ctx.map.items.is_empty());
        assert_eq!(arena.entity(2).iter_inventory().count(), 1);
        assert!(matches!(
            arena.entity(2).action,
            EntityAction::RandomWalkInSector(_, _, _, _, _)
        ));
        assert_eq!(arena.ctx.entity_proximity_alerts.get(&2), Some(&4.0));
        assert!(
            arena
                .ctx
                .to_execute_entity
                .iter()
                .any(|(id, event, _)| *id == 2 && event == "respawn")
        );

        arena.set_entity_attr(2, "HP", Value::Int(0));
        arena.set_entity_attr(2, "mode", Value::Str("dead".into()));
        arena.set_entity_attr(2, "visible", Value::Bool(false));
        assert!(drop_items_into_ruleset_loot_container(
            &mut arena.ctx,
            2,
            ""
        ));
        assert_eq!(arena.ctx.map.items.len(), 1);
    }

    #[test]
    fn player_death_does_not_auto_respawn() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [respawn.npc]
            enabled = true
            delay_seconds = 1
            "#,
        );
        arena.add_entity(1, "Player", 10, 2, None);
        arena.set_entity_attr(1, "player", Value::Bool(true));
        arena.set_entity_attr(1, "HP", Value::Int(0));
        arena.set_entity_attr(1, "mode", Value::Str("dead".into()));
        arena.set_entity_attr(1, "visible", Value::Bool(false));

        update_entity_respawns(&mut arena.ctx);
        arena.ctx.ticks = 10;
        update_entity_respawns(&mut arena.ctx);

        assert_eq!(arena.mode(1), "dead");
        assert_eq!(arena.hp(1), 0);
        assert!(!arena.entity(1).attributes.get_bool_default("visible", true));
    }

    #[test]
    fn ruleset_spell_damage_uses_damaged_event_cost_and_cooldown() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [race_relations.Human]
            Orc = "hostile"

            [actions.holy_light]
            name = "Holy Light"
            kind = "spell"
            requires = { spell = "holy_light" }
            target = "hostile_entity"
            range = 5
            cooldown = 5.0
            cost = { MP = 4 }
            result = { damage = "spells.holy_light.damage" }

            [spells.holy_light]
            name = "Holy Light"
            kind = "damage"
            damage_kind = "arcane"
            range = 5
            cost_mp = 4

            [spells.holy_light.damage]
            roll = "1d1"
            bonus = 3
            damage_kind = "arcane"
            "#,
        );
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "holy_light" {
                    use_action("holy_light");
                }
            }
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("last_attacker", value.attacker_id);
                    set_attr("last_damage", value.amount);
                    set_attr("last_kind", value.kind);
                    set_attr("last_source_item", value.source_item_id);
                }
            }
            "#,
        );
        arena.add_entity(1, "Cleric", 20, 1, Some(2));
        arena.add_entity(2, "Orc", 20, 1, None);
        arena.set_entity_attr(1, "race", Value::Str("Human".into()));
        arena.set_entity_attr(2, "race", Value::Str("Orc".into()));
        arena.set_entity_attr(1, "MP", Value::Int(10));

        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        assert_eq!(arena.hp(2), 20);
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 16);
        assert_eq!(arena.mp(1), 6);
        assert!(is_spell_on_cooldown(&arena.ctx, 1, "holy_light"));
        assert_eq!(arena.attr_f32(2, "last_attacker") as u32, 1);
        assert_eq!(arena.attr_f32(2, "last_damage") as i32, 4);
        assert_eq!(arena.attr_str(2, "last_kind"), "arcane");
        assert_eq!(arena.attr_f32(2, "last_source_item") as u32, 0);

        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 16);
        assert_eq!(arena.mp(1), 6);
    }

    #[test]
    fn ruleset_minor_heal_spends_mp_respects_max_hp_and_cooldown() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
            requires = { spell = "minor_heal" }
            target = "friendly_or_self"
            range = 5
            cooldown = 4.0
            cost = { MP = 3 }
            result = { healing = "spells.minor_heal.healing" }

            [spells.minor_heal]
            name = "Minor Heal"
            kind = "heal"
            range = 5
            cost_mp = 3

            [spells.minor_heal.healing]
            roll = "1d1"
            bonus = 5
            "#,
        );
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "minor_heal" {
                    use_action("minor_heal");
                }
            }
            "#,
        );
        arena.add_entity(1, "Cleric", 6, 1, Some(1));
        arena.set_entity_attr(1, "MAX_HP", Value::Int(10));
        arena.set_entity_attr(1, "MP", Value::Int(8));

        arena.run_entity_event(1, "intent", VMValue::from_string("minor_heal"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(1), 10);
        assert_eq!(arena.mp(1), 5);
        assert!(is_spell_on_cooldown(&arena.ctx, 1, "minor_heal"));

        arena.run_entity_event(1, "intent", VMValue::from_string("minor_heal"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(1), 10);
        assert_eq!(arena.mp(1), 5);
    }

    #[test]
    fn ruleset_spells_do_not_spend_resources_on_invalid_targets() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [race_relations.Human]
            Orc = "hostile"

            [actions.holy_light]
            name = "Holy Light"
            kind = "spell"
            requires = { spell = "holy_light" }
            target = "hostile_entity"
            range = 5
            cooldown = 5.0
            cost = { MP = 4 }
            result = { damage = "spells.holy_light.damage" }

            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
            requires = { spell = "minor_heal" }
            target = "friendly_or_self"
            range = 5
            cooldown = 4.0
            cost = { MP = 3 }
            result = { healing = "spells.minor_heal.healing" }

            [spells.holy_light]
            name = "Holy Light"
            kind = "damage"
            damage_kind = "arcane"
            range = 5
            cost_mp = 4

            [spells.holy_light.damage]
            roll = "1d1"
            bonus = 3
            damage_kind = "arcane"

            [spells.minor_heal]
            name = "Minor Heal"
            kind = "heal"
            range = 5
            cost_mp = 3

            [spells.minor_heal.healing]
            roll = "1d1"
            bonus = 5
            "#,
        );
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "holy_light" {
                    use_action("holy_light");
                }
                if event == "intent" && value == "minor_heal" {
                    use_action("minor_heal");
                }
            }
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("damage_events", get_attr("damage_events") + 1);
                }
            }
            "#,
        );
        arena.add_entity(1, "Cleric", 10, 1, Some(1));
        arena.add_entity(2, "Orc", 5, 1, None);
        arena.set_entity_attr(1, "race", Value::Str("Human".into()));
        arena.set_entity_attr(2, "race", Value::Str("Orc".into()));
        arena.set_entity_attr(1, "MAX_HP", Value::Int(10));
        arena.set_entity_attr(2, "MAX_HP", Value::Int(10));
        arena.set_entity_attr(1, "MP", Value::Int(10));

        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(1), 10);
        assert_eq!(arena.mp(1), 10);
        assert!(!is_spell_on_cooldown(&arena.ctx, 1, "holy_light"));

        arena.set_entity_attr(1, "target", Value::UInt(2));
        arena.set_entity_attr(1, "attack_target", Value::UInt(2));
        arena.run_entity_event(1, "intent", VMValue::from_string("minor_heal"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 5);
        assert_eq!(arena.mp(1), 10);
        assert!(!is_spell_on_cooldown(&arena.ctx, 1, "minor_heal"));
        assert_eq!(arena.attr_f32(2, "damage_events") as i32, -1);
    }

    #[test]
    fn ruleset_spell_failure_for_not_enough_mp_has_no_side_effects() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [race_relations.Human]
            Orc = "hostile"

            [actions.holy_light]
            name = "Holy Light"
            kind = "spell"
            requires = { spell = "holy_light" }
            target = "hostile_entity"
            range = 5
            cooldown = 5.0
            cost = { MP = 4 }
            result = { damage = "spells.holy_light.damage" }

            [spells.holy_light]
            name = "Holy Light"
            kind = "damage"
            damage_kind = "arcane"
            range = 5
            cost_mp = 4

            [spells.holy_light.damage]
            roll = "1d1"
            bonus = 3
            damage_kind = "arcane"
            "#,
        );
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "holy_light" {
                    use_action("holy_light");
                }
            }
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("damage_events", get_attr("damage_events") + 1);
                }
            }
            "#,
        );
        arena.add_entity(1, "Cleric", 10, 1, Some(2));
        arena.add_entity(2, "Orc", 10, 1, None);
        arena.set_entity_attr(1, "race", Value::Str("Human".into()));
        arena.set_entity_attr(2, "race", Value::Str("Orc".into()));
        arena.set_entity_attr(1, "MP", Value::Int(3));

        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 10);
        assert_eq!(arena.mp(1), 3);
        assert!(!is_spell_on_cooldown(&arena.ctx, 1, "holy_light"));
        assert_eq!(arena.attr_f32(2, "damage_events") as i32, -1);
    }

    #[test]
    fn lethal_ruleset_spell_fires_death_and_kill_once() {
        let mut arena = HeadlessRulesArena::with_rules(
            r#"
            [race_relations.Human]
            Orc = "hostile"

            [actions.holy_light]
            name = "Holy Light"
            kind = "spell"
            requires = { spell = "holy_light" }
            target = "hostile_entity"
            range = 5
            cooldown = 5.0
            cost = { MP = 4 }
            result = { damage = "spells.holy_light.damage" }

            [spells.holy_light]
            name = "Holy Light"
            kind = "damage"
            damage_kind = "arcane"
            range = 5
            cost_mp = 4

            [spells.holy_light.damage]
            roll = "1d1"
            bonus = 7
            damage_kind = "arcane"
            "#,
        );
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "holy_light" {
                    use_action("holy_light");
                }
                if event == "kill" {
                    set_attr("kill_count", get_attr("kill_count") + 1);
                    set_attr("last_kill", value);
                }
            }
            "#,
        );
        arena.add_script_class(
            "Orc",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("damage_events", get_attr("damage_events") + 1);
                    set_target(value.attacker_id);
                    attack();
                }
                if event == "death" {
                    set_attr("death_count", get_attr("death_count") + 1);
                }
            }
            "#,
        );
        arena.add_entity(1, "Cleric", 20, 1, Some(2));
        arena.add_entity(2, "Orc", 5, 2, None);
        arena.set_entity_attr(1, "race", Value::Str("Human".into()));
        arena.set_entity_attr(2, "race", Value::Str("Orc".into()));
        arena.set_entity_attr(1, "MP", Value::Int(10));

        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), 0);
        assert_eq!(arena.mode(2), "dead");
        assert_eq!(arena.attr_f32(2, "damage_events") as i32, 1);
        assert_eq!(arena.attr_f32(2, "death_count") as i32, 1);
        assert_eq!(arena.attr_f32(1, "kill_count") as i32, 1);
        assert_eq!(arena.attr_f32(1, "last_kill") as u32, 2);
        assert_eq!(arena.hp(1), 20);

        arena.ctx.ticks += 50;
        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        arena.drain_entity_events();

        assert_eq!(arena.attr_f32(2, "damage_events") as i32, 1);
        assert_eq!(arena.attr_f32(2, "death_count") as i32, 1);
        assert_eq!(arena.attr_f32(1, "kill_count") as i32, 1);
    }

    #[test]
    fn official_ruleset_applies_character_defaults_and_unlocks() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_official_entity(1, "Warrior", "Human", 1, None);
        arena.add_official_entity(2, "Cleric", "Human", 1, None);
        arena.add_official_entity(3, "Cleric", "Human", 2, None);
        arena.add_official_entity(4, "Ranger", "Human", 1, None);
        arena.add_official_entity(5, "Citizen", "Human", 1, None);
        arena.set_entity_attr(5, "profession", Value::Str("Blacksmith".into()));

        assert_eq!(arena.hp(1), 16);
        assert_eq!(arena.entity(1).attributes.get_int_default("MAX_HP", 0), 16);
        assert_eq!(arena.entity(1).attributes.get_int_default("STR", 0), 12);
        assert_eq!(arena.entity(1).attributes.get_int_default("ARMOR", 0), 1);
        assert!(arena.has_str_array_attr(1, "start_equipped_items", "training_sword"));
        assert!(arena.has_str_array_attr(1, "start_equipped_items", "padded_armor"));
        assert!(arena.has_str_array_attr(1, "start_items", "linen_shirt"));
        assert!(arena.has_str_array_attr(1, "abilities", "basic_attack"));
        assert!(arena.has_str_array_attr(1, "abilities", "guard"));

        assert!(arena.has_str_array_attr(2, "spells", "minor_heal"));
        assert!(arena.has_str_array_attr(2, "start_items", "blessed_herb"));
        assert!(!arena.has_str_array_attr(2, "spells", "holy_light"));
        assert!(arena.has_str_array_attr(3, "spells", "minor_heal"));
        assert!(arena.has_str_array_attr(3, "spells", "holy_light"));
        assert_eq!(arena.entity(3).attributes.get_int_default("MAX_MP", 0), 11);

        assert_eq!(arena.hp(4), 14);
        assert_eq!(arena.entity(4).attributes.get_int_default("DEX", 0), 12);
        assert!(arena.has_str_array_attr(4, "start_equipped_items", "hunting_bow"));
        assert!(arena.has_str_array_attr(4, "start_items", "wooden_arrows"));

        assert_eq!(arena.hp(5), 10);
        assert_eq!(arena.attr_str(5, "profession"), "Blacksmith");
        assert!(arena.has_str_array_attr(5, "start_equipped_items", "linen_shirt"));
        assert!(!arena.has_str_array_attr(5, "abilities", "basic_attack"));
    }

    #[test]
    fn official_ruleset_warrior_attack_uses_weapon_and_hostility() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
                if event == "damaged" {
                    set_attr("last_attacker", value.attacker_id);
                    set_attr("last_damage", value.amount);
                    set_attr("last_kind", value.kind);
                    set_attr("last_source_item", value.source_item_id);
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Warrior", "Human", 1, Some(2));
        arena.add_official_entity(2, "Warrior", "Orc", 1, None);
        arena.equip_official_item(1, 101, "weapons", "training_sword");

        assert_eq!(
            entity_disposition_by_id(&arena.ctx, 1, 2).as_deref(),
            Some("hostile")
        );
        assert!(entity_is_hostile_by_id(&arena.ctx, 1, 2));
        assert_eq!(
            current_attack_cooldown_for_entity(&arena.ctx, arena.entity(1)),
            1.0
        );
        let base_damage = current_attack_base_damage_for_entity(&arena.ctx, 1);
        assert!((5..=10).contains(&base_damage));

        let orc_hp = arena.hp(2);
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "basic_attack",
            Some(2)
        ));
        arena.drain_entity_events();

        assert!(arena.hp(2) < orc_hp);
        assert_eq!(arena.attr_str(2, "last_kind"), "physical");
        assert_eq!(arena.attr_f32(2, "last_attacker") as u32, 1);
        assert_eq!(arena.attr_f32(2, "last_source_item") as u32, 101);
        assert!(arena.attr_f32(2, "last_damage") >= 1.0);
    }

    #[test]
    fn official_ruleset_cleric_spells_use_costs_cooldowns_and_unlocks() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "holy_light" {
                    use_action("holy_light");
                }
                if event == "intent" && value == "minor_heal" {
                    use_action("minor_heal");
                }
            }
            "#,
        );
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("last_attacker", value.attacker_id);
                    set_attr("last_damage", value.amount);
                    set_attr("last_kind", value.kind);
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Cleric", "Human", 2, Some(2));
        arena.add_official_entity(2, "Warrior", "Orc", 1, None);
        arena.add_official_inventory_item(1, 201, "reagents", "blessed_herb");

        assert!(arena.has_str_array_attr(1, "spells", "minor_heal"));
        assert!(arena.has_str_array_attr(1, "spells", "holy_light"));
        assert_eq!(arena.mp(1), 11);
        assert_eq!(arena.inventory_item_quantity(1, "blessed_herb"), 3);
        assert_eq!(
            entity_disposition_by_id(&arena.ctx, 1, 2).as_deref(),
            Some("hostile")
        );

        let orc_hp = arena.hp(2);
        arena.run_entity_event(1, "intent", VMValue::from_string("holy_light"));
        assert_eq!(arena.hp(2), orc_hp);
        arena.drain_entity_events();

        assert!(arena.hp(2) < orc_hp);
        assert_eq!(arena.mp(1), 7);
        assert!(is_spell_on_cooldown(&arena.ctx, 1, "holy_light"));
        assert_eq!(arena.attr_str(2, "last_kind"), "arcane");
        assert_eq!(arena.attr_f32(2, "last_attacker") as u32, 1);

        arena.set_entity_attr(1, "HP", Value::Int(5));
        arena.set_entity_attr(1, "target", Value::UInt(1));
        arena.set_entity_attr(1, "attack_target", Value::UInt(1));
        arena.run_entity_event(1, "intent", VMValue::from_string("minor_heal"));
        arena.drain_entity_events();

        assert!(arena.hp(1) > 5);
        assert!(arena.hp(1) <= arena.entity(1).attributes.get_int_default("MAX_HP", 0));
        assert_eq!(arena.mp(1), 4);
        assert_eq!(arena.inventory_item_quantity(1, "blessed_herb"), 2);
        assert!(is_spell_on_cooldown(&arena.ctx, 1, "minor_heal"));
    }

    #[test]
    fn official_ruleset_minor_heal_requires_blessed_herb() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Cleric",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "minor_heal" {
                    use_action("minor_heal");
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Cleric", "Human", 1, Some(1));
        arena.set_entity_attr(1, "HP", Value::Int(5));
        arena.set_entity_attr(1, "target", Value::UInt(1));
        arena.set_entity_attr(1, "attack_target", Value::UInt(1));

        arena.run_entity_event(1, "intent", VMValue::from_string("minor_heal"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(1), 5);
        assert_eq!(arena.mp(1), 8);
        assert_eq!(arena.inventory_item_quantity(1, "blessed_herb"), 0);
        assert!(!is_spell_on_cooldown(&arena.ctx, 1, "minor_heal"));
    }

    #[test]
    fn official_ruleset_recipe_consumes_material_stacks_and_merges_outputs() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_official_entity(1, "Ranger", "Human", 1, None);
        arena.add_official_inventory_item(1, 201, "materials", "green_wood");
        arena.add_official_inventory_item(1, 202, "materials", "feather");
        arena.add_official_inventory_item(1, 203, "ammunition", "wooden_arrows");

        assert!(craft_ruleset_recipe(&mut arena.ctx, 1, "wooden_arrows"));

        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 4);
        assert_eq!(arena.inventory_item_quantity(1, "feather"), 3);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 30);
    }

    #[test]
    fn official_ruleset_craft_actions_execute_recipes() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.load_official_locales();
        arena.add_official_entity(1, "Ranger", "Human", 1, None);
        arena.add_official_inventory_item(1, 201, "materials", "green_wood");
        arena.add_official_inventory_item(1, 202, "materials", "feather");

        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "craft_wooden_arrows",
            None
        ));

        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 4);
        assert_eq!(arena.inventory_item_quantity(1, "feather"), 3);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 10);
        assert!(
            arena
                .message_texts()
                .iter()
                .any(|(text, role)| text == "Crafted Wooden Arrows" && role == "success")
        );
    }

    #[test]
    fn official_ruleset_recipe_missing_material_has_no_side_effects() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_official_entity(1, "Ranger", "Human", 1, None);
        arena.add_official_inventory_item(1, 201, "materials", "green_wood");

        assert!(!craft_ruleset_recipe(&mut arena.ctx, 1, "wooden_arrows"));

        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 5);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 0);
    }

    #[test]
    fn official_ruleset_gather_herbs_uses_resource_node_and_respawns() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.load_official_locales();
        arena.add_official_entity(1, "Citizen", "Human", 1, None);
        arena.add_official_world_item(301, "resources", "wild_herb_node", 1.0, 0.0);

        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_herbs",
            Some(301)
        ));
        assert_eq!(arena.inventory_item_quantity(1, "wild_herb"), 2);
        assert!(
            arena
                .message_texts()
                .iter()
                .any(|(text, role)| text == "You gather Wild Herb x2" && role == "success")
        );
        let node = arena
            .ctx
            .map
            .items
            .iter()
            .find(|item| item.id == 301)
            .expect("wild herb node");
        assert!(node.attributes.get_bool_default("resource_depleted", false));
        assert!(!node.attributes.get_bool_default("visible", true));

        assert!(!execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_herbs",
            Some(301)
        ));

        arena.ctx.delta_time = 300.0;
        crate::server::region::update_spell_items(&mut arena.ctx);
        arena.ctx.entity_state_data.clear();

        let node = arena
            .ctx
            .map
            .items
            .iter()
            .find(|item| item.id == 301)
            .expect("wild herb node");
        assert!(!node.attributes.get_bool_default("resource_depleted", true));
        assert!(node.attributes.get_bool_default("visible", false));

        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_herbs",
            Some(301)
        ));
        assert_eq!(arena.inventory_item_quantity(1, "wild_herb"), 4);
    }

    #[test]
    fn official_ruleset_gather_wood_uses_resource_node_and_skill_gate() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_official_entity(1, "Citizen", "Human", 1, None);
        arena.add_official_world_item(301, "resources", "green_wood_node", 1.0, 0.0);

        arena
            .ctx
            .rules
            .get_mut("actions")
            .and_then(toml::Value::as_table_mut)
            .and_then(|actions| actions.get_mut("gather_wood"))
            .and_then(toml::Value::as_table_mut)
            .expect("gather_wood action")
            .insert("required_skill".into(), toml::Value::Integer(25));

        assert!(!execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_wood",
            Some(301)
        ));
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 0);

        arena.set_entity_attr(1, "skill_woodworking", Value::Int(25));

        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_wood",
            Some(301)
        ));
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 3);
        let node = arena
            .ctx
            .map
            .items
            .iter()
            .find(|item| item.id == 301)
            .expect("green wood node");
        assert!(node.attributes.get_bool_default("resource_depleted", false));
    }

    #[test]
    fn official_ruleset_gathered_materials_craft_arrows_and_power_bow() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Ranger",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Ranger", "Human", 1, Some(2));
        arena.add_official_entity(2, "Warrior", "Orc", 1, None);
        arena.clear_inventory(1);
        arena.equip_official_item(1, 101, "weapons", "hunting_bow");
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 1)
            .and_then(|entity| entity.equipped.get_mut("main_hand"))
            .expect("ranger bow")
            .set_attribute("quality", Value::Int(20));
        arena.add_official_world_item(301, "resources", "green_wood_node", 1.0, 0.0);
        arena.add_official_world_item(302, "resources", "bird_nest_node", 1.0, 0.0);

        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_wood",
            Some(301)
        ));
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_feathers",
            Some(302)
        ));
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 3);
        assert_eq!(arena.inventory_item_quantity(1, "feather"), 2);

        assert!(craft_ruleset_recipe(&mut arena.ctx, 1, "wooden_arrows"));
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 2);
        assert_eq!(arena.inventory_item_quantity(1, "feather"), 0);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 10);

        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 1)
            .unwrap()
            .position = Vec3::new(0.0, 1.0, 0.0);
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 2)
            .unwrap()
            .position = Vec3::new(4.0, 1.0, 0.0);

        let orc_hp = arena.hp(2);
        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert!(arena.hp(2) < orc_hp);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 9);
    }

    #[test]
    fn official_ruleset_end_to_end_resource_craft_combat_and_spells() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.load_official_locales();
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("last_attacker", value.attacker_id);
                    set_attr("last_damage", value.amount);
                    set_attr("last_kind", value.kind);
                    set_attr("last_source_item", value.source_item_id);
                    set_target(value.attacker_id);
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Ranger", "Human", 1, None);
        arena.add_official_entity(2, "Cleric", "Human", 2, None);
        arena.add_official_entity(3, "Warrior", "Orc", 1, None);
        arena.add_official_entity(4, "Warrior", "Orc", 1, None);
        arena.clear_inventory(1);
        arena.clear_inventory(2);
        arena.equip_official_item(1, 101, "weapons", "hunting_bow");
        arena.add_official_inventory_item(2, 201, "reagents", "blessed_herb");
        arena.add_official_world_item(301, "resources", "wild_herb_node", 1.0, 0.0);
        arena.add_official_world_item(302, "resources", "green_wood_node", 1.0, 0.0);
        arena.add_official_world_item(303, "resources", "green_wood_node", 1.0, 0.5);
        arena.add_official_world_item(304, "resources", "bird_nest_node", 1.0, 1.0);

        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_herbs",
            Some(301)
        ));
        assert!(
            arena
                .message_texts()
                .iter()
                .any(|(text, role)| text == "You gather Wild Herb x2" && role == "success")
        );
        arena.ctx.entity_state_data.clear();
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_wood",
            Some(302)
        ));
        arena.ctx.entity_state_data.clear();
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_wood",
            Some(303)
        ));
        arena.ctx.entity_state_data.clear();
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "gather_feathers",
            Some(304)
        ));
        assert_eq!(arena.inventory_item_quantity(1, "wild_herb"), 2);
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 6);
        assert_eq!(arena.inventory_item_quantity(1, "feather"), 2);

        assert!(craft_ruleset_recipe(&mut arena.ctx, 1, "wooden_arrows"));
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 10);
        assert!(craft_ruleset_recipe(&mut arena.ctx, 1, "hunting_bow"));
        assert_eq!(arena.inventory_item_quantity(1, "hunting_bow"), 1);

        arena.set_entity_attr(3, "HP", Value::Int(30));
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 1)
            .unwrap()
            .position = Vec3::new(0.0, 1.0, 0.0);
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 3)
            .unwrap()
            .position = Vec3::new(4.0, 1.0, 0.0);

        let orc_hp = arena.hp(3);
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            1,
            "basic_attack",
            Some(3)
        ));
        arena.drain_entity_events();
        assert!(arena.hp(3) < orc_hp);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 9);
        assert_eq!(arena.attr_f32(3, "last_attacker") as u32, 1);
        assert_eq!(arena.attr_str(3, "last_kind"), "physical");
        assert_eq!(arena.target(3), Some(1));

        arena.set_entity_attr(2, "HP", Value::Int(6));
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            2,
            "minor_heal",
            Some(2)
        ));
        assert!(arena.hp(2) > 6);
        assert_eq!(arena.inventory_item_quantity(2, "blessed_herb"), 2);
        arena.ctx.entity_state_data.clear();

        arena.set_entity_attr(4, "HP", Value::Int(30));
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 2)
            .unwrap()
            .position = Vec3::new(0.0, 1.0, 0.0);
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 4)
            .unwrap()
            .position = Vec3::new(3.0, 1.0, 0.0);

        let second_orc_hp = arena.hp(4);
        assert!(execute_ruleset_action(
            &mut arena.ctx,
            2,
            "holy_light",
            Some(4)
        ));
        arena.drain_entity_events();
        assert!(arena.hp(4) < second_orc_hp);
        assert_eq!(arena.attr_f32(4, "last_attacker") as u32, 2);
        assert_eq!(arena.attr_str(4, "last_kind"), "arcane");
    }

    #[test]
    fn official_ruleset_blessed_herb_requires_cleric_spell_and_wild_herb() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_official_entity(1, "Warrior", "Human", 1, None);
        arena.add_official_entity(2, "Cleric", "Human", 1, None);
        arena.add_official_inventory_item(1, 201, "materials", "wild_herb");
        arena.add_official_inventory_item(2, 202, "materials", "wild_herb");

        assert!(!craft_ruleset_recipe(&mut arena.ctx, 1, "blessed_herb"));
        assert_eq!(arena.inventory_item_quantity(1, "wild_herb"), 5);
        assert_eq!(arena.inventory_item_quantity(1, "blessed_herb"), 0);

        assert!(craft_ruleset_recipe(&mut arena.ctx, 2, "blessed_herb"));
        assert_eq!(arena.inventory_item_quantity(2, "wild_herb"), 4);
        assert_eq!(arena.inventory_item_quantity(2, "blessed_herb"), 1);
    }

    #[test]
    fn official_ruleset_recipe_skill_sets_output_quality() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_official_entity(1, "Ranger", "Human", 1, None);
        arena.set_entity_attr(1, "skill_fletching", Value::Int(0));
        arena.add_official_inventory_item(1, 201, "materials", "green_wood");

        assert!(craft_ruleset_recipe(&mut arena.ctx, 1, "hunting_bow"));
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 2);
        assert_eq!(arena.inventory_item_quantity(1, "hunting_bow"), 1);
        let low_quality = arena
            .ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == 1)
            .and_then(|entity| {
                entity.iter_inventory().find_map(|(_, item)| {
                    (item.attributes.get_str("ruleset_id") == Some("hunting_bow"))
                        .then(|| item.attributes.get_int_default("quality", 100))
                })
            })
            .unwrap();
        assert!(low_quality < 50);

        arena.clear_inventory(1);
        arena.add_official_inventory_item(1, 202, "materials", "green_wood");
        arena.set_entity_attr(1, "skill_fletching", Value::Int(25));

        assert!(craft_ruleset_recipe(&mut arena.ctx, 1, "hunting_bow"));
        assert_eq!(arena.inventory_item_quantity(1, "green_wood"), 2);
        assert_eq!(arena.inventory_item_quantity(1, "hunting_bow"), 1);
        let better_quality = arena
            .ctx
            .map
            .entities
            .iter()
            .find(|entity| entity.id == 1)
            .and_then(|entity| {
                entity.iter_inventory().find_map(|(_, item)| {
                    (item.attributes.get_str("ruleset_id") == Some("hunting_bow"))
                        .then(|| item.attributes.get_int_default("quality", 100))
                })
            })
            .unwrap();
        assert!(better_quality > low_quality);
        assert_eq!(better_quality, 58);
    }

    #[test]
    fn official_ruleset_ranger_uses_bow_damage_and_cooldown() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Ranger",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
            }
            "#,
        );
        arena.add_script_class(
            "Warrior",
            r#"
            fn event(event, value) {
                if event == "damaged" {
                    set_attr("last_attacker", value.attacker_id);
                    set_attr("last_damage", value.amount);
                    set_attr("last_kind", value.kind);
                    set_attr("last_source_item", value.source_item_id);
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Ranger", "Human", 1, Some(2));
        arena.add_official_entity(2, "Warrior", "Orc", 1, None);
        arena.equip_official_item(1, 101, "weapons", "hunting_bow");
        arena.add_official_inventory_item(1, 201, "ammunition", "wooden_arrows");
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 1)
            .unwrap()
            .position = Vec3::new(0.0, 1.0, 0.0);
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 2)
            .unwrap()
            .position = Vec3::new(4.0, 1.0, 0.0);

        assert_eq!(
            current_attack_cooldown_for_entity(&arena.ctx, arena.entity(1)),
            1.5
        );
        let base_damage = current_attack_base_damage_for_entity(&arena.ctx, 1);
        assert!((4..=9).contains(&base_damage));

        let orc_hp = arena.hp(2);
        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert!(arena.hp(2) < orc_hp);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 19);
        assert_eq!(arena.attr_str(2, "last_kind"), "physical");
        assert_eq!(arena.attr_f32(2, "last_attacker") as u32, 1);
        assert_eq!(arena.attr_f32(2, "last_source_item") as u32, 101);
    }

    #[test]
    fn official_ruleset_ranger_bow_requires_arrows() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Ranger",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Ranger", "Human", 1, Some(2));
        arena.add_official_entity(2, "Warrior", "Orc", 1, None);
        arena.equip_official_item(1, 101, "weapons", "hunting_bow");

        let orc_hp = arena.hp(2);
        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert_eq!(arena.hp(2), orc_hp);
        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 0);
        assert!(
            !arena
                .ctx
                .entity_state_data
                .get(&1)
                .is_some_and(|state| state.contains("intent: attack"))
        );
    }

    #[test]
    fn official_ruleset_bow_can_consume_multiple_arrows_from_stack() {
        let mut arena = HeadlessRulesArena::with_official_rules();
        arena.add_script_class(
            "Ranger",
            r#"
            fn event(event, value) {
                if event == "intent" && value == "attack" {
                    attack();
                }
            }
            "#,
        );
        arena.add_official_entity(1, "Ranger", "Human", 1, Some(2));
        arena.add_official_entity(2, "Warrior", "Orc", 1, None);
        arena.equip_official_item(1, 101, "weapons", "hunting_bow");
        arena.add_official_inventory_item(1, 201, "ammunition", "wooden_arrows");
        arena
            .ctx
            .map
            .entities
            .iter_mut()
            .find(|entity| entity.id == 1)
            .unwrap()
            .equipped
            .get_mut("main_hand")
            .unwrap()
            .set_attribute("ammunition_quantity", Value::Int(2));

        arena.run_entity_event(1, "intent", VMValue::from_string("attack"));
        arena.drain_entity_events();

        assert_eq!(arena.inventory_item_quantity(1, "wooden_arrows"), 18);
    }
}

pub fn run_server_named_fn(
    exec: &mut Execution,
    name: &str,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) -> bool {
    if let Some(index) = program.user_functions_name_map.get(name).copied() {
        exec.reset(program.globals);
        let previous_debug_function = region_ctx.current_debug_function.clone();
        region_ctx.current_debug_function = name.to_string();
        if region_ctx.debug_mode {
            let target = eldrin_debug_target_for_ctx(region_ctx);
            region_ctx.eldrin_debug.begin_invocation(target, name);
        }
        let mut host = RegionHost { ctx: region_ctx };
        let _ret = exec.execute_function_host(args, index, program, &mut host);
        region_ctx.current_debug_function = previous_debug_function;
        true
    } else {
        false
    }
}

// Run an event
pub fn run_server_fn(
    exec: &mut Execution,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) {
    let _ = run_server_named_fn(exec, "event", args, program, region_ctx);
}

// Run a user_event
pub fn run_client_fn(
    exec: &mut Execution,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) {
    if let Some(index) = program.user_functions_name_map.get("user_event").copied() {
        exec.reset(program.globals);
        let previous_debug_function = region_ctx.current_debug_function.clone();
        region_ctx.current_debug_function = "user_event".to_string();
        if region_ctx.debug_mode {
            let target = eldrin_debug_target_for_ctx(region_ctx);
            region_ctx
                .eldrin_debug
                .begin_invocation(target, "user_event");
        }
        let mut host = RegionHost { ctx: region_ctx };
        let _ret = exec.execute_function_host(args, index, program, &mut host);
        region_ctx.current_debug_function = previous_debug_function;
    }
}
