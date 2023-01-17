mod editor;
mod widget;

pub mod prelude {
    pub const GAME_TICK_IN_MS   : u128 = 250;

    pub const KEY_ESCAPE        : u32 = 0;
    pub const KEY_RETURN        : u32 = 1;
    pub const KEY_DELETE        : u32 = 2;
    pub const KEY_UP            : u32 = 3;
    pub const KEY_RIGHT         : u32 = 4;
    pub const KEY_DOWN          : u32 = 5;
    pub const KEY_LEFT          : u32 = 6;
    pub const KEY_SPACE         : u32 = 7;
    pub const KEY_TAB           : u32 = 8;

    pub use serde::{Deserialize, Serialize};

    pub use core_server::prelude::*;
    pub use core_shared::prelude::*;
    pub use core_render::prelude::*;

    pub use crate::draw2d::Draw2D;
    pub use crate::widget::*;
    pub use crate::widget::hlayout::*;
    pub use crate::widget::vlayout::*;
    pub use crate::editor::*;
    pub use crate::context::*;
    pub use crate::atom::*;
    pub use crate::tilemapwidget::*;
    pub use crate::tileselector::*;
    pub use crate::characterselector::*;
    pub use crate::lootselector::*;

    pub use crate::editor::dialog::*;
    pub use crate::editor::codeeditorwidget::*;
    pub use crate::editor::toolbar::*;
    pub use crate::editor::controlbar::*;
    pub use crate::editor::node::*;
    pub use crate::editor::tilemapwidget::*;
    pub use crate::editor::regionoptions::*;
    pub use crate::editor::regionwidget::*;
    pub use crate::editor::traits::*;
    pub use crate::editor::nodegraph::*;
    pub use crate::editor::behavioroptions::*;
    pub use crate::editor::behavior_overview_options::*;
    pub use crate::editor::systemsoptions::*;
    pub use crate::editor::systems_overview_options::*;
    pub use crate::editor::itemsoptions::*;
    pub use crate::editor::items_overview_options::*;
    pub use crate::editor::region_overview_options::*;
    pub use crate::editor::gameoptions::*;
    pub use crate::dialog_position::*;
    pub use crate::screeneditor_options::*;
    pub use crate::tilemapoptions::*;
    pub use crate::statusbar::*;
    pub use crate::node_preview::*;
    pub use crate::assets_overview_options::*;

    pub use code_editor::prelude::*;
}

use prelude::*;

use lazy_static::lazy_static;
use std::sync::Mutex;

use std::os::raw::{c_char};
use std::ffi::{CStr};

lazy_static! {
    static ref EDITOR : Mutex<Editor<'static>> = Mutex::new(Editor::new(1248, 700));
}

#[no_mangle]
pub extern "C" fn rust_draw(pixels: *mut u8, width: u32, height: u32, anim_counter: usize) {
    let length = width as usize * height as usize * 4;
    let slice = unsafe { std::slice::from_raw_parts_mut(pixels, length) };

    EDITOR.lock().unwrap().draw(slice, width as usize, height as usize, anim_counter);
}

#[no_mangle]
pub extern "C" fn rust_init(p: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(p) };
    if let Some(path) = c_str.to_str().ok() {
        EDITOR.lock().unwrap().init(path.to_string());
    }
}

#[no_mangle]
pub extern "C" fn rust_target_fps() -> u32 {
    EDITOR.lock().unwrap().get_target_fps() as u32
}

#[no_mangle]
pub extern "C" fn rust_hover(x: f32, y: f32) -> bool {
    //println!("hover {} {}", x, y);
    EDITOR.lock().unwrap().mouse_hover((x as usize, y as usize))
}

#[no_mangle]
pub extern "C" fn rust_touch_down(x: f32, y: f32) -> bool {
    // println!("touch down {} {}", x, y);
    EDITOR.lock().unwrap().mouse_down((x as usize, y as usize))
}

#[no_mangle]
pub extern "C" fn rust_touch_dragged(x: f32, y: f32) -> bool {
    //println!("touch dragged {} {}", x, y);
    EDITOR.lock().unwrap().mouse_dragged((x as usize, y as usize))
}

#[no_mangle]
pub extern "C" fn rust_touch_up(x: f32, y: f32) -> bool {
    //println!("touch up {} {}", x, y);
    EDITOR.lock().unwrap().mouse_up((x as usize, y as usize))
}

#[no_mangle]
pub extern "C" fn rust_touch_wheel(x: f32, y: f32) -> bool {
    //println!("touch up {} {}", x, y);
    EDITOR.lock().unwrap().mouse_wheel((x as isize, y as isize))
}

#[no_mangle]
pub extern "C" fn rust_key_down(p: *const c_char) -> bool {
    let c_str = unsafe { CStr::from_ptr(p) };
    if let Some(key) = c_str.to_str().ok() {
        if let Some(ch ) = key.chars().next() {
            return EDITOR.lock().unwrap().key_down(Some(ch), None);
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn rust_special_key_down(key: u32) -> bool {
    if key == KEY_ESCAPE {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Escape))
    } else
    if key == KEY_RETURN {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Return))
    } else
    if key == KEY_DELETE {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Delete))
    } else
    if key == KEY_UP {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Up))
    } else
    if key == KEY_RIGHT {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Right))
    } else
    if key == KEY_DOWN {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Down))
    } else
    if key == KEY_LEFT {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Left))
    } else
    if key == KEY_SPACE {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Space))
    } else {
    //if key == KEY_TAB {
        EDITOR.lock().unwrap().key_down(None, Some(WidgetKey::Tab))
    }
}

// Get the the current time in ms
fn _get_time() -> u128 {
    let stop = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
        stop.as_millis()
}