//! Retained GPU equivalent of the modal's reference scaled()/copy_into().
//! In particular scaling rounds each source coordinate before flattening it;
//! the last source column can carry into the next row. Keep that byte policy
//! here rather than changing the independently specified sprite sampler.
use renderkit::{AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, TargetDescriptor};
use std::sync::Arc;

pub(super) struct Fitter {
    context: Arc<GpuContext>,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}
impl Fitter {
    pub fn new(context: Arc<GpuContext>) -> Self {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("modal preview fitting"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("modal preview fitting"),
                source: wgpu::ShaderSource::Wgsl(include_str!("fitting.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("modal preview fitting"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("modal preview fitting"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        Self {
            context,
            pipeline,
            layout,
        }
    }
    pub fn render(
        &self,
        source: &Arc<GpuImage>,
        size: (u32, u32),
        max_side: i32,
    ) -> Result<Arc<GpuImage>, String> {
        if self.context.is_lost() {
            return Err("Modal preview GPU device was lost".into());
        }
        let side = size.0.min(size.1).min(max_side.max(1) as u32);
        let descriptor = ImageDescriptor {
            width: size.0,
            height: size.1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        };
        let mut target = TargetDescriptor::color(size.0, size.1, descriptor.format);
        target.usage |= wgpu::TextureUsages::STORAGE_BINDING;
        let lease = self
            .context
            .acquire_target(target)
            .map_err(|e| e.to_string())?;
        let output =
            GpuImage::from_target(&self.context, lease, descriptor).map_err(|e| e.to_string())?;
        let params = [
            side,
            (size.0 - side) / 2,
            (size.1 - side) / 2,
            u32::from(source.descriptor.format.is_srgb()),
        ];
        let bytes: Vec<_> = params.iter().flat_map(|v| v.to_ne_bytes()).collect();
        let uniform = self
            .context
            .upload_immutable_buffer(&bytes, wgpu::BufferUsages::UNIFORM)
            .map_err(|e| e.to_string())?;
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("modal preview fitting"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&output.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.buffer().as_entire_binding(),
                    },
                ],
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("modal preview fitting"),
                });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("modal preview fitting"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        }
        self.context.queue.submit([encoder.finish()]);
        Ok(output)
    }
}
