pub mod app;
pub mod misc;

pub use app::EldironPlayerApp;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
use scenevm::{Atom, prelude::Mat3};
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
use scenevm::{RenderResult, SceneVM, SceneVMApp, SceneVMRenderCtx, SceneVMResult};

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
use std::ffi::c_void;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
struct FfiRenderCtx {
    size: (u32, u32),
    last_result: RenderResult,
    presented: bool,
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
impl FfiRenderCtx {
    fn new(size: (u32, u32)) -> Self {
        Self {
            size,
            last_result: RenderResult::InitPending,
            presented: false,
        }
    }

    fn begin_frame(&mut self) {
        self.presented = false;
    }

    fn ensure_presented(&mut self, vm: &mut SceneVM) -> SceneVMResult<RenderResult> {
        if !self.presented {
            let res = self.present(vm)?;
            self.last_result = res;
        }
        Ok(self.last_result)
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
impl SceneVMRenderCtx for FfiRenderCtx {
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn present(&mut self, vm: &mut SceneVM) -> SceneVMResult<RenderResult> {
        let res = vm.render_to_window();
        if let Ok(r) = res {
            self.last_result = r;
        }
        self.presented = true;
        res
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[repr(C)]
pub struct EldironPlayerRunner {
    app: EldironPlayerApp,
    vm: SceneVM,
    ctx: FfiRenderCtx,
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_create(
    layer_ptr: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
) -> *mut EldironPlayerRunner {
    if layer_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let mut vm = SceneVM::new_with_metal_layer(layer_ptr, width, height);
    let mut app = EldironPlayerApp::default();
    app.set_native_mode(false);
    app.set_scale(scale);
    app.initialize();
    let s = scale.max(0.0001);
    vm.execute(Atom::SetTransform2D(Mat3::<f32>::new(
        s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 1.0,
    )));

    let safe_scale = scale.max(0.0001);
    let logical_w = ((width as f32) / safe_scale).round().max(1.0) as u32;
    let logical_h = ((height as f32) / safe_scale).round().max(1.0) as u32;
    let ctx = FfiRenderCtx::new((logical_w, logical_h));

    app.init(&mut vm, (logical_w, logical_h));

    Box::into_raw(Box::new(EldironPlayerRunner { app, vm, ctx }))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_destroy(ptr: *mut EldironPlayerRunner) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(ptr));
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_resize(
    ptr: *mut EldironPlayerRunner,
    width: u32,
    height: u32,
    scale: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.vm.resize_window_surface(width, height);
        let safe_scale = scale.max(0.0001);
        let logical_w = ((width as f32) / safe_scale).round().max(1.0) as u32;
        let logical_h = ((height as f32) / safe_scale).round().max(1.0) as u32;
        r.ctx.size = (logical_w, logical_h);
        r.app.set_scale(scale);
        r.vm.execute(Atom::SetTransform2D(Mat3::<f32>::new(
            safe_scale, 0.0, 0.0, 0.0, safe_scale, 0.0, 0.0, 0.0, 1.0,
        )));
        r.app.resize(&mut r.vm, (logical_w, logical_h));
        r.ctx.presented = false;
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_render(ptr: *mut EldironPlayerRunner) -> i32 {
    if let Some(r) = unsafe { ptr.as_mut() } {
        let should_render = r.app.needs_update(&r.vm) || !r.ctx.presented;
        if !should_render {
            return match r.ctx.last_result {
                RenderResult::Presented => 0,
                RenderResult::InitPending => 1,
                RenderResult::ReadbackPending => 2,
            };
        }
        r.ctx.begin_frame();
        r.app.update(&mut r.vm);
        r.app.render(&mut r.vm, &mut r.ctx);
        match r.ctx.ensure_presented(&mut r.vm) {
            Ok(RenderResult::Presented) => 0,
            Ok(RenderResult::InitPending) => 1,
            Ok(RenderResult::ReadbackPending) => 2,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_down(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.mouse_down(&mut r.vm, x, y);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_up(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.mouse_up(&mut r.vm, x, y);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_mouse_move(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.mouse_move(&mut r.vm, x, y);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_scroll(
    ptr: *mut EldironPlayerRunner,
    dx: f32,
    dy: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.scroll(&mut r.vm, dx, dy);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn eldiron_player_runner_pinch(
    ptr: *mut EldironPlayerRunner,
    scale: f32,
    center_x: f32,
    center_y: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.pinch(&mut r.vm, scale, (center_x, center_y));
    }
}

// Compatibility aliases for the SceneVM Swift template.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_create(
    layer_ptr: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
) -> *mut EldironPlayerRunner {
    unsafe { eldiron_player_runner_create(layer_ptr, width, height, scale) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_destroy(ptr: *mut EldironPlayerRunner) {
    unsafe { eldiron_player_runner_destroy(ptr) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_resize(
    ptr: *mut EldironPlayerRunner,
    width: u32,
    height: u32,
    scale: f32,
) {
    unsafe { eldiron_player_runner_resize(ptr, width, height, scale) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_render(ptr: *mut EldironPlayerRunner) -> i32 {
    unsafe { eldiron_player_runner_render(ptr) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_mouse_down(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    unsafe { eldiron_player_runner_mouse_down(ptr, x, y) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_mouse_up(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    unsafe { eldiron_player_runner_mouse_up(ptr, x, y) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_mouse_move(
    ptr: *mut EldironPlayerRunner,
    x: f32,
    y: f32,
) {
    unsafe { eldiron_player_runner_mouse_move(ptr, x, y) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_scroll(
    ptr: *mut EldironPlayerRunner,
    dx: f32,
    dy: f32,
) {
    unsafe { eldiron_player_runner_scroll(ptr, dx, dy) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_pinch(
    ptr: *mut EldironPlayerRunner,
    scale: f32,
    center_x: f32,
    center_y: f32,
) {
    unsafe { eldiron_player_runner_pinch(ptr, scale, center_x, center_y) }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_save_project(
    _ptr: *mut EldironPlayerRunner,
    out_json: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if !out_json.is_null() {
        unsafe {
            *out_json = std::ptr::null();
        }
    }
    if !out_len.is_null() {
        unsafe {
            *out_len = 0;
        }
    }
    -1
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_load_project(
    _ptr: *mut EldironPlayerRunner,
    _json_data: *const u8,
    _json_len: usize,
) -> i32 {
    -1
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_json(_json_ptr: *const u8, _json_len: usize) {}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_has_unsaved_changes(
    _ptr: *mut EldironPlayerRunner,
) -> i32 {
    0
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_export_data(
    _ptr: *mut EldironPlayerRunner,
    _format: *const u8,
    _format_len: usize,
    out_data: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if !out_data.is_null() {
        unsafe {
            *out_data = std::ptr::null();
        }
    }
    if !out_len.is_null() {
        unsafe {
            *out_len = 0;
        }
    }
    -1
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_import_data(
    _ptr: *mut EldironPlayerRunner,
    _data: *const u8,
    _data_len: usize,
    _file_type: *const u8,
    _file_type_len: usize,
) -> i32 {
    -1
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_data(_data_ptr: *const u8, _data_len: usize) {}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_set_theme(
    _ptr: *mut EldironPlayerRunner,
    _is_dark: i32,
    _width: u32,
    _height: u32,
) {
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_undo(_ptr: *mut EldironPlayerRunner) -> i32 {
    0
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_redo(_ptr: *mut EldironPlayerRunner) -> i32 {
    0
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_can_undo(_ptr: *mut EldironPlayerRunner) -> i32 {
    0
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_can_redo(_ptr: *mut EldironPlayerRunner) -> i32 {
    0
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_undo_description(
    _ptr: *mut EldironPlayerRunner,
    out_str: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if !out_str.is_null() {
        unsafe {
            *out_str = std::ptr::null();
        }
    }
    if !out_len.is_null() {
        unsafe {
            *out_len = 0;
        }
    }
    -1
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_redo_description(
    _ptr: *mut EldironPlayerRunner,
    out_str: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    if !out_str.is_null() {
        unsafe {
            *out_str = std::ptr::null();
        }
    }
    if !out_len.is_null() {
        unsafe {
            *out_len = 0;
        }
    }
    -1
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_string(_str_ptr: *const u8, _str_len: usize) {}
