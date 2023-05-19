// This module contains script support structs. These are passed to the Rhai scripting engine to provide
// data structures which can be accessed from both Rust and scripts.

use crate::prelude::*;
use rhai::{ Engine };

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
    pub pos             : (usize, usize),
    pub pos_signed      : (isize, isize)
}

impl ScriptPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            pos         : (x as usize, y as usize),
            pos_signed  : (x as isize, y as isize),
        }
    }

    pub fn x(&mut self) -> i32 {
        self.pos.0 as i32
    }

    pub fn y(&mut self) -> i32 {
        self.pos.1 as i32
    }

    pub fn register(engine: &mut Engine) {
        engine.register_type_with_name::<ScriptPosition>("Position")
            .register_get("x", ScriptPosition::x)
            .register_get("y", ScriptPosition::y)
            .register_fn("pos", ScriptPosition::new);
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

    pub fn width(&mut self) -> i32 {
        self.rect.2 as i32
    }

    pub fn height(&mut self) -> i32 {
        self.rect.3 as i32
    }

    pub fn pos(&mut self) -> ScriptPosition {
        ScriptPosition { pos: (self.rect.0, self.rect.1), pos_signed: (self.rect.0 as isize, self.rect.1 as isize) }
    }

    pub fn is_inside(&mut self, pos: ScriptPosition) -> bool {
        pos.pos.0 >= self.rect.0 && pos.pos.1 >= self.rect.1 && pos.pos.0 < self.rect.0 + self.rect.2 && pos.pos.1 < self.rect.1 + self.rect.3
    }

    // Rust side only
    pub fn contains(&self, pos: (usize, usize)) -> bool {
        pos.0 >= self.rect.0 && pos.1 >= self.rect.1 && pos.0 < self.rect.0 + self.rect.2 && pos.1 < self.rect.1 + self.rect.3

    }

    pub fn register(engine: &mut Engine) {
        engine.register_type_with_name::<ScriptRect>("Rect")
            .register_fn("rect", ScriptRect::new)
            .register_fn("is_inside", ScriptRect::is_inside)
            .register_get("x", ScriptRect::x)
            .register_get("y", ScriptRect::y)
            .register_get("width", ScriptRect::width)
            .register_get("height", ScriptRect::height)
            .register_get("pos", ScriptRect::pos);
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

    pub fn new_with_alpha(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            value       : [r as u8, g as u8, b as u8, a as u8],
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
    DrawGame2D(ScriptRect),
    DrawGameOffset2D(ScriptRect, ScriptPosition),
    DrawGame3D(ScriptRect),
    DrawRegion(String, ScriptRect, i32),
    DrawText(ScriptPosition, String, String, f32, ScriptRGB),
    DrawTextRect(ScriptRect, String, String, f32, ScriptRGB, String),
    DrawMessages(ScriptRect, String, f32, ScriptRGB),
    DrawShape(ScriptShape),
}

// --- ScriptCommand

#[derive(PartialEq, Clone, Debug)]
pub enum ScriptServerCmd {
    Action(String, String, Option<String>),
    ActionCoordinate(String, Option<String>),
    ActionInventory(String, i32),
    ActionGear(String, i32),
    ActionValidMouseRect(ScriptRect),
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

    // Action

    pub fn action(&mut self, action: &str, direction: &str) {
        self.action_commands.push(ScriptServerCmd::Action(action.to_owned(), direction.to_owned().to_lowercase(), None));
    }

    pub fn action_coordinate(&mut self, action: &str) {
        self.action_commands.push(ScriptServerCmd::ActionCoordinate(action.to_owned(), None));
    }

    pub fn action_spell(&mut self, action: &str, direction: &str, spell: &str) {
        self.action_commands.push(ScriptServerCmd::Action(action.to_owned(), direction.to_owned().to_lowercase(), Some(spell.to_string())));
    }

    pub fn action_spell_coordinate(&mut self, action: &str, spell: &str) {
        self.action_commands.push(ScriptServerCmd::ActionCoordinate(action.to_owned(), Some(spell.to_string())));
    }

    // Valid Mouse Rect

    pub fn action_set_valid_mouse_rect(&mut self, rect: ScriptRect ) {
        self.action_commands.push(ScriptServerCmd::ActionValidMouseRect(rect));
    }

    // Gear Action

    pub fn action_gear(&mut self, action: &str, gear_index: i32 ) {
        self.action_commands.push(ScriptServerCmd::ActionGear(action.to_owned(), gear_index));
    }

    // Inventory Action

    pub fn action_inventory(&mut self, action: &str, inventory_index: i32 ) {
        self.action_commands.push(ScriptServerCmd::ActionInventory(action.to_owned(), inventory_index));
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

    pub fn draw_game_2d(&mut self, rect: ScriptRect) {
        self.draw_commands.push(ScriptDrawCmd::DrawGame2D(rect));
    }

    pub fn draw_game_offset_2d(&mut self, rect: ScriptRect, offset: ScriptPosition) {
        self.draw_commands.push(ScriptDrawCmd::DrawGameOffset2D(rect, offset));
    }

    pub fn draw_game_3d(&mut self, rect: ScriptRect) {
        self.draw_commands.push(ScriptDrawCmd::DrawGame3D(rect));
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

pub struct ScriptInfo {
    pub width               : i32,
    pub height              : i32,
    pub tile_size           : i32,
    pub player              : rhai::Map,
    pub tilemaps            : ScriptTilemaps,
    pub region              : rhai::Map,
    pub display_mode_3d     : bool,
    pub date                : Date,
}

impl ScriptInfo {
    pub fn new() -> Self {
        Self {
            width           : 0,
            height          : 0,
            tile_size       : 0,
            player          : rhai::Map::new(),
            tilemaps        : ScriptTilemaps::new(),
            region          : rhai::Map::new(),
            display_mode_3d : false,
            date            : Date::new(),
        }
    }
}

// Global functions

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref SHEET : Mutex<Sheet> = Mutex::new(Sheet::new());
    pub static ref SCRIPTCMD : Mutex<ScriptCmd> = Mutex::new(ScriptCmd::new());
    pub static ref MESSAGECMD : Mutex<ScriptMessageCmd> = Mutex::new(ScriptMessageCmd::new());
    pub static ref INFOCMD : Mutex<ScriptInfo> = Mutex::new(ScriptInfo::new());
}

/// Register the global cmd functions for drawing etc.
pub fn register_global_cmd_functions(engine: &mut Engine) {

    // Action Cmds

    engine.register_fn("action", |action: &str, direction: &str| {
        SCRIPTCMD.lock().unwrap().action_commands.push(ScriptServerCmd::Action(action.to_owned(), direction.to_owned().to_lowercase(), None));
    });

    engine.register_fn("action_at_coordinate", |action: &str| {
        SCRIPTCMD.lock().unwrap().action_commands.push(ScriptServerCmd::ActionCoordinate(action.to_owned(), None));
    });

    engine.register_fn("spell_at_coordinate", |spell: &str| {
        SCRIPTCMD.lock().unwrap().action_commands.push(ScriptServerCmd::ActionCoordinate("Cast".to_owned(), Some(spell.to_owned())));
    });

    engine.register_fn("action_gear", |action: &str,  gear_index: i32| {
        SCRIPTCMD.lock().unwrap().action_commands.push(ScriptServerCmd::ActionGear(action.to_owned(), gear_index));
    });

    engine.register_fn("action_inventory", |action: &str,  inventory_index: i32| {
        SCRIPTCMD.lock().unwrap().action_commands.push(ScriptServerCmd::ActionInventory(action.to_owned(), inventory_index));
    });

    engine.register_fn("set_valid_mouse_rect", |rect: ScriptRect| {
        SCRIPTCMD.lock().unwrap().action_commands.push(ScriptServerCmd::ActionValidMouseRect(rect));
    });

    // Draw Cmds

    engine.register_fn("draw_shape", |shape: ScriptShape | {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawShape(shape));
    });
    engine.register_fn("draw_rect", |rect: ScriptRect, rgb: ScriptRGB| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawRect(rect, rgb));
    });
    engine.register_fn("draw_tile", |pos: ScriptPosition, tile: ScriptTile| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawTile(pos, tile));
    });
    engine.register_fn("draw_tile_sat", |pos: ScriptPosition, tile: ScriptTile, rgb: ScriptRGB| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawTileSat(pos, tile, rgb));
    });
    engine.register_fn("draw_tile_mult", |pos: ScriptPosition, tile: ScriptTile, rgb: ScriptRGB| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawTileMult(pos, tile, rgb));
    });
    engine.register_fn("draw_tile_sized", |pos: ScriptPosition, tile: ScriptTile, size: i32| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawTileSized(pos, tile, size));
    });
    engine.register_fn("draw_frame", |rect: ScriptRect, tile: ScriptTile| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawFrame(rect, tile));
    });
    engine.register_fn("draw_frame_sat", |rect: ScriptRect, rgb: ScriptRGB, tile: ScriptTile| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawFrameSat(rect, rgb, tile));
    });
    engine.register_fn("draw_text", |pos: ScriptPosition, text: &str, font_name: &str, size: f32, rgb: ScriptRGB| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawText(pos, text.to_owned(), font_name.to_owned(), size as f32, rgb));
    });
    engine.register_fn("draw_text_rect", |rect: ScriptRect, text: &str, font_name: &str, size: f32, rgb: ScriptRGB, align: String| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawTextRect(rect, text.to_owned(), font_name.to_owned(), size as f32, rgb, align));
    });
    engine.register_fn("draw_messages", | rect: ScriptRect, font_name: &str, size: f32, rgb: ScriptRGB| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawMessages(rect, font_name.to_owned(), size as f32, rgb));
    });
    engine.register_fn("draw_game_2d", |rect: ScriptRect| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawGame2D(rect));
    });
    engine.register_fn("draw_game_offset_2d", |rect: ScriptRect, offset: ScriptPosition| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawGameOffset2D(rect, offset));
    });
    engine.register_fn("draw_game_3d", |rect: ScriptRect| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawGame3D(rect));
    });
    engine.register_fn("draw_region", |name: &str, rect: ScriptRect, size: i32| {
        SCRIPTCMD.lock().unwrap().draw_commands.push(ScriptDrawCmd::DrawRegion(name.to_owned(), rect, size));
    });

    // Message Cmds

    engine.register_fn("message_status", |message: &str| {
        MESSAGECMD.lock().unwrap().status(message);
    });

    // Info Cmds

    engine.register_fn("get_width", || -> i32 {
        INFOCMD.lock().unwrap().width
    });

    engine.register_fn("get_height", || -> i32 {
        INFOCMD.lock().unwrap().height
    });

    engine.register_fn("get_tile_size", || -> i32 {
        INFOCMD.lock().unwrap().tile_size
    });

    engine.register_fn("set_tile_size", |size: i32| {
        INFOCMD.lock().unwrap().tile_size = size;
    });

    engine.register_fn("get_player", || -> rhai::Map {
        INFOCMD.lock().unwrap().player.clone()
    });

    engine.register_fn("get_tilemaps", || -> ScriptTilemaps {
        INFOCMD.lock().unwrap().tilemaps.clone()
    });

    engine.register_fn("get_region", || -> rhai::Map {
        INFOCMD.lock().unwrap().region.clone()
    });

    engine.register_fn("get_display_mode_3d", || -> bool {
        INFOCMD.lock().unwrap().display_mode_3d
    });

    engine.register_fn("get_display_mode_2d", || -> bool {
        !INFOCMD.lock().unwrap().display_mode_3d
    });

    engine.register_fn("set_display_mode_3d", |display_mode: bool| {
        INFOCMD.lock().unwrap().display_mode_3d = display_mode;
    });

    engine.register_fn("set_display_mode_2d", |display_mode: bool| {
        INFOCMD.lock().unwrap().display_mode_3d = !display_mode;
    });

    engine.register_fn("get_inventory", || -> Inventory {
        SHEET.lock().unwrap().inventory.clone()
    });

    engine.register_fn("get_spells", || -> Spells {
        SHEET.lock().unwrap().spells.clone()
    });

    engine.register_fn("get_weapons", || -> Weapons {
        SHEET.lock().unwrap().weapons.clone()
    });

    engine.register_fn("get_gear", || -> Gear {
        SHEET.lock().unwrap().gear.clone()
    });

    engine.register_fn("get_wealth", || -> Currency {
        SHEET.lock().unwrap().wealth.clone()
    });

    engine.register_fn("get_skills", || -> Skills {
        SHEET.lock().unwrap().skills.clone()
    });

    engine.register_fn("get_experience", || -> Experience {
        SHEET.lock().unwrap().experience.clone()
    });

    engine.register_fn("get_date", || -> Date {
        INFOCMD.lock().unwrap().date.clone()
    });

}
