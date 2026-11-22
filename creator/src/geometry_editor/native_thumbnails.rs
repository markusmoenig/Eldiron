//! Geometry inspection cards render into immutable shared-device images. CPU
//! work prepares vertices/paint descriptions, never a thumbnail pixel buffer.
use super::*;
use renderkit::{
    AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, ImageHandle, ImageRegistry,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use theframework::paint::Rect;
use theframework::{paint::*, render_wgpu::PaintRenderer};
pub(super) const SIDE: u32 = 64;
const MAX_IMAGES: usize = 8192; // 128 MiB of immutable visible RGBA outputs.
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}
enum Plan {
    Paint(PaintList),
    Triangles {
        background: RGBA,
        vertices: Vec<Vertex>,
    },
}
fn plan(
    value: &Value,
    palette: &ThePalette,
    theme: &dyn TheTheme,
    text: &mut theframework::text::TextSystem,
) -> Plan {
    let background = *theme.color(MaterialControl);
    let color = |role| theme.color(role).map(|c| c as f32 / 255.);
    let mut list = PaintList::default();
    list.rect(Rect::new(0., 0., 64., 64.), 0., color(MaterialControl));
    let stroke = |list: &mut PaintList, a: [f32; 2], b: [f32; 2], role| {
        list.ops.push(PaintOp::Stroke {
            path: Arc::new(Path {
                elements: vec![PathElement::Move(a), PathElement::Line(b)],
            }),
            stroke: Stroke {
                width: 1.,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                brush: color(role).into(),
            },
        });
    };
    match value {
        Value::Contour(points) => {
            return Plan::Paint(presentation::contour_paint(points, 64, 64, theme, None, text).0);
        }
        Value::Rig(rig) | Value::Motion(denrim_materials::geometry::rig::Motion { rig, .. }) => {
            let poses = rig.rest();
            let lo: [f32; 3] = std::array::from_fn(|k| {
                poses
                    .iter()
                    .map(|p| p.translation[k])
                    .fold(f32::INFINITY, f32::min)
            });
            let hi: [f32; 3] = std::array::from_fn(|k| {
                poses
                    .iter()
                    .map(|p| p.translation[k])
                    .fold(f32::NEG_INFINITY, f32::max)
            });
            let scale = 48. / (hi[1] - lo[1]).max(hi[0] - lo[0]).max(0.1);
            let project = |p: [f32; 3]| {
                [
                    32. + (p[0] - (lo[0] + hi[0]) / 2.) * scale,
                    32. - (p[1] - (lo[1] + hi[1]) / 2.) * scale,
                ]
            };
            for (i, joint) in rig.joints.iter().enumerate() {
                if let Some(parent) = joint.parent {
                    stroke(
                        &mut list,
                        project(poses[i].translation),
                        project(poses[parent].translation),
                        MaterialWire,
                    );
                }
            }
        }
        Value::Grip(_) => {
            stroke(&mut list, [12., 44.], [52., 44.], SelectedTextEditBorder1);
            stroke(&mut list, [32., 54.], [32., 10.], MaterialText);
        }
        Value::Material(surface) => {
            let mut vertices = Vec::new();
            if let Some(color) = palette
                .colors
                .get(surface.palette as usize)
                .and_then(Option::as_ref)
            {
                let color = color.to_u8_array().map(|v| v as f32 / 255.);
                for [x, y] in [
                    [7., 7.],
                    [57., 7.],
                    [7., 57.],
                    [7., 57.],
                    [57., 7.],
                    [57., 57.],
                ] {
                    vertices.push(Vertex {
                        position: [x, y, 0.5],
                        color,
                    });
                }
            }
            return Plan::Triangles {
                background,
                vertices,
            };
        }
        Value::Mesh(mesh) => {
            let (lo, hi) = mesh.bounds();
            let center: [f32; 3] = std::array::from_fn(|i| (lo[i] + hi[i]) / 2.);
            let extent = (0..3)
                .map(|i| (hi[i] - lo[i]).powi(2))
                .sum::<f32>()
                .sqrt()
                .max(0.01)
                * 0.6;
            let project = |p: [f32; 3]| {
                let p: [f32; 3] = std::array::from_fn(|i| p[i] - center[i]);
                let x = 0.866 * p[0] - 0.5 * p[2];
                let z = 0.5 * p[0] + 0.866 * p[2];
                // Preserve the reference's pixel-space rounding before converting
                // to clip coordinates; algebraic cancellation moves shared edges.
                let px = 32. + x / extent * 29.;
                let py = 32. - (0.98 * p[1] - 0.2 * z) / extent * 29.;
                [px, py, z]
            };
            let mut vertices = Vec::new();
            for part in &mesh.parts {
                let base = part
                    .surface
                    .as_ref()
                    .and_then(|s| palette.colors.get(s.palette as usize))
                    .and_then(Option::as_ref)
                    .map(TheColor::to_u8_array)
                    .unwrap_or(*theme.color(MaterialText));
                for triangle in part.vertices.chunks_exact(3) {
                    let normal = triangle[0].normal;
                    let shade = (0.4
                        + 0.6 * (normal[0] * -0.4 + normal[1] * 0.5 + normal[2] * 0.7).abs())
                    .clamp(0., 1.);
                    let color = [
                        (base[0] as f32 * shade) as u8,
                        (base[1] as f32 * shade) as u8,
                        (base[2] as f32 * shade) as u8,
                        255,
                    ]
                    .map(|v| v as f32 / 255.);
                    for v in triangle {
                        vertices.push(Vertex {
                            position: project(v.position),
                            color,
                        });
                    }
                }
            }
            return Plan::Triangles {
                background,
                vertices,
            };
        }
    }
    Plan::Paint(list)
}
// A small tiled GPU raster pass retains the reference's raw f32 depth and
// triangle order. Normalized hardware depth loses low bits on coplanar faces.
// CPU work bins triangle bounds, not pixels; each GPU lane owns one output pixel.
pub(super) struct Renderer {
    context: Arc<GpuContext>,
    paint: PaintRenderer,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}
impl Renderer {
    pub(super) fn new(context: Arc<GpuContext>) -> Result<Self, String> {
        let mut entries = Vec::new();
        for binding in 0..=2 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: wgpu::TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("geometry card tiled raster"),
                entries: &entries,
            });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("geometry card tiled raster"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
struct Triangle {a:vec4<f32>,b:vec4<f32>,c:vec4<f32>,color:vec4<u32>};
@group(0) @binding(0) var<storage,read> triangles:array<Triangle>;
@group(0) @binding(1) var<storage,read> bins:array<vec2<u32>>;
@group(0) @binding(2) var<storage,read> indices:array<u32>;
@group(0) @binding(3) var output:texture_storage_2d<rgba8unorm,write>;
@group(0) @binding(4) var<uniform> background:vec4<u32>;
fn edge(a:vec2<f32>,b:vec2<f32>,p:vec2<f32>)->f32 {
    return (b.x-a.x)*(p.y-a.y)-(b.y-a.y)*(p.x-a.x);
}
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
    let p=vec2<f32>(id.xy)+vec2<f32>(0.5);
    let bin=bins[id.y/8u*8u+id.x/8u];
    var depth=-3.402823e38;
    var color=background;
    for(var at=0u;at<bin.y;at++) {
        let t=triangles[indices[bin.x+at]];
        let w=vec3<f32>(edge(t.b.xy,t.c.xy,p),edge(t.c.xy,t.a.xy,p),edge(t.a.xy,t.b.xy,p))/t.a.w;
        if(any(w<vec3<f32>(0.))){continue;}
        let z=(w.x*t.a.z+w.y*t.b.z)+w.z*t.c.z;
        if(z<=depth){continue;}
        depth=z;color=t.color;
    }
    textureStore(output,vec2<i32>(id.xy),vec4<f32>(color)/255.);
}
"#
                    .into(),
                ),
            });
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("geometry card tiled raster"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("geometry card tiled raster"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        Ok(Self {
            paint: PaintRenderer::new(context.clone()).map_err(|e| e.to_string())?,
            context,
            pipeline,
            layout,
        })
    }
    pub(super) fn render_value(
        &mut self,
        value: &Value,
        palette: &ThePalette,
        theme: &dyn TheTheme,
        text: &mut theframework::text::TextSystem,
        images: &ImageRegistry,
    ) -> Result<Arc<GpuImage>, String> {
        self.render(&plan(value, palette, theme, text), images)
    }
    fn render(&mut self, plan: &Plan, images: &ImageRegistry) -> Result<Arc<GpuImage>, String> {
        if self.context.is_lost() {
            return Err("Geometry card GPU device was lost".into());
        }
        let Plan::Triangles {
            background,
            vertices,
        } = plan
        else {
            let Plan::Paint(list) = plan else {
                unreachable!()
            };
            let image = GpuImage::new(&self.context, ImageDescriptor::rgba(SIDE, SIDE))
                .map_err(|e| e.to_string())?;
            self.paint
                .render(
                    list,
                    images,
                    &image.view,
                    image.descriptor.format,
                    (SIDE, SIDE),
                    1.,
                    [0.; 4],
                )
                .map_err(|e| e.to_string())?;
            return Ok(image);
        };
        if vertices.len() > 1_000_000
            || vertices
                .iter()
                .flat_map(|v| v.position.iter().chain(&v.color))
                .any(|v| !v.is_finite())
        {
            return Err("Invalid or oversized geometry card vertex source".into());
        }
        let mut bins: [Vec<u32>; 64] = std::array::from_fn(|_| Vec::new());
        let mut triangles = Vec::new();
        let mut triangle_count = 0u32;
        let mut references = 0usize;
        for triangle in vertices.chunks_exact(3) {
            let [a, b, c] = std::array::from_fn::<_, 3, _>(|i| triangle[i].position);
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if area.abs() < 1e-5 {
                continue;
            }
            let lo: [usize; 2] = std::array::from_fn(|k| {
                triangle
                    .iter()
                    .map(|v| v.position[k])
                    .fold(f32::INFINITY, f32::min)
                    .floor()
                    .max(0.) as usize
            });
            let hi: [usize; 2] = std::array::from_fn(|k| {
                triangle
                    .iter()
                    .map(|v| v.position[k])
                    .fold(f32::NEG_INFINITY, f32::max)
                    .ceil()
                    .clamp(0., 63.) as usize
            });
            if lo[0] > hi[0] || lo[1] > hi[1] {
                continue;
            }
            for y in lo[1] / 8..=hi[1] / 8 {
                for x in lo[0] / 8..=hi[0] / 8 {
                    bins[y * 8 + x].push(triangle_count);
                    references += 1;
                }
            }
            if references > 8 * 1024 * 1024 {
                return Err(
                    "Geometry card bin references exceed their 32 MiB source budget".into(),
                );
            }
            for row in [
                [a[0], a[1], a[2], area],
                [b[0], b[1], b[2], 0.],
                [c[0], c[1], c[2], 0.],
            ] {
                for value in row {
                    triangles.extend(value.to_ne_bytes());
                }
            }
            for value in triangle[0].color {
                triangles.extend(((value * 255.).round() as u32).to_ne_bytes());
            }
            triangle_count += 1;
        }
        let mut headers = Vec::new();
        let mut indices = Vec::new();
        let mut count = 0u32;
        for bin in bins {
            headers.extend(count.to_ne_bytes());
            headers.extend((bin.len() as u32).to_ne_bytes());
            count += bin.len() as u32;
            for index in bin {
                indices.extend(index.to_ne_bytes());
            }
        }
        if triangles.is_empty() {
            triangles.resize(64, 0);
        }
        if indices.is_empty() {
            indices.resize(4, 0);
        }
        let upload = |_label, contents: &[u8], usage| {
            self.context
                .upload_immutable_buffer(contents, usage)
                .map_err(|e| e.to_string())
        };
        let triangle_buffer = upload(
            "geometry card triangles",
            &triangles,
            wgpu::BufferUsages::STORAGE,
        )?;
        let headers = upload(
            "geometry card bin ranges",
            &headers,
            wgpu::BufferUsages::STORAGE,
        )?;
        let indices = upload(
            "geometry card bin references",
            &indices,
            wgpu::BufferUsages::STORAGE,
        )?;
        let background_bytes: Vec<_> = background
            .iter()
            .flat_map(|v| (*v as u32).to_ne_bytes())
            .collect();
        let background = upload(
            "geometry card background",
            &background_bytes,
            wgpu::BufferUsages::UNIFORM,
        )?;
        let descriptor = ImageDescriptor {
            width: SIDE,
            height: SIDE,
            format: wgpu::TextureFormat::Rgba8Unorm,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        };
        let mut target = renderkit::TargetDescriptor::color(SIDE, SIDE, descriptor.format);
        target.usage = wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        let lease = self
            .context
            .acquire_target(target)
            .map_err(|e| e.to_string())?;
        let image =
            GpuImage::from_target(&self.context, lease, descriptor).map_err(|e| e.to_string())?;
        let bindings = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("geometry card tiled raster"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: triangle_buffer.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: headers.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: indices.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&image.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: background.buffer().as_entire_binding(),
                    },
                ],
            });
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("geometry card tiled raster"),
                });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("geometry card tiled raster"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.dispatch_workgroups(8, 8, 1);
        }
        self.context.queue.submit([encoder.finish()]);
        Ok(image)
    }
}
#[derive(Default)]
pub(super) struct Cache {
    owner: Option<u64>,
    key: Option<(String, [RGBA; 5])>,
    images: HashMap<String, ImageHandle>,
    failed: HashSet<String>,
    retired: Vec<ImageHandle>,
    renderer: Option<Renderer>,
}
pub(super) struct Output {
    pub handles: HashMap<String, ImageHandle>,
    pub waiting: bool,
    pub errors: Vec<String>,
}
impl Cache {
    pub fn clear(&mut self) {
        self.retired.extend(self.images.drain().map(|(_, h)| h));
        self.failed.clear();
        self.key = None;
        self.renderer = None;
    }
    #[cfg(test)]
    pub(super) fn handles(&self) -> HashMap<String, ImageHandle> {
        self.images.clone()
    }
    pub fn prepare(
        &mut self,
        images: &mut ImageRegistry,
        key: Option<&str>,
        evaluation: Option<&Evaluation>,
        palette: &ThePalette,
        theme: &dyn TheTheme,
        text: &mut theframework::text::TextSystem,
        keys: &[String],
    ) -> Result<Output, String> {
        if self.owner != Some(images.owner_id()) {
            self.clear();
            self.retired.clear();
            self.owner = Some(images.owner_id());
        }
        let colors = [
            MaterialControl,
            MaterialCanvas,
            MaterialWire,
            MaterialText,
            SelectedTextEditBorder1,
        ]
        .map(|r| *theme.color(r));
        let domain = key.map(|k| (k.to_owned(), colors));
        if self.key != domain {
            self.clear();
            self.key = domain;
        }
        for old in self.retired.drain(..) {
            images.remove(old);
        }
        if keys.len() > MAX_IMAGES {
            return Err("Geometry cards exceed the 128 MiB visible-output budget".into());
        }
        let active: HashSet<_> = keys.iter().cloned().collect();
        self.images.retain(|key, handle| {
            if active.contains(key) {
                true
            } else {
                images.remove(*handle);
                false
            }
        });
        self.failed.retain(|key| active.contains(key));
        let mut errors = Vec::new();
        if let Some(evaluation) = evaluation.filter(|_| self.key.is_some()) {
            if let Some(key) = keys
                .iter()
                .find(|key| !self.images.contains_key(*key) && !self.failed.contains(*key))
            {
                let result = (|| {
                    let value = evaluation
                        .operations
                        .get(key)
                        .ok_or("Geometry operation is missing")?;
                    if self.renderer.is_none() {
                        self.renderer = Some(Renderer::new(images.context().clone())?);
                    }
                    let plan = plan(value, palette, theme, text);
                    let image = self.renderer.as_mut().unwrap().render(&plan, images)?;
                    images.insert(image).map_err(|e| e.to_string())
                })();
                match result {
                    Ok(handle) => {
                        self.images.insert(key.clone(), handle);
                    }
                    Err(error) => {
                        self.failed.insert(key.clone());
                        errors.push(format!("Geometry card {key}: {error}"));
                    }
                }
            }
        }
        if active.is_empty() {
            self.renderer = None;
        }
        let waiting = evaluation.is_some()
            && self.key.is_some()
            && keys
                .iter()
                .any(|key| !self.images.contains_key(key) && !self.failed.contains(key));
        Ok(Output {
            handles: self.images.clone(),
            waiting,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capture(gpu: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
        let mut read = renderkit::ReadbackTicket::capture(gpu, image).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        read.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
    }
    fn complete(
        cache: &mut Cache,
        images: &mut ImageRegistry,
        key: &str,
        evaluation: &Evaluation,
        palette: &ThePalette,
        theme: &dyn TheTheme,
        text: &mut theframework::text::TextSystem,
        keys: &[String],
    ) -> Output {
        for _ in 0..=keys.len() {
            let output = cache
                .prepare(
                    images,
                    Some(key),
                    Some(evaluation),
                    palette,
                    theme,
                    text,
                    keys,
                )
                .unwrap();
            assert!(output.errors.is_empty(), "{:?}", output.errors);
            if !output.waiting {
                return output;
            }
        }
        panic!("geometry cards did not complete");
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_geometry_graph_cache_retires_hidden_sources_and_recovers_without_display_transfers() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut images = ImageRegistry::new(gpu.clone());
        let mut cache = Cache::default();
        let palette = Project::default().palette;
        let theme = TheBlackBlueTheme::new();
        let mut text = theframework::text::TextSystem::default();
        let evaluation = Document::load(denrim_materials::geometry::SWORD_EXAMPLE)
            .unwrap()
            .evaluate()
            .unwrap();
        let keys: Vec<_> = evaluation.operations.keys().cloned().collect();
        assert!(keys.len() > 3);
        let before = gpu.transfers();
        let first = complete(
            &mut cache,
            &mut images,
            "source-a",
            &evaluation,
            &palette,
            &theme,
            &mut text,
            &keys,
        );
        assert_eq!(first.handles.len(), keys.len());
        assert_eq!(
            gpu.transfers(),
            before,
            "cards publish without image uploads, status maps or color readback"
        );
        let old = images.get(first.handles[&keys[0]]).unwrap();
        let old_pixels = capture(&gpu, old.clone());
        let before = gpu.transfers();
        assert_eq!(
            complete(
                &mut cache,
                &mut images,
                "source-a",
                &evaluation,
                &palette,
                &theme,
                &mut text,
                &keys
            )
            .handles,
            first.handles
        );
        assert_eq!(gpu.transfers(), before);
        let one = complete(
            &mut cache,
            &mut images,
            "source-a",
            &evaluation,
            &palette,
            &theme,
            &mut text,
            &keys[..1],
        );
        assert_eq!(one.handles.len(), 1);
        for key in &keys[1..] {
            assert!(images.get(first.handles[key]).is_none());
        }
        let second = complete(
            &mut cache,
            &mut images,
            "source-b",
            &evaluation,
            &palette,
            &theme,
            &mut text,
            &keys,
        );
        assert!(images.get(one.handles[&keys[0]]).is_none());
        let foreign = second.handles[&keys[0]];
        let mut replacement =
            ImageRegistry::new(pollster::block_on(GpuContext::headless()).unwrap());
        let rebuilt = complete(
            &mut cache,
            &mut replacement,
            "source-b",
            &evaluation,
            &palette,
            &theme,
            &mut text,
            &keys,
        );
        assert!(
            images.get(foreign).is_some(),
            "never retire another registry's handle"
        );
        assert_eq!(
            replacement.context().transfers(),
            renderkit::TransferCounters::default()
        );
        let other_theme = TheBrownTheme::new();
        let themed = complete(
            &mut cache,
            &mut replacement,
            "source-b",
            &evaluation,
            &palette,
            &other_theme,
            &mut text,
            &keys,
        );
        for handle in rebuilt.handles.values() {
            assert!(replacement.get(*handle).is_none());
        }
        assert_eq!(themed.handles.len(), keys.len());
        let hidden = cache
            .prepare(
                &mut replacement,
                Some("source-b"),
                Some(&evaluation),
                &palette,
                &other_theme,
                &mut text,
                &[],
            )
            .unwrap();
        assert!(hidden.handles.is_empty() && !hidden.waiting);
        assert!(cache.renderer.is_none());
        let reopened = complete(
            &mut cache,
            &mut replacement,
            "source-b",
            &evaluation,
            &palette,
            &theme,
            &mut text,
            &keys,
        );
        cache.clear();
        let reset = cache
            .prepare(
                &mut replacement,
                None,
                None,
                &palette,
                &theme,
                &mut text,
                &[],
            )
            .unwrap();
        assert!(reset.handles.is_empty() && !reset.waiting);
        for handle in reopened.handles.values() {
            assert!(replacement.get(*handle).is_none());
        }
        assert_eq!(
            capture(&gpu, old),
            old_pixels,
            "retained consumers keep immutable old pixels"
        );
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_geometry_graph_thumbnails_preserve_projection_palette_alpha_and_vector_coverage() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let images = ImageRegistry::new(gpu.clone());
        let mut renderer = Renderer::new(gpu.clone()).unwrap();
        let mut text = theframework::text::TextSystem::default();
        let theme = TheBlackBlueTheme::new();
        let mut palette = Project::default().palette;
        palette.colors[0] = Some(TheColor::new(0.8, 0.1, 0.4, 0.3));
        let mut values = Vec::new();
        for source in [
            denrim_materials::geometry::SWORD_EXAMPLE,
            denrim_materials::geometry::HUMAN_EXAMPLE,
        ] {
            let evaluation = Document::load(source).unwrap().evaluate().unwrap();
            for (_, value) in evaluation.operations {
                if !values.iter().any(|v: &Value| v.kind() == value.kind()) {
                    values.push(value);
                }
            }
            values.push(Value::Mesh(evaluation.mesh));
        }
        let rig = denrim_materials::geometry::rig::Rig {
            joints: [[0., 0., 0.], [0.4, 1., 0.], [-0.7, 0.9, 0.]]
                .into_iter()
                .enumerate()
                .map(|(i, translation)| denrim_materials::geometry::rig::Joint {
                    id: format!("joint-{i}"),
                    label: format!("Joint {i}"),
                    parent: i.checked_sub(1),
                    local: denrim_materials::geometry::rig::Pose {
                        translation,
                        rotation: [0., 0., 0., 1.],
                    },
                })
                .collect(),
            tip: None,
        };
        values.push(Value::Rig(rig.clone()));
        values.push(Value::Motion(denrim_materials::geometry::rig::Motion {
            rig,
            duration: 1.,
            looping: true,
            max_cycles_per_second: None,
            tracks: Vec::new(),
            events: Vec::new(),
        }));
        values.push(Value::Material(denrim_materials::geometry::Surface {
            palette: 0,
            roughness: 0.5,
            metallic: 0.,
        }));
        let kinds: HashSet<_> = values.iter().map(Value::kind).collect();
        for kind in ["Contour", "Mesh", "Material", "Grip", "Rig"] {
            assert!(kinds.contains(kind), "missing {kind} fixture");
        }
        let mut review = vec![0; 128 * 64 * values.len() * 4];
        for (row, value) in values.iter().enumerate() {
            let reference = presentation::thumbnail(value, &palette, &theme);
            let before = gpu.transfers();
            let image = renderer
                .render(&plan(value, &palette, &theme, &mut text), &images)
                .unwrap();
            assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
            assert_eq!(gpu.transfers().uploaded_bytes, before.uploaded_bytes);
            let actual = capture(&gpu, image);
            if matches!(value, Value::Material(_) | Value::Mesh(_)) {
                let differences: Vec<_> = actual
                    .chunks_exact(4)
                    .zip(reference.pixels().chunks_exact(4))
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, (a, b))| (i, a.to_vec(), b.to_vec()))
                    .collect();
                assert!(
                    differences.is_empty(),
                    "{}: {} mismatches; first {:?}",
                    value.kind(),
                    differences.len(),
                    differences.iter().take(8).collect::<Vec<_>>()
                );
            } else {
                // The common vector renderer antialiases lines/circles. Require
                // identical spatial features within two source pixels, then
                // inspect the paired independent-reference/native captures.
                let background = if matches!(value, Value::Contour(_)) {
                    *theme.color(MaterialCanvas)
                } else {
                    *theme.color(MaterialControl)
                };
                let marked = |image: &[u8], x: usize, y: usize| {
                    image[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4] != background
                };
                for (a, b) in [
                    (&actual[..], reference.pixels()),
                    (reference.pixels(), &actual[..]),
                ] {
                    for y in 0..64 {
                        for x in 0..64 {
                            if marked(a, x, y) {
                                assert!(
                                    (y.saturating_sub(2)..=(y + 2).min(63))
                                        .any(|v| (x.saturating_sub(2)..=(x + 2).min(63))
                                            .any(|u| marked(b, u, v))),
                                    "{} feature at {x},{y} moved",
                                    value.kind()
                                );
                            }
                        }
                    }
                }
            }
            for y in 0..64 {
                let at = ((row * 64 + y) * 128) * 4;
                review[at..at + 64 * 4]
                    .copy_from_slice(&reference.pixels()[y * 64 * 4..(y + 1) * 64 * 4]);
                review[at + 64 * 4..at + 128 * 4]
                    .copy_from_slice(&actual[y * 64 * 4..(y + 1) * 64 * 4]);
            }
        }
        std::fs::write(
            "/private/tmp/eldiron-native-geometry-thumbnail-pairs.png",
            TheRGBABuffer::from(review, 128, 64 * values.len() as u32)
                .to_png()
                .unwrap(),
        )
        .unwrap();
    }
}
