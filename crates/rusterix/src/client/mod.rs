pub mod action;
pub mod command;
pub mod daylight;
pub mod draw2d;
pub mod parser;
pub mod resolver;
pub mod rules_ui;
pub mod text_command;
pub mod widget;

use instant::{Duration, Instant};
use scenevm::GeoId;

use crate::prelude::*;
use crate::{
    BrushPreview, Command, D2ConceptBuilder, D2PreviewBuilder, EntityAction, MapMini, PlayerCamera,
    Rect, SceneHandler, Surface, Value,
    client::action::ClientAction,
    client::command::{ClientCommandBinding, command_from_legacy_fields, parse_client_command},
    client::rules_ui::{CommandState, ContainerUiTemplate, RulesDescription},
    client::widget::{
        ButtonStateStyle, ButtonVisualState, TextInputWidget, Widget, avatar::AvatarWidget,
        deco::DecoWidget, game::GameWidget, messages::MessagesWidget, screen::ScreenWidget,
        text::TextWidget,
    },
};
use draw2d::Draw2D;
use fontdue::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use theframework::prelude::*;
use toml::*;

pub(crate) fn apply_2d_visibility_mask(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    mapmini: &MapMini,
    grid_size: f32,
    top_left: Vec2<f32>,
    player_pos: Vec2<f32>,
    visibility_range_2d: f32,
    visibility_alpha_2d: f32,
    fill: Pixel,
) {
    if visibility_range_2d <= 0.0
        || visibility_alpha_2d <= 0.0
        || grid_size <= 0.0
        || width == 0
        || height == 0
    {
        return;
    }

    let start_x = top_left.x.floor() as i32 - 1;
    let start_y = top_left.y.floor() as i32 - 1;
    let end_x = (top_left.x + width as f32 / grid_size).ceil() as i32 + 1;
    let end_y = (top_left.y + height as f32 / grid_size).ceil() as i32 + 1;
    let from_tile = player_pos.map(|v| v.floor() as i32);

    for tile_y in start_y..end_y {
        for tile_x in start_x..end_x {
            let center = Vec2::new(tile_x as f32 + 0.5, tile_y as f32 + 0.5);
            let to_tile = Vec2::new(tile_x, tile_y);
            if (center - player_pos).magnitude() <= visibility_range_2d
                && mapmini.is_tile_visible(from_tile, to_tile)
            {
                continue;
            }

            let x0 = (((tile_x as f32) - top_left.x) * grid_size).floor() as i32;
            let y0 = (((tile_y as f32) - top_left.y) * grid_size).floor() as i32;
            let x1 = ((((tile_x + 1) as f32) - top_left.x) * grid_size).ceil() as i32;
            let y1 = ((((tile_y + 1) as f32) - top_left.y) * grid_size).ceil() as i32;

            let x0 = x0.clamp(0, width as i32) as usize;
            let y0 = y0.clamp(0, height as i32) as usize;
            let x1 = x1.clamp(0, width as i32) as usize;
            let y1 = y1.clamp(0, height as i32) as usize;

            if x0 >= x1 || y0 >= y1 {
                continue;
            }

            for y in y0..y1 {
                let row = y * width * 4;
                for x in x0..x1 {
                    let idx = row + x * 4;
                    pixels[idx] = ((pixels[idx] as f32 * (1.0 - visibility_alpha_2d))
                        + (fill[0] as f32 * visibility_alpha_2d))
                        .round() as u8;
                    pixels[idx + 1] = ((pixels[idx + 1] as f32 * (1.0 - visibility_alpha_2d))
                        + (fill[1] as f32 * visibility_alpha_2d))
                        .round() as u8;
                    pixels[idx + 2] = ((pixels[idx + 2] as f32 * (1.0 - visibility_alpha_2d))
                        + (fill[2] as f32 * visibility_alpha_2d))
                        .round() as u8;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct OpenContainerPanel {
    item_id: u32,
    owner_entity_id: Option<u32>,
    position: Vec2<i32>,
}

#[derive(Clone, Copy, Debug)]
struct ContainerItemSource {
    container_item_id: u32,
    container_owner_entity_id: Option<u32>,
}

#[derive(Clone, Debug)]
struct ContainerPanelLayout {
    rect: Rect,
    slots: Vec<Rect>,
    title_bar_rect: Option<Rect>,
    close_rect: Option<Rect>,
    title_rect: Option<(isize, isize, isize, isize)>,
}

pub struct Client {
    pub curr_map_id: Uuid,

    pub builder_d2: D2PreviewBuilder,
    pub builder_d2_concept: D2ConceptBuilder,
    pub map_tool_type_d2: MapToolType,

    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub scene_d2: Scene,
    pub scene_d3: Scene,

    pub scene: Scene,

    pub animation_frame: usize,
    pub server_time: TheTime,

    pub brush_preview: Option<BrushPreview>,

    pub messages_font: Option<Font>,
    pub messages_font_size: f32,
    pub messages_font_color: Pixel,

    pub draw2d: Draw2D,

    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, String, TheTime)>,

    // Name of player entity templates
    player_entities: Vec<String>,

    pub current_map: String,
    pub current_sector: String,
    current_screen: String,

    config: toml::Table,

    pub viewport: Vec2<i32>,
    grid_size: f32,
    pub target_fps: i32,
    pub game_tick_ms: i32,
    pub firstp_eye_level: f32,
    firstp_camera_y: Option<f32>,
    active_player_camera: Option<PlayerCamera>,

    // The offset we copy the target into
    pub target_offset: Vec2<i32>,

    // The target we render into
    target: TheRGBABuffer,

    // The UI overlay
    overlay: TheRGBABuffer,

    // The widgets
    game_widgets: FxHashMap<Uuid, GameWidget>,
    button_widgets: FxHashMap<u32, Widget>,
    avatar_widgets: FxHashMap<Uuid, AvatarWidget>,
    text_widgets: FxHashMap<Uuid, TextWidget>,
    text_input_widgets: FxHashMap<u32, TextInputWidget>,
    deco_widgets: FxHashMap<Uuid, DecoWidget>,
    screen_widget: Option<ScreenWidget>,

    messages_widgets: Vec<MessagesWidget>,

    // Button widgets which are active (clicked)
    activated_widgets: Vec<u32>,

    // Button widgets which are permanently active
    permanently_activated_widgets: Vec<u32>,
    pressed_widget: Option<u32>,

    pending_runtime_commands: Vec<ClientCommandBinding>,
    game_started: bool,
    ui_state: FxHashMap<String, String>,
    focused_text_input: Option<u32>,
    pending_game_camera_pos: Option<Vec2<f32>>,

    /// Client Action
    client_action: Arc<Mutex<ClientAction>>,

    /// Hidden widgets,
    widgets_to_hide: Vec<String>,

    // Choice map
    choice_map: Option<FxHashMap<char, Choice>>,

    // Intent
    intent: String,
    key_down_intent: Option<String>,
    click_intents_2d: bool,

    currencies: Currencies,

    first_game_draw: bool,

    // Upscale mode: "none" (default, centered), "aspect" (scale to aspect ratio)
    upscale_mode: String,

    // Current scale factor used for aspect mode (1.0 when no scaling)
    upscale_factor: f32,

    // Default mouse cursor
    default_cursor: Option<Uuid>,

    // Current mouse cursor
    curr_cursor: Option<Uuid>,

    // Current intent cursor
    curr_intent_cursor: Option<Uuid>,

    // Current clicked intent cursor
    curr_clicked_intent_cursor: Option<Uuid>,

    // Cursor position
    cursor_pos: Vec2<i32>,

    // Hovered item id
    hovered_item_id: Option<u32>,

    // Hovered entity id
    hovered_entity_id: Option<u32>,

    // Hover distance
    hover_distance: f32,
    hovered_world_pos: Option<Vec3<f32>>,
    last_3d_hover_pick_at: Option<Instant>,
    tooltip_hover_key: Option<String>,
    tooltip_hover_since: Option<Instant>,

    // Dragged inventory/equipped item id
    dragging_item_id: Option<u32>,
    dragging_item_owner_entity_id: Option<u32>,
    dragging_source_widget_id: Option<u32>,
    dragging_item_from_world: bool,
    dragging_item_container_source: Option<ContainerItemSource>,
    dragging_started: bool,
    drag_start_pos: Vec2<i32>,
    open_container_panel: Option<OpenContainerPanel>,
    open_container_panel_positions: FxHashMap<(u32, Option<u32>), Vec2<i32>>,
    open_container_panel_rect: Option<Rect>,
    open_container_slot_rects: Vec<Rect>,
    open_container_title_rect: Option<Rect>,
    open_container_close_rect: Option<Rect>,
    dragging_container_panel: bool,
    container_panel_drag_offset: Vec2<i32>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Clear all active say bubbles currently rendered above entities/items.
    pub fn clear_say_messages(&mut self) {
        self.messages_to_draw.clear();
    }

    fn say_table_from_widget(widget: &GameWidget) -> Option<toml::value::Table> {
        widget
            .toml_str
            .parse::<toml::Table>()
            .ok()
            .and_then(|table| table.get("say").and_then(toml::Value::as_table).cloned())
    }

    fn say_color_from_table(say: &toml::value::Table, category: &str) -> Option<Pixel> {
        if let Some(hex) = say.get(category).and_then(toml::Value::as_str) {
            return Some(Self::hex_to_rgba_u8(hex));
        }
        if let Some(hex) = say.get("default").and_then(toml::Value::as_str) {
            return Some(Self::hex_to_rgba_u8(hex));
        }
        if let Some(hex) = say.get("").and_then(toml::Value::as_str) {
            return Some(Self::hex_to_rgba_u8(hex));
        }
        None
    }

    fn get_say_color_from_config(config: &toml::Table, category: &str, fallback: Pixel) -> Pixel {
        if let Some(say) = config.get("say").and_then(toml::Value::as_table)
            && let Some(color) = Self::say_color_from_table(say, category)
        {
            return color;
        }
        fallback
    }

    fn get_widget_say_color(
        widget_say: Option<&toml::value::Table>,
        config: &toml::Table,
        category: &str,
        fallback: Pixel,
    ) -> Pixel {
        if let Some(say) = widget_say
            && let Some(color) = Self::say_color_from_table(say, category)
        {
            return color;
        }
        Self::get_say_color_from_config(config, category, fallback)
    }

    fn get_say_color(&self, category: &str) -> Pixel {
        Self::get_say_color_from_config(&self.config, category, self.messages_font_color)
    }

    fn say_duration_minutes_from_table(say: &toml::value::Table) -> Option<f32> {
        say.get("duration")
            .and_then(|v| {
                v.as_float()
                    .map(|f| f as f32)
                    .or_else(|| v.as_integer().map(|i| i as f32))
            })
            .map(|v| v.max(0.0))
    }

    fn get_widget_say_background_enabled(
        widget_say: Option<&toml::value::Table>,
        config: &toml::Table,
    ) -> bool {
        if let Some(say) = widget_say
            && let Some(enabled) = say.get("background_enabled").and_then(toml::Value::as_bool)
        {
            return enabled;
        }
        config
            .get("say")
            .and_then(toml::Value::as_table)
            .and_then(|say| say.get("background_enabled"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true)
    }

    fn say_background_color_from_table(say: &toml::value::Table) -> Option<Pixel> {
        if let Some(hex) = say.get("background_color").and_then(toml::Value::as_str) {
            return Some(Self::hex_to_rgba_u8(hex));
        }
        if let Some(hex) = say.get("background").and_then(toml::Value::as_str) {
            return Some(Self::hex_to_rgba_u8(hex));
        }
        None
    }

    fn get_widget_say_background_color(
        widget_say: Option<&toml::value::Table>,
        config: &toml::Table,
    ) -> Pixel {
        if let Some(say) = widget_say
            && let Some(color) = Self::say_background_color_from_table(say)
        {
            return color;
        }
        if let Some(say) = config.get("say").and_then(toml::Value::as_table)
            && let Some(color) = Self::say_background_color_from_table(say)
        {
            return color;
        }
        [0, 0, 0, 128]
    }

    fn get_say_background_enabled(&self) -> bool {
        Self::get_widget_say_background_enabled(None, &self.config)
    }

    fn get_say_background_color(&self) -> Pixel {
        Self::get_widget_say_background_color(None, &self.config)
    }

    fn get_say_duration_ticks(&self) -> i64 {
        let ticks_per_minute = self
            .get_config_i32_default("game", "ticks_per_minute", 4)
            .max(1);
        let widget_duration = self
            .game_widgets
            .values()
            .find_map(Self::say_table_from_widget)
            .and_then(|say| Self::say_duration_minutes_from_table(&say));
        let duration_minutes = widget_duration
            .or_else(|| {
                self.config
                    .get("say")
                    .and_then(toml::Value::as_table)
                    .and_then(Self::say_duration_minutes_from_table)
            })
            .unwrap_or(1.0)
            .max(0.0);
        let ticks = (duration_minutes * ticks_per_minute as f32).round() as i64;
        ticks.max(1)
    }

    fn choice_expired(&self, choice: &Choice) -> bool {
        let ticks_per_minute = self
            .get_config_i32_default("game", "ticks_per_minute", 4)
            .max(1) as u32;
        let now_ticks = self.server_time.to_ticks(ticks_per_minute);
        let (_, _, expires_at_tick, _) = choice.session_meta();
        now_ticks > expires_at_tick
    }

    fn choice_key_from_input(value: &str) -> Option<char> {
        let trimmed = value.trim();
        if trimmed.len() == 1 {
            return trimmed.chars().next().filter(|c| c.is_ascii_digit());
        }

        let lower = trimmed.to_ascii_lowercase();
        ["digit", "numpad"].iter().find_map(|prefix| {
            lower
                .strip_prefix(prefix)
                .and_then(|suffix| suffix.chars().find(|c| c.is_ascii_digit()))
        })
    }

    fn deactivate_matches(widget: &Widget, token: &str) -> bool {
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

    fn shortcut_labels_for_binding(&self, binding: &ClientCommandBinding) -> Vec<String> {
        self.client_action
            .lock()
            .map(|action| action.shortcut_labels_for_binding(binding))
            .unwrap_or_default()
    }

    fn resolved_widget_command(
        widget: &Widget,
        assets: &Assets,
        entity: Option<&Entity>,
        ui_state: &FxHashMap<String, String>,
    ) -> Option<String> {
        if let Some(slot) = widget.command_slot.as_deref() {
            return Self::command_for_slot(slot, assets, entity, ui_state);
        }
        widget.command.clone().or_else(|| {
            widget
                .command_binding()
                .map(|binding| binding.command_string())
        })
    }

    fn resolved_widget_binding(
        widget: &Widget,
        assets: &Assets,
        entity: Option<&Entity>,
        ui_state: &FxHashMap<String, String>,
    ) -> Option<ClientCommandBinding> {
        Self::resolved_widget_command(widget, assets, entity, ui_state)
            .as_deref()
            .and_then(parse_client_command)
    }

    fn resolved_widget_intent_payload(
        widget: &Widget,
        assets: &Assets,
        entity: Option<&Entity>,
        ui_state: &FxHashMap<String, String>,
    ) -> Option<String> {
        Self::resolved_widget_binding(widget, assets, entity, ui_state)
            .and_then(|binding| binding.intent_payload())
    }

    fn command_for_slot(
        slot: &str,
        assets: &Assets,
        entity: Option<&Entity>,
        ui_state: &FxHashMap<String, String>,
    ) -> Option<String> {
        let slot = slot.trim();
        if slot.is_empty() {
            return None;
        }

        let suffix = Self::slot_attr_suffix(slot);
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
            return Self::normalize_slot_command(command);
        }

        let (group, index) = Self::split_command_slot(slot)?;
        let class = entity
            .and_then(|entity| {
                entity
                    .get_attr_string("class")
                    .or_else(|| entity.get_attr_string("class_name"))
            })
            .or_else(|| ui_state.get("start.class").cloned())
            .unwrap_or_else(|| "Warrior".to_string());

        let rules = assets.rules.parse::<Table>().ok()?;
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
        Self::normalize_slot_command(command)
    }

    fn split_command_slot(slot: &str) -> Option<(&str, usize)> {
        let (group, index) = slot.rsplit_once('.')?;
        let group = group.trim();
        let index = index.trim().parse::<usize>().ok()?;
        (!group.is_empty()).then_some((group, index))
    }

    fn slot_attr_suffix(slot: &str) -> String {
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

    fn normalize_slot_command(command: &str) -> Option<String> {
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

    fn add_shortcut_line(description: &mut RulesDescription, shortcuts: Vec<String>) {
        if shortcuts.is_empty() {
            return;
        }
        let label = if shortcuts.len() == 1 {
            "Shortcut"
        } else {
            "Shortcuts"
        };
        description
            .lines
            .push(format!("{}: {}", label, shortcuts.join(", ")));
    }

    fn draw_deco_widgets_with_layer<F>(
        deco_widgets: &mut FxHashMap<Uuid, DecoWidget>,
        buffer: &mut TheRGBABuffer,
        map: &Map,
        currencies: &Currencies,
        assets: &Assets,
        layer_filter: F,
    ) where
        F: Fn(i32) -> bool,
    {
        for widget in deco_widgets.values_mut() {
            if layer_filter(widget.layer) {
                widget.update_draw(buffer, map, currencies, assets);
            }
        }
    }

    /// Returns the currently active game-widget camera mode if present.
    /// Prioritizes first-person over iso over 2D when multiple game widgets exist.
    pub fn active_game_widget_camera_mode(&self) -> Option<PlayerCamera> {
        let mut found_iso = false;
        let mut found_d2 = false;
        for widget in self.game_widgets.values() {
            match widget.camera {
                PlayerCamera::D3FirstP | PlayerCamera::D3FirstPGrid => {
                    return Some(widget.camera.clone());
                }
                PlayerCamera::D3Iso => found_iso = true,
                PlayerCamera::D2 | PlayerCamera::D2Grid => found_d2 = true,
            }
        }
        if found_iso {
            Some(PlayerCamera::D3Iso)
        } else if found_d2 {
            Some(PlayerCamera::D2)
        } else {
            None
        }
    }

    fn update_active_player_camera(&mut self, map: &Map) {
        self.active_player_camera = map
            .entities
            .iter()
            .find(|entity| entity.is_player())
            .and_then(|entity| match entity.attributes.get("player_camera") {
                Some(crate::Value::PlayerCamera(camera)) => Some(camera.clone()),
                _ => None,
            });
    }

    fn parse_player_camera_mode(camera: &str) -> Option<PlayerCamera> {
        match camera.to_ascii_lowercase().as_str() {
            "2d" => Some(PlayerCamera::D2),
            "2d_grid" => Some(PlayerCamera::D2Grid),
            "iso" => Some(PlayerCamera::D3Iso),
            "iso_grid" => Some(PlayerCamera::D2Grid),
            "firstp" => Some(PlayerCamera::D3FirstP),
            "firstp_grid" => Some(PlayerCamera::D3FirstPGrid),
            _ => None,
        }
    }

    fn is_2d_camera(camera: &PlayerCamera) -> bool {
        matches!(camera, PlayerCamera::D2 | PlayerCamera::D2Grid)
    }

    fn set_game_widget_camera_mode(&mut self, target: Option<&str>, camera: PlayerCamera) {
        for widget in self.game_widgets.values_mut() {
            if match target {
                Some(name) => widget.name == name,
                None => true,
            } {
                widget.set_camera_mode(camera.clone());
            }
        }
    }

    pub fn new() -> Self {
        Self {
            curr_map_id: Uuid::default(),

            builder_d2: D2PreviewBuilder::new(),
            builder_d2_concept: D2ConceptBuilder::new(),
            map_tool_type_d2: MapToolType::General,

            camera_d3: Box::new(D3FirstPCamera::new()),
            builder_d3: D3Builder::new(),

            scene_d2: Scene::default(),
            scene_d3: Scene::default(),

            scene: Scene::default(),

            animation_frame: 0,
            server_time: TheTime::default(),

            brush_preview: None,

            messages_font: None,
            draw2d: Draw2D::default(),

            messages_font_size: 15.0,
            messages_font_color: [229, 229, 1, 255],

            messages_to_draw: FxHashMap::default(),

            player_entities: Vec::new(),

            current_map: String::new(),
            current_sector: String::new(),
            current_screen: String::new(),

            config: toml::Table::default(),
            viewport: Vec2::zero(),
            grid_size: 32.0,
            target_fps: 30,
            game_tick_ms: 250,
            firstp_eye_level: 1.7,
            firstp_camera_y: None,
            active_player_camera: None,

            target_offset: Vec2::zero(),
            target: TheRGBABuffer::default(),
            overlay: TheRGBABuffer::default(),

            game_widgets: FxHashMap::default(),
            button_widgets: FxHashMap::default(),
            avatar_widgets: FxHashMap::default(),
            text_widgets: FxHashMap::default(),
            text_input_widgets: FxHashMap::default(),
            deco_widgets: FxHashMap::default(),
            screen_widget: None,

            messages_widgets: Vec::new(),

            activated_widgets: vec![],
            permanently_activated_widgets: vec![],
            pressed_widget: None,
            pending_runtime_commands: vec![],
            game_started: false,
            ui_state: FxHashMap::default(),
            focused_text_input: None,
            pending_game_camera_pos: None,
            widgets_to_hide: vec![],

            client_action: Arc::new(Mutex::new(ClientAction::default())),
            currencies: Currencies::default(),
            intent: String::new(),
            key_down_intent: None,
            click_intents_2d: false,

            choice_map: None,

            first_game_draw: false,

            upscale_mode: "none".to_string(),
            upscale_factor: 1.0,

            default_cursor: None,
            curr_cursor: None,
            curr_intent_cursor: None,
            curr_clicked_intent_cursor: None,
            cursor_pos: Vec2::zero(),
            hovered_entity_id: None,
            hovered_item_id: None,
            hovered_world_pos: None,

            hover_distance: f32::MAX,
            last_3d_hover_pick_at: None,
            tooltip_hover_key: None,
            tooltip_hover_since: None,
            dragging_item_id: None,
            dragging_item_owner_entity_id: None,
            dragging_source_widget_id: None,
            dragging_item_from_world: false,
            dragging_item_container_source: None,
            dragging_started: false,
            drag_start_pos: Vec2::zero(),
            open_container_panel: None,
            open_container_panel_positions: FxHashMap::default(),
            open_container_panel_rect: None,
            open_container_slot_rects: Vec::new(),
            open_container_title_rect: None,
            open_container_close_rect: None,
            dragging_container_panel: false,
            container_panel_drag_offset: Vec2::zero(),
        }
    }

    /// Increase the anim counter.
    pub fn inc_animation_frame(&mut self) {
        self.animation_frame += 1;

        for widget in self.game_widgets.values_mut() {
            widget.scene.animation_frame += 1;
        }
        if let Some(widget) = self.screen_widget.as_mut() {
            widget.scene.animation_frame += 1;
        }
    }

    /// Set the server time
    pub fn set_server_time(&mut self, time: TheTime) {
        self.server_time = time;
    }

    /// Set the current map id.
    pub fn set_curr_map_id(&mut self, id: Uuid) {
        self.curr_map_id = id;
    }

    /// Set the D3 Camera
    pub fn set_camera_d3(&mut self, camera: Box<dyn D3Camera>) {
        if camera.id() != "firstp" {
            self.firstp_camera_y = None;
        }
        self.camera_d3 = camera;
    }

    fn apply_player_camera_d3(&mut self, entity: &Entity) {
        if self.camera_d3.id() == "firstp" {
            let target_y = entity.position.y;
            let smoothed_y = match self.firstp_camera_y {
                Some(current) if (target_y - current).abs() <= 2.0 => {
                    current + (target_y - current) * 0.28
                }
                _ => target_y,
            };
            self.firstp_camera_y = Some(smoothed_y);

            let mut visual_entity = entity.clone();
            visual_entity.position.y = smoothed_y;
            visual_entity.apply_to_camera(&mut self.camera_d3, self.firstp_eye_level);
        } else {
            self.firstp_camera_y = None;
            entity.apply_to_camera(&mut self.camera_d3, self.firstp_eye_level);
        }
    }

    pub fn set_map_tool_type_d2(&mut self, tool: MapToolType) {
        self.map_tool_type_d2 = tool;
        self.builder_d2.set_map_tool_type(tool);
        self.builder_d2_concept.set_map_tool_type(tool);
    }

    pub fn set_map_hover_info_d2(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
        self.builder_d2.set_map_hover_info(hover, hover_cursor);
        self.builder_d2_concept
            .set_map_hover_info(hover, hover_cursor);
    }

    pub fn set_camera_info_d2(&mut self, pos: Option<vek::Vec3<f32>>, look_at: Option<Vec3<f32>>) {
        self.builder_d2.set_camera_info(pos, look_at);
        self.builder_d2_concept.set_camera_info(pos, look_at);
    }

    pub fn set_clip_rect_d2(&mut self, clip_rect: Option<Rect>) {
        self.builder_d2.set_clip_rect(clip_rect);
        self.builder_d2_concept.set_clip_rect(clip_rect);
    }

    /// Build the 2D scene from the map.
    pub fn build_custom_scene_d2(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        assets: &Assets,
        values: &ValueContainer,
        edit_surface: &Option<Surface>,
        scene_handler: &mut SceneHandler,
        draw_sectors: bool,
    ) {
        self.update_active_player_camera(map);
        self.curr_map_id = map.id;
        if self.map_tool_type_d2 == MapToolType::Dungeon {
            self.scene_d2 = self
                .builder_d2_concept
                .build(map, assets, screen_size, values);
            self.builder_d2_concept.build_entities_items(
                map,
                assets,
                &mut self.scene_d2,
                screen_size,
                edit_surface,
                scene_handler,
                draw_sectors,
            );
        } else {
            self.scene_d2 = self.builder_d2.build(map, assets, screen_size, values);
            self.builder_d2.build_entities_items(
                map,
                assets,
                &mut self.scene_d2,
                screen_size,
                edit_surface,
                scene_handler,
                draw_sectors,
            );
        }
        if self.map_tool_type_d2 != MapToolType::Dungeon {
            scene_handler.build_dynamics_2d(map, self.animation_frame, assets);
        }
    }

    /// Apply the entities to the 2D scene.
    pub fn apply_entities_items_d2(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        assets: &Assets,
        edit_surface: &Option<Surface>,
        scene_handler: &mut SceneHandler,
        draw_sectors: bool,
    ) {
        self.update_active_player_camera(map);
        self.builder_d2.build_entities_items(
            map,
            assets,
            &mut self.scene,
            screen_size,
            edit_surface,
            scene_handler,
            draw_sectors,
        );
        scene_handler.build_dynamics_2d(map, self.animation_frame, assets);
    }

    /// Build the 3D scene from the map.
    pub fn build_custom_scene_d3(&mut self, map: &Map, assets: &Assets, values: &ValueContainer) {
        self.update_active_player_camera(map);
        self.curr_map_id = map.id;
        self.scene_d3 = self.builder_d3.build(
            map,
            assets,
            Vec2::zero(), // Only needed for 2D builders
            &self.camera_d3.id(),
            values,
        );
    }

    /// Apply the entities to the 3D scene.
    pub fn apply_entities_items_d3(
        &mut self,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.update_active_player_camera(map);
        for entity in &map.entities {
            if entity.is_player() {
                self.apply_player_camera_d3(entity);
            }
        }
        self.builder_d3.build_entities_items(
            map,
            self.camera_d3.as_ref(),
            assets,
            &mut self.scene,
            scene_handler,
        );
    }

    /// Process messages from the server to be displayed after drawing.
    pub fn process_messages(&mut self, map: &Map, messages: Vec<crate::server::Say>) {
        let ticks_per_minute = self
            .get_config_i32_default("game", "ticks_per_minute", 4)
            .max(1);
        let now_ticks = self.server_time.to_ticks(ticks_per_minute as u32);
        // Remove expired messages
        let expired_keys: Vec<_> = self
            .messages_to_draw
            .iter()
            .filter(|(_, (_, _, _, _, expire_time))| *expire_time <= self.server_time)
            .map(|(id, _)| *id)
            .collect();

        for id in expired_keys {
            self.messages_to_draw.remove(&id);
        }

        // Add new messages
        let duration_ticks = self.get_say_duration_ticks();
        for (sender_entity_id, sender_item_id, message, category) in messages {
            if let Some(sender_item_id) = sender_item_id {
                for item in &map.items {
                    if item.id == sender_item_id {
                        if let Some(font) = &self.messages_font {
                            let text_size =
                                self.draw2d
                                    .get_text_size(font, self.messages_font_size, &message);

                            let expire_time = TheTime::from_ticks(
                                now_ticks + duration_ticks,
                                ticks_per_minute as u32,
                            );

                            self.messages_to_draw.insert(
                                sender_item_id,
                                (
                                    item.get_pos_xz(),
                                    message.clone(),
                                    text_size.0,
                                    category.clone(),
                                    expire_time,
                                ),
                            );
                        }
                    }
                }
            } else if let Some(sender_entity_id) = sender_entity_id {
                for entity in &map.entities {
                    if entity.id == sender_entity_id {
                        if let Some(font) = &self.messages_font {
                            let text_size =
                                self.draw2d
                                    .get_text_size(font, self.messages_font_size, &message);

                            let expire_time = TheTime::from_ticks(
                                now_ticks + duration_ticks,
                                ticks_per_minute as u32,
                            );

                            self.messages_to_draw.insert(
                                sender_entity_id,
                                (
                                    entity.get_pos_xz(),
                                    message.clone(),
                                    text_size.0,
                                    category.clone(),
                                    expire_time,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw the 2D scene.
    pub fn draw_custom_d2(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.scene_d2.animation_frame = self.animation_frame;
        let screen_size = Vec2::new(width as f32, height as f32);
        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            map.offset.x + screen_size.x / 2.0,
            -map.offset.y + screen_size.y / 2.0,
        ));
        let scale_matrix = Mat3::new(
            map.grid_size,
            0.0,
            0.0,
            0.0,
            map.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
            .render_mode(RenderMode::render_2d());
        rast.hour = self.server_time.to_f32();
        rast.mapmini = self.scene_d2.mapmini.clone();
        rast.rasterize(&mut self.scene_d2, pixels, width, height, 64, assets);

        // Composite SceneVM 2D overlay layer on top so profile/screen editors show lines/handles.
        if scene_handler.vm.vm_layer_count() > 1
            && scene_handler.vm.is_layer_enabled(1) == Some(true)
        {
            let mut enabled_before: Vec<bool> = (0..scene_handler.vm.vm_layer_count())
                .map(|i| scene_handler.vm.is_layer_enabled(i).unwrap_or(true))
                .collect();
            scene_handler.vm.set_layer_enabled(0, false);
            scene_handler.vm.set_layer_enabled(1, true);
            for layer in 2..scene_handler.vm.vm_layer_count() {
                scene_handler.vm.set_layer_enabled(layer, false);
            }
            scene_handler.vm.set_active_vm(1);
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                map.grid_size,
                map.subdivisions,
                map.offset.x,
                -map.offset.y,
            )));
            scene_handler.vm.execute(scenevm::Atom::SetGP2(Vec4::one()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));
            scene_handler.vm.set_active_vm(0);

            let mut overlay = vec![0_u8; width * height * 4];
            scene_handler
                .vm
                .render_frame(&mut overlay, width as u32, height as u32);

            for (dst, src) in pixels.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
                let sa = src[3] as f32 / 255.0;
                if sa <= 0.0 {
                    continue;
                }
                let inv = 1.0 - sa;
                dst[0] = ((src[0] as f32 * sa) + (dst[0] as f32 * inv))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst[1] = ((src[1] as f32 * sa) + (dst[1] as f32 * inv))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst[2] = ((src[2] as f32 * sa) + (dst[2] as f32 * inv))
                    .round()
                    .clamp(0.0, 255.0) as u8;
                dst[3] = 255;
            }

            for (i, enabled) in enabled_before.drain(..).enumerate() {
                scene_handler.vm.set_layer_enabled(i, enabled);
            }
        }
    }

    /// Draw the 2D scene.
    pub fn draw_d2(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        _assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        pub fn map_grid_to_local(
            screen_size: Vec2<f32>,
            grid_pos: Vec2<f32>,
            map: &Map,
        ) -> Vec2<f32> {
            let grid_space_pos = grid_pos * map.grid_size;
            grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
        }

        self.scene.animation_frame = self.animation_frame;
        let screen_size = Vec2::new(width as f32, height as f32);
        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            map.offset.x + screen_size.x / 2.0,
            -map.offset.y + screen_size.y / 2.0,
        ));
        let scale_matrix = Mat3::new(
            map.grid_size,
            0.0,
            0.0,
            0.0,
            map.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;
        let top_left = Vec2::new(
            (-screen_size.x / 2.0 - map.offset.x) / map.grid_size,
            (map.offset.y - screen_size.y / 2.0) / map.grid_size,
        );
        let player_pos = map
            .entities
            .iter()
            .find(|entity| entity.is_player())
            .map(|entity| entity.get_pos_xz())
            .unwrap_or_else(Vec2::zero);

        let scenevm_mode_2d = scene_handler.settings.scenevm_mode_2d();
        scene_handler.vm.set_active_vm(0);
        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                map.grid_size,
                map.subdivisions,
                map.offset.x,
                -map.offset.y,
            )));
        }

        let hour = self.server_time.to_f32();

        // Ensure base scene layer is visible in editor 2D mode.
        let overlay_layer_enabled = if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.is_layer_enabled(1).unwrap_or(true)
        } else {
            false
        };
        scene_handler.vm.set_layer_enabled(0, true);
        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_layer_enabled(1, overlay_layer_enabled);
        }
        for layer in 2..scene_handler.vm.vm_layer_count() {
            scene_handler.vm.set_layer_enabled(layer, false);
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm_mode_2d));

        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_2d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_2d();
        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                map.grid_size,
                map.subdivisions,
                map.offset.x,
                -map.offset.y,
            )));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetTransform2D(transform));

        // Set the transform for the overlay if active
        if scene_handler.vm.vm_layer_count() > 1 && overlay_layer_enabled {
            for layer in 2..scene_handler.vm.vm_layer_count() {
                scene_handler.vm.set_layer_enabled(layer, false);
            }
            scene_handler.vm.set_active_vm(1);
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
            // 2D overlay shader expects grid params in GP0 and a non-zero GP2.x to draw background/grid.
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                map.grid_size,
                map.subdivisions,
                map.offset.x,
                -map.offset.y,
            )));
            scene_handler.vm.execute(scenevm::Atom::SetGP2(Vec4::one()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));
            scene_handler.vm.set_active_vm(0);
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));

        scene_handler
            .vm
            .render_frame(pixels, width as u32, height as u32);

        let bg = scene_handler
            .settings
            .background_color_2d
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
        apply_2d_visibility_mask(
            pixels,
            width,
            height,
            &self.scene_d2.mapmini,
            map.grid_size,
            top_left,
            player_pos,
            scene_handler.settings.visibility_range_2d,
            scene_handler.settings.visibility_alpha_2d,
            bg,
        );

        // Draw Messages

        if let Some(font) = &self.messages_font {
            let say_bg_enabled = self.get_say_background_enabled();
            let say_bg_color = self.get_say_background_color();
            for (grid_pos, message, text_size, category, _) in self.messages_to_draw.values() {
                let color = self.get_say_color(category);
                let position = map_grid_to_local(screen_size, *grid_pos, map);

                let tuple = (
                    position.x as isize - *text_size as isize / 2 - 5,
                    position.y as isize - self.messages_font_size as isize - map.grid_size as isize,
                    *text_size as isize + 10,
                    22,
                );

                if say_bg_enabled && say_bg_color[3] > 0 {
                    self.draw2d.blend_rect_safe(
                        pixels,
                        &tuple,
                        width,
                        &say_bg_color,
                        &(0, 0, width as isize, height as isize),
                    );
                }

                self.draw2d.text_rect_blend_safe(
                    pixels,
                    &tuple,
                    width,
                    font,
                    self.messages_font_size,
                    message,
                    &color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );
            }
        }
    }

    /// Draw the 3D scene.
    pub fn draw_d3(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        _assets: &Assets,
        scene_handler: &mut SceneHandler,
        editor_neutral_background: bool,
    ) {
        self.scene.animation_frame = self.animation_frame;

        let hour = self.server_time.to_f32();

        scene_handler.apply_dungeon_render_overrides(map);
        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_3d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_3d();
        if editor_neutral_background {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::new(0.018, 0.018, 0.020, 0.0)));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::new(0.0, 0.0, 0.0, 1.0)));

        scene_handler.vm.execute(scenevm::Atom::SetRenderMode(
            scene_handler.settings.scenevm_mode_3d(),
        ));

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: self.camera_d3.as_scenevm_camera(),
        });

        // In 3D mode, enable overlay layers.
        for layer in 1..scene_handler.vm.vm_layer_count() {
            scene_handler.vm.set_layer_enabled(layer, true);
        }

        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_active_vm(1);

            let screen_size = Vec2::new(width as f32, height as f32);
            let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
                map.offset.x + screen_size.x / 2.0,
                -map.offset.y + screen_size.y / 2.0,
            ));
            let scale_matrix = Mat3::new(
                map.grid_size,
                0.0,
                0.0,
                0.0,
                map.grid_size,
                0.0,
                0.0,
                0.0,
                1.0,
            );
            let transform = translation_matrix * scale_matrix;
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
            // Prevent stale 2D grid/background params from leaking into 3D rendering.
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP0(Vec4::zero()));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP2(Vec4::zero()));

            for layer in 2..scene_handler.vm.vm_layer_count() {
                scene_handler.vm.set_active_vm(layer);
                scene_handler.apply_runtime_render_state_settings();
                scene_handler.settings.apply_3d(&mut scene_handler.vm);
                scene_handler.apply_runtime_render_state_3d();
                scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
                    camera: self.camera_d3.as_scenevm_camera(),
                });
                scene_handler.vm.execute(scenevm::Atom::SetRenderMode(
                    scene_handler.settings.scenevm_mode_3d(),
                ));
                // Keep editor overlay lines readable and color-accurate regardless of world lighting.
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP2(Vec4::new(0.0, 0.0, 0.0, 0.0))); // sun off
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP3(Vec4::new(1.0, 1.0, 1.0, 1.0))); // full ambient
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP4(Vec4::new(0.0, 0.0, 0.0, 0.0))); // fog off
            }
            scene_handler.vm.set_active_vm(0);
        }

        // scene_handler.vm.print_geometry_stats();

        scene_handler
            .vm
            .render_frame(pixels, width as u32, height as u32);

        if let Some(font) = &self.messages_font {
            let view = self.camera_d3.view_matrix();
            let proj = self
                .camera_d3
                .projection_matrix(width as f32, height as f32);
            let vp = proj * view;
            let say_bg_enabled = self.get_say_background_enabled();
            let say_bg_color = self.get_say_background_color();

            for (grid_pos, message, text_size, category, _) in self.messages_to_draw.values() {
                let color = self.get_say_color(category);
                let world = Vec4::new(grid_pos.x, 1.8, grid_pos.y, 1.0);
                let clip = vp * world;
                if clip.w <= 0.0 {
                    continue;
                }

                let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
                if ndc.z < -1.0 || ndc.z > 1.0 {
                    continue;
                }

                let sx = ((ndc.x * 0.5 + 0.5) * width as f32) as isize;
                let sy = ((1.0 - (ndc.y * 0.5 + 0.5)) * height as f32) as isize;

                let tuple = (
                    sx - *text_size as isize / 2 - 5,
                    sy - self.messages_font_size as isize - 14,
                    *text_size as isize + 10,
                    22,
                );

                if say_bg_enabled && say_bg_color[3] > 0 {
                    self.draw2d.blend_rect_safe(
                        pixels,
                        &tuple,
                        width,
                        &say_bg_color,
                        &(0, 0, width as isize, height as isize),
                    );
                }

                self.draw2d.text_rect_blend_safe(
                    pixels,
                    &tuple,
                    width,
                    font,
                    self.messages_font_size,
                    message,
                    &color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );
            }
        }
    }

    /// Get an i32 config value
    fn get_config_i32_default(&self, table: &str, key: &str, default: i32) -> i32 {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_integer() {
                    return v as i32;
                }
            }
        }
        default
    }

    fn get_config_f32_default(&self, table: &str, key: &str, default: f32) -> f32 {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_float() {
                    return v as f32;
                } else if let Some(v) = value.as_integer() {
                    return v as f32;
                }
            }
        }
        default
    }

    fn get_config_bool_default(&self, table: &str, key: &str, default: bool) -> bool {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_bool() {
                    return v;
                }
            }
        }
        default
    }

    fn get_config_string_default(&self, table: &str, key: &str, default: &str) -> String {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_str() {
                    return v.to_string();
                }
            }
        }
        default.to_string()
    }

    fn get_uuid(map: &toml::map::Map<String, toml::Value>, key: &str) -> Option<Uuid> {
        map.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }

    fn hex_to_rgba_u8(hex: &str) -> [u8; 4] {
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

    fn color_from_table(table: &toml::value::Table, key: &str) -> Option<[u8; 4]> {
        table
            .get(key)
            .and_then(toml::Value::as_str)
            .map(Self::hex_to_rgba_u8)
    }

    fn ui_style_color(
        ui: &toml::value::Table,
        state: Option<&str>,
        key: &str,
        flat_key: &str,
    ) -> Option<[u8; 4]> {
        if let Some(color) = Self::color_from_table(ui, flat_key) {
            return Some(color);
        }

        let style = ui.get("style").and_then(toml::Value::as_table)?;
        match state {
            Some(state) => style
                .get(state)
                .and_then(toml::Value::as_table)
                .and_then(|state_style| Self::color_from_table(state_style, key)),
            None => Self::color_from_table(style, key),
        }
    }

    fn button_state_style_from_ui(
        ui: &toml::value::Table,
        state: &str,
        background_key: &str,
        border_key: &str,
        label_key: &str,
    ) -> ButtonStateStyle {
        ButtonStateStyle {
            background_color: Self::ui_style_color(ui, Some(state), "background", background_key),
            border_color: Self::ui_style_color(ui, Some(state), "border", border_key),
            label_color: Self::ui_style_color(ui, Some(state), "text", label_key)
                .or_else(|| Self::ui_style_color(ui, Some(state), "color", label_key)),
        }
    }

    fn button_visual_state(
        hovered: bool,
        selected: bool,
        pressed: bool,
        command_state: Option<&CommandState>,
    ) -> ButtonVisualState {
        if command_state.is_some_and(|state| !state.enabled || state.cooldown_remaining > 0.0) {
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

    fn command_is_walk(command: Option<&str>) -> bool {
        matches!(
            command.and_then(parse_client_command),
            Some(ClientCommandBinding::Intent(intent)) if intent.trim().is_empty()
        )
    }

    /// Setup the client with the given assets.
    pub fn setup(&mut self, assets: &mut Assets, scene_handler: &mut SceneHandler) -> Vec<Command> {
        let mut commands = vec![];
        self.first_game_draw = true;
        self.intent = String::new();
        self.game_started = false;
        self.ui_state.clear();
        self.focused_text_input = None;
        self.pending_game_camera_pos = None;

        self.permanently_activated_widgets.clear();
        self.activated_widgets.clear();
        self.pressed_widget = None;
        self.pending_runtime_commands.clear();

        scene_handler.sync_base_render_settings(&assets.config);

        // Init config
        match assets.config.parse::<Table>() {
            Ok(data) => {
                self.config = data;
            }
            Err(err) => {
                eprintln!("Client: Error parsing config: {}", err);
            }
        }

        self.currencies = Currencies::from_rules_source(&assets.rules);

        // Get all player entities
        self.player_entities.clear();
        for (name, character) in assets.entities.iter() {
            match character.1.parse::<Table>() {
                Ok(data) => {
                    if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                        if let Some(value) = game.get("player") {
                            if let Some(v) = value.as_bool() {
                                if v {
                                    self.player_entities.push(name.to_string());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Client: Error parsing entity {}: {}", name, err);
                }
            }
        }

        self.viewport = Vec2::new(
            self.get_config_i32_default("viewport", "width", 1280),
            self.get_config_i32_default("viewport", "height", 720),
        );

        self.target_fps = self.get_config_i32_default("game", "target_fps", 30);
        self.game_tick_ms = self.get_config_i32_default("game", "game_tick_ms", 250);
        self.firstp_eye_level = self.get_config_f32_default("game", "firstp_eye_level", 1.7);
        self.click_intents_2d = self.get_config_bool_default("game", "click_intents_2d", false)
            || self.get_config_bool_default("game", "persistent_2d_intents", false);
        self.grid_size = self.get_config_i32_default("viewport", "grid_size", 32) as f32;
        self.upscale_mode = self.get_config_string_default("viewport", "upscale", "none");

        self.default_cursor = None;
        let tile_id_str = self.get_config_string_default("viewport", "cursor_id", "");
        if !tile_id_str.is_empty() {
            if let Ok(uuid) = Uuid::parse_str(&tile_id_str) {
                self.default_cursor = Some(uuid);
            }
        }

        // Create the target buffer
        self.target = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));
        // Create the overlay buffer
        self.overlay = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));

        // Find the start region
        self.current_map = self.get_config_string_default("game", "start_region", "");

        // Find the start screen
        self.current_screen = self.get_config_string_default("game", "start_screen", "");
        let has_start_screen = !self.current_screen.trim().is_empty()
            && assets.screens.contains_key(&self.current_screen);
        let start_screen_has_game_widget =
            has_start_screen && Self::screen_has_widget_role(assets, &self.current_screen, "game");

        // Auto Init Players
        let auto_init_player = self.get_config_bool_default("game", "auto_create_player", false);
        if let Some(map) = assets.maps.get(&self.current_map) {
            if auto_init_player && (!has_start_screen || start_screen_has_game_widget) {
                for entity in map.entities.iter() {
                    if let Some(class_name) = entity.get_attr_string("class_name") {
                        if self.player_entities.contains(&class_name) {
                            commands.push(Command::CreateEntity(map.id, entity.clone()));
                            self.game_started = true;
                            // Init scripting for this entity
                            self.client_action = Arc::new(Mutex::new(ClientAction::default()));
                            self.client_action.lock().unwrap().init(class_name, assets);
                            break;
                        }
                    }
                }
            }
        } else {
            eprintln!("Did not find start map");
        }

        if has_start_screen {
            self.init_screen(self.current_screen.clone(), assets, scene_handler);
        } else if !self.current_screen.trim().is_empty() {
            eprintln!("Did not find start screen");
        }

        commands
    }

    fn screen_has_widget_role(assets: &Assets, screen_name: &str, role_name: &str) -> bool {
        let Some(screen) = assets.screens.get(screen_name) else {
            return false;
        };

        screen
            .sectors
            .iter()
            .any(|sector| Self::sector_ui_role(sector).is_some_and(|role| role == role_name))
    }

    fn sector_ui_role(sector: &crate::Sector) -> Option<String> {
        let crate::Value::Str(data) = sector.properties.get("data")? else {
            return None;
        };
        let table = data.parse::<Table>().ok()?;
        table
            .get("ui")
            .and_then(toml::Value::as_table)
            .and_then(|ui| ui.get("role"))
            .and_then(toml::Value::as_str)
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .map(str::to_string)
    }

    fn screen_base_render_map(screen: &Map) -> Map {
        let mut map = screen.clone();
        map.sectors
            .retain(|sector| Self::sector_ui_role(sector).is_none());
        map
    }

    pub fn process_pending_runtime_commands(
        &mut self,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) -> Vec<Command> {
        let pending = std::mem::take(&mut self.pending_runtime_commands);
        let mut commands = Vec::new();

        for command in pending {
            match command {
                ClientCommandBinding::Screen(screen_command) => {
                    self.process_screen_command(&screen_command, assets, scene_handler);
                }
                ClientCommandBinding::Game(game_command) => {
                    self.process_game_command(&game_command, assets, scene_handler, &mut commands);
                }
                _ => {}
            }
        }

        commands
    }

    fn process_screen_command(
        &mut self,
        command: &str,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) {
        if let Some(screen_name) = command.trim().strip_prefix("goto.") {
            self.goto_screen(screen_name, assets, scene_handler);
        }
    }

    fn set_ui_state(&mut self, binding: &str, value: &str) {
        let binding = binding.trim();
        if binding.is_empty() {
            return;
        }
        self.ui_state
            .insert(binding.to_string(), value.trim().to_string());
        self.sync_bound_button_activation(binding);
    }

    fn sync_bound_button_activation(&mut self, binding: &str) {
        let selected_value = self.ui_state.get(binding).map(|value| value.trim());
        let mut selected_button_id = None;

        for (id, widget) in self.button_widgets.iter() {
            if widget.binding.as_deref() != Some(binding) {
                continue;
            }

            let single_selection = widget
                .selection
                .as_deref()
                .map(|selection| selection.eq_ignore_ascii_case("single"))
                .unwrap_or_else(|| widget.group.is_some());
            if !single_selection {
                continue;
            }

            self.activated_widgets.retain(|active_id| active_id != id);
            self.permanently_activated_widgets
                .retain(|active_id| active_id != id);

            if let (Some(selected), Some(value)) = (selected_value, widget.value.as_deref())
                && value.trim().eq_ignore_ascii_case(selected)
            {
                selected_button_id = Some(*id);
            }
        }

        if let Some(id) = selected_button_id {
            if !self.activated_widgets.contains(&id) {
                self.activated_widgets.push(id);
            }
            if !self.permanently_activated_widgets.contains(&id) {
                self.permanently_activated_widgets.push(id);
            }
        }
    }

    fn apply_bound_button_activations(&mut self) {
        let bindings: Vec<String> = self.ui_state.keys().cloned().collect();
        for binding in bindings {
            self.sync_bound_button_activation(&binding);
        }
    }

    fn process_game_command(
        &mut self,
        command: &str,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
        commands: &mut Vec<Command>,
    ) {
        let command = command.trim();
        let class = if command == "start" {
            self.ui_state
                .get("start.class")
                .filter(|value| !value.trim().is_empty())
                .cloned()
        } else {
            command
                .strip_prefix("start_class.")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };

        if let Some(class) = &class {
            self.set_ui_state("start.class", class);
        }

        let Some(command) = self.create_start_player_command(class.as_deref(), assets) else {
            return;
        };
        commands.push(command);
        self.game_started = true;

        let play_screen = self.get_config_string_default("game", "play_screen", "");
        if !play_screen.trim().is_empty() {
            self.goto_screen(&play_screen, assets, scene_handler);
        }
    }

    fn create_start_player_command(
        &mut self,
        class: Option<&str>,
        assets: &Assets,
    ) -> Option<Command> {
        if self.game_started {
            return None;
        }

        let map = assets.maps.get(&self.current_map)?;
        let mut entity = map.entities.iter().find_map(|entity| {
            entity.get_attr_string("class_name").and_then(|class_name| {
                self.player_entities
                    .contains(&class_name)
                    .then(|| (entity.clone(), class_name))
            })
        })?;

        if let Some(class) = class {
            entity
                .0
                .set_attribute("_start_class", Value::Str(class.to_string()));
        }
        let player_name = self
            .ui_state
            .get("start.name")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        if let Some(player_name) = player_name {
            entity
                .0
                .set_attribute("_start_name", Value::Str(player_name.to_string()));
        }
        if let Some(entrance) = map.named_area_center("entrance") {
            entity.0.set_pos_xz(entrance);
        }
        entity.0.set_attribute("player", Value::Bool(true));
        self.pending_game_camera_pos = Some(entity.0.get_pos_xz());

        self.client_action = Arc::new(Mutex::new(ClientAction::default()));
        self.client_action.lock().unwrap().init(entity.1, assets);

        Some(Command::CreateEntity(map.id, entity.0))
    }

    fn goto_screen(
        &mut self,
        screen_name: &str,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) -> bool {
        let screen_name = screen_name.trim();
        if screen_name.is_empty() || !assets.screens.contains_key(screen_name) {
            return false;
        }

        self.current_screen = screen_name.to_string();
        self.init_screen(self.current_screen.clone(), assets, scene_handler);
        self.apply_pending_game_camera_pos();
        self.first_game_draw = true;
        true
    }

    fn apply_pending_game_camera_pos(&mut self) {
        let Some(pos) = self.pending_game_camera_pos else {
            return;
        };
        if self.game_widgets.is_empty() {
            return;
        }
        for widget in self.game_widgets.values_mut() {
            widget.player_pos = pos;
        }
        self.pending_game_camera_pos = None;
    }

    /// Draw the game into the internal buffer
    pub fn draw_game(
        &mut self,
        map: &Map,
        assets: &Assets,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
        scene_handler: &mut SceneHandler,
    ) {
        // Keep scene timing in sync with config
        scene_handler.set_timings(self.target_fps as f32, self.game_tick_ms);
        self.update_active_player_camera(map);

        // Reset the intent to the server value
        self.current_sector.clear();
        if let Some(leader) = Self::resolve_party_entity(map, Some("leader")) {
            self.intent = leader.get_attr_string("intent").unwrap_or_default();
            self.current_sector = leader
                .get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    map.find_sector_at(leader.get_pos_xz())
                        .map(|s| s.name.clone())
                })
                .unwrap_or_default();
        }

        self.target.fill([0, 0, 0, 255]);
        let say_config = self.config.clone();
        let say_fallback_color = self.messages_font_color;
        // First process the game widgets
        for widget in self.game_widgets.values_mut() {
            widget.firstp_eye_level = self.firstp_eye_level;
            widget.apply_entities(map, assets, self.animation_frame, scene_handler);
            widget.draw(
                map,
                &self.server_time,
                self.animation_frame,
                assets,
                scene_handler,
            );

            if let Some(font) = &self.messages_font {
                let widget_say = Self::say_table_from_widget(widget);
                let say_bg_enabled =
                    Self::get_widget_say_background_enabled(widget_say.as_ref(), &say_config);
                let say_bg_color =
                    Self::get_widget_say_background_color(widget_say.as_ref(), &say_config);
                if Self::is_2d_camera(&widget.camera) {
                    let width = widget.buffer.dim().width as usize;
                    let height = widget.buffer.dim().height as usize;
                    let pixels = widget.buffer.pixels_mut();
                    let overlay_scale = widget.upscale.max(1.0);
                    let tile_size = (widget.grid_size * overlay_scale).round() as isize;

                    for (grid_pos, message, text_size, category, _) in
                        self.messages_to_draw.values()
                    {
                        let color = Self::get_widget_say_color(
                            widget_say.as_ref(),
                            &say_config,
                            category,
                            say_fallback_color,
                        );
                        let sx =
                            ((grid_pos.x - widget.top_left.x) * widget.grid_size * overlay_scale)
                                .round() as isize;
                        let sy =
                            ((grid_pos.y - widget.top_left.y) * widget.grid_size * overlay_scale)
                                .round() as isize;

                        let tuple = (
                            sx - *text_size as isize / 2 - 5,
                            sy - self.messages_font_size as isize - tile_size,
                            *text_size as isize + 10,
                            22,
                        );

                        if say_bg_enabled && say_bg_color[3] > 0 {
                            self.draw2d.blend_rect_safe(
                                pixels,
                                &tuple,
                                width,
                                &say_bg_color,
                                &(0, 0, width as isize, height as isize),
                            );
                        }

                        self.draw2d.text_rect_blend_safe(
                            pixels,
                            &tuple,
                            width,
                            font,
                            self.messages_font_size,
                            message,
                            &color,
                            draw2d::TheHorizontalAlign::Center,
                            draw2d::TheVerticalAlign::Center,
                            &(0, 0, width as isize, height as isize),
                        );
                    }
                } else {
                    let width = widget.buffer.dim().width as usize;
                    let height = widget.buffer.dim().height as usize;
                    let pixels = widget.buffer.pixels_mut();

                    let view = widget.camera_d3.view_matrix();
                    let proj = widget
                        .camera_d3
                        .projection_matrix(width as f32, height as f32);
                    let vp = proj * view;

                    for (grid_pos, message, text_size, category, _) in
                        self.messages_to_draw.values()
                    {
                        let color = Self::get_widget_say_color(
                            widget_say.as_ref(),
                            &say_config,
                            category,
                            say_fallback_color,
                        );
                        let world = Vec4::new(grid_pos.x, 1.8, grid_pos.y, 1.0);
                        let clip = vp * world;
                        if clip.w <= 0.0 {
                            continue;
                        }

                        let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
                        if ndc.z < -1.0 || ndc.z > 1.0 {
                            continue;
                        }

                        let sx = ((ndc.x * 0.5 + 0.5) * width as f32) as isize;
                        let sy = ((1.0 - (ndc.y * 0.5 + 0.5)) * height as f32) as isize;

                        let tuple = (
                            sx - *text_size as isize / 2 - 5,
                            sy - self.messages_font_size as isize - 14,
                            *text_size as isize + 10,
                            22,
                        );

                        if say_bg_enabled && say_bg_color[3] > 0 {
                            self.draw2d.blend_rect_safe(
                                pixels,
                                &tuple,
                                width,
                                &say_bg_color,
                                &(0, 0, width as isize, height as isize),
                            );
                        }

                        self.draw2d.text_rect_blend_safe(
                            pixels,
                            &tuple,
                            width,
                            font,
                            self.messages_font_size,
                            message,
                            &color,
                            draw2d::TheHorizontalAlign::Center,
                            draw2d::TheVerticalAlign::Center,
                            &(0, 0, width as isize, height as isize),
                        );
                    }
                }
            }

            self.target
                .copy_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
        }
        self.draw_hovered_world_item_pile(map);

        // Negative-layer deco widgets sit between the game view and screen-rendered
        // controls, so they can dim the game without dimming command icons.
        Self::draw_deco_widgets_with_layer(
            &mut self.deco_widgets,
            &mut self.target,
            map,
            &self.currencies,
            assets,
            |layer| layer < 0,
        );

        if let Some(screen) = assets.screens.get(&self.current_screen) {
            if let Some(screen_widget) = &mut self.screen_widget {
                let (start_x, start_y) = crate::utils::align_screen_to_grid(
                    self.viewport.x as f32,
                    self.viewport.y as f32,
                    self.grid_size,
                );

                screen_widget.builder_d2.activated_widgets = self.activated_widgets.clone();
                screen_widget.grid_size = self.grid_size;

                // Add the current intent to the activated widgets
                let leader = Self::resolve_party_entity(map, Some("leader"));
                for w in self.button_widgets.iter() {
                    if Self::resolved_widget_intent_payload(w.1, assets, leader, &self.ui_state)
                        .as_deref()
                        == Some(self.intent.as_str())
                    {
                        screen_widget.builder_d2.activated_widgets.push(*w.0);
                    }
                }

                screen_widget.offset = Vec2::new(start_x, start_y);

                let base_screen = Self::screen_base_render_map(screen);
                screen_widget.build(&base_screen, assets);
                screen_widget.draw(&base_screen, &self.server_time, assets);
                Self::punch_game_widget_holes(
                    &mut screen_widget.buffer,
                    screen_widget.background_color,
                    self.game_widgets.values(),
                );

                self.target.blend_into(0, 0, &screen_widget.buffer);
            }
        }

        // Draw normal deco widgets on top of the screen render.
        Self::draw_deco_widgets_with_layer(
            &mut self.deco_widgets,
            &mut self.target,
            map,
            &self.currencies,
            assets,
            |layer| layer >= 0,
        );

        // Draw the messages on top
        for widget in &mut self.messages_widgets {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let map = widget.update_draw(
                    &mut self.target,
                    assets,
                    map,
                    &self.server_time,
                    messages.clone(),
                    choices.clone(),
                );
                if map.is_some() {
                    self.choice_map = map;
                } else if !widget.has_active_choices() {
                    self.choice_map = None;
                }
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            } else {
                let map = widget.process_messages(
                    assets,
                    map,
                    &self.server_time,
                    messages.clone(),
                    choices.clone(),
                );
                if map.is_some() {
                    self.choice_map = map;
                }
            }
        }

        // Draw the text widgets on top
        for widget in self.text_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(
                    &mut self.target,
                    map,
                    &self.currencies,
                    assets,
                    &self.server_time,
                    &self.ui_state,
                );
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            }
        }

        // Draw avatar preview widgets on top of text and below buttons.
        for widget in self.avatar_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = Self::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(&mut self.target, assets, entity, &self.draw2d);
            }
        }

        for widget in self.text_input_widgets.values() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(
                    &mut self.target,
                    assets,
                    &self.draw2d,
                    self.focused_text_input == Some(widget.id),
                );
            }
        }

        // Draw the button widgets which support inventory / gear on top
        for widget in self.button_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = Self::resolve_party_entity(map, widget.party.as_deref());
                let hovered = widget.rect.contains(Vec2::new(
                    self.cursor_pos.x as f32,
                    self.cursor_pos.y as f32,
                ));
                let resolved_command =
                    Self::resolved_widget_command(widget, assets, entity, &self.ui_state);
                let command_state = resolved_command
                    .as_deref()
                    .map(|command| rules_ui::command_state(assets, entity, command));
                let selected = self.activated_widgets.contains(&widget.id)
                    || (self.intent.trim().is_empty()
                        && Self::command_is_walk(resolved_command.as_deref()));
                let visual_state = Self::button_visual_state(
                    hovered,
                    selected,
                    self.pressed_widget == Some(widget.id),
                    command_state.as_ref(),
                );
                widget.update_draw(
                    &mut self.target,
                    map,
                    assets,
                    entity,
                    &self.draw2d,
                    &self.animation_frame,
                    visual_state,
                    resolved_command.as_deref(),
                );
                if let Some(state) = command_state {
                    if !state.enabled || state.cooldown_remaining > 0.0 {
                        Self::draw_command_state_overlay(
                            &mut self.target,
                            &self.draw2d,
                            widget.rect,
                            &state,
                        );
                    }
                }
            }
        }

        self.draw_open_container_panel(map, assets);
        self.draw_drag_drop_highlights(map);

        // Drag preview icon for inventory/equipped drag & drop.
        if self.dragging_started && self.dragging_item_id.is_some() {
            let dragged_item = self.find_dragged_item(map);
            if let Some(item) = dragged_item {
                let preview_size = 48usize;
                let x = self.cursor_pos.x as usize;
                let y = self.cursor_pos.y as usize;
                let half = preview_size / 2;
                Widget::draw_item_icon(
                    &mut self.target,
                    Rect {
                        x: x.saturating_sub(half) as f32,
                        y: y.saturating_sub(half) as f32,
                        width: preview_size as f32,
                        height: preview_size as f32,
                    },
                    assets,
                    item,
                    &self.draw2d,
                    self.animation_frame,
                );
            }
        }

        self.draw_current_target_rect(map);
        self.draw_hover_tooltip(map, assets);

        // Draw the cursor (centered on cursor_pos)
        if let Some(cursor) = self.curr_cursor {
            if let Some(tile) = assets.tiles.get(&cursor) {
                if let Some(texture) = tile.textures.first() {
                    let x = self.cursor_pos.x as isize - texture.width as isize / 2;
                    let y = self.cursor_pos.y as isize - texture.height as isize / 2;
                    let stride = self.target.stride();
                    let safe_rect = (
                        0,
                        0,
                        self.target.dim().width as usize,
                        self.target.dim().height as usize,
                    );
                    self.draw2d.blend_slice_safe(
                        self.target.pixels_mut(),
                        &texture.data,
                        &(x, y, texture.width, texture.height),
                        stride,
                        &safe_rect,
                    );
                }
            }
        }
    }

    /// Copy the game buffer into the external buffer
    pub fn insert_game_buffer(&mut self, buffer: &mut TheRGBABuffer) {
        let bg_color = [30, 30, 30, 255];

        if self.upscale_mode == "aspect" {
            // Scale to fit while maintaining aspect ratio, centered
            let dim = buffer.dim();
            let src_width = self.viewport.x as f32;
            let src_height = self.viewport.y as f32;
            let dst_width = dim.width as f32;
            let dst_height = dim.height as f32;

            let scale = (dst_width / src_width).min(dst_height / src_height);
            let scaled_width = (src_width * scale) as i32;
            let scaled_height = (src_height * scale) as i32;

            let offset_x = (dim.width - scaled_width) / 2;
            let offset_y = (dim.height - scaled_height) / 2;

            self.target_offset = Vec2::new(offset_x, offset_y);
            self.upscale_factor = scale;

            // Only fill letterbox/pillarbox areas instead of entire buffer
            Self::fill_borders(
                buffer,
                offset_x,
                offset_y,
                scaled_width,
                scaled_height,
                bg_color,
            );

            Self::scale_buffer_into(&self.target, buffer, offset_x, offset_y, scale);
        } else {
            self.upscale_factor = 1.0;
            // "none" mode: center without scaling
            buffer.fill(bg_color);
            if self.first_game_draw {
                let dim = buffer.dim();
                if dim.width > self.viewport.x {
                    self.target_offset.x = (dim.width - self.viewport.x) / 2;
                }
                if dim.height > self.viewport.y {
                    self.target_offset.y = (dim.height - self.viewport.y) / 2;
                }
                self.first_game_draw = false;
            }
            buffer.copy_into(self.target_offset.x, self.target_offset.y, &self.target);
        }
    }

    /// Returns the first game widget rect in viewport coordinates.
    pub fn game_widget_rect(&self) -> Option<Rect> {
        self.game_widgets.values().next().map(|w| w.rect)
    }

    /// Startup window size multiplier from `[viewport].window_scale` (default `1.0`).
    pub fn window_scale(&self) -> f32 {
        self.get_config_f32_default("viewport", "window_scale", 1.0)
            .max(0.1)
    }

    /// Returns the presentation transform from viewport coordinates into a surface size.
    /// Output is `(scale, offset_x, offset_y)`.
    pub fn presentation_transform_for_surface(&self, width: u32, height: u32) -> (f32, f32, f32) {
        let vw = self.viewport.x.max(1) as f32;
        let vh = self.viewport.y.max(1) as f32;
        let sw = width.max(1) as f32;
        let sh = height.max(1) as f32;

        if self.upscale_mode == "aspect" {
            let scale = (sw / vw).min(sh / vh).max(0.0001);
            let scaled_w = vw * scale;
            let scaled_h = vh * scale;
            let offset_x = ((sw - scaled_w) * 0.5).floor();
            let offset_y = ((sh - scaled_h) * 0.5).floor();
            (scale, offset_x, offset_y)
        } else {
            // "none" mode: no scaling, centered when the target is larger.
            let offset_x = ((sw - vw) * 0.5).max(0.0).floor();
            let offset_y = ((sh - vh) * 0.5).max(0.0).floor();
            (1.0, offset_x, offset_y)
        }
    }

    fn punch_game_widget_holes<'a, I>(
        buffer: &mut TheRGBABuffer,
        background_color: [u8; 4],
        widgets: I,
    ) where
        I: IntoIterator<Item = &'a GameWidget>,
    {
        let bw = buffer.dim().width.max(0) as usize;
        let bh = buffer.dim().height.max(0) as usize;
        if bw == 0 || bh == 0 {
            return;
        }

        let pixels = buffer.pixels_mut();
        for widget in widgets {
            // Match the exact integer placement used when the game widget buffer is copied
            // into the target. Using the float rect with ceil/floor can expose a 1 px edge.
            let x0 = (widget.rect.x as i32).max(0) as usize;
            let y0 = (widget.rect.y as i32).max(0) as usize;
            let x1 = x0
                .saturating_add(widget.buffer.dim().width.max(0) as usize)
                .min(bw);
            let y1 = y0
                .saturating_add(widget.buffer.dim().height.max(0) as usize)
                .min(bh);
            if x0 >= x1 || y0 >= y1 {
                continue;
            }

            for y in y0..y1 {
                let row = y * bw * 4;
                for x in x0..x1 {
                    let i = row + x * 4;
                    if pixels[i] == background_color[0]
                        && pixels[i + 1] == background_color[1]
                        && pixels[i + 2] == background_color[2]
                        && pixels[i + 3] == background_color[3]
                    {
                        pixels[i + 3] = 0;
                    }
                }
            }
        }
    }

    /// Prepare the primary game widget for direct GPU presentation.
    /// Returns false when no game widget exists.
    pub fn prepare_scenevm_direct(
        &mut self,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
        size: (u32, u32),
    ) -> bool {
        // Keep input mapping in sync with direct SceneVM presentation path.
        self.update_active_player_camera(map);
        let (scale, offset_x, offset_y) = self.presentation_transform_for_surface(size.0, size.1);
        self.upscale_factor = scale.max(0.0001);
        self.target_offset = Vec2::new(offset_x as i32, offset_y as i32);

        let Some(widget) = self.game_widgets.values_mut().next() else {
            return false;
        };

        // The direct SceneVM path renders only the game widget into the GPU scene.
        // Keep the widget's logical buffer size identical to the classic client path;
        // presentation scaling/offset is applied later by the wgpu client.
        let width = widget.rect.width.round().max(1.0) as i32;
        let height = widget.rect.height.round().max(1.0) as i32;
        let current_dim = widget.buffer.dim();
        if current_dim.width != width || current_dim.height != height {
            widget.buffer = TheRGBABuffer::new(TheDim::sized(width, height));
        }

        widget.firstp_eye_level = self.firstp_eye_level;
        widget.apply_entities(map, assets, self.animation_frame, scene_handler);
        widget.prepare_frame(
            map,
            &self.server_time,
            self.animation_frame,
            assets,
            scene_handler,
        );
        true
    }

    /// Render only screen/UI widgets into a transparent overlay buffer.
    pub fn draw_ui_overlay_only(
        &mut self,
        map: &Map,
        assets: &Assets,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
        width: u32,
        height: u32,
    ) -> &TheRGBABuffer {
        let w = width.max(1) as i32;
        let h = height.max(1) as i32;
        let dim = self.overlay.dim();
        if dim.width != w || dim.height != h {
            self.overlay = TheRGBABuffer::new(TheDim::sized(w, h));
        }
        self.overlay.fill([0, 0, 0, 0]);
        let say_config = self.config.clone();
        let say_fallback_color = self.messages_font_color;

        if let Some(leader) = Self::resolve_party_entity(map, Some("leader")) {
            self.intent = leader.get_attr_string("intent").unwrap_or_default();
            self.current_sector = leader
                .get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    map.find_sector_at(leader.get_pos_xz())
                        .map(|s| s.name.clone())
                })
                .unwrap_or_default();
        }

        // Negative-layer deco widgets sit below screen-rendered controls in the
        // direct presentation path too.
        Self::draw_deco_widgets_with_layer(
            &mut self.deco_widgets,
            &mut self.overlay,
            map,
            &self.currencies,
            assets,
            |layer| layer < 0,
        );

        if let Some(screen) = assets.screens.get(&self.current_screen)
            && let Some(screen_widget) = &mut self.screen_widget
        {
            let (start_x, start_y) =
                crate::utils::align_screen_to_grid(w as f32, h as f32, self.grid_size);

            screen_widget.builder_d2.activated_widgets = self.activated_widgets.clone();
            screen_widget.grid_size = self.grid_size;

            let leader = Self::resolve_party_entity(map, Some("leader"));
            for w in self.button_widgets.iter() {
                if Self::resolved_widget_intent_payload(w.1, assets, leader, &self.ui_state)
                    .as_deref()
                    == Some(self.intent.as_str())
                {
                    screen_widget.builder_d2.activated_widgets.push(*w.0);
                }
            }

            screen_widget.offset = Vec2::new(start_x, start_y);
            let base_screen = Self::screen_base_render_map(screen);
            screen_widget.build(&base_screen, assets);
            screen_widget.draw(&base_screen, &self.server_time, assets);
            Self::punch_game_widget_holes(
                &mut screen_widget.buffer,
                screen_widget.background_color,
                self.game_widgets.values(),
            );
            self.overlay.blend_into(0, 0, &screen_widget.buffer);
        }

        // Draw "say" bubbles projected from 3D game widgets into the overlay.
        if let Some(font) = &self.messages_font {
            let overlay_w = self.overlay.dim().width as usize;
            let overlay_h = self.overlay.dim().height as usize;
            let pixels = self.overlay.pixels_mut();
            for game in self.game_widgets.values() {
                if Self::is_2d_camera(&game.camera) {
                    continue;
                }
                let gw = game.rect.width.max(1.0) as usize;
                let gh = game.rect.height.max(1.0) as usize;
                if gw == 0 || gh == 0 {
                    continue;
                }

                let view = game.camera_d3.view_matrix();
                let proj = game.camera_d3.projection_matrix(gw as f32, gh as f32);
                let vp = proj * view;
                let widget_say = Self::say_table_from_widget(game);
                let say_bg_enabled =
                    Self::get_widget_say_background_enabled(widget_say.as_ref(), &say_config);
                let say_bg_color =
                    Self::get_widget_say_background_color(widget_say.as_ref(), &say_config);

                for (grid_pos, message, text_size, category, _) in self.messages_to_draw.values() {
                    let color = Self::get_widget_say_color(
                        widget_say.as_ref(),
                        &say_config,
                        category,
                        say_fallback_color,
                    );
                    let world = Vec4::new(grid_pos.x, 1.8, grid_pos.y, 1.0);
                    let clip = vp * world;
                    if clip.w <= 0.0 {
                        continue;
                    }

                    let ndc = Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w);
                    if ndc.z < -1.0 || ndc.z > 1.0 {
                        continue;
                    }

                    let local_sx = ((ndc.x * 0.5 + 0.5) * gw as f32) as isize;
                    let local_sy = ((1.0 - (ndc.y * 0.5 + 0.5)) * gh as f32) as isize;
                    let sx = game.rect.x as isize + local_sx;
                    let sy = game.rect.y as isize + local_sy;

                    let tuple = (
                        sx - *text_size as isize / 2 - 5,
                        sy - self.messages_font_size as isize - 14,
                        *text_size as isize + 10,
                        22,
                    );

                    if say_bg_enabled && say_bg_color[3] > 0 {
                        self.draw2d.blend_rect_safe(
                            pixels,
                            &tuple,
                            overlay_w,
                            &say_bg_color,
                            &(0, 0, overlay_w as isize, overlay_h as isize),
                        );
                    }

                    self.draw2d.text_rect_blend_safe(
                        pixels,
                        &tuple,
                        overlay_w,
                        font,
                        self.messages_font_size,
                        message,
                        &color,
                        draw2d::TheHorizontalAlign::Center,
                        draw2d::TheVerticalAlign::Center,
                        &(0, 0, overlay_w as isize, overlay_h as isize),
                    );
                }
            }
        }

        Self::draw_deco_widgets_with_layer(
            &mut self.deco_widgets,
            &mut self.overlay,
            map,
            &self.currencies,
            assets,
            |layer| layer >= 0,
        );

        for widget in &mut self.messages_widgets {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let map = widget.update_draw(
                    &mut self.overlay,
                    assets,
                    map,
                    &self.server_time,
                    messages.clone(),
                    choices.clone(),
                );
                if map.is_some() {
                    self.choice_map = map;
                } else if !widget.has_active_choices() {
                    self.choice_map = None;
                }
                self.overlay
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            } else {
                let map = widget.process_messages(
                    assets,
                    map,
                    &self.server_time,
                    messages.clone(),
                    choices.clone(),
                );
                if map.is_some() {
                    self.choice_map = map;
                }
            }
        }

        for widget in self.text_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(
                    &mut self.overlay,
                    map,
                    &self.currencies,
                    assets,
                    &self.server_time,
                    &self.ui_state,
                );
                self.overlay
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            }
        }

        for widget in self.avatar_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = Self::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(&mut self.overlay, assets, entity, &self.draw2d);
            }
        }

        for widget in self.text_input_widgets.values() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(
                    &mut self.overlay,
                    assets,
                    &self.draw2d,
                    self.focused_text_input == Some(widget.id),
                );
            }
        }

        for widget in self.button_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = Self::resolve_party_entity(map, widget.party.as_deref());
                let hovered = widget.rect.contains(Vec2::new(
                    self.cursor_pos.x as f32,
                    self.cursor_pos.y as f32,
                ));
                let resolved_command =
                    Self::resolved_widget_command(widget, assets, entity, &self.ui_state);
                let command_state = resolved_command
                    .as_deref()
                    .map(|command| rules_ui::command_state(assets, entity, command));
                let selected = self.activated_widgets.contains(&widget.id)
                    || (self.intent.trim().is_empty()
                        && Self::command_is_walk(resolved_command.as_deref()));
                let visual_state = Self::button_visual_state(
                    hovered,
                    selected,
                    self.pressed_widget == Some(widget.id),
                    command_state.as_ref(),
                );
                widget.update_draw(
                    &mut self.overlay,
                    map,
                    assets,
                    entity,
                    &self.draw2d,
                    &self.animation_frame,
                    visual_state,
                    resolved_command.as_deref(),
                );
                if let Some(state) = command_state {
                    if !state.enabled || state.cooldown_remaining > 0.0 {
                        Self::draw_command_state_overlay(
                            &mut self.overlay,
                            &self.draw2d,
                            widget.rect,
                            &state,
                        );
                    }
                }
            }
        }

        if self.dragging_started && self.dragging_item_id.is_some() {
            let dragged_item = self.find_dragged_item(map);
            if let Some(item) = dragged_item
                && let Some(Value::Source(source)) = item.attributes.get("source")
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                let index = self.animation_frame % tile.textures.len();
                let texture = &tile.textures[index];
                let preview_size = 28usize;
                let x = self.cursor_pos.x as usize;
                let y = self.cursor_pos.y as usize;
                let half = preview_size / 2;
                let stride = self.overlay.stride();
                self.draw2d.blend_scale_chunk(
                    self.overlay.pixels_mut(),
                    &(
                        x.saturating_sub(half),
                        y.saturating_sub(half),
                        preview_size,
                        preview_size,
                    ),
                    stride,
                    &texture.data,
                    &(texture.width, texture.height),
                );
            }
        }

        std::mem::swap(&mut self.target, &mut self.overlay);
        self.draw_hover_tooltip(map, assets);
        std::mem::swap(&mut self.target, &mut self.overlay);

        if let Some(cursor) = self.curr_cursor
            && let Some(tile) = assets.tiles.get(&cursor)
            && let Some(texture) = tile.textures.first()
        {
            let x = self.cursor_pos.x as isize - texture.width as isize / 2;
            let y = self.cursor_pos.y as isize - texture.height as isize / 2;
            let stride = self.overlay.stride();
            let safe_rect = (
                0,
                0,
                self.overlay.dim().width as usize,
                self.overlay.dim().height as usize,
            );
            self.draw2d.blend_slice_safe(
                self.overlay.pixels_mut(),
                &texture.data,
                &(x, y, texture.width, texture.height),
                stride,
                &safe_rect,
            );
        }

        &self.overlay
    }

    /// Fill only the border areas (letterbox/pillarbox) around the content area.
    fn fill_borders(
        buffer: &mut TheRGBABuffer,
        offset_x: i32,
        offset_y: i32,
        content_width: i32,
        content_height: i32,
        color: [u8; 4],
    ) {
        let dim = buffer.dim();
        let buf_width = dim.width as usize;
        let buf_height = dim.height as usize;
        let pixels = buffer.pixels_mut();

        // Top border
        if offset_y > 0 {
            let top_pixels = offset_y as usize * buf_width * 4;
            for chunk in pixels[..top_pixels].chunks_exact_mut(4) {
                chunk.copy_from_slice(&color);
            }
        }

        // Bottom border
        let bottom_start_y = (offset_y + content_height) as usize;
        if bottom_start_y < buf_height {
            let bottom_start = bottom_start_y * buf_width * 4;
            for chunk in pixels[bottom_start..].chunks_exact_mut(4) {
                chunk.copy_from_slice(&color);
            }
        }

        // Left and right borders (only for rows in the content area)
        let content_start_y = offset_y.max(0) as usize;
        let content_end_y = ((offset_y + content_height) as usize).min(buf_height);

        for y in content_start_y..content_end_y {
            let row_start = y * buf_width * 4;

            // Left border
            if offset_x > 0 {
                let left_end = row_start + offset_x as usize * 4;
                for chunk in pixels[row_start..left_end].chunks_exact_mut(4) {
                    chunk.copy_from_slice(&color);
                }
            }

            // Right border
            let right_start_x = (offset_x + content_width) as usize;
            if right_start_x < buf_width {
                let right_start = row_start + right_start_x * 4;
                let row_end = row_start + buf_width * 4;
                for chunk in pixels[right_start..row_end].chunks_exact_mut(4) {
                    chunk.copy_from_slice(&color);
                }
            }
        }
    }

    /// Scale source buffer into destination buffer at the given offset and scale factor.
    fn scale_buffer_into(
        src: &TheRGBABuffer,
        dst: &mut TheRGBABuffer,
        offset_x: i32,
        offset_y: i32,
        scale: f32,
    ) {
        let src_width = src.dim().width as usize;
        let src_height = src.dim().height as usize;
        let dst_width = dst.dim().width as usize;
        let dst_height = dst.dim().height as usize;

        let scaled_width = (src_width as f32 * scale) as i32;
        let scaled_height = (src_height as f32 * scale) as i32;

        // Pre-calculate valid render bounds
        let y_start = 0.max(-offset_y);
        let y_end = scaled_height.min(dst_height as i32 - offset_y);
        let x_start = 0.max(-offset_x);
        let x_end = scaled_width.min(dst_width as i32 - offset_x);

        if y_start >= y_end || x_start >= x_end {
            return;
        }

        let src_pixels = src.pixels();
        let dst_pixels = dst.pixels_mut();

        let inv_scale = 1.0 / scale;

        // Pre-compute source X indices for the row
        let src_x_indices: Vec<usize> = (x_start..x_end)
            .map(|dx| ((dx as f32 * inv_scale) as usize).min(src_width - 1))
            .collect();

        let dst_x_offset = (offset_x + x_start) as usize * 4;

        // Process rows in parallel
        dst_pixels
            .par_chunks_mut(dst_width * 4)
            .enumerate()
            .skip((offset_y + y_start) as usize)
            .take((y_end - y_start) as usize)
            .for_each(|(dst_y, dst_row)| {
                let dy = dst_y as i32 - offset_y;
                let src_y = ((dy as f32 * inv_scale) as usize).min(src_height - 1);
                let src_row_start = src_y * src_width * 4;

                for (i, &src_x) in src_x_indices.iter().enumerate() {
                    let dst_idx = dst_x_offset + i * 4;
                    let src_idx = src_row_start + src_x * 4;
                    dst_row[dst_idx..dst_idx + 4]
                        .copy_from_slice(&src_pixels[src_idx..src_idx + 4]);
                }
            });
    }

    /// Transform screen coordinates to viewport coordinates, accounting for offset and scale.
    fn screen_to_viewport(&self, coord: Vec2<i32>) -> Vec2<i32> {
        let x = ((coord.x - self.target_offset.x) as f32 / self.upscale_factor) as i32;
        let y = ((coord.y - self.target_offset.y) as f32 / self.upscale_factor) as i32;
        Vec2::new(x, y)
    }

    fn has_drag_drop_targets(&self) -> bool {
        self.button_widgets.values().any(|widget| {
            widget.drag_drop && (widget.inventory_index.is_some() || widget.equipped_slot.is_some())
        })
    }

    fn item_is_container(item: &crate::Item) -> bool {
        item.is_container()
            || item.attributes.get_bool_default("container", false)
            || item.attributes.get_int_default("container_slots", 0) > 0
    }

    fn item_can_enter_container(item: &Item, container: &Item) -> bool {
        let max_capacity = container.max_capacity.max(1) as usize;
        let contents = container.container.as_ref();
        if contents.is_none_or(|contents| contents.len() < max_capacity) {
            return true;
        }
        contents.is_some_and(|contents| {
            contents
                .iter()
                .any(|existing| existing.can_stack_with(item))
        })
    }

    fn find_container_item<'a>(
        map: &'a Map,
        item_id: u32,
        owner_entity_id: Option<u32>,
    ) -> Option<&'a crate::Item> {
        if let Some(owner_id) = owner_entity_id
            && let Some(entity) = map.entities.iter().find(|entity| entity.id == owner_id)
        {
            return entity
                .inventory
                .iter()
                .flatten()
                .chain(entity.equipped.values())
                .find(|item| item.id == item_id);
        }

        map.items
            .iter()
            .find(|item| item.id == item_id)
            .or_else(|| {
                map.entities.iter().find_map(|entity| {
                    entity
                        .inventory
                        .iter()
                        .flatten()
                        .chain(entity.equipped.values())
                        .find(|item| item.id == item_id)
                })
            })
    }

    fn toggle_container_panel(&mut self, item_id: u32, owner_entity_id: Option<u32>, anchor: Rect) {
        if self.open_container_panel.is_some_and(|panel| {
            panel.item_id == item_id && panel.owner_entity_id == owner_entity_id
        }) {
            self.close_floaters();
        } else {
            self.open_container_panel_at_anchor(item_id, owner_entity_id, Some(anchor));
        }
    }

    pub fn process_open_container_requests(
        &mut self,
        requests: Vec<crate::server::OpenContainerRequest>,
    ) {
        for (item_id, owner_entity_id) in requests {
            self.open_container_panel_at_anchor(item_id, owner_entity_id, None);
        }
    }

    fn open_container_panel_at_anchor(
        &mut self,
        item_id: u32,
        owner_entity_id: Option<u32>,
        anchor: Option<Rect>,
    ) {
        let target_width = self.target.dim().width as i32;
        let target_height = self.target.dim().height as i32;
        let position = self
            .open_container_panel_positions
            .get(&(item_id, owner_entity_id))
            .copied()
            .unwrap_or_else(|| {
                if let Some(anchor) = anchor {
                    let x = (anchor.x + anchor.width + 12.0)
                        .round()
                        .clamp(2.0, (target_width - 24).max(2) as f32)
                        as i32;
                    let y = anchor
                        .y
                        .round()
                        .clamp(2.0, (target_height - 24).max(2) as f32)
                        as i32;
                    Vec2::new(x, y)
                } else {
                    Vec2::new(
                        (target_width / 2 - 96).max(2),
                        (target_height / 2 - 64).max(2),
                    )
                }
            });
        self.open_container_panel = Some(OpenContainerPanel {
            item_id,
            owner_entity_id,
            position: Vec2::new(
                position.x.clamp(2, (target_width - 24).max(2)),
                position.y.clamp(2, (target_height - 24).max(2)),
            ),
        });
        self.open_container_panel_rect = None;
        self.open_container_slot_rects.clear();
        self.open_container_title_rect = None;
        self.open_container_close_rect = None;
        self.dragging_container_panel = false;
    }

    fn close_floaters(&mut self) -> bool {
        let had_floater = self.open_container_panel.is_some();
        if let Some(panel) = self.open_container_panel {
            self.open_container_panel_positions
                .insert((panel.item_id, panel.owner_entity_id), panel.position);
        }
        self.open_container_panel = None;
        self.open_container_panel_rect = None;
        self.open_container_slot_rects.clear();
        self.open_container_title_rect = None;
        self.open_container_close_rect = None;
        self.dragging_container_panel = false;
        self.tooltip_hover_key = None;
        self.tooltip_hover_since = None;
        had_floater
    }

    fn open_container_item<'a>(&self, map: &'a Map) -> Option<&'a crate::Item> {
        let panel = self.open_container_panel?;
        let item = Self::find_container_item(map, panel.item_id, panel.owner_entity_id)?;
        Self::item_is_container(item).then_some(item)
    }

    fn container_panel_layout(&self, map: &Map, assets: &Assets) -> Option<ContainerPanelLayout> {
        let item = self.open_container_item(map)?;
        let template = rules_ui::container_template_for_item(assets, item);
        let slots = item.attributes.get_int_default("container_slots", 0).max(0) as usize;
        let item_count = item.container.as_ref().map(Vec::len).unwrap_or(0);
        let slot_count = slots.max(item.max_capacity as usize).max(item_count).max(1);
        Some(Self::build_container_panel_layout(
            self.open_container_panel?.position,
            &template,
            slot_count,
            self.target.dim().width as i32,
            self.target.dim().height as i32,
        ))
    }

    fn open_container_slot_item_at_point<'a>(
        &'a self,
        map: &'a Map,
        p: Vec2<i32>,
    ) -> Option<(ContainerItemSource, &'a Item, Rect)> {
        let panel = self.open_container_panel?;
        let container = self.open_container_item(map)?;
        let point = Vec2::new(p.x as f32, p.y as f32);
        self.open_container_slot_rects
            .iter()
            .enumerate()
            .find_map(|(index, slot)| {
                if !slot.contains(point) {
                    return None;
                }
                let item = container
                    .container
                    .as_ref()
                    .and_then(|items| items.get(index))?;
                Some((
                    ContainerItemSource {
                        container_item_id: panel.item_id,
                        container_owner_entity_id: panel.owner_entity_id,
                    },
                    item,
                    *slot,
                ))
            })
    }

    fn clear_item_drag(&mut self) {
        self.dragging_item_id = None;
        self.dragging_item_owner_entity_id = None;
        self.dragging_source_widget_id = None;
        self.dragging_item_from_world = false;
        self.dragging_item_container_source = None;
        self.dragging_started = false;
        self.pressed_widget = None;
    }

    fn build_container_panel_layout(
        position: Vec2<i32>,
        template: &ContainerUiTemplate,
        slot_count: usize,
        target_width: i32,
        target_height: i32,
    ) -> ContainerPanelLayout {
        let columns = template.columns.max(1);
        let rows = template
            .rows
            .unwrap_or_else(|| slot_count.div_ceil(columns))
            .max(1);
        let slot_size = template.slot_size.max(8);
        let gap = template.gap.max(0);
        let padding = template.padding.max(0);
        let title_h = if template.title { 26 } else { 0 };
        let content_w = columns as i32 * slot_size + (columns.saturating_sub(1) as i32 * gap);
        let content_h = rows as i32 * slot_size + (rows.saturating_sub(1) as i32 * gap);
        let width = content_w + padding * 2;
        let height = content_h + padding * 2 + title_h;

        let mut x = position.x;
        let mut y = position.y;
        if x + width > target_width {
            x = (target_width - width - 2).max(2);
        }
        x = x.max(2);
        if y + height > target_height {
            y = (target_height - height - 2).max(2);
        }
        y = y.max(2);

        let title_bar_rect = template.title.then_some(Rect {
            x: x as f32,
            y: y as f32,
            width: width as f32,
            height: title_h as f32,
        });
        let close_rect = template.title.then_some(Rect {
            x: (x + width - 27) as f32,
            y: (y + 3) as f32,
            width: 22.0,
            height: 20.0,
        });

        let start_x = x + padding;
        let start_y = y + padding + title_h;
        let mut slot_rects = Vec::with_capacity(slot_count);
        for index in 0..slot_count {
            let col = index % columns;
            let row = index / columns;
            slot_rects.push(Rect {
                x: (start_x + col as i32 * (slot_size + gap)) as f32,
                y: (start_y + row as i32 * (slot_size + gap)) as f32,
                width: slot_size as f32,
                height: slot_size as f32,
            });
        }

        ContainerPanelLayout {
            rect: Rect {
                x: x as f32,
                y: y as f32,
                width: width as f32,
                height: height as f32,
            },
            slots: slot_rects,
            title_bar_rect,
            close_rect,
            title_rect: template.title.then_some((
                (x + padding) as isize,
                (y + 3) as isize,
                (width - padding * 2 - 30).max(1) as isize,
                20,
            )),
        }
    }

    fn party_members<'a>(map: &'a Map) -> Vec<&'a Entity> {
        let mut members: Vec<&Entity> = map
            .entities
            .iter()
            .filter(|entity| {
                entity.is_player()
                    || entity.attributes.get_int("party_index").is_some()
                    || entity.attributes.get_bool_default("party_member", false)
                    || entity
                        .attributes
                        .get_str("party_role")
                        .is_some_and(|role| !role.trim().is_empty())
            })
            .collect();

        if members.is_empty() {
            if let Some(player) = map.entities.iter().find(|entity| entity.is_player()) {
                members.push(player);
            }
        }

        members.sort_by_key(|entity| {
            (
                entity
                    .attributes
                    .get_int("party_index")
                    .unwrap_or_else(|| if entity.is_player() { 0 } else { i32::MAX / 2 }),
                entity.id,
            )
        });
        members
    }

    fn resolve_party_entity<'a>(map: &'a Map, binding: Option<&str>) -> Option<&'a Entity> {
        let binding = binding.map(str::trim).filter(|value| !value.is_empty());
        let members = Self::party_members(map);

        match binding {
            None | Some("leader") | Some("player") => members.first().copied(),
            Some(value) => {
                if let Some(index) = value.strip_prefix("party.")
                    && let Ok(index) = index.parse::<usize>()
                {
                    return members.get(index).copied();
                }

                members
                    .iter()
                    .copied()
                    .find(|entity| {
                        entity.attributes.get_str("party_role") == Some(value)
                            || entity.attributes.get_str("name") == Some(value)
                    })
                    .or_else(|| members.first().copied())
            }
        }
    }

    fn item_click_distance(map: &Map, item_id: u32) -> f32 {
        let Some(player_pos) = Self::resolve_party_entity(map, Some("leader"))
            .or_else(|| map.entities.iter().find(|entity| entity.is_player()))
            .map(|entity| entity.get_pos_xz())
        else {
            return 0.0;
        };

        map.items
            .iter()
            .find(|item| item.id == item_id)
            .map(|item| player_pos.distance(item.get_pos_xz()))
            .unwrap_or(0.0)
    }

    fn find_dragged_item<'a>(&self, map: &'a Map) -> Option<&'a Item> {
        let item_id = self.dragging_item_id?;

        if let Some(source) = self.dragging_item_container_source {
            return Self::find_container_item(
                map,
                source.container_item_id,
                source.container_owner_entity_id,
            )
            .and_then(|container| {
                container
                    .container
                    .as_ref()
                    .and_then(|items| items.iter().find(|item| item.id == item_id))
            });
        }

        if let Some(owner_id) = self.dragging_item_owner_entity_id
            && let Some(owner) = map.entities.iter().find(|entity| entity.id == owner_id)
            && let Some(item) = owner
                .get_item(item_id)
                .or_else(|| owner.equipped.values().find(|item| item.id == item_id))
        {
            return Some(item);
        }

        map.entities
            .iter()
            .find_map(|entity| {
                entity
                    .get_item(item_id)
                    .or_else(|| entity.equipped.values().find(|item| item.id == item_id))
            })
            .or_else(|| map.items.iter().find(|item| item.id == item_id))
    }

    fn drag_distance_exceeded(&self, p: Vec2<i32>) -> bool {
        (p - self.drag_start_pos).map(|v| v as f32).magnitude() >= 6.0
    }

    fn move_open_container_panel_to_cursor(&mut self, p: Vec2<i32>) {
        if let Some(panel) = self.open_container_panel.as_mut() {
            let target_width = self.target.dim().width as i32;
            let target_height = self.target.dim().height as i32;
            panel.position = Vec2::new(
                (p.x - self.container_panel_drag_offset.x).clamp(2, (target_width - 24).max(2)),
                (p.y - self.container_panel_drag_offset.y).clamp(2, (target_height - 24).max(2)),
            );
            self.open_container_panel_rect = None;
            self.open_container_slot_rects.clear();
            self.open_container_title_rect = None;
            self.open_container_close_rect = None;
        }
        self.tooltip_hover_key = None;
        self.tooltip_hover_since = None;
    }

    fn quantize_2d_tile_pos(pos: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(pos.x.floor(), pos.y.floor())
    }

    fn active_intent_cursor_ids(
        &self,
    ) -> Option<(Option<Uuid>, Option<Uuid>, Option<Uuid>, Option<Uuid>)> {
        self.activated_widgets.iter().rev().find_map(|button_id| {
            self.button_widgets.get(button_id).and_then(|widget| {
                let has_intent = widget
                    .intent_payload()
                    .map(|intent| !intent.trim().is_empty())
                    .unwrap_or(false)
                    || (widget.command_slot.is_some() && !self.intent.trim().is_empty());
                has_intent.then(|| {
                    (
                        widget.entity_cursor_id,
                        widget.entity_clicked_cursor_id,
                        widget.item_cursor_id,
                        widget.item_clicked_cursor_id,
                    )
                })
            })
        })
    }

    fn apply_active_intent_cursor(&mut self, entity_target: bool, item_target: bool) {
        let Some((
            entity_cursor_id,
            entity_clicked_cursor_id,
            item_cursor_id,
            item_clicked_cursor_id,
        )) = self.active_intent_cursor_ids()
        else {
            return;
        };

        if entity_target {
            self.curr_intent_cursor = entity_cursor_id.or(item_cursor_id);
            self.curr_clicked_intent_cursor = entity_clicked_cursor_id.or(item_clicked_cursor_id);
            if let Some(cursor_id) = self.curr_intent_cursor {
                self.curr_cursor = Some(cursor_id);
            }
        } else if item_target {
            self.curr_intent_cursor = item_cursor_id;
            self.curr_clicked_intent_cursor = item_clicked_cursor_id;
            if let Some(cursor_id) = self.curr_intent_cursor {
                self.curr_cursor = Some(cursor_id);
            }
        }
    }

    fn immediate_2d_intent_mode(&self) -> bool {
        let camera = self
            .active_player_camera
            .clone()
            .or_else(|| self.active_game_widget_camera_mode());
        matches!(
            camera,
            Some(crate::PlayerCamera::D2 | crate::PlayerCamera::D2Grid)
        ) && !self.click_intents_2d
    }

    fn is_movement_action(action: &EntityAction) -> bool {
        matches!(
            action,
            EntityAction::Forward
                | EntityAction::Backward
                | EntityAction::Left
                | EntityAction::Right
                | EntityAction::StrafeLeft
                | EntityAction::StrafeRight
                | EntityAction::ForwardLeft
                | EntityAction::ForwardRight
                | EntityAction::BackwardLeft
                | EntityAction::BackwardRight
        )
    }

    fn consume_one_shot_2d_intent(&mut self) {
        if self.click_intents_2d || !self.game_widget_is_2d() {
            return;
        }
        self.intent.clear();
        self.activated_widgets.retain(|id| {
            self.button_widgets
                .get(id)
                .map(|widget| {
                    widget.command_slot.is_none()
                        && widget
                            .intent_payload()
                            .map(|intent| intent.trim().is_empty())
                            .unwrap_or(true)
                })
                .unwrap_or(true)
        });
        self.permanently_activated_widgets.retain(|id| {
            self.button_widgets
                .get(id)
                .map(|widget| {
                    widget.command_slot.is_none()
                        && widget
                            .intent_payload()
                            .map(|intent| intent.trim().is_empty())
                            .unwrap_or(true)
                })
                .unwrap_or(true)
        });
        self.curr_intent_cursor = None;
        self.curr_clicked_intent_cursor = None;
        self.curr_cursor = self.default_cursor;
    }

    fn is_targeting_button(widget: &Widget) -> bool {
        widget.command_slot.is_some()
            || matches!(
                widget.command_binding(),
                Some(ClientCommandBinding::Intent(_) | ClientCommandBinding::RulesAction(_))
            )
    }

    fn activate_targeting_button(&mut self, button_id: u32) {
        self.activated_widgets.retain(|id| {
            *id != button_id
                && self
                    .button_widgets
                    .get(id)
                    .map(|widget| !Self::is_targeting_button(widget))
                    .unwrap_or(true)
        });
        self.permanently_activated_widgets.retain(|id| {
            *id != button_id
                && self
                    .button_widgets
                    .get(id)
                    .map(|widget| !Self::is_targeting_button(widget))
                    .unwrap_or(true)
        });
        if !self.activated_widgets.contains(&button_id) {
            self.activated_widgets.push(button_id);
        }
        if !self.permanently_activated_widgets.contains(&button_id) {
            self.permanently_activated_widgets.push(button_id);
        }

        if let Some(widget) = self.button_widgets.get(&button_id) {
            self.curr_intent_cursor = widget.item_cursor_id;
            self.curr_clicked_intent_cursor = widget.item_clicked_cursor_id;
            self.curr_cursor = self.default_cursor;
        }
    }

    fn activate_walk_button(&mut self, button_id: u32) {
        self.intent.clear();
        self.activated_widgets.retain(|id| {
            *id != button_id
                && self
                    .button_widgets
                    .get(id)
                    .map(|widget| !Self::is_targeting_button(widget))
                    .unwrap_or(true)
        });
        self.permanently_activated_widgets.retain(|id| {
            *id != button_id
                && self
                    .button_widgets
                    .get(id)
                    .map(|widget| !Self::is_targeting_button(widget))
                    .unwrap_or(true)
        });
        if !self.activated_widgets.contains(&button_id) {
            self.activated_widgets.push(button_id);
        }
        if !self.permanently_activated_widgets.contains(&button_id) {
            self.permanently_activated_widgets.push(button_id);
        }
        self.curr_intent_cursor = None;
        self.curr_clicked_intent_cursor = None;
        self.curr_cursor = self.default_cursor;
    }

    fn drop_position_at_viewport(&self, p: Vec2<i32>) -> Option<Vec2<f32>> {
        for widget in self.game_widgets.values() {
            if !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                continue;
            }
            if Self::is_2d_camera(&widget.camera) {
                let dx = p.x as f32 - widget.rect.x;
                let dy = p.y as f32 - widget.rect.y;
                let gx = widget.top_left.x + dx / widget.grid_size;
                let gy = widget.top_left.y + dy / widget.grid_size;
                return Some(Vec2::new(gx, gy));
            }

            if let Some(world_pos) = self.hovered_world_pos {
                return Some(Vec2::new(world_pos.x, world_pos.z));
            }
        }
        None
    }

    /// Check if a screen coordinate is inside the game viewport area.
    pub fn is_inside_game(&self, coord: Vec2<i32>) -> bool {
        let p = self.screen_to_viewport(coord);
        p.x >= 0 && p.y >= 0 && p.x < self.viewport.x && p.y < self.viewport.y
    }

    fn hovered_3d_widget_at(&self, p: Vec2<i32>) -> bool {
        self.game_widgets.values().any(|widget| {
            widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
                && !Self::is_2d_camera(&widget.camera)
        })
    }

    fn should_refresh_3d_hover_pick(&mut self) -> bool {
        const HOVER_PICK_INTERVAL: Duration = Duration::from_millis(200);
        let now = Instant::now();
        if let Some(last) = self.last_3d_hover_pick_at
            && now.saturating_duration_since(last) < HOVER_PICK_INTERVAL
        {
            return false;
        }
        self.last_3d_hover_pick_at = Some(now);
        true
    }

    /// Drag event
    pub fn touch_dragged(
        &mut self,
        coord: Vec2<i32>,
        _map: &Map,
        _scene_handler: &mut SceneHandler,
    ) {
        let p = self.screen_to_viewport(coord);
        self.cursor_pos = p;
        if self.dragging_container_panel {
            self.move_open_container_panel_to_cursor(p);
            return;
        }
        if self.dragging_item_id.is_some() && !self.dragging_started {
            if self.drag_distance_exceeded(p) {
                self.dragging_started = true;
            }
        }

        if self.dragging_item_id.is_some() {
            if self.hovered_3d_widget_at(p) && !self.should_refresh_3d_hover_pick() {
                return;
            }
            self.hovered_world_pos = None;
            for widget in self.game_widgets.values() {
                if !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
                    || Self::is_2d_camera(&widget.camera)
                {
                    continue;
                }
                let dx = p.x as f32 - widget.rect.x;
                let dy = p.y as f32 - widget.rect.y;
                let screen_uv = Vec2::new(dx / widget.rect.width, dy / widget.rect.height);
                if let Some((_, world_pos, _)) = _scene_handler.vm.pick_geo_id_at_uv(
                    widget.rect.width as u32,
                    widget.rect.height as u32,
                    [screen_uv.x, screen_uv.y],
                    false,
                    true,
                ) {
                    self.hovered_world_pos = Some(world_pos);
                }
                break;
            }
        }
    }

    ///Hover event, used to adjust the screen cursor based on the widget or game object under the mouse
    pub fn touch_hover(&mut self, coord: Vec2<i32>, map: &Map, scene_handler: &mut SceneHandler) {
        let p = self.screen_to_viewport(coord);
        self.cursor_pos = p;
        if self.dragging_container_panel {
            self.move_open_container_panel_to_cursor(p);
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return;
        }
        let drop_intent_active = self
            .get_current_intent()
            .map(|i| i.eq_ignore_ascii_case("drop"))
            .unwrap_or(false);

        if !drop_intent_active
            && self.hovered_3d_widget_at(p)
            && !self.should_refresh_3d_hover_pick()
        {
            return;
        }

        // Temporary, we have to make this widget dependent
        self.curr_cursor = self.default_cursor;
        self.hovered_entity_id = None;
        self.hovered_item_id = None;
        self.hovered_world_pos = None;
        self.curr_intent_cursor = None;
        self.curr_clicked_intent_cursor = None;
        self.hover_distance = f32::MAX;
        let mut pending_cursor_target: Option<(bool, bool)> = None;

        if self
            .open_container_panel_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            return;
        }

        // Drop intent targets inventory/equipped widgets, not world billboards/items.
        if drop_intent_active {
            for (_, widget) in self.button_widgets.iter() {
                if !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    continue;
                }

                let mut has_item = false;
                if let Some(entity) = Self::resolve_party_entity(map, widget.party.as_deref()) {
                    if let Some(inventory_index) = widget.inventory_index {
                        has_item = entity
                            .inventory
                            .get(inventory_index)
                            .and_then(|item| item.as_ref())
                            .is_some();
                    } else if let Some(slot) = &widget.equipped_slot {
                        has_item = entity.get_equipped_item(slot).is_some();
                    }
                }

                if has_item {
                    // Cursor style comes from the active intent button(s), same as world hover.
                    for button_id in &self.activated_widgets {
                        if let Some(active_widget) = self.button_widgets.get(button_id) {
                            self.curr_intent_cursor = active_widget.item_cursor_id;
                            self.curr_clicked_intent_cursor = active_widget.item_clicked_cursor_id;
                            if let Some(cursor_id) = active_widget.item_cursor_id {
                                self.curr_cursor = Some(cursor_id);
                            }
                        }
                    }
                }
            }
            return;
        }

        for (_, widget) in self.game_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                let dx = p.x as f32 - widget.rect.x;
                let dy = p.y as f32 - widget.rect.y;

                if Self::is_2d_camera(&widget.camera) {
                    let gx = widget.top_left.x + dx / widget.grid_size;
                    let gy = widget.top_left.y + dy / widget.grid_size;
                    let tile_pos = Self::quantize_2d_tile_pos(Vec2::new(gx, gy));

                    if let Some(entity) = map.entities.iter().find(|entity| {
                        Self::quantize_2d_tile_pos(entity.get_pos_xz()) == tile_pos
                            && entity.attributes.get_str_default("mode", "active".into()) != "dead"
                    }) {
                        self.hovered_entity_id = Some(entity.id);
                        pending_cursor_target = Some((true, false));
                    } else if let Some(item) = map
                        .items
                        .iter()
                        .rev()
                        .find(|item| Self::quantize_2d_tile_pos(item.get_pos_xz()) == tile_pos)
                    {
                        self.hovered_item_id = Some(item.id);
                        pending_cursor_target = Some((false, true));
                    } else if let Some(entity) = map
                        .entities
                        .iter()
                        .find(|entity| Self::quantize_2d_tile_pos(entity.get_pos_xz()) == tile_pos)
                    {
                        self.hovered_entity_id = Some(entity.id);
                        pending_cursor_target = Some((true, false));
                    }
                } else {
                    // We cast a ray into the game view and get the GeoId
                    let screen_uv = Vec2::new(dx / widget.rect.width, dy / widget.rect.height);
                    if let Some((geoid, world_pos, distance)) = scene_handler.vm.pick_geo_id_at_uv(
                        widget.rect.width as u32,
                        widget.rect.height as u32,
                        [screen_uv.x, screen_uv.y],
                        false,
                        true,
                    ) {
                        self.hovered_world_pos = Some(world_pos);
                        match geoid {
                            GeoId::Character(entity_id) => {
                                self.hovered_entity_id = Some(entity_id);
                                self.hover_distance = distance;
                                pending_cursor_target = Some((true, false));
                            }
                            GeoId::Hole(sector_id, hole_id) => {
                                if let Some(item) = SceneHandler::find_item_by_profile_attrs(
                                    map,
                                    Some(sector_id),
                                    Some(hole_id),
                                ) {
                                    // if let Some(cursor_id_str) = item.get_attr_string("cursor_id") {
                                    //     if !cursor_id_str.is_empty() {
                                    //         if let Ok(uuid) = Uuid::parse_str(&cursor_id_str) {
                                    //             self.curr_cursor = Some(uuid);
                                    //         }
                                    //     }
                                    // }
                                    self.hovered_item_id = Some(item.id);
                                    self.hover_distance = distance;
                                    pending_cursor_target = Some((false, true));
                                }
                            }
                            GeoId::Sector(sector_id) => {
                                if let Some(item) =
                                    SceneHandler::find_item_by_sector_id(map, sector_id)
                                {
                                    self.hovered_item_id = Some(item.id);
                                    self.hover_distance = distance;
                                    pending_cursor_target = Some((false, true));
                                }
                            }
                            GeoId::Item(item_id) => {
                                self.hovered_item_id = Some(item_id);
                                self.hover_distance = distance;
                                pending_cursor_target = Some((false, true));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if let Some((entity_target, item_target)) = pending_cursor_target {
            self.apply_active_intent_cursor(entity_target, item_target);
        }
    }

    /// Click / touch down event
    pub fn touch_down(
        &mut self,
        coord: Vec2<i32>,
        map: &Map,
        assets: &Assets,
    ) -> Option<EntityAction> {
        let mut action = None;
        let mut camera_action = None;
        let mut render_camera_switches: Vec<(Option<String>, PlayerCamera)> = Vec::new();
        let mut selected_walk_button_id = None;
        let mut selected_targeting_button_id = None;
        let mut bound_state_update: Option<(String, String)> = None;
        let active_intent = self.get_current_intent_for_action();
        self.dragging_item_id = None;
        self.dragging_item_owner_entity_id = None;
        self.dragging_source_widget_id = None;
        self.dragging_item_from_world = false;
        self.dragging_item_container_source = None;
        self.dragging_started = false;

        // Adjust cursor
        if self.curr_clicked_intent_cursor.is_some() {
            self.curr_cursor = self.curr_clicked_intent_cursor;
        } else {
            self.curr_cursor = self.default_cursor;
        }

        // Transform screen coordinates to viewport coordinates
        let p = self.screen_to_viewport(coord);

        if let Some(close_rect) = self.open_container_close_rect
            && close_rect.contains(Vec2::new(p.x as f32, p.y as f32))
        {
            self.close_floaters();
            return None;
        }

        if let Some(title_rect) = self.open_container_title_rect
            && title_rect.contains(Vec2::new(p.x as f32, p.y as f32))
            && let Some(panel) = self.open_container_panel
        {
            self.dragging_container_panel = true;
            self.container_panel_drag_offset =
                Vec2::new(p.x - panel.position.x, p.y - panel.position.y);
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return None;
        }

        if let Some((source, item, _)) = self.open_container_slot_item_at_point(map, p) {
            self.dragging_item_id = Some(item.id);
            self.dragging_item_container_source = Some(source);
            self.drag_start_pos = p;
            return None;
        }

        if let Some(rect) = self.open_container_panel_rect
            && rect.contains(Vec2::new(p.x as f32, p.y as f32))
        {
            return None;
        }

        let mut clicked_text_input = None;
        for (id, widget) in self.text_input_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                clicked_text_input = Some(*id);
                break;
            }
        }
        if clicked_text_input.is_some() {
            self.focused_text_input = clicked_text_input;
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return None;
        }
        self.focused_text_input = None;

        // Give paused/scrollback message widgets first chance to consume input before
        // buttons or the game map turn it into player actions.
        for widget in self.messages_widgets.iter_mut() {
            let inside = widget.rect.contains(Vec2::new(p.x as f32, p.y as f32));
            if (inside || widget.blocks_input())
                && let Some(action) = widget.touch_down(p)
            {
                return Some(action);
            }
        }
        // If we hovered over an item in 3D, send an explicit ItemClicked intent
        if let Some(entity_id) = self.hovered_entity_id {
            let intent = self.get_current_intent_for_action();
            if intent.is_some() {
                self.consume_one_shot_2d_intent();
            }
            return Some(EntityAction::EntityClicked(
                entity_id,
                self.hover_distance,
                intent,
            ));
        }

        // If we hovered over an item in 3D, send an explicit ItemClicked intent or start a drag
        if let Some(item_id) = self.hovered_item_id {
            if self.has_drag_drop_targets() {
                self.dragging_item_id = Some(item_id);
                self.dragging_item_owner_entity_id = None;
                self.dragging_item_from_world = true;
                self.drag_start_pos = self.screen_to_viewport(coord);
                return None;
            }
            let intent = self.get_current_intent_for_action();
            if intent.is_none()
                && let Some(item) = Self::find_container_item(map, item_id, None)
                && Self::item_is_container(item)
            {
                self.toggle_container_panel(
                    item_id,
                    None,
                    Rect::new(p.x as f32, p.y as f32, 1.0, 1.0),
                );
                return None;
            }
            if intent.is_some() {
                self.consume_one_shot_2d_intent();
            }
            return Some(EntityAction::ItemClicked(
                item_id,
                Self::item_click_distance(map, item_id),
                intent,
                None,
            ));
        }

        for (id, widget) in self.button_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                self.pressed_widget = Some(*id);
                self.activated_widgets.push(*id);

                if let (Some(binding), Some(value)) =
                    (widget.binding.as_deref(), widget.value.as_deref())
                {
                    bound_state_update = Some((binding.to_string(), value.to_string()));
                }

                if widget.drag_drop {
                    if let Some(entity) = Self::resolve_party_entity(map, widget.party.as_deref()) {
                        if let Some(inventory_index) = &widget.inventory_index
                            && let Some(item) = entity
                                .inventory
                                .get(*inventory_index)
                                .and_then(|item| item.as_ref())
                        {
                            self.dragging_item_id = Some(item.id);
                            self.dragging_item_owner_entity_id = Some(entity.id);
                            self.dragging_source_widget_id = Some(*id);
                            self.drag_start_pos = p;
                            return None;
                        }
                        if let Some(slot) = &widget.equipped_slot
                            && let Some(item) = entity.get_equipped_item(slot)
                        {
                            self.dragging_item_id = Some(item.id);
                            self.dragging_item_owner_entity_id = Some(entity.id);
                            self.dragging_source_widget_id = Some(*id);
                            self.drag_start_pos = p;
                            return None;
                        }
                    }
                }

                // Command buttons work in both 2D and 3D. Control commands become
                // immediate movement/camera input; intent and rules commands set the
                // active targeting state and become one-shot actions in classic 2D.
                let command_entity = Self::resolve_party_entity(map, widget.party.as_deref());
                if let Some(binding) =
                    Self::resolved_widget_binding(widget, assets, command_entity, &self.ui_state)
                {
                    match binding {
                        ClientCommandBinding::Control(act) => {
                            action = Some(act);
                        }
                        ClientCommandBinding::Intent(intent) => {
                            let payload = intent;
                            if payload.trim().is_empty() {
                                self.intent.clear();
                                selected_walk_button_id = Some(*id);
                            } else {
                                self.intent = payload.clone();
                                selected_targeting_button_id = Some(*id);
                                action = Some(EntityAction::Intent(payload));
                            }
                        }
                        ClientCommandBinding::RulesAction(rules_action) => {
                            let payload = format!("action:{}", rules_action);
                            self.intent = payload.clone();
                            selected_targeting_button_id = Some(*id);
                            action = Some(EntityAction::Intent(payload));
                        }
                        ClientCommandBinding::Screen(_) | ClientCommandBinding::Game(_) => {
                            self.pending_runtime_commands.push(binding);
                        }
                        ClientCommandBinding::Ui(_) => {}
                    }
                }

                if let Some(hide) = &widget.hide {
                    self.widgets_to_hide.clear();
                    for h in hide {
                        self.widgets_to_hide.push(h.clone());
                    }
                }
                if let Some(show) = &widget.show {
                    for s in show {
                        self.widgets_to_hide.retain(|x| x != s);
                    }
                }
                if let Some(entity) = Self::resolve_party_entity(map, widget.party.as_deref()) {
                    if let Some(inventory_index) = &widget.inventory_index
                        && let Some(item) = entity
                            .inventory
                            .get(*inventory_index)
                            .and_then(|item| item.as_ref())
                    {
                        if active_intent.is_none() && Self::item_is_container(item) {
                            self.toggle_container_panel(item.id, Some(entity.id), widget.rect);
                            return None;
                        }
                        action = Some(EntityAction::ItemClicked(
                            item.id,
                            0.0,
                            active_intent.clone(),
                            Some(entity.id),
                        ));
                    }
                    if action.is_none()
                        && let Some(slot) = &widget.equipped_slot
                        && let Some(item) = entity.get_equipped_item(slot)
                    {
                        action = Some(EntityAction::ItemClicked(
                            item.id,
                            0.0,
                            active_intent.clone(),
                            Some(entity.id),
                        ));
                    }
                }

                if let Some(camera) = &widget.camera {
                    render_camera_switches.push((widget.camera_target.clone(), camera.clone()));
                }
                if let Some(player_camera) = &widget.player_camera {
                    camera_action = Some(EntityAction::SetPlayerCamera(player_camera.clone()));
                }

                // Deactivate the widgets and activate this widget
                if !widget.deactivate.is_empty() {
                    for widget_to_deactivate in &widget.deactivate {
                        for (id, widget) in self.button_widgets.iter() {
                            if Self::deactivate_matches(widget, widget_to_deactivate) {
                                self.activated_widgets.retain(|x| x != id);
                                self.permanently_activated_widgets.retain(|x| x != id);
                            }
                        }
                    }
                    self.activated_widgets.push(widget.id);
                    self.permanently_activated_widgets.push(widget.id);
                }
            }
        }
        if let Some(button_id) = selected_targeting_button_id {
            self.activate_targeting_button(button_id);
        }
        if let Some(button_id) = selected_walk_button_id {
            self.activate_walk_button(button_id);
        }
        if let Some((binding, value)) = bound_state_update {
            self.set_ui_state(&binding, &value);
        }
        for (target, camera) in render_camera_switches {
            self.set_game_widget_camera_mode(target.as_deref(), camera);
        }

        if camera_action.is_some() {
            action = camera_action;
        }

        // Test against clicks on interactive messages (multiple choice)
        if action.is_none() {
            for widget in self.messages_widgets.iter_mut() {
                if let Some(action) = widget.touch_down(p) {
                    return Some(action);
                }
            }
        }

        // Test against clicks on the map
        if action.is_none() {
            let player_pos = Self::resolve_party_entity(map, Some("leader"))
                .or_else(|| map.entities.iter().find(|entity| entity.is_player()))
                .map(|entity| entity.get_pos_xz())
                .unwrap_or(Vec2::zero());

            for (_, widget) in self.game_widgets.iter() {
                if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    if Self::is_2d_camera(&widget.camera) {
                        let dx = p.x as f32 - widget.rect.x;
                        let dy = p.y as f32 - widget.rect.y;

                        let gx = widget.top_left.x + dx / widget.grid_size;
                        let gy = widget.top_left.y + dy / widget.grid_size;

                        let pos = Vec2::new(gx, gy);
                        let tile_pos = Self::quantize_2d_tile_pos(pos);

                        // In 2D, items often share a tile with the player or another entity.
                        // When drag-drop targets exist, prefer starting an item drag before
                        // the broader entity click path consumes the cell.
                        if self.has_drag_drop_targets()
                            && let Some(item) = map.items.iter().rev().find(|item| {
                                tile_pos == Self::quantize_2d_tile_pos(item.get_pos_xz())
                            })
                        {
                            self.dragging_item_id = Some(item.id);
                            self.dragging_item_owner_entity_id = None;
                            self.dragging_item_from_world = true;
                            self.drag_start_pos = self.screen_to_viewport(coord);
                            return None;
                        }

                        for entity in map.entities.iter() {
                            if entity.attributes.get_str_default("mode", "active".into()) == "dead"
                            {
                                continue;
                            }
                            let entity_pos = entity.get_pos_xz();
                            if tile_pos == Self::quantize_2d_tile_pos(entity_pos) {
                                let distance = player_pos.distance(entity_pos);
                                let intent = self.get_current_intent_for_action();
                                if intent.is_some() {
                                    self.consume_one_shot_2d_intent();
                                }
                                return Some(EntityAction::EntityClicked(
                                    entity.id, distance, intent,
                                ));
                            }
                        }

                        for item in map.items.iter().rev() {
                            let item_pos = item.get_pos_xz();
                            if tile_pos == Self::quantize_2d_tile_pos(item_pos) {
                                let distance = player_pos.distance(item_pos);
                                let intent = self.get_current_intent_for_action();
                                if intent.is_some() {
                                    self.consume_one_shot_2d_intent();
                                }
                                return Some(EntityAction::ItemClicked(
                                    item.id, distance, intent, None,
                                ));
                            }
                        }

                        // Try entities again but include dead ones too
                        for entity in map.entities.iter() {
                            let entity_pos = entity.get_pos_xz();
                            if tile_pos == Self::quantize_2d_tile_pos(entity_pos) {
                                let distance = player_pos.distance(entity_pos);
                                let intent = self.get_current_intent_for_action();
                                if intent.is_some() {
                                    self.consume_one_shot_2d_intent();
                                }
                                return Some(EntityAction::EntityClicked(
                                    entity.id, distance, intent,
                                ));
                            }
                        }

                        return Some(EntityAction::TerrainClicked(tile_pos));
                    }
                }
            }
        }

        if action.as_ref().is_some_and(Self::is_movement_action)
            && self.get_current_intent().is_some()
        {
            self.consume_one_shot_2d_intent();
        }

        action
    }

    /// Click / touch up event
    pub fn touch_up(&mut self, coord: Vec2<i32>, map: &Map) -> Option<EntityAction> {
        let mut action = None;
        if self.dragging_container_panel {
            self.dragging_container_panel = false;
            self.pressed_widget = None;
            return None;
        }
        let dragged_item_id = self.dragging_item_id;
        let dragged_item_owner_entity_id = self.dragging_item_owner_entity_id;
        let dragged_source_widget_id = self.dragging_source_widget_id;
        let dragged_item_from_world = self.dragging_item_from_world;
        let dragged_container_source = self.dragging_item_container_source;
        let p = self.screen_to_viewport(coord);
        let dragging_started = self.dragging_started || self.drag_distance_exceeded(p);

        if let Some(item_id) = dragged_item_id {
            if !dragging_started {
                if let Some(source) = dragged_container_source {
                    action = Some(EntityAction::MoveContainerItem {
                        item_id,
                        container_item_id: source.container_item_id,
                        container_owner_entity_id: source.container_owner_entity_id,
                        target_entity_id: None,
                        to_inventory_index: None,
                        to_equipped_slot: None,
                    });
                } else if dragged_item_from_world {
                    if let Some(item) = Self::find_container_item(map, item_id, None)
                        && Self::item_is_container(item)
                    {
                        self.toggle_container_panel(
                            item_id,
                            None,
                            Rect::new(p.x as f32, p.y as f32, 1.0, 1.0),
                        );
                        self.dragging_item_id = None;
                        self.dragging_item_owner_entity_id = None;
                        self.dragging_source_widget_id = None;
                        self.dragging_item_from_world = false;
                        self.dragging_started = false;
                        return None;
                    }
                    let intent = self.get_current_intent_for_action();
                    if intent.is_some() {
                        self.consume_one_shot_2d_intent();
                    }
                    action = Some(EntityAction::ItemClicked(
                        item_id,
                        Self::item_click_distance(map, item_id),
                        intent,
                        None,
                    ));
                } else if let Some(source_id) = dragged_source_widget_id
                    && let Some(widget) = self.button_widgets.get(&source_id)
                    && widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
                {
                    if let Some(owner_id) = dragged_item_owner_entity_id
                        && let Some(item) = Self::find_container_item(map, item_id, Some(owner_id))
                        && Self::item_is_container(item)
                    {
                        self.toggle_container_panel(item_id, Some(owner_id), widget.rect);
                        self.dragging_item_id = None;
                        self.dragging_item_owner_entity_id = None;
                        self.dragging_source_widget_id = None;
                        self.dragging_item_from_world = false;
                        self.dragging_started = false;
                        return None;
                    }
                    let intent = self.get_current_intent_for_action();
                    if intent.is_some() {
                        self.consume_one_shot_2d_intent();
                    }
                    action = Some(EntityAction::ItemClicked(
                        item_id,
                        0.0,
                        intent,
                        dragged_item_owner_entity_id,
                    ));
                }
            } else {
                if let Some(panel) = self.open_container_panel
                    && self
                        .open_container_panel_rect
                        .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
                    && item_id != panel.item_id
                    && dragged_container_source.is_none_or(|source| {
                        source.container_item_id != panel.item_id
                            || source.container_owner_entity_id != panel.owner_entity_id
                    })
                {
                    action = Some(EntityAction::MoveItemToContainer {
                        item_id,
                        owner_entity_id: dragged_item_owner_entity_id,
                        source_container_item_id: dragged_container_source
                            .map(|source| source.container_item_id),
                        source_container_owner_entity_id: dragged_container_source
                            .and_then(|source| source.container_owner_entity_id),
                        container_item_id: panel.item_id,
                        container_owner_entity_id: panel.owner_entity_id,
                    });
                }
                for (_, widget) in self.button_widgets.iter() {
                    if action.is_some() {
                        break;
                    }
                    if !widget.drag_drop || !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
                    {
                        continue;
                    }
                    let target_entity_id = Self::resolve_party_entity(map, widget.party.as_deref())
                        .map(|entity| entity.id);
                    if let Some(target_index) = widget.inventory_index {
                        action = Some(if let Some(source) = dragged_container_source {
                            EntityAction::MoveContainerItem {
                                item_id,
                                container_item_id: source.container_item_id,
                                container_owner_entity_id: source.container_owner_entity_id,
                                target_entity_id,
                                to_inventory_index: Some(target_index),
                                to_equipped_slot: None,
                            }
                        } else {
                            EntityAction::MoveItem {
                                item_id,
                                owner_entity_id: dragged_item_owner_entity_id,
                                target_entity_id,
                                to_inventory_index: Some(target_index),
                                to_equipped_slot: None,
                            }
                        });
                        break;
                    }
                    if let Some(target_slot) = &widget.equipped_slot {
                        action = Some(if let Some(source) = dragged_container_source {
                            EntityAction::MoveContainerItem {
                                item_id,
                                container_item_id: source.container_item_id,
                                container_owner_entity_id: source.container_owner_entity_id,
                                target_entity_id,
                                to_inventory_index: None,
                                to_equipped_slot: Some(target_slot.clone()),
                            }
                        } else {
                            EntityAction::MoveItem {
                                item_id,
                                owner_entity_id: dragged_item_owner_entity_id,
                                target_entity_id,
                                to_inventory_index: None,
                                to_equipped_slot: Some(target_slot.clone()),
                            }
                        });
                        break;
                    }
                }
                if action.is_none()
                    && let Some(position) = self.drop_position_at_viewport(p)
                {
                    action = Some(if let Some(source) = dragged_container_source {
                        EntityAction::DropContainerItemAt {
                            item_id,
                            container_item_id: source.container_item_id,
                            container_owner_entity_id: source.container_owner_entity_id,
                            position,
                        }
                    } else {
                        EntityAction::DropItemAt {
                            item_id,
                            owner_entity_id: dragged_item_owner_entity_id,
                            position,
                        }
                    });
                }
            }
        }
        self.clear_item_drag();
        self.pressed_widget = None;

        self.activated_widgets = self.permanently_activated_widgets.clone();

        // Reset cursor after click release. Hover logic applies intent cursors contextually.
        self.curr_cursor = self.default_cursor;

        for widget in self.messages_widgets.iter_mut() {
            widget.touch_up();
        }
        action
    }

    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        let immediate_2d_intent = self.immediate_2d_intent_mode();
        let is_escape = event == "key_down"
            && matches!(
                &value,
                Value::Str(v) if matches!(v.trim().to_ascii_lowercase().as_str(), "escape" | "esc")
            );

        // Make sure we do not send action events after a key down intent was handled
        // Otherwise the character would move a bit because "intent" is already cleared
        if event == "key_up" {
            self.key_down_intent = None;
        }

        if is_escape && self.close_floaters() {
            return EntityAction::Off;
        }

        if event == "key_down"
            && let Value::Str(v) = &value
            && self.focused_text_input_key_down(v)
        {
            return EntityAction::Off;
        }

        // --- Check for multiple choice

        if let Some(choice_map) = &self.choice_map.clone() {
            if event == "key_down" {
                if let Value::Str(v) = &value {
                    if let Some(c) = Self::choice_key_from_input(v) {
                        if let Some(choice) = choice_map.get(&c) {
                            let choice = if self.choice_expired(choice) {
                                let (from, to, expires_at_tick, max_distance) =
                                    choice.session_meta();
                                Choice::Cancel(from, to, expires_at_tick, max_distance)
                            } else {
                                choice.clone()
                            };
                            if matches!(choice, Choice::Cancel(_, _, _, _)) {
                                self.choice_map = None;
                            }
                            return EntityAction::Choice(choice);
                        }
                    }
                }
            }
        }

        for widget in self.messages_widgets.iter_mut() {
            if let Some(action) = widget.user_event(&event, &value) {
                return action;
            }
        }

        if immediate_2d_intent && event == "key_down" {
            if let Some(key_down_intent) = &self.key_down_intent
                && !key_down_intent.is_empty()
            {
                return EntityAction::Off;
            }
        }

        if immediate_2d_intent && self.key_down_intent.is_none() && event == "key_down" {
            self.key_down_intent = Some(self.intent.clone());
        }

        // ---

        let is_key_down = event == "key_down";
        let action = self.client_action.lock().unwrap().user_event(event, value);

        if is_key_down {
            if let EntityAction::Intent(intent_name) = &action {
                self.apply_intent_button_activation(intent_name);
                // The server also needs the selected intent for the next directional
                // input in classic 2D mode; the next target/move consumes it.
            }
        }

        if is_key_down
            && immediate_2d_intent
            && Self::is_movement_action(&action)
            && self
                .key_down_intent
                .as_ref()
                .is_some_and(|intent| !intent.trim().is_empty())
        {
            self.consume_one_shot_2d_intent();
        }

        let action_str: String = action.to_string();
        if action_str == "none" {
            self.activated_widgets = self.permanently_activated_widgets.clone();
        } else {
            for (id, widget) in self.button_widgets.iter_mut() {
                let command_matches = matches!(widget.command_binding(), Some(ClientCommandBinding::Control(ref control)) if control.to_string() == action_str);
                if (widget.action == action_str || command_matches)
                    && !self.activated_widgets.contains(id)
                {
                    self.activated_widgets.push(*id);
                }
            }
        }

        action
    }

    pub fn focused_text_input_key_down(&mut self, raw_key: &str) -> bool {
        let Some(input_id) = self.focused_text_input else {
            return false;
        };

        let key = raw_key.trim();
        let lower = key.to_ascii_lowercase();
        if matches!(lower.as_str(), "escape" | "esc" | "enter" | "return") {
            self.focused_text_input = None;
            return true;
        }

        let Some(widget) = self.text_input_widgets.get_mut(&input_id) else {
            self.focused_text_input = None;
            return false;
        };

        if matches!(lower.as_str(), "backspace" | "delete") || matches!(raw_key, "\u{8}" | "\u{7f}")
        {
            widget.text.pop();
        } else if lower == "space" {
            widget.text.push(' ');
        } else if raw_key.chars().count() == 1
            && !raw_key.chars().next().is_some_and(char::is_control)
        {
            widget.text.push_str(raw_key);
        }

        if !widget.binding.trim().is_empty() {
            self.ui_state
                .insert(widget.binding.clone(), widget.text.clone());
        }
        true
    }

    pub fn scroll_messages(&mut self, delta_y: isize) -> bool {
        let mut handled = false;
        for widget in self.messages_widgets.iter_mut() {
            handled |= widget.scroll(delta_y);
        }
        handled
    }

    pub fn hover_tooltip_pending(&self) -> bool {
        self.tooltip_hover_since.is_some()
    }

    /// Apply the same intent-button toggle behavior as clicking a button:
    /// deactivate configured peers and keep the selected intent button active.
    fn apply_intent_button_activation(&mut self, intent_name: &str) {
        let intent_raw = intent_name.trim();
        let mut intent_norm = intent_raw.to_ascii_lowercase();
        let mut spell_template_norm: Option<String> = None;
        if let Some((prefix, value)) = intent_raw.split_once(':')
            && prefix.trim().eq_ignore_ascii_case("spell")
        {
            let spell = value.trim();
            if !spell.is_empty() {
                intent_norm = "spell".to_string();
                spell_template_norm = Some(spell.to_ascii_lowercase());
            }
        }

        let mut selected_button_id: Option<u32> = None;
        let mut deactivate_names: Vec<String> = Vec::new();
        let mut selected_intent: Option<String> = None;
        let mut best_score: i32 = i32::MIN;

        for (id, widget) in self.button_widgets.iter() {
            let mut intent_match = widget
                .intent_payload()
                .map(|s| s.trim().eq_ignore_ascii_case(intent_raw))
                .unwrap_or(false)
                || widget
                    .intent
                    .as_ref()
                    .map(|s| s.trim().eq_ignore_ascii_case(&intent_norm))
                    .unwrap_or(false);
            if intent_match && let Some(spell_template_norm) = &spell_template_norm {
                intent_match = widget
                    .spell
                    .as_ref()
                    .map(|s| s.trim().to_ascii_lowercase() == *spell_template_norm)
                    .unwrap_or(false);
            }

            // Fallbacks for projects that encoded intent-ish data in action.
            let action_norm = widget.action.trim().to_ascii_lowercase();
            let action_match = !intent_norm.is_empty()
                && (action_norm == intent_norm
                    || action_norm == format!("intent({})", intent_norm)
                    || action_norm == format!("intent(\"{}\")", intent_norm)
                    || action_norm == format!("intent('{}')", intent_norm));

            if intent_match || action_match {
                // Prefer dedicated intent toggle buttons (e.g. UseIntent/LookIntent)
                // over inventory/equipment widgets that may also carry an intent.
                let mut score: i32 = 0;
                if intent_match {
                    score += 100;
                }
                if spell_template_norm.is_some() {
                    score += 50;
                }
                if !widget.deactivate.is_empty() {
                    score += 30;
                }
                if widget.inventory_index.is_none() && widget.equipped_slot.is_none() {
                    score += 30;
                } else {
                    score -= 40;
                }
                if widget.drag_drop {
                    score -= 20;
                }
                if widget.name.to_ascii_lowercase().ends_with("intent") {
                    score += 20;
                }
                if action_match && !intent_match {
                    score -= 10;
                }
                if score > best_score {
                    best_score = score;
                    selected_button_id = Some(*id);
                    deactivate_names = widget.deactivate.clone();
                    selected_intent = widget
                        .intent_payload()
                        .or_else(|| Some(intent_name.to_string()));
                }
            }
        }

        // Keep fallback intent state in sync with keyboard shortcuts.
        self.intent = selected_intent.unwrap_or_else(|| intent_name.to_string());

        let Some(button_id) = selected_button_id else {
            return;
        };

        if intent_raw.is_empty() {
            self.activate_walk_button(button_id);
            return;
        }

        // Deactivate all other targeting buttons so shortcut intent is authoritative.
        for (id, widget) in self.button_widgets.iter() {
            if *id != button_id && Self::is_targeting_button(widget) {
                self.activated_widgets.retain(|x| x != id);
                self.permanently_activated_widgets.retain(|x| x != id);
            }
        }

        // Also process explicit deactivate names for non-intent companion buttons.
        if !deactivate_names.is_empty() {
            for widget_to_deactivate in &deactivate_names {
                for (id, widget) in self.button_widgets.iter() {
                    if Self::deactivate_matches(widget, widget_to_deactivate) {
                        self.activated_widgets.retain(|x| x != id);
                        self.permanently_activated_widgets.retain(|x| x != id);
                    }
                }
            }
        }

        // Move selected button to the end so get_current_intent() resolves to it.
        self.activated_widgets.retain(|x| *x != button_id);
        self.permanently_activated_widgets
            .retain(|x| *x != button_id);
        self.activated_widgets.push(button_id);
        self.permanently_activated_widgets.push(button_id);

        // Sync cursors immediately to the newly selected intent button.
        if let Some(widget) = self.button_widgets.get(&button_id) {
            self.curr_intent_cursor = widget.item_cursor_id;
            self.curr_clicked_intent_cursor = widget.item_clicked_cursor_id;
            self.curr_cursor = self.default_cursor;
        }
    }

    // Init the screen
    pub fn init_screen(
        &mut self,
        screen_name: String,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.game_widgets.clear();
        self.button_widgets.clear();
        self.avatar_widgets.clear();
        self.text_widgets.clear();
        self.text_input_widgets.clear();
        self.deco_widgets.clear();
        self.messages_widgets.clear();
        self.focused_text_input = None;

        self.screen_widget = Some(ScreenWidget {
            buffer: TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y)),
            background_color: Self::hex_to_rgba_u8(&self.get_config_string_default(
                "viewport",
                "screen_background",
                &self.get_config_string_default("viewport", "background_color_2d", "#000000"),
            )),
            ..Default::default()
        });

        // Iterate sectors and apply layer property for sorted drawing
        if let Some(screen) = assets.screens.get_mut(&screen_name) {
            for sector in screen.sectors.iter_mut() {
                if let Some(crate::Value::Str(data)) = sector.properties.get("data") {
                    if let Ok(table) = data.parse::<Table>() {
                        if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                            if let Some(value) = ui.get("layer") {
                                if let Some(v) = value.as_integer() {
                                    sector.properties.set("layer", Value::Int(v as i32));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(screen) = assets.screens.get(&screen_name) {
            for widget in screen.sectors.iter() {
                let bb = widget.bounding_box(screen);

                let (start_x, start_y) = crate::utils::align_screen_to_grid(
                    self.viewport.x as f32,
                    self.viewport.y as f32,
                    self.grid_size,
                );

                let x = (bb.min.x - start_x) * self.grid_size;
                let y = (bb.min.y - start_y) * self.grid_size;
                let width = bb.size().x * self.grid_size;
                let height = bb.size().y * self.grid_size;

                let mut textures = Vec::new();
                if let Some(source) = widget.properties.get_default_source()
                    && let Some(tile) = source.tile_from_tile_list(assets)
                    && let Some(texture) = tile.textures.first()
                {
                    textures.push(texture.clone());
                }
                if let Some(source) = widget.properties.get_source("ceiling_source")
                    && let Some(tile) = source.tile_from_tile_list(assets)
                    && let Some(texture) = tile.textures.first()
                {
                    textures.push(texture.clone());
                }

                if let Some(crate::Value::Str(data)) = widget.properties.get("data") {
                    if let Ok(table) = data.parse::<Table>() {
                        let grid_size = self.grid_size;

                        let mut role = "none";
                        if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                            if let Some(value) = ui.get("role") {
                                if let Some(v) = value.as_str() {
                                    role = v;
                                }
                            }
                        }

                        if role == "game" {
                            let mut game_widget = GameWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::new(TheDim::sized(
                                    width as i32,
                                    height as i32,
                                )),
                                grid_size,
                                ..Default::default()
                            };

                            game_widget.init();
                            if let Some(map) = assets.maps.get(&self.current_map) {
                                game_widget.build(map, assets, scene_handler);
                            }
                            self.game_widgets.insert(widget.creator_id, game_widget);
                        } else if role == "button" {
                            let mut command = None;
                            let mut command_slot = None;
                            let mut action = "";
                            let mut intent = None;
                            let mut spell = None;
                            let mut group = None;
                            let mut binding = None;
                            let mut value = None;
                            let mut selection = None;
                            let mut show: Option<Vec<String>> = None;
                            let mut hide: Option<Vec<String>> = None;
                            let mut deactivate: Vec<String> = vec![];
                            let mut camera: Option<PlayerCamera> = None;
                            let mut player_camera: Option<PlayerCamera> = None;
                            let mut camera_target: Option<String> = None;
                            let mut party: Option<String> = None;
                            let mut inventory_index: Option<usize> = None;
                            let mut equipped_slot: Option<String> = None;
                            let mut portrait = false;
                            let mut drag_drop = false;

                            let mut entity_cursor_id = None;
                            let mut entity_clicked_cursor_id = None;
                            let mut item_cursor_id = None;
                            let mut item_clicked_cursor_id = None;
                            let mut border_size: i32 = 0;
                            let mut border_color: [u8; 4] = [255, 255, 255, 255];
                            let mut label = String::new();
                            let mut label_font = String::new();
                            let mut label_font_size = 18.0;
                            let mut label_color: [u8; 4] = [255, 255, 255, 255];
                            let mut background_color = None;
                            let mut hover_style = ButtonStateStyle::default();
                            let mut selected_style = ButtonStateStyle::default();
                            let mut pressed_style = ButtonStateStyle::default();
                            let mut disabled_style = ButtonStateStyle::default();

                            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                                // Check for command. This is the preferred button API.
                                if let Some(value) = ui.get("command")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        command = Some(trimmed.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("command_slot")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        command_slot = Some(trimmed.to_string());
                                    }
                                }

                                // Check for action
                                if let Some(value) = ui.get("action") {
                                    if let Some(v) = value.as_str() {
                                        action = v;
                                    }
                                }

                                // Check for intent
                                if let Some(value) = ui.get("intent") {
                                    if let Some(v) = value.as_str() {
                                        intent = Some(v.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("spell")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        spell = Some(trimmed.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("group")
                                    && let Some(v) = value.as_str()
                                {
                                    let trimmed = v.trim();
                                    if !trimmed.is_empty() {
                                        group = Some(trimmed.to_string());
                                    }
                                }
                                if let Some(v) = ui
                                    .get("bind")
                                    .or_else(|| ui.get("binding"))
                                    .and_then(toml::Value::as_str)
                                    .map(str::trim)
                                    .filter(|value| !value.is_empty())
                                {
                                    binding = Some(v.to_string());
                                }
                                if let Some(v) = ui.get("value").and_then(toml::Value::as_str) {
                                    value = Some(v.to_string());
                                }
                                if let Some(v) = ui
                                    .get("selection")
                                    .and_then(toml::Value::as_str)
                                    .map(str::trim)
                                    .filter(|value| !value.is_empty())
                                {
                                    selection = Some(v.to_string());
                                }

                                if let Some(value) = ui.get("label").or_else(|| ui.get("text"))
                                    && let Some(v) = value.as_str()
                                {
                                    label = v.to_string();
                                }
                                if let Some(value) = ui.get("font")
                                    && let Some(v) = value.as_str()
                                {
                                    label_font = v.to_string();
                                }
                                if let Some(value) = ui.get("font_size")
                                    && let Some(v) = value.as_float()
                                {
                                    label_font_size = v as f32;
                                }
                                if let Some(value) = ui.get("color")
                                    && let Some(v) = value.as_str()
                                {
                                    label_color = Self::hex_to_rgba_u8(v);
                                }
                                if let Some(color) = Self::ui_style_color(ui, None, "text", "color")
                                    .or_else(|| Self::ui_style_color(ui, None, "color", "color"))
                                {
                                    label_color = color;
                                }
                                background_color = Self::ui_style_color(
                                    ui,
                                    None,
                                    "background",
                                    "background_color",
                                );
                                hover_style = Self::button_state_style_from_ui(
                                    ui,
                                    "hover",
                                    "hover_background_color",
                                    "hover_border_color",
                                    "hover_color",
                                );
                                selected_style = Self::button_state_style_from_ui(
                                    ui,
                                    "selected",
                                    "selected_background_color",
                                    "selected_border_color",
                                    "selected_color",
                                );
                                pressed_style = Self::button_state_style_from_ui(
                                    ui,
                                    "pressed",
                                    "pressed_background_color",
                                    "pressed_border_color",
                                    "pressed_color",
                                );
                                disabled_style = Self::button_state_style_from_ui(
                                    ui,
                                    "disabled",
                                    "disabled_background_color",
                                    "disabled_border_color",
                                    "disabled_color",
                                );

                                // Check for show
                                if let Some(value) = ui.get("show") {
                                    if let Some(va) = value.as_array() {
                                        let mut c = vec![];
                                        for v in va {
                                            if let Some(v) = v.as_str() {
                                                c.push(v.to_string());
                                            }
                                        }
                                        if !c.is_empty() {
                                            show = Some(c);
                                        }
                                    }
                                }

                                // Check for hide
                                if let Some(value) = ui.get("hide") {
                                    if let Some(va) = value.as_array() {
                                        let mut c = vec![];
                                        for v in va {
                                            if let Some(v) = v.as_str() {
                                                c.push(v.to_string());
                                            }
                                        }
                                        if !c.is_empty() {
                                            hide = Some(c);
                                        }
                                    }
                                }

                                // Check for deactivate
                                if let Some(value) = ui.get("deactivate") {
                                    if let Some(va) = value.as_array() {
                                        let mut c = vec![];
                                        for v in va {
                                            if let Some(v) = v.as_str() {
                                                c.push(v.to_string());
                                            }
                                        }
                                        deactivate = c;
                                    }
                                }

                                // Check camera mode switch for game widget rendering
                                if let Some(value) = ui.get("camera")
                                    && let Some(v) = value.as_str()
                                {
                                    camera = Self::parse_player_camera_mode(v);
                                }

                                // Check player camera mapping switch for server controls.
                                if let Some(value) = ui.get("player_camera")
                                    && let Some(v) = value.as_str()
                                {
                                    player_camera = Self::parse_player_camera_mode(v);
                                }

                                // Optional game widget name target (defaults to all game widgets)
                                if let Some(value) = ui.get("camera_target")
                                    && let Some(v) = value.as_str()
                                    && !v.is_empty()
                                {
                                    camera_target = Some(v.to_string());
                                }

                                if let Some(value) = ui.get("party").and_then(toml::Value::as_str) {
                                    let binding = value.trim();
                                    if !binding.is_empty() {
                                        party = Some(binding.to_string());
                                    }
                                }

                                // Check for active
                                if let Some(active_value) = ui.get("active") {
                                    if let Some(v) = active_value.as_bool()
                                        && v
                                    {
                                        self.activated_widgets.push(widget.id);
                                        self.permanently_activated_widgets.push(widget.id);
                                        if let (Some(binding), Some(bound_value)) =
                                            (binding.as_deref(), value.as_deref())
                                            && !binding.trim().is_empty()
                                        {
                                            self.ui_state.insert(
                                                binding.to_string(),
                                                bound_value.to_string(),
                                            );
                                        }
                                        if let Some(hide) = &hide {
                                            self.widgets_to_hide = hide.clone();
                                        }
                                    }
                                }

                                // Check for inventory
                                if let Some(value) = ui.get("inventory_index") {
                                    if let Some(v) = value.as_integer() {
                                        inventory_index = Some(v as usize);
                                    }
                                }
                                if let Some(value) = ui.get("equipped_slot")
                                    && let Some(v) = value.as_str()
                                {
                                    let slot = v.trim();
                                    if !slot.is_empty() {
                                        equipped_slot = Some(slot.to_string());
                                    }
                                }
                                if let Some(value) = ui.get("drag_drop")
                                    && let Some(v) = value.as_bool()
                                {
                                    drag_drop = v;
                                }
                                if let Some(value) = ui.get("portrait")
                                    && let Some(v) = value.as_bool()
                                {
                                    portrait = v;
                                }

                                if inventory_index.is_some() || equipped_slot.is_some() {
                                    drag_drop = ui
                                        .get("drag_drop")
                                        .and_then(toml::Value::as_bool)
                                        .unwrap_or(true);
                                }

                                // Check for the entity / item cursor ids
                                entity_cursor_id = Self::get_uuid(ui, "entity_cursor_id");
                                entity_clicked_cursor_id =
                                    Self::get_uuid(ui, "entity_clicked_cursor_id");
                                item_cursor_id = Self::get_uuid(ui, "item_cursor_id");
                                item_clicked_cursor_id =
                                    Self::get_uuid(ui, "item_clicked_cursor_id");

                                // Check for border
                                if let Some(value) = ui.get("border_size") {
                                    if let Some(v) = value.as_integer() {
                                        border_size = v as i32;
                                    }
                                }
                                if let Some(value) = ui.get("border_color") {
                                    if let Some(v) = value.as_str() {
                                        border_color = Self::hex_to_rgba_u8(v);
                                    }
                                }
                                if let Some(color) =
                                    Self::ui_style_color(ui, None, "border", "border_color")
                                {
                                    border_color = color;
                                }

                                command = command_from_legacy_fields(
                                    command.as_deref(),
                                    (!action.trim().is_empty()).then_some(action),
                                    intent.as_deref(),
                                    spell.as_deref(),
                                );
                            }

                            let button_widget = Widget {
                                name: widget.name.clone(),
                                id: widget.id,
                                rect: Rect::new(x, y, width, height),
                                action: action.into(),
                                command,
                                command_slot,
                                intent,
                                spell,
                                group,
                                binding,
                                value,
                                selection,
                                show,
                                hide,
                                deactivate,
                                camera,
                                player_camera,
                                camera_target,
                                party,
                                inventory_index,
                                equipped_slot,
                                portrait,
                                drag_drop,
                                textures,
                                entity_cursor_id,
                                entity_clicked_cursor_id,
                                item_cursor_id,
                                item_clicked_cursor_id,
                                border_color,
                                border_size,
                                label,
                                label_font,
                                label_font_size,
                                label_color,
                                background_color,
                                hover_style,
                                selected_style,
                                pressed_style,
                                disabled_style,
                            };

                            self.button_widgets.insert(widget.id, button_widget);
                        } else if role == "input" {
                            let mut binding = widget.name.clone();
                            let mut text = String::new();
                            let mut font = String::new();
                            let mut font_size = 22.0;
                            let mut color: [u8; 4] = [242, 242, 242, 255];
                            let mut background_color: [u8; 4] = [17, 17, 17, 204];
                            let mut border_color: [u8; 4] = [136, 136, 136, 255];
                            let mut border_size: i32 = 1;

                            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                                if let Some(value) = ui
                                    .get("bind")
                                    .or_else(|| ui.get("binding"))
                                    .and_then(toml::Value::as_str)
                                {
                                    let trimmed = value.trim();
                                    if !trimmed.is_empty() {
                                        binding = trimmed.to_string();
                                    }
                                }
                                if let Some(value) = ui
                                    .get("text")
                                    .or_else(|| ui.get("default"))
                                    .and_then(toml::Value::as_str)
                                {
                                    text = value.to_string();
                                }
                                if let Some(value) = ui.get("font").and_then(toml::Value::as_str) {
                                    font = value.to_string();
                                }
                                if let Some(value) =
                                    ui.get("font_size").and_then(toml::Value::as_float)
                                {
                                    font_size = value as f32;
                                }
                                if let Some(value) = ui.get("color").and_then(toml::Value::as_str) {
                                    color = Self::hex_to_rgba_u8(value);
                                }
                                if let Some(value) =
                                    ui.get("background_color").and_then(toml::Value::as_str)
                                {
                                    background_color = Self::hex_to_rgba_u8(value);
                                }
                                if let Some(value) =
                                    ui.get("border_color").and_then(toml::Value::as_str)
                                {
                                    border_color = Self::hex_to_rgba_u8(value);
                                }
                                if let Some(value) =
                                    ui.get("border_size").and_then(toml::Value::as_integer)
                                {
                                    border_size = value as i32;
                                }
                            }

                            if !binding.trim().is_empty() {
                                self.ui_state.insert(binding.clone(), text.clone());
                            }

                            self.text_input_widgets.insert(
                                widget.id,
                                TextInputWidget {
                                    name: widget.name.clone(),
                                    id: widget.id,
                                    rect: Rect::new(x, y, width, height),
                                    binding,
                                    text,
                                    font,
                                    font_size,
                                    color,
                                    background_color,
                                    border_color,
                                    border_size,
                                },
                            );
                        } else if role == "messages" {
                            let mut widget = MessagesWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::new(TheDim::sized(
                                    width as i32,
                                    height as i32,
                                )),
                                ..Default::default()
                            };
                            widget.init(assets);
                            self.messages_widgets.push(widget);
                        } else if role == "avatar" {
                            let mut avatar_widget = AvatarWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::new(TheDim::sized(
                                    width as i32,
                                    height as i32,
                                )),
                                ..Default::default()
                            };
                            avatar_widget.init();
                            self.avatar_widgets.insert(widget.creator_id, avatar_widget);
                        } else if role == "text" {
                            let mut text_widget = TextWidget {
                                name: widget.name.clone(),
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::new(TheDim::sized(
                                    width as i32,
                                    height as i32,
                                )),
                                ..Default::default()
                            };
                            text_widget.init(assets);
                            self.text_widgets.insert(widget.creator_id, text_widget);
                        } else if role == "deco" {
                            let mut deco_widget = DecoWidget {
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::new(TheDim::sized(
                                    width as i32,
                                    height as i32,
                                )),
                                ..Default::default()
                            };
                            deco_widget.init(assets);
                            self.deco_widgets.insert(widget.creator_id, deco_widget);
                        }
                    }
                }
            }
        }
        self.apply_bound_button_activations();
    }

    /// Returns true if the game camera is 2D
    fn game_widget_is_2d(&self) -> bool {
        for (_, w) in &self.game_widgets {
            if Self::is_2d_camera(&w.camera) {
                return true;
            }
        }
        false
    }

    /// Returns the intent of the currently activated button
    fn get_current_intent(&self) -> Option<String> {
        // Newer activations should win, and non-intent buttons (e.g. camera toggles)
        // must not mask an existing intent.
        for button_id in self.activated_widgets.iter().rev() {
            if let Some(widget) = self.button_widgets.get(button_id) {
                if let Some(intent) = widget.intent_payload()
                    && !intent.is_empty()
                {
                    return Some(intent);
                }
            }
        }
        if self.intent.is_empty() {
            None
        } else {
            Some(self.intent.clone())
        }
    }

    /// Returns the current intent payload for server actions.
    /// Spell intent buttons encode their selected template as `spell:<template>`.
    fn get_current_intent_for_action(&self) -> Option<String> {
        for button_id in self.activated_widgets.iter().rev() {
            if let Some(widget) = self.button_widgets.get(button_id)
                && let Some(intent) = widget.intent_payload()
            {
                if intent.is_empty() {
                    continue;
                }
                return Some(intent);
            }
        }
        if self.intent.is_empty() {
            None
        } else {
            Some(self.intent.clone())
        }
    }

    fn current_target_entity_id(map: &Map) -> Option<u32> {
        let leader = Self::resolve_party_entity(map, Some("leader"))?;

        let parse_target_attr = |value: Option<&Value>| -> Option<u32> {
            match value {
                Some(Value::UInt(id)) => Some(*id),
                Some(Value::Int(id)) if *id > 0 => Some(*id as u32),
                Some(Value::Int64(id)) if *id > 0 => Some(*id as u32),
                Some(Value::Str(value)) => value.trim().parse::<u32>().ok().filter(|id| *id > 0),
                _ => None,
            }
        };

        parse_target_attr(leader.attributes.get("attack_target"))
            .or_else(|| parse_target_attr(leader.attributes.get("target")))
    }

    fn draw_current_target_rect(&mut self, map: &Map) {
        let color_hex = self.get_config_string_default("viewport", "target_rect_color", "");
        if color_hex.trim().is_empty() {
            return;
        }

        let Some(target_id) = Self::current_target_entity_id(map) else {
            return;
        };
        let Some(target) = map.entities.iter().find(|entity| entity.id == target_id) else {
            return;
        };

        let color = Self::hex_to_rgba_u8(&color_hex);
        let stride = self.target.stride();

        for widget in self.game_widgets.values() {
            if !Self::is_2d_camera(&widget.camera) {
                continue;
            }

            let x = widget.rect.x + (target.get_pos_xz().x - widget.top_left.x) * widget.grid_size;
            let y = widget.rect.y + (target.get_pos_xz().y - widget.top_left.y) * widget.grid_size;
            let size = widget.grid_size.max(1.0).round() as usize;

            let rx = x.floor() as isize;
            let ry = y.floor() as isize;
            let rw = size as isize;
            let rh = size as isize;

            let safe = (
                0isize,
                0isize,
                self.target.dim().width as isize,
                self.target.dim().height as isize,
            );
            if rx + rw <= safe.0
                || ry + rh <= safe.1
                || rx >= safe.2
                || ry >= safe.3
                || rw <= 0
                || rh <= 0
            {
                continue;
            }

            let rect = (
                rx.max(0) as usize,
                ry.max(0) as usize,
                size.min((safe.2 - rx.max(0)) as usize),
                size.min((safe.3 - ry.max(0)) as usize),
            );
            if rect.2 == 0 || rect.3 == 0 {
                continue;
            }

            self.draw2d
                .rect_outline_thickness(self.target.pixels_mut(), &rect, stride, &color, 2);
        }
    }

    fn draw_command_state_overlay(
        target: &mut TheRGBABuffer,
        draw2d: &Draw2D,
        rect: Rect,
        state: &CommandState,
    ) {
        let stride = target.stride();
        let safe = (
            0_isize,
            0_isize,
            target.dim().width as isize,
            target.dim().height as isize,
        );
        let r = (
            rect.x.round() as isize,
            rect.y.round() as isize,
            rect.width.round().max(1.0) as isize,
            rect.height.round().max(1.0) as isize,
        );
        let alpha = if state.cooldown_remaining > 0.0 {
            145
        } else {
            175
        };
        draw2d.blend_rect_safe(target.pixels_mut(), &r, stride, &[0, 0, 0, alpha], &safe);
    }

    fn draw_hovered_world_item_pile(&mut self, map: &Map) {
        let Some(item_id) = self.hovered_item_id else {
            return;
        };
        if self.dragging_item_id.is_some() || self.dragging_container_panel {
            return;
        }
        let Some(item) = map.items.iter().find(|item| item.id == item_id) else {
            return;
        };
        let tile_pos = Self::quantize_2d_tile_pos(item.get_pos_xz());
        let pile_count = map
            .items
            .iter()
            .filter(|item| Self::quantize_2d_tile_pos(item.get_pos_xz()) == tile_pos)
            .count();

        let point = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);
        let Some(tile_rect) = self.game_widgets.values().find_map(|widget| {
            if !Self::is_2d_camera(&widget.camera) || !widget.rect.contains(point) {
                return None;
            }
            Some(Rect::new(
                widget.rect.x + (tile_pos.x - widget.top_left.x) * widget.grid_size,
                widget.rect.y + (tile_pos.y - widget.top_left.y) * widget.grid_size,
                widget.grid_size,
                widget.grid_size,
            ))
        }) else {
            return;
        };

        Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, tile_rect);

        if pile_count <= 1 {
            return;
        }
        let badge_size = 18.0_f32.min(tile_rect.width.max(12.0));
        let badge_rect = Rect::new(
            tile_rect.x + tile_rect.width - badge_size,
            tile_rect.y,
            badge_size,
            badge_size,
        );
        let stride = self.target.stride();
        let safe = (
            0_isize,
            0_isize,
            self.target.dim().width as isize,
            self.target.dim().height as isize,
        );
        let rect = (
            badge_rect.x.round() as isize,
            badge_rect.y.round() as isize,
            badge_rect.width.round().max(1.0) as isize,
            badge_rect.height.round().max(1.0) as isize,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &rect,
            stride,
            &[20, 24, 30, 220],
            &safe,
        );
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(
                badge_rect.x.round().max(0.0) as usize,
                badge_rect.y.round().max(0.0) as usize,
                badge_rect.width.round().max(1.0) as usize,
                badge_rect.height.round().max(1.0) as usize,
            ),
            stride,
            &[255, 236, 132, 255],
            1,
        );
        if let Some(font) = self.messages_font.as_ref() {
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &rect,
                stride,
                font,
                self.messages_font_size.clamp(10.0, 13.0),
                &pile_count.to_string(),
                &[245, 238, 220, 255],
                draw2d::TheHorizontalAlign::Center,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }
    }

    fn draw_drag_drop_highlights(&mut self, map: &Map) {
        if !self.dragging_started || self.dragging_item_id.is_none() {
            return;
        }
        let Some(item) = self.find_dragged_item(map).cloned() else {
            return;
        };
        let point = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);

        if let Some(panel) = self.open_container_panel
            && let Some(container) = self.open_container_item(map)
            && self
                .open_container_panel_rect
                .is_some_and(|rect| rect.contains(point))
            && item.id != panel.item_id
            && self.dragging_item_container_source.is_none_or(|source| {
                source.container_item_id != panel.item_id
                    || source.container_owner_entity_id != panel.owner_entity_id
            })
            && Self::item_can_enter_container(&item, container)
        {
            if let Some(slot) = self
                .open_container_slot_rects
                .iter()
                .copied()
                .find(|slot| slot.contains(point))
            {
                Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, slot);
                return;
            }
        }

        for widget in self.button_widgets.values() {
            if !widget.drag_drop || !widget.rect.contains(point) {
                continue;
            }
            if widget.inventory_index.is_some() {
                Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, widget.rect);
                return;
            }
            if let Some(target_slot) = &widget.equipped_slot {
                let item_slot = item
                    .attributes
                    .get_str("slot")
                    .map(|slot| slot.trim().to_ascii_lowercase());
                if item_slot.as_deref() == Some(target_slot.trim().to_ascii_lowercase().as_str()) {
                    Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, widget.rect);
                    return;
                }
            }
        }
    }

    fn draw_drag_target_highlight(target: &mut TheRGBABuffer, draw2d: &Draw2D, rect: Rect) {
        let stride = target.stride();
        let safe = (
            0_isize,
            0_isize,
            target.dim().width as isize,
            target.dim().height as isize,
        );
        let fill = (
            rect.x.round() as isize,
            rect.y.round() as isize,
            rect.width.round().max(1.0) as isize,
            rect.height.round().max(1.0) as isize,
        );
        draw2d.blend_rect_safe(
            target.pixels_mut(),
            &fill,
            stride,
            &[238, 210, 96, 70],
            &safe,
        );
        draw2d.rect_outline_thickness(
            target.pixels_mut(),
            &(
                rect.x.round().max(0.0) as usize,
                rect.y.round().max(0.0) as usize,
                rect.width.round().max(1.0) as usize,
                rect.height.round().max(1.0) as usize,
            ),
            stride,
            &[255, 236, 132, 255],
            2,
        );
    }

    fn draw_open_container_panel(&mut self, map: &Map, assets: &Assets) {
        if self.open_container_panel.is_none() {
            return;
        }
        let Some(item) = self.open_container_item(map).cloned() else {
            self.close_floaters();
            return;
        };
        let Some(layout) = self.container_panel_layout(map, assets) else {
            self.close_floaters();
            return;
        };
        if let Some(panel) = self.open_container_panel.as_mut() {
            panel.position = Vec2::new(layout.rect.x.round() as i32, layout.rect.y.round() as i32);
        }
        self.open_container_panel_rect = Some(layout.rect);
        self.open_container_slot_rects = layout.slots.clone();
        self.open_container_title_rect = layout.title_bar_rect;
        self.open_container_close_rect = layout.close_rect;
        let template = rules_ui::container_template_for_item(assets, &item);
        let stride = self.target.stride();
        let target_dim = self.target.dim();
        let safe = (
            0_isize,
            0_isize,
            target_dim.width as isize,
            target_dim.height as isize,
        );
        let panel_rect = (
            layout.rect.x.round() as isize,
            layout.rect.y.round() as isize,
            layout.rect.width.round().max(1.0) as isize,
            layout.rect.height.round().max(1.0) as isize,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &panel_rect,
            stride,
            &template.background_color,
            &safe,
        );
        self.draw_container_template_tiles(&layout, &template, assets);
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round() as usize,
                layout.rect.y.round() as usize,
                layout.rect.width.round().max(1.0) as usize,
                layout.rect.height.round().max(1.0) as usize,
            ),
            stride,
            &template.border_color,
            1,
        );

        if let Some(title_bar) = layout.title_bar_rect {
            let rect = (
                title_bar.x.round() as isize,
                title_bar.y.round() as isize,
                title_bar.width.round().max(1.0) as isize,
                title_bar.height.round().max(1.0) as isize,
            );
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &rect,
                stride,
                &[20, 24, 30, 220],
                &safe,
            );
        }

        if let Some(title_rect) = layout.title_rect
            && let Some(font) = self
                .messages_font
                .as_ref()
                .or_else(|| assets.fonts.values().next())
        {
            let title = item
                .attributes
                .get_str("name")
                .map(str::to_string)
                .unwrap_or_else(|| "Container".to_string());
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &title_rect,
                stride,
                font,
                self.messages_font_size.clamp(12.0, 16.0),
                &title,
                &[236, 233, 214, 255],
                draw2d::TheHorizontalAlign::Left,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }
        if let Some(close_rect) = layout.close_rect {
            let close_hovered = close_rect.contains(Vec2::new(
                self.cursor_pos.x as f32,
                self.cursor_pos.y as f32,
            ));
            let close_background = if close_hovered {
                [70, 78, 88, 245]
            } else {
                [42, 47, 54, 230]
            };
            let close_border = if close_hovered {
                [174, 179, 183, 255]
            } else {
                [98, 105, 116, 255]
            };
            let close_color = if close_hovered {
                [245, 238, 220, 255]
            } else {
                [220, 220, 210, 255]
            };
            let rect = (
                close_rect.x.round() as isize,
                close_rect.y.round() as isize,
                close_rect.width.round().max(1.0) as isize,
                close_rect.height.round().max(1.0) as isize,
            );
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &rect,
                stride,
                &close_background,
                &safe,
            );
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    close_rect.x.round() as usize,
                    close_rect.y.round() as usize,
                    close_rect.width.round().max(1.0) as usize,
                    close_rect.height.round().max(1.0) as usize,
                ),
                stride,
                &close_border,
                1,
            );
            Self::draw_close_x(&self.draw2d, &mut self.target, close_rect, &close_color);
        }

        for (index, slot_rect) in layout.slots.iter().enumerate() {
            let rect = (
                slot_rect.x.round() as isize,
                slot_rect.y.round() as isize,
                slot_rect.width.round().max(1.0) as isize,
                slot_rect.height.round().max(1.0) as isize,
            );
            if !template
                .tiles
                .slot
                .as_deref()
                .is_some_and(|tile| self.draw_tile_reference(assets, tile, *slot_rect))
            {
                self.draw2d.blend_rect_safe(
                    self.target.pixels_mut(),
                    &rect,
                    stride,
                    &template.slot_color,
                    &safe,
                );
            }
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    slot_rect.x.round() as usize,
                    slot_rect.y.round() as usize,
                    slot_rect.width.round().max(1.0) as usize,
                    slot_rect.height.round().max(1.0) as usize,
                ),
                stride,
                &template.slot_border_color,
                1,
            );
            if let Some(container_item) = item.container.as_ref().and_then(|items| items.get(index))
            {
                Widget::draw_item_icon(
                    &mut self.target,
                    *slot_rect,
                    assets,
                    container_item,
                    &self.draw2d,
                    self.animation_frame,
                );
            }
        }
    }

    fn draw_container_template_tiles(
        &mut self,
        layout: &ContainerPanelLayout,
        template: &ContainerUiTemplate,
        assets: &Assets,
    ) {
        let rect = layout.rect;
        let edge = template
            .slot_size
            .min((rect.width as i32 / 3).max(1))
            .min((rect.height as i32 / 3).max(1))
            .max(8) as f32;

        if let Some(tile) = template.tiles.center.as_deref() {
            self.draw_tile_reference(assets, tile, rect);
        }
        if let Some(tile) = template.tiles.top.as_deref() {
            self.draw_tile_reference(
                assets,
                tile,
                Rect::new(
                    rect.x + edge,
                    rect.y,
                    (rect.width - edge * 2.0).max(1.0),
                    edge,
                ),
            );
        }
        if let Some(tile) = template.tiles.bottom.as_deref() {
            self.draw_tile_reference(
                assets,
                tile,
                Rect::new(
                    rect.x + edge,
                    rect.y + rect.height - edge,
                    (rect.width - edge * 2.0).max(1.0),
                    edge,
                ),
            );
        }
        if let Some(tile) = template.tiles.left.as_deref() {
            self.draw_tile_reference(
                assets,
                tile,
                Rect::new(
                    rect.x,
                    rect.y + edge,
                    edge,
                    (rect.height - edge * 2.0).max(1.0),
                ),
            );
        }
        if let Some(tile) = template.tiles.right.as_deref() {
            self.draw_tile_reference(
                assets,
                tile,
                Rect::new(
                    rect.x + rect.width - edge,
                    rect.y + edge,
                    edge,
                    (rect.height - edge * 2.0).max(1.0),
                ),
            );
        }
        for (tile, tile_rect) in [
            (
                template.tiles.top_left.as_deref(),
                Rect::new(rect.x, rect.y, edge, edge),
            ),
            (
                template.tiles.top_right.as_deref(),
                Rect::new(rect.x + rect.width - edge, rect.y, edge, edge),
            ),
            (
                template.tiles.bottom_left.as_deref(),
                Rect::new(rect.x, rect.y + rect.height - edge, edge, edge),
            ),
            (
                template.tiles.bottom_right.as_deref(),
                Rect::new(
                    rect.x + rect.width - edge,
                    rect.y + rect.height - edge,
                    edge,
                    edge,
                ),
            ),
        ] {
            if let Some(tile) = tile {
                self.draw_tile_reference(assets, tile, tile_rect);
            }
        }
    }

    fn draw_tile_reference(&mut self, assets: &Assets, tile_ref: &str, rect: Rect) -> bool {
        let Some(tile) = Self::resolve_tile_reference(assets, tile_ref) else {
            return false;
        };
        let Some(texture) = tile
            .textures
            .get(self.animation_frame % tile.textures.len().max(1))
        else {
            return false;
        };
        let stride = self.target.stride();
        self.draw2d.blend_scale_chunk(
            self.target.pixels_mut(),
            &(
                rect.x.round().max(0.0) as usize,
                rect.y.round().max(0.0) as usize,
                rect.width.round().max(1.0) as usize,
                rect.height.round().max(1.0) as usize,
            ),
            stride,
            &texture.data,
            &(texture.width, texture.height),
        );
        true
    }

    fn draw_close_x(draw2d: &Draw2D, target: &mut TheRGBABuffer, rect: Rect, color: &Pixel) {
        let stride = target.stride();
        let safe = (
            0_isize,
            0_isize,
            target.dim().width as isize,
            target.dim().height as isize,
        );
        let left = rect.x.round() as i32 + 6;
        let top = rect.y.round() as i32 + 5;
        let size = (rect.width.min(rect.height).round() as i32 - 10).max(6);
        for step in 0..size {
            for (x, y) in [
                (left + step, top + step),
                (left + size - 1 - step, top + step),
            ] {
                draw2d.blend_rect_safe(
                    target.pixels_mut(),
                    &(x as isize, y as isize, 2, 2),
                    stride,
                    color,
                    &safe,
                );
            }
        }
    }

    fn resolve_tile_reference<'a>(assets: &'a Assets, tile_ref: &str) -> Option<&'a crate::Tile> {
        let trimmed = tile_ref.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Ok(id) = Uuid::parse_str(trimmed) {
            return assets.tiles.get(&id);
        }
        let needle = trimmed.to_ascii_lowercase();
        assets.tiles.values().find(|tile| {
            tile.alias
                .split([',', ';'])
                .map(str::trim)
                .filter(|alias| !alias.is_empty())
                .any(|alias| alias.eq_ignore_ascii_case(&needle))
                || tile.alias.eq_ignore_ascii_case(&needle)
        })
    }

    fn draw_hover_tooltip(&mut self, map: &Map, assets: &Assets) {
        if self.dragging_started || self.dragging_item_id.is_some() || self.dragging_container_panel
        {
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return;
        }

        let Some((description, anchor, state, hover_key, delay, prefer_below)) =
            self.hover_description(map, assets)
        else {
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return;
        };
        if description.title.trim().is_empty() {
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return;
        }

        let now = Instant::now();
        if self.tooltip_hover_key.as_deref() != Some(hover_key.as_str()) {
            self.tooltip_hover_key = Some(hover_key);
            self.tooltip_hover_since = Some(now);
            if delay > Duration::ZERO {
                return;
            }
        } else if delay > Duration::ZERO
            && self
                .tooltip_hover_since
                .map(|since| now.saturating_duration_since(since) < delay)
                .unwrap_or(true)
        {
            return;
        }

        let Some(font) = self
            .messages_font
            .as_ref()
            .or_else(|| assets.fonts.values().next())
        else {
            return;
        };

        let font_size = self.messages_font_size.clamp(12.0, 18.0);
        let mut raw_lines: Vec<(String, usize)> = Vec::new();
        raw_lines.push((description.title, 0));
        if let Some(subtitle) = description.subtitle
            && !subtitle.trim().is_empty()
        {
            raw_lines.push((subtitle, 1));
        }

        for line in description.lines.into_iter().take(7) {
            let role = if line.contains(':') { 4 } else { 2 };
            raw_lines.push((line, role));
        }
        if let Some(state) = state
            && !state.enabled
            && let Some(reason) = state.disabled_reason
        {
            raw_lines.push((reason, 3));
        }

        let min_text_width = 72_i32;
        let max_text_width = 244_i32;
        let mut desired_text_width = min_text_width;
        for (line, _) in &raw_lines {
            for paragraph in line.split('\n') {
                let paragraph = paragraph.trim();
                if paragraph.is_empty() {
                    continue;
                }
                let measured_width = self.draw2d.get_text_size(font, font_size, paragraph).0 as i32;
                desired_text_width = desired_text_width.max(measured_width + 12);
            }
        }
        let tooltip_text_width = desired_text_width.clamp(min_text_width, max_text_width);

        let mut lines: Vec<(String, usize)> = Vec::new();
        for (line, role) in raw_lines {
            for wrapped in Self::wrap_tooltip_line(
                &self.draw2d,
                font,
                font_size,
                &line,
                tooltip_text_width as f32,
            ) {
                lines.push((wrapped, role));
            }
        }

        let padding = 7_i32;
        let line_h = (font_size + 3.0).ceil() as i32;
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut cursor_y = padding;
        for index in 0..lines.len() {
            if index > 0 {
                let previous_role = lines[index - 1].1;
                let role = lines[index].1;
                if previous_role != role {
                    cursor_y += match role {
                        1 => 2,
                        2 => 5,
                        3 => 6,
                        4 => 6,
                        _ => 3,
                    };
                }
            }
            line_offsets.push(cursor_y);
            cursor_y += line_h;
        }
        let text_w = tooltip_text_width;
        let width = text_w + padding * 2;
        let height = cursor_y + padding;

        let target_dim = self.target.dim();
        let (mut x, mut y) = if prefer_below {
            (
                (anchor.x + (anchor.width - width as f32) * 0.5).round() as i32,
                (anchor.y + anchor.height + 8.0).round() as i32,
            )
        } else {
            let right_x = (anchor.x + anchor.width + 8.0).round() as i32;
            let left_x = (anchor.x.round() as i32 - width - 8).max(2);
            let x = if right_x + width <= target_dim.width {
                right_x
            } else {
                left_x
            };
            (x, (anchor.y + 2.0).round() as i32)
        };
        if x + width > target_dim.width {
            x = (target_dim.width - width - 2).max(2);
        }
        x = x.max(2);
        if y + height > target_dim.height {
            y = if prefer_below {
                (anchor.y.round() as i32 - height - 8).max(2)
            } else {
                (target_dim.height - height - 2).max(2)
            };
        }
        y = y.max(2);

        let stride = self.target.stride();
        let safe = (
            0_isize,
            0_isize,
            target_dim.width as isize,
            target_dim.height as isize,
        );
        let rect = (x as isize, y as isize, width as isize, height as isize);
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &rect,
            stride,
            &[10, 12, 15, 230],
            &safe,
        );
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(x as usize, y as usize, width as usize, height as usize),
            stride,
            &[98, 105, 116, 255],
            1,
        );

        for (index, (line, role)) in lines.iter().enumerate() {
            let color = match role {
                0 => [236, 233, 214, 255],
                1 => [174, 179, 183, 255],
                3 => [218, 184, 129, 255],
                _ => [207, 211, 214, 255],
            };
            let text_rect = (
                (x + padding) as isize,
                (y + line_offsets[index]) as isize,
                text_w as isize,
                line_h as isize,
            );
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &text_rect,
                stride,
                font,
                font_size,
                line,
                &color,
                draw2d::TheHorizontalAlign::Left,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }
    }

    fn wrap_tooltip_line(
        draw2d: &Draw2D,
        font: &Font,
        font_size: f32,
        text: &str,
        max_width: f32,
    ) -> Vec<String> {
        let max_width = max_width.max(font_size);
        let mut lines = Vec::new();

        for paragraph in text.split('\n') {
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

                if draw2d.get_text_size(font, font_size, &candidate).0 as f32 <= max_width {
                    current = candidate;
                    continue;
                }

                if !current.is_empty() {
                    lines.push(current);
                }

                if draw2d.get_text_size(font, font_size, word).0 as f32 <= max_width {
                    current = word.to_string();
                    continue;
                }

                let mut chunk = String::new();
                for ch in word.chars() {
                    let candidate = format!("{}{}", chunk, ch);
                    if !chunk.is_empty()
                        && draw2d.get_text_size(font, font_size, &candidate).0 as f32 > max_width
                    {
                        lines.push(chunk);
                        chunk = ch.to_string();
                    } else {
                        chunk = candidate;
                    }
                }
                current = chunk;
            }

            if !current.is_empty() {
                lines.push(current);
            }
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    fn hover_description(
        &self,
        map: &Map,
        assets: &Assets,
    ) -> Option<(
        RulesDescription,
        Rect,
        Option<CommandState>,
        String,
        Duration,
        bool,
    )> {
        let p = self.cursor_pos;
        if let Some(item) = self.open_container_item(map)
            && let Some(layout) = self.container_panel_layout(map, assets)
        {
            for (index, slot_rect) in layout.slots.iter().enumerate() {
                if !slot_rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    continue;
                }
                if let Some(container_item) =
                    item.container.as_ref().and_then(|items| items.get(index))
                {
                    return Some((
                        rules_ui::describe_item(container_item),
                        *slot_rect,
                        None,
                        format!("container:{}:{}", item.id, container_item.id),
                        Duration::from_millis(650),
                        false,
                    ));
                }
            }
        }
        if self
            .open_container_panel_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            return None;
        }

        let open_container = self.open_container_panel;
        let open_container_rect = self.open_container_panel_rect;
        for widget in self.button_widgets.values() {
            if !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                continue;
            }
            let entity = Self::resolve_party_entity(map, widget.party.as_deref());
            if let Some(entity) = entity {
                if let Some(inventory_index) = widget.inventory_index
                    && let Some(item) = entity
                        .inventory
                        .get(inventory_index)
                        .and_then(|item| item.as_ref())
                {
                    if open_container.is_some_and(|panel| {
                        panel.item_id == item.id && panel.owner_entity_id == Some(entity.id)
                    }) || open_container_rect
                        .is_some_and(|rect| Self::rects_intersect(rect, widget.rect))
                    {
                        return None;
                    }
                    return Some((
                        rules_ui::describe_item(item),
                        widget.rect,
                        None,
                        format!("inventory:{}:{}", widget.id, item.id),
                        Duration::from_millis(650),
                        false,
                    ));
                }
                if let Some(slot) = &widget.equipped_slot
                    && let Some(item) = entity.get_equipped_item(slot)
                {
                    return Some((
                        rules_ui::describe_item(item),
                        widget.rect,
                        None,
                        format!("equipped:{}:{}", widget.id, item.id),
                        Duration::from_millis(650),
                        false,
                    ));
                }
            }
            if let Some(command) =
                Self::resolved_widget_command(widget, assets, entity, &self.ui_state)
                && let Some(binding) = parse_client_command(&command)
            {
                let shortcuts = self.shortcut_labels_for_binding(&binding);
                match binding {
                    ClientCommandBinding::Control(_) => return None,
                    ClientCommandBinding::RulesAction(_) => {
                        let mut description = rules_ui::describe_command(assets, entity, &command);
                        Self::add_shortcut_line(&mut description, shortcuts);
                        let state = rules_ui::command_state(assets, entity, &command);
                        return Some((
                            description,
                            widget.rect,
                            Some(state),
                            format!("command:{}:{}", widget.id, command),
                            Duration::from_millis(650),
                            true,
                        ));
                    }
                    ClientCommandBinding::Intent(_)
                    | ClientCommandBinding::Screen(_)
                    | ClientCommandBinding::Game(_)
                    | ClientCommandBinding::Ui(_) => {
                        let mut description = rules_ui::describe_command(assets, entity, &command);
                        Self::add_shortcut_line(&mut description, shortcuts);
                        let state = rules_ui::command_state(assets, entity, &command);
                        return Some((
                            description,
                            widget.rect,
                            Some(state),
                            format!("command:{}:{}", widget.id, command),
                            Duration::from_millis(650),
                            true,
                        ));
                    }
                }
            }
        }

        if let Some(item_id) = self.hovered_item_id
            && let Some(item) = map.items.iter().find(|item| item.id == item_id)
        {
            let mut description = rules_ui::describe_item(item);
            let tile_pos = Self::quantize_2d_tile_pos(item.get_pos_xz());
            let pile_count = map
                .items
                .iter()
                .filter(|item| Self::quantize_2d_tile_pos(item.get_pos_xz()) == tile_pos)
                .count();
            if pile_count > 1 {
                description
                    .lines
                    .push(format!("Pile: {} items", pile_count));
            }
            return Some((
                description,
                Rect::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32, 1.0, 1.0),
                None,
                format!("world_item:{}", item.id),
                Duration::from_millis(650),
                false,
            ));
        }

        None
    }

    fn rects_intersect(a: Rect, b: Rect) -> bool {
        a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
    }
}
