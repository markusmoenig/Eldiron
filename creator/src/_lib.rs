// Lib file needed when compiled for Xcode to a static library

// use theframework::*;
pub mod actionlist;
pub mod actions;
pub mod codeeditor;
pub mod configeditor;
pub mod dockmanager;
pub mod docks;
pub mod editcamera;
pub mod editor;
pub mod effectpicker;
pub mod hud;
pub mod infoviewer;
pub mod mapeditor;
pub mod minimap;
pub mod misc;
pub mod nodeeditor;
pub mod panels;
pub mod rendereditor;
pub mod self_update;
pub mod shapepicker;
pub mod sidebar;
pub mod tilemapeditor;
pub mod tilepicker;
pub mod toollist;
pub mod tools;
pub mod undo;
pub mod utils;
pub mod worldeditor;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

const DEFAULT_VLAYOUT_RATIO: f32 = 0.62;

pub mod prelude {

    pub use ::serde::{Deserialize, Serialize};

    pub use shared::prelude::*;
    pub use std::sync::{LazyLock, RwLock};
    pub use theframework::prelude::*;

    pub use crate::actionlist::*;
    pub use crate::codeeditor::*;
    pub use crate::effectpicker::*;
    pub use crate::mapeditor::*;
    pub use crate::misc::*;
    pub use crate::panels::*;
    pub use crate::shapepicker::*;
    pub use crate::sidebar::*;
    pub use crate::tilemapeditor::*;
    pub use crate::tilepicker::*;
    pub use crate::toollist::*;
    pub use crate::undo::material_undo::*;
    pub use crate::undo::palette_undo::*;
    pub use crate::undo::region_undo::*;
    pub use crate::undo::*;
    pub use crate::utils::*;

    pub use crate::actions::*;
    pub use crate::docks::*;
    pub use crate::tools::*;

    pub use crate::tools::code::CodeTool;
    pub use crate::tools::game::GameTool;
    pub use crate::tools::linedef::LinedefTool;
    pub use crate::tools::sector::SectorTool;
    pub use crate::tools::selection::SelectionTool;
    pub use crate::tools::tileset::TilesetTool;
    pub use crate::tools::vertex::VertexTool;

    pub use crate::docks::tiles::*;

    pub use crate::configeditor::ConfigEditor;
    pub use crate::dockmanager::DockManager;
    pub use crate::editcamera::{CustomMoveAction, EditCamera};
    pub use crate::infoviewer::InfoViewer;
    pub use crate::nodeeditor::{NodeContext, NodeEditor};
    pub use crate::rendereditor::{RenderEditor, RenderMoveAction};
    pub use crate::worldeditor::WorldEditor;

    pub use toml::Table;

    pub const KEY_ESCAPE: u32 = 0;
    pub const KEY_RETURN: u32 = 1;
    pub const KEY_DELETE: u32 = 2;
    pub const KEY_UP: u32 = 3;
    pub const KEY_RIGHT: u32 = 4;
    pub const KEY_DOWN: u32 = 5;
    pub const KEY_LEFT: u32 = 6;
    pub const KEY_SPACE: u32 = 7;
    pub const KEY_TAB: u32 = 8;
}

use crate::editor::Editor;

pub use prelude::*;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref APP: Mutex<Editor> = Mutex::new(Editor::new());
    static ref CTX: Mutex<TheContext> = Mutex::new(TheContext::new(800, 600));
    static ref UI: Mutex<TheUI> = Mutex::new(TheUI::new());
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_init() {
    UI.lock().unwrap().init(&mut CTX.lock().unwrap());
    APP.lock().unwrap().init(&mut CTX.lock().unwrap());
    APP.lock()
        .unwrap()
        .init_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());
}

/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_draw(pixels: *mut u8, width: u32, height: u32) {
    let length = width as usize * height as usize * 4;
    let slice = unsafe { std::slice::from_raw_parts_mut(pixels, length) };

    CTX.lock().unwrap().width = width as usize;
    CTX.lock().unwrap().height = height as usize;

    UI.lock().unwrap().draw(slice, &mut CTX.lock().unwrap());
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_update() -> bool {
    //println!("update");
    UI.lock().unwrap().update(&mut CTX.lock().unwrap());
    APP.lock()
        .unwrap()
        .update_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());
    APP.lock().unwrap().update(&mut CTX.lock().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_target_fps() -> u32 {
    30
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_hover(x: f32, y: f32) -> bool {
    //println!("hover {} {}", x, y);
    UI.lock().unwrap().hover(x, y, &mut CTX.lock().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_down(x: f32, y: f32) -> bool {
    //println!("touch down {} {}", x, y);
    UI.lock()
        .unwrap()
        .touch_down(x, y, &mut CTX.lock().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_dragged(x: f32, y: f32) -> bool {
    //println!("touch dragged {} {}", x, y);
    UI.lock()
        .unwrap()
        .touch_dragged(x, y, &mut CTX.lock().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_up(x: f32, y: f32) -> bool {
    //println!("touch up {} {}", x, y);
    UI.lock().unwrap().touch_up(x, y, &mut CTX.lock().unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_wheel(x: f32, y: f32) -> bool {
    //println!("touch up {} {}", x, y);
    UI.lock()
        .unwrap()
        .mouse_wheel((x as i32, y as i32), &mut CTX.lock().unwrap())
}

/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_key_down(p: *const c_char) -> bool {
    let c_str = unsafe { CStr::from_ptr(p) };
    if let Ok(key) = c_str.to_str() {
        if let Some(ch) = key.chars().next() {
            return UI
                .lock()
                .unwrap()
                .key_down(Some(ch), None, &mut CTX.lock().unwrap());
        }
    }
    false
}

/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_key_up(p: *const c_char) -> bool {
    let c_str = unsafe { CStr::from_ptr(p) };
    if let Ok(key) = c_str.to_str() {
        if let Some(ch) = key.chars().next() {
            return UI
                .lock()
                .unwrap()
                .key_up(Some(ch), None, &mut CTX.lock().unwrap());
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_special_key_down(key: u32) -> bool {
    if key == KEY_ESCAPE {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Escape), &mut CTX.lock().unwrap())
    } else if key == KEY_RETURN {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Return), &mut CTX.lock().unwrap())
    } else if key == KEY_DELETE {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Delete), &mut CTX.lock().unwrap())
    } else if key == KEY_UP {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Up), &mut CTX.lock().unwrap())
    } else if key == KEY_RIGHT {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Right), &mut CTX.lock().unwrap())
    } else if key == KEY_DOWN {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Down), &mut CTX.lock().unwrap())
    } else if key == KEY_LEFT {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Left), &mut CTX.lock().unwrap())
    } else if key == KEY_SPACE {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Space), &mut CTX.lock().unwrap())
    } else {
        //if key == KEY_TAB {
        UI.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Tab), &mut CTX.lock().unwrap())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_key_modifier_changed(
    shift: bool,
    ctrl: bool,
    alt: bool,
    logo: bool,
) -> bool {
    UI.lock()
        .unwrap()
        .modifier_changed(shift, ctrl, alt, logo, &mut CTX.lock().unwrap());
    APP.lock().unwrap().modifier_changed(shift, ctrl, alt, logo)
}

/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_dropped_file(p: *const c_char) {
    let path_str = unsafe { CStr::from_ptr(p) };
    if let Ok(path) = path_str.to_str() {
        APP.lock().unwrap().dropped_file(path.to_string());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_open() {
    APP.lock().unwrap().open();
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_save() {
    APP.lock().unwrap().save();
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_save_as() {
    APP.lock().unwrap().save_as();
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_cut() -> *mut c_char {
    let text = APP.lock().unwrap().cut();
    CString::new(text).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_copy() -> *mut c_char {
    let text = APP.lock().unwrap().copy();
    CString::new(text).unwrap().into_raw()
}

/// # Safety
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_paste(p: *const c_char) {
    let text_str = unsafe { CStr::from_ptr(p) };
    if let Ok(text) = text_str.to_str() {
        APP.lock().unwrap().paste(text.to_string());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_undo() {
    APP.lock().unwrap().undo();
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_redo() {
    APP.lock().unwrap().redo();
}
