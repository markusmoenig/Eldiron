
use std::{path::PathBuf, collections::{HashMap, HashSet}};

use core_shared::{asset::{Asset, TileUsage}, update::GameUpdate, regiondata::GameRegionData, message::MessageData};
use crate::{draw2d::Draw2D, script_types::*};
use rhai::{ Engine, Scope, AST, Dynamic };

use core_shared::actions::*;

#[cfg(target_arch = "wasm32")]
use audio_engine::{AudioEngine, WavDecoder};

#[cfg(feature = "embed_binaries")]
#[allow(unused_imports)]
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

    pub regions                 : HashMap<usize, GameRegionData>,

    pub messages                : Vec<MessageData>,

    pub last_position           : (usize, isize, isize),
    pub transition_steps        : isize,
    pub transition_counter      : isize,
    pub transition_active       : bool,

    pub player_id               : usize,

    #[cfg(target_arch = "wasm32")]
    pub audio_engine            : Option<AudioEngine>
}

impl GameRender<'_> {

    #[allow(unused)]
    pub fn new(path: PathBuf, player_id: usize) -> Self {

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

        engine.register_type_with_name::<ScriptRect>("Rect")
            .register_fn("rgb", ScriptRGB::new)
            .register_fn("rgba", ScriptRGB::new_with_alpha);

        engine.on_print(|x| println!("{}", x));

        #[cfg(target_arch = "wasm32")]
        let mut audio_engine : Option<AudioEngine> = None;

        #[cfg(target_arch = "wasm32")]
        if let Some(audio) = AudioEngine::new().ok() {
            audio_engine = Some(audio);
        }

        Self {

            engine,
            scope               : Scope::new(),
            ast                 : None,

            draw2d              : Draw2D {},
            asset,
            frame               : vec![0; 1024 * 608 * 4],
            width               : 1024,
            height              : 608,
            tile_size           : 32,

            regions             : HashMap::new(),

            messages            : vec![],

            last_position       : (100000, 0, 0),
            transition_steps    : 5,
            transition_counter  : 0,
            transition_active   : false,

            player_id,

            #[cfg(target_arch = "wasm32")]
            audio_engine
        }
    }

    pub fn process_update(&mut self, update: &GameUpdate) -> Option<(String, Option<usize>)> {

        // New screen script ?
        if let Some(screen_script) = &update.screen {

            let result = self.engine.compile_with_scope(&self.scope, screen_script.as_str());

            if result.is_ok() {
                if let Some(ast) = result.ok() {

                    self.messages = vec![];
                    self.last_position = (100000, 0, 0);
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
                        tilemaps.maps.insert(self.asset.tileset.maps_names[index].clone(), self.asset.tileset.maps_ids[index] as i64);
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
                            let map = self.asset.get_map_of_id(tile.id.0);
                            self.draw2d.draw_animated_tile( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.1, tile.id.2), anim_counter, self.tile_size);
                        //}
                    },
                    ScriptDrawCmd::DrawTileSat(pos, tile, rgb) => {
                        //if rect.is_safe(self.width, self.height) {
                            let map = self.asset.get_map_of_id(tile.id.0);
                            self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.1, tile.id.2), anim_counter, self.tile_size, rgb.value);
                        //}
                    },
                    ScriptDrawCmd::DrawTileSized(pos, tile, size) => {
                        //if rect.is_safe(self.width, self.height) {
                            let map = self.asset.get_map_of_id(tile.id.0);
                            self.draw2d.draw_animated_tile( &mut self.frame[..], &(pos.pos.0, pos.pos.1), &map, stride, &(tile.id.1, tile.id.2), anim_counter, *size as usize);
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
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.1 += 2; }
                                    else { t = tile.id.clone(); t.1 += 1; }

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile( &mut self.frame[..], &(x, top_y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size);

                                    let mut t;
                                    if i == 0 { t = tile.id.clone(); t.2 += 2; }
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.1 += 2; t.2 += 2; }
                                    else { t = tile.id.clone(); t.1 += 1; t.2 += 2; }

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile( &mut self.frame[..], &(x, bottom_y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size);

                                    x += self.tile_size;
                                }

                                let right_x = rect.rect.0 + rect.rect.2 - self.tile_size;

                                let mut y = rect.rect.1 + self.tile_size;
                                for _i in 0..tiles_y - 2 {
                                    let mut t = tile.id.clone(); t.2 += 1;

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile( &mut self.frame[..], &(rect.rect.0, y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size);

                                    let mut t = tile.id.clone(); t.1 += 2; t.2 += 1;

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile( &mut self.frame[..], &(right_x, y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size);

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
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.1 += 2; }
                                    else { t = tile.id.clone(); t.1 += 1; }

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(x, top_y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size, rgb.value);

                                    let mut t;
                                    if i == 0 { t = tile.id.clone(); t.2 += 2; }
                                    else if i == tiles_x - 1 { t = tile.id.clone(); t.1 += 2; t.2 += 2; }
                                    else { t = tile.id.clone(); t.1 += 1; t.2 += 2; }

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(x, bottom_y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size, rgb.value);

                                    x += self.tile_size;
                                }

                                let right_x = rect.rect.0 + rect.rect.2 - self.tile_size;

                                let mut y = rect.rect.1 + self.tile_size;
                                for _i in 0..tiles_y - 2 {
                                    let mut t = tile.id.clone(); t.2 += 1;

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(rect.rect.0, y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size, rgb.value);

                                    let mut t = tile.id.clone(); t.1 += 2; t.2 += 1;

                                    let map = self.asset.get_map_of_id(t.0);
                                    self.draw2d.draw_animated_tile_sat( &mut self.frame[..], &(right_x, y), &map, stride, &(t.1, t.2), anim_counter, self.tile_size, rgb.value);

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
                            if let Some(position) = update.position {

                                if self.transition_active == false {
                                    if position.0 != self.last_position.0 && self.last_position.0 != 100000 {
                                        // Start transition
                                        self.transition_active = true;
                                        self.transition_counter = 1;
                                        self.transition_steps = 6;
                                    } else {
                                        self.last_position = position;
                                    }
                                }


                                if self.transition_active {
                                    self.draw_game_rect(rect.rect, self.last_position, anim_counter, update, None, None);

                                    let mut r = rect.rect.clone();

                                    let mut set: HashSet<(isize, isize)> = HashSet::new();

                                    let x_tiles = rect.rect.2 / self.tile_size;

                                    let step_x = (x_tiles as f32 / self.transition_steps as f32) as f32;

                                    r.0 = x_tiles / 2 - (((step_x * self.transition_counter as f32) / 2.0)) as usize;
                                    r.2 = (step_x * self.transition_counter as f32) as usize;

                                    for y in 0..r.3 {
                                        for x in r.0..r.0+r.2 {
                                            set.insert((x as isize, y as isize));
                                        }
                                    }

                                    self.draw_game_rect(rect.rect, position, anim_counter, update, Some([0, 0, 0, 255]), Some(set));

                                    self.transition_counter += 1;
                                    if self.transition_counter == self.transition_steps {
                                        self.transition_active = false;
                                        self.last_position = position;
                                    }
                                } else
                                if self.transition_active == false {
                                    self.draw_game_rect(rect.rect, position, anim_counter, update, None, None);
                                }
                            }
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

    /// Draws the game in the given rect
    pub fn draw_game_rect(&mut self, rect: (usize, usize, usize, usize), position: (usize, isize, isize), anim_counter: usize, update: &GameUpdate, clear:  Option<[u8; 4]>, set: Option<HashSet<(isize, isize)>>) {

        //self.draw2d.draw_rect(&mut self.frame[..], &rect, self.width, &[0, 0, 0, 255]);

        let stride = self.width;
        let tile_size = self.tile_size;

        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        if let Some(region) = self.regions.get(&position.0) {

            let mut offset = (0, 0);
            offset.0 = position.1;
            offset.1 = position.2;

            let region_width = region.max_pos.0 - region.min_pos.0;
            let region_height = region.max_pos.1 - region.min_pos.1;

            if region_width * tile_size as isize  <= rect.2 as isize {
                offset.0 = region.min_pos.0;
            } else {
                let left = x_tiles / 2;
                offset.0 -= left;
            }

            if region_height * tile_size as isize  <= rect.3 as isize {
                offset.1 = region.min_pos.1;
            } else {
                let top = y_tiles / 2;
                offset.1 -= top;
            }

            // Draw Region
            for y in 0..y_tiles {
                for x in 0..x_tiles {

                    let values = self.get_region_value(region, (x + offset.0, y + offset.1), update);
                    for value in values {
                        let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

                        if let Some(set) = &set {
                            if set.contains(&(x, y)) == false {
                                continue;
                            }
                        }

                        if clear.is_some() {
                            self.draw2d.draw_rect(&mut self.frame[..], &(pos.0, pos.1, tile_size, tile_size), stride, &clear.unwrap());
                        }

                        let map = self.asset.get_map_of_id(value.0);
                        self.draw2d.draw_animated_tile(&mut self.frame[..], &pos, map, stride, &(value.1, value.2), anim_counter, tile_size);
                    }
                }
            }

            // Draw Characters
            for character in &update.characters {

                let position = character.position;
                let tile = character.tile;

                // Row check
                if position.1 >= offset.0 && position.1 < offset.0 + x_tiles {
                    // Column check
                    if position.2 >= offset.1 && position.2 < offset.1 + y_tiles {
                        // Visible
                        let pos = (rect.0 + left_offset + ((position.1 - offset.0) as usize) * tile_size, rect.1 + top_offset + ((position.2 - offset.1) as usize) * tile_size);

                        if let Some(set) = &set {
                            if set.contains(&(((pos.0 - rect.0) / self.tile_size) as isize, ((pos.1 - rect.1) / self.tile_size) as isize)) == false {
                                continue;
                            }
                        }

                        let map = self.asset.get_map_of_id(tile.0);
                        self.draw2d.draw_animated_tile(&mut self.frame[..], &pos, map, stride, &(tile.1, tile.2), anim_counter, tile_size);
                    }
                }
            }
        } else {
            println!("Region not found");
        }

    }

    /// Gets the given region value
    pub fn get_region_value(&self, region: &GameRegionData, pos: (isize, isize), update: &GameUpdate) -> Vec<(usize, usize, usize, TileUsage)> {
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

    pub fn key_down(&mut self, key: String, player_id: usize) -> (Vec<String>, Option<(String, Option<usize>)>) {
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

    pub fn mouse_down(&mut self, pos: (usize, usize), player_id: usize) -> (Vec<String>, Option<(String, Option<usize>)>) {
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


    fn process_cmds(&mut self, player_id: usize) -> Vec<String> {
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
            use rodio::{Decoder, OutputStream, Sink};

            for (index, n) in self.asset.audio_names.iter().enumerate() {
                if *n == name {

                    let file = std::io::BufReader::new(std::fs::File::open(self.asset.audio_paths[index].clone()).unwrap());

                    let handle = std::thread::spawn(move || {

                        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                        let sink = Sink::try_new(&stream_handle).unwrap();

                        sink.set_volume(0.5);
                        let source = Decoder::new(file).unwrap();

                        sink.append(source);
                        sink.sleep_until_end();
                    });
                }

                break;
            }
        }

        #[cfg(feature = "embed_binaries")]
        {
            // for (index, n) in self.asset.audio_names.iter().enumerate() {
            //     if *n == name {
            //         if let Some(bytes) = Embedded::get(self.asset.audio_paths[index].to_str().unwrap()) {
            //             if let Some(audio_engine) = &self.audio_engine {
            //                 if let Some(mut sound) = audio_engine.new_sound(WavDecoder::new(std::io::Cursor::new(bytes.data))).ok() {
            //                     sound.play();
            //                 }
            //             }
            //         }
            //     }
            // }
        }

    }

}