
use crate::widget:: {ScreenWidget, Widget};

use crate::prelude::*;
use crate::asset::Asset;

mod tilemap;
use tilemap::TileMapEditor;

/// The Editor struct
pub struct Editor {
    widgets                 : Vec<Box<dyn Widget>>,
    curr_index              : u32,
}

impl ScreenWidget for Editor {
    
    fn new() -> Self where Self: Sized {

        let mut widgets = vec!();

        //let text : Box<dyn Widget> = Box::new(TextWidget::new("Hallo".to_string(), (0,0, WIDTH, HEIGHT)));

        let tilemap_editor : Box<dyn Widget> = Box::new(TileMapEditor::new(vec!(), (0,0, WIDTH, HEIGHT)));
        widgets.push(tilemap_editor);

        //let mut curr_screen = editor;

        Self {
            widgets,
            curr_index      : 0
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {
        //let start = self.get_time();

        self.widgets[self.curr_index as usize].draw(frame, anim_counter, asset);

        //let stop = self.get_time();

        //println!("{:?}", stop - start);
    }

    /// Returns the current widgets
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>> {
        &self.widgets
    }

    fn mouse_down(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        self.widgets[self.curr_index as usize].mouse_down(pos, asset)
    }

    fn mouse_up(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        self.widgets[self.curr_index as usize].mouse_up(pos, asset)
    }

    fn mouse_dragged(&mut self, pos: (u32, u32), asset: &mut Asset) -> bool {
        self.widgets[self.curr_index as usize].mouse_dragged(pos, asset)
    }
}