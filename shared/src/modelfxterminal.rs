//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXTerminalRole {
    Face,
    UV,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXColor {
    Color(TheColor),
}

impl ModelFXColor {
    pub fn create(index: u8) -> Self {
        Self::Color(match index {
            0 => TheColor::from_hex("#d9ac8b"),
            1 => TheColor::from_hex("#3e6958"),
            2 => TheColor::from_hex("#b1a58d"),
            3 => TheColor::from_hex("#624c3c"),
            4 => TheColor::from_hex("#243d5c"),
            5 => TheColor::from_hex("#e0c872"),
            6 => TheColor::from_hex("#b03a48"),
            7 => TheColor::from_hex("#d4804d"),
            8 => TheColor::from_hex("#5c8b93"),
            _ => TheColor::from_hex("#e3cfb4"),
        })
    }
    pub fn color(&self) -> &TheColor {
        match self {
            Self::Color(color) => color,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ModelFXTerminal {
    pub role: ModelFXTerminalRole,
    pub color: ModelFXColor,
}

impl ModelFXTerminal {
    pub fn new(role: ModelFXTerminalRole, index: u8) -> Self {
        Self {
            role,
            color: ModelFXColor::create(index),
        }
    }
}
