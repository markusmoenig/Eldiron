use crate::prelude::*;

use crate::widget::*;

use crate::optionsgrid::OptionsGridWidget;
use crate::asset::Asset;
use crate::tab::TabWidget;

use crate::tileset::TileUsage;

//use core::cell::Cell;

pub struct TileSelectorWidget {
    rect                    : (u32, u32, u32, u32),
    options_grid            : OptionsGridWidget,
    tab_widget              : TabWidget,
    tiles                   : Option<Vec<(u32, u32, u32)>>,
    
    helper                  : TileSelectorHelper
}

impl Widget for TileSelectorWidget {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32), asset: &Asset) -> Self where Self: Sized {

        let options_grid = OptionsGridWidget::new(vec!["Environment".to_string(), "EnvBlocking".to_string(), "Character".to_string(), "UtilityChar".to_string(), "Water".to_string(), "Harmful".to_string()], 
             (rect.0 + 100, rect.1, rect.2 - 200, UI_ELEMENT_HEIGHT), asset);

        Self {
            rect,
            options_grid                : options_grid,    
            tab_widget                  : TabWidget::new(vec!(),(rect.0, rect.1 + UI_ELEMENT_HEIGHT, rect.2, rect.3 - UI_ELEMENT_HEIGHT - 1), asset),
            tiles                       : None,
            helper                      : TileSelectorHelper {}
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {

        // Collect tiles
        if self.tiles == None {
            let mut tiles: Vec<(u32, u32, u32)> = vec![];

            for (_index, map) in &asset.tileset.maps {
                let amount = map.max_tiles();
                for offset in 0..amount {
                    let id = map.offset_to_id(offset);
                    let tile = map.get_tile(id);
                    if tile.usage != TileUsage::Unused {
                        tiles.push((map.settings.id, id.0, id.1 ));
                    }
                }
            }

            println!("dd {}", tiles.len());
            self.tiles = Some(tiles);
        }

        if let Some(tiles) = &self.tiles {

            let grid = self.helper.get_available_grid(self);
            let grid_size = self.helper.get_grid_size();
            let tiles_per_page = grid.0 * grid.1;

            let mut x = self.rect.0;
            let mut y = self.rect.1 + UI_ELEMENT_HEIGHT;

            for offset in 0..tiles_per_page {
                if offset < tiles.len() as u32 {
                    let id = tiles[offset as usize];

                    let mut map_grid_size = 16;
                    if let Some(map) = asset.tileset.maps.get(&id.0) {
                        map_grid_size = map.settings.grid_size;
                    }

                    asset.draw_tile(frame, &(x, y), id.0, &(id.1, id.2), grid_size as f32 / map_grid_size as f32);

                    x += grid_size;
                    if x >= self.rect.0 + self.rect.2 {
                        x = self.rect.0;
                        y += grid_size;
                    }
                }
            }
        }

        self.options_grid.draw(frame, anim_counter, asset);
        self.tab_widget.draw(frame, anim_counter, asset);
    }

    fn mouse_down(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;
        if self.options_grid.mouse_down(pos, asset) {
            consumed = true;
        }
        consumed
    }

    fn mouse_up(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;
        if self.options_grid.mouse_down(pos, asset) {
            consumed = true;
        }
        consumed
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}

struct TileSelectorHelper;

impl TileSelectorHelper {
    pub fn get_available_grid(&self, tilesel_widget: &TileSelectorWidget) -> (u32, u32) {
        let grid_size = self.get_grid_size();
        (tilesel_widget.rect.2 / grid_size, (tilesel_widget.rect.3 - UI_ELEMENT_HEIGHT) / grid_size)
    }

    pub fn get_grid_size(&self) -> u32 {
        32
    }
}