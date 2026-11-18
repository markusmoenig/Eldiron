//! Authoritative source operations shared by local, remote and NPC callers.
use super::crafting::item_number;
use super::*;
use eldiron_ruleset::world_resources::{
    self as rules, Extraction, ResourceBinding, ResourceProfile, ResourceState, StockUnit,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const CLOCK: &str = "__world_resource_clock_ms";
const PREFIX: &str = "__world_resource/";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    pub access: Uuid,
    pub interaction: String,
    pub quantity: u32,
    pub tool: Option<u32>,
    pub vessel: Option<u32>,
}
impl ResourceRequest {
    pub fn bounded(&self) -> bool {
        !self.access.is_nil()
            && !self.interaction.is_empty()
            && self.interaction.len() <= 128
            && !self.interaction.chars().any(char::is_control)
            && self.quantity > 0
            && self.quantity <= 1_000_000
            && self.tool != Some(0)
            && self.vessel != Some(0)
    }
}
#[derive(Clone, Debug)]
pub struct Access {
    pub id: Uuid,
    pub source: String,
    pub binding: ResourceBinding,
    pub profile: ResourceProfile,
    pub min: Vec2<f32>,
    pub max: Vec2<f32>,
    pub item: Option<u32>,
    pub min_y: f32,
    pub max_y: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceView {
    pub access: Uuid,
    pub source: String,
    pub profile: String,
    pub phase: String,
    pub appearance: String,
    pub supplies: BTreeMap<String, Option<u32>>,
    pub next_recovery_ms: Option<u64>,
    pub revision: u64,
    pub tile: Option<Uuid>,
}
fn now(state: &ValueContainer) -> Result<u64, String> {
    match state.get(CLOCK) {
        None => Ok(0),
        Some(Value::Int64(n)) if *n >= 0 => Ok(*n as u64),
        _ => Err("Invalid saved resource clock; explicit recovery is required".into()),
    }
}

fn key(source: &str) -> String {
    format!("{PREFIX}{source}")
}
fn read(state: &ValueContainer, access: &Access) -> Result<ResourceState, String> {
    let text = state
        .get_str(&key(&access.source))
        .ok_or("Resource state is missing; explicit initialization or migration is required")?;
    let mut resource: ResourceState =
        serde_json::from_str(text).map_err(|e| format!("Invalid saved resource state: {e}"))?;
    resource.validate()?;
    resource.reconcile_policy(&access.profile)?;
    resource.policy = access.profile.clone();
    Ok(resource)
}
fn write(
    state: &mut ValueContainer,
    access: &Access,
    resource: &ResourceState,
) -> Result<(), String> {
    state.set(
        &key(&access.source),
        Value::Str(serde_json::to_string(resource).map_err(|e| e.to_string())?),
    );
    Ok(())
}
fn add_access(
    result: &mut BTreeMap<Uuid, Access>,
    map: &Map,
    root: &toml::Table,
    id: Uuid,
    data: &str,
    min: Vec2<f32>,
    max: Vec2<f32>,
    item: Option<u32>,
) -> Result<(), String> {
    let Some(binding) = rules::binding(data)? else {
        return Ok(());
    };
    if id.is_nil()
        || !min.x.is_finite()
        || !min.y.is_finite()
        || !max.x.is_finite()
        || !max.y.is_finite()
    {
        return Err("Invalid resource access identity or bounds".into());
    }
    let profile = rules::resolve(root, &binding)?;
    let source = if let Some(pool) = &binding.pool {
        format!("pool:{pool}")
    } else if let Some(instance) = &binding.instance {
        format!("instance:{instance}")
    } else {
        format!("placement:{}:{id}", map.id)
    };
    let (min_y, max_y) = if let Some(object) = map.geometry_objects.iter().find(|o| o.id == id) {
        object
            .vertices
            .iter()
            .map(|v| object.transform_point(*v).y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), y| {
                (lo.min(y), hi.max(y))
            })
    } else if let Some(item) = item.and_then(|id| map.items.iter().find(|i| i.id == id)) {
        (item.position.y, item.position.y)
    } else {
        let y = map
            .sectors
            .iter()
            .find(|s| s.creator_id == id)
            .map(|s| s.properties.get_float_default("floor_height", 0.0))
            .unwrap_or(0.0);
        (y, y)
    };
    if !min_y.is_finite() || !max_y.is_finite() {
        return Err("Resource height must be finite".into());
    }
    let access = Access {
        id,
        source,
        binding,
        profile,
        min,
        max,
        item,
        min_y,
        max_y,
    };
    if result.insert(id, access).is_some() {
        return Err("Duplicate resource access identity".into());
    }
    Ok(())
}
pub fn accesses(map: &Map, root: &toml::Table) -> Result<BTreeMap<Uuid, Access>, String> {
    let mut result = BTreeMap::new();
    for object in &map.geometry_objects {
        if let Some(data) = object.properties.get_str("data") {
            if let Some(bounds) = object.bbox() {
                add_access(
                    &mut result,
                    map,
                    root,
                    object.id,
                    data,
                    bounds.min,
                    bounds.max,
                    None,
                )?;
            } else if rules::binding(data)?.is_some() {
                return Err("A resource object must have finite geometry bounds".into());
            }
        }
    }
    for sector in &map.sectors {
        if let Some(data) = sector.properties.get_str("data") {
            if rules::binding(data)?.is_some() {
                let bounds = sector.bounding_box(map);
                sector
                    .center(map)
                    .ok_or("A resource tile/sector must have a position")?;
                add_access(
                    &mut result,
                    map,
                    root,
                    sector.creator_id,
                    data,
                    bounds.min,
                    bounds.max,
                    None,
                )?;
            }
        }
    }
    for line in &map.linedefs {
        if let Some(data) = line.properties.get_str("data") {
            if rules::binding(data)?.is_some() {
                let vertices = map
                    .vertices
                    .iter()
                    .filter(|v| v.id == line.start_vertex || v.id == line.end_vertex)
                    .map(|v| v.as_vec2())
                    .collect::<Vec<_>>();
                if vertices.len() != 2 {
                    return Err("A resource edge must have two valid vertices".into());
                }
                let min = Vec2::new(
                    vertices[0].x.min(vertices[1].x),
                    vertices[0].y.min(vertices[1].y),
                );
                let max = Vec2::new(
                    vertices[0].x.max(vertices[1].x),
                    vertices[0].y.max(vertices[1].y),
                );
                add_access(
                    &mut result,
                    map,
                    root,
                    line.creator_id,
                    data,
                    min,
                    max,
                    None,
                )?;
            }
        }
    }
    for item in &map.items {
        if let Some(data) = item.attributes.get_str("resource_binding") {
            let id = item
                .attributes
                .get_str("resource_access_id")
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or("Resource item has no persistent access UUID")?;
            let point = item.get_pos_xz();
            add_access(
                &mut result,
                map,
                root,
                id,
                data,
                point,
                point,
                Some(item.id),
            )?;
        }
    }
    Ok(result)
}
pub fn initialize(ctx: &mut RegionCtx, restoring: bool) -> Result<(), String> {
    ctx.resource_access = accesses(&ctx.map, &ctx.rules)?;
    if restoring {
        return Ok(());
    }
    let world = ctx.world.clone();
    world.with_state(|state| {
        for access in ctx.resource_access.values() {
            if state.get(&key(&access.source)).is_some() {
                read(state, access)?;
            } else {
                let resource = ResourceState::new(access.profile.clone(), now(state)?)?;
                write(state, access, &resource)?;
            }
        }
        Ok(())
    })
}
pub fn views(ctx: &RegionCtx) -> Result<Vec<ResourceView>, String> {
    let snapshot = ctx.world.snapshot();
    let live_items: std::collections::BTreeSet<_> =
        ctx.map.items.iter().map(|item| item.id).collect();
    ctx.resource_access
        .values()
        .filter(|access| access.item.is_none_or(|item| live_items.contains(&item)))
        .map(|access| {
            let mut resource = read(&snapshot, access)?;
            resource.settle(now(&snapshot)?)?;
            let appearance = resource.appearance(
                &access.profile.interactions[&access.profile.default_interaction].supply,
            );
            let tile = access
                .binding
                .visual_tiles
                .get(&resource.phase)
                .or_else(|| access.binding.visual_tiles.get(&appearance))
                .or_else(|| access.binding.visual_tiles.get("healthy"))
                .and_then(|id| Uuid::parse_str(id).ok());
            Ok(ResourceView {
                tile,
                access: access.id,
                source: access.source.clone(),
                profile: access.binding.profile.clone(),
                phase: resource.phase.clone(),
                appearance: resource.appearance(
                    &access.profile.interactions[&access.profile.default_interaction].supply,
                ),
                supplies: resource
                    .supplies
                    .iter()
                    .map(|(k, v)| (k.clone(), v.remaining))
                    .collect(),
                next_recovery_ms: resource
                    .supplies
                    .values()
                    .filter_map(|s| s.next_recovery_ms)
                    .min(),
                revision: resource.revision,
            })
        })
        .collect()
}
pub fn apply_views(map: &mut Map, views: &[ResourceView]) {
    let mut visual_changed = false;
    for view in views {
        let apply = |properties: &mut ValueContainer| {
            properties.set("resource_phase", Value::Str(view.phase.clone()));
            properties.set("resource_appearance", Value::Str(view.appearance.clone()));
            properties.set("resource_source", Value::Str(view.source.clone()));
            properties.set(
                "resource_supplies",
                Value::Str(serde_json::to_string(&view.supplies).unwrap_or_default()),
            );
        };
        for object in &mut map.geometry_objects {
            if object.id == view.access {
                apply(&mut object.properties);
                if let Some(tile) = view.tile {
                    for face in &mut object.faces {
                        let source = Some(PixelSource::TileId(tile));
                        if face.tile != source {
                            face.tile = source;
                            visual_changed = true;
                        }
                    }
                }
            }
        }
        for sector in &mut map.sectors {
            if sector.creator_id == view.access {
                if view.tile.is_some()
                    && sector.properties.get_str("resource_appearance") != Some(&view.appearance)
                {
                    visual_changed = true;
                }
                apply(&mut sector.properties);
                if let Some(tile) = view.tile {
                    sector
                        .properties
                        .set("source", Value::Source(PixelSource::TileId(tile)));
                    sector
                        .properties
                        .set("floor_source", Value::Source(PixelSource::TileId(tile)));
                }
            }
        }
        for line in &mut map.linedefs {
            if line.creator_id == view.access {
                if view.tile.is_some()
                    && line.properties.get_str("resource_appearance") != Some(&view.appearance)
                {
                    visual_changed = true;
                }
                apply(&mut line.properties);
                if let Some(tile) = view.tile {
                    line.properties
                        .set("source", Value::Source(PixelSource::TileId(tile)));
                    line.properties
                        .set("wall_source", Value::Source(PixelSource::TileId(tile)));
                }
            }
        }
        for item in &mut map.items {
            if item.attributes.get_str("resource_access_id")
                == Some(view.access.to_string().as_str())
            {
                apply(&mut item.attributes);
            }
        }
    }
    if visual_changed {
        map.changed = map.changed.wrapping_add(1);
    }
}
fn distance(access: &Access, point: Vec2<f32>) -> f32 {
    point.distance(Vec2::new(
        point.x.clamp(access.min.x, access.max.x),
        point.y.clamp(access.min.y, access.max.y),
    ))
}
pub fn validate_request(
    ctx: &RegionCtx,
    actor: u32,
    request: &ResourceRequest,
) -> Result<(), String> {
    if !request.bounded() {
        return Err("Invalid resource request".into());
    }
    let access = ctx
        .resource_access
        .get(&request.access)
        .ok_or("Unknown resource access point")?;
    let interaction = access
        .profile
        .interactions
        .get(&request.interaction)
        .ok_or("Unknown resource interaction")?;
    let actor = ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == actor)
        .ok_or("Actor is unavailable")?;
    if ctx.paused || access.binding.disabled || actor.get_mode() == "dead" {
        return Err("Resource access is disabled".into());
    }
    if let Some(item) = access.item {
        if !ctx
            .map
            .items
            .iter()
            .any(|i| i.id == item && i.attributes.get_bool_default("visible", true))
        {
            return Err("Remains are no longer accessible".into());
        }
    }
    let height_distance = actor.position.y - actor.position.y.clamp(access.min_y, access.max_y);
    if !actor.position.y.is_finite()
        || !actor.position.x.is_finite()
        || !actor.position.z.is_finite()
        || distance(access, actor.get_pos_xz()).hypot(height_distance) > interaction.range
    {
        return Err("Resource access is out of reach".into());
    }
    Ok(())
}
fn owned_item(actor: &Entity, id: u32) -> Option<&Item> {
    actor
        .get_item(id)
        .or_else(|| actor.equipped.values().find(|item| item.id == id))
}
fn owned_item_mut(actor: &mut Entity, id: u32) -> Option<&mut Item> {
    if actor.get_item_slot(id).is_some() {
        actor.get_item_mut(id)
    } else {
        actor.equipped.values_mut().find(|item| item.id == id)
    }
}
pub fn execute(
    ctx: &mut RegionCtx,
    actor_id: u32,
    request: &ResourceRequest,
) -> Result<Extraction, String> {
    validate_request(ctx, actor_id, request)?;
    let access = ctx
        .resource_access
        .get(&request.access)
        .ok_or("Unknown resource access point")?
        .clone();
    let interaction = access.profile.interactions[&request.interaction].clone();
    let actor_index = ctx
        .map
        .entities
        .iter()
        .position(|e| e.id == actor_id)
        .ok_or("Actor is unavailable")?;
    let world = ctx.world.clone();
    let result = world.with_state(|state| {
        let at = now(state)?;
        let mut resource = read(state, &access)?;
        resource.settle(at)?;
        // Elapsed recovery/expiry is independent of action eligibility.
        write(state, &access, &resource)?;
        let mut actor = ctx.map.entities[actor_index].clone();
        if let Some(skill) = &interaction.skill {
            if entity_skill_points(&actor, skill) < interaction.minimum_skill as i32 {
                return Err("Insufficient resource skill".into());
            }
        }
        if let Some(capability) = &interaction.tool {
            let tool = request
                .tool
                .and_then(|id| owned_item(&actor, id))
                .ok_or("Select an owned usable tool")?;
            let capabilities = match tool.attributes.get("tool_capabilities") {
                Some(Value::StrArray(values)) => values.as_slice(),
                _ => &[],
            };
            if !capabilities.contains(capability)
                || item_number(tool, "condition", 100)
                    < interaction.tool_wear.max(1) as i32
            {
                return Err("Selected tool lacks the capability or usable condition".into());
            }
        } else if request.tool.is_some() {
            return Err("This interaction does not use a tool".into());
        }
        let supply = &access.profile.supplies[&interaction.supply];
        let limit = if supply.unit == StockUnit::Volume {
            let vessel = request
                .vessel
                .and_then(|id| owned_item(&actor, id))
                .ok_or("Select an owned liquid vessel")?;
            let capacity = item_number(vessel, "liquid_capacity", 0);
            let current = item_number(vessel, "liquid_quantity", 0);
            if capacity <= 0 || current < 0 || current > capacity {
                return Err("Invalid vessel contents or capacity".into());
            }
            if current > 0
                && vessel.attributes.get_str("liquid_item") != Some(&supply.outputs[0].item)
            {
                return Err("Vessel contains an incompatible liquid".into());
            }
            (capacity - current) as u32
        } else {
            if request.vessel.is_some() {
                return Err("Solid collection does not use a vessel".into());
            }
            u32::MAX
        };
        let outcome = resource.extract(&request.interaction, request.quantity, limit, at)?;
        if supply.unit == StockUnit::Volume {
            let vessel = owned_item_mut(&mut actor, request.vessel.unwrap())
                .ok_or("Vessel is no longer owned")?;
            let quantity = item_number(vessel, "liquid_quantity", 0)
                + outcome.stock_units as i32;
            vessel.set_attribute("liquid_item", Value::Str(supply.outputs[0].item.clone()));
            vessel.set_attribute("liquid_quantity", Value::Int(quantity));
        } else {
            for output in &outcome.outputs {
                let mut item=ruleset_item_from_table(&ctx.rules,&output.item,1,&world).ok_or("Resource output definition is missing")?;
                if item.is_stackable() {
                    item.set_stack_quantity(i32::try_from(output.quantity).map_err(|_|"Resource stack quantity exceeds inventory representation")?);
                    actor.add_item(item).map_err(|_|"The complete output bundle does not fit")?;
                } else {
                    actor.add_item(item).map_err(|_|"The complete output bundle does not fit")?;
                    for _ in 1..output.quantity {
                        let item=ruleset_item_from_table(&ctx.rules,&output.item,1,&world).ok_or("Resource output definition is missing")?;
                        actor.add_item(item).map_err(|_|"The complete output bundle does not fit")?;
                    }
                }
            }
        }
        if let Some(tool) = request.tool {
            let tool = owned_item_mut(&mut actor, tool).ok_or("Tool is no longer owned")?;
            let condition =
                item_number(tool, "condition", 100) - interaction.tool_wear as i32;
            tool.set_attribute("condition", Value::Int(condition));
        }
        if let Some(skill) = &interaction.skill {
            let current = entity_skill_points(&actor, skill);
            let cap = ctx
                .rules
                .get("skills")
                .and_then(|v| v.get(skill))
                .map(|v| {
                    v.get("max")
                        .and_then(toml::Value::as_integer)
                        .unwrap_or(100)
                })
                .unwrap_or(100)
                .max(0) as i32;
            if current < cap && interaction.skill_gain > 0 {
                actor.set_attribute(
                    &format!("skill_{}", skill.to_ascii_lowercase()),
                    Value::Int(
                        current
                            .saturating_add(interaction.skill_gain as i32)
                            .min(cap),
                    ),
                );
            }
        }
        actor.set_attribute(
            "last_resource_result",
            Value::Str(serde_json::to_string(&serde_json::json!({"access":request.access,"source":access.source,"interaction":request.interaction,"tool":request.tool,"vessel":request.vessel,"extraction":outcome})).map_err(|e|e.to_string())?),
        );
        write(state, &access, &resource)?;
        ctx.map.entities[actor_index] = actor;
        Ok(outcome)
    });
    result
}
/// Ordinary Use on a bound map feature selects its declared default operation.
/// Explicit requests retain exact tool/vessel identities through the wire path.
pub fn click(ctx: &mut RegionCtx, actor_id: u32, point: Vec2<f32>, intent: &str) -> bool {
    if !matches!(intent, "use" | "gather") {
        return false;
    }
    let Some(access) = ctx
        .resource_access
        .values()
        .find(|a| {
            point.x >= a.min.x - 0.1
                && point.x <= a.max.x + 0.1
                && point.y >= a.min.y - 0.1
                && point.y <= a.max.y + 0.1
        })
        .cloned()
    else {
        return false;
    };
    let Some(actor) = ctx.map.entities.iter().find(|e| e.id == actor_id) else {
        return true;
    };
    let request = default_request(&access, actor);
    report(ctx, actor_id, &request);
    true
}
fn default_request(access: &Access, actor: &Entity) -> ResourceRequest {
    let interaction_id = access.binding.interaction.as_ref().unwrap_or(&access.profile.default_interaction);
    let interaction = &access.profile.interactions[interaction_id];
    let tool=interaction.tool.as_ref().and_then(|cap|actor.iter_inventory().map(|(_,i)|i).chain(actor.equipped.values()).find(|i|matches!(i.attributes.get("tool_capabilities"),Some(Value::StrArray(caps)) if caps.contains(cap)) && item_number(i, "condition", 100)>=interaction.tool_wear.max(1) as i32).map(|i|i.id));
    let vessel = if access.profile.supplies[&interaction.supply].unit == StockUnit::Volume {
        actor
            .iter_inventory()
            .map(|(_, i)| i)
            .chain(actor.equipped.values())
            .find(|i| {
                item_number(i, "liquid_capacity", 0)
                    > item_number(i, "liquid_quantity", 0)
                    && (item_number(i, "liquid_quantity", 0) == 0
                        || i.attributes.get_str("liquid_item")
                            == Some(&access.profile.supplies[&interaction.supply].outputs[0].item))
            })
            .map(|i| i.id)
    } else {
        None
    };
    ResourceRequest {
        access: access.id,
        interaction: interaction_id.clone(),
        quantity: interaction.quantity,
        tool,
        vessel,
    }
}
pub fn report(ctx: &mut RegionCtx, actor: u32, request: &ResourceRequest) {
    match execute(ctx, actor, request) {
        Ok(outcome) => send_message(
            ctx,
            actor,
            format!(
                "Collected {} from {}. State: {}.",
                outcome
                    .outputs
                    .iter()
                    .map(|o| format!("{} {}", o.quantity, o.item.replace('_', " ")))
                    .collect::<Vec<_>>()
                    .join(", "),
                ctx.resource_access[&request.access].profile.name,
                outcome.appearance
            ),
            "success",
        ),
        Err(error) => send_message(ctx, actor, error, "warning"),
    }
}

/// One persistent creation marker per creature incarnation. Removing artwork
/// or replaying a death callback never creates a second source.
pub fn create_remains(ctx: &mut RegionCtx, entity_id: u32) -> Result<Option<Uuid>, String> {
    let Some(entity) = ctx.find_entity(entity_id) else {
        return Ok(None);
    };
    if entity.get_mode() != "dead" {
        return Ok(None);
    }
    let Some(profile) = entity
        .attributes
        .get_str("remains_profile")
        .map(str::to_owned)
    else {
        return Ok(None);
    };
    if let Some(id) = entity.attributes.get_str("__resource_death_id") {
        return Uuid::parse_str(id).map(Some).map_err(|e| e.to_string());
    }
    let position = entity.position;
    let id = Uuid::new_v4();
    let binding = ResourceBinding {
        profile,
        instance: Some(id.to_string()),
        ..Default::default()
    };
    let policy = rules::resolve(&ctx.rules, &binding)?;
    if !policy.spent_when_empty
        || policy
            .supplies
            .values()
            .any(|s| s.recovery.is_some() || s.capacity.is_none())
    {
        return Err("Remains require a finite, single-use profile without recovery".into());
    }
    let source = format!("instance:{id}");
    let existing_corpse = ctx
        .map
        .items
        .iter()
        .position(|item| item.attributes.get_uint("corpse_entity_id") == Some(entity_id));
    let position = existing_corpse
        .map(|index| ctx.map.items[index].position)
        .unwrap_or(position);
    let item_id = existing_corpse
        .map(|index| ctx.map.items[index].id)
        .unwrap_or_else(|| ctx.world.next_id());
    let access = Access {
        id,
        source,
        binding: binding.clone(),
        profile: policy,
        min: Vec2::new(position.x, position.z),
        max: Vec2::new(position.x, position.z),
        item: Some(item_id),
        min_y: position.y,
        max_y: position.y,
    };
    let world = ctx.world.clone();
    world.with_state(|state| {
        let resource = ResourceState::new(access.profile.clone(), now(state)?)?;
        if state.get(&key(&access.source)).is_some() {
            return Err("Death source identity collision".into());
        }
        write(state, &access, &resource)
    })?;
    let mut item = existing_corpse
        .map(|index| ctx.map.items[index].clone())
        .unwrap_or_else(Item::new);
    item.id = item_id;
    item.position = position;
    if existing_corpse.is_none() {
        item.creator_id = id;
    }
    if existing_corpse.is_none() {
        item.item_type = access.profile.name.clone();
    }
    for (key, value) in [
        ("name", Value::Str(access.profile.name.clone())),
        (
            "description",
            Value::Str("Remains with a finite supply. Use to collect.".into()),
        ),
        ("resource_access_id", Value::Str(id.to_string())),
        (
            "resource_binding",
            Value::Str(
                toml::to_string(&BTreeMap::from([("resource", binding)]))
                    .map_err(|e| e.to_string())?,
            ),
        ),
        ("takeable", Value::Bool(false)),
        ("static", Value::Bool(true)),
        ("visible", Value::Bool(true)),
        ("resource_node", Value::Bool(true)),
    ] {
        if existing_corpse.is_some()
            && !matches!(
                key,
                "resource_access_id" | "resource_binding" | "resource_node"
            )
        {
            continue;
        }
        item.set_attribute(key, value);
    }
    item.set_attribute("corpse_entity_id", Value::UInt(entity_id));
    if let Some(index) = existing_corpse {
        ctx.map.items[index] = item;
    } else {
        ctx.map.items.push(item);
    }
    ctx.resource_access.insert(id, access);
    if let Some(entity) = ctx.map.entities.iter_mut().find(|e| e.id == entity_id) {
        entity.set_attribute("__resource_death_id", Value::Str(id.to_string()));
    } else if let Some(entity) = ctx.stashed_entities.get_mut(&entity_id) {
        entity.set_attribute("__resource_death_id", Value::Str(id.to_string()));
    }
    Ok(Some(id))
}
pub fn item_click(ctx: &mut RegionCtx, actor: u32, item: u32, intent: &str) -> bool {
    let Some(access) = ctx
        .resource_access
        .values()
        .find(|a| a.item == Some(item))
        .cloned()
    else {
        return false;
    };
    if !matches!(intent, "use" | "gather") {
        return false;
    }
    let Some(owner) = ctx.map.entities.iter().find(|e| e.id == actor) else {
        return true;
    };
    let request = default_request(&access, owner);
    report(ctx, actor, &request);
    true
}
