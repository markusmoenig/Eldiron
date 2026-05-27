use crate::{Entity, EntityAction};
use theframework::prelude::*;

/// Messages to the Region
#[derive(Debug)]
pub enum Command {
    CreateEntity(Uuid, Entity),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientCommandBinding {
    Control(EntityAction),
    Intent(String),
    RulesAction(String),
    Ui(String),
}

impl ClientCommandBinding {
    pub fn intent_payload(&self) -> Option<String> {
        match self {
            Self::Intent(intent) => Some(intent.clone()),
            Self::RulesAction(action) => Some(format!("action:{}", action)),
            _ => None,
        }
    }
}

pub fn parse_client_command(command: &str) -> Option<ClientCommandBinding> {
    let command = command.trim();
    if command.is_empty() {
        return None;
    }

    if let Some(value) = command.strip_prefix("control.") {
        return value
            .trim()
            .parse::<EntityAction>()
            .ok()
            .map(ClientCommandBinding::Control);
    }
    if let Some(value) = command.strip_prefix("intent.") {
        let value = value.trim();
        return (!value.is_empty()).then(|| ClientCommandBinding::Intent(value.to_string()));
    }
    if let Some(value) = command.strip_prefix("rules.") {
        let value = value.trim();
        return (!value.is_empty()).then(|| ClientCommandBinding::RulesAction(value.to_string()));
    }
    if let Some(value) = command.strip_prefix("ui.") {
        let value = value.trim();
        return (!value.is_empty()).then(|| ClientCommandBinding::Ui(value.to_string()));
    }

    command
        .parse::<EntityAction>()
        .ok()
        .map(ClientCommandBinding::Control)
}

pub fn command_from_legacy_fields(
    command: Option<&str>,
    action: Option<&str>,
    intent: Option<&str>,
    spell: Option<&str>,
) -> Option<String> {
    if let Some(command) = command.map(str::trim).filter(|value| !value.is_empty()) {
        return Some(command.to_string());
    }

    if let Some(intent) = intent.map(str::trim).filter(|value| !value.is_empty()) {
        if intent.eq_ignore_ascii_case("spell")
            && let Some(spell) = spell.map(str::trim).filter(|value| !value.is_empty())
        {
            return Some(format!("intent.spell:{}", spell));
        }
        return Some(format!("intent.{}", intent));
    }

    if let Some(action) = action.map(str::trim).filter(|value| !value.is_empty()) {
        return Some(format!("control.{}", action));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_namespaced_client_commands() {
        assert_eq!(
            parse_client_command("control.forward"),
            Some(ClientCommandBinding::Control(EntityAction::Forward))
        );
        assert_eq!(
            parse_client_command("intent.attack"),
            Some(ClientCommandBinding::Intent("attack".into()))
        );
        assert_eq!(
            parse_client_command("rules.basic_attack"),
            Some(ClientCommandBinding::RulesAction("basic_attack".into()))
        );
        assert_eq!(
            parse_client_command("ui.inventory"),
            Some(ClientCommandBinding::Ui("inventory".into()))
        );
    }

    #[test]
    fn converts_legacy_button_fields_to_commands() {
        assert_eq!(
            command_from_legacy_fields(None, None, Some("attack"), None).as_deref(),
            Some("intent.attack")
        );
        assert_eq!(
            command_from_legacy_fields(None, Some("forward"), None, None).as_deref(),
            Some("control.forward")
        );
        assert_eq!(
            command_from_legacy_fields(None, None, Some("spell"), Some("minor_heal")).as_deref(),
            Some("intent.spell:minor_heal")
        );
        assert_eq!(
            command_from_legacy_fields(Some("rules.basic_attack"), Some("forward"), None, None)
                .as_deref(),
            Some("rules.basic_attack")
        );
    }
}
