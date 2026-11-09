//! Deferred isometric lighting on SceneVM's existing device and queue.
//! The host supplies an unlit canvas and compact surface attributes; lights and
//! shadow rays are evaluated on the GPU. No asset or device is created per frame.
use bytemuck::{Pod, Zeroable};
pub mod shadows;
pub use shadows::{ShadowGeometry, ShadowVertex};
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Uniforms {
    pub projection: [f32; 4], // origin.xy, pixels/unit, width
    pub sun: [f32; 4],
    pub sun_color: [f32; 4],
    pub ambient: [f32; 4],
    pub counts: [u32; 4], // width,height,lights,casters
    pub shadow: [f32; 4], // distance,strength,particles,unused
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct PointLight {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub parameters: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Caster {
    pub min: [f32; 4],
    pub max: [f32; 4],
    pub position_scale: [f32; 4],
    pub grid: [u32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Particle {
    pub position_radius: [f32; 4],
    pub color: [f32; 4],
}

#[cfg(feature = "gpu")]
mod gpu {
    use super::*;
    use crate::{RenderResult, SceneVM, SceneVMError, SceneVMResult, Texture};

    struct Buffer {
        buffer: wgpu::Buffer,
        capacity: usize,
    }
    impl Buffer {
        fn new(device: &wgpu::Device, uniform: bool) -> Self {
            Self {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("iso-buffer"),
                    size: 256,
                    usage: wgpu::BufferUsages::COPY_DST
                        | if uniform {
                            wgpu::BufferUsages::UNIFORM
                        } else {
                            wgpu::BufferUsages::STORAGE
                        },
                    mapped_at_creation: false,
                }),
                capacity: 256,
            }
        }
        fn write(
            &mut self,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            data: &[u8],
            uniform: bool,
        ) {
            if data.len() > self.capacity {
                self.capacity = data.len().next_power_of_two();
                self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("iso-buffer"),
                    size: self.capacity as u64,
                    usage: wgpu::BufferUsages::COPY_DST
                        | if uniform {
                            wgpu::BufferUsages::UNIFORM
                        } else {
                            wgpu::BufferUsages::STORAGE
                        },
                    mapped_at_creation: false,
                });
            }
            if !data.is_empty() {
                queue.write_buffer(&self.buffer, 0, data);
            }
        }
    }
    pub(crate) struct Pass {
        pipeline: wgpu::ComputePipeline,
        particle_pipeline: wgpu::RenderPipeline,
        particle_layout: wgpu::BindGroupLayout,
        layout: wgpu::BindGroupLayout,
        buffers: [Buffer; 7],
        output: Texture,
        shadows: shadows::gpu::Pass,
    }
    impl Pass {
        pub fn new(device: &wgpu::Device) -> Self {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("scenevm-iso-lighting"),
                source: wgpu::ShaderSource::Wgsl(include_str!("isometric.wgsl").into()),
            });
            let mut entries = (0..7)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: if binding == 0 {
                            wgpu::BufferBindingType::Uniform
                        } else {
                            wgpu::BufferBindingType::Storage { read_only: true }
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect::<Vec<_>>();
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
            for binding in [9, 10] {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                });
            }
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iso-layout"),
                entries: &entries,
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("iso-pipeline-layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("iso-lighting"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
            let particle_entries = [0u32, 2, 6].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: if binding == 0 {
                    wgpu::ShaderStages::VERTEX_FRAGMENT
                } else if binding == 2 {
                    wgpu::ShaderStages::FRAGMENT
                } else {
                    wgpu::ShaderStages::VERTEX
                },
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage { read_only: true }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
            let particle_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("iso-particle-layout"),
                    entries: &particle_entries,
                });
            let particle_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("iso-particle-pipeline-layout"),
                    bind_group_layouts: &[Some(&particle_layout)],
                    immediate_size: 0,
                });
            let particle_pipeline =
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("iso-particle-billboards"),
                    layout: Some(&particle_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("particle_vertex"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("particle_fragment"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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
                pipeline,
                particle_pipeline,
                particle_layout,
                layout,
                buffers: std::array::from_fn(|i| Buffer::new(device, i == 0)),
                output: Texture::new(1, 1),
                shadows: shadows::gpu::Pass::new(device),
            }
        }
        fn draw(
            &mut self,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            u: &Uniforms,
            albedo: &mut [u8],
            surfaces: &[[u32; 2]],
            lights: &[PointLight],
            casters: &[Caster],
            columns: &[[f32; 2]],
            particles: &[Particle],
            geometry: Option<&ShadowGeometry>,
        ) -> SceneVMResult<RenderResult> {
            let [w, h, _, _] = u.counts;
            if w == 0
                || h == 0
                || w > 8192
                || h > 8192
                || albedo.len() != w as usize * h as usize * 4
                || surfaces.len() != w as usize * h as usize
                || u.counts[2] as usize != lights.len()
                || u.counts[3] as usize != casters.len()
            {
                return Err(SceneVMError::InvalidOperation(
                    "Invalid iso frame dimensions or light counts".into(),
                ));
            }
            #[cfg(target_arch = "wasm32")]
            if self
                .output
                .gpu
                .as_ref()
                .and_then(|g| g.map_ready.as_ref())
                .is_some()
                && !self.output.try_finish_download_from_gpu()
            {
                if self.output.data.len() == albedo.len() {
                    albedo.copy_from_slice(&self.output.data);
                }
                return Ok(RenderResult::ReadbackPending);
            }
            let data: [&[u8]; 7] = [
                bytemuck::bytes_of(u),
                albedo,
                bytemuck::cast_slice(surfaces),
                bytemuck::cast_slice(lights),
                bytemuck::cast_slice(casters),
                bytemuck::cast_slice(columns),
                bytemuck::cast_slice(particles),
            ];
            let limit = device.limits().max_storage_buffer_binding_size as usize;
            if data[1..].iter().any(|d| d.len() > limit) {
                return Err(SceneVMError::BufferAllocationFailed(
                    "Iso frame exceeds device storage limits".into(),
                ));
            }
            for (i, data) in data.into_iter().enumerate() {
                self.buffers[i].write(device, queue, data, i == 0);
            }
            if self.output.width != w || self.output.height != h {
                self.output = Texture::new(w, h);
            }
            self.output.ensure_gpu_with(device);
            let view = &self.output.gpu.as_ref().unwrap().view;
            let mut bindings = self
                .buffers
                .iter()
                .enumerate()
                .map(|(i, b)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: b.buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>();
            bindings.push(wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(view),
            });
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scenevm-iso-frame"),
            });
            self.shadows
                .draw(device, queue, &mut encoder, u, lights, geometry)?;
            bindings.push(wgpu::BindGroupEntry {
                binding: 8,
                resource: self.shadows.views.as_entire_binding(),
            });
            for (binding, view) in [(9, &self.shadows.sun_view), (10, &self.shadows.point_view)] {
                bindings.push(wgpu::BindGroupEntry {
                    binding,
                    resource: wgpu::BindingResource::TextureView(view),
                });
            }
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("iso-frame"),
                layout: &self.layout,
                entries: &bindings,
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("iso-lighting-and-shadows"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
            }
            if !particles.is_empty() {
                let entries = [0u32, 2, 6].map(|binding| wgpu::BindGroupEntry {
                    binding,
                    resource: self.buffers[binding as usize].buffer.as_entire_binding(),
                });
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("iso-particle-frame"),
                    layout: &self.particle_layout,
                    entries: &entries,
                });
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("iso-live-particles"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.particle_pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.draw(0..6, 0..particles.len() as u32);
            }
            queue.submit(Some(encoder.finish()));
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.output.download_from_gpu_with(device, queue);
                albedo.copy_from_slice(&self.output.data);
                Ok(RenderResult::Presented)
            }
            #[cfg(target_arch = "wasm32")]
            {
                // Use SceneVM's asynchronous readback; never block the browser.
                albedo.copy_from_slice(&self.output.data);
                self.output.download_from_gpu_with(device, queue);
                Ok(RenderResult::ReadbackPending)
            }
        }
    }
    impl SceneVM {
        pub fn render_isometric(
            &mut self,
            uniforms: &Uniforms,
            albedo: &mut [u8],
            surfaces: &[[u32; 2]],
            lights: &[PointLight],
            casters: &[Caster],
            columns: &[[f32; 2]],
            particles: &[Particle],
        ) -> SceneVMResult<RenderResult> {
            self.render_isometric_geometry(
                uniforms, albedo, surfaces, lights, casters, columns, particles, None,
            )
        }
        pub fn render_isometric_geometry(
            &mut self,
            uniforms: &Uniforms,
            albedo: &mut [u8],
            surfaces: &[[u32; 2]],
            lights: &[PointLight],
            casters: &[Caster],
            columns: &[[f32; 2]],
            particles: &[Particle],
            geometry: Option<&ShadowGeometry>,
        ) -> SceneVMResult<RenderResult> {
            if self.gpu.is_none() {
                return Ok(self.render_frame(albedo, uniforms.counts[0], uniforms.counts[1]));
            }
            let gpu = self.gpu.as_ref().unwrap();
            let pass = self
                .isometric_pass
                .get_or_insert_with(|| Pass::new(&gpu.device));
            pass.draw(
                &gpu.device,
                &gpu.queue,
                uniforms,
                albedo,
                surfaces,
                lights,
                casters,
                columns,
                particles,
                geometry,
            )
        }
    }
}
#[cfg(feature = "gpu")]
pub(crate) use gpu::Pass;

#[cfg(all(test, feature = "gpu"))]
mod tests {
    use super::*;
    #[test]
    fn iso_shader_validates() {
        for source in [
            include_str!("isometric.wgsl"),
            include_str!("isometric_shadow.wgsl"),
        ] {
            let module = wgpu::naga::front::wgsl::parse_str(source).expect("iso shader parses");
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("iso shader validates");
        }
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn iso_gpu_lights_shadows_window_spans_and_resize() {
        let mut scene = crate::SceneVM::new(8, 8);
        let mut u = Uniforms {
            projection: [4., 4., 4., 8.],
            sun: [0., 1., 0., 0.],
            sun_color: [1., 1., 1., 0.],
            counts: [8, 8, 0, 0],
            shadow: [10., 1., 0., 0.],
            ..Default::default()
        };
        let normals = vec![[128 | 255 << 8 | 14 << 16, 0f32.to_bits()]; 64];
        let mut pixels = vec![160u8; 64 * 4];
        scene
            .render_isometric(&u, &mut pixels, &normals, &[], &[], &[], &[])
            .unwrap();
        assert!(pixels[0] > 150);
        // Bitmap artwork already contains shading. Darkness must not recolor
        // prelit sprites, while the depth surface remains available to particles.
        let prelit = vec![[128 | 255 << 8, (0f32.to_bits() & 0xffffff00) | 1]; 64];
        let original = [77, 113, 158, 255u8].repeat(64);
        let mut bitmap = original.clone();
        u.ambient = [0.; 4];
        u.sun_color = [0.; 4];
        scene
            .render_isometric(&u, &mut bitmap, &prelit, &[], &[], &[], &[])
            .unwrap();
        assert_eq!(
            bitmap, original,
            "Imported prelit pixels must survive GPU lighting"
        );
        u.sun_color = [1., 1., 1., 0.];
        let caster = Caster {
            min: [-10., 0., -10., 0.],
            max: [10., 3., 10., 0.],
            position_scale: [0., 0., 0., 1.],
            grid: [1, 1, 0, 2 << 8],
        };
        u.counts[3] = 1;
        let mut floor = vec![160u8; 64 * 4];
        scene
            .render_isometric(
                &u,
                &mut floor,
                &normals,
                &[],
                &[caster],
                &[[0., 0.5], [2., 3.]],
                &[],
            )
            .unwrap();
        assert_eq!(floor[0], 0, "sun is occluded by the lower stone span");
        // A horizontal light travels through the window's open height interval.
        u.sun = [1., 0., 0., 0.];
        let wall = vec![[255 | 128 << 8 | 14 << 16, 1.2f32.to_bits()]; 64];
        let mut opening = vec![160u8; 64 * 4];
        scene
            .render_isometric(
                &u,
                &mut opening,
                &wall,
                &[],
                &[caster],
                &[[0., 0.5], [2., 3.]],
                &[],
            )
            .unwrap();
        assert!(
            opening[0] > 150,
            "window height remains transparent to shadow rays"
        );
        u.sun_color = [0.; 4];
        u.counts[2] = 1;
        u.counts[3] = 0;
        let light = PointLight {
            position: [0., 2., 0., 0.],
            color: [1., 0., 0., 0.],
            parameters: [10., 20., 0., 0.],
        };
        let mut local = vec![160u8; 64 * 4];
        scene
            .render_isometric(&u, &mut local, &normals, &[light], &[], &[], &[])
            .unwrap();
        assert!(
            local[0] > 0 && local[1] == 0,
            "point light color is evaluated on GPU"
        );
        u.counts = [2, 2, 0, 0];
        u.ambient = [1., 1., 1., 0.];
        let mut resized = vec![120u8; 16];
        scene
            .render_isometric(&u, &mut resized, &normals[..4], &[], &[], &[], &[])
            .unwrap();
        assert!(
            resized[0] > 110,
            "render target resizes without losing the pipeline"
        );
        u.projection = [1., 1., 4., 2.];
        u.ambient = [0.; 4];
        u.shadow[2] = 1.;
        let particle = Particle {
            position_radius: [0., 0., 0., 0.5],
            color: [1., 0., 0., 1.],
        };
        let blank = vec![[0, f32::INFINITY.to_bits()]; 4];
        let mut visible = [0, 0, 0, 255u8].repeat(4);
        scene
            .render_isometric(&u, &mut visible, &blank, &[], &[], &[], &[particle])
            .unwrap();
        assert!(
            visible[0] > 0 && visible[1] == 0,
            "live billboard is drawn by the GPU particle pass"
        );
        let foreground = vec![[128 | 255 << 8 | 14 << 16, 2f32.to_bits()]; 4];
        let mut hidden = [0, 0, 0, 255u8].repeat(4);
        scene
            .render_isometric(&u, &mut hidden, &foreground, &[], &[], &[], &[particle])
            .unwrap();
        assert_eq!(
            hidden[0], 0,
            "foreground iso geometry occludes the particle"
        );
    }
}

#[cfg(all(test, feature = "gpu"))]
mod mesh_shadow_tests {
    use super::*;
    #[test]
    #[ignore = "requires native GPU"]
    fn exact_mesh_sun_local_light_alpha_and_moving_geometry() {
        let mut scene = crate::SceneVM::new(1, 1);
        let mut u = Uniforms {
            projection: [0.5, 0.5, 1., 1.],
            sun: [0.70710677, 0.70710677, 0., 0.],
            sun_color: [1., 1., 1., 0.],
            counts: [1, 1, 0, 0],
            shadow: [10., 1., 0., 1.],
            ..Default::default()
        };
        let normal = [[128 | 255 << 8 | 14 << 16, 0]];
        let mut mesh = ShadowGeometry {
            revision: 1,
            ..Default::default()
        };
        // A thin diagonal roof triangle has empty space inside its bounds.
        mesh.triangle([[0.3, 1., -1.], [1.5, 1., -1.], [0.3, 1., 1.]]);
        let render = |scene: &mut crate::SceneVM,
                      u: &Uniforms,
                      mesh: &ShadowGeometry,
                      lights: &[PointLight]| {
            let mut pixel = [160, 160, 160, 255];
            scene
                .render_isometric_geometry(
                    u,
                    &mut pixel,
                    &normal,
                    lights,
                    &[],
                    &[],
                    &[],
                    Some(mesh),
                )
                .unwrap();
            pixel[0]
        };
        // At x=1,z=0 the ray is outside this triangle, despite being inside its AABB.
        let open = render(&mut scene, &u, &mesh, &[]);
        assert!(open > 100, "triangle cavity must stay open: {open}");
        mesh.triangle([[1.5, 1., -1.], [1.5, 1., 1.], [0.3, 1., 1.]]);
        mesh.revision += 1;
        let shadow = render(&mut scene, &u, &mesh, &[]);
        assert!(
            shadow < open / 2,
            "actual roof triangles must cast a sun shadow: {shadow} vs {open}"
        );
        // Alpha holes in those same triangles must transmit light.
        mesh.alpha = vec![0];
        for v in &mut mesh.vertices {
            v.uv_alpha = [0.5, 0.5, 0., f32::from_bits(1 | 1 << 16)];
        }
        mesh.revision += 1;
        assert!(
            render(&mut scene, &u, &mesh, &[]) > open / 2,
            "transparent foliage texels cast no shadow"
        );
        mesh.alpha[0] = 1;
        mesh.revision += 1;
        assert!(render(&mut scene, &u, &mesh, &[]) < open / 2);
        let light = PointLight {
            position: [2., 2., 0., 0.],
            color: [1., 0., 0., 0.],
            parameters: [20., 10., 0., 0.],
        };
        u.sun_color = [0.; 4];
        u.counts[2] = 1;
        let blocked = render(&mut scene, &u, &mesh, &[light]);
        // Opening the roof at runtime must update local-light shadows too.
        mesh.alpha[0] = 0;
        mesh.revision += 1;
        let local = render(&mut scene, &u, &mesh, &[light]);
        assert!(
            local > 100 && blocked < local / 2,
            "point-light shadow maps: {blocked} vs {local}"
        );
        // Ordinary prelit bitmap art remains byte-for-byte unchanged next to geometry.
        let mut pixel = [77, 113, 158, 255];
        scene
            .render_isometric_geometry(
                &u,
                &mut pixel,
                &[[normal[0][0], 1]],
                &[light],
                &[],
                &[],
                &[],
                Some(&mesh),
            )
            .unwrap();
        assert_eq!(pixel, [77, 113, 158, 255]);
        // Updating geometry, light position and the sun uses the same cached pipelines.
        for v in &mut mesh.vertices {
            v.position[1] = 3.;
        }
        mesh.alpha[0] = 1;
        mesh.revision += 1;
        assert!(
            render(&mut scene, &u, &mesh, &[light]) > 100,
            "roof moved beyond the point light"
        );
        u.sun_color = [1., 1., 1., 0.];
        u.sun = [-0.70710677, 0.70710677, 0., 0.];
        u.counts[2] = 0;
        assert!(
            render(&mut scene, &u, &mesh, &[]) > 100,
            "sun direction is live"
        );
    }
}

#[cfg(all(test, feature = "gpu"))]
mod soft_shadow_tests {
    use super::*;
    #[test]
    #[ignore = "requires native GPU"]
    fn point_shadow_softness_blends_edges_preserves_contact_and_crosses_cube_faces() {
        let mut scene=crate::SceneVM::new(1,1);
        let mut mesh=ShadowGeometry{revision:84,..Default::default()};
        for triangle in [
            [[-0.4,1.,-0.4],[0.4,1.,-0.4],[-0.4,1.,0.4]],
            [[0.4,1.,-0.4],[0.4,1.,0.4],[-0.4,1.,0.4]],
            [[-4.,0.,-4.],[4.,0.,-4.],[-4.,0.,4.]],
            [[4.,0.,-4.],[4.,0.,4.],[-4.,0.,4.]],
        ] {mesh.triangle(triangle);}
        let surfaces=[[128|255<<8|14<<16,0]];
        let mut sample=|x:f32,softness:f32| {
            let u=Uniforms{projection:[0.5-x,0.5+x*0.5,1.,1.],counts:[1,1,1,0],shadow:[10.,1.,0.,1.],..Default::default()};
            let light=PointLight{position:[0.,2.,0.,0.],color:[1.,1.,1.,0.],parameters:[12.,8.,0.,softness]};
            let mut pixel=[160,160,160,255];
            scene.render_isometric_geometry(&u,&mut pixel,&surfaces,&[light],&[],&[],&[],Some(&mesh)).unwrap();pixel[0]
        };
        let hard=sample(0.75,0.);
        let soft=sample(0.75,0.2);
        let clear=sample(1.3,0.2);
        assert!(soft>hard+15&&soft<clear-10,"partial penumbra replaces a sharp edge: hard={hard},soft={soft},clear={clear}");
        assert!(sample(0.,0.2)<20,"fully occluded regions stay dark");
        assert!(clear>100,"the receiving floor does not shadow itself");
        let a=sample(1.99,0.2);let b=sample(2.01,0.2);
        assert!(a>80&&b>80&&a.abs_diff(b)<10,"filtering across cube faces stays continuous: {a} vs {b}");
    }
}
