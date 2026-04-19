use std::sync::Arc;
#[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
use std::num::NonZeroU32;

use winit::{
    dpi::PhysicalSize,
    window::Window,
};

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    not(target_arch = "wasm32")
))]
use pixels::{Pixels, SurfaceTexture};
#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    not(target_arch = "wasm32")
))]
use web_time::Instant;
#[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
use softbuffer::Surface;

#[cfg(not(any(feature = "winit_app_pixels", feature = "winit_app_softbuffer")))]
compile_error!(
    "theframework: `run_winit_app` requires either `winit_app`/`winit_app_pixels` or `winit_app_softbuffer`."
);

#[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
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

#[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
pub(crate) struct SoftbufferBackend {
    surface: Surface<Arc<Window>, Arc<Window>>,
}

#[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
impl SoftbufferBackend {
    fn new(window: Arc<Window>, scale_factor: f32) -> Self {
        #[cfg(target_arch = "wasm32")]
        let _ = scale_factor;
        let size = window.inner_size();

        let context = softbuffer::Context::new(window.clone()).unwrap();
        let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        let (surface_width, surface_height) = {
            #[cfg(target_os = "macos")]
            let surface_scale = scale_factor;
            #[cfg(all(not(target_os = "macos"), not(target_arch = "wasm32")))]
            let surface_scale = 1.0;
            #[cfg(target_arch = "wasm32")]
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
        #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
        let _ = (window, scale_factor);
        #[cfg(target_os = "macos")]
        let blit_scale_factor = scale_factor;
        #[cfg(target_arch = "wasm32")]
        let blit_scale_factor = 1.0;
        #[cfg(all(not(target_os = "macos"), not(target_arch = "wasm32")))]
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
        blit_rgba_into_softbuffer(
            ui_frame,
            blit_scale_factor,
            width,
            height,
            &mut *buffer,
        );
        buffer.present().unwrap();
    }

    fn resize(
        &mut self,
        size: PhysicalSize<u32>,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) {
        #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
        let _ = (size, width, height, scale_factor);
        #[cfg(all(not(target_os = "macos"), not(target_arch = "wasm32")))]
        let _ = (width, height, scale_factor);
        #[cfg(all(not(target_os = "macos"), not(target_arch = "wasm32")))]
        self.surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .unwrap();
        #[cfg(target_arch = "wasm32")]
        self.surface
            .resize(
                NonZeroU32::new(width.max(1)).unwrap(),
                NonZeroU32::new(height.max(1)).unwrap(),
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

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    not(target_arch = "wasm32")
))]
pub(crate) struct PixelsBackend {
    pixels: Pixels<'static>,
    last_present_failed_at: Option<Instant>,
}

#[cfg(all(
    feature = "winit_app_pixels",
    not(feature = "winit_app_softbuffer"),
    not(target_arch = "wasm32")
))]
impl PixelsBackend {
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

    fn resize(&mut self, size: PhysicalSize<u32>, width: u32, height: u32) {
        self.pixels.resize_buffer(width.max(1), height.max(1)).unwrap();
        self.pixels
            .resize_surface(size.width.max(1), size.height.max(1))
            .unwrap();
    }
}

pub(crate) enum TheWinitBackend {
    #[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
    Softbuffer(SoftbufferBackend),
    #[cfg(all(
        feature = "winit_app_pixels",
        not(feature = "winit_app_softbuffer"),
        not(target_arch = "wasm32")
    ))]
    Pixels(PixelsBackend),
}

impl TheWinitBackend {
    #[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
    pub(crate) fn new(window: Arc<Window>, width: usize, height: usize, scale_factor: f32) -> Self {
        let _ = (width, height);
        Self::Softbuffer(SoftbufferBackend::new(window, scale_factor))
    }

    #[cfg(all(
        feature = "winit_app_pixels",
        not(feature = "winit_app_softbuffer"),
        not(target_arch = "wasm32")
    ))]
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
        #[cfg(all(
            feature = "winit_app_pixels",
            not(feature = "winit_app_softbuffer"),
            not(target_arch = "wasm32")
        ))]
        let _ = (window, width, height, scale_factor);

        match self {
            #[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
            Self::Softbuffer(backend) => {
                backend.present(window, ui_frame, width, height, scale_factor);
            }
            #[cfg(all(
                feature = "winit_app_pixels",
                not(feature = "winit_app_softbuffer"),
                not(target_arch = "wasm32")
            ))]
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
        #[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
        let _ = window;
        #[cfg(all(
            feature = "winit_app_pixels",
            not(feature = "winit_app_softbuffer"),
            not(target_arch = "wasm32")
        ))]
        let _ = (window, scale_factor);

        match self {
            #[cfg(any(feature = "winit_app_softbuffer", target_arch = "wasm32"))]
            Self::Softbuffer(backend) => backend.resize(size, width, height, scale_factor),
            #[cfg(all(
                feature = "winit_app_pixels",
                not(feature = "winit_app_softbuffer"),
                not(target_arch = "wasm32")
            ))]
            Self::Pixels(backend) => backend.resize(size, width, height),
        }
    }
}
