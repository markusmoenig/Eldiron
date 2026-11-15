//! Main-thread Xcode facade. No CPU destination bitmap crosses this ABI.
use crate::{native_application::NativeCreatorApplication, prelude::*};
use renderkit::SurfaceState;
use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
};
use theframework::platform::*;

mod image_clipboard;
mod owner;
mod v1;

type CreatorHost = ForeignApplicationHost<UiApplicationAdapter<NativeCreatorApplication>>;
struct HostedCreator {
    app: CreatorHost,
    clipboard_png: Option<(u64, Vec<u8>)>,
}
unsafe extern "C" {
    fn pthread_main_np() -> i32;
}
thread_local! {
    static APP: RefCell<owner::Owner<HostedCreator>> = const { RefCell::new(owner::Owner::new()) };
    static SELECTED: Cell<Option<u64>> = const { Cell::new(None) };
}

fn with_app<T: Default>(f: impl FnOnce(&mut CreatorHost) -> T) -> T {
    with_hosted(|host| f(&mut host.app))
}
// Legacy callbacks are scoped only during a validated synchronous v1 forward.
// An unversioned caller otherwise addresses its own explicitly created host.
fn selected(owner: &owner::Owner<HostedCreator>) -> Option<u64> {
    SELECTED.with(Cell::get).or_else(|| owner.legacy())
}
fn with_selected<T>(handle: u64, f: impl FnOnce() -> T) -> Option<T> {
    if handle == 0
        || unsafe { pthread_main_np() } == 0
        || !APP.with(|app| {
            app.try_borrow()
                .ok()
                .is_some_and(|app| app.contains(handle))
        })
    {
        return None;
    }
    struct Restore(Option<u64>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SELECTED.with(|current| current.set(self.0));
        }
    }
    let _restore = Restore(SELECTED.with(|current| current.replace(Some(handle))));
    Some(f())
}
fn with_hosted<T: Default>(f: impl FnOnce(&mut HostedCreator) -> T) -> T {
    if unsafe { pthread_main_np() } == 0 {
        return T::default();
    }
    catch_unwind(AssertUnwindSafe(|| {
        APP.with(|app| {
            let mut app = app.try_borrow_mut().ok()?;
            let id = selected(&app)?;
            app.get_mut(id).map(f)
        })
    }))
    .ok()
    .flatten()
    .unwrap_or_default()
}
fn with_hosted_handle<T>(handle: u64, f: impl FnOnce(&mut HostedCreator) -> T) -> Option<T> {
    if handle == 0 || unsafe { pthread_main_np() } == 0 {
        return None;
    }
    catch_unwind(AssertUnwindSafe(|| {
        APP.with(|app| app.try_borrow_mut().ok()?.get_mut(handle).map(f))
    }))
    .ok()
    .flatten()
}
unsafe fn create_host(
    layer: *mut c_void,
    width: u32,
    height: u32,
    density: f32,
    legacy: bool,
) -> Option<u64> {
    if unsafe { pthread_main_np() } == 0 || layer.is_null() {
        return None;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let available = APP.with(|app| {
            app.try_borrow().ok().is_some_and(|app| {
                app.layer_available(layer as usize, None) && (!legacy || app.legacy().is_none())
            })
        });
        if !available {
            return None;
        }
        let _ = crate::i18n::select_system_locales();
        let presenter = match unsafe { presenter(layer, (width, height)) } {
            Ok(value) => value,
            Err(error) => {
                eprintln!("[EldironCreator] {error}");
                return None;
            }
        };
        let mut host = match ForeignApplicationHost::new(
            UiApplicationAdapter::new(NativeCreatorApplication::new(
                vec!["eldiron-creator".into()],
                false,
            )),
            presenter,
            density,
        ) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("[EldironCreator] {error}");
                return None;
            }
        };
        #[cfg(not(target_os = "tvos"))]
        if !legacy {
            host.application.application.host_enable_document_io();
        }
        APP.with(|app| {
            app.try_borrow_mut().ok()?.install(
                layer as usize,
                HostedCreator {
                    app: host,
                    clipboard_png: None,
                },
                legacy,
            )
        })
    }))
    .ok()
    .flatten()
}
fn destroy_host(handle: u64) -> bool {
    if with_hosted_handle(handle, |host| host.app.suspend()).is_none() {
        return false;
    }
    // Drop the surface/application after releasing the registry borrow.
    let host = APP.with(|app| app.borrow_mut().take(handle));
    host.is_some()
}

unsafe fn presenter(
    layer: *mut c_void,
    size: (u32, u32),
) -> renderkit::Result<GpuPresenter<'static>> {
    GpuPresenter::new(pollster::block_on(unsafe {
        SurfaceState::from_metal_layer(layer, size)
    })?)
}

/// The host retains layer until rust_shutdown, serializing all calls on main.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_init(
    layer: *mut c_void,
    width: u32,
    height: u32,
    density: f32,
) -> bool {
    unsafe { create_host(layer, width, height, density, true) }.is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_shutdown() {
    if unsafe { pthread_main_np() } == 0 {
        return;
    }
    if let Some(id) = APP.with(|app| app.borrow().legacy()) {
        destroy_host(id);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_resize(width: u32, height: u32, density: f32) {
    with_app(|app| app.resize((width, height), density));
}

/// Returns 0 for idle/presented, 1 for a deferred drawable, -1 for failure.
#[unsafe(no_mangle)]
pub extern "C" fn rust_render() -> i32 {
    with_app(|app| {
        app.update();
        Some(match app.present() {
            Ok(Some(PresentResult::SurfaceLost)) => -2,
            Ok(Some(PresentResult::Deferred)) => 1,
            Ok(_) => 0,
            Err(e) => {
                eprintln!("[EldironCreator] {e}");
                -1
            }
        })
    })
    .unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_replace_layer(layer: *mut c_void) -> bool {
    if unsafe { pthread_main_np() } == 0 {
        return false;
    }
    let id = APP.with(|app| {
        let app = app.try_borrow().ok()?;
        let id = selected(&app)?;
        app.layer_available(layer as usize, Some(id)).then_some(id)
    });
    let Some(id) = id else {
        return false;
    };
    let replaced = with_hosted_handle(id, |host| {
        match unsafe { presenter(layer, host.app.host.physical_size) }
            .and_then(|next| host.app.replace_presenter(next))
        {
            Ok(()) => true,
            Err(error) => {
                eprintln!("[EldironCreator] {error}");
                false
            }
        }
    })
    .unwrap_or(false);
    replaced && APP.with(|app| app.borrow_mut().replace_layer(id, layer as usize))
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_suspend() {
    with_app(|app| app.suspend());
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_resume() {
    with_app(|app| app.resume());
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_focus(focused: bool) {
    event(InputEvent::Focus(focused));
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_target_fps() -> u32 {
    60
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_has_changes() -> bool {
    with_app(|app| app.application.document_edited())
}

fn event(event: InputEvent) -> bool {
    with_app(|app| app.input(&event).consumed)
}
fn button(x: f32, y: f32, button: PointerButton, pressed: bool) -> bool {
    event(InputEvent::PointerButton {
        button,
        pressed,
        position: [x, y],
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_hover(x: f32, y: f32) -> bool {
    event(InputEvent::PointerMoved { position: [x, y] })
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_down(x: f32, y: f32) -> bool {
    button(x, y, PointerButton::Primary, true)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_right_touch_down(x: f32, y: f32) -> bool {
    button(x, y, PointerButton::Secondary, true)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_up(x: f32, y: f32) -> bool {
    button(x, y, PointerButton::Primary, false)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_right_touch_up(x: f32, y: f32) -> bool {
    button(x, y, PointerButton::Secondary, false)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_dragged(x: f32, y: f32) -> bool {
    rust_hover(x, y)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_right_touch_dragged(x: f32, y: f32) -> bool {
    rust_hover(x, y)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_mouse_leave() -> bool {
    event(InputEvent::PointerLeft)
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_mouse_motion(x: f32, y: f32) -> bool {
    event(InputEvent::RelativeMotion { delta: [x, y] })
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_wheel(x: f32, y: f32) -> bool {
    event(InputEvent::Scroll {
        delta: [x, y],
        precise: false,
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_touch_precise_scroll(x: f32, y: f32) -> bool {
    event(InputEvent::Scroll {
        delta: [x, y],
        precise: true,
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_pinch(delta: f32) -> bool {
    event(InputEvent::Pinch(delta))
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_key_modifier_changed(
    shift: bool,
    control: bool,
    alt: bool,
    super_key: bool,
) -> bool {
    event(InputEvent::Modifiers(Modifiers {
        shift,
        control,
        alt,
        super_key,
    }))
}

unsafe fn string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_dropped_file(ptr: *const c_char) {
    if let Some(path) = unsafe { string(ptr) } {
        event(InputEvent::FileDropped(path.into()));
    }
}

fn command(name: &str) {
    with_app(|app| {
        app.application
            .session
            .context
            .ui
            .send(TheEvent::StateChanged(
                TheId::named(name),
                TheWidgetState::Clicked,
            ));
        app.request_redraw();
    });
}
macro_rules! commands { ($($function:ident=>$name:literal),* $(,)?)=>{$(
    #[unsafe(no_mangle)] pub extern "C" fn $function(){command($name);}
)*}; }
commands! {
    rust_new=>"New",rust_open=>"Open",rust_close=>"Close",rust_save=>"Save",rust_save_as=>"Save As",
    rust_play=>"Play",rust_pause=>"Pause",rust_stop=>"Stop",
    rust_show_settings=>"Show Settings",rust_show_rules=>"Show Rules",rust_show_locales=>"Show Locales",
    rust_show_audio_fx=>"Show Audio FX",rust_show_authoring=>"Show Authoring",
    rust_show_project_sidebar=>"Show Project Sidebar",rust_show_actions_sidebar=>"Show Actions Sidebar",
    rust_show_debug_log=>"Show Debug Log",rust_show_console=>"Show Console",rust_show_help_sidebar=>"Show Help Sidebar",
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_set_creator_theme(index: u32) {
    command(match index {
        1 => "Theme Black",
        2 => "Theme Light",
        _ => "Theme Brown",
    });
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_get_creator_theme() -> u32 {
    with_app(|app| Some(app.application.application.host_creator_theme())).unwrap_or_else(|| {
        match crate::editor::CreatorTheme::load() {
            crate::editor::CreatorTheme::Brown => 0,
            crate::editor::CreatorTheme::Black => 1,
            crate::editor::CreatorTheme::Light => 2,
        }
    })
}

/// Native Edit menu commands use the same controller/clipboard route as keys.
#[unsafe(no_mangle)]
pub extern "C" fn rust_edit_command(code: u32) {
    // Native menus must enter the same document/widget action route directly.
    // Synthesizing transient Command modifiers loses deferred authoring actions
    // when this host has no in-canvas menubar accelerator registration.
    command(match code {
        0 => "Cut",
        1 => "Copy",
        2 => "Paste",
        3 => "Undo",
        4 => "Redo",
        _ => return,
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_undo() {
    rust_edit_command(3);
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_redo() {
    rust_edit_command(4);
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_take_requests() -> *mut c_char {
    with_hosted(|host| {
        let app = &mut host.app;
        let mut requests: Vec<_> = std::mem::take(&mut app.host.requests)
            .into_iter()
            .map(|r| match r {
                HostRequest::Exit => serde_json::json!({"kind":"exit"}),
                HostRequest::Fail(message) => serde_json::json!({"kind":"error","message":message}),
                HostRequest::Title(title) => serde_json::json!({"kind":"title","title":title}),
                HostRequest::ClipboardRead { request } => {
                    serde_json::json!({"kind":"read","request":request})
                }
                HostRequest::ClipboardWrite { text, request } => {
                    serde_json::json!({"kind":"write","text":text,"request":request})
                }
                HostRequest::ClipboardImage(ImageClipboardRequest::Read { request }) => {
                    serde_json::json!({"kind":"image_read","request":request})
                }
                HostRequest::ClipboardImage(ImageClipboardRequest::Write { request, image }) => {
                    host.clipboard_png = Some((
                        request,
                        theframework::platform::image_clipboard::encode_png(&image)
                            .unwrap_or_default(),
                    ));
                    let length = host
                        .clipboard_png
                        .as_ref()
                        .map_or(0, |(_, bytes)| bytes.len());
                    serde_json::json!({"kind":"image_write","request":request,"length":length})
                }
            })
            .collect();
        requests.extend(
            app.application
                .application
                .host_document_requests()
                .into_iter()
                .map(|(kind, document)| serde_json::json!({"kind":kind,"document":document})),
        );
        CString::new(serde_json::to_string(&requests).unwrap())
            .unwrap()
            .into_raw()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_host_state() -> *mut c_char {
    with_app(|app| {
        let source = app
            .host
            .text_input
            .and_then(|_| app.application.native_text_snapshot(&mut app.host));
        let selection = source.as_ref().and_then(|s| {
            Some([
                s.utf16_from_byte(s.selection.0)?,
                s.utf16_from_byte(s.selection.1)?,
            ])
        });
        let input = app.host.text_input.map(|s| serde_json::json!({
            "target": s.target.raw(),
            "revision": source.as_ref().filter(|source| source.target == s.target).map(|source|source.revision),
            "selection": selection,
            "area": [s.area.x,s.area.y,s.area.width,s.area.height],
        }));
        let transfers = app.presenter.context().transfers();
        let state = serde_json::json!({
            "presentations": app.presentations(),
            "updates": app.updates(),
            "physical_size": app.host.physical_size,
            "logical_size": app.host.logical_size,
            "density": app.host.density,
            "root_cpu_bytes": app.application.session.ui.canvas.buffer.pixels().len(),
            "readback_bytes": transfers.readback_bytes,
            "asset_bake_readback_bytes": transfers.asset_bake_readback_bytes,
            "asset_bake_readback_requests": transfers.asset_bake_readback_requests,
            "normal_readback_bytes": transfers.readback_bytes - transfers.asset_bake_readback_bytes,
            "view": app.application.application.host_view_diagnostics(),
            "text_input": input,
            "focused_widget": app.application.session.context.ui.focus.as_ref().map(|id| id.name.as_str()),
        });
        CString::new(state.to_string()).unwrap().into_raw()
    })
}

fn json_app(f: impl FnOnce(&mut CreatorHost) -> Result<serde_json::Value, String>) -> *mut c_char {
    match with_app(|app| Some(f(app))) {
        Some(Ok(value)) => CString::new(value.to_string()).unwrap().into_raw(),
        Some(Err(error)) => {
            eprintln!("[EldironCreator] {error}");
            std::ptr::null_mut()
        }
        None => std::ptr::null_mut(),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_text_snapshot() -> *mut c_char {
    json_app(|app| {
        native_text_wire::snapshot(app.application.native_text_snapshot(&mut app.host).as_ref())
            .map_err(str::to_owned)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn rust_text_query(
    target: u64,
    revision: u64,
    kind: u32,
    start: isize,
    end: isize,
    x: f32,
    y: f32,
) -> *mut c_char {
    json_app(|app| {
        let Some(source) = app
            .application
            .native_text_snapshot(&mut app.host)
            .filter(|s| s.target.raw() == target && s.revision == revision)
        else {
            return Ok(serde_json::Value::Null);
        };
        let request =
            native_text_wire::query(&source, kind, start, end, x, y).map_err(str::to_owned)?;
        let answer = app.application.native_text_query(&mut app.host, &request);
        Ok(native_text_wire::answer(&source, answer))
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_text_edit(
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
    if len > 1024 * 1024 * 1024 || (len > 0 && data.is_null()) {
        return -1;
    }
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return -1;
    };
    with_app(|app| {
        let Some(source) = app
            .application
            .native_text_snapshot(&mut app.host)
            .filter(|s| s.target.raw() == target && s.revision == revision)
        else {
            return Some(0);
        };
        match native_text_wire::edit(
            &source,
            kind,
            start,
            end,
            text.into(),
            cursor_start,
            cursor_end,
        ) {
            Ok(action) => Some(i32::from(
                app.input(&InputEvent::NativeText {
                    target: source.target,
                    revision,
                    action,
                })
                .consumed,
            )),
            Err(error) => {
                eprintln!("[EldironCreator] {error}");
                Some(-1)
            }
        }
    })
    .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_physical_key(
    code: u32,
    pressed: bool,
    repeated: bool,
    logical: *const c_char,
) -> bool {
    let physical = PhysicalKey::from_native_code(code);
    let logical = physical
        .named_logical()
        .map(LogicalKey::Named)
        .or_else(|| {
            unsafe { string(logical) }
                .filter(|text| !text.is_empty())
                .map(LogicalKey::Character)
        })
        .unwrap_or(LogicalKey::Other);
    event(InputEvent::Key {
        physical,
        logical,
        pressed,
        repeat: repeated,
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_clipboard_complete(
    request: u64,
    write: bool,
    success: bool,
    ptr: *const c_char,
) {
    let result = if write {
        ClipboardResult::Written(success)
    } else {
        ClipboardResult::Text(if success {
            unsafe { string(ptr) }
        } else {
            None
        })
    };
    event(InputEvent::Clipboard { request, result });
}
