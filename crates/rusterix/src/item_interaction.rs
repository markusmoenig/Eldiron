//! CPU item addresses and command construction shared by graphical owners.
//! A gesture owns geometry/lifetimes; the authoritative server owns capacity,
//! permission, equipment and stacking rules. No UI, window or renderer lives here.
use crate::{
    EntityAction, Item, Map, party::resolve_party_entity, world_interaction::item_is_container,
};
use vek::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemRef {
    pub item: u32,
    /// None is the world namespace, never a search through unrelated actors.
    pub owner: Option<u32>,
}
impl ItemRef {
    pub fn resolve(self, map: &Map) -> Option<&Item> {
        match self.owner {
            Some(owner) => {
                let owner = map.entities.iter().find(|entity| entity.id == owner)?;
                owner
                    .inventory
                    .iter()
                    .flatten()
                    .chain(owner.equipped.values())
                    .find(|item| item.id == self.item)
            }
            None => map.items.iter().find(|item| item.id == self.item),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemSource {
    Direct(ItemRef),
    Container { container: ItemRef, item: u32 },
}
impl ItemSource {
    pub fn resolve(self, map: &Map) -> Option<&Item> {
        match self {
            Self::Direct(item) => item.resolve(map),
            Self::Container { container, item } => container
                .resolve(map)?
                .container
                .as_ref()?
                .iter()
                .find(|entry| entry.id == item),
        }
    }
    pub fn item(self) -> u32 {
        match self {
            Self::Direct(item) => item.item,
            Self::Container { item, .. } => item,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ItemDestination {
    Actor {
        entity: Option<u32>,
        inventory: Option<usize>,
        equipped: Option<String>,
    },
    Container(ItemRef),
    World(Vec2<f32>),
}
#[derive(Clone, Debug, PartialEq)]
pub enum ItemClick {
    Action {
        action: EntityAction,
        uses_intent: bool,
    },
    ToggleContainer(ItemRef),
}
pub fn click(
    map: &Map,
    source: ItemSource,
    intent: Option<&str>,
    distance: f32,
    open_container: bool,
) -> Option<ItemClick> {
    let item = source.resolve(map)?;
    if !distance.is_finite() || distance < 0. {
        return None;
    }
    match source {
        ItemSource::Container { container, item } => Some(ItemClick::Action {
            action: EntityAction::MoveContainerItem {
                item_id: item,
                container_item_id: container.item,
                container_owner_entity_id: container.owner,
                target_entity_id: None,
                to_inventory_index: None,
                to_equipped_slot: None,
            },
            uses_intent: false,
        }),
        ItemSource::Direct(address) => {
            let intent = intent.filter(|intent| !intent.trim().is_empty());
            if open_container && item_is_container(item) {
                return Some(ItemClick::ToggleContainer(address));
            }
            Some(ItemClick::Action {
                action: EntityAction::ItemClicked(
                    address.item,
                    distance,
                    intent.map(str::to_owned),
                    address.owner,
                ),
                uses_intent: intent.is_some(),
            })
        }
    }
}
pub fn transfer(map: &Map, source: ItemSource, target: ItemDestination) -> Option<EntityAction> {
    source.resolve(map)?;
    match target {
        ItemDestination::Actor {
            entity,
            inventory,
            equipped,
        } => {
            match entity {
                Some(id) => {
                    map.entities.iter().find(|entity| entity.id == id)?;
                }
                None => {
                    resolve_party_entity(map, None)?;
                }
            }
            if inventory.is_some() && equipped.is_some()
                || equipped
                    .as_deref()
                    .is_some_and(|slot| slot.trim().is_empty())
            {
                return None;
            }
            Some(match source {
                ItemSource::Direct(item) => EntityAction::MoveItem {
                    item_id: item.item,
                    owner_entity_id: item.owner,
                    target_entity_id: entity,
                    to_inventory_index: inventory,
                    to_equipped_slot: equipped,
                },
                ItemSource::Container { container, item } => EntityAction::MoveContainerItem {
                    item_id: item,
                    container_item_id: container.item,
                    container_owner_entity_id: container.owner,
                    target_entity_id: entity,
                    to_inventory_index: inventory,
                    to_equipped_slot: equipped,
                },
            })
        }
        ItemDestination::Container(target) => {
            if !item_is_container(target.resolve(map)?) {
                return None;
            }
            if matches!(source, ItemSource::Direct(item) if item == target)
                || matches!(source, ItemSource::Container { container, .. } if container == target)
            {
                return None;
            }
            let (owner, parent) = match source {
                ItemSource::Direct(item) => (item.owner, None),
                ItemSource::Container { container, .. } => (None, Some(container)),
            };
            Some(EntityAction::MoveItemToContainer {
                item_id: source.item(),
                owner_entity_id: owner,
                source_container_item_id: parent.map(|item| item.item),
                source_container_owner_entity_id: parent.and_then(|item| item.owner),
                container_item_id: target.item,
                container_owner_entity_id: target.owner,
            })
        }
        ItemDestination::World(position) => {
            if !position.x.is_finite() || !position.y.is_finite() {
                return None;
            }
            Some(match source {
                ItemSource::Direct(item) => EntityAction::DropItemAt {
                    item_id: item.item,
                    owner_entity_id: item.owner,
                    position,
                },
                ItemSource::Container { container, item } => EntityAction::DropContainerItemAt {
                    item_id: item,
                    container_item_id: container.item,
                    container_owner_entity_id: container.owner,
                    position,
                },
            })
        }
    }
}

/// Compatibility presenters retain layout/picking decisions while transfers
/// share strict source addresses and command construction.
pub fn validate_transfer(map: &Map, action: EntityAction) -> Option<EntityAction> {
    let (source, target) = match &action {
        EntityAction::MoveItem {
            item_id,
            owner_entity_id,
            target_entity_id,
            to_inventory_index,
            to_equipped_slot,
        } => (
            ItemSource::Direct(ItemRef {
                item: *item_id,
                owner: *owner_entity_id,
            }),
            ItemDestination::Actor {
                entity: *target_entity_id,
                inventory: *to_inventory_index,
                equipped: to_equipped_slot.clone(),
            },
        ),
        EntityAction::MoveContainerItem {
            item_id,
            container_item_id,
            container_owner_entity_id,
            target_entity_id,
            to_inventory_index,
            to_equipped_slot,
        } => (
            ItemSource::Container {
                container: ItemRef {
                    item: *container_item_id,
                    owner: *container_owner_entity_id,
                },
                item: *item_id,
            },
            ItemDestination::Actor {
                entity: *target_entity_id,
                inventory: *to_inventory_index,
                equipped: to_equipped_slot.clone(),
            },
        ),
        EntityAction::MoveItemToContainer {
            item_id,
            owner_entity_id,
            source_container_item_id,
            source_container_owner_entity_id,
            container_item_id,
            container_owner_entity_id,
        } => (
            source_container_item_id.map_or(
                ItemSource::Direct(ItemRef {
                    item: *item_id,
                    owner: *owner_entity_id,
                }),
                |parent| ItemSource::Container {
                    container: ItemRef {
                        item: parent,
                        owner: *source_container_owner_entity_id,
                    },
                    item: *item_id,
                },
            ),
            ItemDestination::Container(ItemRef {
                item: *container_item_id,
                owner: *container_owner_entity_id,
            }),
        ),
        EntityAction::DropItemAt {
            item_id,
            owner_entity_id,
            position,
        } => (
            ItemSource::Direct(ItemRef {
                item: *item_id,
                owner: *owner_entity_id,
            }),
            ItemDestination::World(*position),
        ),
        EntityAction::DropContainerItemAt {
            item_id,
            container_item_id,
            container_owner_entity_id,
            position,
        } => (
            ItemSource::Container {
                container: ItemRef {
                    item: *container_item_id,
                    owner: *container_owner_entity_id,
                },
                item: *item_id,
            },
            ItemDestination::World(*position),
        ),
        _ => return Some(action),
    };
    transfer(map, source, target).map(|_| action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entity, Value};
    fn fixture() -> Map {
        let mut map = Map::default();
        let mut world = Item::new();
        world.id = 7;
        map.items.push(world);
        for id in [1, 2] {
            let mut entity = Entity::new();
            entity.id = id;
            entity.set_attribute("player", Value::Bool(id == 1));
            let mut item = Item::new();
            item.id = 7;
            entity.inventory.push(Some(item));
            map.entities.push(entity);
        }
        let mut container = Item::new();
        container.id = 8;
        container.set_attribute("container", Value::Bool(true));
        let mut child = Item::new();
        child.id = 9;
        container.container = Some(vec![child]);
        map.entities[0].inventory.push(Some(container));
        map
    }
    #[test]
    fn addresses_keep_world_actor_and_container_namespaces_and_reject_missing_owners() {
        let map = fixture();
        let world = ItemRef {
            item: 7,
            owner: None,
        };
        let first = ItemRef {
            item: 7,
            owner: Some(1),
        };
        let second = ItemRef {
            item: 7,
            owner: Some(2),
        };
        assert!(!std::ptr::eq(
            world.resolve(&map).unwrap(),
            first.resolve(&map).unwrap()
        ));
        assert!(!std::ptr::eq(
            first.resolve(&map).unwrap(),
            second.resolve(&map).unwrap()
        ));
        assert!(
            ItemRef {
                owner: Some(99),
                ..first
            }
            .resolve(&map)
            .is_none()
        );
        assert!(
            ItemSource::Container {
                container: ItemRef {
                    item: 8,
                    owner: Some(2)
                },
                item: 9
            }
            .resolve(&map)
            .is_none()
        );
    }
    #[test]
    fn moves_preserve_serialized_addresses_and_container_extraction_defaults() {
        let map = fixture();
        let source = ItemSource::Direct(ItemRef {
            item: 7,
            owner: Some(1),
        });
        assert_eq!(
            transfer(
                &map,
                source,
                ItemDestination::Actor {
                    entity: Some(2),
                    inventory: Some(4),
                    equipped: None
                }
            ),
            Some(EntityAction::MoveItem {
                item_id: 7,
                owner_entity_id: Some(1),
                target_entity_id: Some(2),
                to_inventory_index: Some(4),
                to_equipped_slot: None
            })
        );
        let child = ItemSource::Container {
            container: ItemRef {
                item: 8,
                owner: Some(1),
            },
            item: 9,
        };
        let mut target = Item::new();
        target.id = 10;
        target.set_attribute("container", Value::Bool(true));
        let mut map = map;
        map.items.push(target);
        let wire = EntityAction::MoveItemToContainer {
            item_id: 9,
            owner_entity_id: Some(1),
            source_container_item_id: Some(8),
            source_container_owner_entity_id: Some(1),
            container_item_id: 10,
            container_owner_entity_id: None,
        };
        assert_eq!(
            validate_transfer(&map, wire.clone()),
            Some(wire),
            "compatibility validation must retain the caller's wire fields"
        );
        assert_eq!(
            click(&map, child, Some("look"), 0., false),
            Some(ItemClick::Action {
                action: EntityAction::MoveContainerItem {
                    item_id: 9,
                    container_item_id: 8,
                    container_owner_entity_id: Some(1),
                    target_entity_id: None,
                    to_inventory_index: None,
                    to_equipped_slot: None
                },
                uses_intent: false
            })
        );
        assert!(matches!(
            transfer(&map, child, ItemDestination::World(Vec2::new(2., 3.))),
            Some(EntityAction::DropContainerItemAt {
                item_id: 9,
                container_item_id: 8,
                container_owner_entity_id: Some(1),
                ..
            })
        ));
    }
    #[test]
    fn invalid_destinations_self_containment_stale_items_and_nonfinite_positions_emit_no_action() {
        let mut map = fixture();
        let bag = ItemRef {
            item: 8,
            owner: Some(1),
        };
        let source = ItemSource::Direct(bag);
        assert!(transfer(&map, source, ItemDestination::Container(bag)).is_none());
        assert!(
            transfer(
                &map,
                ItemSource::Container {
                    container: bag,
                    item: 9
                },
                ItemDestination::Container(bag)
            )
            .is_none()
        );
        assert!(
            transfer(
                &map,
                source,
                ItemDestination::Actor {
                    entity: Some(99),
                    inventory: None,
                    equipped: None
                }
            )
            .is_none()
        );
        assert!(
            transfer(
                &map,
                source,
                ItemDestination::Actor {
                    entity: Some(2),
                    inventory: Some(0),
                    equipped: Some("body".into())
                }
            )
            .is_none()
        );
        assert!(
            transfer(
                &map,
                source,
                ItemDestination::World(Vec2::new(f32::NAN, 0.))
            )
            .is_none()
        );
        map.entities[0].inventory.clear();
        assert!(transfer(&map, source, ItemDestination::World(Vec2::zero())).is_none());
    }
}
