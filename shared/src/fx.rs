//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// The effect of a wall.
pub enum WallFX {
    Normal,
    MoveUp,
    MoveRight,
    MoveDown,
    MoveLeft,
    FadeOut,
}

impl WallFX {
    /// Returns the wall effect for the given string.
    pub fn from_string(s: &str) -> Self {
        match s {
            "Move Up" => Self::MoveUp,
            "Move Right" => Self::MoveRight,
            "Move Down" => Self::MoveDown,
            "Move Left" => Self::MoveLeft,
            "Fade Out" => Self::FadeOut,
            _ => Self::Normal,
        }
    }

    pub fn apply(
        &self,
        x: &mut i32,
        y: &mut i32,
        alpha: &mut f32,
        move_delta: &i32,
        time_delta: &f32,
    ) {
        match self {
            Self::Normal => {}
            Self::MoveUp => {
                *y += move_delta;
            }
            Self::MoveRight => {
                *x -= move_delta;
            }
            Self::MoveDown => {
                *y -= move_delta;
            }
            Self::MoveLeft => {
                *x += move_delta;
            }
            Self::FadeOut => {
                *alpha = 1.0 - *time_delta;
            }
        }
    }
}
