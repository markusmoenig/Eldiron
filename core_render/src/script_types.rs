// This module contains script support structs. These are passed to the Rhai scripting engine to provide
// data structures which can be accessed from both Rust and scripts.

use crate::prelude::*;

// --- Tilemaps

#[derive(Debug, Clone)]
pub struct ScriptTilemaps {
    pub maps            : HashMap<String, Uuid>
}

impl ScriptTilemaps {
    pub fn new() -> Self {
        Self {
            maps: HashMap::new()
        }
    }

    /// Returns the tilemap
    pub fn get(&mut self, name: &str) -> ScriptTilemap {
        let mut rc = Uuid::new_v4();
        if let Some(id) = self.maps.get(&name.to_owned()) {
            rc = *id;
        }
        ScriptTilemap { id: rc }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptTilemap {
    pub         id : Uuid,
}

impl ScriptTilemap {
    pub fn new(id: Uuid) -> Self {
        Self {
            id
        }
    }

    /// Returns the tile
    pub fn get_tile(&mut self, x: i32, y: i32) -> ScriptTile {
        ScriptTile { id: TileId::new(self.id, x as u16, y as u16) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptTile {
    pub id              : TileId,
}

impl ScriptTile {
    pub fn new(id: TileId) -> Self {
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
    pub fn new(x: i32, y: i32) -> Self {
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
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
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

    pub fn x(&mut self) -> i32 {
        self.rect.0 as i32
    }

    pub fn y(&mut self) -> i32 {
        self.rect.1 as i32
    }

    pub fn pos(&mut self) -> ScriptPosition {
        ScriptPosition { pos: (self.rect.0, self.rect.1) }
    }

    pub fn is_inside(&mut self, pos: ScriptPosition) -> bool {
        pos.pos.0 >= self.rect.0 && pos.pos.1 >= self.rect.1 && pos.pos.0 < self.rect.0 + self.rect.2 && pos.pos.1 < self.rect.1 + self.rect.3
    }

}

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptRGB {
    pub value            : [u8;4]
}

// --- ScriptRGB

impl ScriptRGB {
    pub fn new(r: i32, g: i32, b: i32) -> Self {
        Self {
            value       : [r as u8, g as u8, b as u8, 255],
        }
    }

    pub fn new_with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            value       : [r, g, b, a],
        }
    }

    pub fn to_normalized(&self) -> [f32; 4] {
        [(self.value[0] as f32) / 255.0, (self.value[1] as f32) / 255.0, (self.value[2] as f32) / 255.0, (self.value[3] as f32) / 255.0]
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptDrawCmd {
    DrawRect(ScriptRect, ScriptRGB),
    DrawTile(ScriptPosition, ScriptTile),
    DrawTileSat(ScriptPosition, ScriptTile, ScriptRGB),
    DrawTileMult(ScriptPosition, ScriptTile, ScriptRGB),
    DrawTileSized(ScriptPosition, ScriptTile, i32),
    DrawFrame(ScriptRect, ScriptTile),
    DrawFrameSat(ScriptRect, ScriptRGB, ScriptTile),
    DrawGame(ScriptRect),
    DrawRegion(String, ScriptRect, i32),
    DrawText(ScriptPosition, String, String, f32, ScriptRGB),
    DrawTextRect(ScriptRect, String, String, f32, ScriptRGB, String),
    DrawMessages(ScriptRect, String, f32, ScriptRGB),
    DrawShape(ScriptShape),
}

// --- ScriptCommand

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptServerCmd {
    Action(String, String),
}

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptCmd {
    pub draw_commands           : Vec<ScriptDrawCmd>,
    pub action_commands         : Vec<ScriptServerCmd>
}

impl ScriptCmd {
    pub fn new() -> Self {
        Self {
            draw_commands       : vec![],
            action_commands     : vec![],
        }
    }

    // Actions

    pub fn action(&mut self, action: &str, direction: &str) {
        self.action_commands.push(ScriptServerCmd::Action(action.to_owned(), direction.to_owned().to_lowercase()));
    }

    // Draw

    pub fn draw_shape(&mut self, shape: ScriptShape) {
        self.draw_commands.push(ScriptDrawCmd::DrawShape(shape));
    }

    pub fn draw_rect(&mut self, rect: ScriptRect, rgb: ScriptRGB) {
        self.draw_commands.push(ScriptDrawCmd::DrawRect(rect, rgb));
    }

    pub fn draw_tile(&mut self, pos: ScriptPosition, tile: ScriptTile) {
        self.draw_commands.push(ScriptDrawCmd::DrawTile(pos, tile));
    }

    pub fn draw_tile_sat(&mut self, pos: ScriptPosition, tile: ScriptTile, rgb: ScriptRGB) {
        self.draw_commands.push(ScriptDrawCmd::DrawTileSat(pos, tile, rgb));
    }

    pub fn draw_tile_mult(&mut self, pos: ScriptPosition, tile: ScriptTile, rgb: ScriptRGB) {
        self.draw_commands.push(ScriptDrawCmd::DrawTileMult(pos, tile, rgb));
    }

    pub fn draw_tile_sized(&mut self, pos: ScriptPosition, tile: ScriptTile, size: i32) {
        self.draw_commands.push(ScriptDrawCmd::DrawTileSized(pos, tile, size));
    }

    pub fn draw_frame(&mut self, rect: ScriptRect, tile: ScriptTile) {
        self.draw_commands.push(ScriptDrawCmd::DrawFrame(rect, tile));
    }

    pub fn draw_frame_sat(&mut self, rect: ScriptRect, rgb: ScriptRGB, tile: ScriptTile) {
        self.draw_commands.push(ScriptDrawCmd::DrawFrameSat(rect, rgb, tile));
    }

    pub fn draw_text(&mut self, pos: ScriptPosition, text: &str, font_name: &str, size: f32, rgb: ScriptRGB) {
        self.draw_commands.push(ScriptDrawCmd::DrawText(pos, text.to_owned(), font_name.to_owned(), size as f32, rgb));
    }

    pub fn draw_text_rect(&mut self, rect: ScriptRect, text: &str, font_name: &str, size: f32, rgb: ScriptRGB, align: String) {
        self.draw_commands.push(ScriptDrawCmd::DrawTextRect(rect, text.to_owned(), font_name.to_owned(), size as f32, rgb, align));
    }

    pub fn draw_messages(&mut self, rect: ScriptRect, font_name: &str, size: f32, rgb: ScriptRGB) {
        self.draw_commands.push(ScriptDrawCmd::DrawMessages(rect, font_name.to_owned(), size as f32, rgb));
    }

    pub fn draw_game(&mut self, rect: ScriptRect) {
        self.draw_commands.push(ScriptDrawCmd::DrawGame(rect));
    }

    pub fn draw_region(&mut self, name: &str, rect: ScriptRect, size: i32) {
        self.draw_commands.push(ScriptDrawCmd::DrawRegion(name.to_owned(), rect, size));
    }

    pub fn clear_draw(&mut self) {
        self.draw_commands.clear();
    }

    pub fn clear_action(&mut self) {
        self.action_commands.clear();
    }

    pub fn clear_all(&mut self) {
        self.draw_commands.clear();
        self.action_commands.clear();
    }
}