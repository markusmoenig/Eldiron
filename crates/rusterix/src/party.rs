//! CPU party selection, shared by runtime descriptions and graphical bindings.
use crate::{Entity, Map};

/// Character selection for a CPU presentation view. Legacy is reserved for
/// authored previews/reference adapters; an unregistered connection must not
/// silently borrow another player's character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerView {
    #[default]
    Legacy,
    Uncontrolled,
    Controlled(u32),
}

impl PlayerView {
    pub fn controlled(id: Option<u32>) -> Self {
        id.map_or(Self::Uncontrolled, Self::Controlled)
    }

    pub fn player(self, map: &Map) -> Option<&Entity> {
        match self {
            Self::Legacy => map.entities.iter().find(|entity| entity.is_player()),
            Self::Controlled(id) => map
                .entities
                .iter()
                .find(|entity| entity.id == id && entity.is_player()),
            Self::Uncontrolled => None,
        }
    }

    /// Ownership/recipient identity survives temporarily absent bodies (death,
    /// transfer, or an incomplete baseline); physical actor lookup stays strict.
    pub fn entity_id(self, map: &Map) -> Option<u32> {
        match self {
            Self::Controlled(id) => Some(id),
            Self::Legacy => self.player(map).map(|entity| entity.id),
            Self::Uncontrolled => None,
        }
    }

    pub fn members(self, map: &Map) -> Vec<&Entity> {
        if self == Self::Legacy {
            return party_members(map);
        }
        let Some(player) = self.player(map) else {
            return vec![];
        };
        let mut members: Vec<_> = map
            .entities
            .iter()
            .filter(|entity| {
                entity.id == player.id
                    || entity.attributes.get_uint("party_leader_id") == Some(player.id)
            })
            .collect();
        members.sort_by_key(|entity| {
            (
                entity.id != player.id,
                entity.attributes.get_int("party_index").unwrap_or(i32::MAX),
                entity.id,
            )
        });
        members
    }

    pub fn resolve<'a>(self, map: &'a Map, binding: Option<&str>) -> Option<&'a Entity> {
        if self == Self::Legacy {
            return resolve_party_entity(map, binding);
        }
        let player = self.player(map)?;
        let binding = binding.map(str::trim).filter(|value| !value.is_empty());
        match binding {
            None | Some("leader" | "player") => Some(player),
            Some(value) => {
                let members = self.members(map);
                if let Some(index) = value
                    .strip_prefix("party.")
                    .and_then(|index| index.parse::<usize>().ok())
                {
                    return members.get(index).copied();
                }
                members
                    .into_iter()
                    .find(|entity| {
                        entity.attributes.get_str("party_role") == Some(value)
                            || entity.attributes.get_str("name") == Some(value)
                    })
                    .or(Some(player))
            }
        }
    }
}
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
