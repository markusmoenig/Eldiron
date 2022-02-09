
use crate::widget:: {ScreenWidget, Widget};

use crate::prelude::*;
use crate::asset::Asset;

mod tilemap;
mod world;

use tilemap::TileMapEditor;
use world::WorldEditor;
use crate::menu::MenuWidget;
use crate::context::ScreenContext;
//use crate::draw2d::Draw2D;

/// The Editor struct
pub struct Editor {
    rect                    : (usize, usize, usize, usize),
    context                 : ScreenContext,
    widgets                 : Vec<Box<dyn Widget>>,
    editor_menu             : MenuWidget,
    curr_index              : u32,
}

impl ScreenWidget for Editor {
    
    fn new(asset: &Asset, width: usize, height: usize) -> Self where Self: Sized {

        let mut widgets = vec!();

        let editor_menu = MenuWidget::new(vec!["Tilemap Editor".to_string(), "World Editor".to_string()], (10, 0, 140,  UI_ELEMENT_HEIGHT), asset);
        
        //let text : Box<dyn Widget> = Box::new(TextWidget::new("Hallo".to_string(), (0,0, WIDTH, HEIGHT)));

        let tilemap_editor : Box<dyn Widget> = Box::new(TileMapEditor::new(vec!(), (0,0, asset.width, asset.height), asset));
        let world_editor : Box<dyn Widget> = Box::new(WorldEditor::new(vec!(), (0,0, asset.width, asset.height), asset));
        widgets.push(tilemap_editor);
        widgets.push(world_editor);

        //let mut curr_screen = editor;

        Self {
            rect            : (0, 0, width, height),
            context         : ScreenContext::new(width, height),
            widgets,
            editor_menu,
            curr_index      : 0
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.context.width = width; self.rect.2 = width;
        self.context.height = height; self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {

        let start = self.get_time();

        let toolbar_height = 50_usize;
        self.context.draw2d.draw_rect(frame, &(0, 0, self.rect.2, toolbar_height), self.context.width, &[25, 25, 25, 255]);
        self.context.draw2d.draw_square_pattern(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[44, 44, 46, 255], &[56, 56, 56, 255], 40);

        // self.context.draw2d.draw_circle(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[255, 255, 255, 255], 200.0);
        // self.context.draw2d.draw_circle_with_border(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &[255, 255, 255, 255], 200.0, &[255, 0, 0, 255], 10.0);

        // self.context.draw2d.draw_rounded_rect(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0));
        self.context.draw2d.draw_rounded_rect_with_border(frame, &(0, toolbar_height, self.rect.2, self.rect.3 - toolbar_height), self.context.width, &(200.0, 200.0), &[255, 255, 255, 255], &(50.0, 50.0, 50.0, 50.0), &[255, 0, 0, 255], 20.0);

        let stop = self.get_time();

        println!("{:?}", stop - start);
    }

    /// Returns the current widgets
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>> {
        &self.widgets
    }

    fn mouse_down(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed;

        consumed = self.widgets[self.curr_index as usize].mouse_down(pos, asset);

        if consumed == false && self.editor_menu.mouse_down(pos, asset) {
            consumed = true;
        }
        consumed
    }

    fn mouse_up(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed;
        consumed = self.widgets[self.curr_index as usize].mouse_up(pos, asset);

        if consumed == false && self.editor_menu.mouse_up(pos, asset) {
            consumed = true;
        }
        consumed
    }

    fn mouse_dragged(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed;
        consumed = self.widgets[self.curr_index as usize].mouse_dragged(pos, asset);

        if consumed == false && self.editor_menu.mouse_dragged(pos, asset) {
            self.curr_index = self.editor_menu.selected_index.get();
            consumed = true;
        }
        consumed
    }
}