//! Winit owns window/event lifecycle; rendering is shared with foreign-layer hosts.
use super::*;
use ::winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{
        DeviceEvent, DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase,
        WindowEvent,
    },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey as WinitPhysical},
    window::{Window, WindowId},
};
use renderkit::{RenderError, Result, SurfaceState};
use std::sync::Arc;
use web_time::{Duration, Instant};

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
type ClipboardState = Option<arboard::Clipboard>;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
#[derive(Default)]
struct ClipboardState;

enum HostEvent {
    Ready(u64, Result<GpuPresenter<'static>>),
    Clipboard(u64, ClipboardResult),
}
#[derive(Clone, Copy, PartialEq)]
struct WindowState {
    text_input: Option<TextInputState>,
    cursor: crate::thecontext::TheCursorIcon,
    visible: bool,
}
struct Host {
    app: Box<dyn GpuApplication>,
    context: HostContext,
    window: Option<Arc<Window>>,
    presenter: Option<GpuPresenter<'static>>,
    proxy: EventLoopProxy<HostEvent>,
    epoch: u64,
    initialized: bool,
    resume_pending: bool,
    pending: bool,
    suspended: bool,
    occluded: bool,
    dirty: bool,
    next_present: Instant,
    next_update: Option<Instant>,
    last_update: Instant,
    pointer: [f32; 2],
    mods: Modifiers,
    ime: ImeSession,
    touch: Option<u64>,
    error: Option<RenderError>,
    applied: Option<WindowState>,
    focused: bool,
    clipboard: ClipboardState,
}
impl Host {
    fn sync_window_state(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let state = WindowState {
            text_input: self.context.text_input.filter(|r| {
                self.focused
                    && r.area.width > 0.
                    && r.area.height > 0.
                    && [r.area.x, r.area.y, r.area.width, r.area.height]
                        .iter()
                        .all(|v| v.is_finite())
            }),
            cursor: self.context.cursor,
            visible: self.context.cursor_visible,
        };
        let old = self.applied;
        let previous = self.ime.target();
        let changed = self
            .ime
            .set_target(state.text_input.map(|state| state.target));
        if changed && previous.is_some() && self.ime.target().is_some() {
            window.set_ime_allowed(false);
        }
        if changed || old.is_none() {
            window.set_ime_allowed(state.text_input.is_some());
        }
        if old.is_none_or(|old| old.text_input != state.text_input) {
            if let Some(input) = state.text_input {
                let area = input.area;
                window.set_ime_cursor_area(
                    ::winit::dpi::LogicalPosition::new(area.x as f64, area.y as f64),
                    ::winit::dpi::LogicalSize::new(
                        area.width.max(1.) as f64,
                        area.height.max(1.) as f64,
                    ),
                );
            }
        }
        if old.is_none_or(|old| old.visible != state.visible) {
            window.set_cursor_visible(state.visible);
        }
        if old.is_none_or(|old| old.cursor != state.cursor) {
            window.set_cursor(cursor(state.cursor));
        }
        self.applied = Some(state);
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: RenderError) {
        log::error!("GPU host: {error}");
        self.error = Some(error);
        event_loop.exit();
    }
    fn request_surface(&mut self) {
        if self.pending || self.suspended {
            return;
        }
        let Some(window) = self.window.clone() else {
            return;
        };
        self.pending = true;
        self.epoch += 1;
        let epoch = self.epoch;
        let size = window.inner_size();
        let proxy = self.proxy.clone();
        let initialize = async move {
            let result = SurfaceState::from_target(window, (size.width, size.height))
                .await
                .and_then(GpuPresenter::new);
            let _ = proxy.send_event(HostEvent::Ready(epoch, result));
        };
        #[cfg(not(target_arch = "wasm32"))]
        futures::executor::block_on(initialize);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(initialize);
    }
    fn route(&mut self, event: InputEvent) {
        if !self.initialized {
            return;
        }
        let response = self.app.input(&event, &mut self.context);
        self.dirty |= response.redraw;
        self.next_update = Some(Instant::now());
        self.sync_window_state();
    }
    fn resize(&mut self, size: PhysicalSize<u32>) {
        let density = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.);
        self.context.resize((size.width, size.height), density);
        self.next_present = Instant::now();
        if self.initialized {
            self.app.resize(&mut self.context);
        }
        if let Some(p) = &mut self.presenter {
            p.surface.resize(size.width, size.height);
        }
        self.dirty = true;
    }
    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.next_present = Instant::now() + Duration::from_millis(8);
        if self.suspended || self.occluded {
            return;
        }
        let Some(p) = &mut self.presenter else {
            return;
        };
        let size = p.surface.physical_size();
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        match p.present_with(self.context.density, [0., 0., 0., 1.], |images| {
            self.app.frame(&mut self.context, images)
        }) {
            Ok(PresentResult::Presented) => {
                self.app.presented(&mut self.context);
                self.dirty = false;
                self.sync_window_state();
            }
            Ok(PresentResult::Deferred) => {
                self.next_present = Instant::now() + Duration::from_millis(100);
                self.dirty = true;
            }
            Ok(PresentResult::SurfaceLost) => {
                self.presenter = None;
                self.request_surface();
            }
            Err(error) => self.fail(event_loop, error),
        }
    }
}
impl ApplicationHandler<HostEvent> for Host {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = false;
        self.last_update = Instant::now();
        self.applied = None;
        if self.window.is_none() {
            let (size_x, size_y) = self.app.initial_size();
            let attributes = Window::default_attributes()
                .with_title(self.app.title())
                .with_inner_size(LogicalSize::new(size_x, size_y));
            match event_loop.create_window(attributes) {
                Ok(window) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        use ::winit::platform::web::WindowExtWebSys;
                        if let Some(canvas) = window.canvas() {
                            if let Some(body) = web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.body())
                            {
                                if let Err(e) = body.append_child(&canvas) {
                                    self.fail(
                                        event_loop,
                                        RenderError(format!("attach canvas: {e:?}")),
                                    );
                                    return;
                                }
                            }
                        }
                    }
                    window.set_ime_allowed(false);
                    self.window = Some(Arc::new(window));
                }
                Err(error) => {
                    self.fail(event_loop, RenderError(format!("create window: {error}")));
                    return;
                }
            }
        }
        if let Some(p) = &mut self.presenter {
            p.surface.resume();
        } else {
            self.request_surface();
        }
        if let Some(window) = &self.window {
            self.resize(window.inner_size());
        }
        self.resume_pending = true;
        if self.initialized && self.presenter.is_some() {
            self.app.resumed(&mut self.context);
            self.resume_pending = false;
        }
        self.dirty = true;
        self.next_update = Some(Instant::now());
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        self.route(InputEvent::Focus(false));
        if self.initialized {
            self.app.suspended(&mut self.context);
        }
        self.presenter = None;
        self.pending = false;
        self.epoch += 1;
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        match event {
            HostEvent::Clipboard(request, result) => {
                self.route(InputEvent::Clipboard { request, result });
            }
            HostEvent::Ready(epoch, result) => {
                if epoch != self.epoch {
                    return;
                }
                self.pending = false;
                if self.suspended {
                    return;
                }
                match result {
                    Ok(mut presenter) => {
                        let result = if self.initialized {
                            self.app
                                .gpu_recreated(&mut self.context, &mut presenter.images)
                        } else {
                            self.app
                                .initialize(&mut self.context, &mut presenter.images)
                        };
                        if let Err(error) = result {
                            self.fail(event_loop, error);
                            return;
                        }
                        self.initialized = true;
                        self.app.resize(&mut self.context);
                        if self.resume_pending {
                            self.app.resumed(&mut self.context);
                            self.resume_pending = false;
                        }
                        if let Some(window) = &self.window {
                            let size = window.inner_size();
                            presenter.surface.resize(size.width, size.height);
                        }
                        self.presenter = Some(presenter);
                        self.dirty = true;
                    }
                    Err(error) => self.fail(event_loop, error),
                }
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|w| w.id() != id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.app.close_requested(&mut self.context) {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    self.resize(w.inner_size());
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::Occluded(value) => {
                self.occluded = value;
                if !value {
                    self.next_present = Instant::now();
                    self.dirty = true;
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.route(InputEvent::Focus(focused));
                if !focused {
                    self.mods = Modifiers::default();
                    self.ime.disable();
                    self.touch = None;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.context.density;
                self.pointer = [position.x as f32 / scale, position.y as f32 / scale];
                self.route(InputEvent::PointerMoved {
                    position: self.pointer,
                });
            }
            WindowEvent::CursorLeft { .. } => self.route(InputEvent::PointerLeft),
            WindowEvent::MouseInput { state, button, .. } => {
                self.route(InputEvent::PointerButton {
                    button: match button {
                        MouseButton::Left => PointerButton::Primary,
                        MouseButton::Right => PointerButton::Secondary,
                        MouseButton::Middle => PointerButton::Middle,
                        MouseButton::Other(id) => PointerButton::Other(id),
                        MouseButton::Back => PointerButton::Other(4),
                        MouseButton::Forward => PointerButton::Other(5),
                    },
                    pressed: state == ElementState::Pressed,
                    position: self.pointer,
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta, precise) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => ([x, y], false),
                    MouseScrollDelta::PixelDelta(p) => (
                        [
                            p.x as f32 / self.context.density,
                            p.y as f32 / self.context.density,
                        ],
                        true,
                    ),
                };
                self.route(InputEvent::Scroll { delta, precise });
            }
            WindowEvent::PinchGesture { delta, .. } => self.route(InputEvent::Pinch(delta as f32)),
            WindowEvent::ModifiersChanged(mods) => {
                let m = mods.state();
                self.mods = Modifiers {
                    shift: m.shift_key(),
                    control: m.control_key(),
                    alt: m.alt_key(),
                    super_key: m.super_key(),
                };
                self.route(InputEvent::Modifiers(self.mods));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let text_target = self.ime.target();
                let logical = match &event.logical_key {
                    Key::Character(s) => LogicalKey::Character(s.to_string()),
                    Key::Named(k) => named(*k)
                        .map(LogicalKey::Named)
                        .unwrap_or(LogicalKey::Other),
                    _ => LogicalKey::Other,
                };
                self.route(InputEvent::Key {
                    physical: physical(event.physical_key),
                    logical,
                    pressed: event.state == ElementState::Pressed,
                    repeat: event.repeat,
                });
                if event.state == ElementState::Pressed
                    && self.focused
                    && self.context.text_input.is_some()
                    && !self.mods.super_key
                    && (!self.mods.control || self.mods.alt)
                {
                    if let Some(text) = event.text {
                        if let Some(event) = self.ime.key_text(text_target, text.to_string()) {
                            self.route(event);
                        }
                    }
                }
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => self.ime.enable(),
                Ime::Disabled => {
                    if let Some(event) = self.ime.disable() {
                        self.route(event);
                    }
                }
                Ime::Preedit(text, cursor) => {
                    if let Some(event) = self.ime.preedit(text, cursor) {
                        self.route(event);
                    }
                }
                Ime::Commit(text) => {
                    for event in self.ime.commit(text) {
                        self.route(event);
                    }
                }
            },
            WindowEvent::DroppedFile(path) => self.route(InputEvent::FileDropped(path)),
            WindowEvent::Touch(touch) => {
                let scale = self.context.density;
                let position = [
                    touch.location.x as f32 / scale,
                    touch.location.y as f32 / scale,
                ];
                match touch.phase {
                    TouchPhase::Started if self.touch.is_none() => {
                        self.touch = Some(touch.id);
                        self.pointer = position;
                        self.route(InputEvent::PointerButton {
                            button: PointerButton::Primary,
                            pressed: true,
                            position,
                        });
                    }
                    TouchPhase::Moved if self.touch == Some(touch.id) => {
                        self.pointer = position;
                        self.route(InputEvent::PointerMoved { position });
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled if self.touch == Some(touch.id) => {
                        self.touch = None;
                        self.route(InputEvent::PointerButton {
                            button: PointerButton::Primary,
                            pressed: false,
                            position,
                        });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.route(InputEvent::RelativeMotion {
                delta: [delta.0 as f32, delta.1 as f32],
            });
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.suspended || !self.initialized {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        if self.next_update.is_some_and(|next| now >= next) {
            let request = self.app.update(&mut self.context, now - self.last_update);
            self.last_update = now;
            self.dirty |= request.redraw;
            self.next_update = request
                .wake_after
                .map(|delay| now + delay.max(Duration::from_millis(1)));
            self.sync_window_state();
        }
        for request in std::mem::take(&mut self.context.requests) {
            match request {
                HostRequest::Exit => event_loop.exit(),
                HostRequest::Fail(error) => {
                    self.fail(event_loop, RenderError(error));
                    return;
                }
                HostRequest::ClipboardRead { request } => {
                    read_clipboard(&mut self.clipboard, request, self.proxy.clone());
                }
                HostRequest::ClipboardWrite { text, request } => {
                    write_clipboard(&mut self.clipboard, text, request, self.proxy.clone());
                }
                HostRequest::Title(title) => {
                    if let Some(w) = &self.window {
                        w.set_title(&title);
                    }
                }
            }
        }
        let can_present = !self.occluded
            && self.presenter.as_ref().is_some_and(|p| {
                let size = p.surface.physical_size();
                size.0 > 0 && size.1 > 0
            });
        if self.dirty && can_present && now >= self.next_present {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        let wake = next_wake(
            self.next_update,
            self.dirty && can_present,
            self.next_present,
            now,
        );
        event_loop.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}
fn next_wake(
    update: Option<Instant>,
    present: bool,
    next_present: Instant,
    now: Instant,
) -> Option<Instant> {
    if present {
        let next = next_present.max(now + Duration::from_millis(1));
        Some(update.map_or(next, |update| update.min(next)))
    } else {
        update
    }
}
fn named(key: NamedKey) -> Option<crate::TheKeyCode> {
    use crate::TheKeyCode as K;
    Some(match key {
        NamedKey::Escape => K::Escape,
        NamedKey::Enter => K::Return,
        NamedKey::Backspace => K::Delete,
        NamedKey::Home => K::Home,
        NamedKey::End => K::End,
        NamedKey::ArrowLeft => K::Left,
        NamedKey::ArrowRight => K::Right,
        NamedKey::ArrowUp => K::Up,
        NamedKey::ArrowDown => K::Down,
        NamedKey::Tab => K::Tab,
        _ => return None,
    })
}
fn physical(key: WinitPhysical) -> PhysicalKey {
    use PhysicalKey as P;
    match key {
        WinitPhysical::Code(code) => match code {
            KeyCode::KeyA => P::Letter(b'A'),
            KeyCode::KeyB => P::Letter(b'B'),
            KeyCode::KeyC => P::Letter(b'C'),
            KeyCode::KeyD => P::Letter(b'D'),
            KeyCode::KeyE => P::Letter(b'E'),
            KeyCode::KeyF => P::Letter(b'F'),
            KeyCode::KeyG => P::Letter(b'G'),
            KeyCode::KeyH => P::Letter(b'H'),
            KeyCode::KeyI => P::Letter(b'I'),
            KeyCode::KeyJ => P::Letter(b'J'),
            KeyCode::KeyK => P::Letter(b'K'),
            KeyCode::KeyL => P::Letter(b'L'),
            KeyCode::KeyM => P::Letter(b'M'),
            KeyCode::KeyN => P::Letter(b'N'),
            KeyCode::KeyO => P::Letter(b'O'),
            KeyCode::KeyP => P::Letter(b'P'),
            KeyCode::KeyQ => P::Letter(b'Q'),
            KeyCode::KeyR => P::Letter(b'R'),
            KeyCode::KeyS => P::Letter(b'S'),
            KeyCode::KeyT => P::Letter(b'T'),
            KeyCode::KeyU => P::Letter(b'U'),
            KeyCode::KeyV => P::Letter(b'V'),
            KeyCode::KeyW => P::Letter(b'W'),
            KeyCode::KeyX => P::Letter(b'X'),
            KeyCode::KeyY => P::Letter(b'Y'),
            KeyCode::KeyZ => P::Letter(b'Z'),
            KeyCode::Digit0 => P::Digit(0),
            KeyCode::Digit1 => P::Digit(1),
            KeyCode::Digit2 => P::Digit(2),
            KeyCode::Digit3 => P::Digit(3),
            KeyCode::Digit4 => P::Digit(4),
            KeyCode::Digit5 => P::Digit(5),
            KeyCode::Digit6 => P::Digit(6),
            KeyCode::Digit7 => P::Digit(7),
            KeyCode::Digit8 => P::Digit(8),
            KeyCode::Digit9 => P::Digit(9),
            KeyCode::F1 => P::Function(1),
            KeyCode::F2 => P::Function(2),
            KeyCode::F3 => P::Function(3),
            KeyCode::F4 => P::Function(4),
            KeyCode::F5 => P::Function(5),
            KeyCode::F6 => P::Function(6),
            KeyCode::F7 => P::Function(7),
            KeyCode::F8 => P::Function(8),
            KeyCode::F9 => P::Function(9),
            KeyCode::F10 => P::Function(10),
            KeyCode::F11 => P::Function(11),
            KeyCode::F12 => P::Function(12),
            KeyCode::F13 => P::Function(13),
            KeyCode::F14 => P::Function(14),
            KeyCode::F15 => P::Function(15),
            KeyCode::F16 => P::Function(16),
            KeyCode::F17 => P::Function(17),
            KeyCode::F18 => P::Function(18),
            KeyCode::F19 => P::Function(19),
            KeyCode::F20 => P::Function(20),
            KeyCode::F21 => P::Function(21),
            KeyCode::F22 => P::Function(22),
            KeyCode::F23 => P::Function(23),
            KeyCode::F24 => P::Function(24),
            KeyCode::F25 => P::Function(25),
            KeyCode::F26 => P::Function(26),
            KeyCode::F27 => P::Function(27),
            KeyCode::F28 => P::Function(28),
            KeyCode::F29 => P::Function(29),
            KeyCode::F30 => P::Function(30),
            KeyCode::F31 => P::Function(31),
            KeyCode::F32 => P::Function(32),
            KeyCode::F33 => P::Function(33),
            KeyCode::F34 => P::Function(34),
            KeyCode::F35 => P::Function(35),
            KeyCode::Escape => P::Escape,
            KeyCode::Enter => P::Enter,
            KeyCode::Space => P::Space,
            KeyCode::Tab => P::Tab,
            KeyCode::Backspace => P::Backspace,
            KeyCode::Delete => P::Delete,
            KeyCode::ArrowLeft => P::Left,
            KeyCode::ArrowRight => P::Right,
            KeyCode::ArrowUp => P::Up,
            KeyCode::ArrowDown => P::Down,
            KeyCode::Home => P::Home,
            KeyCode::End => P::End,
            KeyCode::PageUp => P::PageUp,
            KeyCode::PageDown => P::PageDown,
            KeyCode::ShiftLeft => P::ShiftLeft,
            KeyCode::ShiftRight => P::ShiftRight,
            KeyCode::ControlLeft => P::ControlLeft,
            KeyCode::ControlRight => P::ControlRight,
            KeyCode::AltLeft => P::AltLeft,
            KeyCode::AltRight => P::AltRight,
            KeyCode::SuperLeft => P::SuperLeft,
            KeyCode::SuperRight => P::SuperRight,
            other => P::Other(other as u32),
        },
        WinitPhysical::Unidentified(_) => P::Other(u32::MAX),
    }
}
fn cursor(icon: crate::thecontext::TheCursorIcon) -> ::winit::window::CursorIcon {
    use crate::thecontext::TheCursorIcon as C;
    use ::winit::window::CursorIcon as W;
    match icon {
        C::Default => W::Default,
        C::Crosshair => W::Crosshair,
        C::Hand => W::Pointer,
        C::Arrow => W::Default,
        C::Text => W::Text,
        C::Wait => W::Wait,
        C::Help => W::Help,
        C::Progress => W::Progress,
        C::NotAllowed => W::NotAllowed,
        C::ContextMenu => W::ContextMenu,
        C::Cell => W::Cell,
        C::VerticalText => W::VerticalText,
        C::Alias => W::Alias,
        C::Copy => W::Copy,
        C::NoDrop => W::NoDrop,
        C::Grab => W::Grab,
        C::Grabbing => W::Grabbing,
        C::AllScroll => W::AllScroll,
        C::ZoomIn => W::ZoomIn,
        C::ZoomOut => W::ZoomOut,
        C::EResize => W::EResize,
        C::NResize => W::NResize,
        C::NEResize => W::NeResize,
        C::NWResize => W::NwResize,
        C::SResize => W::SResize,
        C::SEResize => W::SeResize,
        C::SWResize => W::SwResize,
        C::WResize => W::WResize,
        C::EWResize => W::EwResize,
        C::NSResize => W::NsResize,
        C::NESWResize => W::NeswResize,
        C::NWSEResize => W::NwseResize,
        C::ColResize => W::ColResize,
        C::RowResize => W::RowResize,
    }
}
pub fn run(app: Box<dyn GpuApplication>) -> Result<()> {
    let event_loop = EventLoop::<HostEvent>::with_user_event()
        .build()
        .map_err(|e| RenderError(e.to_string()))?;
    let proxy = event_loop.create_proxy();
    let size = app.initial_size();
    let now = Instant::now();
    #[allow(unused_mut)]
    let mut host = Host {
        app,
        context: HostContext::new(size),
        window: None,
        presenter: None,
        proxy,
        epoch: 0,
        initialized: false,
        resume_pending: true,
        pending: false,
        suspended: false,
        occluded: false,
        dirty: true,
        next_present: now,
        next_update: Some(now),
        last_update: now,
        pointer: [0.; 2],
        mods: Modifiers::default(),
        ime: ImeSession::default(),
        touch: None,
        error: None,
        applied: None,
        focused: true,
        clipboard: Default::default(),
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop
            .run_app(&mut host)
            .map_err(|e| RenderError(e.to_string()))?;
        if let Some(error) = host.error {
            return Err(error);
        }
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        use ::winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(host);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_hosts_sleep_until_input_and_animation_keeps_its_deadline() {
        let now = Instant::now();
        assert_eq!(next_wake(None, false, now, now), None);
        let update = now + Duration::from_millis(33);
        assert_eq!(next_wake(Some(update), false, now, now), Some(update));
        assert_eq!(
            next_wake(None, true, now, now),
            Some(now + Duration::from_millis(1))
        );
        let present = now + Duration::from_millis(8);
        assert_eq!(next_wake(Some(update), true, present, now), Some(present));
    }
    #[test]
    fn physical_commands_include_forward_delete_pages_and_function_keys() {
        let key = |code| physical(WinitPhysical::Code(code));
        assert_eq!(key(KeyCode::Delete), PhysicalKey::Delete);
        assert_eq!(key(KeyCode::Backspace), PhysicalKey::Backspace);
        assert_eq!(key(KeyCode::PageUp), PhysicalKey::PageUp);
        assert_eq!(key(KeyCode::PageDown), PhysicalKey::PageDown);
        assert_eq!(key(KeyCode::F1), PhysicalKey::Function(1));
        assert_eq!(key(KeyCode::F12), PhysicalKey::Function(12));
        assert_eq!(key(KeyCode::F35), PhysicalKey::Function(35));
    }
}

// Platform I/O only. Request tokens route completions back to the originating
// editor; focus/content validation remains in the editing controller.
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn read_clipboard(clipboard: &mut ClipboardState, request: u64, proxy: EventLoopProxy<HostEvent>) {
    if clipboard.is_none() {
        *clipboard = arboard::Clipboard::new().ok();
    }
    let text = clipboard.as_mut().and_then(|c| c.get_text().ok());
    let _ = proxy.send_event(HostEvent::Clipboard(request, ClipboardResult::Text(text)));
}
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn write_clipboard(
    clipboard: &mut ClipboardState,
    text: String,
    request: Option<u64>,
    proxy: EventLoopProxy<HostEvent>,
) {
    if clipboard.is_none() {
        *clipboard = arboard::Clipboard::new().ok();
    }
    let success = clipboard.as_mut().is_some_and(|c| c.set_text(text).is_ok());
    if let Some(request) = request {
        let _ = proxy.send_event(HostEvent::Clipboard(
            request,
            ClipboardResult::Written(success),
        ));
    }
}
#[cfg(target_arch = "wasm32")]
fn read_clipboard(_: &mut ClipboardState, request: u64, proxy: EventLoopProxy<HostEvent>) {
    wasm_bindgen_futures::spawn_local(async move {
        let text = if let Some(window) = web_sys::window() {
            wasm_bindgen_futures::JsFuture::from(window.navigator().clipboard().read_text())
                .await
                .ok()
                .and_then(|text| text.as_string())
        } else {
            None
        };
        let _ = proxy.send_event(HostEvent::Clipboard(request, ClipboardResult::Text(text)));
    });
}
#[cfg(target_arch = "wasm32")]
fn write_clipboard(
    _: &mut ClipboardState,
    text: String,
    request: Option<u64>,
    proxy: EventLoopProxy<HostEvent>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        let success = if let Some(window) = web_sys::window() {
            wasm_bindgen_futures::JsFuture::from(window.navigator().clipboard().write_text(&text))
                .await
                .is_ok()
        } else {
            false
        };
        if let Some(request) = request {
            let _ = proxy.send_event(HostEvent::Clipboard(
                request,
                ClipboardResult::Written(success),
            ));
        }
    });
}
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "macos",
    target_os = "windows",
    target_os = "linux"
)))]
fn read_clipboard(_: &mut ClipboardState, request: u64, proxy: EventLoopProxy<HostEvent>) {
    let _ = proxy.send_event(HostEvent::Clipboard(request, ClipboardResult::Text(None)));
}
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "macos",
    target_os = "windows",
    target_os = "linux"
)))]
fn write_clipboard(
    _: &mut ClipboardState,
    _: String,
    request: Option<u64>,
    proxy: EventLoopProxy<HostEvent>,
) {
    if let Some(request) = request {
        let _ = proxy.send_event(HostEvent::Clipboard(
            request,
            ClipboardResult::Written(false),
        ));
    }
}
