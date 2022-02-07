use crate::prelude::*;

use crate::widget::*;

use crate::optionsgrid::OptionsGridWidget;
use crate::asset::Asset;
use crate::tab::{ TabWidget, TabWidgetHelper };

use crate::tileset::TileUsage;

//use core::cell::Cell;
#[derive(Clone, PartialEq)]
pub enum TileSelectorUsage {
    All,
    Environment,
    Character,
}

pub struct TileSelectorWidget {
    rect                    : (u32, u32, u32, u32),
    options_grid            : OptionsGridWidget,
    tab_widget              : TabWidget,
    tab_helper              : TabWidgetHelper,
    tiles                   : Option<Vec<(u32, u32, u32)>>,
    
    usage                   : TileSelectorUsage,
    helper                  : TileSelectorHelper,

    pub selected            : Option<(u32, u32, u32)>
}

impl Widget for TileSelectorWidget {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32), asset: &Asset) -> Self where Self: Sized {

        let options_grid = OptionsGridWidget::new(vec!["Environment".to_string(), "EnvBlocking".to_string(), "Character".to_string(), "UtilityChar".to_string(), "Water".to_string(), "Harmful".to_string()], 
             (rect.0 + 100, rect.1, rect.2 - 200, UI_ELEMENT_HEIGHT), asset);

        Self {
            rect,
            options_grid                : options_grid,    
            tab_widget                  : TabWidget::new(vec!(),(rect.0, rect.1 + UI_ELEMENT_HEIGHT, rect.2, rect.3 - UI_ELEMENT_HEIGHT - 1), asset),
            tab_helper                  : TabWidgetHelper {},
            tiles                       : None,
            usage                       : TileSelectorUsage::All,
            helper                      : TileSelectorHelper {},
            selected                    : None,
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {

        let grid = self.helper.get_available_grid(self);
        let grid_size = self.helper.get_grid_size();
        let tiles_per_page = grid.0 * grid.1;

        // Collect tiles
        if self.tiles == None {
            let mut tiles: Vec<(u32, u32, u32)> = vec![];

            for (_index, map) in &asset.tileset.maps {
                let amount = map.max_tiles();
                for offset in 0..amount {
                    let id = map.offset_to_id(offset);
                    let tile = map.get_tile(id);
                    let sel_index = self.options_grid.selected_index;

                    if self.usage == TileSelectorUsage::All {
                        if tile.usage == TileUsage::Environment && sel_index == 0 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::EnvBlocking && sel_index == 1 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::Character && sel_index == 2 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::UtilityChar && sel_index == 3 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::Water && sel_index == 4 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::Harmful && sel_index == 5 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        }                               
                    } else
                    if self.usage == TileSelectorUsage::Environment {
                        if tile.usage == TileUsage::Environment && sel_index == 0 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::EnvBlocking && sel_index == 1 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::Water && sel_index == 2 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::Harmful && sel_index == 3 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        }                               
                    }  else
                    if self.usage == TileSelectorUsage::Character {
                        if tile.usage == TileUsage::Character && sel_index == 0 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        } else
                        if tile.usage == TileUsage::UtilityChar && sel_index == 1 {
                            tiles.push((map.settings.id, id.0, id.1 ));
                        }                             
                    }                                        
                }
            }

            //println!("dd {}", tiles.len());
            self.tab_helper.set_pagination(&mut self.tab_widget, tiles.len() as u32 / tiles_per_page);
            self.tiles = Some(tiles);
        }

        if let Some(tiles) = &self.tiles {

            let mut x = self.rect.0;
            let mut y = self.rect.1 + UI_ELEMENT_HEIGHT;

            let page_offset = self.tab_widget.curr_page.get() * tiles_per_page;

            for offset in page_offset..page_offset + tiles_per_page {
                if offset < tiles.len() as u32 {
                    let id = tiles[offset as usize];

                    let mut map_grid_size = 16;
                    if let Some(map) = asset.tileset.maps.get(&id.0) {
                        map_grid_size = map.settings.grid_size;
                    }

                    asset.draw_tile(frame, &(x, y), id.0, &(id.1, id.2), grid_size as f32 / map_grid_size as f32);

                    if let Some(selected) = self.selected {
                        if selected == id {
                            asset.draw_rect_outline(frame, &(x, y, grid_size, grid_size), self.get_color_text());
                        }
                    }

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
            if self.options_grid.clicked == true {
                self.tiles = None;
                self.tab_widget.curr_page.set(0);
            }
        }
        if consumed == false {
            // Pages
            if self.tab_widget.mouse_down(pos, asset) {
                consumed = true;
            }
        }
        if consumed == false {
            if self.tab_widget.contains_pos_for(pos, self.tab_widget.get_content_rect()) {

                let grid = self.helper.get_available_grid(self);
                let grid_size = self.helper.get_grid_size();
                let tiles_per_page = grid.0 * grid.1;

                let x = (pos.0 - self.tab_widget.get_content_rect().0) / grid_size;
                let y = (pos.1 - self.tab_widget.get_content_rect().1) / grid_size;

                //println!("x y {} {}", x, y);
                let offset = x + y * grid.0 + self.tab_widget.curr_page.get() * tiles_per_page;

                if let Some(tiles) = &self.tiles {

                    //println!("offset {}", offset);
                    if offset < tiles.len() as u32 {

                        self.selected = Some(tiles[offset as usize]);

                        //println!("sel {:?}", self.selected);
                        return true;
                    }
                }
            }
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

pub struct TileSelectorHelper;

impl TileSelectorHelper {

    pub fn set_usage(&self, tilesel_widget: &mut TileSelectorWidget, usage: TileSelectorUsage) {
        if usage == TileSelectorUsage::All {
            tilesel_widget.options_grid.text = vec!["Environment".to_string(), "EnvBlocking".to_string(), "Character".to_string(), "UtilityChar".to_string(), "Water".to_string(), "Harmful".to_string()];
        }
        if usage == TileSelectorUsage::Environment {
            tilesel_widget.options_grid.text = vec!["Environment".to_string(), "EnvBlocking".to_string(), "Water".to_string(), "Harmful".to_string()];
        }
        if usage == TileSelectorUsage::Character {
            tilesel_widget.options_grid.text = vec!["Character".to_string(), "UtilityChar".to_string()];
        }                

        tilesel_widget.tiles = None;
        tilesel_widget.usage = usage;
        tilesel_widget.options_grid.selected_index = 0;
    }

    pub fn get_available_grid(&self, tilesel_widget: &TileSelectorWidget) -> (u32, u32) {
        let grid_size = self.get_grid_size();
        (tilesel_widget.rect.2 / grid_size, (tilesel_widget.rect.3 - UI_ELEMENT_HEIGHT) / grid_size)
    }

    pub fn get_grid_size(&self) -> u32 {
        32
    }
}