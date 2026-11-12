//! CPU party selection, shared by runtime descriptions and graphical bindings.
use crate::{Entity, Map};
pub fn party_members<'a>(map: &'a Map) -> Vec<&'a Entity> {
    let mut members: Vec<&Entity> = map
        .entities
        .iter()
        .filter(|entity| {
            entity.is_player()
                || entity.attributes.get_int("party_index").is_some()
                || entity.attributes.get_bool_default("party_member", false)
                || entity
                    .attributes
                    .get_str("party_role")
                    .is_some_and(|role| !role.trim().is_empty())
        })
        .collect();

    if members.is_empty() {
        if let Some(player) = map.entities.iter().find(|entity| entity.is_player()) {
            members.push(player);
        }
    }

    members.sort_by_key(|entity| {
        (
            entity
                .attributes
                .get_int("party_index")
                .unwrap_or_else(|| if entity.is_player() { 0 } else { i32::MAX / 2 }),
            entity.id,
        )
    });
    members
}

pub fn resolve_party_entity<'a>(map: &'a Map, binding: Option<&str>) -> Option<&'a Entity> {
    let binding = binding.map(str::trim).filter(|value| !value.is_empty());
    let members = party_members(map);

    match binding {
        None | Some("leader") | Some("player") => members.first().copied(),
        Some(value) => {
            if let Some(index) = value.strip_prefix("party.")
                && let Ok(index) = index.parse::<usize>()
            {
                return members.get(index).copied();
            }

            members
                .iter()
                .copied()
                .find(|entity| {
                    entity.attributes.get_str("party_role") == Some(value)
                        || entity.attributes.get_str("name") == Some(value)
                })
                .or_else(|| members.first().copied())
        }
    }
}
