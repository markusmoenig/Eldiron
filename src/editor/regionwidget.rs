use server::asset::{ Asset };
use server::asset::tileset::TileUsage;

use crate::widget::context::ScreenContext;
use crate::editor::{ TileSelectorWidget, RegionOptions, NodeGraph, GraphMode };
use super::regionoptions::RegionEditorMode;
use server::gamedata::behavior::{ BehaviorType };

pub struct RegionWidget {
    pub rect                : (usize, usize, usize, usize),
    pub region_id           : usize,

    grid_size               : usize,

    offset                  : (isize, isize),
    screen_offset           : (usize, usize),

    pub tile_selector       : TileSelectorWidget,

    pub behavior_graph      : NodeGraph,

    mouse_wheel_delta       : (isize, isize),

    mouse_hover_pos         : (usize, usize),
    pub clicked             : Option<(isize, isize)>,
}

impl RegionWidget {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let bottom_size = 250;

        // Tile Selector
        let mut tile_selector = TileSelectorWidget::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), asset, &context);
        tile_selector.set_tile_type(vec![TileUsage::Environment, TileUsage::EnvBlocking, TileUsage::Water], None, &asset);

        // Graph
        let mut behavior_graph = NodeGraph::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), asset, &context, BehaviorType::Regions, vec![]);

        behavior_graph.set_mode(GraphMode::Detail, &context);

        Self {
            rect,
            region_id               : 0,
            grid_size               : 32,

            offset                  : (0, 0),
            screen_offset           : (0, 0),

            tile_selector,
            behavior_graph,

            mouse_wheel_delta       : (0, 0),
            mouse_hover_pos         : (0, 0),
            clicked                 : None,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;

        self.tile_selector.rect = (self.rect.0, self.rect.1 + self.rect.3 - 250, width, 250);
        self.tile_selector.resize(width, 250);
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, region_options: &mut RegionOptions) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &[0,0,0,255]);

        let editor_mode = region_options.get_editor_mode();

        let mut rect = self.rect;
        if editor_mode != RegionEditorMode::Areas {
            rect.3 -= 250;
        }
        let grid_size = self.grid_size;

        let left_offset = (self.rect.2 % grid_size) / 2;
        let top_offset = (self.rect.3 % grid_size) / 2;

        self.screen_offset = (left_offset, top_offset);

        //let grid = (rect.2 / grid_size, rect.3 / grid_size);
        //let max_tiles = grid.0 * grid.1;

        if let Some(region) = context.data.regions.get(&self.region_id) {

            let x_tiles = (rect.2 / grid_size) as isize;
            let y_tiles = (rect.3 / grid_size) as isize;

            for y in 0..y_tiles {
                for x in 0..x_tiles {
                    if let Some(value) = region.get_value((x - self.offset.0, y - self.offset.1)) {
                        let pos = (rect.0 + left_offset + (x as usize) * grid_size, rect.1 + top_offset + (y as usize) * grid_size);

                        let map = asset.get_map_of_id(value.0);
                        context.draw2d.draw_animated_tile(frame, &pos, map,context.width,&(value.1, value.2), anim_counter, grid_size);
                    }
                }
            }
        }

        if editor_mode == RegionEditorMode::Tiles {
            self.tile_selector.draw(frame, context.width, anim_counter, asset, context);
        } else
        if editor_mode == RegionEditorMode::Areas {
            if let Some(region) = context.data.regions.get(&self.region_id) {

                let x_tiles = (rect.2 / grid_size) as isize;
                let y_tiles = (rect.3 / grid_size) as isize;

                let curr_area_index = context.curr_region_area_index;

                for y in 0..y_tiles {
                    for x in 0..x_tiles {

                        let rx = x - self.offset.0;
                        let ry = y - self.offset.1;

                        for area_index in 0..region.data.areas.len() {

                            if region.data.areas[area_index].area.contains(&(rx, ry)) {
                                let pos = (rect.0 + left_offset + (x as usize) * grid_size, rect.1 + top_offset + (y as usize) * grid_size);

                                let mut c = context.color_white.clone();
                                if curr_area_index == area_index {
                                    c[3] = 100;
                                } else {
                                    c[3] = 50;
                                }
                                context.draw2d.blend_rect(frame, &(pos.0, pos.1, grid_size, grid_size), context.width, &c);
                            }
                        }
                    }
                }
            }
        } else
        if editor_mode == RegionEditorMode::Behavior {
            self.behavior_graph.draw(frame, anim_counter, asset, context);
        }

        if self.mouse_hover_pos != (0,0) {
            if let Some(id) = self.get_tile_id(self.mouse_hover_pos) {
                let pos = (rect.0 + left_offset + ((id.0 + self.offset.0) as usize) * grid_size, rect.1 + top_offset + ((id.1 + self.offset.1) as usize) * grid_size);
                if  pos.0 + grid_size < rect.0 + rect.2 && pos.1 + grid_size < rect.1 + rect.3 {
                    context.draw2d.draw_rect_outline(frame, &(pos.0, pos.1, grid_size, grid_size), context.width, context.color_light_white);
                    context.draw2d.draw_rect_outline(frame, &(pos.0 + 1, pos.1 + 1, grid_size - 2, grid_size - 2), context.width, context.color_black);
                }
            }
        }
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, region_options: &mut RegionOptions) -> bool {

        if context.contains_pos_for(pos, self.tile_selector.rect) {
            if self.tile_selector.mouse_down(pos, asset, context) {
                return true;
            }
        }

        if context.contains_pos_for(pos, self.rect) {
            if let Some(id) = self.get_tile_id(pos) {
                self.clicked = Some(id);
                let editor_mode = region_options.get_editor_mode();

                if editor_mode == RegionEditorMode::Tiles {
                    if let Some(selected) = &self.tile_selector.selected {
                        if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                            region.set_value(id, selected.clone());
                            region.save_data();
                        }
                    }
                } else
                if editor_mode == RegionEditorMode::Areas {
                    if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                        if region.data.areas.len() > 0 {
                            let area = &mut region.data.areas[context.curr_region_area_index];
                            if area.area.contains(&id) == false {
                                area.area.push(id);
                            }
                            region.save_data();
                        }
                    }
                }
            }
            return true;
        }

        if self.tile_selector.mouse_down(pos, asset, context) {

            if let Some(selected) = &self.tile_selector.selected {
                context.curr_region_tile = Some(selected.clone());
            } else {
                context.curr_region_tile = None;
            }
        }

        false
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext, _region_options: &mut RegionOptions) -> bool {
        self.clicked = None;

        false
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext, _region_options: &mut RegionOptions) -> bool {
        self.mouse_hover_pos = pos.clone();
        true
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext, region_options: &mut RegionOptions) -> bool {
        if context.contains_pos_for(pos, self.rect) {

            if let Some(id) = self.get_tile_id(pos) {
                if self.clicked != Some(id) {

                    self.clicked = Some(id);
                    let editor_mode = region_options.get_editor_mode();

                    if editor_mode == RegionEditorMode::Tiles {
                        if let Some(selected) = &self.tile_selector.selected {
                            if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                region.set_value(id, selected.clone());
                                region.save_data();
                            }
                        }
                    }
                }
            }

            return true;
        }
        false
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if context.contains_pos_for(self.mouse_hover_pos, self.tile_selector.rect) && self.tile_selector.mouse_wheel(delta, asset, context) {
        } else {
            self.mouse_wheel_delta.0 += delta.0;
            self.mouse_wheel_delta.1 += delta.1;

            self.offset.0 -= self.mouse_wheel_delta.0 / self.grid_size as isize;
            self.offset.1 += self.mouse_wheel_delta.1 / self.grid_size as isize;

            self.mouse_wheel_delta.0 -= (self.mouse_wheel_delta.0 / self.grid_size as isize) * self.grid_size as isize;
            self.mouse_wheel_delta.1 -= (self.mouse_wheel_delta.1 / self.grid_size as isize) * self.grid_size as isize;
        }
        true
    }

    /// Sets a new map index
    pub fn set_region_id(&mut self, id: usize, context: &mut ScreenContext, region_options: &mut RegionOptions) {
        self.region_id = id;

        if let Some(region) = context.data.regions.get_mut(&self.region_id) {
            region_options.area_widgets[0].text = region.get_area_names();
            region_options.area_widgets[0].dirty = true;
            context.curr_region_area_index = 0;
            if region.behaviors.len() > 0 {
                self.behavior_graph.set_behavior_id(region.behaviors[0].data.id, context);
            }
        }
    }

    /// Get the tile id
    pub fn get_tile_id(&self, pos: (usize, usize)) -> Option<(isize, isize)> {
        let grid_size = self.grid_size;
        if pos.0 > self.rect.0 + self.screen_offset.0 && pos.1 > self.rect.1 + self.screen_offset.1 {
            let x = ((pos.0 - self.rect.0 - self.screen_offset.0) / grid_size) as isize - self.offset.0;
            let y = ((pos.1 - self.rect.1 - self.screen_offset.0) / grid_size) as isize - self.offset.1;
            return Some((x, y));
        }
        None
    }
}