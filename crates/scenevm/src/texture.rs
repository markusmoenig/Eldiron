#[cfg(feature = "gpu")]
use crate::GPUState;

/// CPU/GPU texture wrapper: stores CPU RGBA bytes and optional GPU resources
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub gpu: Option<TextureGPU>,
}

#[cfg(feature = "gpu")]
pub struct TextureGPU {
    pub device: wgpu::Device,
    pub context_generation: Option<u64>,
    pub(crate) target_lease: Option<renderkit::TargetLease>,
    mip_pipeline: Option<wgpu::ComputePipeline>,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub readback: Option<wgpu::Buffer>,
    pub padded_bytes_per_row: u32,
    #[cfg(target_arch = "wasm32")]
    pub map_ready: Option<std::rc::Rc<std::cell::Cell<bool>>>,
}

#[cfg(not(feature = "gpu"))]
pub struct TextureGPU;

#[cfg(feature = "gpu")]
fn mip_level_count(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height).max(1);
    (32 - max_dim.leading_zeros()).max(1)
}

#[cfg(all(feature = "gpu", test))]
fn downsample_rgba8_box(src: &[u8], src_w: u32, src_h: u32) -> (Vec<u8>, u32, u32) {
    let dst_w = (src_w / 2).max(1);
    let dst_h = (src_h / 2).max(1);
    let mut dst = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];

    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx0 = (x * 2).min(src_w - 1);
            let sy0 = (y * 2).min(src_h - 1);
            let sx1 = (sx0 + 1).min(src_w - 1);
            let sy1 = (sy0 + 1).min(src_h - 1);

            let i00 = ((sy0 * src_w + sx0) * 4) as usize;
            let i10 = ((sy0 * src_w + sx1) * 4) as usize;
            let i01 = ((sy1 * src_w + sx0) * 4) as usize;
            let i11 = ((sy1 * src_w + sx1) * 4) as usize;

            let di = ((y * dst_w + x) * 4) as usize;
            let a0 = src[i00 + 3] as u32;
            let a1 = src[i10 + 3] as u32;
            let a2 = src[i01 + 3] as u32;
            let a3 = src[i11 + 3] as u32;
            let a_sum = a0 + a1 + a2 + a3;

            // Alpha-weighted RGB downsample to avoid dark/black fringes in mip levels.
            if a_sum > 0 {
                let r_sum = (src[i00] as u32) * a0
                    + (src[i10] as u32) * a1
                    + (src[i01] as u32) * a2
                    + (src[i11] as u32) * a3;
                let g_sum = (src[i00 + 1] as u32) * a0
                    + (src[i10 + 1] as u32) * a1
                    + (src[i01 + 1] as u32) * a2
                    + (src[i11 + 1] as u32) * a3;
                let b_sum = (src[i00 + 2] as u32) * a0
                    + (src[i10 + 2] as u32) * a1
                    + (src[i01 + 2] as u32) * a2
                    + (src[i11 + 2] as u32) * a3;
                dst[di] = (r_sum / a_sum) as u8;
                dst[di + 1] = (g_sum / a_sum) as u8;
                dst[di + 2] = (b_sum / a_sum) as u8;
            } else {
                dst[di] = 0;
                dst[di + 1] = 0;
                dst[di + 2] = 0;
            }
            dst[di + 3] = (a_sum / 4) as u8;
        }
    }

    (dst, dst_w, dst_h)
}

#[cfg(feature = "gpu")]
fn mip_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scenevm-texture-mips"),
        source: wgpu::ShaderSource::Wgsl(include_str!("texture_mips.wgsl").into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("scenevm-texture-mips"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    })
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

    /// GPU output storage without an authoritative CPU image.
    pub fn render_target(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: Vec::new(),
            gpu: None,
        }
    }

    /// GPU-only output storage, preserving the legacy mip/storage contract.
    #[cfg(feature = "gpu")]
    pub(crate) fn pooled_render_target(context: &renderkit::GpuContext, width: u32, height: u32) -> renderkit::Result<Self> {
        let levels = mip_level_count(width, height);
        let mut descriptor = renderkit::TargetDescriptor::color(width, height, wgpu::TextureFormat::Rgba8Unorm);
        descriptor.mip_levels = levels;
        descriptor.usage |= wgpu::TextureUsages::STORAGE_BINDING;
        let lease = context.acquire_target(descriptor)?;
        let texture = lease.texture().clone();
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("scenevm-pooled-target-mip0"), base_mip_level: 0,
            mip_level_count: Some(1), ..Default::default()
        });
        // New scene targets historically expose transparent storage, including
        // their unused lower mips. Reused storage must restore that contract
        // before image() can publish it, even before the first scene render.
        let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scenevm target initialization"),
        });
        for mip in 0..levels {
            let mip_view = texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: mip, mip_level_count: Some(1), ..Default::default()
            });
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &mip_view, resolve_target: None, depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })], ..Default::default()
            });
        }
        context.queue.submit([encoder.finish()]);
        Ok(Self { width, height, data: Vec::new(), gpu: Some(TextureGPU {
            device: context.device.clone(), context_generation: Some(context.generation()),
            target_lease: Some(lease), mip_pipeline: None, texture, view, readback: None,
            padded_bytes_per_row: (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
            #[cfg(target_arch = "wasm32")]
            map_ready: None,
        }) })
    }

    #[cfg(feature = "gpu")]
    fn ensure_readback(&mut self, device: &wgpu::Device) {
        let gpu = self.gpu.as_mut().expect("GPU texture initialized");
        if gpu.readback.is_none() {
            gpu.readback = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scenevm-requested-readback"),
                size: gpu.padded_bytes_per_row as u64 * self.height as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
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
    #[cfg(feature = "gpu")]
    pub fn ensure_gpu_with(&mut self, device: &wgpu::Device) {
        let need_realloc = match &self.gpu {
            None => true,
            Some(g) => {
                let size = g.texture.size();
                g.device != *device || size.width != self.width || size.height != self.height
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
                mip_level_count: mip_level_count(self.width, self.height),
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
            let view = tex.create_view(&wgpu::TextureViewDescriptor {
                label: Some("scenevm-Texture-view-mip0"),
                format: None,
                dimension: None,
                usage: None,
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: Some(1),
                base_array_layer: 0,
                array_layer_count: Some(1),
            });
            let bpp = 4u32;
            let unpadded = self.width * bpp;
            let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
            let padded = ((unpadded + align - 1) / align) * align;
            let readback = None;
            self.gpu = Some(TextureGPU {
                device: device.clone(),
                context_generation: None,
                target_lease: None,
                mip_pipeline: None,
                texture: tex,
                view,
                readback,
                padded_bytes_per_row: padded,
                #[cfg(target_arch = "wasm32")]
                map_ready: None,
            });
        }
    }

    /// Explicit device domain for application-owned renderers. WGPU device
    /// equality alone is insufficient across separate WGPU instances.
    #[cfg(feature = "gpu")]
    pub fn ensure_gpu_in_domain(&mut self, device: &wgpu::Device, generation: u64) {
        if self
            .gpu
            .as_ref()
            .is_some_and(|g| g.context_generation != Some(generation))
        {
            self.gpu = None;
        }
        self.ensure_gpu_with(device);
        self.gpu.as_mut().unwrap().context_generation = Some(generation);
    }

    /// Copy CPU pixels into a destination slice (alias for cpu_blit_to_slice)
    pub fn copy_to_slice(&self, dst: &mut [u8], buf_w: u32, buf_h: u32) {
        self.cpu_blit_to_slice(dst, buf_w, buf_h);
    }

    /// Upload CPU RGBA8 data to the GPU using raw handles (no &GPUState borrow needed).
    #[cfg(feature = "gpu")]
    pub fn upload_to_gpu_with(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.ensure_gpu_with(device);
        let g = self.gpu.as_mut().expect("Texture GPU not allocated");
        // Upload mip 0 once. Generating the rest on the CPU cloned and filtered
        // both large atlases on the UI thread whenever a camera needed them.
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
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        let levels = mip_level_count(self.width, self.height);
        if levels <= 1 {
            return;
        }
        let pipeline = g.mip_pipeline.get_or_insert_with(|| mip_pipeline(device));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scenevm-texture-mips"),
        });
        let layout = pipeline.get_bind_group_layout(0);
        let mip_view = |mip| {
            g.texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        for mip in 1..levels {
            let src = mip_view(mip - 1);
            let dst = mip_view(mip);
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scenevm-texture-mip-level"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&dst),
                    },
                ],
            });
            // Each pass provides the dependency between adjacent mip levels.
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                (self.width >> mip).max(1).div_ceil(8),
                (self.height >> mip).max(1).div_ceil(8),
                1,
            );
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Download the GPU texture into `self.data` using raw handles.
    #[cfg(feature = "gpu")]
    pub fn download_from_gpu_with(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.ensure_gpu_with(device);
        self.ensure_readback(device);
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
                buffer: g.readback.as_ref().expect("readback requested"),
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
        let _readback_submission = queue.submit(std::iter::once(encoder.finish()));

        let slice = g.readback.as_ref().expect("readback requested").slice(..);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            // Sleep on this frame's GPU fence instead of spinning on the UI
            // thread until map_async completes. Poll also delivers the callback.
            if device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(_readback_submission),
                    timeout: None,
                })
                .is_err()
                || !matches!(rx.recv(), Ok(Ok(())))
            {
                return;
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
            if let Some(readback) = &g.readback {
                readback.unmap();
            }
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
    #[cfg(feature = "gpu")]
    fn ensure_gpu(&mut self, gpu: &GPUState) {
        self.ensure_gpu_with(&gpu.device);
    }

    /// Upload CPU RGBA8 data to the GPU texture (creates it if needed).
    #[cfg(feature = "gpu")]
    pub fn upload_to_gpu(&mut self, gpu: &GPUState) {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        self.upload_to_gpu_with(&device, &queue);
    }

    /// Download the GPU texture into `self.data`. Blocks until the copy completes.
    #[cfg(feature = "gpu")]
    pub fn download_from_gpu(&mut self, gpu: &GPUState) {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        self.download_from_gpu_with(&device, &queue);
    }

    /// Try to finish an in-flight download started on WASM. Returns true when completed.
    #[cfg(feature = "gpu")]
    pub fn try_finish_download_from_gpu(&mut self) -> bool {
        if self.gpu.is_none() {
            return false;
        }
        let gref = self.gpu.as_ref().unwrap();
        let Some(readback) = gref.readback.as_ref() else {
            return false;
        };
        let slice = readback.slice(..);

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
            if let Some(readback) = &g.readback {
                readback.unmap();
            }
            #[cfg(target_arch = "wasm32")]
            {
                g.map_ready = None;
            }
        }
        true
    }

    /// Blit this texture to the current GPU storage texture used by SceneVM (copy region = min size).
    #[cfg(feature = "gpu")]
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

#[cfg(all(test, feature = "gpu", not(target_arch = "wasm32")))]
mod mip_tests {
    use super::*;

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn gpu_mips_match_alpha_weighted_cpu_filter_after_repeated_uploads() {
        let scene = crate::SceneVM::new(16, 16);
        let gpu = scene.gpu.as_ref().expect("native GPU");
        for (width, height) in [(1, 1), (1, 17), (17, 1), (15, 9), (64, 32)] {
            let mut texture = Texture::new(width, height);
            for seed in [13usize, 97] {
                for (i, pixel) in texture.data.chunks_exact_mut(4).enumerate() {
                    pixel.copy_from_slice(&[
                        ((i * 31 + seed) % 256) as u8,
                        ((i * 17 + seed * 2) % 256) as u8,
                        ((i * 7 + seed * 3) % 256) as u8,
                        if i % 3 == 0 {
                            0
                        } else {
                            ((i * 43 + seed) % 256) as u8
                        },
                    ]);
                }
                texture.upload_to_gpu_with(&gpu.device, &gpu.queue);
                texture.ensure_readback(&gpu.device);
                let mut expected = texture.data.clone();
                let (mut w, mut h) = (width, height);
                let g = texture.gpu.as_ref().unwrap();
                for mip in 0..mip_level_count(width, height) {
                    let mut encoder = gpu.device.create_command_encoder(&Default::default());
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &g.texture,
                            mip_level: mip,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: g.readback.as_ref().expect("readback requested"),
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(g.padded_bytes_per_row),
                                rows_per_image: Some(h),
                            },
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    let submission = gpu.queue.submit(Some(encoder.finish()));
                    let slice = g.readback.as_ref().expect("readback requested").slice(..);
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);
                    slice.map_async(wgpu::MapMode::Read, move |r| {
                        tx.send(r).unwrap();
                    });
                    gpu.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: Some(submission),
                            timeout: None,
                        })
                        .unwrap();
                    rx.recv().unwrap().unwrap();
                    let bytes = slice.get_mapped_range();
                    let actual = bytes
                        .chunks(g.padded_bytes_per_row as usize)
                        .take(h as usize)
                        .flat_map(|row| row[..w as usize * 4].iter().copied())
                        .collect::<Vec<_>>();
                    assert_eq!(
                        actual, expected,
                        "{width}x{height} mip {mip}, upload seed {seed}"
                    );
                    drop(bytes);
                    if let Some(readback) = &g.readback {
                        readback.unmap();
                    }
                    (expected, w, h) = downsample_rgba8_box(&expected, w, h);
                }
            }
        }
    }
}
