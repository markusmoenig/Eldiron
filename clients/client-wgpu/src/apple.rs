//! ABI-compatible Xcode player facade over the common application/compositor.
//!
//! All callbacks are serialized on the creation thread. The host retains each
//! CAMetalLayer until Rust has destroyed/replaced its surface, and never acquires
//! or presents drawables itself. Rust owns the WGPU surface/device/composition.
//! Input coordinates are logical points; resize dimensions are physical pixels.
use crate::native_player::foreign::ForeignPlayerHost;
use renderkit::SurfaceState;
use std::{
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    thread::{self, ThreadId},
    time::Instant,
};
use theframework::platform::*;

#[repr(C)]
pub struct EldironPlayerRunner {
    core: ForeignPlayerHost,
    layer: *mut c_void,
    thread: ThreadId,
    last_update: Instant,
    error: String,
}
unsafe fn presenter(layer: *mut c_void, size: (u32, u32)) -> Result<GpuPresenter<'static>, String> {
    let surface = pollster::block_on(unsafe { SurfaceState::from_metal_layer(layer, size) })
        .map_err(|e| e.to_string())?;
    GpuPresenter::new(surface).map_err(|e| e.to_string())
}
fn call(
    ptr: *mut EldironPlayerRunner,
    f: impl FnOnce(&mut EldironPlayerRunner) -> Result<i32, String>,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let r = unsafe { ptr.as_mut() }.ok_or("null player runner")?;
        if r.thread != thread::current().id() {
            return Err("Player callbacks must run on their creation thread".into());
        }
        f(r)
    }));
    match result {
        Ok(Ok(code)) => code,
        result => {
            let message = match result {
                Ok(Err(e)) => e,
                Err(_) => "Panic in native player callback".into(),
                _ => unreachable!(),
            };
            if let Some(r) = unsafe { ptr.as_mut() }.filter(|r| r.thread == thread::current().id())
            {
                if r.error != message {
                    eprintln!("[EldironPlayer] {message}");
                }
                r.error = message;
            }
            -1
        }
    }
}
fn event(ptr: *mut EldironPlayerRunner, event: InputEvent) -> i32 {
    call(ptr, |r| Ok(i32::from(r.core.input(&event).consumed)))
}
unsafe fn bytes<'a>(data: *const u8, len: usize) -> Result<&'a [u8], String> {
    if len > 1024 * 1024 * 1024 {
        return Err("Native input exceeds the supported size".into());
    }
    if len == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err("null native input".into());
    }
    Ok(unsafe { std::slice::from_raw_parts(data, len) })
}
unsafe fn text(data: *const u8, len: usize) -> Result<String, String> {
    String::from_utf8(unsafe { bytes(data, len) }?.to_vec()).map_err(|e| e.to_string())
}
unsafe fn clear_output(out: *mut *const u8, len: *mut usize) {
    if !out.is_null() {
        unsafe {
            *out = std::ptr::null();
        }
    }
    if !len.is_null() {
        unsafe {
            *len = 0;
        }
    }
}
unsafe fn output(data: &[u8], out: *mut *const u8, len: *mut usize) -> Result<i32, String> {
    if !out.is_null() {
        unsafe {
            *out = std::ptr::null();
        }
    }
    if !len.is_null() {
        unsafe {
            *len = 0;
        }
    }
    if out.is_null() || len.is_null() {
        return Err("null native output destination".into());
    }
    if !data.is_empty() {
        let owned = data.to_vec().into_boxed_slice();
        unsafe {
            *len = owned.len();
            *out = Box::into_raw(owned) as *const u8;
        }
    }
    Ok(0)
}
unsafe fn free(data: *const u8, len: usize) {
    if !data.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                data.cast_mut(),
                len,
            )));
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_create(
    layer: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
) -> *mut EldironPlayerRunner {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<_, String> {
        let core = ForeignPlayerHost::new(
            unsafe { presenter(layer, (width, height)) }?,
            (width, height),
            scale,
        );
        let mut runner = EldironPlayerRunner {
            core,
            layer,
            thread: thread::current().id(),
            last_update: Instant::now(),
            error: String::new(),
        };
        // Document hosts may create the layer before supplying project bytes.
        // Keep the conventional standalone ./game.eldiron startup if available.
        if let Ok(source) = std::fs::read("game.eldiron") {
            runner
                .core
                .load(&source, false)
                .map_err(|e| e.to_string())?;
        }
        runner.last_update = Instant::now();
        Ok(Box::into_raw(Box::new(runner)))
    }));
    match result {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            eprintln!("[EldironPlayer] {e}");
            std::ptr::null_mut()
        }
        Err(_) => {
            eprintln!("[EldironPlayer] Panic creating native runner");
            std::ptr::null_mut()
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_destroy(ptr: *mut EldironPlayerRunner) {
    if ptr.is_null() {
        return;
    }
    let _ = call(ptr, |r| {
        r.core.suspend();
        Ok(0)
    });
    // The caller's serialized creation-thread/layer lifetime contract applies
    // through destruction; do not move Metal teardown to a background thread.
    if unsafe { (*ptr).thread } == thread::current().id() {
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
            drop(Box::from_raw(ptr));
        }));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_resize(
    ptr: *mut EldironPlayerRunner,
    width: u32,
    height: u32,
    scale: f32,
) {
    call(ptr, |r| {
        r.core.resize((width, height), scale);
        Ok(0)
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_render(ptr: *mut EldironPlayerRunner) -> i32 {
    call(ptr, |r| {
        let now = Instant::now();
        r.core.update(now.saturating_duration_since(r.last_update));
        r.last_update = now;
        match r.core.present().map_err(|e| e.to_string())? {
            Some(PresentResult::SurfaceLost) => {
                let next = unsafe { presenter(r.layer, r.core.host.physical_size) }?;
                r.core.replace_presenter(next).map_err(|e| e.to_string())?;
                Ok(1)
            }
            Some(PresentResult::Deferred) => Ok(1),
            Some(PresentResult::Presented) | None => Ok(0),
        }
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_down(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    event(
        ptr,
        InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: true,
            position: [x, y],
        },
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_up(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    event(
        ptr,
        InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position: [x, y],
        },
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_move(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    event(ptr, InputEvent::PointerMoved { position: [x, y] });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_motion(
    ptr: *mut EldironPlayerRunner,
    dx: f32,
    dy: f32,
) {
    event(ptr, InputEvent::RelativeMotion { delta: [dx, dy] });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_leave(ptr: *mut EldironPlayerRunner) {
    event(ptr, InputEvent::PointerLeft);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_scroll(
    ptr: *mut EldironPlayerRunner,
    dx: f32,
    dy: f32,
) {
    event(
        ptr,
        InputEvent::Scroll {
            delta: [dx, dy],
            precise: true,
        },
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_pinch(
    ptr: *mut EldironPlayerRunner,
    scale: f32,
    _x: f32,
    _y: f32,
) {
    event(ptr, InputEvent::Pinch(scale));
}

// Keep both established symbol families and the static library identity.
macro_rules! alias {
    ($name:ident,$target:ident,($($arg:ident:$ty:ty),*)$(->$ret:ty)?) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg:$ty),*) $(->$ret)? { unsafe { $target($($arg),*) } }
    };
}
alias!(unified_app_runner_create,eldiron_player_runner_create,(layer:*mut c_void,width:u32,height:u32,scale:f32)->*mut EldironPlayerRunner);
alias!(unified_app_runner_destroy,eldiron_player_runner_destroy,(ptr:*mut EldironPlayerRunner));
alias!(unified_app_runner_resize,eldiron_player_runner_resize,(ptr:*mut EldironPlayerRunner,width:u32,height:u32,scale:f32));
alias!(unified_app_runner_render,eldiron_player_runner_render,(ptr:*mut EldironPlayerRunner)->i32);
alias!(unified_app_runner_mouse_down,eldiron_player_runner_mouse_down,(ptr:*mut EldironPlayerRunner,x:f32,y:f32));
alias!(unified_app_runner_mouse_up,eldiron_player_runner_mouse_up,(ptr:*mut EldironPlayerRunner,x:f32,y:f32));
alias!(unified_app_runner_mouse_move,eldiron_player_runner_mouse_move,(ptr:*mut EldironPlayerRunner,x:f32,y:f32));
alias!(unified_app_runner_mouse_motion,eldiron_player_runner_mouse_motion,(ptr:*mut EldironPlayerRunner,dx:f32,dy:f32));
alias!(unified_app_runner_mouse_leave,eldiron_player_runner_mouse_leave,(ptr:*mut EldironPlayerRunner));
alias!(unified_app_runner_scroll,eldiron_player_runner_scroll,(ptr:*mut EldironPlayerRunner,dx:f32,dy:f32));
alias!(unified_app_runner_pinch,eldiron_player_runner_pinch,(ptr:*mut EldironPlayerRunner,scale:f32,x:f32,y:f32));

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_load_project(
    ptr: *mut EldironPlayerRunner,
    data: *const u8,
    len: usize,
) -> i32 {
    call(ptr, |r| {
        r.core
            .load(unsafe { bytes(data, len) }?, false)
            .map_err(|e| e.to_string())?;
        r.last_update = Instant::now();
        r.error.clear();
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_save_project(
    ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe {
        clear_output(out, len);
    }
    call(ptr, |r| unsafe { output(r.core.source(), out, len) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_json(data: *const u8, len: usize) {
    unsafe { free(data, len) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_data(data: *const u8, len: usize) {
    unsafe { free(data, len) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_string(data: *const u8, len: usize) {
    unsafe { free(data, len) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_last_error(
    ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe {
        clear_output(out, len);
    }
    call(ptr, |r| unsafe { output(r.error.as_bytes(), out, len) })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_suspend(ptr: *mut EldironPlayerRunner) {
    call(ptr, |r| {
        r.core.suspend();
        Ok(0)
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_resume(ptr: *mut EldironPlayerRunner) {
    call(ptr, |r| {
        r.core.resume();
        r.last_update = Instant::now();
        Ok(0)
    });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_replace_layer(
    ptr: *mut EldironPlayerRunner,
    layer: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
) -> i32 {
    call(ptr, |r| {
        let next = unsafe { presenter(layer, (width, height)) }?;
        r.core.host.resize((width, height), scale);
        r.core.replace_presenter(next).map_err(|e| e.to_string())?;
        r.layer = layer;
        r.last_update = Instant::now();
        Ok(0)
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_focus(ptr: *mut EldironPlayerRunner, focused: i32) {
    event(ptr, InputEvent::Focus(focused != 0));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_modifiers(ptr: *mut EldironPlayerRunner, bits: u32) {
    event(
        ptr,
        InputEvent::Modifiers(Modifiers {
            shift: bits & 1 != 0,
            control: bits & 2 != 0,
            alt: bits & 4 != 0,
            super_key: bits & 8 != 0,
        }),
    );
}
fn physical(code: u32) -> PhysicalKey {
    PhysicalKey::from_native_code(code)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_key(
    ptr: *mut EldironPlayerRunner,
    code: u32,
    pressed: i32,
    repeat: i32,
) -> i32 {
    event(
        ptr,
        InputEvent::Key {
            physical: physical(code),
            logical: LogicalKey::Other,
            pressed: pressed != 0,
            repeat: repeat != 0,
        },
    )
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_text(
    ptr: *mut EldironPlayerRunner,
    target: u64,
    data: *const u8,
    len: usize,
) -> i32 {
    call(ptr, |r| {
        let target = TextInputTarget::from_raw(target).ok_or("invalid text target")?;
        let text = unsafe { text(data, len) }?;
        Ok(i32::from(
            r.core.input(&InputEvent::TextFor { target, text }).consumed,
        ))
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_composition(
    ptr: *mut EldironPlayerRunner,
    target: u64,
    data: *const u8,
    len: usize,
    start: isize,
    end: isize,
) -> i32 {
    call(ptr, |r| {
        let target = TextInputTarget::from_raw(target).ok_or("invalid composition target")?;
        let text = unsafe { text(data, len) }?;
        let cursor = if start < 0 || end < 0 {
            None
        } else {
            let (start, end) = (start as usize, end as usize);
            if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                return Err("Composition cursor must use UTF-8 byte boundaries".into());
            }
            Some((start, end))
        };
        Ok(i32::from(
            r.core
                .input(&InputEvent::CompositionFor {
                    target,
                    text,
                    cursor,
                })
                .consumed,
        ))
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_clipboard(
    ptr: *mut EldironPlayerRunner,
    request: u64,
    write: i32,
    success: i32,
    data: *const u8,
    len: usize,
) -> i32 {
    call(ptr, |r| {
        let result = if write != 0 {
            ClipboardResult::Written(success != 0)
        } else {
            ClipboardResult::Text(if success != 0 {
                Some(unsafe { text(data, len) }?)
            } else {
                None
            })
        };
        Ok(i32::from(
            r.core
                .input(&InputEvent::Clipboard { request, result })
                .consumed,
        ))
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_pointer_button(
    ptr: *mut EldironPlayerRunner,
    button: u16,
    pressed: i32,
    x: f32,
    y: f32,
) -> i32 {
    event(
        ptr,
        InputEvent::PointerButton {
            button: match button {
                0 => PointerButton::Primary,
                1 => PointerButton::Secondary,
                2 => PointerButton::Middle,
                v => PointerButton::Other(v),
            },
            pressed: pressed != 0,
            position: [x, y],
        },
    )
}
/// Source/selection metadata is an explicit native query, never a frame capture.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_text_snapshot(
    ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe {
        clear_output(out, len);
    }
    call(ptr, |r| {
        let snapshot = native_text_wire::snapshot(r.core.native_text_snapshot().as_ref())?;
        unsafe {
            output(
                &serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?,
                out,
                len,
            )
        }
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_text_query(
    ptr: *mut EldironPlayerRunner,
    target: u64,
    revision: u64,
    kind: u32,
    start: isize,
    end: isize,
    x: f32,
    y: f32,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe {
        clear_output(out, len);
    }
    call(ptr, |r| {
        let answer = if let Some(source) = r
            .core
            .native_text_snapshot()
            .filter(|s| s.target.raw() == target && s.revision == revision)
        {
            let request = native_text_wire::query(&source, kind, start, end, x, y)?;
            native_text_wire::answer(&source, r.core.native_text_query(&request))
        } else {
            serde_json::Value::Null
        };
        unsafe {
            output(
                &serde_json::to_vec(&answer).map_err(|e| e.to_string())?,
                out,
                len,
            )
        }
    })
}

/// Native ranges/cursors are UTF-16. Validate both editor identity and displayed
/// source revision before translating them to shared-editor byte operations.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_text_edit(
    ptr: *mut EldironPlayerRunner,
    target: u64,
    revision: u64,
    kind: u32,
    start: isize,
    end: isize,
    data: *const u8,
    len: usize,
    cursor_start: isize,
    cursor_end: isize,
) -> i32 {
    call(ptr, |r| {
        let Some(source) = r
            .core
            .native_text_snapshot()
            .filter(|s| s.target.raw() == target && s.revision == revision)
        else {
            return Ok(0);
        };
        let action = native_text_wire::edit(
            &source,
            kind,
            start,
            end,
            unsafe { text(data, len) }?,
            cursor_start,
            cursor_end,
        )?;
        Ok(i32::from(
            r.core
                .input(&InputEvent::NativeText {
                    target: source.target,
                    revision,
                    action,
                })
                .consumed,
        ))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_host_state(
    ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe {
        clear_output(out, len);
    }
    call(ptr, |r| {
        let source = r.core.native_text_snapshot();
        let selection = source.as_ref().and_then(|s| {
            Some([
                s.utf16_from_byte(s.selection.0)?,
                s.utf16_from_byte(s.selection.1)?,
            ])
        });
        let input = r.core.host.text_input.map(|s|serde_json::json!({"target":s.target.raw(),
            "revision":source.as_ref().filter(|source|source.target==s.target).map(|source|source.revision),
            "selection":selection,
            "area":[s.area.x,s.area.y,s.area.width,s.area.height]}));
        let app = r.core.application.as_ref();
        let state = serde_json::json!({
            "title": app.map(GpuApplication::title).unwrap_or_else(|| "Eldiron Player".into()),
            "cursor": format!("{:?}", r.core.host.cursor),
            "cursor_visible": r.core.host.cursor_visible,
            "text_input": input,
            "presentations": r.core.presentations(),
            "session_id": app.map(|app| app.player.runtime.view().session_id.to_string()),
            "tick_sequence": app.map_or(0, |app| app.player.runtime.tick_sequence()),
            "frame_count": app.map_or(0, |app| app.frame_count()),
            "readback_bytes": r.core.presenter.context().transfers().readback_bytes,
        });
        unsafe {
            output(
                &serde_json::to_vec(&state).map_err(|e| e.to_string())?,
                out,
                len,
            )
        }
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_take_requests(
    ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe {
        clear_output(out, len);
    }
    call(ptr, |r| {
        let requests = r
            .core
            .host
            .requests
            .iter()
            .map(|r| match r {
                HostRequest::Exit => serde_json::json!({"kind":"exit"}),
                HostRequest::Fail(message) => serde_json::json!({"kind":"error","message":message}),
                HostRequest::Title(title) => serde_json::json!({"kind":"title","title":title}),
                HostRequest::ClipboardRead { request } => {
                    serde_json::json!({"kind":"clipboard_read","request":request})
                }
                HostRequest::ClipboardWrite { text, request } => {
                    serde_json::json!({"kind":"clipboard_write","text":text,"request":request})
                }
            })
            .collect::<Vec<_>>();
        let result = unsafe {
            output(
                &serde_json::to_vec(&requests).map_err(|e| e.to_string())?,
                out,
                len,
            )
        }?;
        r.core.host.requests.clear();
        Ok(result)
    })
}
// Player documents preserve their authored source, rather than authoring runtime
// changes. Keep the template's editor-only ABI requests explicitly unsupported.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_has_unsaved_changes(
    _ptr: *mut EldironPlayerRunner,
) -> i32 {
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_set_theme(
    _ptr: *mut EldironPlayerRunner,
    _dark: i32,
    _width: u32,
    _height: u32,
) {
}
macro_rules! unsupported_action {($($name:ident),*)=>{$(#[unsafe(no_mangle)] pub unsafe extern "C" fn $name(_ptr:*mut EldironPlayerRunner)->i32 {0})*};}
unsupported_action!(
    unified_app_runner_undo,
    unified_app_runner_redo,
    unified_app_runner_can_undo,
    unified_app_runner_can_redo
);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_undo_description(
    _ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    let _ = unsafe { output(&[], out, len) };
    -1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_redo_description(
    ptr: *mut EldironPlayerRunner,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    unsafe { unified_app_runner_undo_description(ptr, out, len) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_export_data(
    _ptr: *mut EldironPlayerRunner,
    _format: *const u8,
    _format_len: usize,
    out: *mut *const u8,
    len: *mut usize,
) -> i32 {
    let _ = unsafe { output(&[], out, len) };
    -1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_import_data(
    _ptr: *mut EldironPlayerRunner,
    _data: *const u8,
    _len: usize,
    _file_type: *const u8,
    _file_type_len: usize,
) -> i32 {
    -1
}

#[cfg(all(test, target_os = "macos"))]
#[path = "apple_tests.rs"]
mod tests;
