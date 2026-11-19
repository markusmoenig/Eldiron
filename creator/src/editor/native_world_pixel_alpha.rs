//! CPU source-pixel guides blend hidden RGB and replace alpha only at covered
//! samples. Keep that policy on GPU before flattening the viewport over black.
use super::*;

pub(super) struct Composer {
    context: Arc<renderkit::GpuContext>,
    mask: Option<PaintRenderer>,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}
impl Composer {
    pub(super) fn new(context: Arc<renderkit::GpuContext>) -> renderkit::Result<Self> {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("isometric source alpha"),
                entries: &[0, 1, 2].map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }),
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("isometric source alpha"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("native_world_pixel_alpha.wgsl").into(),
                ),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("isometric source alpha"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("isometric source alpha"),
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
        Ok(Self {
            context,
            mask: None,
            layout,
            pipeline,
        })
    }
    pub(super) fn compose(
        &mut self,
        source_paint: &PaintList,
        source: &GpuImage,
        painted: Arc<GpuImage>,
        images: &ImageRegistry,
        painted_contains_coverage: bool,
    ) -> Result<Arc<GpuImage>, String> {
        let descriptor = painted.descriptor;
        let size = (descriptor.width, descriptor.height);
        if size != (source.descriptor.width, source.descriptor.height)
            || source.device_generation() != self.context.generation()
        {
            return Err("isometric source alpha dimensions/device mismatch".into());
        }
        let mask = if painted_contains_coverage {
            painted.clone()
        } else {
            self.coverage(source_paint, descriptor, images)?
        };
        let output =
            GpuImage::render_target(&self.context, descriptor).map_err(|e| e.to_string())?;
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("isometric source alpha"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&painted.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&mask.view),
                    },
                ],
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("isometric source alpha"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("isometric source alpha"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.context.queue.submit([encoder.finish()]);
        Ok(output)
    }

    fn coverage(
        &mut self,
        source_paint: &PaintList,
        descriptor: ImageDescriptor,
        images: &ImageRegistry,
    ) -> Result<Arc<GpuImage>, String> {
        let mask = GpuImage::render_target(
            &self.context,
            ImageDescriptor {
                color_space: ColorSpace::Linear,
                alpha_mode: AlphaMode::Premultiplied,
                ..descriptor
            },
        )
        .map_err(|e| e.to_string())?;
        let coverage = PaintList {
            ops: source_paint
                .ops
                .iter()
                .filter_map(|op| match op {
                    PaintOp::SampledLine { from, to, color } if color[3] > 0. => {
                        Some(PaintOp::SampledLine {
                            from: *from,
                            to: *to,
                            color: [1.; 4],
                        })
                    }
                    _ => None,
                })
                .collect(),
        };
        if self.mask.is_none() {
            self.mask = Some(PaintRenderer::new(self.context.clone()).map_err(|e| e.to_string())?);
        }
        self.mask.as_mut().unwrap()
            .render(
                &coverage,
                images,
                &mask.view,
                descriptor.format,
                (descriptor.width, descriptor.height),
                1.,
                [0.; 4],
            )
            .map_err(|e| e.to_string())?;
        Ok(mask)
    }
}
