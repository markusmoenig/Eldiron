use crate::text_game as sg;
use rusterix::Map;
use std::collections::BTreeSet;

pub type TextGameMessage = (Option<u32>, Option<u32>, u32, String, String);
pub type TextGameSay = (Option<u32>, Option<u32>, String, String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextSessionOutput {
    RenderRoom,
    Plain(String),
    Message { text: String, category: String },
}

#[derive(Default, Clone)]
pub struct TextSession {
    initialized: bool,
    last_sector_id: Option<u32>,
    last_nearby_attackers: BTreeSet<String>,
    last_room_items: BTreeSet<String>,
    suppress_next_sector_render: Option<u32>,
    suppress_next_description_for_sector: Option<u32>,
    last_announced_hour: Option<u8>,
    auto_attack_target: Option<u32>,
}

impl TextSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn auto_attack_target(&self) -> Option<u32> {
        self.auto_attack_target
    }

    pub fn clear_auto_attack_target(&mut self) {
        self.auto_attack_target = None;
    }

    pub fn set_current_hour(&mut self, hour: Option<u8>) {
        self.last_announced_hour = hour;
    }

    pub fn startup(
        &mut self,
        map: &Map,
        authoring: &str,
        current_hour: Option<u8>,
    ) -> Vec<TextSessionOutput> {
        self.initialized = true;
        self.last_sector_id = sg::current_player_and_sector(map).map(|(_, sector)| sector.id);
        self.last_nearby_attackers = current_nearby_attackers(map, authoring);
        self.last_room_items = current_room_items(map, authoring);
        self.last_announced_hour = current_hour;

        match sg::authoring_startup_display(authoring) {
            sg::StartupDisplay::Description => sg::render_current_sector_description(map)
                .map(TextSessionOutput::Plain)
                .into_iter()
                .collect(),
            sg::StartupDisplay::Room => vec![TextSessionOutput::RenderRoom],
            sg::StartupDisplay::None => Vec::new(),
        }
    }

    pub fn after_movement(&mut self, map: &Map, authoring: &str) -> Vec<TextSessionOutput> {
        self.last_sector_id = sg::current_player_and_sector(map).map(|(_, sector)| sector.id);
        self.suppress_next_sector_render = self.last_sector_id;
        self.suppress_next_description_for_sector = self.last_sector_id;
        self.last_nearby_attackers = current_nearby_attackers(map, authoring);
        self.last_room_items = current_room_items(map, authoring);
        vec![TextSessionOutput::RenderRoom]
    }

    pub fn collect(
        &mut self,
        map: &Map,
        authoring: &str,
        mut messages: Vec<TextGameMessage>,
        mut says: Vec<TextGameSay>,
        current_hour: Option<u8>,
        current_hour_label: Option<String>,
        auto_attack_on_attack: bool,
    ) -> Vec<TextSessionOutput> {
        let mut output = Vec::new();
        let mut rendered_room_this_update = false;
        let current_sector_id = sg::current_player_and_sector(map).map(|(_, sector)| sector.id);

        if !self.initialized {
            output.extend(self.startup(map, authoring, current_hour));
            rendered_room_this_update = output
                .iter()
                .any(|entry| matches!(entry, TextSessionOutput::RenderRoom));
        } else if let Some(current_sector_id) = current_sector_id
            && Some(current_sector_id) != self.last_sector_id
        {
            self.last_sector_id = Some(current_sector_id);
            if self.suppress_next_sector_render == Some(current_sector_id) {
                self.suppress_next_sector_render = None;
            } else {
                output.push(TextSessionOutput::RenderRoom);
                rendered_room_this_update = true;
            }
            self.last_nearby_attackers = current_nearby_attackers(map, authoring);
        }

        let player_id = sg::current_player_and_sector(map).map(|(player, _)| player.id);
        let current_description = sg::render_current_sector_description(map);
        let mut saw_death = false;

        for (sender_entity, _sender_item, receiver_id, message, category) in messages.drain(..) {
            if saw_death {
                break;
            }
            if let Some(player_id) = player_id
                && receiver_id != player_id
            {
                continue;
            }
            if auto_attack_on_attack
                && is_under_attack_message(&message)
                && let (Some(sender_id), Some(player_id)) = (sender_entity, player_id)
                && sender_id != player_id
            {
                self.auto_attack_target = Some(sender_id);
            }
            if current_sector_id.is_some()
                && self.suppress_next_description_for_sector == current_sector_id
                && current_description
                    .as_deref()
                    .map(|text| text.trim() == message.trim())
                    .unwrap_or(false)
            {
                self.suppress_next_description_for_sector = None;
                continue;
            }
            if !should_print_text_message(&message, &category) {
                continue;
            }
            if current_description
                .as_deref()
                .map(|text| text.trim() == message.trim())
                .unwrap_or(false)
            {
                if rendered_room_this_update {
                    continue;
                }
                output.push(TextSessionOutput::RenderRoom);
                rendered_room_this_update = true;
                continue;
            }

            output.push(TextSessionOutput::Message {
                text: message.clone(),
                category: category.clone(),
            });
            if message.trim() == "You died. Try again!" {
                saw_death = true;
            }
        }

        for (_sender_entity, _sender_item, message, _category) in says.drain(..) {
            if saw_death {
                break;
            }
            output.push(TextSessionOutput::Plain(message));
        }

        if !saw_death && let (Some(hour), Some(label)) = (current_hour, current_hour_label) {
            let previous_hour = self.last_announced_hour.replace(hour);
            if previous_hour != Some(hour) {
                output.push(TextSessionOutput::Plain(format!("It is {}.", label)));
            }
        } else if let Some(hour) = current_hour {
            self.last_announced_hour = Some(hour);
        }

        let current_nearby_attackers = current_nearby_attackers(map, authoring);
        let current_room_items = current_room_items(map, authoring);
        if !rendered_room_this_update && !saw_death {
            let new_attackers: Vec<String> = current_nearby_attackers
                .difference(&self.last_nearby_attackers)
                .cloned()
                .collect();
            if !new_attackers.is_empty() {
                output.push(TextSessionOutput::Plain(
                    sg::render_nearby_attacker_appearance_sentence(&new_attackers),
                ));
            }

            let new_items: Vec<String> = current_room_items
                .difference(&self.last_room_items)
                .cloned()
                .collect();
            let already_announced_drop = output.iter().any(|entry| match entry {
                TextSessionOutput::Plain(text) => text.contains("falls to the floor"),
                TextSessionOutput::Message { text, .. } => text.contains("falls to the floor"),
                TextSessionOutput::RenderRoom => false,
            });
            if !already_announced_drop && !new_items.is_empty() {
                output.push(TextSessionOutput::Plain(if new_items.len() == 1 {
                    "Something falls to the floor.".to_string()
                } else {
                    "Several things fall to the floor.".to_string()
                }));
            }
        }

        if saw_death {
            self.auto_attack_target = None;
            self.last_nearby_attackers.clear();
            self.last_room_items.clear();
            self.suppress_next_description_for_sector = None;
        } else {
            self.last_nearby_attackers = current_nearby_attackers;
            self.last_room_items = current_room_items;
        }

        output
    }
}

fn current_nearby_attackers(map: &Map, authoring: &str) -> BTreeSet<String> {
    sg::build_text_room(map, authoring)
        .map(|room| room.nearby_attackers.into_iter().collect())
        .unwrap_or_default()
}

fn current_room_items(map: &Map, authoring: &str) -> BTreeSet<String> {
    sg::build_text_room(map, authoring)
        .map(|room| room.items.into_iter().collect())
        .unwrap_or_default()
}

fn should_print_text_message(message: &str, category: &str) -> bool {
    !(category == "warning" && message.trim() == "{system.cant_do_that_yet}")
}

fn is_under_attack_message(message: &str) -> bool {
    message.trim_start().starts_with("You are under attack by ")
}
