//! Shared command slots, UI placeholders, party targeting and button policies.
use super::*;
use crate::Entity;
use crate::client::widget::ButtonVisualState;
use crate::client::{
    command::{ClientCommandBinding, parse_client_command},
    rules_ui::CommandState,
};

pub fn deactivate_matches(widget: &Widget, token: &str) -> bool {
    let t = token.trim();
    if t.is_empty() {
        return false;
    }
    if widget.name.eq_ignore_ascii_case(t) {
        return true;
    }
    if let Some(group) = &widget.group {
        return group.trim().eq_ignore_ascii_case(t);
    }
    false
}

pub fn resolved_widget_command(
    widget: &Widget,
    assets: &Assets,
    entity: Option<&Entity>,
    ui_state: &FxHashMap<String, String>,
) -> Option<String> {
    if let Some(slot) = widget.command_slot.as_deref() {
        return command_for_slot(slot, assets, entity, ui_state);
    }
    widget
        .command
        .clone()
        .or_else(|| {
            widget
                .command_binding()
                .map(|binding| binding.command_string())
        })
        .map(|command| resolve_ui_placeholders(&command, ui_state))
}

pub fn resolve_ui_placeholders(input: &str, ui_state: &FxHashMap<String, String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find("{UI.") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let key = after[..end].trim();
        out.push_str(ui_state.get(key).map(String::as_str).unwrap_or_default());
        rest = &after[end + 1..];
    }

    out.push_str(rest);
    out
}

pub fn resolved_widget_binding(
    widget: &Widget,
    assets: &Assets,
    entity: Option<&Entity>,
    ui_state: &FxHashMap<String, String>,
) -> Option<ClientCommandBinding> {
    resolved_widget_command(widget, assets, entity, ui_state)
        .as_deref()
        .and_then(parse_client_command)
}

pub fn resolved_widget_intent_payload(
    widget: &Widget,
    assets: &Assets,
    entity: Option<&Entity>,
    ui_state: &FxHashMap<String, String>,
) -> Option<String> {
    resolved_widget_binding(widget, assets, entity, ui_state)
        .and_then(|binding| binding.intent_payload())
}

pub fn contextual_open_close_command(
    command: Option<String>,
    hovered: Option<u32>,
    map: &Map,
) -> Option<String> {
    if command.as_deref() == Some("intent.open_close")
        && let Some(item) = hovered.and_then(|id| map.items.iter().find(|i| i.id == id))
        && item.attributes.get_bool_default("openable", false)
    {
        return Some(format!(
            "intent.{}",
            crate::item_state::canonical_intent(item, "open_close")
        ));
    }
    command
}

pub fn command_for_slot(
    slot: &str,
    assets: &Assets,
    entity: Option<&Entity>,
    ui_state: &FxHashMap<String, String>,
) -> Option<String> {
    let slot = slot.trim();
    if slot.is_empty() {
        return None;
    }

    let suffix = slot_attr_suffix(slot);
    if let Some(command) = entity
        .and_then(|entity| {
            entity
                .attributes
                .get_str(&format!("command_slot_{}", suffix))
                .or_else(|| {
                    entity
                        .attributes
                        .get_str(&format!("action_slot_{}", suffix))
                })
        })
        .map(str::trim)
        .filter(|command| !command.is_empty())
    {
        return normalize_slot_command(command);
    }

    let (group, index) = split_command_slot(slot)?;
    let rules = assets.rules_table()?;
    let class = entity
        .and_then(|entity| {
            entity
                .get_attr_string("class")
                .or_else(|| entity.get_attr_string("class_name"))
        })
        .or_else(|| ui_state.get("start.class").cloned())
        .or_else(|| {
            eldiron_ruleset::resolve_identity_defaults(&rules)
                .ok()
                .and_then(|identity| identity.class)
        })?;

    let command = rules
        .get("classes")?
        .as_table()?
        .get(class.trim())?
        .as_table()?
        .get("action_bar")?
        .as_table()?
        .get(group)?
        .as_array()?
        .get(index)?
        .as_str()?;
    normalize_slot_command(command)
}

pub fn split_command_slot(slot: &str) -> Option<(&str, usize)> {
    let (group, index) = slot.rsplit_once('.')?;
    let group = group.trim();
    let index = index.trim().parse::<usize>().ok()?;
    (!group.is_empty()).then_some((group, index))
}

pub fn slot_attr_suffix(slot: &str) -> String {
    let mut suffix = String::new();
    for ch in slot.chars() {
        if ch.is_ascii_alphanumeric() {
            suffix.push(ch.to_ascii_lowercase());
        } else {
            suffix.push('_');
        }
    }
    while suffix.contains("__") {
        suffix = suffix.replace("__", "_");
    }
    suffix.trim_matches('_').to_string()
}

pub fn normalize_slot_command(command: &str) -> Option<String> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if parse_client_command(command).is_some() {
        Some(command.to_string())
    } else {
        Some(format!("rules.{}", command))
    }
}

pub fn button_visual_state(
    hovered: bool,
    selected: bool,
    pressed: bool,
    command_state: Option<&CommandState>,
) -> ButtonVisualState {
    if command_state.is_some_and(|state| !state.enabled) {
        return ButtonVisualState::Disabled;
    }

    if pressed {
        return ButtonVisualState::Pressed;
    }

    if selected {
        return ButtonVisualState::Selected;
    }

    if hovered {
        return ButtonVisualState::Hover;
    }

    ButtonVisualState::Normal
}

pub fn command_is_walk(command: Option<&str>) -> bool {
    matches!(
        command.and_then(parse_client_command),
        Some(ClientCommandBinding::Intent(intent)) if intent.trim().is_empty()
    )
}

pub fn widget_name_is_hidden(name: &str, hidden: &[String]) -> bool {
    hidden.iter().any(|pattern| {
        pattern
            .strip_suffix('*')
            .map(|prefix| name.starts_with(prefix))
            .unwrap_or_else(|| name == pattern)
    })
}

pub use crate::party::{party_members, resolve_party_entity};
