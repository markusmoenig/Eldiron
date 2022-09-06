
use std::{path::PathBuf, collections::{HashMap, HashSet}};

use crate::prelude::*;
use core_shared::{asset::{Asset}, update::GameUpdate, regiondata::{GameRegionData}, message::MessageData, light::Light};
use crate::{draw2d::Draw2D, script_types::*};
use rhai::{ Engine, Scope, AST, Dynamic };

use core_shared::actions::*;

use audio_engine::{AudioEngine, WavDecoder};

#[derive(Eq, Hash, PartialEq)]
pub enum Group {
    Effect,
    Music,
}

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

pub struct GameRender<'a> {

    engine                      : Engine,
    scope                       : Scope<'a>,
    ast                         : Option<AST>,

    draw2d                      : Draw2D,
    pub asset                   : Asset,

    pub frame                   : Vec<u8>,
    pub width                   : usize,
    pub height                  : usize,
    pub tile_size               : usize,

    pub regions                 : HashMap<Uuid, GameRegionData>,
    pub lights                  : HashMap<Uuid, Vec<Light>>,

    pub messages                : Vec<MessageData>,

    pub last_position           : Option<Position>,
    pub transition_steps        : isize,
    pub transition_counter      : isize,
    pub transition_active       : bool,

    pub player_id               : Uuid,

    //#[cfg(target_arch = "wasm32")]
    pub audio_engine            : Option<AudioEngine<Group>>
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

        engine.register_type_with_name::<ScriptTilemaps>("Tilemaps")
            .register_fn("get", ScriptTilemaps::get);

        engine.register_type_with_name::<ScriptTilemap>("Tilemap")
            .register_fn("get_tile", ScriptTilemap::get_tile);

        engine.register_type_with_name::<ScriptTile>("Tile");

        engine.register_type_with_name::<ScriptDraw>("Draw")
            .register_fn("rect", ScriptDraw::rect)
            .register_fn("tile", ScriptDraw::tile)
            .register_fn("tile_sat", ScriptDraw::tile_sat)
            .register_fn("tile_sized", ScriptDraw::tile_sized)
            .register_fn("frame", ScriptDraw::frame)
            .register_fn("frame_sat", ScriptDraw::frame_sat)
            .register_fn("game", ScriptDraw::game)
            .register_fn("region", ScriptDraw::region)
            .register_fn("messages", ScriptDraw::messages)
            .register_fn("text", ScriptDraw::text);

        engine.register_type_with_name::<ScriptCmd>("Cmd")
            .register_fn("move", ScriptCmd::cmd_move);

        engine.register_type_with_name::<ScriptMessageCmd>("MessageCmd")
            .register_fn("status", ScriptMessageCmd::status);

        engine.register_type_with_name::<ScriptRect>("Rect")
            .register_fn("rect", ScriptRect::new);

        engine.register_type_with_name::<ScriptPosition>("Position")
            .register_fn("pos", ScriptPosition::new);

        engine.register_type_with_name::<ScriptRGB>("RGB")
            .register_fn("rgb", ScriptRGB::new)
            .register_fn("rgba", ScriptRGB::new_with_alpha);

        engine.on_print(|x| println!("{}", x));

        Self {

            engine,
            scope               : Scope::new(),
            ast                 : None,

            draw2d              : Draw2D { scissor: None },
            asset,
            frame               : vec![0; 1024 * 608 * 4],
            width               : 1024,
            height              : 608,
            tile_size           : 32,

            regions             : HashMap::new(),
            lights              : HashMap::new(),

            messages            : vec![],

            last_position       : None,
            transition_steps    : 5,
            transition_counter  : 0,
            transition_active   : false,

            player_id,

            audio_engine        : None,
        }
    }

    pub fn process_update(&mut self, update: &GameUpdate) -> Option<(String, Option<usize>)> {

        // New screen script ?
        if let Some(screen_script) = &update.screen {

            let result = self.engine.compile_with_scope(&self.scope, screen_script.as_str());

            if result.is_ok() {
                if let Some(ast) = result.ok() {

                    self.messages = vec![];
                    self.last_position = None;
                    self.transition_active = false;

                    self.scope = Scope::new();
                    self.scope.set_value("width", 1024 as i64);
                    self.scope.set_value("height", 608 as i64);
                    self.scope.set_value("tile_size", 32 as i64);
                    self.scope.set_value("draw", ScriptDraw::new());
                    self.scope.set_value("cmd", ScriptCmd::new());
                    self.scope.set_value("message", ScriptMessageCmd::new());

                    let mut tilemaps = ScriptTilemaps::new();
                    for index in 0..self.asset.tileset.maps_names.len() {
                        tilemaps.maps.insert(self.asset.tileset.maps_names[index].clone(), self.asset.tileset.maps_ids[index]);
                    }
                    self.scope.set_value("tilemaps", tilemaps);

                    let result = self.engine.eval_ast_with_scope::<Dynamic>(&mut self.scope, &ast);
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

                    if let Some(width) = self.scope.get_value::<i64>("width") {
                        self.width = width as usize;
                    }
                    if let Some(height) = self.scope.get_value::<i64>("height") {
                        self.height = height as usize;
                    }
                    if let Some(tile_size) = self.scope.get_value::<i64>("tile_size") {
                        self.tile_size = tile_size as usize;
                    }

                    if self.frame.len() != self.width * self.height * 4 {
                        self.frame = vec![0; self.width * self.height * 4];
                    }

                    self.ast = Some(ast);

                    self.process_cmds(self.player_id);
                }
            } else
            if let Some(err) = result.err() {
                return Some((err.0.to_string(), err.1.line()));
            }
        }

        // Got a new region ?
        if let Some(region) = &update.region {
            self.regions.insert(region.id, region.clone());
        }

        // Get new messages
        if update.messages.is_empty() == false {
            for m in &update.messages {
                self.messages.push(m.clone());
            }
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

        None
    }

    /// Draw the server response
    pub fn draw(&mut self, anim_counter: usize, update: &GameUpdate) -> Option<(String, Option<usize>)> {

        let error = self.process_update(update);
        if error.is_some() {
            return error;
        }

        // Call the draw function
        if let Some(ast) = &self.ast {
            let result = self.engine.call_fn_raw(
                            &mut self.scope,
                            &ast,
                            false,
                            false,
                            "draw",
                            None,
                            []
                        );

            if result.is_err() {
                if let Some(err) = result.err() {
                    //println!("{:?}", err.,t);
                    let mut string = err.to_string();
                    let mut parts = string.split("(");
                    if let Some(first) = parts.next() {
                        string = first.to_owned();
                    }
                    return Some((string, err.position().line()));
                }
            }
        }

        if let Some(mut draw) = self.scope.get_value::<ScriptDraw>("draw") {

            //let game_frame = &mut self.frame[..];
            let stride = self.width;

            for cmd in &draw.commands {

                match cmd {
                    ScriptDrawCmd::DrawRect(rect, rgb) => {
                        if rect.is_safe(self.width, self.height) {
                            self.draw2d.draw_rect( &mut self.frame[..], &rect.rect, stride, &rgb.value);
                        }
                    },
                    ScriptDrawCmd::DrawTile(pos, tile) => {
                        //if rect.is_safe(self.width, self.height) {
                            if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                                self.draw2d.draw_animated_tile( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.x_off as usize, tile.id.y_off as usize), anim_counter, self.tile_size);
                            }
                        //}
                    },
                    ScriptDrawCmd::DrawTileSat(pos, tile, rgb) => {
                        //if rect.is_safe(self.width, self.height) {
                            if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                                self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.x_off as usize, tile.id.y_off as usize), anim_counter, self.tile_size, rgb.value);
                            }
                        //}
                    },
                    ScriptDrawCmd::DrawTileSized(pos, tile, size) => {
                        //if rect.is_safe(self.width, self.height) {
                            if let Some(map) = self.asset.get_map_of_id(tile.id.tilemap) {
                                self.draw2d.draw_animated_tile( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.x_off as usize, tile.id.y_off as usize), anim_counter, *size as usize);
                            }
                        //}
                    },
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
                                    if i == 0 { t = tile.id.clone(); }
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.x_off += 2; }
                                    else { t = tile.id.clone(); t.x_off += 1; }

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile( &mut self.frame[..], &(x, top_y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size);
                                    }

                                    let mut t;
                                    if i == 0 { t = tile.id.clone(); t.y_off += 2; }
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.x_off += 2; t.y_off += 2; }
                                    else { t = tile.id.clone(); t.x_off += 1; t.y_off += 2; }

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile( &mut self.frame[..], &(x, bottom_y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size);
                                    }

                                    x += self.tile_size;
                                }

                                let right_x = rect.rect.0 + rect.rect.2 - self.tile_size;

                                let mut y = rect.rect.1 + self.tile_size;
                                for _i in 0..tiles_y - 2 {
                                    let mut t = tile.id.clone(); t.y_off += 1;

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile( &mut self.frame[..], &(rect.rect.0, y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size);
                                    }

                                    let mut t = tile.id.clone(); t.x_off += 2; t.y_off += 1;

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile( &mut self.frame[..], &(right_x, y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size);
                                    }

                                    y += self.tile_size;
                                }
                            }
                        }
                    },
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
                                    if i == 0 { t = tile.id.clone(); }
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.x_off += 2; }
                                    else { t = tile.id.clone(); t.x_off += 1; }

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(x, top_y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size, rgb.value);
                                    }

                                    let mut t;
                                    if i == 0 { t = tile.id.clone(); t.y_off += 2; }
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.x_off += 2; t.y_off += 2; }
                                    else { t = tile.id.clone(); t.x_off += 1; t.y_off += 2; }

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(x, bottom_y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size, rgb.value);
                                    }

                                    x += self.tile_size;
                                }

                                let right_x = rect.rect.0 + rect.rect.2 - self.tile_size;

                                let mut y = rect.rect.1 + self.tile_size;
                                for _i in 0..tiles_y - 2 {
                                    let mut t = tile.id.clone(); t.y_off += 1;

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(rect.rect.0, y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size, rgb.value);
                                    }

                                    let mut t = tile.id.clone(); t.x_off += 2; t.y_off += 1;

                                    if let Some(map) = self.asset.get_map_of_id(t.tilemap) {
                                        self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(right_x, y), &map, stride, &(t.x_off as usize, t.y_off as usize), anim_counter, self.tile_size, rgb.value);
                                    }

                                    y += self.tile_size;
                                }
                            }
                        }
                    },
                    ScriptDrawCmd::DrawText(pos, text, font_name, size, rgb) => {
                        if let Some(font) = self.asset.game_fonts.get(font_name) {
                            self.draw2d.blend_text( &mut self.frame[..], &pos.pos, stride, font, *size, text, &rgb.value);
                        }
                    },
                    ScriptDrawCmd::DrawMessages(rect, font_name, size, rgb) => {
                        if let Some(font) = self.asset.game_fonts.get(font_name) {
                            let max_lines = (rect.rect.3) / (*size as usize);
                            let available_messages = self.messages.len();

                            for l in 0..max_lines {
                                if l >= available_messages {
                                    break;
                                }
                                self.draw2d.blend_text_rect(&mut self.frame[..], &(rect.rect.0, rect.rect.1 + rect.rect.3 - (l+1) * (*size as usize), rect.rect.2, *size as usize), stride, &font, *size, self.messages[available_messages - 1 - l].message.as_str(), &rgb.value, crate::draw2d::TextAlignment::Left);
                            }
                        }
                    },
                    ScriptDrawCmd::DrawGame(rect) => {
                        if rect.is_safe(self.width, self.height) {
                            //let frame = &mut self.frame[..];
                            self.process_game_draw(rect.rect, anim_counter, update, &mut None, self.width);
                        }
                    },
                    ScriptDrawCmd::DrawRegion(_name, _rect, _size) => {

                        /*
                        for (index, n) in self.regions_names.iter().enumerate() {
                            if n == name {
                                if let Some(region) = self.regions.get(&self.regions_ids[index]) {

                                    _ = self.draw2d.as_ref().unwrap().draw_region_content(game_frame, region, &rect.rect, stride, *size as usize, self.game_anim_counter, &self.asset.as_ref().unwrap());
                                }
                            }
                        }*/
                    }
                }
            }

            draw.clear();
            self.scope.set_value("draw", draw);
        }

        None
    }

    pub fn process_game_draw(&mut self, rect: (usize, usize, usize, usize), anim_counter: usize, update: &GameUpdate, external_frame: &mut Option<&mut [u8]>, stride: usize) {
        if let Some(position) = update.position.clone(){

            if self.transition_active == false {
                if self.last_position.is_some() && position.region != self.last_position.clone().unwrap().region {
                    // Start transition
                    self.transition_active = true;
                    self.transition_counter = 1;
                    self.transition_steps = 6;
                } else {
                    self.last_position = Some(position.clone());
                }
            }

            if self.transition_active {
                self.draw_game_rect(rect, self.last_position.clone().unwrap().clone(), anim_counter, update, None, external_frame, stride);

                let mut r = rect.clone();

                let mut set: HashSet<(isize, isize)> = HashSet::new();

                let x_tiles = rect.2 / self.tile_size;

                let step_x = (x_tiles as f32 / self.transition_steps as f32) as f32;

                r.0 = x_tiles / 2 - (((step_x * self.transition_counter as f32) / 2.0)) as usize;
                r.2 = (step_x * self.transition_counter as f32) as usize;

                for y in 0..r.3 {
                    for x in r.0..r.0+r.2 {
                        set.insert((x as isize, y as isize));
                    }
                }

                self.draw_game_rect(rect, position.clone(), anim_counter, update, Some(set), external_frame, stride);

                self.transition_counter += 1;
                if self.transition_counter == self.transition_steps {
                    self.transition_active = false;
                    self.last_position = Some(position.clone());
                }
            } else
            if self.transition_active == false {
                self.draw_game_rect(rect, position.clone(), anim_counter, update, None, external_frame, stride);
            }
        }
    }

    /// Draws the game in the given rect
    pub fn draw_game_rect(&mut self, rect: (usize, usize, usize, usize), cposition: Position, anim_counter: usize, update: &GameUpdate, set: Option<HashSet<(isize, isize)>>, external_frame: &mut Option<&mut [u8]>, stride: usize) {

        self.draw2d.scissor = Some(rect);

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

            // Compute the light_map
            let mut light_map : HashMap<(isize, isize), f64> = HashMap::new();
            if let Some(lights) = self.lights.get(&region.id) {
                light_map = compute_lighting(&region, lights);
            }

            // Clear if not in a transition
            if set.is_none() {
                self.draw2d.draw_rect(&mut self.frame[..], &rect, self.width, &background);
            }

            let mut offset = (0_isize, 0_isize);

            let mut gr = (0, 0);

            if let Some(old_position) = &update.old_position {

                let t = (update.curr_transition_time as f64 * (self.tile_size as f64 / (update.max_transition_time as f64 + 1.0))) as isize;

                if position.x > old_position.x {
                    gr.0 = t;
                } else
                if position.x < old_position.x {
                    gr.0 = -t;
                }
                if position.y > old_position.y {
                    gr.1 = t;
                } else
                if position.y < old_position.y {
                    gr.1 = -t;
                }

                position = old_position.clone();
            }

            offset.0 = position.x;
            offset.1 = position.y;

            let region_width = region.max_pos.0 - region.min_pos.0;
            let region_height = region.max_pos.1 - region.min_pos.1;

            if region_width * tile_size as isize  <= rect.2 as isize {
                offset.0 = region.min_pos.0;
                gr.0 = 0;
            } else {
                let left = x_tiles / 2;
                offset.0 -= left;
            }

            if region_height * tile_size as isize  <= rect.3 as isize {
                gr.1 = 0;
                offset.1 = region.min_pos.1;
            } else {
                let top = y_tiles / 2;
                offset.1 -= top;
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

            let base_light = 0.5;

            // Draw Region

            for y in from_y..y_tiles {
                for x in from_x..x_tiles {

                    let values = self.get_region_value(region, (x + offset.0 as isize, y + offset.1), update);
                    for value in values {
                        let pos = (rect.0 + left_offset + (x * tile_size as isize - gr.0) as usize, rect.1 + top_offset + (y * tile_size as isize - gr.1) as usize);

                        if let Some(set) = &set {
                            if set.contains(&(x, y)) == false {
                                continue;
                            }
                        }

                        let frame;
                        if external_frame.is_some() {
                            frame = external_frame.as_deref_mut().unwrap();
                        } else {
                            frame = &mut self.frame[..];
                        }

                        if set.is_some() {
                            self.draw2d.draw_rect(frame/*if external_frame.is_some() { &mut(external_frame.as_deref_mut().unwrap()) } else { &mut self.frame[..]}*/, &(pos.0, pos.1, tile_size, tile_size), stride, &background);
                            //self.draw2d.draw_rect(&mut self.frame[..], &(pos.0, pos.1, tile_size, tile_size), stride, &clear.unwrap());
                        }

                        let mut light = base_light;
                        if let Some(l) = light_map.get(&(x + offset.0, y + offset.1)) {
                            light += *l;
                        }

                        if let Some(map) = self.asset.get_map_of_id(value.tilemap) {
                            self.draw2d.draw_animated_tile_with_blended_color(frame/*&mut self.frame[..]*/, &pos, map, stride, &(value.x_off as usize, value.y_off as usize), anim_counter, tile_size, &background, light);
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

                    if character.id != update.id || (character.id == update.id && gr.0 == 0 && gr.1 == 0) {
                        let t = (character.curr_transition_time as f64 * (self.tile_size as f64 / (character.max_transition_time as f64 + 1.0))) as isize;

                        if position.x > old_position.x {
                            tr.0 = t;
                        } else
                        if position.x < old_position.x {
                            tr.0 = -t;
                        }

                        if position.y > old_position.y {
                            tr.1 = t;
                        } else
                        if position.y < old_position.y {
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
                        let pos = (rect.0 + left_offset + (((position.x - offset.0) * tile_size as isize) + tr.0) as usize, rect.1 + top_offset + ((position.y - offset.1) * tile_size as isize + tr.1) as usize);

                        if let Some(set) = &set {
                            if set.contains(&(((pos.0 - rect.0) / self.tile_size) as isize, ((pos.1 - rect.1) / self.tile_size) as isize)) == false {
                                continue;
                            }
                        }

                        let mut light = base_light;
                        if let Some(l) = light_map.get(&(position.x, position.y)) {
                            light += *l;
                        }

                        if let Some(map) = self.asset.get_map_of_id(tile.tilemap) {
                            self.draw2d.draw_animated_tile_with_blended_color(frame, &pos, map, stride, &(tile.x_off as usize, tile.y_off as usize), anim_counter, tile_size, &background, light);
                        }
                    }
                }
            }
        } else {
            println!("Region not found");
        }

        self.draw2d.scissor = None;
    }

    /// Gets the given region value
    pub fn get_region_value(&self, region: &GameRegionData, pos: (isize, isize), update: &GameUpdate) -> Vec<TileData> {
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

    pub fn key_down(&mut self, key: String, player_id: Uuid) -> (Vec<String>, Option<(String, Option<usize>)>) {
        // Call the draw function
        if let Some(ast) = &self.ast {
            let result = self.engine.call_fn_raw(
                            &mut self.scope,
                            &ast,
                            false,
                            false,
                            "key_down",
                            None,
                            [key.into()]
                        );

            if result.is_err() {
                if let Some(err) = result.err() {
                    //println!("{:?}", err.,t);
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

    pub fn mouse_down(&mut self, pos: (usize, usize), player_id: Uuid) -> (Vec<String>, Option<(String, Option<usize>)>) {
        // Call the draw function

        if let Some(ast) = &self.ast {
            let result = self.engine.call_fn_raw(
                            &mut self.scope,
                            &ast,
                            false,
                            false,
                            "touch_down",
                            None,
                            [(pos.0 as i64).into(), (pos.1 as i64).into()]
                        );

            if result.is_err() {
                if let Some(err) = result.err() {
                    //println!("{:?}", err.,t);
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


    fn process_cmds(&mut self, player_id: Uuid) -> Vec<String> {
        let mut commands = vec![];

        if let Some(mut cmd) = self.scope.get_value::<ScriptCmd>("cmd") {

            for cmd in &cmd.commands {

                match cmd {
                    ScriptServerCmd::Move(direction) => {
                        let mut dir : Option<PlayerDirection> = None;

                        if direction == "west" {
                            dir = Some(PlayerDirection::West);
                        } else
                        if direction == "north" {
                            dir = Some(PlayerDirection::North);
                        } else
                        if direction == "east" {
                            dir = Some(PlayerDirection::East);
                        } else
                        if direction == "south" {
                            dir = Some(PlayerDirection::South);
                        }

                        if let Some(dir) = dir {
                            if let Some(action) = pack_action(player_id, "onMove".to_string(), dir, "".to_string()) {
                                commands.push(action);
                            }
                        }
                    }
                }
            }

            cmd.clear();
            self.scope.set_value("cmd", cmd);
        }

        if let Some(mut cmd) = self.scope.get_value::<ScriptMessageCmd>("message") {

            for cmd in &cmd.messages {

                match cmd {
                    ScriptMessage::Status(message) => {
                        self.messages.push(MessageData { message_type: core_shared::message::MessageType::Status, message: message.clone(), from: "System".to_string() });
                    }
                }
            }

            cmd.clear();
            self.scope.set_value("message", cmd);
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
                    if let Some(file) = std::fs::File::open(self.asset.audio_paths[index].clone()).ok() {
                        if let Some(audio_engine) = &self.audio_engine {

                            let buffered = std::io::BufReader::new(file);

                            if let Some(wav) = WavDecoder::new(buffered).ok() {
                                if let Some(mut sound) = audio_engine.new_sound_with_group(Group::Effect, wav).ok() {
                                    sound.play();
                                    //audio_engine.set_group_volume(Group::Effect, 0.1);
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
                    if let Some(bytes) = Embedded::get(self.asset.audio_paths[index].to_str().unwrap()) {
                        if let Some(audio_engine) = &self.audio_engine {

                            let buffered = std::io::BufReader::new(std::io::Cursor::new(bytes.data));

                            if let Some(wav) = WavDecoder::new(buffered).ok() {

                                if let Some(mut sound) = audio_engine.new_sound_with_group(Group::Effect, wav).ok() {
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