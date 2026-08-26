pub mod avatar;
pub mod deco;
pub mod game;
pub mod game_backend;
pub mod messages;
pub mod profile;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BorderGradientDirection {
    Horizontal,
    #[default]
    Vertical,
    Diagonal,
    DiagonalReverse,
}

impl BorderGradientDirection {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "horizontal" | "left_to_right" => Self::Horizontal,
            "diagonal" | "top_left_to_bottom_right" => Self::Diagonal,
            "diagonal_reverse" | "top_right_to_bottom_left" => Self::DiagonalReverse,
            _ => Self::Vertical,
        }
    }
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
    pub binding_append: bool,
    pub binding_separator: String,
    pub binding_max_parts: Option<usize>,
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
    /// Optional button chrome drawn below command or item artwork.
    pub chrome_textures: Vec<Texture>,
    /// Source-pixel inset used to nine-slice the button chrome.
    pub chrome_slice: usize,
    /// Optional decorative artwork drawn above command or item artwork.
    pub frame_textures: Vec<Texture>,
    /// Source-pixel inset used to nine-slice the foreground frame.
    pub frame_slice: usize,
    /// Optional pixel inset for command artwork. Defaults to 12% of the slot size.
    pub icon_inset: Option<f32>,
    pub textures: Vec<Texture>,
    pub entity_cursor_id: Option<Uuid>,
    pub entity_clicked_cursor_id: Option<Uuid>,
    pub item_cursor_id: Option<Uuid>,
    pub item_clicked_cursor_id: Option<Uuid>,
    pub border_color: Pixel,
    pub border_size: i32,
    pub border_gradient_color: Option<Pixel>,
    pub border_gradient_direction: BorderGradientDirection,
    pub border_radius: f32,
    pub border_painter: ThePainter,
    pub show_icon: bool,
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
    fn resolved_font<'a>(&self, assets: &'a Assets) -> Option<&'a fontdue::Font> {
        if let Some(font) = assets
            .fonts
            .get(self.font.trim())
            .or_else(|| assets.fonts.values().next())
        {
            Some(font)
        } else {
            Widget::fallback_font()
        }
    }

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

        if let Some(font) = self.resolved_font(assets) {
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

fn texture_axis_coordinate(
    position: usize,
    destination_len: usize,
    source_len: usize,
    slice: usize,
) -> usize {
    if destination_len == 0 || source_len == 0 {
        return 0;
    }

    let source_slice = slice.min(source_len.saturating_sub(1) / 2);
    let destination_slice = source_slice.min(destination_len / 2);
    if source_slice == 0 || destination_slice == 0 {
        return position.saturating_mul(source_len) / destination_len;
    }

    if position < destination_slice {
        return position.saturating_mul(source_slice) / destination_slice;
    }
    if position >= destination_len - destination_slice {
        let edge_position = position - (destination_len - destination_slice);
        return source_len - source_slice
            + edge_position.saturating_mul(source_slice) / destination_slice;
    }

    let destination_middle = destination_len - destination_slice * 2;
    let source_middle = source_len - source_slice * 2;
    source_slice
        + (position - destination_slice).saturating_mul(source_middle) / destination_middle.max(1)
}

/// Draws a texture as a normal scaled layer or as a nine-slice when `slice` is non-zero.
pub(crate) fn blend_texture_layer(
    buffer: &mut TheRGBABuffer,
    rect: Rect,
    draw2d: &Draw2D,
    texture: &Texture,
    slice: usize,
) {
    let destination_width = rect.width.round().max(0.0) as usize;
    let destination_height = rect.height.round().max(0.0) as usize;
    if destination_width == 0
        || destination_height == 0
        || texture.width == 0
        || texture.height == 0
    {
        return;
    }

    let destination_x = rect.x.round() as isize;
    let destination_y = rect.y.round() as isize;
    let buffer_width = buffer.dim().width.max(0) as isize;
    let buffer_height = buffer.dim().height.max(0) as isize;
    let stride = buffer.stride();
    let frame = buffer.pixels_mut();

    for y in 0..destination_height {
        let frame_y = destination_y + y as isize;
        if frame_y < 0 || frame_y >= buffer_height {
            continue;
        }
        let source_y = texture_axis_coordinate(y, destination_height, texture.height, slice)
            .min(texture.height - 1);
        for x in 0..destination_width {
            let frame_x = destination_x + x as isize;
            if frame_x < 0 || frame_x >= buffer_width {
                continue;
            }
            let source_x = texture_axis_coordinate(x, destination_width, texture.width, slice)
                .min(texture.width - 1);
            let source_index = (source_x + source_y * texture.width) * 4;
            if source_index + 3 >= texture.data.len() {
                continue;
            }
            let alpha = texture.data[source_index + 3];
            if alpha == 0 {
                continue;
            }
            let frame_index = (frame_x as usize + frame_y as usize * stride) * 4;
            if frame_index + 3 >= frame.len() {
                continue;
            }
            let source = [
                texture.data[source_index],
                texture.data[source_index + 1],
                texture.data[source_index + 2],
                alpha,
            ];
            let background = [
                frame[frame_index],
                frame[frame_index + 1],
                frame[frame_index + 2],
                frame[frame_index + 3],
            ];
            frame[frame_index..frame_index + 4].copy_from_slice(&draw2d.mix_color(
                &background,
                &source,
                alpha as f32 / 255.0,
            ));
        }
    }
}

pub(crate) fn draw_widget_border(
    buffer: &mut TheRGBABuffer,
    rect: Rect,
    draw2d: &Draw2D,
    painter: &mut ThePainter,
    border_size: i32,
    start_color: Pixel,
    end_color: Option<Pixel>,
    direction: BorderGradientDirection,
    radius: f32,
) {
    if border_size <= 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    if end_color.is_none() && radius <= 0.0 {
        let stride = buffer.stride();
        draw2d.rect_outline_thickness(
            buffer.pixels_mut(),
            &(
                rect.x.max(0.0) as usize,
                rect.y.max(0.0) as usize,
                rect.width.max(0.0) as usize,
                rect.height.max(0.0) as usize,
            ),
            stride,
            &start_color,
            border_size as usize,
        );
        return;
    }

    let surface_width = buffer.dim().width.max(0) as usize;
    let surface_height = buffer.dim().height.max(0) as usize;
    let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), surface_width, surface_height)
    else {
        return;
    };
    let stroke_width = border_size as f32;
    let inset = stroke_width * 0.5;
    let path_width = (rect.width - stroke_width).max(0.0);
    let path_height = (rect.height - stroke_width).max(0.0);
    if path_width <= 0.0 || path_height <= 0.0 {
        return;
    }

    let mut path = ThePath::new();
    let path_origin = (rect.x + inset, rect.y + inset);
    if radius > 0.0 {
        let radius = radius.max(inset).min(path_width.min(path_height) * 0.5);
        path.add_round_rect(path_origin, path_width, path_height, radius, radius);
    } else {
        path.add_rect(path_origin, path_width, path_height);
    }

    let paint = if let Some(end_color) = end_color {
        let left = rect.x;
        let top = rect.y;
        let right = rect.x + rect.width;
        let bottom = rect.y + rect.height;
        let (start, end) = match direction {
            BorderGradientDirection::Horizontal => ([left, top], [right, top]),
            BorderGradientDirection::Vertical => ([left, top], [left, bottom]),
            BorderGradientDirection::Diagonal => ([left, top], [right, bottom]),
            BorderGradientDirection::DiagonalReverse => ([right, top], [left, bottom]),
        };
        ThePaint::linear_gradient(start, end, start_color, end_color)
    } else {
        ThePaint::solid(start_color)
    };
    painter.stroke_path(
        &mut surface,
        &path,
        &ThePathStroke::new(stroke_width, paint).with_join(TheLineJoin::Round),
    );
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
            binding_append: false,
            binding_separator: " ".to_string(),
            binding_max_parts: None,
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
            chrome_textures: vec![],
            chrome_slice: 0,
            frame_textures: vec![],
            frame_slice: 0,
            icon_inset: None,
            textures: vec![],
            entity_cursor_id: None,
            entity_clicked_cursor_id: None,
            item_cursor_id: None,
            item_clicked_cursor_id: None,
            border_color: WHITE,
            border_size: 0,
            border_gradient_color: None,
            border_gradient_direction: BorderGradientDirection::Vertical,
            border_radius: 0.0,
            border_painter: ThePainter::new(),
            show_icon: true,
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
        let state_style = self.state_style(visual_state);
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

        if !self.chrome_textures.is_empty() {
            let texture_index =
                Self::texture_index_for_state(self.chrome_textures.len(), visual_state);
            let texture = &self.chrome_textures[texture_index];
            blend_texture_layer(buffer, self.rect, draw2d, texture, self.chrome_slice);
        }

        let is_item_slot = self.inventory_index.is_some() || self.equipped_slot.is_some();
        let mut drew_primary_texture = false;
        if self.show_icon
            && !is_item_slot
            && let Some(texture) = Self::command_icon_texture(
                assets,
                resolved_command.or(self.command.as_deref()),
                visual_state,
            )
        {
            Self::draw_command_icon_texture(
                buffer,
                self.rect,
                draw2d,
                texture,
                visual_state,
                self.icon_inset,
            );
            drew_primary_texture = true;
        }
        if !drew_primary_texture && !self.textures.is_empty() {
            let texture_index = Self::texture_index_for_state(self.textures.len(), visual_state);
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

        if !self.frame_textures.is_empty() {
            let texture_index =
                Self::texture_index_for_state(self.frame_textures.len(), visual_state);
            blend_texture_layer(
                buffer,
                self.rect,
                draw2d,
                &self.frame_textures[texture_index],
                self.frame_slice,
            );
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
            draw_widget_border(
                buffer,
                self.rect,
                draw2d,
                &mut self.border_painter,
                self.border_size,
                border_color,
                self.border_gradient_color,
                self.border_gradient_direction,
                self.border_radius,
            );
        }

        self.draw_label(buffer, assets, draw2d, visual_state);
    }

    fn state_style(&self, visual_state: ButtonVisualState) -> ButtonStateStyle {
        match visual_state {
            ButtonVisualState::Normal => ButtonStateStyle::default(),
            ButtonVisualState::Hover => self.hover_style,
            ButtonVisualState::Pressed => self.pressed_style,
            ButtonVisualState::Selected => self.selected_style,
            ButtonVisualState::Disabled => self.disabled_style,
        }
    }

    pub fn draw_label(
        &self,
        buffer: &mut TheRGBABuffer,
        assets: &Assets,
        draw2d: &Draw2D,
        visual_state: ButtonVisualState,
    ) {
        if self.label.trim().is_empty() {
            return;
        }

        let fallback = Self::fallback_font();
        let font = if self.label_font.trim().is_empty() {
            assets.fonts.values().next().or(fallback)
        } else {
            assets
                .fonts
                .get(self.label_font.trim())
                .or_else(|| assets.fonts.values().next())
                .or(fallback)
        };

        let Some(font) = font else {
            return;
        };

        let stride = buffer.stride();
        let buffer_width = buffer.dim().width as isize;
        let buffer_height = buffer.dim().height as isize;
        let state_style = self.state_style(visual_state);
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

    fn fallback_font() -> Option<&'static fontdue::Font> {
        static FALLBACK_FONT: std::sync::OnceLock<Option<fontdue::Font>> =
            std::sync::OnceLock::new();
        FALLBACK_FONT
            .get_or_init(|| {
                theframework::Embedded::get("fonts/Roboto-Bold.ttf").and_then(|font_bytes| {
                    fontdue::Font::from_bytes(font_bytes.data, fontdue::FontSettings::default())
                        .ok()
                })
            })
            .as_ref()
    }

    pub(crate) fn command_icon_texture<'a>(
        assets: &'a Assets,
        command: Option<&str>,
        visual_state: ButtonVisualState,
    ) -> Option<&'a Texture> {
        let root = assets.rules_table()?;
        let command = command?;
        let resolved_action_icon = match parse_client_command(command) {
            Some(ClientCommandBinding::RulesAction(action_id)) => {
                eldiron_ruleset::resolve_action(&root, &action_id)
                    .ok()
                    .flatten()
                    .and_then(|action| eldiron_ruleset::resolve_action_icon(&root, &action))
            }
            _ => None,
        };
        let command_table = Self::command_icon_table(&root, command)?;
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
                    .map(str::to_string)
            })
            .or_else(|| {
                ui.and_then(|ui| ui.get("icon"))
                    .or_else(|| command_table.get("icon"))
                    .and_then(toml::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
            .or(resolved_action_icon)?;
        let icon_name = Self::resolve_icon_texture_id(&root, &icon_name);

        assets.textures.get(icon_name.as_str())
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
            ClientCommandBinding::Ui(command) => {
                table_at(root, &["ui", "commands", command.trim()])
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

    fn draw_command_icon_texture(
        buffer: &mut TheRGBABuffer,
        rect: Rect,
        draw2d: &Draw2D,
        texture: &Texture,
        visual_state: ButtonVisualState,
        configured_inset: Option<f32>,
    ) {
        let stride = buffer.stride();
        let inset = configured_inset
            .unwrap_or_else(|| (rect.width.min(rect.height) * 0.12).round().max(2.0))
            .max(0.0);
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
                let treated = Self::command_icon_pixel(
                    [
                        texture.data[s],
                        texture.data[s + 1],
                        texture.data[s + 2],
                        source_alpha,
                    ],
                    visual_state,
                );
                let background = [frame[d], frame[d + 1], frame[d + 2], frame[d + 3]];
                frame[d..d + 4].copy_from_slice(&draw2d.mix_color(
                    &background,
                    &treated,
                    treated[3] as f32 / 255.0,
                ));
            }
        }
    }

    fn command_icon_pixel(source: Pixel, visual_state: ButtonVisualState) -> Pixel {
        match visual_state {
            ButtonVisualState::Normal => source,
            ButtonVisualState::Disabled => {
                let gray = (source[0] as f32 * 0.299
                    + source[1] as f32 * 0.587
                    + source[2] as f32 * 0.114)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                [gray, gray, gray, (source[3] as f32 * 0.58).round() as u8]
            }
            ButtonVisualState::Selected => {
                let selected = [255_u8, 226_u8, 150_u8];
                let mix = 0.34_f32;
                [
                    (source[0] as f32 * (1.0 - mix) + selected[0] as f32 * mix).round() as u8,
                    (source[1] as f32 * (1.0 - mix) + selected[1] as f32 * mix).round() as u8,
                    (source[2] as f32 * (1.0 - mix) + selected[2] as f32 * mix).round() as u8,
                    source[3],
                ]
            }
            ButtonVisualState::Hover => [
                (source[0] as f32 * 1.25).round().clamp(0.0, 255.0) as u8,
                (source[1] as f32 * 1.25).round().clamp(0.0, 255.0) as u8,
                (source[2] as f32 * 1.25).round().clamp(0.0, 255.0) as u8,
                source[3],
            ],
            ButtonVisualState::Pressed => [
                (source[0] as f32 * 0.76).round() as u8,
                (source[1] as f32 * 0.76).round() as u8,
                (source[2] as f32 * 0.76).round() as u8,
                source[3],
            ],
        }
    }

    fn item_icon_state(item: &Item) -> &'static str {
        if let Some(active) = item.attributes.get_bool("active") {
            return if active { "on" } else { "off" };
        }
        if let Some(state) = item.attributes.get_str("state") {
            return if state.trim().eq_ignore_ascii_case("off") {
                "off"
            } else {
                "on"
            };
        }
        "on"
    }

    fn item_icon_asset_keys(item: &Item) -> Vec<String> {
        let mut keys = vec![item.creator_id.to_string().to_ascii_lowercase()];
        keys.extend(
            [
                item.attributes.get_str("creator_template_id"),
                item.attributes.get_str("ruleset_path"),
                item.attributes.get_str("ruleset_id"),
                item.attributes.get_str("class_name"),
                item.attributes.get_str("name"),
                (!item.item_type.trim().is_empty()).then_some(item.item_type.as_str()),
            ]
            .into_iter()
            .flatten()
            .map(|key| key.trim().to_ascii_lowercase()),
        );
        keys
    }

    fn custom_item_icon_frames<'a>(assets: &'a Assets, item: &Item) -> Option<&'a Vec<Texture>> {
        let state = Self::item_icon_state(item);
        for key in Self::item_icon_asset_keys(item) {
            if let Some(frames) = assets.item_icons.get(&format!("{key}:{state}")) {
                return Some(frames);
            }
            if let Some(frames) = assets.item_icons.get(&key) {
                return Some(frames);
            }
        }
        None
    }

    pub(crate) fn project_item_icon_frames<'a>(
        assets: &'a Assets,
        item: &Item,
    ) -> Option<&'a Vec<Texture>> {
        let state = Self::item_icon_state(item);
        for key in Self::item_icon_asset_keys(item) {
            let state_key = format!("{key}:{state}");
            if assets.project_item_icon_keys.contains(&state_key)
                && let Some(frames) = assets.item_icons.get(&state_key)
            {
                return Some(frames);
            }
            if assets.project_item_icon_keys.contains(&key)
                && let Some(frames) = assets.item_icons.get(&key)
            {
                return Some(frames);
            }
        }
        None
    }

    fn texture_index_for_state(len: usize, visual_state: ButtonVisualState) -> usize {
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
            ButtonVisualState::Hover => {
                if len > 4 {
                    4
                } else {
                    0
                }
            }
            ButtonVisualState::Normal => 0,
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
        if let Some(frames) = Self::custom_item_icon_frames(assets, item)
            && !frames.is_empty()
        {
            let texture = &frames[animation_frame % frames.len()];
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
        if !drawn
            && let Some(Value::Source(source)) = item.attributes.get("source")
            && let Some(tile) = source.tile_from_tile_list(assets)
            && !tile.textures.is_empty()
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

        if !drawn
            && let Some(tile) = AvatarRuntimeBuilder::explicit_item_tile(item, assets)
            && !tile.textures.is_empty()
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

        // Ruleset PNGs are authoritative RGBA artwork. Prefer them over the
        // palette-driven generators, which now serve only as missing-art fallbacks.
        if !drawn && Self::draw_item_icon_texture(buffer, rect, assets, item, draw2d) {
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

    /// Resolve the same default item icon used by runtime UI consumers.
    ///
    /// Editor surfaces such as ruleset Help use this entry point so previews
    /// cannot drift from the icon chosen by the game client.
    pub fn item_generated_icon_square(assets: &Assets, item: &Item) -> Option<(u32, Vec<u8>)> {
        if let Some(icon) = Self::custom_item_icon_square(assets, item) {
            return Some(icon);
        }
        if let Some(icon) = Self::item_icon_texture_square(assets, item) {
            return Some(icon);
        }
        if !AvatarRuntimeBuilder::item_allows_generated_icon(item, assets) {
            return None;
        }
        Self::item_avatar_channel_icon_square(assets, item)
            .or_else(|| Self::item_template_mask_icon_square(assets, item))
            .or_else(|| Self::item_equipment_icon_square(assets, item))
    }

    fn custom_item_icon_square(assets: &Assets, item: &Item) -> Option<(u32, Vec<u8>)> {
        Self::item_icon_frames_square(Self::custom_item_icon_frames(assets, item)?, 0)
    }

    pub(crate) fn project_item_icon_square(
        assets: &Assets,
        item: &Item,
        animation_frame: usize,
    ) -> Option<(u32, Vec<u8>)> {
        Self::item_icon_frames_square(
            Self::project_item_icon_frames(assets, item)?,
            animation_frame,
        )
    }

    fn item_icon_frames_square(
        frames: &[Texture],
        animation_frame: usize,
    ) -> Option<(u32, Vec<u8>)> {
        let texture = frames.get(animation_frame % frames.len().max(1))?;
        let width = texture.width;
        let height = texture.height;
        let size = width.max(height).max(1);
        let expected = width.checked_mul(height)?.checked_mul(4)?;
        if texture.data.len() < expected {
            return None;
        }

        let size_u32 = u32::try_from(size).ok()?;
        if width == size && height == size {
            return Some((size_u32, texture.data[..expected].to_vec()));
        }

        let mut icon = vec![0_u8; size.checked_mul(size)?.checked_mul(4)?];
        let offset_x = (size - width) / 2;
        let offset_y = (size - height) / 2;
        for y in 0..height {
            let source_start = y * width * 4;
            let source_end = source_start + width * 4;
            let target_start = ((y + offset_y) * size + offset_x) * 4;
            let target_end = target_start + width * 4;
            icon[target_start..target_end].copy_from_slice(&texture.data[source_start..source_end]);
        }
        Some((size_u32, icon))
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
        // Every official item may own a PNG named after its ruleset id. This
        // avoids forcing action icons and visually richer item defaults to
        // share one semantic texture id.
        let authored_item_icon = item
            .attributes
            .get_str("ruleset_id")
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .filter(|id| assets.textures.contains_key(*id));
        let explicit_icon = item
            .attributes
            .get_str("icon")
            .or_else(|| item.attributes.get_str("icon_template"));
        let root = assets.rules_table();
        let icon_id = authored_item_icon.map(str::to_string).or_else(|| {
            root.as_ref()
                .and_then(|root| {
                    eldiron_ruleset::resolve_item_icon(
                        root,
                        item.attributes.get_str("ruleset_kind"),
                        explicit_icon,
                    )
                })
                .or_else(|| explicit_icon.map(str::to_string))
        })?;
        let texture = assets.textures.get(&icon_id).or_else(|| {
            root.as_ref()
                .map(|root| Self::resolve_icon_texture_id(root, &icon_id))
                .and_then(|texture_id| assets.textures.get(texture_id.as_str()))
        })?;
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
                let dst = ((y + offset_y) * size + x + offset_x) * 4;
                icon[dst..dst + 4].copy_from_slice(&texture.data[src..src + 4]);
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
            return Self::ruleset_palette_color(assets, index, fallback);
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
            return Self::ruleset_palette_color(assets, index, fallback);
        }
        if let Some(index) = item.attributes.get_int(&index_key) {
            return Self::ruleset_palette_color(assets, index, fallback);
        }
        fallback
    }

    fn ruleset_palette_color(assets: &Assets, index: i32, fallback: [u8; 4]) -> [u8; 4] {
        if index < 0 {
            return fallback;
        }
        let index = index as usize;
        if index < assets.ruleset_palette.colors.len()
            && let Some(color) = &assets.ruleset_palette[index]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_state_textures_can_override_hover_artwork() {
        assert_eq!(
            Widget::texture_index_for_state(5, ButtonVisualState::Hover),
            4
        );
        assert_eq!(
            Widget::texture_index_for_state(4, ButtonVisualState::Hover),
            0
        );
    }

    #[test]
    fn nine_slice_preserves_texture_corners_and_center() {
        let pixels: Vec<u8> = (0..9)
            .flat_map(|index| [index * 20, index * 20, index * 20, 255])
            .collect();
        let texture = Texture::new(pixels, 3, 3);
        let mut buffer = TheRGBABuffer::new(TheDim::sized(5, 5));

        blend_texture_layer(
            &mut buffer,
            Rect::new(0.0, 0.0, 5.0, 5.0),
            &Draw2D::default(),
            &texture,
            1,
        );

        let pixel = |x: usize, y: usize| buffer.pixels()[(x + y * 5) * 4];
        assert_eq!(pixel(0, 0), 0);
        assert_eq!(pixel(4, 0), 40);
        assert_eq!(pixel(2, 2), 80);
        assert_eq!(pixel(0, 4), 120);
        assert_eq!(pixel(4, 4), 160);
    }

    #[test]
    fn generated_item_icon_colors_use_ruleset_palette() {
        let mut assets = Assets::default();
        assets.ruleset_palette.colors[10] = Some(TheColor::from_u8(41, 82, 123, 255));
        assets.palette.colors[10] = Some(TheColor::from_u8(240, 96, 16, 255));

        let mut item = Item::default();
        item.attributes.set("color_index", Value::Int(10));
        item.attributes.set("blade_color_index", Value::Int(10));

        assert_eq!(
            Widget::item_icon_color(&assets, &item, [0, 0, 0, 255]),
            [41, 82, 123, 255]
        );
        assert_eq!(
            Widget::item_role_color(&assets, &item, "blade", [0, 0, 0, 255]),
            [41, 82, 123, 255]
        );
    }

    #[test]
    fn authored_item_icon_pixels_are_preserved() {
        let mut assets = Assets::default();
        assets.textures.insert(
            "authored".to_string(),
            Texture::new(vec![12, 34, 56, 78, 90, 123, 210, 255], 2, 1),
        );
        let mut item = Item::default();
        item.attributes.set("icon", Value::Str("authored".into()));

        let (size, pixels) = Widget::item_icon_texture_square(&assets, &item).unwrap();
        assert_eq!(size, 2);
        assert_eq!(&pixels[0..8], &[12, 34, 56, 78, 90, 123, 210, 255]);
    }

    #[test]
    fn ui_commands_resolve_replaceable_ruleset_icons() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [ui.commands.spellbook]
            icon = "custom_spellbook"
        "#
        .into();
        assets.textures.insert(
            "custom_spellbook".to_string(),
            Texture::new(vec![12, 34, 56, 255], 1, 1),
        );

        let texture =
            Widget::command_icon_texture(&assets, Some("ui.spellbook"), ButtonVisualState::Normal)
                .expect("UI command should resolve its ruleset icon");
        assert_eq!(texture.data, vec![12, 34, 56, 255]);
    }

    #[test]
    fn ruleset_item_uses_its_own_authored_icon_before_semantic_fallback() {
        let mut assets = Assets::default();
        assets.textures.insert(
            "official_item".to_string(),
            Texture::new(vec![12, 34, 56, 255], 1, 1),
        );
        assets.textures.insert(
            "shared_action".to_string(),
            Texture::new(vec![210, 190, 170, 255], 1, 1),
        );
        let mut item = Item::default();
        item.attributes
            .set("ruleset_id", Value::Str("official_item".into()));
        item.attributes
            .set("icon", Value::Str("shared_action".into()));

        let (_, pixels) = Widget::item_icon_texture_square(&assets, &item).unwrap();
        assert_eq!(pixels, vec![12, 34, 56, 255]);
    }

    #[test]
    fn active_item_selects_on_state_icon_frames() {
        let mut assets = Assets::default();
        assets.item_icons.insert(
            "torch:off".to_string(),
            vec![Texture::new(vec![20, 30, 40, 255], 1, 1)],
        );
        assets.item_icons.insert(
            "torch:on".to_string(),
            vec![Texture::new(vec![240, 160, 40, 255], 1, 1)],
        );
        let mut item = Item::default();
        item.attributes
            .set("ruleset_id", Value::Str("torch".into()));
        item.attributes.set("active", Value::Bool(false));

        let off = Widget::custom_item_icon_frames(&assets, &item).unwrap();
        assert_eq!(&off[0].data[0..4], &[20, 30, 40, 255]);

        item.attributes.set("active", Value::Bool(true));
        let on = Widget::custom_item_icon_frames(&assets, &item).unwrap();
        assert_eq!(&on[0].data[0..4], &[240, 160, 40, 255]);
    }

    #[test]
    fn project_item_icon_overrides_ordinary_icon_for_world_billboards() {
        let mut assets = Assets::default();
        assets.item_icons.insert(
            "torch:on".to_string(),
            vec![Texture::new(vec![240, 160, 40, 255], 1, 1)],
        );
        assets.textures.insert(
            "torch".to_string(),
            Texture::new(vec![255, 255, 255, 255], 1, 1),
        );

        let mut item = Item::default();
        item.attributes
            .set("ruleset_id", Value::Str("torch".into()));
        item.attributes.set("active", Value::Bool(true));

        let (_, pixels) = Widget::item_generated_icon_square(&assets, &item).unwrap();
        assert_eq!(pixels, vec![240, 160, 40, 255]);
    }

    #[test]
    fn rectangular_project_item_icon_is_centered_for_world_billboards() {
        let mut assets = Assets::default();
        assets.item_icons.insert(
            "custom:on".to_string(),
            vec![Texture::new(vec![10, 20, 30, 255, 40, 50, 60, 255], 1, 2)],
        );

        let mut item = Item::default();
        item.attributes
            .set("class_name", Value::Str("custom".into()));

        let (size, pixels) = Widget::item_generated_icon_square(&assets, &item).unwrap();
        assert_eq!(size, 2);
        assert_eq!(
            pixels,
            vec![10, 20, 30, 255, 0, 0, 0, 0, 40, 50, 60, 255, 0, 0, 0, 0]
        );
    }

    #[test]
    fn project_off_icon_resolves_by_creator_template_id() {
        let mut assets = Assets::default();
        let template_id = Uuid::new_v4().to_string();
        let key = format!("{template_id}:off");
        assets.item_icons.insert(
            key.clone(),
            vec![
                Texture::new(vec![7, 8, 9, 255], 1, 1),
                Texture::new(vec![70, 80, 90, 255], 1, 1),
            ],
        );
        assets.project_item_icon_keys.insert(key);

        let mut item = Item::default();
        item.attributes
            .set("creator_template_id", Value::Str(template_id));
        item.attributes.set("active", Value::Bool(false));
        item.attributes
            .set("source", Value::Source(PixelSource::TileId(Uuid::new_v4())));

        let (_, first) = Widget::project_item_icon_square(&assets, &item, 0).unwrap();
        let (_, second) = Widget::project_item_icon_square(&assets, &item, 1).unwrap();
        let (_, wrapped) = Widget::project_item_icon_square(&assets, &item, 3).unwrap();
        assert_eq!(first, vec![7, 8, 9, 255]);
        assert_eq!(second, vec![70, 80, 90, 255]);
        assert_eq!(wrapped, second);
    }

    #[test]
    fn bundled_icon_is_not_treated_as_a_project_world_override() {
        let mut assets = Assets::default();
        assets.item_icons.insert(
            "torch:off".to_string(),
            vec![Texture::new(vec![7, 8, 9, 255], 1, 1)],
        );

        let mut item = Item::default();
        item.attributes
            .set("ruleset_id", Value::Str("torch".into()));
        item.attributes.set("active", Value::Bool(false));

        assert!(Widget::project_item_icon_square(&assets, &item, 0).is_none());
    }

    #[test]
    fn command_icon_normal_state_preserves_authored_color() {
        let source = [37, 149, 211, 203];
        assert_eq!(
            Widget::command_icon_pixel(source, ButtonVisualState::Normal),
            source
        );
        let disabled = Widget::command_icon_pixel(source, ButtonVisualState::Disabled);
        assert_eq!(disabled[0], disabled[1]);
        assert_eq!(disabled[1], disabled[2]);
        assert!(disabled[3] < source[3]);
    }
}
