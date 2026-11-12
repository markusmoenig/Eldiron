//! Baked tile source display on the injected device. CPU data is an authored
//! source asset; checkerboard, resampling and byte-compatible alpha are GPU work.
use renderkit::{AlphaMode, ColorSpace, GpuContext, GpuImage, ImageAsset, ImageDescriptor};
use std::sync::Arc;
use wgpu::util::DeviceExt;
const SOURCE_BYTES: usize = 16 * 1024 * 1024;
const OUTPUT_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Clone)]
pub(super) struct Request {
    pub epoch: u64,
    pub revision: u64,
    pub source: Arc<ImageAsset>,
    pub size: [u32; 2],
    pub checker: [[u8; 4]; 2],
}
pub(super) struct Frame {
    pub epoch: u64,
    pub revision: u64,
    pub image: Arc<GpuImage>,
    pub size: [u32; 2],
}
impl Frame {
    pub fn paint(&self, image: renderkit::ImageHandle) -> theframework::paint::PaintList {
        let mut list = theframework::paint::PaintList::default();
        list.image(
            image,
            theframework::paint::Rect::new(0., 0., self.size[0] as f32, self.size[1] as f32),
            theframework::paint::Sampling::Nearest,
        );
        list
    }
}
pub(super) struct Preview {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    source: Option<(Arc<ImageAsset>, Arc<GpuImage>)>,
    targets: Vec<Arc<GpuImage>>,
}
impl Preview {
    pub fn new(context: Arc<GpuContext>) -> Self {
        let device = &context.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("baked geometry source"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("baked geometry source"),
            source: wgpu::ShaderSource::Wgsl(include_str!("baked_preview.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("baked geometry source"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("baked geometry source"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            context,
            layout,
            pipeline,
            source: None,
            targets: Vec::new(),
        }
    }
    pub fn generation(&self) -> u64 {
        self.context.generation()
    }
    pub fn source_valid(source: &rusterix::Texture) -> bool {
        source.width > 0
            && source.height > 0
            && source.width <= u32::MAX as usize
            && source.height <= u32::MAX as usize
            && source
                .width
                .checked_mul(source.height)
                .and_then(|v| v.checked_mul(4))
                == Some(source.data.len())
            && source.data.len() <= SOURCE_BYTES
    }
    pub fn render(&mut self, request: &Request, density: f32) -> Result<Option<Frame>, String> {
        if self.context.is_lost() {
            return Err("Baked preview device was lost".into());
        }
        if !density.is_finite()
            || density <= 0.
            || request.size.contains(&0)
            || request.source.width() == 0
            || request.source.height() == 0
            || request.source.pixels().len() > SOURCE_BYTES
        {
            return Err("Invalid baked geometry preview dimensions/source".into());
        }
        let limit = self.context.device.limits().max_texture_dimension_2d;
        let physical = request.size.map(|v| (v as f64 * density as f64).round());
        if physical.iter().any(|v| *v < 1. || *v > limit as f64)
            || request.source.width() > limit
            || request.source.height() > limit
        {
            return Err("Baked geometry preview exceeds GPU texture limits".into());
        }
        let [w, h] = physical.map(|v| v as u32);
        let bytes = w as u64 * h as u64 * 4;
        if bytes > OUTPUT_BYTES {
            return Err("Baked geometry preview exceeds its 128 MiB output budget".into());
        }
        self.targets.retain(|i| {
            Arc::strong_count(i) > 1 || (i.descriptor.width == w && i.descriptor.height == h)
        });
        let image = if let Some(image) = self.targets.iter().find(|i| {
            Arc::strong_count(i) == 1 && i.descriptor.width == w && i.descriptor.height == h
        }) {
            image.clone()
        } else {
            let resident: u64 = self
                .targets
                .iter()
                .map(|i| i.descriptor.width as u64 * i.descriptor.height as u64 * 4)
                .sum();
            if self.targets.len() >= 3 || resident + bytes > OUTPUT_BYTES {
                return Ok(None);
            }
            let image = GpuImage::new(
                &self.context,
                ImageDescriptor {
                    width: w,
                    height: h,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    color_space: ColorSpace::Srgb,
                    alpha_mode: AlphaMode::Straight,
                },
            )
            .map_err(|e| e.to_string())?;
            self.targets.push(image.clone());
            image
        };
        if self.source.as_ref().is_none_or(|(asset, _)| {
            !Arc::ptr_eq(asset, &request.source) && **asset != *request.source
        }) {
            let descriptor = ImageDescriptor {
                width: request.source.width(),
                height: request.source.height(),
                format: wgpu::TextureFormat::Rgba8Unorm,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Straight,
            };
            let source = GpuImage::new(&self.context, descriptor).map_err(|e| e.to_string())?;
            self.context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &source.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                request.source.pixels(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(request.source.width() * 4),
                    rows_per_image: Some(request.source.height()),
                },
                source.texture.size(),
            );
            self.context
                .record_upload(request.source.pixels().len() as u64);
            self.source = Some((request.source.clone(), source));
        }
        let width = request.size[0] as f32;
        let height = request.size[1] as f32;
        let (scale, origin) = shared::isometric::preview_placement(
            width,
            height,
            [
                request.source.width() as f32,
                request.source.height() as f32,
            ],
        );
        let mut parameters = Vec::new();
        for value in [
            width, height, w as f32, h as f32, origin[0], origin[1], scale, 0.,
        ] {
            parameters.extend(value.to_ne_bytes());
        }
        for value in request.checker.into_iter().flatten() {
            parameters.extend((value as u32).to_ne_bytes());
        }
        let uniform = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("baked geometry viewport"),
                contents: &parameters,
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("baked geometry viewport"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &self.source.as_ref().unwrap().1.view,
                        ),
                    },
                ],
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("baked geometry viewport"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("baked geometry viewport"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &image.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.draw(0..3, 0..1);
        }
        self.context.queue.submit([encoder.finish()]);
        Ok(Some(Frame {
            epoch: request.epoch,
            revision: request.revision,
            image,
            size: request.size,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::ReadbackTicket;
    use theframework::{prelude::*, render_wgpu::PaintRenderer};
    fn capture(gpu: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
        let mut ticket = ReadbackTicket::capture(gpu, image).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_geometry_baked_sources_match_reference_without_viewport_uploads() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut preview = Preview::new(gpu.clone());
        let checker = [[17, 39, 62, 255], [104, 89, 72, 255]];
        for (source_w, source_h) in [(3, 5), (16, 7), (39, 43)] {
            let mut pixels = Vec::new();
            for y in 0..source_h {
                for x in 0..source_w {
                    pixels.extend([
                        (x * 47 % 256) as u8,
                        (y * 19 % 256) as u8,
                        ((x + y) * 13 % 256) as u8,
                        ([0, 1, 33, 128, 254, 255][(x + y) % 6]),
                    ]);
                }
            }
            let texture = rusterix::Texture::new(pixels.clone(), source_w, source_h);
            let source =
                Arc::new(ImageAsset::rgba(source_w as u32, source_h as u32, pixels).unwrap());
            for (w, h) in [(96, 64), (113, 177), (301, 121)] {
                let request = Request {
                    epoch: 1,
                    revision: 2,
                    source: source.clone(),
                    size: [w, h],
                    checker,
                };
                let before = gpu.transfers();
                let frame = preview.render(&request, 1.).unwrap().unwrap();
                assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
                let actual = capture(&gpu, frame.image.clone());
                let mut expected = TheRGBABuffer::new(TheDim::sized(w as i32, h as i32));
                shared::isometric::draw_preview(&mut expected, &texture, checker);
                assert_eq!(
                    actual,
                    expected.pixels(),
                    "source={source_w}x{source_h} viewport={w}x{h}"
                );
                let before = gpu.transfers();
                drop(frame);
                let repeated = preview.render(&request, 1.).unwrap().unwrap();
                assert_eq!(
                    gpu.transfers(),
                    before,
                    "only changed source pixels may be uploaded"
                );
                let mut images = renderkit::ImageRegistry::new(gpu.clone());
                let image = images.insert(repeated.image.clone()).unwrap();
                let output = GpuImage::new(&gpu, ImageDescriptor::rgba(w, h)).unwrap();
                PaintRenderer::new(gpu.clone())
                    .unwrap()
                    .render(
                        &repeated.paint(image),
                        &images,
                        &output.view,
                        output.descriptor.format,
                        (w, h),
                        1.,
                        [0.; 4],
                    )
                    .unwrap();
                let composed = capture(&gpu, output);
                assert!(
                    composed
                        .iter()
                        .zip(&actual)
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "common compositor decodes encoded GPU output exactly once"
                );
            }
        }
        let source = Arc::new(ImageAsset::rgba(1, 1, vec![255, 3, 8, 128]).unwrap());
        let request = Request {
            epoch: 3,
            revision: 4,
            source,
            size: [96, 64],
            checker,
        };
        let first = preview.render(&request, 1.).unwrap().unwrap();
        let original = capture(&gpu, first.image.clone());
        let second = preview.render(&request, 1.).unwrap().unwrap();
        let third = preview.render(&request, 1.).unwrap().unwrap();
        assert!(preview.render(&request, 1.).unwrap().is_none());
        assert_eq!(capture(&gpu, first.image.clone()), original);
        drop(second);
        drop(third);
        let before = gpu.transfers();
        let scaled = preview.render(&request, 1.5).unwrap().unwrap();
        assert_eq!(
            (
                scaled.image.descriptor.width,
                scaled.image.descriptor.height
            ),
            (144, 96)
        );
        assert_eq!(gpu.transfers(), before);
        assert!(preview.render(&request, f32::NAN).is_err());
        assert!(preview.render(&request, 1e9).is_err());
        let mut invalid = rusterix::Texture::new(vec![1, 2, 3, 4], 1, 1);
        invalid.data.pop();
        assert!(!Preview::source_valid(&invalid));
    }
}
