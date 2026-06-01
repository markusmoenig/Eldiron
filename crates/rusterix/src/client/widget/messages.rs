use crate::{
    Assets, Choice, Currencies, Entity, EntityAction, Map, MsgParser, Pixel, Rect, Tile, Value,
    client::{
        draw2d,
        resolver::{MessageContext, MsgResolver},
    },
};
use draw2d::Draw2D;
use instant::{Duration, Instant};
use std::collections::VecDeque;
use theframework::prelude::*;

#[derive(Clone)]
pub struct MessageLine {
    pub id: Uuid,
    pub text: String,
    pub rect: Rect,
    pub choice: Option<Choice>,
    pub choice_key: Option<char>,
    pub color: Pixel,
    pub sender_entity: Option<u32>,
}

#[derive(Clone)]
pub(crate) enum PendingMessage {
    Line(MessageLine),
    Continue,
    Timer(Duration),
}

pub struct MessagesWidget {
    pub name: String,
    pub rect: Rect,
    pub toml_str: String,
    pub buffer: TheRGBABuffer,
    pub font: Option<fontdue::Font>,
    pub font_size: f32,
    pub messages: Vec<MessageLine>,
    pub(crate) pending_messages: VecDeque<PendingMessage>,
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
    pub press_to_continue: bool,
    pub pause_blocks_input: bool,
    pub continue_prompt: String,
    pub continue_prompt_color: Pixel,
    pub command_input: bool,
    pub command_prompt: String,
    pub command_prompt_color: Pixel,
    pub command_text: String,
    pub command_active: bool,
    pub background: bool,
    pub background_color: Pixel,
    pub background_padding: f32,
    pub max_messages: usize,
    pub scrollback: bool,
    pub(crate) paused: bool,
    pub(crate) pause_until: Option<Instant>,
    pub(crate) scroll_offset: usize,
    pub(crate) page_start_index: usize,
}

impl Default for MessagesWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl MessagesWidget {
    const CHOICE_COLUMN_SEPARATOR: char = '\u{1f}';

    fn command_input_enabled(&self) -> bool {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            false
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            self.command_input
        }
    }

    pub fn new() -> Self {
        Self {
            name: String::new(),
            rect: Rect::default(),
            toml_str: String::new(),
            buffer: TheRGBABuffer::default(),
            font: None,
            font_size: 20.0,
            messages: vec![],
            pending_messages: VecDeque::new(),
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
            press_to_continue: false,
            pause_blocks_input: true,
            continue_prompt: "Press to continue".into(),
            continue_prompt_color: [170, 170, 170, 255],
            command_input: false,
            command_prompt: "@".into(),
            command_prompt_color: [215, 153, 33, 255],
            command_text: String::new(),
            command_active: false,
            background: false,
            background_color: [0, 0, 0, 150],
            background_padding: 0.0,
            max_messages: 100,
            scrollback: true,
            paused: false,
            pause_until: None,
            scroll_offset: 0,
            page_start_index: 0,
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
                if let Some(value) = ui
                    .get("press_to_continue")
                    .or_else(|| ui.get("pause_on_overflow"))
                    .and_then(toml::Value::as_bool)
                {
                    self.press_to_continue = value;
                }
                if let Some(value) = ui
                    .get("pause_blocks_input")
                    .or_else(|| ui.get("block_input_during_pause"))
                    .and_then(toml::Value::as_bool)
                {
                    self.pause_blocks_input = value;
                }
                if let Some(value) = ui
                    .get("continue_prompt")
                    .or_else(|| ui.get("pause_prompt"))
                    .and_then(toml::Value::as_str)
                {
                    self.continue_prompt = value.to_string();
                }
                if let Some(value) = ui
                    .get("continue_prompt_color")
                    .or_else(|| ui.get("pause_prompt_color"))
                    .and_then(toml::Value::as_str)
                {
                    self.continue_prompt_color = self.hex_to_rgba_u8(value);
                }
                if let Some(value) = ui.get("command_input").and_then(toml::Value::as_bool) {
                    self.command_input = value;
                }
                if let Some(value) = ui.get("command_prompt").and_then(toml::Value::as_str) {
                    self.command_prompt = value.to_string();
                }
                if let Some(value) = ui.get("command_prompt_color").and_then(toml::Value::as_str) {
                    self.command_prompt_color = self.hex_to_rgba_u8(value);
                }
                if let Some(value) = ui.get("background").and_then(toml::Value::as_bool) {
                    self.background = value;
                }
                if let Some(value) = ui
                    .get("background_color")
                    .or_else(|| ui.get("background"))
                    .and_then(toml::Value::as_str)
                {
                    self.background = true;
                    self.background_color = self.hex_to_rgba_u8(value);
                }
                if let Some(value) = ui.get("background_padding") {
                    if let Some(v) = value.as_float() {
                        self.background_padding = (v as f32).max(0.0);
                    } else if let Some(v) = value.as_integer() {
                        self.background_padding = (v as f32).max(0.0);
                    }
                }
                if let Some(value) = ui.get("max_messages") {
                    if let Some(v) = value.as_integer() {
                        self.max_messages = (v as usize).max(1);
                    }
                }
                if let Some(value) = ui.get("scrollback").and_then(toml::Value::as_bool) {
                    self.scrollback = value;
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

    fn parse_pause_duration(msg: &str, category: &str) -> Option<Duration> {
        let lower = category.trim().to_ascii_lowercase();
        let value = lower
            .strip_prefix("pause:")
            .or_else(|| lower.strip_prefix("timer:"))
            .unwrap_or(msg.trim());
        let seconds = value.parse::<f32>().ok()?;
        (seconds > 0.0).then(|| Duration::from_secs_f32(seconds))
    }

    fn is_pause_category(category: &str) -> bool {
        matches!(
            category.trim().to_ascii_lowercase().as_str(),
            "pause" | "continue" | "message_pause" | "pause_to_continue"
        ) || category.trim().to_ascii_lowercase().starts_with("pause:")
            || category.trim().to_ascii_lowercase().starts_with("timer:")
    }

    fn queue_pause(&mut self, msg: &str, category: &str) {
        if let Some(duration) = Self::parse_pause_duration(msg, category) {
            self.pending_messages
                .push_back(PendingMessage::Timer(duration));
        } else {
            self.pending_messages.push_back(PendingMessage::Continue);
        }
    }

    fn continue_after_pause(&mut self) {
        self.paused = false;
        self.pause_until = None;
        self.page_start_index = self.messages.len();
        self.scroll_offset = 0;
    }

    fn pause_for_continue(&mut self) {
        self.paused = true;
        self.pause_until = None;
    }

    fn pause_for_duration(&mut self, duration: Duration) {
        self.paused = true;
        self.pause_until = Some(Instant::now() + duration);
    }

    fn advance_pause_timer(&mut self) {
        if self
            .pause_until
            .is_some_and(|pause_until| Instant::now() >= pause_until)
        {
            self.continue_after_pause();
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
        self.advance_pause_timer();
        let currencies = Currencies::from_rules_source(&assets.rules);
        let starts_new_reveal_batch = !self.paused && self.pending_messages.is_empty();
        let new_batch_start_index = self.messages.len();

        // Append new messages
        for (sender_entity, sender_item, receiver_id, msg, category) in &messages {
            if Self::is_pause_category(category) {
                self.queue_pause(msg, category);
                continue;
            }
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
            self.pending_messages
                .push_back(PendingMessage::Line(MessageLine {
                    id: Uuid::new_v4(),
                    text: message.clone(),
                    rect: Rect::default(),
                    choice: None,
                    choice_key: None,
                    color,
                    sender_entity: *sender_entity,
                }));
        }

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
                                            item_price = item
                                                .attributes
                                                .get("worth")
                                                .and_then(|worth| worth.to_i32())
                                                .unwrap_or(0)
                                                as i64;
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
                let text = if matches!(choice, Choice::ItemToSell(_, _, _, _, _)) {
                    let label = format!("{}) {}", index + 1, item_name);
                    let price = currencies.format_base_amount(item_price);
                    format!("{}{}{}", label, Self::CHOICE_COLUMN_SEPARATOR, price)
                } else {
                    format!("{}) {}", index + 1, item_name)
                };

                self.pending_messages
                    .push_back(PendingMessage::Line(MessageLine {
                        id: Uuid::new_v4(),
                        text,
                        rect: Rect::default(),
                        choice: Some(rendered_choice),
                        choice_key: Some((b'1' + index as u8) as char),
                        color,
                        sender_entity: Some(choices.from),
                    }));
            }
            self.pending_messages
                .push_back(PendingMessage::Line(MessageLine {
                    id: Uuid::new_v4(),
                    text: self.resolve_msg("0) {system.exit_menu}", map, assets, time),
                    rect: Rect::default(),
                    choice: Some(Choice::Cancel(
                        choices.from,
                        choices.to,
                        choices.expires_at_tick,
                        choices.max_distance,
                    )),
                    choice_key: Some('0'),
                    color,
                    sender_entity: Some(choices.from),
                }));
        }

        if starts_new_reveal_batch && !self.pending_messages.is_empty() {
            self.page_start_index = new_batch_start_index;
        }
        self.reveal_pending_messages();
        self.purge_old_messages();

        let choice_map = self.active_choice_map();
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
            if self.background {
                let padding = self.background_padding;
                let bg_rect = (
                    (self.rect.x - padding).round() as isize,
                    (self.rect.y - padding).round() as isize,
                    (self.rect.width + padding * 2.0).round().max(1.0) as isize,
                    (self.rect.height + padding * 2.0).round().max(1.0) as isize,
                );
                let safe = (0_isize, 0_isize, width as isize, height as isize);
                self.draw2d.blend_rect_safe(
                    buffer.pixels_mut(),
                    &bg_rect,
                    stride,
                    &self.background_color,
                    &safe,
                );
            }
            let prompt_reserved_height =
                if self.command_input_enabled() || (self.paused && self.pause_until.is_none()) {
                    self.font_size.ceil() + self.message_spacing.max(self.spacing)
                } else {
                    0.0
                };
            let content_bottom = self.rect.y + (self.rect.height - prompt_reserved_height).max(0.0);
            let mut y = if self.top_down {
                self.rect.y
            } else {
                content_bottom - self.font_size.ceil()
            };
            let draw2d = &self.draw2d;

            let scroll_offset = self.scroll_offset.min(self.messages.len());
            for message_line in self.messages.iter_mut().rev().skip(scroll_offset) {
                let id = message_line.id;
                let message = &message_line.text;
                let rect = &mut message_line.rect;
                let color = message_line.color;
                let sender_entity = message_line.sender_entity;
                let portrait_tile = if self.portrait {
                    Self::portrait_tile_for_sender(sender_entity, map, assets)
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

                let color = if id == self.clicked {
                    darken(color, 100)
                } else {
                    color
                };
                let mut drew_block = false;

                if self.top_down {
                    if y > content_bottom {
                        *rect = Rect::default();
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
                        if line_y > content_bottom {
                            break;
                        }
                        drew_block = true;

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
                        *rect = Rect::default();
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
                        if line_y + self.font_size < self.rect.y || line_y > content_bottom {
                            continue;
                        }
                        drew_block = true;
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
                if !drew_block {
                    *rect = Rect::default();
                }
            }

            if self.paused && self.pause_until.is_none() {
                self.draw_continue_prompt(buffer, font, map, assets, time, stride, &clip_rect);
            } else if self.command_input_enabled() {
                self.draw_command_input(buffer, font, stride, &clip_rect);
            }
        }

        choice_map
    }

    fn reveal_pending_messages(&mut self) {
        if self.paused {
            return;
        }

        while let Some(entry) = self.pending_messages.pop_front() {
            match entry {
                PendingMessage::Line(line) => {
                    self.scroll_offset = 0;
                    self.messages.push(line);
                    if self.press_to_continue
                        && !self.pending_messages.is_empty()
                        && self.page_is_full(true)
                    {
                        self.pause_for_continue();
                        break;
                    }
                }
                PendingMessage::Continue => {
                    self.pause_for_continue();
                    break;
                }
                PendingMessage::Timer(duration) => {
                    self.pause_for_duration(duration);
                    break;
                }
            }
        }

        if !self.paused && self.pending_messages.is_empty() {
            self.page_start_index = self.messages.len();
        }
    }

    fn purge_old_messages(&mut self) {
        if self.messages.len() > self.max_messages {
            let excess = self.messages.len() - self.max_messages;
            self.messages.drain(0..excess);
            self.page_start_index = self.page_start_index.saturating_sub(excess);
            self.scroll_offset = self.scroll_offset.saturating_sub(excess);
        }
    }

    fn page_is_full(&self, reserve_prompt: bool) -> bool {
        let Some(font) = &self.font else {
            return false;
        };
        let prompt_reserved_height = if reserve_prompt {
            self.font_size.ceil() + self.message_spacing.max(self.spacing)
        } else {
            0.0
        };
        let available_height = (self.rect.height - prompt_reserved_height).max(self.font_size);
        let mut total_height = 0.0;
        for message_line in self.messages.iter().skip(self.page_start_index) {
            let portrait_width = if self.portrait && message_line.sender_entity.is_some() {
                self.portrait_size + self.portrait_gap
            } else {
                0.0
            };
            let text_width = (self.rect.width - portrait_width).max(self.font_size);
            let lines = Self::wrap_message_lines(
                &self.draw2d,
                font,
                self.font_size,
                &message_line.text,
                text_width,
            );
            let line_height = self.font_size + self.spacing;
            let text_height = if lines.is_empty() {
                self.font_size
            } else {
                self.font_size + (lines.len().saturating_sub(1) as f32 * line_height)
            };
            let block_height = text_height.max(if portrait_width > 0.0 {
                self.portrait_size
            } else {
                0.0
            });
            if total_height > 0.0 {
                total_height += self.message_spacing;
            }
            total_height += block_height;
        }
        total_height > available_height
    }

    fn draw_continue_prompt(
        &self,
        buffer: &mut TheRGBABuffer,
        font: &fontdue::Font,
        map: &Map,
        assets: &Assets,
        time: &TheTime,
        stride: usize,
        clip_rect: &(isize, isize, isize, isize),
    ) {
        let prompt = self.resolve_msg(&self.continue_prompt, map, assets, time);
        let tuple = (
            self.rect.x as isize,
            (self.rect.y + self.rect.height - self.font_size.ceil()) as isize,
            self.rect.width as isize,
            self.font_size as isize,
        );
        self.draw2d.text_rect_blend_safe_clip(
            buffer.pixels_mut(),
            &tuple,
            stride,
            font,
            self.font_size,
            &prompt,
            &self.continue_prompt_color,
            draw2d::TheHorizontalAlign::Right,
            draw2d::TheVerticalAlign::Center,
            clip_rect,
        );
    }

    fn command_input_rect(&self) -> Rect {
        Rect::new(
            self.rect.x,
            self.rect.y + self.rect.height - self.font_size.ceil(),
            self.rect.width,
            self.font_size.ceil(),
        )
    }

    fn draw_command_input(
        &self,
        buffer: &mut TheRGBABuffer,
        font: &fontdue::Font,
        stride: usize,
        clip_rect: &(isize, isize, isize, isize),
    ) {
        let text = if self.command_active {
            format!("{} {}_", self.command_prompt, self.command_text)
        } else if self.command_text.is_empty() {
            format!("{} ", self.command_prompt)
        } else {
            format!("{} {}", self.command_prompt, self.command_text)
        };
        let rect = self.command_input_rect();
        let tuple = (
            rect.x as isize,
            rect.y as isize,
            rect.width as isize,
            rect.height as isize,
        );
        self.draw2d.text_rect_blend_safe_clip(
            buffer.pixels_mut(),
            &tuple,
            stride,
            font,
            self.font_size,
            &text,
            &self.command_prompt_color,
            draw2d::TheHorizontalAlign::Left,
            draw2d::TheVerticalAlign::Center,
            clip_rect,
        );
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
        let inside = self
            .rect
            .contains(Vec2::new(coord.x as f32, coord.y as f32));
        if self.command_input_enabled()
            && self
                .command_input_rect()
                .contains(Vec2::new(coord.x as f32, coord.y as f32))
        {
            self.command_active = true;
            return Some(EntityAction::Off);
        }
        if inside && self.paused && self.pause_until.is_none() {
            self.continue_after_pause();
            self.reveal_pending_messages();
            return Some(EntityAction::Off);
        }
        if self.paused && self.pause_blocks_input {
            return Some(EntityAction::Off);
        }
        for message_line in &self.messages {
            if message_line
                .rect
                .contains(Vec2::new(coord.x as f32, coord.y as f32))
            {
                if let Some(choice) = &message_line.choice {
                    self.clicked = message_line.id;
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
            .any(|message_line| message_line.choice.is_some())
    }

    fn active_choice_map(&self) -> FxHashMap<char, Choice> {
        let mut choice_map = FxHashMap::default();
        for message_line in &self.messages {
            let Some(choice) = &message_line.choice else {
                continue;
            };
            let Some(key) = message_line.choice_key else {
                continue;
            };
            choice_map.insert(key, choice.clone());
        }
        choice_map
    }

    pub fn blocks_input(&self) -> bool {
        self.paused && self.pause_blocks_input
    }

    pub fn user_event(&mut self, event: &str, value: &Value) -> Option<EntityAction> {
        self.advance_pause_timer();
        if self.command_input_enabled()
            && !self.paused
            && event == "key_down"
            && let Value::Str(v) = value
        {
            let key = v.as_str();
            let key_lower = key.trim().to_ascii_lowercase();
            if !self.command_active {
                if matches!(key_lower.as_str(), "enter" | "return" | "\n") || key == "/" {
                    self.command_active = true;
                    return Some(EntityAction::Off);
                }
                return None;
            }

            match key_lower.as_str() {
                "enter" | "return" | "\n" => {
                    let command = self.command_text.trim().to_string();
                    self.command_text.clear();
                    self.command_active = false;
                    return if command.is_empty() {
                        Some(EntityAction::Off)
                    } else {
                        Some(EntityAction::TextCommand(command))
                    };
                }
                "escape" => {
                    self.command_text.clear();
                    self.command_active = false;
                    return Some(EntityAction::Off);
                }
                "backspace" | "delete" => {
                    self.command_text.pop();
                    return Some(EntityAction::Off);
                }
                "space" => {
                    self.command_text.push(' ');
                    return Some(EntityAction::Off);
                }
                _ => {
                    if key.chars().count() == 1
                        && let Some(ch) = key.chars().next()
                        && !ch.is_control()
                    {
                        self.command_text.push(ch);
                        return Some(EntityAction::Off);
                    }
                }
            }
        }
        if !self.paused {
            return None;
        }

        if event == "key_down"
            && self.pause_until.is_none()
            && let Value::Str(v) = value
        {
            let key = v.trim().to_ascii_lowercase();
            if matches!(key.as_str(), " " | "space" | "enter" | "return" | "\n") {
                self.continue_after_pause();
                self.reveal_pending_messages();
                return Some(EntityAction::Off);
            }
        }

        self.pause_blocks_input.then_some(EntityAction::Off)
    }

    pub fn scroll(&mut self, delta_y: isize) -> bool {
        if !self.scrollback || self.messages.is_empty() {
            return false;
        }

        let step = (delta_y.unsigned_abs() / 120).max(1);
        if delta_y > 0 {
            self.scroll_offset =
                (self.scroll_offset + step).min(self.messages.len().saturating_sub(1));
        } else if delta_y < 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(step);
        }
        true
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

        for message_line in &mut self.messages {
            let Some(active_choice) = message_line.choice.as_ref() else {
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
                message_line.choice = None;
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
