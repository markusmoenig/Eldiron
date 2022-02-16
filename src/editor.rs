
use crate::widget:: {ScreenWidget, Widget};

//use crate::prelude::*;
use crate::asset::Asset;

mod toolbar;
mod nodegraph;
mod tilemapoptions;
mod tilemap;
// mod world;

use crate::editor::toolbar::ToolBar;
use tilemap::TileMap;
// use world::WorldEditor;
// use crate::menu::MenuWidget;
use crate::context::ScreenContext;
//use crate::draw2d::Draw2D;

use crate::node::NodeWidget;
use crate::widget::node::NodeWidgetType;

use crate::editor::nodegraph::{ NodeGraph, GraphMode };

use self::tilemapoptions::TileMapOptions;

#[derive (PartialEq)]
enum EditorState {
    TILES_OVERVIEW,
    TILES_DETAIL
}

/// The Editor struct
pub struct Editor {
    rect                    : (usize, usize, usize, usize),
    state                   : EditorState,
    context                 : ScreenContext,
    toolbar                 : ToolBar,

    tilemap_options         : TileMapOptions,
    tilemap                 : TileMap,

    node_graph_tiles        : NodeGraph,
    left_width              : usize,
}

impl ScreenWidget for Editor {
    
    fn new(asset: &Asset, width: usize, height: usize) -> Self where Self: Sized {
        
        let left_width = 180_usize;
        let context = ScreenContext::new(width, height);

        let toolbar = ToolBar::new(vec!(), (0,0, width, context.toolbar_height), asset, &context);

        // Tile Views
        let tilemap_options = TileMapOptions::new(vec!(), (0, context.toolbar_height, left_width, height - context.toolbar_height), asset, &context);
        let tilemap = TileMap::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context);

        let mut tile_nodes = vec![];
        for t in &asset.tileset.maps_names {
            let node = NodeWidget::new(vec![t.to_string()], NodeWidgetType::Tile, vec![]);
            tile_nodes.push(node);
        }

        let node_graph_tiles = NodeGraph::new(vec!(), (0, context.toolbar_height, width, height - context.toolbar_height), asset, &context, tile_nodes);

        Self {
            rect            : (0, 0, width, height),
            state           : EditorState::TILES_OVERVIEW,
            context,
            toolbar,

            tilemap_options,
            tilemap,

            node_graph_tiles,
            left_width
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.context.width = width; self.rect.2 = width;
        self.context.height = height; self.rect.3 = height;
        self.toolbar.resize(width, height, &self.context);

        self.tilemap_options.resize(self.left_width, height - self.context.toolbar_height, &self.context);
        self.tilemap.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
        self.node_graph_tiles.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset) {

        let start = self.get_time();

        self.toolbar.draw(frame, anim_counter, asset, &mut self.context);

        if self.state == EditorState::TILES_OVERVIEW {
            self.node_graph_tiles.draw(frame, anim_counter, asset, &mut self.context);
        } else
        if self.state == EditorState::TILES_DETAIL {
            self.tilemap_options.draw(frame, anim_counter, asset, &mut self.context);
            self.tilemap.draw(frame, anim_counter, asset, &mut self.context);
        }

        // self.context.draw2d.draw_square_pattern(frame, &(0, self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), self.context.width, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

        // self.context.draw2d.draw_circle(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[255, 255, 255, 255], 200.0);
        // self.context.draw2d.draw_circle_with_border(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[255, 255, 255, 255], 200.0, &[255, 0, 0, 255], 10.0);

        // self.context.draw2d.draw_rounded_rect(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0));
        // self.context.draw2d.draw_rounded_rect_with_border(frame, &(0, self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), self.context.width, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0), &[255, 0, 0, 255], 20.0);

        let stop = self.get_time();

        println!("{:?}", stop - start);
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;

        if self.toolbar.mouse_down(pos, asset, &mut self.context) {            
            self.tilemap.set_tilemap_index(self.toolbar.widgets[0].curr_index);

            if self.toolbar.widgets[1].selected {
                self.node_graph_tiles.set_mode( GraphMode::Overview, (0, self.rect.1 + self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                self.state = EditorState::TILES_OVERVIEW;
            } else
            if self.toolbar.widgets[1].right_selected {
                self.node_graph_tiles.set_mode( GraphMode::Detail, (self.left_width, self.rect.1 + self.context.toolbar_height, self.rect.2 - self.rect.2, self.rect.3 - self.context.toolbar_height), &self.context);
                self.state = EditorState::TILES_DETAIL;
            }            
            self.tilemap.set_tilemap_index(self.toolbar.widgets[0].curr_index);

            consumed = true;
        }

        if self.state == EditorState::TILES_OVERVIEW {
            if consumed == false && self.node_graph_tiles.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::TILES_DETAIL {
            if consumed == false && self.tilemap_options.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }
            if consumed == false && self.tilemap.mouse_down(pos, asset, &mut self.context) {
                consumed = true;
            }      
        }  

        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;
        if self.toolbar.mouse_up(pos, asset, &mut self.context) {            
            self.tilemap.set_tilemap_index(self.toolbar.widgets[0].curr_index);
            consumed = true;
        }

        if self.state == EditorState::TILES_OVERVIEW {
            if consumed == false && self.node_graph_tiles.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::TILES_DETAIL {
            if self.tilemap_options.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.tilemap.mouse_up(pos, asset, &mut self.context) {
                consumed = true;
            }    
        }    
        consumed
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;

        if self.state == EditorState::TILES_OVERVIEW {
            if consumed == false && self.node_graph_tiles.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
        } else
        if self.state == EditorState::TILES_DETAIL {
            if self.tilemap_options.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }
            if self.tilemap.mouse_dragged(pos, asset, &mut self.context) {
                consumed = true;
            }    
        }
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;
        //consumed = self.widgets[self.curr_index as usize].mouse_hover(pos, asset);

        if consumed == false && self.toolbar.mouse_hover(pos, asset, &mut self.context) {
            consumed = true;
        }
        consumed
    }
}