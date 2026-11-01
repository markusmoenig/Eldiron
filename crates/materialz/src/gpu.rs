//! Single-material WGPU runtime. Hosts own submission, polling, and presentation.
use crate::{
    bsdf::Channel,
    integration::{
        Diagnostic, EditableMaterialAsset, InstanceRecord, Location, ParameterKey, Result,
    },
    wgsl,
};
use std::{
    borrow::Cow,
    sync::atomic::{AtomicU64, Ordering},
};
use wgpu::util::DeviceExt;

const CHANNELS: [Channel; 6] = [
    Channel::BaseColor,
    Channel::Opacity,
    Channel::Roughness,
    Channel::Metallic,
    Channel::Emission,
    Channel::EmissionStrength,
];
const PREVIEW_CHANNELS: [Channel; 8] = [
    Channel::BaseColor,
    Channel::Opacity,
    Channel::Roughness,
    Channel::Metallic,
    Channel::Emission,
    Channel::EmissionStrength,
    Channel::Height,
    Channel::Normal,
];
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy)]
pub struct SurfaceDomain {
    pub origin: [f32; 2],
    pub axis_u: [f32; 2],
    pub axis_v: [f32; 2],
}
impl SurfaceDomain {
    pub fn rect(origin: [f32; 2], span: [f32; 2]) -> Self {
        Self {
            origin,
            axis_u: [span[0], 0.],
            axis_v: [0., span[1]],
        }
    }
    fn mapped(self, m: [f32; 6]) -> Self {
        let v = |p: [f32; 2]| [m[0] * p[0] + m[1] * p[1], m[2] * p[0] + m[3] * p[1]];
        let mut origin = v(self.origin);
        origin[0] += m[4];
        origin[1] += m[5];
        Self {
            origin,
            axis_u: v(self.axis_u),
            axis_v: v(self.axis_v),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct RenderRequest {
    /// Normalized animation time: 0 and 1 are the same point of a Loop Flow cycle.
    pub time: f32,
    pub resolution: [u32; 2],
    pub domain: SurfaceDomain,
}

#[derive(Debug, Clone)]
pub struct OutputLayout {
    pub channel: Channel,
    pub format: wgpu::TextureFormat,
    pub components: &'static str,
    pub default: &'static [f32],
    pub range: [f32; 2],
    pub color_space: &'static str,
    pub alpha: &'static str,
}
pub fn output_layout() -> [OutputLayout; 5] {
    std::array::from_fn(|i| channel_layout(CHANNELS[i]))
}
/// Additional authoring maps do not change the five-map runtime/bake profile.
pub fn preview_output_layout() -> [OutputLayout; 7] {
    [
        Channel::BaseColor,
        Channel::Opacity,
        Channel::Roughness,
        Channel::Metallic,
        Channel::Emission,
        Channel::Height,
        Channel::Normal,
    ]
    .map(channel_layout)
}
fn channel_layout(channel: Channel) -> OutputLayout {
    let rgb = matches!(
        channel,
        Channel::BaseColor | Channel::Emission | Channel::Normal
    );
    OutputLayout {
        channel,
        format: if rgb {
            wgpu::TextureFormat::Rgba16Float
        } else {
            wgpu::TextureFormat::R32Float
        },
        components: if rgb { "rgb" } else { "r" },
        default: channel.default(),
        range: [
            0.,
            if channel == Channel::Emission {
                65504.
            } else {
                1.
            },
        ],
        color_space: if channel == Channel::Normal {
            "linear vector"
        } else if rgb {
            "linear sRGB primaries"
        } else {
            "linear scalar"
        },
        alpha: if rgb {
            "constant one; straight RGB"
        } else {
            "separate scalar"
        },
    }
}
pub struct SurfaceMap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub layout: OutputLayout,
}
pub struct Surface {
    pub maps: [SurfaceMap; 5],
    pub resolution: [u32; 2],
    owner: u64,
}
impl Surface {
    pub fn map(&self, channel: Channel) -> Option<&SurfaceMap> {
        self.maps.iter().find(|m| m.layout.channel == channel)
    }
}
pub struct PreviewSurface {
    pub maps: [SurfaceMap; 7],
    pub resolution: [u32; 2],
    owner: u64,
}
impl PreviewSurface {
    pub fn map(&self, channel: Channel) -> Option<&SurfaceMap> {
        self.maps.iter().find(|m| m.layout.channel == channel)
    }
}
struct Field {
    shader: wgsl::Shader,
    pipeline: wgpu::ComputePipeline,
    passes: Vec<wgpu::ComputePipeline>,
}
pub struct MaterialInstance {
    asset: EditableMaterialAsset,
    record: InstanceRecord,
    fields: Vec<Field>,
    owner: u64,
    generation: u64,
}
impl MaterialInstance {
    pub fn snapshot(&self) -> InstanceRecord {
        self.record.clone()
    }
    pub fn asset(&self) -> &EditableMaterialAsset {
        &self.asset
    }
    /// Authored channels outside the minimal profile remain in the editable asset.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let mut seen = std::collections::HashSet::new();
        if let Some(m) = self
            .asset
            .documents()
            .flat_map(|d| &d.project.materials)
            .find(|m| m.id == self.asset.selected_material)
        {
            for name in m
                .defaults
                .iter()
                .map(|(n, _)| n.as_str())
                .chain(m.outputs.iter().map(|o| o.channel.as_str()))
            {
                if seen.insert(name)
                    && !self
                        .fields
                        .iter()
                        .any(|f| f.shader.plan.channel.name() == name)
                {
                    let mut d=self.asset.error("profile",format!("The Eldiron profile omits authored {name}; its editable data is retained"));
                    d.severity = crate::integration::Severity::Warning;
                    diagnostics.push(d);
                }
            }
        }
        diagnostics
    }
    /// Unchanged by compatible value updates; useful for host reuse telemetry.
    pub fn pipeline_generation(&self) -> u64 {
        self.generation
    }
    pub fn set_parameter(&mut self, key: ParameterKey, value: Vec<f32>) -> Result<()> {
        let location = self.asset.locate(&key)?;
        let mut next = self.record.clone();
        next.set(key, value);
        self.update(next)
            .map_err(|e| Diagnostic::new("update", location, e.message))
    }
    /// Transactional. Incompatible structures require a new compiled instance.
    pub fn update(&mut self, record: InstanceRecord) -> Result<()> {
        self.apply(self.asset.clone(), record)
    }
    /// Refresh edited recipe values/defaults. Structural changes return a
    /// diagnostic and leave this instance and its pipelines untouched.
    pub fn update_asset(&mut self, asset: &EditableMaterialAsset) -> Result<()> {
        if asset.root.id != self.asset.root.id
            || asset.selected_material != self.asset.selected_material
        {
            return Err(self.asset.error(
                "update",
                "Changing asset identity or selected material requires compilation",
            ));
        }
        self.apply(asset.clone(), self.record.clone())
    }
    fn apply(&mut self, asset: EditableMaterialAsset, record: InstanceRecord) -> Result<()> {
        let program = asset.program(&record)?;
        let mut shaders = self
            .fields
            .iter()
            .map(|f| f.shader.clone())
            .collect::<Vec<_>>();
        for shader in &mut shaders {
            shader.refresh(&program).map_err(|e| {
                self.asset.error(
                    "update",
                    format!("Pipeline structure changed; compile a new instance: {e}"),
                )
            })?;
        }
        for (field, shader) in self.fields.iter_mut().zip(shaders) {
            field.shader = shader;
        }
        self.record = record;
        self.asset = asset;
        Ok(())
    }
}

pub struct MaterialZ {
    device: wgpu::Device,
    // Retains the host queue; encoding stages immutable snapshots on its encoder.
    _queue: wgpu::Queue,
    owner: u64,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pack_rgb: wgpu::ComputePipeline,
    pack_scalar: wgpu::ComputePipeline,
    mip: wgpu::ComputePipeline,
}
async fn pipeline(
    device: &wgpu::Device,
    source: &str,
    layout: Option<&wgpu::PipelineLayout>,
    location: Location,
) -> Result<wgpu::ComputePipeline> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("MaterialZ"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
    });
    let result = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("MaterialZ"),
        layout,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(error) = scope.pop().await {
        return Err(Diagnostic::new("compile", location, error));
    }
    Ok(result)
}
fn bytes(values: &[[f32; 4]]) -> Vec<u8> {
    values
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
impl MaterialZ {
    pub async fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let location = Location::asset("MaterialZ runtime");
        if device.limits().max_storage_buffers_per_shader_stage < 4 {
            return Err(Diagnostic::new(
                "initialize",
                location,
                "At least four storage buffers per compute stage are required",
            ));
        }
        let entries = std::array::from_fn::<_, 4, _>(|i| wgpu::BindGroupLayoutEntry {
            binding: i as u32,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: matches!(i, 1 | 2),
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MaterialZ evaluation"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MaterialZ evaluation"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pack_rgb = pipeline(
            device,
            &PACK.replace("OUTPUT_FORMAT", "rgba16float"),
            None,
            location.clone(),
        )
        .await?;
        let pack_scalar = pipeline(
            device,
            &PACK.replace("OUTPUT_FORMAT", "r32float"),
            None,
            location.clone(),
        )
        .await?;
        let mip = pipeline(device, MIP, None, location).await?;
        Ok(Self {
            device: device.clone(),
            _queue: queue.clone(),
            owner: NEXT.fetch_add(1, Ordering::Relaxed),
            layout,
            pipeline_layout,
            pack_rgb,
            pack_scalar,
            mip,
        })
    }
    pub async fn compile(
        &self,
        asset: &EditableMaterialAsset,
        record: InstanceRecord,
    ) -> Result<MaterialInstance> {
        self.compile_channels(asset, record, &CHANNELS).await
    }
    /// Full PBR authoring preview, including Height and Normal Map.
    pub async fn compile_preview(
        &self,
        asset: &EditableMaterialAsset,
        record: InstanceRecord,
    ) -> Result<MaterialInstance> {
        self.compile_channels(asset, record, &PREVIEW_CHANNELS)
            .await
    }
    async fn compile_channels(
        &self,
        asset: &EditableMaterialAsset,
        record: InstanceRecord,
        channels: &[Channel],
    ) -> Result<MaterialInstance> {
        let program = asset.program(&record)?;
        let mut fields = vec![];
        for &channel in channels {
            let shader = wgsl::compile(&program, &asset.selected_material, channel)
                .map_err(|e| asset.error("compile", format!("{}: {e}", channel.name())))?;
            let location = asset.error("compile", "").location;
            let main = pipeline(
                &self.device,
                &shader.source,
                Some(&self.pipeline_layout),
                location.clone(),
            )
            .await?;
            let mut passes = vec![];
            for source in &shader.passes {
                passes.push(
                    pipeline(
                        &self.device,
                        source,
                        Some(&self.pipeline_layout),
                        location.clone(),
                    )
                    .await?,
                );
            }
            fields.push(Field {
                shader,
                pipeline: main,
                passes,
            });
        }
        Ok(MaterialInstance {
            asset: asset.clone(),
            record,
            fields,
            owner: self.owner,
            generation: NEXT.fetch_add(1, Ordering::Relaxed),
        })
    }
    fn check_request(&self, request: &RenderRequest) -> Result<()> {
        let [w, h] = request.resolution;
        let d = request.domain;
        let determinant = d.axis_u[0] * d.axis_v[1] - d.axis_u[1] * d.axis_v[0];
        if !request.time.is_finite()
            || w == 0
            || h == 0
            || w > self.device.limits().max_texture_dimension_2d
            || h > self.device.limits().max_texture_dimension_2d
            || d.origin
                .into_iter()
                .chain(d.axis_u)
                .chain(d.axis_v)
                .any(|v| !v.is_finite())
            || !determinant.is_finite()
            || determinant == 0.
        {
            return Err(Diagnostic::new(
                "render",
                Location::asset("MaterialZ runtime"),
                "Request needs supported nonzero dimensions and a finite nonsingular coordinate domain",
            ));
        }
        if u64::from(w) * u64::from(h) * 124 > 256 * 1024 * 1024 {
            return Err(Diagnostic::new(
                "render",
                Location::asset("MaterialZ runtime"),
                "Output and evaluation maps exceed the 256 MB working budget",
            ));
        }
        Ok(())
    }
    pub fn create_surface(&self, request: &RenderRequest) -> Result<Surface> {
        self.check_request(request)?;
        let maps = output_layout().map(|layout| self.create_map(request, layout));
        Ok(Surface {
            maps,
            resolution: request.resolution,
            owner: self.owner,
        })
    }
    pub fn create_preview_surface(&self, request: &RenderRequest) -> Result<PreviewSurface> {
        self.check_request(request)?;
        Ok(PreviewSurface {
            maps: preview_output_layout().map(|layout| self.create_map(request, layout)),
            resolution: request.resolution,
            owner: self.owner,
        })
    }
    fn create_map(&self, request: &RenderRequest, layout: OutputLayout) -> SurfaceMap {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(layout.channel.name()),
            size: wgpu::Extent3d {
                width: request.resolution[0],
                height: request.resolution[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: layout.format,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        SurfaceMap {
            texture,
            view,
            layout,
        }
    }
    fn buffer(&self, label: &str, content: &[u8]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: content,
                usage: wgpu::BufferUsages::STORAGE,
            })
    }
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        material: &MaterialInstance,
        request: &RenderRequest,
        surface: &Surface,
    ) -> Result<RenderTicket> {
        if surface.owner != self.owner || surface.resolution != request.resolution {
            return Err(material.asset.error(
                "render",
                "Output surface belongs to another runtime or resolution",
            ));
        }
        self.encode_into(
            encoder,
            material,
            request,
            std::array::from_fn(|i| &surface.maps[i].view),
        )
    }
    pub fn encode_preview(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        material: &MaterialInstance,
        request: &RenderRequest,
        surface: &PreviewSurface,
    ) -> Result<RenderTicket> {
        if surface.owner != self.owner || surface.resolution != request.resolution {
            return Err(material.asset.error(
                "render",
                "Output surface belongs to another runtime or resolution",
            ));
        }
        let targets = surface.maps.iter().map(|map| &map.view).collect::<Vec<_>>();
        self.encode_targets(
            encoder,
            material,
            request,
            &targets,
            &preview_output_layout(),
        )
    }
    /// Caller-owned full 2D views in output_layout() order, with matching formats.
    pub fn encode_into(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        material: &MaterialInstance,
        request: &RenderRequest,
        targets: [&wgpu::TextureView; 5],
    ) -> Result<RenderTicket> {
        self.encode_targets(encoder, material, request, &targets, &output_layout())
    }
    fn encode_targets(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        material: &MaterialInstance,
        request: &RenderRequest,
        targets: &[&wgpu::TextureView],
        layouts: &[OutputLayout],
    ) -> Result<RenderTicket> {
        self.check_request(request)
            .map_err(|e| material.asset.error("render", e.message))?;
        if material.owner != self.owner {
            return Err(material
                .asset
                .error("render", "Material belongs to another runtime"));
        }
        for (target, layout) in targets.iter().zip(layouts) {
            let t = target.texture();
            if t.format() != layout.format
                || t.dimension() != wgpu::TextureDimension::D2
                || t.sample_count() != 1
                || t.width() != request.resolution[0]
                || t.height() != request.resolution[1]
                || t.depth_or_array_layers() != 1
                || !t.usage().contains(wgpu::TextureUsages::STORAGE_BINDING)
            {
                return Err(material.asset.error(
                    "render",
                    format!("Invalid target for {}", layout.channel.name()),
                ));
            }
        }
        let field_indices = layouts
            .iter()
            .map(|layout| {
                material.fields.iter().position(|field| field.shader.plan.channel == layout.channel)
                    .ok_or_else(|| material.asset.error("render", format!("The compiled profile omits {}; use compile_preview for authoring maps", layout.channel.name())))
            })
            .collect::<Result<Vec<_>>>()?;
        let strength_index = material
            .fields
            .iter()
            .position(|field| field.shader.plan.channel == Channel::EmissionStrength)
            .ok_or_else(|| {
                material
                    .asset
                    .error("render", "The compiled profile omits emission strength")
            })?;
        let mut mapped = *request;
        mapped.domain = mapped.domain.mapped(material.record.uv_transform);
        self.check_request(&mapped)
            .map_err(|e| material.asset.error("render", e.message))?;
        let plans = material
            .fields
            .iter()
            .map(|f| CachePlan::new(&f.shader, &mapped))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| material.asset.error("render", e))?;
        let pixel_bytes = u64::from(request.resolution[0]) * u64::from(request.resolution[1]) * 16;
        let output_bytes = layouts
            .iter()
            .map(|layout| {
                if layout.format == wgpu::TextureFormat::Rgba16Float {
                    8
                } else {
                    4
                }
            })
            .sum::<u64>();
        let mut total = pixel_bytes * material.fields.len() as u64
            + u64::from(request.resolution[0]) * u64::from(request.resolution[1]) * output_bytes;
        for plan in &plans {
            total += plan.bytes;
            if plan.bytes > self.device.limits().max_buffer_size
                || plan.bytes > u64::from(self.device.limits().max_storage_buffer_binding_size)
            {
                return Err(material
                    .asset
                    .error("render", "Intermediate buffer exceeds device limits"));
            }
        }
        if total > 256 * 1024 * 1024
            || pixel_bytes > u64::from(self.device.limits().max_storage_buffer_binding_size)
        {
            return Err(material.asset.error(
                "render",
                "Render exceeds the 256 MB working budget or storage-buffer limits",
            ));
        }
        let mut outputs = vec![];
        for (field, plan) in material.fields.iter().zip(plans) {
            let output = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MaterialZ field"),
                size: pixel_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let cache = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("MaterialZ intermediates"),
                size: plan.bytes.max(16),
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            let mut parameter_bytes = field.shader.parameter_bytes();
            if parameter_bytes.is_empty() {
                parameter_bytes.resize(16, 0);
            }
            let parameters = self.buffer("MaterialZ immutable parameters", &parameter_bytes);
            for (i, pipeline) in field.passes.iter().enumerate() {
                let config = self.buffer(
                    "MaterialZ cache domain",
                    &bytes(&plan.config(
                        plan.resolution,
                        SurfaceDomain::rect(plan.origin, plan.span),
                        request.time,
                    )),
                );
                self.dispatch(
                    encoder,
                    pipeline,
                    &output,
                    &config,
                    &parameters,
                    &cache,
                    plan.resolution,
                );
                for level in 1..plan.levels {
                    let config = self.buffer(
                        "MaterialZ mip config",
                        &words(&[
                            plan.resolution[0],
                            plan.resolution[1],
                            plan.stride,
                            level,
                            i as u32 * 2,
                            0,
                            0,
                            0,
                        ]),
                    );
                    let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("MaterialZ mip"),
                        layout: &self.mip.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: cache.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: config.as_entire_binding(),
                            },
                        ],
                    });
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("MaterialZ mip"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.mip);
                    pass.set_bind_group(0, &group, &[]);
                    pass.dispatch_workgroups(
                        (plan.resolution[0] >> level).max(1).div_ceil(8),
                        (plan.resolution[1] >> level).max(1).div_ceil(8),
                        2,
                    );
                }
            }
            let config = self.buffer(
                "MaterialZ output domain",
                &bytes(&plan.config(mapped.resolution, mapped.domain, request.time)),
            );
            self.dispatch(
                encoder,
                &field.pipeline,
                &output,
                &config,
                &parameters,
                &cache,
                mapped.resolution,
            );
            outputs.push(output);
        }
        let status = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MaterialZ diagnostic flags"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        for (i, layout) in layouts.iter().enumerate() {
            let rgb = layout.format == wgpu::TextureFormat::Rgba16Float;
            let pipeline = if rgb {
                &self.pack_rgb
            } else {
                &self.pack_scalar
            };
            let config = self.buffer(
                "MaterialZ output packing",
                &words(&[
                    request.resolution[0],
                    request.resolution[1],
                    i as u32,
                    u32::from(layout.channel == Channel::Emission),
                ]),
            );
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MaterialZ output packing"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: outputs[field_indices[i]].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: outputs[strength_index].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(targets[i]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: config.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: status.as_entire_binding(),
                    },
                ],
            });
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MaterialZ output packing"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                request.resolution[0].div_ceil(8),
                request.resolution[1].div_ceil(8),
                1,
            );
        }
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MaterialZ diagnostics readback"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&status, 0, &readback, 0, 4);
        Ok(RenderTicket {
            readback,
            location: material.asset.error("render", "").location,
        })
    }
    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        output: &wgpu::Buffer,
        config: &wgpu::Buffer,
        parameters: &wgpu::Buffer,
        cache: &wgpu::Buffer,
        size: [u32; 2],
    ) {
        let entries = [output, config, parameters, cache].map(|buffer| buffer.as_entire_binding());
        let entries = std::array::from_fn::<_, 4, _>(|i| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: entries[i].clone(),
        });
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MaterialZ evaluation"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("MaterialZ evaluation"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(size[0].div_ceil(8), size[1].div_ceil(8), 1);
    }
}
pub struct RenderTicket {
    pub readback: wgpu::Buffer,
    location: Location,
}
impl RenderTicket {
    /// Call after submitting the host encoder, then let the host poll its device.
    pub fn map_diagnostics(&self, callback: impl FnOnce(Result<()>) + Send + 'static) {
        let buffer = self.readback.clone();
        let location = self.location.clone();
        self.readback.map_async(wgpu::MapMode::Read,..,move |result| {
            let result=result.map_err(|e|Diagnostic::new("readback",location.clone(),e)).and_then(|_| {
                let data=buffer.get_mapped_range(..);let flags=u32::from_le_bytes(data[..4].try_into().unwrap());drop(data);buffer.unmap();
                if flags==0 {Ok(())}else{Err(Diagnostic::new("render",location,format!("Nonfinite or out-of-format output; channel flags {flags:#x} (Color, Opacity, Roughness, Metallic, Emission, Height, Normal Map)")))}
            });callback(result);
        });
    }
}

struct CachePlan {
    resolution: [u32; 2],
    origin: [f32; 2],
    span: [f32; 2],
    stride: u32,
    levels: u32,
    bytes: u64,
}
impl CachePlan {
    fn new(shader: &wgsl::Shader, request: &RenderRequest) -> std::result::Result<Self, String> {
        if shader.passes.is_empty() {
            return Ok(Self {
                resolution: [1, 1],
                origin: [0., 0.],
                span: [1., 1.],
                stride: 1,
                levels: 1,
                bytes: 16,
            });
        }
        let p = &shader.plan;
        let d = request.domain;
        let mut low = [f32::INFINITY; 2];
        let mut high = [f32::NEG_INFINITY; 2];
        let mut spacing = f32::INFINITY;
        let mut max_spacing = 0f32;
        let mut scale = 1f32;
        for (i, m) in p.domains.iter().enumerate() {
            let v = |a: [f32; 2]| [m[0] * a[0] + m[1] * a[1], m[2] * a[0] + m[3] * a[1]];
            let a = v(d.axis_u);
            let b = v(d.axis_v);
            let step = (a[0].hypot(a[1]) / request.resolution[0] as f32)
                .max(b[0].hypot(b[1]) / request.resolution[1] as f32);
            spacing = spacing.min(step);
            max_spacing = max_spacing.max(step);
            scale = scale.max((m[0].abs() + m[1].abs()).max(m[2].abs() + m[3].abs()));
            if !p.domain_fixed[i] {
                for u in [0., 1.] {
                    for w in [0., 1.] {
                        let point = v([
                            d.origin[0] + u * d.axis_u[0] + w * d.axis_v[0],
                            d.origin[1] + u * d.axis_u[1] + w * d.axis_v[1],
                        ]);
                        for j in 0..2 {
                            let x = point[j] + m[j + 4];
                            low[j] = low[j].min(x);
                            high[j] = high[j].max(x);
                        }
                    }
                }
            }
        }
        let mut fixed_scale = 0f32;
        for rect in &p.fixed_domains {
            for j in 0..2 {
                low[j] = low[j].min(rect[j]);
                high[j] = high[j].max(rect[j] + rect[j + 2]);
                fixed_scale = fixed_scale.max(rect[j + 2]);
            }
        }
        if fixed_scale > 0. {
            spacing = spacing.max(fixed_scale / 1024.);
        }
        spacing = spacing.max(1e-6);
        let halo = p.uv_padding * scale + p.pixel_padding * max_spacing;
        let mut resolution = [0; 2];
        let mut span = [0.; 2];
        for j in 0..2 {
            low[j] -= halo;
            high[j] += halo;
            let n = ((high[j] - low[j]) / spacing).ceil();
            if !n.is_finite() || !(1. ..=8192.).contains(&n) {
                return Err("Intermediate domain is nonfinite or exceeds 8192 pixels".into());
            }
            resolution[j] = n as u32;
            span[j] = n * spacing;
        }
        let mut size = resolution;
        let mut stride = 0u32;
        let mut levels = 0;
        loop {
            stride += size[0] * size[1];
            levels += 1;
            if size == [1, 1] {
                break;
            }
            size = size.map(|n| (n / 2).max(1));
        }
        // cfg stores this exactly as f32, so bound it before conversion.
        if stride > 16_777_216 {
            return Err("Intermediate mip chain exceeds address precision budget".into());
        }
        let bytes = u64::from(stride) * shader.passes.len() as u64 * 32;
        Ok(Self {
            resolution,
            origin: low,
            span,
            stride,
            levels,
            bytes,
        })
    }
    fn config(&self, size: [u32; 2], d: SurfaceDomain, time: f32) -> [[f32; 4]; 5] {
        [
            [
                size[0] as f32,
                size[1] as f32,
                self.resolution[0] as f32,
                self.resolution[1] as f32,
            ],
            [d.origin[0], d.origin[1], d.axis_u[0], d.axis_u[1]],
            [
                d.axis_v[0],
                d.axis_v[1],
                self.stride as f32,
                self.levels as f32,
            ],
            [self.origin[0], self.origin[1], self.span[0], self.span[1]],
            [time, 0., 0., 0.],
        ]
    }
}

const PACK: &str = r#"
@group(0) @binding(0) var<storage,read> source:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> strength:array<vec4<f32>>;
@group(0) @binding(2) var output:texture_storage_2d<OUTPUT_FORMAT,write>;
@group(0) @binding(3) var<storage,read> config:vec4<u32>;
@group(0) @binding(4) var<storage,read_write> status:array<atomic<u32>>;
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) gid:vec3<u32>) {
    if any(gid.xy>=config.xy){return;}
    let i=gid.y*config.x+gid.x;var v=source[i];
    if !all(abs(v)<=vec4<f32>(3.402823e38)) {atomicOr(&status[0],1u<<config.z);v=vec4<f32>(0.0);}
    if config.w==1u {
        let s=strength[i].x;
        if !(abs(s)<=3.402823e38) {atomicOr(&status[0],1u<<config.z);}
        v=vec4<f32>(max(v.xyz,vec3<f32>(0.0))*max(s,0.0),1.0);
        if !all(v.xyz<=vec3<f32>(65504.0)) {atomicOr(&status[0],1u<<config.z);v=vec4<f32>(0.0,0.0,0.0,1.0);}
    } else {v=clamp(v,vec4<f32>(0.0),vec4<f32>(1.0));v.w=1.0;}
    textureStore(output,vec2<i32>(gid.xy),v);
}
"#;
const MIP: &str = r#"
@group(0) @binding(0) var<storage,read_write> pixels:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read> cfg:array<u32>;
fn offset(level:u32)->u32 {var size=vec2<u32>(cfg[0],cfg[1]);var n=0u;for(var i=0u;i<level;i++){n+=size.x*size.y;size=max(size/2u,vec2<u32>(1u));}return n;}
@compute @workgroup_size(8,8) fn main(@builtin(global_invocation_id) gid:vec3<u32>) {
    let level=cfg[3];let size=max(vec2<u32>(cfg[0],cfg[1])>>vec2<u32>(level),vec2<u32>(1u));
    if any(gid.xy>=size){return;}
    let previous=max(vec2<u32>(cfg[0],cfg[1])>>vec2<u32>(level-1u),vec2<u32>(1u));
    let lo=vec2<f32>(gid.xy)*vec2<f32>(previous)/vec2<f32>(size);
    let hi=vec2<f32>(gid.xy+vec2<u32>(1u))*vec2<f32>(previous)/vec2<f32>(size);
    var sum=vec4<f32>(0.0);var weight=0.0;
    let layer=(cfg[4]+gid.z)*cfg[2];
    for(var y=u32(floor(lo.y));y<u32(ceil(hi.y));y++){for(var x=u32(floor(lo.x));x<u32(ceil(hi.x));x++){
        let w=max(0.0,min(hi.x,f32(x+1u))-max(lo.x,f32(x)))*max(0.0,min(hi.y,f32(y+1u))-max(lo.y,f32(y)));
        sum+=pixels[layer+offset(level-1u)+min(y,previous.y-1u)*previous.x+min(x,previous.x-1u)]*w;weight+=w;
    }}
    pixels[layer+offset(level)+gid.y*size.x+gid.x]=sum/max(weight,0.000001);
}
"#;
