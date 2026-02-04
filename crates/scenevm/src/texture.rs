use crate::GPUState;

/// CPU/GPU texture wrapper: stores CPU RGBA bytes and optional GPU resources
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub gpu: Option<TextureGPU>,
}

pub struct TextureGPU {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub readback: wgpu::Buffer,
    pub padded_bytes_per_row: u32,
    #[cfg(target_arch = "wasm32")]
    pub map_ready: Option<std::rc::Rc<std::cell::Cell<bool>>>,
}

impl Texture {
    /// Create a CPU-only texture with zero-initialized RGBA8 data
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            data: vec![0u8; len],
            gpu: None,
        }
    }

    /// Create from existing RGBA data (will truncate/extend to fit width*height*4)
    pub fn from_rgba(width: u32, height: u32, mut data: Vec<u8>) -> Self {
        let need = (width as usize) * (height as usize) * 4;
        if data.len() < need {
            data.resize(need, 0);
        }
        if data.len() > need {
            data.truncate(need);
        }
        Self {
            width,
            height,
            data,
            gpu: None,
        }
    }

    /// CPU-side accessors
    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.data
    }
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Ensure GPU texture exists using a raw device (no GPUState needed)
    pub fn ensure_gpu_with(&mut self, device: &wgpu::Device) {
        let need_realloc = match &self.gpu {
            None => true,
            Some(g) => {
                let size = g.texture.size();
                size.width != self.width || size.height != self.height
            }
        };
        if need_realloc {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scenevm-Texture"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let bpp = 4u32;
            let unpadded = self.width * bpp;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
            let padded = ((unpadded + align - 1) / align) * align;
            let readback_size = padded as u64 * self.height as u64;
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scenevm-Texture-readback"),
                size: readback_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.gpu = Some(TextureGPU {
                texture: tex,
                view,
                readback,
                padded_bytes_per_row: padded,
                #[cfg(target_arch = "wasm32")]
                map_ready: None,
            });
        }
    }

    /// Copy CPU pixels into a destination slice (alias for cpu_blit_to_slice)
    pub fn copy_to_slice(&self, dst: &mut [u8], buf_w: u32, buf_h: u32) {
        self.cpu_blit_to_slice(dst, buf_w, buf_h);
    }

    /// Upload CPU RGBA8 data to the GPU using raw handles (no &GPUState borrow needed).
    pub fn upload_to_gpu_with(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.ensure_gpu_with(device);
        let g = self.gpu.as_ref().expect("Texture GPU not allocated");
        let bpp = 4u32;
        let bytes_per_row = self.width * bpp;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &g.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Download the GPU texture into `self.data` using raw handles.
    pub fn download_from_gpu_with(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.ensure_gpu_with(device);
        let g = self.gpu.as_ref().expect("Texture GPU not allocated");

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scenevm-Texture-dl-encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &g.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &g.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(g.padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = g.readback.slice(..);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            loop {
                let _ = device.poll(wgpu::PollType::Poll);
                match rx.try_recv() {
                    Ok(Ok(())) => break,
                    Ok(Err(_)) => break,
                    Err(std::sync::mpsc::TryRecvError::Empty) => continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                }
            }

            let data = slice.get_mapped_range();
            let need = (self.width as usize) * (self.height as usize) * 4;
            if self.data.len() != need {
                self.data.resize(need, 0);
            }
            let unpadded_bpr = (self.width * 4) as usize;
            let padded_bpr = g.padded_bytes_per_row as usize;
            for row in 0..(self.height as usize) {
                let src_off = row * padded_bpr;
                let dst_off = row * unpadded_bpr;
                self.data[dst_off..dst_off + unpadded_bpr]
                    .copy_from_slice(&data[src_off..src_off + unpadded_bpr]);
            }
            drop(data);
            g.readback.unmap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            use std::cell::Cell;
            use std::rc::Rc;
            let ready = Rc::new(Cell::new(false));
            let ready_cb = Rc::clone(&ready);
            slice.map_async(wgpu::MapMode::Read, move |_| {
                ready_cb.set(true);
            });
            if let Some(gpu) = self.gpu.as_mut() {
                gpu.map_ready = Some(ready);
            }
        }
    }

    /// Ensure a GPU texture exists matching current size, creating or resizing as needed.
    fn ensure_gpu(&mut self, gpu: &GPUState) {
        let need_realloc = match &self.gpu {
            None => true,
            Some(g) => {
                let size = g.texture.size();
                size.width != self.width || size.height != self.height
            }
        };
        if need_realloc {
            let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scenevm-Texture"),
                size: wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

            // Create readback buffer sized to padded row alignment for downloads
            let bpp = 4u32;
            let unpadded = self.width * bpp;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
            let padded = ((unpadded + align - 1) / align) * align;
            let readback_size = padded as u64 * self.height as u64;
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scenevm-Texture-readback"),
                size: readback_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            self.gpu = Some(TextureGPU {
                texture: tex,
                view,
                readback,
                padded_bytes_per_row: padded,
                #[cfg(target_arch = "wasm32")]
                map_ready: None,
            });
        }
    }

    /// Upload CPU RGBA8 data to the GPU texture (creates it if needed).
    pub fn upload_to_gpu(&mut self, gpu: &GPUState) {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        self.upload_to_gpu_with(&device, &queue);
    }

    /// Download the GPU texture into `self.data`. Blocks until the copy completes.
    pub fn download_from_gpu(&mut self, gpu: &GPUState) {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        self.download_from_gpu_with(&device, &queue);
    }

    /// Try to finish an in-flight download started on WASM. Returns true when completed.
    pub fn try_finish_download_from_gpu(&mut self) -> bool {
        if self.gpu.is_none() {
            return false;
        }
        let gref = self.gpu.as_ref().unwrap();
        let slice = gref.readback.slice(..);

        #[cfg(target_arch = "wasm32")]
        {
            // If not marked as ready yet, the map isn't complete.
            let Some(flag) = &gref.map_ready else {
                return false;
            };
            if !flag.get() {
                return false;
            }
        }

        // Now safe to read; mapping is complete
        let data = slice.get_mapped_range();
        let need = (self.width as usize) * (self.height as usize) * 4;
        if self.data.len() != need {
            self.data.resize(need, 0);
        }
        let unpadded_bpr = (self.width * 4) as usize;
        let padded_bpr = gref.padded_bytes_per_row as usize;
        for row in 0..(self.height as usize) {
            let src_off = row * padded_bpr;
            let dst_off = row * unpadded_bpr;
            self.data[dst_off..dst_off + unpadded_bpr]
                .copy_from_slice(&data[src_off..src_off + unpadded_bpr]);
        }
        drop(data);
        // Unmap and clear the ready flag (if any)
        if let Some(g) = self.gpu.as_mut() {
            g.readback.unmap();
            #[cfg(target_arch = "wasm32")]
            {
                g.map_ready = None;
            }
        }
        true
    }

    /// Blit this texture to the current GPU storage texture used by SceneVM (copy region = min size).
    pub fn gpu_blit_to_storage(&mut self, gpu: &GPUState, dest: &wgpu::Texture) {
        self.ensure_gpu(gpu);
        // Upload latest CPU data first (no-op if unchanged)
        self.upload_to_gpu(gpu);
        let g = self.gpu.as_ref().unwrap();
        let w = self.width.min(dest.size().width);
        let h = self.height.min(dest.size().height);
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scenevm-Texture-blit-encoder"),
            });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &g.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: dest,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Copy CPU data into a destination pixel slice described by (buf_w, buf_h).
    /// This will NOT resize the destination; it copies only the overlapping region line-by-line.
    pub fn cpu_blit_to_slice(&self, dst: &mut [u8], buf_w: u32, buf_h: u32) {
        // Fast path: identical dimensions and lengths — single copy
        let expected_dst_len = (buf_w as usize) * (buf_h as usize) * 4;
        let expected_src_len = (self.width as usize) * (self.height as usize) * 4;
        if self.width == buf_w
            && self.height == buf_h
            && dst.len() == expected_dst_len
            && self.data.len() == expected_src_len
        {
            dst.copy_from_slice(&self.data);
            return;
        }

        // Safe path: copy only the overlapping region line-by-line
        let copy_w = self.width.min(buf_w);
        let copy_h = self.height.min(buf_h);
        let src_stride = (self.width * 4) as usize;
        let dst_stride = (buf_w * 4) as usize;
        let row_bytes = (copy_w * 4) as usize;

        for row in 0..(copy_h as usize) {
            let s_off = row * src_stride;
            let d_off = row * dst_stride;
            let s_end = s_off + row_bytes;
            let d_end = d_off + row_bytes;
            if s_end <= self.data.len() && d_end <= dst.len() {
                dst[d_off..d_end].copy_from_slice(&self.data[s_off..s_end]);
            } else {
                break; // Out-of-bounds safety guard
            }
        }
    }
}
