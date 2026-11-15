//! Private regional raster retries retain raw scene color/depth across attempts.
//! Post effects always see the complete camera-sized raw image. Owners publish
//! only after every region has resolved, keeping scene/camera/settings frozen.
use crate::paint_pages::PaintDispatchRegion;
use wgpu::util::DeviceExt;

const CLEAR: &str = r#"
@group(0) @binding(0) var<uniform> color: vec4<f32>;
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2<f32>(-1., -3.), vec2<f32>(-1., 1.), vec2<f32>(3., 1.));
    return vec4<f32>(p[index], 0., 1.);
}
struct Clear { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32 };
@fragment fn fs_main() -> Clear { return Clear(color, 1.); }
"#;

pub(super) struct RegionClear {
    samples: u32,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}
impl RegionClear {
    pub(super) fn prepare(
        slot: &mut Option<Self>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        samples: u32,
        color: [f32; 4],
    ) {
        if slot.as_ref().is_none_or(|clear| clear.samples != samples) {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("raster region clear layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                }],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("raster region clear pipeline layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("raster region color/depth clear"),
                source: wgpu::ShaderSource::Wgsl(CLEAR.into()),
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("raster regional retry clear"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            });
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raster region clear color"),
                contents: bytemuck::cast_slice(&color),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raster region clear group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });
            *slot = Some(Self {
                samples,
                pipeline,
                uniform,
                group,
            });
        } else {
            queue.write_buffer(
                &slot.as_ref().unwrap().uniform,
                0,
                bytemuck::cast_slice(&color),
            );
        }
    }
    /// The caller selects the absolute region scissor on the full-sized target.
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Intersect, preserving an empty result. Clamping to one pixel would introduce
/// unrelated dependencies when a private region lies outside the camera viewport.
pub(super) fn scissor(
    pass: &mut wgpu::RenderPass<'_>,
    region: PaintDispatchRegion,
    bounds: [u32; 4],
) {
    let start = [
        region.origin[0].max(bounds[0]),
        region.origin[1].max(bounds[1]),
    ];
    let end = [
        (region.origin[0] + region.size[0]).min(bounds[0] + bounds[2]),
        (region.origin[1] + region.size[1]).min(bounds[1] + bounds[3]),
    ];
    if start[0] >= end[0] || start[1] >= end[1] {
        pass.set_scissor_rect(0, 0, 0, 0);
    } else {
        pass.set_scissor_rect(start[0], start[1], end[0] - start[0], end[1] - start[1]);
    }
}
