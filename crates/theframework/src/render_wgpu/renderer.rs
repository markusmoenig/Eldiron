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
    inverse: [f32; 4],
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
    PixelTriangle([Point; 3]),
    PixelLine([i32; 4]),
    LogicalPixelLine {
        endpoints: [i32; 4],
        basis: [f32; 4],
    },
    SampledLine {
        endpoints: [f32; 4],
        basis: [f32; 4],
        samples: u32,
    },
    Sprite {
        scale: Point,
        basis: [f32; 4],
    },
    GlyphBackdrop(Color),
    PixelNearest(Point),
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
    // Preserve target leases while a parent pass still holds unsubmitted draws.
    _image: Arc<GpuImage>,
    stencil: u32,
    phase: u8,
    instances: u32,
}
#[derive(Clone, Copy)]
enum Field<'a> {
    HueLightness(Rect, [f32; 4]),
    Distance(&'a DistanceField, Point),
    Radial(&'a PixelRadialLayer, Rect, [f32; 4]),
}
impl Field<'_> {
    fn count(self) -> u32 {
        match self {
            Self::HueLightness(..) => 0,
            Self::Distance(field, _) => field.primitives.len() as u32,
            Self::Radial(layer, ..) => {
                layer.radials.len() as u32 + 1 + (layer.size[1] as u32).div_ceil(20)
            }
        }
    }
    fn origin(self) -> [f32; 4] {
        match self {
            Self::HueLightness(rect, _) => [rect.x, rect.y, 0., 0.],
            Self::Distance(_, p) => [p[0], p[1], 0., 0.],
            Self::Radial(_, _, basis) => basis,
        }
    }
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
    background: Option<Color>,
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
        self.draw_with_effects(
            self.geometry(&batch.vertices, &batch.indices),
            &Brush::Solid(batch.color),
            Some((&batch.image, Sampling::Linear)),
            size,
            level,
            0,
            None,
            batch.background.map(ImageEffect::GlyphBackdrop),
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
            self.context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("ui-paint-pipeline"),
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
                            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                            write_mask: if phase == 0 {
                                wgpu::ColorWrites::ALL
                            } else {
                                wgpu::ColorWrites::empty()
                            },
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24PlusStencil8,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: wgpu::StencilState {
                            front: stencil,
                            back: stencil,
                            read_mask: 255,
                            write_mask: if phase == 0 { 0 } else { 255 },
                        },
                        bias: Default::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..Default::default()
                    },
                    multiview_mask: None,
                    cache: None,
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
        image: Option<(&Arc<GpuImage>, Sampling)>,
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
        image: Option<(&Arc<GpuImage>, Sampling)>,
        size: (u32, u32),
        stencil: u32,
        phase: u8,
        field: Option<Field<'_>>,
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
                | (u32::from(image.descriptor.alpha_mode == AlphaMode::Opaque) << 6)
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
        let pixel_size = match effect {
            Some(ImageEffect::PixelNearest(size)) => Some(size),
            _ => None,
        };
        let glyph_background = match effect {
            Some(ImageEffect::GlyphBackdrop(color)) => Some(color),
            _ => None,
        };
        let internal_image = matches!(effect, Some(ImageEffect::Group | ImageEffect::Present(_)));
        let mut params = Params {
            viewport: [size.0 as f32, size.1 as f32],
            mode: (if let Some(ImageEffect::Present(srgb)) = effect {
                if srgb { 8 } else { 9 }
            } else if matches!(effect, Some(ImageEffect::LogicalPixelLine { .. })) {
                17
            } else if matches!(effect, Some(ImageEffect::Sprite { .. })) {
                15
            } else if matches!(effect, Some(ImageEffect::SampledLine { .. })) {
                14
            } else if matches!(effect, Some(ImageEffect::PixelTriangle(_))) {
                13
            } else if matches!(effect, Some(ImageEffect::PixelLine(_))) {
                12
            } else if pixel_size.is_some() {
                11
            } else if glyph_background.is_some() {
                10
            } else if outline.is_some() {
                6
            } else if threshold.is_some() {
                7
            } else if matches!(field, Some(Field::HueLightness(..))) {
                18
            } else if matches!(field, Some(Field::Radial(..))) {
                16
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
            source: field.map_or(source | if internal_image { 32 } else { 0 }, Field::count),
            color0: threshold.map_or_else(
                || {
                    filter.map_or(color0, |f| {
                        [color0[0], color0[1], color0[2], color0[3] * f.alpha_scale]
                    })
                },
                |(dark, _, _)| dark,
            ),
            color1: glyph_background.map_or_else(
                || {
                    threshold.map_or_else(
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
                    )
                },
                |color| self.color(color),
            ),
            gradient: pixel_size.map_or_else(
                || {
                    threshold.map_or_else(
                        || {
                            outline.map_or_else(
                                || {
                                    filter.map_or_else(
                                        || field.map_or(gradient, Field::origin),
                                        |f| [f.bias[0], f.bias[1], f.bias[2], f.minimum],
                                    )
                                },
                                |(_, step, width)| [step[0], step[1], width, 0.],
                            )
                        },
                        |(_, _, threshold)| [54. / 256., 183. / 256., 19. / 256., threshold as f32],
                    )
                },
                |size| [size[0], size[1], 0., 0.],
            ),
            inverse: [0.; 4],
        };
        if let Some(Field::HueLightness(rect, basis)) = field {
            params.color1 = [rect.x, rect.y, rect.width, rect.height];
            params.gradient = basis;
        }
        if let Some(Field::Radial(layer, rect, _)) = field {
            // The radial shader explicitly quantizes/composites encoded bytes,
            // then converts its final opaque color at the declared boundary.
            params.color0 = layer.top.map(|v| (v * 255.).round());
            params.color1 = layer.bottom.map(|v| (v * 255.).round());
            params.color0[3] = rect.width / layer.size[0];
            params.color1[3] = rect.height / layer.size[1];
            if layer.opaque {
                params.mode |= 512;
            }
        }
        if let Some(ImageEffect::PixelTriangle(points)) = effect {
            params.color1 = [points[0][0], points[0][1], points[1][0], points[1][1]];
            params.gradient = [points[2][0], points[2][1], 0., 0.];
        }
        if let Some(ImageEffect::PixelLine(points)) = effect {
            // Numeric 16-bit halves are exact f32 values, including negative
            // coordinates. Preserve exact endpoint halves without NaN bit packing.
            params.color1 = points.map(|p| ((p as u32) >> 16) as f32);
            params.gradient = points.map(|p| ((p as u32) & 65535) as f32);
        }
        if let Some(ImageEffect::SampledLine {
            endpoints, basis, ..
        }) = effect
        {
            params.color1 = endpoints;
            params.gradient = basis;
        }
        if let Some(ImageEffect::Sprite { scale, basis }) = effect {
            params.color1 = [scale[0], scale[1], 0., 0.];
            params.gradient = basis;
        }
        if let Some(ImageEffect::LogicalPixelLine { basis, .. }) = effect {
            params.gradient = basis;
        }
        let pixel_basis = match effect {
            Some(
                ImageEffect::Sprite { basis, .. }
                | ImageEffect::SampledLine { basis, .. }
                | ImageEffect::LogicalPixelLine { basis, .. },
            ) => Some(basis),
            _ => match field {
                Some(Field::Radial(_, _, basis) | Field::HueLightness(_, basis)) => Some(basis),
                _ => None,
            },
        };
        if let Some([a, b, c, d]) = pixel_basis {
            // Avoid per-fragment approximate reciprocal division at exact
            // logical pixel boundaries, especially 125/150/175% DPI.
            let (a, b, c, d) = (a as f64, b as f64, c as f64, d as f64);
            let determinant = a * d - b * c;
            params.inverse = [
                (d / determinant) as f32,
                (-c / determinant) as f32,
                (-b / determinant) as f32,
                (a / determinant) as f32,
            ];
        }
        let uniform = self
            .context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ui-paint-params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let texture = image.map_or(&self.white, |(image, _)| image);
        let sampler = &self.samplers[&image.map(|(_, s)| s).unwrap_or(Sampling::Linear)];
        let mut field_data = field.map(|field| match field {
            Field::HueLightness(..) => Vec::new(),
            Field::Distance(field, _) => distance_data(field, self.legacy_display),
            Field::Radial(layer, ..) => radial_data(layer),
        });
        if let Some(ImageEffect::LogicalPixelLine { endpoints, .. }) = effect {
            field_data = Some(vec![DistanceData {
                bounds: endpoints.map(|p| ((p as u32) >> 16) as f32),
                radii: endpoints.map(|p| ((p as u32) & 65535) as f32),
                fill: [0.; 4],
                border: [0.; 4],
                meta: [0.; 4],
            }]);
        }
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
            _image: texture.clone(),
            stencil,
            phase,
            instances: match effect {
                Some(ImageEffect::SampledLine { samples, .. }) => samples,
                _ => 1,
            },
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
            self.display_target = Some(GpuImage::render_target(
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
            pass.draw_indexed(0..draw.geometry.count, 0, 0..draw.instances);
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
                PaintOp::HueLightness { rect } => {
                    let [a, b, c, d, _, _] = transform.0;
                    let basis = [a * density, b * density, c * density, d * density];
                    if transform.inverse().is_some() {
                        let (mut v, ix) = mesh::quad(*rect, Rect::default(), transform, density);
                        for vertex in &mut v {
                            vertex.color = [rect.x, rect.y, rect.width, rect.height];
                        }
                        let mut geometry = self.geometry(&v, &ix);
                        geometry.colored = false;
                        draws.push(self.draw_with_effects(
                            geometry,
                            &Brush::Solid([1.; 4]),
                            None,
                            size,
                            level,
                            0,
                            Some(Field::HueLightness(*rect, basis)),
                            None,
                        ));
                    }
                }
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
                            Some(Field::Distance(field, [rect.x, rect.y])),
                            None,
                        ));
                    }
                }
                PaintOp::PixelRadialLayer { rect, layer } => {
                    let [a, b, c, d, _, _] = transform.0;
                    let basis = [a * density, b * density, c * density, d * density];
                    if rect.width > 0. && rect.height > 0. && transform.inverse().is_some() {
                        if basis.iter().any(|v| !v.is_finite()) {
                            return Err(RenderError("nonfinite radial transform".into()));
                        }
                        let (mut vertices, indices) =
                            mesh::quad(*rect, Rect::default(), transform, density);
                        for vertex in &mut vertices {
                            vertex.color = [rect.x, rect.y, rect.width, rect.height];
                        }
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid([1.; 4]),
                            None,
                            size,
                            level,
                            0,
                            Some(Field::Radial(layer, *rect, basis)),
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
                PaintOp::PixelTriangle { points, color } => {
                    let physical = points.map(|p| transform.point(p).map(|v| v * density));
                    if physical.iter().flatten().any(|v| !v.is_finite()) {
                        return Err(RenderError("nonfinite transformed pixel triangle".into()));
                    }
                    if let Some((vertices, indices)) = mesh::pixel_triangle(physical, size) {
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid(*color),
                            None,
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::PixelTriangle(physical)),
                        ));
                    }
                }
                PaintOp::PixelLine { from, to, color } => {
                    let physical =
                        |point: Point| transform.point(point).map(|v| (v * density).round() as i32);
                    let a = physical(*from);
                    let b = physical(*to);
                    if let Some((vertices, indices)) = mesh::pixel_line(a, b, size) {
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid(*color),
                            None,
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::PixelLine([a[0], a[1], b[0], b[1]])),
                        ));
                    }
                }
                PaintOp::LogicalPixelLine { from, to, color } => {
                    let a = from.map(|n| n.round() as i32);
                    let b = to.map(|n| n.round() as i32);
                    let [ma, mb, mc, md, _, _] = transform.0;
                    let basis = [ma * density, mb * density, mc * density, md * density];
                    if basis.iter().any(|n| !n.is_finite()) {
                        return Err(RenderError("nonfinite logical line transform".into()));
                    }
                    if transform.inverse().is_some() {
                        let rect = Rect::new(
                            a[0].min(b[0]) as f32,
                            a[1].min(b[1]) as f32,
                            (a[0].abs_diff(b[0]) as f64 + 1.) as f32,
                            (a[1].abs_diff(b[1]) as f64 + 1.) as f32,
                        );
                        let (mut vertices, indices) =
                            mesh::quad(rect, Rect::default(), transform, density);
                        for v in &mut vertices {
                            v.color = [rect.x, rect.y, rect.width, rect.height];
                        }
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid(*color),
                            None,
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::LogicalPixelLine {
                                endpoints: [a[0], a[1], b[0], b[1]],
                                basis,
                            }),
                        ));
                    }
                }
                PaintOp::SampledLine { from, to, color } => {
                    let delta = [to[0] - from[0], to[1] - from[1]];
                    let transformed = [transform.point(*from), transform.point(*to)];
                    if delta
                        .iter()
                        .chain(transformed.iter().flatten())
                        .any(|v| !v.is_finite() || !(v * density).is_finite())
                    {
                        return Err(RenderError("nonfinite sampled line transform".into()));
                    }
                    let steps = delta[0].abs().max(delta[1].abs()).ceil().min(8192.) as u32;
                    if steps > 0 && transform.inverse().is_some() {
                        let (vertices, indices) = mesh::quad(
                            Rect::new(0., 0., 1., 1.),
                            Rect::default(),
                            transform,
                            density,
                        );
                        let [a, b, c, d, _, _] = transform.0;
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid(*color),
                            None,
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::SampledLine {
                                endpoints: [from[0], from[1], to[0], to[1]],
                                basis: [a * density, b * density, c * density, d * density],
                                samples: steps + 1,
                            }),
                        ));
                    }
                }
                PaintOp::PixelRects { rects, color } => {
                    if transform.inverse().is_some() {
                        let mut vertices = Vec::new();
                        let mut indices = Vec::new();
                        for rect in rects.iter() {
                            let rect = Rect::new(
                                rect.x.round(),
                                rect.y.round(),
                                rect.width.round(),
                                rect.height.round(),
                            );
                            if rect.width <= 0. || rect.height <= 0. {
                                continue;
                            }
                            let (mut next, next_indices) =
                                mesh::quad(rect, Rect::default(), transform, density);
                            if next
                                .iter()
                                .any(|v| v.position.iter().any(|v| !v.is_finite()))
                            {
                                return Err(RenderError(
                                    "nonfinite pixel rectangle transform".into(),
                                ));
                            }
                            for v in &mut next {
                                v.color = [rect.x, rect.y, rect.width, rect.height];
                            }
                            let offset = u32::try_from(vertices.len()).map_err(|_| {
                                RenderError("pixel rectangle batch is too large".into())
                            })?;
                            indices.extend(next_indices.into_iter().map(|i| i + offset));
                            vertices.extend(next);
                        }
                        if !indices.is_empty() {
                            let mut geometry = self.geometry(&vertices, &indices);
                            geometry.colored = false;
                            let [a, b, c, d, _, _] = transform.0;
                            draws.push(self.draw_with_effects(
                                geometry,
                                &Brush::Solid(*color),
                                None,
                                size,
                                level,
                                0,
                                None,
                                Some(ImageEffect::Sprite {
                                    scale: [1., 1.],
                                    basis: [a * density, b * density, c * density, d * density],
                                }),
                            ));
                        }
                    }
                }
                PaintOp::SpriteImage {
                    image,
                    origin,
                    scale,
                    tint,
                } => {
                    let image = images.get(*image).ok_or_else(|| {
                        RenderError("paint references a stale sprite image handle".into())
                    })?;
                    let origin = origin.map(f32::round);
                    let extent = [
                        (image.descriptor.width as f32 * scale[0]).round(),
                        (image.descriptor.height as f32 * scale[1]).round(),
                    ];
                    let [a, b, c, d, _, _] = transform.0;
                    let basis = [a * density, b * density, c * density, d * density];
                    if origin
                        .iter()
                        .chain(extent.iter())
                        .chain(basis.iter())
                        .any(|v| !v.is_finite())
                    {
                        return Err(RenderError("nonfinite sprite transform".into()));
                    }
                    if extent[0] > 0. && extent[1] > 0. && transform.inverse().is_some() {
                        let rect = Rect::new(origin[0], origin[1], extent[0], extent[1]);
                        let (mut vertices, indices) =
                            mesh::quad(rect, Rect::default(), transform, density);
                        for vertex in &mut vertices {
                            vertex.color = [origin[0], origin[1], extent[0], extent[1]];
                        }
                        let mut geometry = self.geometry(&vertices, &indices);
                        geometry.colored = false;
                        draws.push(self.draw_with_effects(
                            geometry,
                            &Brush::Solid(*tint),
                            Some((&image, Sampling::Nearest)),
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::Sprite {
                                scale: *scale,
                                basis,
                            }),
                        ));
                    }
                }
                PaintOp::PixelImage { image, rect, tint } => {
                    let image = images.get(*image).ok_or_else(|| {
                        RenderError("paint references a stale image handle".into())
                    })?;
                    if rect.width > 0. && rect.height > 0. {
                        let size_physical = [
                            rect.width * transform.0[0].hypot(transform.0[1]) * density,
                            rect.height * transform.0[2].hypot(transform.0[3]) * density,
                        ];
                        let (vertices, indices) =
                            mesh::quad(*rect, Rect::new(0., 0., 1., 1.), transform, density);
                        draws.push(self.draw_with_effects(
                            self.geometry(&vertices, &indices),
                            &Brush::Solid(*tint),
                            Some((&image, Sampling::Nearest)),
                            size,
                            level,
                            0,
                            None,
                            Some(ImageEffect::PixelNearest(size_physical)),
                        ));
                    }
                }
                PaintOp::GlyphRun { run, origin, color }
                | PaintOp::OpaqueGlyphRun {
                    run, origin, color, ..
                } => {
                    let background = match &ops[i] {
                        PaintOp::OpaqueGlyphRun { background, .. } => Some(*background),
                        _ => None,
                    };
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
                                    background,
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
                    let group = GpuImage::render_target(
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
        let stencil = self.context.acquire_target(renderkit::TargetDescriptor::attachment(
            size.0, size.1, wgpu::TextureFormat::Depth24PlusStencil8, pipelines.samples,
        ))?;
        let msaa = if pipelines.samples > 1 {
            Some(self.context.acquire_target(renderkit::TargetDescriptor::attachment(
                size.0, size.1, format, pipelines.samples,
            ))?)
        } else { None };
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
                    view: msaa.as_ref().map_or(target, |texture| texture.view()),
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
                    view: stencil.view(),
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
                pass.draw_indexed(0..draw.geometry.count, 0, 0..draw.instances);
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
fn radial_data(layer: &PixelRadialLayer) -> Vec<DistanceData> {
    let bytes = |color: Color| color.map(|v| (v * 255.).round());
    // Compact one-dimensional byte-color metadata preserves the CPU gradient's
    // truncation without Metal's reciprocal/FMA changing an exact row boundary.
    // Twenty packed RGB rows occupy one storage entry; no viewport pixels exist.
    let height = layer.size[1] as usize;
    let top = bytes(layer.top);
    let bottom = bytes(layer.bottom);
    let rows = height.div_ceil(20);
    let mut data = vec![DistanceData {
        bounds: [layer.size[0], layer.size[1], rows as f32, 0.],
        radii: [0.; 4],
        fill: [0.; 4],
        border: [0.; 4],
        meta: [0.; 4],
    }];
    for block in 0..rows {
        let mut words = [0.; 20];
        for (index, word) in words.iter_mut().enumerate() {
            let row = block * 20 + index;
            if row >= height {
                break;
            }
            let t = row as f32 / height as f32;
            let color = [0, 1, 2].map(|k| (top[k] * (1. - t) + bottom[k] * t) as u8);
            *word = (color[0] as u32 | ((color[1] as u32) << 8) | ((color[2] as u32) << 16)) as f32;
        }
        data.push(DistanceData {
            bounds: words[0..4].try_into().unwrap(),
            radii: words[4..8].try_into().unwrap(),
            fill: words[8..12].try_into().unwrap(),
            border: words[12..16].try_into().unwrap(),
            meta: words[16..20].try_into().unwrap(),
        });
    }
    data.extend(layer.radials.iter().map(|radial| match radial {
        PixelRadial::SoftCircle {
            center,
            radius,
            color,
        } => DistanceData {
            bounds: [center[0], center[1], *radius, 0.],
            radii: [0.; 4],
            fill: bytes(*color),
            border: [0.; 4],
            meta: [0.; 4],
        },
        PixelRadial::Disc {
            rect,
            color,
            border_width,
            border,
        } => {
            let half_border = border_width * 0.5;
            DistanceData {
                bounds: [rect.x, rect.y, rect.width, rect.height],
                radii: [
                    (rect.x + rect.width * 0.5 - half_border).round(),
                    (rect.y + rect.height * 0.5 - half_border).round(),
                    rect.width * 0.5 - half_border,
                    0.,
                ],
                fill: bytes(*color),
                border: bytes(*border),
                meta: [1., *border_width, 0., 0.],
            }
        }
    }));
    data
}
