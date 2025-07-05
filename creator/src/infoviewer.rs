use crate::prelude::*;
use rusterix::Value;
use shared::prelude::{Project, ServerContext};
use theframework::prelude::*;

pub struct InfoViewer {
    pub visible: bool,
    pub info_mode: i32,
}

#[allow(clippy::new_without_default)]
impl InfoViewer {
    pub fn new() -> Self {
        Self {
            visible: false,
            info_mode: 0,
        }
    }

    pub fn build(&self) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut textedit = TheTextAreaEdit::new(TheId::named("InfoView"));
        textedit.auto_scroll_to_cursor(false);
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("TOML");
            }
        }
        textedit.set_continuous(true);
        textedit.display_line_number(true);
        textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        textedit.readonly(true);
        center.set_widget(textedit);

        center
    }

    pub fn update(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if !self.visible {
            return;
        }

        let mut output = vec![];

        match server_ctx.cc {
            ContentContext::CharacterInstance(uuid) => {
                if let Some(map) = project.get_map(server_ctx) {
                    for entity in map.entities.iter() {
                        if entity.creator_id == uuid {
                            if self.info_mode == 0 {
                                if let Some(name) = entity.attributes.get_str("name") {
                                    output.push(format!("# {}, ({})", name, entity.id));
                                }
                                // Attributes
                                for key in entity.attributes.keys_sorted() {
                                    if key != "source" && key != "setup" && key != "name" {
                                        if let Some(value) = entity.attributes.get(key) {
                                            match value {
                                                Value::Str(text) => {
                                                    output.push(format!("{key} = \"{text}\""));
                                                }
                                                Value::Bool(value) => {
                                                    output.push(format!("{key} = {value}"));
                                                }
                                                Value::Float(value) => {
                                                    output.push(format!("{key} = {value}"));
                                                }
                                                Value::Int(value) => {
                                                    output.push(format!("{key} = {value}"));
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            } else if self.info_mode == 1 {
                                // Inventory
                                for (slot, item) in entity.iter_inventory() {
                                    if let Some(name) = item.attributes.get_str("name") {
                                        output.push(format!(
                                            "# Slot {}: {}, ({})",
                                            slot, name, item.id
                                        ));
                                    }

                                    // Attributes
                                    for key in item.attributes.keys_sorted() {
                                        if key != "source" && key != "setup" && key != "name" {
                                            if let Some(value) = item.attributes.get(key) {
                                                match value {
                                                    Value::Str(text) => {
                                                        output.push(format!("{key} = \"{text}\""));
                                                    }
                                                    Value::Bool(value) => {
                                                        output.push(format!("{key} = {value}"));
                                                    }
                                                    Value::Float(value) => {
                                                        output.push(format!("{key} = {value}"));
                                                    }
                                                    Value::Int(value) => {
                                                        output.push(format!("{key} = {value}",));
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if self.info_mode == 2 {
                                // Slots
                                for (slot, item) in entity.equipped.iter() {
                                    if let Some(name) = item.attributes.get_str("name") {
                                        output.push(format!("# {}: {} ({})", slot, name, item.id));
                                    }

                                    // Attributes
                                    for key in item.attributes.keys_sorted() {
                                        if key != "source" && key != "setup" && key != "name" {
                                            if let Some(value) = item.attributes.get(key) {
                                                match value {
                                                    Value::Str(text) => {
                                                        output.push(format!("{key} = \"{text}\"",));
                                                    }
                                                    Value::Bool(value) => {
                                                        output.push(format!("{key} = {value}"));
                                                    }
                                                    Value::Float(value) => {
                                                        output.push(format!("{key} = {value}",));
                                                    }
                                                    Value::Int(value) => {
                                                        output.push(format!("{key} = {value}",));
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            break;
                        }
                    }
                }
            }
            ContentContext::ItemInstance(uuid) => {
                if let Some(map) = project.get_map(server_ctx) {
                    for item in map.items.iter() {
                        if item.creator_id == uuid {
                            if let Some(name) = item.attributes.get_str("name") {
                                output.push(format!("# {}, ({})", name, item.id));
                            }
                            for key in item.attributes.keys_sorted() {
                                if key != "source" && key != "setup" && key != "name" {
                                    if let Some(value) = item.attributes.get(key) {
                                        match value {
                                            Value::Str(text) => {
                                                output.push(format!("{key} = \"{text}\""));
                                            }
                                            Value::Bool(value) => {
                                                output.push(format!("{key} = {value}"));
                                            }
                                            Value::Float(value) => {
                                                output.push(format!("{key} = {value}"));
                                            }
                                            Value::Int(value) => {
                                                output.push(format!("{key} = {value}"));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }

        ui.set_widget_value("InfoView", ctx, TheValue::Text(output.join("\n")));
    }
}
