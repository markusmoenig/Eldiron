use crate::asset::Asset;

use crate::widget:: {ScreenWidget};

/// Which window do we show currently
enum GameState {
    StartMenu
}

/// The main Game struct
pub struct Game {
    window                  : GameState,
}

impl ScreenWidget for Game  {

    fn new(_asset: &Asset, _width: usize, _height: usize) -> Self where Self: Sized {
        Self {
            window          : GameState::StartMenu,
        }
    }

    /// Update the game state
    fn update(&mut self) {
    }

    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&mut self, _frame: &mut [u8], _anim_counter: usize, _asset: &mut Asset) {

        // Draw the current window
        match self.window {
            GameState::StartMenu => println!("{}", 1),
        }

        let start = self.get_time();

        let stop = self.get_time();

        println!("{:?}", stop - start);
    }

    fn get_target_fps(&self) -> usize {
        4
    }
}