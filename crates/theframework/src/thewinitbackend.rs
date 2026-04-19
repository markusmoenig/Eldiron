#[cfg(feature = "winit_app_softbuffer")]
use std::num::NonZeroU32;
use std::sync::Arc;
#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
use std::{cell::RefCell, rc::Rc};

use winit::{dpi::PhysicalSize, window::Window};

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
use js_sys::{Function, Reflect};
#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
use pixels::PixelsBuilder;
#[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
use pixels::{Pixels, SurfaceTexture};
#[cfg(feature = "winit_app_softbuffer")]
use softbuffer::Surface;
#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
use wasm_bindgen_futures::spawn_local;
#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    not(target_arch = "wasm32")
))]
use web_time::Instant;

#[cfg(not(any(feature = "winit_app_pixels", feature = "winit_app_softbuffer")))]
compile_error!(
    "theframework: `run_winit_app` requires either `winit_app`/`winit_app_pixels` or `winit_app_softbuffer`."
);

#[cfg(feature = "winit_app_softbuffer")]
fn blit_rgba_into_softbuffer(
    ui_frame: &[u8],
    scale_factor: f32,
    width: usize,
    height: usize,
    dest: &mut [u32],
) {
    let dest_width = (width as f32 * scale_factor).round() as usize;
    let dest_height = (height as f32 * scale_factor).round() as usize;

    if scale_factor == 1.0 {
        for (dst, rgba) in dest.iter_mut().zip(ui_frame.chunks_exact(4)) {
            *dst = (rgba[2] as u32) | ((rgba[1] as u32) << 8) | ((rgba[0] as u32) << 16);
        }
    } else {
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

#[cfg(feature = "winit_app_softbuffer")]
pub(crate) struct SoftbufferBackend {
    surface: Surface<Arc<Window>, Arc<Window>>,
}

#[cfg(feature = "winit_app_softbuffer")]
impl SoftbufferBackend {
    fn new(window: Arc<Window>, scale_factor: f32) -> Self {
        let size = window.inner_size();

        let context = softbuffer::Context::new(window.clone()).unwrap();
        let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

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

        if let (Some(width), Some(height)) = (
            NonZeroU32::new(surface_width),
            NonZeroU32::new(surface_height),
        ) {
            surface.resize(width, height).unwrap();
        }

        Self { surface }
    }

    fn present(
        &mut self,
        window: &Arc<Window>,
        ui_frame: &[u8],
        width: usize,
        height: usize,
        scale_factor: f32,
    ) {
        #[cfg(target_os = "macos")]
        let _ = (window, scale_factor);
        #[cfg(target_os = "macos")]
        let blit_scale_factor = scale_factor;
        #[cfg(not(target_os = "macos"))]
        let blit_scale_factor = {
            let buffer = self.surface.buffer_mut().unwrap();
            let inner_size = window.inner_size();
            let desired_scale = inner_size.width as f32 / width as f32;

            let dest_width = inner_size.width as usize;
            let dest_height = inner_size.height as usize;
            let required_size = dest_width * dest_height;

            if buffer.len() >= required_size {
                desired_scale
            } else {
                println!(
                    "Warning: Buffer too small for scale_factor {}. Required: {}, Available: {}. Falling back to scale_factor = 1.0",
                    desired_scale,
                    required_size,
                    buffer.len()
                );
                1.0
            }
        };

        let mut buffer = self.surface.buffer_mut().unwrap();
        blit_rgba_into_softbuffer(ui_frame, blit_scale_factor, width, height, &mut *buffer);
        buffer.present().unwrap();
    }

    fn resize(&mut self, size: PhysicalSize<u32>, width: u32, height: u32, scale_factor: f32) {
        #[cfg(target_os = "macos")]
        let _ = (size, width, height, scale_factor);
        #[cfg(not(target_os = "macos"))]
        let _ = (width, height, scale_factor);
        #[cfg(not(target_os = "macos"))]
        self.surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .unwrap();
        #[cfg(target_os = "macos")]
        self.surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .unwrap();
    }
}

#[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
pub(crate) struct PixelsBackend {
    #[cfg(target_arch = "wasm32")]
    state: Rc<RefCell<PixelsBackendState>>,
    #[cfg(not(target_arch = "wasm32"))]
    pixels: Pixels<'static>,
    #[cfg(not(target_arch = "wasm32"))]
    last_present_failed_at: Option<Instant>,
}

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
enum PixelsBackendState {
    Pending {
        surface_size: PhysicalSize<u32>,
        buffer_size: (u32, u32),
    },
    Ready(Pixels<'static>),
    Failed {
        message: String,
        logged: bool,
    },
}

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
fn preferred_canvas_format() -> pixels::wgpu::TextureFormat {
    let fallback = pixels::wgpu::TextureFormat::Bgra8Unorm;

    let Some(window) = web_sys::window() else {
        return fallback;
    };
    let navigator = window.navigator();
    let Ok(gpu) = Reflect::get(navigator.as_ref(), &JsValue::from_str("gpu")) else {
        return fallback;
    };
    let Ok(getter) = Reflect::get(&gpu, &JsValue::from_str("getPreferredCanvasFormat")) else {
        return fallback;
    };
    let Some(getter) = getter.dyn_into::<Function>().ok() else {
        return fallback;
    };
    let Ok(format) = getter.call0(&gpu) else {
        return fallback;
    };
    let Some(format) = format.as_string() else {
        return fallback;
    };

    match format.as_str() {
        "rgba8unorm" => pixels::wgpu::TextureFormat::Rgba8Unorm,
        "bgra8unorm" => pixels::wgpu::TextureFormat::Bgra8Unorm,
        "rgba8unorm-srgb" => pixels::wgpu::TextureFormat::Rgba8UnormSrgb,
        "bgra8unorm-srgb" => pixels::wgpu::TextureFormat::Bgra8UnormSrgb,
        _ => fallback,
    }
}

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    target_arch = "wasm32"
))]
fn is_srgb_format(format: pixels::wgpu::TextureFormat) -> bool {
    matches!(
        format,
        pixels::wgpu::TextureFormat::Rgba8UnormSrgb | pixels::wgpu::TextureFormat::Bgra8UnormSrgb
    )
}

#[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
impl PixelsBackend {
    #[cfg(target_arch = "wasm32")]
    fn new(window: Arc<Window>, width: u32, height: u32) -> Self {
        let size = window.inner_size();
        let state = Rc::new(RefCell::new(PixelsBackendState::Pending {
            surface_size: size,
            buffer_size: (width.max(1), height.max(1)),
        }));

        let state_clone = state.clone();
        let window_clone = window.clone();
        spawn_local(async move {
            let surface_texture =
                SurfaceTexture::new(size.width.max(1), size.height.max(1), window_clone.clone());
            let mut builder = PixelsBuilder::new(width.max(1), height.max(1), surface_texture);
            builder = builder.alpha_mode(pixels::wgpu::CompositeAlphaMode::Opaque);
            let format = preferred_canvas_format();
            builder = builder.surface_texture_format(format);
            if !is_srgb_format(format) {
                builder = builder.texture_format(pixels::wgpu::TextureFormat::Rgba8Unorm);
            }
            let result = builder.build_async().await;
            let mut state_ref = state_clone.borrow_mut();
            match result {
                Ok(mut pixels) => {
                    let (surface_size, buffer_size) = match &*state_ref {
                        PixelsBackendState::Pending {
                            surface_size,
                            buffer_size,
                        } => (*surface_size, *buffer_size),
                        _ => (size, (width.max(1), height.max(1))),
                    };
                    let _ = pixels.resize_buffer(buffer_size.0, buffer_size.1);
                    let _ = pixels
                        .resize_surface(surface_size.width.max(1), surface_size.height.max(1));
                    *state_ref = PixelsBackendState::Ready(pixels);
                }
                Err(err) => {
                    *state_ref = PixelsBackendState::Failed {
                        message: err.to_string(),
                        logged: false,
                    };
                }
            }
            window_clone.request_redraw();
        });

        Self { state }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn new(window: Arc<Window>, width: u32, height: u32) -> Self {
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width.max(1), size.height.max(1), window);
        let pixels = Pixels::new(width.max(1), height.max(1), surface_texture).unwrap();
        Self {
            pixels,
            last_present_failed_at: None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn present(&mut self, ui_frame: &[u8]) {
        let mut state = self.state.borrow_mut();
        match &mut *state {
            PixelsBackendState::Pending { .. } => {}
            PixelsBackendState::Ready(pixels) => {
                pixels.frame_mut().copy_from_slice(ui_frame);
                if let Err(err) = pixels.render() {
                    web_sys::console::warn_1(
                        &format!("Warning: pixels present failed: {err}").into(),
                    );
                }
            }
            PixelsBackendState::Failed { message, logged } => {
                if !*logged {
                    web_sys::console::warn_1(
                        &format!("Warning: pixels init failed: {message}").into(),
                    );
                    *logged = true;
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn present(&mut self, ui_frame: &[u8]) {
        self.pixels.frame_mut().copy_from_slice(ui_frame);
        if let Err(err) = self.pixels.render() {
            let now = Instant::now();
            let should_log = self
                .last_present_failed_at
                .is_none_or(|last| now.duration_since(last).as_secs() >= 1);
            if should_log {
                println!("Warning: pixels present failed: {err}");
                self.last_present_failed_at = Some(now);
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn resize(&mut self, size: PhysicalSize<u32>, width: u32, height: u32) {
        let mut state = self.state.borrow_mut();
        match &mut *state {
            PixelsBackendState::Pending {
                surface_size,
                buffer_size,
            } => {
                *surface_size = size;
                *buffer_size = (width.max(1), height.max(1));
            }
            PixelsBackendState::Ready(pixels) => {
                let _ = pixels.resize_buffer(width.max(1), height.max(1));
                let _ = pixels.resize_surface(size.width.max(1), size.height.max(1));
            }
            PixelsBackendState::Failed { .. } => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resize(&mut self, size: PhysicalSize<u32>, width: u32, height: u32) {
        self.pixels
            .resize_buffer(width.max(1), height.max(1))
            .unwrap();
        self.pixels
            .resize_surface(size.width.max(1), size.height.max(1))
            .unwrap();
    }
}

pub(crate) enum TheWinitBackend {
    #[cfg(feature = "winit_app_softbuffer")]
    Softbuffer(SoftbufferBackend),
    #[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
    Pixels(PixelsBackend),
}

impl TheWinitBackend {
    #[cfg(feature = "winit_app_softbuffer")]
    pub(crate) fn new(window: Arc<Window>, width: usize, height: usize, scale_factor: f32) -> Self {
        let _ = (width, height);
        Self::Softbuffer(SoftbufferBackend::new(window, scale_factor))
    }

    #[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
    pub(crate) fn new(window: Arc<Window>, width: usize, height: usize, scale_factor: f32) -> Self {
        let _ = scale_factor;
        Self::Pixels(PixelsBackend::new(window, width as u32, height as u32))
    }

    pub(crate) fn present(
        &mut self,
        window: &Arc<Window>,
        ui_frame: &[u8],
        width: usize,
        height: usize,
        scale_factor: f32,
    ) {
        #[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
        let _ = (window, width, height, scale_factor);

        match self {
            #[cfg(feature = "winit_app_softbuffer")]
            Self::Softbuffer(backend) => {
                backend.present(window, ui_frame, width, height, scale_factor);
            }
            #[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
            Self::Pixels(backend) => backend.present(ui_frame),
        }
    }

    pub(crate) fn resize(
        &mut self,
        window: &Arc<Window>,
        size: PhysicalSize<u32>,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) {
        #[cfg(feature = "winit_app_softbuffer")]
        let _ = window;
        #[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
        let _ = (window, scale_factor);

        match self {
            #[cfg(feature = "winit_app_softbuffer")]
            Self::Softbuffer(backend) => backend.resize(size, width, height, scale_factor),
            #[cfg(all(feature = "winit_app_pixels", not(feature = "winit_app_softbuffer")))]
            Self::Pixels(backend) => backend.resize(size, width, height),
        }
    }
}
