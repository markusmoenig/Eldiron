pub mod avatar;
pub mod deco;
pub mod game;
pub mod game_backend;
pub mod messages;
pub mod screen;
pub mod stat;
pub mod text;

use crate::{
    Assets, Entity, Item, Map, Pixel, PlayerCamera, Rect, Texture, Value, WHITE,
    avatar_builder::AvatarRuntimeBuilder,
    client::command::{ClientCommandBinding, parse_client_command},
    client::draw2d,
};
use draw2d::Draw2D;
use theframework::prelude::*;
use toml::Table;

#[derive(Clone, Copy, Default)]
pub struct ButtonStateStyle {
    pub background_color: Option<Pixel>,
    pub border_color: Option<Pixel>,
    pub label_color: Option<Pixel>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVisualState {
    #[default]
    Normal,
    Hover,
    Pressed,
    Selected,
    Disabled,
}

/// Used right now for button widgets
pub struct Widget {
    pub name: String,
    pub id: u32,
    pub rect: Rect,
    pub action: String,
    pub command: Option<String>,
    pub command_slot: Option<String>,
    pub intent: Option<String>,
    pub spell: Option<String>,
    pub group: Option<String>,
    pub binding: Option<String>,
    pub value: Option<String>,
    pub selection: Option<String>,
    pub show: Option<Vec<String>>,
    pub hide: Option<Vec<String>>,
    pub deactivate: Vec<String>,
    pub camera: Option<PlayerCamera>,
    pub player_camera: Option<PlayerCamera>,
    pub camera_target: Option<String>,
    pub party: Option<String>,
    pub inventory_index: Option<usize>,
    pub equipped_slot: Option<String>,
    pub portrait: bool,
    pub drag_drop: bool,
    pub textures: Vec<Texture>,
    pub entity_cursor_id: Option<Uuid>,
    pub entity_clicked_cursor_id: Option<Uuid>,
    pub item_cursor_id: Option<Uuid>,
    pub item_clicked_cursor_id: Option<Uuid>,
    pub border_color: Pixel,
    pub border_size: i32,
    pub label: String,
    pub label_font: String,
    pub label_font_size: f32,
    pub label_color: Pixel,
    pub background_color: Option<Pixel>,
    pub hover_style: ButtonStateStyle,
    pub selected_style: ButtonStateStyle,
    pub pressed_style: ButtonStateStyle,
    pub disabled_style: ButtonStateStyle,
}

pub struct TextInputWidget {
    pub name: String,
    pub id: u32,
    pub rect: Rect,
    pub binding: String,
    pub text: String,
    pub font: String,
    pub font_size: f32,
    pub color: Pixel,
    pub background_color: Pixel,
    pub border_color: Pixel,
    pub border_size: i32,
}

impl TextInputWidget {
    pub fn update_draw(
        &self,
        buffer: &mut TheRGBABuffer,
        assets: &Assets,
        draw2d: &Draw2D,
        focused: bool,
    ) {
        let stride = buffer.stride();
        let width = buffer.dim().width as isize;
        let height = buffer.dim().height as isize;
        let rect = (
            self.rect.x.floor() as isize,
            self.rect.y.floor() as isize,
            self.rect.width.ceil() as isize,
            self.rect.height.ceil() as isize,
        );

        draw2d.blend_rect_safe(
            buffer.pixels_mut(),
            &rect,
            stride,
            &self.background_color,
            &(0, 0, width, height),
        );

        let border_color = if focused {
            [238, 214, 118, 255]
        } else {
            self.border_color
        };
        if self.border_size > 0 {
            draw2d.rect_outline_thickness(
                buffer.pixels_mut(),
                &(
                    self.rect.x.max(0.0) as usize,
                    self.rect.y.max(0.0) as usize,
                    self.rect.width.max(0.0) as usize,
                    self.rect.height.max(0.0) as usize,
                ),
                stride,
                &border_color,
                self.border_size as usize,
            );
        }

        if let Some(font) = assets
            .fonts
            .get(self.font.trim())
            .or_else(|| assets.fonts.values().next())
        {
            let display_text = if focused {
                format!("{}_", self.text)
            } else {
                self.text.clone()
            };
            draw2d.text_rect_blend_safe(
                buffer.pixels_mut(),
                &(
                    self.rect.x.floor() as isize + 8,
                    self.rect.y.floor() as isize,
                    self.rect.width.ceil() as isize - 16,
                    self.rect.height.ceil() as isize,
                ),
                stride,
                font,
                self.font_size,
                &display_text,
                &self.color,
                draw2d::TheHorizontalAlign::Left,
                draw2d::TheVerticalAlign::Center,
                &(0, 0, width, height),
            );
        }
    }
}

fn table_at<'a>(root: &'a Table, path: &[&str]) -> Option<&'a Table> {
    let mut current = root;
    for key in path {
        current = current.get(*key)?.as_table()?;
    }
    Some(current)
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            id: 0,
            rect: Rect::default(),
            action: String::new(),
            command: None,
            command_slot: None,
            intent: None,
            spell: None,
            group: None,
            binding: None,
            value: None,
            selection: None,
            show: None,
            hide: None,
            deactivate: vec![],
            camera: None,
            player_camera: None,
            camera_target: None,
            party: None,
            inventory_index: None,
            equipped_slot: None,
            portrait: false,
            drag_drop: false,
            textures: vec![],
            entity_cursor_id: None,
            entity_clicked_cursor_id: None,
            item_cursor_id: None,
            item_clicked_cursor_id: None,
            border_color: WHITE,
            border_size: 0,
            label: String::new(),
            label_font: String::new(),
            label_font_size: 18.0,
            label_color: WHITE,
            background_color: None,
            hover_style: ButtonStateStyle::default(),
            selected_style: ButtonStateStyle::default(),
            pressed_style: ButtonStateStyle::default(),
            disabled_style: ButtonStateStyle::default(),
        }
    }

    pub fn command_binding(&self) -> Option<ClientCommandBinding> {
        self.command
            .as_deref()
            .and_then(parse_client_command)
            .or_else(|| {
                self.intent.as_ref().and_then(|intent| {
                    let intent = intent.trim();
                    if intent.is_empty() {
                        Some(ClientCommandBinding::Intent(String::new()))
                    } else if intent.eq_ignore_ascii_case("spell") {
                        self.spell
                            .as_ref()
                            .map(|spell| spell.trim())
                            .filter(|spell| !spell.is_empty())
                            .map(|spell| ClientCommandBinding::Intent(format!("spell:{}", spell)))
                            .or_else(|| Some(ClientCommandBinding::Intent(intent.to_string())))
                    } else {
                        Some(ClientCommandBinding::Intent(intent.to_string()))
                    }
                })
            })
            .or_else(|| {
                self.action
                    .trim()
                    .parse::<crate::EntityAction>()
                    .ok()
                    .map(ClientCommandBinding::Control)
            })
    }

    pub fn intent_payload(&self) -> Option<String> {
        self.command_binding()
            .and_then(|binding| binding.intent_payload())
    }

    pub fn update_draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        _map: &Map,
        assets: &Assets,
        entity: Option<&Entity>,
        draw2d: &Draw2D,
        animation_frame: &usize,
        visual_state: ButtonVisualState,
        resolved_command: Option<&str>,
    ) {
        let stride = buffer.stride();
        let buffer_width = buffer.dim().width as isize;
        let buffer_height = buffer.dim().height as isize;
        let state_style = match visual_state {
            ButtonVisualState::Normal => ButtonStateStyle::default(),
            ButtonVisualState::Hover => self.hover_style,
            ButtonVisualState::Pressed => self.pressed_style,
            ButtonVisualState::Selected => self.selected_style,
            ButtonVisualState::Disabled => self.disabled_style,
        };
        let is_command_slot = self.command_slot.is_some();
        let is_command_button = is_command_slot
            || resolved_command
                .or(self.command.as_deref())
                .and_then(parse_client_command)
                .is_some();

        let effective_background_color =
            if is_command_button && matches!(visual_state, ButtonVisualState::Selected) {
                self.background_color
            } else {
                state_style.background_color.or(self.background_color)
            };
        if let Some(background_color) = effective_background_color {
            let color = match visual_state {
                ButtonVisualState::Selected if is_command_button => background_color,
                ButtonVisualState::Selected => state_style.background_color.unwrap_or([
                    background_color[0].saturating_add(34),
                    background_color[1].saturating_add(30),
                    background_color[2].saturating_add(16),
                    background_color[3],
                ]),
                ButtonVisualState::Hover => state_style.background_color.unwrap_or([
                    background_color[0].saturating_add(18),
                    background_color[1].saturating_add(18),
                    background_color[2].saturating_add(18),
                    background_color[3],
                ]),
                ButtonVisualState::Pressed => state_style.background_color.unwrap_or([
                    background_color[0].saturating_sub(20),
                    background_color[1].saturating_sub(20),
                    background_color[2].saturating_sub(20),
                    background_color[3],
                ]),
                ButtonVisualState::Disabled => state_style.background_color.unwrap_or([
                    background_color[0] / 2,
                    background_color[1] / 2,
                    background_color[2] / 2,
                    background_color[3],
                ]),
                ButtonVisualState::Normal => background_color,
            };
            draw2d.blend_rect_safe(
                buffer.pixels_mut(),
                &(
                    self.rect.x.floor() as isize,
                    self.rect.y.floor() as isize,
                    self.rect.width.ceil() as isize,
                    self.rect.height.ceil() as isize,
                ),
                stride,
                &color,
                &(0, 0, buffer_width, buffer_height),
            );
        }

        let is_item_slot = self.inventory_index.is_some() || self.equipped_slot.is_some();
        let mut drew_primary_texture = false;
        if !is_item_slot
            && let Some((texture, color)) = Self::command_icon_texture(
                assets,
                resolved_command.or(self.command.as_deref()),
                visual_state,
            )
        {
            Self::draw_tinted_texture(buffer, self.rect, draw2d, texture, color);
            drew_primary_texture = true;
        }
        if !drew_primary_texture && !self.textures.is_empty() {
            let texture_index = self.texture_index_for_state(visual_state);
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    self.rect.x as usize,
                    self.rect.y as usize,
                    self.rect.width as usize,
                    self.rect.height as usize,
                ),
                stride,
                &self.textures[texture_index].data,
                &(
                    self.textures[texture_index].width as usize,
                    self.textures[texture_index].height as usize,
                ),
            );
        }

        let entity = entity;
        let item_to_draw = if let Some(inventory_index) = &self.inventory_index {
            entity.and_then(|entity| {
                entity
                    .inventory
                    .get(*inventory_index)
                    .and_then(|item| item.as_ref())
            })
        } else if let Some(slot) = &self.equipped_slot {
            entity.and_then(|entity| entity.get_equipped_item(slot))
        } else {
            None
        };

        if self.portrait
            && let Some(entity) = entity
            && let Some(tile) = Self::portrait_tile_for_entity(entity, assets)
        {
            let index = *animation_frame % tile.textures.len();
            let rect = self.rect.with_border(4.0);
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                stride,
                &tile.textures[index].data,
                &(
                    tile.textures[index].width as usize,
                    tile.textures[index].height as usize,
                ),
            );
        } else if let Some(item) = item_to_draw {
            Self::draw_item_icon(buffer, self.rect, assets, item, draw2d, *animation_frame);
        }

        if self.border_size > 0 {
            let border_color = match visual_state {
                ButtonVisualState::Selected => {
                    if is_command_button {
                        [255, 255, 255, 255]
                    } else {
                        state_style.border_color.unwrap_or([238, 214, 118, 255])
                    }
                }
                ButtonVisualState::Hover => state_style.border_color.unwrap_or([
                    self.border_color[0].saturating_add(34),
                    self.border_color[1].saturating_add(34),
                    self.border_color[2].saturating_add(34),
                    self.border_color[3],
                ]),
                ButtonVisualState::Pressed => state_style.border_color.unwrap_or([
                    self.border_color[0].saturating_sub(24),
                    self.border_color[1].saturating_sub(24),
                    self.border_color[2].saturating_sub(24),
                    self.border_color[3],
                ]),
                ButtonVisualState::Disabled => state_style.border_color.unwrap_or([
                    self.border_color[0] / 2,
                    self.border_color[1] / 2,
                    self.border_color[2] / 2,
                    self.border_color[3],
                ]),
                ButtonVisualState::Normal => self.border_color,
            };
            draw2d.rect_outline_thickness(
                buffer.pixels_mut(),
                &(
                    self.rect.x as usize,
                    self.rect.y as usize,
                    self.rect.width as usize,
                    self.rect.height as usize,
                ),
                stride,
                &border_color,
                self.border_size as usize,
            );
        }

        if !self.label.trim().is_empty() {
            let font = if self.label_font.trim().is_empty() {
                assets.fonts.values().next()
            } else {
                assets
                    .fonts
                    .get(self.label_font.trim())
                    .or_else(|| assets.fonts.values().next())
            };

            if let Some(font) = font {
                draw2d.text_rect_blend_safe(
                    buffer.pixels_mut(),
                    &(
                        self.rect.x.floor() as isize + 4,
                        self.rect.y.floor() as isize,
                        self.rect.width.ceil() as isize - 8,
                        self.rect.height.ceil() as isize,
                    ),
                    stride,
                    font,
                    self.label_font_size,
                    &self.label,
                    &state_style.label_color.unwrap_or(self.label_color),
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, buffer_width, buffer_height),
                );
            }
        }
    }

    pub(crate) fn command_icon_texture<'a>(
        assets: &'a Assets,
        command: Option<&str>,
        visual_state: ButtonVisualState,
    ) -> Option<(&'a Texture, Pixel)> {
        let root = assets.rules.parse::<Table>().ok()?;
        let command_table = Self::command_icon_table(&root, command?)?;
        let ui = command_table.get("ui").and_then(toml::Value::as_table);

        let icon_key = match visual_state {
            ButtonVisualState::Selected => ["selected_icon", "icon"],
            ButtonVisualState::Pressed => ["pressed_icon", "selected_icon"],
            ButtonVisualState::Disabled => ["disabled_icon", "icon"],
            ButtonVisualState::Hover | ButtonVisualState::Normal => ["icon", "normal_icon"],
        };
        let icon_name = icon_key
            .iter()
            .find_map(|key| {
                ui.and_then(|ui| ui.get(*key))
                    .or_else(|| command_table.get(*key))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| {
                ui.and_then(|ui| ui.get("icon"))
                    .or_else(|| command_table.get("icon"))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })?;
        let icon_name = Self::resolve_icon_texture_id(&root, icon_name);

        let color = match visual_state {
            ButtonVisualState::Selected => Self::command_icon_color(
                command_table,
                ui,
                &["selected_icon_color", "icon_selected_color"],
                [255, 255, 255, 255],
            ),
            ButtonVisualState::Pressed => Self::command_icon_color(
                command_table,
                ui,
                &["pressed_icon_color", "icon_pressed_color"],
                [255, 255, 255, 255],
            ),
            ButtonVisualState::Disabled => Self::command_icon_color(
                command_table,
                ui,
                &["disabled_icon_color", "icon_disabled_color"],
                [112, 112, 112, 255],
            ),
            ButtonVisualState::Hover => Self::command_icon_color(
                command_table,
                ui,
                &["hover_icon_color", "icon_hover_color"],
                [190, 190, 190, 255],
            ),
            ButtonVisualState::Normal => Self::command_icon_color(
                command_table,
                ui,
                &["icon_color", "normal_icon_color"],
                [150, 150, 150, 255],
            ),
        };

        assets
            .textures
            .get(icon_name.as_str())
            .map(|texture| (texture, color))
    }

    fn command_icon_table<'a>(root: &'a Table, command: &str) -> Option<&'a Table> {
        match parse_client_command(command)? {
            ClientCommandBinding::RulesAction(action_id) => {
                table_at(root, &["actions", action_id.as_str()])
            }
            ClientCommandBinding::Intent(intent) => {
                let intent_id = if intent.trim().is_empty() {
                    "walk"
                } else {
                    intent
                        .split_once(':')
                        .map(|(head, _)| head)
                        .unwrap_or(intent.as_str())
                };
                table_at(root, &["intents", intent_id.trim()])
            }
            _ => None,
        }
    }

    fn resolve_icon_texture_id(root: &Table, icon_id: &str) -> String {
        table_at(root, &["icons", icon_id])
            .and_then(|icon| icon.get("texture"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(icon_id)
            .to_string()
    }

    fn command_icon_color(
        action: &Table,
        ui: Option<&Table>,
        keys: &[&str],
        fallback: Pixel,
    ) -> Pixel {
        keys.iter()
            .find_map(|key| {
                ui.and_then(|ui| ui.get(*key))
                    .or_else(|| action.get(*key))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(Self::hex_to_rgba_u8)
            })
            .or_else(|| {
                ui.and_then(|ui| ui.get("icon_color"))
                    .or_else(|| action.get("icon_color"))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(Self::hex_to_rgba_u8)
            })
            .unwrap_or(fallback)
    }

    fn draw_tinted_texture(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        draw2d: &Draw2D,
        texture: &Texture,
        color: Pixel,
    ) {
        let stride = buffer.stride();
        let inset = (rect.width.min(rect.height) * 0.12).round().max(2.0);
        let dest_x = (rect.x + inset).round().max(0.0) as usize;
        let dest_y = (rect.y + inset).round().max(0.0) as usize;
        let dest_w = (rect.width - inset * 2.0).round().max(1.0) as usize;
        let dest_h = (rect.height - inset * 2.0).round().max(1.0) as usize;
        let x_ratio = texture.width as f32 / dest_w as f32;
        let y_ratio = texture.height as f32 / dest_h as f32;
        let frame = buffer.pixels_mut();

        for sy in 0..dest_h {
            let y = (sy as f32 * y_ratio) as usize;
            for sx in 0..dest_w {
                let x = (sx as f32 * x_ratio) as usize;
                let d = (dest_x + sx) * 4 + (dest_y + sy) * stride * 4;
                if d + 3 >= frame.len() {
                    continue;
                }
                let s = x * 4 + y * texture.width * 4;
                if s + 3 >= texture.data.len() {
                    continue;
                }
                let source_alpha = texture.data[s + 3];
                if source_alpha == 0 {
                    continue;
                }
                let shade = texture.data[s]
                    .max(texture.data[s + 1])
                    .max(texture.data[s + 2]);
                let shade = shade as u16;
                let tinted = [
                    ((color[0] as u16 * shade) / 255) as u8,
                    ((color[1] as u16 * shade) / 255) as u8,
                    ((color[2] as u16 * shade) / 255) as u8,
                    ((source_alpha as u16 * color[3] as u16) / 255) as u8,
                ];
                let background = [frame[d], frame[d + 1], frame[d + 2], frame[d + 3]];
                frame[d..d + 4].copy_from_slice(&draw2d.mix_color(
                    &background,
                    &tinted,
                    tinted[3] as f32 / 255.0,
                ));
            }
        }
    }

    fn hex_to_rgba_u8(hex: &str) -> [u8; 4] {
        let hex = hex.trim().trim_start_matches('#');
        if !(hex.len() == 6 || hex.len() == 8) {
            return [255, 255, 255, 255];
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255)
        } else {
            255
        };
        [r, g, b, a]
    }

    fn texture_index_for_state(&self, visual_state: ButtonVisualState) -> usize {
        let len = self.textures.len();
        match visual_state {
            ButtonVisualState::Selected => {
                if len > 1 {
                    1
                } else {
                    0
                }
            }
            ButtonVisualState::Pressed => {
                if len > 2 {
                    2
                } else if len > 1 {
                    1
                } else {
                    0
                }
            }
            ButtonVisualState::Disabled => {
                if len > 3 {
                    3
                } else {
                    0
                }
            }
            ButtonVisualState::Hover | ButtonVisualState::Normal => 0,
        }
    }

    fn portrait_tile_for_entity(entity: &Entity, assets: &Assets) -> Option<crate::Tile> {
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

    pub fn draw_item_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
        animation_frame: usize,
    ) -> bool {
        let stride = buffer.stride();
        let rect = rect.with_border(4.0);
        let mut drawn = false;
        if let Some(Value::Source(source)) = item.attributes.get("source")
            && let Some(tile) = source.tile_from_tile_list(assets)
        {
            let index = animation_frame % tile.textures.len();
            let texture = &tile.textures[index];
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                stride,
                &texture.data,
                &(texture.width, texture.height),
            );
            drawn = true;
        }

        if !drawn && let Some(tile) = AvatarRuntimeBuilder::explicit_item_tile(item, assets) {
            let index = animation_frame % tile.textures.len();
            let texture = &tile.textures[index];
            draw2d.blend_scale_chunk(
                buffer.pixels_mut(),
                &(
                    rect.x as usize,
                    rect.y as usize,
                    rect.width as usize,
                    rect.height as usize,
                ),
                stride,
                &texture.data,
                &(texture.width, texture.height),
            );
            drawn = true;
        }

        if !drawn && Self::draw_generated_avatar_channel_icon(buffer, rect, assets, item, draw2d) {
            drawn = true;
        }

        if !drawn && Self::draw_item_template_mask_icon(buffer, rect, assets, item, draw2d) {
            drawn = true;
        }

        if !drawn && !AvatarRuntimeBuilder::item_has_explicit_tile(item, assets) {
            drawn = Self::draw_generated_equipment_icon(buffer, rect, assets, item, draw2d);
        }

        if !drawn && Self::draw_item_icon_texture(buffer, rect, assets, item, draw2d) {
            drawn = true;
        }
        if drawn {
            Self::draw_stack_badge(buffer, rect, item, draw2d);
        }
        drawn
    }

    fn draw_stack_badge(buffer: &mut TheRGBABuffer, rect: Rect, item: &Item, _draw2d: &Draw2D) {
        let quantity = item.stack_quantity();
        if quantity <= 1 {
            return;
        }
        let text = quantity.min(999).to_string();
        let digit_w = 6_i32;
        let digit_h = 10_i32;
        let spacing = 2_i32;
        let text_w = text.len() as i32 * digit_w + (text.len().saturating_sub(1) as i32 * spacing);
        let x = (rect.x + rect.width - text_w as f32 - 2.0).round() as i32;
        let y = (rect.y + rect.height - digit_h as f32 - 2.0).round() as i32;
        let mut cursor_x = x;
        for ch in text.chars() {
            Self::draw_stack_digit(buffer, cursor_x + 1, y + 1, ch, &[18, 18, 18, 210]);
            Self::draw_stack_digit(buffer, cursor_x, y, ch, &[174, 179, 183, 255]);
            cursor_x += digit_w + spacing;
        }
    }

    fn draw_stack_digit(buffer: &mut TheRGBABuffer, x: i32, y: i32, ch: char, color: &[u8; 4]) {
        let pattern = match ch {
            '0' => ["111", "101", "101", "101", "111"],
            '1' => ["010", "110", "010", "010", "111"],
            '2' => ["111", "001", "111", "100", "111"],
            '3' => ["111", "001", "111", "001", "111"],
            '4' => ["101", "101", "111", "001", "001"],
            '5' => ["111", "100", "111", "001", "111"],
            '6' => ["111", "100", "111", "101", "111"],
            '7' => ["111", "001", "010", "010", "010"],
            '8' => ["111", "101", "111", "101", "111"],
            '9' => ["111", "101", "111", "001", "111"],
            _ => return,
        };
        let stride = buffer.stride();
        let pixels = buffer.pixels_mut();
        for (py, row) in pattern.iter().enumerate() {
            for (px, bit) in row.chars().enumerate() {
                if bit != '1' {
                    continue;
                }
                for oy in 0..2 {
                    for ox in 0..2 {
                        let sx = x + px as i32 * 2 + ox;
                        let sy = y + py as i32 * 2 + oy;
                        if sx < 0 || sy < 0 {
                            continue;
                        }
                        let index = (sy as usize * stride + sx as usize) * 4;
                        if index + 3 >= pixels.len() {
                            continue;
                        }
                        pixels[index..index + 4].copy_from_slice(color);
                    }
                }
            }
        }
    }

    pub(crate) fn item_generated_icon_square(
        assets: &Assets,
        item: &Item,
    ) -> Option<(u32, Vec<u8>)> {
        if !AvatarRuntimeBuilder::item_allows_generated_icon(item, assets) {
            return None;
        }
        Self::item_avatar_channel_icon_square(assets, item)
            .or_else(|| Self::item_template_mask_icon_square(assets, item))
            .or_else(|| Self::item_equipment_icon_square(assets, item))
            .or_else(|| Self::item_icon_texture_square(assets, item))
    }

    pub(crate) fn item_avatar_channel_icon_square(
        assets: &Assets,
        item: &Item,
    ) -> Option<(u32, Vec<u8>)> {
        let Some(Value::StrArray(channels)) = item.attributes.get("avatar_channels") else {
            return None;
        };
        if channels.is_empty() {
            return None;
        }

        let mut color = Self::item_icon_color(assets, item, [188, 173, 159, 255]);
        color[3] = 255;
        let (icon, (width, height)) =
            Self::avatar_channel_icon_texture(assets, item, channels, color)?;
        let size = width.max(height).max(1);
        if width == size && height == size {
            return Some((size as u32, icon));
        }

        let mut square = vec![0; size * size * 4];
        let offset_x = (size - width) / 2;
        let offset_y = (size - height) / 2;
        for y in 0..height {
            let src = y * width * 4;
            let dst = ((y + offset_y) * size + offset_x) * 4;
            square[dst..dst + width * 4].copy_from_slice(&icon[src..src + width * 4]);
        }
        Some((size as u32, square))
    }

    fn draw_generated_avatar_channel_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
    ) -> bool {
        let Some(Value::StrArray(channels)) = item.attributes.get("avatar_channels") else {
            return false;
        };
        if channels.is_empty() {
            return false;
        }

        let mut color = Self::item_icon_color(assets, item, [188, 173, 159, 255]);
        color[3] = 255;

        let Some((icon, source_size)) =
            Self::avatar_channel_icon_texture(assets, item, channels, color)
        else {
            return false;
        };
        let dest = Self::fit_rect(rect, source_size);
        let stride = buffer.stride();
        draw2d.blend_scale_chunk(buffer.pixels_mut(), &dest, stride, &icon, &source_size);
        true
    }

    fn draw_item_template_mask_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
    ) -> bool {
        let Some((size, icon)) = Self::item_template_mask_icon_square(assets, item) else {
            return false;
        };
        let dest = Self::fit_rect(rect, (size as usize, size as usize));
        let stride = buffer.stride();
        draw2d.blend_scale_chunk(
            buffer.pixels_mut(),
            &dest,
            stride,
            &icon,
            &(size as usize, size as usize),
        );
        true
    }

    fn item_template_mask_icon_square(assets: &Assets, item: &Item) -> Option<(u32, Vec<u8>)> {
        let mut blade = Self::item_role_color(
            assets,
            item,
            "blade",
            Self::item_icon_color(assets, item, [187, 195, 208, 255]),
        );
        blade[3] = 255;
        let mut grip = Self::item_role_color(assets, item, "grip", [165, 120, 80, 255]);
        grip[3] = 255;
        let mut accent = Self::item_role_color(assets, item, "accent", [48, 56, 67, 255]);
        accent[3] = 255;
        let mut highlight = Self::item_role_color(assets, item, "highlight", [241, 246, 240, 255]);
        highlight[3] = 255;

        Self::item_template_mask_square(item, blade, grip, accent, highlight)
    }

    fn draw_item_icon_texture(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
    ) -> bool {
        let Some((size, icon)) = Self::item_icon_texture_square(assets, item) else {
            return false;
        };
        let dest = Self::fit_rect(rect, (size as usize, size as usize));
        let stride = buffer.stride();
        draw2d.blend_scale_chunk(
            buffer.pixels_mut(),
            &dest,
            stride,
            &icon,
            &(size as usize, size as usize),
        );
        true
    }

    fn item_icon_texture_square(assets: &Assets, item: &Item) -> Option<(u32, Vec<u8>)> {
        let icon_id = item
            .attributes
            .get_str("icon")
            .or_else(|| item.attributes.get_str("icon_template"))?
            .trim();
        if icon_id.is_empty() {
            return None;
        }
        let texture = assets.textures.get(icon_id).or_else(|| {
            assets
                .rules
                .parse::<Table>()
                .ok()
                .map(|root| Self::resolve_icon_texture_id(&root, icon_id))
                .and_then(|texture_id| assets.textures.get(texture_id.as_str()))
        })?;
        let mut color = Self::item_icon_color(assets, item, [216, 216, 216, 255]);
        color[3] = 255;

        let size = texture.width.max(texture.height).max(1);
        let offset_x = (size - texture.width) / 2;
        let offset_y = (size - texture.height) / 2;
        let mut icon = vec![0_u8; size * size * 4];
        for y in 0..texture.height {
            for x in 0..texture.width {
                let src = (y * texture.width + x) * 4;
                if src + 3 >= texture.data.len() {
                    continue;
                }
                let alpha = texture.data[src + 3];
                if alpha == 0 {
                    continue;
                }
                let shade = texture.data[src]
                    .max(texture.data[src + 1])
                    .max(texture.data[src + 2]) as u16;
                let dst = ((y + offset_y) * size + x + offset_x) * 4;
                icon[dst] = ((color[0] as u16 * shade) / 255) as u8;
                icon[dst + 1] = ((color[1] as u16 * shade) / 255) as u8;
                icon[dst + 2] = ((color[2] as u16 * shade) / 255) as u8;
                icon[dst + 3] = ((alpha as u16 * color[3] as u16) / 255) as u8;
            }
        }
        Some((size as u32, icon))
    }

    fn draw_generated_equipment_icon(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        assets: &Assets,
        item: &Item,
        draw2d: &Draw2D,
    ) -> bool {
        let Some((size, icon)) = Self::item_equipment_icon_square(assets, item) else {
            return false;
        };
        let dest = Self::fit_rect(rect, (size as usize, size as usize));
        let stride = buffer.stride();
        draw2d.blend_scale_chunk(
            buffer.pixels_mut(),
            &dest,
            stride,
            &icon,
            &(size as usize, size as usize),
        );
        true
    }

    fn item_equipment_icon_square(assets: &Assets, item: &Item) -> Option<(u32, Vec<u8>)> {
        let category = item
            .attributes
            .get_str("category")
            .or_else(|| item.attributes.get_str("ruleset_kind"))
            .or_else(|| item.attributes.get_str("slot"))?
            .trim()
            .to_ascii_lowercase();
        let template = item
            .attributes
            .get_str("icon_template")
            .or_else(|| item.attributes.get_str("visual_template"))
            .or_else(|| item.attributes.get_str("rig_template"))
            .unwrap_or(&category)
            .trim()
            .to_ascii_lowercase();
        let mut blade = Self::item_role_color(
            assets,
            item,
            "blade",
            Self::item_icon_color(assets, item, [187, 195, 208, 255]),
        );
        blade[3] = 255;
        let mut grip = Self::item_role_color(assets, item, "grip", [165, 120, 80, 255]);
        grip[3] = 255;
        let mut accent = Self::item_role_color(assets, item, "accent", [48, 56, 67, 255]);
        accent[3] = 255;
        let mut highlight = Self::item_role_color(assets, item, "highlight", [241, 246, 240, 255]);
        highlight[3] = 255;

        if let Some(icon) = Self::item_template_mask_square(item, blade, grip, accent, highlight) {
            return Some(icon);
        }

        if template == "sword_diagonal" {
            let mut icon = vec![0_u8; 16 * 16 * 4];
            Self::draw_icon_line(&mut icon, 16, 4, 11, 12, 3, blade);
            Self::draw_icon_line(&mut icon, 16, 5, 11, 13, 3, blade);
            Self::draw_icon_pixel(&mut icon, 16, 13, 2, highlight);
            Self::draw_icon_line(&mut icon, 16, 8, 13, 12, 9, accent);
            Self::draw_icon_pixel(&mut icon, 16, 7, 12, accent);
            Self::draw_icon_rect(&mut icon, 16, 2, 13, 3, 2, grip);
            Self::draw_icon_pixel(&mut icon, 16, 1, 15, grip);
            return Some((16, icon));
        }

        let mut icon = vec![0_u8; 24 * 24 * 4];

        match template.as_str() {
            "sword" => {
                Self::draw_icon_line(&mut icon, 24, 7, 17, 16, 8, blade);
                Self::draw_icon_line(&mut icon, 24, 8, 17, 17, 8, blade);
                Self::draw_icon_line(&mut icon, 24, 13, 20, 18, 15, accent);
                Self::draw_icon_rect(&mut icon, 24, 5, 18, 4, 3, grip);
                Self::draw_icon_pixel(&mut icon, 24, 18, 7, highlight);
            }
            "axe" => {
                Self::draw_icon_line(&mut icon, 24, 8, 20, 16, 8, grip);
                Self::draw_icon_rect(&mut icon, 24, 13, 5, 6, 6, blade);
                Self::draw_icon_pixel(&mut icon, 24, 12, 7, blade);
                Self::draw_icon_pixel(&mut icon, 24, 19, 8, blade);
                Self::draw_icon_pixel(&mut icon, 24, 17, 5, highlight);
            }
            "mace" => {
                Self::draw_icon_line(&mut icon, 24, 8, 20, 16, 8, grip);
                Self::draw_icon_rect(&mut icon, 24, 14, 5, 6, 6, blade);
                Self::draw_icon_pixel(&mut icon, 24, 13, 7, blade);
                Self::draw_icon_pixel(&mut icon, 24, 20, 7, blade);
                Self::draw_icon_pixel(&mut icon, 24, 17, 4, highlight);
            }
            "shield" => {
                Self::draw_icon_rect(&mut icon, 24, 7, 5, 10, 12, blade);
                Self::draw_icon_rect(&mut icon, 24, 8, 4, 8, 14, blade);
                Self::draw_icon_rect(&mut icon, 24, 10, 6, 4, 10, grip);
                Self::draw_icon_rect(&mut icon, 24, 11, 5, 2, 12, highlight);
                Self::draw_icon_pixel(&mut icon, 24, 11, 18, blade);
                Self::draw_icon_pixel(&mut icon, 24, 12, 18, blade);
            }
            "bow" => {
                for y in 4..20 {
                    let x = if y < 9 {
                        8
                    } else if y < 15 {
                        7
                    } else {
                        8
                    };
                    Self::draw_icon_pixel(&mut icon, 24, x, y, grip);
                }
                Self::draw_icon_line(&mut icon, 24, 15, 5, 15, 19, highlight);
            }
            _ => return None,
        }

        Some((24, icon))
    }

    fn item_template_mask_square(
        item: &Item,
        blade: [u8; 4],
        grip: [u8; 4],
        accent: [u8; 4],
        highlight: [u8; 4],
    ) -> Option<(u32, Vec<u8>)> {
        let width = item.attributes.get_int("visual_template_width")? as usize;
        let height = item.attributes.get_int("visual_template_height")? as usize;
        let Some(Value::StrArray(rows)) = item.attributes.get("visual_template_pixels") else {
            return None;
        };
        if width == 0 || height == 0 || rows.len() != height {
            return None;
        }

        let size = width.max(height);
        let offset_x = (size - width) / 2;
        let offset_y = (size - height) / 2;
        let mut icon = vec![0_u8; size * size * 4];
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return None;
            }
            for (x, ch) in row.chars().enumerate() {
                let color = match ch {
                    'B' | 'b' => blade,
                    'G' | 'g' => grip,
                    'A' | 'a' => accent,
                    'H' | 'h' => highlight,
                    '.' | ' ' => continue,
                    _ => continue,
                };
                let i = ((y + offset_y) * size + x + offset_x) * 4;
                icon[i..i + 4].copy_from_slice(&color);
            }
        }

        Some((size as u32, icon))
    }

    fn avatar_channel_icon_texture(
        assets: &Assets,
        item: &Item,
        channels: &[String],
        color: [u8; 4],
    ) -> Option<(Vec<u8>, (usize, usize))> {
        let avatar = Self::item_icon_avatar(assets, item)?;
        let frame = avatar
            .animations
            .iter()
            .find(|animation| animation.name.eq_ignore_ascii_case("idle"))
            .or_else(|| avatar.animations.first())?
            .perspectives
            .iter()
            .find(|perspective| perspective.direction == crate::AvatarDirection::Front)
            .or_else(|| {
                avatar
                    .animations
                    .iter()
                    .find(|animation| animation.name.eq_ignore_ascii_case("idle"))
                    .or_else(|| avatar.animations.first())
                    .and_then(|animation| animation.perspectives.first())
            })?
            .frames
            .first()?;

        let width = frame.texture.width;
        let height = frame.texture.height;
        let selected = Self::selected_avatar_marker_channels(channels);
        if !selected.iter().any(|selected| *selected) {
            return None;
        }

        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut marker_min_y = [usize::MAX; 9];
        let mut marker_max_y = [0usize; 9];
        let mut mask = vec![None; width * height];

        for (i, pixel) in frame.texture.data.chunks_exact(4).enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let Some(marker) = Self::avatar_marker_channel(pixel) else {
                continue;
            };
            if !selected[marker] {
                continue;
            }
            let x = i % width;
            let y = i / width;
            mask[i] = Some(marker);
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            marker_min_y[marker] = marker_min_y[marker].min(y);
            marker_max_y[marker] = marker_max_y[marker].max(y);
        }

        if min_x > max_x || min_y > max_y {
            return None;
        }

        let pad = 1usize;
        let out_width = (max_x - min_x + 1) + pad * 2;
        let out_height = (max_y - min_y + 1) + pad * 2;
        let mut out = vec![0; out_width * out_height * 4];
        let outline = Self::shade_color(color, -64);
        let ramp = Self::build_item_icon_shade_ramp(color);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let i = y * width + x;
                if mask[i].is_none() {
                    continue;
                }
                let ox = x - min_x + pad;
                let oy = y - min_y + pad;
                for (nx, ny) in [
                    (ox.wrapping_sub(1), oy),
                    (ox + 1, oy),
                    (ox, oy.wrapping_sub(1)),
                    (ox, oy + 1),
                ] {
                    if nx >= out_width || ny >= out_height {
                        continue;
                    }
                    let ni = (ny * out_width + nx) * 4;
                    if out[ni + 3] == 0 {
                        out[ni..ni + 4].copy_from_slice(&outline);
                    }
                }
            }
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let i = y * width + x;
                let Some(marker) = mask[i] else {
                    continue;
                };
                let y0 = marker_min_y[marker];
                let y1 = marker_max_y[marker];
                let local_y = if y0 == usize::MAX || y1 <= y0 {
                    0.5
                } else {
                    (y.saturating_sub(y0)) as f32 / (y1 - y0) as f32
                };
                let shade = Self::item_icon_shade_index(x, y, local_y, marker as u32);
                let ox = x - min_x + pad;
                let oy = y - min_y + pad;
                let oi = (oy * out_width + ox) * 4;
                out[oi..oi + 4].copy_from_slice(&ramp[shade]);
            }
        }

        Some((out, (out_width, out_height)))
    }

    fn item_icon_avatar<'a>(assets: &'a Assets, item: &Item) -> Option<&'a crate::Avatar> {
        item.attributes
            .get_str("icon_avatar")
            .or_else(|| item.attributes.get_str("avatar"))
            .or_else(|| assets.default_avatar.as_deref())
            .and_then(|name| assets.avatars.get(name.trim()))
            .or_else(|| assets.avatars.get("humanoid"))
            .or_else(|| assets.avatars.values().next())
    }

    fn selected_avatar_marker_channels(channels: &[String]) -> [bool; 9] {
        let mut selected = [false; 9];
        for channel in channels {
            match channel.trim().to_ascii_lowercase().as_str() {
                "skin" | "head" | "face" => {
                    selected[0] = true;
                    selected[1] = true;
                }
                "torso" => selected[2] = true,
                "arms" => selected[3] = true,
                "legs" => selected[4] = true,
                "hair" => selected[5] = true,
                "eyes" => selected[6] = true,
                "hands" => selected[7] = true,
                "feet" => selected[8] = true,
                _ => {}
            }
        }
        selected
    }

    fn avatar_marker_channel(pixel: &[u8]) -> Option<usize> {
        match [pixel[0], pixel[1], pixel[2]] {
            [255, 0, 255] => Some(0),
            [200, 0, 200] => Some(1),
            [0, 0, 255] => Some(2),
            [0, 120, 255] => Some(3),
            [0, 255, 0] => Some(4),
            [255, 255, 0] => Some(5),
            [0, 255, 255] => Some(6),
            [255, 128, 0] => Some(7),
            [255, 80, 0] => Some(8),
            _ => None,
        }
    }

    fn fit_rect(rect: Rect, source_size: (usize, usize)) -> (usize, usize, usize, usize) {
        let rect_width = rect.width.max(1.0);
        let rect_height = rect.height.max(1.0);
        let scale = (rect_width / source_size.0.max(1) as f32)
            .min(rect_height / source_size.1.max(1) as f32)
            .max(0.01);
        let width = (source_size.0 as f32 * scale)
            .round()
            .min(rect_width)
            .max(1.0) as usize;
        let height = (source_size.1 as f32 * scale)
            .round()
            .min(rect_height)
            .max(1.0) as usize;
        let x = (rect.x + (rect_width - width as f32) * 0.5)
            .round()
            .max(0.0) as usize;
        let y = (rect.y + (rect_height - height as f32) * 0.5)
            .round()
            .max(0.0) as usize;
        (x, y, width, height)
    }

    fn build_item_icon_shade_ramp(base: [u8; 4]) -> [[u8; 4]; 4] {
        [
            Self::modulate_icon_color(base, 1.18),
            Self::modulate_icon_color(base, 1.0),
            Self::modulate_icon_color(base, 0.82),
            Self::modulate_icon_color(base, 0.64),
        ]
    }

    fn modulate_icon_color(base: [u8; 4], factor: f32) -> [u8; 4] {
        [
            (base[0] as f32 * factor).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * factor).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * factor).clamp(0.0, 255.0) as u8,
            base[3],
        ]
    }

    fn item_icon_shade_index(x: usize, y: usize, local_y: f32, marker_seed: u32) -> usize {
        const BAYER4: [f32; 16] = [
            0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
        ];
        let d = BAYER4[(y & 3) * 4 + (x & 3)] / 15.0;
        let bias = (marker_seed % 3) as f32 * 0.03;
        (local_y.clamp(0.0, 1.0) * 2.7 + d * 0.6 + bias).clamp(0.0, 3.0) as usize
    }

    fn draw_icon_pixel(rgba: &mut [u8], width: usize, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if width == 0 || x >= width || y >= rgba.len() / (width * 4) {
            return;
        }
        let i = (y * width + x) * 4;
        rgba[i..i + 4].copy_from_slice(&color);
    }

    fn draw_icon_rect(
        rgba: &mut [u8],
        width: usize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: [u8; 4],
    ) {
        for yy in y..y + h {
            for xx in x..x + w {
                Self::draw_icon_pixel(rgba, width, xx, yy, color);
            }
        }
    }

    fn draw_icon_line(
        rgba: &mut [u8],
        width: usize,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: [u8; 4],
    ) {
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            Self::draw_icon_pixel(rgba, width, x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn item_icon_color(assets: &Assets, item: &Item, fallback: [u8; 4]) -> [u8; 4] {
        if let Some(Value::Color(color)) = item.attributes.get("icon_color") {
            return color.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("icon_color") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(Value::Color(color)) = item.attributes.get("color") {
            return color.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str("color") {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(index) = item
            .attributes
            .get_int("icon_color")
            .or_else(|| item.attributes.get_int("color"))
            .or_else(|| item.attributes.get_int("color_index"))
        {
            return Self::palette_color(assets, index, fallback);
        }
        fallback
    }

    fn item_role_color(assets: &Assets, item: &Item, role: &str, fallback: [u8; 4]) -> [u8; 4] {
        let color_key = format!("{role}_color");
        let index_key = format!("{role}_color_index");
        if let Some(Value::Color(color)) = item.attributes.get(&color_key) {
            return color.to_u8_array();
        }
        if let Some(hex) = item.attributes.get_str(&color_key) {
            return TheColor::from_hex(hex).to_u8_array();
        }
        if let Some(index) = item.attributes.get_int(&color_key) {
            return Self::palette_color(assets, index, fallback);
        }
        if let Some(index) = item.attributes.get_int(&index_key) {
            return Self::palette_color(assets, index, fallback);
        }
        fallback
    }

    fn palette_color(assets: &Assets, index: i32, fallback: [u8; 4]) -> [u8; 4] {
        if index < 0 {
            return fallback;
        }
        let index = index as usize;
        if index < assets.palette.colors.len()
            && let Some(color) = &assets.palette[index]
        {
            return color.to_u8_array();
        }
        fallback
    }

    fn shade_color(color: [u8; 4], delta: i16) -> [u8; 4] {
        [
            (color[0] as i16 + delta).clamp(0, 255) as u8,
            (color[1] as i16 + delta).clamp(0, 255) as u8,
            (color[2] as i16 + delta).clamp(0, 255) as u8,
            color[3],
        ]
    }
}
