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

enum HostEvent {
    Ready(u64, Result<GpuPresenter<'static>>),
}
struct Host {
    app: Box<dyn GpuApplication>,
    ui: UiSession,
    window: Option<Arc<Window>>,
    presenter: Option<GpuPresenter<'static>>,
    proxy: EventLoopProxy<HostEvent>,
    epoch: u64,
    initialized: bool,
    pending: bool,
    suspended: bool,
    occluded: bool,
    dirty: bool,
    next_present: Instant,
    next_update: Instant,
    last_update: Instant,
    pointer: [f32; 2],
    mods: Modifiers,
    ime_enabled: bool,
    touch: Option<u64>,
    error: Option<RenderError>,
}
impl Host {
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
        let response = self.app.input(&event, &mut self.ui);
        self.dirty |= response.redraw;
    }
    fn resize(&mut self, size: PhysicalSize<u32>) {
        let density = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.);
        self.ui.resize((size.width, size.height), density);
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
        match self
            .app
            .frame(&mut self.ui, &mut p.images)
            .and_then(|list| p.present(&list, self.ui.context.scale_factor, [0., 0., 0., 1.]))
        {
            Ok(PresentResult::Presented) => self.dirty = false,
            Ok(PresentResult::Deferred) => {
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
                    window.set_ime_allowed(true);
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
        self.app.resumed(&mut self.ui);
        self.dirty = true;
        self.next_update = Instant::now();
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.suspended = true;
        self.route(InputEvent::Focus(false));
        self.app.suspended(&mut self.ui);
        self.presenter = None;
        self.pending = false;
        self.epoch += 1;
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: HostEvent) {
        match event {
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
                            self.app.gpu_recreated(&mut self.ui, &mut presenter.images)
                        } else {
                            self.app.initialize(&mut self.ui, &mut presenter.images)
                        };
                        if let Err(error) = result {
                            self.fail(event_loop, error);
                            return;
                        }
                        self.initialized = true;
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
                if self.app.close_requested(&mut self.ui) {
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
                    self.dirty = true;
                }
            }
            WindowEvent::Focused(focused) => {
                self.route(InputEvent::Focus(focused));
                if !focused {
                    self.mods = Modifiers::default();
                    self.ime_enabled = false;
                    self.touch = None;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.ui.context.scale_factor;
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
                    MouseScrollDelta::LineDelta(x, y) => ([x * 20., y * 20.], false),
                    MouseScrollDelta::PixelDelta(p) => (
                        [
                            p.x as f32 / self.ui.context.scale_factor,
                            p.y as f32 / self.ui.context.scale_factor,
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
                    && !self.ime_enabled
                    && !self.mods.super_key
                    && (!self.mods.control || self.mods.alt)
                {
                    if let Some(text) = event.text {
                        let text: String = text.chars().filter(|c| !c.is_control()).collect();
                        if !text.is_empty() {
                            self.route(InputEvent::Text(text));
                        }
                    }
                }
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => self.ime_enabled = true,
                Ime::Disabled => {
                    self.ime_enabled = false;
                    self.route(InputEvent::Composition {
                        text: String::new(),
                        cursor: None,
                    });
                }
                Ime::Preedit(text, cursor) => self.route(InputEvent::Composition { text, cursor }),
                Ime::Commit(text) => {
                    self.route(InputEvent::Composition {
                        text: String::new(),
                        cursor: None,
                    });
                    self.route(InputEvent::Text(text));
                }
            },
            WindowEvent::DroppedFile(path) => self.route(InputEvent::FileDropped(path)),
            WindowEvent::Touch(touch) => {
                let scale = self.ui.context.scale_factor;
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
        if now >= self.next_update {
            crate::platform_services::service_files(&mut self.ui.context.ui);
            let request = self.app.update(&mut self.ui, now - self.last_update);
            self.last_update = now;
            self.dirty |= request.redraw || self.ui.context.ui.redraw_all || self.ui.ui.is_dirty;
            // UI requesters, tooltip deadlines and editing timers remain responsive.
            self.next_update = now
                + request
                    .wake_after
                    .unwrap_or(Duration::from_millis(50))
                    .min(Duration::from_millis(50));
        }
        for request in self.ui.requests.drain(..) {
            match request {
                HostRequest::Exit => event_loop.exit(),
                HostRequest::Title(title) => {
                    if let Some(w) = &self.window {
                        w.set_title(&title);
                    }
                }
            }
        }
        if self.dirty && !self.occluded && now >= self.next_present {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(if self.dirty && !self.occluded {
            self.next_update
                .min(self.next_present.max(now + Duration::from_millis(1)))
        } else {
            self.next_update
        }));
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
        ui: UiSession::new((size.0 as usize, size.1 as usize), 1.),
        window: None,
        presenter: None,
        proxy,
        epoch: 0,
        initialized: false,
        pending: false,
        suspended: false,
        occluded: false,
        dirty: true,
        next_present: now,
        next_update: now,
        last_update: now,
        pointer: [0.; 2],
        mods: Modifiers::default(),
        ime_enabled: false,
        touch: None,
        error: None,
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
