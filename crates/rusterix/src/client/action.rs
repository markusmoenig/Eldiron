use crate::{
    Assets, EntityAction, Value,
    client::command::{ClientCommandBinding, parse_client_command},
};
use rustc_hash::FxHashMap;
use std::str::FromStr;
use toml::Table;

pub struct ClientAction {
    class_name: String,
    input_map: FxHashMap<String, ClientCommandBinding>,
    forward_down: bool,
    backward_down: bool,
    left_down: bool,
    right_down: bool,
    strafe_left_down: bool,
    strafe_right_down: bool,
    last_cardinal_action: EntityAction,
}

impl Default for ClientAction {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientAction {
    pub fn new() -> Self {
        Self {
            class_name: String::new(),
            input_map: FxHashMap::default(),
            forward_down: false,
            backward_down: false,
            left_down: false,
            right_down: false,
            strafe_left_down: false,
            strafe_right_down: false,
            last_cardinal_action: EntityAction::Off,
        }
    }

    /// Init
    pub fn init(&mut self, class_name: String, assets: &Assets) {
        self.input_map.clear();
        self.forward_down = false;
        self.backward_down = false;
        self.left_down = false;
        self.right_down = false;
        self.strafe_left_down = false;
        self.strafe_right_down = false;
        self.last_cardinal_action = EntityAction::Off;
        self.class_name = class_name;

        if let Some((_, entity_data)) = assets.entities.get(&self.class_name) {
            self.input_map = Self::parse_input_map(entity_data);
        }
    }

    /// Execute the user event
    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        if let Value::Str(v) = value {
            let key = v.to_ascii_lowercase();
            if let Some(cmd) = self.input_map.get(&key).cloned() {
                if event == "key_down" {
                    return self.handle_key_down(cmd);
                }
                if event == "key_up" {
                    return self.handle_key_up(cmd);
                }
            }
        }

        if event == "key_up" {
            return self.current_movement_action();
        }

        EntityAction::Off
    }

    pub fn shortcut_labels_for_binding(&self, binding: &ClientCommandBinding) -> Vec<String> {
        let mut labels: Vec<String> = self
            .input_map
            .iter()
            .filter_map(|(key, mapped)| {
                Self::bindings_match(mapped, binding).then(|| Self::format_shortcut_key(key))
            })
            .collect();
        labels.sort();
        labels
    }

    fn bindings_match(a: &ClientCommandBinding, b: &ClientCommandBinding) -> bool {
        match (a, b) {
            (ClientCommandBinding::Control(a), ClientCommandBinding::Control(b)) => a == b,
            (ClientCommandBinding::Intent(a), ClientCommandBinding::Intent(b)) => {
                a.trim().eq_ignore_ascii_case(b.trim())
            }
            (ClientCommandBinding::RulesAction(a), ClientCommandBinding::RulesAction(b)) => {
                a.trim().eq_ignore_ascii_case(b.trim())
            }
            (ClientCommandBinding::Ui(a), ClientCommandBinding::Ui(b)) => {
                a.trim().eq_ignore_ascii_case(b.trim())
            }
            (ClientCommandBinding::Screen(a), ClientCommandBinding::Screen(b))
            | (ClientCommandBinding::Game(a), ClientCommandBinding::Game(b)) => {
                a.trim().eq_ignore_ascii_case(b.trim())
            }
            _ => false,
        }
    }

    fn format_shortcut_key(key: &str) -> String {
        match key.trim().to_ascii_lowercase().as_str() {
            " " | "space" => "Space".to_string(),
            "escape" | "esc" => "Esc".to_string(),
            "return" | "enter" => "Enter".to_string(),
            "tab" => "Tab".to_string(),
            "backspace" => "Backspace".to_string(),
            "up" | "arrowup" | "arrow_up" => "Up".to_string(),
            "down" | "arrowdown" | "arrow_down" => "Down".to_string(),
            "left" | "arrowleft" | "arrow_left" => "Left".to_string(),
            "right" | "arrowright" | "arrow_right" => "Right".to_string(),
            value if value.len() == 1 => value.to_ascii_uppercase(),
            value => value.replace('_', " "),
        }
    }

    fn parse_input_map(entity_data: &str) -> FxHashMap<String, ClientCommandBinding> {
        let mut map = FxHashMap::default();
        let Ok(table) = entity_data.parse::<Table>() else {
            return map;
        };

        let Some(input) = table.get("input").and_then(toml::Value::as_table) else {
            return map;
        };

        for (key, value) in input {
            if let Some(cmd) = value.as_str()
                && let Some(action) = Self::parse_input_command(cmd)
            {
                map.insert(key.to_ascii_lowercase(), action);
            }
        }

        map
    }

    fn parse_input_command(command: &str) -> Option<ClientCommandBinding> {
        let s = command.trim();
        if let Some(command) = parse_client_command(s) {
            return Some(command);
        }
        let Some(open) = s.find('(') else {
            return EntityAction::from_str(s)
                .ok()
                .map(ClientCommandBinding::Control);
        };
        let Some(close) = s.rfind(')') else {
            return None;
        };
        if close <= open {
            return None;
        }

        let func = s[..open].trim().to_ascii_lowercase();
        let arg = s[open + 1..close]
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();

        match func.as_str() {
            "command" => parse_client_command(&arg),
            "action" => EntityAction::from_str(&arg)
                .ok()
                .map(ClientCommandBinding::Control),
            "control" => EntityAction::from_str(&arg)
                .ok()
                .map(ClientCommandBinding::Control),
            "intent" => Some(ClientCommandBinding::Intent(arg)),
            "rules" | "rules_action" => {
                if arg.is_empty() {
                    None
                } else {
                    Some(ClientCommandBinding::RulesAction(arg))
                }
            }
            "spell" => {
                if arg.is_empty() {
                    None
                } else {
                    Some(ClientCommandBinding::Intent(format!("spell:{}", arg)))
                }
            }
            _ => None,
        }
    }

    fn handle_key_down(&mut self, cmd: ClientCommandBinding) -> EntityAction {
        match cmd {
            ClientCommandBinding::Intent(name) => EntityAction::Intent(name),
            ClientCommandBinding::RulesAction(action) => {
                EntityAction::Intent(format!("action:{}", action))
            }
            ClientCommandBinding::Control(action) => {
                self.set_movement_key(action, true);
                self.current_movement_action()
            }
            ClientCommandBinding::Screen(_) | ClientCommandBinding::Game(_) => EntityAction::Off,
            ClientCommandBinding::Ui(_) => EntityAction::Off,
        }
    }

    fn handle_key_up(&mut self, cmd: ClientCommandBinding) -> EntityAction {
        if let ClientCommandBinding::Control(action) = cmd {
            self.set_movement_key(action, false);
        }
        self.current_movement_action()
    }

    fn set_movement_key(&mut self, action: EntityAction, is_down: bool) {
        match action {
            EntityAction::Forward => {
                self.forward_down = is_down;
                self.update_last_cardinal_action(EntityAction::Forward, is_down);
            }
            EntityAction::Backward => {
                self.backward_down = is_down;
                self.update_last_cardinal_action(EntityAction::Backward, is_down);
            }
            EntityAction::Left => {
                self.left_down = is_down;
                self.update_last_cardinal_action(EntityAction::Left, is_down);
            }
            EntityAction::Right => {
                self.right_down = is_down;
                self.update_last_cardinal_action(EntityAction::Right, is_down);
            }
            EntityAction::StrafeLeft => self.strafe_left_down = is_down,
            EntityAction::StrafeRight => self.strafe_right_down = is_down,
            EntityAction::Off => {
                if is_down {
                    self.forward_down = false;
                    self.backward_down = false;
                    self.left_down = false;
                    self.right_down = false;
                    self.strafe_left_down = false;
                    self.strafe_right_down = false;
                    self.last_cardinal_action = EntityAction::Off;
                }
            }
            _ => {}
        }
    }

    fn update_last_cardinal_action(&mut self, action: EntityAction, is_down: bool) {
        if is_down {
            self.last_cardinal_action = action;
        } else if self.last_cardinal_action == action {
            self.last_cardinal_action = self.first_held_cardinal_action();
        }
    }

    fn first_held_cardinal_action(&self) -> EntityAction {
        if self.forward_down {
            EntityAction::Forward
        } else if self.backward_down {
            EntityAction::Backward
        } else if self.left_down {
            EntityAction::Left
        } else if self.right_down {
            EntityAction::Right
        } else {
            EntityAction::Off
        }
    }

    fn current_movement_action(&self) -> EntityAction {
        if self.last_cardinal_action != EntityAction::Off {
            self.last_cardinal_action.clone()
        } else {
            match (self.strafe_left_down, self.strafe_right_down) {
                (true, false) => EntityAction::StrafeLeft,
                (false, true) => EntityAction::StrafeRight,
                _ => EntityAction::Off,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(action: &mut ClientAction, entity_action: EntityAction) -> EntityAction {
        action.handle_key_down(ClientCommandBinding::Control(entity_action))
    }

    fn release(action: &mut ClientAction, entity_action: EntityAction) -> EntityAction {
        action.handle_key_up(ClientCommandBinding::Control(entity_action))
    }

    #[test]
    fn simultaneous_cardinal_keys_do_not_create_diagonal_actions() {
        let mut action = ClientAction::new();

        assert_eq!(
            press(&mut action, EntityAction::Forward),
            EntityAction::Forward
        );
        assert_eq!(press(&mut action, EntityAction::Right), EntityAction::Right);
        assert_eq!(
            release(&mut action, EntityAction::Right),
            EntityAction::Forward
        );

        assert_eq!(press(&mut action, EntityAction::Left), EntityAction::Left);
        assert_eq!(
            release(&mut action, EntityAction::Forward),
            EntityAction::Left
        );
        assert_eq!(release(&mut action, EntityAction::Left), EntityAction::Off);
    }

    #[test]
    fn input_commands_accept_namespaced_commands() {
        assert_eq!(
            ClientAction::parse_input_command("control.forward"),
            Some(ClientCommandBinding::Control(EntityAction::Forward))
        );
        assert_eq!(
            ClientAction::parse_input_command("command(rules.basic_attack)"),
            Some(ClientCommandBinding::RulesAction("basic_attack".into()))
        );
        assert_eq!(
            ClientAction::parse_input_command("command(intent.)"),
            Some(ClientCommandBinding::Intent(String::new()))
        );
        assert_eq!(
            ClientAction::parse_input_command("intent(attack)"),
            Some(ClientCommandBinding::Intent("attack".into()))
        );
        assert_eq!(
            ClientAction::parse_input_command("intent()"),
            Some(ClientCommandBinding::Intent(String::new()))
        );
        assert_eq!(
            ClientAction::parse_input_command("action(forward)"),
            Some(ClientCommandBinding::Control(EntityAction::Forward))
        );
    }

    #[test]
    fn shortcut_labels_match_command_bindings() {
        let mut action = ClientAction::new();
        action.input_map.insert(
            "t".into(),
            ClientCommandBinding::RulesAction("basic_attack".into()),
        );
        action
            .input_map
            .insert("space".into(), ClientCommandBinding::Intent(String::new()));

        assert_eq!(
            action.shortcut_labels_for_binding(&ClientCommandBinding::RulesAction(
                "BASIC_ATTACK".into()
            )),
            vec!["T".to_string()]
        );
        assert_eq!(
            action.shortcut_labels_for_binding(&ClientCommandBinding::Intent(String::new())),
            vec!["Space".to_string()]
        );
    }
}
