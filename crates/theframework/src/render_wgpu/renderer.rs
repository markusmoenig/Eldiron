use super::{
    atlas::GlyphAtlas,
    mesh::{self, Vertex},
};
use crate::{paint::*, text::GlyphCache};
use renderkit::{
    AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, ImageRegistry, RenderError,
    Result, wgpu,
};
use std::{collections::HashMap, sync::Arc};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    viewport: [f32; 2],
    mode: u32,
    source: u32,
    color0: Color,
    color1: Color,
    gradient: [f32; 4],
}
#[derive(Clone)]
struct Geometry {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    colored: bool,
}
#[derive(Clone, Copy)]
enum ImageEffect {
    Group,
    Present(bool),
    Filter(ImageColorFilter),
    Threshold {
        dark: Color,
        light: Color,
        threshold: u8,
    },
    Outline {
        uv: Rect,
        step: Point,
        width: f32,
    },
}
struct Draw {
    geometry: Geometry,
    bind: wgpu::BindGroup,
    stencil: u32,
    phase: u8,
}
struct Pipelines {
    draw: wgpu::RenderPipeline,
    push: wgpu::RenderPipeline,
    pop: wgpu::RenderPipeline,
    samples: u32,
}
fn linear(c: Color) -> Color {
    let f = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

/// Work submitted by the latest top-level render, including isolated opacity passes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub draw_calls: usize,
    pub glyph_quads: usize,
}

struct GlyphBatch {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    image: Arc<GpuImage>,
    color: Color,
}

pub struct PaintRenderer {
    context: Arc<GpuContext>,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    samplers: HashMap<Sampling, wgpu::Sampler>,
    white: Arc<GpuImage>,
    empty_field: wgpu::Buffer,
    pipelines: HashMap<wgpu::TextureFormat, Arc<Pipelines>>,
    atlas: GlyphAtlas,
    pub glyphs: GlyphCache,
    stats: FrameStats,
    legacy_display: bool,
    display_target: Option<Arc<GpuImage>>,
    display_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
}
impl PaintRenderer {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ui-paint-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ui-paint-pipeline-layout"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("ui-paint-shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("paint.wgsl").into()),
            });
        use wgpu::{
            AddressMode::{ClampToEdge as C, Repeat as R},
            FilterMode::{Linear as L, Nearest as N},
        };
        let samplers = [
            (Sampling::Nearest, N, C, C),
            (Sampling::Linear, L, C, C),
            (Sampling::RepeatNearest, N, R, R),
            (Sampling::RepeatLinear, L, R, R),
            (Sampling::RepeatNearestX, N, R, C),
            (Sampling::RepeatNearestY, N, C, R),
            (Sampling::RepeatLinearX, L, R, C),
            (Sampling::RepeatLinearY, L, C, R),
        ]
        .into_iter()
        .map(|(mode, filter, u, v)| {
            (
                mode,
                context.device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: u,
                    address_mode_v: v,
                    mag_filter: filter,
                    min_filter: filter,
                    ..Default::default()
                }),
            )
        })
        .collect();
        let white = GpuImage::new(&context, ImageDescriptor::rgba(1, 1))?;
        context.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &white.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            white.texture.size(),
        );
        let empty_field = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui-empty-distance-field"),
                contents: &[0; 80],
                usage: wgpu::BufferUsages::STORAGE,
            });
        Ok(Self {
            empty_field,
            samplers,
            context,
            layout,
            pipeline_layout,
            shader,
            white,
            pipelines: HashMap::new(),
            atlas: GlyphAtlas::default(),
            glyphs: GlyphCache::default(),
            stats: FrameStats::default(),
            legacy_display: false,
            display_target: None,
            display_pipelines: HashMap::new(),
        })
    }
    fn color(&self, color: Color) -> Color {
        if self.legacy_display {
            color
        } else {
            linear(color)
        }
    }
    pub fn frame_stats(&self) -> FrameStats {
        self.stats
    }

    fn glyph_draw(&self, batch: GlyphBatch, size: (u32, u32), level: u32) -> Draw {
        self.draw(
            self.geometry(&batch.vertices, &batch.indices),
            &Brush::Solid(batch.color),
            Some((&batch.image, Sampling::Linear)),
            size,
            level,
            0,
        )
    }

    fn pipelines(&mut self, format: wgpu::TextureFormat) -> Arc<Pipelines> {
        if let Some(p) = self.pipelines.get(&format) {
            return p.clone();
        }
        let samples = if self
            .context
            .adapter
            .get_texture_format_features(format)
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
        {
            4
        } else {
            1
        };
        let make = |phase: u8| {
            let stencil = wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Equal,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: match phase {
                    1 => wgpu::StencilOperation::IncrementClamp,
                    2 => wgpu::StencilOperation::DecrementClamp,
                    _ => wgpu::StencilOperation::Keep,
                },
            };
            self.context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
                label:Some("ui-paint-pipeline"),layout:Some(&self.pipeline_layout),
                vertex:wgpu::VertexState{module:&self.shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[Vertex::layout()]},
                fragment:Some(wgpu::FragmentState{module:&self.shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState{format,blend:Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),write_mask:if phase==0{wgpu::ColorWrites::ALL}else{wgpu::ColorWrites::empty()}})]}),
                primitive:wgpu::PrimitiveState::default(),depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth24PlusStencil8,depth_write_enabled:Some(false),depth_compare:Some(wgpu::CompareFunction::Always),stencil:wgpu::StencilState{front:stencil,back:stencil,read_mask:255,write_mask:if phase==0{0}else{255}},bias:Default::default()}),
                multisample:wgpu::MultisampleState{count:samples,..Default::default()},multiview_mask:None,cache:None,
            })
        };
        let p = Arc::new(Pipelines {
            draw: make(0),
            push: make(1),
            pop: make(2),
            samples,
        });
        self.pipelines.insert(format, p.clone());
        p
    }
    fn geometry(&self, vertices: &[Vertex], indices: &[u32]) -> Geometry {
        let vertices = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui-vertices"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let buffer = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Geometry {
            vertices,
            indices: buffer,
            count: indices.len() as u32,
            colored: false,
        }
    }
    fn draw(
        &self,
        geometry: Geometry,
        brush: &Brush,
        image: Option<(&GpuImage, Sampling)>,
        size: (u32, u32),
        stencil: u32,
        phase: u8,
    ) -> Draw {
        self.draw_with_effects(geometry, brush, image, size, stencil, phase, None, None)
    }
    fn draw_with_effects(
        &self,
        geometry: Geometry,
        brush: &Brush,
        image: Option<(&GpuImage, Sampling)>,
        size: (u32, u32),
        stencil: u32,
        phase: u8,
        field: Option<(&DistanceField, Point)>,
        effect: Option<ImageEffect>,
    ) -> Draw {
        let (mode, color0, color1, gradient) = match brush {
            Brush::Solid(c) => (0, self.color(*c), [0.; 4], [0.; 4]),
            Brush::LinearGradient {
                start,
                end,
                start_color,
                end_color,
            } => (
                1,
                self.color(*start_color),
                self.color(*end_color),
                [start[0], start[1], end[0], end[1]],
            ),
        };
        let source = image.map_or(0, |(image, sampling)| {
            let encoded = image.descriptor.color_space == ColorSpace::Srgb
                && !image.descriptor.format.is_srgb();
            let premultiplied = image.descriptor.alpha_mode == AlphaMode::Premultiplied;
            let linear = matches!(
                sampling,
                Sampling::Linear
                    | Sampling::RepeatLinear
                    | Sampling::RepeatLinearX
                    | Sampling::RepeatLinearY
            );
            // Raw GPU images must decode and premultiply individual texels
            // before interpolation. Already normalized images retain hardware
            // filtering, including the shared glyph/source-asset atlases.
            let manual = linear && (encoded || image.descriptor.alpha_mode == AlphaMode::Straight);
            let repeat_x = matches!(
                sampling,
                Sampling::RepeatLinear
                    | Sampling::RepeatNearest
                    | Sampling::RepeatLinearX
                    | Sampling::RepeatNearestX
            );
            let repeat_y = matches!(
                sampling,
                Sampling::RepeatLinear
                    | Sampling::RepeatNearest
                    | Sampling::RepeatLinearY
                    | Sampling::RepeatNearestY
            );
            u32::from(encoded)
                | (u32::from(premultiplied) << 1)
                | (u32::from(manual) << 2)
                | (u32::from(repeat_x) << 3)
                | (u32::from(repeat_y) << 4)
        });
        let filter = match effect {
            Some(ImageEffect::Filter(f)) => Some(f),
            _ => None,
        };
        let outline = match effect {
            Some(ImageEffect::Outline { uv, step, width }) => Some((uv, step, width)),
            _ => None,
        };
        let threshold = match effect {
            Some(ImageEffect::Threshold {
                dark,
                light,
                threshold,
            }) => Some((self.color(dark), self.color(light), threshold)),
            _ => None,
        };
        let internal_image = matches!(effect, Some(ImageEffect::Group | ImageEffect::Present(_)));
        let params = Params {
            viewport: [size.0 as f32, size.1 as f32],
            mode: (if let Some(ImageEffect::Present(srgb)) = effect {
                if srgb { 8 } else { 9 }
            } else if outline.is_some() {
                6
            } else if threshold.is_some() {
                7
            } else if field.is_some() {
                4
            } else if filter.is_some() {
                5
            } else if geometry.colored {
                3
            } else if image.is_some() {
                2
            } else {
                mode
            }) | if self.legacy_display { 256 } else { 0 },
            source: field.map_or(source | if internal_image { 32 } else { 0 }, |(f, _)| {
                f.primitives.len() as u32
            }),
            color0: threshold.map_or_else(
                || {
                    filter.map_or(color0, |f| {
                        [color0[0], color0[1], color0[2], color0[3] * f.alpha_scale]
                    })
                },
                |(dark, _, _)| dark,
            ),
            color1: threshold.map_or_else(
                || {
                    outline.map_or_else(
                        || {
                            filter.map_or(color1, |f| {
                                [f.scale[0], f.scale[1], f.scale[2], f.saturation]
                            })
                        },
                        |(uv, _, _)| [uv.x, uv.y, uv.x + uv.width, uv.y + uv.height],
                    )
                },
                |(_, light, _)| light,
            ),
            gradient: threshold.map_or_else(
                || {
                    outline.map_or_else(
                        || {
                            filter.map_or_else(
                                || field.map_or(gradient, |(_, p)| [p[0], p[1], 0., 0.]),
                                |f| [f.bias[0], f.bias[1], f.bias[2], f.minimum],
                            )
                        },
                        |(_, step, width)| [step[0], step[1], width, 0.],
                    )
                },
                |(_, _, threshold)| [54. / 256., 183. / 256., 19. / 256., threshold as f32],
            ),
        };
        let uniform = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui-paint-params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let texture = image.map_or(&*self.white, |(image, _)| image);
        let sampler = &self.samplers[&image.map(|(_, s)| s).unwrap_or(Sampling::Linear)];
        let field_data = field.map(|(f, _)| distance_data(f, self.legacy_display));
        let field_buffer = field_data.as_ref().filter(|v| !v.is_empty()).map(|data| {
            self.context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ui-distance-primitives"),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        });
        let bind = self
            .context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ui-paint-bind"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: field_buffer
                            .as_ref()
                            .unwrap_or(&self.empty_field)
                            .as_entire_binding(),
                    },
                ],
            });
        Draw {
            geometry,
            bind,
            stencil,
            phase,
        }
    }
    /// Execute paint in order. Explicit capture is available separately through renderkit.
    pub fn render(
        &mut self,
        list: &PaintList,
        images: &ImageRegistry,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        density: f32,
        clear: Color,
    ) -> Result<()> {
        self.stats = FrameStats::default();
        list.validate().map_err(|e| RenderError(e.into()))?;
        if size.0 == 0 || size.1 == 0 || !density.is_finite() || density <= 0. {
            return Err(RenderError("invalid paint target".into()));
        }
        if images.context().generation() != self.context.generation() {
            return Err(RenderError(
                "paint registry belongs to another device".into(),
            ));
        }
        self.legacy_display = matches!(
            list.ops.first(),
            Some(PaintOp::DisplayCompositing(
                DisplayCompositing::LegacyOpaque
            ))
        );
        if self.legacy_display {
            return self.render_display(list, images, target, format, size, density, clear);
        }
        self.render_ops(
            &list.ops,
            images,
            target,
            format,
            size,
            density,
            clear,
            Transform::default(),
        )
    }
    fn render_display(
        &mut self,
        list: &PaintList,
        images: &ImageRegistry,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        density: f32,
        clear: Color,
    ) -> Result<()> {
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm
                | wgpu::TextureFormat::Rgba8UnormSrgb
                | wgpu::TextureFormat::Bgra8Unorm
                | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(RenderError(
                "opaque display compatibility requires an 8-bit display target".into(),
            ));
        }
        if self
            .display_target
            .as_ref()
            .is_none_or(|image| (image.descriptor.width, image.descriptor.height) != size)
        {
            self.display_target = Some(GpuImage::new(
                &self.context,
                ImageDescriptor {
                    width: size.0,
                    height: size.1,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    color_space: ColorSpace::Linear,
                    alpha_mode: AlphaMode::Premultiplied,
                },
            )?);
        }
        let image = self.display_target.as_ref().unwrap().clone();
        self.render_ops(
            &list.ops,
            images,
            &image.view,
            image.descriptor.format,
            size,
            density,
            clear,
            Transform::default(),
        )?;
        if !self.display_pipelines.contains_key(&format) {
            let pipeline =
                self.context
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("opaque compatibility display"),
                        layout: Some(&self.pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &self.shader,
                            entry_point: Some("vs"),
                            compilation_options: Default::default(),
                            buffers: &[Vertex::layout()],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &self.shader,
                            entry_point: Some("fs"),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format,
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
            self.display_pipelines.insert(format, pipeline);
        }
        let (vertices, indices) = mesh::quad(
            Rect::new(0., 0., size.0 as f32, size.1 as f32),
            Rect::new(0., 0., 1., 1.),
            Transform::default(),
            1.,
        );
        let draw = self.draw_with_effects(
            self.geometry(&vertices, &indices),
            &Brush::Solid([1.; 4]),
            Some((&image, Sampling::Nearest)),
            size,
            0,
            0,
            None,
            Some(ImageEffect::Present(format.is_srgb())),
        );
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("opaque compatibility display"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("opaque compatibility display"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
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
            pass.set_pipeline(&self.display_pipelines[&format]);
            pass.set_bind_group(0, &draw.bind, &[]);
            pass.set_vertex_buffer(0, draw.geometry.vertices.slice(..));
            pass.set_index_buffer(draw.geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.geometry.count, 0, 0..1);
        }
        self.context.queue.submit([encoder.finish()]);
        self.stats.draw_calls += 1;
        Ok(())
    }
    fn render_ops(
        &mut self,
        ops: &[PaintOp],
        images: &ImageRegistry,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        density: f32,
        clear: Color,
        initial: Transform,
    ) -> Result<()> {
        let mut transforms = vec![initial];
        let mut clips: Vec<Geometry> = vec![];
        let mut draws = Vec::new();
        let mut i = 0;
        while i < ops.len() {
            let transform = *transforms.last().unwrap();
            let level = clips.len() as u32;
            match &ops[i] {
                PaintOp::DistanceField { rect, field } => {
                    if rect.width > 0. && rect.height > 0. {
                        let (v, ix) = mesh::quad(*rect, Rect::default(), transform, density);
                        draws.push(self.draw_with_effects(
                            self.geometry(&v, &ix),
                            &Brush::Solid(field.background),
                            None,
                            size,
                            level,
                            0,
                            Some((field, [rect.x, rect.y])),
                            None,
                        ));
                    }
                }
                PaintOp::ImageOutline {
                    image,
                    rect,
                    uv,
                    sampling,
                    width,
                    color,
                } => {
                    if *width > 0. && color[3] > 0. {
                        let image = images.get(*image).ok_or_else(|| {
                            RenderError("paint references a stale outline image".into())
                        })?;
                        let step = [uv.width / rect.width, uv.height / rect.height];
                        let expanded = Rect::new(
                            rect.x - width,
                            rect.y - width,
                            rect.width + width * 2.,
                            rect.height + width * 2.,
                        );
                        let expanded_uv = Rect::new(
                            uv.x - step[0] * width,
                            uv.y - step[1] * width,
                            uv.width + step[0] * width * 2.,
                            uv.height + step[1] * width * 2.,
                        );
                        let (v, ix) = mesh::quad(expanded, expanded_uv, transform, density);
                        draws.push(self.draw_with_effects(
                            self.geometry(&v, &ix),
                            &Brush::Solid(*color),
                            Some((&image, *sampling)),
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::Outline {
                                uv: *uv,
                                step,
                                width: *width,
                            }),
                        ));
                    }
                }
                PaintOp::ThresholdImage {
                    image,
                    rect,
                    uv,
                    sampling,
                    threshold,
                    dark,
                    light,
                } => {
                    let image = images.get(*image).ok_or_else(|| {
                        RenderError("paint references a stale image handle".into())
                    })?;
                    if rect.width > 0. && rect.height > 0. {
                        let (vertices, indices) = mesh::quad(*rect, *uv, transform, density);
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid([1.; 4]),
                            Some((&image, *sampling)),
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::Threshold {
                                dark: *dark,
                                light: *light,
                                threshold: *threshold,
                            }),
                        ));
                    }
                }
                PaintOp::FilteredImage {
                    image,
                    rect,
                    uv,
                    sampling,
                    tint,
                    filter,
                } => {
                    let image = images.get(*image).ok_or_else(|| {
                        RenderError("paint references a stale image handle".into())
                    })?;
                    if rect.width > 0. && rect.height > 0. {
                        let (vertices, indices) = mesh::quad(*rect, *uv, transform, density);
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid(*tint),
                            Some((&image, *sampling)),
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::Filter(*filter)),
                        ));
                    }
                }
                PaintOp::ImageMesh {
                    vertices,
                    indices,
                    image,
                    sampling,
                    tint,
                } => {
                    if !indices.is_empty() {
                        let image = images.get(*image).ok_or_else(|| {
                            RenderError("paint references a stale image handle".into())
                        })?;
                        let vertices = mesh::textured(vertices, transform, density);
                        draws.push(self.draw(
                            self.geometry(&vertices, indices),
                            &Brush::Solid(*tint),
                            Some((&image, *sampling)),
                            size,
                            level,
                            0,
                        ));
                    }
                }
                PaintOp::Mesh { vertices, indices } => {
                    if !indices.is_empty() {
                        let vertices = mesh::colored(vertices, transform, density);
                        let mut geometry = self.geometry(&vertices, indices);
                        geometry.colored = true;
                        draws.push(self.draw(
                            geometry,
                            &Brush::Solid([1.; 4]),
                            None,
                            size,
                            level,
                            0,
                        ));
                    }
                }
                PaintOp::Rect {
                    rect,
                    radius,
                    brush,
                } => {
                    if rect.width > 0. && rect.height > 0. {
                        let (v, ix) = mesh::tessellate(
                            &mesh::rect_path(*rect, *radius),
                            FillRule::NonZero,
                            None,
                            transform,
                            density,
                        )?;
                        draws.push(self.draw(self.geometry(&v, &ix), brush, None, size, level, 0));
                    }
                }
                PaintOp::Receipt(_) | PaintOp::DisplayCompositing(_) => {}
                PaintOp::Fill { path, rule, brush } => {
                    let (v, ix) = mesh::tessellate(path, *rule, None, transform, density)?;
                    draws.push(self.draw(self.geometry(&v, &ix), brush, None, size, level, 0));
                }
                PaintOp::Stroke { path, stroke } => {
                    let (v, ix) = mesh::tessellate(
                        path,
                        FillRule::NonZero,
                        Some(stroke),
                        transform,
                        density,
                    )?;
                    draws.push(self.draw(
                        self.geometry(&v, &ix),
                        &stroke.brush,
                        None,
                        size,
                        level,
                        0,
                    ));
                }
                PaintOp::Image {
                    image,
                    rect,
                    uv,
                    sampling,
                    tint,
                } => {
                    let image = images.get(*image).ok_or_else(|| {
                        RenderError("paint references a stale image handle".into())
                    })?;
                    let (v, ix) = mesh::quad(*rect, *uv, transform, density);
                    draws.push(self.draw(
                        self.geometry(&v, &ix),
                        &Brush::Solid(*tint),
                        Some((&image, *sampling)),
                        size,
                        level,
                        0,
                    ));
                }
                PaintOp::GlyphRun { run, origin, color } => {
                    let scale = density
                        * transform.0[0]
                            .hypot(transform.0[1])
                            .max(transform.0[2].hypot(transform.0[3]));
                    if scale > 0. {
                        let mut batch: Option<GlyphBatch> = None;
                        for glyph in run.glyphs.iter() {
                            let local =
                                [origin[0] + glyph.position[0], origin[1] + glyph.position[1]];
                            let physical = transform.point(local).map(|x| x * density);
                            if let Some((key, bitmap)) =
                                self.glyphs.glyph(run, glyph.id, scale, physical)
                            {
                                if bitmap.width == 0 || bitmap.height == 0 {
                                    continue;
                                }
                                let entry = if let Some(e) = self.atlas.get(&key) {
                                    e
                                } else {
                                    self.atlas.insert(&self.context, key, bitmap)?
                                };
                                let snapped =
                                    [physical[0].floor() / density, physical[1].floor() / density];
                                let local = transform
                                    .inverse()
                                    .ok_or_else(|| RenderError("singular glyph transform".into()))?
                                    .point(snapped);
                                let rect = Rect::new(
                                    local[0] + entry.bitmap.left as f32 / scale,
                                    local[1] - entry.bitmap.top as f32 / scale,
                                    entry.bitmap.width as f32 / scale,
                                    entry.bitmap.height as f32 / scale,
                                );
                                // Bitmap dimensions are physical pixels; divide by the complete
                                // raster scale before the logical transform is applied again.
                                let tint = if entry.bitmap.color {
                                    [1., 1., 1., color[3]]
                                } else {
                                    *color
                                };
                                if batch.as_ref().is_some_and(|b| {
                                    !Arc::ptr_eq(&b.image, &entry.image) || b.color != tint
                                }) {
                                    draws.push(self.glyph_draw(batch.take().unwrap(), size, level));
                                }
                                let batch = batch.get_or_insert_with(|| GlyphBatch {
                                    vertices: Vec::new(),
                                    indices: Vec::new(),
                                    image: entry.image.clone(),
                                    color: tint,
                                });
                                mesh::append_quad(
                                    &mut batch.vertices,
                                    &mut batch.indices,
                                    rect,
                                    entry.uv,
                                    transform,
                                    density,
                                );
                                self.stats.glyph_quads += 1;
                            }
                        }
                        if let Some(batch) = batch {
                            draws.push(self.glyph_draw(batch, size, level));
                        }
                    }
                }
                PaintOp::PushTransform(t) => transforms.push(t.then(transform)),
                PaintOp::PopTransform => {
                    if transforms.len() <= 1 {
                        return Err(RenderError("paint transform underflow".into()));
                    }
                    transforms.pop();
                }
                PaintOp::PushClip(rect) => {
                    let (v, ix) = mesh::tessellate(
                        &mesh::rect_path(*rect, 0.),
                        FillRule::NonZero,
                        None,
                        transform,
                        density,
                    )?;
                    let geometry = self.geometry(&v, &ix);
                    draws.push(self.draw(
                        geometry.clone(),
                        &Brush::Solid([1.; 4]),
                        None,
                        size,
                        level,
                        1,
                    ));
                    clips.push(geometry);
                }
                PaintOp::PushPathClip(path, rule) => {
                    let (v, ix) = mesh::tessellate(path, *rule, None, transform, density)?;
                    let geometry = self.geometry(&v, &ix);
                    draws.push(self.draw(
                        geometry.clone(),
                        &Brush::Solid([1.; 4]),
                        None,
                        size,
                        level,
                        1,
                    ));
                    clips.push(geometry);
                }
                PaintOp::PopClip => {
                    let geometry = clips
                        .pop()
                        .ok_or_else(|| RenderError("paint clip underflow".into()))?;
                    draws.push(self.draw(geometry, &Brush::Solid([1.; 4]), None, size, level, 2));
                }
                PaintOp::PushOpacity(opacity) => {
                    let mut depth = 1;
                    let start = i + 1;
                    let mut end = start;
                    while end < ops.len() {
                        match ops[end] {
                            PaintOp::PushOpacity(_) => depth += 1,
                            PaintOp::PopOpacity => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        end += 1;
                    }
                    if depth != 0 {
                        return Err(RenderError("unclosed opacity group".into()));
                    }
                    let group = GpuImage::new(
                        &self.context,
                        ImageDescriptor {
                            width: size.0,
                            height: size.1,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            color_space: ColorSpace::Linear,
                            alpha_mode: AlphaMode::Premultiplied,
                        },
                    )?;
                    self.render_ops(
                        &ops[start..end],
                        images,
                        &group.view,
                        group.descriptor.format,
                        size,
                        density,
                        [0.; 4],
                        transform,
                    )?;
                    let (v, ix) = mesh::quad(
                        Rect::new(0., 0., size.0 as f32, size.1 as f32),
                        Rect::new(0., 0., 1., 1.),
                        Transform::default(),
                        1.,
                    );
                    draws.push(self.draw_with_effects(
                        self.geometry(&v, &ix),
                        &Brush::Solid([1., 1., 1., opacity.clamp(0., 1.)]),
                        Some((&group, Sampling::Nearest)),
                        size,
                        level,
                        0,
                        None,
                        Some(ImageEffect::Group),
                    ));
                    i = end;
                }
                PaintOp::PopOpacity => {
                    return Err(RenderError("unexpected opacity group end".into()));
                }
            }
            if clips.len() > 255 {
                return Err(RenderError("stencil clip nesting exceeds 255".into()));
            }
            i += 1;
        }
        if !clips.is_empty() || transforms.len() != 1 {
            return Err(RenderError("unclosed paint scope".into()));
        }
        let pipelines = self.pipelines(format);
        let scratch = |format, samples| {
            self.context
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("ui-frame-attachment"),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
        };
        let stencil = scratch(wgpu::TextureFormat::Depth24PlusStencil8, pipelines.samples)
            .create_view(&Default::default());
        let msaa = (pipelines.samples > 1)
            .then(|| scratch(format, pipelines.samples).create_view(&Default::default()));
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ui-paint-frame"),
                });
        let c = self.color(clear);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-ordered-paint"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa.as_ref().unwrap_or(target),
                    resolve_target: msaa.as_ref().map(|_| target),
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: (c[0] * c[3]) as f64,
                            g: (c[1] * c[3]) as f64,
                            b: (c[2] * c[3]) as f64,
                            a: c[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &stencil,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for draw in &draws {
                if draw.geometry.count == 0 {
                    continue;
                }
                pass.set_pipeline(match draw.phase {
                    1 => &pipelines.push,
                    2 => &pipelines.pop,
                    _ => &pipelines.draw,
                });
                pass.set_stencil_reference(draw.stencil);
                pass.set_bind_group(0, &draw.bind, &[]);
                pass.set_vertex_buffer(0, draw.geometry.vertices.slice(..));
                pass.set_index_buffer(draw.geometry.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.geometry.count, 0, 0..1);
                self.stats.draw_calls += 1;
            }
        }
        self.context.queue.submit([encoder.finish()]);
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DistanceData {
    bounds: [f32; 4],
    radii: [f32; 4],
    fill: Color,
    border: Color,
    meta: [f32; 4],
}
fn distance_data(field: &DistanceField, encoded: bool) -> Vec<DistanceData> {
    field
        .primitives
        .iter()
        .map(|p| {
            let (kind, bounds, radii) = match p.geometry {
                DistanceGeometry::Circle { center, radius } => {
                    (0., [center[0], center[1], radius, radius], [0.; 4])
                }
                DistanceGeometry::Hexagon { center, radius } => {
                    (1., [center[0], center[1], radius, radius], [0.; 4])
                }
                DistanceGeometry::Rhombus { center, half_size } => (
                    2.,
                    [center[0], center[1], half_size[0], half_size[1]],
                    [0.; 4],
                ),
                DistanceGeometry::RoundedRect { rect, radii } => (
                    3.,
                    [
                        rect.x + rect.width * 0.5,
                        rect.y + rect.height * 0.5,
                        rect.width * 0.5,
                        rect.height * 0.5,
                    ],
                    radii,
                ),
            };
            DistanceData {
                bounds,
                radii,
                fill: if encoded { p.fill } else { linear(p.fill) },
                border: if encoded { p.border } else { linear(p.border) },
                meta: [kind, p.border_width.max(0.), 0., 0.],
            }
        })
        .collect()
}
