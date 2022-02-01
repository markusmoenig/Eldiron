use crate::asset::Asset;

use crate::widget:: {ScreenWidget, Widget};

/// Which window do we show currently
enum GameState {
    StartMenu
}

/// The main Game struct
pub struct Game {
    window                  : GameState,
    widgets                 : Vec<Box<dyn Widget>>
}

impl ScreenWidget for Game  {
    
    fn new() -> Self where Self: Sized {
        Self {
            window          : GameState::StartMenu,
            widgets         : vec!()
        }
    }

    /// Update the game state
    fn update(&mut self) {
    }
    
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, _frame: &mut [u8], _anim_counter: u32, _asset: &mut Asset) {

        // Draw the current window
        match self.window {
            GameState::StartMenu => println!("{}", 1),
        }

        let start = self.get_time();

        let stop = self.get_time();

        println!("{:?}", stop - start);
    }  

    /// Returns the current widgets
    fn get_widgets(&self) -> &Vec<Box<dyn Widget>> {
        &self.widgets
    }
}