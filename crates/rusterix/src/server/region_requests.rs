//! Explicit remote gameplay vocabulary. No executable engine action, actor ID,
//! event name, price, range, speed or clock is accepted from the requester.
use super::*;
use serde::{Deserialize, Serialize};
use theframework_core::Uuid;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemOwner {
    Owned,
    World,
    Corpse(u32),
    Follower(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemAddress {
    pub item: u32,
    pub owner: ItemOwner,
}
impl ItemAddress {
    pub fn owner_id(self, actor: u32) -> Option<u32> {
        match self.owner {
            ItemOwner::Owned => Some(actor),
            ItemOwner::World => None,
            ItemOwner::Corpse(id) => Some(id),
            ItemOwner::Follower(id) => Some(id),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemSource {
    Direct(ItemAddress),
    Container { container: ItemAddress, item: u32 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemDestination {
    Inventory(Option<u32>),
    Equipment(String),
    Container(ItemAddress),
    Ground([f32; 2]),
    FollowerInventory { entity: u32, index: Option<u32> },
    FollowerEquipment { entity: u32, slot: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum GameRequest {
    Intent(String),
    Camera(PlayerCamera),
    InspectSelf(SelfInspection),
    Craft {
        recipe: String,
    },
    Entity {
        target: u32,
        intent: Option<String>,
    },
    Item {
        target: ItemAddress,
        intent: Option<String>,
    },
    Prop {
        instance: Uuid,
        part: Uuid,
        target: Option<Uuid>,
        intent: String,
        explicit: bool,
    },
    OpenContainer(ItemAddress),
    Terrain {
        position: [f32; 2],
        intent: String,
    },
    Transfer {
        source: ItemSource,
        destination: ItemDestination,
    },
    Choose {
        offer: Uuid,
    },
    CommandSlot {
        slot: String,
        command: Option<String>,
    },
    Resource(resources::ResourceRequest),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfInspection {
    Inventory,
    Stats,
    Look,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestError {
    Invalid,
    NotControlled,
    StaleContext,
    NotFound,
    Forbidden,
    OutOfRange,
    ExpiredOffer,
    Unsupported,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestOutcome {
    Processed,
    Denied(RequestError),
}
fn identifier(text: &str, max: usize) -> bool {
    text.len() <= max && !text.chars().any(char::is_control)
}
impl GameRequest {
    pub fn requires_extensions(&self) -> bool {
        let address = |a: &ItemAddress| matches!(a.owner, ItemOwner::Follower(_));
        match self {
            Self::Camera(_) | Self::InspectSelf(_) | Self::Craft { .. } => true,
            Self::Terrain { intent, .. } => intent.is_empty(),
            Self::Item { target, .. } | Self::OpenContainer(target) => address(target),
            Self::Transfer {
                source,
                destination,
            } => {
                let source = match source {
                    ItemSource::Direct(a) => address(a),
                    ItemSource::Container { container, .. } => address(container),
                };
                source
                    || match destination {
                        ItemDestination::FollowerInventory { .. }
                        | ItemDestination::FollowerEquipment { .. } => true,
                        ItemDestination::Container(a) => address(a),
                        _ => false,
                    }
            }
            _ => false,
        }
    }
    /// Decoder and typed local callers share the same cheap structural bounds.
    pub fn bounded(&self) -> bool {
        let address = |a: &ItemAddress| {
            a.item > 0 && !matches!(a.owner, ItemOwner::Corpse(0) | ItemOwner::Follower(0))
        };
        let intent = |s: &str| identifier(s, 256);
        match self {
            Self::Camera(_) | Self::InspectSelf(_) => true,
            Self::Craft { recipe } => {
                !recipe.is_empty()
                    && recipe.len() <= 128
                    && recipe
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            }
            Self::Intent(s) => intent(s),
            Self::Resource(request) => request.bounded(),
            Self::Entity { target, intent: i } => *target > 0 && i.as_deref().is_none_or(intent),
            Self::Item { target, intent: i } => address(target) && i.as_deref().is_none_or(intent),
            Self::Prop {
                instance,
                part,
                target,
                intent: i,
                ..
            } => {
                !instance.is_nil()
                    && !part.is_nil()
                    && target.is_none_or(|id| !id.is_nil())
                    && intent(i)
            }
            Self::OpenContainer(a) => address(a),
            Self::Terrain {
                position,
                intent: i,
            } => {
                position
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                    && intent(i)
            }
            Self::Transfer {
                source,
                destination,
            } => {
                let source = match source {
                    ItemSource::Direct(a) => address(a),
                    ItemSource::Container { container, item } => address(container) && *item > 0,
                };
                source
                    && match destination {
                        ItemDestination::Inventory(index) => index.is_none_or(|i| i < 4096),
                        ItemDestination::FollowerInventory { entity, index } => {
                            *entity > 0 && index.is_none_or(|i| i < 4096)
                        }
                        ItemDestination::FollowerEquipment { entity, slot } => {
                            *entity > 0 && !slot.is_empty() && identifier(slot, 64)
                        }
                        ItemDestination::Equipment(slot) => {
                            !slot.is_empty() && identifier(slot, 64)
                        }
                        ItemDestination::Container(a) => address(a),
                        ItemDestination::Ground(p) => {
                            p.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                        }
                    }
            }
            Self::Choose { offer } => !offer.is_nil(),
            Self::CommandSlot { slot, command } => {
                identifier(slot, 64) && command.as_deref().is_none_or(|c| identifier(c, 256))
            }
        }
    }
}
fn follower(ctx: &RegionCtx, actor: u32, owner: u32) -> Result<&Entity, RequestError> {
    ctx.map
        .entities
        .iter()
        .find(|e| {
            e.id == owner
                && !e.is_player()
                && e.get_mode() != "dead"
                && e.attributes.get_bool_default("party_member", false)
                && e.attributes.get_uint("party_leader_id") == Some(actor)
        })
        .ok_or(RequestError::Forbidden)
}
pub(super) fn remote_inventory_owner_allowed(
    ctx: &RegionCtx,
    actor: u32,
    owner: Option<u32>,
    container: bool,
) -> bool {
    if !ctx
        .map
        .entities
        .iter()
        .find(|e| e.id == actor)
        .is_some_and(|e| e.attributes.get_bool_default("__frame_motion", false))
    {
        return true;
    }
    owner.is_none_or(|id| {
        id == actor
            || follower(ctx, actor, id).is_ok()
            || container
                && ctx
                    .map
                    .entities
                    .iter()
                    .any(|e| e.id == id && e.get_mode() == "dead")
    })
}
fn allowed_intent(ctx: &mut RegionCtx, actor: u32, intent: &str) -> Result<String, RequestError> {
    if !identifier(intent, 256) {
        return Err(RequestError::Invalid);
    }
    let resolved =
        resolve_ruleset_invocation_intent(ctx, actor, intent).ok_or(RequestError::Unsupported)?;
    if let Some(id) = resolved.strip_prefix("action:") {
        if resolved_ruleset_action(ctx, id.trim())
            .ok()
            .flatten()
            .is_none()
        {
            return Err(RequestError::Unsupported);
        }
    } else if resolved.starts_with("spell:") {
        return Err(RequestError::Unsupported);
    } else if !matches!(
        resolved.as_str(),
        "" | "look"
            | "talk"
            | "attack"
            | "take"
            | "drop"
            | "use"
            | "open"
            | "close"
            | "open_close"
            | "equip"
            | "unequip"
            | "buy"
            | "sell"
    ) && ctx
        .rules
        .get("intents")
        .and_then(|v| v.get(&resolved))
        .is_none()
        && ctx
            .entity_classes
            .get(&actor)
            .and_then(|class| ctx.entity_class_data.get(class))
            .and_then(|data| intent_rule_config_from_data(data, &resolved))
            .is_none()
        && resolved_ruleset_action_for_intent(ctx, &resolved).is_none()
    {
        return Err(RequestError::Unsupported);
    }
    Ok(resolved)
}
fn read_item<'a>(
    ctx: &'a RegionCtx,
    actor: u32,
    address: ItemAddress,
) -> Result<&'a Item, RequestError> {
    match address.owner {
        ItemOwner::Owned => ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == actor)
            .and_then(|e| {
                e.get_item(address.item)
                    .or_else(|| e.equipped.values().find(|i| i.id == address.item))
            }),
        ItemOwner::World => ctx
            .map
            .items
            .iter()
            .find(|i| i.id == address.item && i.attributes.get_bool_default("visible", true)),
        ItemOwner::Follower(owner) => {
            follower(ctx, actor, owner)?
                .get_item(address.item)
                .or_else(|| {
                    follower(ctx, actor, owner)
                        .ok()?
                        .equipped
                        .values()
                        .find(|i| i.id == address.item)
                })
        }
        ItemOwner::Corpse(owner) => {
            if !ctx
                .map
                .entities
                .iter()
                .any(|e| e.id == owner && e.get_mode() == "dead")
            {
                return Err(RequestError::Forbidden);
            }
            ctx.map
                .entities
                .iter()
                .find(|e| e.id == owner)
                .and_then(|e| {
                    e.get_item(address.item)
                        .or_else(|| e.equipped.values().find(|i| i.id == address.item))
                })
        }
    }
    .ok_or(RequestError::NotFound)
}
fn container_access(
    ctx: &RegionCtx,
    actor: u32,
    address: ItemAddress,
    opening: bool,
) -> Result<(), RequestError> {
    read_item(ctx, actor, address)?;
    let owner = address.owner_id(actor);
    if !RegionInstance::can_inspect_container(ctx, actor, address.item, owner) {
        return Err(RequestError::Forbidden);
    }
    if !opening
        && !matches!(address.owner, ItemOwner::Owned | ItemOwner::Follower(_))
        && ctx.active_container_sessions.get(&actor) != Some(&(address.item, owner))
    {
        return Err(RequestError::Forbidden);
    }
    Ok(())
}
fn range(ctx: &RegionCtx, actor: u32, intent: &str, distance: f32) -> Result<(), RequestError> {
    let rule = intent_rule_config(ctx, actor, intent);
    if !distance.is_finite() || distance > intent_distance_limit(ctx, actor, intent, &rule) {
        Err(RequestError::OutOfRange)
    } else {
        Ok(())
    }
}
impl RegionInstance {
    /// Validate against live authoritative state. Normal handlers repeat their
    /// own gameplay/range/cooldown/capacity checks when consuming the command.
    pub fn prepare_game_request(
        &self,
        actor: u32,
        request: &GameRequest,
        offered: Option<&Choice>,
    ) -> Result<Vec<EntityAction>, RequestError> {
        if !request.bounded() {
            return Err(RequestError::Invalid);
        }
        with_regionctx(self.context.clone(), |ctx| {
            let player = ctx
                .map
                .entities
                .iter()
                .find(|e| e.id == actor && e.is_player())
                .ok_or(RequestError::NotControlled)?;
            if player.get_mode() == "dead" {
                return Err(RequestError::Forbidden);
            }
            let selected = player.attributes.get_str_default("intent", String::new());
            let action = match request {
                GameRequest::InspectSelf(kind) => EntityAction::TextCommand(
                    match kind {
                        SelfInspection::Inventory => "inventory",
                        SelfInspection::Stats => "stats",
                        SelfInspection::Look => "look",
                    }
                    .into(),
                ),
                GameRequest::Craft { recipe } => {
                    if !Self::text_command_recipe_ids(ctx).contains(recipe) {
                        return Err(RequestError::Unsupported);
                    }
                    EntityAction::TextCommand(format!("craft {recipe}"))
                }
                GameRequest::Camera(camera) => {
                    let current = match player.attributes.get("player_camera") {
                        Some(Value::PlayerCamera(camera)) => *camera,
                        _ => PlayerCamera::D2,
                    };
                    let name = match camera {
                        PlayerCamera::D2 => "2d",
                        PlayerCamera::D2Grid => "2d_grid",
                        PlayerCamera::D2Iso => "2d_iso",
                        PlayerCamera::D2IsoGrid => "2d_iso_grid",
                        PlayerCamera::D3Iso => "iso",
                        PlayerCamera::D3FirstP => "firstp",
                        PlayerCamera::D3FirstPGrid => "firstp_grid",
                        PlayerCamera::D3FirstPMouse => "firstp_mouse",
                    };
                    let allowed = ctx
                        .config
                        .get("multiplayer")
                        .and_then(|v| v.get("gameplay"))
                        .and_then(|v| v.get("allowed_cameras"))
                        .and_then(toml::Value::as_array)
                        .is_some_and(|v| v.iter().any(|v| v.as_str() == Some(name)));
                    if current != *camera && !allowed {
                        return Err(RequestError::Forbidden);
                    }
                    EntityAction::SetPlayerCamera(*camera)
                }
                GameRequest::Resource(request) => {
                    resources::validate_request(ctx, actor, request)
                        .map_err(|_| RequestError::Forbidden)?;
                    EntityAction::ResourceInteract(request.clone())
                }
                GameRequest::Intent(raw) => {
                    allowed_intent(ctx, actor, raw)?;
                    EntityAction::Intent(raw.clone())
                }
                GameRequest::Entity { target, intent } => {
                    if !ctx.map.entities.iter().any(|e| {
                        e.id == *target
                            && (e.id == actor
                                || e.attributes.get_bool_default("visible", true)
                                || follower(ctx, actor, e.id).is_ok())
                    }) {
                        return Err(RequestError::NotFound);
                    }
                    let intent =
                        allowed_intent(ctx, actor, intent.as_deref().unwrap_or(&selected))?;
                    let distance = Self::entity_click_distance(ctx, actor, *target)
                        .ok_or(RequestError::NotFound)?;
                    range(ctx, actor, &intent, distance)?;
                    EntityAction::EntityClicked(*target, distance, Some(intent))
                }
                GameRequest::Item { target, intent } => {
                    if matches!(target.owner, ItemOwner::Corpse(_)) {
                        return Err(RequestError::Forbidden);
                    }
                    read_item(ctx, actor, *target)?;
                    let intent =
                        allowed_intent(ctx, actor, intent.as_deref().unwrap_or(&selected))?;
                    let distance =
                        Self::item_click_distance(ctx, actor, target.item, target.owner_id(actor))
                            .ok_or(RequestError::NotFound)?;
                    range(ctx, actor, &intent, distance)?;
                    EntityAction::ItemClicked(
                        target.item,
                        distance,
                        Some(intent),
                        target.owner_id(actor),
                    )
                }
                GameRequest::Prop {
                    instance,
                    part,
                    target,
                    intent,
                    explicit,
                } => {
                    let prop = ctx
                        .map
                        .block_prop_instances
                        .iter()
                        .find(|p| p.id == *instance)
                        .ok_or(RequestError::NotFound)?;
                    let asset = ctx
                        .assets
                        .block_props
                        .get(&prop.asset_id)
                        .ok_or(RequestError::NotFound)?;
                    if asset.find_part(*part).is_none()
                        || target.is_some_and(|id| {
                            asset
                                .find_interaction_target(id)
                                .is_none_or(|t| t.part_id != *part)
                        })
                    {
                        return Err(RequestError::NotFound);
                    }
                    let anchor = target
                        .and_then(|id| crate::block_prop_interaction_world_anchor(asset, prop, id))
                        .or_else(|| crate::block_prop_part_world_anchor(asset, prop, *part))
                        .ok_or(RequestError::NotFound)?;
                    let authored_range = target
                        .and_then(|id| asset.find_interaction_target(id))
                        .and_then(|t| t.component_id)
                        .and_then(|id| asset.components.iter().find(|c| c.id == id))
                        .map(|c| {
                            c.properties
                                .get_float_default("interaction_range", 3.)
                                .max(0.)
                        });
                    let position = ctx
                        .map
                        .entities
                        .iter()
                        .find(|e| e.id == actor)
                        .unwrap()
                        .position;
                    let distance = Self::world_interaction_distance(position, anchor);
                    let intent = allowed_intent(ctx, actor, intent)?;
                    if let Some(limit) = authored_range {
                        if !distance.is_finite() || distance > limit {
                            return Err(RequestError::OutOfRange);
                        }
                    } else {
                        range(ctx, actor, &intent, distance)?;
                    }
                    EntityAction::BlockPropInteract {
                        instance_id: *instance,
                        part_id: *part,
                        target_id: *target,
                        verb: intent,
                        explicit: *explicit,
                    }
                }
                GameRequest::Terrain { position, intent } => {
                    let intent = allowed_intent(ctx, actor, intent)?;
                    if intent.is_empty() {
                        let player = ctx.map.entities.iter().find(|e| e.id == actor).unwrap();
                        if !get_config_bool_default(ctx, "game", "auto_walk_2d", false)
                            || !matches!(
                                player.attributes.get("player_camera"),
                                Some(Value::PlayerCamera(
                                    PlayerCamera::D2
                                        | PlayerCamera::D2Grid
                                        | PlayerCamera::D2Iso
                                        | PlayerCamera::D2IsoGrid
                                ))
                            )
                        {
                            return Err(RequestError::Forbidden);
                        }
                        return Ok(vec![
                            EntityAction::Intent(String::new()),
                            EntityAction::TerrainClicked(Vec2::new(position[0], position[1])),
                        ]);
                    }
                    let pos = ctx
                        .map
                        .entities
                        .iter()
                        .find(|e| e.id == actor)
                        .unwrap()
                        .get_pos_xz();
                    range(
                        ctx,
                        actor,
                        &intent,
                        pos.distance(Vec2::new(position[0], position[1])),
                    )?;
                    // The same ordered actor intent is installed before the
                    // terrain command; clients never choose an event or speed.
                    return Ok(vec![
                        EntityAction::Intent(intent),
                        EntityAction::TerrainClicked(Vec2::new(position[0], position[1])),
                    ]);
                }
                GameRequest::OpenContainer(address) => {
                    container_access(ctx, actor, *address, true)?;
                    EntityAction::OpenContainer {
                        item_id: address.item,
                        owner_entity_id: address.owner_id(actor),
                    }
                }
                GameRequest::Transfer {
                    source,
                    destination,
                } => {
                    match source {
                        ItemSource::Direct(address) => {
                            if matches!(address.owner, ItemOwner::Corpse(_)) {
                                return Err(RequestError::Forbidden);
                            }
                            read_item(ctx, actor, *address)?;
                            if address.owner == ItemOwner::World
                                && !world_item_in_drag_reach(ctx, actor, address.item)
                            {
                                return Err(RequestError::OutOfRange);
                            }
                        }
                        ItemSource::Container { container, item } => {
                            container_access(ctx, actor, *container, false)?;
                            if !read_item(ctx, actor, *container)?
                                .container
                                .as_ref()
                                .is_some_and(|items| items.iter().any(|i| i.id == *item))
                            {
                                return Err(RequestError::NotFound);
                            }
                        }
                    }
                    let target_actor = match destination {
                        ItemDestination::FollowerInventory { entity, .. }
                        | ItemDestination::FollowerEquipment { entity, .. } => {
                            follower(ctx, actor, *entity)?;
                            *entity
                        }
                        _ => actor,
                    };
                    let (mut inventory, equipped) = match destination {
                        ItemDestination::FollowerInventory { entity, index } => {
                            let owner = follower(ctx, actor, *entity)?;
                            if index.is_some_and(|i| i as usize >= owner.inventory.len()) {
                                return Err(RequestError::Invalid);
                            }
                            (index.map(|i| i as usize), None)
                        }
                        ItemDestination::FollowerEquipment { slot, .. } => {
                            (None, Some(slot.clone()))
                        }
                        ItemDestination::Inventory(index) => {
                            if index.is_some_and(|i| {
                                i as usize
                                    >= ctx
                                        .map
                                        .entities
                                        .iter()
                                        .find(|e| e.id == actor)
                                        .unwrap()
                                        .inventory
                                        .len()
                            }) {
                                return Err(RequestError::Invalid);
                            }
                            (index.map(|i| i as usize), None)
                        }
                        ItemDestination::Equipment(slot) => (None, Some(slot.clone())),
                        ItemDestination::Container(container) => {
                            container_access(ctx, actor, *container, false)?;
                            (None, None)
                        }
                        ItemDestination::Ground(p) => {
                            if !drag_position_in_reach(ctx, actor, Vec2::new(p[0], p[1])) {
                                return Err(RequestError::OutOfRange);
                            }
                            (None, None)
                        }
                    };
                    // An explicit inventory request without a slot means the
                    // first free slot, including when removing worn equipment.
                    // The legacy profile-drop action with no destination is a
                    // no-op for owned items, so make this destination concrete.
                    if matches!(
                        destination,
                        ItemDestination::Inventory(None)
                            | ItemDestination::FollowerInventory { index: None, .. }
                    ) && let ItemSource::Direct(address) = source
                        && matches!(address.owner, ItemOwner::Owned | ItemOwner::Follower(_))
                        && let Some(entity) = ctx
                            .map
                            .entities
                            .iter()
                            .find(|e| e.id == address.owner_id(actor).unwrap())
                        && entity.equipped.values().any(|item| item.id == address.item)
                    {
                        let entity = ctx
                            .map
                            .entities
                            .iter()
                            .find(|e| e.id == target_actor)
                            .unwrap();
                        inventory = Some(
                            entity
                                .inventory
                                .iter()
                                .position(Option::is_none)
                                .ok_or(RequestError::Invalid)?,
                        );
                    }
                    match (source, destination) {
                        (source, ItemDestination::Container(container)) => {
                            let (item, owner, source_container, source_owner) = match source {
                                ItemSource::Direct(a) => (a.item, a.owner_id(actor), None, None),
                                ItemSource::Container { container, item } => {
                                    (*item, None, Some(container.item), container.owner_id(actor))
                                }
                            };
                            EntityAction::MoveItemToContainer {
                                item_id: item,
                                owner_entity_id: owner,
                                source_container_item_id: source_container,
                                source_container_owner_entity_id: source_owner,
                                container_item_id: container.item,
                                container_owner_entity_id: container.owner_id(actor),
                            }
                        }
                        (ItemSource::Direct(a), ItemDestination::Ground(p)) => {
                            EntityAction::DropItemAt {
                                item_id: a.item,
                                owner_entity_id: a.owner_id(actor),
                                position: Vec2::new(p[0], p[1]),
                            }
                        }
                        (ItemSource::Container { container, item }, ItemDestination::Ground(p)) => {
                            EntityAction::DropContainerItemAt {
                                item_id: *item,
                                container_item_id: container.item,
                                container_owner_entity_id: container.owner_id(actor),
                                position: Vec2::new(p[0], p[1]),
                            }
                        }
                        (ItemSource::Direct(a), _) => EntityAction::MoveItem {
                            item_id: a.item,
                            owner_entity_id: a.owner_id(actor).or(Some(actor)),
                            target_entity_id: Some(target_actor),
                            to_inventory_index: inventory,
                            to_equipped_slot: equipped,
                        },
                        (ItemSource::Container { container, item }, _) => {
                            EntityAction::MoveContainerItem {
                                item_id: *item,
                                container_item_id: container.item,
                                container_owner_entity_id: container.owner_id(actor),
                                target_entity_id: Some(target_actor),
                                to_inventory_index: inventory,
                                to_equipped_slot: equipped,
                            }
                        }
                    }
                }
                GameRequest::Choose { .. } => {
                    let choice = offered.ok_or(RequestError::ExpiredOffer)?;
                    let (from, to, expires, distance) = choice.session_meta();
                    if to != actor
                        || !ctx
                            .map
                            .entities
                            .iter()
                            .any(|e| e.id == from && e.get_mode() != "dead")
                        || !choice_session_is_valid(ctx, from, to, expires, distance)
                    {
                        return Err(RequestError::ExpiredOffer);
                    }
                    EntityAction::Choice(choice.clone())
                }
                GameRequest::CommandSlot { slot, command } => {
                    if command_slot_attribute(slot).is_none() {
                        return Err(RequestError::Invalid);
                    }
                    if let Some(command) = command
                        && ruleset_command_for_entity(
                            &ctx.rules,
                            ctx.map.entities.iter().find(|e| e.id == actor).unwrap(),
                            command,
                        )
                        .is_none()
                    {
                        return Err(RequestError::Unsupported);
                    }
                    EntityAction::SetCommandSlot {
                        slot: slot.clone(),
                        command: command.clone(),
                    }
                }
            };
            Ok(vec![action])
        })
        .unwrap_or(Err(RequestError::NotControlled))
    }
}

#[cfg(test)]
#[path = "region_requests_tests.rs"]
mod tests;
