//! Resolve GPU command-input text against the recipient's visible replica.
//! Only explicit requests cross the wire; the authority repeats all checks.
use crate::*;
use std::collections::BTreeSet;
#[derive(Clone, Copy)]
enum Target {
    Entity(u32),
    Item(ItemAddress),
}
fn targets(map: &rusterix::Map, actor: u32) -> Vec<(String, Target)> {
    let mut targets = vec![];
    for entity in &map.entities {
        if entity.attributes.get_bool_default("visible", true) {
            if let Some(name) = entity.attributes.get_str("name") {
                targets.push((name.to_lowercase(), Target::Entity(entity.id)));
            }
        }
        let companion = !entity.is_player()
            && entity.get_mode() != "dead"
            && entity.attributes.get_bool_default("party_member", false)
            && entity.attributes.get_uint("party_leader_id") == Some(actor);
        if entity.id == actor || companion {
            for item in entity
                .inventory
                .iter()
                .flatten()
                .chain(entity.equipped.values())
            {
                if let Some(name) = item.attributes.get_str("name") {
                    targets.push((
                        name.to_lowercase(),
                        Target::Item(ItemAddress {
                            item: item.id,
                            owner: if companion {
                                ItemOwner::Follower(entity.id)
                            } else {
                                ItemOwner::Owned
                            },
                        }),
                    ));
                }
            }
        }
    }
    for item in &map.items {
        if item.attributes.get_bool_default("visible", true) {
            if let Some(name) = item.attributes.get_str("name") {
                targets.push((
                    name.to_lowercase(),
                    Target::Item(ItemAddress {
                        item: item.id,
                        owner: ItemOwner::World,
                    }),
                ));
            }
        }
    }
    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusterix::{Entity, Item, Map, Value};
    #[test]
    fn command_text_resolves_visible_targets_owned_containers_and_declared_actions() {
        let mut map = Map::default();
        let mut actor = Entity::default();
        actor.id = 1;
        actor.set_attribute("name", Value::Str("Hero".into()));
        let mut bag = Item::default();
        bag.id = 10;
        bag.attributes.set("name", Value::Str("Bag".into()));
        let mut coin = Item::default();
        coin.id = 11;
        coin.attributes.set("name", Value::Str("Coin".into()));
        bag.container = Some(vec![coin]);
        actor.inventory = vec![Some(bag)];
        let mut goblin = Entity::default();
        goblin.id = 2;
        goblin.set_attribute("name", Value::Str("Goblin".into()));
        let mut hidden = Entity::default();
        hidden.id = 3;
        hidden.set_attribute("name", Value::Str("Secret".into()));
        hidden.set_attribute("visible", Value::Bool(false));
        map.entities = vec![actor, goblin, hidden];
        let rules:toml::Table="[spells.fire_bolt]\nname='Fire Bolt'\n[actions.cast_fire_bolt]\nkind='spell'\ntarget='hostile_entity'\n[actions.cast_fire_bolt.requires]\nspell='fire_bolt'\n[actions.heal]\nkind='spell'\ntarget='self'\n[recipes.bowl]\nname='Bowl'".parse().unwrap();
        assert_eq!(
            request("attack Goblin", &map, 1, &rules).unwrap(),
            GameRequest::Entity {
                target: 2,
                intent: Some("attack".into())
            }
        );
        assert_eq!(
            request("cast fire bolt on Goblin", &map, 1, &rules).unwrap(),
            GameRequest::Entity {
                target: 2,
                intent: Some("action:cast_fire_bolt".into())
            }
        );
        assert_eq!(
            request("use heal", &map, 1, &rules).unwrap(),
            GameRequest::Entity {
                target: 1,
                intent: Some("action:heal".into())
            }
        );
        assert_eq!(
            request("craft bowl", &map, 1, &rules).unwrap(),
            GameRequest::Craft {
                recipe: "bowl".into()
            }
        );
        assert_eq!(
            request("open Bag", &map, 1, &rules).unwrap(),
            GameRequest::OpenContainer(ItemAddress {
                item: 10,
                owner: ItemOwner::Owned
            })
        );
        assert_eq!(
            request("take Coin from Bag", &map, 1, &rules).unwrap(),
            GameRequest::Transfer {
                source: ItemSource::Container {
                    container: ItemAddress {
                        item: 10,
                        owner: ItemOwner::Owned
                    },
                    item: 11
                },
                destination: ItemDestination::Inventory(None)
            }
        );
        assert!(request("attack Secret", &map, 1, &rules).is_err());
        assert!(request("grant_admin", &map, 1, &rules).is_err());
        assert!(request("craft bowl\nadmin", &map, 1, &rules).is_err());
        map.entities[0].set_attribute("name", Value::Str("Goblin".into()));
        assert!(
            request("attack Goblin", &map, 1, &rules).is_err(),
            "ambiguous names must not select an arbitrary body"
        );
    }
}
fn resolve(list: &[(String, Target)], name: &str, actor: u32) -> Result<Target, RequestError> {
    let name = name.trim().to_lowercase();
    if matches!(name.as_str(), "me" | "self") {
        return Ok(Target::Entity(actor));
    }
    let mut matches = list.iter().filter(|(candidate, _)| candidate == &name);
    let target = matches.next().ok_or(RequestError::NotFound)?.1;
    if matches.next().is_some() {
        return Err(RequestError::Invalid);
    }
    Ok(target)
}
fn addressed(target: Target, intent: String) -> GameRequest {
    match target {
        Target::Entity(target) => GameRequest::Entity {
            target,
            intent: Some(intent),
        },
        Target::Item(target) => GameRequest::Item {
            target,
            intent: Some(intent),
        },
    }
}
fn item(target: Target) -> Result<ItemAddress, RequestError> {
    match target {
        Target::Item(item) => Ok(item),
        _ => Err(RequestError::NotFound),
    }
}
pub(crate) fn request(
    text: &str,
    map: &rusterix::Map,
    actor: u32,
    rules: &toml::Table,
) -> Result<GameRequest, RequestError> {
    if text.len() > 512 || text.chars().any(char::is_control) {
        return Err(RequestError::Invalid);
    }
    let ids = |key: &str| {
        rules
            .get(key)
            .and_then(toml::Value::as_table)
            .map(|v| v.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default()
    };
    let mut intents = ids("intents");
    intents.extend(
        [
            "attack", "talk", "take", "drop", "use", "look", "open", "close", "equip", "unequip",
            "buy", "sell",
        ]
        .map(str::to_string),
    );
    let actions = ids("actions");
    let spells = ids("spells");
    let recipes = ids("recipes");
    let targets = targets(map, actor);
    let invoke = |action: String, target: Option<String>| -> Result<GameRequest, RequestError> {
        if !actions.contains(&action) {
            return Err(RequestError::Unsupported);
        }
        let intent = format!("action:{action}");
        if let Some(target) = target {
            return Ok(addressed(resolve(&targets, &target, actor)?, intent));
        }
        if rules
            .get("actions")
            .and_then(|v| v.get(&action))
            .and_then(|v| v.get("target"))
            .and_then(toml::Value::as_str)
            == Some("self")
        {
            Ok(addressed(Target::Entity(actor), intent))
        } else {
            Ok(GameRequest::Intent(intent))
        }
    };
    use rusterix::text_command::TextCommand as T;
    let request = match rusterix::text_command::parse_text_command(
        text, &intents, &spells, &actions, &recipes,
    ) {
        T::Inventory => GameRequest::InspectSelf(SelfInspection::Inventory),
        T::Stats => GameRequest::InspectSelf(SelfInspection::Stats),
        T::Look(None) => GameRequest::InspectSelf(SelfInspection::Look),
        T::Look(Some(target)) => addressed(resolve(&targets, &target, actor)?, "look".into()),
        T::Intent {
            intent,
            target: Some(target),
        } => addressed(resolve(&targets, &target, actor)?, intent),
        T::Intent {
            intent,
            target: None,
        } => GameRequest::Intent(intent),
        T::Action { action, target } => invoke(action, target)?,
        T::Cast { spell, target } => {
            let matches: Vec<_> = rules
                .get("actions")
                .and_then(toml::Value::as_table)
                .into_iter()
                .flat_map(|v| v.iter())
                .filter(|(id, v)| {
                    v.get("kind").and_then(toml::Value::as_str) == Some("spell")
                        && (*id == &spell
                            || v.get("spell").and_then(toml::Value::as_str) == Some(spell.as_str())
                            || v.get("requires")
                                .and_then(|v| v.get("spell"))
                                .and_then(toml::Value::as_str)
                                == Some(spell.as_str()))
                })
                .map(|(id, _)| id.clone())
                .collect();
            if matches.len() != 1 {
                return Err(RequestError::Unsupported);
            }
            invoke(matches[0].clone(), target)?
        }
        T::Craft { recipe } => GameRequest::Craft { recipe },
        T::Open { container } => {
            GameRequest::OpenContainer(item(resolve(&targets, &container, actor)?)?)
        }
        T::PutIn {
            item: name,
            container,
        } => GameRequest::Transfer {
            source: ItemSource::Direct(item(resolve(&targets, &name, actor)?)?),
            destination: ItemDestination::Container(item(resolve(&targets, &container, actor)?)?),
        },
        T::TakeFrom {
            item: name,
            container,
        } => {
            let container = item(resolve(&targets, &container, actor)?)?;
            let bag = match container.owner {
                ItemOwner::World => map.items.iter().find(|i| i.id == container.item),
                ItemOwner::Owned | ItemOwner::Follower(_) => map
                    .entities
                    .iter()
                    .find(|e| Some(e.id) == container.owner_id(actor))
                    .and_then(|e| {
                        e.get_item(container.item)
                            .or_else(|| e.equipped.values().find(|i| i.id == container.item))
                    }),
                _ => None,
            }
            .ok_or(RequestError::NotFound)?;
            let matches: Vec<_> = bag
                .container
                .iter()
                .flatten()
                .filter(|i| {
                    i.attributes
                        .get_str("name")
                        .is_some_and(|n| n.eq_ignore_ascii_case(name.trim()))
                })
                .collect();
            if matches.len() != 1 {
                return Err(RequestError::NotFound);
            }
            GameRequest::Transfer {
                source: ItemSource::Container {
                    container,
                    item: matches[0].id,
                },
                destination: ItemDestination::Inventory(None),
            }
        }
        _ => return Err(RequestError::Unsupported),
    };
    if request.bounded() {
        Ok(request)
    } else {
        Err(RequestError::Invalid)
    }
}
