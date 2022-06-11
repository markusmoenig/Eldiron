
// This module contains script support structs. These are passed to the Rhai scripting engine to provide
// data structures which can be accessed from both Rust and scripts.

//use crate::gamedata::MessageType;

// --- Sending Messages
/*
#[derive(Debug, Clone)]
pub struct ScriptMessages {
    messages: Vec<(String, MessageType)>
}

impl ScriptMessages {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    pub fn debug(&mut self, message: &str) {
        self.messages.push((message.to_owned(), MessageType::Debug));
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}*/

// --- Drawing

// --- ScriptPosition

use std::cmp::max;

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptPosition {
    pub pos             : (usize, usize)
}

impl ScriptPosition {
    pub fn new(x: i64, y: i64) -> Self {
        Self {
            pos         : (x as usize, y as usize),
        }
    }
}

// --- ScriptRect

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptRect {
    pub rect            : (usize, usize, usize, usize)
}

impl ScriptRect {
    pub fn new(x: i64, y: i64, width: i64, height: i64) -> Self {
        Self {
            rect : (max(x, 0) as usize, max(y, 0) as usize, max(width, 0) as usize, max(height, 0) as usize)
        }
    }

    /// Returns true if this rect is safe for the given screen dimensions
    pub fn is_safe(&self, width: usize, height: usize) -> bool {
        if self.rect.0 + self.rect.2 > width {
            return false;
        }
        if self.rect.1 + self.rect.3 > height {
            return false;
        }
        return true;
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptRGB {
    pub value            : [u8;4]
}

// --- ScriptRGB

impl ScriptRGB {
    pub fn new(r: i64, g: i64, b: i64) -> Self {
        Self {
            value       : [r as u8, g as u8, b as u8, 255],
        }
    }

    pub fn new_with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            value       : [r, g, b, a],
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptDrawCmd {
    DrawRect(ScriptRect, ScriptRGB),
    DrawGame(ScriptRect),
    DrawRegion(String, ScriptRect, i64),
    DrawText(ScriptPosition, String, String, f32, ScriptRGB),
}

// --- ScriptDraw

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptDraw {

    pub commands         : Vec<ScriptDrawCmd>,

}

impl ScriptDraw {
    pub fn new() -> Self {
        Self {
            commands    : vec![],
        }
    }

    pub fn rect(&mut self, rect: ScriptRect, rgb: ScriptRGB) {
        self.commands.push(ScriptDrawCmd::DrawRect(rect, rgb));
    }

    pub fn text(&mut self, pos: ScriptPosition, text: &str, font_name: &str, size: f64, rgb: ScriptRGB) {
        self.commands.push(ScriptDrawCmd::DrawText(pos, text.to_owned(), font_name.to_owned(), size as f32, rgb));
    }

    pub fn game(&mut self, rect: ScriptRect) {
        self.commands.push(ScriptDrawCmd::DrawGame(rect));
    }

    pub fn region(&mut self, name: &str, rect: ScriptRect, size: i64) {
        self.commands.push(ScriptDrawCmd::DrawRegion(name.to_owned(), rect, size));
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}