pub mod action;
pub mod command;
pub mod daylight;
pub mod draw2d;
pub mod parser;
pub mod resolver;
pub mod widget;

use scenevm::GeoId;
use std::str::FromStr;

use crate::prelude::*;
use crate::{
    BrushPreview, Command, D2ConceptBuilder, D2PreviewBuilder, EntityAction, PlayerCamera, Rect,
    SceneHandler, Surface, Value,
    client::action::ClientAction,
    client::widget::{
        Widget, avatar::AvatarWidget, deco::DecoWidget, game::GameWidget, messages::MessagesWidget,
        screen::ScreenWidget, text::TextWidget,
    },
};
use draw2d::Draw2D;
use fontdue::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use theframework::prelude::*;
use toml::*;

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

    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, Pixel, TheTime)>,

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
    deco_widgets: FxHashMap<Uuid, DecoWidget>,
    screen_widget: Option<ScreenWidget>,

    messages_widget: Option<MessagesWidget>,

    // Button widgets which are active (clicked)
    activated_widgets: Vec<u32>,

    // Button widgets which are permanently active
    permanently_activated_widgets: Vec<u32>,

    /// Client Action
    client_action: Arc<Mutex<ClientAction>>,

    /// Hidden widgets,
    widgets_to_hide: Vec<String>,

    // Choice map
    choice_map: Option<FxHashMap<char, Choice>>,

    // Intent
    intent: String,
    key_down_intent: Option<String>,

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

    // Dragged inventory/equipped item id
    dragging_item_id: Option<u32>,
    dragging_item_owner_entity_id: Option<u32>,
    dragging_source_widget_id: Option<u32>,
    dragging_item_from_world: bool,
    dragging_started: bool,
    drag_start_pos: Vec2<i32>,
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

    fn get_say_color(&self, category: &str) -> Pixel {
        if let Some(say) = self.config.get("say").and_then(toml::Value::as_table) {
            if let Some(hex) = say.get(category).and_then(toml::Value::as_str) {
                return Self::hex_to_rgba_u8(hex);
            }
            if let Some(hex) = say.get("default").and_then(toml::Value::as_str) {
                return Self::hex_to_rgba_u8(hex);
            }
            if let Some(hex) = say.get("").and_then(toml::Value::as_str) {
                return Self::hex_to_rgba_u8(hex);
            }
        }
        self.messages_font_color
    }

    fn get_say_duration_ticks(&self) -> i64 {
        let ticks_per_minute = self
            .get_config_i32_default("game", "ticks_per_minute", 4)
            .max(1);
        let duration_minutes = self
            .config
            .get("say")
            .and_then(toml::Value::as_table)
            .and_then(|say| say.get("duration"))
            .and_then(|v| {
                v.as_float()
                    .map(|f| f as f32)
                    .or_else(|| v.as_integer().map(|i| i as f32))
            })
            .unwrap_or(1.0)
            .max(0.0);
        let ticks = (duration_minutes * ticks_per_minute as f32).round() as i64;
        ticks.max(1)
    }

    fn get_say_background_enabled(&self) -> bool {
        self.get_config_bool_default("say", "background_enabled", true)
    }

    fn get_say_background_color(&self) -> Pixel {
        if let Some(say) = self.config.get("say").and_then(toml::Value::as_table) {
            if let Some(hex) = say.get("background_color").and_then(toml::Value::as_str) {
                return Self::hex_to_rgba_u8(hex);
            }
            if let Some(hex) = say.get("background").and_then(toml::Value::as_str) {
                return Self::hex_to_rgba_u8(hex);
            }
        }
        [0, 0, 0, 128]
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

            target_offset: Vec2::zero(),
            target: TheRGBABuffer::default(),
            overlay: TheRGBABuffer::default(),

            game_widgets: FxHashMap::default(),
            button_widgets: FxHashMap::default(),
            avatar_widgets: FxHashMap::default(),
            text_widgets: FxHashMap::default(),
            deco_widgets: FxHashMap::default(),
            screen_widget: None,

            messages_widget: None,

            activated_widgets: vec![],
            permanently_activated_widgets: vec![],
            widgets_to_hide: vec![],

            client_action: Arc::new(Mutex::new(ClientAction::default())),
            currencies: Currencies::default(),
            intent: String::new(),
            key_down_intent: None,

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
            dragging_item_id: None,
            dragging_item_owner_entity_id: None,
            dragging_source_widget_id: None,
            dragging_item_from_world: false,
            dragging_started: false,
            drag_start_pos: Vec2::zero(),
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
        self.camera_d3 = camera;
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
        for entity in &map.entities {
            if entity.is_player() {
                entity.apply_to_camera(&mut self.camera_d3, self.firstp_eye_level);
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
            let color = self.get_say_color(&category);
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
                                    color,
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
                                    color,
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
            if scene_handler.vm.vm_layer_count() > 2 {
                scene_handler.vm.set_layer_enabled(2, false);
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
        if scene_handler.vm.vm_layer_count() > 2 {
            scene_handler.vm.set_layer_enabled(2, false);
        }

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm_mode_2d));

        scene_handler.settings.apply_hour(hour);
        scene_handler.apply_runtime_render_state_settings();
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
            if scene_handler.vm.vm_layer_count() > 2 {
                scene_handler.vm.set_layer_enabled(2, false);
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

        // Draw Messages

        if let Some(font) = &self.messages_font {
            let say_bg_enabled = self.get_say_background_enabled();
            let say_bg_color = self.get_say_background_color();
            for (grid_pos, message, text_size, color, _) in self.messages_to_draw.values() {
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
                    color,
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
    ) {
        self.scene.animation_frame = self.animation_frame;

        let hour = self.server_time.to_f32();

        scene_handler.settings.apply_hour(hour);
        scene_handler.apply_dungeon_render_overrides(map);
        scene_handler.apply_runtime_render_state_settings();
        scene_handler.settings.apply_3d(&mut scene_handler.vm);
        scene_handler.apply_runtime_render_state_3d();

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
        if scene_handler.vm.vm_layer_count() > 2 {
            scene_handler.vm.set_layer_enabled(1, true);
            scene_handler.vm.set_layer_enabled(2, true);
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

            scene_handler.vm.set_active_vm(2);
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

            for (grid_pos, message, text_size, color, _) in self.messages_to_draw.values() {
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
                    color,
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

    /// Setup the client with the given assets.
    pub fn setup(&mut self, assets: &mut Assets, scene_handler: &mut SceneHandler) -> Vec<Command> {
        let mut commands = vec![];
        self.first_game_draw = true;
        self.intent = String::new();

        self.permanently_activated_widgets.clear();
        self.activated_widgets.clear();

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

        let mut currencies = Currencies::default();
        _ = currencies.add_currency(Currency {
            name: "Gold".into(),
            symbol: "G".into(),
            exchange_rate: 1.0,
            max_limit: None,
        });
        currencies.base_currency = "G".to_string();
        self.currencies = currencies;

        // Get all player entities
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

        // Auto Init Players
        let auto_init_player = self.get_config_bool_default("game", "auto_create_player", false);
        if let Some(map) = assets.maps.get(&self.current_map) {
            if auto_init_player {
                for entity in map.entities.iter() {
                    if let Some(class_name) = entity.get_attr_string("class_name") {
                        if self.player_entities.contains(&class_name) {
                            commands.push(Command::CreateEntity(map.id, entity.clone()));
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

        if assets.screens.contains_key(&self.current_screen) {
            self.init_screen(self.current_screen.clone(), assets, scene_handler);
        } else {
            eprintln!("Did not find start screen");
        }

        commands
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

        // Reset the intent to the server value
        self.current_sector.clear();
        if let Some(leader) = Self::resolve_party_entity(map, Some("leader")) {
            self.intent = leader.get_attr_string("intent").unwrap_or_default();
            self.current_sector = leader
                .get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .or_else(|| map.find_sector_at(leader.get_pos_xz()).map(|s| s.name.clone()))
                .unwrap_or_default();
        }

        self.target.fill([0, 0, 0, 255]);
        let say_bg_enabled = self.get_say_background_enabled();
        let say_bg_color = self.get_say_background_color();
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

            if !Self::is_2d_camera(&widget.camera) {
                if let Some(font) = &self.messages_font {
                    let width = widget.buffer.dim().width as usize;
                    let height = widget.buffer.dim().height as usize;
                    let pixels = widget.buffer.pixels_mut();

                    let view = widget.camera_d3.view_matrix();
                    let proj = widget
                        .camera_d3
                        .projection_matrix(width as f32, height as f32);
                    let vp = proj * view;

                    for (grid_pos, message, text_size, color, _) in self.messages_to_draw.values() {
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
                            color,
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
                for w in self.button_widgets.iter() {
                    if w.1.intent.is_some() && w.1.intent.as_ref().unwrap() == &self.intent {
                        screen_widget.builder_d2.activated_widgets.push(w.0.clone());
                    }
                }

                screen_widget.offset = Vec2::new(start_x, start_y);

                screen_widget.build(screen, assets);
                screen_widget.draw(screen, &self.server_time, assets);
                Self::punch_game_widget_holes(
                    &mut screen_widget.buffer,
                    screen_widget.background_color,
                    self.game_widgets.values(),
                );

                self.target.blend_into(0, 0, &screen_widget.buffer);
            }
        }

        // Draw the deco widgets on top
        for widget in self.deco_widgets.values_mut() {
            widget.update_draw(&mut self.target, map, &self.currencies, assets);
            self.target
                .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
        }

        // Draw the messages on top
        if let Some(widget) = &mut self.messages_widget {
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
                    messages,
                    choices,
                );
                if map.is_some() {
                    self.choice_map = map;
                }
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            } else {
                let map =
                    widget.process_messages(assets, map, &self.server_time, messages, choices);
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
                widget.update_draw(
                    &mut self.target,
                    map,
                    assets,
                    entity,
                    &self.draw2d,
                    &self.animation_frame,
                    if self.activated_widgets.contains(&widget.id) {
                        1
                    } else {
                        0
                    },
                );
            }
        }

        // Drag preview icon for inventory/equipped drag & drop.
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
                let stride = self.target.stride();
                self.draw2d.blend_scale_chunk(
                    self.target.pixels_mut(),
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
    )
    where
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
        let (scale, offset_x, offset_y) = self.presentation_transform_for_surface(size.0, size.1);
        self.upscale_factor = scale.max(0.0001);
        self.target_offset = Vec2::new(offset_x as i32, offset_y as i32);

        let Some(widget) = self.game_widgets.values_mut().next() else {
            return false;
        };

        let width = size.0.max(1) as i32;
        let height = size.1.max(1) as i32;
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
        let say_bg_enabled = self.get_say_background_enabled();
        let say_bg_color = self.get_say_background_color();

        if let Some(leader) = Self::resolve_party_entity(map, Some("leader")) {
            self.intent = leader.get_attr_string("intent").unwrap_or_default();
            self.current_sector = leader
                .get_attr_string("sector")
                .filter(|s| !s.is_empty())
                .or_else(|| map.find_sector_at(leader.get_pos_xz()).map(|s| s.name.clone()))
                .unwrap_or_default();
        }

        if let Some(screen) = assets.screens.get(&self.current_screen)
            && let Some(screen_widget) = &mut self.screen_widget
        {
            let (start_x, start_y) =
                crate::utils::align_screen_to_grid(w as f32, h as f32, self.grid_size);

            screen_widget.builder_d2.activated_widgets = self.activated_widgets.clone();
            screen_widget.grid_size = self.grid_size;

            for w in self.button_widgets.iter() {
                if w.1.intent.is_some() && w.1.intent.as_ref().unwrap() == &self.intent {
                    screen_widget.builder_d2.activated_widgets.push(w.0.clone());
                }
            }

            screen_widget.offset = Vec2::new(start_x, start_y);
            screen_widget.build(screen, assets);
            screen_widget.draw(screen, &self.server_time, assets);
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

                for (grid_pos, message, text_size, color, _) in self.messages_to_draw.values() {
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
                        color,
                        draw2d::TheHorizontalAlign::Center,
                        draw2d::TheVerticalAlign::Center,
                        &(0, 0, overlay_w as isize, overlay_h as isize),
                    );
                }
            }
        }

        for widget in self.deco_widgets.values_mut() {
            widget.update_draw(&mut self.overlay, map, &self.currencies, assets);
            self.overlay
                .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
        }

        if let Some(widget) = &mut self.messages_widget {
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
                    messages,
                    choices,
                );
                if map.is_some() {
                    self.choice_map = map;
                }
                self.overlay
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            } else {
                let map =
                    widget.process_messages(assets, map, &self.server_time, messages, choices);
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
                widget.update_draw(
                    &mut self.overlay,
                    map,
                    assets,
                    entity,
                    &self.draw2d,
                    &self.animation_frame,
                    if self.activated_widgets.contains(&widget.id) {
                        1
                    } else {
                        0
                    },
                );
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
            widget.drag_drop
                && (widget.inventory_index.is_some() || widget.equipped_slot.is_some())
        })
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
                entity.attributes.get_int("party_index").unwrap_or_else(|| {
                    if entity.is_player() {
                        0
                    } else {
                        i32::MAX / 2
                    }
                }),
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
        let Some(player_pos) = map
            .entities
            .iter()
            .find(|entity| entity.is_player())
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

    fn quantize_2d_tile_pos(pos: Vec2<f32>) -> Vec2<f32> {
        Vec2::new(pos.x.trunc(), pos.y.trunc())
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

    /// Drag event
    pub fn touch_dragged(
        &mut self,
        coord: Vec2<i32>,
        _map: &Map,
        _scene_handler: &mut SceneHandler,
    ) {
        let p = self.screen_to_viewport(coord);
        self.cursor_pos = p;
        if self.dragging_item_id.is_some() && !self.dragging_started {
            if self.drag_distance_exceeded(p) {
                self.dragging_started = true;
            }
        }

        if self.dragging_item_id.is_some() {
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
        let drop_intent_active = self
            .get_current_intent()
            .map(|i| i.eq_ignore_ascii_case("drop"))
            .unwrap_or(false);

        // Temporary, we have to make this widget dependent
        self.curr_cursor = self.default_cursor;
        self.hovered_entity_id = None;
        self.hovered_item_id = None;
        self.hovered_world_pos = None;
        self.curr_intent_cursor = None;
        self.curr_clicked_intent_cursor = None;
        self.hover_distance = f32::MAX;

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

                if !Self::is_2d_camera(&widget.camera) {
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
                                for button_id in &self.activated_widgets {
                                    if let Some(widget) = self.button_widgets.get(button_id) {
                                        self.curr_intent_cursor =
                                            widget.entity_cursor_id.or(widget.item_cursor_id);
                                        self.curr_clicked_intent_cursor = widget
                                            .entity_clicked_cursor_id
                                            .or(widget.item_clicked_cursor_id);
                                        self.hover_distance = distance;

                                        if let Some(cursor_id) =
                                            widget.entity_cursor_id.or(widget.item_cursor_id)
                                        {
                                            self.curr_cursor = Some(cursor_id);
                                        }
                                    }
                                }
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
                                    for button_id in &self.activated_widgets {
                                        if let Some(widget) = self.button_widgets.get(button_id) {
                                            self.curr_intent_cursor = widget.item_cursor_id;
                                            self.curr_clicked_intent_cursor =
                                                widget.item_clicked_cursor_id;
                                            self.hover_distance = distance;

                                            if let Some(cursor_id) = widget.item_cursor_id {
                                                self.curr_cursor = Some(cursor_id);
                                            }
                                        }
                                    }
                                }
                            }
                            GeoId::Sector(sector_id) => {
                                if let Some(item) =
                                    SceneHandler::find_item_by_sector_id(map, sector_id)
                                {
                                    self.hovered_item_id = Some(item.id);
                                    for button_id in &self.activated_widgets {
                                        if let Some(widget) = self.button_widgets.get(button_id) {
                                            self.curr_intent_cursor = widget.item_cursor_id;
                                            self.curr_clicked_intent_cursor =
                                                widget.item_clicked_cursor_id;
                                            self.hover_distance = distance;

                                            if let Some(cursor_id) = widget.item_cursor_id {
                                                self.curr_cursor = Some(cursor_id);
                                            }
                                        }
                                    }
                                }
                            }
                            GeoId::Item(item_id) => {
                                self.hovered_item_id = Some(item_id);
                                for button_id in &self.activated_widgets {
                                    if let Some(widget) = self.button_widgets.get(button_id) {
                                        self.curr_intent_cursor = widget.item_cursor_id;
                                        self.curr_clicked_intent_cursor =
                                            widget.item_clicked_cursor_id;
                                        self.hover_distance = distance;

                                        if let Some(cursor_id) = widget.item_cursor_id {
                                            self.curr_cursor = Some(cursor_id);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Click / touch down event
    pub fn touch_down(&mut self, coord: Vec2<i32>, map: &Map) -> Option<EntityAction> {
        let mut action = None;
        let mut camera_action = None;
        let mut render_camera_switches: Vec<(Option<String>, PlayerCamera)> = Vec::new();
        let active_intent = self.get_current_intent_for_action();
        self.dragging_item_id = None;
        self.dragging_item_owner_entity_id = None;
        self.dragging_source_widget_id = None;
        self.dragging_item_from_world = false;
        self.dragging_started = false;

        // Adjust cursor
        if self.curr_clicked_intent_cursor.is_some() {
            self.curr_cursor = self.curr_clicked_intent_cursor;
        } else {
            self.curr_cursor = self.default_cursor;
        }

        // If we hovered over an item in 3D, send an explicit ItemClicked intent
        if let Some(entity_id) = self.hovered_entity_id {
            return Some(EntityAction::EntityClicked(
                entity_id,
                self.hover_distance,
                self.get_current_intent_for_action(),
            ));
        }

        // If we hovered over an item in 3D, send an explicit ItemClicked intent or start a drag
        if let Some(item_id) = self.hovered_item_id {
            if self.has_drag_drop_targets() {
                eprintln!(
                    "[client:touch_down] 3d world-item drag start item_id={} coord=({}, {})",
                    item_id, coord.x, coord.y
                );
                self.dragging_item_id = Some(item_id);
                self.dragging_item_owner_entity_id = None;
                self.dragging_item_from_world = true;
                self.drag_start_pos = self.screen_to_viewport(coord);
                return None;
            }
            return Some(EntityAction::ItemClicked(
                item_id,
                self.hover_distance,
                self.get_current_intent_for_action(),
                None,
            ));
        }

        // Transform screen coordinates to viewport coordinates
        let p = self.screen_to_viewport(coord);
        for (id, widget) in self.button_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                self.activated_widgets.push(*id);

                if widget.drag_drop {
                    if let Some(entity) =
                        Self::resolve_party_entity(map, widget.party.as_deref())
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

                // Action buttons should work in both 2D and 3D. Intent (if present) sets
                // the active intent state and only becomes a one-shot action in 2D when no
                // directional action is defined.
                let parsed_action = EntityAction::from_str(&widget.action).ok();
                if let Some(act) = parsed_action.clone() {
                    action = Some(act);
                }
                if let Some(intent) = &widget.intent {
                    self.intent = intent.clone();
                    if parsed_action.is_none() && self.game_widget_is_2d() {
                        if intent.eq_ignore_ascii_case("spell")
                            && let Some(spell) = &widget.spell
                            && !spell.trim().is_empty()
                        {
                            action = Some(EntityAction::Intent(format!("spell:{}", spell.trim())));
                        } else {
                            action = Some(EntityAction::Intent(intent.clone()));
                        }
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
        for (target, camera) in render_camera_switches {
            self.set_game_widget_camera_mode(target.as_deref(), camera);
        }

        if camera_action.is_some() {
            action = camera_action;
        }

        // Test against clicks on interactive messages (multiple choice)
        if action.is_none() {
            for widget in self.messages_widget.iter_mut() {
                if let Some(action) = widget.touch_down(p) {
                    return Some(action);
                }
            }
        }

        // Test against clicks on the map
        if action.is_none() {
            let mut player_pos: Vec2<f32> = Vec2::zero();
            for entity in map.entities.iter() {
                if entity.is_player() {
                    player_pos = entity.get_pos_xz();
                }
            }

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
                            && let Some(item) =
                                map.items.iter().find(|item| {
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
                                return Some(EntityAction::EntityClicked(
                                    entity.id,
                                    distance,
                                    self.get_current_intent_for_action(),
                                ));
                            }
                        }

                        for item in map.items.iter() {
                            let item_pos = item.get_pos_xz();
                            if tile_pos == Self::quantize_2d_tile_pos(item_pos) {
                                let distance = player_pos.distance(item_pos);
                                return Some(EntityAction::ItemClicked(item.id, distance, None, None));
                            }
                        }

                        // Try entities again but include dead ones too
                        for entity in map.entities.iter() {
                            let entity_pos = entity.get_pos_xz();
                            if tile_pos == Self::quantize_2d_tile_pos(entity_pos) {
                                let distance = player_pos.distance(entity_pos);
                                return Some(EntityAction::EntityClicked(
                                    entity.id,
                                    distance,
                                    self.get_current_intent_for_action(),
                                ));
                            }
                        }

                        return Some(EntityAction::TerrainClicked(tile_pos));
                    }
                }
            }
        }

        action
    }

    /// Click / touch up event
    pub fn touch_up(&mut self, coord: Vec2<i32>, map: &Map) -> Option<EntityAction> {
        let mut action = None;
        let dragged_item_id = self.dragging_item_id;
        let dragged_item_owner_entity_id = self.dragging_item_owner_entity_id;
        let dragged_source_widget_id = self.dragging_source_widget_id;
        let dragged_item_from_world = self.dragging_item_from_world;
        let p = self.screen_to_viewport(coord);
        let dragging_started = self.dragging_started || self.drag_distance_exceeded(p);

        if let Some(item_id) = dragged_item_id {
            if !dragging_started {
                if dragged_item_from_world {
                    action = Some(EntityAction::ItemClicked(
                        item_id,
                        Self::item_click_distance(map, item_id),
                        self.get_current_intent_for_action(),
                        None,
                    ));
                } else if let Some(source_id) = dragged_source_widget_id
                    && let Some(widget) = self.button_widgets.get(&source_id)
                    && widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
                {
                    action = Some(EntityAction::ItemClicked(
                        item_id,
                        0.0,
                        self.get_current_intent_for_action(),
                        dragged_item_owner_entity_id,
                    ));
                }
            } else {
                for (_, widget) in self.button_widgets.iter() {
                    if !widget.drag_drop || !widget.rect.contains(Vec2::new(p.x as f32, p.y as f32))
                    {
                        continue;
                    }
                    let target_entity_id = Self::resolve_party_entity(map, widget.party.as_deref())
                        .map(|entity| entity.id);
                    if let Some(target_index) = widget.inventory_index {
                        action = Some(EntityAction::MoveItem {
                            item_id,
                            owner_entity_id: dragged_item_owner_entity_id,
                            target_entity_id,
                            to_inventory_index: Some(target_index),
                            to_equipped_slot: None,
                        });
                        break;
                    }
                    if let Some(target_slot) = &widget.equipped_slot {
                        action = Some(EntityAction::MoveItem {
                            item_id,
                            owner_entity_id: dragged_item_owner_entity_id,
                            target_entity_id,
                            to_inventory_index: None,
                            to_equipped_slot: Some(target_slot.clone()),
                        });
                        break;
                    }
                }
                if action.is_none()
                    && !dragged_item_from_world
                    && let Some(position) = self.drop_position_at_viewport(p)
                {
                    action = Some(EntityAction::DropItemAt {
                        item_id,
                        owner_entity_id: dragged_item_owner_entity_id,
                        position,
                    });
                }
            }
        }
        self.dragging_item_id = None;
        self.dragging_item_owner_entity_id = None;
        self.dragging_source_widget_id = None;
        self.dragging_item_from_world = false;
        self.dragging_started = false;

        self.activated_widgets = self.permanently_activated_widgets.clone();

        // Reset cursor after click release. Hover logic applies intent cursors contextually.
        self.curr_cursor = self.default_cursor;

        for widget in self.messages_widget.iter_mut() {
            widget.touch_up();
        }
        action
    }

    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        let immediate_2d_intent = matches!(
            self.active_game_widget_camera_mode(),
            Some(crate::PlayerCamera::D2 | crate::PlayerCamera::D2Grid)
        );

        // Make sure we do not send action events after a key down intent was handled
        // Otherwise the character would move a bit because "intent" is already cleared
        if event == "key_up" {
            self.key_down_intent = None;
        }

        if immediate_2d_intent && event == "key_down" {
            if let Some(key_down_intent) = &self.key_down_intent {
                if !key_down_intent.is_empty() {
                    return EntityAction::Off;
                }
            }
        }

        if immediate_2d_intent && self.key_down_intent.is_none() && event == "key_down" {
            self.key_down_intent = Some(self.intent.clone());
        }

        // --- Check for multiple choice

        if let Some(choice_map) = &self.choice_map.clone() {
            if event == "key_down" {
                if let Value::Str(v) = &value {
                    if let Some(c) = v.chars().next() {
                        if let Some(choice) = choice_map.get(&c) {
                            // println!("selected {:?}", choice);
                            if matches!(choice, Choice::Cancel(_, _)) {
                                self.choice_map = None;
                            }
                            return EntityAction::Choice(choice.clone());
                        }
                    }
                }
            }
        }

        // ---

        let is_key_down = event == "key_down";
        let mut action = self.client_action.lock().unwrap().user_event(event, value);

        if is_key_down && let EntityAction::Intent(intent_name) = &action {
            if immediate_2d_intent {
                // 2D uses immediate intents (one-shot on next directional action),
                // so shortcuts must not force sticky button activation.
                self.intent = intent_name.clone();
            } else {
                // 3D uses intent as a persistent state (same behavior as clicking a button).
                self.apply_intent_button_activation(intent_name);
                // In 3D, keyboard intent shortcuts are UI state changes only.
                // Do not forward intent actions to the server directly.
                action = EntityAction::Off;
            }
        }

        let action_str: String = action.to_string();
        if action_str == "none" {
            self.activated_widgets = self.permanently_activated_widgets.clone();
        } else {
            for (id, widget) in self.button_widgets.iter_mut() {
                if widget.action == action_str && !self.activated_widgets.contains(id) {
                    self.activated_widgets.push(*id);
                }
            }
        }

        action
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
            let action_match = action_norm == intent_norm
                || action_norm == format!("intent({})", intent_norm)
                || action_norm == format!("intent(\"{}\")", intent_norm)
                || action_norm == format!("intent('{}')", intent_norm);

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
                    selected_intent = if spell_template_norm.is_some() {
                        widget
                            .spell
                            .as_ref()
                            .map(|s| format!("spell:{}", s.trim()))
                            .or_else(|| widget.intent.clone())
                    } else {
                        widget
                            .intent
                            .clone()
                            .or_else(|| Some(intent_name.to_string()))
                    };
                }
            }
        }

        // Keep fallback intent state in sync with keyboard shortcuts.
        self.intent = selected_intent.unwrap_or_else(|| intent_name.to_string());

        let Some(button_id) = selected_button_id else {
            return;
        };

        // Deactivate all other intent buttons so shortcut intent is authoritative.
        for (id, widget) in self.button_widgets.iter() {
            if *id != button_id
                && let Some(intent) = &widget.intent
                && !intent.is_empty()
            {
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
        self.deco_widgets.clear();
        self.messages_widget = None;

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

                let textures = vec![];

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
                            let mut action = "";
                            let mut intent = None;
                            let mut spell = None;
                            let mut group = None;
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

                            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
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

                                if let Some(value) = ui.get("party").and_then(toml::Value::as_str)
                                {
                                    let binding = value.trim();
                                    if !binding.is_empty() {
                                        party = Some(binding.to_string());
                                    }
                                }

                                // Check for active
                                if let Some(value) = ui.get("active") {
                                    if let Some(v) = value.as_bool()
                                        && v
                                    {
                                        self.activated_widgets.push(widget.id);
                                        self.permanently_activated_widgets.push(widget.id);
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
                            }

                            let button_widget = Widget {
                                name: widget.name.clone(),
                                id: widget.id,
                                rect: Rect::new(x, y, width, height),
                                action: action.into(),
                                intent,
                                spell,
                                group,
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
                            };

                            self.button_widgets.insert(widget.id, button_widget);
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
                            self.messages_widget = Some(widget);
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
                if let Some(intent) = &widget.intent
                    && !intent.is_empty()
                {
                    return Some(intent.clone());
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
                && let Some(intent) = &widget.intent
            {
                if intent.is_empty() {
                    continue;
                }
                if intent.eq_ignore_ascii_case("spell")
                    && let Some(spell) = &widget.spell
                    && !spell.trim().is_empty()
                {
                    return Some(format!("spell:{}", spell.trim()));
                }
                return Some(intent.clone());
            }
        }
        if self.intent.is_empty() {
            None
        } else {
            Some(self.intent.clone())
        }
    }
}
