use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextCommand {
    Empty,
    Look(Option<String>),
    Inventory,
    Stats,
    Move(String),
    Go(String),
    Intent {
        intent: String,
        target: Option<String>,
    },
    Cast {
        spell: String,
        target: Option<String>,
    },
    Craft {
        recipe: String,
    },
    Action {
        action: String,
        target: Option<String>,
    },
    Unknown,
}

pub fn normalize_ruleset_id(input: &str) -> String {
    let mut out = String::new();
    let mut last_was_separator = false;
    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.is_empty() {
            if !last_was_separator {
                out.push('_');
                last_was_separator = true;
            }
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn ruleset_id_match_len(payload: &str, ruleset_id: &str) -> Option<usize> {
    let payload = payload.trim_start();
    let mut consumed = 0usize;
    let words = ruleset_id
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }

    for (index, word) in words.iter().enumerate() {
        let rest = payload.get(consumed..)?.trim_start();
        consumed = payload.len() - rest.len();
        if !rest
            .get(..word.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(word))
        {
            return None;
        }
        consumed += word.len();
        if index + 1 < words.len() {
            let rest = payload.get(consumed..)?;
            let next = rest.chars().next()?;
            if !(next.is_whitespace() || next == '_' || next == '-') {
                return None;
            }
        }
    }

    let rest = payload.get(consumed..).unwrap_or_default();
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(consumed)
}

pub fn split_ruleset_id_and_target(
    payload: &str,
    known_ids: &BTreeSet<String>,
) -> (String, Option<String>) {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }

    let mut best: Option<(&str, usize)> = None;
    for id in known_ids {
        if let Some(len) = ruleset_id_match_len(trimmed, id)
            && best.is_none_or(|(_, best_len)| len > best_len)
        {
            best = Some((id.as_str(), len));
        }
    }

    if let Some((id, len)) = best {
        let target = trimmed
            .get(len..)
            .unwrap_or_default()
            .trim()
            .trim_start_matches("at ")
            .trim_start_matches("on ")
            .trim()
            .to_string();
        return (
            id.to_string(),
            if target.is_empty() {
                None
            } else {
                Some(target)
            },
        );
    }

    (normalize_ruleset_id(trimmed), None)
}

pub fn parse_text_command(
    input: &str,
    supported_intents: &BTreeSet<String>,
    known_spells: &BTreeSet<String>,
    known_actions: &BTreeSet<String>,
    known_recipes: &BTreeSet<String>,
) -> TextCommand {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return TextCommand::Empty;
    }

    let lower = trimmed.to_ascii_lowercase();
    let direction = match lower.as_str() {
        "n" => Some("north"),
        "e" => Some("east"),
        "s" => Some("south"),
        "w" => Some("west"),
        "north" | "south" | "east" | "west" => Some(lower.as_str()),
        _ => None,
    };
    if let Some(direction) = direction {
        return TextCommand::Move(direction.to_string());
    }

    match lower.as_str() {
        "look" | "l" => return TextCommand::Look(None),
        "inventory" | "inv" => return TextCommand::Inventory,
        "stats" | "stat" => return TextCommand::Stats,
        "help" => return TextCommand::Unknown,
        _ => {}
    }

    if let Some(target) = trimmed
        .strip_prefix("go ")
        .or_else(|| trimmed.strip_prefix("Go "))
    {
        let target = target.trim();
        return if target.is_empty() {
            TextCommand::Unknown
        } else {
            TextCommand::Go(target.to_string())
        };
    }

    if lower.starts_with("cast ") {
        let original_payload = trimmed.get("cast ".len()..).unwrap_or("").trim_start();
        let (spell, target) = split_ruleset_id_and_target(original_payload, known_spells);
        return if spell.is_empty() {
            TextCommand::Unknown
        } else {
            TextCommand::Cast { spell, target }
        };
    }

    if lower.starts_with("craft ") {
        let original_payload = trimmed.get("craft ".len()..).unwrap_or("").trim_start();
        let (recipe, _) = split_ruleset_id_and_target(original_payload, known_recipes);
        return if recipe.is_empty() {
            TextCommand::Unknown
        } else {
            TextCommand::Craft { recipe }
        };
    }

    if lower.starts_with("action ") {
        let original_payload = trimmed.get("action ".len()..).unwrap_or("").trim_start();
        let (action, target) = split_ruleset_id_and_target(original_payload, known_actions);
        return if action.is_empty() {
            TextCommand::Unknown
        } else {
            TextCommand::Action { action, target }
        };
    }

    if lower.starts_with("use action ") {
        let original_payload = trimmed
            .get("use action ".len()..)
            .unwrap_or("")
            .trim_start();
        let (action, target) = split_ruleset_id_and_target(original_payload, known_actions);
        return if action.is_empty() {
            TextCommand::Unknown
        } else {
            TextCommand::Action { action, target }
        };
    }

    if let Some(payload) = lower.strip_prefix("use ") {
        let original_payload = &trimmed[trimmed.len() - payload.len()..];
        let (action, target) = split_ruleset_id_and_target(original_payload, known_actions);
        if !action.is_empty() {
            return TextCommand::Action { action, target };
        }
    }

    let (action, target) = split_ruleset_id_and_target(trimmed, known_actions);
    if known_actions.contains(&action) {
        return TextCommand::Action { action, target };
    }

    if let Some(payload) = trimmed.strip_prefix("intent ") {
        let payload = payload.trim();
        let mut parts = payload.splitn(2, char::is_whitespace);
        let intent = parts.next().unwrap_or("").trim();
        let target = parts
            .next()
            .map(str::trim)
            .filter(|target| !target.is_empty());
        return if intent.is_empty() {
            TextCommand::Unknown
        } else {
            TextCommand::Intent {
                intent: intent.to_ascii_lowercase(),
                target: target.map(str::to_string),
            }
        };
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let verb = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let target = parts
        .next()
        .map(str::trim)
        .filter(|target| !target.is_empty());
    if !verb.is_empty()
        && target.is_some()
        && (supported_intents.contains(&verb)
            || matches!(
                verb.as_str(),
                "attack" | "take" | "get" | "pickup" | "pick" | "look" | "use" | "drop" | "talk"
            ))
    {
        let intent = match verb.as_str() {
            "get" | "pickup" | "pick" => "take".to_string(),
            _ => verb,
        };
        return TextCommand::Intent {
            intent,
            target: target.map(str::to_string),
        };
    }

    TextCommand::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spells(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn actions(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_known_spell_with_spaced_name_and_target() {
        assert_eq!(
            parse_text_command(
                "cast minor heal orc",
                &BTreeSet::new(),
                &spells(&["minor_heal"]),
                &BTreeSet::new(),
                &BTreeSet::new(),
            ),
            TextCommand::Cast {
                spell: "minor_heal".into(),
                target: Some("orc".into()),
            }
        );
    }

    #[test]
    fn parses_known_spell_without_target() {
        assert_eq!(
            parse_text_command(
                "cast minor heal",
                &BTreeSet::new(),
                &spells(&["minor_heal"]),
                &BTreeSet::new(),
                &BTreeSet::new(),
            ),
            TextCommand::Cast {
                spell: "minor_heal".into(),
                target: None,
            }
        );
    }

    #[test]
    fn normalizes_unknown_spell_payload() {
        assert_eq!(
            parse_text_command(
                "cast holy light",
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::new(),
            ),
            TextCommand::Cast {
                spell: "holy_light".into(),
                target: None,
            }
        );
    }

    #[test]
    fn parses_known_action_with_spaced_name_and_target() {
        assert_eq!(
            parse_text_command(
                "use power strike orc",
                &BTreeSet::new(),
                &BTreeSet::new(),
                &actions(&["power_strike"]),
                &BTreeSet::new(),
            ),
            TextCommand::Action {
                action: "power_strike".into(),
                target: Some("orc".into()),
            }
        );
    }

    #[test]
    fn parses_known_action_without_use_prefix() {
        assert_eq!(
            parse_text_command(
                "gather herbs",
                &BTreeSet::new(),
                &BTreeSet::new(),
                &actions(&["gather_herbs"]),
                &BTreeSet::new(),
            ),
            TextCommand::Action {
                action: "gather_herbs".into(),
                target: None,
            }
        );
    }

    #[test]
    fn parses_known_recipe_with_spaced_name() {
        assert_eq!(
            parse_text_command(
                "craft blessed herb",
                &BTreeSet::new(),
                &BTreeSet::new(),
                &BTreeSet::new(),
                &actions(&["blessed_herb"]),
            ),
            TextCommand::Craft {
                recipe: "blessed_herb".into(),
            }
        );
    }
}
