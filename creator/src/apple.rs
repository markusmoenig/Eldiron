//! Main-thread Xcode facade. No CPU destination bitmap crosses this ABI.
use crate::{native_application::NativeCreatorApplication, prelude::*};
use renderkit::SurfaceState;
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
};
use theframework::platform::*;

mod owner;
mod v1;

type CreatorHost = ForeignApplicationHost<UiApplicationAdapter<NativeCreatorApplication>>;
unsafe extern "C" {
    fn pthread_main_np() -> i32;
}
thread_local! {
    static APP: RefCell<owner::Owner<CreatorHost>> = const { RefCell::new(owner::Owner::new()) };
}

fn with_app<T: Default>(f: impl FnOnce(&mut CreatorHost) -> T) -> T {
    if unsafe { pthread_main_np() } == 0 {
        return T::default();
    }
    catch_unwind(AssertUnwindSafe(|| {
        APP.with(|app| app.borrow_mut().as_mut().map(f).unwrap_or_default())
    }))
    .unwrap_or_else(|_| {
        eprintln!("[EldironCreator] Panic in native callback");
        T::default()
    })
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
    if unsafe { pthread_main_np() } == 0 {
        return false;
    }
    catch_unwind(AssertUnwindSafe(|| {
        APP.with(|app| {
            if app.borrow().is_some() || layer.is_null() {
                return false;
            }
            let _ = crate::i18n::select_system_locales();
            let result = unsafe { presenter(layer, (width, height)) }.and_then(|presenter| {
                ForeignApplicationHost::new(
                    UiApplicationAdapter::new(NativeCreatorApplication::new(
                        vec!["eldiron-creator".into()],
                        false,
                    )),
                    presenter,
                    density,
                )
            });
            match result {
                Ok(host) => app.borrow_mut().install(host).is_some(),
                Err(e) => {
                    eprintln!("[EldironCreator] {e}");
                    false
                }
            }
        })
    }))
    .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_shutdown() {
    if unsafe { pthread_main_np() } == 0 {
        return;
    }
    with_app(|app| app.suspend());
    APP.with(|app| {
        app.borrow_mut().take();
    });
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
    with_app(|app| {
        match unsafe { presenter(layer, app.host.physical_size) }
            .and_then(|next| app.replace_presenter(next))
        {
            Ok(()) => true,
            Err(e) => {
                eprintln!("[EldironCreator] {e}");
                false
            }
        }
    })
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
    match crate::editor::CreatorTheme::load() {
        crate::editor::CreatorTheme::Brown => 0,
        crate::editor::CreatorTheme::Black => 1,
        crate::editor::CreatorTheme::Light => 2,
    }
}

/// Native Edit menu commands use the same controller/clipboard route as keys.
#[unsafe(no_mangle)]
pub extern "C" fn rust_edit_command(code: u32) {
    let (letter, shift) = match code {
        0 => (b'x', false),
        1 => (b'c', false),
        2 => (b'v', false),
        3 => (b'z', false),
        4 => (b'z', true),
        _ => return,
    };
    with_app(|app| {
        let ui = &app.application.session.ui;
        let old = Modifiers {
            shift: ui.shift,
            control: ui.ctrl,
            alt: ui.alt,
            super_key: ui.logo,
        };
        app.input(&InputEvent::Modifiers(Modifiers {
            super_key: true,
            shift,
            ..Default::default()
        }));
        for pressed in [true, false] {
            app.input(&InputEvent::Key {
                physical: PhysicalKey::Letter(letter),
                logical: LogicalKey::Character((letter as char).to_string()),
                pressed,
                repeat: false,
            });
        }
        app.input(&InputEvent::Modifiers(old));
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
    with_app(|app| {
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
        let state = serde_json::json!({
            "presentations": app.presentations(),
            "updates": app.updates(),
            "physical_size": app.host.physical_size,
            "logical_size": app.host.logical_size,
            "density": app.host.density,
            "root_cpu_bytes": app.application.session.ui.canvas.buffer.pixels().len(),
            "readback_bytes": app.presenter.context().transfers().readback_bytes,
            "text_input": input,
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
    let logical = unsafe { string(logical) }
        .filter(|text| !text.is_empty())
        .map(LogicalKey::Character)
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
