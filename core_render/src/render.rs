use crate::draw2d::Draw2D;
use crate::prelude::*;
use rhai::{module_resolvers::StaticModuleResolver, Dynamic, Engine, Map, Module, Scope, AST};
use std::path::PathBuf;

use audio_engine::{AudioEngine, OggDecoder, WavDecoder};

use crate::raycast::{Facing, Raycast};

#[derive(Eq, Hash, PartialEq)]
pub enum Group {
    Effect,
    Music,
}

#[derive(Eq, PartialEq, Clone)]
pub enum DisplayMode {
    TwoD,
    ThreeD,
}

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

pub struct GameRender<'a> {
    engine: Engine,
    scope: Scope<'a>,
    ast: Option<AST>,

    draw2d: Draw2D,
    pub asset: Asset,

    pub frame: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub tile_size: usize,

    pub regions: FxHashMap<Uuid, GameRegionData>,
    pub lights: FxHashMap<Uuid, Vec<LightData>>,

    pub messages: Vec<MessageData>,
    pub multi_choice_data: Vec<MultiChoiceData>,

    pub last_position: Option<Position>,
    pub transition_steps: isize,
    pub transition_counter: isize,
    pub transition_active: bool,

    pub player_id: Uuid,

    pub last_update: GameUpdate,

    //#[cfg(target_arch = "wasm32")]
    pub audio_engine: Option<AudioEngine<Group>>,

    pub raycast: Raycast,

    pub this_map: Dynamic,

    pub vendor_rects: Vec<(usize, usize, usize, usize, Uuid)>,

    pub character_effects: FxHashMap<Uuid, (TileId, usize)>,

    pub force_display_mode: Option<DisplayMode>,
    pub display_mode: DisplayMode,

    mouse_pos: Option<(usize, usize)>,
    mouse_region_pos: Option<(isize, isize)>,
    draw_mouse_pos_once: bool,

    valid_mouse_rect: Option<ScriptRect>,

    // The region and screen rects of the 2d game drawing area
    region_rect_2d: (isize, isize, isize, isize),
    screen_rect_2d: (usize, usize, usize, usize),

    // Screen scripts and their utility scripts
    scripts: FxHashMap<String, String>,

    // We limit redraws to anim_counter updates, otherwise it flickers too much
    last_anim_counter: usize,
    last_light_map: FxHashMap<(isize, isize), f32>,

    pub indie_messages: Vec<String>,
}

impl GameRender<'_> {
    #[allow(unused)]
    pub fn new(path: PathBuf, player_id: Uuid) -> Self {
        let mut asset = Asset::new();
        #[cfg(not(feature = "embed_binaries"))]
        asset.load_from_path(path);
        #[cfg(feature = "embed_binaries")]
        asset.load_from_embedded();

        let mut engine = Engine::new();

        register_global_cmd_functions(&mut engine);

        engine
            .register_type_with_name::<ScriptTilemaps>("Tilemaps")
            .register_fn("get", ScriptTilemaps::get);

        engine
            .register_type_with_name::<ScriptImages>("Images")
            .register_fn("get", ScriptImages::get);

        engine
            .register_type_with_name::<ScriptTilemap>("Tilemap")
            .register_fn("get_tile", ScriptTilemap::get_tile);

        engine.register_type_with_name::<ScriptMessageCmd>("MessageCmd");

        ScriptPosition::register(&mut engine);
        ScriptRect::register(&mut engine);

        engine
            .register_type_with_name::<ScriptRGB>("RGB")
            .register_fn("rgb", ScriptRGB::new)
            .register_fn("rgba", ScriptRGB::new_with_alpha);

        engine
            .register_type_with_name::<ScriptShape>("Shape")
            .register_fn("shape", ScriptShape::shape)
            .register_set("color", ScriptShape::set_color)
            .register_set("border_color", ScriptShape::set_border_color)
            .register_set("border_size", ScriptShape::set_border_size)
            .register_fn("add_rect", ScriptShape::add_rect)
            .register_fn("add_rounded_rect", ScriptShape::add_rounded_rect);

        engine.on_print(|x| println!("{}", x));

        engine.register_fn("to_string", |x: f32| format!("{}", x.round() as i32));

        script_register_inventory_api(&mut engine);
        script_register_spells_api(&mut engine);
        script_register_gear_api(&mut engine);
        script_register_weapons_api(&mut engine);
        script_register_experience_api(&mut engine);
        script_register_date_api(&mut engine);

        Sheet::register(&mut engine);
        Currency::register(&mut engine);
        CharacterData::register(&mut engine);
        CharacterList::register(&mut engine);

        let this_map = Map::new();

        Self {
            engine,
            scope: Scope::new(),
            ast: None,

            draw2d: Draw2D { scissor: None },
            asset,
            frame: vec![0; 1024 * 608 * 4],
            width: 1024,
            height: 608,
            tile_size: 32,

            regions: FxHashMap::default(),
            lights: FxHashMap::default(),

            messages: vec![],
            multi_choice_data: vec![],

            last_position: None,
            transition_steps: 5,
            transition_counter: 0,
            transition_active: false,

            player_id,

            last_update: GameUpdate::new(),

            audio_engine: None,

            raycast: Raycast::new(),

            this_map: this_map.into(),

            vendor_rects: vec![],

            character_effects: FxHashMap::default(),

            force_display_mode: None,
            display_mode: DisplayMode::TwoD,

            mouse_pos: None,
            mouse_region_pos: None,
            draw_mouse_pos_once: false,

            valid_mouse_rect: None,

            region_rect_2d: (0, 0, 1, 1),
            screen_rect_2d: (0, 0, 1, 1),

            scripts: FxHashMap::default(),

            last_anim_counter: 0,
            last_light_map: FxHashMap::default(),

            indie_messages: vec![],
        }
    }

    pub fn process_game_settings(&mut self, _properties: &PropertySink) {}

    pub fn process_update(&mut self, update: &GameUpdate) -> Option<(String, Option<usize>)> {
        *SHEET.lock().unwrap() = update.sheet.clone();

        // Screen scripts ?
        if let Some(screen_scripts) = &update.screen_scripts {
            self.scripts = screen_scripts.clone();

            // Create a module resolver

            let mut resolver = StaticModuleResolver::new();

            for (name, script) in screen_scripts {
                if let Some(ast) = self.engine.compile(script).ok() {
                    let rc = Module::eval_ast_as_new(Scope::new(), &ast, &self.engine);

                    if rc.is_ok() {
                        if let Some(module) = rc.ok() {
                            resolver.insert(name.replace(".rhai", ""), module);
                        }
                    } else {
                        println!("Error in {}: {}", name, rc.err().unwrap().to_string());
                    }
                }
            }

            self.engine.set_module_resolver(resolver);
        }

        // New screen script ?
        if let Some(screen_script_name) = &update.screen_script_name {
            if let Some(screen_script) = self.scripts.get(screen_script_name) {
                let result = self
                    .engine
                    .compile_with_scope(&self.scope, screen_script.as_str());

                if result.is_ok() {
                    if let Some(ast) = result.ok() {
                        self.messages = vec![];
                        self.multi_choice_data = vec![];
                        self.last_position = None;
                        self.transition_active = false;

                        let this_map = Map::new();

                        self.scope = Scope::new();

                        // Width / Height / Tilesize
                        INFOCMD.lock().unwrap().width = update.screen_size.0;
                        INFOCMD.lock().unwrap().height = update.screen_size.1;
                        INFOCMD.lock().unwrap().tile_size = update.def_square_tile_size;

                        // Tilemaps
                        let mut tilemaps = ScriptTilemaps::new();
                        for index in 0..self.asset.tileset.maps_names.len() {
                            tilemaps.maps.insert(
                                self.asset.tileset.maps_names[index].clone(),
                                self.asset.tileset.maps_ids[index],
                            );
                        }

                        INFOCMD.lock().unwrap().tilemaps = tilemaps;

                        let mut images = ScriptImages::new();
                        for index in 0..self.asset.tileset.images_names.len() {
                            images.maps.insert(
                                self.asset.tileset.images_names[index].clone(),
                                self.asset.tileset.images_ids[index],
                            );
                        }

                        INFOCMD.lock().unwrap().images = images;

                        self.this_map = this_map.into();

                        let result = self
                            .engine
                            .eval_ast_with_scope::<Dynamic>(&mut self.scope, &ast);
                        if result.is_err() {
                            if let Some(err) = result.err() {
                                let mut string = err.to_string();
                                let mut parts = string.split("(");
                                if let Some(first) = parts.next() {
                                    string = first.to_owned();
                                }
                                return Some((string, err.position().line()));
                            }
                        }

                        #[allow(deprecated)]
                        let result = self.engine.call_fn_raw(
                            &mut self.scope,
                            &ast,
                            true,
                            true,
                            "init",
                            Some(&mut self.this_map),
                            [],
                        );

                        if let Some(err) = result.err() {
                            print!("Error in init(): {}", err.to_string());
                        }

                        self.tile_size = INFOCMD.lock().unwrap().tile_size as usize;

                        if self.frame.len() != self.width * self.height * 4 {
                            self.frame = vec![0; self.width * self.height * 4];
                        }

                        self.ast = Some(ast);

                        self.indie_messages = self.process_cmds(self.player_id);
                    }
                } else if let Some(err) = result.err() {
                    println!("{} : {:?}", err.0.to_string(), err.1.line());
                    return Some((err.0.to_string(), err.1.line()));
                }
            }
        }

        // Got a new region ?
        if let Some(region) = &update.region {
            self.regions.insert(region.id, region.clone());
            self.display_mode = if self.force_display_mode == Some(DisplayMode::ThreeD) {
                DisplayMode::ThreeD
            } else {
                DisplayMode::TwoD
            };

            let properties = &region.settings;

            if let Some(r) = properties.get("supports_3d") {
                if let Some(value) = r.as_bool() {
                    INFOCMD
                        .lock()
                        .unwrap()
                        .region
                        .insert("supports_3d".into(), Dynamic::from(value));
                    if value == true {
                        self.raycast.load_region(&self.asset, region);
                    }
                }
            }

            // 3D ? Load the region into the raycaster
            if self.display_mode == DisplayMode::ThreeD {
                self.raycast.load_region(&self.asset, region);
            }
        }

        // Get new messages
        if update.messages.is_empty() == false {
            for m in &update.messages {
                self.messages.push(m.clone());
            }
        }

        INFOCMD.lock().unwrap().messages = update.messages.clone();

        // Get new multi_choice_data
        if update.multi_choice_data.is_empty() == false {
            self.multi_choice_data.clear();
            for mcd in &update.multi_choice_data {
                self.multi_choice_data.push(mcd.clone());

                if mcd.header.is_empty() == false {
                    let message = MessageData {
                        message_type: MessageType::Vendor,
                        message: mcd.header.clone(),
                        from: "".to_string(),
                        right: None,
                        center: None,
                        buffer: None,
                    };
                    self.messages.push(message);
                }

                let mut text = format!("{}. {}", mcd.answer, mcd.text);
                let mut right: Option<String> = None;
                if let Some(amount) = mcd.item_amount {
                    if amount > 1 {
                        text += format!(" ({})", amount).as_str();
                    }
                }
                if let Some(mut value) = mcd.item_price {
                    right = Some(value.to_string());
                }
                let message = MessageData {
                    message_type: MessageType::Vendor,
                    message: text,
                    from: mcd.id.to_string(),
                    center: None,
                    right,
                    buffer: None,
                };
                self.messages.push(message);
            }
        }

        // Copy the characters
        INFOCMD.lock().unwrap().characters = CharacterList::new(update.characters.clone());

        // Clear the multi choice data if we have no ongoing communication
        if update.communication.is_empty() {
            self.multi_choice_data.clear();
        }

        // Play audio
        if update.audio.is_empty() == false {
            for m in &update.audio {
                self.play_audio(m.clone());
            }
        }

        // Insert the lights
        if let Some(position) = &update.position {
            self.lights.insert(position.region, update.lights.clone());
        }

        // Set the date
        INFOCMD.lock().unwrap().date = update.date.clone();

        None
    }

    /// Draw the server response
    pub fn draw(
        &mut self,
        anim_counter: usize,
        update: Option<&GameUpdate>,
    ) -> Option<(String, Option<usize>)> {
        self.frame.fill(0);

        if let Some(update) = update {
            let error = self.process_update(update);
            if error.is_some() {
                return error;
            }
            self.last_update = update.clone();
        }

        // Call the draw function
        if let Some(ast) = &self.ast {
            #[allow(deprecated)]
            let result = self.engine.call_fn_raw(
                &mut self.scope,
                &ast,
                true,
                true,
                "draw",
                Some(&mut self.this_map),
                [],
            );

            if result.is_err() {
                if let Some(err) = result.err() {
                    let mut string = err.to_string();
                    let mut parts = string.split("(");
                    if let Some(first) = parts.next() {
                        string = first.to_owned();
                    }
                    println!("{}", string);
                    return Some((string, err.position().line()));
                }
            }
        }

        // Draw
        let to_draw = SCRIPTCMD.lock().unwrap().draw_commands.clone();
        SCRIPTCMD.lock().unwrap().draw_commands.clear();

        fn is_safe(rect: (usize, usize, usize, usize), width: usize, height: usize) -> bool {
            if rect.0 + rect.2 > width {
                return false;
            }
            if rect.1 + rect.3 > height {
                return false;
            }
            return true;
        }

        for cmd in to_draw {
            let stride = self.width;

            match cmd {
                ScriptDrawCmd::DrawRect(rect, rgb) => {
                    if rect.is_safe(self.width, self.height) {
                        if rgb.value[3] == 255 {
                            self.draw2d.draw_rect(
                                &mut self.frame[..],
                                &rect.rect,
                                stride,
                                &rgb.value,
                            );
                        } else {
                            self.draw2d.blend_rect(
                                &mut self.frame[..],
                                &rect.rect,
                                stride,
                                &rgb.value,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawRectOutline(rect, rgb) => {
                    if rect.is_safe(self.width, self.height) {
                        self.draw2d.draw_rect_outline(
                            &mut self.frame[..],
                            &rect.rect,
                            stride,
                            &rgb.value,
                        );
                    }
                }
                ScriptDrawCmd::DrawShape(shape) => {
                    shape.draw(&mut self.frame[..], (self.width, self.height));
                }
                ScriptDrawCmd::DrawImage(pos, image, width, height, blend) => {
                    if is_safe(
                        (pos.pos.0, pos.pos.1, width as usize, height as usize),
                        self.width,
                        self.height,
                    ) {
                        if let Some(image) = self.asset.tileset.images.get(&image.id) {
                            //self.draw2d.draw_animated_tile( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.x_off as usize, tile.id.y_off as usize), anim_counter, self.tile_size);
                            self.draw2d.scale_chunk(
                                &mut self.frame[..],
                                &(pos.pos.0, pos.pos.1, width as usize, height as usize),
                                stride,
                                &image.pixels,
                                &(image.width, image.height),
                                blend,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawTile(pos, tile) => {
                    if is_safe(
                        (pos.pos.0, pos.pos.1, self.tile_size, self.tile_size),
                        self.width,
                        self.height,
                    ) {
                        if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                            self.draw2d.draw_animated_tile(
                                &mut self.frame[..],
                                &(pos.pos.0, pos.pos.1),
                                &map,
                                stride,
                                &(tile.id.x_off as usize, tile.id.y_off as usize),
                                anim_counter,
                                self.tile_size,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawTileSat(pos, tile, rgb) => {
                    if is_safe(
                        (pos.pos.0, pos.pos.1, self.tile_size, self.tile_size),
                        self.width,
                        self.height,
                    ) {
                        if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                            self.draw2d.draw_animated_tile_sat(
                                &mut self.frame[..],
                                &(pos.pos.0, pos.pos.1),
                                &map,
                                stride,
                                &(tile.id.x_off as usize, tile.id.y_off as usize),
                                anim_counter,
                                self.tile_size,
                                rgb.value,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawTileMult(pos, tile, rgb) => {
                    if is_safe(
                        (pos.pos.0, pos.pos.1, self.tile_size, self.tile_size),
                        self.width,
                        self.height,
                    ) {
                        if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                            self.draw2d.draw_animated_tile_mult(
                                &mut self.frame[..],
                                &(pos.pos.0, pos.pos.1),
                                &map,
                                stride,
                                &(tile.id.x_off as usize, tile.id.y_off as usize),
                                anim_counter,
                                self.tile_size,
                                rgb.value,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawTileSized(pos, tile, size) => {
                    if is_safe(
                        (pos.pos.0, pos.pos.1, size as usize, size as usize),
                        self.width,
                        self.height,
                    ) {
                        if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                            self.draw2d.draw_animated_tile(
                                &mut self.frame[..],
                                &(pos.pos.0, pos.pos.1),
                                &map,
                                stride,
                                &(tile.id.x_off as usize, tile.id.y_off as usize),
                                anim_counter,
                                size as usize,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawFrame(rect, tile) => {
                    if rect.is_safe(self.width, self.height) {
                        if rect.rect.2 >= 3 * self.tile_size && rect.rect.3 >= 3 * self.tile_size {
                            let tiles_x = rect.rect.2 / self.tile_size;
                            let tiles_y = rect.rect.3 / self.tile_size;
                            let mut x = rect.rect.0;

                            let top_y = rect.rect.1;
                            let bottom_y = rect.rect.1 + rect.rect.3 - self.tile_size;

                            for i in 0..tiles_x {
                                let mut t;
                                if i == 0 {
                                    t = tile.id.clone();
                                } else if i == tiles_x - 1 {
                                    t = tile.id.clone();
                                    t.x_off += 2;
                                } else {
                                    t = tile.id.clone();
                                    t.x_off += 1;
                                }

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile(
                                        &mut self.frame[..],
                                        &(x, top_y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                    );
                                }

                                let mut t;
                                if i == 0 {
                                    t = tile.id.clone();
                                    t.y_off += 2;
                                } else if i == tiles_x - 1 {
                                    t = tile.id.clone();
                                    t.x_off += 2;
                                    t.y_off += 2;
                                } else {
                                    t = tile.id.clone();
                                    t.x_off += 1;
                                    t.y_off += 2;
                                }

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile(
                                        &mut self.frame[..],
                                        &(x, bottom_y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                    );
                                }

                                x += self.tile_size;
                            }

                            let right_x = rect.rect.0 + rect.rect.2 - self.tile_size;

                            let mut y = rect.rect.1 + self.tile_size;
                            for _i in 0..tiles_y - 2 {
                                let mut t = tile.id.clone();
                                t.y_off += 1;

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile(
                                        &mut self.frame[..],
                                        &(rect.rect.0, y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                    );
                                }

                                let mut t = tile.id.clone();
                                t.x_off += 2;
                                t.y_off += 1;

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile(
                                        &mut self.frame[..],
                                        &(right_x, y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                    );
                                }

                                y += self.tile_size;
                            }
                        }
                    }
                }
                ScriptDrawCmd::DrawFrameSat(rect, rgb, tile) => {
                    if rect.is_safe(self.width, self.height) {
                        if rect.rect.2 >= 3 * self.tile_size && rect.rect.3 >= 3 * self.tile_size {
                            let tiles_x = rect.rect.2 / self.tile_size;
                            let tiles_y = rect.rect.3 / self.tile_size;
                            let mut x = rect.rect.0;

                            let top_y = rect.rect.1;
                            let bottom_y = rect.rect.1 + rect.rect.3 - self.tile_size;

                            for i in 0..tiles_x {
                                let mut t;
                                if i == 0 {
                                    t = tile.id.clone();
                                } else if i == tiles_x - 1 {
                                    t = tile.id.clone();
                                    t.x_off += 2;
                                } else {
                                    t = tile.id.clone();
                                    t.x_off += 1;
                                }

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile_sat(
                                        &mut self.frame[..],
                                        &(x, top_y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                        rgb.value,
                                    );
                                }

                                let mut t;
                                if i == 0 {
                                    t = tile.id.clone();
                                    t.y_off += 2;
                                } else if i == tiles_x - 1 {
                                    t = tile.id.clone();
                                    t.x_off += 2;
                                    t.y_off += 2;
                                } else {
                                    t = tile.id.clone();
                                    t.x_off += 1;
                                    t.y_off += 2;
                                }

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile_sat(
                                        &mut self.frame[..],
                                        &(x, bottom_y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                        rgb.value,
                                    );
                                }

                                x += self.tile_size;
                            }

                            let right_x = rect.rect.0 + rect.rect.2 - self.tile_size;

                            let mut y = rect.rect.1 + self.tile_size;
                            for _i in 0..tiles_y - 2 {
                                let mut t = tile.id.clone();
                                t.y_off += 1;

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile_sat(
                                        &mut self.frame[..],
                                        &(rect.rect.0, y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                        rgb.value,
                                    );
                                }

                                let mut t = tile.id.clone();
                                t.x_off += 2;
                                t.y_off += 1;

                                if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                    self.draw2d.draw_animated_tile_sat(
                                        &mut self.frame[..],
                                        &(right_x, y),
                                        &map,
                                        stride,
                                        &(t.x_off as usize, t.y_off as usize),
                                        anim_counter,
                                        self.tile_size,
                                        rgb.value,
                                    );
                                }

                                y += self.tile_size;
                            }
                        }
                    }
                }
                ScriptDrawCmd::DrawText(pos, text, font_name, size, rgb) => {
                    if let Some(font) = self.asset.game_fonts.get(&font_name) {
                        self.draw2d.blend_text_safe(
                            &mut self.frame[..],
                            &pos.pos,
                            stride,
                            font,
                            size,
                            text.as_str(),
                            &rgb.value,
                            (0, 0, self.width, self.height),
                        );
                    }
                }
                ScriptDrawCmd::DrawTextRect(rect, text, font_name, size, rgb, align) => {
                    if rect.is_safe(self.width, self.height) {
                        if let Some(font) = self.asset.game_fonts.get(&font_name) {
                            let al = align.to_lowercase();

                            if al == "right" {
                                self.draw2d.blend_text_rect(
                                    &mut self.frame[..],
                                    &rect.rect,
                                    stride,
                                    font,
                                    size,
                                    text.as_str(),
                                    &rgb.value,
                                    crate::draw2d::TextAlignment::Right,
                                );
                            } else if al == "center" {
                                self.draw2d.blend_text_rect(
                                    &mut self.frame[..],
                                    &rect.rect,
                                    stride,
                                    font,
                                    size,
                                    text.as_str(),
                                    &rgb.value,
                                    crate::draw2d::TextAlignment::Center,
                                );
                            } else {
                                self.draw2d.blend_text_rect(
                                    &mut self.frame[..],
                                    &rect.rect,
                                    stride,
                                    font,
                                    size,
                                    text.as_str(),
                                    &rgb.value,
                                    crate::draw2d::TextAlignment::Left,
                                );
                            }
                        }
                    }
                }
                ScriptDrawCmd::DrawMessages(rect, font_name, font_size, rgb) => {
                    if rect.is_safe(self.width, self.height) {
                        if let Some(font) = self.asset.game_fonts.get(&font_name) {
                            let mut y = rect.rect.1 + rect.rect.3 - 5;

                            // Draw Messages

                            for index in 0..self.messages.len() {
                                if self.messages[index].buffer.is_none() {
                                    self.messages[index].buffer =
                                        Some(self.draw2d.create_buffer_for_message(
                                            rect.rect.2,
                                            font,
                                            font_size,
                                            &self.messages[index],
                                            &rgb.value,
                                        ));
                                }
                            }

                            let mut message_index = (self.messages.len() - 1) as i32;
                            self.vendor_rects = vec![];

                            while message_index >= 0 {
                                if let Some(buffer) = &self.messages[message_index as usize].buffer
                                {
                                    y -= buffer.1;

                                    if self.messages[message_index as usize].message_type
                                        == MessageType::Vendor
                                    {
                                        if let Some(id) = Uuid::parse_str(
                                            self.messages[message_index as usize].from.as_str(),
                                        )
                                        .ok()
                                        {
                                            self.vendor_rects.push((
                                                rect.rect.0,
                                                y,
                                                buffer.0,
                                                buffer.1,
                                                id,
                                            ));
                                        }
                                    }

                                    self.draw2d.blend_slice_safe(
                                        &mut self.frame[..],
                                        &buffer.2,
                                        &((rect.rect.0) as isize, y as isize, buffer.0, buffer.1),
                                        self.width,
                                        &rect.rect,
                                    );

                                    y -= 5;
                                }
                                message_index -= 1;
                            }
                        }
                    }
                }
                ScriptDrawCmd::DrawGame2D(rect) => {
                    if rect.is_safe(self.width, self.height) {
                        if let Some(update) = update {
                            self.process_game_draw_2d(
                                rect.rect,
                                anim_counter,
                                update,
                                &mut None,
                                self.width,
                                (0, 0),
                            );
                        } else {
                            let update = self.last_update.clone();
                            self.process_game_draw_2d(
                                rect.rect,
                                anim_counter,
                                &update,
                                &mut None,
                                self.width,
                                (0, 0),
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawGameOffset2D(rect, offset) => {
                    if rect.is_safe(self.width, self.height) {
                        if let Some(update) = update {
                            self.process_game_draw_2d(
                                rect.rect,
                                anim_counter,
                                update,
                                &mut None,
                                self.width,
                                offset.pos_signed,
                            );
                        } else {
                            let update = self.last_update.clone();
                            self.process_game_draw_2d(
                                rect.rect,
                                anim_counter,
                                &update,
                                &mut None,
                                self.width,
                                offset.pos_signed,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawGame3D(rect) => {
                    if rect.is_safe(self.width, self.height) {
                        if let Some(update) = update {
                            self.process_game_draw_3d(
                                rect.rect,
                                anim_counter,
                                update,
                                &mut None,
                                self.width,
                            );
                        } else {
                            let update = self.last_update.clone();
                            self.process_game_draw_3d(
                                rect.rect,
                                anim_counter,
                                &update,
                                &mut None,
                                self.width,
                            );
                        }
                    }
                }
                ScriptDrawCmd::DrawRegion(_name, _rect, _size) => {}
            }
        }

        None
    }

    // Display the preview in the default region settings mode
    pub fn process_game_draw_auto(
        &mut self,
        rect: (usize, usize, usize, usize),
        anim_counter: usize,
        update: &GameUpdate,
        external_frame: &mut Option<&mut [u8]>,
        stride: usize,
        center_offset: (isize, isize),
    ) {
        if self.display_mode == DisplayMode::ThreeD {
            self.process_game_draw_3d(rect, anim_counter, &update, external_frame, stride);
        } else {
            self.process_game_draw_2d(
                rect,
                anim_counter,
                &update,
                external_frame,
                stride,
                center_offset,
            );
        }
    }

    pub fn process_game_draw_3d(
        &mut self,
        rect: (usize, usize, usize, usize),
        _anim_counter: usize,
        update: &GameUpdate,
        external_frame: &mut Option<&mut [u8]>,
        stride: usize,
    ) {
        if let Some(position) = update.position.clone() {
            if let Some(region) = self.regions.get(&position.region) {
                if external_frame.is_some() {
                    let frame = external_frame.as_deref_mut().unwrap();
                    self.raycast.render(
                        frame,
                        (position.x as i32, position.y as i32),
                        &region.id,
                        rect,
                        stride,
                        update,
                        &self.asset,
                    );
                } else {
                    self.raycast.render(
                        &mut self.frame[..],
                        (position.x as i32, position.y as i32),
                        &region.id,
                        rect,
                        self.width,
                        update,
                        &self.asset,
                    );
                }
            }
        }
    }

    pub fn process_game_draw_2d(
        &mut self,
        rect: (usize, usize, usize, usize),
        anim_counter: usize,
        update: &GameUpdate,
        external_frame: &mut Option<&mut [u8]>,
        stride: usize,
        center_offset: (isize, isize),
    ) {
        if let Some(position) = update.position.clone() {
            if self.transition_active == false {
                if self.last_position.is_some()
                    && position.region != self.last_position.clone().unwrap().region
                {
                    // Start transition
                    self.transition_active = true;
                    self.transition_counter = 1;
                    self.transition_steps = 6;
                } else {
                    self.last_position = Some(position.clone());
                }
            }

            if self.transition_active {
                self.draw_game_tile_2d_rect(
                    rect,
                    self.last_position.clone().unwrap().clone(),
                    anim_counter,
                    update,
                    None,
                    external_frame,
                    stride,
                    center_offset,
                );

                let mut r = rect.clone();

                let mut set: FxHashSet<(isize, isize)> = FxHashSet::default();

                let x_tiles = rect.2 / self.tile_size;

                let step_x = (x_tiles as f32 / self.transition_steps as f32) as f32;

                r.0 = x_tiles / 2 - ((step_x * self.transition_counter as f32) / 2.0) as usize;
                r.2 = (step_x * self.transition_counter as f32) as usize;

                for y in 0..r.3 {
                    for x in r.0..r.0 + r.2 {
                        set.insert((x as isize, y as isize));
                    }
                }

                self.draw_game_tile_2d_rect(
                    rect,
                    position.clone(),
                    anim_counter,
                    update,
                    Some(set),
                    external_frame,
                    stride,
                    center_offset,
                );

                self.transition_counter += 1;
                if self.transition_counter == self.transition_steps {
                    self.transition_active = false;
                    self.last_position = Some(position.clone());
                }
            } else if self.transition_active == false {
                self.draw_game_tile_2d_rect(
                    rect,
                    position.clone(),
                    anim_counter,
                    update,
                    None,
                    external_frame,
                    stride,
                    center_offset,
                );
            }
        }
    }

    /// Draws the game in the given rect
    pub fn draw_game_tile_2d_rect(
        &mut self,
        rect: (usize, usize, usize, usize),
        cposition: Position,
        anim_counter: usize,
        update: &GameUpdate,
        set: Option<FxHashSet<(isize, isize)>>,
        external_frame: &mut Option<&mut [u8]>,
        stride: usize,
        center_offset: (isize, isize),
    ) {
        self.draw2d.scissor = Some(rect);
        self.mouse_region_pos = None;

        let mut position = cposition;

        let tile_size = self.tile_size;

        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let mut x_tiles = (rect.2 / tile_size) as isize;
        let mut y_tiles = (rect.3 / tile_size) as isize;

        if let Some(region) = self.regions.get(&position.region) {
            // Get background color
            let mut background = [0, 0, 0, 255];
            if let Some(property) = region.settings.get(&"background") {
                if let Some(color) = property.to_rgb() {
                    background = color;
                }
            }

            // Get base lighting
            let mut base_light = 1.0;
            if let Some(property) = region.settings.get(&"base_lighting") {
                base_light = property.to_float() as f32;
            }

            if let Some(property) = region.settings.get(&"lighting") {
                fn get_brightness(minutes: i32) -> f32 {
                    let sunrise = 300; // 5:00 am
                    let sunset = 1200; // 8:00 pm
                    let transition_duration = 60; // 1 hour

                    let daylight_start = sunrise + transition_duration;
                    let daylight_end = sunset + transition_duration;

                    if minutes < sunrise || minutes > daylight_end {
                        return 0.0; // it's dark outside
                    }

                    if minutes >= sunrise && minutes <= daylight_start {
                        // transition from darkness to daylight
                        let transition_start = sunrise;
                        let time_since_transition_start = minutes - transition_start;
                        let brightness =
                            time_since_transition_start as f32 / transition_duration as f32;
                        return brightness;
                    } else if minutes >= sunset && minutes <= daylight_end {
                        // transition from daylight to darkness
                        let transition_start = sunset;
                        let time_since_transition_start = minutes - transition_start;
                        let brightness =
                            1.0 - time_since_transition_start as f32 / transition_duration as f32;
                        return brightness;
                    } else {
                        return 1.0;
                    }
                }

                if let Some(v) = property.as_string() {
                    if v == "timeofday" {
                        let daylight = get_brightness(update.date.minutes_in_day) as f32;
                        base_light = base_light.max(daylight);
                    }
                }
            }

            // Compute the light_map
            let mut light_map: FxHashMap<(isize, isize), f32> = FxHashMap::default();
            if anim_counter != self.last_anim_counter {
                if let Some(lights) = self.lights.get(&region.id) {
                    light_map = compute_lighting(&region, lights);
                    self.last_light_map = light_map.clone();
                }
            } else {
                light_map = self.last_light_map.clone();
            }

            // Get base lighting
            let mut full_visibility = true;
            if let Some(property) = region.settings.get(&"visibility") {
                if let Some(str) = property.as_string() {
                    if str == "limited" {
                        full_visibility = false;
                    }
                }
            }

            let mut visible_distance = 10;
            if full_visibility == false {
                if let Some(property) = region.settings.get(&"visible_distance") {
                    visible_distance = property.to_int();
                }
            }

            // Clear if not in a transition
            if set.is_none() {
                //self.draw2d.draw_rect(&mut self.frame[..], &rect, self.width, &background);
            }

            let mut offset = (0_isize, 0_isize);

            let mut gr = (0, 0);

            if let Some(old_position) = &update.old_position {
                let t = (update.curr_transition_time as f32
                    * (self.tile_size as f32 / (update.max_transition_time as f32 + 1.0)))
                    as isize;

                if position.x > old_position.x {
                    gr.0 = t;
                } else if position.x < old_position.x {
                    gr.0 = -t;
                }
                if position.y > old_position.y {
                    gr.1 = t;
                } else if position.y < old_position.y {
                    gr.1 = -t;
                }

                position = old_position.clone();
            }

            offset.0 = position.x + center_offset.0;
            offset.1 = position.y + center_offset.1;

            let region_width = region.max_pos.0 - region.min_pos.0;
            let region_height = region.max_pos.1 - region.min_pos.1;

            if region_width * tile_size as isize <= rect.2 as isize && center_offset.0 == 0 {
                offset.0 = region.min_pos.0;
                gr.0 = 0;
            } else {
                let left = x_tiles / 2;

                let distance_to_right = region.max_pos.0 - position.x + center_offset.0;
                let distance_to_left = position.x - region.min_pos.0 + center_offset.0;

                if distance_to_left < left + 1 {
                    offset.0 = region.min_pos.0;
                    if distance_to_left == left && gr.0 > 0 {
                        // At the transition point going left do not clear
                    } else {
                        gr.0 = 0;
                    }
                } else if distance_to_right < left + 1 {
                    offset.0 = region.max_pos.0 - x_tiles
                        + 1
                        + if center_offset.0 > 0 {
                            center_offset.0 * 2 - 1
                        } else {
                            0
                        };
                    if distance_to_right == left && gr.0 < 0 {
                        // At the transition point going right do not clear
                    } else {
                        gr.0 = 0;
                    }
                } else {
                    offset.0 -= left;
                }
            }

            if region_height * tile_size as isize <= rect.3 as isize && center_offset.1 == 0 {
                gr.1 = 0;
                offset.1 = region.min_pos.1;
            } else {
                let top = y_tiles / 2;

                let uneven = if top % 2 == 1 { 1 } else { 0 };

                let distance_to_bottom = region.max_pos.1 - position.y + center_offset.1;
                let distance_to_top = position.y - region.min_pos.1 + center_offset.1 + uneven;

                if distance_to_top < top + 1 {
                    offset.1 = region.min_pos.1;
                    if distance_to_top == top && gr.1 > 0 {
                        // At the transition point going downward do not clear
                    } else {
                        gr.1 = 0;
                    }
                } else if distance_to_bottom < top + 1 {
                    offset.1 = region.max_pos.1 - y_tiles
                        + 1
                        + if center_offset.1 > 0 {
                            center_offset.1 * 2 - 1
                        } else {
                            0
                        };
                    if distance_to_bottom == top && gr.1 < 0 {
                        // At the transition point going upward do not clear
                    } else {
                        gr.1 = 0;
                    }
                } else {
                    offset.1 -= top - uneven;
                }
            }

            // Expand the drawn area if scrolling is in progress

            let mut from_x = 0;
            let mut from_y = 0;

            if gr.0 != 0 {
                if gr.0 < 0 {
                    from_x = -1;
                } else {
                    x_tiles += 1;
                }
            }

            if gr.1 != 0 {
                if gr.1 < 0 {
                    from_y = -1;
                } else {
                    y_tiles += 1;
                }
            }

            // Draw Region

            for y in from_y..y_tiles {
                for x in from_x..x_tiles {
                    let pos_x = x + offset.0;
                    let pos_y = y + offset.1;

                    if full_visibility == false {
                        let a = position.x - pos_x;
                        let b = position.y - pos_y;

                        let d = ((a * a + b * b) as f32).sqrt() + 0.1;

                        if d > visible_distance as f32 {
                            continue;
                        }
                    }

                    let mut values = self.get_region_value(region, (pos_x, pos_y), update);

                    if let Some(loots) = update.loot.get(&(pos_x, pos_y)) {
                        for loot in loots {
                            if let Some(state) = &loot.state {
                                if let Some(tile) = state.tile.clone() {
                                    values.push(tile);
                                }
                            } else if let Some(tile) = loot.tile.clone() {
                                values.push(tile);
                            }
                        }
                    }

                    // Draw the tile(s)

                    let pos = (
                        rect.0 + left_offset + (x * tile_size as isize - gr.0) as usize,
                        rect.1 + top_offset + (y * tile_size as isize - gr.1) as usize,
                    );

                    // Store the region & screen position from the top-left tile
                    // To be able to easily calculate mouse to region coordinates
                    if x == from_x && y == from_y {
                        self.region_rect_2d = (pos_x, pos_y, x_tiles, y_tiles);
                        self.screen_rect_2d = (pos.0, pos.1, tile_size, tile_size);
                    }

                    let frame;
                    if external_frame.is_some() {
                        frame = external_frame.as_deref_mut().unwrap();
                    } else {
                        frame = &mut self.frame[..];
                    }

                    for value in values {
                        if let Some(set) = &set {
                            if set.contains(&(x, y)) == false {
                                continue;
                            }
                        }

                        if set.is_some() {
                            self.draw2d.draw_safe_rect(
                                frame,
                                &(pos.0, pos.1, tile_size, tile_size),
                                stride,
                                &background,
                            );
                        }

                        let mut light = base_light;
                        if let Some(l) = light_map.get(&(x + offset.0, y + offset.1)) {
                            light += *l;
                        }

                        if let Some(map) = self.asset.get_map_of_id(value.tilemap) {
                            self.draw2d.draw_animated_tile_with_blended_color(
                                frame, /*&mut self.frame[..]*/
                                &pos,
                                map,
                                stride,
                                &(value.x_off as usize, value.y_off as usize),
                                anim_counter,
                                tile_size,
                                &background,
                                light,
                            );
                        }
                    }

                    // Is the mouse on this position ?

                    if self.mouse_region_pos.is_some() {
                        continue;
                    }
                    if let Some(mouse_pos) = &self.mouse_pos {
                        if pos.0 <= mouse_pos.0
                            && pos.1 <= mouse_pos.1
                            && pos.0 + tile_size > mouse_pos.0
                            && pos.1 + tile_size > mouse_pos.1
                        {
                            self.draw2d.draw_rect_outline(
                                frame,
                                &(pos.0, pos.1, tile_size, tile_size),
                                stride,
                                &[128, 128, 128, 255],
                            );

                            if self.draw_mouse_pos_once {
                                self.mouse_pos = None;
                                self.draw_mouse_pos_once = false;
                            } else {
                                self.mouse_region_pos = Some((pos_x, pos_y));
                            }
                        }
                    }
                }
            }

            // Draw Characters
            for character in &update.characters {
                let mut position = character.position.clone();
                let tile = character.tile.clone();

                let mut tr = (0, 0);

                if let Some(old_position) = &character.old_position {
                    if character.id != update.id
                        || (character.id == update.id && gr.0 == 0 && gr.1 == 0)
                    {
                        let t = (character.curr_transition_time as f32
                            * (self.tile_size as f32
                                / (character.max_transition_time as f32 + 1.0)))
                            as isize;

                        if position.x > old_position.x {
                            tr.0 = t;
                        } else if position.x < old_position.x {
                            tr.0 = -t;
                        }

                        if position.y > old_position.y {
                            tr.1 = t;
                        } else if position.y < old_position.y {
                            tr.1 = -t;
                        }
                    }

                    position = old_position.clone();
                }

                if character.id != update.id {
                    tr.0 -= gr.0;
                    tr.1 -= gr.1;
                }

                let frame;
                if external_frame.is_some() {
                    frame = external_frame.as_deref_mut().unwrap();
                } else {
                    frame = &mut self.frame[..];
                }

                // Row check
                if position.x >= offset.0 && position.x < offset.0 + x_tiles {
                    // Column check
                    if position.y as isize >= offset.1 && position.y < offset.1 + y_tiles {
                        // Visible
                        let pos = (
                            rect.0
                                + left_offset
                                + (((position.x - offset.0) * tile_size as isize) + tr.0) as usize,
                            rect.1
                                + top_offset
                                + ((position.y - offset.1) * tile_size as isize + tr.1) as usize,
                        );

                        if let Some(set) = &set {
                            if set.contains(&(
                                ((pos.0 - rect.0) / self.tile_size) as isize,
                                ((pos.1 - rect.1) / self.tile_size) as isize,
                            )) == false
                            {
                                continue;
                            }
                        }

                        let mut light = base_light;
                        if let Some(l) = light_map.get(&(position.x, position.y)) {
                            light += *l;
                        }

                        if let Some(map) = self.asset.get_map_of_id(tile.tilemap) {
                            self.draw2d.draw_animated_tile_with_blended_color(
                                frame,
                                &pos,
                                map,
                                stride,
                                &(tile.x_off as usize, tile.y_off as usize),
                                anim_counter,
                                tile_size,
                                &background,
                                light,
                            );
                        }

                        // Insert effects into queue (right now only one is handled)
                        if character.effects.is_empty() == false {
                            for e in &character.effects {
                                self.character_effects
                                    .insert(character.id, (e.clone(), anim_counter));
                            }
                        }

                        // Play current effect
                        if let Some(fx) = self.character_effects.get(&character.id) {
                            let anim_c = anim_counter - fx.1;

                            if let Some(map) = self.asset.get_map_of_id(fx.0.tilemap) {
                                let grid_pos = (fx.0.x_off as usize, fx.0.y_off as usize);
                                self.draw2d.draw_animated_tile_with_blended_color(
                                    frame,
                                    &pos,
                                    map,
                                    stride,
                                    &grid_pos,
                                    anim_c,
                                    tile_size,
                                    &background,
                                    light,
                                );

                                if let Some(tile) = map.get_tile(&grid_pos) {
                                    if tile.anim_tiles.len() < anim_c {
                                        self.character_effects.remove(&character.id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("Region not found");
        }

        self.draw2d.scissor = None;
        self.last_anim_counter = anim_counter;
    }

    /// Gets the given region value
    pub fn get_region_value(
        &self,
        region: &GameRegionData,
        pos: (isize, isize),
        update: &GameUpdate,
    ) -> Vec<TileData> {
        let mut rc = vec![];

        if let Some(t) = update.displacements.get(&pos) {
            rc.push(t.clone());
        } else {
            if let Some(t) = region.layer1.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = region.layer2.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = region.layer3.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = region.layer4.get(&pos) {
                rc.push(t.clone());
            }
        }
        rc
    }

    pub fn key_down(
        &mut self,
        key: String,
        player_id: Uuid,
    ) -> (Vec<String>, Option<(String, Option<usize>)>) {
        let mut key_string = key.clone();

        for c in key.chars() {
            if c.is_ascii() {
                if c as u16 == 127 {
                    key_string = "DEL".into();
                }
            }
        }

        // Check if we have an active multiple choice communication
        if self.multi_choice_data.is_empty() == false {
            for mcd in &self.multi_choice_data {
                if mcd.answer == key.clone() {
                    if let Some(action) =
                        pack_multi_choice_answer_action(player_id, "Answer".to_string(), mcd.id)
                    {
                        return (vec![action], None);
                    }
                }
            }
            return (vec![], None);
        }

        // Call the key_down function
        if let Some(ast) = &self.ast {
            #[allow(deprecated)]
            let result = self.engine.call_fn_raw(
                &mut self.scope,
                &ast,
                true,
                true,
                "key_down",
                Some(&mut self.this_map),
                [key_string.into()],
            );

            if result.is_err() {
                if let Some(err) = result.err() {
                    let mut string = err.to_string();
                    let mut parts = string.split("(");
                    if let Some(first) = parts.next() {
                        string = first.to_owned();
                    }
                    return (vec![], Some((string, err.position().line())));
                }
            } else {
                self.draw(self.last_anim_counter, None);
            }
        }

        (self.process_cmds(player_id), None)
    }

    pub fn mouse_down(
        &mut self,
        pos: (usize, usize),
        player_id: Uuid,
    ) -> (Vec<String>, Option<(String, Option<usize>)>) {
        // Check mouse pos
        if let Some(valid_mouse_rect) = &self.valid_mouse_rect {
            if valid_mouse_rect.contains(pos) {
                self.mouse_pos = Some(pos);
            } else {
                self.mouse_pos = None;
            }
        } else {
            self.mouse_pos = Some(pos);
        }

        // Check if we have an active multiple choice communication
        if self.multi_choice_data.is_empty() == false {
            for r in &self.vendor_rects {
                if pos.0 >= r.0 && pos.1 >= r.1 && pos.0 < r.0 + r.2 && pos.1 < r.1 + r.3 {
                    if let Some(action) =
                        pack_multi_choice_answer_action(player_id, "Answer".to_string(), r.4)
                    {
                        return (vec![action], None);
                    }
                }
            }
            return (vec![], None);
        }

        // Call the touch_down function

        if let Some(ast) = &self.ast {
            #[allow(deprecated)]
            let result = self.engine.call_fn_raw(
                &mut self.scope,
                &ast,
                true,
                false,
                "touch_down",
                Some(&mut self.this_map),
                [(pos.0 as i32).into(), (pos.1 as i32).into()],
            );

            if result.is_err() {
                if let Some(err) = result.err() {
                    let mut string = err.to_string();
                    let mut parts = string.split("(");
                    if let Some(first) = parts.next() {
                        string = first.to_owned();
                    }
                    return (vec![], Some((string, err.position().line())));
                }
            }
        }

        (self.process_cmds(player_id), None)
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize)) {
        // Check mouse pos
        if let Some(valid_mouse_rect) = &self.valid_mouse_rect {
            if valid_mouse_rect.contains(pos) {
                self.mouse_pos = Some(pos);
            } else {
                self.mouse_pos = None;
            }
        } else {
            self.mouse_pos = Some(pos);
        }
    }

    fn process_cmds(&mut self, player_id: Uuid) -> Vec<String> {
        let mut commands = vec![];

        let display_mode_3d = INFOCMD.lock().unwrap().display_mode_3d;

        let action_commands = SCRIPTCMD.lock().unwrap().action_commands.clone();
        SCRIPTCMD.lock().unwrap().action_commands.clear();

        for cmd in action_commands {
            match cmd {
                ScriptServerCmd::EnterGameAndCreateCharacter(name, class, race, screen) => {
                    if let Some(json) = pack_enter_game_and_create(name, class, race, screen) {
                        commands.push(json);
                    }
                }
                ScriptServerCmd::EnterGameWithCharacter(name) => {
                    if let Some(json) = pack_enter_game_with(name) {
                        commands.push(json);
                    }
                }
                ScriptServerCmd::LoginUser(user, password, screen) => {
                    if let Some(json) = pack_login_user(user, password, screen) {
                        commands.push(json);
                    }
                }
                ScriptServerCmd::LoginLocalUser(user, screen) => {
                    if let Some(json) = pack_login_local_user(user, screen) {
                        commands.push(json);
                    }
                }
                ScriptServerCmd::RegisterUser(user, password, screen) => {
                    if let Some(json) = pack_register_user(user, password, screen) {
                        commands.push(json);
                    }
                }
                ScriptServerCmd::Action(action, direction, spell) => {
                    let mut dir: Option<PlayerDirection>;

                    if direction == "west" {
                        dir = Some(PlayerDirection::West);
                    } else if direction == "north" {
                        dir = Some(PlayerDirection::North);
                    } else if direction == "east" {
                        dir = Some(PlayerDirection::East);
                    } else if direction == "south" {
                        dir = Some(PlayerDirection::South);
                    } else {
                        dir = Some(PlayerDirection::None);
                    }

                    let mut processed_cmd = false;

                    // 3D mode overrides left / right
                    if display_mode_3d && action == "Move" {
                        if dir == Some(PlayerDirection::West) {
                            if self.raycast.facing == Facing::North {
                                self.raycast.facing = Facing::West;
                            } else if self.raycast.facing == Facing::West {
                                self.raycast.facing = Facing::South;
                            } else if self.raycast.facing == Facing::South {
                                self.raycast.facing = Facing::East;
                            } else if self.raycast.facing == Facing::East {
                                self.raycast.facing = Facing::North;
                            }
                            self.raycast.raycaster.turn_by(90.0);
                            processed_cmd = true;
                        } else if dir == Some(PlayerDirection::East) {
                            if self.raycast.facing == Facing::North {
                                self.raycast.facing = Facing::East;
                            } else if self.raycast.facing == Facing::East {
                                self.raycast.facing = Facing::South;
                            } else if self.raycast.facing == Facing::South {
                                self.raycast.facing = Facing::West;
                            } else if self.raycast.facing == Facing::West {
                                self.raycast.facing = Facing::North;
                            }
                            self.raycast.raycaster.turn_by(-90.0);
                            processed_cmd = true;
                        } else if dir == Some(PlayerDirection::North) {
                            if self.raycast.facing == Facing::West {
                                dir = Some(PlayerDirection::West);
                            } else if self.raycast.facing == Facing::South {
                                dir = Some(PlayerDirection::South);
                            } else if self.raycast.facing == Facing::East {
                                dir = Some(PlayerDirection::East);
                            }
                        } else if dir == Some(PlayerDirection::South) {
                            if self.raycast.facing == Facing::North {
                                dir = Some(PlayerDirection::South);
                            } else if self.raycast.facing == Facing::West {
                                dir = Some(PlayerDirection::East);
                            } else if self.raycast.facing == Facing::South {
                                dir = Some(PlayerDirection::North);
                            } else if self.raycast.facing == Facing::East {
                                dir = Some(PlayerDirection::West);
                            }
                        }
                    }

                    if processed_cmd == false {
                        if let Some(dir) = dir {
                            if let Some(action) =
                                pack_action(player_id, action.clone(), dir, spell.clone())
                            {
                                commands.push(action);
                            }
                        }
                    }
                }
                ScriptServerCmd::ActionCoordinate(action, spell) => {
                    // If we don't have the current mouse region pos, manually comp it
                    if self.mouse_region_pos.is_none() {
                        if let Some(mouse_pos) = &self.mouse_pos {
                            if mouse_pos.0 >= self.screen_rect_2d.0
                                && mouse_pos.1 >= self.screen_rect_2d.1
                            {
                                let sdx = mouse_pos.0 - self.screen_rect_2d.0;
                                let sdy = mouse_pos.1 - self.screen_rect_2d.1;
                                let ox = (sdx / self.screen_rect_2d.2) as isize;
                                let oy = (sdy / self.screen_rect_2d.3) as isize;
                                if ox < self.region_rect_2d.2 && oy < self.region_rect_2d.3 {
                                    let px = self.region_rect_2d.0 + ox;
                                    let py = self.region_rect_2d.1 + oy;
                                    self.mouse_region_pos = Some((px, py));
                                }
                            }
                        }
                    }

                    if let Some(region_pos) = &self.mouse_region_pos {
                        if let Some(action) = pack_action_coordinate(
                            player_id,
                            action.clone(),
                            *region_pos,
                            spell.clone(),
                        ) {
                            commands.push(action);
                        }
                    }

                    self.mouse_region_pos = None;
                    self.draw_mouse_pos_once = true;
                }
                ScriptServerCmd::ActionGear(action, gear_index) => {
                    if let Some(action) =
                        pack_gear_action(player_id, action.clone(), gear_index as u16)
                    {
                        commands.push(action);
                    }
                }
                ScriptServerCmd::ActionInventory(action, inv_index) => {
                    if let Some(action) =
                        pack_inventory_action(player_id, action.clone(), inv_index as u16)
                    {
                        commands.push(action);
                    }
                }
                ScriptServerCmd::ActionValidMouseRect(rect) => {
                    self.valid_mouse_rect = Some(rect.clone());
                }
            }
        }

        let messages = MESSAGECMD.lock().unwrap().messages.clone();
        MESSAGECMD.lock().unwrap().clear();

        for cmd in &messages {
            match cmd {
                ScriptMessage::Status(message) => {
                    self.messages.push(MessageData {
                        message_type: core_shared::message::MessageType::Status,
                        message: message.clone(),
                        from: "System".to_string(),
                        buffer: None,
                        right: None,
                        center: None,
                    });
                }
                ScriptMessage::Debug(message) => {
                    self.messages.push(MessageData {
                        message_type: core_shared::message::MessageType::Debug,
                        message: message.clone(),
                        from: "System".to_string(),
                        buffer: None,
                        right: None,
                        center: None,
                    });
                }
                ScriptMessage::Error(message) => {
                    self.messages.push(MessageData {
                        message_type: core_shared::message::MessageType::Error,
                        message: message.clone(),
                        from: "System".to_string(),
                        buffer: None,
                        right: None,
                        center: None,
                    });
                }
            }
        }

        commands
    }

    #[allow(unused_variables)]
    pub fn play_audio(&mut self, name: String) {
        #[cfg(not(feature = "embed_binaries"))]
        {
            if self.audio_engine.is_none() {
                self.audio_engine = AudioEngine::with_groups::<Group>().ok();
            }

            for (index, n) in self.asset.audio_names.iter().enumerate() {
                if *n == name {
                    //if let Some(bytes) = Embedded::get(self.asset.audio_paths[index].to_str().unwrap()) {
                    if let Some(file) =
                        std::fs::File::open(self.asset.audio_paths[index].clone()).ok()
                    {
                        if let Some(audio_engine) = &self.audio_engine {
                            let buffered = std::io::BufReader::new(file);

                            if name.ends_with("wav") {
                                if let Some(wav) = WavDecoder::new(buffered).ok() {
                                    if let Some(mut sound) =
                                        audio_engine.new_sound_with_group(Group::Effect, wav).ok()
                                    {
                                        sound.play();
                                        //audio_engine.set_group_volume(Group::Effect, 0.1);
                                    }
                                }
                            } else if name.ends_with("ogg") {
                                if let Some(ogg) = OggDecoder::new(buffered).ok() {
                                    if let Some(mut sound) =
                                        audio_engine.new_sound_with_group(Group::Effect, ogg).ok()
                                    {
                                        sound.play();
                                        //audio_engine.set_group_volume(Group::Effect, 0.1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(feature = "embed_binaries")]
        {
            if self.audio_engine.is_none() {
                self.audio_engine = AudioEngine::with_groups::<Group>().ok();
            }

            for (index, n) in self.asset.audio_names.iter().enumerate() {
                if *n == name {
                    if let Some(bytes) =
                        Embedded::get(self.asset.audio_paths[index].to_str().unwrap())
                    {
                        if let Some(audio_engine) = &self.audio_engine {
                            let buffered =
                                std::io::BufReader::new(std::io::Cursor::new(bytes.data));

                            if name.ends_with("wav") {
                                if let Some(wav) = WavDecoder::new(buffered).ok() {
                                    if let Some(mut sound) =
                                        audio_engine.new_sound_with_group(Group::Effect, wav).ok()
                                    {
                                        sound.play();
                                        audio_engine.set_group_volume(Group::Effect, 0.1);
                                    }
                                }
                            } else if name.ends_with("ogg") {
                                if let Some(ogg) = OggDecoder::new(buffered).ok() {
                                    if let Some(mut sound) =
                                        audio_engine.new_sound_with_group(Group::Effect, ogg).ok()
                                    {
                                        sound.play();
                                        audio_engine.set_group_volume(Group::Effect, 0.1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
