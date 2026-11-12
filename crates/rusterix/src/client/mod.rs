pub mod action;
pub mod content;
pub mod screen_model;
use screen_model::*;
pub mod command {
    pub use crate::input_binding::*;
}
pub mod daylight;
pub mod draw2d;
mod intent_cursor;
pub mod parser {
    pub use crate::message_text::parser::*;
}
pub mod resolver {
    pub use crate::message_text::resolver::*;
}
pub mod rules_ui;
pub mod text_command;
pub mod widget;
mod world_text;

use instant::{Duration, Instant};
use scenevm::GeoId;

#[cfg(test)]
use crate::client::widget::BorderGradientDirection;
use crate::prelude::*;
use crate::{
    BrushPreview, Command, D2PreviewBuilder, Entity, EntityAction, MapMini, PlayerCamera, Rect,
    SceneHandler, Surface, Value,
    client::action::ClientAction,
    client::command::{ClientCommandBinding, parse_client_command},
    client::rules_ui::{CommandState, ContainerUiTemplate, RulesDescription},
    client::widget::{
        ButtonVisualState, TextInputWidget, Widget,
        avatar::AvatarWidget,
        choice::{ChoiceInteraction, ChoiceOption, ChoiceWidget, ChoiceWidgetKind},
        deco::DecoWidget,
        game::GameWidget,
        messages::MessagesWidget,
        profile::ProfileWidget,
        screen::ScreenWidget,
        stat::StatWidget,
        text::TextWidget,
    },
};
use draw2d::Draw2D;
use fontdue::*;
use rayon::prelude::*;
use screen_model::{
    CatalogDetailRow, CatalogDetailRowKind, CatalogEntryLayout as ActionsPanelEntryLayout,
    CatalogPanelKind as CatalogPanelContent, CatalogPanelLayout as ActionsPanelLayout,
    CatalogTabLayout as ActionsPanelTabLayout, EquipmentPanelLayout, EquipmentPanelSlotLayout,
    InventoryPanelSlotLayout,
};
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

#[derive(Clone, Debug)]
struct InventoryPanelLayout {
    rect: Rect,
    title_rect: Rect,
    close_rect: Rect,
    tab_rect: Rect,
    sort_rect: Rect,
    capacity_rect: Rect,
    previous_page_rect: Option<Rect>,
    next_page_rect: Option<Rect>,
    page_rect: Option<Rect>,
    slots: Vec<InventoryPanelSlotLayout>,
    page: usize,
    page_count: usize,
}

#[derive(Default)]
struct ClientDrawDebugTiming {
    window_started: Option<Instant>,
    frames: u32,
    total: Duration,
    setup: Duration,
    game_prepare: Duration,
    game_entities: Duration,
    game_render: Duration,
    game_composite: Duration,
    screen_base: Duration,
    text: Duration,
    avatars: Duration,
    profiles: Duration,
    stats: Duration,
    foreground: Duration,
    messages: Duration,
    inputs: Duration,
    buttons: Duration,
    button_resolve: Duration,
    button_state: Duration,
    button_draw: Duration,
    button_overlay: Duration,
    misc: Duration,
    particle_stats: crate::ParticleDebugStats,
}

#[derive(Default)]
struct ClientDrawDebugSample {
    total: Duration,
    setup: Duration,
    game_prepare: Duration,
    game_entities: Duration,
    game_render: Duration,
    game_composite: Duration,
    screen_base: Duration,
    text: Duration,
    avatars: Duration,
    profiles: Duration,
    stats: Duration,
    foreground: Duration,
    messages: Duration,
    inputs: Duration,
    buttons: Duration,
    button_resolve: Duration,
    button_state: Duration,
    button_draw: Duration,
    button_overlay: Duration,
    misc: Duration,
    particle_stats: crate::ParticleDebugStats,
}

impl ClientDrawDebugTiming {
    fn record(&mut self, sample: ClientDrawDebugSample) {
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *TRACE.get_or_init(|| std::env::var_os("ELDIRON_TRACE_FRAME_TIMINGS").is_some()) {
            let ms = |d: Duration| d.as_secs_f64() * 1000.;
            eprintln!(
                "[FrameTrace][ClientDraw] total={:.3} prepare={:.3} entities={:.3} render={:.3} composite={:.3} hud={:.3}",
                ms(sample.total),
                ms(sample.game_prepare),
                ms(sample.game_entities),
                ms(sample.game_render),
                ms(sample.game_composite),
                ms(sample.total.saturating_sub(
                    sample.game_prepare
                        + sample.game_entities
                        + sample.game_render
                        + sample.game_composite
                ))
            );
        }
        let now = Instant::now();
        let window_started = *self.window_started.get_or_insert(now);
        self.frames = self.frames.saturating_add(1);
        self.total += sample.total;
        self.setup += sample.setup;
        self.game_prepare += sample.game_prepare;
        self.game_entities += sample.game_entities;
        self.game_render += sample.game_render;
        self.game_composite += sample.game_composite;
        self.screen_base += sample.screen_base;
        self.text += sample.text;
        self.avatars += sample.avatars;
        self.profiles += sample.profiles;
        self.stats += sample.stats;
        self.foreground += sample.foreground;
        self.messages += sample.messages;
        self.inputs += sample.inputs;
        self.buttons += sample.buttons;
        self.button_resolve += sample.button_resolve;
        self.button_state += sample.button_state;
        self.button_draw += sample.button_draw;
        self.button_overlay += sample.button_overlay;
        self.misc += sample.misc;
        self.particle_stats = sample.particle_stats;

        let elapsed = now.saturating_duration_since(window_started);
        if elapsed < Duration::from_secs(2) {
            return;
        }

        let frames = self.frames.max(1) as f64;
        let avg_ms = |duration: Duration| duration.as_secs_f64() * 1000.0 / frames;
        eprintln!(
            "[RenderDebug][ClientDraw] frames={} avg_ms total={:.2} setup={:.2} game_prepare={:.2} game_entities={:.2} game_render={:.2} game_composite={:.2} screen_base={:.2} text={:.2} avatars={:.2} profiles={:.2} stats={:.2} foreground={:.2} messages={:.2} inputs={:.2} buttons={:.2} button_resolve={:.2} button_state={:.2} button_draw={:.2} button_overlay={:.2} misc={:.2}",
            self.frames,
            avg_ms(self.total),
            avg_ms(self.setup),
            avg_ms(self.game_prepare),
            avg_ms(self.game_entities),
            avg_ms(self.game_render),
            avg_ms(self.game_composite),
            avg_ms(self.screen_base),
            avg_ms(self.text),
            avg_ms(self.avatars),
            avg_ms(self.profiles),
            avg_ms(self.stats),
            avg_ms(self.foreground),
            avg_ms(self.messages),
            avg_ms(self.inputs),
            avg_ms(self.buttons),
            avg_ms(self.button_resolve),
            avg_ms(self.button_state),
            avg_ms(self.button_draw),
            avg_ms(self.button_overlay),
            avg_ms(self.misc),
        );
        let d2 = self.particle_stats.d2;
        let d3 = self.particle_stats.d3;
        eprintln!(
            "[RenderDebug][Particles] 2d emitters={} active={} billboards={}/{} dropped={} steps={} last_update_build_ms={:.2} 3d emitters={} active={} billboards={}/{} dropped={} steps={} last_update_build_ms={:.2}",
            d2.active_emitters,
            d2.active_particles,
            d2.rendered_billboards,
            d2.billboard_budget,
            d2.dropped_billboards,
            d2.simulation_steps,
            d2.update_build_ms,
            d3.active_emitters,
            d3.active_particles,
            d3.rendered_billboards,
            d3.billboard_budget,
            d3.dropped_billboards,
            d3.simulation_steps,
            d3.update_build_ms,
        );
        *self = Self {
            window_started: Some(now),
            ..Self::default()
        };
    }
}

pub struct Client {
    isometric_ground: crate::isometric::GroundCache,
    /// Creator chrome only; runtime render settings remain project-owned.
    pub editor_canvas_colors: Option<[RGBA; 3]>,
    pub use_editor_canvas: bool,
    pub curr_map_id: Uuid,

    pub builder_d2: D2PreviewBuilder,
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
    draw_debug_timing: ClientDrawDebugTiming,

    world_texts: Vec<world_text::Passage>,
    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, String, TheTime)>,

    // Name of player entity templates
    player_entities: Vec<String>,

    pub current_map: String,
    pub current_sector: String,
    current_screen: String,

    config: toml::Table,

    pub viewport: Vec2<i32>,
    /// Authored viewport used as the reference canvas for screen coordinates.
    reference_viewport: Vec2<i32>,
    /// Latest logical client surface size.
    surface_viewport: Vec2<i32>,
    /// The active screen opts into surface-sized game rendering and anchored UI.
    screen_responsive: bool,
    grid_size: f32,
    pub target_fps: i32,
    pub game_tick_ms: i32,
    pub firstp_eye_level: f32,
    pub mouse_look: crate::camera::mouse_look::MouseLook,
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
    action_bar_button_ids: FxHashMap<u32, Vec<u32>>,
    avatar_widgets: FxHashMap<Uuid, AvatarWidget>,
    profile_widgets: FxHashMap<Uuid, ProfileWidget>,
    stat_widgets: FxHashMap<Uuid, StatWidget>,
    text_widgets: FxHashMap<Uuid, TextWidget>,
    text_input_widgets: FxHashMap<u32, TextInputWidget>,
    choice_widgets: FxHashMap<u32, ChoiceWidget>,
    choice_controller: ChoiceController,
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
    selected_targeting_intent: Option<String>,
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

    // Stable target on a linked Block / Prop instance under the 3D cursor.
    hovered_block_prop_target: Option<BlockPropInteractionHit>,

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
    inventory_panel_open: bool,
    inventory_panel_rect: Option<Rect>,
    inventory_panel_title_rect: Option<Rect>,
    inventory_panel_close_rect: Option<Rect>,
    inventory_panel_previous_page_rect: Option<Rect>,
    inventory_panel_next_page_rect: Option<Rect>,
    inventory_panel_slots: Vec<InventoryPanelSlotLayout>,
    inventory_panel_position: Option<Vec2<i32>>,
    inventory_panel_page: usize,
    inventory_panel_selected_item: Option<u32>,
    inventory_panel_tabs: Option<ChoiceWidget>,
    inventory_panel_sort: Option<ChoiceWidget>,
    dragging_inventory_panel_item: bool,
    dragging_inventory_panel: bool,
    inventory_panel_drag_offset: Vec2<i32>,
    toolbar_inventory_panel_config: InventoryPanelConfig,
    custom_inventory_panel_config: Option<InventoryPanelConfig>,
    equipment_panel_open: bool,
    equipment_panel_rect: Option<Rect>,
    equipment_panel_title_rect: Option<Rect>,
    equipment_panel_close_rect: Option<Rect>,
    equipment_panel_avatar_rect: Option<Rect>,
    equipment_panel_slots: Vec<EquipmentPanelSlotLayout>,
    equipment_panel_position: Option<Vec2<i32>>,
    equipment_panel_avatar: AvatarWidget,
    dragging_equipment_panel_item: bool,
    dragging_equipment_panel: bool,
    equipment_panel_drag_offset: Vec2<i32>,
    toolbar_equipment_panel_config: EquipmentPanelConfig,
    custom_equipment_panel_config: Option<EquipmentPanelConfig>,
    preferences_panel_open: bool,
    preferences_panel_rect: Option<Rect>,
    preferences_panel_close_rect: Option<Rect>,
    preferences_reset_rect: Option<Rect>,
    preferences_tooltips_choice: Option<ChoiceWidget>,
    preferences_delay_choice: Option<ChoiceWidget>,
    toolbar_preferences_panel_config: PreferencesPanelConfig,
    tooltips_enabled: bool,
    tooltip_delay_ms: u64,
    actions_panel_open: bool,
    actions_panel_content: CatalogPanelContent,
    actions_panel_rect: Option<Rect>,
    actions_panel_title_rect: Option<Rect>,
    actions_panel_close_rect: Option<Rect>,
    actions_panel_assign_rect: Option<Rect>,
    actions_panel_previous_page_rect: Option<Rect>,
    actions_panel_next_page_rect: Option<Rect>,
    actions_panel_scroll_track_rect: Option<Rect>,
    actions_panel_scroll_thumb_rect: Option<Rect>,
    actions_panel_page_count: usize,
    actions_panel_detail_rect: Option<Rect>,
    actions_panel_detail_scroll_track_rect: Option<Rect>,
    actions_panel_detail_scroll_thumb_rect: Option<Rect>,
    actions_panel_detail_scroll: f32,
    actions_panel_detail_scroll_max: f32,
    dragging_actions_detail_scrollbar: bool,
    actions_detail_scrollbar_drag_offset: f32,
    actions_panel_tabs: Vec<ActionsPanelTabLayout>,
    actions_panel_entries: Vec<ActionsPanelEntryLayout>,
    actions_panel_tab: String,
    actions_panel_selected_command: Option<String>,
    actions_assignment_mode: bool,
    pending_action_assignment: Option<String>,
    dragging_action_command: Option<String>,
    dragging_actions_panel: bool,
    actions_panel_drag_offset: Vec2<i32>,
    actions_panel_position: Option<Vec2<i32>>,
    actions_panel_page: usize,
    toolbar_actions_panel_config: CatalogPanelConfig,
    toolbar_spellbook_config: CatalogPanelConfig,
    custom_actions_panel_config: Option<CatalogPanelConfig>,
    custom_spellbook_config: Option<CatalogPanelConfig>,
    actions_panel_catalog_rules: String,
    actions_panel_catalog_class: Option<String>,
    actions_panel_catalog: Vec<rules_ui::ActionCatalogGroup>,
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
        self.world_texts.clear();
    }

    fn say_table_from_widget(widget: &GameWidget) -> Option<toml::value::Table> {
        widget
            .toml_str
            .parse::<toml::Table>()
            .ok()
            .and_then(|table| table.get("say").and_then(toml::Value::as_table).cloned())
    }

    fn get_say_color_from_config(config: &toml::Table, category: &str, fallback: Pixel) -> Pixel {
        crate::speech::style::color(config, None, category, fallback)
    }

    fn get_widget_say_color(
        widget_say: Option<&toml::value::Table>,
        config: &toml::Table,
        category: &str,
        fallback: Pixel,
    ) -> Pixel {
        crate::speech::style::color(config, widget_say, category, fallback)
    }

    fn get_say_color(&self, category: &str) -> Pixel {
        Self::get_say_color_from_config(&self.config, category, self.messages_font_color)
    }

    fn get_widget_say_background_enabled(
        widget_say: Option<&toml::value::Table>,
        config: &toml::Table,
    ) -> bool {
        crate::speech::style::background_enabled(config, widget_say)
    }

    fn get_widget_say_background_color(
        widget_say: Option<&toml::value::Table>,
        config: &toml::Table,
    ) -> Pixel {
        crate::speech::style::background_color(config, widget_say)
    }

    fn get_say_background_enabled(&self) -> bool {
        Self::get_widget_say_background_enabled(None, &self.config)
    }

    fn get_say_background_color(&self) -> Pixel {
        Self::get_widget_say_background_color(None, &self.config)
    }

    fn get_say_duration_ticks(&self) -> i64 {
        let local = self
            .game_widgets
            .values()
            .find_map(Self::say_table_from_widget);
        crate::speech::style::duration_ticks(&self.config, local.as_ref())
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

    fn shortcut_labels_for_binding(&self, binding: &ClientCommandBinding) -> Vec<String> {
        self.client_action
            .lock()
            .map(|action| action.shortcut_labels_for_binding(binding))
            .unwrap_or_default()
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
        let mut found_iso_2d = None;
        for widget in self.game_widgets.values() {
            match widget.camera {
                PlayerCamera::D3FirstP
                | PlayerCamera::D3FirstPMouse
                | PlayerCamera::D3FirstPGrid => {
                    return Some(widget.camera.clone());
                }
                PlayerCamera::D3Iso => found_iso = true,
                PlayerCamera::D2 | PlayerCamera::D2Grid => found_d2 = true,
                PlayerCamera::D2Iso | PlayerCamera::D2IsoGrid => {
                    found_iso_2d = Some(widget.camera.clone())
                }
            }
        }
        if found_iso {
            Some(PlayerCamera::D3Iso)
        } else if found_iso_2d.is_some() {
            found_iso_2d
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

    pub fn begin_mouse_look(&mut self, coord: Vec2<i32>, map: &Map) -> bool {
        let coord = self.screen_to_viewport(coord);
        let point = Vec2::new(coord.x as f32, coord.y as f32);
        let Some(widget) = self.game_widgets.values().find(|widget| {
            widget.camera == PlayerCamera::D3FirstPMouse && widget.rect.contains(point)
        }) else {
            return false;
        };
        let Some(player) = map.entities.iter().find(|entity| entity.is_player()) else {
            return false;
        };
        self.mouse_look.begin(
            player.orientation,
            widget.mouse_sensitivity,
            widget.mouse_invert_y,
        );
        true
    }

    pub fn end_mouse_look(&mut self) -> bool {
        let active = self.mouse_look.active;
        self.mouse_look.active = false;
        active
    }

    pub fn mouse_look_hover(&mut self, coord: Vec2<i32>, map: &Map) {
        let viewport_coord = self.screen_to_viewport(coord);
        let point = Vec2::new(viewport_coord.x as f32, viewport_coord.y as f32);
        if self.game_widgets.values().any(|widget| {
            widget.camera == PlayerCamera::D3FirstPMouse && widget.rect.contains(point)
        }) {
            if !self.mouse_look.active {
                self.begin_mouse_look(coord, map);
            }
        } else {
            self.end_mouse_look();
        }
    }

    fn is_2d_camera(camera: &PlayerCamera) -> bool {
        screen_model::is_2d_camera(camera)
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
            isometric_ground: Default::default(),
            editor_canvas_colors: None,
            use_editor_canvas: false,
            curr_map_id: Uuid::default(),

            builder_d2: D2PreviewBuilder::new(),
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
            draw_debug_timing: ClientDrawDebugTiming::default(),

            messages_font_size: 15.0,
            messages_font_color: [229, 229, 1, 255],

            messages_to_draw: FxHashMap::default(),
            world_texts: Vec::new(),

            player_entities: Vec::new(),

            current_map: String::new(),
            current_sector: String::new(),
            current_screen: String::new(),

            config: toml::Table::default(),
            viewport: Vec2::zero(),
            reference_viewport: Vec2::zero(),
            surface_viewport: Vec2::zero(),
            screen_responsive: false,
            grid_size: 32.0,
            target_fps: 30,
            game_tick_ms: 250,
            firstp_eye_level: 1.7,
            mouse_look: Default::default(),
            firstp_camera_y: None,
            active_player_camera: None,

            target_offset: Vec2::zero(),
            target: TheRGBABuffer::default(),
            overlay: TheRGBABuffer::default(),

            game_widgets: FxHashMap::default(),
            button_widgets: FxHashMap::default(),
            action_bar_button_ids: FxHashMap::default(),
            avatar_widgets: FxHashMap::default(),
            profile_widgets: FxHashMap::default(),
            stat_widgets: FxHashMap::default(),
            text_widgets: FxHashMap::default(),
            text_input_widgets: FxHashMap::default(),
            choice_widgets: FxHashMap::default(),
            choice_controller: ChoiceController::default(),
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
            selected_targeting_intent: None,
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
            hovered_block_prop_target: None,
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
            inventory_panel_open: false,
            inventory_panel_rect: None,
            inventory_panel_title_rect: None,
            inventory_panel_close_rect: None,
            inventory_panel_previous_page_rect: None,
            inventory_panel_next_page_rect: None,
            inventory_panel_slots: Vec::new(),
            inventory_panel_position: None,
            inventory_panel_page: 0,
            inventory_panel_selected_item: None,
            inventory_panel_tabs: None,
            inventory_panel_sort: None,
            dragging_inventory_panel_item: false,
            dragging_inventory_panel: false,
            inventory_panel_drag_offset: Vec2::zero(),
            toolbar_inventory_panel_config: InventoryPanelConfig::default(),
            custom_inventory_panel_config: None,
            equipment_panel_open: false,
            equipment_panel_rect: None,
            equipment_panel_title_rect: None,
            equipment_panel_close_rect: None,
            equipment_panel_avatar_rect: None,
            equipment_panel_slots: Vec::new(),
            equipment_panel_position: None,
            equipment_panel_avatar: AvatarWidget::new(),
            dragging_equipment_panel_item: false,
            dragging_equipment_panel: false,
            equipment_panel_drag_offset: Vec2::zero(),
            toolbar_equipment_panel_config: EquipmentPanelConfig::default(),
            custom_equipment_panel_config: None,
            preferences_panel_open: false,
            preferences_panel_rect: None,
            preferences_panel_close_rect: None,
            preferences_reset_rect: None,
            preferences_tooltips_choice: None,
            preferences_delay_choice: None,
            toolbar_preferences_panel_config: PreferencesPanelConfig::default(),
            tooltips_enabled: true,
            tooltip_delay_ms: 650,
            actions_panel_open: false,
            actions_panel_content: CatalogPanelContent::Actions,
            actions_panel_rect: None,
            actions_panel_title_rect: None,
            actions_panel_close_rect: None,
            actions_panel_assign_rect: None,
            actions_panel_previous_page_rect: None,
            actions_panel_next_page_rect: None,
            actions_panel_scroll_track_rect: None,
            actions_panel_scroll_thumb_rect: None,
            actions_panel_page_count: 1,
            actions_panel_detail_rect: None,
            actions_panel_detail_scroll_track_rect: None,
            actions_panel_detail_scroll_thumb_rect: None,
            actions_panel_detail_scroll: 0.0,
            actions_panel_detail_scroll_max: 0.0,
            dragging_actions_detail_scrollbar: false,
            actions_detail_scrollbar_drag_offset: 0.0,
            actions_panel_tabs: Vec::new(),
            actions_panel_entries: Vec::new(),
            actions_panel_tab: "all".to_string(),
            actions_panel_selected_command: None,
            actions_assignment_mode: false,
            pending_action_assignment: None,
            dragging_action_command: None,
            dragging_actions_panel: false,
            actions_panel_drag_offset: Vec2::zero(),
            actions_panel_position: None,
            actions_panel_page: 0,
            toolbar_actions_panel_config: CatalogPanelConfig::actions_default(),
            toolbar_spellbook_config: CatalogPanelConfig::default(),
            custom_actions_panel_config: None,
            custom_spellbook_config: None,
            actions_panel_catalog_rules: String::new(),
            actions_panel_catalog_class: None,
            actions_panel_catalog: Vec::new(),
        }
    }

    /// Increase the anim counter.
    pub fn inc_animation_frame(&mut self) {
        self.animation_frame += 1;
        for widget in self.deco_widgets.values_mut() {
            widget.animation_frame = self.animation_frame;
        }
        for widget in self.profile_widgets.values_mut() {
            widget.animation_frame = self.animation_frame;
        }

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
            self.camera_d3.set_parameter_f32(
                "pitch",
                if self.active_player_camera == Some(PlayerCamera::D3FirstPMouse) {
                    self.mouse_look.pitch
                } else {
                    0.0
                },
            );
        } else {
            self.firstp_camera_y = None;
            entity.apply_to_camera(&mut self.camera_d3, self.firstp_eye_level);
        }
    }

    pub fn set_map_tool_type_d2(&mut self, tool: MapToolType) {
        self.map_tool_type_d2 = tool;
        self.builder_d2.set_map_tool_type(tool);
    }

    pub fn set_map_hover_info_d2(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
        self.builder_d2.set_map_hover_info(hover, hover_cursor);
    }

    pub fn set_camera_info_d2(&mut self, pos: Option<vek::Vec3<f32>>, look_at: Option<Vec3<f32>>) {
        self.builder_d2.set_camera_info(pos, look_at);
    }

    pub fn set_clip_rect_d2(&mut self, clip_rect: Option<Rect>) {
        self.builder_d2.set_clip_rect(clip_rect);
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
        if map.camera == crate::MapCamera::TwoDIso {
            self.curr_map_id = map.id;
            // The native iso canvas doesn't consume the legacy scene's collision
            // map. GameWidget owns its own navigation data for play.
            return;
        }
        self.curr_map_id = map.id;
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
        scene_handler.build_dynamics_2d(map, self.animation_frame, assets, &Default::default());
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
        if map.camera == crate::MapCamera::TwoDIso {
            return;
        }
        self.builder_d2.build_entities_items(
            map,
            assets,
            &mut self.scene,
            screen_size,
            edit_surface,
            scene_handler,
            draw_sectors,
        );
        scene_handler.build_dynamics_2d(map, self.animation_frame, assets, &Default::default());
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
        if map.camera == crate::MapCamera::TwoDIso {
            self.draw_d2(map, pixels, width, height, assets, scene_handler);
            return;
        }
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
        let rendered_grid_size = if self.builder_d2.draw_grid {
            map.grid_size
        } else {
            0.0
        };

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
                rendered_grid_size,
                map.subdivisions,
                map.offset.x,
                -map.offset.y,
            )));
            scene_handler.vm.execute(scenevm::Atom::SetGP2(Vec4::one()));
            // Overlay compute output is display-space, unlike the base raster clear.
            let color = |c: RGBA| {
                Vec4::new(
                    c[0] as f32 / 255.,
                    c[1] as f32 / 255.,
                    c[2] as f32 / 255.,
                    1.,
                )
            };
            if let Some(colors) = self.editor_canvas_colors.filter(|_| self.use_editor_canvas) {
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP5(color(colors[0])));
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP6(color(colors[1])));
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP7(color(colors[2])));
            } else {
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP5(Vec4::zero()));
            }

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
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        if map.camera == crate::MapCamera::TwoDIso {
            scene_handler.apply_runtime_render_state_settings();
            scene_handler.settings.apply_hour(self.server_time.to_f32());
            scene_handler.build_dynamics_2d(map, self.animation_frame, assets, &Default::default());
            let particles = scene_handler.isometric_particles(map, assets, self.animation_frame);
            let lighting = crate::isometric::lighting::Lighting::new(
                &scene_handler.settings,
                scene_handler.vm.vm.lights.values(),
                self.animation_frame,
            );
            let mut buffer = TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
            let avatars = scene_handler.isometric_avatars(map, assets, self.animation_frame);
            if let Err(error) = crate::isometric::draw_scene_gpu_cached(
                map,
                assets,
                &mut buffer,
                crate::isometric::Projection::editor(map, Vec2::new(width as f32, height as f32)),
                self.animation_frame as f32 * assets.animation_tick_seconds(),
                Some(self.builder_d2.isometric_guides()),
                &avatars,
                Some(&mut self.isometric_ground),
                &lighting,
                &mut scene_handler.vm,
                &particles,
            ) {
                eprintln!("{error}");
            }
            pixels.copy_from_slice(buffer.pixels());
            return;
        }
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
        let rendered_grid_size = if self.builder_d2.draw_grid {
            map.grid_size
        } else {
            0.0
        };
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
                rendered_grid_size,
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
        let colors = self.editor_canvas_colors.filter(|_| self.use_editor_canvas);
        let rgba = |c: RGBA| {
            // Raster clears pass through gamma encoding; custom compute output
            // already writes display-space pixels, just like the overlay layer.
            let component = |v: u8| {
                let value = v as f32 / 255.;
                if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
                    value
                } else {
                    value.powf(2.2)
                }
            };
            Vec4::new(component(c[0]), component(c[1]), component(c[2]), 1.)
        };
        if let Some([background, grid, subgrid]) = colors {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetBackground(rgba(background)));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP5(rgba(background)));
            scene_handler.vm.execute(scenevm::Atom::SetGP6(rgba(grid)));
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP7(rgba(subgrid)));
        } else {
            scene_handler
                .vm
                .execute(scenevm::Atom::SetGP5(Vec4::zero()));
        }

        if matches!(scenevm_mode_2d, scenevm::RenderMode::Compute2D) {
            scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
                rendered_grid_size,
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
                rendered_grid_size,
                map.subdivisions,
                map.offset.x,
                -map.offset.y,
            )));
            scene_handler.vm.execute(scenevm::Atom::SetGP2(Vec4::one()));
            // Overlay compute output is display-space, unlike the base raster clear.
            let color = |c: RGBA| {
                Vec4::new(
                    c[0] as f32 / 255.,
                    c[1] as f32 / 255.,
                    c[2] as f32 / 255.,
                    1.,
                )
            };
            if let Some(colors) = self.editor_canvas_colors.filter(|_| self.use_editor_canvas) {
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP5(color(colors[0])));
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP6(color(colors[1])));
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP7(color(colors[2])));
            } else {
                scene_handler
                    .vm
                    .execute(scenevm::Atom::SetGP5(Vec4::zero()));
            }

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
        assets: &Assets,
        scene_handler: &mut SceneHandler,
        editor_neutral_background: bool,
    ) {
        self.draw_d3_with_camera_override(
            map,
            pixels,
            width,
            height,
            assets,
            scene_handler,
            editor_neutral_background,
            None,
            true,
            false,
        );
    }

    /// Draw live 3D dynamics and editor overlays over transparency while the
    /// static world is supplied by a baked background.
    pub fn draw_d3_dynamic_overlay(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.draw_d3_with_camera_override(
            map,
            pixels,
            width,
            height,
            assets,
            scene_handler,
            false,
            None,
            false,
            true,
        );
    }

    pub fn draw_d3_with_camera_override(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        _assets: &Assets,
        scene_handler: &mut SceneHandler,
        editor_neutral_background: bool,
        camera_override: Option<scenevm::Camera3D>,
        static_geometry_enabled: bool,
        transparent_background: bool,
    ) {
        scene_handler.vm.set_active_vm(0);
        self.scene.animation_frame = self.animation_frame;

        let hour = self.server_time.to_f32();

        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_3d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_3d();
        if editor_neutral_background {
            scene_handler.vm.execute(scenevm::Atom::SetGP0({
                let rgb = self
                    .editor_canvas_colors
                    .filter(|_| self.use_editor_canvas)
                    .map(|colors| {
                        std::array::from_fn(|i| {
                            let display = (colors[0][i] as f32 / 255.).powf(2.2);
                            display / (1. - display).max(0.001)
                        })
                    })
                    .unwrap_or([0.018, 0.018, 0.020]);
                Vec4::new(rgb[0], rgb[1], rgb[2], 0.)
            }));
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(if transparent_background {
                Vec4::zero()
            } else {
                Vec4::new(0.0, 0.0, 0.0, 1.0)
            }));

        let base_render_mode = if !static_geometry_enabled {
            scenevm::RenderMode::Raster3D
        } else if scene_handler.vm.layer_progressive_sample_index(0).is_some() {
            scenevm::RenderMode::Compute3D
        } else {
            scene_handler.settings.scenevm_mode_3d()
        };
        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(base_render_mode));
        scene_handler
            .vm
            .set_layer_raster3d_static_geometry_enabled(0, static_geometry_enabled);
        // Organic foliage/paint stamps are static baked content even though Raster3D draws
        // them through a dedicated billboard pipeline. Do not draw them again in the live
        // dynamic overlay.
        scene_handler.vm.execute(scenevm::Atom::SetOrganicVisible {
            visible: static_geometry_enabled,
        });

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: camera_override.unwrap_or_else(|| {
                self.camera_d3
                    .as_scenevm_camera_for_surface(width as f32, height as f32)
            }),
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
                    camera: self
                        .camera_d3
                        .as_scenevm_camera_for_surface(width as f32, height as f32),
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
            let view = self
                .camera_d3
                .view_matrix_for_surface(width as f32, height as f32);
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

    /// Whether the project requested aggregated client and renderer timings.
    pub fn frame_timings_enabled(&self) -> bool {
        self.get_config_bool_default("debug", "frame_timings", false)
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

    /// Setup the client with the given assets.
    pub fn setup(&mut self, assets: &mut Assets, scene_handler: &mut SceneHandler) -> Vec<Command> {
        let mut commands = vec![];
        self.first_game_draw = true;
        self.intent = String::new();
        self.selected_targeting_intent = None;
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
        scenevm::set_render_debug_enabled(self.frame_timings_enabled());

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

        let viewport_width = self.get_config_i32_default("viewport", "width", 1280);
        let viewport_height = self.get_config_i32_default("viewport", "height", 720);
        let viewport_grid_size = self.get_config_i32_default("viewport", "grid_size", 32);
        self.viewport = if self
            .get_config_string_default("viewport", "unit", "pixel")
            .eq_ignore_ascii_case("cell")
        {
            Vec2::new(
                viewport_width.max(1) * viewport_grid_size.max(1),
                viewport_height.max(1) * viewport_grid_size.max(1),
            )
        } else {
            Vec2::new(viewport_width, viewport_height)
        };
        self.reference_viewport = self.viewport;
        self.surface_viewport = self.viewport;
        self.screen_responsive = false;
        self.target_fps = self.get_config_i32_default("game", "target_fps", 30);
        self.game_tick_ms = self.get_config_i32_default("game", "game_tick_ms", 250);
        self.firstp_eye_level = self.get_config_f32_default("game", "firstp_eye_level", 1.7);
        self.click_intents_2d = self.get_config_bool_default("game", "persistent_intents", false)
            || self.get_config_bool_default("game", "click_intents_2d", false)
            || self.get_config_bool_default("game", "persistent_2d_intents", false);
        self.grid_size = viewport_grid_size as f32;
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
                            if let Some(data) = entity.attributes.get_str("_input_bindings") {
                                self.client_action.lock().unwrap().set_input_data(data);
                            }
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
        } else if self.current_screen.trim().is_empty() {
            self.init_region_fallback(assets, scene_handler);
        } else if !self.current_screen.trim().is_empty() {
            eprintln!("Did not find start screen");
        }

        commands
    }

    fn screen_has_widget_role(assets: &Assets, screen_name: &str, role_name: &str) -> bool {
        let Some(screen) = assets.screens.get(screen_name) else {
            return false;
        };

        screen.sectors.iter().any(|sector| {
            screen_model::sector_ui_role(sector).is_some_and(|role| role == role_name)
        })
    }

    fn screen_base_render_map(screen: &Map) -> Map {
        screen_model::screen_static_layer_render_map(screen, false)
    }

    fn screen_foreground_render_map(screen: &Map) -> Map {
        screen_model::screen_static_layer_render_map(screen, true)
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

    fn set_or_append_ui_state(
        &mut self,
        binding: &str,
        value: &str,
        append: bool,
        separator: &str,
        max_parts: Option<usize>,
    ) {
        let value = screen_model::button_binding_value(
            &self.ui_state,
            binding,
            value,
            append,
            separator,
            max_parts,
        );
        self.set_ui_state(binding, &value);
    }

    fn sync_bound_button_activation(&mut self, binding: &str) {
        screen_model::sync_bound_button_activation(
            &self.button_widgets,
            &self.ui_state,
            &mut self.activated_widgets,
            &mut self.permanently_activated_widgets,
            binding,
        );
    }

    /// Editor quick play uses authored entity configuration rather than setup-screen choices.
    /// This is opt-in; normal client startup continues to show the configured start screen.
    pub fn start_with_entity_defaults(
        &mut self,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) -> Vec<Command> {
        self.ui_state.remove("start.class");
        self.ui_state.remove("start.name");
        let mut commands = Vec::new();
        self.process_game_command("start", assets, scene_handler, &mut commands);
        commands
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
        let prepared = crate::player_start::prepare_player_start_with_templates(
            map,
            &self.player_entities,
            crate::player_start::PlayerStartOptions {
                class,
                name: self.ui_state.get("start.name").map(String::as_str),
                entrance: true,
            },
        )
        .ok()?;
        self.pending_game_camera_pos = Some(Vec2::from(prepared.info.position));
        self.client_action = Arc::new(Mutex::new(ClientAction::default()));
        self.client_action
            .lock()
            .unwrap()
            .init(prepared.info.template, assets);
        if let Some(data) = &prepared.info.input_bindings {
            self.client_action.lock().unwrap().set_input_data(data);
        }
        Some(prepared.command)
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

    fn clear_screen_widgets(&mut self) {
        for id in self.action_bar_button_ids.values().flatten() {
            self.activated_widgets.retain(|active| active != id);
            self.permanently_activated_widgets
                .retain(|active| active != id);
            if self.pressed_widget == Some(*id) {
                self.pressed_widget = None;
            }
        }
        self.game_widgets.clear();
        self.mouse_look = Default::default();
        self.button_widgets.clear();
        self.action_bar_button_ids.clear();
        self.avatar_widgets.clear();
        self.profile_widgets.clear();
        self.stat_widgets.clear();
        self.text_widgets.clear();
        self.text_input_widgets.clear();
        self.choice_widgets.clear();
        self.choice_controller = ChoiceController::default();
        self.deco_widgets.clear();
        self.messages_widgets.clear();
        self.screen_widget = None;
        self.focused_text_input = None;
        self.toolbar_actions_panel_config = CatalogPanelConfig::actions_default();
        self.toolbar_spellbook_config = CatalogPanelConfig::default();
        self.toolbar_inventory_panel_config = InventoryPanelConfig::default();
        self.toolbar_equipment_panel_config = EquipmentPanelConfig::default();
        self.toolbar_preferences_panel_config = PreferencesPanelConfig::default();
        self.custom_actions_panel_config = None;
        self.custom_spellbook_config = None;
        self.custom_inventory_panel_config = None;
        self.custom_equipment_panel_config = None;
        self.inventory_panel_position = None;
        self.inventory_panel_page = 0;
        self.inventory_panel_tabs = None;
        self.inventory_panel_sort = None;
        self.inventory_panel_slots.clear();
        self.equipment_panel_position = None;
        self.equipment_panel_slots.clear();
        self.preferences_tooltips_choice = None;
        self.preferences_delay_choice = None;
        self.actions_panel_page = 0;
        self.actions_panel_position = None;
        self.actions_panel_tab = "all".to_string();
        self.actions_panel_selected_command = None;
        self.actions_panel_detail_scroll = 0.0;
    }

    fn draw_choice_widgets(
        widgets: &mut FxHashMap<u32, ChoiceWidget>,
        open_dropdown: Option<u32>,
        buffer: &mut TheRGBABuffer,
        assets: &Assets,
        draw2d: &Draw2D,
        cursor: Vec2<f32>,
        ui_state: &FxHashMap<String, String>,
        hidden: &[String],
    ) {
        let mut ids = widgets.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        if let Some(open_id) = open_dropdown
            && let Some(index) = ids.iter().position(|id| *id == open_id)
        {
            ids.remove(index);
            ids.push(open_id);
        }

        for id in ids {
            let Some(widget) = widgets.get_mut(&id) else {
                continue;
            };
            if screen_model::widget_name_is_hidden(&widget.name, hidden) {
                continue;
            }
            if Some(id) != open_dropdown {
                widget.sync_value(ui_state.get(&widget.binding).map(String::as_str));
            }
            widget.draw(buffer, assets, draw2d, cursor);
        }
    }

    fn interact_choice_widgets(&mut self, point: Vec2<f32>) -> bool {
        let outcome = self.choice_controller.pointer(
            &mut self.choice_widgets,
            &self.widgets_to_hide,
            &self.ui_state,
            point,
        );
        if let Some((binding, value)) = outcome.selection {
            self.set_ui_state(&binding, &value);
        }
        if outcome.consumed {
            self.focused_text_input = None;
        }
        outcome.consumed
    }

    fn init_region_fallback(&mut self, assets: &Assets, scene_handler: &mut SceneHandler) {
        self.clear_screen_widgets();

        let mut game_widget = self.region_fallback_widget();
        if let Some(map) = assets.maps.get(&self.current_map) {
            game_widget.build(map, assets, scene_handler);
        }
        self.game_widgets.insert(Uuid::default(), game_widget);
    }

    fn region_fallback_widget(&self) -> GameWidget {
        let width = self.viewport.x.max(1);
        let height = self.viewport.y.max(1);
        let mut model = ScreenElements::region_fallback(
            &self.current_map,
            ScreenLayout {
                screen_responsive: false,
                reference_viewport: Vec2::new(width, height),
                viewport: Vec2::new(width, height),
                grid_size: self.grid_size,
            },
            [0, 0, 0, 255],
            FxHashMap::default(),
        );
        let mut game_widget = model.game_widgets.remove(&Uuid::nil()).unwrap();
        // Only the compatibility renderer allocates a destination bitmap.
        game_widget.buffer = TheRGBABuffer::new(TheDim::sized(width, height));
        game_widget
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
        self.draw_game_with_widget_overlay(
            map,
            assets,
            messages,
            choices,
            scene_handler,
            |_, _| false,
        );
    }

    /// Draw the game into the internal buffer, allowing callers to update 3D widget render
    /// state after it has been prepared for the current camera and before it is rendered.
    pub fn draw_game_with_widget_overlay<F>(
        &mut self,
        map: &Map,
        assets: &Assets,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
        scene_handler: &mut SceneHandler,
        widget_overlay: F,
    ) where
        F: FnMut(&mut GameWidget, &mut SceneHandler) -> bool,
    {
        self.draw_game_with_widget_overlays(
            map,
            assets,
            messages,
            choices,
            scene_handler,
            widget_overlay,
            |_, _| {},
        );
    }

    /// Draw the game with separate pre-render state preparation and post-render pixel overlays.
    pub fn draw_game_with_widget_overlays<F, G>(
        &mut self,
        map: &Map,
        assets: &Assets,
        messages: Vec<crate::server::Message>,
        choices: Vec<crate::MultipleChoice>,
        scene_handler: &mut SceneHandler,
        mut widget_overlay: F,
        mut post_widget_overlay: G,
    ) where
        F: FnMut(&mut GameWidget, &mut SceneHandler) -> bool,
        G: FnMut(&mut GameWidget, &mut SceneHandler),
    {
        let messages = messages
            .into_iter()
            .filter(|m| m.4 != "look")
            .collect::<Vec<_>>();
        let debug_enabled = self.frame_timings_enabled();
        let debug_total_started = Instant::now();
        let debug_setup_started = Instant::now();
        let mut debug_game_prepare = Duration::ZERO;
        let mut debug_game_entities = Duration::ZERO;
        let mut debug_game_render = Duration::ZERO;
        let mut debug_game_composite = Duration::ZERO;
        let mut debug_screen_base = Duration::ZERO;
        let mut debug_text = Duration::ZERO;
        let mut debug_avatars = Duration::ZERO;
        let mut debug_profiles = Duration::ZERO;
        let mut debug_stats = Duration::ZERO;
        let mut debug_foreground = Duration::ZERO;
        let mut debug_messages = Duration::ZERO;
        let mut debug_inputs = Duration::ZERO;
        let mut debug_buttons = Duration::ZERO;
        let mut debug_button_resolve = Duration::ZERO;
        let mut debug_button_state = Duration::ZERO;
        let mut debug_button_draw = Duration::ZERO;
        let mut debug_button_overlay = Duration::ZERO;
        let mut debug_misc = Duration::ZERO;

        scene_handler.vm.set_active_vm(0);
        // Keep scene timing in sync with config
        scene_handler.set_timings(self.target_fps as f32, self.game_tick_ms);
        self.update_active_player_camera(map);

        // Preserve selected slot targeting until the user chooses a target.
        self.current_sector.clear();
        if let Some(leader) = screen_model::resolve_party_entity(map, Some("leader")) {
            self.intent = self
                .selected_slot_intent(map, assets)
                .or_else(|| self.selected_targeting_intent.clone())
                .unwrap_or_else(|| leader.get_attr_string("intent").unwrap_or_default());
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
        let debug_setup = debug_setup_started.elapsed();
        // First process the game widgets
        for widget in self.game_widgets.values_mut() {
            let stage_started = Instant::now();
            widget.firstp_eye_level = self.firstp_eye_level;
            widget.mouse_look_pitch = self.mouse_look.pitch;
            widget.apply_entities(map, assets, self.animation_frame, scene_handler);
            debug_game_entities += stage_started.elapsed();
            let stage_started = Instant::now();
            widget.prepare_frame(
                map,
                &self.server_time,
                self.animation_frame,
                assets,
                scene_handler,
            );
            debug_game_prepare += stage_started.elapsed();

            let stage_started = Instant::now();
            let _ = widget_overlay(widget, scene_handler);
            widget.render_prepared_frame(
                map,
                &self.server_time,
                self.animation_frame,
                assets,
                scene_handler,
            );
            debug_game_render += stage_started.elapsed();

            let stage_started = Instant::now();
            post_widget_overlay(widget, scene_handler);

            if let Some(font) = &self.messages_font {
                let widget_say = Self::say_table_from_widget(widget);
                let say_bg_enabled =
                    Self::get_widget_say_background_enabled(widget_say.as_ref(), &say_config);
                let say_bg_color =
                    Self::get_widget_say_background_color(widget_say.as_ref(), &say_config);
                if Self::is_2d_camera(&widget.camera) {
                    let width = widget.buffer.dim().width as usize;
                    let height = widget.buffer.dim().height as usize;
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
                        let screen_pos = widget.ground_to_screen(*grid_pos) * overlay_scale;
                        let sx = screen_pos.x.round() as isize;
                        let sy = screen_pos.y.round() as isize;

                        let pixels = widget.buffer.pixels_mut();
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

                    let view = widget
                        .camera_d3
                        .view_matrix_for_surface(width as f32, height as f32);
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
            debug_game_composite += stage_started.elapsed();
        }
        let stage_started = Instant::now();
        // World objects reveal their descriptions only through an explicit Look.

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
        debug_misc += stage_started.elapsed();

        let stage_started = Instant::now();
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
                let leader = screen_model::resolve_party_entity(map, Some("leader"));
                for w in self.button_widgets.iter() {
                    if screen_model::resolved_widget_intent_payload(
                        w.1,
                        assets,
                        leader,
                        &self.ui_state,
                    )
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
                let background = screen_widget.background_color;
                let holes: Vec<_> = self
                    .game_widgets
                    .values()
                    .map(|w| {
                        (
                            w.rect.x as i32,
                            w.rect.y as i32,
                            w.buffer.dim().width,
                            w.buffer.dim().height,
                        )
                    })
                    .collect();
                screen_widget.composite_cached(&mut self.target, assets, &holes, |buffer| {
                    Self::punch_game_widget_holes(buffer, background, self.game_widgets.values());
                });
            }
        }
        debug_screen_base += stage_started.elapsed();

        // Draw normal deco widgets on top of the screen render.
        let stage_started = Instant::now();
        Self::draw_deco_widgets_with_layer(
            &mut self.deco_widgets,
            &mut self.target,
            map,
            &self.currencies,
            assets,
            |layer| layer >= 0,
        );
        debug_misc += stage_started.elapsed();

        // Draw the text widgets on top
        let stage_started = Instant::now();
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
        debug_text += stage_started.elapsed();

        // Draw avatar preview widgets on top of text and below buttons.
        let stage_started = Instant::now();
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
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(&mut self.target, assets, entity, &self.draw2d);
            }
        }
        debug_avatars += stage_started.elapsed();

        let stage_started = Instant::now();
        for widget in self.profile_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(&mut self.target, assets, entity, &self.draw2d);
            }
        }
        debug_profiles += stage_started.elapsed();

        let stage_started = Instant::now();
        for widget in self.stat_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(
                    &mut self.target,
                    assets,
                    entity,
                    &self.draw2d,
                    self.animation_frame,
                );
            }
        }
        debug_stats += stage_started.elapsed();

        let stage_started = Instant::now();
        if let Some(screen) = assets.screens.get(&self.current_screen)
            && let Some(screen_widget) = &mut self.screen_widget
        {
            let foreground_screen = Self::screen_foreground_render_map(screen);
            if !foreground_screen.sectors.is_empty() {
                screen_widget.build(&foreground_screen, assets);
                screen_widget.draw_transparent(&foreground_screen, &self.server_time, assets);
                self.target.blend_into(0, 0, &screen_widget.buffer);
            }
        }
        debug_foreground += stage_started.elapsed();

        // Draw messages after the screen foreground so message widgets can be
        // placed as semi-transparent overlays inside the game widget.
        let stage_started = Instant::now();
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
        debug_messages += stage_started.elapsed();

        let stage_started = Instant::now();
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
        debug_inputs += stage_started.elapsed();

        // Draw the button widgets which support inventory / gear on top
        let stage_started = Instant::now();
        for id in screen_model::ordered_button_ids(&self.button_widgets) {
            let widget = self.button_widgets.get_mut(&id).unwrap();
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let button_stage_started = Instant::now();
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                let hovered = widget.rect.contains(Vec2::new(
                    self.cursor_pos.x as f32,
                    self.cursor_pos.y as f32,
                ));
                let resolved_command =
                    screen_model::resolved_widget_command(widget, assets, entity, &self.ui_state);
                let resolved_command = screen_model::contextual_open_close_command(
                    resolved_command,
                    self.hovered_item_id,
                    map,
                );
                debug_button_resolve += button_stage_started.elapsed();

                let button_stage_started = Instant::now();
                let command_state = resolved_command
                    .as_deref()
                    .map(|command| rules_ui::command_state(assets, entity, command));
                debug_button_state += button_stage_started.elapsed();

                let selected = self.activated_widgets.contains(&widget.id)
                    || (self.intent.trim().is_empty()
                        && screen_model::command_is_walk(resolved_command.as_deref()));
                let visual_state = screen_model::button_visual_state(
                    hovered,
                    selected,
                    self.pressed_widget == Some(widget.id),
                    command_state.as_ref(),
                );
                let button_stage_started = Instant::now();
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
                debug_button_draw += button_stage_started.elapsed();

                let button_stage_started = Instant::now();
                if let Some(state) = command_state {
                    if !state.enabled || state.cooldown_remaining > 0.0 {
                        Self::draw_command_state_overlay(
                            &mut self.target,
                            &self.draw2d,
                            widget.rect,
                            &state,
                            assets,
                            resolved_command.as_deref(),
                            visual_state,
                            widget.show_icon,
                            self.animation_frame,
                        );
                        if !widget.show_icon {
                            widget.draw_label(&mut self.target, assets, &self.draw2d, visual_state);
                        }
                    }
                }
                debug_button_overlay += button_stage_started.elapsed();
            }
        }
        debug_buttons += stage_started.elapsed();

        Self::draw_choice_widgets(
            &mut self.choice_widgets,
            self.choice_controller.open_id(),
            &mut self.target,
            assets,
            &self.draw2d,
            Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32),
            &self.ui_state,
            &self.widgets_to_hide,
        );

        let stage_started = Instant::now();
        self.draw_open_container_panel(map, assets);
        self.draw_inventory_panel(map, assets);
        self.draw_equipment_panel(map, assets);
        self.draw_actions_panel(map, assets);
        self.draw_preferences_panel(assets);
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
        self.draw_dragged_action_preview(map, assets);

        self.draw_world_texts(map, assets);
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
        self.draw_intent_cursor(map, assets, false);
        debug_misc += stage_started.elapsed();

        if debug_enabled {
            self.draw_debug_timing.record(ClientDrawDebugSample {
                total: debug_total_started.elapsed(),
                setup: debug_setup,
                game_prepare: debug_game_prepare,
                game_entities: debug_game_entities,
                game_render: debug_game_render,
                game_composite: debug_game_composite,
                screen_base: debug_screen_base,
                text: debug_text,
                avatars: debug_avatars,
                profiles: debug_profiles,
                stats: debug_stats,
                foreground: debug_foreground,
                messages: debug_messages,
                inputs: debug_inputs,
                buttons: debug_buttons,
                button_resolve: debug_button_resolve,
                button_state: debug_button_state,
                button_draw: debug_button_draw,
                button_overlay: debug_button_overlay,
                misc: debug_misc,
                particle_stats: scene_handler.particle_debug_stats(),
            });
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
            let dim = buffer.dim();
            self.target_offset = Vec2::new(
                ((dim.width - self.viewport.x) / 2).max(0),
                ((dim.height - self.viewport.y) / 2).max(0),
            );
            self.first_game_draw = false;
            buffer.copy_into(self.target_offset.x, self.target_offset.y, &self.target);
        }
    }

    /// Returns the first game widget rect in viewport coordinates.
    pub fn game_widget_rect(&self) -> Option<Rect> {
        self.game_widgets.values().next().map(|w| w.rect)
    }

    pub fn for_each_game_widget_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut GameWidget),
    {
        for widget in self.game_widgets.values_mut() {
            f(widget);
        }
    }

    /// Startup window size multiplier from `[viewport].window_scale` (default `1.0`).
    pub fn window_scale(&self) -> f32 {
        self.get_config_f32_default("viewport", "window_scale", 1.0)
            .max(0.1)
    }

    /// Returns the presentation transform from viewport coordinates into a surface size.
    /// Output is `(scale, offset_x, offset_y)`.
    pub fn presentation_transform_for_surface(&self, width: u32, height: u32) -> (f32, f32, f32) {
        if self.screen_responsive {
            return (1.0, 0.0, 0.0);
        }
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

    pub fn screen_is_responsive(&self) -> bool {
        self.screen_responsive
    }

    /// Update the logical presentation surface. Fixed screens retain their
    /// authored viewport; responsive screens adopt the surface and relayout.
    pub fn resize_surface(&mut self, width: u32, height: u32, assets: &Assets) {
        self.surface_viewport = Vec2::new(width.max(1) as i32, height.max(1) as i32);
        let desired = if self.screen_responsive {
            self.surface_viewport
        } else {
            self.reference_viewport
        };
        if desired == self.viewport {
            return;
        }

        self.viewport = desired;
        self.target = TheRGBABuffer::new(TheDim::sized(desired.x, desired.y));
        self.overlay = TheRGBABuffer::new(TheDim::sized(desired.x, desired.y));
        if let Some(screen_widget) = self.screen_widget.as_mut() {
            screen_widget.buffer = TheRGBABuffer::new(TheDim::sized(desired.x, desired.y));
        }
        self.relayout_active_screen(assets);
        self.first_game_draw = true;
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

        // Isometric 2D uses the native sprite compositor and the normal widget buffer.
        if widget.camera.is_isometric_2d() {
            return false;
        }

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
        widget.mouse_look_pitch = self.mouse_look.pitch;
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
        let messages = messages
            .into_iter()
            .filter(|m| m.4 != "look")
            .collect::<Vec<_>>();
        let w = width.max(1) as i32;
        let h = height.max(1) as i32;
        let dim = self.overlay.dim();
        if dim.width != w || dim.height != h {
            self.overlay = TheRGBABuffer::new(TheDim::sized(w, h));
        }
        self.overlay.fill([0, 0, 0, 0]);
        let say_config = self.config.clone();
        let say_fallback_color = self.messages_font_color;

        if let Some(leader) = screen_model::resolve_party_entity(map, Some("leader")) {
            self.intent = self
                .selected_slot_intent(map, assets)
                .or_else(|| self.selected_targeting_intent.clone())
                .unwrap_or_else(|| leader.get_attr_string("intent").unwrap_or_default());
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

            let leader = screen_model::resolve_party_entity(map, Some("leader"));
            for w in self.button_widgets.iter() {
                if screen_model::resolved_widget_intent_payload(w.1, assets, leader, &self.ui_state)
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

                let view = game.camera_d3.view_matrix_for_surface(gw as f32, gh as f32);
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
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(&mut self.overlay, assets, entity, &self.draw2d);
            }
        }

        for widget in self.profile_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(&mut self.overlay, assets, entity, &self.draw2d);
            }
        }

        for widget in self.stat_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                widget.update_draw(
                    &mut self.overlay,
                    assets,
                    entity,
                    &self.draw2d,
                    self.animation_frame,
                );
            }
        }

        if let Some(screen) = assets.screens.get(&self.current_screen)
            && let Some(screen_widget) = &mut self.screen_widget
        {
            let foreground_screen = Self::screen_foreground_render_map(screen);
            if !foreground_screen.sectors.is_empty() {
                screen_widget.build(&foreground_screen, assets);
                screen_widget.draw_transparent(&foreground_screen, &self.server_time, assets);
                self.overlay.blend_into(0, 0, &screen_widget.buffer);
            }
        }

        // Draw messages after the screen foreground in the direct SceneVM path too.
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

        for id in screen_model::ordered_button_ids(&self.button_widgets) {
            let widget = self.button_widgets.get_mut(&id).unwrap();
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
                let hovered = widget.rect.contains(Vec2::new(
                    self.cursor_pos.x as f32,
                    self.cursor_pos.y as f32,
                ));
                let resolved_command =
                    screen_model::resolved_widget_command(widget, assets, entity, &self.ui_state);
                let resolved_command = screen_model::contextual_open_close_command(
                    resolved_command,
                    self.hovered_item_id,
                    map,
                );
                let command_state = resolved_command
                    .as_deref()
                    .map(|command| rules_ui::command_state(assets, entity, command));
                let selected = self.activated_widgets.contains(&widget.id)
                    || (self.intent.trim().is_empty()
                        && screen_model::command_is_walk(resolved_command.as_deref()));
                let visual_state = screen_model::button_visual_state(
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
                            assets,
                            resolved_command.as_deref(),
                            visual_state,
                            widget.show_icon,
                            self.animation_frame,
                        );
                        if !widget.show_icon {
                            widget.draw_label(
                                &mut self.overlay,
                                assets,
                                &self.draw2d,
                                visual_state,
                            );
                        }
                    }
                }
            }
        }

        Self::draw_choice_widgets(
            &mut self.choice_widgets,
            self.choice_controller.open_id(),
            &mut self.overlay,
            assets,
            &self.draw2d,
            Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32),
            &self.ui_state,
            &self.widgets_to_hide,
        );

        std::mem::swap(&mut self.target, &mut self.overlay);
        self.draw_inventory_panel(map, assets);
        self.draw_actions_panel(map, assets);
        std::mem::swap(&mut self.target, &mut self.overlay);

        if self.dragging_started && self.dragging_item_id.is_some() {
            let dragged_item = self.find_dragged_item(map);
            if let Some(item) = dragged_item
                && let Some(Value::Source(source)) = item.attributes.get("source")
                && let Some(tile) = source.tile_from_tile_list(assets)
            {
                let index = tile.frame_index(self.animation_frame);
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
        self.draw_dragged_action_preview(map, assets);
        std::mem::swap(&mut self.target, &mut self.overlay);

        std::mem::swap(&mut self.target, &mut self.overlay);
        self.draw_world_texts(map, assets);
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

        self.draw_intent_cursor(map, assets, true);
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
        self.inventory_panel_open
            || self.equipment_panel_open
            || !self.profile_widgets.is_empty()
            || self.button_widgets.values().any(|widget| {
                widget.drag_drop
                    && (widget.inventory_index.is_some() || widget.equipped_slot.is_some())
            })
    }

    fn item_is_container(item: &crate::Item) -> bool {
        crate::world_interaction::item_is_container(item)
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
        if let Some(owner) = owner_entity_id {
            return crate::item_interaction::ItemRef {
                item: item_id,
                owner: Some(owner),
            }
            .resolve(map);
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

    fn toggle_inventory_panel(&mut self) {
        let should_open = !self.inventory_panel_open;
        self.close_floaters();
        self.inventory_panel_open = should_open;
        self.inventory_panel_page = 0;
        self.inventory_panel_position = None;
        self.inventory_panel_selected_item = None;
        self.inventory_panel_tabs = None;
        self.inventory_panel_sort = None;
    }

    fn close_inventory_panel(&mut self) {
        self.inventory_panel_open = false;
        self.inventory_panel_rect = None;
        self.inventory_panel_title_rect = None;
        self.inventory_panel_close_rect = None;
        self.inventory_panel_previous_page_rect = None;
        self.inventory_panel_next_page_rect = None;
        self.inventory_panel_slots.clear();
        self.inventory_panel_tabs = None;
        self.inventory_panel_sort = None;
        self.dragging_inventory_panel = false;
        self.dragging_inventory_panel_item = false;
        self.inventory_panel_page = 0;
        self.inventory_panel_selected_item = None;
    }

    fn toggle_equipment_panel(&mut self) {
        let should_open = !self.equipment_panel_open;
        self.close_floaters();
        self.equipment_panel_open = should_open;
        self.equipment_panel_position = None;
    }

    fn close_equipment_panel(&mut self) {
        self.equipment_panel_open = false;
        self.equipment_panel_rect = None;
        self.equipment_panel_title_rect = None;
        self.equipment_panel_close_rect = None;
        self.equipment_panel_avatar_rect = None;
        self.equipment_panel_slots.clear();
        self.dragging_equipment_panel_item = false;
        self.dragging_equipment_panel = false;
    }

    fn toggle_preferences_panel(&mut self) {
        let should_open = !self.preferences_panel_open;
        self.close_floaters();
        self.preferences_panel_open = should_open;
        self.preferences_tooltips_choice = None;
        self.preferences_delay_choice = None;
    }

    fn close_preferences_panel(&mut self) {
        self.preferences_panel_open = false;
        self.preferences_panel_rect = None;
        self.preferences_panel_close_rect = None;
        self.preferences_reset_rect = None;
        self.preferences_tooltips_choice = None;
        self.preferences_delay_choice = None;
    }

    fn reset_floating_panel_positions(&mut self) {
        self.inventory_panel_position = None;
        self.equipment_panel_position = None;
        self.actions_panel_position = None;
        self.open_container_panel_positions.clear();
    }

    fn active_equipment_panel_config(&self) -> EquipmentPanelConfig {
        self.custom_equipment_panel_config
            .clone()
            .unwrap_or_else(|| self.toolbar_equipment_panel_config.clone())
    }

    fn equipment_slot_label(slot: &str) -> String {
        screen_model::equipment_slot_label(slot)
    }
    fn equipment_panel_layout(
        &mut self,
        map: &Map,
        assets: &Assets,
    ) -> Option<EquipmentPanelLayout> {
        if !self.equipment_panel_open {
            return None;
        }
        let actor = screen_model::resolve_party_entity(map, None)?;
        let layout = screen_model::equipment_panel_geometry(
            actor,
            &self.active_equipment_panel_config(),
            assets,
            self.equipment_panel_position,
            Vec2::new(self.target.dim().width, self.target.dim().height),
        );
        self.equipment_panel_position = Some(Vec2::new(
            layout.rect.x.round() as i32,
            layout.rect.y.round() as i32,
        ));
        Some(layout)
    }

    fn inventory_panel_choice(
        config: &InventoryPanelConfig,
        kind: ChoiceWidgetKind,
        id: u32,
        rect: Rect,
        binding: &str,
        options: Vec<ChoiceOption>,
    ) -> ChoiceWidget {
        let selected = options
            .iter()
            .position(|option| option.value == "all" || option.value == "newest")
            .unwrap_or(0);
        ChoiceWidget {
            name: if kind == ChoiceWidgetKind::TabBar {
                "Inventory Categories".to_string()
            } else {
                "Inventory Sort".to_string()
            },
            id,
            rect,
            kind,
            binding: binding.to_string(),
            options,
            selected,
            open: false,
            font: config.font.clone(),
            font_size: config.font_size,
            spacing: 1.0,
            text_padding: 8.0,
            item_height: config.footer_height,
            indicator_size: if kind == ChoiceWidgetKind::TabBar {
                2.0
            } else {
                0.0
            },
            equal_widths: false,
            open_upwards: true,
            background_color: if kind == ChoiceWidgetKind::TabBar {
                config.tab_background_color
            } else {
                config.dropdown_background_color
            },
            hover_color: [29, 33, 32, 245],
            selected_color: config.tab_selected_color,
            panel_color: config.dropdown_panel_color,
            border_color: config.border_color,
            text_color: config.text_color,
            muted_text_color: config.muted_text_color,
            indicator_color: config.selected_slot_color,
            border_size: 1,
        }
    }

    fn inventory_panel_layout(&mut self, map: &Map) -> Option<InventoryPanelLayout> {
        if !self.inventory_panel_open {
            return None;
        }
        let actor = screen_model::resolve_party_entity(map, None)?;
        let geometry = screen_model::inventory_panel_geometry(
            actor,
            &self.active_inventory_panel_config(),
            self.ui_state
                .get("inventory.category")
                .map(String::as_str)
                .unwrap_or("all"),
            self.ui_state
                .get("inventory.sort")
                .map(String::as_str)
                .unwrap_or("newest"),
            self.inventory_panel_page,
            self.inventory_panel_position,
            Vec2::new(self.target.dim().width, self.target.dim().height),
        );
        self.inventory_panel_page = geometry.page;
        self.inventory_panel_position = Some(Vec2::new(
            geometry.rect.x.round() as i32,
            geometry.rect.y.round() as i32,
        ));
        Some(InventoryPanelLayout {
            rect: geometry.rect,
            title_rect: geometry.title_rect,
            close_rect: geometry.close_rect,
            tab_rect: geometry.tab_rect,
            sort_rect: geometry.sort_rect,
            capacity_rect: geometry.capacity_rect,
            previous_page_rect: geometry.previous_page_rect,
            next_page_rect: geometry.next_page_rect,
            page_rect: geometry.page_rect,
            slots: geometry.slots().collect(),
            page: geometry.page,
            page_count: geometry.page_count,
        })
    }

    fn toggle_catalog_panel(&mut self, content: CatalogPanelContent) {
        let should_open = !self.actions_panel_open || self.actions_panel_content != content;
        self.close_floaters();
        self.actions_panel_content = content;
        self.actions_panel_open = should_open;
        self.actions_panel_page = 0;
        self.actions_panel_position = None;
        self.actions_panel_tab = "all".to_string();
        self.actions_panel_selected_command = None;
        self.actions_panel_detail_scroll = 0.0;
    }

    fn close_actions_panel(&mut self) {
        self.actions_panel_open = false;
        self.actions_panel_rect = None;
        self.actions_panel_title_rect = None;
        self.actions_panel_close_rect = None;
        self.actions_panel_assign_rect = None;
        self.actions_panel_previous_page_rect = None;
        self.actions_panel_next_page_rect = None;
        self.actions_panel_scroll_track_rect = None;
        self.actions_panel_scroll_thumb_rect = None;
        self.actions_panel_page_count = 1;
        self.actions_panel_detail_rect = None;
        self.actions_panel_detail_scroll_track_rect = None;
        self.actions_panel_detail_scroll_thumb_rect = None;
        self.actions_panel_detail_scroll = 0.0;
        self.actions_panel_detail_scroll_max = 0.0;
        self.dragging_actions_detail_scrollbar = false;
        self.actions_panel_tabs.clear();
        self.actions_panel_entries.clear();
        self.actions_assignment_mode = false;
        self.pending_action_assignment = None;
        self.dragging_action_command = None;
        self.dragging_actions_panel = false;
        self.actions_panel_page = 0;
        self.actions_panel_tab = "all".to_string();
        self.actions_panel_selected_command = None;
    }

    fn apply_ui_command(&mut self, command: &str) -> bool {
        match command.trim().to_ascii_lowercase().as_str() {
            "inventory" | "items" | "bag" => {
                self.toggle_inventory_panel();
                true
            }
            "equipment" | "gear" | "character" => {
                self.toggle_equipment_panel();
                true
            }
            "preferences" | "prefs" | "settings" => {
                self.toggle_preferences_panel();
                true
            }
            "actions" | "action_catalog" | "abilities" => {
                self.toggle_catalog_panel(CatalogPanelContent::Actions);
                true
            }
            "spellbook" | "spells" => {
                self.toggle_catalog_panel(CatalogPanelContent::Spellbook);
                true
            }
            _ => false,
        }
    }

    fn activate_actions_panel_command(
        &mut self,
        map: &Map,
        assets: &Assets,
        command: &str,
    ) -> Option<EntityAction> {
        let actor = screen_model::resolve_party_entity(map, None);
        let state = rules_ui::command_state(assets, actor, command);
        if !state.enabled {
            return None;
        }
        let Some(ClientCommandBinding::RulesAction(action_id)) = parse_client_command(command)
        else {
            return None;
        };
        let payload = format!("action:{}", action_id);
        self.intent = payload.clone();
        self.apply_intent_button_activation(&payload);
        self.immediate_2d_intent_mode()
            .then_some(EntityAction::Intent(payload))
    }

    fn close_floaters(&mut self) -> bool {
        let had_floater = self.open_container_panel.is_some()
            || self.actions_panel_open
            || self.inventory_panel_open
            || self.equipment_panel_open
            || self.preferences_panel_open
            || self.choice_controller.open_id().is_some();
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
        self.choice_controller.cancel(&mut self.choice_widgets);
        self.close_inventory_panel();
        self.close_equipment_panel();
        self.close_preferences_panel();
        self.close_actions_panel();
        self.tooltip_hover_key = None;
        self.tooltip_hover_since = None;
        had_floater
    }

    fn actions_panel_layout(&mut self, map: &Map, assets: &Assets) -> Option<ActionsPanelLayout> {
        if !self.actions_panel_open {
            return None;
        }
        let actor = screen_model::resolve_party_entity(map, None);
        let actor_class = actor.and_then(|actor| {
            actor
                .get_attr_string("class")
                .or_else(|| actor.get_attr_string("class_name"))
        });
        if self.actions_panel_catalog_rules != assets.rules
            || self.actions_panel_catalog_class != actor_class
        {
            self.actions_panel_catalog = rules_ui::action_catalog(assets, actor);
            self.actions_panel_catalog_rules.clone_from(&assets.rules);
            self.actions_panel_catalog_class = actor_class;
        }
        let layout = screen_model::catalog_panel_geometry(
            &self.actions_panel_catalog,
            &self.active_catalog_panel_config(),
            &self.actions_panel_tab,
            self.actions_panel_page,
            self.actions_panel_position,
            Vec2::new(
                self.target.dim().width as i32,
                self.target.dim().height as i32,
            ),
        );
        self.actions_panel_tab.clone_from(&layout.tab);
        self.actions_panel_page = layout.page;
        self.actions_panel_position = Some(Vec2::new(
            layout.rect.x.round() as i32,
            layout.rect.y.round() as i32,
        ));
        Some(layout)
    }

    fn open_container_item<'a>(&self, map: &'a Map) -> Option<&'a crate::Item> {
        let panel = self.open_container_panel?;
        let item = Self::find_container_item(map, panel.item_id, panel.owner_entity_id)?;
        Self::item_is_container(item).then_some(item)
    }

    fn container_panel_layout(&self, map: &Map, assets: &Assets) -> Option<ContainerPanelLayout> {
        let item = self.open_container_item(map)?;
        let template = rules_ui::container_template_for_item(assets, item);
        let slot_count = screen_model::container_slot_count(item);
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
        self.dragging_inventory_panel_item = false;
        self.dragging_equipment_panel_item = false;
        self.dragging_started = false;
        self.pressed_widget = None;
    }

    fn clear_action_drag(&mut self) {
        self.dragging_action_command = None;
        self.dragging_started = false;
        self.pressed_widget = None;
    }

    fn command_slot_at_point(&self, p: Vec2<i32>) -> Option<String> {
        let point = Vec2::new(p.x as f32, p.y as f32);
        self.button_widgets.values().find_map(|widget| {
            widget
                .rect
                .contains(point)
                .then(|| widget.command_slot.clone())
                .flatten()
        })
    }

    fn assign_pending_action_at_point(&mut self, p: Vec2<i32>) -> Option<EntityAction> {
        let command = self.pending_action_assignment.clone()?;
        let slot = self.command_slot_at_point(p)?;
        self.pending_action_assignment = None;
        self.actions_assignment_mode = false;
        Some(EntityAction::SetCommandSlot {
            slot,
            command: Some(command),
        })
    }

    fn build_container_panel_layout(
        position: Vec2<i32>,
        template: &ContainerUiTemplate,
        slot_count: usize,
        target_width: i32,
        target_height: i32,
    ) -> ContainerPanelLayout {
        let geometry = screen_model::ContainerGeometry::new(
            position,
            template,
            slot_count,
            Vec2::new(target_width, target_height),
        );
        ContainerPanelLayout {
            rect: geometry.rect,
            slots: geometry.slots().map(|(_, rect)| rect).collect(),
            title_bar_rect: geometry.title_bar,
            close_rect: geometry.close,
            title_rect: geometry.title.map(|rect| {
                (
                    rect.x as isize,
                    rect.y as isize,
                    rect.width as isize,
                    rect.height as isize,
                )
            }),
        }
    }

    fn item_click_distance(map: &Map, item: u32) -> f32 {
        crate::world_interaction::item_click_distance(map, item)
    }
    fn apply_world_click(
        &mut self,
        click: crate::world_interaction::WorldClick,
        coord: Vec2<i32>,
    ) -> Option<EntityAction> {
        use crate::world_interaction::WorldClick;
        match click {
            WorldClick::Action {
                action,
                uses_intent,
            } => {
                if uses_intent {
                    self.consume_one_shot_2d_intent();
                }
                Some(action)
            }
            WorldClick::BeginItemDrag(item) => {
                self.dragging_item_id = Some(item);
                self.dragging_item_owner_entity_id = None;
                self.dragging_item_from_world = true;
                self.drag_start_pos = self.screen_to_viewport(coord);
                None
            }
            WorldClick::OpenContainer(item) => {
                if self
                    .open_container_panel
                    .is_some_and(|panel| panel.item_id == item && panel.owner_entity_id.is_none())
                {
                    self.close_floaters();
                    None
                } else {
                    Some(EntityAction::OpenContainer {
                        item_id: item,
                        owner_entity_id: None,
                    })
                }
            }
        }
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

    fn move_catalog_panel_to_cursor(&mut self, p: Vec2<i32>) {
        let target_width = self.target.dim().width as i32;
        let target_height = self.target.dim().height as i32;
        self.actions_panel_position = Some(Vec2::new(
            (p.x - self.actions_panel_drag_offset.x).clamp(2, (target_width - 24).max(2)),
            (p.y - self.actions_panel_drag_offset.y).clamp(2, (target_height - 24).max(2)),
        ));
        self.actions_panel_rect = None;
        self.actions_panel_title_rect = None;
        self.actions_panel_close_rect = None;
        self.actions_panel_assign_rect = None;
        self.actions_panel_previous_page_rect = None;
        self.actions_panel_next_page_rect = None;
        self.actions_panel_tabs.clear();
        self.actions_panel_entries.clear();
        self.tooltip_hover_key = None;
        self.tooltip_hover_since = None;
    }

    fn move_equipment_panel_to_cursor(&mut self, p: Vec2<i32>) {
        let target_width = self.target.dim().width as i32;
        let target_height = self.target.dim().height as i32;
        self.equipment_panel_position = Some(Vec2::new(
            (p.x - self.equipment_panel_drag_offset.x).clamp(2, (target_width - 24).max(2)),
            (p.y - self.equipment_panel_drag_offset.y).clamp(2, (target_height - 24).max(2)),
        ));
        self.equipment_panel_rect = None;
        self.equipment_panel_title_rect = None;
        self.equipment_panel_close_rect = None;
        self.equipment_panel_slots.clear();
        self.tooltip_hover_key = None;
        self.tooltip_hover_since = None;
    }

    fn quantize_2d_tile_pos(pos: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(pos.x.floor(), pos.y.floor())
    }

    fn apply_active_intent_cursor(&mut self, entity_target: bool, item_target: bool) {
        let Some(pair) = screen_model::active_intent_cursors(
            &self.activated_widgets,
            &self.button_widgets,
            &self.intent,
        )
        .and_then(|cursors| cursors.target(entity_target, item_target)) else {
            return;
        };
        self.curr_intent_cursor = pair.normal;
        self.curr_clicked_intent_cursor = pair.pressed;
        if let Some(cursor) = pair.normal {
            self.curr_cursor = Some(cursor);
        }
    }

    /// Intent input contract:
    ///
    /// - Classic 2D selects and immediately emits an intent for directional use.
    /// - 3D, and 2D with `persistent_intents`, only select the intent. They must
    ///   not emit `EntityAction::Intent`; the later target interaction carries it.
    ///
    /// Every shortcut, screen button, and action-catalog selection must use this
    /// decision. See the `three_d_intent_*_selects_without_emitting_action`
    /// regression tests before changing this behavior.
    fn immediate_2d_intent_mode(&self) -> bool {
        Self::is_immediate_2d_intent_mode(
            self.active_player_camera.clone(),
            self.active_game_widget_camera_mode(),
            self.click_intents_2d,
        )
    }

    fn is_immediate_2d_intent_mode(
        player_camera: Option<PlayerCamera>,
        widget_camera: Option<PlayerCamera>,
        click_intents_2d: bool,
    ) -> bool {
        screen_model::immediate_2d_intents(player_camera, widget_camera, click_intents_2d)
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
        if !self.immediate_2d_intent_mode() {
            return;
        }
        self.intent.clear();
        self.selected_targeting_intent = None;
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

    fn activate_targeting_button(&mut self, button_id: u32) {
        screen_model::select_targeting_button(
            &self.button_widgets,
            &mut self.activated_widgets,
            &mut self.permanently_activated_widgets,
            button_id,
        );

        if let Some(widget) = self.button_widgets.get(&button_id) {
            self.curr_intent_cursor = widget.item_cursor_id;
            self.curr_clicked_intent_cursor = widget.item_clicked_cursor_id;
            self.curr_cursor = self.default_cursor;
        }
    }

    fn activate_walk_button(&mut self, button_id: u32) {
        self.intent.clear();
        self.selected_targeting_intent = Some(String::new());
        screen_model::select_targeting_button(
            &self.button_widgets,
            &mut self.activated_widgets,
            &mut self.permanently_activated_widgets,
            button_id,
        );

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
                let ground = widget.screen_to_ground(Vec2::new(dx, dy));
                let gx = ground.x;
                let gy = ground.y;
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

    fn refresh_3d_pick_at(
        &mut self,
        p: Vec2<i32>,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) -> bool {
        let point = Vec2::new(p.x as f32, p.y as f32);
        let Some(widget) = self.game_widgets.values().find(|widget| {
            widget.rect.contains(point)
                && (!Self::is_2d_camera(&widget.camera) || widget.camera.is_isometric_2d())
        }) else {
            return false;
        };

        self.hovered_entity_id = None;
        self.hovered_item_id = None;
        self.hovered_block_prop_target = None;
        self.hovered_world_pos = None;
        self.hover_distance = f32::MAX;

        let dx = p.x as f32 - widget.rect.x;
        let dy = p.y as f32 - widget.rect.y;
        if widget.camera.is_isometric_2d() {
            if let Some(projection) = widget.isometric_projection {
                let sprites = scene_handler.isometric_avatars(map, assets, self.animation_frame);
                self.hovered_item_id = crate::isometric::pick_item(
                    map,
                    assets,
                    projection,
                    Vec2::new(dx, dy),
                    &sprites,
                );
                if let Some(item) = self
                    .hovered_item_id
                    .and_then(|id| map.items.iter().find(|i| i.id == id))
                {
                    self.hovered_world_pos = Some(item.position);
                    self.hover_distance = Self::item_click_distance(map, item.id);
                }
            }
            return self.hovered_item_id.is_some();
        }
        let screen_uv = Vec2::new(dx / widget.rect.width, dy / widget.rect.height);
        let Some((geoid, world_pos, distance)) = scene_handler.vm.pick_geo_id_at_uv(
            widget.rect.width as u32,
            widget.rect.height as u32,
            [screen_uv.x, screen_uv.y],
            false,
            true,
        ) else {
            return true;
        };

        self.hovered_world_pos = Some(world_pos);
        self.hover_distance = distance;
        match geoid {
            GeoId::Character(entity_id) => {
                self.hovered_entity_id = Some(entity_id);
            }
            GeoId::Hole(sector_id, hole_id) => {
                if let Some(item) =
                    SceneHandler::find_item_by_profile_attrs(map, Some(sector_id), Some(hole_id))
                {
                    self.hovered_item_id = Some(item.id);
                }
            }
            GeoId::Sector(sector_id) => {
                if let Some(item) = SceneHandler::find_item_by_sector_id(map, sector_id) {
                    self.hovered_item_id = Some(item.id);
                }
            }
            GeoId::Item(item_id) => {
                self.hovered_item_id = Some(item_id);
            }
            GeoId::GeometryObject(object_id) => {
                if let Some(item) = SceneHandler::find_item_by_geometry_object_id(map, object_id) {
                    self.hovered_item_id = Some(item.id);
                }
                let paint_surface_id = scene_handler
                    .vm
                    .pick_paint_surface_at_uv(
                        widget.rect.width as u32,
                        widget.rect.height as u32,
                        [screen_uv.x, screen_uv.y],
                    )
                    .filter(|surface| surface.valid)
                    .map(|surface| surface.paint_geo);
                self.hovered_block_prop_target = resolve_block_prop_interaction_hit(
                    &map.block_prop_instances,
                    &assets.block_props,
                    object_id,
                    paint_surface_id,
                );
            }
            _ => {}
        }
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
        if self.dragging_actions_detail_scrollbar {
            if let (Some(track), Some(thumb)) = (
                self.actions_panel_detail_scroll_track_rect,
                self.actions_panel_detail_scroll_thumb_rect,
            ) {
                let travel = (track.height - thumb.height).max(1.0);
                let thumb_y = (p.y as f32 - self.actions_detail_scrollbar_drag_offset)
                    .clamp(track.y, track.y + travel);
                self.actions_panel_detail_scroll =
                    ((thumb_y - track.y) / travel) * self.actions_panel_detail_scroll_max;
            }
            return;
        }
        if self.dragging_inventory_panel {
            self.inventory_panel_position = Some(Vec2::new(
                p.x - self.inventory_panel_drag_offset.x,
                p.y - self.inventory_panel_drag_offset.y,
            ));
            return;
        }
        if self.dragging_equipment_panel {
            self.move_equipment_panel_to_cursor(p);
            return;
        }
        if self.dragging_actions_panel {
            self.move_catalog_panel_to_cursor(p);
            return;
        }
        if self.dragging_container_panel {
            self.move_open_container_panel_to_cursor(p);
            return;
        }
        if self.dragging_action_command.is_some() && !self.dragging_started {
            if self.drag_distance_exceeded(p) {
                self.dragging_started = true;
            }
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
    pub fn touch_hover(
        &mut self,
        coord: Vec2<i32>,
        map: &Map,
        assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
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
        self.hovered_block_prop_target = None;
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
        if self
            .actions_panel_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            return;
        }
        if self
            .equipment_panel_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
            || self
                .preferences_panel_rect
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
                if let Some(entity) =
                    screen_model::resolve_party_entity(map, widget.party.as_deref())
                {
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

        if self.button_widgets.values().any(|widget| {
            !screen_model::widget_name_is_hidden(&widget.name, &self.widgets_to_hide)
                && widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
        }) {
            return;
        }

        for (_, widget) in self.game_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                let dx = p.x as f32 - widget.rect.x;
                let dy = p.y as f32 - widget.rect.y;

                if Self::is_2d_camera(&widget.camera) {
                    if widget.camera.is_isometric_2d()
                        && let Some(projection) = widget.isometric_projection
                    {
                        let sprites =
                            scene_handler.isometric_avatars(map, assets, self.animation_frame);
                        if let Some(item) = crate::isometric::pick_item(
                            map,
                            assets,
                            projection,
                            Vec2::new(dx, dy),
                            &sprites,
                        ) {
                            self.hovered_item_id = Some(item);
                            pending_cursor_target = Some((false, true));
                            continue;
                        }
                    }
                    let ground = widget.screen_to_ground(Vec2::new(dx, dy));
                    let gx = ground.x;
                    let gy = ground.y;
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
                            GeoId::GeometryObject(object_id) => {
                                if let Some(item) =
                                    SceneHandler::find_item_by_geometry_object_id(map, object_id)
                                {
                                    self.hovered_item_id = Some(item.id);
                                }
                                let paint_surface_id = scene_handler
                                    .vm
                                    .pick_paint_surface_at_uv(
                                        widget.rect.width as u32,
                                        widget.rect.height as u32,
                                        [screen_uv.x, screen_uv.y],
                                    )
                                    .filter(|surface| surface.valid)
                                    .map(|surface| surface.paint_geo);
                                self.hovered_block_prop_target = resolve_block_prop_interaction_hit(
                                    &map.block_prop_instances,
                                    &assets.block_props,
                                    object_id,
                                    paint_surface_id,
                                );
                                if self.hovered_block_prop_target.is_some()
                                    || self.hovered_item_id.is_some()
                                {
                                    self.hover_distance = distance;
                                    pending_cursor_target = Some((false, true));
                                }
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
        scene_handler: &mut SceneHandler,
    ) -> Option<EntityAction> {
        let mut action = None;
        let mut camera_action = None;
        let mut render_camera_switches: Vec<(Option<String>, PlayerCamera)> = Vec::new();
        let mut selected_walk_button_id = None;
        let mut selected_targeting_button_id = None;
        let immediate_2d_intent = self.immediate_2d_intent_mode();
        let mut pending_ui_command = None;
        let mut bound_state_update: Option<(String, String, bool, String, Option<usize>)> = None;
        let active_intent = self.get_current_intent_for_action();
        self.dragging_item_id = None;
        self.dragging_item_owner_entity_id = None;
        self.dragging_source_widget_id = None;
        self.dragging_item_from_world = false;
        self.dragging_item_container_source = None;
        self.dragging_inventory_panel_item = false;
        self.dragging_equipment_panel_item = false;
        self.dragging_action_command = None;
        self.dragging_started = false;

        // Adjust cursor
        if self.curr_clicked_intent_cursor.is_some() {
            self.curr_cursor = self.curr_clicked_intent_cursor;
        } else {
            self.curr_cursor = self.default_cursor;
        }

        // Transform screen coordinates to viewport coordinates
        let p = self.screen_to_viewport(coord);
        let point = Vec2::new(p.x as f32, p.y as f32);
        if self.interact_world_text(point) {
            return None;
        }

        if self
            .preferences_tooltips_choice
            .as_ref()
            .is_some_and(|choice| choice.open)
        {
            let interaction = self
                .preferences_tooltips_choice
                .as_mut()
                .map(|choice| choice.interact(point))
                .unwrap_or(ChoiceInteraction::None);
            if let ChoiceInteraction::Selected(value) = interaction {
                self.tooltips_enabled = value.eq_ignore_ascii_case("on");
                self.tooltip_hover_key = None;
                self.tooltip_hover_since = None;
            }
            return None;
        }
        if self
            .preferences_delay_choice
            .as_ref()
            .is_some_and(|choice| choice.open)
        {
            let interaction = self
                .preferences_delay_choice
                .as_mut()
                .map(|choice| choice.interact(point))
                .unwrap_or(ChoiceInteraction::None);
            if let ChoiceInteraction::Selected(value) = interaction {
                self.tooltip_delay_ms = match value.as_str() {
                    "instant" => screen_model::TooltipDelay::Instant,
                    "short" => screen_model::TooltipDelay::Short,
                    _ => screen_model::TooltipDelay::Normal,
                }
                .milliseconds();
                self.tooltip_hover_key = None;
                self.tooltip_hover_since = None;
            }
            return None;
        }
        if self
            .preferences_panel_close_rect
            .is_some_and(|rect| rect.contains(point))
        {
            self.close_preferences_panel();
            return None;
        }
        if self
            .preferences_tooltips_choice
            .as_ref()
            .is_some_and(|choice| choice.contains_interactive_point(point))
        {
            if let Some(choice) = self.preferences_tooltips_choice.as_mut() {
                choice.interact(point);
            }
            return None;
        }
        if self
            .preferences_delay_choice
            .as_ref()
            .is_some_and(|choice| choice.contains_interactive_point(point))
        {
            if let Some(choice) = self.preferences_delay_choice.as_mut() {
                choice.interact(point);
            }
            return None;
        }
        if self
            .preferences_reset_rect
            .is_some_and(|rect| rect.contains(point))
        {
            self.reset_floating_panel_positions();
            return None;
        }
        if self
            .preferences_panel_rect
            .is_some_and(|rect| rect.contains(point))
        {
            return None;
        }

        if self
            .equipment_panel_close_rect
            .is_some_and(|rect| rect.contains(point))
        {
            self.close_equipment_panel();
            return None;
        }
        if let Some(title_rect) = self.equipment_panel_title_rect
            && title_rect.contains(point)
            && let Some(panel_rect) = self.equipment_panel_rect
        {
            self.dragging_equipment_panel = true;
            self.equipment_panel_drag_offset = Vec2::new(
                p.x - panel_rect.x.round() as i32,
                p.y - panel_rect.y.round() as i32,
            );
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return None;
        }
        if let Some(slot) = self
            .equipment_panel_slots
            .iter()
            .find(|slot| slot.rect.contains(point))
            .cloned()
        {
            if let Some(item_id) = slot.item_id
                && let Some(actor) = screen_model::resolve_party_entity(map, None)
            {
                self.dragging_item_id = Some(item_id);
                self.dragging_item_owner_entity_id = Some(actor.id);
                self.dragging_equipment_panel_item = true;
                self.drag_start_pos = p;
            }
            return None;
        }
        if self
            .equipment_panel_rect
            .is_some_and(|rect| rect.contains(point))
        {
            return None;
        }

        if self
            .inventory_panel_sort
            .as_ref()
            .is_some_and(|sort| sort.open)
        {
            let interaction = self
                .inventory_panel_sort
                .as_mut()
                .map(|sort| sort.interact(point))
                .unwrap_or(ChoiceInteraction::None);
            if let ChoiceInteraction::Selected(value) = interaction {
                self.set_ui_state("inventory.sort", &value);
                self.inventory_panel_page = 0;
            }
            return None;
        }
        if self
            .inventory_panel_close_rect
            .is_some_and(|rect| rect.contains(point))
        {
            self.close_inventory_panel();
            return None;
        }
        if self
            .inventory_panel_previous_page_rect
            .is_some_and(|rect| rect.contains(point))
        {
            self.inventory_panel_page = self.inventory_panel_page.saturating_sub(1);
            return None;
        }
        if self
            .inventory_panel_next_page_rect
            .is_some_and(|rect| rect.contains(point))
        {
            self.inventory_panel_page = self.inventory_panel_page.saturating_add(1);
            return None;
        }
        if self
            .inventory_panel_tabs
            .as_ref()
            .is_some_and(|tabs| tabs.contains_interactive_point(point))
        {
            let interaction = self
                .inventory_panel_tabs
                .as_mut()
                .map(|tabs| tabs.interact(point))
                .unwrap_or(ChoiceInteraction::None);
            if let ChoiceInteraction::Selected(value) = interaction {
                self.set_ui_state("inventory.category", &value);
                self.inventory_panel_page = 0;
                self.inventory_panel_selected_item = None;
            }
            return None;
        }
        if self
            .inventory_panel_sort
            .as_ref()
            .is_some_and(|sort| sort.contains_interactive_point(point))
        {
            if let Some(sort) = self.inventory_panel_sort.as_mut() {
                sort.interact(point);
            }
            return None;
        }
        if let Some(title_rect) = self.inventory_panel_title_rect
            && title_rect.contains(point)
            && let Some(panel_rect) = self.inventory_panel_rect
        {
            self.dragging_inventory_panel = true;
            self.inventory_panel_drag_offset = Vec2::new(
                p.x - panel_rect.x.round() as i32,
                p.y - panel_rect.y.round() as i32,
            );
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return None;
        }
        if let Some(slot) = self
            .inventory_panel_slots
            .iter()
            .find(|slot| slot.rect.contains(point))
            .cloned()
        {
            if let Some(item_id) = slot.item_id
                && let Some(actor) = screen_model::resolve_party_entity(map, None)
            {
                self.inventory_panel_selected_item = Some(item_id);
                self.dragging_item_id = Some(item_id);
                self.dragging_item_owner_entity_id = Some(actor.id);
                self.dragging_inventory_panel_item = true;
                self.drag_start_pos = p;
            }
            return None;
        }
        if self
            .inventory_panel_rect
            .is_some_and(|rect| rect.contains(point))
        {
            return None;
        }

        if self
            .actions_panel_close_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            self.close_actions_panel();
            return None;
        }
        if self
            .actions_panel_assign_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            self.actions_assignment_mode = !self.actions_assignment_mode;
            self.pending_action_assignment = None;
            return None;
        }
        if self
            .actions_panel_previous_page_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            self.actions_panel_page = self.actions_panel_page.saturating_sub(1);
            self.actions_panel_entries.clear();
            self.actions_panel_detail_scroll = 0.0;
            return None;
        }
        if self
            .actions_panel_next_page_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            self.actions_panel_page = self.actions_panel_page.saturating_add(1);
            self.actions_panel_entries.clear();
            self.actions_panel_detail_scroll = 0.0;
            return None;
        }
        if let Some(track) = self.actions_panel_scroll_track_rect
            && track.contains(point)
            && self.actions_panel_page_count > 1
        {
            let progress = ((point.y - track.y) / track.height.max(1.0)).clamp(0.0, 1.0);
            self.actions_panel_page = (progress
                * self.actions_panel_page_count.saturating_sub(1) as f32)
                .round() as usize;
            self.actions_panel_entries.clear();
            self.actions_panel_detail_scroll = 0.0;
            return None;
        }
        if let Some(track) = self.actions_panel_detail_scroll_track_rect
            && track.contains(point)
            && self.actions_panel_detail_scroll_max > 0.0
        {
            if let Some(thumb) = self.actions_panel_detail_scroll_thumb_rect
                && thumb.contains(point)
            {
                self.dragging_actions_detail_scrollbar = true;
                self.actions_detail_scrollbar_drag_offset = point.y - thumb.y;
            } else {
                let thumb_height = self
                    .actions_panel_detail_scroll_thumb_rect
                    .map(|thumb| thumb.height)
                    .unwrap_or(24.0);
                let travel = (track.height - thumb_height).max(1.0);
                let progress = ((point.y - track.y - thumb_height * 0.5) / travel).clamp(0.0, 1.0);
                self.actions_panel_detail_scroll = progress * self.actions_panel_detail_scroll_max;
            }
            return None;
        }
        if let Some(tab) = self
            .actions_panel_tabs
            .iter()
            .find(|tab| tab.rect.contains(Vec2::new(p.x as f32, p.y as f32)))
            .cloned()
        {
            self.actions_panel_tab = tab.id;
            self.actions_panel_page = 0;
            self.actions_panel_selected_command = None;
            self.actions_panel_entries.clear();
            self.actions_panel_detail_scroll = 0.0;
            return None;
        }
        if let Some(title_rect) = self.actions_panel_title_rect
            && title_rect.contains(Vec2::new(p.x as f32, p.y as f32))
            && let Some(panel_rect) = self.actions_panel_rect
        {
            self.dragging_actions_panel = true;
            self.actions_panel_drag_offset = Vec2::new(
                p.x - panel_rect.x.round() as i32,
                p.y - panel_rect.y.round() as i32,
            );
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return None;
        }
        if let Some(entry) = self
            .actions_panel_entries
            .iter()
            .find(|entry| entry.rect.contains(Vec2::new(p.x as f32, p.y as f32)))
            .cloned()
        {
            if self.actions_panel_selected_command.as_deref() != Some(entry.command.as_str()) {
                self.actions_panel_detail_scroll = 0.0;
            }
            self.actions_panel_selected_command = Some(entry.command.clone());
            if self.actions_assignment_mode {
                self.pending_action_assignment = Some(entry.command);
                return None;
            }
            self.dragging_action_command = Some(entry.command);
            self.drag_start_pos = p;
            return None;
        }
        if self
            .actions_panel_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            return None;
        }

        if let Some(action) = self.assign_pending_action_at_point(p) {
            return Some(action);
        }
        if self.actions_assignment_mode {
            return None;
        }

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

        if self.interact_choice_widgets(Vec2::new(p.x as f32, p.y as f32)) {
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
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

        // The game viewport can extend underneath the toolbar. Its buttons own
        // their clicks, including intent selectors which emit no server action.
        let hit_button_id =
            screen_model::topmost_button_at(&self.button_widgets, &self.widgets_to_hide, point);
        let hit_button = hit_button_id.is_some();

        if active_intent.is_some() && !hit_button {
            for widget in self.avatar_widgets.values() {
                let hidden = self.widgets_to_hide.iter().any(|pattern| {
                    if pattern.ends_with('*') {
                        let prefix = &pattern[..pattern.len() - 1];
                        widget.name.starts_with(prefix)
                    } else {
                        widget.name == *pattern
                    }
                });
                if hidden || !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    continue;
                }
                if let Some(entity) =
                    screen_model::resolve_party_entity(map, widget.party.as_deref())
                {
                    self.consume_one_shot_2d_intent();
                    return Some(EntityAction::EntityClicked(
                        entity.id,
                        0.0,
                        active_intent.clone(),
                    ));
                }
            }
            for widget in self.profile_widgets.values() {
                let hidden = self.widgets_to_hide.iter().any(|pattern| {
                    if pattern.ends_with('*') {
                        let prefix = &pattern[..pattern.len() - 1];
                        widget.name.starts_with(prefix)
                    } else {
                        widget.name == *pattern
                    }
                });
                if hidden || !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    continue;
                }
                // A profile may deliberately have a transparent button layered
                // over it (for example, to open the inventory screen). That UI
                // action takes precedence over using the profile as an intent
                // target. Drag/drop targeting is handled separately on release.
                if self
                    .button_widgets
                    .values()
                    .any(|button| button.rect.contains(Vec2::new(p.x as f32, p.y as f32)))
                {
                    continue;
                }
                if let Some(entity) =
                    screen_model::resolve_party_entity(map, widget.party.as_deref())
                {
                    self.consume_one_shot_2d_intent();
                    return Some(EntityAction::EntityClicked(
                        entity.id,
                        0.0,
                        active_intent.clone(),
                    ));
                }
            }
        }

        let clicked_3d_game_widget =
            !hit_button && self.refresh_3d_pick_at(p, map, assets, scene_handler);

        if clicked_3d_game_widget {
            let hit = crate::world_interaction::WorldHit {
                region: map.id,
                entity: self.hovered_entity_id,
                item: self.hovered_item_id,
                prop: self.hovered_block_prop_target,
                world: self.hovered_world_pos,
                distance: self.hover_distance,
                cell: None,
            };
            let intent = self.get_current_intent_for_action();
            if let Some(click) = crate::world_interaction::click(
                map,
                assets,
                hit,
                intent.as_deref(),
                self.has_drag_drop_targets(),
            ) {
                return self.apply_world_click(click, coord);
            }
        }

        for (id, widget) in self.button_widgets.iter() {
            if hit_button_id == Some(*id) {
                self.pressed_widget = Some(*id);
                if !self.activated_widgets.contains(id) {
                    self.activated_widgets.push(*id);
                }

                if let (Some(binding), Some(value)) =
                    (widget.binding.as_deref(), widget.value.as_deref())
                {
                    bound_state_update = Some((
                        binding.to_string(),
                        value.to_string(),
                        widget.binding_append,
                        widget.binding_separator.clone(),
                        widget.binding_max_parts,
                    ));
                }

                if widget.drag_drop {
                    if let Some(entity) =
                        screen_model::resolve_party_entity(map, widget.party.as_deref())
                    {
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
                let command_entity =
                    screen_model::resolve_party_entity(map, widget.party.as_deref());
                if let Some(binding) = screen_model::resolved_widget_binding(
                    widget,
                    assets,
                    command_entity,
                    &self.ui_state,
                ) {
                    match binding {
                        ClientCommandBinding::Control(act) => {
                            action = Some(act);
                        }
                        ClientCommandBinding::Intent(intent) => {
                            let payload = intent;
                            if payload.trim().is_empty() {
                                self.intent.clear();
                                self.selected_targeting_intent = Some(String::new());
                                selected_walk_button_id = Some(*id);
                            } else {
                                self.intent = payload.clone();
                                self.selected_targeting_intent = Some(payload.clone());
                                selected_targeting_button_id = Some(*id);
                                if immediate_2d_intent {
                                    action = Some(EntityAction::Intent(payload));
                                }
                            }
                        }
                        ClientCommandBinding::RulesAction(rules_action) => {
                            let payload = format!("action:{}", rules_action);
                            self.intent = payload.clone();
                            self.selected_targeting_intent = Some(payload.clone());
                            selected_targeting_button_id = Some(*id);
                            if immediate_2d_intent {
                                action = Some(EntityAction::Intent(payload));
                            }
                        }
                        ClientCommandBinding::Screen(_) | ClientCommandBinding::Game(_) => {
                            self.pending_runtime_commands.push(binding);
                        }
                        ClientCommandBinding::Ui(command) => {
                            pending_ui_command = Some(command);
                        }
                    }
                }

                screen_model::apply_button_visibility(widget, &mut self.widgets_to_hide);
                if let Some(entity) =
                    screen_model::resolve_party_entity(map, widget.party.as_deref())
                {
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

                screen_model::apply_button_deactivation(
                    &self.button_widgets,
                    &mut self.activated_widgets,
                    &mut self.permanently_activated_widgets,
                    *id,
                );
            }
        }
        if let Some(button_id) = selected_targeting_button_id {
            self.activate_targeting_button(button_id);
        }
        if let Some(button_id) = selected_walk_button_id {
            self.activate_walk_button(button_id);
        }
        if let Some((binding, value, append, separator, max_parts)) = bound_state_update {
            self.set_or_append_ui_state(&binding, &value, append, &separator, max_parts);
        }
        for (target, camera) in render_camera_switches {
            self.set_game_widget_camera_mode(target.as_deref(), camera);
        }

        if camera_action.is_some() {
            action = camera_action;
        }
        if let Some(command) = pending_ui_command {
            self.apply_ui_command(&command);
            return None;
        }

        // Test against clicks on interactive messages (multiple choice)
        if !hit_button && action.is_none() {
            for widget in self.messages_widgets.iter_mut() {
                if let Some(action) = widget.touch_down(p) {
                    return Some(action);
                }
            }
        }

        // Test against clicks on the map
        if !hit_button && action.is_none() {
            for (_, widget) in self.game_widgets.iter() {
                if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    if Self::is_2d_camera(&widget.camera) {
                        let dx = p.x as f32 - widget.rect.x;
                        let dy = p.y as f32 - widget.rect.y;

                        let ground = widget.screen_to_ground(Vec2::new(dx, dy));
                        let gx = ground.x;
                        let gy = ground.y;

                        let intent = self.get_current_intent_for_action();
                        let can_drag = self.has_drag_drop_targets();
                        if let Some(hit) = crate::world_interaction::cell_hit(
                            map,
                            Vec2::new(gx, gy),
                            can_drag && intent.is_none(),
                        ) {
                            if let Some(click) = crate::world_interaction::click(
                                map,
                                assets,
                                hit,
                                intent.as_deref(),
                                can_drag,
                            ) {
                                return self.apply_world_click(click, coord);
                            }
                        }
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
    pub fn touch_up(
        &mut self,
        coord: Vec2<i32>,
        map: &Map,
        assets: &Assets,
    ) -> Option<EntityAction> {
        let mut action = None;
        if self.dragging_actions_detail_scrollbar {
            self.dragging_actions_detail_scrollbar = false;
            self.pressed_widget = None;
            return None;
        }
        if self.dragging_actions_panel {
            self.dragging_actions_panel = false;
            self.pressed_widget = None;
            return None;
        }
        if self.dragging_container_panel {
            self.dragging_container_panel = false;
            self.pressed_widget = None;
            return None;
        }
        if self.dragging_inventory_panel {
            self.dragging_inventory_panel = false;
            self.pressed_widget = None;
            return None;
        }
        if self.dragging_equipment_panel {
            self.dragging_equipment_panel = false;
            self.pressed_widget = None;
            return None;
        }
        let p = self.screen_to_viewport(coord);
        if let Some(command) = self.dragging_action_command.clone() {
            let dragging_started = self.dragging_started || self.drag_distance_exceeded(p);
            action = if dragging_started {
                self.command_slot_at_point(p)
                    .map(|slot| EntityAction::SetCommandSlot {
                        slot,
                        command: Some(command),
                    })
            } else {
                self.activate_actions_panel_command(map, assets, &command)
            };
            self.clear_action_drag();
            self.activated_widgets = self.permanently_activated_widgets.clone();
            self.curr_cursor = self.default_cursor;
            for widget in self.messages_widgets.iter_mut() {
                widget.touch_up();
            }
            return action;
        }
        let dragged_item_id = self.dragging_item_id;
        let dragged_item_owner_entity_id = self.dragging_item_owner_entity_id;
        let dragged_source_widget_id = self.dragging_source_widget_id;
        let dragged_item_from_world = self.dragging_item_from_world;
        let dragged_container_source = self.dragging_item_container_source;
        let dragged_inventory_panel_item = self.dragging_inventory_panel_item;
        let dragged_equipment_panel_item = self.dragging_equipment_panel_item;
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
                        if self.open_container_panel.is_some_and(|panel| {
                            panel.item_id == item_id && panel.owner_entity_id.is_none()
                        }) {
                            self.close_floaters();
                        } else {
                            action = Some(EntityAction::OpenContainer {
                                item_id,
                                owner_entity_id: None,
                            });
                        }
                        self.dragging_item_id = None;
                        self.dragging_item_owner_entity_id = None;
                        self.dragging_source_widget_id = None;
                        self.dragging_item_from_world = false;
                        self.dragging_started = false;
                        return action;
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
                } else if dragged_inventory_panel_item {
                    if let Some(owner_id) = dragged_item_owner_entity_id
                        && let Some(item) = Self::find_container_item(map, item_id, Some(owner_id))
                        && Self::item_is_container(item)
                    {
                        let anchor = self
                            .inventory_panel_slots
                            .iter()
                            .find(|slot| slot.item_id == Some(item_id))
                            .map(|slot| slot.rect)
                            .unwrap_or(Rect::new(p.x as f32, p.y as f32, 1.0, 1.0));
                        self.toggle_container_panel(item_id, Some(owner_id), anchor);
                        self.clear_item_drag();
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
                } else if dragged_equipment_panel_item {
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
                if let Some(slot) = self
                    .inventory_panel_slots
                    .iter()
                    .find(|slot| slot.rect.contains(Vec2::new(p.x as f32, p.y as f32)))
                    && let Some(target_index) = slot.inventory_index
                {
                    let target_entity_id =
                        screen_model::resolve_party_entity(map, None).map(|entity| entity.id);
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
                }
                if action.is_none()
                    && let Some(target) = self
                        .equipment_panel_slots
                        .iter()
                        .find(|slot| slot.rect.contains(Vec2::new(p.x as f32, p.y as f32)))
                    && let Some(item) = self.find_dragged_item(map)
                    && item
                        .attributes
                        .get_str("slot")
                        .is_some_and(|slot| slot.trim().eq_ignore_ascii_case(&target.slot))
                {
                    let target_entity_id =
                        screen_model::resolve_party_entity(map, None).map(|entity| entity.id);
                    action = Some(if let Some(source) = dragged_container_source {
                        EntityAction::MoveContainerItem {
                            item_id,
                            container_item_id: source.container_item_id,
                            container_owner_entity_id: source.container_owner_entity_id,
                            target_entity_id,
                            to_inventory_index: None,
                            to_equipped_slot: Some(target.slot.clone()),
                        }
                    } else {
                        EntityAction::MoveItem {
                            item_id,
                            owner_entity_id: dragged_item_owner_entity_id,
                            target_entity_id,
                            to_inventory_index: None,
                            to_equipped_slot: Some(target.slot.clone()),
                        }
                    });
                }
                if let Some(panel) = self.open_container_panel
                    && action.is_none()
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
                    let target_entity_id =
                        screen_model::resolve_party_entity(map, widget.party.as_deref())
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
                if action.is_none() {
                    for widget in self.profile_widgets.values() {
                        let hidden = self.widgets_to_hide.iter().any(|pattern| {
                            if pattern.ends_with('*') {
                                widget.name.starts_with(&pattern[..pattern.len() - 1])
                            } else {
                                widget.name == *pattern
                            }
                        });
                        if hidden || !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                            continue;
                        }
                        let target_entity_id =
                            screen_model::resolve_party_entity(map, widget.party.as_deref())
                                .map(|entity| entity.id);
                        action = Some(if let Some(source) = dragged_container_source {
                            EntityAction::MoveContainerItem {
                                item_id,
                                container_item_id: source.container_item_id,
                                container_owner_entity_id: source.container_owner_entity_id,
                                target_entity_id,
                                to_inventory_index: None,
                                to_equipped_slot: None,
                            }
                        } else {
                            EntityAction::MoveItem {
                                item_id,
                                owner_entity_id: dragged_item_owner_entity_id,
                                target_entity_id,
                                to_inventory_index: None,
                                to_equipped_slot: None,
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
        self.choice_controller.pointer_up();

        self.activated_widgets = self.permanently_activated_widgets.clone();

        // Reset cursor after click release. Hover logic applies intent cursors contextually.
        self.curr_cursor = self.default_cursor;

        for widget in self.messages_widgets.iter_mut() {
            widget.touch_up();
        }
        action.and_then(|action| crate::item_interaction::validate_transfer(map, action))
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

        if is_escape && !self.world_texts.is_empty() {
            self.world_texts.clear();
            return EntityAction::Off;
        }

        if is_escape && self.close_floaters() {
            return EntityAction::Off;
        }

        if self.choice_controller.open_id().is_some()
            && matches!(event.as_str(), "key_down" | "key_up")
        {
            if event == "key_down"
                && let Value::Str(key) = &value
            {
                let navigation = match key.trim().to_ascii_lowercase().as_str() {
                    "up" | "arrowup" => Some(ChoiceNavigation::Previous),
                    "down" | "arrowdown" => Some(ChoiceNavigation::Next),
                    "home" => Some(ChoiceNavigation::First),
                    "end" => Some(ChoiceNavigation::Last),
                    "enter" | "return" | "space" => Some(ChoiceNavigation::Accept),
                    "tab" | "escape" | "esc" => Some(ChoiceNavigation::Cancel),
                    _ => None,
                };
                if let Some(navigation) = navigation {
                    let outcome = self.choice_controller.navigate(
                        &mut self.choice_widgets,
                        &self.widgets_to_hide,
                        &self.ui_state,
                        navigation,
                    );
                    if let Some((binding, value)) = outcome.selection {
                        self.set_ui_state(&binding, &value);
                    }
                }
            }
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
                            let expired = self.choice_expired(choice);
                            let choice = if expired {
                                let (from, to, expires_at_tick, max_distance) =
                                    choice.session_meta();
                                Choice::Cancel(from, to, expires_at_tick, max_distance)
                            } else {
                                choice.clone()
                            };
                            if expired {
                                for widget in &mut self.messages_widgets {
                                    widget.dismiss_active_choices();
                                }
                            } else {
                                for widget in &mut self.messages_widgets {
                                    widget.select_active_choice_if_matches(c, &choice);
                                }
                            }
                            self.choice_map = None;
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

        if let Value::Str(key) = &value {
            let ui_command = self
                .client_action
                .lock()
                .ok()
                .and_then(|action| action.binding_for_key(key))
                .and_then(|binding| match binding {
                    ClientCommandBinding::Ui(command) => Some(command),
                    _ => None,
                });
            if let Some(command) = ui_command {
                if event == "key_down" {
                    self.apply_ui_command(&command);
                }
                return EntityAction::Off;
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
        self.client_action.lock().unwrap().set_isometric(
            self.active_player_camera
                .as_ref()
                .is_some_and(PlayerCamera::is_isometric_2d),
        );
        let mut action = self.client_action.lock().unwrap().user_event(event, value);

        if is_key_down {
            if let EntityAction::Intent(intent_name) = &action {
                self.apply_intent_button_activation(intent_name);
                if !immediate_2d_intent {
                    // In 3D (and persistent-target 2D), choosing an intent only
                    // changes targeting state. The server receives it with the
                    // later explicit target click/interact event.
                    action = EntityAction::Off;
                }
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
        let point = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);
        if self
            .actions_panel_detail_rect
            .is_some_and(|rect| rect.contains(point))
            && self.actions_panel_detail_scroll_max > 0.0
        {
            let steps = (delta_y.unsigned_abs() / 120).max(1) as f32;
            let delta = steps * 32.0;
            if delta_y > 0 {
                self.actions_panel_detail_scroll = (self.actions_panel_detail_scroll + delta)
                    .min(self.actions_panel_detail_scroll_max);
            } else if delta_y < 0 {
                self.actions_panel_detail_scroll =
                    (self.actions_panel_detail_scroll - delta).max(0.0);
            }
            return true;
        }
        if self
            .actions_panel_rect
            .is_some_and(|rect| rect.contains(point))
            && self.actions_panel_page_count > 1
        {
            if delta_y > 0 {
                self.actions_panel_page = (self.actions_panel_page + 1)
                    .min(self.actions_panel_page_count.saturating_sub(1));
            } else if delta_y < 0 {
                self.actions_panel_page = self.actions_panel_page.saturating_sub(1);
            }
            self.actions_panel_entries.clear();
            self.actions_panel_detail_scroll = 0.0;
            return true;
        }

        let mut handled = false;
        let cursor_pos = self.cursor_pos;
        for widget in self.messages_widgets.iter_mut() {
            handled |= widget.scroll_at(delta_y, Some(cursor_pos));
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
        self.selected_targeting_intent = Some(self.intent.clone());

        let Some(button_id) = selected_button_id else {
            return;
        };

        if intent_raw.is_empty() {
            self.activate_walk_button(button_id);
            return;
        }

        // Deactivate all other targeting buttons so shortcut intent is authoritative.
        for (id, widget) in self.button_widgets.iter() {
            if *id != button_id && screen_model::is_targeting_button(widget) {
                self.activated_widgets.retain(|x| x != id);
                self.permanently_activated_widgets.retain(|x| x != id);
            }
        }

        // Also process explicit deactivate names for non-intent companion buttons.
        if !deactivate_names.is_empty() {
            for widget_to_deactivate in &deactivate_names {
                for (id, widget) in self.button_widgets.iter() {
                    if screen_model::deactivate_matches(widget, widget_to_deactivate) {
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

    fn active_inventory_panel_config(&self) -> InventoryPanelConfig {
        self.custom_inventory_panel_config
            .clone()
            .unwrap_or_else(|| self.toolbar_inventory_panel_config.clone())
    }

    fn catalog_panel_font<'a>(assets: &'a Assets, name: &str) -> Option<&'a Font> {
        if !name.trim().is_empty()
            && let Some(font) = assets.fonts.get(name.trim())
        {
            return Some(font);
        }
        Widget::fallback_font()
    }

    fn active_catalog_panel_config(&self) -> CatalogPanelConfig {
        match self.actions_panel_content {
            CatalogPanelContent::Actions => self
                .custom_actions_panel_config
                .clone()
                .unwrap_or_else(|| self.toolbar_actions_panel_config.clone()),
            CatalogPanelContent::Spellbook => self
                .custom_spellbook_config
                .clone()
                .unwrap_or_else(|| self.toolbar_spellbook_config.clone()),
        }
    }

    fn relayout_action_bar(&mut self, sector_id: u32, creator_id: Uuid, rect: Rect, table: &Table) {
        if let Some(widget) = self.deco_widgets.get_mut(&creator_id) {
            widget.rect = rect;
            widget.separators = screen_model::action_bar_group_separators(rect, table);
            Self::resize_widget_buffer(&mut widget.buffer, rect);
        }
        let rects = screen_model::action_bar_slot_rects(rect, table);
        if let Some(ids) = self.action_bar_button_ids.get(&sector_id) {
            for (id, rect) in ids.iter().zip(rects) {
                if let Some(widget) = self.button_widgets.get_mut(id) {
                    widget.rect = rect;
                }
            }
        }
    }

    fn screen_layout(&self) -> ScreenLayout {
        ScreenLayout {
            screen_responsive: self.screen_responsive,
            reference_viewport: self.reference_viewport,
            viewport: self.viewport,
            grid_size: self.grid_size,
        }
    }

    fn resize_widget_buffer(buffer: &mut TheRGBABuffer, rect: Rect) {
        let width = rect.width.round().max(1.0) as i32;
        let height = rect.height.round().max(1.0) as i32;
        if buffer.dim().width != width || buffer.dim().height != height {
            *buffer = TheRGBABuffer::new(TheDim::sized(width, height));
        }
    }

    fn relayout_active_screen(&mut self, assets: &Assets) {
        if !self.screen_responsive {
            return;
        }
        let Some(screen) = assets.screens.get(&self.current_screen) else {
            return;
        };
        let resolved = screen_model::placements(screen, self.screen_layout());

        for ScreenPlacement {
            role,
            sector_id,
            creator_id,
            name,
            rect,
            table,
        } in resolved
        {
            match role.as_str() {
                "game" => {
                    if let Some(widget) = self.game_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                        widget.force_dynamics_rebuild = true;
                    }
                }
                "button" => {
                    if let Some(widget) = self.button_widgets.get_mut(&sector_id) {
                        widget.rect = rect;
                    }
                }
                "input" => {
                    if let Some(widget) = self.text_input_widgets.get_mut(&sector_id) {
                        widget.rect = rect;
                    }
                }
                "tab_bar" | "dropdown" => {
                    if let Some(widget) = self.choice_widgets.get_mut(&sector_id) {
                        widget.rect = rect;
                    }
                }
                "messages" => {
                    for widget in self
                        .messages_widgets
                        .iter_mut()
                        .filter(|widget| widget.name == name)
                    {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                    }
                }
                "avatar" => {
                    if let Some(widget) = self.avatar_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                    }
                }
                "profile" => {
                    if let Some(widget) = self.profile_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                    }
                }
                "stat" => {
                    if let Some(widget) = self.stat_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                    }
                }
                "text" => {
                    if let Some(widget) = self.text_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                    }
                }
                "deco" => {
                    if let Some(widget) = self.deco_widgets.get_mut(&creator_id) {
                        widget.rect = rect;
                        Self::resize_widget_buffer(&mut widget.buffer, rect);
                    }
                }
                "action_bar" => {
                    self.relayout_action_bar(sector_id, creator_id, rect, &table);
                }
                "spellbook" => {
                    if let Some(config) = self.custom_spellbook_config.as_mut() {
                        config.authored_rect = Some(rect);
                    }
                }
                "actions" => {
                    if let Some(config) = self.custom_actions_panel_config.as_mut() {
                        config.authored_rect = Some(rect);
                    }
                }
                "inventory" => {
                    if let Some(config) = self.custom_inventory_panel_config.as_mut() {
                        config.authored_rect = Some(rect);
                    }
                }
                "equipment" => {
                    if let Some(config) = self.custom_equipment_panel_config.as_mut() {
                        config.authored_rect = Some(rect);
                    }
                }
                _ => {}
            }
        }
    }

    // Init the screen
    pub fn init_screen(
        &mut self,
        screen_name: String,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.clear_screen_widgets();
        self.screen_responsive = assets.screen_is_responsive(&screen_name);
        self.viewport = if self.screen_responsive {
            self.surface_viewport
        } else {
            self.reference_viewport
        };
        let background = screen_model::screen_background(&self.config);
        let layout = self.screen_layout();
        // Only the explicit legacy adapter owns viewport/overlay/widget pixels.
        self.target = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));
        self.overlay = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));
        let mut screen_widget = ScreenWidget::new();
        screen_widget.buffer = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));
        screen_widget.background_color = background;
        self.screen_widget = Some(screen_widget);
        // Preserve the legacy raster map layer normalization until its retirement.
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

        let Some(screen) = assets.screens.get(&screen_name) else {
            return;
        };
        let model = ScreenElements::compile(
            screen,
            assets,
            layout,
            background,
            std::mem::take(&mut self.ui_state),
            self.animation_frame,
        );
        self.game_widgets = model.game_widgets;
        self.button_widgets = model.button_widgets;
        self.action_bar_button_ids = model.action_bar_button_ids;
        self.avatar_widgets = model.avatar_widgets;
        self.profile_widgets = model.profile_widgets;
        self.stat_widgets = model.stat_widgets;
        self.text_widgets = model.text_widgets;
        self.text_input_widgets = model.text_input_widgets;
        self.choice_widgets = model.choice_widgets;
        self.deco_widgets = model.deco_widgets;
        self.messages_widgets = model.messages_widgets;
        self.activated_widgets = model.activated_widgets;
        self.permanently_activated_widgets = model.permanently_activated_widgets;
        self.ui_state = model.ui_state;
        self.widgets_to_hide = model.widgets_to_hide;
        self.custom_inventory_panel_config = model.custom_inventory_panel_config;
        self.custom_equipment_panel_config = model.custom_equipment_panel_config;
        self.custom_actions_panel_config = model.custom_actions_panel_config;
        self.custom_spellbook_config = model.custom_spellbook_config;
        self.toolbar_inventory_panel_config = model.toolbar_inventory_panel_config;
        self.toolbar_equipment_panel_config = model.toolbar_equipment_panel_config;
        self.toolbar_preferences_panel_config = model.toolbar_preferences_panel_config;
        self.toolbar_actions_panel_config = model.toolbar_actions_panel_config;
        self.toolbar_spellbook_config = model.toolbar_spellbook_config;

        for widget in self.game_widgets.values_mut() {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
            if let Some(map) = assets.maps.get(&self.current_map) {
                widget.build(map, assets, scene_handler);
            }
        }
        for widget in &mut self.messages_widgets {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
            widget.init(assets);
        }
        for widget in self.text_widgets.values_mut() {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
            widget.init(assets);
        }
        for widget in self.avatar_widgets.values_mut() {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
        }
        for widget in self.profile_widgets.values_mut() {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
        }
        for widget in self.stat_widgets.values_mut() {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
        }
        for widget in self.deco_widgets.values_mut() {
            Self::resize_widget_buffer(&mut widget.buffer, widget.rect);
        }
    }

    /// Returns the intent of the currently activated button
    /// Slot buttons resolve from the selected entity's current rules/configuration.
    /// Keep their selection while waiting for a target; the server has not received it yet.
    fn selected_slot_intent(&self, map: &Map, assets: &Assets) -> Option<String> {
        self.activated_widgets.iter().rev().find_map(|id| {
            let widget = self.button_widgets.get(id)?;
            widget.command_slot.as_ref()?;
            let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
            screen_model::resolved_widget_intent_payload(widget, assets, entity, &self.ui_state)
                .filter(|intent| !intent.is_empty())
        })
    }

    fn get_current_intent(&self) -> Option<String> {
        screen_model::selected_intent(&self.activated_widgets, &self.button_widgets, &self.intent)
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

    fn draw_command_state_overlay(
        target: &mut TheRGBABuffer,
        draw2d: &Draw2D,
        rect: Rect,
        state: &CommandState,
        assets: &Assets,
        command: Option<&str>,
        visual_state: ButtonVisualState,
        show_icon: bool,
        animation_frame: usize,
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

        if show_icon
            && let Some(texture) =
                Widget::command_icon_texture(assets, command, visual_state, animation_frame)
            && Self::draw_alpha_masked_command_overlay(target, rect, texture, alpha)
        {
            return;
        }

        draw2d.blend_rect_safe(target.pixels_mut(), &r, stride, &[0, 0, 0, alpha], &safe);
    }

    fn draw_alpha_masked_command_overlay(
        target: &mut TheRGBABuffer,
        rect: Rect,
        texture: &crate::Texture,
        overlay_alpha: u8,
    ) -> bool {
        let stride = target.stride();
        let inset = (rect.width.min(rect.height) * 0.12).round().max(2.0);
        let dest_x = (rect.x + inset).round().max(0.0) as usize;
        let dest_y = (rect.y + inset).round().max(0.0) as usize;
        let dest_w = (rect.width - inset * 2.0).round().max(1.0) as usize;
        let dest_h = (rect.height - inset * 2.0).round().max(1.0) as usize;
        let x_ratio = texture.width as f32 / dest_w as f32;
        let y_ratio = texture.height as f32 / dest_h as f32;
        let frame = target.pixels_mut();
        let mut drew = false;

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

                let source_alpha = texture.data[s + 3] as u16;
                if source_alpha == 0 {
                    continue;
                }
                let alpha = ((overlay_alpha as u16 * source_alpha) / 255).min(255) as u8;
                if alpha == 0 {
                    continue;
                }
                let keep = 255_u16.saturating_sub(alpha as u16);
                frame[d] = ((frame[d] as u16 * keep) / 255) as u8;
                frame[d + 1] = ((frame[d + 1] as u16 * keep) / 255) as u8;
                frame[d + 2] = ((frame[d + 2] as u16 * keep) / 255) as u8;
                drew = true;
            }
        }

        drew
    }

    fn draw_drag_drop_highlights(&mut self, map: &Map) {
        if (self.dragging_started && self.dragging_action_command.is_some())
            || self.pending_action_assignment.is_some()
        {
            let point = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);
            if let Some(widget) = self
                .button_widgets
                .values()
                .find(|widget| widget.command_slot.is_some() && widget.rect.contains(point))
            {
                Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, widget.rect);
            }
            return;
        }
        if !self.dragging_started || self.dragging_item_id.is_none() {
            return;
        }
        let Some(item) = self.find_dragged_item(map).cloned() else {
            return;
        };
        let point = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);

        if let Some(slot) = self
            .inventory_panel_slots
            .iter()
            .find(|slot| slot.inventory_index.is_some() && slot.rect.contains(point))
        {
            Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, slot.rect);
            return;
        }

        if let Some(slot) = self
            .equipment_panel_slots
            .iter()
            .find(|slot| slot.rect.contains(point))
        {
            if item
                .attributes
                .get_str("slot")
                .is_some_and(|item_slot| item_slot.trim().eq_ignore_ascii_case(&slot.slot))
            {
                Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, slot.rect);
            }
            return;
        }

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

        for widget in self.profile_widgets.values() {
            let hidden = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    widget.name.starts_with(&pattern[..pattern.len() - 1])
                } else {
                    widget.name == *pattern
                }
            });
            if !hidden && widget.rect.contains(point) {
                Self::draw_drag_target_highlight(&mut self.target, &self.draw2d, widget.rect);
                return;
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

    fn draw_dragged_action_preview(&mut self, map: &Map, assets: &Assets) {
        if !self.dragging_started {
            return;
        }
        let Some(command) = self.dragging_action_command.as_deref() else {
            return;
        };
        let size = 42.0;
        let rect = Rect::new(
            self.cursor_pos.x as f32 - size * 0.5,
            self.cursor_pos.y as f32 - size * 0.5,
            size,
            size,
        );
        let actor = screen_model::resolve_party_entity(map, None);
        let mut icon = Widget::new();
        icon.rect = rect;
        icon.command = Some(command.to_string());
        icon.update_draw(
            &mut self.target,
            map,
            assets,
            actor,
            &self.draw2d,
            &self.animation_frame,
            ButtonVisualState::Selected,
            Some(command),
        );
        let stride = self.target.stride();
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
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

        if let Some(title_rect) = layout.title_rect {
            let font = if let Some(font) = self.messages_font.as_ref() {
                Some(font)
            } else {
                Widget::fallback_font()
            };
            if let Some(font) = font {
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

    fn draw_inventory_panel(&mut self, map: &Map, assets: &Assets) {
        let Some(layout) = self.inventory_panel_layout(map) else {
            self.inventory_panel_rect = None;
            self.inventory_panel_slots.clear();
            return;
        };
        let Some(actor) = screen_model::resolve_party_entity(map, None) else {
            self.close_inventory_panel();
            return;
        };
        let config = self.active_inventory_panel_config();
        self.inventory_panel_rect = Some(layout.rect);
        self.inventory_panel_title_rect = Some(layout.title_rect);
        self.inventory_panel_close_rect = Some(layout.close_rect);
        self.inventory_panel_previous_page_rect = layout.previous_page_rect;
        self.inventory_panel_next_page_rect = layout.next_page_rect;
        self.inventory_panel_slots = layout.slots.clone();

        let stride = self.target.stride();
        let safe = (
            0_isize,
            0_isize,
            self.target.dim().width as isize,
            self.target.dim().height as isize,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round() as isize,
                layout.rect.y.round() as isize,
                layout.rect.width.round().max(1.0) as isize,
                layout.rect.height.round().max(1.0) as isize,
            ),
            stride,
            &config.background_color,
            &safe,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round() as isize,
                layout.rect.y.round() as isize,
                layout.rect.width.round().max(1.0) as isize,
                config.title_height.round().max(1.0) as isize,
            ),
            stride,
            &config.title_background_color,
            &safe,
        );
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round().max(0.0) as usize,
                layout.rect.y.round().max(0.0) as usize,
                layout.rect.width.round().max(1.0) as usize,
                layout.rect.height.round().max(1.0) as usize,
            ),
            stride,
            &config.border_color,
            1,
        );

        let font = Self::catalog_panel_font(assets, &config.font);
        if let Some(font) = font {
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &(
                    layout.title_rect.x.round() as isize,
                    layout.title_rect.y.round() as isize,
                    layout.title_rect.width.round().max(1.0) as isize,
                    layout.title_rect.height.round().max(1.0) as isize,
                ),
                stride,
                font,
                config.title_font_size,
                &config.title,
                &config.text_color,
                draw2d::TheHorizontalAlign::Center,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }

        let close_hovered = layout.close_rect.contains(Vec2::new(
            self.cursor_pos.x as f32,
            self.cursor_pos.y as f32,
        ));
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.close_rect.x.round() as isize,
                layout.close_rect.y.round() as isize,
                layout.close_rect.width.round().max(1.0) as isize,
                layout.close_rect.height.round().max(1.0) as isize,
            ),
            stride,
            if close_hovered {
                &[70, 62, 47, 245]
            } else {
                &[25, 27, 27, 230]
            },
            &safe,
        );
        Self::draw_close_x(
            &self.draw2d,
            &mut self.target,
            layout.close_rect,
            if close_hovered {
                &config.text_color
            } else {
                &config.muted_text_color
            },
        );

        if self.inventory_panel_tabs.is_none() {
            self.inventory_panel_tabs = Some(Self::inventory_panel_choice(
                &config,
                ChoiceWidgetKind::TabBar,
                u32::MAX - 20,
                layout.tab_rect,
                "inventory.category",
                config.categories.clone(),
            ));
        }
        if let Some(tabs) = self.inventory_panel_tabs.as_mut() {
            tabs.rect = layout.tab_rect;
            tabs.sync_value(self.ui_state.get("inventory.category").map(String::as_str));
            tabs.draw(
                &mut self.target,
                assets,
                &self.draw2d,
                Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32),
            );
        }

        for slot in &layout.slots {
            let hovered = slot.rect.contains(Vec2::new(
                self.cursor_pos.x as f32,
                self.cursor_pos.y as f32,
            ));
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    slot.rect.x.round() as isize,
                    slot.rect.y.round() as isize,
                    slot.rect.width.round().max(1.0) as isize,
                    slot.rect.height.round().max(1.0) as isize,
                ),
                stride,
                if hovered {
                    &[33, 35, 31, 245]
                } else {
                    &config.slot_background_color
                },
                &safe,
            );
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    slot.rect.x.round().max(0.0) as usize,
                    slot.rect.y.round().max(0.0) as usize,
                    slot.rect.width.round().max(1.0) as usize,
                    slot.rect.height.round().max(1.0) as usize,
                ),
                stride,
                if slot.item_id == self.inventory_panel_selected_item {
                    &config.selected_slot_color
                } else {
                    &config.slot_border_color
                },
                1,
            );
            let Some(index) = slot.inventory_index else {
                continue;
            };
            let Some(item) = actor.inventory.get(index).and_then(|item| item.as_ref()) else {
                continue;
            };
            Widget::draw_item_icon(
                &mut self.target,
                slot.rect,
                assets,
                item,
                &self.draw2d,
                self.animation_frame,
            );
            if item.stack_quantity() > 1
                && let Some(font) = font
            {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        (slot.rect.x + 3.0).round() as isize,
                        (slot.rect.y + slot.rect.height - 18.0).round() as isize,
                        (slot.rect.width - 6.0).round().max(1.0) as isize,
                        16,
                    ),
                    stride,
                    font,
                    config.font_size.max(11.0),
                    &item.stack_quantity().to_string(),
                    &config.text_color,
                    draw2d::TheHorizontalAlign::Right,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }

        if let Some(font) = font {
            let occupied = actor.inventory.iter().filter(|item| item.is_some()).count();
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &(
                    layout.capacity_rect.x.round() as isize,
                    layout.capacity_rect.y.round() as isize,
                    layout.capacity_rect.width.round().max(1.0) as isize,
                    layout.capacity_rect.height.round().max(1.0) as isize,
                ),
                stride,
                font,
                config.font_size,
                &format!("{} / {}", occupied, actor.inventory.len()),
                &config.muted_text_color,
                draw2d::TheHorizontalAlign::Right,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
            if let (Some(previous), Some(next), Some(page_rect)) = (
                layout.previous_page_rect,
                layout.next_page_rect,
                layout.page_rect,
            ) {
                for (rect, label) in [(previous, "‹"), (next, "›")] {
                    self.draw2d.text_rect_blend_safe(
                        self.target.pixels_mut(),
                        &(
                            rect.x.round() as isize,
                            rect.y.round() as isize,
                            rect.width.round() as isize,
                            rect.height.round() as isize,
                        ),
                        stride,
                        font,
                        config.title_font_size,
                        label,
                        &config.text_color,
                        draw2d::TheHorizontalAlign::Center,
                        draw2d::TheVerticalAlign::Center,
                        &safe,
                    );
                }
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        page_rect.x.round() as isize,
                        page_rect.y.round() as isize,
                        page_rect.width.round() as isize,
                        page_rect.height.round() as isize,
                    ),
                    stride,
                    font,
                    config.font_size,
                    &format!("{} / {}", layout.page + 1, layout.page_count),
                    &config.muted_text_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }

        if self.inventory_panel_sort.is_none() {
            self.inventory_panel_sort = Some(Self::inventory_panel_choice(
                &config,
                ChoiceWidgetKind::Dropdown,
                u32::MAX - 21,
                layout.sort_rect,
                "inventory.sort",
                config.sort_options.clone(),
            ));
        }
        if let Some(sort) = self.inventory_panel_sort.as_mut() {
            sort.rect = layout.sort_rect;
            sort.item_height = config.footer_height;
            sort.sync_value(self.ui_state.get("inventory.sort").map(String::as_str));
            sort.draw(
                &mut self.target,
                assets,
                &self.draw2d,
                Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32),
            );
        }
    }

    fn draw_equipment_panel(&mut self, map: &Map, assets: &Assets) {
        let Some(layout) = self.equipment_panel_layout(map, assets) else {
            self.equipment_panel_rect = None;
            self.equipment_panel_slots.clear();
            return;
        };
        let Some(actor) = screen_model::resolve_party_entity(map, None) else {
            self.close_equipment_panel();
            return;
        };
        let config = self.active_equipment_panel_config();
        self.equipment_panel_rect = Some(layout.rect);
        self.equipment_panel_title_rect = Some(layout.title_rect);
        self.equipment_panel_close_rect = Some(layout.close_rect);
        self.equipment_panel_avatar_rect = Some(layout.avatar_rect);
        self.equipment_panel_slots = layout.slots.clone();
        let stride = self.target.stride();
        let safe = (
            0_isize,
            0_isize,
            self.target.dim().width as isize,
            self.target.dim().height as isize,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round() as isize,
                layout.rect.y.round() as isize,
                layout.rect.width.round().max(1.0) as isize,
                layout.rect.height.round().max(1.0) as isize,
            ),
            stride,
            &config.background_color,
            &safe,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.title_rect.x.round() as isize,
                layout.title_rect.y.round() as isize,
                layout.title_rect.width.round().max(1.0) as isize,
                layout.title_rect.height.round().max(1.0) as isize,
            ),
            stride,
            &config.title_background_color,
            &safe,
        );
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round().max(0.0) as usize,
                layout.rect.y.round().max(0.0) as usize,
                layout.rect.width.round().max(1.0) as usize,
                layout.rect.height.round().max(1.0) as usize,
            ),
            stride,
            &config.border_color,
            1,
        );
        let font = Self::catalog_panel_font(assets, &config.font);
        if let Some(font) = font {
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &(
                    layout.title_rect.x.round() as isize,
                    layout.title_rect.y.round() as isize,
                    layout.title_rect.width.round().max(1.0) as isize,
                    layout.title_rect.height.round().max(1.0) as isize,
                ),
                stride,
                font,
                config.title_font_size,
                &config.title,
                &config.text_color,
                draw2d::TheHorizontalAlign::Center,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }
        let close_hovered = layout.close_rect.contains(Vec2::new(
            self.cursor_pos.x as f32,
            self.cursor_pos.y as f32,
        ));
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.close_rect.x.round() as isize,
                layout.close_rect.y.round() as isize,
                layout.close_rect.width.round().max(1.0) as isize,
                layout.close_rect.height.round().max(1.0) as isize,
            ),
            stride,
            if close_hovered {
                &[70, 62, 47, 245]
            } else {
                &[25, 27, 27, 230]
            },
            &safe,
        );
        Self::draw_close_x(
            &self.draw2d,
            &mut self.target,
            layout.close_rect,
            if close_hovered {
                &config.text_color
            } else {
                &config.muted_text_color
            },
        );

        self.equipment_panel_avatar.rect = layout.avatar_rect;
        self.equipment_panel_avatar.border_size = 0;
        self.equipment_panel_avatar.show_weapons = true;
        Self::resize_widget_buffer(&mut self.equipment_panel_avatar.buffer, layout.avatar_rect);
        self.equipment_panel_avatar.update_draw(
            &mut self.target,
            assets,
            Some(actor),
            &self.draw2d,
        );

        let cursor = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);
        for slot in &layout.slots {
            let hovered = slot.rect.contains(cursor);
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    slot.rect.x.round() as isize,
                    slot.rect.y.round() as isize,
                    slot.rect.width.round().max(1.0) as isize,
                    slot.rect.height.round().max(1.0) as isize,
                ),
                stride,
                if hovered {
                    &[33, 35, 31, 245]
                } else {
                    &config.slot_background_color
                },
                &safe,
            );
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    slot.rect.x.round().max(0.0) as usize,
                    slot.rect.y.round().max(0.0) as usize,
                    slot.rect.width.round().max(1.0) as usize,
                    slot.rect.height.round().max(1.0) as usize,
                ),
                stride,
                if slot.item_id.is_some() {
                    &config.occupied_slot_color
                } else {
                    &config.slot_border_color
                },
                1,
            );
            if let Some(item) = actor.get_equipped_item(&slot.slot) {
                Widget::draw_item_icon(
                    &mut self.target,
                    slot.rect,
                    assets,
                    item,
                    &self.draw2d,
                    self.animation_frame,
                );
            }
            if let Some(font) = font {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        slot.label_rect.x.round() as isize,
                        slot.label_rect.y.round() as isize,
                        slot.label_rect.width.round().max(1.0) as isize,
                        slot.label_rect.height.round().max(1.0) as isize,
                    ),
                    stride,
                    font,
                    config.font_size,
                    &Self::equipment_slot_label(&slot.slot),
                    &config.muted_text_color,
                    if slot.rect.x < layout.avatar_rect.x {
                        draw2d::TheHorizontalAlign::Left
                    } else {
                        draw2d::TheHorizontalAlign::Right
                    },
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }
    }

    fn preferences_choice(
        config: &PreferencesPanelConfig,
        id: u32,
        rect: Rect,
        binding: &str,
        options: Vec<ChoiceOption>,
        selected: usize,
    ) -> ChoiceWidget {
        ChoiceWidget {
            name: binding.to_string(),
            id,
            rect,
            kind: ChoiceWidgetKind::Dropdown,
            binding: binding.to_string(),
            options,
            selected,
            open: false,
            font: config.font.clone(),
            font_size: config.font_size,
            spacing: 0.0,
            text_padding: 8.0,
            item_height: config.row_height,
            indicator_size: 0.0,
            equal_widths: false,
            open_upwards: false,
            background_color: [13, 16, 16, 245],
            hover_color: [29, 33, 32, 245],
            selected_color: [22, 29, 30, 245],
            panel_color: [8, 11, 11, 252],
            border_color: config.border_color,
            text_color: config.text_color,
            muted_text_color: config.muted_text_color,
            indicator_color: [190, 156, 91, 255],
            border_size: 1,
        }
    }

    fn draw_preferences_panel(&mut self, assets: &Assets) {
        if !self.preferences_panel_open {
            self.preferences_panel_rect = None;
            return;
        }
        let config = self.toolbar_preferences_panel_config.clone();
        let geometry = screen_model::preferences_panel_geometry(
            &config,
            None,
            Vec2::new(self.target.dim().width, self.target.dim().height),
        );
        let rect = geometry.rect;
        let (x, y) = (rect.x, rect.y);
        let close_rect = geometry.close;
        let tooltips_rect = geometry.tooltips;
        let delay_rect = geometry.delay;
        let reset_rect = geometry.reset;
        let label_width = geometry.labels[0].width + 8.;
        let first_y = geometry.labels[0].y;
        self.preferences_panel_rect = Some(rect);
        self.preferences_panel_close_rect = Some(close_rect);
        self.preferences_reset_rect = Some(reset_rect);
        let stride = self.target.stride();
        let safe = (
            0_isize,
            0_isize,
            self.target.dim().width as isize,
            self.target.dim().height as isize,
        );
        for (area, color) in [
            (rect, config.background_color),
            (
                Rect::new(x, y, config.width, config.title_height),
                config.title_background_color,
            ),
        ] {
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    area.x.round() as isize,
                    area.y.round() as isize,
                    area.width.round().max(1.0) as isize,
                    area.height.round().max(1.0) as isize,
                ),
                stride,
                &color,
                &safe,
            );
        }
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(
                rect.x.round().max(0.0) as usize,
                rect.y.round().max(0.0) as usize,
                rect.width.round().max(1.0) as usize,
                rect.height.round().max(1.0) as usize,
            ),
            stride,
            &config.border_color,
            1,
        );
        let font = Self::catalog_panel_font(assets, &config.font);
        if let Some(font) = font {
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &(
                    x as isize,
                    y as isize,
                    config.width as isize,
                    config.title_height as isize,
                ),
                stride,
                font,
                config.title_font_size,
                &config.title,
                &config.text_color,
                draw2d::TheHorizontalAlign::Center,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
            for (label, row_y) in [
                ("Tooltips", first_y),
                ("Tooltip delay", first_y + config.row_height + 4.0),
            ] {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        (x + config.padding).round() as isize,
                        row_y.round() as isize,
                        (label_width - 8.0).round() as isize,
                        config.row_height.round() as isize,
                    ),
                    stride,
                    font,
                    config.font_size,
                    label,
                    &config.muted_text_color,
                    draw2d::TheHorizontalAlign::Left,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
            let hovered = reset_rect.contains(Vec2::new(
                self.cursor_pos.x as f32,
                self.cursor_pos.y as f32,
            ));
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    reset_rect.x.round() as isize,
                    reset_rect.y.round() as isize,
                    reset_rect.width.round() as isize,
                    reset_rect.height.round() as isize,
                ),
                stride,
                if hovered {
                    &[29, 33, 32, 245]
                } else {
                    &[13, 16, 16, 245]
                },
                &safe,
            );
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &(
                    reset_rect.x.round() as isize,
                    reset_rect.y.round() as isize,
                    reset_rect.width.round() as isize,
                    reset_rect.height.round() as isize,
                ),
                stride,
                font,
                config.font_size,
                "Reset floating window positions",
                &config.text_color,
                draw2d::TheHorizontalAlign::Center,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }
        let close_hovered = close_rect.contains(Vec2::new(
            self.cursor_pos.x as f32,
            self.cursor_pos.y as f32,
        ));
        Self::draw_close_x(
            &self.draw2d,
            &mut self.target,
            close_rect,
            if close_hovered {
                &config.text_color
            } else {
                &config.muted_text_color
            },
        );

        if self.preferences_tooltips_choice.is_none() {
            self.preferences_tooltips_choice = Some(Self::preferences_choice(
                &config,
                u32::MAX - 30,
                tooltips_rect,
                "preferences.tooltips",
                vec![
                    ChoiceOption {
                        label: "On".into(),
                        value: "on".into(),
                    },
                    ChoiceOption {
                        label: "Off".into(),
                        value: "off".into(),
                    },
                ],
                usize::from(!self.tooltips_enabled),
            ));
        }
        if self.preferences_delay_choice.is_none() {
            let selected =
                match screen_model::TooltipDelay::from_milliseconds(self.tooltip_delay_ms) {
                    screen_model::TooltipDelay::Instant => 0,
                    screen_model::TooltipDelay::Short => 1,
                    screen_model::TooltipDelay::Normal => 2,
                };
            self.preferences_delay_choice = Some(Self::preferences_choice(
                &config,
                u32::MAX - 31,
                delay_rect,
                "preferences.tooltip_delay",
                vec![
                    ChoiceOption {
                        label: "Instant".into(),
                        value: "instant".into(),
                    },
                    ChoiceOption {
                        label: "Short".into(),
                        value: "short".into(),
                    },
                    ChoiceOption {
                        label: "Normal".into(),
                        value: "normal".into(),
                    },
                ],
                selected,
            ));
        }
        let cursor = Vec2::new(self.cursor_pos.x as f32, self.cursor_pos.y as f32);
        let tooltips_open = self
            .preferences_tooltips_choice
            .as_ref()
            .is_some_and(|choice| choice.open);
        if tooltips_open {
            if let Some(choice) = self.preferences_delay_choice.as_mut() {
                choice.rect = delay_rect;
                choice.draw(&mut self.target, assets, &self.draw2d, cursor);
            }
            if let Some(choice) = self.preferences_tooltips_choice.as_mut() {
                choice.rect = tooltips_rect;
                choice.draw(&mut self.target, assets, &self.draw2d, cursor);
            }
        } else {
            if let Some(choice) = self.preferences_tooltips_choice.as_mut() {
                choice.rect = tooltips_rect;
                choice.draw(&mut self.target, assets, &self.draw2d, cursor);
            }
            if let Some(choice) = self.preferences_delay_choice.as_mut() {
                choice.rect = delay_rect;
                choice.draw(&mut self.target, assets, &self.draw2d, cursor);
            }
        }
    }

    fn catalog_detail_rows(
        description: RulesDescription,
        state: &CommandState,
    ) -> Vec<CatalogDetailRow> {
        screen_model::catalog_detail_rows(description, state)
    }

    fn draw_actions_panel(&mut self, map: &Map, assets: &Assets) {
        let Some(layout) = self.actions_panel_layout(map, assets) else {
            self.actions_panel_rect = None;
            self.actions_panel_title_rect = None;
            self.actions_panel_close_rect = None;
            self.actions_panel_assign_rect = None;
            self.actions_panel_previous_page_rect = None;
            self.actions_panel_next_page_rect = None;
            self.actions_panel_scroll_track_rect = None;
            self.actions_panel_scroll_thumb_rect = None;
            self.actions_panel_page_count = 1;
            self.actions_panel_detail_rect = None;
            self.actions_panel_detail_scroll_track_rect = None;
            self.actions_panel_detail_scroll_thumb_rect = None;
            self.actions_panel_tabs.clear();
            self.actions_panel_entries.clear();
            return;
        };
        let config = self.active_catalog_panel_config();
        self.actions_panel_rect = Some(layout.rect);
        self.actions_panel_title_rect = Some(layout.title_rect);
        self.actions_panel_close_rect = Some(layout.close_rect);
        self.actions_panel_assign_rect = layout.assign_rect;
        self.actions_panel_previous_page_rect = layout.previous_page_rect;
        self.actions_panel_next_page_rect = layout.next_page_rect;
        self.actions_panel_scroll_track_rect = layout.scroll_track_rect;
        self.actions_panel_scroll_thumb_rect = layout.scroll_thumb_rect;
        self.actions_panel_page_count = layout.page_count;
        self.actions_panel_detail_rect = layout.detail_rect;
        self.actions_panel_detail_scroll_track_rect = None;
        self.actions_panel_detail_scroll_thumb_rect = None;
        self.actions_panel_detail_scroll_max = 0.0;
        self.actions_panel_tabs = layout.tabs.clone();
        self.actions_panel_entries = layout.entries.clone();

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
            &config.background_color,
            &safe,
        );
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.rect.x.round() as isize,
                layout.rect.y.round() as isize,
                layout.rect.width.round().max(1.0) as isize,
                config.title_height.round().max(1.0) as isize,
            ),
            stride,
            &config.title_background_color,
            &safe,
        );
        if let Some(texture) = &config.frame_texture {
            widget::blend_texture_layer(
                &mut self.target,
                layout.rect,
                &self.draw2d,
                texture,
                config.frame_slice,
            );
        }
        if config.border_size > 0 {
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    layout.rect.x.round() as usize,
                    layout.rect.y.round() as usize,
                    layout.rect.width.round().max(1.0) as usize,
                    layout.rect.height.round().max(1.0) as usize,
                ),
                stride,
                &config.border_color,
                config.border_size as usize,
            );
        }

        let actor = screen_model::resolve_party_entity(map, None);
        let title = if config.title.trim().is_empty() {
            self.actions_panel_content.title()
        } else {
            config.title.trim()
        };
        let font = Self::catalog_panel_font(assets, &config.font);
        let title_font_name = if config.title_font.trim().is_empty() {
            config.font.as_str()
        } else {
            config.title_font.as_str()
        };
        let title_font = Self::catalog_panel_font(assets, title_font_name);
        if let Some(title_font) = title_font {
            self.draw2d.text_rect_blend_safe(
                self.target.pixels_mut(),
                &(
                    layout.title_rect.x.round() as isize,
                    layout.title_rect.y.round() as isize,
                    layout.title_rect.width.round().max(1.0) as isize,
                    layout.title_rect.height.round().max(1.0) as isize,
                ),
                stride,
                title_font,
                config.title_font_size,
                title,
                &config.text_color,
                draw2d::TheHorizontalAlign::Center,
                draw2d::TheVerticalAlign::Center,
                &safe,
            );
        }

        let close_hovered = layout.close_rect.contains(Vec2::new(
            self.cursor_pos.x as f32,
            self.cursor_pos.y as f32,
        ));
        self.draw2d.blend_rect_safe(
            self.target.pixels_mut(),
            &(
                layout.close_rect.x.round() as isize,
                layout.close_rect.y.round() as isize,
                layout.close_rect.width.round().max(1.0) as isize,
                layout.close_rect.height.round().max(1.0) as isize,
            ),
            stride,
            if close_hovered {
                &[70, 78, 88, 245]
            } else {
                &[42, 47, 54, 230]
            },
            &safe,
        );
        self.draw2d.rect_outline_thickness(
            self.target.pixels_mut(),
            &(
                layout.close_rect.x.round() as usize,
                layout.close_rect.y.round() as usize,
                layout.close_rect.width.round().max(1.0) as usize,
                layout.close_rect.height.round().max(1.0) as usize,
            ),
            stride,
            if close_hovered {
                &[174, 179, 183, 255]
            } else {
                &[98, 105, 116, 255]
            },
            1,
        );
        Self::draw_close_x(
            &self.draw2d,
            &mut self.target,
            layout.close_rect,
            &config.text_color,
        );

        if let Some(assign_rect) = layout.assign_rect {
            let assign_hovered = assign_rect.contains(Vec2::new(
                self.cursor_pos.x as f32,
                self.cursor_pos.y as f32,
            ));
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    assign_rect.x.round() as isize,
                    assign_rect.y.round() as isize,
                    assign_rect.width.round().max(1.0) as isize,
                    assign_rect.height.round().max(1.0) as isize,
                ),
                stride,
                if self.actions_assignment_mode {
                    &[92, 78, 34, 245]
                } else if assign_hovered {
                    &[70, 78, 88, 245]
                } else {
                    &[42, 47, 54, 230]
                },
                &safe,
            );
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    assign_rect.x.round() as usize,
                    assign_rect.y.round() as usize,
                    assign_rect.width.round().max(1.0) as usize,
                    assign_rect.height.round().max(1.0) as usize,
                ),
                stride,
                if self.actions_assignment_mode {
                    &[255, 222, 116, 255]
                } else {
                    &config.border_color
                },
                1,
            );
            if let Some(font) = font {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        assign_rect.x.round() as isize,
                        assign_rect.y.round() as isize,
                        assign_rect.width.round().max(1.0) as isize,
                        assign_rect.height.round().max(1.0) as isize,
                    ),
                    stride,
                    font,
                    config.small_font_size,
                    "Assign",
                    &config.text_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }

        if let (Some(previous), Some(next), Some(label)) = (
            layout.previous_page_rect,
            layout.next_page_rect,
            layout.page_label_rect,
        ) {
            for (rect, enabled) in [
                (previous, layout.page > 0),
                (next, layout.page + 1 < layout.page_count),
            ] {
                self.draw2d.blend_rect_safe(
                    self.target.pixels_mut(),
                    &(
                        rect.x.round() as isize,
                        rect.y.round() as isize,
                        rect.width.round().max(1.0) as isize,
                        rect.height.round().max(1.0) as isize,
                    ),
                    stride,
                    if enabled {
                        &[42, 47, 54, 230]
                    } else {
                        &[24, 27, 31, 180]
                    },
                    &safe,
                );
                self.draw2d.rect_outline_thickness(
                    self.target.pixels_mut(),
                    &(
                        rect.x.round() as usize,
                        rect.y.round() as usize,
                        rect.width.round().max(1.0) as usize,
                        rect.height.round().max(1.0) as usize,
                    ),
                    stride,
                    &config.border_color,
                    1,
                );
            }
            if let Some(font) = font {
                for (rect, text) in [(previous, "<"), (next, ">")] {
                    self.draw2d.text_rect_blend_safe(
                        self.target.pixels_mut(),
                        &(
                            rect.x.round() as isize,
                            rect.y.round() as isize,
                            rect.width.round().max(1.0) as isize,
                            rect.height.round().max(1.0) as isize,
                        ),
                        stride,
                        font,
                        11.0,
                        text,
                        &config.text_color,
                        draw2d::TheHorizontalAlign::Center,
                        draw2d::TheVerticalAlign::Center,
                        &safe,
                    );
                }
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        label.x.round() as isize,
                        label.y.round() as isize,
                        label.width.round().max(1.0) as isize,
                        label.height.round().max(1.0) as isize,
                    ),
                    stride,
                    font,
                    10.0,
                    &format!("{}/{}", layout.page + 1, layout.page_count),
                    &config.muted_text_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }

        if let (Some(track), Some(thumb)) = (layout.scroll_track_rect, layout.scroll_thumb_rect) {
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    track.x.round() as isize,
                    track.y.round() as isize,
                    track.width.round().max(1.0) as isize,
                    track.height.round().max(1.0) as isize,
                ),
                stride,
                &[25, 27, 25, 220],
                &safe,
            );
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    thumb.x.round() as isize,
                    thumb.y.round() as isize,
                    thumb.width.round().max(1.0) as isize,
                    thumb.height.round().max(1.0) as isize,
                ),
                stride,
                &config.separator_color,
                &safe,
            );
        }

        for tab in &layout.tabs {
            let selected = tab.id.eq_ignore_ascii_case(&self.actions_panel_tab);
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    tab.rect.x.round() as isize,
                    tab.rect.y.round() as isize,
                    tab.rect.width.round().max(1.0) as isize,
                    tab.rect.height.round().max(1.0) as isize,
                ),
                stride,
                if selected {
                    &config.tab_selected_color
                } else {
                    &config.tab_background_color
                },
                &safe,
            );
            self.draw2d.rect_outline_thickness(
                self.target.pixels_mut(),
                &(
                    tab.rect.x.round() as usize,
                    tab.rect.y.round() as usize,
                    tab.rect.width.round().max(1.0) as usize,
                    tab.rect.height.round().max(1.0) as usize,
                ),
                stride,
                &config.separator_color,
                1,
            );
            if let Some(font) = font {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        tab.rect.x.round() as isize + 4,
                        tab.rect.y.round() as isize,
                        (tab.rect.width - 8.0).round().max(1.0) as isize,
                        tab.rect.height.round().max(1.0) as isize,
                    ),
                    stride,
                    font,
                    config.small_font_size,
                    &tab.name,
                    if selected {
                        &config.text_color
                    } else {
                        &config.muted_text_color
                    },
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }

        if let Some(font) = font {
            for group in &layout.groups {
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        group.title_rect.x.round() as isize,
                        group.title_rect.y.round() as isize,
                        group.title_rect.width.round().max(1.0) as isize,
                        group.title_rect.height.round().max(1.0) as isize,
                    ),
                    stride,
                    font,
                    config.font_size,
                    &group.name,
                    &config.muted_text_color,
                    draw2d::TheHorizontalAlign::Left,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
            if let Some(empty_rect) = layout.empty_rect {
                let message = match self.actions_panel_content {
                    CatalogPanelContent::Actions => "No actions available for this character.",
                    CatalogPanelContent::Spellbook => "No abilities available for this character.",
                };
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        empty_rect.x.round() as isize,
                        empty_rect.y.round() as isize,
                        empty_rect.width.round().max(1.0) as isize,
                        empty_rect.height.round().max(1.0) as isize,
                    ),
                    stride,
                    font,
                    config.font_size,
                    message,
                    &config.muted_text_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
            }
        }

        let visible_selected = self
            .actions_panel_selected_command
            .as_ref()
            .filter(|command| {
                layout
                    .entries
                    .iter()
                    .any(|entry| entry.command.eq_ignore_ascii_case(command))
            });
        let detail_command = visible_selected
            .cloned()
            .or_else(|| layout.entries.first().map(|entry| entry.command.clone()));
        if self.actions_panel_selected_command.as_deref() != detail_command.as_deref() {
            self.actions_panel_selected_command = detail_command.clone();
            self.actions_panel_detail_scroll = 0.0;
        }
        if let Some(detail_rect) = layout.detail_rect {
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    detail_rect.x.round() as isize,
                    detail_rect.y.round() as isize,
                    detail_rect.width.round().max(1.0) as isize,
                    detail_rect.height.round().max(1.0) as isize,
                ),
                stride,
                &config.detail_background_color,
                &safe,
            );
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    detail_rect.x.round() as isize,
                    detail_rect.y.round() as isize,
                    1,
                    detail_rect.height.round().max(1.0) as isize,
                ),
                stride,
                &config.separator_color,
                &safe,
            );
            if let (Some(command), Some(font), Some(title_font)) =
                (detail_command.as_deref(), font, title_font)
            {
                let description = rules_ui::describe_command(assets, actor, command);
                let state = rules_ui::command_state(assets, actor, command);
                let title = description.title.clone();
                let subtitle = description.subtitle.clone();
                let rows = Self::catalog_detail_rows(description, &state);
                let text_x = detail_rect.x + 12.0;
                let text_width = (detail_rect.width - 32.0).max(1.0);
                let mut text_y = detail_rect.y + 12.0;
                self.draw2d.text_rect_blend_safe(
                    self.target.pixels_mut(),
                    &(
                        text_x.round() as isize,
                        text_y.round() as isize,
                        text_width.round() as isize,
                        (config.title_font_size + 5.0).round() as isize,
                    ),
                    stride,
                    title_font,
                    (config.title_font_size - 2.0).max(8.0),
                    &title,
                    &config.text_color,
                    draw2d::TheHorizontalAlign::Left,
                    draw2d::TheVerticalAlign::Center,
                    &safe,
                );
                text_y += config.title_font_size + 7.0;
                if let Some(subtitle) = subtitle {
                    let subtitle = subtitle.to_ascii_uppercase();
                    self.draw2d.text_rect_blend_safe(
                        self.target.pixels_mut(),
                        &(
                            text_x.round() as isize,
                            text_y.round() as isize,
                            text_width.round() as isize,
                            (config.font_size + 4.0).round() as isize,
                        ),
                        stride,
                        font,
                        config.small_font_size,
                        &subtitle,
                        &[190, 156, 91, 255],
                        draw2d::TheHorizontalAlign::Left,
                        draw2d::TheVerticalAlign::Center,
                        &safe,
                    );
                    text_y += config.small_font_size + 8.0;
                }
                self.draw2d.blend_rect_safe(
                    self.target.pixels_mut(),
                    &(
                        text_x.round() as isize,
                        text_y.round() as isize,
                        text_width.round() as isize,
                        1,
                    ),
                    stride,
                    &config.separator_color,
                    &safe,
                );
                text_y += 10.0;

                let content_bottom = detail_rect.y + detail_rect.height - 10.0;
                let content_height = (content_bottom - text_y).max(1.0);
                let content_clip = (
                    text_x.round() as isize,
                    text_y.round() as isize,
                    text_width.round() as isize,
                    content_height.round() as isize,
                );
                let line_height = config.font_size + 4.0;
                let fact_label_width = (text_width * 0.36).clamp(58.0, 82.0);
                let fact_value_width = (text_width - fact_label_width - 6.0).max(1.0);
                let mut rendered =
                    Vec::<(CatalogDetailRowKind, Option<String>, String, f32)>::new();
                for row in rows {
                    let (width, before) = match row.kind {
                        CatalogDetailRowKind::Summary => (text_width, 0.0),
                        CatalogDetailRowKind::Section => (text_width, 8.0),
                        CatalogDetailRowKind::Fact => (fact_value_width, 2.0),
                        CatalogDetailRowKind::Effect => (text_width, 2.0),
                        CatalogDetailRowKind::Warning => (text_width, 8.0),
                    };
                    let wrapped = if row.kind == CatalogDetailRowKind::Section {
                        vec![row.text]
                    } else {
                        Self::wrap_tooltip_line(
                            &self.draw2d,
                            font,
                            if row.kind == CatalogDetailRowKind::Section {
                                config.small_font_size
                            } else {
                                config.font_size
                            },
                            &row.text,
                            width,
                        )
                    };
                    for (index, line) in wrapped.into_iter().enumerate() {
                        rendered.push((
                            row.kind,
                            (index == 0).then(|| row.label.clone()).flatten(),
                            line,
                            if index == 0 { before } else { 0.0 },
                        ));
                    }
                }
                let total_height = rendered
                    .iter()
                    .map(|(kind, _, _, before)| {
                        before
                            + if *kind == CatalogDetailRowKind::Section {
                                config.small_font_size + 5.0
                            } else {
                                line_height
                            }
                    })
                    .sum::<f32>();
                self.actions_panel_detail_scroll_max = (total_height - content_height).max(0.0);
                self.actions_panel_detail_scroll = self
                    .actions_panel_detail_scroll
                    .clamp(0.0, self.actions_panel_detail_scroll_max);

                let mut row_y = text_y - self.actions_panel_detail_scroll;
                for (kind, label, line, before) in rendered {
                    row_y += before;
                    let height = if kind == CatalogDetailRowKind::Section {
                        config.small_font_size + 5.0
                    } else {
                        line_height
                    };
                    if row_y + height >= text_y && row_y <= content_bottom {
                        let (color, size) = match kind {
                            CatalogDetailRowKind::Summary => {
                                (config.muted_text_color, config.font_size)
                            }
                            CatalogDetailRowKind::Section => {
                                ([190, 156, 91, 255], config.small_font_size)
                            }
                            CatalogDetailRowKind::Fact | CatalogDetailRowKind::Effect => {
                                (config.text_color, config.font_size)
                            }
                            CatalogDetailRowKind::Warning => {
                                ([220, 145, 108, 255], config.font_size)
                            }
                        };
                        if kind == CatalogDetailRowKind::Fact {
                            if let Some(label) = label.as_deref() {
                                self.draw2d.text_rect_blend_safe_clip(
                                    self.target.pixels_mut(),
                                    &(
                                        text_x.round() as isize,
                                        row_y.round() as isize,
                                        fact_label_width.round() as isize,
                                        height.round() as isize,
                                    ),
                                    stride,
                                    font,
                                    config.small_font_size,
                                    label,
                                    &config.muted_text_color,
                                    draw2d::TheHorizontalAlign::Left,
                                    draw2d::TheVerticalAlign::Center,
                                    &content_clip,
                                );
                            }
                            self.draw2d.text_rect_blend_safe_clip(
                                self.target.pixels_mut(),
                                &(
                                    (text_x + fact_label_width + 6.0).round() as isize,
                                    row_y.round() as isize,
                                    fact_value_width.round() as isize,
                                    height.round() as isize,
                                ),
                                stride,
                                font,
                                size,
                                &line,
                                &color,
                                draw2d::TheHorizontalAlign::Left,
                                draw2d::TheVerticalAlign::Center,
                                &content_clip,
                            );
                        } else {
                            self.draw2d.text_rect_blend_safe_clip(
                                self.target.pixels_mut(),
                                &(
                                    text_x.round() as isize,
                                    row_y.round() as isize,
                                    text_width.round() as isize,
                                    height.round() as isize,
                                ),
                                stride,
                                font,
                                size,
                                &line,
                                &color,
                                draw2d::TheHorizontalAlign::Left,
                                draw2d::TheVerticalAlign::Center,
                                &content_clip,
                            );
                        }
                    }
                    row_y += height;
                }

                if self.actions_panel_detail_scroll_max > 0.0 {
                    let track = Rect::new(
                        detail_rect.x + detail_rect.width - 8.0,
                        text_y,
                        4.0,
                        content_height,
                    );
                    let thumb_height = (content_height * content_height / total_height)
                        .clamp(24.0, content_height);
                    let travel = (content_height - thumb_height).max(0.0);
                    let progress = self.actions_panel_detail_scroll
                        / self.actions_panel_detail_scroll_max.max(1.0);
                    let thumb = Rect::new(
                        track.x,
                        track.y + travel * progress,
                        track.width,
                        thumb_height,
                    );
                    self.actions_panel_detail_scroll_track_rect = Some(track);
                    self.actions_panel_detail_scroll_thumb_rect = Some(thumb);
                    for (rect, color) in
                        [(track, [25, 27, 25, 220]), (thumb, config.separator_color)]
                    {
                        self.draw2d.blend_rect_safe(
                            self.target.pixels_mut(),
                            &(
                                rect.x.round() as isize,
                                rect.y.round() as isize,
                                rect.width.round().max(1.0) as isize,
                                rect.height.round().max(1.0) as isize,
                            ),
                            stride,
                            &color,
                            &safe,
                        );
                    }
                } else {
                    self.actions_panel_detail_scroll_track_rect = None;
                    self.actions_panel_detail_scroll_thumb_rect = None;
                }
            }
        } else {
            self.actions_panel_detail_scroll_track_rect = None;
            self.actions_panel_detail_scroll_thumb_rect = None;
            self.actions_panel_detail_scroll_max = 0.0;
        }

        for entry in &layout.entries {
            let state = rules_ui::command_state(assets, actor, &entry.command);
            let hovered = entry.rect.contains(Vec2::new(
                self.cursor_pos.x as f32,
                self.cursor_pos.y as f32,
            ));
            let selected = self
                .actions_panel_selected_command
                .as_deref()
                .is_some_and(|command| command.eq_ignore_ascii_case(&entry.command))
                || parse_client_command(&entry.command)
                    .and_then(|binding| binding.intent_payload())
                    .is_some_and(|payload| payload.eq_ignore_ascii_case(self.intent.trim()));
            let assignment_selected = self
                .pending_action_assignment
                .as_deref()
                .is_some_and(|command| command.eq_ignore_ascii_case(&entry.command));
            let visual_state = if !state.enabled {
                ButtonVisualState::Disabled
            } else if selected || assignment_selected {
                ButtonVisualState::Selected
            } else if hovered {
                ButtonVisualState::Hover
            } else {
                ButtonVisualState::Normal
            };
            let background = if !state.enabled {
                [24, 27, 31, 220]
            } else if selected || assignment_selected {
                [64, 68, 49, 245]
            } else if hovered {
                [48, 54, 62, 242]
            } else {
                config.slot_background_color
            };
            self.draw2d.blend_rect_safe(
                self.target.pixels_mut(),
                &(
                    entry.rect.x.round() as isize,
                    entry.rect.y.round() as isize,
                    entry.rect.width.round().max(1.0) as isize,
                    entry.rect.height.round().max(1.0) as isize,
                ),
                stride,
                &background,
                &safe,
            );
            if let Some(texture) = &config.slot_texture {
                widget::blend_texture_layer(
                    &mut self.target,
                    entry.rect,
                    &self.draw2d,
                    texture,
                    config.slot_slice,
                );
            }
            if config.slot_border_size > 0 {
                self.draw2d.rect_outline_thickness(
                    self.target.pixels_mut(),
                    &(
                        entry.rect.x.round() as usize,
                        entry.rect.y.round() as usize,
                        entry.rect.width.round().max(1.0) as usize,
                        entry.rect.height.round().max(1.0) as usize,
                    ),
                    stride,
                    if selected || assignment_selected {
                        &[238, 214, 118, 255]
                    } else if hovered && state.enabled {
                        &[174, 179, 183, 255]
                    } else {
                        &config.slot_border_color
                    },
                    config.slot_border_size as usize,
                );
            }

            let mut icon = Widget::new();
            icon.rect = entry.icon_rect;
            icon.command = Some(entry.command.clone());
            icon.update_draw(
                &mut self.target,
                map,
                assets,
                actor,
                &self.draw2d,
                &self.animation_frame,
                visual_state,
                Some(&entry.command),
            );
            // Disabled icons are already converted to a muted grayscale by Widget.
            // Only cooldowns need the additional masked darkening overlay here.
            if state.cooldown_remaining > 0.0 {
                Self::draw_command_state_overlay(
                    &mut self.target,
                    &self.draw2d,
                    entry.icon_rect,
                    &state,
                    assets,
                    Some(&entry.command),
                    visual_state,
                    true,
                    self.animation_frame,
                );
            }

            if config.show_names
                && let Some(font) = font
            {
                let lines = Self::wrap_tooltip_line(
                    &self.draw2d,
                    font,
                    10.0,
                    &entry.name,
                    entry.rect.width - 6.0,
                );
                for (line_index, line) in lines.into_iter().take(2).enumerate() {
                    self.draw2d.text_rect_blend_safe(
                        self.target.pixels_mut(),
                        &(
                            (entry.rect.x + 3.0).round() as isize,
                            (entry.rect.y + entry.rect.height - 19.0 + line_index as f32 * 10.0)
                                .round() as isize,
                            (entry.rect.width - 6.0).round().max(1.0) as isize,
                            10,
                        ),
                        stride,
                        font,
                        10.0,
                        &line,
                        if state.enabled {
                            &[214, 216, 209, 255]
                        } else {
                            &[118, 121, 124, 255]
                        },
                        draw2d::TheHorizontalAlign::Center,
                        draw2d::TheVerticalAlign::Center,
                        &safe,
                    );
                }
            }
        }
    }

    fn draw_container_template_tiles(
        &mut self,
        layout: &ContainerPanelLayout,
        template: &ContainerUiTemplate,
        assets: &Assets,
    ) {
        for (source, rect) in screen_model::container_frame_tiles(layout.rect, template) {
            self.draw_tile_reference(assets, source, rect);
        }
    }

    fn draw_tile_reference(&mut self, assets: &Assets, tile_ref: &str, rect: Rect) -> bool {
        let Some(tile) = Self::resolve_tile_reference(assets, tile_ref) else {
            return false;
        };
        let Some(texture) = tile.textures.get(tile.frame_index(self.animation_frame)) else {
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
        screen_model::resolve_tile_reference(assets, tile_ref)
    }

    fn draw_hover_tooltip(&mut self, map: &Map, assets: &Assets) {
        if !self.tooltips_enabled
            || self.dragging_started
            || self.dragging_item_id.is_some()
            || self.dragging_container_panel
        {
            self.tooltip_hover_key = None;
            self.tooltip_hover_since = None;
            return;
        }

        let Some((description, anchor, state, hover_key, _delay, prefer_below)) =
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

        let delay = Duration::from_millis(self.tooltip_delay_ms);
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

        let font = if let Some(font) = self.messages_font.as_ref() {
            Some(font)
        } else {
            Widget::fallback_font()
        };
        let Some(font) = font else {
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
        let point = Vec2::new(p.x as f32, p.y as f32);
        if let Some(slot) = self
            .equipment_panel_slots
            .iter()
            .find(|slot| slot.rect.contains(point))
            && let Some(actor) = screen_model::resolve_party_entity(map, None)
            && let Some(item) = actor.get_equipped_item(&slot.slot)
        {
            return Some((
                rules_ui::describe_item(item, assets),
                slot.rect,
                None,
                format!("equipment-panel:{}:{}", actor.id, item.id),
                Duration::from_millis(650),
                false,
            ));
        }
        if self
            .equipment_panel_rect
            .is_some_and(|rect| rect.contains(point))
            || self
                .preferences_panel_rect
                .is_some_and(|rect| rect.contains(point))
        {
            return None;
        }
        if let Some(slot) = self
            .inventory_panel_slots
            .iter()
            .find(|slot| slot.rect.contains(point))
            && let Some(index) = slot.inventory_index
            && let Some(actor) = screen_model::resolve_party_entity(map, None)
            && let Some(item) = actor.inventory.get(index).and_then(|item| item.as_ref())
        {
            return Some((
                rules_ui::describe_item(item, assets),
                slot.rect,
                None,
                format!("inventory-panel:{}:{}", actor.id, item.id),
                Duration::from_millis(650),
                false,
            ));
        }
        if self
            .inventory_panel_rect
            .is_some_and(|rect| rect.contains(point))
            || self
                .inventory_panel_sort
                .as_ref()
                .is_some_and(|sort| sort.open && sort.popup_rect().contains(point))
        {
            return None;
        }
        if let Some(entry) = self
            .actions_panel_entries
            .iter()
            .find(|entry| entry.rect.contains(point))
        {
            if self.active_catalog_panel_config().show_details {
                return None;
            }
            let actor = screen_model::resolve_party_entity(map, None);
            let description = rules_ui::describe_command(assets, actor, &entry.command);
            let state = rules_ui::command_state(assets, actor, &entry.command);
            return Some((
                description,
                entry.rect,
                Some(state),
                format!("actions-panel:{}", entry.command),
                Duration::from_millis(350),
                false,
            ));
        }
        if self
            .actions_panel_rect
            .is_some_and(|rect| rect.contains(Vec2::new(p.x as f32, p.y as f32)))
        {
            return None;
        }
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
                        rules_ui::describe_item(container_item, assets),
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
            let entity = screen_model::resolve_party_entity(map, widget.party.as_deref());
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
                        rules_ui::describe_item(item, assets),
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
                        rules_ui::describe_item(item, assets),
                        widget.rect,
                        None,
                        format!("equipped:{}:{}", widget.id, item.id),
                        Duration::from_millis(650),
                        false,
                    ));
                }
            }
            if let Some(command) =
                screen_model::resolved_widget_command(widget, assets, entity, &self.ui_state)
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

        None
    }

    fn rects_intersect(a: Rect, b: Rect) -> bool {
        a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mouse_look_tracks_scaled_viewport_hover_without_a_button() {
        let mut client = Client::default();
        client.target_offset = Vec2::new(100, 50);
        client.upscale_factor = 2.0;
        let mut widget = GameWidget::new();
        widget.set_camera_mode(PlayerCamera::D3FirstPMouse);
        widget.rect = Rect::new(0.0, 0.0, 200.0, 100.0);
        client.game_widgets.insert(Uuid::new_v4(), widget);
        let mut map = Map::default();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        map.entities.push(player);
        client.mouse_look_hover(Vec2::new(200, 100), &map);
        assert!(client.mouse_look.active);
        client.mouse_look.motion(Vec2::new(0.0, -10.0));
        let pitch = client.mouse_look.pitch;
        client.mouse_look_hover(Vec2::new(600, 100), &map);
        assert!(!client.mouse_look.active);
        client.mouse_look_hover(Vec2::new(200, 100), &map);
        assert!(client.mouse_look.active);
        assert_eq!(client.mouse_look.pitch, pitch);
    }
    use super::*;

    #[test]
    fn command_slots_use_the_ruleset_identity_default_and_allow_classless_rules() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Artificer"

            [classes.Artificer.action_bar]
            primary = ["tinker"]
        "#
        .to_string();
        let ui_state = FxHashMap::default();
        assert_eq!(
            screen_model::command_for_slot("primary.0", &assets, None, &ui_state).as_deref(),
            Some("rules.tinker")
        );

        assets.rules = "[actions.inspect]\nkind = \"interaction\"".to_string();
        assert_eq!(
            screen_model::command_for_slot("primary.0", &assets, None, &ui_state),
            None
        );
    }

    #[test]
    fn region_fallback_widget_fills_viewport() {
        let mut client = Client::new();
        client.current_map = "Start".to_string();
        client.viewport = Vec2::new(320, 180);
        client.grid_size = 24.0;

        let widget = client.region_fallback_widget();
        assert_eq!(widget.name, "Start");
        assert_eq!(widget.rect.x, 0.0);
        assert_eq!(widget.rect.y, 0.0);
        assert_eq!(widget.rect.width, 320.0);
        assert_eq!(widget.rect.height, 180.0);
        assert_eq!(widget.buffer.dim().width, 320);
        assert_eq!(widget.buffer.dim().height, 180);
        assert_eq!(widget.grid_size, 24.0);
    }

    #[test]
    fn responsive_game_widget_fills_surface() {
        let mut client = Client::new();
        client.screen_responsive = true;
        client.reference_viewport = Vec2::new(960, 540);
        client.viewport = Vec2::new(1280, 720);
        let table = "[ui]\nrole = \"game\"\n".parse::<toml::Table>().unwrap();

        let rect =
            client
                .screen_layout()
                .resolve(Rect::new(32.0, 46.0, 640.0, 352.0), "game", &table);

        assert_eq!(rect, Rect::new(0.0, 0.0, 1280.0, 720.0));
    }

    #[test]
    fn responsive_bottom_center_anchor_uses_surface_center_and_offsets() {
        let mut client = Client::new();
        client.screen_responsive = true;
        client.reference_viewport = Vec2::new(960, 540);
        client.viewport = Vec2::new(1280, 720);
        let table = r#"
            [ui]
            role = "button"

            [layout]
            anchor = "bottom_center"
            x = 0
            y = -20
        "#
        .parse::<toml::Table>()
        .unwrap();

        let rect =
            client
                .screen_layout()
                .resolve(Rect::new(400.0, 460.0, 160.0, 60.0), "button", &table);

        assert_eq!(rect, Rect::new(560.0, 640.0, 160.0, 60.0));
    }

    #[test]
    fn action_bar_auto_sizes_and_centers_generated_slots() {
        let table = r#"
            [ui]
            role = "action_bar"
            slot_size = 52
            spacing = 4
            padding = 8
            buttons = [
                { command = "intent.look" },
                { command_slot = "main.0" },
                { command = "ui.actions", label = "Actions", show_icon = false },
            ]
        "#
        .parse::<toml::Table>()
        .unwrap();

        let authored =
            screen_model::action_bar_authored_rect(Rect::new(100.0, 200.0, 32.0, 32.0), &table);
        assert_eq!(authored, Rect::new(100.0, 200.0, 180.0, 68.0));

        let slots = screen_model::action_bar_slot_rects(authored, &table);
        assert_eq!(slots.len(), 3);
        assert_eq!(slots[0], Rect::new(108.0, 208.0, 52.0, 52.0));
        assert_eq!(slots[1], Rect::new(164.0, 208.0, 52.0, 52.0));
        assert_eq!(slots[2], Rect::new(220.0, 208.0, 52.0, 52.0));

        let buttons = screen_model::action_bar_buttons(&table);
        assert_eq!(buttons[0].command.as_deref(), Some("intent.look"));
        assert_eq!(buttons[1].command_slot.as_deref(), Some("main.0"));
        assert_eq!(buttons[2].label, "Actions");
        assert_eq!(buttons[2].show_icon, Some(false));
    }

    #[test]
    fn nested_border_style_parses_gradient_and_keeps_flat_fallbacks() {
        let table = r##"
            border_size = 1
            border_color = "#112233"

            [border]
            size = 3
            radius = 4
            from = "#e5c477"
            to = "#5d3b18"
            direction = "diagonal"
        "##
        .parse::<toml::Table>()
        .unwrap();

        let (from, size, to, direction, radius) =
            screen_model::border_style_from_table(&table, [0, 0, 0, 255], 0);
        assert_eq!(from, [229, 196, 119, 255]);
        assert_eq!(size, 3);
        assert_eq!(to, Some([93, 59, 24, 255]));
        assert_eq!(direction, BorderGradientDirection::Diagonal);
        assert_eq!(radius, 4.0);
    }

    #[test]
    fn responsive_action_bar_anchors_the_auto_sized_group() {
        let mut client = Client::new();
        client.screen_responsive = true;
        client.reference_viewport = Vec2::new(960, 540);
        client.viewport = Vec2::new(1280, 720);
        let table = r#"
            [ui]
            role = "action_bar"
            slot_size = 52
            spacing = 4
            padding = 8
            buttons = ["intent.look", "intent.use", "ui.actions"]

            [layout]
            anchor = "bottom_center"
            x = 0
            y = -16
        "#
        .parse::<toml::Table>()
        .unwrap();

        let authored =
            screen_model::action_bar_authored_rect(Rect::new(0.0, 0.0, 32.0, 32.0), &table);
        let rect = client
            .screen_layout()
            .resolve(authored, "action_bar", &table);
        assert_eq!(rect, Rect::new(550.0, 636.0, 180.0, 68.0));
    }

    #[test]
    fn responsive_action_bar_defaults_to_bottom_center_without_layout_settings() {
        let mut client = Client::new();
        client.screen_responsive = true;
        client.reference_viewport = Vec2::new(960, 540);
        client.viewport = Vec2::new(1280, 720);
        let table = r#"
            [ui]
            role = "action_bar"
            slot_size = 52
            spacing = 4
            padding = 8
            buttons = ["intent.look", "intent.use", "ui.actions"]
        "#
        .parse::<toml::Table>()
        .unwrap();

        let authored =
            screen_model::action_bar_authored_rect(Rect::new(100.0, 400.0, 32.0, 32.0), &table);
        let rect = client
            .screen_layout()
            .resolve(authored, "action_bar", &table);
        assert_eq!(rect, Rect::new(550.0, 636.0, 180.0, 68.0));
    }

    #[test]
    fn responsive_action_bar_can_fill_the_bottom_edge() {
        let mut client = Client::new();
        client.screen_responsive = true;
        client.reference_viewport = Vec2::new(960, 540);
        client.viewport = Vec2::new(1280, 720);
        let table = r#"
            [ui]
            role = "action_bar"
            fill_width = true
            slot_size = 44
            padding = 5
            edge_padding = 14

            [[ui.groups]]
            align = "left"
            buttons = ["ui.spellbook"]

            [[ui.groups]]
            align = "right"
            buttons = ["intent.look"]

            [layout]
            anchor = "bottom_center"
            y = 0
        "#
        .parse::<toml::Table>()
        .unwrap();

        let authored =
            screen_model::action_bar_authored_rect(Rect::new(0.0, 0.0, 32.0, 32.0), &table);
        let rect = client
            .screen_layout()
            .resolve(authored, "action_bar", &table);
        assert_eq!(rect, Rect::new(0.0, 666.0, 1280.0, 54.0));
        assert_eq!(
            screen_model::action_bar_slot_rects(rect, &table),
            vec![
                Rect::new(14.0, 671.0, 44.0, 44.0),
                Rect::new(1222.0, 671.0, 44.0, 44.0),
            ]
        );
    }

    #[test]
    fn grouped_action_bar_keeps_navigation_center_and_actions_apart() {
        let table = r#"
            [ui]
            role = "action_bar"
            slot_size = 52
            spacing = 4
            padding = 8
            group_spacing = 18

            [[ui.groups]]
            align = "left"
            buttons = ["ui.spellbook", "ui.actions"]

            [[ui.groups]]
            align = "center"
            buttons = [{ command_slot = "main.0" }]

            [[ui.groups]]
            align = "right"
            buttons = ["intent.look", "intent.use"]
        "#
        .parse::<toml::Table>()
        .unwrap();

        let authored =
            screen_model::action_bar_authored_rect(Rect::new(100.0, 200.0, 20.0, 20.0), &table);
        assert_eq!(authored, Rect::new(100.0, 200.0, 320.0, 68.0));
        let slots = screen_model::action_bar_slot_rects(authored, &table);
        assert_eq!(slots.len(), 5);
        assert_eq!(slots[0], Rect::new(108.0, 208.0, 52.0, 52.0));
        assert_eq!(slots[1], Rect::new(164.0, 208.0, 52.0, 52.0));
        assert_eq!(slots[2], Rect::new(234.0, 208.0, 52.0, 52.0));
        assert_eq!(slots[3], Rect::new(304.0, 208.0, 52.0, 52.0));
        assert_eq!(slots[4], Rect::new(360.0, 208.0, 52.0, 52.0));
        assert_eq!(
            screen_model::action_bar_group_separators(authored, &table),
            vec![225.0, 295.0]
        );
    }

    #[test]
    fn fixed_screen_ignores_widget_anchor_settings() {
        let mut client = Client::new();
        client.screen_responsive = false;
        client.reference_viewport = Vec2::new(960, 540);
        client.viewport = Vec2::new(1280, 720);
        let table = r#"
            [layout]
            anchor = "bottom_right"
            x = -16
            y = -16
        "#
        .parse::<toml::Table>()
        .unwrap();
        let authored = Rect::new(20.0, 30.0, 80.0, 80.0);

        assert_eq!(
            client.screen_layout().resolve(authored, "button", &table),
            authored
        );
    }

    #[test]
    fn one_shot_intents_are_only_immediate_in_2d_without_click_targeting() {
        assert!(screen_model::immediate_2d_intent_camera(
            Some(PlayerCamera::D2),
            false
        ));
        assert!(screen_model::immediate_2d_intent_camera(
            Some(PlayerCamera::D2Grid),
            false
        ));
        assert!(!screen_model::immediate_2d_intent_camera(
            Some(PlayerCamera::D3FirstP),
            false
        ));
        assert!(!screen_model::immediate_2d_intent_camera(
            Some(PlayerCamera::D3FirstPGrid),
            false
        ));
        assert!(!screen_model::immediate_2d_intent_camera(
            Some(PlayerCamera::D2),
            true
        ));
        assert!(Client::is_immediate_2d_intent_mode(
            Some(PlayerCamera::D2),
            Some(PlayerCamera::D2Grid),
            false,
        ));
        assert!(!Client::is_immediate_2d_intent_mode(
            Some(PlayerCamera::D2),
            Some(PlayerCamera::D3FirstP),
            false,
        ));
        assert!(!Client::is_immediate_2d_intent_mode(
            Some(PlayerCamera::D3Iso),
            Some(PlayerCamera::D2),
            false,
        ));
    }

    #[test]
    fn keyboard_shortcuts_activate_world_intents_and_rules_action_buttons() {
        let mut assets = Assets::default();
        assets.entities.insert(
            "Player".into(),
            (
                String::new(),
                r#"
                    [input]
                    w = "control.forward"
                    u = "intent.use"
                    l = "intent.look"
                    t = "rules.basic_attack"
                "#
                .into(),
            ),
        );
        let mut client = Client::new();
        client
            .client_action
            .lock()
            .unwrap()
            .init("Player".into(), &assets);
        client.button_widgets.insert(
            1,
            Widget {
                name: "Intent Use".into(),
                id: 1,
                command: Some("intent.use".into()),
                ..Default::default()
            },
        );
        client.button_widgets.insert(
            2,
            Widget {
                name: "Intent Look".into(),
                id: 2,
                command: Some("intent.look".into()),
                ..Default::default()
            },
        );
        client.button_widgets.insert(
            3,
            Widget {
                name: "Command Slot 1".into(),
                id: 3,
                command: Some("rules.basic_attack".into()),
                ..Default::default()
            },
        );
        client.active_player_camera = Some(PlayerCamera::D2);

        assert_eq!(
            client.user_event("key_down".into(), Value::Str("u".into())),
            EntityAction::Intent("use".into())
        );
        assert_eq!(client.get_current_intent().as_deref(), Some("use"));
        assert!(client.activated_widgets.contains(&1));

        let _ = client.user_event("key_up".into(), Value::Str("u".into()));
        assert_eq!(
            client.user_event("key_down".into(), Value::Str("t".into())),
            EntityAction::Intent("action:basic_attack".into())
        );
        assert_eq!(
            client.get_current_intent().as_deref(),
            Some("action:basic_attack")
        );
        assert!(client.activated_widgets.contains(&3));
        assert!(!client.activated_widgets.contains(&1));

        let _ = client.user_event("key_up".into(), Value::Str("t".into()));
        assert_eq!(
            client.user_event("key_down".into(), Value::Str("w".into())),
            EntityAction::Forward
        );
        assert_eq!(client.get_current_intent().as_deref(), None);
    }

    #[test]
    fn three_d_intent_shortcut_selects_without_emitting_action() {
        let mut assets = Assets::default();
        assets.entities.insert(
            "Player".into(),
            (
                String::new(),
                r#"
                    [input]
                    u = "intent.use"
                "#
                .into(),
            ),
        );
        let mut d3_client = Client::new();
        d3_client
            .client_action
            .lock()
            .unwrap()
            .init("Player".into(), &assets);
        d3_client.button_widgets.insert(
            1,
            Widget {
                name: "Intent Use".into(),
                id: 1,
                command: Some("intent.use".into()),
                ..Default::default()
            },
        );
        d3_client.active_player_camera = Some(PlayerCamera::D3FirstP);

        assert_eq!(
            d3_client.user_event("key_down".into(), Value::Str("u".into())),
            EntityAction::Off
        );
        assert_eq!(d3_client.get_current_intent().as_deref(), Some("use"));
        assert!(d3_client.activated_widgets.contains(&1));
    }

    #[test]
    fn three_d_intent_button_selects_without_emitting_action() {
        let mut client = Client::new();
        client.active_player_camera = Some(PlayerCamera::D3FirstP);
        client.button_widgets.insert(
            1,
            Widget {
                name: "Intent Use".into(),
                id: 1,
                rect: Rect::new(0.0, 0.0, 64.0, 64.0),
                command: Some("intent.use".into()),
                ..Default::default()
            },
        );

        assert_eq!(
            client.touch_down(
                Vec2::new(16, 16),
                &Map::default(),
                &Assets::default(),
                &mut SceneHandler::default(),
            ),
            None
        );
        assert_eq!(client.get_current_intent().as_deref(), Some("use"));
        assert!(client.activated_widgets.contains(&1));
    }

    #[test]
    fn toolbar_buttons_consume_clicks_over_the_game_viewport() {
        let map = Map::default();
        let assets = Assets::default();
        let mut scene = SceneHandler::default();
        for camera in [
            PlayerCamera::D2,
            PlayerCamera::D2Iso,
            PlayerCamera::D3FirstP,
        ] {
            let mut client = Client::new();
            client.click_intents_2d = true;
            client.active_player_camera = Some(camera.clone());
            let mut game = GameWidget::new();
            game.set_camera_mode(camera.clone());
            game.rect = Rect::new(0., 0., 320., 240.);
            client.game_widgets.insert(Uuid::new_v4(), game);
            client.button_widgets.insert(
                1,
                Widget {
                    name: "Intent".into(),
                    id: 1,
                    rect: Rect::new(0., 0., 64., 64.),
                    ..Default::default()
                },
            );
            client.hovered_item_id = Some(986);
            client.touch_hover(Vec2::new(16, 16), &map, &assets, &mut scene);
            assert_eq!(client.hovered_item_id, None);
            for intent in ["open_close", "use", "look", ""] {
                client.button_widgets.get_mut(&1).unwrap().command =
                    Some(format!("intent.{intent}"));
                // Picking the world behind a button must not run, even when a
                // previous pointer position had an entity under it.
                client.hovered_entity_id = Some(987);
                assert_eq!(
                    client.touch_down(Vec2::new(16, 16), &map, &assets, &mut scene),
                    None,
                    "{camera:?}: {intent} clicked through to the map"
                );
                assert_eq!(
                    client.get_current_intent().as_deref(),
                    (!intent.is_empty()).then_some(intent)
                );
                assert_eq!(client.hovered_entity_id, Some(987));
                assert_eq!(client.touch_up(Vec2::new(16, 16), &map, &assets), None);
            }
            // A button without a command also owns its rectangle.
            client.button_widgets.get_mut(&1).unwrap().command = None;
            assert_eq!(
                client.touch_down(Vec2::new(16, 16), &map, &assets, &mut scene),
                None
            );
            if camera == PlayerCamera::D2 {
                // Hidden UI must not block genuine world clicks.
                client.widgets_to_hide.push("Intent".into());
                assert!(matches!(
                    client.touch_down(Vec2::new(16, 16), &map, &assets, &mut scene),
                    Some(EntityAction::TerrainClicked(_))
                ));
            }
        }
    }

    #[test]
    fn world_items_do_not_have_automatic_hover_descriptions() {
        let mut map = Map::default();
        let mut item = Item::new();
        item.id = 12;
        item.set_attribute("description", Value::Str("Hidden until Look.".into()));
        map.items.push(item);
        let mut client = Client::new();
        client.hovered_item_id = Some(12);
        assert!(client.hover_description(&map, &Assets::default()).is_none());
    }

    #[test]
    fn actions_shortcut_opens_ruleset_grouped_catalogue() {
        let mut assets = Assets::default();
        assets.entities.insert(
            "Player".into(),
            (
                String::new(),
                r#"
                    [input]
                    tab = "ui.actions"
                "#
                .into(),
            ),
        );
        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = [
                "rules.basic_attack",
                "rules.minor_heal",
                "rules.gather_herbs",
            ]

            [actions.basic_attack]
            name = "Basic Attack"
            kind = "attack"

            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"

            [actions.gather_herbs]
            name = "Gather Herbs"
            kind = "gather"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Cleric".into()));
        let mut map = Map::default();
        map.entities.push(player);

        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));
        client
            .client_action
            .lock()
            .unwrap()
            .init("Player".into(), &assets);

        assert_eq!(
            client.user_event("key_down".into(), Value::Str("tab".into())),
            EntityAction::Off
        );
        assert!(client.actions_panel_open);
        let layout = client
            .actions_panel_layout(&map, &assets)
            .expect("Actions panel should resolve the class catalogue");
        assert_eq!(
            layout
                .groups
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Combat", "Spells", "Utility"]
        );
        assert_eq!(layout.entries.len(), 3);

        let _ = client.user_event("key_up".into(), Value::Str("tab".into()));
        let _ = client.user_event("key_down".into(), Value::Str("tab".into()));
        assert!(!client.actions_panel_open);
    }

    #[test]
    fn spellbook_command_reuses_the_complete_ability_catalogue() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = ["rules.basic_attack", "rules.minor_heal"]

            [actions.basic_attack]
            name = "Basic Attack"
            kind = "attack"

            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Cleric".into()));
        let mut map = Map::default();
        map.entities.push(player);
        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));

        assert!(client.apply_ui_command("spellbook"));
        assert!(client.actions_panel_open);
        assert_eq!(client.actions_panel_content, CatalogPanelContent::Spellbook);
        let layout = client
            .actions_panel_layout(&map, &assets)
            .expect("Spellbook should use the shared catalogue panel");
        assert_eq!(layout.groups.len(), 2);
        assert_eq!(layout.groups[0].name, "Combat");
        assert_eq!(layout.groups[1].name, "Spells");
        assert_eq!(layout.entries.len(), 2);
        assert_eq!(layout.entries[0].command, "rules.basic_attack");
        assert_eq!(layout.entries[1].command, "rules.minor_heal");

        client.actions_panel_tab = "spells".into();
        let spells = client.actions_panel_layout(&map, &assets).unwrap();
        assert_eq!(spells.groups.len(), 1);
        assert_eq!(spells.groups[0].name, "Spells");
        assert_eq!(spells.entries.len(), 1);
        assert_eq!(spells.entries[0].command, "rules.minor_heal");

        assert!(client.apply_ui_command("actions"));
        assert_eq!(client.actions_panel_content, CatalogPanelContent::Actions);
        let layout = client.actions_panel_layout(&map, &assets).unwrap();
        assert_eq!(layout.entries.len(), 2);
    }

    #[test]
    fn toolbar_spellbook_config_controls_grid_and_pages() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = ["rules.one", "rules.two", "rules.three"]

            [actions.one]
            name = "One"
            kind = "spell"

            [actions.two]
            name = "Two"
            kind = "spell"

            [actions.three]
            name = "Three"
            kind = "spell"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Cleric".into()));
        let mut map = Map::default();
        map.entities.push(player);
        let table = r#"
            columns = 2
            rows = 1
            cell_size = 44
            spacing = 3
            padding = 7
            icon_inset = 5
            show_names = false
        "#
        .parse::<toml::Table>()
        .unwrap();
        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));
        client.toolbar_spellbook_config =
            screen_model::catalog_panel_config(&table, None, None, &assets);
        client.apply_ui_command("spellbook");

        let first = client.actions_panel_layout(&map, &assets).unwrap();
        assert_eq!(first.entries.len(), 2);
        assert_eq!(first.page_count, 2);
        assert_eq!(first.entries[0].rect.width, 44.0);

        client.actions_panel_page = 1;
        let second = client.actions_panel_layout(&map, &assets).unwrap();
        assert_eq!(second.entries.len(), 1);
        assert_eq!(second.entries[0].command, "rules.three");
    }

    #[test]
    fn spellbook_keeps_authored_geometry_when_switching_tabs() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Warrior"

            [classes.Warrior.action_bar]
            main = ["rules.attack", "rules.gather"]

            [actions.attack]
            name = "Attack"
            kind = "attack"

            [actions.gather]
            name = "Gather"
            kind = "gather"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Warrior".into()));
        let mut map = Map::default();
        map.entities.push(player);
        let table = r#"
            columns = 4
            rows = 3
            cell_size = 48
        "#
        .parse::<toml::Table>()
        .unwrap();
        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));
        client.toolbar_spellbook_config =
            screen_model::catalog_panel_config(&table, None, None, &assets);
        client.apply_ui_command("spellbook");

        let all = client.actions_panel_layout(&map, &assets).unwrap();
        client.actions_panel_tab = "utility".into();
        let utility = client.actions_panel_layout(&map, &assets).unwrap();

        assert_eq!(utility.rect.x, all.rect.x);
        assert_eq!(utility.rect.y, all.rect.y);
        assert_eq!(utility.rect.width, all.rect.width);
        assert_eq!(utility.rect.height, all.rect.height);
    }

    #[test]
    fn spellbook_details_are_grouped_into_summary_facts_effects_and_warning() {
        let description = RulesDescription {
            title: "Power Strike".into(),
            subtitle: Some("Attack".into()),
            lines: vec![
                "Make a heavier martial attack.".into(),
                "Activation: 1 Action".into(),
                "Range: Weapon".into(),
                "Applies: Staggered".into(),
            ],
        };
        let state = CommandState {
            enabled: false,
            disabled_reason: Some("Available at level 2".into()),
            ..CommandState::default()
        };

        let rows = Client::catalog_detail_rows(description, &state);
        assert!(
            rows.iter()
                .any(|row| row.kind == CatalogDetailRowKind::Summary)
        );
        assert!(
            rows.iter()
                .any(|row| row.kind == CatalogDetailRowKind::Fact)
        );
        assert!(
            rows.iter()
                .any(|row| row.kind == CatalogDetailRowKind::Effect)
        );
        assert!(
            rows.iter()
                .any(|row| row.kind == CatalogDetailRowKind::Warning)
        );
    }

    #[test]
    fn equipment_panel_uses_ruleset_owned_slots_around_the_avatar() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [equipment]
            weapon_slots = ["grip", "guard"]
            armor_slots = ["crown", "shell", "boots"]
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        let mut map = Map::default();
        map.entities.push(player);
        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));

        assert!(client.apply_ui_command("equipment"));
        let layout = client
            .equipment_panel_layout(&map, &assets)
            .expect("Equipment should resolve ruleset-owned slots");
        assert_eq!(
            layout
                .slots
                .iter()
                .map(|slot| slot.slot.as_str())
                .collect::<Vec<_>>(),
            vec!["grip", "crown", "boots", "guard", "shell"]
        );
        assert!(
            layout.slots[..3]
                .iter()
                .all(|slot| slot.rect.x < layout.avatar_rect.x)
        );
        assert!(
            layout.slots[3..]
                .iter()
                .all(|slot| slot.rect.x > layout.avatar_rect.x + layout.avatar_rect.width)
        );
    }

    #[test]
    fn equipment_and_preferences_configs_parse_toolbar_overrides() {
        let equipment = r##"
            slot_size = 60
            avatar_scale = 1.25
            left_slots = ["main_hand", "head"]
            right_slots = ["off_hand"]
            border_color = "#806739"
        "##
        .parse::<toml::Table>()
        .unwrap();
        let equipment = screen_model::equipment_panel_config(&equipment, None);
        assert_eq!(equipment.slot_size, 60.0);
        assert_eq!(equipment.avatar_scale, 1.25);
        assert_eq!(equipment.left_slots, vec!["main_hand", "head"]);
        assert_eq!(equipment.right_slots, vec!["off_hand"]);
        assert_eq!(equipment.border_color, [128, 103, 57, 255]);

        let preferences = r#"
            width = 340
            row_height = 38
            title = "Interface"
        "#
        .parse::<toml::Table>()
        .unwrap();
        let preferences = screen_model::preferences_panel_config(&preferences);
        assert_eq!(preferences.width, 340.0);
        assert_eq!(preferences.row_height, 38.0);
        assert_eq!(preferences.title, "Interface");

        let mut client = Client::new();
        assert!(client.apply_ui_command("preferences"));
        assert!(client.preferences_panel_open);
        assert!(client.apply_ui_command("preferences"));
        assert!(!client.preferences_panel_open);
    }

    #[test]
    fn toolbar_inventory_config_parses_grid_choices_and_style() {
        let table = r##"
            columns = 9
            rows = 6
            cell_size = 46
            spacing = 3
            padding = 8
            font = "Project UI"
            categories = [
                { label = "All Items", value = "all" },
                { label = "Gear", value = "equipment" },
            ]
            sort_options = ["Newest", "Name"]
            border_color = "#8f7444"
        "##
        .parse::<toml::Table>()
        .unwrap();

        let config = screen_model::inventory_panel_config(&table, None);
        assert_eq!(config.columns, 9);
        assert_eq!(config.rows, 6);
        assert_eq!(config.cell_size, 46.0);
        assert_eq!(config.spacing, 3.0);
        assert_eq!(config.padding, 8.0);
        assert_eq!(config.font, "Project UI");
        assert_eq!(config.categories[1].value, "equipment");
        assert_eq!(config.sort_options[1].value, "name");
        assert_eq!(config.border_color, [143, 116, 68, 255]);

        let mut client = Client::new();
        assert!(client.apply_ui_command("inventory"));
        assert!(client.inventory_panel_open);
        assert!(client.apply_ui_command("inventory"));
        assert!(!client.inventory_panel_open);
    }

    #[test]
    fn empty_spellbook_still_has_a_visible_panel() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Warrior"

            [classes.Warrior.action_bar]
            main = []
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Warrior".into()));
        let mut map = Map::default();
        map.entities.push(player);
        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));

        assert!(client.apply_ui_command("spellbook"));
        let layout = client.actions_panel_layout(&map, &assets).unwrap();
        assert!(layout.entries.is_empty());
        assert!(layout.empty_rect.is_some());
        assert!(layout.rect.height > layout.title_rect.height);
    }

    #[test]
    fn three_d_intent_catalog_selects_without_emitting_action() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = ["rules.minor_heal"]

            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
            target = "friendly_or_self"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Cleric".into()));
        let mut map = Map::default();
        map.entities.push(player);

        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));
        client.active_player_camera = Some(PlayerCamera::D3FirstP);
        client.actions_panel_open = true;
        client.draw_actions_panel(&map, &assets);
        let entry = client.actions_panel_entries[0].clone();

        assert_eq!(
            client.activate_actions_panel_command(&map, &assets, &entry.command),
            None
        );
        assert_eq!(
            client.get_current_intent().as_deref(),
            Some("action:minor_heal")
        );
        assert!(client.actions_panel_open);
    }

    #[test]
    fn actions_panel_catalog_cache_invalidates_when_rules_change() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = ["rules.minor_heal"]

            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Cleric".into()));
        let mut map = Map::default();
        map.entities.push(player);

        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));
        client.actions_panel_open = true;
        let first = client.actions_panel_layout(&map, &assets).unwrap();
        assert_eq!(first.entries[0].command, "rules.minor_heal");

        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = ["rules.holy_light"]

            [actions.holy_light]
            name = "Holy Light"
            kind = "spell"
        "#
        .into();
        let changed = client.actions_panel_layout(&map, &assets).unwrap();
        assert_eq!(changed.entries[0].command, "rules.holy_light");
    }

    #[test]
    fn slot_attack_selection_stays_selected_during_first_person_movement() {
        let mut assets = Assets::default();
        assets.rules = "[classes.Warrior.action_bar]\nmain = [\"rules.basic_attack\"]".into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Warrior".into()));
        player.set_attribute("intent", Value::Str(String::new()));
        let mut map = Map::default();
        map.entities.push(player);
        let mut client = Client::new();
        client.button_widgets.insert(
            42,
            Widget {
                id: 42,
                command_slot: Some("main.0".into()),
                ..Default::default()
            },
        );
        client.activate_targeting_button(42);
        client.intent.clear();
        let payload = client.selected_slot_intent(&map, &assets).unwrap();
        assert_eq!(payload, "action:basic_attack");
        client.intent = payload;
        assert_eq!(
            client.get_current_intent_for_action().as_deref(),
            Some("action:basic_attack")
        );
        map.entities[0].set_attribute("command_slot_main_0", Value::Str("rules.minor_heal".into()));
        assert_eq!(
            client.selected_slot_intent(&map, &assets).as_deref(),
            Some("action:minor_heal")
        );
        client.active_player_camera = Some(PlayerCamera::D3FirstPGrid);
        client
            .client_action
            .lock()
            .unwrap()
            .set_input_data("[input]\nw = \"control.forward\"\na = \"control.left\"");
        for (key, expected) in [("w", EntityAction::Forward), ("a", EntityAction::Left)] {
            assert_eq!(
                client.user_event("key_down".into(), Value::Str(key.into())),
                expected
            );
            assert_eq!(
                client.user_event("key_up".into(), Value::Str(key.into())),
                EntityAction::Off
            );
            assert!(client.activated_widgets.contains(&42));
            assert!(client.permanently_activated_widgets.contains(&42));
            assert_eq!(
                client.get_current_intent_for_action().as_deref(),
                Some("action:basic_attack")
            );
            assert_eq!(
                client.selected_slot_intent(&map, &assets).as_deref(),
                Some("action:minor_heal")
            );
        }
    }

    #[test]
    fn actions_panel_assign_mode_targets_reusable_command_slots() {
        let mut assets = Assets::default();
        assets.rules = r#"
            [identity.defaults]
            class = "Cleric"

            [classes.Cleric.action_bar]
            main = ["rules.minor_heal"]

            [actions.minor_heal]
            name = "Minor Heal"
            kind = "spell"
            target = "friendly_or_self"
        "#
        .into();
        let mut player = Entity::new();
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("class", Value::Str("Cleric".into()));
        let mut map = Map::default();
        map.entities.push(player);

        let mut client = Client::new();
        client.target = TheRGBABuffer::new(TheDim::sized(1280, 720));
        client.actions_panel_open = true;
        client.actions_assignment_mode = true;
        client.button_widgets.insert(
            42,
            Widget {
                id: 42,
                rect: Rect::new(20.0, 650.0, 48.0, 48.0),
                command_slot: Some("main.0".into()),
                ..Default::default()
            },
        );
        client.draw_actions_panel(&map, &assets);
        let entry = client.actions_panel_entries[0].clone();
        client.pending_action_assignment = Some(entry.command);
        assert_eq!(
            client.assign_pending_action_at_point(Vec2::new(22, 652)),
            Some(EntityAction::SetCommandSlot {
                slot: "main.0".into(),
                command: Some("rules.minor_heal".into()),
            })
        );
        assert!(!client.actions_assignment_mode);
        assert!(client.pending_action_assignment.is_none());
    }

    #[test]
    fn command_cooldown_overlay_preserves_icon_transparency() {
        let mut target = TheRGBABuffer::new(TheDim::sized(8, 6));
        for pixel in target.pixels_mut().chunks_exact_mut(4) {
            pixel.copy_from_slice(&[100, 120, 140, 255]);
        }
        let texture = crate::Texture::new(vec![0, 0, 0, 0, 255, 255, 255, 255], 2, 1);

        assert!(Client::draw_alpha_masked_command_overlay(
            &mut target,
            Rect::new(0.0, 0.0, 6.0, 5.0),
            &texture,
            128,
        ));

        let pixels = target.pixels();
        let transparent_dest = (2 + 2 * target.stride()) * 4;
        let opaque_dest = (3 + 2 * target.stride()) * 4;
        assert_eq!(
            &pixels[transparent_dest..transparent_dest + 4],
            &[100, 120, 140, 255]
        );
        assert!(
            pixels[opaque_dest] < 100
                && pixels[opaque_dest + 1] < 120
                && pixels[opaque_dest + 2] < 140,
            "opaque icon pixel should be darkened, got {:?}",
            &pixels[opaque_dest..opaque_dest + 4]
        );
        assert_eq!(pixels[opaque_dest + 3], 255);
    }
    #[test]
    fn editor_quick_start_preserves_entity_defaults_and_spawns_only_once() {
        let mut assets = Assets::default();
        let mut map = Map::default();
        let mut player = Entity::new();
        player.set_attribute("class_name", Value::Str("Player".into()));
        player.set_attribute("class", Value::Str("Warrior".into()));
        player.set_attribute("name", Value::Str("Default Hero".into()));
        player.set_attribute(
            "_entity_configuration",
            Value::Str("[attributes]\nclass = \"Warrior\"".into()),
        );
        map.entities.push(player);
        assets.maps.insert("Test".into(), map);
        let mut client = Client::new();
        client.current_map = "Test".into();
        client.player_entities.push("Player".into());
        client
            .ui_state
            .insert("start.class".into(), "Cleric".into());
        client
            .ui_state
            .insert("start.name".into(), "Setup Hero".into());
        let mut scene = SceneHandler::default();
        let commands = client.start_with_entity_defaults(&mut assets, &mut scene);
        assert_eq!(commands.len(), 1);
        let Command::CreateEntity(_, entity) = &commands[0] else {
            panic!("Expected player creation");
        };
        assert_eq!(entity.attributes.get_str("class"), Some("Warrior"));
        assert_eq!(entity.attributes.get_str("name"), Some("Default Hero"));
        assert!(entity.attributes.get_str("_entity_configuration").is_some());
        assert!(entity.attributes.get_str("_start_class").is_none());
        assert!(entity.attributes.get_str("_start_name").is_none());
        assert!(
            client
                .start_with_entity_defaults(&mut assets, &mut scene)
                .is_empty()
        );
    }
}

#[cfg(test)]
#[path = "tests/screen_materials.rs"]
mod screen_material_tests;
