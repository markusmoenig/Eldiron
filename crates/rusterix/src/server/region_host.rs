use crate::server::message::RegionMessage;
use crate::server::region::add_debug_value;
use crate::vm::*;
use crate::{
    Choice, EntityAction, Item, MultipleChoice, PixelSource, PlayerCamera, RegionCtx, Value,
};
use rand::Rng;
use scenevm::GeoId;
use theframework::prelude::TheValue;
use vek::Vec2;

struct RegionHost<'a> {
    ctx: &'a mut RegionCtx,
}

fn opening_geo_for_item(item: &Item) -> Option<GeoId> {
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

impl<'a> HostHandler for RegionHost<'a> {
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
            "set_player_camera" => {
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if let Some(camera) = args.get(0).and_then(|v| v.as_string()) {
                        let player_camera = match camera {
                            "iso" => PlayerCamera::D3Iso,
                            "firstp" => PlayerCamera::D3FirstP,
                            _ => PlayerCamera::D2,
                        };
                        entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
                    }
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
                    self.ctx.curr_debug_loc = Some((event.to_string(), x, y));
                }
            }
            "set_tile" => {
                if let (Some(mode), Some(item_id)) = (
                    args.get(0).and_then(|v| v.as_string()),
                    self.ctx.curr_item_id,
                ) {
                    if let Ok(uuid) = theframework::prelude::Uuid::try_parse(mode) {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            item.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                        }
                    }
                }
            }
            "set_emit_light" => {
                let active = args.get(0).map(|v| v.is_truthy()).unwrap_or(false);
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
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            // Single conversion path with optional type hints (string tag or attr type).
                            let converted = val.to_value_with_hint(item.attributes.get(key));
                            item.set_attribute(key, converted);

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
                                if let Some(geo_id) = opening_geo_for_item(item) {
                                    let blocking =
                                        item.attributes.get_bool_default("blocking", false);
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
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        let converted = val.to_value_with_hint(entity.attributes.get(key));
                        entity.set_attribute(key, converted);
                    }
                }
            }
            "toggle_attr" => {
                if let Some(key) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        let mut push_active: Option<(u32, String, VMValue)> = None;
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            item.attributes.toggle(key);
                            if key == "active" {
                                if let Some(class_name) = item.attributes.get_str("class_name") {
                                    let value = VMValue::from_bool(
                                        item.attributes.get_bool_default("active", false),
                                    );
                                    push_active = Some((item.id, class_name.to_string(), value));
                                }
                            }
                        }
                        if let Some((id, _class_name, value)) = push_active {
                            self.ctx.to_execute_item.push((id, "active".into(), value));
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        entity.attributes.toggle(key);
                    }
                }
            }
            "id" => {
                return Some(VMValue::broadcast(self.ctx.curr_entity_id as f32));
            }
            "get_attr_of" => {
                if let (Some(id_val), Some(key)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let id = id_val.x as u32;
                    if let Some(entity) = self.ctx.get_entity_mut(id) {
                        if let Some(v) = entity.attributes.get(key).cloned() {
                            return Some(VMValue::from_value(&v));
                        }
                    } else if let Some(item) = self.ctx.get_item_mut(id) {
                        if let Some(v) = item.attributes.get(key).cloned() {
                            return Some(VMValue::from_value(&v));
                        }
                    }
                }
            }
            "get_attr" => {
                if let Some(key) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            if let Some(v) = item.attributes.get(key).cloned() {
                                return Some(VMValue::from_value(&v));
                            }
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        if let Some(v) = entity.attributes.get(key).cloned() {
                            return Some(VMValue::from_value(&v));
                        }
                    }
                }
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
                    return Some(VMValue::broadcast(r as f32));
                } else {
                    let r: f32 = rand::random();
                    return Some(VMValue::broadcast(r));
                }
            }
            "notify_in" => {
                if let (Some(mins), Some(notification)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let minutes = mins.x as i32;
                    let target_tick =
                        self.ctx.ticks + (self.ctx.ticks_per_minute as i32 * minutes) as i64;
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
            "set_proximity_tracking" => {
                let turn_on = args.get(0).map(|v| v.is_truthy()).unwrap_or(false);
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
                    return Some(v);
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
                        return Some(v);
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
                return Some(v);
            }
            "list_get" => {
                // list is arg0 (comma-separated string), index is arg1
                let idx = args.get(1).map(|v| v.x as i32).unwrap_or(0);
                if let Some(list_str) = args.get(0).and_then(|v| v.as_string()) {
                    let parts: Vec<&str> = list_str.split(',').filter(|s| !s.is_empty()).collect();
                    if parts.is_empty() {
                        return Some(VMValue::zero());
                    }
                    let clamped = if idx < 0 {
                        0
                    } else if (idx as usize) >= parts.len() {
                        parts.len() - 1
                    } else {
                        idx as usize
                    };
                    if let Ok(val) = parts[clamped].parse::<f32>() {
                        return Some(VMValue::broadcast(val));
                    }
                    return Some(VMValue::zero());
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
                    return Some(VMValue::broadcast(if exists { 1.0 } else { 0.0 }));
                }
            }
            "is_entity" => {
                if let Some(id) = args.get(0) {
                    let entity_id = id.x as u32;
                    let exists = self.ctx.map.entities.iter().any(|e| e.id == entity_id);
                    return Some(VMValue::broadcast(if exists { 1.0 } else { 0.0 }));
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
                            return Some(VMValue::broadcast(dist));
                        }
                    }
                    return Some(VMValue::zero());
                }
            }
            "deal_damage" => {
                if let (Some(target), Some(amount)) = (args.get(0), args.get(1)) {
                    let id = target.x as u32;
                    let dmg = amount.x as i32;
                    let subject_id = if self.ctx.curr_item_id.is_some() {
                        self.ctx.curr_item_id.unwrap()
                    } else {
                        self.ctx.curr_entity_id
                    };
                    self.ctx.to_execute_entity.push((
                        id,
                        "take_damage".into(),
                        VMValue::new(subject_id as f32, dmg as f32, 0.0),
                    ));
                }
            }
            "took_damage" => {
                if let (Some(from), Some(amount_val)) = (args.get(0), args.get(1)) {
                    let mut kill = false;

                    let from = from.x as u32;
                    // Make sure we don't heal by accident
                    let amount = amount_val.x.max(0.0) as i32;

                    if amount == 0 {
                        return None;
                    }

                    let id = self.ctx.curr_entity_id;
                    let health_attr = self.ctx.health_attr.clone();

                    let mut enqueue_death = false;

                    // Check for death
                    {
                        if let Some(entity) = self.ctx.get_entity_mut(id) {
                            if let Some(mut health) = entity.attributes.get_int(&health_attr) {
                                // Reduce the health of the target
                                health -= amount;
                                health = health.max(0);
                                // Set the new health
                                entity.set_attribute(&health_attr, Value::Int(health));

                                let mode = entity.attributes.get_str_default("mode", "".into());
                                if health <= 0 && mode != "dead" {
                                    enqueue_death = true;

                                    entity.set_attribute("mode", Value::Str("dead".into()));
                                    entity.action = EntityAction::Off;
                                    self.ctx.entity_proximity_alerts.remove(&id);

                                    kill = true;
                                }
                            }
                        }
                    }

                    if enqueue_death {
                        self.ctx
                            .to_execute_entity
                            .push((id, "death".into(), VMValue::zero()));
                    }

                    // if receiver got killed, send a "kill" event to the attacker
                    if kill {
                        self.ctx.to_execute_entity.push((
                            from,
                            "kill".into(),
                            VMValue::broadcast(id as f32),
                        ));
                    }
                }
            }
            "block_events" => {
                if let (Some(minutes), Some(event)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let target_tick =
                        self.ctx.ticks + (self.ctx.ticks_per_minute as f32 * minutes.x) as i64;
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
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
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

                        let mut choices = MultipleChoice::new(region_id, entity.id, to);
                        for item_id in matching_item_ids {
                            let choice = Choice::ItemToSell(item_id, entity.id, to);
                            choices.add(choice);
                        }

                        if let Some(sender) = self.ctx.from_sender.get() {
                            let _ = sender.send(RegionMessage::MultipleChoice(choices));
                        }
                    }
                }
            }
            "drop_items" => {
                if let Some(filter) = args.get(0).and_then(|v| v.as_string()) {
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

                        let mut removed_items = Vec::new();
                        for slot in matching_slots {
                            if let Some(mut item) = entity.remove_item_from_slot(slot) {
                                // Drop at the entity position and mark dirty so the server transmits
                                item.position = entity.position;
                                item.mark_all_dirty();
                                removed_items.push(item);
                            }
                        }
                        self.ctx.map.items.extend(removed_items);
                    }
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

                    if region_name.is_empty() {
                        // Teleport entity in this region to the given sector.
                        let center = {
                            let map = &self.ctx.map;
                            map.sectors
                                .iter()
                                .find(|s| s.name == dest)
                                .and_then(|s| s.center(map))
                        };

                        if let Some(center) = center {
                            // First move the entity
                            if let Some(entity) = self.ctx.get_current_entity_mut() {
                                let id = entity.id;
                                entity.set_pos_xz(center);
                                // Then run section change checks using a fresh borrow
                                self.ctx.check_player_for_section_change_id(id);
                            }
                        } else if self.ctx.debug_mode {
                            add_debug_value(
                                &mut self.ctx,
                                TheValue::Text("Unknown Sector".into()),
                                true,
                            );
                        }
                    } else {
                        // Remove the entity from this region and send it to another region.
                        let entity_id = self.ctx.curr_entity_id;
                        if let Some(pos) =
                            self.ctx.map.entities.iter().position(|e| e.id == entity_id)
                        {
                            let removed = self.ctx.map.entities.remove(pos);
                            self.ctx.entity_classes.remove(&removed.id);

                            if let Some(sender) = self.ctx.from_sender.get() {
                                let _ = sender.send(RegionMessage::TransferEntity(
                                    self.ctx.region_id,
                                    removed,
                                    region_name.to_string(),
                                    dest.to_string(),
                                ));
                            }
                        }
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
                    let mut coord: Option<vek::Vec2<f32>> = None;
                    for sector in &self.ctx.map.sectors {
                        if sector.name == dest {
                            coord = sector.center(&self.ctx.map);
                        }
                    }

                    if let Some(coord) = coord {
                        if let Some(entity) = self.ctx.get_current_entity_mut() {
                            entity.action = EntityAction::Goto(coord, speed);
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

// Run an event
pub fn run_server_fn(
    exec: &mut Execution,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) {
    if let Some(index) = program.user_functions_name_map.get("event").copied() {
        exec.reset(program.globals);
        let mut host = RegionHost { ctx: region_ctx };
        let _ret = exec.execute_function_host(args, index, program, &mut host);
    }
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
        let mut host = RegionHost { ctx: region_ctx };
        let _ret = exec.execute_function_host(args, index, program, &mut host);
    }
}
