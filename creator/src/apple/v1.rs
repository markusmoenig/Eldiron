//! Versioned, handle-scoped entry points. Legacy symbols remain compatibility
//! adapters; new hosts retain a handle and capture it in every delayed callback.
use super::*;
const VERSION: u32 = 1;
fn valid(handle: u64) -> bool {
    handle != 0
        && unsafe { pthread_main_np() } != 0
        && APP.with(|app| app.try_borrow().ok().is_some_and(|app| app.id() == handle))
}
fn with_host<T>(handle: u64, f: impl FnOnce(&mut CreatorHost) -> T) -> Option<T> {
    with_hosted_handle(handle, |host| f(&mut host.app))
}
fn json(
    handle: u64,
    f: impl FnOnce(&mut CreatorHost) -> Result<serde_json::Value, String>,
) -> *mut c_char {
    let Some(result) = with_host(handle, f) else {
        return std::ptr::null_mut();
    };
    let value = match result {
        Ok(value) => serde_json::json!({"ok":true,"value":value}),
        Err(error) => serde_json::json!({"ok":false,"error":error}),
    };
    CString::new(value.to_string()).unwrap().into_raw()
}
unsafe fn document(ptr: *const c_char) -> Result<Uuid, String> {
    unsafe { string(ptr) }
        .ok_or("Missing document identity")?
        .parse()
        .map_err(|_| "Invalid document identity".into())
}
unsafe fn path(ptr: *const c_char) -> Result<Option<std::path::PathBuf>, String> {
    if ptr.is_null() {
        Ok(None)
    } else {
        unsafe { string(ptr) }
            .map(|s| Some(s.into()))
            .ok_or("Invalid UTF-8 document path".into())
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_abi_version() -> u32 {
    VERSION
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_create_v1(
    version: u32,
    layer: *mut c_void,
    width: u32,
    height: u32,
    density: f32,
) -> u64 {
    if version != VERSION || !unsafe { rust_init(layer, width, height, density) } {
        return 0;
    }
    APP.with(|app| {
        let mut app = app.borrow_mut();
        #[cfg(not(target_os = "tvos"))]
        if let Some(host) = app.as_mut() {
            host.app.application.application.host_enable_document_io();
        }
        app.id()
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_destroy_v1(handle: u64) -> bool {
    if !valid(handle) {
        return false;
    }
    rust_shutdown();
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_render_v1(handle: u64) -> i32 {
    if !valid(handle) {
        return -1;
    }
    rust_render()
}
// These synchronous main-thread forwards preserve the established input/text
// implementation. Owner validation ends before entering its legacy borrow.
macro_rules! queries { ($($name:ident => $old:ident($($arg:ident:$ty:ty),*) -> $ret:ty = $empty:expr;)+) => {$ (
    #[unsafe(no_mangle)] pub extern "C" fn $name(handle:u64,$($arg:$ty),*) -> $ret {
        if !valid(handle) { return $empty; } super::$old($($arg),*)
    }
)+}; }
macro_rules! setters { ($($name:ident => $old:ident($($arg:ident:$ty:ty),*);)+) => {$ (
    #[unsafe(no_mangle)] pub extern "C" fn $name(handle:u64,$($arg:$ty),*) -> bool {
        if !valid(handle) { return false; } super::$old($($arg),*); true
    }
)+}; }
queries! {
    eldiron_creator_state_v1 => rust_host_state() -> *mut c_char = std::ptr::null_mut();
    eldiron_creator_requests_v1 => rust_take_requests() -> *mut c_char = std::ptr::null_mut();
    eldiron_creator_text_snapshot_v1 => rust_text_snapshot() -> *mut c_char = std::ptr::null_mut();
    eldiron_creator_text_query_v1 => rust_text_query(target:u64,revision:u64,kind:u32,start:isize,end:isize,x:f32,y:f32) -> *mut c_char = std::ptr::null_mut();
    eldiron_creator_hover_v1 => rust_hover(x:f32,y:f32) -> bool = false;
    eldiron_creator_touch_down_v1 => rust_touch_down(x:f32,y:f32) -> bool = false;
    eldiron_creator_touch_up_v1 => rust_touch_up(x:f32,y:f32) -> bool = false;
    eldiron_creator_touch_dragged_v1 => rust_touch_dragged(x:f32,y:f32) -> bool = false;
    eldiron_creator_right_touch_down_v1 => rust_right_touch_down(x:f32,y:f32) -> bool = false;
    eldiron_creator_right_touch_up_v1 => rust_right_touch_up(x:f32,y:f32) -> bool = false;
    eldiron_creator_right_touch_dragged_v1 => rust_right_touch_dragged(x:f32,y:f32) -> bool = false;
    eldiron_creator_mouse_motion_v1 => rust_mouse_motion(x:f32,y:f32) -> bool = false;
    eldiron_creator_mouse_leave_v1 => rust_mouse_leave() -> bool = false;
    eldiron_creator_scroll_v1 => rust_touch_wheel(x:f32,y:f32) -> bool = false;
    eldiron_creator_precise_scroll_v1 => rust_touch_precise_scroll(x:f32,y:f32) -> bool = false;
    eldiron_creator_pinch_v1 => rust_pinch(delta:f32) -> bool = false;
    eldiron_creator_modifiers_v1 => rust_key_modifier_changed(shift:bool,control:bool,alt:bool,super_key:bool) -> bool = false;
    eldiron_creator_has_changes_v1 => rust_has_changes() -> bool = false;
}
setters! {
    eldiron_creator_resize_v1 => rust_resize(width:u32,height:u32,density:f32);
    eldiron_creator_suspend_v1 => rust_suspend();
    eldiron_creator_resume_v1 => rust_resume();
    eldiron_creator_focus_v1 => rust_focus(focused:bool);
    eldiron_creator_edit_command_v1 => rust_edit_command(command:u32);
}
/// Stable menu codes; file dialogs can instead use the byte transaction API.
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_command_v1(handle: u64, code: u32) -> bool {
    if !valid(handle) {
        return false;
    }
    let name = match code {
        0 => "New",
        1 => "Open",
        2 => "Close",
        3 => "Save",
        4 => "Save As",
        5 => "Play",
        6 => "Pause",
        7 => "Stop",
        8 => "Show Settings",
        9 => "Show Rules",
        10 => "Show Locales",
        11 => "Show Audio FX",
        12 => "Show Authoring",
        13 => "Show Project Sidebar",
        14 => "Show Actions Sidebar",
        15 => "Show Debug Log",
        16 => "Show Console",
        17 => "Show Help Sidebar",
        _ => return false,
    };
    command(name);
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_set_theme_v1(handle: u64, theme: u32) -> bool {
    if !valid(handle) || theme > 2 {
        return false;
    }
    rust_set_creator_theme(theme);
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_get_theme_v1(handle: u64) -> u32 {
    if valid(handle) {
        rust_get_creator_theme()
    } else {
        u32::MAX
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_drop_file_v1(handle: u64, path: *const c_char) -> bool {
    if !valid(handle) || path.is_null() {
        return false;
    }
    unsafe { rust_dropped_file(path) };
    true
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_replace_layer_v1(handle: u64, layer: *mut c_void) -> bool {
    valid(handle) && !layer.is_null() && unsafe { rust_replace_layer(layer) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_key_v1(
    handle: u64,
    code: u32,
    pressed: bool,
    repeated: bool,
    logical: *const c_char,
) -> bool {
    valid(handle) && unsafe { rust_physical_key(code, pressed, repeated, logical) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_clipboard_complete_v1(
    handle: u64,
    request: u64,
    write: bool,
    success: bool,
    text: *const c_char,
) -> bool {
    if !valid(handle) {
        return false;
    }
    unsafe { rust_clipboard_complete(request, write, success, text) };
    true
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_text_edit_v1(
    handle: u64,
    target: u64,
    revision: u64,
    kind: u32,
    start: isize,
    end: isize,
    data: *const u8,
    length: usize,
    cursor_start: isize,
    cursor_end: isize,
) -> i32 {
    if !valid(handle) {
        return -1;
    }
    unsafe {
        rust_text_edit(
            target,
            revision,
            kind,
            start,
            end,
            data,
            length,
            cursor_start,
            cursor_end,
        )
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn eldiron_creator_document_info_v1(handle: u64) -> *mut c_char {
    json(handle, |host| {
        serde_json::to_value(host.application.application.host_document_info())
            .map_err(|e| e.to_string())
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_document_load_v1(
    handle: u64,
    expected: *const c_char,
    data: *const u8,
    length: usize,
    origin: *const c_char,
    replace_clean: bool,
) -> *mut c_char {
    json(handle, |host| {
        let expected = unsafe { document(expected) }?;
        let path = unsafe { path(origin) }?;
        if data.is_null() || length == 0 || length > 1024 * 1024 * 1024 {
            return Err("Invalid project byte range".into());
        }
        let bytes = unsafe { std::slice::from_raw_parts(data, length) };
        host.request_redraw();
        host.update();
        let ui = &mut host.application;
        let info = ui.application.host_load_document(
            &mut ui.session,
            expected,
            bytes,
            path,
            replace_clean,
        )?;
        host.request_redraw();
        serde_json::to_value(info).map_err(|e| e.to_string())
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_document_prepare_save_v1(
    handle: u64,
    expected: *const c_char,
) -> *mut c_char {
    json(handle, |host| {
        let expected = unsafe { document(expected) }?;
        host.request_redraw();
        host.update();
        let ui = &mut host.application;
        serde_json::to_value(
            ui.application
                .host_prepare_document_save(&mut ui.session, expected)?,
        )
        .map_err(|e| e.to_string())
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_document_copy_save_v1(
    handle: u64,
    ticket: u64,
    destination: *mut u8,
    capacity: usize,
) -> usize {
    with_host(handle, |host| {
        let Some(bytes) = host
            .application
            .application
            .host_document_save_bytes(ticket)
        else {
            return 0;
        };
        if destination.is_null() {
            return if capacity == 0 { bytes.len() } else { 0 };
        }
        if capacity >= bytes.len() {
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };
        }
        bytes.len()
    })
    .unwrap_or(0)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_creator_document_finish_save_v1(
    handle: u64,
    ticket: u64,
    success: bool,
    origin: *const c_char,
) -> *mut c_char {
    json(handle, |host| {
        let path = unsafe { path(origin) }?;
        host.request_redraw();
        host.update();
        let ui = &mut host.application;
        let result =
            ui.application
                .host_complete_document_save(&mut ui.session, ticket, success, path)?;
        host.request_redraw();
        serde_json::to_value(result).map_err(|e| e.to_string())
    })
}
