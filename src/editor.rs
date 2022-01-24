
use crate::widget:: {ScreenWidget, Widget };
use crate::widget::text::TextWidget;

use crate::prelude::*;
use crate::asset::Asset;

/// Which Window do we show currently
enum EditorState {
    TileSet,
    Tiles,
}

/// The Editor struct
pub struct Editor<'a> {
    state                   : EditorState,
    asset                   : Asset<'a>,
    widgets                 : Vec<Box<dyn Widget>>
}

impl ScreenWidget for Editor<'_>  {
    
    fn new() -> Self where Self: Sized {

        let mut widgets = vec!();

        let text : Box<dyn Widget> = Box::new(TextWidget::new("Hallo".to_string(), [0,0, WIDTH, HEIGHT]));
        widgets.push(text);

        Self {
            state           : EditorState::TileSet,
            asset           : Asset::new(),
            widgets
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8]) {
        for w in &self.widgets {
            w.draw(frame, &self.asset);
        }
    }

    /// Returns the asset structure
    fn get_asset(&self) -> &Asset {
        &self.asset
    }

    /// Returns the current widgets
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>> {
        &self.widgets
    }
}