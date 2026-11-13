use super::*;
use scenevm::surface_output::{SurfaceOutput, SurfaceSelection};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OverlayParams {
    size: [u32; 4],
    camera: [f32; 4],
    forward: [f32; 4],
    background: [f32; 4],
    bits: [u32; 4],
}
/// Composes live actors and editor guides against baked depth on the host GPU.
/// The input overlay contains SceneVM's already-premultiplied display bytes.
pub struct OrthographicBakeOverlay {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    target: Option<Arc<GpuImage>>,
}
impl OrthographicBakeOverlay {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("baked world overlay"),
                entries: &[
                    buffer(0, wgpu::BufferBindingType::Uniform),
                    texture(1),
                    buffer(2, wgpu::BufferBindingType::Storage { read_only: true }),
                    texture(3),
                    texture(4),
                    texture(5),
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });
        let module = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("baked depth composition"),
                source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("baked depth composition"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("compose"),
                compilation_options: Default::default(),
                cache: None,
            });
        Ok(Self {
            context,
            layout,
            pipeline,
            target: None,
        })
    }
    pub fn render(
        &mut self,
        bake: &OrthographicBakeOutput,
        overlay: &GpuImage,
        surface: &SurfaceOutput,
        background: [u8; 4],
    ) -> Result<Arc<GpuImage>> {
        let generation = self.context.generation();
        if self.context.is_lost()
            || bake.image.device_generation() != generation
            || overlay.device_generation() != generation
            || surface.device_generation() != generation
        {
            return Err(RenderError(
                "bake overlay belongs to a lost or different GPU domain".into(),
            ));
        }
        let size = bake.size;
        if surface.selection() != SurfaceSelection::Dynamic
            || surface.size() != size
            || (overlay.descriptor.width, overlay.descriptor.height) != size
            || (bake.image.descriptor.width, bake.image.descriptor.height) != size
            || bake.depth.size() != u64::from(size.0) * u64::from(size.1) * 4
        {
            return Err(RenderError(
                "incompatible bake overlay targets or surface policy".into(),
            ));
        }
        if self
            .target
            .as_ref()
            .is_none_or(|target| (target.descriptor.width, target.descriptor.height) != size)
        {
            let descriptor = ImageDescriptor {
                width: size.0,
                height: size.1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Opaque,
            };
            let texture = self
                .context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("baked world composition"),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: descriptor.format,
                    usage: wgpu::TextureUsages::STORAGE_BINDING
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
            self.target = Some(Arc::new(GpuImage::from_texture(
                &self.context,
                texture,
                descriptor,
            )?));
        }
        let target = self.target.as_ref().unwrap().clone();
        let camera = surface.camera();
        let params = OverlayParams {
            size: [size.0, size.1, u32::from(bake.depth_enabled), 0],
            camera: [camera.pos.x, camera.pos.y, camera.pos.z, 0.],
            forward: [camera.forward.x, camera.forward.y, camera.forward.z, 0.],
            background: background.map(|v| v as f32),
            bits: [f32::INFINITY.to_bits(), 0, 0, 0],
        };
        let uniform = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bake overlay camera"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.context
            .record_upload(std::mem::size_of::<OverlayParams>() as u64);
        let view = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    view(1, &bake.image.view),
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: bake.depth.as_entire_binding(),
                    },
                    view(3, &overlay.view),
                    view(4, surface.world()),
                    view(5, surface.metadata()),
                    view(6, &target.view),
                ],
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("bake dynamic overlay"),
                });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bake dynamic overlay"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        }
        self.context.queue.submit(Some(encoder.finish()));
        Ok(target)
    }
}
