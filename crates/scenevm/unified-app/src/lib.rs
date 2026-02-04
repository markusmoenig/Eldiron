use scenevm::{
    Atom, GeoId, Light, Poly2D, Poly3D, RenderMode, RenderResult, SceneVM, SceneVMApp,
    SceneVMRenderCtx, SceneVMResult,
};
use std::ffi::c_void;
use uuid::Uuid;
use vek::{Mat4, Vec3, Vec4};

fn pack_material(
    roughness: f32,
    metallic: f32,
    opacity: f32,
    emissive: f32,
    normal_x: Option<f32>,
    normal_y: Option<f32>,
) -> [u8; 4] {
    let r = (roughness.clamp(0.0, 1.0) * 15.0).round() as u8;
    let m = (metallic.clamp(0.0, 1.0) * 15.0).round() as u8;
    let o = (opacity.clamp(0.0, 1.0) * 15.0).round() as u8;
    let e = (emissive.clamp(0.0, 1.0) * 15.0).round() as u8;

    let mat_lo = r | (m << 4);
    let mat_hi = o | (e << 4);

    let nx = normal_x.unwrap_or(0.0);
    let ny = normal_y.unwrap_or(0.0);
    let norm_x = ((nx.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8;
    let norm_y = ((ny.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8;

    [mat_lo, mat_hi, norm_x, norm_y]
}

pub struct TemplateApp {
    matrix: Mat4<f32>,
}

impl TemplateApp {
    pub fn new() -> Self {
        Self {
            matrix: Mat4::identity(),
        }
    }
}

impl SceneVMApp for TemplateApp {
    fn initial_window_size(&self) -> Option<(u32, u32)> {
        Some((960, 540))
    }

    fn window_title(&self) -> Option<String> {
        Some("SceneVM Unified Template".to_string())
    }

    fn init(&mut self, vm: &mut SceneVM, _size: (u32, u32)) {
        let tile_id = Uuid::new_v4();
        let overlay_tile = Uuid::new_v4();

        vm.execute(Atom::SetBackground(Vec4::zero()));
        vm.execute(Atom::AddSolidWithMaterial {
            id: tile_id,
            color: [180, 180, 200, 255],
            material: pack_material(0.1, 0.0, 1.0, 0.0, None, None),
        });
        vm.execute(Atom::AddSolid {
            id: overlay_tile,
            color: [255, 96, 96, 180],
        });
        vm.execute(Atom::BuildAtlas);

        vm.execute(Atom::AddPoly3D {
            poly: Poly3D::cube(GeoId::Unknown(0), tile_id, Vec3::zero(), 2.0),
        });
        vm.execute(Atom::AddLight {
            id: GeoId::Light(0),
            light: Light::new_pointlight(Vec3::new(0.0, 1.5, -4.0))
                .with_color(Vec3::new(1.0, 0.95, 0.9))
                .with_intensity(160.0)
                .with_radius(12.0)
                .with_end_distance(18.0),
        });

        vm.execute(Atom::SetGP3(Vec4::new(0.6, 0.6, 0.7, 0.15)));
        vm.execute(Atom::SetGP6(Vec4::new(10.0, 50.0, 2.0, 16.0)));
        vm.execute(Atom::SetRenderMode(RenderMode::Compute3D));

        let overlay_index = vm.add_vm_layer();
        vm.set_active_vm(overlay_index);
        vm.execute(Atom::AddPoly {
            poly: Poly2D::poly(
                GeoId::Unknown(0),
                overlay_tile,
                vec![[40.0, 40.0], [260.0, 40.0], [260.0, 180.0], [40.0, 180.0]],
                vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                vec![(0, 1, 2), (0, 2, 3)],
            ),
        });
        vm.set_active_vm(0);
    }

    fn target_fps(&self) -> Option<f32> {
        Some(30.0)
    }

    fn needs_update(&mut self, _vm: &SceneVM) -> bool {
        true
    }

    fn update(&mut self, vm: &mut SceneVM) {
        let rot = Mat4::<f32>::rotation_y(0.02) * Mat4::<f32>::rotation_x(0.01);
        self.matrix = rot * self.matrix;
        vm.execute(Atom::SetTransform3D(self.matrix));
    }

    fn render(&mut self, vm: &mut SceneVM, ctx: &mut dyn SceneVMRenderCtx) {
        let _ = ctx.present(vm);
    }

    fn mouse_down(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        println!("mouse_down ({:.1}, {:.1})", x, y);
    }

    fn mouse_up(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        println!("mouse_up ({:.1}, {:.1})", x, y);
    }

    fn mouse_move(&mut self, _vm: &mut SceneVM, x: f32, y: f32) {
        println!("mouse_move ({:.1}, {:.1})", x, y);
    }

    fn scroll(&mut self, _vm: &mut SceneVM, dx: f32, dy: f32) {
        println!("scroll dx {:.2}, dy {:.2}", dx, dy);
    }

    fn pinch(&mut self, _vm: &mut SceneVM, scale: f32, center: (f32, f32)) {
        println!(
            "pinch scale {:.3} at ({:.1}, {:.1})",
            scale, center.0, center.1
        );
    }
}

// ---------- FFI runner for CAMetalLayer (macOS/iOS) ----------
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
pub struct SceneVMAppRunner {
    app: TemplateApp,
    vm: SceneVM,
    ctx: FfiRenderCtx,
    scale: f32,
}

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
) -> *mut SceneVMAppRunner {
    if layer_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let mut vm = SceneVM::new_with_metal_layer(layer_ptr, width, height);
    let mut app = TemplateApp::new();
    let logical_w = ((width as f32) / scale.max(0.0001)).round().max(1.0) as u32;
    let logical_h = ((height as f32) / scale.max(0.0001)).round().max(1.0) as u32;
    let ctx = FfiRenderCtx::new((logical_w, logical_h));

    // Set initial theme before init() so widgets are created with correct theme
    #[cfg(feature = "ui")]
    {
        let is_dark = app.is_dark_mode();
        app.set_theme(&mut vm, is_dark, ctx.size);
    }

    app.init(&mut vm, ctx.size);
    Box::into_raw(Box::new(SceneVMAppRunner {
        app,
        vm,
        ctx,
        scale,
    }))
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_destroy(ptr: *mut SceneVMAppRunner) {
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
pub unsafe extern "C" fn unified_app_runner_resize(
    ptr: *mut SceneVMAppRunner,
    width: u32,
    height: u32,
    scale: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.scale = scale;
        r.vm.resize_window_surface(width, height);
        let logical_w = ((width as f32) / scale.max(0.0001)).round().max(1.0) as u32;
        let logical_h = ((height as f32) / scale.max(0.0001)).round().max(1.0) as u32;
        r.app.resize(&mut r.vm, (logical_w, logical_h));
        r.ctx.size = (logical_w, logical_h);
        r.ctx.presented = false; // force a new present on next render
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_render(ptr: *mut SceneVMAppRunner) -> i32 {
    if let Some(r) = unsafe { ptr.as_mut() } {
        // Always render if we haven't presented since last resize/loop iteration.
        let should_render = r.app.needs_update(&r.vm) || !r.ctx.presented;
        if !should_render {
            // Re-use last result without doing any GPU work.
            return match r.ctx.last_result {
                RenderResult::Presented => 0,
                RenderResult::InitPending => 1,
                RenderResult::ReadbackPending => 2,
            };
        }
        r.ctx.begin_frame();
        r.app.update(&mut r.vm);
        let _ = r.app.render(&mut r.vm, &mut r.ctx);
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
pub unsafe extern "C" fn unified_app_runner_mouse_down(ptr: *mut SceneVMAppRunner, x: f32, y: f32) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.mouse_down(&mut r.vm, x, y);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_mouse_up(ptr: *mut SceneVMAppRunner, x: f32, y: f32) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.mouse_up(&mut r.vm, x, y);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_mouse_move(ptr: *mut SceneVMAppRunner, x: f32, y: f32) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.mouse_move(&mut r.vm, x, y);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_scroll(ptr: *mut SceneVMAppRunner, dx: f32, dy: f32) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.scroll(&mut r.vm, dx, dy);
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_pinch(
    ptr: *mut SceneVMAppRunner,
    scale: f32,
    center_x: f32,
    center_y: f32,
) {
    if let Some(r) = unsafe { ptr.as_mut() } {
        r.app.pinch(&mut r.vm, scale, (center_x, center_y));
    }
}

// ---------- Project Management FFI ----------

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_save_project(
    ptr: *mut SceneVMAppRunner,
    out_json: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    #[cfg(feature = "ui")]
    {
        if let Some(r) = unsafe { ptr.as_mut() } {
            if let Some(json) = r.app.save_to_json(&mut r.vm) {
                let json_bytes = json.into_bytes();
                let len = json_bytes.len();
                let boxed = json_bytes.into_boxed_slice();
                let ptr = Box::into_raw(boxed) as *const u8;

                if !out_json.is_null() {
                    unsafe {
                        *out_json = ptr;
                    }
                }
                if !out_len.is_null() {
                    unsafe {
                        *out_len = len;
                    }
                }
                return 0; // Success
            }
            return -2; // save_to_json returned None
        }
        return -1; // Invalid pointer
    }
    #[cfg(not(feature = "ui"))]
    {
        let _ = (ptr, out_json, out_len);
        -4 // Feature not enabled
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_load_project(
    ptr: *mut SceneVMAppRunner,
    json_data: *const u8,
    json_len: usize,
) -> i32 {
    #[cfg(feature = "ui")]
    {
        if json_data.is_null() || json_len == 0 {
            return -2; // Invalid input
        }

        if let Some(r) = unsafe { ptr.as_mut() } {
            let json_slice = unsafe { std::slice::from_raw_parts(json_data, json_len) };
            if let Ok(json_str) = std::str::from_utf8(json_slice) {
                if r.app.load_from_json(&mut r.vm, json_str) {
                    return 0; // Success
                }
                return -3; // load_from_json returned false
            }
            return -2; // Invalid UTF-8
        }
        return -1; // Invalid pointer
    }
    #[cfg(not(feature = "ui"))]
    {
        let _ = (ptr, json_data, json_len);
        -5 // Feature not enabled
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_json(json_ptr: *const u8, json_len: usize) {
    if !json_ptr.is_null() && json_len > 0 {
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(
                json_ptr as *mut u8,
                json_len,
            ));
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_has_unsaved_changes(ptr: *mut SceneVMAppRunner) -> i32 {
    #[cfg(feature = "ui")]
    {
        if let Some(r) = unsafe { ptr.as_ref() } {
            return if r.app.has_unsaved_changes() { 1 } else { 0 };
        }
        return -1; // Invalid pointer
    }
    #[cfg(not(feature = "ui"))]
    {
        let _ = ptr;
        0 // No unsaved changes if feature disabled
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_export_data(
    ptr: *mut SceneVMAppRunner,
    format: *const u8,
    format_len: usize,
    out_data: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    #[cfg(feature = "ui")]
    {
        if format.is_null() || format_len == 0 {
            return -2; // Invalid format
        }

        if let Some(r) = unsafe { ptr.as_mut() } {
            let format_slice = unsafe { std::slice::from_raw_parts(format, format_len) };
            if let Ok(_format_str) = std::str::from_utf8(format_slice) {
                // For now, just export as JSON regardless of format
                // Apps can override this to support multiple formats
                if let Some(json) = r.app.save_to_json(&mut r.vm) {
                    let data_bytes = json.into_bytes();
                    let len = data_bytes.len();
                    let boxed = data_bytes.into_boxed_slice();
                    let ptr = Box::into_raw(boxed) as *const u8;

                    if !out_data.is_null() {
                        unsafe {
                            *out_data = ptr;
                        }
                    }
                    if !out_len.is_null() {
                        unsafe {
                            *out_len = len;
                        }
                    }
                    return 0; // Success
                }
                return -3; // Export failed
            }
            return -2; // Invalid UTF-8 in format
        }
        return -1; // Invalid pointer
    }
    #[cfg(not(feature = "ui"))]
    {
        let _ = (ptr, format, format_len, out_data, out_len);
        -4 // Feature not enabled
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_import_data(
    ptr: *mut SceneVMAppRunner,
    data: *const u8,
    data_len: usize,
    _file_type: *const u8,
    _file_type_len: usize,
) -> i32 {
    #[cfg(feature = "ui")]
    {
        if data.is_null() || data_len == 0 {
            return -2; // Invalid data
        }

        if let Some(r) = unsafe { ptr.as_mut() } {
            let data_slice = unsafe { std::slice::from_raw_parts(data, data_len) };

            // Try to import as JSON (apps can override for other formats)
            if let Ok(json_str) = std::str::from_utf8(data_slice) {
                if r.app.load_from_json(&mut r.vm, json_str) {
                    return 0; // Success
                }
                return -3; // Import failed
            }
            return -2; // Invalid UTF-8
        }
        return -1; // Invalid pointer
    }
    #[cfg(not(feature = "ui"))]
    {
        let _ = (ptr, data, data_len, _file_type, _file_type_len);
        -4 // Feature not enabled
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_free_data(data_ptr: *const u8, data_len: usize) {
    if !data_ptr.is_null() && data_len > 0 {
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(
                data_ptr as *mut u8,
                data_len,
            ));
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    any(target_os = "macos", target_os = "ios")
))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unified_app_runner_set_theme(
    ptr: *mut SceneVMAppRunner,
    is_dark: i32,
    width: u32,
    height: u32,
) {
    #[cfg(feature = "ui")]
    {
        if let Some(r) = unsafe { ptr.as_mut() } {
            r.app.set_theme(&mut r.vm, is_dark != 0, (width, height));
        }
    }
    #[cfg(not(feature = "ui"))]
    {
        let _ = (ptr, is_dark, width, height);
    }
}
