//! Application-owned graphical presentation. No window surface or server clock.
mod scene_views;
pub use scene_views::*;

pub mod screen;

mod game_views;
pub use game_views::*;
