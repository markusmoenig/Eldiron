use std::{path::PathBuf, collections::HashMap};

use core_shared::{asset::{Asset, TileUsage}, update::GameUpdate, regiondata::GameRegionData};

use crate::draw2d::Draw2D;

pub struct GameRender {

    draw2d                      : Draw2D,
    asset                       : Asset,

    pub frame                   : Vec<u8>,
    pub width                   : usize,
    pub height                  : usize,

    pub regions                 : HashMap<usize, GameRegionData>
}

impl GameRender {

    pub fn new(path: PathBuf) -> Self {

        let mut asset = Asset::new();
        asset.load_from_path(path);

        Self {

            draw2d              : Draw2D {},
            asset,
            frame               : vec![0; 800 * 600 * 4],
            width               : 800,
            height              : 600,

            regions             : HashMap::new()
        }
    }

    pub fn draw(&mut self, anim_counter: usize, update: &GameUpdate) {
        //println!("{:?}", update.displacements.len());

        // Got a new region ?
        if let Some(region) = &update.region {
            //println!("got region {:?}", region.id);
            self.regions.insert(region.id, region.clone());
        }

        self.draw_game_rect((0, 0, 800, 600), anim_counter, update);
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