use crate::{
    Assets, Choice, EntityAction, Map, MsgParser, Pixel, Rect,
    client::{
        draw2d,
        resolver::{MessageContext, MsgResolver},
    },
};
use draw2d::Draw2D;
use theframework::prelude::*;

pub struct MessagesWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub font: Option<fontdue::Font>,
    pub font_size: f32,
    pub messages: Vec<(Uuid, String, Rect, Option<Choice>, Pixel)>,
    pub draw2d: Draw2D,
    pub spacing: f32,
    pub column_width: f32,
    pub table: toml::Table,
    pub top_down: bool,
    pub default_color: Pixel,
    pub clicked: Uuid,
    pub parser: MsgParser,
    pub resolver: MsgResolver,
}

impl Default for MessagesWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl MessagesWidget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            font: None,
            font_size: 20.0,
            messages: vec![],
            draw2d: Draw2D::default(),
            spacing: 1.0,
            column_width: 20.0,
            table: toml::Table::default(),
            top_down: false,
            default_color: [170, 170, 170, 255],
            clicked: Uuid::nil(),
            parser: MsgParser::new(),
            resolver: MsgResolver::default(),
        }
    }

    pub fn init(&mut self, assets: &Assets) {
        let mut font_name = String::new();
        if let Ok(config) = assets.config.parse::<toml::Table>() {
            if let Some(locale) = config
                .get("game")
                .and_then(toml::Value::as_table)
                .and_then(|game| game.get("locale"))
                .and_then(toml::Value::as_str)
            {
                self.resolver.set_locale(locale);
            }
        }
        if let Ok(table) = self.toml_str.parse::<toml::Table>() {
            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("font") {
                    if let Some(v) = value.as_str() {
                        font_name = v.into();
                    }
                }
                if let Some(value) = ui.get("font_size") {
                    if let Some(v) = value.as_float() {
                        self.font_size = v as f32;
                    } else if let Some(v) = value.as_integer() {
                        self.font_size = v as f32;
                    }
                }
                if let Some(value) = ui.get("spacing") {
                    if let Some(v) = value.as_float() {
                        self.spacing = v as f32;
                    } else if let Some(v) = value.as_integer() {
                        self.spacing = v as f32;
                    }
                }
                if let Some(value) = ui.get("column_width") {
                    if let Some(v) = value.as_float() {
                        self.column_width = v as f32;
                    } else if let Some(v) = value.as_integer() {
                        self.column_width = v as f32;
                    }
                }
                if let Some(value) = ui.get("top_down") {
                    if let Some(v) = value.as_bool() {
                        self.top_down = v;
                    }
                }
                if let Some(value) = ui.get("default") {
                    if let Some(v) = value.as_str() {
                        self.default_color = self.hex_to_rgba_u8(v);
                    }
                }
            }
            self.table = table;
        }

        if let Some(font) = assets.fonts.get(&font_name) {
            self.font = Some(font.clone());
        }
    }

    /// Process the incoming messages
    pub fn process_messages(
        &mut self,
        assets: &Assets,
        map: &Map,
        time: &TheTime,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
    ) -> Option<FxHashMap<char, Choice>> {
        // Append new messages
        for (sender_entity, sender_item, receiver_id, msg, category) in &messages {
            let mut color = self.default_color;
            if let Some(ui) = self.table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get(category) {
                    if let Some(v) = value.as_str() {
                        color = self.hex_to_rgba_u8(v);
                    }
                }
            }

            let message = self.resolver.resolve_with_context(
                self.parser.parse(msg),
                map,
                assets,
                MessageContext {
                    sender_entity: *sender_entity,
                    sender_item: *sender_item,
                    receiver_entity: Some(*receiver_id),
                    world_time: Some(*time),
                },
            );
            self.messages.push((
                Uuid::new_v4(),
                message.clone(),
                Rect::default(),
                None,
                color,
            ));
        }

        let mut choice_map = FxHashMap::default();

        let column_width = self.column_width as i32;
        for choices in choices {
            // Insert the cancel choice.
            choice_map.insert('0', Choice::Cancel(choices.from, choices.to));

            let mut color = self.default_color;
            if let Some(ui) = self.table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("multiple_choice") {
                    if let Some(v) = value.as_str() {
                        color = self.hex_to_rgba_u8(v);
                    }
                }
            }

            for (index, choice) in choices.choices.iter().enumerate() {
                let mut item_name: String = "".into();
                let mut item_price = 0;

                choice_map.insert((b'1' + index as u8) as char, choice.clone());

                match choice {
                    Choice::ItemToSell(item_id, seller_id, _) => {
                        for entity in map.entities.iter() {
                            if entity.id == *seller_id {
                                for item in entity.inventory.iter() {
                                    if let Some(item) = item {
                                        if item.id == *item_id {
                                            item_name = item
                                                .get_attr_string("name")
                                                .unwrap_or("".to_string());
                                            item_price =
                                                item.attributes.get_int_default("worth", 0) as i64;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                // Pad item_name to fixed width, align left
                let padded_name = format!("{:<width$}", item_name, width = column_width as usize);
                let text = format!("{}) {} {}G", index + 1, padded_name, item_price);

                self.messages.push((
                    Uuid::new_v4(),
                    text,
                    Rect::default(),
                    Some(choice.clone()),
                    color,
                ));
            }
            self.messages.push((
                Uuid::new_v4(),
                self.resolve_msg("0) {system.exit_menu}", map, assets, time),
                Rect::default(),
                Some(Choice::Cancel(choices.from, choices.to)),
                color,
            ));
        }

        // Purge the messages which are scrolled out of scope
        let max_messages = 100;
        if self.messages.len() > max_messages {
            let excess = self.messages.len() - max_messages;
            self.messages.drain(0..excess);
        }

        if choice_map.is_empty() {
            None
        } else {
            Some(choice_map)
        }
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        assets: &Assets,
        map: &Map,
        time: &TheTime,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
    ) -> Option<FxHashMap<char, Choice>> {
        let width = buffer.dim().width;
        let height = buffer.dim().height;

        fn darken(color: [u8; 4], amount: u8) -> [u8; 4] {
            [
                color[0].saturating_sub(amount),
                color[1].saturating_sub(amount),
                color[2].saturating_sub(amount),
                color[3],
            ]
        }

        let choice_map = self.process_messages(assets, map, time, messages, choices);

        // Draw bottom up
        if let Some(font) = &self.font {
            let stride = buffer.stride();
            let mut y = if self.top_down {
                self.rect.y
            } else {
                self.rect.y + self.rect.height - self.font_size.ceil()
            };

            for (id, message, rect, _choice, color) in self.messages.iter_mut().rev() {
                let lines =
                    Self::wrap_message_lines(font, self.font_size, message, self.rect.width);
                let line_height = self.font_size + self.spacing;
                let block_height = if lines.is_empty() {
                    self.font_size
                } else {
                    self.font_size + (lines.len().saturating_sub(1) as f32 * line_height)
                };

                let color = if *id == self.clicked {
                    darken(*color, 100)
                } else {
                    *color
                };

                if self.top_down {
                    if y > self.rect.y + self.rect.height {
                        break;
                    }

                    *rect = Rect::new(self.rect.x, y, self.rect.width, block_height);

                    for (index, line) in lines.iter().enumerate() {
                        let line_y = y + index as f32 * line_height;
                        if line_y > self.rect.y + self.rect.height {
                            break;
                        }

                        let tuple = (
                            self.rect.x as isize,
                            line_y.floor() as isize,
                            self.rect.width as isize,
                            self.font_size as isize,
                        );

                        self.draw2d.text_rect_blend_safe(
                            buffer.pixels_mut(),
                            &tuple,
                            stride,
                            font,
                            self.font_size,
                            line,
                            &color,
                            draw2d::TheHorizontalAlign::Left,
                            draw2d::TheVerticalAlign::Center,
                            &(0, 0, width as isize, height as isize),
                        );
                    }

                    y += block_height + self.spacing;
                } else {
                    let block_top = y - (lines.len().saturating_sub(1) as f32 * line_height);
                    if block_top + self.font_size < self.rect.y {
                        break;
                    }

                    *rect = Rect::new(self.rect.x, block_top, self.rect.width, block_height);

                    for (index, line) in lines.iter().enumerate() {
                        let line_y = block_top + index as f32 * line_height;
                        let tuple = (
                            self.rect.x as isize,
                            line_y.floor() as isize,
                            self.rect.width as isize,
                            self.font_size as isize,
                        );

                        self.draw2d.text_rect_blend_safe(
                            buffer.pixels_mut(),
                            &tuple,
                            stride,
                            font,
                            self.font_size,
                            line,
                            &color,
                            draw2d::TheHorizontalAlign::Left,
                            draw2d::TheVerticalAlign::Center,
                            &(0, 0, width as isize, height as isize),
                        );
                    }

                    y = block_top - line_height;
                }
            }
        }

        choice_map
    }

    /// Converts a hex color string to a [u8; 4] (RGBA).
    /// Accepts "#RRGGBB" or "#RRGGBBAA" formats.
    fn hex_to_rgba_u8(&self, hex: &str) -> [u8; 4] {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            6 => match (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                (Ok(r), Ok(g), Ok(b)) => [r, g, b, 255],
                _ => [255, 255, 255, 255],
            },
            8 => match (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
                u8::from_str_radix(&hex[6..8], 16),
            ) {
                (Ok(r), Ok(g), Ok(b), Ok(a)) => [r, g, b, a],
                _ => [255, 255, 255, 255],
            },
            _ => [255, 255, 255, 255],
        }
    }

    pub fn touch_down(&mut self, coord: Vec2<i32>) -> Option<EntityAction> {
        for (id, _, rect, choice, _) in &self.messages {
            if rect.contains(Vec2::new(coord.x as f32, coord.y as f32)) {
                if let Some(choice) = choice {
                    self.clicked = id.clone();
                    return Some(EntityAction::Choice(choice.clone()));
                }
            }
        }
        None
    }

    pub fn touch_up(&mut self) {
        self.clicked = Uuid::nil();
    }

    /// Resolves a message
    fn resolve_msg(&self, msg: &str, map: &Map, assets: &Assets, time: &TheTime) -> String {
        self.resolver.resolve_with_context(
            self.parser.parse(msg),
            map,
            assets,
            MessageContext {
                world_time: Some(*time),
                ..Default::default()
            },
        )
    }

    fn wrap_message_lines(
        font: &fontdue::Font,
        font_size: f32,
        message: &str,
        max_width: f32,
    ) -> Vec<String> {
        let max_width = max_width.max(font_size);
        let mut lines = Vec::new();

        for paragraph in message.split('\n') {
            if paragraph.trim().is_empty() {
                lines.push(String::new());
                continue;
            }

            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current, word)
                };

                if Self::measure_text_width(font, font_size, &candidate) <= max_width {
                    current = candidate;
                } else {
                    if !current.is_empty() {
                        lines.push(current);
                    }

                    if Self::measure_text_width(font, font_size, word) <= max_width {
                        current = word.to_string();
                    } else {
                        let mut chunk = String::new();
                        for ch in word.chars() {
                            let candidate = format!("{}{}", chunk, ch);
                            if !chunk.is_empty()
                                && Self::measure_text_width(font, font_size, &candidate) > max_width
                            {
                                lines.push(chunk);
                                chunk = ch.to_string();
                            } else {
                                chunk = candidate;
                            }
                        }
                        current = chunk;
                    }
                }
            }

            lines.push(current);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        while lines.len() > 1 && lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        lines
    }

    fn measure_text_width(font: &fontdue::Font, font_size: f32, text: &str) -> f32 {
        text.chars()
            .map(|ch| font.metrics(ch, font_size).advance_width)
            .sum()
    }
}
