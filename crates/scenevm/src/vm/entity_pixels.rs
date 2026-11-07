//! Characters and ground items keep real meshes, lighting and shadows, but rasterize
//! on a coarse screen grid. The expansion pass writes their sampled depth into the
//! full-resolution world depth buffer, so walls and other surfaces still occlude them.
use super::{SCENEVM_3D_RASTER_WGSL, Vert3DPod};
use wgpu::util::DeviceExt;

const COMPOSITE: &str = r#"
struct Grid { block: u32, _pad0: u32, _pad1: u32, _pad2: u32 };
@group(0) @binding(0) var pixels: texture_2d<f32>;
@group(0) @binding(1) var depths: texture_depth_2d;
@group(0) @binding(2) var<uniform> grid: Grid;
@vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2<f32>(-1., -3.), vec2<f32>(-1., 1.), vec2<f32>(3., 1.));
    return vec4<f32>(p[id], 0., 1.);
}
struct Output { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32 };
@fragment fn fs_main(@builtin(position) position: vec4<f32>) -> Output {
    let p = vec2<i32>(position.xy / f32(grid.block));
    let color = textureLoad(pixels, p, 0);
    if (color.a <= 0.001) { discard; }
    var out: Output;
    out.color = color;
    out.depth = textureLoad(depths, p, 0);
    return out;
}
"#;

pub(super) struct EntityPixelPass {
    render_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    composite_layout: wgpu::BindGroupLayout,
    composite_group: wgpu::BindGroup,
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    indices: wgpu::Buffer,
    index_capacity: u64,
    index_count: u32,
    size: (u32, u32),
    block: u32,
    samples: u32,
}
impl EntityPixelPass {
    fn targets(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        block: u32,
        layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::TextureView, wgpu::TextureView, wgpu::BindGroup) {
        let target = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: width.div_ceil(block).max(1),
                        height: height.div_ceil(block).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let color = target("entity-pixels-color", wgpu::TextureFormat::Rgba16Float);
        let depth = target("entity-pixels-depth", wgpu::TextureFormat::Depth32Float);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("entity-pixels-grid"),
            contents: bytemuck::cast_slice(&[block, 0u32, 0, 0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("entity-pixels-composite-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        (color, depth, group)
    }
    pub(super) fn prepare(
        slot: &mut Option<Self>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        block: u32,
        samples: u32,
        indices: &[u32],
    ) {
        if slot.as_ref().is_none_or(|p| p.samples != samples) {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("entity-pixels-material"),
                source: wgpu::ShaderSource::Wgsl(SCENEVM_3D_RASTER_WGSL.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("entity-pixels-material-layout"),
                bind_group_layouts: &[Some(material_layout)],
                immediate_size: 0,
            });
            let attrs = [
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 36,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: 40,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 44,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 14,
                    format: wgpu::VertexFormat::Uint32x4,
                },
                wgpu::VertexAttribute {
                    offset: 80,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 96,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Uint32x4,
                },
            ];
            let depth_state = || wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            };
            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("entity-pixels-render"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[("ENTITY_PIXEL_ART", 1.0)],
                        ..Default::default()
                    },
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vert3DPod>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attrs,
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &[("ENTITY_PIXEL_ART", 1.0)],
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Some(depth_state()),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("entity-pixels-expansion"),
                source: wgpu::ShaderSource::Wgsl(COMPOSITE.into()),
            });
            let composite_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("entity-pixels-composite-layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
            let composite_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("entity-pixels-expansion-layout"),
                    bind_group_layouts: &[Some(&composite_layout)],
                    immediate_size: 0,
                });
            let composite_pipeline =
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("entity-pixels-composite"),
                    layout: Some(&composite_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &composite_shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &composite_shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba16Float,
                            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: Default::default(),
                    depth_stencil: Some(depth_state()),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..Default::default()
                    },
                    multiview_mask: None,
                    cache: None,
                });
            let (color, depth, composite_group) =
                Self::targets(device, width, height, block, &composite_layout);
            let capacity = (indices.len() * 4).max(4) as u64;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("entity-pixels-indices"),
                size: capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            *slot = Some(Self {
                render_pipeline,
                composite_pipeline,
                composite_layout,
                composite_group,
                color,
                depth,
                indices: buffer,
                index_capacity: capacity,
                index_count: 0,
                size: (width, height),
                block,
                samples,
            });
        }
        let pass = slot.as_mut().unwrap();
        if pass.size != (width, height) || pass.block != block {
            (pass.color, pass.depth, pass.composite_group) =
                Self::targets(device, width, height, block, &pass.composite_layout);
            pass.size = (width, height);
            pass.block = block;
        }
        // Reuse the index allocation across animated poses.
        let bytes = bytemuck::cast_slice(indices);
        if bytes.len() as u64 > pass.index_capacity {
            pass.index_capacity = (bytes.len() as u64).next_power_of_two();
            pass.indices = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("entity-pixels-indices"),
                size: pass.index_capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&pass.indices, 0, bytes);
        pass.index_count = indices.len() as u32;
    }
    pub(super) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        materials: &wgpu::BindGroup,
        vertices: &wgpu::Buffer,
        viewport: Option<[f32; 4]>,
        width: u32,
        height: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("entity-pixels-low-resolution"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.color,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let [x, y, w, h] = viewport.unwrap_or([0., 0., width as f32, height as f32]);
        let x = x.max(0.).min((width - 1) as f32).floor();
        let y = y.max(0.).min((height - 1) as f32).floor();
        let w = w.max(1.).min(width as f32 - x).floor();
        let h = h.max(1.).min(height as f32 - y).floor();
        let block = self.block as f32;
        pass.set_viewport(x / block, y / block, w / block, h / block, 0., 1.);
        pass.set_pipeline(&self.render_pipeline);
        pass.set_bind_group(0, materials, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
    pub(super) fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth
    }

    pub(super) fn composite(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        depth: &wgpu::TextureView,
        viewport: Option<[f32; 4]>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("entity-pixels-expand-and-depth-test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some([x, y, w, h]) = viewport {
            let x = x.max(0.).min((self.size.0 - 1) as f32) as u32;
            let y = y.max(0.).min((self.size.1 - 1) as f32) as u32;
            let w = w.max(1.).min((self.size.0 - x) as f32) as u32;
            let h = h.max(1.).min((self.size.1 - y) as f32) as u32;
            pass.set_scissor_rect(x, y, w, h);
        }
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &self.composite_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pixel_post_shader_validates_for_single_sample_and_msaa() {
        for samples in [1, 4] {
            let source = super::super::raster3d_post_source(samples);
            let module = wgpu::naga::front::wgsl::parse_str(&source).unwrap();
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
        }
    }

    #[test]
    fn pixel_expansion_shader_validates() {
        let module = wgpu::naga::front::wgsl::parse_str(super::COMPOSITE).unwrap();
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
    }
}
