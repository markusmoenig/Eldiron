
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

// --- Tilemaps

#[derive(Debug, Clone)]
pub struct ScriptTilemaps {
    pub maps            : HashMap<String, i64>
}

impl ScriptTilemaps {
    pub fn new() -> Self {
        Self {
            maps: HashMap::new()
        }
    }

    /// Returns the tilemap
    pub fn get(&mut self, name: &str) -> ScriptTilemap {
        let mut rc : i64 = 0;
        if let Some(id) = self.maps.get(&name.to_owned()) {
            rc = *id;
        }
        ScriptTilemap { id: rc as usize }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptTilemap {
    pub         id : usize,
}

impl ScriptTilemap {
    pub fn new(id: usize) -> Self {
        Self {
            id
        }
    }

    /// Returns the tile
    pub fn get_tile(&mut self, x: i64, y: i64) -> ScriptTile {
        ScriptTile { id: (self.id, x as usize, y as usize) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptTile {
    pub id              : (usize, usize, usize),
}

impl ScriptTile {
    pub fn new(id: (usize, usize, usize)) -> Self {
        Self {
            id
        }
    }
}

// --- Drawing

// --- ScriptPosition

use std::{cmp::max, collections::HashMap};

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
    DrawTile(ScriptPosition, ScriptTile),
    DrawTileSat(ScriptPosition, ScriptTile, ScriptRGB),
    DrawTileSized(ScriptPosition, ScriptTile, i64),
    DrawFrame(ScriptRect, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile),
    DrawFrameSat(ScriptRect, ScriptRGB, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile, ScriptTile),
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

    pub fn tile(&mut self, pos: ScriptPosition, tile: ScriptTile) {
        self.commands.push(ScriptDrawCmd::DrawTile(pos, tile));
    }

    pub fn tile_sat(&mut self, pos: ScriptPosition, tile: ScriptTile, rgb: ScriptRGB) {
        self.commands.push(ScriptDrawCmd::DrawTileSat(pos, tile, rgb));
    }

    pub fn tile_sized(&mut self, pos: ScriptPosition, tile: ScriptTile, size: i64) {
        self.commands.push(ScriptDrawCmd::DrawTileSized(pos, tile, size));
    }

    pub fn frame(&mut self, rect: ScriptRect, t1: ScriptTile, t2: ScriptTile, t3: ScriptTile, t4: ScriptTile, t5: ScriptTile, t6: ScriptTile, t7: ScriptTile, t8: ScriptTile) {
        self.commands.push(ScriptDrawCmd::DrawFrame(rect, t1, t2, t3, t4, t5, t6, t7, t8));
    }

    pub fn frame_sat(&mut self, rect: ScriptRect, rgb: ScriptRGB, t1: ScriptTile, t2: ScriptTile, t3: ScriptTile, t4: ScriptTile, t5: ScriptTile, t6: ScriptTile, t7: ScriptTile, t8: ScriptTile) {
        self.commands.push(ScriptDrawCmd::DrawFrameSat(rect, rgb, t1, t2, t3, t4, t5, t6, t7, t8));
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