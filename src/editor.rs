
use crate::widget:: {ScreenWidget, Widget};

//use crate::prelude::*;
use crate::asset::Asset;

mod toolbar;
mod nodegraph;
mod tilemapoptions;
// mod tilemap;
// mod world;

use crate::editor::toolbar::ToolBar;
// use tilemap::TileMapEditor;
// use world::WorldEditor;
// use crate::menu::MenuWidget;
use crate::context::ScreenContext;
//use crate::draw2d::Draw2D;

use crate::editor::nodegraph::NodeGraph;

use self::tilemapoptions::TileMapOptions;

/// The Editor struct
pub struct Editor {
    rect                    : (usize, usize, usize, usize),
    context                 : ScreenContext,
    toolbar                 : ToolBar,
    tilemap_options         : TileMapOptions,
    node_graph              : NodeGraph,
    left_width              : usize,
}

impl ScreenWidget for Editor {
    
    fn new(asset: &Asset, width: usize, height: usize) -> Self where Self: Sized {
        
        let left_width = 180_usize;
        let context = ScreenContext::new(width, height);

        let toolbar = ToolBar::new(vec!(), (0,0, width, context.toolbar_height), asset, &context);
        let tilemap_options = TileMapOptions::new(vec!(), (0, context.toolbar_height, left_width, height), asset, &context);
        let node_graph = NodeGraph::new(vec!(), (left_width, context.toolbar_height, width - left_width, height - context.toolbar_height), asset, &context);

        //let editor_menu = MenuWidget::new(vec!["Tilemap Editor".to_string(), "World Editor".to_string()], (10, 0, 140,  UI_ELEMENT_HEIGHT), asset);
        
        //let text : Box<dyn Widget> = Box::new(TextWidget::new("Hallo".to_string(), (0,0, WIDTH, HEIGHT)));

        /*
        let tilemap_editor : Box<dyn Widget> = Box::new(TileMapEditor::new(vec!(), (0,0, asset.width, asset.height), asset));
        let world_editor : Box<dyn Widget> = Box::new(WorldEditor::new(vec!(), (0,0, asset.width, asset.height), asset));
        widgets.push(tilemap_editor);
        widgets.push(world_editor);
        */

        //let mut curr_screen = editor;

        Self {
            rect            : (0, 0, width, height),
            context,
            toolbar,
            tilemap_options,
            node_graph,
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
        self.tilemap_options.resize(self.left_width, height, &self.context);
        self.node_graph.resize(width - self.left_width, height - self.context.toolbar_height, &self.context);
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset) {

        let start = self.get_time();

        self.toolbar.draw(frame, anim_counter, asset, &mut self.context);
        self.tilemap_options.draw(frame, anim_counter, asset, &mut self.context);
        self.node_graph.draw(frame, anim_counter, asset, &mut self.context);

        // self.context.draw2d.draw_square_pattern(frame, &(0, self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), self.context.width, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

        // self.context.draw2d.draw_circle(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[255, 255, 255, 255], 200.0);
        // self.context.draw2d.draw_circle_with_border(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[255, 255, 255, 255], 200.0, &[255, 0, 0, 255], 10.0);

        // self.context.draw2d.draw_rounded_rect(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0));
        // self.context.draw2d.draw_rounded_rect_with_border(frame, &(0, self.context.toolbar_height, self.rect.2, self.rect.3 - self.context.toolbar_height), self.context.width, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0), &[255, 0, 0, 255], 20.0);

        let stop = self.get_time();

        println!("{:?}", stop - start);
    }

    fn mouse_down(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;

        //consumed = self.widgets[self.curr_index as usize].mouse_down(pos, asset);

        // if consumed == false && self.editor_menu.mouse_down(pos, asset) {
        //     consumed = true;
        // }
        consumed
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset) -> bool {
        let mut consumed = false;
        //consumed = self.widgets[self.curr_index as usize].mouse_up(pos, asset);

        // if consumed == false && self.editor_menu.mouse_up(pos, asset) {
        //     consumed = true;
        // }
        consumed
    }

    fn mouse_dragged(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;
        //consumed = self.widgets[self.curr_index as usize].mouse_dragged(pos, asset);

        // if consumed == false && self.editor_menu.mouse_dragged(pos, asset) {
        //     self.curr_index = self.editor_menu.selected_index.get();
        //     consumed = true;
        // }
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