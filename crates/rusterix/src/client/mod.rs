pub mod action;
pub mod command;
pub mod daylight;
pub mod draw2d;
pub mod parser;
pub mod resolver;
pub mod widget;

use scenevm::{Atom, GeoId};
use std::str::FromStr;

use crate::prelude::*;
use crate::{
    AccumBuffer, BrushPreview, Command, D2PreviewBuilder, EntityAction, Rect, SceneHandler,
    ShapeFXGraph, Surface, Tracer, Value,
    client::action::ClientAction,
    client::widget::{
        Widget, deco::DecoWidget, game::GameWidget, messages::MessagesWidget, screen::ScreenWidget,
        text::TextWidget,
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

    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub scene_d2: Scene,
    pub scene_d3: Scene,

    pub scene: Scene,

    pub animation_frame: usize,
    pub server_time: TheTime,

    pub brush_preview: Option<BrushPreview>,

    /// Global render graph
    pub global: ShapeFXGraph,

    pub messages_font: Option<Font>,
    pub messages_font_size: f32,
    pub messages_font_color: Pixel,

    pub draw2d: Draw2D,

    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, TheTime)>,

    // Name of player entity templates
    player_entities: Vec<String>,

    pub current_map: String,
    current_screen: String,

    config: toml::Table,

    pub viewport: Vec2<i32>,
    grid_size: f32,
    pub target_fps: i32,
    pub game_tick_ms: i32,

    // The offset we copy the target into
    pub target_offset: Vec2<i32>,

    // The target we render into
    target: TheRGBABuffer,

    // The UI overlay
    overlay: TheRGBABuffer,

    // The widgets
    game_widgets: FxHashMap<Uuid, GameWidget>,
    button_widgets: FxHashMap<u32, Widget>,
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
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            curr_map_id: Uuid::default(),

            builder_d2: D2PreviewBuilder::new(),

            camera_d3: Box::new(D3FirstPCamera::new()),
            builder_d3: D3Builder::new(),

            scene_d2: Scene::default(),
            scene_d3: Scene::default(),

            scene: Scene::default(),

            animation_frame: 0,
            server_time: TheTime::default(),

            brush_preview: None,

            global: ShapeFXGraph::default(),

            messages_font: None,
            draw2d: Draw2D::default(),

            messages_font_size: 15.0,
            messages_font_color: [229, 229, 1, 255],

            messages_to_draw: FxHashMap::default(),

            player_entities: Vec::new(),

            current_map: String::new(),
            current_screen: String::new(),

            config: toml::Table::default(),
            viewport: Vec2::zero(),
            grid_size: 32.0,
            target_fps: 30,
            game_tick_ms: 250,

            target_offset: Vec2::zero(),
            target: TheRGBABuffer::default(),
            overlay: TheRGBABuffer::default(),

            game_widgets: FxHashMap::default(),
            button_widgets: FxHashMap::default(),
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

            hover_distance: f32::MAX,
        }
    }

    /// Increase the anim counter.
    pub fn inc_animation_frame(&mut self) {
        self.animation_frame += 1;

        for widget in self.game_widgets.values_mut() {
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
                entity.apply_to_camera(&mut self.camera_d3);
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
    pub fn process_messages(&mut self, map: &Map, messages: Vec<crate::server::Message>) {
        // Remove expired messages
        let expired_keys: Vec<_> = self
            .messages_to_draw
            .iter()
            .filter(|(_, (_, _, _, expire_time))| *expire_time < self.server_time)
            .map(|(id, _)| *id)
            .collect();

        for id in expired_keys {
            self.messages_to_draw.remove(&id);
        }

        // Add new messages
        for (sender_entity_id, sender_item_id, _, message, _category) in messages {
            if let Some(sender_item_id) = sender_item_id {
                for item in &map.items {
                    if item.id == sender_item_id {
                        if let Some(font) = &self.messages_font {
                            let text_size =
                                self.draw2d
                                    .get_text_size(font, self.messages_font_size, &message);

                            let ticks = self.server_time.to_ticks(4);
                            let expire_time = TheTime::from_ticks(ticks + 4, 4);

                            self.messages_to_draw.insert(
                                sender_item_id,
                                (item.get_pos_xz(), message.clone(), text_size.0, expire_time),
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

                            let ticks = self.server_time.to_ticks(4);
                            let expire_time = TheTime::from_ticks(ticks + 4, 4);

                            self.messages_to_draw.insert(
                                sender_entity_id,
                                (
                                    entity.get_pos_xz(),
                                    message.clone(),
                                    text_size.0,
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
        _assets: &Assets,
        scene_handler: &mut SceneHandler,
    ) {
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

        // let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
        //     .render_mode(RenderMode::render_2d());
        // rast.render_graph = self.global.clone();
        // rast.hour = self.server_time.to_f32();
        // rast.mapmini = self.scene.mapmini.clone();
        // rast.rasterize(&mut self.scene_d2, pixels, width, height, 64, assets);

        scene_handler.vm.set_active_vm(1);
        scene_handler.vm.set_layer_enabled(0, false);
        scene_handler.vm.set_layer_enabled(1, true);
        // scene_handler.vm.set_layer_activity_logging(true);

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));

        scene_handler.vm.execute(Atom::SetGP0(Vec4::new(
            map.grid_size,
            map.subdivisions,
            map.offset.x,
            -map.offset.y,
        )));

        // Ambient
        scene_handler.vm.execute(Atom::SetGP1(Vec4::one()));

        // Enable background clearing in the overlay shadr
        scene_handler.vm.execute(Atom::SetGP2(Vec4::one()));

        // Background
        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::zero()));

        // Transform
        scene_handler.vm.execute(Atom::SetTransform2D(transform));

        // Render
        scene_handler
            .vm
            .render_frame(pixels, width as u32, height as u32);

        scene_handler.vm.execute(Atom::SetGP2(Vec4::zero()));
        scene_handler.vm.set_active_vm(0);
        scene_handler.vm.set_layer_enabled(0, true);
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

        // let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
        //     .render_mode(RenderMode::render_2d());
        // rast.render_graph = self.global.clone();
        // rast.hour = self.server_time.to_f32();
        // rast.mapmini = self.scene.mapmini.clone();
        // rast.rasterize(&mut self.scene, pixels, width, height, 64, assets);

        scene_handler.vm.execute(scenevm::Atom::SetGP0(Vec4::new(
            map.grid_size,
            map.subdivisions,
            map.offset.x,
            -map.offset.y,
        )));

        let hour = self.server_time.to_f32();

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute2D));

        scene_handler.settings.apply_hour(hour);
        scene_handler.settings.apply_2d(&mut scene_handler.vm);

        scene_handler
            .vm
            .execute(scenevm::Atom::SetTransform2D(transform));

        // Set the transform for the overlay if active
        if scene_handler.vm.vm_layer_count() > 1 {
            scene_handler.vm.set_active_vm(1);
            scene_handler
                .vm
                .execute(scenevm::Atom::SetTransform2D(transform));
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
            .execute(scenevm::Atom::SetBackground(Vec4::zero()));

        scene_handler
            .vm
            .render_frame(pixels, width as u32, height as u32);

        // Draw Messages

        if let Some(font) = &self.messages_font {
            for (grid_pos, message, text_size, _) in self.messages_to_draw.values() {
                let position = map_grid_to_local(screen_size, *grid_pos, map);

                let tuple = (
                    position.x as isize - *text_size as isize / 2 - 5,
                    position.y as isize - self.messages_font_size as isize - map.grid_size as isize,
                    *text_size as isize + 10,
                    22,
                );

                self.draw2d.blend_rect_safe(
                    pixels,
                    &tuple,
                    width,
                    &[0, 0, 0, 128],
                    &(0, 0, width as isize, height as isize),
                );

                self.draw2d.text_rect_blend_safe(
                    pixels,
                    &tuple,
                    width,
                    font,
                    self.messages_font_size,
                    message,
                    &self.messages_font_color,
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
        scene_handler.settings.apply_3d(&mut scene_handler.vm);

        scene_handler
            .vm
            .execute(scenevm::Atom::SetAnimationCounter(self.animation_frame));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetBackground(Vec4::new(0.0, 0.0, 0.0, 1.0)));

        scene_handler
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute3D));

        scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
            camera: self.camera_d3.as_scenevm_camera(),
        });

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

            scene_handler.vm.set_active_vm(2);
            scene_handler.vm.execute(scenevm::Atom::SetCamera3D {
                camera: self.camera_d3.as_scenevm_camera(),
            });
            scene_handler
                .vm
                .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute3D));
            scene_handler.vm.set_active_vm(0);
        }

        // scene_handler.vm.print_geometry_stats();

        scene_handler
            .vm
            .render_frame(pixels, width as u32, height as u32);
    }

    /// Trace the 3D scene.
    pub fn trace(&mut self, accum: &mut AccumBuffer, assets: &Assets) {
        self.scene.animation_frame = self.animation_frame;
        let mut tracer = Tracer::default();
        tracer.render_graph = self.global.clone();
        tracer.hour = self.server_time.to_f32();
        tracer.trace(self.camera_d3.as_ref(), &mut self.scene, accum, 64, assets);
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

    fn _get_config_f32_default(&self, table: &str, key: &str, default: f32) -> f32 {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_float() {
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

        _ = scene_handler.settings.read(&assets.config);

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
        let mut player_entity = Entity::default();

        // Keep scene timing in sync with config
        scene_handler.set_timings(self.target_fps as f32, self.game_tick_ms);

        // Reset the intent to the server value
        for entity in map.entities.iter() {
            if entity.is_player() {
                self.intent = entity.get_attr_string("intent").unwrap_or_default();
                player_entity = entity.clone();
            }
        }

        self.target.fill([0, 0, 0, 255]);
        // First process the game widgets
        for widget in self.game_widgets.values_mut() {
            widget.apply_entities(map, assets, self.animation_frame, scene_handler);
            widget.draw(
                map,
                &self.server_time,
                self.animation_frame,
                assets,
                scene_handler,
            );

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
                let map = widget.update_draw(&mut self.target, assets, map, messages, choices);
                if map.is_some() {
                    self.choice_map = map;
                }
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            } else {
                let map = widget.process_messages(assets, map, messages, choices);
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
                widget.update_draw(&mut self.target, map, &self.currencies, assets);
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
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
                widget.update_draw(
                    &mut self.target,
                    map,
                    assets,
                    &player_entity,
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
    }

    ///Hover event, used to adjust the screen cursor based on the widget or game object under the mouse
    pub fn touch_hover(&mut self, coord: Vec2<i32>, map: &Map, scene_handler: &mut SceneHandler) {
        let p = self.screen_to_viewport(coord);
        self.cursor_pos = p;

        // Temporary, we have to make this widget dependent
        self.curr_cursor = self.default_cursor;
        self.hovered_entity_id = None;
        self.hovered_item_id = None;
        self.curr_intent_cursor = None;
        self.curr_clicked_intent_cursor = None;
        self.hover_distance = f32::MAX;

        for (_, widget) in self.game_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                let dx = p.x as f32 - widget.rect.x;
                let dy = p.y as f32 - widget.rect.y;

                if widget.camera != crate::PlayerCamera::D2 {
                    // We cast a ray into the game view and get the GeoId
                    let screen_uv = Vec2::new(dx / widget.rect.width, dy / widget.rect.height);
                    if let Some((geoid, _, distance)) = scene_handler.vm.pick_geo_id_at_uv(
                        widget.rect.width as u32,
                        widget.rect.height as u32,
                        [screen_uv.x, screen_uv.y],
                        false,
                        true,
                    ) {
                        match geoid {
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
                            GeoId::Item(item_id) => {
                                self.hovered_item_id = Some(item_id);

                                /*
                                for item in &map.items {
                                    if item.id == item_id {
                                        if let Some(cursor_id_str) =
                                            item.get_attr_string("cursor_id")
                                        {
                                            if !cursor_id_str.is_empty() {
                                                if let Ok(uuid) = Uuid::parse_str(&cursor_id_str) {
                                                    self.curr_cursor = Some(uuid);
                                                }
                                            }
                                        }
                                        self.hovered_item_id = Some(item.id);
                                        break;
                                    }
                                }*/
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

        // Adjust cursor
        if self.curr_clicked_intent_cursor.is_some() {
            self.curr_cursor = self.curr_clicked_intent_cursor;
        } else {
            self.curr_cursor = self.default_cursor;
        }

        // If we hovered over an item in 3D, send an explicit ItemClicked intent
        if let Some(item_id) = self.hovered_item_id {
            return Some(EntityAction::ItemClicked(
                item_id,
                self.hover_distance,
                self.get_current_intent(),
            ));
        }

        // Transform screen coordinates to viewport coordinates
        let p = self.screen_to_viewport(coord);

        for (id, widget) in self.button_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                self.activated_widgets.push(*id);

                if let Some(intent) = &widget.intent {
                    self.intent = intent.clone();
                    if self.game_widget_is_2d() {
                        action = Some(EntityAction::Intent(intent.clone()));
                    }
                    // break;
                } else if let Ok(act) = EntityAction::from_str(&widget.action) {
                    if self.game_widget_is_2d() {
                        action = Some(act);
                    }
                    // break;
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
                if let Some(inventory_index) = &widget.inventory_index {
                    for entity in map.entities.iter() {
                        if entity.is_player() {
                            if let Some(item) = entity.inventory.get(*inventory_index) {
                                if let Some(item) = item {
                                    action = Some(EntityAction::ItemClicked(item.id, 0.0, None));
                                    break;
                                }
                            }
                        }
                    }
                }

                // Deactivate the widgets and activate this widget
                if !widget.deactivate.is_empty() {
                    for widget_to_deactivate in &widget.deactivate {
                        for (id, widget) in self.button_widgets.iter() {
                            if *widget_to_deactivate == widget.name {
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
                    if widget.camera == crate::PlayerCamera::D2 {
                        let dx = p.x as f32 - widget.rect.x;
                        let dy = p.y as f32 - widget.rect.y;

                        let gx = widget.top_left.x + dx / widget.grid_size;
                        let gy = widget.top_left.y + dy / widget.grid_size;

                        let pos = Vec2::new(gx, gy);

                        for entity in map.entities.iter() {
                            if entity.attributes.get_str_default("mode", "active".into()) == "dead"
                            {
                                continue;
                            }
                            let p = entity.get_pos_xz();
                            if pos.floor() == p.floor() {
                                let distance = player_pos.distance(p);
                                return Some(EntityAction::EntityClicked(entity.id, distance));
                            }
                        }

                        for item in map.items.iter() {
                            let p = item.get_pos_xz();
                            if pos.floor() == p.floor() {
                                let distance = player_pos.distance(p);
                                return Some(EntityAction::ItemClicked(item.id, distance, None));
                            }
                        }

                        // Try entities again but include dead ones too
                        for entity in map.entities.iter() {
                            let p = entity.get_pos_xz();
                            if pos.floor() == p.floor() {
                                let distance = player_pos.distance(p);
                                return Some(EntityAction::EntityClicked(entity.id, distance));
                            }
                        }

                        return Some(EntityAction::TerrainClicked(pos));
                    }
                }
            }
        }

        action
    }

    /// Click / touch up event
    pub fn touch_up(&mut self, _coord: Vec2<i32>, _map: &Map) {
        self.activated_widgets = self.permanently_activated_widgets.clone();

        // Adjust cursor
        if self.curr_intent_cursor.is_some() {
            self.curr_cursor = self.curr_intent_cursor;
        } else {
            self.curr_cursor = self.default_cursor;
        }

        for widget in self.messages_widget.iter_mut() {
            widget.touch_up();
        }
    }

    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        // Make sure we do not send action events after a key down intent was handled
        // Otherwise the character would move a bit because "intent" is already cleared
        if event == "key_up" {
            self.key_down_intent = None;
        }

        if event == "key_down" {
            if let Some(key_down_intent) = &self.key_down_intent {
                if !key_down_intent.is_empty() {
                    return EntityAction::Off;
                }
            }
        }

        if self.key_down_intent.is_none() && event == "key_down" {
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

        let action = self.client_action.lock().unwrap().user_event(event, value);

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

    // Init the screen
    pub fn init_screen(
        &mut self,
        screen_name: String,
        assets: &mut Assets,
        scene_handler: &mut SceneHandler,
    ) {
        self.game_widgets.clear();
        self.button_widgets.clear();
        self.text_widgets.clear();
        self.deco_widgets.clear();
        self.messages_widget = None;

        self.screen_widget = Some(ScreenWidget {
            buffer: TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y)),
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

                // if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                //     widget.properties.get("screen_graph")
                // {
                //     if let Some(graph) = screen.shapefx_graphs.get(id) {
                //         textures =
                //             graph.create_screen_widgets(width as usize, height as usize, assets);
                //     }
                // }

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
                                rect: Rect::new(x, y, width, height),
                                toml_str: data.clone(),
                                buffer: TheRGBABuffer::new(TheDim::sized(
                                    width as i32,
                                    height as i32,
                                )),
                                grid_size,
                                ..Default::default()
                            };

                            if let Some(map) = assets.maps.get(&self.current_map) {
                                game_widget.build(map, assets, scene_handler);
                            }
                            game_widget.init();
                            self.game_widgets.insert(widget.creator_id, game_widget);
                        } else if role == "button" {
                            let mut action = "";
                            let mut intent = None;
                            let mut show: Option<Vec<String>> = None;
                            let mut hide: Option<Vec<String>> = None;
                            let mut deactivate: Vec<String> = vec![];
                            let mut inventory_index: Option<usize> = None;

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
                                show,
                                hide,
                                deactivate,
                                inventory_index,
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
            if w.camera == crate::PlayerCamera::D2 {
                return true;
            }
        }
        false
    }

    /// Returns the intent of the currently activated button
    fn get_current_intent(&self) -> Option<String> {
        for button_id in &self.activated_widgets {
            if let Some(widget) = self.button_widgets.get(button_id) {
                return widget.intent.clone();
            }
        }
        None
    }
}
