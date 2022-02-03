
use crate::widget:: {ScreenWidget, Widget};

use crate::prelude::*;
use crate::asset::Asset;

mod tilemap;
mod world;

use tilemap::TileMapEditor;
use world::WorldEditor;
use crate::menu::MenuWidget;

/// The Editor struct
pub struct Editor {
    rect                    : (u32, u32, u32, u32),
    widgets                 : Vec<Box<dyn Widget>>,
    editor_menu             : MenuWidget,
    curr_index              : u32,
}

impl ScreenWidget for Editor {
    
    fn new(asset: &Asset) -> Self where Self: Sized {

        let mut widgets = vec!();

        let editor_menu = MenuWidget::new(vec!["Tilemap Editor".to_string(), "World Editor".to_string()], (10, 0, 140,  UI_ELEMENT_HEIGHT), asset);
        
        //let text : Box<dyn Widget> = Box::new(TextWidget::new("Hallo".to_string(), (0,0, WIDTH, HEIGHT)));

        let tilemap_editor : Box<dyn Widget> = Box::new(TileMapEditor::new(vec!(), (0,0, WIDTH, HEIGHT), asset));
        let world_editor : Box<dyn Widget> = Box::new(WorldEditor::new(vec!(), (0,0, WIDTH, HEIGHT), asset));
        widgets.push(tilemap_editor);
        widgets.push(world_editor);

        //let mut curr_screen = editor;

        Self {
            rect : (0, 0, WIDTH, HEIGHT),
            widgets,
            editor_menu,
            curr_index      : 0
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {
        //let start = self.get_time();

        asset.draw_rect(frame, &self.rect, [0,0,0,255]);
        asset.draw_rect(frame, &(0, 0, WIDTH, UI_ELEMENT_HEIGHT), self.editor_menu.get_color_background());

        self.widgets[self.curr_index as usize].draw(frame, anim_counter, asset);

        self.editor_menu.draw(frame, anim_counter, asset);

        //let stop = self.get_time();

        //println!("{:?}", stop - start);
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