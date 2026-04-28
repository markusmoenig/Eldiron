use crate::{
    Assets, Choice, Entity, EntityAction, Map, MsgParser, Pixel, Rect, Tile,
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
    pub messages: Vec<(Uuid, String, Rect, Option<Choice>, Pixel, Option<u32>)>,
    pub draw2d: Draw2D,
    pub spacing: f32,
    pub message_spacing: f32,
    pub column_width: f32,
    pub table: toml::Table,
    pub top_down: bool,
    pub default_color: Pixel,
    pub clicked: Uuid,
    pub parser: MsgParser,
    pub resolver: MsgResolver,
    pub handle_messages: bool,
    pub handle_dialogs: bool,
    pub handle_multiple_choice: bool,
    pub handle_offer_inventory: bool,
    pub portrait: bool,
    pub portrait_size: f32,
    pub portrait_gap: f32,
}

impl Default for MessagesWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl MessagesWidget {
    const CHOICE_COLUMN_SEPARATOR: char = '\u{1f}';

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
            message_spacing: 8.0,
            column_width: 20.0,
            table: toml::Table::default(),
            top_down: false,
            default_color: [170, 170, 170, 255],
            clicked: Uuid::nil(),
            parser: MsgParser::new(),
            resolver: MsgResolver::default(),
            handle_messages: true,
            handle_dialogs: true,
            handle_multiple_choice: true,
            handle_offer_inventory: true,
            portrait: false,
            portrait_size: 64.0,
            portrait_gap: 12.0,
        }
    }

    fn accepts_message_category(&self, category: &str) -> bool {
        match category.trim().to_ascii_lowercase().as_str() {
            "dialog" => self.handle_dialogs,
            "multiple_choice" => self.handle_multiple_choice,
            "text_only" => false,
            _ => self.handle_messages,
        }
    }

    fn accepts_choice(&self, choice: &Choice) -> bool {
        match choice {
            Choice::ItemToSell(_, _, _, _, _) => self.handle_offer_inventory,
            Choice::ScriptChoice(_, _, _, _, _, _, _) => self.handle_multiple_choice,
            Choice::DialogChoice(_) => self.handle_dialogs,
            Choice::Cancel(_, _, _, _) => true,
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
                if let Some(value) = ui.get("message_spacing") {
                    if let Some(v) = value.as_float() {
                        self.message_spacing = v as f32;
                    } else if let Some(v) = value.as_integer() {
                        self.message_spacing = v as f32;
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
                if let Some(value) = ui.get("portrait").and_then(toml::Value::as_bool) {
                    self.portrait = value;
                }
                if let Some(value) = ui.get("portrait_size") {
                    if let Some(v) = value.as_float() {
                        self.portrait_size = (v as f32).max(1.0);
                    } else if let Some(v) = value.as_integer() {
                        self.portrait_size = (v as f32).max(1.0);
                    }
                }
                if let Some(value) = ui.get("portrait_gap") {
                    if let Some(v) = value.as_float() {
                        self.portrait_gap = (v as f32).max(0.0);
                    } else if let Some(v) = value.as_integer() {
                        self.portrait_gap = (v as f32).max(0.0);
                    }
                }
                if let Some(handles) = ui.get("handles").and_then(toml::Value::as_array) {
                    self.handle_messages = false;
                    self.handle_dialogs = false;
                    self.handle_multiple_choice = false;
                    self.handle_offer_inventory = false;
                    for handle in handles.iter().filter_map(toml::Value::as_str) {
                        match handle.trim().to_ascii_lowercase().as_str() {
                            "message" | "messages" => self.handle_messages = true,
                            "dialog" | "dialogs" => self.handle_dialogs = true,
                            "multiple_choice" | "multiple_choices" => {
                                self.handle_multiple_choice = true
                            }
                            "offer_inventory" | "inventory" | "inventory_offers" => {
                                self.handle_offer_inventory = true
                            }
                            "all" => {
                                self.handle_messages = true;
                                self.handle_dialogs = true;
                                self.handle_multiple_choice = true;
                                self.handle_offer_inventory = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
            self.table = table;
        }

        if let Some(font) = assets.fonts.get(&font_name) {
            self.font = Some(font.clone());
        }
    }

    fn portrait_tile_for_entity(entity: &Entity, assets: &Assets) -> Option<Tile> {
        if let Some(source) = entity.attributes.get_source("portrait_tile_id") {
            return source.tile_from_tile_list(assets);
        }
        if let Some(id) = entity.attributes.get_id("portrait_tile_id") {
            return assets.tiles.get(&id).cloned();
        }
        entity
            .attributes
            .get_str("portrait_tile_id")
            .and_then(|value| Uuid::parse_str(value.trim()).ok())
            .and_then(|id| assets.tiles.get(&id).cloned())
    }

    fn portrait_tile_for_sender(
        sender_entity: Option<u32>,
        map: &Map,
        assets: &Assets,
    ) -> Option<Tile> {
        let entity = map
            .entities
            .iter()
            .find(|entity| Some(entity.id) == sender_entity)?;
        Self::portrait_tile_for_entity(entity, assets)
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
        self.deactivate_inactive_choices(assets, map, time);

        // Append new messages
        for (sender_entity, sender_item, receiver_id, msg, category) in &messages {
            if !self.accepts_message_category(category) {
                continue;
            }
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
                *sender_entity,
            ));
        }

        let mut choice_map = FxHashMap::default();

        for choices in choices {
            let accepted_choices: Vec<_> = choices
                .choices
                .iter()
                .filter(|choice| self.accepts_choice(choice))
                .cloned()
                .collect();
            if accepted_choices.is_empty() {
                continue;
            }

            // Insert the cancel choice.
            choice_map.insert(
                '0',
                Choice::Cancel(
                    choices.from,
                    choices.to,
                    choices.expires_at_tick,
                    choices.max_distance,
                ),
            );

            let mut color = self.default_color;
            if let Some(ui) = self.table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("multiple_choice") {
                    if let Some(v) = value.as_str() {
                        color = self.hex_to_rgba_u8(v);
                    }
                }
            }

            for (index, choice) in accepted_choices.iter().enumerate() {
                let mut rendered_choice = choice.clone();
                let mut item_name: String = "".into();
                let mut item_price = 0;

                match choice {
                    Choice::ItemToSell(item_id, seller_id, _, _, _) => {
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
                    Choice::ScriptChoice(
                        label,
                        choice_attr,
                        from_id,
                        to_id,
                        choice_index,
                        expires_at_tick,
                        max_distance,
                    ) => {
                        item_name = self.resolver.resolve_with_context(
                            self.parser.parse(label),
                            map,
                            assets,
                            MessageContext {
                                sender_entity: Some(*from_id),
                                receiver_entity: Some(*to_id),
                                world_time: Some(*time),
                                ..Default::default()
                            },
                        );
                        rendered_choice = Choice::ScriptChoice(
                            item_name.clone(),
                            choice_attr.clone(),
                            *from_id,
                            *to_id,
                            *choice_index,
                            *expires_at_tick,
                            *max_distance,
                        );
                    }
                    Choice::DialogChoice(dialog_choice) => {
                        item_name = self.resolver.resolve_with_context(
                            self.parser.parse(&dialog_choice.label),
                            map,
                            assets,
                            MessageContext {
                                sender_entity: Some(dialog_choice.from),
                                receiver_entity: Some(dialog_choice.to),
                                world_time: Some(*time),
                                ..Default::default()
                            },
                        );
                        let mut next_choice = dialog_choice.clone();
                        next_choice.label = item_name.clone();
                        rendered_choice = Choice::DialogChoice(next_choice);
                    }
                    _ => {}
                }

                choice_map.insert((b'1' + index as u8) as char, rendered_choice.clone());

                let text = if matches!(choice, Choice::ItemToSell(_, _, _, _, _)) {
                    let label = format!("{}) {}", index + 1, item_name);
                    let price = format!("{}G", item_price);
                    format!("{}{}{}", label, Self::CHOICE_COLUMN_SEPARATOR, price)
                } else {
                    format!("{}) {}", index + 1, item_name)
                };

                self.messages.push((
                    Uuid::new_v4(),
                    text,
                    Rect::default(),
                    Some(rendered_choice),
                    color,
                    Some(choices.from),
                ));
            }
            self.messages.push((
                Uuid::new_v4(),
                self.resolve_msg("0) {system.exit_menu}", map, assets, time),
                Rect::default(),
                Some(Choice::Cancel(
                    choices.from,
                    choices.to,
                    choices.expires_at_tick,
                    choices.max_distance,
                )),
                color,
                Some(choices.from),
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
            let clip_rect = (
                self.rect.x.max(0.0) as isize,
                self.rect.y.max(0.0) as isize,
                self.rect.width.min(width as f32).max(0.0) as isize,
                self.rect.height.min(height as f32).max(0.0) as isize,
            );
            let mut y = if self.top_down {
                self.rect.y
            } else {
                self.rect.y + self.rect.height - self.font_size.ceil()
            };
            let draw2d = &self.draw2d;

            for (id, message, rect, _choice, color, sender_entity) in self.messages.iter_mut().rev()
            {
                let portrait_tile = if self.portrait {
                    Self::portrait_tile_for_sender(*sender_entity, map, assets)
                } else {
                    None
                };
                let portrait_width = if portrait_tile.is_some() {
                    self.portrait_size + self.portrait_gap
                } else {
                    0.0
                };
                let text_width = (self.rect.width - portrait_width).max(self.font_size);
                let lines =
                    Self::wrap_message_lines(draw2d, font, self.font_size, message, text_width);
                let line_height = self.font_size + self.spacing;
                let block_gap = self.message_spacing;
                let text_height = if lines.is_empty() {
                    self.font_size
                } else {
                    self.font_size + (lines.len().saturating_sub(1) as f32 * line_height)
                };
                let block_height = text_height.max(if portrait_tile.is_some() {
                    self.portrait_size
                } else {
                    0.0
                });

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

                    if let Some(tile) = &portrait_tile {
                        if let Some(texture) = tile.textures.first() {
                            let portrait_x = self.rect.x;
                            let portrait_y = y;
                            self.draw2d.blend_scale_chunk(
                                buffer.pixels_mut(),
                                &(
                                    portrait_x.max(0.0) as usize,
                                    portrait_y.max(0.0) as usize,
                                    self.portrait_size as usize,
                                    self.portrait_size as usize,
                                ),
                                stride,
                                &texture.data,
                                &(texture.width as usize, texture.height as usize),
                            );
                        }
                    }

                    for (index, line) in lines.iter().enumerate() {
                        let line_y = y + index as f32 * line_height;
                        if line_y > self.rect.y + self.rect.height {
                            break;
                        }

                        let tuple = (
                            (self.rect.x + portrait_width) as isize,
                            line_y.floor() as isize,
                            text_width as isize,
                            self.font_size as isize,
                        );

                        if let Some((left, right)) = line.split_once(Self::CHOICE_COLUMN_SEPARATOR)
                        {
                            self.draw2d.text_rect_blend_safe_clip(
                                buffer.pixels_mut(),
                                &tuple,
                                stride,
                                font,
                                self.font_size,
                                left,
                                &color,
                                draw2d::TheHorizontalAlign::Left,
                                draw2d::TheVerticalAlign::Center,
                                &clip_rect,
                            );
                            self.draw2d.text_rect_blend_safe_clip(
                                buffer.pixels_mut(),
                                &tuple,
                                stride,
                                font,
                                self.font_size,
                                right,
                                &color,
                                draw2d::TheHorizontalAlign::Right,
                                draw2d::TheVerticalAlign::Center,
                                &clip_rect,
                            );
                        } else {
                            self.draw2d.text_rect_blend_safe_clip(
                                buffer.pixels_mut(),
                                &tuple,
                                stride,
                                font,
                                self.font_size,
                                line,
                                &color,
                                draw2d::TheHorizontalAlign::Left,
                                draw2d::TheVerticalAlign::Center,
                                &clip_rect,
                            );
                        }
                    }

                    y += block_height + block_gap;
                } else {
                    let block_bottom = y + self.font_size;
                    let block_top = block_bottom - block_height;
                    let text_top = block_bottom - text_height;
                    // A single wrapped message can be taller than the widget. Keep drawing it
                    // while its bottom is visible so the latest/lower lines stay pinned.
                    if block_bottom < self.rect.y {
                        break;
                    }

                    *rect = Rect::new(self.rect.x, block_top, self.rect.width, block_height);

                    if let Some(tile) = &portrait_tile {
                        if let Some(texture) = tile.textures.first() {
                            let portrait_x = self.rect.x;
                            let portrait_y = block_top;
                            self.draw2d.blend_scale_chunk(
                                buffer.pixels_mut(),
                                &(
                                    portrait_x.max(0.0) as usize,
                                    portrait_y.max(0.0) as usize,
                                    self.portrait_size as usize,
                                    self.portrait_size as usize,
                                ),
                                stride,
                                &texture.data,
                                &(texture.width as usize, texture.height as usize),
                            );
                        }
                    }

                    for (index, line) in lines.iter().enumerate() {
                        let line_y = text_top + index as f32 * line_height;
                        if line_y + self.font_size < self.rect.y
                            || line_y > self.rect.y + self.rect.height
                        {
                            continue;
                        }
                        let tuple = (
                            (self.rect.x + portrait_width) as isize,
                            line_y.floor() as isize,
                            text_width as isize,
                            self.font_size as isize,
                        );

                        if let Some((left, right)) = line.split_once(Self::CHOICE_COLUMN_SEPARATOR)
                        {
                            self.draw2d.text_rect_blend_safe_clip(
                                buffer.pixels_mut(),
                                &tuple,
                                stride,
                                font,
                                self.font_size,
                                left,
                                &color,
                                draw2d::TheHorizontalAlign::Left,
                                draw2d::TheVerticalAlign::Center,
                                &clip_rect,
                            );
                            self.draw2d.text_rect_blend_safe_clip(
                                buffer.pixels_mut(),
                                &tuple,
                                stride,
                                font,
                                self.font_size,
                                right,
                                &color,
                                draw2d::TheHorizontalAlign::Right,
                                draw2d::TheVerticalAlign::Center,
                                &clip_rect,
                            );
                        } else {
                            self.draw2d.text_rect_blend_safe_clip(
                                buffer.pixels_mut(),
                                &tuple,
                                stride,
                                font,
                                self.font_size,
                                line,
                                &color,
                                draw2d::TheHorizontalAlign::Left,
                                draw2d::TheVerticalAlign::Center,
                                &clip_rect,
                            );
                        }
                    }

                    y = block_top - block_gap - self.font_size;
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
        for (id, _, rect, choice, _, _) in &self.messages {
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

    pub fn has_active_choices(&self) -> bool {
        self.messages
            .iter()
            .any(|(_, _, _, choice, _, _)| choice.is_some())
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

    fn deactivate_inactive_choices(&mut self, assets: &Assets, map: &Map, time: &TheTime) {
        let ticks_per_minute = assets
            .config
            .parse::<toml::Table>()
            .ok()
            .and_then(|config| {
                config
                    .get("game")
                    .and_then(toml::Value::as_table)
                    .and_then(|game| game.get("ticks_per_minute"))
                    .and_then(|value| {
                        value
                            .as_integer()
                            .or_else(|| value.as_float().map(|v| v as i64))
                    })
            })
            .unwrap_or(4)
            .max(1) as u32;
        let now_ticks = time.to_ticks(ticks_per_minute);

        for (_, _, _, choice, _, _) in &mut self.messages {
            let Some(active_choice) = choice.as_ref() else {
                continue;
            };

            let (from, to, expires_at_tick, max_distance) = active_choice.session_meta();
            let expired = now_ticks > expires_at_tick;
            let in_range = map
                .entities
                .iter()
                .find(|entity| entity.id == from)
                .zip(map.entities.iter().find(|entity| entity.id == to))
                .is_some_and(|(from_entity, to_entity)| {
                    from_entity.get_pos_xz().distance(to_entity.get_pos_xz()) <= max_distance
                });

            if expired || !in_range {
                *choice = None;
            }
        }
    }

    fn wrap_message_lines(
        draw2d: &Draw2D,
        font: &fontdue::Font,
        font_size: f32,
        message: &str,
        max_width: f32,
    ) -> Vec<String> {
        let max_width = (max_width - 1.0).max(font_size);
        let mut lines = Vec::new();

        for paragraph in message.split('\n') {
            if paragraph.contains(Self::CHOICE_COLUMN_SEPARATOR) {
                lines.push(paragraph.to_string());
                continue;
            }

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

                if Self::measure_text_width(draw2d, font, font_size, &candidate) <= max_width {
                    current = candidate;
                } else {
                    if !current.is_empty() {
                        lines.push(current);
                    }

                    if Self::measure_text_width(draw2d, font, font_size, word) <= max_width {
                        current = word.to_string();
                    } else {
                        let mut chunk = String::new();
                        for ch in word.chars() {
                            let candidate = format!("{}{}", chunk, ch);
                            if !chunk.is_empty()
                                && Self::measure_text_width(draw2d, font, font_size, &candidate)
                                    > max_width
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

    fn measure_text_width(
        draw2d: &Draw2D,
        font: &fontdue::Font,
        font_size: f32,
        text: &str,
    ) -> f32 {
        draw2d.get_text_size(font, font_size, text).0 as f32
    }
}
