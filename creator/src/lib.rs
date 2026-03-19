#[cfg(not(target_arch = "wasm32"))]
#[macro_use]
mod macros;

pub mod actionlist;
pub mod actions;
pub mod configeditor;
pub mod dockmanager;
pub mod docks;
pub mod editcamera;
pub mod editor;
pub mod editor_tools;
pub mod hud;
#[cfg(not(target_arch = "wasm32"))]
pub mod i18n;
pub mod mapeditor;
pub mod minimap;
pub mod misc;
#[cfg(all(not(target_arch = "wasm32"), feature = "self-update"))]
pub mod self_update;
pub mod sidebar;
pub mod textplay;
pub mod toollist;
pub mod tools;
pub mod undo;
pub mod utils;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

pub const DEFAULT_VLAYOUT_RATIO: f32 = 0.62;

#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use ::serde::{Deserialize, Serialize};

    pub use codegridfx::prelude::*;
    pub use shared::prelude::*;
    pub use std::sync::{LazyLock, RwLock};
    pub use theframework::prelude::*;

    pub use crate::mapeditor::*;
    pub use crate::misc::*;
    // pub use crate::previewview::*;
    pub use crate::actionlist::*;
    pub use crate::sidebar::*;
    pub use crate::textplay::*;
    pub use crate::toollist::*;
    pub use crate::undo::project_atoms::*;
    pub use crate::undo::project_helper::*;
    pub use crate::undo::project_undo::*;
    pub use crate::undo::*;
    pub use crate::utils::*;

    pub use crate::tools::game::GameTool;
    pub use crate::tools::linedef::LinedefTool;
    pub use crate::tools::sector::SectorTool;
    // pub use crate::tools::tileset::TilesetTool;
    pub use crate::tools::vertex::VertexTool;

    pub use crate::docks::tiles::*;

    pub use crate::actions::*;
    pub use crate::docks::*;
    pub use crate::editor_tools::*;
    pub use crate::tools::*;

    pub use crate::configeditor::ConfigEditor;
    pub use crate::editcamera::{CustomMoveAction, EditCamera};

    pub use crate::dockmanager::{DockManager, DockManagerState};

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

// --- FFI exports for the Xcode static library build ---
// Build with: cargo build -p eldiron-creator --lib --release --no-default-features --features staticlib

#[cfg(feature = "staticlib")]
mod ffi {
    use super::editor::Editor;
    use super::prelude::*;

    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::ptr;

    use lazy_static::lazy_static;
    use std::sync::Mutex;

    lazy_static! {
        static ref APP: Mutex<Editor> = Mutex::new(Editor::new());
        static ref CTX: Mutex<TheContext> = Mutex::new(TheContext::new(800, 600, 1.0));
        static ref UI: Mutex<TheUI> = Mutex::new(TheUI::new());
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_init() {
        UI.lock().unwrap().init(&mut CTX.lock().unwrap());
        APP.lock().unwrap().init(&mut CTX.lock().unwrap());
        APP.lock()
            .unwrap()
            .init_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());

        // Keep startup behavior aligned with winit path.
        APP.lock().unwrap().set_cmd_line_args(
            vec!["eldiron-creator".to_string()],
            &mut CTX.lock().unwrap(),
        );
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_resize(width: u32, height: u32, scale_factor: f32) {
        let mut ctx = CTX.lock().unwrap();
        if ctx.width != width as usize
            || ctx.height != height as usize
            || (ctx.scale_factor - scale_factor).abs() > f32::EPSILON
        {
            ctx.width = width as usize;
            ctx.height = height as usize;
            ctx.scale_factor = scale_factor;
            ctx.ui.relayout = true;
            ctx.ui.redraw_all = true;
        }
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
        UI.lock().unwrap().update(&mut CTX.lock().unwrap());
        APP.lock()
            .unwrap()
            .update_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());
        APP.lock().unwrap().update(&mut CTX.lock().unwrap())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_target_fps() -> u32 {
        APP.lock().unwrap().target_fps().clamp(1.0, 120.0) as u32
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_hover(x: f32, y: f32) -> bool {
        UI.lock().unwrap().hover(x, y, &mut CTX.lock().unwrap())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_touch_down(x: f32, y: f32) -> bool {
        UI.lock()
            .unwrap()
            .touch_down(x, y, &mut CTX.lock().unwrap())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_touch_dragged(x: f32, y: f32) -> bool {
        UI.lock()
            .unwrap()
            .touch_dragged(x, y, &mut CTX.lock().unwrap())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_touch_up(x: f32, y: f32) -> bool {
        UI.lock().unwrap().touch_up(x, y, &mut CTX.lock().unwrap())
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_touch_wheel(x: f32, y: f32) -> bool {
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
    pub extern "C" fn rust_new() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("New"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_play() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Play"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_pause() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Pause"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_stop() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Stop"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_open() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Open"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_close() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Close"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_save() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Save"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_save_as() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Save As"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_settings() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Settings"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_rules() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Rules"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_locales() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Locales"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_audio_fx() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Audio FX"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_authoring() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Authoring"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_debug_log() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Debug Log"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_show_console() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Show Console"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_cut() -> *mut c_char {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Cut"),
            TheWidgetState::Clicked,
        ));
        APP.lock()
            .unwrap()
            .update_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());

        if let Some(TheValue::Text(text)) = &CTX.lock().unwrap().ui.clipboard {
            return CString::new(text.clone()).unwrap().into_raw();
        }
        ptr::null_mut()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_copy() -> *mut c_char {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Copy"),
            TheWidgetState::Clicked,
        ));
        APP.lock()
            .unwrap()
            .update_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());

        if let Some(TheValue::Text(text)) = &CTX.lock().unwrap().ui.clipboard {
            return CString::new(text.clone()).unwrap().into_raw();
        }
        ptr::null_mut()
    }

    /// # Safety
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn rust_paste(p: *const c_char) {
        let text_str = unsafe { CStr::from_ptr(p) };
        if let Ok(text) = text_str.to_str() {
            {
                let mut ctx = CTX.lock().unwrap();
                ctx.ui.clipboard = Some(TheValue::Text(text.to_string()));
                ctx.ui.clipboard_app_type = Some("text/plain".to_string());
            }

            CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
                TheId::named("Paste"),
                TheWidgetState::Clicked,
            ));

            APP.lock()
                .unwrap()
                .update_ui(&mut UI.lock().unwrap(), &mut CTX.lock().unwrap());
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_undo() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Undo"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_redo() {
        CTX.lock().unwrap().ui.send(TheEvent::StateChanged(
            TheId::named("Redo"),
            TheWidgetState::Clicked,
        ));
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn rust_has_changes() -> bool {
        APP.lock().unwrap().has_changes()
    }
}
