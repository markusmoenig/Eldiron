
// This module contains script support structs. These are passed to the Rhai scripting engine to provide
// data structures which can be accessed from both Rust and scripts.

use crate::gamedata::MessageType;

// --- Sending Messages

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
}

// --- Drawing

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptRect {
    pub rect            : (usize, usize, usize, usize)
}

impl ScriptRect {
    pub fn new(x: i64, y: i64, width: i64, height: i64) -> Self {
        Self {
            rect        : (x as usize, y as usize, width as usize,  height as usize),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptRGB {
    pub value            : [u8;4]
}

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
}

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

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}