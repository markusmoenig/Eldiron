
use std::{path::PathBuf, collections::HashMap};

use core_shared::{asset::{Asset, TileUsage}, update::GameUpdate, regiondata::GameRegionData};

use crate::{draw2d::Draw2D, script_types::*};

use rhai::{ Engine, Scope, AST, Dynamic };

pub struct GameRender<'a> {

    engine                      : Engine,
    scope                       : Scope<'a>,
    ast                         : Option<AST>,

    draw2d                      : Draw2D,
    asset                       : Asset,

    pub frame                   : Vec<u8>,
    pub width                   : usize,
    pub height                  : usize,

    pub regions                 : HashMap<usize, GameRegionData>
}

impl GameRender<'_> {

    pub fn new(path: PathBuf) -> Self {

        let mut asset = Asset::new();
        asset.load_from_path(path);

        let mut engine = Engine::new();

        engine.register_type::<ScriptDraw>()
            .register_fn("rect", ScriptDraw::rect)
            .register_fn("game", ScriptDraw::game)
            .register_fn("region", ScriptDraw::region)
            .register_fn("text", ScriptDraw::text);

        engine.register_type::<ScriptRect>()
            .register_fn("rect", ScriptRect::new);

        engine.register_type::<ScriptPosition>()
            .register_fn("pos2d", ScriptPosition::new);

        engine.register_type::<ScriptRect>()
            .register_fn("rgb", ScriptRGB::new)
            .register_fn("rgba", ScriptRGB::new_with_alpha);

        Self {

            engine,
            scope               : Scope::new(),
            ast                 : None,

            draw2d              : Draw2D {},
            asset,
            frame               : vec![0; 800 * 600 * 4],
            width               : 800,
            height              : 600,

            regions             : HashMap::new()
        }
    }

    pub fn process_update(&mut self, update: &GameUpdate) {
        //println!("{:?}", update.displacements.len());

        // New screen script ?
        if let Some(screen_script) = &update.screen {
            println!("Script {}", screen_script);
            if let Some(ast) = self.engine.compile_with_scope(&self.scope, screen_script.as_str()).ok() {
                self.scope = Scope::new();
                self.scope.set_value("width", 1024 as i64);
                self.scope.set_value("height", 608 as i64);
                self.scope.set_value("tile_size", 32 as i64);
                let r = self.engine.eval_ast_with_scope::<Dynamic>(&mut self.scope, &ast);
                self.ast = Some(ast);
            }
        }

        // Got a new region ?
        if let Some(region) = &update.region {
            //println!("got region {:?}", region.id);
            self.regions.insert(region.id, region.clone());
        }
    }

    pub fn draw(&mut self, anim_counter: usize, update: &GameUpdate) {

        self.process_update(update);

        // Call the draw function
        if let Some(ast) = &self.ast {
            let r = self.engine.eval_ast_with_scope::<Dynamic>(&mut self.scope, &ast);
        }

        if let Some(mut draw) = self.scope.get_value::<ScriptDraw>("draw") {


            let game_frame = &mut self.frame[..];
            let stride = self.width;

            for cmd in &draw.commands {

                match cmd {
                    ScriptDrawCmd::DrawRect(rect, rgb) => {
                        self.draw2d.draw_rect(game_frame, &rect.rect, stride, &rgb.value);
                    },
                    ScriptDrawCmd::DrawText(pos, font_name, text, size, rgb) => {
                        if let Some(font) = self.asset.game_fonts.get(font_name) {
                            self.draw2d.blend_text(game_frame, &pos.pos, stride, font, *size, text, &rgb.value);
                        }
                    },
                    ScriptDrawCmd::DrawGame(rect, size) => {
                        //draw2d.draw_rect(game_frame, &rect.rect, stride, &rgb.value);

                        /*
                        let region_id = self.regions_ids[0];

                        if let Some(region) = self.regions.get(&region_id) {
                            // Find the behavior instance for the current behavior id
                            let mut inst_index = 0_usize;
                            let behavior_id = self.behaviors_ids[0];
                            for index in 0..self.instances.len() {
                                if self.instances[index].behavior_id == behavior_id {
                                    inst_index = index;
                                    break;
                                }
                            }

                            _ = self.draw2d.as_ref().unwrap().draw_region_centered_with_instances(game_frame, region, &rect.rect, inst_index, stride, *size as usize, self.game_anim_counter, &self.asset.as_ref().unwrap(), &self.instances);

                        }*/
                    },
                    ScriptDrawCmd::DrawRegion(name, rect, size) => {
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
        //self.draw_game_rect((0, 0, 800, 600), anim_counter, update);

    }

    pub fn draw_game_rect(&mut self, rect: (usize, usize, usize, usize), anim_counter: usize, update: &GameUpdate) {

        self.draw2d.draw_rect(&mut self.frame[..], &rect, self.width, &[0, 0, 0, 255]);

        let stride = self.width;
        let tile_size = 32;

        let left_offset = (rect.2 % tile_size) / 2;
        let top_offset = (rect.3 % tile_size) / 2;

        let x_tiles = (rect.2 / tile_size) as isize;
        let y_tiles = (rect.3 / tile_size) as isize;

        let mut center = (0, 0);
        if let Some(position) = update.position {

            if let Some(region) = self.regions.get(&position.0) {
                center.0 = position.1;
                center.1 = position.2;

                let mut offset = center.clone();

                offset.0 -= x_tiles / 2;
                offset.1 -= y_tiles / 2;

                // Draw Region
                for y in 0..y_tiles {
                    for x in 0..x_tiles {

                        let values = self.get_region_value(region, (x + offset.0, y + offset.1), update);
                        for value in values {
                            let pos = (rect.0 + left_offset + (x as usize) * tile_size, rect.1 + top_offset + (y as usize) * tile_size);

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

                            let map = self.asset.get_map_of_id(tile.0);
                            self.draw2d.draw_animated_tile(&mut self.frame[..], &pos, map, stride, &(tile.1, tile.2), anim_counter, tile_size);
                        }
                    }
                }
            }
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
}