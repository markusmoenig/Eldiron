use std::{num::NonZeroU32, sync::Arc};

#[cfg(feature = "ui")]
use rfd::MessageDialog;

use crate::thecontext::TheCursorIcon;

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowExtMacOS;

use crate::prelude::*;
use softbuffer::Surface;
use web_time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{
        DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, StartCause, Touch,
        TouchPhase, WindowEvent,
    },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, ModifiersState, NamedKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};

// Platform-aware accelerator modifiers (AltGr-safe)
#[inline]
fn is_accel_mods(m: &ModifiersState) -> bool {
    #[cfg(target_os = "macos")]
    {
        // macOS commonly uses ⌘ and ⌥ for accelerators
        m.super_key() || m.alt_key() || m.control_key()
    }
    #[cfg(not(target_os = "macos"))]
    {
        // On Windows/Linux, avoid treating Alt as an accelerator by default (AltGr = Ctrl+Alt)
        m.super_key() || m.control_key()
    }
}

// US-ANSI physical keycode to ascii (letters, digits, punctuation, space)
fn accel_physical_to_ascii(code: KeyCode, shift: bool) -> Option<char> {
    let (base, shifted) = match code {
        // Letters
        KeyCode::KeyA => ('a', 'A'),
        KeyCode::KeyB => ('b', 'B'),
        KeyCode::KeyC => ('c', 'C'),
        KeyCode::KeyD => ('d', 'D'),
        KeyCode::KeyE => ('e', 'E'),
        KeyCode::KeyF => ('f', 'F'),
        KeyCode::KeyG => ('g', 'G'),
        KeyCode::KeyH => ('h', 'H'),
        KeyCode::KeyI => ('i', 'I'),
        KeyCode::KeyJ => ('j', 'J'),
        KeyCode::KeyK => ('k', 'K'),
        KeyCode::KeyL => ('l', 'L'),
        KeyCode::KeyM => ('m', 'M'),
        KeyCode::KeyN => ('n', 'N'),
        KeyCode::KeyO => ('o', 'O'),
        KeyCode::KeyP => ('p', 'P'),
        KeyCode::KeyQ => ('q', 'Q'),
        KeyCode::KeyR => ('r', 'R'),
        KeyCode::KeyS => ('s', 'S'),
        KeyCode::KeyT => ('t', 'T'),
        KeyCode::KeyU => ('u', 'U'),
        KeyCode::KeyV => ('v', 'V'),
        KeyCode::KeyW => ('w', 'W'),
        KeyCode::KeyX => ('x', 'X'),
        KeyCode::KeyY => ('y', 'Y'),
        KeyCode::KeyZ => ('z', 'Z'),
        // Digits (US)
        KeyCode::Digit0 => ('0', ')'),
        KeyCode::Digit1 => ('1', '!'),
        KeyCode::Digit2 => ('2', '@'),
        KeyCode::Digit3 => ('3', '#'),
        KeyCode::Digit4 => ('4', '$'),
        KeyCode::Digit5 => ('5', '%'),
        KeyCode::Digit6 => ('6', '^'),
        KeyCode::Digit7 => ('7', '&'),
        KeyCode::Digit8 => ('8', '*'),
        KeyCode::Digit9 => ('9', '('),
        // Punctuation (US ANSI)
        KeyCode::Minus => ('-', '_'),
        KeyCode::Equal => ('=', '+'),
        KeyCode::BracketLeft => ('[', '{'),
        KeyCode::BracketRight => (']', '}'),
        KeyCode::Semicolon => (';', ':'),
        KeyCode::Quote => ('\'', '"'),
        KeyCode::Comma => (',', '<'),
        KeyCode::Period => ('.', '>'),
        KeyCode::Slash => ('/', '?'),
        KeyCode::Backquote => ('`', '~'),
        KeyCode::Backslash => ('\\', '|'),
        KeyCode::IntlBackslash => ('\\', '|'),
        // Space
        KeyCode::Space => (' ', ' '),
        _ => return None,
    };
    Some(if shift { shifted } else { base })
}

fn blit_rgba_into_softbuffer(
    ui_frame: &[u8],
    scale_factor: f32,
    width: usize,
    height: usize,
    dest: &mut [u32],
) {
    // Round to match the resized surface at fractional DPI across platforms
    let dest_width = (width as f32 * scale_factor).round() as usize;
    let dest_height = (height as f32 * scale_factor).round() as usize;

    if scale_factor == 1.0 {
        // Direct copy without extra allocation.
        for (dst, rgba) in dest.iter_mut().zip(ui_frame.chunks_exact(4)) {
            *dst = (rgba[2] as u32) | ((rgba[1] as u32) << 8) | ((rgba[0] as u32) << 16);
        }
    } else {
        // Nearest-neighbor upscaling with fractional scale factors
        for dest_y in 0..dest_height {
            let src_y = (dest_y as f32 / scale_factor) as usize;
            if src_y >= height {
                continue;
            }

            for dest_x in 0..dest_width {
                let src_x = (dest_x as f32 / scale_factor) as usize;
                if src_x >= width {
                    continue;
                }

                let src_offset = (src_y * width + src_x) * 4;
                let r = ui_frame[src_offset] as u32;
                let g = ui_frame[src_offset + 1] as u32;
                let b = ui_frame[src_offset + 2] as u32;
                let color = b | (g << 8) | (r << 16);

                let dest_offset = dest_y * dest_width + dest_x;
                if dest_offset < dest.len() {
                    dest[dest_offset] = color;
                }
            }
        }
    }
}

fn translate_coord_to_local(x: f32, y: f32, scale_factor: f32) -> (f32, f32) {
    (x / scale_factor, y / scale_factor)
}

struct TheWinitContext {
    window: Arc<Window>,
    ctx: TheContext,
    ui_frame: Vec<u8>,
    surface: Surface<Arc<Window>, Arc<Window>>,
}

impl TheWinitContext {
    fn from_window(window: Arc<Window>) -> Self {
        #[cfg(not(target_os = "macos"))]
        let scale_factor = 1.0;
        // Make sure to set the initial scale factor on macOS
        #[cfg(target_os = "macos")]
        let scale_factor = window.scale_factor() as f32;

        let size = window.inner_size();

        // WASM-specific fix: On WASM with Retina displays, inner_size() returns physical size
        // We need to divide by scale factor to get logical size
        #[cfg(target_arch = "wasm32")]
        let (width, height) = {
            let wasm_scale = window.scale_factor() as f32;
            (
                (size.width as f32 / wasm_scale) as usize,
                (size.height as f32 / wasm_scale) as usize,
            )
        };
        #[cfg(not(target_arch = "wasm32"))]
        let (width, height) = (size.width as usize, size.height as usize);

        // println!("=== from_window DEBUG ===");
        // println!(
        //     "Window scale_factor (from winit): {}",
        //     window.scale_factor()
        // );
        // println!("Using scale_factor: {}", scale_factor);
        // println!("Physical size (inner_size): {}x{}", size.width, size.height);
        // println!("Context size: {}x{}", width, height);
        // println!("ui_frame size: {} bytes", width * height * 4);

        let ctx = TheContext::new(width, height, scale_factor);

        let ui_frame = vec![0; (width * height * 4) as usize];

        let context = softbuffer::Context::new(window.clone()).unwrap();
        let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        // On Windows/Linux, don't scale the surface (use 1.0)
        // On macOS, scale by scale_factor
        // On WASM, use logical size directly
        #[cfg(target_arch = "wasm32")]
        let (surface_width, surface_height) = (width as u32, height as u32);
        #[cfg(not(target_arch = "wasm32"))]
        let (surface_width, surface_height) = {
            #[cfg(target_os = "macos")]
            let surface_scale = scale_factor;
            #[cfg(not(target_os = "macos"))]
            let surface_scale = 1.0;

            (
                size.width * surface_scale as u32,
                size.height * surface_scale as u32,
            )
        };
        // println!("Surface size: {}x{}", surface_width, surface_height);

        if let (Some(width), Some(height)) = (
            NonZeroU32::new(surface_width),
            NonZeroU32::new(surface_height),
        ) {
            surface.resize(width, height).unwrap();
        }
        // println!("========================\n");

        TheWinitContext {
            window,
            ctx,
            ui_frame,
            surface,
        }
    }
}

struct TheWinitApp {
    args: Option<Vec<String>>,
    ctx: Option<TheWinitContext>,
    app: Box<dyn TheTrait>,

    mods: ModifiersState,
    target_frame_time: Duration,
    next_frame_time: Instant,
    last_cursor_pos: Option<(f32, f32)>,
    left_mouse_down: bool,
    has_changes: bool,

    #[cfg(feature = "ui")]
    ui: TheUI,
}

impl TheWinitApp {
    fn new(args: Option<Vec<String>>, app: Box<dyn TheTrait>) -> Self {
        let fps = app.target_fps();

        TheWinitApp {
            args,
            ctx: None,
            app,
            mods: ModifiersState::empty(),
            target_frame_time: Duration::from_secs_f64(1.0 / fps),
            next_frame_time: Instant::now(),
            last_cursor_pos: None,
            left_mouse_down: false,
            has_changes: false,
            #[cfg(feature = "ui")]
            ui: TheUI::new(),
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Option<Arc<Window>> {
        let window_title = self.app.window_title();
        let mut icon: Option<Icon> = None;
        if let Some(window_icon) = self.app.window_icon() {
            icon = Icon::from_rgba(window_icon.0, window_icon.1, window_icon.2).ok();
        }

        let (width, height) = self.app.default_window_size();
        let size = LogicalSize::new(width as f64, height as f64);

        let window_attributes = WindowAttributes::default()
            .with_title(window_title)
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_window_icon(icon); //TODO on Windows

        #[cfg(target_arch = "wasm32")]
        let window_attributes = {
            use winit::platform::web::WindowAttributesExtWebSys;

            window_attributes.with_append(true)
        };

        let window = event_loop.create_window(window_attributes).unwrap();

        Some(Arc::new(window))
    }

    fn init_context(&mut self, window: Arc<Window>) -> TheWinitContext {
        let mut ctx = TheWinitContext::from_window(window);

        #[cfg(feature = "ui")]
        {
            self.ui.init(&mut ctx.ctx);

            self.ui.canvas.root = true;
            self.ui.canvas.set_dim(
                TheDim::new(0, 0, ctx.ctx.width as i32, ctx.ctx.height as i32),
                &mut ctx.ctx,
            );

            self.app.init_ui(&mut self.ui, &mut ctx.ctx);
            self.ui
                .canvas
                .layout(ctx.ctx.width as i32, ctx.ctx.height as i32, &mut ctx.ctx);
        }

        #[cfg(feature = "i18n")]
        ctx.ctx.load_system_fonts(self.app.fonts_to_load());

        self.app.init(&mut ctx.ctx);

        // If available set the command line arguments to the trait.
        if let Some(args) = self.args.take() {
            self.app.set_cmd_line_args(args, &mut ctx.ctx);
        }

        ctx
    }

    fn render(&mut self) {
        let Some(ctx) = &mut self.ctx else {
            return;
        };

        if ctx.ctx.width == 0 || ctx.ctx.height == 0 {
            return;
        }

        #[cfg(feature = "ui")]
        self.app.pre_ui(&mut ctx.ctx);

        #[cfg(feature = "ui")]
        self.ui.draw(&mut ctx.ui_frame, &mut ctx.ctx);

        // We always call this for apps who use the "ui" feature
        // but do not use the UI API
        self.app.draw(&mut ctx.ui_frame, &mut ctx.ctx);

        // On Windows/Linux, try to use the actual scale_factor, but verify the dest buffer is large enough
        // On macOS, use the actual scale_factor for Retina displays
        #[cfg(target_os = "macos")]
        let blit_scale_factor = ctx.ctx.scale_factor;
        #[cfg(not(target_os = "macos"))]
        let blit_scale_factor = {
            let buffer = ctx.surface.buffer_mut().unwrap();
            let inner_size = ctx.window.inner_size();
            // Derive scale from the actual surface size to avoid double rounding (Windows fractional DPI)
            let desired_scale = inner_size.width as f32 / ctx.ctx.width as f32;

            let dest_width = inner_size.width as usize;
            let dest_height = inner_size.height as usize;
            let required_size = dest_width * dest_height;

            // Check if the destination buffer is large enough for the upscaled blit
            // If not, fall back to scale_factor = 1.0 to avoid crashes/panics
            if buffer.len() >= required_size {
                desired_scale
            } else {
                println!(
                    "Warning: Buffer too small for scale_factor {}. Required: {}, Available: {}. Falling back to scale_factor = 1.0",
                    desired_scale, required_size, buffer.len()
                );
                1.0
            }
        };

        let mut buffer = ctx.surface.buffer_mut().unwrap();
        blit_rgba_into_softbuffer(
            &ctx.ui_frame,
            blit_scale_factor,
            ctx.ctx.width,
            ctx.ctx.height,
            &mut *buffer,
        );
        buffer.present().unwrap();

        #[cfg(feature = "ui")]
        self.app.post_ui(&mut ctx.ctx);
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        let Some(ctx) = &mut self.ctx else {
            return;
        };

        if size.width != 0 && size.height != 0 {
            // println!("=== resize DEBUG ===");
            // println!("New physical size: {}x{}", size.width, size.height);

            let scale_factor = ctx.window.scale_factor() as f32;

            // On non-macOS, if DPI scale is fractional, render at physical resolution with scale_factor forced to 1.0
            #[cfg(all(not(target_os = "macos"), not(target_arch = "wasm32")))]
            let (effective_scale, width, height) = if scale_factor.fract() != 0.0 {
                (1.0_f32, size.width, size.height)
            } else {
                (
                    scale_factor,
                    (size.width as f32 / scale_factor).round() as u32,
                    (size.height as f32 / scale_factor).round() as u32,
                )
            };
            // macOS and WASM: keep logical sizing based on scale_factor
            #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
            let (effective_scale, width, height) = (
                scale_factor,
                (size.width as f32 / scale_factor).round() as u32,
                (size.height as f32 / scale_factor).round() as u32,
            );

            ctx.ctx.scale_factor = effective_scale;

            // WASM-specific: surface should use logical size
            #[cfg(target_arch = "wasm32")]
            ctx.surface
                .resize(
                    NonZeroU32::new(width as u32).unwrap(),
                    NonZeroU32::new(height as u32).unwrap(),
                )
                .unwrap();

            // Desktop: surface uses physical size
            #[cfg(not(target_arch = "wasm32"))]
            {
                #[cfg(not(target_os = "macos"))]
                ctx.surface
                    .resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    )
                    .unwrap();
                #[cfg(target_os = "macos")]
                ctx.surface
                    .resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    )
                    .unwrap();
            }

            // println!("Window scale_factor: {}", scale_factor);
            // println!("New logical size: {}x{}", width, height);
            // println!("New ui_frame size: {} bytes", width * height * 4);

            ctx.ctx.width = width as usize;
            ctx.ctx.height = height as usize;

            ctx.ui_frame.resize((width * height * 4) as usize, 0);
            // println!("===================\n");

            #[cfg(feature = "ui")]
            self.ui
                .canvas
                .set_dim(TheDim::new(0, 0, width as i32, height as i32), &mut ctx.ctx);
            #[cfg(feature = "ui")]
            ctx.ctx.ui.send(TheEvent::Resize);

            ctx.window.request_redraw();
        }
    }
}

impl ApplicationHandler for TheWinitApp {
    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {}

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.ctx.is_none() {
            if let Some(window) = self.create_window(event_loop) {
                self.ctx = Some(self.init_context(window));
                // Set initial cursor to default and ensure it's visible
                if let Some(ctx) = &mut self.ctx {
                    ctx.ctx.set_cursor_icon(TheCursorIcon::Default);
                    ctx.window.set_cursor_visible(true);
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CloseRequested => {
                if !self.app.closing() {
                    #[cfg(feature = "ui")]
                    {
                        if self.app.has_changes() {
                            let result = MessageDialog::new()
                                .set_title("Unsaved Changes")
                                .set_description(
                                    "You have unsaved changes. Are you sure you want to quit?",
                                )
                                .set_buttons(rfd::MessageButtons::YesNo)
                                .show();

                            if result == rfd::MessageDialogResult::Yes {
                                event_loop.exit();
                            }
                        } else {
                            event_loop.exit();
                        }
                    }
                    #[cfg(not(feature = "ui"))]
                    {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.resize(size);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(ctx) = &mut self.ctx {
                    ctx.ctx.scale_factor = scale_factor as f32;
                    let size = ctx.window.inner_size();
                    self.resize(size);
                }
            }
            event => {
                let Some(ctx) = &mut self.ctx else {
                    return;
                };

                match event {
                    WindowEvent::KeyboardInput {
                        event: key_event, ..
                    } => {
                        if key_event.state == ElementState::Pressed {
                            let key = match &key_event.logical_key {
                                Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace) => {
                                    Some(TheKeyCode::Delete)
                                }
                                Key::Named(NamedKey::ArrowUp) => Some(TheKeyCode::Up),
                                Key::Named(NamedKey::ArrowRight) => Some(TheKeyCode::Right),
                                Key::Named(NamedKey::ArrowDown) => Some(TheKeyCode::Down),
                                Key::Named(NamedKey::ArrowLeft) => Some(TheKeyCode::Left),
                                Key::Named(NamedKey::Space) => Some(TheKeyCode::Space),
                                Key::Named(NamedKey::Tab) => Some(TheKeyCode::Tab),
                                Key::Named(NamedKey::Enter) => Some(TheKeyCode::Return),
                                Key::Named(NamedKey::Escape) => Some(TheKeyCode::Escape),
                                Key::Character(str) => {
                                    // Accelerator: use physical key with modifiers (ignore composed text like "å")
                                    if is_accel_mods(&self.mods) {
                                        if let winit::keyboard::PhysicalKey::Code(code) =
                                            key_event.physical_key
                                        {
                                            if let Some(ch) =
                                                accel_physical_to_ascii(code, self.mods.shift_key())
                                            {
                                                #[cfg(feature = "ui")]
                                                if self.ui.key_down(Some(ch), None, &mut ctx.ctx) {
                                                    ctx.window.request_redraw();
                                                }
                                                if self.app.key_down(Some(ch), None, &mut ctx.ctx) {
                                                    ctx.window.request_redraw();
                                                }
                                                return;
                                            }
                                        }
                                    }
                                    if str.is_ascii() {
                                        for ch in str.chars() {
                                            #[cfg(feature = "ui")]
                                            if self.ui.key_down(Some(ch), None, &mut ctx.ctx) {
                                                ctx.window.request_redraw();
                                            }
                                            if self.app.key_down(Some(ch), None, &mut ctx.ctx) {
                                                ctx.window.request_redraw();
                                            }
                                        }
                                    }
                                    None
                                }
                                _ => None,
                            };
                            if key.is_some() {
                                #[cfg(feature = "ui")]
                                if self.ui.key_down(None, key.clone(), &mut ctx.ctx) {
                                    ctx.window.request_redraw();
                                }
                                if self.app.key_down(None, key, &mut ctx.ctx) {
                                    ctx.window.request_redraw();
                                }
                            }

                            // Update cursor icon after keyboard events (focus may have changed)
                            if ctx.ctx.cursor_changed() {
                                let cursor_icon = match ctx.ctx.cursor_icon() {
                                    TheCursorIcon::Default => winit::window::CursorIcon::Default,
                                    TheCursorIcon::Crosshair => {
                                        winit::window::CursorIcon::Crosshair
                                    }
                                    TheCursorIcon::Hand => winit::window::CursorIcon::Pointer,
                                    TheCursorIcon::Arrow => winit::window::CursorIcon::Default,
                                    TheCursorIcon::Text => winit::window::CursorIcon::Text,
                                    TheCursorIcon::Wait => winit::window::CursorIcon::Wait,
                                    TheCursorIcon::Help => winit::window::CursorIcon::Help,
                                    TheCursorIcon::Progress => winit::window::CursorIcon::Progress,
                                    TheCursorIcon::NotAllowed => {
                                        winit::window::CursorIcon::NotAllowed
                                    }
                                    TheCursorIcon::ContextMenu => {
                                        winit::window::CursorIcon::ContextMenu
                                    }
                                    TheCursorIcon::Cell => winit::window::CursorIcon::Cell,
                                    TheCursorIcon::VerticalText => {
                                        winit::window::CursorIcon::VerticalText
                                    }
                                    TheCursorIcon::Alias => winit::window::CursorIcon::Alias,
                                    TheCursorIcon::Copy => winit::window::CursorIcon::Copy,
                                    TheCursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
                                    TheCursorIcon::Grab => winit::window::CursorIcon::Grab,
                                    TheCursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
                                    TheCursorIcon::AllScroll => {
                                        winit::window::CursorIcon::AllScroll
                                    }
                                    TheCursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
                                    TheCursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
                                    TheCursorIcon::EResize => winit::window::CursorIcon::EResize,
                                    TheCursorIcon::NResize => winit::window::CursorIcon::NResize,
                                    TheCursorIcon::NEResize => winit::window::CursorIcon::NeResize,
                                    TheCursorIcon::NWResize => winit::window::CursorIcon::NwResize,
                                    TheCursorIcon::SResize => winit::window::CursorIcon::SResize,
                                    TheCursorIcon::SEResize => winit::window::CursorIcon::SeResize,
                                    TheCursorIcon::SWResize => winit::window::CursorIcon::SwResize,
                                    TheCursorIcon::WResize => winit::window::CursorIcon::WResize,
                                    TheCursorIcon::EWResize => winit::window::CursorIcon::EwResize,
                                    TheCursorIcon::NSResize => winit::window::CursorIcon::NsResize,
                                    TheCursorIcon::NESWResize => {
                                        winit::window::CursorIcon::NeswResize
                                    }
                                    TheCursorIcon::NWSEResize => {
                                        winit::window::CursorIcon::NwseResize
                                    }
                                    TheCursorIcon::ColResize => {
                                        winit::window::CursorIcon::ColResize
                                    }
                                    TheCursorIcon::RowResize => {
                                        winit::window::CursorIcon::RowResize
                                    }
                                };
                                ctx.window.set_cursor(cursor_icon);
                                ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                                ctx.ctx.reset_cursor_changed();
                            }
                            if ctx.ctx.cursor_visible_changed() {
                                ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                                ctx.ctx.reset_cursor_visible_changed();
                            }
                        }
                        if key_event.state == ElementState::Released {
                            let key = match &key_event.logical_key {
                                Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace) => {
                                    Some(TheKeyCode::Delete)
                                }
                                Key::Named(NamedKey::ArrowUp) => Some(TheKeyCode::Up),
                                Key::Named(NamedKey::ArrowRight) => Some(TheKeyCode::Right),
                                Key::Named(NamedKey::ArrowDown) => Some(TheKeyCode::Down),
                                Key::Named(NamedKey::ArrowLeft) => Some(TheKeyCode::Left),
                                Key::Named(NamedKey::Space) => Some(TheKeyCode::Space),
                                Key::Named(NamedKey::Tab) => Some(TheKeyCode::Tab),
                                Key::Named(NamedKey::Enter) => Some(TheKeyCode::Return),
                                Key::Named(NamedKey::Escape) => Some(TheKeyCode::Escape),
                                Key::Character(str) => {
                                    // Accelerator release: use physical key with modifiers (ignore composed text)
                                    if is_accel_mods(&self.mods) {
                                        if let winit::keyboard::PhysicalKey::Code(code) =
                                            key_event.physical_key
                                        {
                                            if let Some(ch) =
                                                accel_physical_to_ascii(code, self.mods.shift_key())
                                            {
                                                #[cfg(feature = "ui")]
                                                if self.ui.key_up(Some(ch), None, &mut ctx.ctx) {
                                                    ctx.window.request_redraw();
                                                }
                                                if self.app.key_up(Some(ch), None, &mut ctx.ctx) {
                                                    ctx.window.request_redraw();
                                                }
                                                return;
                                            }
                                        }
                                    }
                                    if str.is_ascii() {
                                        for ch in str.chars() {
                                            #[cfg(feature = "ui")]
                                            if self.ui.key_up(Some(ch), None, &mut ctx.ctx) {
                                                ctx.window.request_redraw();
                                            }
                                            if self.app.key_up(Some(ch), None, &mut ctx.ctx) {
                                                ctx.window.request_redraw();
                                            }
                                        }
                                    }
                                    None
                                }
                                _ => None,
                            };
                            if key.is_some() {
                                #[cfg(feature = "ui")]
                                if self.ui.key_up(None, key.clone(), &mut ctx.ctx) {
                                    ctx.window.request_redraw();
                                }
                                if self.app.key_up(None, key, &mut ctx.ctx) {
                                    ctx.window.request_redraw();
                                }
                            }

                            // Update cursor icon after keyboard events (focus may have changed)
                            if ctx.ctx.cursor_changed() {
                                let cursor_icon = match ctx.ctx.cursor_icon() {
                                    TheCursorIcon::Default => winit::window::CursorIcon::Default,
                                    TheCursorIcon::Crosshair => {
                                        winit::window::CursorIcon::Crosshair
                                    }
                                    TheCursorIcon::Hand => winit::window::CursorIcon::Pointer,
                                    TheCursorIcon::Arrow => winit::window::CursorIcon::Default,
                                    TheCursorIcon::Text => winit::window::CursorIcon::Text,
                                    TheCursorIcon::Wait => winit::window::CursorIcon::Wait,
                                    TheCursorIcon::Help => winit::window::CursorIcon::Help,
                                    TheCursorIcon::Progress => winit::window::CursorIcon::Progress,
                                    TheCursorIcon::NotAllowed => {
                                        winit::window::CursorIcon::NotAllowed
                                    }
                                    TheCursorIcon::ContextMenu => {
                                        winit::window::CursorIcon::ContextMenu
                                    }
                                    TheCursorIcon::Cell => winit::window::CursorIcon::Cell,
                                    TheCursorIcon::VerticalText => {
                                        winit::window::CursorIcon::VerticalText
                                    }
                                    TheCursorIcon::Alias => winit::window::CursorIcon::Alias,
                                    TheCursorIcon::Copy => winit::window::CursorIcon::Copy,
                                    TheCursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
                                    TheCursorIcon::Grab => winit::window::CursorIcon::Grab,
                                    TheCursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
                                    TheCursorIcon::AllScroll => {
                                        winit::window::CursorIcon::AllScroll
                                    }
                                    TheCursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
                                    TheCursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
                                    TheCursorIcon::EResize => winit::window::CursorIcon::EResize,
                                    TheCursorIcon::NResize => winit::window::CursorIcon::NResize,
                                    TheCursorIcon::NEResize => winit::window::CursorIcon::NeResize,
                                    TheCursorIcon::NWResize => winit::window::CursorIcon::NwResize,
                                    TheCursorIcon::SResize => winit::window::CursorIcon::SResize,
                                    TheCursorIcon::SEResize => winit::window::CursorIcon::SeResize,
                                    TheCursorIcon::SWResize => winit::window::CursorIcon::SwResize,
                                    TheCursorIcon::WResize => winit::window::CursorIcon::WResize,
                                    TheCursorIcon::EWResize => winit::window::CursorIcon::EwResize,
                                    TheCursorIcon::NSResize => winit::window::CursorIcon::NsResize,
                                    TheCursorIcon::NESWResize => {
                                        winit::window::CursorIcon::NeswResize
                                    }
                                    TheCursorIcon::NWSEResize => {
                                        winit::window::CursorIcon::NwseResize
                                    }
                                    TheCursorIcon::ColResize => {
                                        winit::window::CursorIcon::ColResize
                                    }
                                    TheCursorIcon::RowResize => {
                                        winit::window::CursorIcon::RowResize
                                    }
                                };
                                ctx.window.set_cursor(cursor_icon);
                                ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                                ctx.ctx.reset_cursor_changed();
                            }
                            if ctx.ctx.cursor_visible_changed() {
                                ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                                ctx.ctx.reset_cursor_visible_changed();
                            }
                        }
                    }
                    WindowEvent::ModifiersChanged(modifiers) => {
                        let state = modifiers.state();
                        // keep a copy of current modifiers for accelerator checks
                        self.mods = state;

                        #[cfg(feature = "ui")]
                        if self.ui.modifier_changed(
                            state.shift_key(),
                            state.control_key(),
                            state.alt_key(),
                            state.super_key(),
                            &mut ctx.ctx,
                        ) {
                            ctx.window.request_redraw();
                        }
                        if self.app.modifier_changed(
                            state.shift_key(),
                            state.control_key(),
                            state.alt_key(),
                            state.super_key(),
                        ) {
                            ctx.window.request_redraw();
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let (x, y) = translate_coord_to_local(
                            position.x as f32,
                            position.y as f32,
                            ctx.ctx.scale_factor,
                        );

                        self.last_cursor_pos = Some((x, y));

                        let mut redraw = false;
                        if self.left_mouse_down {
                            #[cfg(feature = "ui")]
                            if self.ui.touch_dragged(x, y, &mut ctx.ctx) {
                                redraw = true;
                            }

                            if self.app.touch_dragged(x, y, &mut ctx.ctx) {
                                redraw = true;
                            }
                        } else {
                            #[cfg(feature = "ui")]
                            if self.ui.hover(x, y, &mut ctx.ctx) {
                                redraw = true;
                            }

                            if self.app.hover(x, y, &mut ctx.ctx) {
                                redraw = true;
                            }
                        }

                        // Update cursor icon immediately after hover detection
                        if ctx.ctx.cursor_changed() {
                            let cursor_icon = match ctx.ctx.cursor_icon() {
                                TheCursorIcon::Default => winit::window::CursorIcon::Default,
                                TheCursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
                                TheCursorIcon::Hand => winit::window::CursorIcon::Pointer,
                                TheCursorIcon::Arrow => winit::window::CursorIcon::Default,
                                TheCursorIcon::Text => winit::window::CursorIcon::Text,
                                TheCursorIcon::Wait => winit::window::CursorIcon::Wait,
                                TheCursorIcon::Help => winit::window::CursorIcon::Help,
                                TheCursorIcon::Progress => winit::window::CursorIcon::Progress,
                                TheCursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
                                TheCursorIcon::ContextMenu => {
                                    winit::window::CursorIcon::ContextMenu
                                }
                                TheCursorIcon::Cell => winit::window::CursorIcon::Cell,
                                TheCursorIcon::VerticalText => {
                                    winit::window::CursorIcon::VerticalText
                                }
                                TheCursorIcon::Alias => winit::window::CursorIcon::Alias,
                                TheCursorIcon::Copy => winit::window::CursorIcon::Copy,
                                TheCursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
                                TheCursorIcon::Grab => winit::window::CursorIcon::Grab,
                                TheCursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
                                TheCursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
                                TheCursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
                                TheCursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
                                TheCursorIcon::EResize => winit::window::CursorIcon::EResize,
                                TheCursorIcon::NResize => winit::window::CursorIcon::NResize,
                                TheCursorIcon::NEResize => winit::window::CursorIcon::NeResize,
                                TheCursorIcon::NWResize => winit::window::CursorIcon::NwResize,
                                TheCursorIcon::SResize => winit::window::CursorIcon::SResize,
                                TheCursorIcon::SEResize => winit::window::CursorIcon::SeResize,
                                TheCursorIcon::SWResize => winit::window::CursorIcon::SwResize,
                                TheCursorIcon::WResize => winit::window::CursorIcon::WResize,
                                TheCursorIcon::EWResize => winit::window::CursorIcon::EwResize,
                                TheCursorIcon::NSResize => winit::window::CursorIcon::NsResize,
                                TheCursorIcon::NESWResize => winit::window::CursorIcon::NeswResize,
                                TheCursorIcon::NWSEResize => winit::window::CursorIcon::NwseResize,
                                TheCursorIcon::ColResize => winit::window::CursorIcon::ColResize,
                                TheCursorIcon::RowResize => winit::window::CursorIcon::RowResize,
                            };
                            ctx.window.set_cursor(cursor_icon);
                            ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                            ctx.ctx.reset_cursor_changed();
                        }
                        if ctx.ctx.cursor_visible_changed() {
                            ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                            ctx.ctx.reset_cursor_visible_changed();
                        }

                        if redraw {
                            ctx.window.request_redraw();
                        }
                    }
                    WindowEvent::Touch(Touch {
                        phase, location, ..
                    }) => {
                        let (x, y) = translate_coord_to_local(
                            location.x as f32,
                            location.y as f32,
                            ctx.ctx.scale_factor,
                        );

                        match phase {
                            TouchPhase::Started => {
                                let mut redraw = false;
                                #[cfg(feature = "ui")]
                                {
                                    if self.ui.touch_down(x as f32, y as f32, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                if self.app.touch_down(x as f32, y as f32, &mut ctx.ctx) {
                                    redraw = true;
                                }

                                if redraw {
                                    ctx.window.request_redraw();
                                }
                            }
                            TouchPhase::Moved => {
                                let mut redraw = false;
                                #[cfg(feature = "ui")]
                                {
                                    if self.ui.touch_dragged(x as f32, y as f32, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                if self.app.touch_dragged(x as f32, y as f32, &mut ctx.ctx) {
                                    redraw = true;
                                }
                                if redraw {
                                    ctx.window.request_redraw();
                                }
                            }
                            TouchPhase::Ended | TouchPhase::Cancelled => {
                                let mut redraw = false;
                                #[cfg(feature = "ui")]
                                {
                                    if self.ui.touch_up(x as f32, y as f32, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                if self.app.touch_up(x as f32, y as f32, &mut ctx.ctx) {
                                    redraw = true;
                                }

                                if redraw {
                                    ctx.window.request_redraw();
                                }
                            }
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if let Some((x, y)) = self.last_cursor_pos {
                            let mut redraw = false;
                            match (button, state) {
                                (MouseButton::Left, ElementState::Pressed) => {
                                    self.left_mouse_down = true;

                                    #[cfg(feature = "ui")]
                                    if self.ui.touch_down(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }

                                    if self.app.touch_down(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                (MouseButton::Left, ElementState::Released) => {
                                    self.left_mouse_down = false;

                                    #[cfg(feature = "ui")]
                                    if self.ui.touch_up(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }

                                    if self.app.touch_up(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                (MouseButton::Right, ElementState::Pressed) => {
                                    #[cfg(feature = "ui")]
                                    if self.ui.context(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }

                                    if self.app.touch_down(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                (MouseButton::Right, ElementState::Released) => {
                                    #[cfg(feature = "ui")]
                                    if self.ui.touch_up(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }

                                    if self.app.touch_up(x, y, &mut ctx.ctx) {
                                        redraw = true;
                                    }
                                }
                                _ => {}
                            }

                            // Update cursor icon after mouse input events that may change focus
                            if ctx.ctx.cursor_changed() {
                                let cursor_icon = match ctx.ctx.cursor_icon() {
                                    TheCursorIcon::Default => winit::window::CursorIcon::Default,
                                    TheCursorIcon::Crosshair => {
                                        winit::window::CursorIcon::Crosshair
                                    }
                                    TheCursorIcon::Hand => winit::window::CursorIcon::Pointer,
                                    TheCursorIcon::Arrow => winit::window::CursorIcon::Default,
                                    TheCursorIcon::Text => winit::window::CursorIcon::Text,
                                    TheCursorIcon::Wait => winit::window::CursorIcon::Wait,
                                    TheCursorIcon::Help => winit::window::CursorIcon::Help,
                                    TheCursorIcon::Progress => winit::window::CursorIcon::Progress,
                                    TheCursorIcon::NotAllowed => {
                                        winit::window::CursorIcon::NotAllowed
                                    }
                                    TheCursorIcon::ContextMenu => {
                                        winit::window::CursorIcon::ContextMenu
                                    }
                                    TheCursorIcon::Cell => winit::window::CursorIcon::Cell,
                                    TheCursorIcon::VerticalText => {
                                        winit::window::CursorIcon::VerticalText
                                    }
                                    TheCursorIcon::Alias => winit::window::CursorIcon::Alias,
                                    TheCursorIcon::Copy => winit::window::CursorIcon::Copy,
                                    TheCursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
                                    TheCursorIcon::Grab => winit::window::CursorIcon::Grab,
                                    TheCursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
                                    TheCursorIcon::AllScroll => {
                                        winit::window::CursorIcon::AllScroll
                                    }
                                    TheCursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
                                    TheCursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
                                    TheCursorIcon::EResize => winit::window::CursorIcon::EResize,
                                    TheCursorIcon::NResize => winit::window::CursorIcon::NResize,
                                    TheCursorIcon::NEResize => winit::window::CursorIcon::NeResize,
                                    TheCursorIcon::NWResize => winit::window::CursorIcon::NwResize,
                                    TheCursorIcon::SResize => winit::window::CursorIcon::SResize,
                                    TheCursorIcon::SEResize => winit::window::CursorIcon::SeResize,
                                    TheCursorIcon::SWResize => winit::window::CursorIcon::SwResize,
                                    TheCursorIcon::WResize => winit::window::CursorIcon::WResize,
                                    TheCursorIcon::EWResize => winit::window::CursorIcon::EwResize,
                                    TheCursorIcon::NSResize => winit::window::CursorIcon::NsResize,
                                    TheCursorIcon::NESWResize => {
                                        winit::window::CursorIcon::NeswResize
                                    }
                                    TheCursorIcon::NWSEResize => {
                                        winit::window::CursorIcon::NwseResize
                                    }
                                    TheCursorIcon::ColResize => {
                                        winit::window::CursorIcon::ColResize
                                    }
                                    TheCursorIcon::RowResize => {
                                        winit::window::CursorIcon::RowResize
                                    }
                                };
                                ctx.window.set_cursor(cursor_icon);
                                ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                                ctx.ctx.reset_cursor_changed();
                            }
                            if ctx.ctx.cursor_visible_changed() {
                                ctx.window.set_cursor_visible(ctx.ctx.cursor_visible());
                                ctx.ctx.reset_cursor_visible_changed();
                            }

                            if redraw {
                                ctx.window.request_redraw();
                            }
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let (x, y) = match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                const LINE_HEIGHT_PX: f32 = 20.0;
                                (x as f32 * LINE_HEIGHT_PX, y as f32 * LINE_HEIGHT_PX)
                            }
                            MouseScrollDelta::PixelDelta(delta) => (delta.x as f32, delta.y as f32),
                        };

                        let mut redraw = false;
                        #[cfg(feature = "ui")]
                        if self.ui.mouse_wheel((x as i32, y as i32), &mut ctx.ctx) {
                            redraw = true;
                        }

                        if self.app.mouse_wheel((x as isize, y as isize), &mut ctx.ctx) {
                            redraw = true;
                        }

                        if redraw {
                            ctx.window.request_redraw();
                        }
                    }
                    WindowEvent::DroppedFile(path) => {
                        self.app.dropped_file(path.to_string_lossy().into_owned());
                        ctx.window.request_redraw();
                    }
                    _ => {}
                }
            }
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, _: DeviceEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let should_redraw = now >= self.next_frame_time;
        if should_redraw {
            if let Some(ctx) = &self.ctx {
                ctx.window.request_redraw();
            }
            self.next_frame_time = now + self.target_frame_time;
        }

        // #[cfg(target_arch = "wasm32")]
        // {
        //     // Avoid WaitUntil on wasm to sidestep duration underflow; simple Wait keeps CPU low.
        //     event_loop.set_control_flow(ControlFlow::Wait);
        // }

        // #[cfg(not(target_arch = "wasm32"))]
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_time));

        let Some(ctx) = &mut self.ctx else {
            return;
        };

        // Check for changes and update window document modified state
        let current_has_changes = self.app.has_changes();
        if current_has_changes != self.has_changes {
            self.has_changes = current_has_changes;
            #[cfg(target_os = "macos")]
            ctx.window.set_document_edited(current_has_changes);
        }

        #[cfg(feature = "ui")]
        if self.ui.update(&mut ctx.ctx) {
            ctx.window.request_redraw();
        }

        #[cfg(feature = "ui")]
        // Test if the app needs an update
        if self.app.update_ui(&mut self.ui, &mut ctx.ctx) {
            ctx.window.request_redraw();
        }

        // Test if the app needs an update
        if self.app.update(&mut ctx.ctx) {
            ctx.window.request_redraw();
        }
    }
}

pub fn run_winit_app(args: Option<Vec<String>>, app: Box<dyn TheTrait>) {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut winit_app = TheWinitApp::new(args, app);

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut winit_app).unwrap();
}
