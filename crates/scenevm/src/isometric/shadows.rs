//! Alpha-tested mesh shadow maps, on the same SceneVM device as iso lighting.
//! Cached recipe triangles and the shared terrain are independent of the camera.
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Pod, Zeroable)]
pub struct ShadowVertex {
    pub position: [f32; 4],
    /// UV, alpha-mask word offset, packed mask width/height (integers bitcast to floats).
    pub uv_alpha: [f32; 4],
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShadowGeometry {
    pub vertices: Vec<ShadowVertex>,
    pub alpha: Vec<u32>,
    /// Runtime cache identity; source frames need no runtime identity.
    #[serde(skip)]
    pub revision: u64,
}
impl ShadowGeometry {
    pub fn append(&mut self, source: &Self, position: [f32; 3], scale: f32) {
        let offset = self.alpha.len() as u32;
        self.alpha.extend_from_slice(&source.alpha);
        if position == [0.; 3] && scale == 1. && offset == 0 {
            self.vertices.extend_from_slice(&source.vertices);
            return;
        }
        self.vertices.extend(source.vertices.iter().map(|v| {
            let mut v = *v;
            for i in 0..3 {
                v.position[i] = v.position[i] * scale + position[i];
            }
            if v.uv_alpha[3].to_bits() != 0 {
                v.uv_alpha[2] = f32::from_bits(v.uv_alpha[2].to_bits() + offset);
            }
            v
        }));
    }
    pub fn triangle(&mut self, points: [[f32; 3]; 3]) {
        self.vertices.extend(points.map(|p| ShadowVertex {
            position: [p[0], p[1], p[2], 0.],
            ..Default::default()
        }));
    }
}

#[cfg(feature = "gpu")]
pub(super) mod gpu {
    use super::*;
    use crate::{
        SceneVMError, SceneVMResult,
        isometric::{PointLight, Uniforms},
    };
    use vek::Vec3;
    use wgpu::util::DeviceExt;
    const SUN_SIZE: u32 = 2048;
    const POINT_SIZE: u32 = 512;
    // WebGPU's guaranteed 256 array layers can hold six views for every light.
    const MAX_POINT_LIGHTS: usize = 32;
    #[repr(C)]
    #[derive(Clone, Copy, Default, Pod, Zeroable)]
    struct View {
        right: [f32; 4],
        up: [f32; 4],
        forward: [f32; 4],
        origin: [f32; 4],
        projection: [f32; 4], // half width/height, near/far
        meta: [u32; 4],       // perspective, light index, enabled, unused
    }
    fn vec4(v: Vec3<f32>) -> [f32; 4] {
        [v.x, v.y, v.z, 0.]
    }
    fn view(
        direction: Vec3<f32>,
        origin: Vec3<f32>,
        projection: [f32; 4],
        light: Option<usize>,
    ) -> View {
        let f = direction.normalized();
        let axis = if f.y.abs() > 0.99 {
            Vec3::unit_z()
        } else {
            Vec3::unit_y()
        };
        let r = f.cross(axis).normalized();
        View {
            right: vec4(r),
            up: vec4(r.cross(f)),
            forward: vec4(f),
            origin: vec4(origin),
            projection,
            meta: [u32::from(light.is_some()), light.unwrap_or(0) as u32, 1, 0],
        }
    }
    fn depth(device: &wgpu::Device, size: u32, layers: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iso-mesh-shadow-depth"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let v = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        (texture, v)
    }
    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &mut wgpu::Buffer,
        capacity: &mut usize,
        bytes: &[u8],
    ) {
        if bytes.len() > *capacity {
            *capacity = bytes.len().next_power_of_two();
            *buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("iso-shadow-cached-buffer"),
                size: *capacity as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            queue.write_buffer(buffer, 0, bytes);
        }
    }
    pub struct Pass {
        pipeline: wgpu::RenderPipeline,
        layout: wgpu::BindGroupLayout,
        vertices: wgpu::Buffer,
        vertex_capacity: usize,
        alpha: wgpu::Buffer,
        alpha_capacity: usize,
        uniforms: wgpu::Buffer,
        pub views: wgpu::Buffer,
        view_capacity: usize,
        sun: wgpu::Texture,
        pub sun_view: wgpu::TextureView,
        point: wgpu::Texture,
        pub point_view: wgpu::TextureView,
        layers: u32,
        revision: Option<u64>,
        count: u32,
    }
    impl Pass {
        pub fn new(device: &wgpu::Device) -> Self {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("iso-mesh-shadows"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../isometric_shadow.wgsl").into()),
            });
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iso-shadow-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(96),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("iso-shadow-pipeline-layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("iso-alpha-tested-mesh-shadow-map"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
            let (sun, sun_view) = depth(device, SUN_SIZE, 1);
            let (point, point_view) = depth(device, POINT_SIZE, 1);
            let storage = |label| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: &[0; 96],
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                })
            };
            Self {
                pipeline,
                layout,
                vertices: storage("iso-shadow-vertices"),
                vertex_capacity: 96,
                alpha: storage("iso-shadow-alpha"),
                alpha_capacity: 96,
                views: storage("iso-shadow-views"),
                view_capacity: 96,
                uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("iso-shadow-camera-uniforms"),
                    size: 256 * (1 + MAX_POINT_LIGHTS * 6) as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                sun,
                sun_view,
                point,
                point_view,
                layers: 1,
                revision: None,
                count: 0,
            }
        }
        pub fn draw(
            &mut self,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            encoder: &mut wgpu::CommandEncoder,
            u: &Uniforms,
            lights: &[PointLight],
            geometry: Option<&ShadowGeometry>,
        ) -> SceneVMResult<()> {
            let geometry = geometry.filter(|g| !g.vertices.is_empty());
            let light_count = lights.len().min(MAX_POINT_LIGHTS);
            let mut views = vec![View::default(); 1 + lights.len() * 6];
            if let Some(g) = geometry {
                if g.vertices.len() % 3 != 0
                    || g.vertices
                        .iter()
                        .any(|v| v.position.iter().any(|x| !x.is_finite()))
                {
                    return Err(SceneVMError::InvalidOperation(
                        "Invalid iso shadow triangles".into(),
                    ));
                }
                if self.revision != Some(g.revision) || self.count as usize != g.vertices.len() {
                    let bytes = bytemuck::cast_slice(&g.vertices);
                    if bytes.len() > device.limits().max_storage_buffer_binding_size as usize {
                        return Err(SceneVMError::BufferAllocationFailed(
                            "Iso shadow geometry exceeds device limits".into(),
                        ));
                    }
                    upload(
                        device,
                        queue,
                        &mut self.vertices,
                        &mut self.vertex_capacity,
                        bytes,
                    );
                    upload(
                        device,
                        queue,
                        &mut self.alpha,
                        &mut self.alpha_capacity,
                        if g.alpha.is_empty() {
                            &[0; 4]
                        } else {
                            bytemuck::cast_slice(&g.alpha)
                        },
                    );
                    self.revision = Some(g.revision);
                    self.count = g.vertices.len() as u32;
                }
                // Fit the sun to the visible world plus its maximum shadow reach, not the entire map.
                let unit = u.projection[2].max(0.001);
                let world = |x: f32, y: f32| {
                    let qx = (x - u.projection[0]) / unit;
                    let qy = (y - u.projection[1]) / unit;
                    Vec3::new(qx * 0.5 - qy, 0., qx * 0.5 + qy)
                };
                let center = world(u.counts[0] as f32 * 0.5, u.counts[1] as f32 * 0.5);
                let radius = ((u.counts[0] as f32 / unit) * 0.5 + u.counts[1] as f32 / unit)
                    .max(8.)
                    + u.shadow[0].min(16.);
                let direction = -Vec3::new(u.sun[0], u.sun[1], u.sun[2]);
                if direction.magnitude_squared() > 0.001 && u.sun_color[..3].iter().any(|v| *v > 0.)
                {
                    views[0] = view(
                        direction,
                        center + direction.normalized() * (-radius),
                        [radius, radius, 0., radius * 2.],
                        None,
                    );
                    // Stabilize the light-space grid during camera movement.
                    let texel = radius * 2. / SUN_SIZE as f32;
                    for axis in 0..2 {
                        let basis = if axis == 0 {
                            Vec3::from_slice(&views[0].right[..3])
                        } else {
                            Vec3::from_slice(&views[0].up[..3])
                        };
                        let origin = Vec3::from_slice(&views[0].origin[..3]);
                        let delta = (origin.dot(basis) / texel).round() * texel - origin.dot(basis);
                        views[0].origin = vec4(origin + basis * delta);
                    }
                }
                let directions = [
                    Vec3::unit_x(),
                    -Vec3::unit_x(),
                    Vec3::unit_y(),
                    -Vec3::unit_y(),
                    Vec3::unit_z(),
                    -Vec3::unit_z(),
                ];
                for (i, l) in lights.iter().take(light_count).enumerate() {
                    if l.parameters[1] <= 0.05 {
                        continue;
                    }
                    for (face, direction) in directions.iter().enumerate() {
                        views[1 + i * 6 + face] = view(
                            *direction,
                            Vec3::from_slice(&l.position[..3]),
                            [1., 1., 0.02, l.parameters[1]],
                            Some(i),
                        );
                    }
                }
            }
            upload(
                device,
                queue,
                &mut self.views,
                &mut self.view_capacity,
                bytemuck::cast_slice(&views),
            );
            if geometry.is_none() || u.shadow[0] <= 0. {
                return Ok(());
            }
            let layers = (light_count * 6).max(1) as u32;
            if layers != self.layers {
                (self.point, self.point_view) = depth(device, POINT_SIZE, layers);
                self.layers = layers;
            }
            let mut uniform_bytes = vec![0u8; (1 + light_count * 6) * 256];
            for (i, v) in views.iter().take(1 + light_count * 6).enumerate() {
                uniform_bytes[i * 256..i * 256 + 96].copy_from_slice(bytemuck::bytes_of(v));
            }
            queue.write_buffer(&self.uniforms, 0, &uniform_bytes);
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iso-shadow-scene"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.uniforms,
                            offset: 0,
                            size: std::num::NonZeroU64::new(96),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.vertices.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.alpha.as_entire_binding(),
                    },
                ],
            });
            for (i, v) in views.iter().take(1 + light_count * 6).enumerate() {
                if v.meta[2] == 0 {
                    continue;
                }
                let target = if i == 0 { &self.sun } else { &self.point };
                let target_view = target.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: i.saturating_sub(1) as u32,
                    array_layer_count: Some(1),
                    ..Default::default()
                });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("iso-mesh-terrain-shadow-map"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &target_view,
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
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind, &[(i * 256) as u32]);
                pass.draw(0..self.count, 0..1);
            }
            Ok(())
        }
    }
}
