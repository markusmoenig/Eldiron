use core_shared::asset::{ Asset };
use core_shared::asset::tileset::TileUsage;

use crate::widget::context::ScreenContext;
use crate::editor::{ TileSelectorWidget, NodeGraph, GraphMode };
use core_server::gamedata::behavior::{ BehaviorType };

use crate::editor::{ EditorOptions, EditorContent };
use crate::editor::regionoptions::RegionEditorMode;

use crate::editor::ToolBar;

pub struct RegionWidget {
    pub rect                : (usize, usize, usize, usize),
    pub region_id           : usize,

    grid_size               : usize,

    offset                  : (isize, isize),
    screen_offset           : (usize, usize),

    pub tile_selector       : TileSelectorWidget,

    pub behavior_graph      : Box::<NodeGraph>,

    mouse_wheel_delta       : (isize, isize),

    mouse_hover_pos         : (usize, usize),
    pub clicked             : Option<(isize, isize)>,

    selector_size           : usize,
}

impl EditorContent for RegionWidget {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _behavior_type: BehaviorType, asset: &Asset, context: &ScreenContext) -> Self {

        let bottom_size = 250;

        // Tile Selector
        let mut tile_selector = TileSelectorWidget::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), asset, &context);
        tile_selector.set_tile_type(vec![TileUsage::Environment], None, None, &asset);

        // Graph
        let mut behavior_graph = NodeGraph::new(vec!(), (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size), BehaviorType::Regions, asset, &context);

        behavior_graph.set_mode(GraphMode::Detail, &context);

        Self {
            rect,
            region_id               : 0,
            grid_size               : 32,

            offset                  : (0, 0),
            screen_offset           : (0, 0),

            tile_selector,
            behavior_graph          : Box::new(behavior_graph),

            mouse_wheel_delta       : (0, 0),
            mouse_hover_pos         : (0, 0),
            clicked                 : None,

            selector_size           : 250,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;

        self.tile_selector.rect = (self.rect.0, self.rect.1 + self.rect.3 - self.selector_size, width, self.selector_size);
        self.tile_selector.resize(width, self.selector_size);
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &[0,0,0,255]);

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            let mut rect = self.rect.clone();
            if editor_mode != RegionEditorMode::Areas {
                rect.3 -= 250;
            }
            let grid_size = self.grid_size;

            let left_offset = (rect.2 % grid_size) / 2;
            let top_offset = (rect.3 % grid_size) / 2;

            self.screen_offset = (left_offset, top_offset);

            if let Some(region) = context.data.regions.get(&self.region_id) {

                if context.is_running == false {
                    let x_tiles = (rect.2 / grid_size) as isize;
                    let y_tiles = (rect.3 / grid_size) as isize;

                    for y in 0..y_tiles {
                        for x in 0..x_tiles {
                            let values = region.get_value((x - self.offset.0, y - self.offset.1));

                            if values.is_empty() == false {
                                let pos = (rect.0 + left_offset + (x as usize) * grid_size, rect.1 + top_offset + (y as usize) * grid_size);
                                for value in values {
                                    let map = asset.get_map_of_id(value.0);
                                    context.draw2d.draw_animated_tile(frame, &pos, map,context.width,&(value.1, value.2), anim_counter, grid_size);
                                }
                            }
                        }
                    }
                } else {
                    context.draw2d.draw_region_with_instances(frame, region, &rect, &(-self.offset.0, -self.offset.1), context.width, grid_size, anim_counter, asset, context);
                }
            }

            if editor_mode == RegionEditorMode::Tiles {
                self.tile_selector.draw(frame, context.width, anim_counter, asset, context);
            } else
            if editor_mode == RegionEditorMode::Areas || editor_mode == RegionEditorMode::Behavior {
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
                                        if editor_mode == RegionEditorMode::Areas {
                                            continue;
                                        }
                                        c[3] = 50;
                                    }
                                    context.draw2d.blend_rect(frame, &(pos.0, pos.1, grid_size, grid_size), context.width, &c);
                                }
                            }
                        }
                    }
                }
            }

            if editor_mode == RegionEditorMode::Behavior {
                self.behavior_graph.draw(frame, anim_counter, asset, context, &mut None);
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
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        let mut consumed = false;

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Tiles {
                if self.tile_selector.mouse_down(pos, asset, context) {
                    consumed = true;
                    if let Some(selected) = &self.tile_selector.selected {
                        context.curr_region_tile = Some(selected.clone());
                    } else {
                        context.curr_region_tile = None;
                    }
                }
            } else
            if editor_mode == RegionEditorMode::Behavior {
                if context.contains_pos_for(pos, self.behavior_graph.rect) {
                    consumed = self.behavior_graph.mouse_down(pos, asset, context, &mut None, &mut None);
                    return consumed;
                }
            }

            if consumed == false && context.contains_pos_for(pos, self.rect) {
                if let Some(id) = self.get_tile_id(pos) {
                    self.clicked = Some(id);
                    let editor_mode = options.get_editor_mode();

                    if editor_mode == RegionEditorMode::Tiles {
                        if let Some(selected) = &self.tile_selector.selected {
                            if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                region.set_value(options.get_layer(), id, selected.clone());
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
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        self.clicked = None;

        let mut consumed = false;

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Behavior {
                if context.contains_pos_for(pos, self.behavior_graph.rect) {
                    consumed = self.behavior_graph.mouse_up(pos, asset, context, &mut None, &mut None);
                }
            }
        }

        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext, _options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        self.mouse_hover_pos = pos.clone();
        true
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        let mut consumed = false;

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Behavior {
                if context.contains_pos_for(pos, self.behavior_graph.rect) {
                    consumed = self.behavior_graph.mouse_dragged(pos, asset, context, &mut None, &mut None);
                    return consumed;
                }
            }

            if consumed == false && context.contains_pos_for(pos, self.rect) {
                if let Some(id) = self.get_tile_id(pos) {
                    if self.clicked != Some(id) {

                        self.clicked = Some(id);
                        let editor_mode = options.get_editor_mode();

                        if editor_mode == RegionEditorMode::Tiles {
                            if let Some(selected) = &self.tile_selector.selected {
                                if let Some(region) = context.data.regions.get_mut(&self.region_id) {
                                    region.set_value(options.get_layer(), id, selected.clone());
                                    region.save_data();
                                }
                            }
                        }
                    }
                }

                consumed = true;
            }
        }

        consumed
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        let mut consumed = false;
        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Tiles {
                if context.contains_pos_for(self.mouse_hover_pos, self.tile_selector.rect) && self.tile_selector.mouse_wheel(delta, asset, context) {
                    consumed = true;
                }
            } else
            if editor_mode == RegionEditorMode::Behavior {
                if context.contains_pos_for(self.mouse_hover_pos, self.behavior_graph.rect) && self.behavior_graph.mouse_wheel(delta, asset, context, &mut None, &mut None) {
                    consumed = true;
                }
            }

            if consumed == false {
                self.mouse_wheel_delta.0 += delta.0;
                self.mouse_wheel_delta.1 += delta.1;

                self.offset.0 -= self.mouse_wheel_delta.0 / self.grid_size as isize;
                self.offset.1 += self.mouse_wheel_delta.1 / self.grid_size as isize;

                self.mouse_wheel_delta.0 -= (self.mouse_wheel_delta.0 / self.grid_size as isize) * self.grid_size as isize;
                self.mouse_wheel_delta.1 -= (self.mouse_wheel_delta.1 / self.grid_size as isize) * self.grid_size as isize;
            }
        }
        true
    }

    /// Sets a region id
    fn set_region_id(&mut self, id: usize, context: &mut ScreenContext, options: &mut Option<Box<dyn EditorOptions>>) {
        self.region_id = id;

        if let Some(region) = context.data.regions.get_mut(&self.region_id) {
            if let Some(options) = options {
                options.set_area_names(region.get_area_names());
                if context.curr_region_area_index >= region.data.areas.len() {
                    context.curr_region_area_index = 0;
                }
                if region.behaviors.len() > 0 {
                    self.behavior_graph.set_behavior_id(region.behaviors[context.curr_region_area_index].data.id, context);
                }
            }
        }

        if let Some(options) = options {
            let mode = options.get_editor_mode();
            if mode == RegionEditorMode::Settings {
                if let Some(region) = context.data.regions.get_mut(&id) {
                    let json = serde_json::to_string_pretty(&region.data.settings).unwrap();
                    context.code_editor_is_active = true;
                    context.code_editor_just_opened = true;
                    context.code_editor_node_behavior_value.4 = json;
                    context.code_editor_node_behavior_id.0 = 130000;
                }
            }
        }

    }

    /// Get the tile id
    fn get_tile_id(&self, pos: (usize, usize)) -> Option<(isize, isize)> {
        let grid_size = self.grid_size;
        if pos.0 > self.rect.0 + self.screen_offset.0 && pos.1 > self.rect.1 + self.screen_offset.1
        && pos.0 < self.rect.0 + self.rect.2 - self.screen_offset.0  && pos.1 < self.rect.1 + self.rect.3 - self.screen_offset.1 - self.selector_size
        {
            let x = ((pos.0 - self.rect.0 - self.screen_offset.0) / grid_size) as isize - self.offset.0;
            let y = ((pos.1 - self.rect.1 - self.screen_offset.0) / grid_size) as isize - self.offset.1;
            return Some((x, y));
        }
        None
    }

    /// Returns the selected tile
    fn get_selected_tile(&self) -> Option<(usize, usize, usize, TileUsage)> {
        self.tile_selector.selected.clone()
    }

    /// Return the tile_selector
    fn get_tile_selector(&mut self) -> Option<&mut TileSelectorWidget> {
        Some(&mut self.tile_selector)
    }

    /// Return the behavior graph
    fn get_behavior_graph(&mut self) -> Option<&mut NodeGraph> {
        Some(&mut self.behavior_graph)
    }

    /// Returns the region_id
    fn get_region_id(&self) -> usize {
        self.region_id
    }

    /// Returns the rect for DnD
    fn get_rect(&self) -> (usize, usize, usize, usize) {
        self.behavior_graph.rect.clone()
    }

    /// Adds the given node to the behavior graph (after DnD)
    fn add_node_of_name(&mut self, name: String, position: (isize, isize), context: &mut ScreenContext) {
        self.behavior_graph.add_node_of_name(name, position, context);
    }

    /// Update the behavior graph when a setting changed
    fn update_from_dialog(&mut self, context: &mut ScreenContext) {
        self.behavior_graph.update_from_dialog(context);
    }
}