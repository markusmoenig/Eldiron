use crate::prelude::*;
use rusterix::{Entity, Item, Linedef, Map, Sector, Value};
use std::collections::BTreeSet;
use toml::Table;
use vek::Vec2;

const TEXT_ROOM_FALLBACK_DISTANCE: f32 = 2.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StartupDisplay {
    Description,
    Room,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExitPresentation {
    List,
    Sentence,
}

#[derive(Clone)]
pub struct TextExit {
    pub direction: String,
    pub title: String,
    pub target_title: String,
    pub target_sector_id: u32,
    pub target_center: Vec2<f32>,
}

#[derive(Clone)]
pub enum TextTarget {
    Entity { id: u32, distance: f32 },
    Item { id: u32, distance: f32 },
}

#[derive(Clone, Default)]
pub struct TextRoom {
    pub title: String,
    pub description: String,
    pub exits: Vec<TextExit>,
    pub live_entities: Vec<String>,
    pub nearby_attackers: Vec<String>,
    pub dead_entities: Vec<String>,
    pub items: Vec<String>,
}

pub fn config_table(src: &str) -> Option<Table> {
    src.parse::<Table>().ok()
}

pub fn authoring_startup_display(src: &str) -> StartupDisplay {
    config_table(src)
        .and_then(|table| {
            table
                .get("startup")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("show"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "none" => StartupDisplay::None,
            "room" => StartupDisplay::Room,
            _ => StartupDisplay::Description,
        })
        .unwrap_or(StartupDisplay::Description)
}

pub fn authoring_startup_welcome(src: &str) -> Option<String> {
    config_table(src)
        .and_then(|table| {
            table
                .get("startup")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("welcome"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn authoring_connection_probe_distance(src: &str) -> f32 {
    config_table(src)
        .and_then(|table| {
            table
                .get("connections")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("probe_distance"))
                .and_then(|value| {
                    value
                        .as_float()
                        .or_else(|| value.as_integer().map(|v| v as f64))
                })
        })
        .map(|value| value as f32)
        .unwrap_or(1.5)
}

pub fn authoring_exit_presentation(src: &str) -> ExitPresentation {
    config_table(src)
        .and_then(|table| {
            table
                .get("exits")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("style"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "sentence" => ExitPresentation::Sentence,
            _ => ExitPresentation::List,
        })
        .unwrap_or(ExitPresentation::List)
}

pub fn current_player_and_sector(map: &Map) -> Option<(&Entity, &Sector)> {
    let player = map.entities.iter().find(|entity| entity.is_player())?;
    let player_pos = player.get_pos_xz();
    let current_sector_id = player
        .attributes
        .get("sector_id")
        .and_then(|value| match value {
            Value::Int64(v) if *v >= 0 => Some(*v as u32),
            Value::Int(v) if *v >= 0 => Some(*v as u32),
            _ => None,
        });
    let current_sector_name = player
        .get_attr_string("sector")
        .filter(|s| !s.is_empty())
        .or_else(|| map.find_sector_at(player_pos).map(|s| s.name.clone()))
        .unwrap_or_default();

    let sector = if let Some(current_sector_id) = current_sector_id {
        map.sectors
            .iter()
            .find(|sector| sector.id == current_sector_id)
            .or_else(|| map.find_sector_at(player_pos))
            .or_else(|| probe_nearest_sector(map, player_pos, TEXT_ROOM_FALLBACK_DISTANCE))
    } else if !current_sector_name.is_empty() {
        map.sectors
            .iter()
            .find(|sector| sector.name == current_sector_name)
            .or_else(|| map.find_sector_at(player_pos))
            .or_else(|| probe_nearest_sector(map, player_pos, TEXT_ROOM_FALLBACK_DISTANCE))
    } else {
        map.find_sector_at(player_pos)
            .or_else(|| probe_nearest_sector(map, player_pos, TEXT_ROOM_FALLBACK_DISTANCE))
    }?;

    Some((player, sector))
}

pub fn display_name_for_entity(entity: &Entity) -> String {
    entity
        .get_attr_string("name")
        .or_else(|| entity.get_attr_string("class_name"))
        .unwrap_or_else(|| format!("Entity {}", entity.id))
}

pub fn display_name_for_item(item: &Item) -> String {
    item.get_attr_string("name")
        .or_else(|| item.get_attr_string("class_name"))
        .unwrap_or_else(|| format!("Item {}", item.id))
}

pub fn entity_is_dead(entity: &Entity) -> bool {
    entity
        .get_attr_string("mode")
        .map(|mode| mode.eq_ignore_ascii_case("dead"))
        .unwrap_or(false)
}

pub fn corpse_name_for_entity(entity: &Entity) -> String {
    let name = display_name_for_entity(entity);
    if name.trim().is_empty() {
        String::new()
    } else {
        format!("corpse of {}", sentence_case_exit_title(&name))
    }
}

pub fn entity_sector_matches(map: &Map, entity: &Entity, sector: &Sector) -> bool {
    entity
        .attributes
        .get("sector_id")
        .and_then(|value| match value {
            Value::Int64(v) if *v >= 0 => Some(*v as u32),
            Value::Int(v) if *v >= 0 => Some(*v as u32),
            _ => None,
        })
        .map(|id| id == sector.id)
        .or_else(|| {
            entity
                .get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .map(|s| s == sector.name)
        })
        .unwrap_or_else(|| {
            let pos = entity.get_pos_xz();
            map.find_sector_at(pos)
                .or_else(|| probe_nearest_sector(map, pos, TEXT_ROOM_FALLBACK_DISTANCE))
                .map(|s| s.id)
                == Some(sector.id)
        })
}

fn entity_target_matches_player(entity: &Entity, player_id: u32) -> bool {
    entity
        .attributes
        .get("target")
        .map(|value| match value {
            Value::UInt(v) => *v == player_id,
            Value::Int(v) if *v >= 0 => *v as u32 == player_id,
            Value::Int64(v) if *v >= 0 => *v as u32 == player_id,
            Value::Str(v) => v.trim().parse::<u32>().ok() == Some(player_id),
            _ => false,
        })
        .unwrap_or(false)
}

pub fn item_sector_matches(map: &Map, item: &Item, sector: &Sector) -> bool {
    item.attributes
        .get("sector_id")
        .and_then(|value| match value {
            Value::Int64(v) if *v >= 0 => Some(*v as u32),
            Value::Int(v) if *v >= 0 => Some(*v as u32),
            _ => None,
        })
        .map(|id| id == sector.id)
        .or_else(|| {
            item.get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .map(|s| s == sector.name)
        })
        .unwrap_or_else(|| {
            let pos = item.get_pos_xz();
            map.find_sector_at(pos)
                .or_else(|| probe_nearest_sector(map, pos, TEXT_ROOM_FALLBACK_DISTANCE))
                .map(|s| s.id)
                == Some(sector.id)
        })
}

pub fn sector_text_metadata(sector: &Sector) -> (String, String) {
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
    }

    (title.trim().to_string(), description.trim_end().to_string())
}

pub fn linedef_text_metadata(linedef: &Linedef) -> (String, String) {
    let mut title = linedef.name.clone();
    let mut description = String::new();

    if let Some(Value::Str(data)) = linedef.properties.get("data")
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
    }

    (title.trim().to_string(), description.trim_end().to_string())
}

pub fn cardinal_direction(from: Vec2<f32>, to: Vec2<f32>) -> String {
    let delta = to - from;
    if delta.x.abs() > delta.y.abs() {
        if delta.x >= 0.0 { "east" } else { "west" }
    } else if delta.y >= 0.0 {
        "south"
    } else {
        "north"
    }
    .to_string()
}

pub fn probe_nearest_sector(map: &Map, origin: Vec2<f32>, max_distance: f32) -> Option<&Sector> {
    let mut best: Option<(&Sector, f32)> = None;

    for sector in &map.sectors {
        if sector.layer.is_some() {
            continue;
        }
        if let Some(distance) = sector.signed_distance(map, origin) {
            let distance = distance.abs();
            if distance <= max_distance {
                match best {
                    Some((_, best_distance)) if distance >= best_distance => {}
                    _ => best = Some((sector, distance)),
                }
            }
        }
    }

    best.map(|(sector, _)| sector)
}

pub fn resolve_text_exits(map: &Map, sector: &Sector, probe_distance: f32) -> Vec<TextExit> {
    let mut exits = Vec::new();
    let Some(current_center) = sector.center(map) else {
        return exits;
    };

    for linedef in &map.linedefs {
        let (line_title, line_description) = linedef_text_metadata(linedef);
        if line_title.is_empty() && line_description.is_empty() {
            continue;
        }

        let Some(v0) = map.get_vertex(linedef.start_vertex) else {
            continue;
        };
        let Some(v1) = map.get_vertex(linedef.end_vertex) else {
            continue;
        };

        let edge = v1 - v0;
        if edge.magnitude_squared() <= f32::EPSILON {
            continue;
        }

        let endpoint_a = probe_nearest_sector(map, v0, probe_distance);
        let endpoint_b = probe_nearest_sector(map, v1, probe_distance);

        let side_a;
        let side_b;
        let (front_sector, back_sector) = if let (Some(a), Some(b)) = (endpoint_a, endpoint_b) {
            (a, b)
        } else {
            let midpoint = (v0 + v1) * 0.5;
            let normal = Vec2::new(-edge.y, edge.x).normalized();
            side_a = probe_nearest_sector(
                map,
                midpoint - normal * (probe_distance * 0.5),
                probe_distance,
            );
            side_b = probe_nearest_sector(
                map,
                midpoint + normal * (probe_distance * 0.5),
                probe_distance,
            );
            let (Some(a), Some(b)) = (side_a, side_b) else {
                continue;
            };
            (a, b)
        };

        if front_sector.id == back_sector.id {
            continue;
        }

        let target_sector = if front_sector.id == sector.id {
            back_sector
        } else if back_sector.id == sector.id {
            front_sector
        } else {
            continue;
        };

        let Some(target_center) = target_sector.center(map) else {
            continue;
        };
        let direction = cardinal_direction(current_center, target_center);
        let (target_title, _) = sector_text_metadata(target_sector);
        let title = if !line_title.is_empty() {
            line_title.clone()
        } else if !target_title.is_empty() {
            target_title.clone()
        } else {
            target_sector.name.clone()
        };

        exits.push(TextExit {
            direction,
            title,
            target_title,
            target_sector_id: target_sector.id,
            target_center,
        });
    }

    exits.sort_by_key(|exit| match exit.direction.as_str() {
        "north" => 0,
        "east" => 1,
        "south" => 2,
        "west" => 3,
        _ => 4,
    });
    exits.dedup_by(|a, b| a.direction == b.direction && a.target_sector_id == b.target_sector_id);
    exits
}

pub fn build_text_room(map: &Map, authoring: &str) -> Option<TextRoom> {
    let (player, sector) = current_player_and_sector(map)?;
    let probe_distance = authoring_connection_probe_distance(authoring);
    let (title, description) = sector_text_metadata(sector);
    let exits = resolve_text_exits(map, sector, probe_distance);
    let live_entities = map
        .entities
        .iter()
        .filter(|entity| !entity.is_player() && !entity_is_dead(entity))
        .filter(|entity| entity_sector_matches(map, entity, sector))
        .map(display_name_for_entity)
        .filter(|name| !name.trim().is_empty())
        .collect();
    let nearby_attackers = map
        .entities
        .iter()
        .filter(|entity| !entity.is_player() && !entity_is_dead(entity))
        .filter(|entity| !entity_sector_matches(map, entity, sector))
        .filter(|entity| entity_target_matches_player(entity, player.id))
        .map(display_name_for_entity)
        .filter(|name| !name.trim().is_empty())
        .collect();
    let dead_entities = map
        .entities
        .iter()
        .filter(|entity| !entity.is_player() && entity_is_dead(entity))
        .filter(|entity| entity_sector_matches(map, entity, sector))
        .map(corpse_name_for_entity)
        .filter(|name| !name.trim().is_empty())
        .collect();
    let items = map
        .items
        .iter()
        .filter(|item| item_sector_matches(map, item, sector))
        .map(display_name_for_item)
        .filter(|name| !name.trim().is_empty())
        .collect();

    Some(TextRoom {
        title,
        description,
        exits,
        live_entities,
        nearby_attackers,
        dead_entities,
        items,
    })
}

pub fn render_current_sector_description(map: &Map) -> Option<String> {
    let (_, sector) = current_player_and_sector(map)?;
    let (_, description) = sector_text_metadata(sector);
    if description.trim().is_empty() {
        None
    } else {
        Some(description.trim_end().to_string())
    }
}

pub fn render_player_inventory(map: &Map) -> Option<String> {
    let (player, _) = current_player_and_sector(map)?;

    let mut lines = vec!["Inventory:".to_string()];
    let configured_slots = player
        .attributes
        .get("inventory_slots")
        .and_then(|value| match value {
            Value::Int(v) if *v >= 0 => Some(*v as usize),
            Value::Int64(v) if *v >= 0 => Some(*v as usize),
            Value::UInt(v) => Some(*v as usize),
            _ => None,
        })
        .unwrap_or(0);

    let slot_count = player.inventory.len().max(configured_slots);
    if slot_count == 0 {
        lines.push("  <empty>".to_string());
    } else {
        for index in 0..slot_count {
            let label = match player.inventory.get(index).and_then(|slot| slot.as_ref()) {
                Some(item) => display_name_for_item(item),
                None => "<empty>".to_string(),
            };
            lines.push(format!("  {}. {}", index + 1, label));
        }
    }

    Some(lines.join("\n"))
}

pub fn normalize_target_name(text: &str) -> String {
    text.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c.is_ascii_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn target_matches(name: &str, query: &str) -> bool {
    let name_norm = normalize_target_name(name);
    let query_norm = normalize_target_name(query);
    !query_norm.is_empty()
        && (name_norm == query_norm
            || name_norm.starts_with(&(query_norm.clone() + " "))
            || name_norm.contains(&format!(" {}", query_norm)))
}

pub fn resolve_text_target(map: &Map, sector: &Sector, query: &str) -> Result<TextTarget, String> {
    let Some((player, _)) = current_player_and_sector(map) else {
        return Err("No local player found.".into());
    };
    let player_pos = player.get_pos_xz();
    let mut matches: Vec<(String, TextTarget)> = Vec::new();

    for entity in &map.entities {
        if entity.is_player() || entity_is_dead(entity) {
            continue;
        }
        let in_room = entity_sector_matches(map, entity, sector);
        let attacking_player = entity_target_matches_player(entity, player.id);
        if !in_room && !attacking_player {
            continue;
        }
        let name = display_name_for_entity(entity);
        if target_matches(&name, query) {
            matches.push((
                name,
                TextTarget::Entity {
                    id: entity.id,
                    distance: player_pos.distance(entity.get_pos_xz()),
                },
            ));
        }
    }

    for item in &map.items {
        if !item_sector_matches(map, item, sector) {
            continue;
        }
        let name = display_name_for_item(item);
        if target_matches(&name, query) {
            matches.push((
                name,
                TextTarget::Item {
                    id: item.id,
                    distance: player_pos.distance(item.get_pos_xz()),
                },
            ));
        }
    }

    if matches.is_empty() {
        return Err(format!("You do not see '{}' here.", query.trim()));
    }
    if matches.len() > 1 {
        matches.sort_by(|a, b| a.0.cmp(&b.0));
        let names = matches
            .into_iter()
            .map(|m| m.0)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("Be more specific: {}", names));
    }

    Ok(matches.remove(0).1)
}

pub fn current_player_supported_intents(project: &Project, map: &Map) -> BTreeSet<String> {
    let mut intents = BTreeSet::new();
    let Some((player, _)) = current_player_and_sector(map) else {
        return intents;
    };
    let Some(class_name) = player.get_attr_string("class_name") else {
        return intents;
    };
    let Some(character) = project.characters.values().find(|c| c.name == class_name) else {
        return intents;
    };
    let Ok(table) = character.data.parse::<Table>() else {
        return intents;
    };
    let Some(input) = table.get("input").and_then(toml::Value::as_table) else {
        return intents;
    };

    for value in input.values().filter_map(toml::Value::as_str) {
        let trimmed = value.trim();
        let lower = trimmed.to_ascii_lowercase();
        if let Some(inner) = lower
            .strip_prefix("intent(")
            .and_then(|v| v.strip_suffix(')'))
            .map(str::trim)
        {
            let inner = inner.trim_matches('"').trim_matches('\'').trim();
            if !inner.is_empty() {
                intents.insert(inner.to_string());
            }
        }
    }

    intents
}

pub fn with_indefinite_article(text: &str) -> String {
    let lower = text.trim_start().to_ascii_lowercase();
    if lower.starts_with("your ")
        || lower.starts_with("my ")
        || lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("in ")
        || lower.starts_with("on ")
        || lower.starts_with("at ")
        || lower.starts_with("under ")
        || lower.starts_with("inside ")
        || lower.starts_with("outside ")
        || lower.starts_with("near ")
        || lower.starts_with("behind ")
        || lower.starts_with("before ")
        || lower.starts_with("after ")
    {
        return text.to_string();
    }

    let first = text
        .trim_start()
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase());
    let article = match first {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    };
    format!("{} {}", article, text)
}

pub fn sentence_case_exit_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut chars = trimmed.chars();
    let first = chars.next().unwrap();
    let mut out = first.to_ascii_lowercase().to_string();
    out.push_str(chars.as_str());
    out
}

pub fn render_exit_sentence(exits: &[TextExit]) -> String {
    let parts: Vec<String> = exits
        .iter()
        .map(|exit| {
            let subject = if !exit.target_title.trim().is_empty() {
                sentence_case_exit_title(&exit.target_title)
            } else {
                sentence_case_exit_title(&exit.title)
            };
            format!("{} to the {}", subject, exit.direction)
        })
        .collect();

    match parts.len() {
        0 => String::new(),
        1 => format!("You see {}.", with_indefinite_article(&parts[0])),
        2 => format!(
            "You see {} and {}.",
            with_indefinite_article(&parts[0]),
            with_indefinite_article(&parts[1])
        ),
        _ => {
            let mut sentence = String::from("You see ");
            for (index, part) in parts.iter().enumerate() {
                let article_part = with_indefinite_article(part);
                if index == parts.len() - 1 {
                    sentence.push_str("and ");
                    sentence.push_str(&article_part);
                } else {
                    sentence.push_str(&article_part);
                    sentence.push_str(", ");
                }
            }
            sentence.push('.');
            sentence
        }
    }
}

pub fn render_presence_sentence(prefix: &str, names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!("{} {} here.", prefix, with_indefinite_article(&names[0])),
        2 => format!(
            "{} {} and {} here.",
            prefix,
            with_indefinite_article(&names[0]),
            with_indefinite_article(&names[1])
        ),
        _ => {
            let mut sentence = format!("{} ", prefix);
            for (index, part) in names.iter().enumerate() {
                let part = with_indefinite_article(part);
                if index == names.len() - 1 {
                    sentence.push_str("and ");
                    sentence.push_str(&part);
                } else {
                    sentence.push_str(&part);
                    sentence.push_str(", ");
                }
            }
            sentence.push_str(" here.");
            sentence
        }
    }
}

pub fn render_nearby_attackers_sentence(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!(
            "{} is attacking you from nearby.",
            with_indefinite_article(&names[0])
        ),
        2 => format!(
            "{} and {} are attacking you from nearby.",
            with_indefinite_article(&names[0]),
            with_indefinite_article(&names[1])
        ),
        _ => {
            let mut sentence = String::new();
            for (index, part) in names.iter().enumerate() {
                let part = with_indefinite_article(part);
                if index == names.len() - 1 {
                    sentence.push_str("and ");
                    sentence.push_str(&part);
                } else {
                    sentence.push_str(&part);
                    sentence.push_str(", ");
                }
            }
            sentence.push_str(" are attacking you from nearby.");
            sentence
        }
    }
}

pub fn render_nearby_attacker_appearance_sentence(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!("{} appears nearby.", names[0].trim()),
        2 => format!("{} and {} appear nearby.", names[0].trim(), names[1].trim()),
        _ => {
            let mut sentence = String::new();
            for (index, part) in names.iter().enumerate() {
                if index == names.len() - 1 {
                    sentence.push_str("and ");
                    sentence.push_str(part.trim());
                } else {
                    sentence.push_str(part.trim());
                    sentence.push_str(", ");
                }
            }
            sentence.push_str(" appear nearby.");
            sentence
        }
    }
}
