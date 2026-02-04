// Lib file needed when compiled for Xcode to a static library

use theframework::prelude::*;

pub mod circle;
use crate::circle::Circle;

pub mod prelude {
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

pub use prelude::*;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref APP: Mutex<Circle> = Mutex::new(Circle::new());
    static ref CTX: Mutex<TheContext> = Mutex::new(TheContext::new(800, 600));
}

#[no_mangle]
pub extern "C" fn rust_init() {
    APP.lock().unwrap().init(&mut CTX.lock().unwrap());
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn rust_draw(pixels: *mut u8, width: u32, height: u32) {
    let length = width as usize * height as usize * 4;
    let slice = unsafe { std::slice::from_raw_parts_mut(pixels, length) };

    CTX.lock().unwrap().width = width as usize;
    CTX.lock().unwrap().height = height as usize;

    APP.lock().unwrap().draw(slice, &mut CTX.lock().unwrap());
}

#[no_mangle]
pub extern "C" fn rust_update() -> bool {
    //println!("update");
    APP.lock().unwrap().update(&mut CTX.lock().unwrap())
}

#[no_mangle]
pub extern "C" fn rust_target_fps() -> u32 {
    30
}

#[no_mangle]
pub extern "C" fn rust_hover(x: f32, y: f32) -> bool {
    //println!("hover {} {}", x, y);
    APP.lock().unwrap().hover(x, y, &mut CTX.lock().unwrap())
}

#[no_mangle]
pub extern "C" fn rust_touch_down(x: f32, y: f32) -> bool {
    //println!("touch down {} {}", x, y);
    APP.lock()
        .unwrap()
        .touch_down(x, y, &mut CTX.lock().unwrap())
}

#[no_mangle]
pub extern "C" fn rust_touch_dragged(x: f32, y: f32) -> bool {
    //println!("touch dragged {} {}", x, y);
    APP.lock()
        .unwrap()
        .touch_dragged(x, y, &mut CTX.lock().unwrap())
}

#[no_mangle]
pub extern "C" fn rust_touch_up(x: f32, y: f32) -> bool {
    //println!("touch up {} {}", x, y);
    APP.lock().unwrap().touch_up(x, y, &mut CTX.lock().unwrap())
}

#[no_mangle]
pub extern "C" fn rust_touch_wheel(x: f32, y: f32) -> bool {
    //println!("touch up {} {}", x, y);
    APP.lock()
        .unwrap()
        .mouse_wheel((x as isize, y as isize), &mut CTX.lock().unwrap())
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn rust_key_down(p: *const c_char) -> bool {
    let c_str = unsafe { CStr::from_ptr(p) };
    if let Ok(key) = c_str.to_str() {
        if let Some(ch) = key.chars().next() {
            return APP
                .lock()
                .unwrap()
                .key_down(Some(ch), None, &mut CTX.lock().unwrap());
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn rust_special_key_down(key: u32) -> bool {
    if key == KEY_ESCAPE {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Escape), &mut CTX.lock().unwrap())
    } else if key == KEY_RETURN {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Return), &mut CTX.lock().unwrap())
    } else if key == KEY_DELETE {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Delete), &mut CTX.lock().unwrap())
    } else if key == KEY_UP {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Up), &mut CTX.lock().unwrap())
    } else if key == KEY_RIGHT {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Right), &mut CTX.lock().unwrap())
    } else if key == KEY_DOWN {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Down), &mut CTX.lock().unwrap())
    } else if key == KEY_LEFT {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Left), &mut CTX.lock().unwrap())
    } else if key == KEY_SPACE {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Space), &mut CTX.lock().unwrap())
    } else {
        //if key == KEY_TAB {
        APP.lock()
            .unwrap()
            .key_down(None, Some(TheKeyCode::Tab), &mut CTX.lock().unwrap())
    }
}

#[no_mangle]
pub extern "C" fn rust_key_modifier_changed(
    shift: bool,
    ctrl: bool,
    alt: bool,
    logo: bool,
) -> bool {
    APP.lock().unwrap().modifier_changed(shift, ctrl, alt, logo)
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn rust_dropped_file(p: *const c_char) {
    let path_str = unsafe { CStr::from_ptr(p) };
    if let Ok(path) = path_str.to_str() {
        APP.lock().unwrap().dropped_file(path.to_string());
    }
}

#[no_mangle]
pub extern "C" fn rust_open() {
    APP.lock().unwrap().open();
}

#[no_mangle]
pub extern "C" fn rust_save() {
    APP.lock().unwrap().save();
}

#[no_mangle]
pub extern "C" fn rust_save_as() {
    APP.lock().unwrap().save_as();
}

#[no_mangle]
pub extern "C" fn rust_cut() -> *mut c_char {
    let text = APP.lock().unwrap().cut();
    CString::new(text).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn rust_copy() -> *mut c_char {
    let text = APP.lock().unwrap().copy();
    CString::new(text).unwrap().into_raw()
}

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn rust_paste(p: *const c_char) {
    let text_str = unsafe { CStr::from_ptr(p) };
    if let Ok(text) = text_str.to_str() {
        APP.lock().unwrap().paste(text.to_string());
    }
}

#[no_mangle]
pub extern "C" fn rust_undo() {
    APP.lock().unwrap().undo();
}

#[no_mangle]
pub extern "C" fn rust_redo() {
    APP.lock().unwrap().redo();
}
