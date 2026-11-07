//! A fixed GPU mesh pipeline, sharing Eldiron's device. Edits only upload vertices.
use denrim_materials::geometry::{Mesh, Surface};
use std::{sync::mpsc, time::Duration};
use theframework::prelude::*;
use wgpu::util::DeviceExt;

pub struct Job {
    pub revision: u64,
    pub surface_seconds: f32,
    pub mesh: Mesh,
    pub palette: ThePalette,
    pub width: u32,
    pub height: u32,
    pub yaw: f32,
    pub pitch: f32,
    pub background: RGBA,
    pub grip: Option<denrim_materials::geometry::Frame>,
    pub axes: [RGBA; 3],
    pub framing: Option<([f32; 3], [f32; 3])>,
    pub bones: Vec<([f32; 3], [f32; 3])>,
    pub attachment_frames: Vec<denrim_materials::geometry::rig::Pose>,
}
// Camera/animation frames may finish after a newer request. Their epoch identifies
// compatible source/layout; their own projection and pose keep picking aligned.
pub struct Request {
    pub epoch: u64,
    pub job: Job,
    pub projection: super::scene::Projection,
    pub posed: Vec<denrim_materials::geometry::rig::Pose>,
}
pub struct Frame {
    pub epoch: u64,
    pub revision: u64,
    pub projection: super::scene::Projection,
    pub posed: Vec<denrim_materials::geometry::rig::Pose>,
    pub image: Result<TheRGBABuffer, String>,
}
#[derive(Default)]
struct Pending {
    request: Option<Request>,
    closed: bool,
}
pub struct Worker {
    pending: std::sync::Arc<(std::sync::Mutex<Pending>, std::sync::Condvar)>,
    pub results: mpsc::Receiver<Frame>,
}
impl Worker {
    fn spawn<F, R>(create: F) -> Self
    where
        F: FnOnce() -> R + Send + 'static,
        R: FnMut(&Job) -> Result<TheRGBABuffer, String>,
    {
        let pending = std::sync::Arc::new((
            std::sync::Mutex::new(Pending::default()),
            std::sync::Condvar::new(),
        ));
        let input = pending.clone();
        let (done, results) = mpsc::channel();
        std::thread::spawn(move || {
            let mut render = create();
            loop {
                let request = {
                    let (lock, wake) = &*input;
                    let mut p = lock.lock().unwrap();
                    while p.request.is_none() && !p.closed {
                        p = wake.wait(p).unwrap();
                    }
                    if p.closed {
                        break;
                    }
                    p.request.take().unwrap()
                };
                let image = render(&request.job);
                if done
                    .send(Frame {
                        epoch: request.epoch,
                        revision: request.job.revision,
                        projection: request.projection,
                        posed: request.posed,
                        image,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self { pending, results }
    }
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self::spawn(move || {
            let renderer = Renderer::new(&device);
            move |job: &Job| renderer.render(&device, &queue, job)
        })
    }
    pub fn submit(&self, request: Request) {
        let (lock, wake) = &*self.pending;
        lock.lock().unwrap().request = Some(request);
        wake.notify_one();
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let (lock, wake) = &*self.pending;
        let mut p = lock.lock().unwrap();
        p.closed = true;
        p.request = None;
        wake.notify_one();
    }
}
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    textures: std::sync::Mutex<super::textures::Cache>,
    bindings: std::sync::Mutex<Option<(u64, wgpu::BindGroup)>>,
    targets: std::sync::Mutex<Option<Targets>>,
    effects: std::sync::Mutex<rusterix::procedural_avatar::effects::Preview>,
}
struct Targets {
    width: u32,
    height: u32,
    stride: u32,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    depth: wgpu::TextureView,
    read: wgpu::Buffer,
}
impl Targets {
    fn new(device: &wgpu::Device, w: u32, h: u32) -> Self {
        let descriptor = wgpu::TextureDescriptor {
            label: Some("MZ mesh preview"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let target = device.create_texture(&descriptor);
        let view = target.create_view(&Default::default());
        let depth = device
            .create_texture(&wgpu::TextureDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                ..descriptor
            })
            .create_view(&Default::default());
        let stride = (w * 4).div_ceil(256) * 256;
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MZ mesh readback"),
            size: stride as u64 * h as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self {
            width: w,
            height: h,
            stride,
            target,
            view,
            depth,
            read,
        }
    }
}
fn linear(v: u8) -> f32 {
    let s = v as f32 / 255.;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
impl Renderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MZ geometry preview"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:Some("MZ geometry preview"),layout:None,
            vertex:wgpu::VertexState{module:&shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[wgpu::VertexBufferLayout{array_stride:60,step_mode:wgpu::VertexStepMode::Vertex,attributes:&wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x3,2=>Float32x3,3=>Float32x2,4=>Float32x2,5=>Float32]}]},
            fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState{format:wgpu::TextureFormat::Rgba8UnormSrgb,blend:None,write_mask:wgpu::ColorWrites::ALL})]}),
            primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},
            depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::Less),stencil:Default::default(),bias:Default::default()}),
            multisample:Default::default(),multiview_mask:None,cache:None,
        });
        Self {
            pipeline,
            textures: Default::default(),
            bindings: Default::default(),
            targets: Default::default(),
            effects: Default::default(),
        }
    }
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        job: &Job,
    ) -> Result<TheRGBABuffer, String> {
        let (w, h) = (job.width.max(16), job.height.max(16));
        if w > device.limits().max_texture_dimension_2d
            || h > device.limits().max_texture_dimension_2d
        {
            return Err("Geometry viewport exceeds the GPU texture size limit".into());
        }
        let (lo, hi) = job.framing.unwrap_or_else(|| job.mesh.bounds());
        let center: [f32; 3] = std::array::from_fn(|i| (lo[i] + hi[i]) * 0.5);
        let extent = (0..3)
            .map(|i| (hi[i] - lo[i]).powi(2))
            .sum::<f32>()
            .sqrt()
            .max(0.01)
            * 0.58;
        let aspect = w as f32 / h as f32;
        let (sy, cy) = job.yaw.to_radians().sin_cos();
        let (sp, cp) = job.pitch.to_radians().sin_cos();
        let rotate = |p: [f32; 3]| {
            let x = cy * p[0] + sy * p[2];
            let z = -sy * p[0] + cy * p[2];
            [x, cp * p[1] - sp * z, sp * p[1] + cp * z]
        };
        let mut specs = vec![];
        for part in &job.mesh.parts {
            if let Some(spec) = &part.texture {
                if !specs.contains(&spec) {
                    specs.push(spec);
                }
            }
        }
        if specs.len() > 16 {
            return Err("A preview supports at most 16 material textures".into());
        }
        let frames = {
            let mut cache = self
                .textures
                .lock()
                .map_err(|_| "Texture cache is unavailable")?;
            specs
                .iter()
                .map(|s| {
                    cache
                        .frame(s, &job.palette, job.surface_seconds)
                        .map(|f| f.0)
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let binding_key =
            super::fingerprint(&format!("{specs:?}:{frames:?}:{:?}", job.palette.colors));
        let bindings = {
            let mut cached = self
                .bindings
                .lock()
                .map_err(|_| "Material bindings are unavailable")?;
            if let Some((_, b)) = cached.as_ref().filter(|(key, _)| *key == binding_key) {
                b.clone()
            } else {
                let size = specs.iter().map(|s| s.size).max().unwrap_or(16);
                let layers = specs.len() as u32 + 1;
                let mut colors = vec![255u8; (size * size * layers * 4) as usize];
                let mut data = colors.clone();
                let mut emission = vec![0u8; colors.len()];
                let mut cache = self
                    .textures
                    .lock()
                    .map_err(|_| "Texture cache is unavailable")?;
                for (i, spec) in specs.iter().enumerate() {
                    let images =
                        cache.image(spec, &job.palette, device, queue, job.surface_seconds)?;
                    for y in 0..size {
                        for x in 0..size {
                            let from = (((y * spec.size / size) * spec.size + x * spec.size / size)
                                * 4) as usize;
                            let to = (((i as u32 + 1) * size * size + y * size + x) * 4) as usize;
                            colors[to..to + 4].copy_from_slice(&images.color[from..from + 4]);
                            data[to..to + 4].copy_from_slice(&images.data[from..from + 4]);
                            emission[to..to + 4].copy_from_slice(&images.emission[from..from + 4]);
                        }
                    }
                }
                drop(cache);
                let upload = |bytes: &[u8], format| {
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Appearance texture atlas"),
                        size: wgpu::Extent3d {
                            width: size,
                            height: size,
                            depth_or_array_layers: layers,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });
                    queue.write_texture(
                        texture.as_image_copy(),
                        bytes,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(size * 4),
                            rows_per_image: Some(size),
                        },
                        texture.size(),
                    );
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2Array),
                        ..Default::default()
                    })
                };
                let color = upload(&colors, wgpu::TextureFormat::Rgba8UnormSrgb);
                let material = upload(&data, wgpu::TextureFormat::Rgba8Unorm);
                let light = upload(&emission, wgpu::TextureFormat::Rgba8UnormSrgb);
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    mag_filter: wgpu::FilterMode::Nearest,
                    min_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                });
                let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Appearance material maps"),
                    layout: &self.pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&color),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&material),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&light),
                        },
                    ],
                });
                *cached = Some((binding_key, bindings.clone()));
                bindings
            }
        };
        let effects_mesh =
            self.effects
                .lock()
                .unwrap()
                .mesh(&job.mesh, &job.palette, job.yaw, job.pitch);
        let mut floats = Vec::with_capacity(effects_mesh.vertex_count() * 15);
        for part in &effects_mesh.parts {
            let surface = part.surface.clone().unwrap_or(Surface {
                palette: 7,
                roughness: 0.5,
                metallic: 0.,
            });
            let color = job
                .palette
                .colors
                .get(surface.palette as usize)
                .and_then(Option::as_ref)
                .map(TheColor::to_u8_array)
                .unwrap_or([128, 128, 128, 255]);
            let color = [linear(color[0]), linear(color[1]), linear(color[2])];
            for v in &part.vertices {
                let p = rotate(std::array::from_fn(|i| v.position[i] - center[i]));
                floats.extend([
                    p[0] / (extent * aspect.max(1.)),
                    p[1] / (extent / (aspect.min(1.))),
                    0.5 - p[2] / (extent * 8.),
                    1.,
                ]);
                floats.extend(rotate(v.normal));
                floats.extend(color);
                floats.extend([surface.roughness, surface.metallic]);
                floats.extend(v.uv);
                floats.push(if part.id.starts_with("@particle/") {
                    -1.
                } else {
                    part.texture
                        .as_ref()
                        .and_then(|t| specs.iter().position(|s| *s == t))
                        .map_or(0., |i| (i + 1) as f32)
                });
            }
        }
        let data = floats
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect::<Vec<_>>();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MZ mesh vertices"),
            contents: &data,
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Reuse large color/depth/readback allocations while the viewport size is stable.
        let mut storage = self
            .targets
            .lock()
            .map_err(|_| "Preview targets are unavailable")?;
        if storage
            .as_ref()
            .is_none_or(|t| t.width != w || t.height != h)
        {
            *storage = Some(Targets::new(device, w, h));
        }
        let targets = storage.as_ref().unwrap();
        let (target, view, depth_view, read, stride) = (
            &targets.target,
            &targets.view,
            &targets.depth,
            &targets.read,
            targets.stride,
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let bg = job.background;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MZ mesh preview"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: linear(bg[0]) as f64,
                            g: linear(bg[1]) as f64,
                            b: linear(bg[2]) as f64,
                            a: bg[3] as f64 / 255.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            if !floats.is_empty() {
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.draw(0..floats.len() as u32 / 15, 0..1);
            }
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(h),
                },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::channel();
        read.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(1))
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let mapped = read.get_mapped_range(..);
        let mut pixels = Vec::with_capacity((w * h * 4) as usize);
        for row in mapped.chunks_exact(stride as usize) {
            pixels.extend_from_slice(&row[..w as usize * 4]);
        }
        drop(mapped);
        read.unmap();
        let project = |p: [f32; 3]| {
            let p = rotate(std::array::from_fn(|i| p[i] - center[i]));
            [
                (p[0] / (extent * aspect.max(1.)) * 0.5 + 0.5) * w as f32,
                (0.5 - p[1] / (extent / aspect.min(1.)) * 0.5) * h as f32,
            ]
        };
        for (a, b) in &job.bones {
            let a = project(*a);
            let b = project(*b);
            let length = (b[0] - a[0]).hypot(b[1] - a[1]).max(0.001);
            let normal = [(a[1] - b[1]) / length, (b[0] - a[0]) / length];
            for offset in [-1., 0., 1.] {
                super::presentation::line(
                    &mut pixels,
                    w as usize,
                    h as usize,
                    [a[0] + normal[0] * offset, a[1] + normal[1] * offset],
                    [b[0] + normal[0] * offset, b[1] + normal[1] * offset],
                    job.axes[0],
                );
            }
        }
        let mut frames = job.attachment_frames.clone();
        if let Some(frame) = &job.grip {
            frames.push(denrim_materials::geometry::rig::Pose::from_frame(frame));
        }
        for frame in frames {
            let origin = project(frame.translation);
            for axis in 0..3 {
                let mut p = [0.; 3];
                p[axis] = extent * 0.18;
                let end = project(frame.point(p));
                super::presentation::line(
                    &mut pixels,
                    w as usize,
                    h as usize,
                    origin,
                    end,
                    job.axes[axis],
                );
            }
        }
        Ok(TheRGBABuffer::from(pixels, w, h))
    }
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;
    fn request(revision: u64) -> Request {
        Request {
            epoch: 7,
            projection: super::super::scene::Projection::new(
                ([-1.; 3], [1.; 3]),
                16,
                16,
                revision as f32,
                0.,
            ),
            posed: vec![],
            job: Job {
                revision,
                surface_seconds: 0.,
                mesh: Mesh::default(),
                palette: ThePalette::default(),
                width: 16,
                height: 16,
                yaw: revision as f32,
                pitch: 0.,
                background: [0; 4],
                grip: None,
                axes: [[0; 4]; 3],
                framing: None,
                bones: vec![],
                attachment_frames: vec![],
            },
        }
    }
    #[test]
    fn in_flight_frame_finishes_while_pending_requests_coalesce_to_the_latest() {
        let (started, started_rx) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let worker = Worker::spawn(move || {
            move |job: &Job| {
                if job.revision == 1 {
                    started.send(()).unwrap();
                    release_rx.recv().unwrap();
                }
                Ok(TheRGBABuffer::new(TheDim::new(0, 0, 16, 16)))
            }
        });
        worker.submit(request(1));
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.submit(request(2));
        worker.submit(request(3));
        release.send(()).unwrap();
        let first = worker.results.recv_timeout(Duration::from_secs(2)).unwrap();
        let last = worker.results.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!((first.revision, last.revision), (1, 3));
        assert_eq!(last.epoch, 7);
        assert!(worker.results.try_recv().is_err());
    }
}
